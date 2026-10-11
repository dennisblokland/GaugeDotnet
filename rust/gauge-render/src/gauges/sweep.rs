//! Port of `SweepGauge`: solid rounded arc with a leading dot.

use tiny_skia::Pixmap;

use crate::canvas::{rect, Blur, Canvas, LineCap, Paint};
use crate::color::Rgba;
use crate::gauges::{
	draw_background, render_static, rounded_label, Gauge, GaugeBase, GaugeConfig, SegmentDisplay, SHADOW_BLUR,
};
use crate::text::{typeface, TextAlign};

const CENTER_X: f32 = 320.0;
const CENTER_Y: f32 = 255.0;
const OUTER_RADIUS: f32 = 210.0;
const ARC_WIDTH: f32 = 35.0;
const START_ANGLE_DEG: f32 = 135.0;
const END_ANGLE_DEG: f32 = 405.0;
const SWEEP_RANGE_DEG: f32 = END_ANGLE_DEG - START_ANGLE_DEG;

pub struct SweepGauge {
	base: GaugeBase,
	current_value: f32,
	smoothing: bool,
	value_display: SegmentDisplay,
	size: (u32, u32),
	static_layer: Option<Pixmap>,
}

fn arc_rect() -> Option<tiny_skia::Rect> {
	rect(CENTER_X - OUTER_RADIUS, CENTER_Y - OUTER_RADIUS, CENTER_X + OUTER_RADIUS, CENTER_Y + OUTER_RADIUS)
}

impl SweepGauge {
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
				CENTER_Y,
				inactive,
				active,
				SHADOW_BLUR,
				config.decimals.max(0) as usize,
				70.0,
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
			if let Some(r) = arc_rect() {
				c.draw_arc(
					r,
					START_ANGLE_DEG,
					SWEEP_RANGE_DEG,
					&Paint::stroke(inactive, ARC_WIDTH).with_cap(LineCap::Round),
				);
			}

			// Tick dots.
			let bump = |v: u8| v.saturating_add(40);
			let dot = Paint::fill(Rgba::new(bump(active.r), bump(active.g), bump(active.b), 80));
			let start = START_ANGLE_DEG.to_radians();
			let range = SWEEP_RANGE_DEG.to_radians();
			let tick_r = OUTER_RADIUS + ARC_WIDTH / 2.0 + 8.0;
			for i in 0..=10 {
				let a = start + (i as f32 / 10.0) * range;
				c.draw_circle(CENTER_X + a.cos() * tick_r, CENTER_Y + a.sin() * tick_r, 3.0, &dot);
			}

			// Labels on every other tick.
			let race = typeface("Race Sport");
			let label_r = OUTER_RADIUS - ARC_WIDTH / 2.0 - 20.0;
			for i in (0..=10).step_by(2) {
				let v = base.min_value + (i as f32 / 10.0) * (base.max_value - base.min_value);
				let a = start + (i as f32 / 10.0) * range;
				let (lx, ly) = (CENTER_X + a.cos() * label_r, CENTER_Y + a.sin() * label_r + 5.0);
				c.draw_text(&rounded_label(v), lx, ly, TextAlign::Center, race, 14.0, &Paint::fill(active));
			}

			let dseg = typeface("DSEG14 Classic");
			let placeholder = "~".repeat(base.unit.chars().count());
			c.draw_text(&placeholder, CENTER_X, CENTER_Y + 80.0, TextAlign::Center, dseg, 22.0, &Paint::fill(inactive));
			c.draw_text_with_blur(
				&base.unit,
				CENTER_X,
				CENTER_Y + 80.0,
				TextAlign::Center,
				dseg,
				22.0,
				&Paint::fill(active),
				Blur::normal(SHADOW_BLUR),
			);

			let title_y = CENTER_Y + OUTER_RADIUS * 45f32.to_radians().sin() + 10.0;
			c.draw_text_with_blur(
				&base.title,
				CENTER_X,
				title_y,
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

impl Gauge for SweepGauge {
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
			canvas.draw_pixmap(p, 0, 0);
		}

		let pct = self.base.fraction(self.current_value);
		let active_sweep = pct * SWEEP_RANGE_DEG;
		if active_sweep > 0.5 {
			if let Some(r) = arc_rect() {
				let glow = Paint::stroke(active.with_alpha(60), ARC_WIDTH + 8.0)
					.with_cap(LineCap::Round)
					.with_blur(Some(Blur::normal(SHADOW_BLUR)));
				canvas.draw_arc(r, START_ANGLE_DEG, active_sweep, &glow);
				canvas.draw_arc(
					r,
					START_ANGLE_DEG,
					active_sweep,
					&Paint::stroke(active, ARC_WIDTH).with_cap(LineCap::Round),
				);
			}
			let tip = START_ANGLE_DEG.to_radians() + pct * SWEEP_RANGE_DEG.to_radians();
			let (tx, ty) = (CENTER_X + tip.cos() * OUTER_RADIUS, CENTER_Y + tip.sin() * OUTER_RADIUS);
			canvas.draw_circle(tx, ty, 6.0, &Paint::fill(Rgba::WHITE).with_blur(Some(Blur::normal(SHADOW_BLUR))));
			canvas.draw_circle(tx, ty, 4.0, &Paint::fill(Rgba::WHITE));
		}

		self.value_display.set_value(self.base.value);
		self.value_display.set_colors(inactive, active);
		self.value_display.draw(canvas);
	}
}
