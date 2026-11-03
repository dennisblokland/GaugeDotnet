//! Colour type and `SKColor.Parse`-compatible hex parsing.

/// Straight (non-premultiplied) 8-bit RGBA colour, like `SKColor`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Rgba {
	pub r: u8,
	pub g: u8,
	pub b: u8,
	pub a: u8,
}

impl Rgba {
	pub const BLACK: Rgba = Rgba::rgb(0, 0, 0);
	pub const WHITE: Rgba = Rgba::rgb(255, 255, 255);
	pub const TRANSPARENT: Rgba = Rgba::new(0, 0, 0, 0);
	/// `SKColors.DeepSkyBlue`
	pub const DEEP_SKY_BLUE: Rgba = Rgba::rgb(0x00, 0xBF, 0xFF);
	/// `SKColors.OrangeRed`
	pub const ORANGE_RED: Rgba = Rgba::rgb(0xFF, 0x45, 0x00);

	pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
		Self { r, g, b, a }
	}

	pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
		Self { r, g, b, a: 255 }
	}

	/// Same as `SKColor.WithAlpha`: replaces (does not multiply) the alpha.
	pub const fn with_alpha(self, a: u8) -> Self {
		Self { a, ..self }
	}

	/// Parses `#RGB`, `#ARGB`, `#RRGGBB` or `#AARRGGBB` (the `#` is optional),
	/// matching `SKColor.TryParse`.
	pub fn parse(hex: &str) -> Option<Self> {
		let s = hex.trim();
		let s = s.strip_prefix('#').unwrap_or(s);
		if !s.chars().all(|c| c.is_ascii_hexdigit()) {
			return None;
		}
		let nib = |i: usize| u8::from_str_radix(&s[i..i + 1], 16).ok().map(|v| v * 17);
		let byte = |i: usize| u8::from_str_radix(&s[i..i + 2], 16).ok();
		match s.len() {
			3 => Some(Self::rgb(nib(0)?, nib(1)?, nib(2)?)),
			4 => Some(Self::new(nib(1)?, nib(2)?, nib(3)?, nib(0)?)),
			6 => Some(Self::rgb(byte(0)?, byte(2)?, byte(4)?)),
			8 => Some(Self::new(byte(2)?, byte(4)?, byte(6)?, byte(0)?)),
			_ => None,
		}
	}

	/// Lenient parse used by the renderers: invalid strings render black
	/// instead of throwing like `SKColor.Parse` does.
	pub fn parse_or_black(hex: &str) -> Self {
		Self::parse(hex).unwrap_or(Self::BLACK)
	}

	pub fn to_skia(self) -> tiny_skia::Color {
		tiny_skia::Color::from_rgba8(self.r, self.g, self.b, self.a)
	}

	/// `#RRGGBB` (alpha dropped), as the built-in gauges format their colours.
	pub fn to_hex_rgb(self) -> String {
		format!("#{:02X}{:02X}{:02X}", self.r, self.g, self.b)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn parses_all_skcolor_forms() {
		assert_eq!(Rgba::parse("#00FFFF"), Some(Rgba::rgb(0, 255, 255)));
		assert_eq!(Rgba::parse("80FF0000"), Some(Rgba::new(255, 0, 0, 0x80)));
		assert_eq!(Rgba::parse("#f0a"), Some(Rgba::rgb(0xFF, 0x00, 0xAA)));
		assert_eq!(Rgba::parse("#8f0a"), Some(Rgba::new(0xFF, 0x00, 0xAA, 0x88)));
		assert_eq!(Rgba::parse("#12345"), None);
		assert_eq!(Rgba::parse("#GG0000"), None);
	}
}
