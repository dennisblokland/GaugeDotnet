//! Calculated-channel expressions (`Rpm / 1000`, `OilTemp - Iat`, ...).
//!
//! Port of `ExpressionEvaluator`: `+ - * /`, unary sign, parentheses, numbers
//! and channel names. Unknown names and malformed input evaluate to 0, and
//! division by zero yields 0, exactly like the C# version.

use crate::Values;

pub fn evaluate(expression: &str, values: &Values) -> f32 {
	let mut p = Parser { src: expression.as_bytes(), pos: 0, values };
	p.parse_add()
}

/// Channel names referenced by an expression (used to know which inputs a
/// calculated channel needs).
pub fn identifiers(expression: &str) -> Vec<String> {
	let b = expression.as_bytes();
	let mut out = Vec::new();
	let mut i = 0;
	while i < b.len() {
		if b[i].is_ascii_alphabetic() || b[i] == b'_' {
			let start = i;
			while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == b'_') {
				i += 1;
			}
			out.push(expression[start..i].to_string());
		} else if b[i].is_ascii_digit() || b[i] == b'.' {
			while i < b.len() && (b[i].is_ascii_digit() || b[i] == b'.') {
				i += 1;
			}
		} else {
			i += 1;
		}
	}
	out
}

struct Parser<'a> {
	src: &'a [u8],
	pos: usize,
	values: &'a Values,
}

impl Parser<'_> {
	fn peek(&self) -> Option<u8> {
		self.src.get(self.pos).copied()
	}

	fn skip_ws(&mut self) {
		while self.peek().is_some_and(|c| c.is_ascii_whitespace()) {
			self.pos += 1;
		}
	}

	fn parse_add(&mut self) -> f32 {
		let mut left = self.parse_mul();
		loop {
			self.skip_ws();
			match self.peek() {
				Some(b'+') => {
					self.pos += 1;
					left += self.parse_mul();
				}
				Some(b'-') => {
					self.pos += 1;
					left -= self.parse_mul();
				}
				_ => return left,
			}
		}
	}

	fn parse_mul(&mut self) -> f32 {
		let mut left = self.parse_unary();
		loop {
			self.skip_ws();
			match self.peek() {
				Some(b'*') => {
					self.pos += 1;
					left *= self.parse_unary();
				}
				Some(b'/') => {
					self.pos += 1;
					let r = self.parse_unary();
					left = if r != 0.0 { left / r } else { 0.0 };
				}
				_ => return left,
			}
		}
	}

	fn parse_unary(&mut self) -> f32 {
		self.skip_ws();
		match self.peek() {
			Some(b'-') => {
				self.pos += 1;
				-self.parse_primary()
			}
			Some(b'+') => {
				self.pos += 1;
				self.parse_primary()
			}
			_ => self.parse_primary(),
		}
	}

	fn parse_primary(&mut self) -> f32 {
		self.skip_ws();
		let Some(c) = self.peek() else {
			return 0.0;
		};
		if c == b'(' {
			self.pos += 1;
			let v = self.parse_add();
			self.skip_ws();
			if self.peek() == Some(b')') {
				self.pos += 1;
			}
			return v;
		}
		let start = self.pos;
		if c.is_ascii_digit() || c == b'.' {
			while self.peek().is_some_and(|c| c.is_ascii_digit() || c == b'.') {
				self.pos += 1;
			}
			return std::str::from_utf8(&self.src[start..self.pos]).ok().and_then(|s| s.parse().ok()).unwrap_or(0.0);
		}
		if c.is_ascii_alphabetic() || c == b'_' {
			while self.peek().is_some_and(|c| c.is_ascii_alphanumeric() || c == b'_') {
				self.pos += 1;
			}
			let name = std::str::from_utf8(&self.src[start..self.pos]).unwrap_or("");
			return crate::lookup(self.values, name).unwrap_or(0.0);
		}
		self.pos += 1;
		0.0
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn vals(pairs: &[(&str, f32)]) -> Values {
		pairs.iter().map(|(k, v)| (k.to_string(), *v)).collect()
	}

	#[test]
	fn arithmetic_and_precedence() {
		let v = Values::new();
		assert_eq!(evaluate("1 + 2 * 3", &v), 7.0);
		assert_eq!(evaluate("(1 + 2) * 3", &v), 9.0);
		assert_eq!(evaluate("-4 + 10 / 4", &v), -1.5);
		assert_eq!(evaluate("5 / 0", &v), 0.0);
		assert_eq!(evaluate("", &v), 0.0);
	}

	#[test]
	fn channel_names_are_case_insensitive() {
		let v = vals(&[("Rpm", 6500.0), ("OilTemp", 100.0), ("Iat", 30.0)]);
		assert_eq!(evaluate("Rpm / 1000", &v), 6.5);
		assert_eq!(evaluate("oiltemp - IAT", &v), 70.0);
		assert_eq!(evaluate("Unknown + 1", &v), 1.0);
	}

	#[test]
	fn identifiers_are_extracted() {
		assert_eq!(identifiers("(Rpm_2 * 3.5) / Map"), vec!["Rpm_2", "Map"]);
	}
}
