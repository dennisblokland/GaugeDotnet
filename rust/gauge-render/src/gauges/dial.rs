//! Port of `DialGauge`: handheld-style boost display with a 270° dial on the
//! left and a stacked title, status text and a large italic readout on the right.

use tiny_skia::{PathBuilder, PremultipliedColorU8};

use crate::canvas::{rect, Canvas, Paint, Sprite};
use crate::color::Rgba;
use crate::gauges::{draw_background, render_static, Gauge, GaugeBase, GaugeConfig};
use crate::text::{typeface, TextAlign, Typeface};

const CENTER_X: f32 = 221.0;
const CENTER_Y: f32 = 243.0;
const BEZEL_RADIUS: f32 = 207.0;
const BEZEL_WIDTH: f32 = 17.0;
const BEZEL_OVERHANG_DEG: f32 = 18.0;
const BEZEL_FADE_DEG: f32 = 15.0;
const BEZEL_START_DEG: f32 = START_ANGLE_DEG - BEZEL_OVERHANG_DEG;
const BEZEL_SWEEP_DEG: f32 = SWEEP_DEG + 2.0 * BEZEL_OVERHANG_DEG;
const START_ANGLE_DEG: f32 = 90.0;
const SWEEP_DEG: f32 = 270.0;
const MINOR_PER_MAJOR: usize = 5;
const TICK_OUTER: f32 = 178.0;
const TICK_INNER: f32 = 157.0;
const DOT_RADIUS: f32 = 170.0;
const LABEL_RADIUS: f32 = 124.0;
const NEEDLE_LENGTH: f32 = 148.0;
const NEEDLE_HALF_WIDTH: f32 = 5.5;
const RIGHT_EDGE: f32 = 612.0;

const BEZEL_COLOR: Rgba = Rgba::rgb(0xA8, 0xA8, 0xB4);
const BEZEL_HIGHLIGHT: Rgba = Rgba::rgb(0xE8, 0xE8, 0xF0);
const NEEDLE_COLOR: Rgba = Rgba::rgb(0xF0, 0x28, 0x48);
const TEXT_COLOR: Rgba = Rgba::WHITE;
const STATUS_COLOR: Rgba = Rgba::rgb(0xD8, 0xDA, 0xEC);
const STATUS_DOT_COLOR: Rgba = Rgba::rgb(0x30, 0x60, 0xFF);

// Widest each text may be before it shrinks, so long values, titles and
// labels stay clear of the dial whatever the data source and range.
const VALUE_MAX_WIDTH: f32 = 180.0;
const STATUS_MAX_WIDTH: f32 = 175.0;
const UNIT_MAX_WIDTH: f32 = 220.0;
const LABEL_MAX_WIDTH: f32 = 56.0;
const TITLE_MAX_WIDTH: [f32; 2] = [270.0, 225.0];

const STATUS_STYLE: TextStyle =
	TextStyle { size: 26.0, bold: 0.0, scale_x: 1.0, skew: 0.0, spacing: 6.0, max_width: STATUS_MAX_WIDTH };

/// What the status block under the title shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StatusMode {
	Peak,
	Min,
	Text,
	None,
}

impl StatusMode {
	/// Case-insensitive; unknown values fall back to `Peak`, like `Enum.TryParse`.
	fn parse(s: &str) -> Self {
		[Self::Peak, Self::Min, Self::Text, Self::None]
			.into_iter()
			.find(|m| format!("{m:?}").eq_ignore_ascii_case(s))
			.unwrap_or(Self::Peak)
	}
}

pub struct DialGauge {
	base: GaugeBase,
	current_value: f32,
	smoothing: bool,
	decimals: usize,
	major_count: usize,
	status_mode: StatusMode,
	status_label: String,
	status_text: String,
	peak: f32,
	min: f32,
	size: (u32, u32),
	static_layer: Option<Sprite>,
}

fn angle_for(fraction: f32) -> f32 {
	(START_ANGLE_DEG + fraction * SWEEP_DEG).to_radians()
}

/// Number of major intervals (4 to 8, preferring 6) that puts every major
/// tick on a round value: a step of 1, 2, 2.5 or 5 times a power of ten.
fn major_intervals(min: f32, max: f32) -> usize {
	let range = max - min;
	if range.is_nan() || range <= 0.0 {
		return 6;
	}
	for n in [6, 5, 7, 4, 8] {
		let step = range / n as f32;
		let magnitude = 10f32.powf(step.log10().floor());
		let mantissa = step / magnitude;
		if [1.0, 2.0, 2.5, 5.0, 10.0].iter().any(|nice: &f32| (mantissa - nice).abs() < 1e-3) {
			return n;
		}
	}
	6
}

/// Tick label: up to two decimals, without trailing zeros or "-0".
fn format_label(v: f32) -> String {
	let rounded = (v * 100.0).round() / 100.0;
	crate::format::format_number(if rounded == 0.0 { 0.0 } else { rounded }, "0.##")
}

fn smooth_step(t: f32) -> f32 {
	let t = t.clamp(0.0, 1.0);
	t * t * (3.0 - 2.0 * t)
}

/// Bezel opacity at a point: full along the scale, fading out over
/// `BEZEL_FADE_DEG` at both ends of the bezel, which runs
/// `BEZEL_OVERHANG_DEG` past the end ticks.
fn bezel_fade(x: f32, y: f32) -> f32 {
	let deg = (y - CENTER_Y).atan2(x - CENTER_X).to_degrees();
	let d = (deg - BEZEL_START_DEG).rem_euclid(360.0);
	if d > BEZEL_SWEEP_DEG {
		return 0.0;
	}
	smooth_step(d / BEZEL_FADE_DEG) * smooth_step((BEZEL_SWEEP_DEG - d) / BEZEL_FADE_DEG)
}

/// Grey ring with a lighter outer rim along the scale and a little past it. It is drawn into its
/// own layer whose alpha is then scaled per pixel by [`bezel_fade`].
fn draw_bezel(target: &mut Canvas) {
	let (width, height) = (target.width(), target.height());
	let mut layer = Canvas::new(width, height);
	let arc = |c: &mut Canvas, r: f32, stroke: f32, color: Rgba| {
		if let Some(oval) = rect(CENTER_X - r, CENTER_Y - r, CENTER_X + r, CENTER_Y + r) {
			c.draw_arc(oval, BEZEL_START_DEG, BEZEL_SWEEP_DEG, &Paint::stroke(color, stroke));
		}
	};
	arc(&mut layer, BEZEL_RADIUS - BEZEL_WIDTH / 2.0, BEZEL_WIDTH, BEZEL_COLOR);
	arc(&mut layer, BEZEL_RADIUS - BEZEL_WIDTH * 0.2, BEZEL_WIDTH * 0.4, BEZEL_HIGHLIGHT);

	for (i, px) in layer.pixmap_mut().pixels_mut().iter_mut().enumerate() {
		if px.alpha() == 0 {
			continue;
		}
		let (x, y) = ((i as u32 % width) as f32 + 0.5, (i as u32 / width) as f32 + 0.5);
		let fade = bezel_fade(x, y);
		let scale = |v: u8| (v as f32 * fade + 0.5) as u8;
		*px = PremultipliedColorU8::from_rgba(scale(px.red()), scale(px.green()), scale(px.blue()), scale(px.alpha()))
			.unwrap_or(PremultipliedColorU8::TRANSPARENT);
	}
	target.draw_pixmap(layer.pixmap(), 0, 0);
}

/// Text style: `bold` is the outline width added around each glyph
/// (`SKPaintStyle.StrokeAndFill`), `scale_x` stretches, `skew` slants and
/// `spacing` adds room between letters. Text wider than `max_width` is
/// shrunk to fit around its anchor.
struct TextStyle {
	size: f32,
	bold: f32,
	scale_x: f32,
	skew: f32,
	spacing: f32,
	max_width: f32,
}

/// `DialGauge.DrawStyledText`; `y` is the baseline.
#[allow(clippy::too_many_arguments)]
fn draw_styled_text(
	c: &mut Canvas,
	text: &str,
	x: f32,
	y: f32,
	align: TextAlign,
	font: &Typeface,
	style: &TextStyle,
	color: Rgba,
) {
	let count = text.chars().count();
	if count == 0 {
		return;
	}
	let mut buf = [0u8; 4];
	let advance = |ch: char, buf: &mut [u8; 4]| font.measure(ch.encode_utf8(buf), style.size);
	let width = style.spacing * (count - 1) as f32 + text.chars().map(|ch| advance(ch, &mut buf)).sum::<f32>();
	let mut pen = match align {
		TextAlign::Left => 0.0,
		TextAlign::Center => -width / 2.0,
		TextAlign::Right => -width,
	};

	let fit = (style.max_width / (width * style.scale_x)).min(1.0);

	c.save();
	c.translate(x, y);
	c.scale(fit, fit);
	c.skew(style.skew, 0.0);
	c.scale(style.scale_x, 1.0);
	for ch in text.chars() {
		let s: &str = ch.encode_utf8(&mut buf);
		if let Some(path) = font.text_path(s, pen, 0.0, style.size, TextAlign::Left) {
			c.draw_path(&path, &Paint::fill(color));
			if style.bold > 0.0 {
				c.draw_path(&path, &Paint::stroke(color, style.bold));
			}
		}
		pen += font.measure(s, style.size) + style.spacing;
	}
	c.restore();
}

impl DialGauge {
	pub fn new(config: &GaugeConfig, width: u32, height: u32) -> Self {
		let base = GaugeBase::new(config);
		let status_mode = StatusMode::parse(&config.status_mode);
		let status_label = if config.status_label.is_empty() {
			match status_mode {
				StatusMode::Min => "Min:",
				StatusMode::Text => "Datalog:",
				_ => "Peak:",
			}
			.to_string()
		} else {
			config.status_label.clone()
		};
		Self {
			current_value: base.value,
			smoothing: config.smoothing,
			decimals: config.decimals.max(0) as usize,
			status_mode,
			status_label,
			status_text: config.status_text.clone(),
			major_count: major_intervals(base.min_value, base.max_value),
			// Peak and Min start from the first reading, not the initial value.
			peak: f32::NAN,
			min: f32::NAN,
			size: (width, height),
			static_layer: None,
			base,
		}
	}

	fn update_static(&mut self) {
		let base = self.base.clone();
		let (mode, label, text) = (self.status_mode, self.status_label.clone(), self.status_text.clone());
		let major_count = self.major_count;
		self.static_layer = Some(render_static(self.size.0, self.size.1, |c| {
			draw_background(c);
			let font = typeface("Aldrich");

			draw_bezel(c);

			// Major ticks and labels.
			let tick = Paint::stroke(TEXT_COLOR, 4.0);
			let label_style =
				TextStyle { size: 36.0, bold: 1.0, scale_x: 1.0, skew: 0.0, spacing: 0.0, max_width: LABEL_MAX_WIDTH };
			for i in 0..=major_count {
				let fraction = i as f32 / major_count as f32;
				let (sin, cos) = angle_for(fraction).sin_cos();
				c.draw_line(
					CENTER_X + cos * TICK_INNER,
					CENTER_Y + sin * TICK_INNER,
					CENTER_X + cos * TICK_OUTER,
					CENTER_Y + sin * TICK_OUTER,
					&tick,
				);
				let label = format_label(base.min_value + fraction * (base.max_value - base.min_value));
				draw_styled_text(
					c,
					&label,
					CENTER_X + cos * LABEL_RADIUS,
					CENTER_Y + sin * LABEL_RADIUS + 13.0,
					TextAlign::Center,
					font,
					&label_style,
					TEXT_COLOR,
				);
			}

			// Minor dots.
			let minor_count = major_count * MINOR_PER_MAJOR;
			for i in (0..=minor_count).filter(|i| i % MINOR_PER_MAJOR != 0) {
				let (sin, cos) = angle_for(i as f32 / minor_count as f32).sin_cos();
				c.draw_circle(CENTER_X + cos * DOT_RADIUS, CENTER_Y + sin * DOT_RADIUS, 2.2, &Paint::fill(TEXT_COLOR));
			}

			// Title, one word per line, right aligned.
			let title_style =
				TextStyle { size: 50.0, bold: 1.4, scale_x: 1.05, skew: -0.2, spacing: 7.0, max_width: f32::INFINITY };
			for (i, word) in base.title.split_whitespace().take(2).enumerate() {
				let style = TextStyle { max_width: TITLE_MAX_WIDTH[i], ..title_style };
				draw_styled_text(
					c,
					word,
					RIGHT_EDGE,
					64.0 + i as f32 * 46.0,
					TextAlign::Right,
					font,
					&style,
					TEXT_COLOR,
				);
			}

			// Status block: the label, plus the fixed text and its dot in Text mode.
			// Peak and Min values change, so `draw` adds them each frame.
			if mode != StatusMode::None {
				draw_styled_text(c, &label, RIGHT_EDGE, 222.0, TextAlign::Right, font, &STATUS_STYLE, STATUS_COLOR);
			}
			if mode == StatusMode::Text {
				let style = TextStyle { max_width: STATUS_MAX_WIDTH - 22.0, ..STATUS_STYLE };
				draw_styled_text(c, &text, RIGHT_EDGE - 22.0, 246.0, TextAlign::Right, font, &style, STATUS_COLOR);
				c.draw_circle(RIGHT_EDGE - 7.0, 238.0, 6.0, &Paint::fill(STATUS_DOT_COLOR));
			}

			// Unit below the readout.
			let unit_style = TextStyle { size: 44.0, max_width: UNIT_MAX_WIDTH, ..title_style };
			draw_styled_text(c, &base.unit, RIGHT_EDGE - 4.0, 452.0, TextAlign::Right, font, &unit_style, TEXT_COLOR);
		}));
		self.base.static_cache_valid = true;
	}
}

impl Gauge for DialGauge {
	fn base(&self) -> &GaugeBase {
		&self.base
	}

	fn base_mut(&mut self) -> &mut GaugeBase {
		&mut self.base
	}

	fn set_value(&mut self, v: f32) {
		self.base.value = v;
		// `f32::max`/`min` ignore the NaN that marks "no reading yet".
		self.peak = self.peak.max(v);
		self.min = self.min.min(v);
	}

	fn reset_saved_state(&mut self) {
		self.peak = self.base.value;
		self.min = self.base.value;
	}

	fn draw(&mut self, canvas: &mut Canvas) {
		if self.smoothing {
			self.current_value += (self.base.value - self.current_value) * 0.1;
		} else {
			self.current_value = self.base.value;
		}
		if !self.base.static_cache_valid || self.static_layer.is_none() {
			self.update_static();
		}
		if let Some(p) = &self.static_layer {
			canvas.draw_sprite(p);
		}

		// Needle: flat bar from the centre towards the ticks.
		let (sin, cos) = angle_for(self.base.fraction(self.current_value)).sin_cos();
		let (perp_x, perp_y) = (-sin * NEEDLE_HALF_WIDTH, cos * NEEDLE_HALF_WIDTH);
		let (tip_x, tip_y) = (CENTER_X + cos * NEEDLE_LENGTH, CENTER_Y + sin * NEEDLE_LENGTH);
		let mut pb = PathBuilder::new();
		pb.move_to(CENTER_X + perp_x, CENTER_Y + perp_y);
		pb.line_to(tip_x + perp_x, tip_y + perp_y);
		pb.line_to(tip_x - perp_x, tip_y - perp_y);
		pb.line_to(CENTER_X - perp_x, CENTER_Y - perp_y);
		pb.close();
		if let Some(path) = pb.finish() {
			canvas.draw_path(&path, &Paint::fill(NEEDLE_COLOR));
		}

		// Digital readout.
		let text = crate::format::format_number(self.base.value, &format!("F{}", self.decimals));
		let style =
			TextStyle { size: 190.0, bold: 1.0, scale_x: 0.85, skew: -0.12, spacing: 2.0, max_width: VALUE_MAX_WIDTH };
		draw_styled_text(
			canvas,
			&text,
			RIGHT_EDGE,
			408.0,
			TextAlign::Right,
			typeface("Saira Extra Condensed"),
			&style,
			TEXT_COLOR,
		);

		if matches!(self.status_mode, StatusMode::Peak | StatusMode::Min) {
			let held = if self.status_mode == StatusMode::Peak { self.peak } else { self.min };
			let held = if held.is_nan() { self.base.value } else { held };
			let text = crate::format::format_number(held, &format!("F{}", self.decimals));
			let font = typeface("Aldrich");
			draw_styled_text(canvas, &text, RIGHT_EDGE, 246.0, TextAlign::Right, font, &STATUS_STYLE, STATUS_COLOR);
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn status_mode_parses_case_insensitively_and_defaults_to_peak() {
		assert_eq!(StatusMode::parse("text"), StatusMode::Text);
		assert_eq!(StatusMode::parse("MIN"), StatusMode::Min);
		assert_eq!(StatusMode::parse("None"), StatusMode::None);
		assert_eq!(StatusMode::parse("bogus"), StatusMode::Peak);
	}

	#[test]
	fn peak_and_min_follow_values_until_reset() {
		let config = GaugeConfig { gauge_type: crate::gauges::GaugeType::Dial, ..GaugeConfig::default() };
		let mut gauge = DialGauge::new(&config, 640, 480);
		assert_eq!(gauge.status_label, "Peak:");
		for v in [3.0, 12.5, -4.0, 1.0] {
			gauge.set_value(v);
		}
		assert_eq!((gauge.peak, gauge.min), (12.5, -4.0));
		gauge.reset_saved_state();
		assert_eq!((gauge.peak, gauge.min), (1.0, 1.0));
	}

	#[test]
	fn major_ticks_land_on_round_values() {
		assert_eq!(major_intervals(-10.0, 20.0), 6); // step 5
		assert_eq!(major_intervals(0.0, 8000.0), 4); // step 2000
		assert_eq!(major_intervals(40.0, 140.0), 5); // step 20
		assert_eq!(major_intervals(10.0, 20.0), 5); // step 2
		assert_eq!(major_intervals(0.0, 7.0), 7); // step 1
		assert_eq!(major_intervals(0.0, 13.0), 6, "no round step: keep 6");
		assert_eq!(major_intervals(5.0, 5.0), 6, "empty range");
		assert_eq!(format_label(-0.001), "0");
		assert_eq!(format_label(2.5), "2.5");
		assert_eq!(format_label(8000.0), "8000");
	}

	#[test]
	fn min_starts_from_first_reading() {
		let config = GaugeConfig { status_mode: "Min".into(), ..GaugeConfig::default() };
		let mut gauge = DialGauge::new(&config, 640, 480);
		assert!(gauge.min.is_nan());
		gauge.set_value(14.7);
		gauge.set_value(11.2);
		assert_eq!(gauge.min, 11.2);
	}

	#[test]
	fn status_label_override_and_defaults() {
		let text = GaugeConfig { status_mode: "Text".into(), ..GaugeConfig::default() };
		assert_eq!(DialGauge::new(&text, 640, 480).status_label, "Datalog:");
		let custom = GaugeConfig { status_mode: "Min".into(), status_label: "Lowest".into(), ..GaugeConfig::default() };
		assert_eq!(DialGauge::new(&custom, 640, 480).status_label, "Lowest");
	}
}
