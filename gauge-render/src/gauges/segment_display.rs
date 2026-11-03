//! Port of `SegmentDisplay`: a right-aligned DSEG14 readout over a dim
//! `~~~~` placeholder, with an outer glow on the lit digits.

use crate::canvas::{Blur, Canvas, Paint, Sprite};
use crate::color::Rgba;
use crate::format::format_number;
use crate::gauges::render_static;
use crate::text::{typeface, TextAlign};

const BLANK_DISPLAY: &str = "~~~~";

pub struct SegmentDisplay {
	width: u32,
	height: u32,
	text_x: f32,
	baseline_y: f32,
	inactive: Rgba,
	active: Rgba,
	blur: Blur,
	decimals: usize,
	font_size: f32,
	current_value: f32,
	formatted: String,
	placeholder: Option<Sprite>,
	/// Lit digits + glow, re-rendered only when the text or colour changes.
	digits: Option<(String, Rgba, Sprite)>,
}

impl SegmentDisplay {
	#[allow(clippy::too_many_arguments)]
	pub fn new(
		width: u32,
		height: u32,
		center_x: f32,
		center_y: f32,
		inactive: Rgba,
		active: Rgba,
		shadow_blur: f32,
		decimals: usize,
		font_size: f32,
	) -> Self {
		let tf = typeface("DSEG14 Classic");
		let text_width = tf.measure(BLANK_DISPLAY, font_size);
		let m = tf.metrics(font_size);
		Self {
			width,
			height,
			// Right-aligned so digits stay put as the value changes.
			text_x: center_x + text_width / 2.0,
			// Baseline that vertically centres the glyphs on center_y.
			baseline_y: center_y - (m.descent + m.ascent) / 2.0,
			inactive,
			active,
			blur: Blur::outer(shadow_blur),
			decimals,
			font_size,
			current_value: 0.0,
			formatted: format_display_value(0.0, decimals),
			placeholder: None,
			digits: None,
		}
	}

	pub fn set_colors(&mut self, inactive: Rgba, active: Rgba) {
		if inactive == self.inactive && active == self.active {
			return;
		}
		self.inactive = inactive;
		self.active = active;
		self.placeholder = None;
	}

	pub fn set_value(&mut self, v: f32) {
		if (v - self.current_value).abs() < 0.00001 {
			return;
		}
		self.current_value = v;
		self.formatted = format_display_value(v, self.decimals);
	}

	pub fn formatted(&self) -> &str {
		&self.formatted
	}

	pub fn draw(&mut self, canvas: &mut Canvas) {
		let tf = typeface("DSEG14 Classic");
		if self.placeholder.is_none() {
			let (x, y, size, color) = (self.text_x, self.baseline_y, self.font_size, self.inactive);
			self.placeholder = Some(render_static(self.width, self.height, |c| {
				c.draw_text(BLANK_DISPLAY, x, y, TextAlign::Right, tf, size, &Paint::fill(color));
			}));
		}
		if let Some(p) = &self.placeholder {
			canvas.draw_sprite(p);
		}
		let stale = !matches!(&self.digits, Some((t, c, _)) if *t == self.formatted && *c == self.active);
		if stale {
			let (text, color, blur) = (self.formatted.clone(), self.active, self.blur);
			let (x, y, size) = (self.text_x, self.baseline_y, self.font_size);
			// Only the text box plus the glow's reach needs rendering.
			let m = tf.metrics(size);
			let pad = (blur.sigma * 3.0).ceil() + 2.0;
			let w = tf.measure(&text, size);
			let (x0, y0) = ((x - w - pad).floor() as i32, (y + m.ascent - pad).floor() as i32);
			let (rw, rh) = ((w + 2.0 * pad).ceil() as u32 + 1, (m.descent - m.ascent + 2.0 * pad).ceil() as u32 + 1);
			let sprite = Sprite::render_in(x0, y0, rw, rh, |c| {
				c.draw_text_with_blur(&text, x, y, TextAlign::Right, tf, size, &Paint::fill(color), blur);
			});
			self.digits = Some((text, color, sprite));
		}
		if let Some((_, _, sprite)) = &self.digits {
			canvas.draw_sprite(sprite);
		}
	}
}

/// Formats with `decimals` places but keeps at most four digits in total, so
/// the value always fits the four-character display.
pub fn format_display_value(value: f32, decimals: usize) -> String {
	let s = format_number(value, &format!("F{decimals}"));
	let (int_part, dec_part) = match s.split_once('.') {
		Some((i, d)) => (i, d),
		None => (s.as_str(), ""),
	};
	if int_part.len() < 4 && !dec_part.is_empty() {
		let allowed = 4 - int_part.len();
		let dec = &dec_part[..allowed.min(dec_part.len())];
		format!("{int_part}.{dec}")
	} else {
		int_part.to_string()
	}
}

#[cfg(test)]
mod tests {
	use super::format_display_value;

	#[test]
	fn keeps_four_digits() {
		assert_eq!(format_display_value(14.734, 2), "14.73");
		assert_eq!(format_display_value(1234.5, 1), "1234");
		assert_eq!(format_display_value(1.23456, 3), "1.235");
		assert_eq!(format_display_value(9.87654, 6), "9.876");
		assert_eq!(format_display_value(42.0, 0), "42");
	}
}
