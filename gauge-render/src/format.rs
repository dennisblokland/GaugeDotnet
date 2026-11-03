//! .NET-compatible formatting so gauge JSON written for the C# app renders the
//! same text: `float.ToString(format)` (invariant culture) and
//! `DateTime.ToString(format)`.
//!
//! The number half follows .NET's `Number.Formatting` closely: standard
//! formats get the exactly rounded decimal digits of the float (ties to
//! even, like .NET's Dragon4), custom formats get 7 significant digits that
//! the pattern then rounds half up, as .NET does.

use chrono::{Datelike, NaiveDateTime, Timelike};

/// Invariant culture's currency symbol.
const CURRENCY: &str = "\u{a4}";

/// `NumberBuffer`: decimal digits (0-9, no trailing zeros) with the decimal
/// point after `scale` digits. Zero has no digits.
struct Number {
	digits: Vec<u8>,
	scale: i32,
	negative: bool,
}

impl Number {
	/// Parses Rust's `d.ddde±x` output.
	fn from_exponential(s: &str, negative: bool) -> Self {
		let (mantissa, exp) = s.split_once('e').unwrap_or((s, "0"));
		let exp: i32 = exp.parse().unwrap_or(0);
		let digits = mantissa.bytes().filter(u8::is_ascii_digit).map(|b| b - b'0').collect();
		Self::normalized(digits, exp + 1, negative)
	}

	/// Parses Rust's fixed `ddd.ddd` output.
	fn from_fixed(s: &str, negative: bool) -> Self {
		let int_len = s.find('.').unwrap_or(s.len()) as i32;
		let digits = s.bytes().filter(u8::is_ascii_digit).map(|b| b - b'0').collect();
		Self::normalized(digits, int_len, negative)
	}

	/// Drops leading and trailing zeros; zero gets no digits and scale 0.
	fn normalized(mut digits: Vec<u8>, mut scale: i32, negative: bool) -> Self {
		let lead = digits.iter().take_while(|&&d| d == 0).count();
		digits.drain(..lead);
		scale -= lead as i32;
		while digits.last() == Some(&0) {
			digits.pop();
		}
		if digits.is_empty() {
			scale = 0;
		}
		Self { digits, scale, negative }
	}

	/// The shortest digits that round-trip (`R`, `G`, empty format).
	fn shortest(v: f32) -> Self {
		if v == 0.0 {
			return Self { digits: Vec::new(), scale: 0, negative: v.is_sign_negative() };
		}
		Self::from_exponential(&format!("{:e}", v.abs()), v.is_sign_negative())
	}

	/// `count` significant digits, exactly rounded (ties to even).
	fn significant(v: f32, count: usize) -> Self {
		if v == 0.0 {
			return Self { digits: Vec::new(), scale: 0, negative: v.is_sign_negative() };
		}
		Self::from_exponential(&format!("{:.*e}", count.max(1) - 1, v.abs()), v.is_sign_negative())
	}

	/// Rounded to `count` fractional digits, exactly (ties to even).
	fn fractional(v: f32, count: usize) -> Self {
		Self::from_fixed(&format!("{:.*}", count, v.abs()), v.is_sign_negative())
	}

	/// `-1` past the last digit, like the NUL terminator .NET reads.
	fn digit(&self, i: usize) -> Option<u8> {
		self.digits.get(i).copied()
	}

	/// `RoundNumber` for custom formats: keeps `pos` digits, rounding half up.
	fn round(&mut self, pos: i32) {
		let mut i = (pos.max(0) as usize).min(self.digits.len());
		if pos >= 0 && (pos as usize) < self.digits.len() && self.digits[pos as usize] >= 5 {
			while i > 0 && self.digits[i - 1] == 9 {
				i -= 1;
			}
			if i > 0 {
				self.digits[i - 1] += 1;
			} else {
				self.scale += 1;
				self.digits[0] = 1;
				i = 1;
			}
		} else {
			while i > 0 && self.digits[i - 1] == 0 {
				i -= 1;
			}
		}
		self.digits.truncate(i);
		if i == 0 {
			self.scale = 0;
		}
	}
}

fn push_digit(out: &mut String, d: Option<u8>) {
	out.push((b'0' + d.unwrap_or(0)) as char);
}

/// `FormatFixed` with the invariant culture: groups of three when `group`.
fn format_fixed(out: &mut String, n: &Number, decimals: usize, group: bool) {
	let int_len = n.scale.max(0) as usize;
	if int_len > 0 {
		for i in 0..int_len {
			push_digit(out, n.digit(i));
			let left = int_len - 1 - i;
			if group && left > 0 && left % 3 == 0 {
				out.push(',');
			}
		}
	} else {
		out.push('0');
	}
	if decimals > 0 {
		out.push('.');
		let leading_zeros = ((-n.scale).max(0) as usize).min(decimals);
		out.extend(std::iter::repeat('0').take(leading_zeros));
		for i in 0..decimals - leading_zeros {
			push_digit(out, n.digit(int_len + i));
		}
	}
}

/// `FormatExponent`: `E+003`, `e-05`, ...
fn format_exponent(out: &mut String, value: i32, exp_char: char, min_digits: usize, positive_sign: bool) {
	out.push(exp_char);
	if value < 0 {
		out.push('-');
	} else if positive_sign {
		out.push('+');
	}
	out.push_str(&format!("{:0min_digits$}", value.unsigned_abs()));
}

/// `FormatGeneral`: plain digits, or scientific notation when the exponent
/// is above `max_digits` or below -4.
fn format_general(out: &mut String, n: &Number, max_digits: i32, exp_char: char) {
	let scientific = n.scale > max_digits || n.scale < -3;
	let mut pos = if scientific { 1 } else { n.scale };
	let mut i = 0;
	if pos > 0 {
		while pos > 0 {
			push_digit(out, n.digit(i));
			i = (i + 1).min(n.digits.len());
			pos -= 1;
		}
	} else {
		out.push('0');
	}
	if i < n.digits.len() || pos < 0 {
		out.push('.');
		while pos < 0 {
			out.push('0');
			pos += 1;
		}
		for &d in &n.digits[i..] {
			out.push((b'0' + d) as char);
		}
	}
	if scientific {
		format_exponent(out, n.scale - 1, exp_char, 2, true);
	}
}

/// A standard format (`F2`, `N`, `G3`, ...), or `None` for a custom pattern.
fn standard(v: f32, format: &str) -> Option<String> {
	let mut chars = format.chars();
	let spec = chars.next().filter(char::is_ascii_alphabetic)?;
	let rest = chars.as_str();
	if !rest.bytes().all(|b| b.is_ascii_digit()) {
		return None;
	}
	let precision: Option<usize> = if rest.is_empty() { None } else { Some(rest.parse().ok()?) };
	let mut out = String::new();
	match spec {
		'F' | 'f' | 'N' | 'n' => {
			let decimals = precision.unwrap_or(2);
			let n = Number::fractional(v, decimals);
			if n.negative {
				out.push('-');
			}
			format_fixed(&mut out, &n, decimals, matches!(spec, 'N' | 'n'));
		}
		'P' | 'p' => {
			let decimals = precision.unwrap_or(2);
			let mut n = Number::fractional(v, decimals + 2);
			if !n.digits.is_empty() {
				n.scale += 2;
			}
			if n.negative {
				out.push('-');
			}
			format_fixed(&mut out, &n, decimals, true);
			out.push_str(" %");
		}
		'C' | 'c' => {
			let decimals = precision.unwrap_or(2);
			let n = Number::fractional(v, decimals);
			if n.negative {
				out.push('(');
			}
			out.push_str(CURRENCY);
			format_fixed(&mut out, &n, decimals, true);
			if n.negative {
				out.push(')');
			}
		}
		'E' | 'e' => {
			let decimals = precision.unwrap_or(6);
			let n = Number::significant(v, decimals + 1);
			if n.negative {
				out.push('-');
			}
			push_digit(&mut out, n.digit(0));
			if decimals > 0 {
				out.push('.');
			}
			for i in 1..=decimals {
				push_digit(&mut out, n.digit(i));
			}
			let exp = if n.digits.is_empty() { 0 } else { n.scale - 1 };
			format_exponent(&mut out, exp, spec, 3, true);
		}
		'G' | 'g' | 'R' | 'r' => {
			let exp_char = if spec.is_ascii_uppercase() { 'E' } else { 'e' };
			let (n, max_digits) = match precision {
				Some(p) if p > 0 && matches!(spec, 'G' | 'g') => (Number::significant(v, p), p as i32),
				_ => {
					let n = Number::shortest(v);
					let max = n.digits.len().max(9) as i32;
					(n, max)
				}
			};
			if n.negative {
				out.push('-');
			}
			format_general(&mut out, &n, max_digits, exp_char);
		}
		// "D"/"X" throw for floats in .NET; render the rounded integer instead of crashing.
		'D' | 'd' | 'X' | 'x' => return Some(standard(v, "F0").unwrap_or_default()),
		_ => return None,
	}
	Some(out)
}

/// Formats like `float.ToString(format, CultureInfo.InvariantCulture)`.
///
/// Supports the standard `C`, `E`, `F`, `G`, `N`, `P` and `R` specifiers
/// (with optional precision) and custom patterns such as `0.0`, `#,##0`,
/// `00.00 'bar'`, `0.0E+00` or `+0.0;-0.0;zero`.
pub fn format_number(v: f32, format: &str) -> String {
	if v.is_nan() {
		return "NaN".into();
	}
	if v.is_infinite() {
		return if v > 0.0 { "Infinity".into() } else { "-Infinity".into() };
	}
	if format.is_empty() {
		return standard(v, "G").unwrap_or_default();
	}
	standard(v, format).unwrap_or_else(|| custom(v, format))
}

/// `FindSection`: where section `n` (0 positive, 1 negative, 2 zero) starts,
/// or 0 when it is missing or empty.
fn find_section(format: &[char], mut section: usize) -> usize {
	if section == 0 {
		return 0;
	}
	let mut i = 0;
	while i < format.len() {
		let c = format[i];
		i += 1;
		match c {
			'\'' | '"' => {
				while i < format.len() {
					i += 1;
					if format[i - 1] == c {
						break;
					}
				}
			}
			'\\' => i = (i + 1).min(format.len()),
			';' => {
				section -= 1;
				if section == 0 {
					return if i < format.len() && format[i] != ';' { i } else { 0 };
				}
			}
			_ => {}
		}
	}
	0
}

/// Custom numeric format strings (`NumberToStringFormat`): `0`, `#`, `.`,
/// `,` (grouping and /1000 scaling), `%`, `‰`, `E+0`-style exponents,
/// quoted literals, `\` escapes and up to three `;` sections.
fn custom(v: f32, format: &str) -> String {
	let fmt: Vec<char> = format.chars().collect();
	let mut n = Number::significant(v, 7);
	let wanted = if n.digits.is_empty() {
		2
	} else if n.negative {
		1
	} else {
		0
	};
	let mut section = find_section(&fmt, wanted);

	// Scan the section's placeholders.
	let (digit_count, decimal_pos, first_digit, last_digit, mut scientific, thousand_seps);
	loop {
		let mut count = 0i32;
		let mut decimal = -1i32;
		let mut first = i32::MAX;
		let mut last = 0i32;
		let mut sci = false;
		let mut thousand_pos = -1i32;
		let mut thousand_count = 0i32;
		let mut seps = false;
		let mut scale_adjust = 0i32;
		let mut i = section;
		while i < fmt.len() && fmt[i] != ';' {
			let c = fmt[i];
			i += 1;
			match c {
				'#' => count += 1,
				'0' => {
					if first == i32::MAX {
						first = count;
					}
					count += 1;
					last = count;
				}
				'.' => {
					if decimal < 0 {
						decimal = count;
					}
				}
				',' => {
					if count > 0 && decimal < 0 {
						if thousand_pos >= 0 {
							if thousand_pos == count {
								thousand_count += 1;
								continue;
							}
							seps = true;
						}
						thousand_pos = count;
						thousand_count = 1;
					}
				}
				'%' => scale_adjust += 2,
				'\u{2030}' => scale_adjust += 3,
				'\'' | '"' => {
					while i < fmt.len() {
						i += 1;
						if fmt[i - 1] == c {
							break;
						}
					}
				}
				'\\' => i = (i + 1).min(fmt.len()),
				'E' | 'e' => {
					let next = fmt.get(i).copied();
					let after = fmt.get(i + 1).copied();
					if next == Some('0') || (matches!(next, Some('+' | '-')) && after == Some('0')) {
						i += 1;
						while fmt.get(i) == Some(&'0') {
							i += 1;
						}
						sci = true;
					}
				}
				_ => {}
			}
		}
		if decimal < 0 {
			decimal = count;
		}
		if thousand_pos >= 0 {
			if thousand_pos == decimal {
				scale_adjust -= thousand_count * 3;
			} else {
				seps = true;
			}
		}
		if !n.digits.is_empty() {
			n.scale += scale_adjust;
			let pos = if sci { count } else { n.scale + count - decimal };
			n.round(pos);
			if n.digits.is_empty() {
				let zero = find_section(&fmt, 2);
				if zero != section {
					section = zero;
					continue;
				}
			}
		} else {
			n.scale = 0;
		}
		(digit_count, decimal_pos, first_digit, last_digit, scientific, thousand_seps) =
			(count, decimal, first, last, sci, seps);
		break;
	}

	let first_digit = if first_digit < decimal_pos { decimal_pos - first_digit } else { 0 };
	let last_digit = if last_digit > decimal_pos { decimal_pos - last_digit } else { 0 };
	let (mut dig_pos, mut adjust) =
		if scientific { (decimal_pos, 0) } else { (n.scale.max(decimal_pos), n.scale - decimal_pos) };

	// Digit positions (counted from the decimal point) that get a separator after them.
	let mut seps: Vec<i32> = Vec::new();
	if thousand_seps {
		let total = dig_pos + adjust.min(0);
		let digits = first_digit.max(total);
		let mut at = 3;
		while digits > at {
			seps.push(at);
			at += 3;
		}
	}
	let separator_after = |out: &mut String, seps: &mut Vec<i32>, dig_pos: i32| {
		if thousand_seps && dig_pos > 1 && seps.last() == Some(&(dig_pos - 1)) {
			out.push(',');
			seps.pop();
		}
	};

	let mut out = String::new();
	if n.negative && section == 0 && n.scale != 0 {
		out.push('-');
	}
	let mut cur = 0usize;
	let mut decimal_written = false;
	let mut i = section;
	while i < fmt.len() && fmt[i] != ';' {
		let c = fmt[i];
		i += 1;
		if adjust > 0 && matches!(c, '#' | '0' | '.') {
			while adjust > 0 {
				push_digit(&mut out, n.digit(cur));
				cur = (cur + 1).min(n.digits.len());
				separator_after(&mut out, &mut seps, dig_pos);
				dig_pos -= 1;
				adjust -= 1;
			}
		}
		match c {
			'#' | '0' => {
				let d = if adjust < 0 {
					adjust += 1;
					(dig_pos <= first_digit).then_some('0')
				} else if let Some(d) = n.digit(cur) {
					cur += 1;
					Some((b'0' + d) as char)
				} else {
					(dig_pos > last_digit).then_some('0')
				};
				if let Some(d) = d {
					out.push(d);
					separator_after(&mut out, &mut seps, dig_pos);
				}
				dig_pos -= 1;
			}
			'.' => {
				if dig_pos == 0
					&& !decimal_written
					&& (last_digit < 0 || (decimal_pos < digit_count && cur < n.digits.len()))
				{
					out.push('.');
					decimal_written = true;
				}
			}
			'\u{2030}' => out.push('\u{2030}'),
			'%' => out.push('%'),
			',' => {}
			'\'' | '"' => {
				while i < fmt.len() && fmt[i] != c {
					out.push(fmt[i]);
					i += 1;
				}
				i = (i + 1).min(fmt.len());
			}
			'\\' => {
				if let Some(&next) = fmt.get(i) {
					out.push(next);
					i += 1;
				}
			}
			'E' | 'e' => {
				let next = fmt.get(i).copied();
				let after = fmt.get(i + 1).copied();
				if scientific && (next == Some('0') || (matches!(next, Some('+' | '-')) && after == Some('0'))) {
					let positive_sign = next == Some('+');
					let mut zeros = usize::from(next == Some('0'));
					i += 1;
					while fmt.get(i) == Some(&'0') {
						zeros += 1;
						i += 1;
					}
					let exp = if n.digits.is_empty() { 0 } else { n.scale - decimal_pos };
					format_exponent(&mut out, exp, c, zeros.min(10), positive_sign);
					scientific = false;
				} else {
					out.push(c);
					if matches!(next, Some('+' | '-')) {
						out.push(next.unwrap());
						i += 1;
					}
					while fmt.get(i) == Some(&'0') {
						out.push('0');
						i += 1;
					}
				}
			}
			other => out.push(other),
		}
	}
	if n.negative && section == 0 && n.scale == 0 && !out.is_empty() {
		out.insert(0, '-');
	}
	out
}

const DAY_NAMES: [&str; 7] = ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"];
const MONTH_NAMES: [&str; 12] = [
	"January",
	"February",
	"March",
	"April",
	"May",
	"June",
	"July",
	"August",
	"September",
	"October",
	"November",
	"December",
];

/// `DateTime.Kind`, plus the local zone's offset from UTC, which the `z` and
/// `K` specifiers print.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DateTimeKind {
	/// `DateTime.Now`.
	Local { offset_secs: i32 },
	/// `DateTime.UtcNow`.
	Utc,
	/// No zone: `K` prints nothing, `z` the local offset.
	Unspecified { offset_secs: i32 },
}

impl DateTimeKind {
	/// The offset `z` prints: zero for UTC times, the local offset otherwise.
	fn offset_secs(self) -> i32 {
		match self {
			Self::Local { offset_secs } | Self::Unspecified { offset_secs } => offset_secs,
			Self::Utc => 0,
		}
	}
}

/// `+02:00`-style offset; `len` 1 gives `+2`, 2 gives `+02`, 3+ adds minutes.
fn push_offset(out: &mut String, offset_secs: i32, len: usize) {
	out.push(if offset_secs >= 0 { '+' } else { '-' });
	let minutes = offset_secs.unsigned_abs() / 60;
	let (h, m) = (minutes / 60, minutes % 60);
	match len {
		1 => out.push_str(&h.to_string()),
		2 => out.push_str(&format!("{h:02}")),
		_ => out.push_str(&format!("{h:02}:{m:02}")),
	}
}

/// Formats a date/time like `DateTime.ToString(format, InvariantCulture)`.
/// Returns `None` for formats .NET would reject (the clock then falls back to
/// `HH:mm:ss`, like the C# renderer).
pub fn format_datetime(dt: &NaiveDateTime, kind: DateTimeKind, format: &str) -> Option<String> {
	let mut dt = *dt;
	let mut kind = kind;
	let expanded: &str = if format.chars().count() == 1 {
		match format {
			"o" | "O" => "yyyy'-'MM'-'dd'T'HH':'mm':'ss'.'fffffffK",
			"r" | "R" => "ddd, dd MMM yyyy HH':'mm':'ss 'GMT'",
			// Full date and time in UTC: local times are converted first.
			"U" => {
				if let DateTimeKind::Local { offset_secs } | DateTimeKind::Unspecified { offset_secs } = kind {
					dt -= chrono::Duration::seconds(offset_secs.into());
					kind = DateTimeKind::Utc;
				}
				"dddd, dd MMMM yyyy HH:mm:ss"
			}
			"d" => "MM/dd/yyyy",
			"D" => "dddd, dd MMMM yyyy",
			"f" => "dddd, dd MMMM yyyy HH:mm",
			"F" => "dddd, dd MMMM yyyy HH:mm:ss",
			"g" => "MM/dd/yyyy HH:mm",
			"G" => "MM/dd/yyyy HH:mm:ss",
			"m" | "M" => "MMMM dd",
			"s" => "yyyy'-'MM'-'dd'T'HH':'mm':'ss",
			"t" => "HH:mm",
			"T" => "HH:mm:ss",
			"u" => "yyyy'-'MM'-'dd HH':'mm':'ss'Z'",
			"y" | "Y" => "yyyy MMMM",
			_ => return None,
		}
	} else if format.is_empty() {
		"MM/dd/yyyy HH:mm:ss"
	} else {
		format
	};

	let chars: Vec<char> = expanded.chars().collect();
	let mut out = String::new();
	let mut i = 0;
	while i < chars.len() {
		let c = chars[i];
		let mut n = 1;
		while i + n < chars.len() && chars[i + n] == c {
			n += 1;
		}
		match c {
			'd' => match n {
				1 => out.push_str(&dt.day().to_string()),
				2 => out.push_str(&format!("{:02}", dt.day())),
				3 => out.push_str(&DAY_NAMES[dt.weekday().num_days_from_sunday() as usize][..3]),
				_ => out.push_str(DAY_NAMES[dt.weekday().num_days_from_sunday() as usize]),
			},
			'M' => match n {
				1 => out.push_str(&dt.month().to_string()),
				2 => out.push_str(&format!("{:02}", dt.month())),
				3 => out.push_str(&MONTH_NAMES[dt.month0() as usize][..3]),
				_ => out.push_str(MONTH_NAMES[dt.month0() as usize]),
			},
			'y' => {
				let y = dt.year();
				match n {
					1 => out.push_str(&(y % 100).to_string()),
					2 => out.push_str(&format!("{:02}", y % 100)),
					_ => out.push_str(&format!("{:0width$}", y, width = n)),
				}
			}
			'h' => {
				let h = match dt.hour() % 12 {
					0 => 12,
					h => h,
				};
				out.push_str(&if n == 1 { h.to_string() } else { format!("{h:02}") });
			}
			'H' => out.push_str(&if n == 1 { dt.hour().to_string() } else { format!("{:02}", dt.hour()) }),
			'm' => out.push_str(&if n == 1 { dt.minute().to_string() } else { format!("{:02}", dt.minute()) }),
			's' => out.push_str(&if n == 1 { dt.second().to_string() } else { format!("{:02}", dt.second()) }),
			'f' | 'F' => {
				if n > 7 {
					return None;
				}
				let frac = format!("{:09}", dt.nanosecond() % 1_000_000_000);
				let mut digits = frac[..n].to_string();
				if c == 'F' {
					digits = digits.trim_end_matches('0').to_string();
					// No fraction left: the separator before it goes too.
					if digits.is_empty() && out.ends_with('.') {
						out.pop();
					}
				}
				out.push_str(&digits);
			}
			't' => {
				let ampm = if dt.hour() < 12 { "AM" } else { "PM" };
				out.push_str(if n == 1 { &ampm[..1] } else { ampm });
			}
			'g' => out.push_str("A.D."),
			'z' => push_offset(&mut out, kind.offset_secs(), n),
			'K' => {
				// Each K prints the zone once ("KK" twice).
				for _ in 0..n {
					match kind {
						DateTimeKind::Local { offset_secs } => push_offset(&mut out, offset_secs, 3),
						DateTimeKind::Utc => out.push('Z'),
						DateTimeKind::Unspecified { .. } => {}
					}
				}
			}
			'\'' | '"' => {
				let mut j = i + 1;
				while j < chars.len() && chars[j] != c {
					out.push(chars[j]);
					j += 1;
				}
				if j >= chars.len() {
					return None; // unterminated quote throws in .NET
				}
				i = j + 1;
				continue;
			}
			'\\' => {
				let next = chars.get(i + 1)?;
				out.push(*next);
				i += 2;
				continue;
			}
			'%' => {
				i += 1;
				continue;
			}
			_ => {
				for _ in 0..n {
					out.push(c);
				}
			}
		}
		i += n;
	}
	Some(out)
}

#[cfg(test)]
mod tests {
	use super::*;
	use chrono::NaiveDate;

	#[test]
	fn standard_numeric_formats() {
		assert_eq!(format_number(1234.4, "F0"), "1234");
		assert_eq!(format_number(14.7, "F1"), "14.7");
		assert_eq!(format_number(0.1, "F3"), "0.100");
		assert_eq!(format_number(999.96, "F1"), "1000.0");
		assert_eq!(format_number(-0.2, "F0"), "-0");
		assert_eq!(format_number(0.006, "F2"), "0.01");
		assert_eq!(format_number(1234567.0, "N0"), "1,234,567");
		assert_eq!(format_number(-1234.5, "N1"), "-1,234.5");
		assert_eq!(format_number(0.256, "P1"), "25.6 %");
		assert_eq!(format_number(-0.5, "P0"), "-50 %");
		assert_eq!(format_number(1234.5, "E2"), "1.23E+003");
		assert_eq!(format_number(0.0, "E1"), "0.0E+000");
		assert_eq!(format_number(1234.5, "e0"), "1e+003");
		assert_eq!(format_number(32.5, "F10"), "32.5000000000");
	}

	#[test]
	fn exact_values_round_ties_to_even() {
		// Standard formats round the float's exact value; true ties go to even.
		assert_eq!(format_number(2.675, "F2"), "2.67", "2.675f is 2.67499995...");
		assert_eq!(format_number(0.125, "F2"), "0.12");
		assert_eq!(format_number(0.375, "F2"), "0.38");
		assert_eq!(format_number(2.5, "F0"), "2");
		assert_eq!(format_number(1.5, "F0"), "2");
		assert_eq!(format_number(0.5, "F0"), "0");
		assert_eq!(format_number(1234.5, "F0"), "1234");
		// Custom formats round 7 significant digits half up.
		assert_eq!(format_number(2.5, "0"), "3");
		assert_eq!(format_number(0.125, "0.00"), "0.13");
		assert_eq!(format_number(2.675, "0.00"), "2.68", "2.675000 at 7 digits");
	}

	#[test]
	fn general_format() {
		assert_eq!(format_number(12.0, ""), "12");
		assert_eq!(format_number(12.5, "G"), "12.5");
		assert_eq!(format_number(-60.0, "G"), "-60");
		assert_eq!(format_number(1234.5, "G3"), "1.23E+03");
		assert_eq!(format_number(1234.5, "g2"), "1.2e+03");
		assert_eq!(format_number(1234.5, "G4"), "1234");
		assert_eq!(format_number(0.000123, "G2"), "0.00012");
		assert_eq!(format_number(0.0001, "G"), "0.0001");
		assert_eq!(format_number(0.00001, "G"), "1E-05");
		assert_eq!(format_number(1e9, "G"), "1E+09");
		assert_eq!(format_number(123456789.0, "G"), "123456790");
		assert_eq!(format_number(1e15, "R"), "1E+15");
		assert_eq!(format_number(4567.89101_f64 as f32, "G"), "4567.891");
		assert_eq!(format_number(f32::MAX, "G"), "3.4028235E+38");
		assert_eq!(format_number(f32::from_bits(1), "G"), "1E-45");
		assert_eq!(format_number(-0.0, "G"), "-0");
	}

	#[test]
	fn currency_format() {
		assert_eq!(format_number(32.5, "C"), "\u{a4}32.50");
		assert_eq!(format_number(-1234.567, "C1"), "(\u{a4}1,234.6)");
		assert_eq!(format_number(5.0, "c0"), "\u{a4}5");
	}

	#[test]
	fn custom_numeric_formats() {
		assert_eq!(format_number(1.23456, "0.00"), "1.23");
		assert_eq!(format_number(5.0, "00.0"), "05.0");
		assert_eq!(format_number(0.5, "#.##"), ".5");
		assert_eq!(format_number(1.0, "#.##"), "1");
		assert_eq!(format_number(12345.6, "#,##0"), "12,346");
		assert_eq!(format_number(1234567.0, "#,#"), "1,234,567");
		assert_eq!(format_number(1234567.0, "#,##0,,"), "1");
		assert_eq!(format_number(1.5, "0.0 'bar'"), "1.5 bar");
		assert_eq!(format_number(-3.0, "0;(0)"), "(3)");
		assert_eq!(format_number(-3.0, "0.0"), "-3.0");
		assert_eq!(format_number(0.42, "0%"), "42%");
		assert_eq!(format_number(0.0042, "0.0\u{2030}"), "4.2\u{2030}");
	}

	#[test]
	fn custom_sections() {
		assert_eq!(format_number(0.0, "+0;-0;zero"), "zero");
		assert_eq!(format_number(5.0, "+0;-0;zero"), "+5");
		assert_eq!(format_number(-5.0, "+0;-0;zero"), "-5");
		assert_eq!(format_number(0.001, "0.0;-0.0;zero"), "zero", "rounds to zero, then uses the zero section");
		assert_eq!(format_number(-5.0, "0;;zero"), "-5", "an empty negative section uses the first");
		assert_eq!(format_number(-0.001, "0.0"), "-0.0");
	}

	#[test]
	fn custom_exponents_and_literals() {
		assert_eq!(format_number(1234.5, "0.00E+00"), "1.23E+03");
		assert_eq!(format_number(0.00123, "0.0e0"), "1.2e-3");
		assert_eq!(format_number(1234.5, "0.0E-0"), "1.2E3");
		assert_eq!(format_number(0.0, "0.0E+0"), "0.0E+0");
		assert_eq!(format_number(5.0, "0E"), "5E", "no digit after E: literal");
		assert_eq!(format_number(1234.0, "00-00"), "12-34");
		assert_eq!(format_number(1234.0, "0 0"), "123 4");
		assert_eq!(format_number(7.0, "#'#'0"), "#7");
	}

	#[test]
	fn datetime_formats() {
		let dt = NaiveDate::from_ymd_opt(2026, 3, 7).unwrap().and_hms_milli_opt(14, 5, 9, 250).unwrap();
		let local = DateTimeKind::Local { offset_secs: 3600 };
		let f = |format: &str| format_datetime(&dt, local, format);
		assert_eq!(f("HH:mm:ss").unwrap(), "14:05:09");
		assert_eq!(f("h:mm tt").unwrap(), "2:05 PM");
		assert_eq!(f("ddd dd MMM yyyy").unwrap(), "Sat 07 Mar 2026");
		assert_eq!(f("HH:mm:ss.fff").unwrap(), "14:05:09.250");
		assert_eq!(f("T").unwrap(), "14:05:09");
		assert_eq!(f("HH'h'mm").unwrap(), "14h05");
		assert!(f("Q").is_none());
	}

	#[test]
	fn datetime_round_trip_rfc_and_universal() {
		let dt = NaiveDate::from_ymd_opt(2026, 3, 7).unwrap().and_hms_milli_opt(14, 5, 9, 250).unwrap();
		let local = DateTimeKind::Local { offset_secs: 2 * 3600 };
		assert_eq!(format_datetime(&dt, local, "o").unwrap(), "2026-03-07T14:05:09.2500000+02:00");
		assert_eq!(format_datetime(&dt, DateTimeKind::Utc, "O").unwrap(), "2026-03-07T14:05:09.2500000Z");
		assert_eq!(format_datetime(&dt, local, "r").unwrap(), "Sat, 07 Mar 2026 14:05:09 GMT");
		assert_eq!(format_datetime(&dt, local, "U").unwrap(), "Saturday, 07 March 2026 12:05:09");
		assert_eq!(format_datetime(&dt, DateTimeKind::Utc, "U").unwrap(), "Saturday, 07 March 2026 14:05:09");
	}

	#[test]
	fn datetime_offsets() {
		let dt = NaiveDate::from_ymd_opt(2026, 3, 7).unwrap().and_hms_opt(14, 5, 9).unwrap();
		let west = DateTimeKind::Local { offset_secs: -(5 * 3600 + 30 * 60) };
		assert_eq!(format_datetime(&dt, west, "z|zz|zzz|K").unwrap(), "-5|-05|-05:30|-05:30");
		assert_eq!(format_datetime(&dt, DateTimeKind::Utc, "zzz K").unwrap(), "+00:00 Z");
		assert_eq!(format_datetime(&dt, DateTimeKind::Unspecified { offset_secs: 3600 }, "zz[K]").unwrap(), "+01[]");
	}

	#[test]
	fn empty_f_fraction_drops_the_point() {
		let whole = NaiveDate::from_ymd_opt(2026, 3, 7).unwrap().and_hms_opt(14, 5, 9).unwrap();
		let utc = DateTimeKind::Utc;
		assert_eq!(format_datetime(&whole, utc, "ss.FFF").unwrap(), "09");
		assert_eq!(format_datetime(&whole, utc, "ss.fff").unwrap(), "09.000");
		let part = whole + chrono::Duration::milliseconds(120);
		assert_eq!(format_datetime(&part, utc, "ss.FFF").unwrap(), "09.12");
	}
}
