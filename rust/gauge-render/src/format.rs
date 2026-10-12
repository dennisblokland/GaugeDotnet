//! .NET-compatible formatting so gauge JSON written for the C# app renders the
//! same text: `float.ToString(format)` (invariant culture) and
//! `DateTime.ToString(format)`.

use chrono::{Datelike, Timelike};

/// Decimal digits of a finite, non-negative number: `digits` with the decimal
/// point after `point` digits (`point` may be negative or beyond the digits).
struct Decimal {
	digits: Vec<u8>,
	point: i32,
}

impl Decimal {
	/// Uses the shortest round-trip representation of the `f32` (as .NET Core
	/// 3.0+ does) so `0.1f` is treated as 0.1, not 0.100000001.
	fn from_f32(v: f32) -> Self {
		let s = format!("{:e}", v.abs());
		let (mantissa, exp) = s.split_once('e').unwrap_or((&s, "0"));
		let exp: i32 = exp.parse().unwrap_or(0);
		let digits: Vec<u8> = mantissa.bytes().filter(u8::is_ascii_digit).map(|b| b - b'0').collect();
		Self { digits, point: exp + 1 }
	}

	/// Rounds half away from zero to `frac` fractional digits.
	fn round_to(&mut self, frac: i32) {
		let keep = self.point + frac;
		if keep < 0 {
			self.digits.clear();
			self.point = 0;
			return;
		}
		let keep = keep as usize;
		if keep >= self.digits.len() {
			return;
		}
		let round_up = self.digits[keep] >= 5;
		self.digits.truncate(keep);
		if round_up {
			let mut i = keep;
			loop {
				if i == 0 {
					self.digits.insert(0, 1);
					self.point += 1;
					break;
				}
				i -= 1;
				if self.digits[i] == 9 {
					self.digits[i] = 0;
				} else {
					self.digits[i] += 1;
					break;
				}
			}
		}
	}

	fn is_zero(&self) -> bool {
		self.digits.iter().all(|&d| d == 0)
	}

	fn digit_at(&self, pos: i32) -> u8 {
		// pos 0 is the first integer digit left of the point when point == 1.
		if pos < 0 || pos as usize >= self.digits.len() {
			0
		} else {
			self.digits[pos as usize]
		}
	}

	/// Integer digits without leading zeros ("" when the integer part is 0).
	fn int_digits(&self) -> String {
		(0..self.point.max(0))
			.map(|i| (b'0' + self.digit_at(i)) as char)
			.collect::<String>()
			.trim_start_matches('0')
			.to_string()
	}

	fn frac_digits(&self, count: usize) -> String {
		(0..count as i32).map(|i| (b'0' + self.digit_at(self.point + i)) as char).collect()
	}
}

fn group_thousands(int: &str) -> String {
	let mut out = String::with_capacity(int.len() + int.len() / 3);
	for (i, c) in int.chars().enumerate() {
		if i > 0 && (int.len() - i) % 3 == 0 {
			out.push(',');
		}
		out.push(c);
	}
	out
}

fn fixed(v: f32, decimals: usize, grouping: bool) -> String {
	let mut d = Decimal::from_f32(v);
	d.round_to(decimals as i32);
	let mut int = d.int_digits();
	if int.is_empty() {
		int.push('0');
	}
	if grouping {
		int = group_thousands(&int);
	}
	// .NET Core 3.0+ keeps the sign of negative values that round to zero ("-0").
	let mut s = if v.is_sign_negative() { format!("-{int}") } else { int };
	if decimals > 0 {
		s.push('.');
		s.push_str(&d.frac_digits(decimals));
	}
	s
}

fn exponential(v: f32, decimals: usize, upper: bool) -> String {
	if v == 0.0 {
		let e = if upper { 'E' } else { 'e' };
		let frac = if decimals > 0 { format!(".{}", "0".repeat(decimals)) } else { String::new() };
		return format!("0{frac}{e}+000");
	}
	let mut d = Decimal::from_f32(v);
	let exp = d.point - 1;
	// Round to decimals+1 significant digits.
	d.round_to(decimals as i32 - exp);
	let exp = d.point - 1;
	let mut s = String::new();
	if v < 0.0 {
		s.push('-');
	}
	s.push((b'0' + d.digit_at(0)) as char);
	if decimals > 0 {
		s.push('.');
		s.extend((1..=decimals as i32).map(|i| (b'0' + d.digit_at(i)) as char));
	}
	s.push(if upper { 'E' } else { 'e' });
	s.push(if exp < 0 { '-' } else { '+' });
	s.push_str(&format!("{:03}", exp.abs()));
	s
}

/// `float.ToString()` / `"G"` / `"R"`: shortest round-trip representation.
fn general(v: f32) -> String {
	if v == 0.0 {
		return if v.is_sign_negative() { "-0".into() } else { "0".into() };
	}
	let a = v.abs();
	if (1e-5..1e15).contains(&a) {
		format!("{v}")
	} else {
		let s = format!("{v:E}");
		let (m, e) = s.split_once('E').unwrap_or((&s, "0"));
		let e: i32 = e.parse().unwrap_or(0);
		format!("{m}E{}{:02}", if e < 0 { '-' } else { '+' }, e.abs())
	}
}

/// Formats like `float.ToString(format, CultureInfo.InvariantCulture)`.
///
/// Supports the standard `F`, `N`, `P`, `E`, `G`/`R` specifiers (with optional
/// precision) and custom patterns such as `0.0`, `#,##0`, `00.00 'bar'`.
pub fn format_number(v: f32, format: &str) -> String {
	if v.is_nan() {
		return "NaN".into();
	}
	if v.is_infinite() {
		return if v > 0.0 { "Infinity".into() } else { "-Infinity".into() };
	}
	let mut chars = format.chars();
	if let Some(spec) = chars.next() {
		let rest = chars.as_str();
		if spec.is_ascii_alphabetic() && rest.chars().all(|c| c.is_ascii_digit()) && rest.len() <= 3 {
			let precision: Option<usize> = rest.parse().ok();
			match spec {
				'F' | 'f' => return fixed(v, precision.unwrap_or(2), false),
				'N' | 'n' => return fixed(v, precision.unwrap_or(2), true),
				'P' | 'p' => return format!("{} %", fixed(v * 100.0, precision.unwrap_or(2), true)),
				'E' | 'e' => return exponential(v, precision.unwrap_or(6), spec == 'E'),
				'G' | 'g' | 'R' | 'r' => return general(v),
				// "D"/"X" throw for floats in .NET; render the rounded integer instead of crashing.
				'D' | 'd' | 'X' | 'x' => return fixed(v, 0, false),
				_ => {}
			}
		}
	}
	if format.is_empty() {
		return general(v);
	}
	custom(v, format)
}

/// Custom numeric format strings (`0`, `#`, `.`, `,`, `%`, quoted literals,
/// `\` escapes and `;` section separators).
fn custom(v: f32, format: &str) -> String {
	let sections = split_sections(format);
	let (section, explicit_negative) = if v < 0.0 && sections.len() > 1 && !sections[1].is_empty() {
		(&sections[1], true)
	} else {
		(&sections[0], false)
	};

	// Tokenise into literal text and the numeric placeholder block.
	let mut prefix = String::new();
	let mut suffix = String::new();
	let mut number = String::new();
	let mut percent = false;
	let mut in_number = false;
	let mut after_number = false;
	let mut it = section.chars().peekable();
	while let Some(c) = it.next() {
		let literal = match c {
			'0' | '#' | '.' | ',' if !after_number => {
				in_number = true;
				number.push(c);
				continue;
			}
			'\'' | '"' => {
				let mut s = String::new();
				for q in it.by_ref() {
					if q == c {
						break;
					}
					s.push(q);
				}
				s
			}
			'\\' => it.next().map(String::from).unwrap_or_default(),
			'%' => {
				percent = true;
				"%".to_string()
			}
			other => other.to_string(),
		};
		if in_number {
			after_number = true;
		}
		if after_number {
			suffix.push_str(&literal);
		} else {
			prefix.push_str(&literal);
		}
	}

	let mut value = v.abs();
	if percent {
		value *= 100.0;
	}
	let (int_pat, frac_pat) = number.split_once('.').unwrap_or((&number, ""));
	// Trailing commas before the point divide by 1000 each.
	let trimmed_int = int_pat.trim_end_matches(',');
	for _ in 0..(int_pat.len() - trimmed_int.len()) {
		value /= 1000.0;
	}
	let grouping = trimmed_int.contains(',');
	let min_int = trimmed_int.chars().filter(|&c| c == '0').count();
	let min_frac = frac_pat.chars().take_while(|&c| c == '0').count();
	let max_frac = frac_pat.chars().filter(|&c| c == '0' || c == '#').count();

	let mut d = Decimal::from_f32(value);
	d.round_to(max_frac as i32);
	let mut int = d.int_digits();
	while int.len() < min_int {
		int.insert(0, '0');
	}
	if grouping {
		int = group_thousands(&int);
	}
	let mut frac = d.frac_digits(max_frac);
	while frac.len() > min_frac && frac.ends_with('0') {
		frac.pop();
	}

	let mut out = String::new();
	if v < 0.0 && !explicit_negative && !d.is_zero() {
		out.push('-');
	}
	out.push_str(&prefix);
	if !number.is_empty() {
		out.push_str(&int);
		if !frac.is_empty() {
			out.push('.');
			out.push_str(&frac);
		}
	}
	out.push_str(&suffix);
	out
}

fn split_sections(format: &str) -> Vec<String> {
	let mut sections = vec![String::new()];
	let mut quote: Option<char> = None;
	let mut escaped = false;
	for c in format.chars() {
		if escaped {
			escaped = false;
		} else if let Some(q) = quote {
			if c == q {
				quote = None;
			}
		} else if c == '\\' {
			escaped = true;
		} else if c == '\'' || c == '"' {
			quote = Some(c);
		} else if c == ';' {
			sections.push(String::new());
			continue;
		}
		sections.last_mut().unwrap().push(c);
	}
	sections
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

/// Formats a date/time like `DateTime.ToString(format, InvariantCulture)`.
/// Returns `None` for formats .NET would reject (the clock then falls back to
/// `HH:mm:ss`, like the C# renderer).
pub fn format_datetime<T>(dt: &T, format: &str) -> Option<String>
where
	T: Datelike + Timelike,
{
	let expanded: &str = if format.chars().count() == 1 {
		match format {
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
				}
				out.push_str(&digits);
			}
			't' => {
				let ampm = if dt.hour() < 12 { "AM" } else { "PM" };
				out.push_str(if n == 1 { &ampm[..1] } else { ampm });
			}
			'g' => out.push_str("A.D."),
			'z' | 'K' => {} // offsets are not available on a naive time; render nothing.
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
		assert_eq!(format_number(1234.5, "F0"), "1235");
		assert_eq!(format_number(2.5, "F0"), "3");
		assert_eq!(format_number(0.125, "F2"), "0.13");
		assert_eq!(format_number(14.7, "F1"), "14.7");
		assert_eq!(format_number(0.1, "F3"), "0.100");
		assert_eq!(format_number(999.96, "F1"), "1000.0");
		assert_eq!(format_number(-0.2, "F0"), "-0");
		assert_eq!(format_number(1234567.0, "N0"), "1,234,567");
		assert_eq!(format_number(0.256, "P1"), "25.6 %");
		assert_eq!(format_number(1234.5, "E2"), "1.23E+003");
		assert_eq!(format_number(12.0, ""), "12");
		assert_eq!(format_number(12.5, "G"), "12.5");
	}

	#[test]
	fn custom_numeric_formats() {
		assert_eq!(format_number(1.23456, "0.00"), "1.23");
		assert_eq!(format_number(5.0, "00.0"), "05.0");
		assert_eq!(format_number(0.5, "#.##"), ".5");
		assert_eq!(format_number(12345.6, "#,##0"), "12,346");
		assert_eq!(format_number(1.5, "0.0 'bar'"), "1.5 bar");
		assert_eq!(format_number(-3.0, "0;(0)"), "(3)");
		assert_eq!(format_number(-3.0, "0.0"), "-3.0");
		assert_eq!(format_number(0.42, "0%"), "42%");
	}

	#[test]
	fn datetime_formats() {
		let dt = NaiveDate::from_ymd_opt(2026, 3, 7).unwrap().and_hms_milli_opt(14, 5, 9, 250).unwrap();
		assert_eq!(format_datetime(&dt, "HH:mm:ss").unwrap(), "14:05:09");
		assert_eq!(format_datetime(&dt, "h:mm tt").unwrap(), "2:05 PM");
		assert_eq!(format_datetime(&dt, "ddd dd MMM yyyy").unwrap(), "Sat 07 Mar 2026");
		assert_eq!(format_datetime(&dt, "HH:mm:ss.fff").unwrap(), "14:05:09.250");
		assert_eq!(format_datetime(&dt, "T").unwrap(), "14:05:09");
		assert_eq!(format_datetime(&dt, "HH'h'mm").unwrap(), "14h05");
		assert!(format_datetime(&dt, "Q").is_none());
	}
}
