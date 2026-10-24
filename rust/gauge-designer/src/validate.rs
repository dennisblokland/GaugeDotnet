//! The validation panel: things that load fine but go wrong on the device.

use gauge_render::color::Rgba;
use gauge_render::format::{format_datetime, DateTimeKind};
use gauge_render::images::resolve;
use gauge_render::model::*;
use me_data::{DATA_SOURCE_NAMES, LEGACY_ALIASES};
use serde_json::Value;

use crate::fields::{fields_for, get_str, type_tag, Kind, CANVAS};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
	Error,
	Warning,
	Info,
}

#[derive(Clone, Debug)]
pub struct Issue {
	pub severity: Severity,
	/// Element the issue is about (`None` = canvas or calculated channels).
	pub element: Option<String>,
	pub message: String,
}

pub fn is_known_source(name: &str) -> bool {
	DATA_SOURCE_NAMES.iter().any(|n| n.eq_ignore_ascii_case(name))
		|| LEGACY_ALIASES.iter().any(|(a, _)| a.eq_ignore_ascii_case(name))
}

/// Checks a whole definition. Issues are sorted errors first.
pub fn validate(def: &CustomGaugeDefinition) -> Vec<Issue> {
	let mut out = Vec::new();
	let channels: Vec<&str> =
		def.calculated_channels.iter().map(|c| c.name.as_str()).filter(|n| !n.is_empty()).collect();
	let is_channel = |name: &str| channels.iter().any(|c| c.eq_ignore_ascii_case(name));
	let base_dir = def.base_directory.as_deref();
	let mut push = |severity, element: Option<&str>, message: String| {
		out.push(Issue { severity, element: element.map(str::to_string), message })
	};

	// Canvas.
	let canvas = serde_json::to_value(def).unwrap_or(Value::Null);
	for field in CANVAS {
		check_field(&canvas, field.key, field.label, field.kind, base_dir, &mut |s, m| {
			push(s, None, format!("Canvas: {m}"))
		});
	}

	// Elements.
	let mut seen_ids: Vec<&str> = Vec::new();
	for element in &def.elements {
		let b = element.base();
		let id = b.id.as_str();
		let who = crate::document::describe(element);
		if seen_ids.contains(&id) {
			push(
				Severity::Warning,
				Some(id),
				format!("{who}: duplicate Id '{id}' (graphs and peak markers share their history)"),
			);
		}
		seen_ids.push(id);

		let value = serde_json::to_value(element).unwrap_or(Value::Null);
		let tag = type_tag(&value).to_string();
		for field in fields_for(&tag) {
			check_field(&value, field.key, field.label, field.kind, base_dir, &mut |s, m| {
				push(s, Some(id), format!("{who}: {m}"))
			});
		}

		if let Some(source) = b.source() {
			if !is_known_source(source) && !is_channel(source) {
				push(
					Severity::Warning,
					Some(id),
					format!("{who}: unknown data source '{source}' (always 0 on the device)"),
				);
			}
			if b.max_value <= b.min_value {
				push(
					Severity::Warning,
					Some(id),
					format!("{who}: Max value is not above Min value, so the value never moves"),
				);
			}
		}
		if b.use_visibility {
			match b.visibility_source.as_deref().filter(|s| !s.is_empty()) {
				None => push(
					Severity::Info,
					Some(id),
					format!("{who}: visibility condition has no source, so it is ignored"),
				),
				Some(s) if !is_known_source(s) && !is_channel(s) => {
					push(Severity::Warning, Some(id), format!("{who}: unknown visibility source '{s}' (reads 0)"))
				}
				_ => {}
			}
			if b.visible_above > b.visible_below {
				push(
					Severity::Warning,
					Some(id),
					format!("{who}: 'visible above' is higher than 'visible below', so it never shows"),
				);
			}
		}
		let bounds = crate::geometry::bounds(element);
		let screen = crate::geometry::Bounds::new(0.0, 0.0, def.width as f32, def.height as f32);
		if !bounds.intersects(&screen) {
			push(Severity::Info, Some(id), format!("{who}: entirely off the canvas"));
		}
	}

	// Calculated channels.
	for (i, ch) in def.calculated_channels.iter().enumerate() {
		let label =
			if ch.name.is_empty() { format!("Calculated channel {}", i + 1) } else { format!("Channel '{}'", ch.name) };
		if ch.name.is_empty() {
			push(Severity::Warning, None, format!("{label}: has no name"));
		} else if def.calculated_channels[..i].iter().any(|c| c.name.eq_ignore_ascii_case(&ch.name)) {
			push(Severity::Warning, None, format!("{label}: defined twice"));
		} else if is_known_source(&ch.name) {
			push(Severity::Warning, None, format!("{label}: has the name of an ECU channel"));
		}
		if let Err(e) = check_expression(&ch.expression) {
			push(Severity::Error, None, format!("{label}: {e} (evaluates to 0)"));
		}
		for name in gauge_render::expression::identifiers(&ch.expression) {
			if name.eq_ignore_ascii_case(&ch.name) {
				push(Severity::Warning, None, format!("{label}: refers to itself"));
			} else if !is_known_source(&name) && !is_channel(&name) {
				push(Severity::Warning, None, format!("{label}: unknown name '{name}' (reads 0)"));
			}
		}
	}
	out.sort_by_key(|i| i.severity);
	out
}

fn check_field(
	obj: &Value,
	key: &str,
	label: &str,
	kind: Kind,
	base_dir: Option<&std::path::Path>,
	push: &mut dyn FnMut(Severity, String),
) {
	let text = get_str(obj, key);
	match kind {
		Kind::Color => {
			if Rgba::parse(text).is_none() {
				push(Severity::Error, format!("{label} '{text}' is not a colour (renders black)"));
			}
		}
		Kind::Image(_) if !text.is_empty() => {
			let path = resolve(text, base_dir);
			if !path.is_file() {
				let hint = if base_dir.is_none() && !std::path::Path::new(text).is_absolute() {
					" (save the gauge first to resolve relative paths)"
				} else {
					""
				};
				push(Severity::Error, format!("{label}: file not found: {}{hint}", path.display()));
			}
		}
		Kind::NumberFormat => {
			if let Some(problem) = check_number_format(text) {
				push(Severity::Warning, format!("{label} '{text}': {problem}"));
			}
		}
		Kind::DateFormat => {
			if let Some(problem) = check_date_format(text) {
				push(Severity::Warning, format!("{label} '{text}': {problem}"));
			}
		}
		_ => {}
	}
}

/// What the device's number formatter does differently from .NET for this
/// format (`None` = fine). See `gauge-render/src/format.rs`.
pub fn check_number_format(format: &str) -> Option<String> {
	let mut chars = format.chars();
	if let Some(spec) = chars.next() {
		let rest = chars.as_str();
		if spec.is_ascii_alphabetic() && rest.chars().all(|c| c.is_ascii_digit()) && rest.len() <= 3 {
			return match spec.to_ascii_uppercase() {
				'F' | 'N' | 'P' | 'E' | 'R' => None,
				'G' if rest.is_empty() => None,
				'G' => Some("the precision after G is ignored".into()),
				'D' | 'X' => Some(".NET rejects this for decimals; the device shows a rounded integer".into()),
				'C' => Some("currency format is not supported and prints literally".into()),
				_ => Some("not a .NET standard number format".into()),
			};
		}
	}
	// Custom format: scan outside quotes.
	let (mut quote, mut escaped, mut sections) = (None, false, 1);
	let chars: Vec<char> = format.chars().collect();
	for (i, &c) in chars.iter().enumerate() {
		if escaped {
			escaped = false;
			continue;
		}
		match (quote, c) {
			(Some(q), c) if c == q => quote = None,
			(Some(_), _) => {}
			(None, '\'' | '"') => quote = Some(c),
			(None, '\\') => escaped = true,
			(None, ';') => sections += 1,
			(None, '‰') => return Some("‰ is not supported".into()),
			(None, 'E' | 'e') if matches!(chars.get(i + 1), Some('+' | '-' | '0')) => {
				return Some("exponent placeholders (E+0) are not supported".into())
			}
			_ => {}
		}
	}
	(sections > 2).then(|| "the third (zero) section is not supported".into())
}

pub fn check_date_format(format: &str) -> Option<String> {
	let now = chrono::NaiveDate::from_ymd_opt(2024, 1, 2).and_then(|d| d.and_hms_opt(3, 4, 5))?;
	if format_datetime(&now, DateTimeKind::Utc, format).is_none() {
		return Some("not a valid date format; the clock falls back to HH:mm:ss".into());
	}
	// Single-letter standard formats are expanded, so only look at custom ones.
	if format.chars().count() > 1 {
		let mut quote = None;
		for c in format.chars() {
			match (quote, c) {
				(Some(q), c) if c == q => quote = None,
				(Some(_), _) => {}
				(None, '\'' | '"') => quote = Some(c),
				(None, 'z' | 'K') => return Some("time-zone specifiers (z, K) print nothing on the device".into()),
				_ => {}
			}
		}
	}
	None
}

/// Parses an expression with the grammar of `ExpressionEvaluator`
/// (`+ - * /`, unary sign, parentheses, numbers, names).
pub fn check_expression(expression: &str) -> Result<(), String> {
	#[derive(Clone, Copy, PartialEq)]
	enum Tok {
		Num,
		Name,
		Op(u8),
	}
	let b = expression.as_bytes();
	let mut toks = Vec::new();
	let mut i = 0;
	while i < b.len() {
		let c = b[i];
		if c.is_ascii_whitespace() {
			i += 1;
		} else if c.is_ascii_digit() || c == b'.' {
			let start = i;
			while i < b.len() && (b[i].is_ascii_digit() || b[i] == b'.') {
				i += 1;
			}
			if expression[start..i].parse::<f64>().is_err() {
				return Err(format!("bad number '{}'", &expression[start..i]));
			}
			toks.push(Tok::Num);
		} else if c.is_ascii_alphabetic() || c == b'_' {
			while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == b'_') {
				i += 1;
			}
			toks.push(Tok::Name);
		} else if b"+-*/()".contains(&c) {
			toks.push(Tok::Op(c));
			i += 1;
		} else {
			let ch = expression[i..].chars().next().unwrap_or('?');
			return Err(format!("unexpected '{ch}'"));
		}
	}
	if toks.is_empty() {
		return Err("empty expression".into());
	}

	struct P {
		toks: Vec<Tok>,
		pos: usize,
	}
	impl P {
		fn peek(&self) -> Option<Tok> {
			self.toks.get(self.pos).copied()
		}
		fn add(&mut self) -> Result<(), String> {
			self.mul()?;
			while matches!(self.peek(), Some(Tok::Op(b'+' | b'-'))) {
				self.pos += 1;
				self.mul()?;
			}
			Ok(())
		}
		fn mul(&mut self) -> Result<(), String> {
			self.unary()?;
			while matches!(self.peek(), Some(Tok::Op(b'*' | b'/'))) {
				self.pos += 1;
				self.unary()?;
			}
			Ok(())
		}
		fn unary(&mut self) -> Result<(), String> {
			match self.peek() {
				Some(Tok::Op(b'+' | b'-')) => {
					self.pos += 1;
					self.unary()
				}
				Some(Tok::Num | Tok::Name) => {
					self.pos += 1;
					Ok(())
				}
				Some(Tok::Op(b'(')) => {
					self.pos += 1;
					self.add()?;
					if self.peek() != Some(Tok::Op(b')')) {
						return Err("missing ')'".into());
					}
					self.pos += 1;
					Ok(())
				}
				Some(Tok::Op(op)) => Err(format!("unexpected '{}'", op as char)),
				None => Err("expression ends early".into()),
			}
		}
	}
	let mut p = P { toks, pos: 0 };
	p.add()?;
	match p.peek() {
		None => Ok(()),
		Some(Tok::Op(b')')) => Err("unmatched ')'".into()),
		Some(_) => Err("missing operator".into()),
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn expressions() {
		for ok in ["Rpm / 1000", "-(OilTemp - Iat) * 1.8 + 32", "2", "+-Map"] {
			assert!(check_expression(ok).is_ok(), "{ok}");
		}
		for bad in ["", "Rpm /", "(Rpm", "Rpm)", "Rpm Map", "Rpm % 2", "1..2"] {
			assert!(check_expression(bad).is_err(), "{bad}");
		}
	}

	#[test]
	fn number_formats() {
		for ok in ["F0", "F1", "N2", "P", "0.00", "#,##0", "00.0 'bar'", "", "G"] {
			assert_eq!(check_number_format(ok), None, "{ok}");
		}
		for bad in ["C2", "G3", "D", "0.0E+0", "0;-0;zero", "0‰", "Z"] {
			assert!(check_number_format(bad).is_some(), "{bad}");
		}
	}

	#[test]
	fn date_formats() {
		assert_eq!(check_date_format("HH:mm:ss"), None);
		assert_eq!(check_date_format("t"), None);
		assert!(check_date_format("Q").is_some());
		assert!(check_date_format("HH:mm zzz").is_some());
		assert_eq!(check_date_format("HH 'zone'"), None);
	}

	#[test]
	fn finds_the_usual_mistakes() {
		let mut def = CustomGaugeDefinition { background_color: "#12345".into(), ..Default::default() };
		let mut arc = ArcElement::default();
		arc.base.data_source = Some("Rmp".into());
		arc.color = "red".into();
		def.elements.push(Element::Arc(arc));
		let img = ImageElement { image_path: "/definitely/not/here.png".into(), ..Default::default() };
		def.elements.push(Element::Image(img));
		def.calculated_channels.push(CalculatedChannel { name: "RpmK".into(), expression: "Rpm /".into() });
		def.calculated_channels.push(CalculatedChannel { name: "X".into(), expression: "Foo * 2".into() });
		let messages: Vec<String> = validate(&def).into_iter().map(|i| i.message).collect();
		let has = |s: &str| messages.iter().any(|m| m.contains(s));
		assert!(has("Background colour '#12345'"), "{messages:?}");
		assert!(has("Colour 'red'"), "{messages:?}");
		assert!(has("unknown data source 'Rmp'"), "{messages:?}");
		assert!(has("file not found"), "{messages:?}");
		assert!(has("expression ends early"), "{messages:?}");
		assert!(has("unknown name 'Foo'"), "{messages:?}");
	}

	#[test]
	fn a_clean_template_has_no_errors_or_warnings() {
		let issues = validate(&crate::templates::rpm_dial());
		assert!(issues.iter().all(|i| i.severity == Severity::Info), "{issues:?}");
	}
}
