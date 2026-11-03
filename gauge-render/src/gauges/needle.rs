//! Port of `NeedleGauge`: classic dial with ticks and a glowing needle.

use tiny_skia::PathBuilder;

use crate::canvas::Sprite;

use crate::canvas::{rect, Blur, Canvas, Paint};
use crate::color::Rgba;
use crate::gauges::{
	draw_background, render_static, rounded_label, Gauge, GaugeBase, GaugeConfig, SegmentDisplay, SHADOW_BLUR,
};
use crate::text::{typeface, TextAlign};

const CENTER_X: f32 = 320.0;
const CENTER_Y: f32 = 260.0;
const RADIUS: f32 = 210.0;
const START_ANGLE_DEG: f32 = 135.0;
const END_ANGLE_DEG: f32 = 405.0;
const NEEDLE_LENGTH: f32 = 180.0;
const NEEDLE_TAIL: f32 = 25.0;
const NEEDLE_HALF_WIDTH: f32 = 5.0;

pub struct NeedleGauge {
	base: GaugeBase,
	current_value: f32,
	smoothing: bool,
	value_display: SegmentDisplay,
	size: (u32, u32),
	static_layer: Option<Sprite>,
}

impl NeedleGauge {
	pub fn new(config: &GaugeConfig, width: u32, height: u32) -> Self {
		let base = GaugeBase::new(config);
		let (active, inactive) = base.colors();
		Self {
			current_value: base.value,
			smoothing: config.smoothing,
			value_display: SegmentDisplay::new(
				width,
				height,
				CENTER_X,
				CENTER_Y + 80.0,
				inactive,
				active,
				SHADOW_BLUR,
				config.decimals.max(0) as usize,
				40.0,
			),
			size: (width, height),
			static_layer: None,
			base,
		}
	}

	fn update_static(&mut self) {
		let (active, inactive) = self.base.colors();
		let base = self.base.clone();
		self.static_layer = Some(render_static(self.size.0, self.size.1, |c| {
			draw_background(c);
			let blur = Some(Blur::normal(SHADOW_BLUR));
			let sweep = END_ANGLE_DEG - START_ANGLE_DEG;

			// Outer arc.
			if let Some(r) = rect(CENTER_X - RADIUS, CENTER_Y - RADIUS, CENTER_X + RADIUS, CENTER_Y + RADIUS) {
				c.draw_arc(r, START_ANGLE_DEG, sweep, &Paint::stroke(inactive, 6.0));
			}
			// Glowing inner arc.
			let inner_r = RADIUS - 15.0;
			if let Some(r) = rect(CENTER_X - inner_r, CENTER_Y - inner_r, CENTER_X + inner_r, CENTER_Y + inner_r) {
				let p = Paint::stroke(active, 2.0);
				c.draw_arc(r, START_ANGLE_DEG, sweep, &p.with_blur(blur));
				c.draw_arc(r, START_ANGLE_DEG, sweep, &p);
			}

			// Major ticks, labels and minor ticks.
			let race = typeface("Race Sport");
			let start = START_ANGLE_DEG.to_radians();
			let range = (END_ANGLE_DEG - START_ANGLE_DEG).to_radians();
			for i in 0..=10 {
				let angle = start + (i as f32 / 10.0) * range;
				let (sin, cos) = angle.sin_cos();
				let major = Paint::stroke(active, 2.0);
				let (ox, oy) = (CENTER_X + cos * RADIUS, CENTER_Y + sin * RADIUS);
				let (ix, iy) = (CENTER_X + cos * (RADIUS - 20.0), CENTER_Y + sin * (RADIUS - 20.0));
				c.draw_line(ox, oy, ix, iy, &major.with_blur(blur));
				c.draw_line(ox, oy, ix, iy, &major);

				let label = rounded_label(base.min_value + (i as f32 / 10.0) * (base.max_value - base.min_value));
				let label_r = RADIUS + 18.0;
				let (lx, ly) = (CENTER_X + cos * label_r, CENTER_Y + sin * label_r + 6.0);
				c.draw_text_with_blur(
					&label,
					lx,
					ly,
					TextAlign::Center,
					race,
					16.0,
					&Paint::fill(active),
					Blur::normal(SHADOW_BLUR),
				);

				if i < 10 {
					let minor = Paint::stroke(active, 1.0);
					for j in 1..5 {
						let a = start + ((i as f32 * 5.0 + j as f32) / 50.0) * range;
						let (ms, mc) = a.sin_cos();
						c.draw_line(
							CENTER_X + mc * RADIUS,
							CENTER_Y + ms * RADIUS,
							CENTER_X + mc * (RADIUS - 10.0),
							CENTER_Y + ms * (RADIUS - 10.0),
							&minor,
						);
					}
				}
			}

			// Unit (segment font) and title.
			let dseg = typeface("DSEG14 Classic");
			let placeholder = "~".repeat(base.unit.chars().count());
			c.draw_text(
				&placeholder,
				CENTER_X,
				CENTER_Y + 130.0,
				TextAlign::Center,
				dseg,
				22.0,
				&Paint::fill(inactive),
			);
			c.draw_text_with_blur(
				&base.unit,
				CENTER_X,
				CENTER_Y + 130.0,
				TextAlign::Center,
				dseg,
				22.0,
				&Paint::fill(active),
				Blur::normal(SHADOW_BLUR),
			);
			c.draw_text_with_blur(
				&base.title,
				CENTER_X,
				CENTER_Y + 155.0,
				TextAlign::Center,
				race,
				14.0,
				&Paint::fill(active),
				Blur::normal(SHADOW_BLUR),
			);
		}));
		self.value_display.set_colors(inactive, active);
		self.base.static_cache_valid = true;
	}
}

impl Gauge for NeedleGauge {
	fn base(&self) -> &GaugeBase {
		&self.base
	}

	fn base_mut(&mut self) -> &mut GaugeBase {
		&mut self.base
	}

	fn draw(&mut self, canvas: &mut Canvas) {
		let (active, inactive) = self.base.colors();
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

		let pct = self.base.fraction(self.current_value);
		let angle = START_ANGLE_DEG.to_radians() + pct * (END_ANGLE_DEG - START_ANGLE_DEG).to_radians();
		let (sin, cos) = angle.sin_cos();
		let (tip_x, tip_y) = (CENTER_X + cos * NEEDLE_LENGTH, CENTER_Y + sin * NEEDLE_LENGTH);
		let (tail_x, tail_y) = (CENTER_X - cos * NEEDLE_TAIL, CENTER_Y - sin * NEEDLE_TAIL);
		let (perp_x, perp_y) = (-sin * NEEDLE_HALF_WIDTH, cos * NEEDLE_HALF_WIDTH);

		let mut pb = PathBuilder::new();
		pb.move_to(tip_x, tip_y);
		pb.line_to(CENTER_X + perp_x, CENTER_Y + perp_y);
		pb.line_to(tail_x, tail_y);
		pb.line_to(CENTER_X - perp_x, CENTER_Y - perp_y);
		pb.close();
		if let Some(path) = pb.finish() {
			canvas.draw_path(&path, &Paint::fill(active).with_blur(Some(Blur::normal(SHADOW_BLUR))));
			canvas.draw_path(&path, &Paint::fill(active));
		}
		// The C# hub paint is never given a colour, so it is SKPaint's default black.
		canvas.draw_circle(CENTER_X, CENTER_Y, 6.0, &Paint::fill(Rgba::BLACK));

		self.value_display.set_value(self.base.value);
		self.value_display.set_colors(inactive, active);
		self.value_display.draw(canvas);
	}
}
