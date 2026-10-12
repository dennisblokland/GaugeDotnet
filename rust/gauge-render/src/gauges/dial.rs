//! Port of `DialGauge`: handheld-style boost display with a 270° dial on the
//! left and a stacked title, status text and a large italic readout on the right.

use tiny_skia::{PathBuilder, PremultipliedColorU8};

use crate::canvas::{rect, Canvas, Paint, Sprite};
use crate::color::Rgba;
use crate::gauges::{draw_background, render_static, rounded_label, Gauge, GaugeBase, GaugeConfig};
use crate::text::{typeface, TextAlign, Typeface};

const CENTER_X: f32 = 221.0;
const CENTER_Y: f32 = 243.0;
const BEZEL_RADIUS: f32 = 207.0;
const BEZEL_WIDTH: f32 = 17.0;
const BEZEL_FADE_START: f32 = 0.15;
const BEZEL_FADE_END: f32 = 0.3;
const START_ANGLE_DEG: f32 = 90.0;
const SWEEP_DEG: f32 = 270.0;
const MAJOR_COUNT: usize = 6;
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

pub struct DialGauge {
	base: GaugeBase,
	current_value: f32,
	smoothing: bool,
	decimals: usize,
	size: (u32, u32),
	static_layer: Option<Sprite>,
}

fn angle_for(fraction: f32) -> f32 {
	(START_ANGLE_DEG + fraction * SWEEP_DEG).to_radians()
}

fn smooth_step(t: f32) -> f32 {
	let t = t.clamp(0.0, 1.0);
	t * t * (3.0 - 2.0 * t)
}

/// Bezel opacity at a point: full along the scale, fading out towards both ends.
fn bezel_fade(x: f32, y: f32) -> f32 {
	let deg = (y - CENTER_Y).atan2(x - CENTER_X).to_degrees();
	let u = (deg - START_ANGLE_DEG).rem_euclid(360.0) / SWEEP_DEG;
	if u > 1.0 {
		return 0.0;
	}
	smooth_step(u / BEZEL_FADE_START) * smooth_step((1.0 - u) / BEZEL_FADE_END)
}

/// Grey ring with a lighter outer rim along the scale. It is drawn into its
/// own layer whose alpha is then scaled per pixel by [`bezel_fade`].
fn draw_bezel(target: &mut Canvas) {
	let (width, height) = (target.width(), target.height());
	let mut layer = Canvas::new(width, height);
	let arc = |c: &mut Canvas, r: f32, stroke: f32, color: Rgba| {
		if let Some(oval) = rect(CENTER_X - r, CENTER_Y - r, CENTER_X + r, CENTER_Y + r) {
			c.draw_arc(oval, START_ANGLE_DEG, SWEEP_DEG, &Paint::stroke(color, stroke));
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
/// `spacing` adds room between letters.
struct TextStyle {
	size: f32,
	bold: f32,
	scale_x: f32,
	skew: f32,
	spacing: f32,
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

	c.save();
	c.translate(x, y);
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
		Self {
			current_value: base.value,
			smoothing: config.smoothing,
			decimals: config.decimals.max(0) as usize,
			size: (width, height),
			static_layer: None,
			base,
		}
	}

	fn update_static(&mut self) {
		let base = self.base.clone();
		self.static_layer = Some(render_static(self.size.0, self.size.1, |c| {
			draw_background(c);
			let font = typeface("Aldrich");

			draw_bezel(c);

			// Major ticks and labels.
			let tick = Paint::stroke(TEXT_COLOR, 4.0);
			let label_style = TextStyle { size: 36.0, bold: 1.0, scale_x: 1.0, skew: 0.0, spacing: 0.0 };
			for i in 0..=MAJOR_COUNT {
				let fraction = i as f32 / MAJOR_COUNT as f32;
				let (sin, cos) = angle_for(fraction).sin_cos();
				c.draw_line(
					CENTER_X + cos * TICK_INNER,
					CENTER_Y + sin * TICK_INNER,
					CENTER_X + cos * TICK_OUTER,
					CENTER_Y + sin * TICK_OUTER,
					&tick,
				);
				let label = rounded_label(base.min_value + fraction * (base.max_value - base.min_value));
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
			let minor_count = MAJOR_COUNT * MINOR_PER_MAJOR;
			for i in (0..=minor_count).filter(|i| i % MINOR_PER_MAJOR != 0) {
				let (sin, cos) = angle_for(i as f32 / minor_count as f32).sin_cos();
				c.draw_circle(CENTER_X + cos * DOT_RADIUS, CENTER_Y + sin * DOT_RADIUS, 2.2, &Paint::fill(TEXT_COLOR));
			}

			// Title, one word per line, right aligned.
			let title_style = TextStyle { size: 50.0, bold: 1.4, scale_x: 1.05, skew: -0.2, spacing: 7.0 };
			for (i, word) in base.title.split_whitespace().take(2).enumerate() {
				draw_styled_text(
					c,
					word,
					RIGHT_EDGE,
					64.0 + i as f32 * 46.0,
					TextAlign::Right,
					font,
					&title_style,
					TEXT_COLOR,
				);
			}

			// Status block.
			let status_style = TextStyle { size: 26.0, bold: 0.0, scale_x: 1.0, skew: 0.0, spacing: 6.0 };
			draw_styled_text(c, "Datalog:", RIGHT_EDGE, 222.0, TextAlign::Right, font, &status_style, STATUS_COLOR);
			draw_styled_text(c, "Stop", RIGHT_EDGE - 22.0, 246.0, TextAlign::Right, font, &status_style, STATUS_COLOR);
			c.draw_circle(RIGHT_EDGE - 7.0, 238.0, 6.0, &Paint::fill(STATUS_DOT_COLOR));

			// Unit below the readout.
			let unit_style = TextStyle { size: 44.0, ..title_style };
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
		let style = TextStyle { size: 190.0, bold: 1.0, scale_x: 0.85, skew: -0.12, spacing: 2.0 };
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
	}
}
