//! Port of `CircularGauge`: segmented 300° ring with tick labels.

use crate::canvas::Sprite;

use crate::canvas::{rect, Blur, Canvas, Paint};
use crate::color::Rgba;
use crate::gauges::{
	draw_background, render_static, rounded_label, Gauge, GaugeBase, GaugeConfig, SegmentDisplay, SHADOW_BLUR,
};
use crate::text::{typeface, TextAlign};

const CENTER_X: f32 = 320.0;
const CENTER_Y: f32 = 240.0;
const LINE_WIDTH: f32 = 40.0;
const RADIUS: f32 = 240.0 - LINE_WIDTH / 2.0 - 10.0;
const START_ANGLE_DEG: f32 = 120.0;
const END_ANGLE_DEG: f32 = 420.0;
/// Gap between segments, in radians.
const SEGMENT_MARGIN: f32 = 0.01;

const GLOW_PAD: i32 = (SHADOW_BLUR * 3.0) as i32 + 2;

pub struct CircularGauge {
	base: GaugeBase,
	current_value: f32,
	smoothing: bool,
	/// Per-segment (start, sweep) in degrees.
	segments: Vec<(f32, f32)>,
	value_display: SegmentDisplay,
	size: (u32, u32),
	static_layer: Option<Sprite>,
	active_layer: Option<(usize, Rgba, Sprite)>,
}

impl CircularGauge {
	pub fn new(config: &GaugeConfig, width: u32, height: u32) -> Self {
		let base = GaugeBase::new(config);
		let (active, inactive) = base.colors();
		let count = config.segment_count.max(1) as usize;
		let start = START_ANGLE_DEG.to_radians();
		let range = (END_ANGLE_DEG - START_ANGLE_DEG).to_radians();
		let segments = (0..count)
			.map(|i| {
				let s = start + (i as f32 / count as f32) * range + SEGMENT_MARGIN / 2.0;
				let e = start + ((i + 1) as f32 / count as f32) * range - SEGMENT_MARGIN / 2.0;
				(s.to_degrees(), (e - s).to_degrees())
			})
			.collect();
		Self {
			current_value: base.value,
			smoothing: config.smoothing,
			segments,
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
			active_layer: None,
			base,
		}
	}

	fn update_static(&mut self) {
		let (active, inactive) = self.base.colors();
		let blur = Some(Blur::normal(SHADOW_BLUR));
		let segments = self.segments.clone();
		let base = self.base.clone();
		self.static_layer = Some(render_static(self.size.0, self.size.1, |c| {
			draw_background(c);
			let Some(arc_rect) = rect(CENTER_X - RADIUS, CENTER_Y - RADIUS, CENTER_X + RADIUS, CENTER_Y + RADIUS)
			else {
				return;
			};

			// 1. Inactive segmented ring.
			let seg = Paint::stroke(inactive, LINE_WIDTH);
			for (start, sweep) in &segments {
				c.draw_arc(arc_rect, *start, *sweep, &seg);
			}

			// 2. Thin inner arc with glow.
			let inner_r = RADIUS - LINE_WIDTH / 2.0 - 5.0;
			if let Some(inner) = rect(CENTER_X - inner_r, CENTER_Y - inner_r, CENTER_X + inner_r, CENTER_Y + inner_r) {
				let p = Paint::stroke(active, 2.0);
				let sweep = END_ANGLE_DEG - START_ANGLE_DEG;
				c.draw_arc(inner, START_ANGLE_DEG, sweep, &p.with_blur(blur));
				c.draw_arc(inner, START_ANGLE_DEG, sweep, &p);
			}

			// 3. Tick labels (glow pass, then crisp pass).
			let race = typeface("Race Sport");
			let step = (base.max_value - base.min_value) / 10.0;
			let start = START_ANGLE_DEG.to_radians();
			let range = (END_ANGLE_DEG - START_ANGLE_DEG).to_radians();
			for pass_blur in [blur, None] {
				let p = Paint::fill(active).with_blur(pass_blur);
				for i in 0..=10 {
					let text = rounded_label(base.min_value + i as f32 * step);
					let angle = start + (i as f32 / 10.0) * range;
					let w = race.measure(&text, 14.0);
					let x = CENTER_X + angle.cos() * (RADIUS - (LINE_WIDTH + w / 4.0));
					let y = CENTER_Y + angle.sin() * (RADIUS - (LINE_WIDTH + 18.0 / 4.0));
					c.draw_text(&text, x, y + 6.0, TextAlign::Center, race, 14.0, &p);
				}
			}

			// 4. Unit: dim placeholder, then glowing text.
			let dseg = typeface("DSEG14 Classic");
			let unit_y = CENTER_Y + RADIUS - 50.0;
			let placeholder = "~".repeat(base.unit.chars().count());
			c.draw_text(&placeholder, CENTER_X, unit_y, TextAlign::Center, dseg, 25.0, &Paint::fill(inactive));
			c.draw_text_with_blur(
				&base.unit,
				CENTER_X,
				unit_y,
				TextAlign::Center,
				dseg,
				25.0,
				&Paint::fill(active),
				Blur::normal(SHADOW_BLUR),
			);

			// 5. Title.
			c.draw_text_with_blur(
				&base.title,
				CENTER_X,
				CENTER_Y + RADIUS - 15.0,
				TextAlign::Center,
				race,
				15.0,
				&Paint::fill(active),
				Blur::normal(SHADOW_BLUR),
			);
		}));
		self.value_display.set_colors(inactive, active);
		self.active_layer = None;
		self.base.static_cache_valid = true;
	}
}

impl Gauge for CircularGauge {
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

		let active_segments = (self.base.fraction(self.current_value) * self.segments.len() as f32).round() as usize;
		let stale = !matches!(&self.active_layer, Some((n, c, _)) if *n == active_segments && *c == active);
		if stale {
			let segs = self.segments[..active_segments.min(self.segments.len())].to_vec();
			let (rx, ry, rw, rh) = {
				let r = (RADIUS + LINE_WIDTH / 2.0) as i32 + GLOW_PAD;
				(CENTER_X as i32 - r, CENTER_Y as i32 - r, 2 * r as u32, 2 * r as u32)
			};
			let layer = Sprite::render_in(rx, ry, rw, rh, |c| {
				let Some(arc_rect) = rect(CENTER_X - RADIUS, CENTER_Y - RADIUS, CENTER_X + RADIUS, CENTER_Y + RADIUS)
				else {
					return;
				};
				let p = Paint::stroke(active, LINE_WIDTH);
				let glow = p.with_blur(Some(Blur::normal(SHADOW_BLUR)));
				for (start, sweep) in &segs {
					c.draw_arc(arc_rect, *start, *sweep, &glow);
				}
				for (start, sweep) in &segs {
					c.draw_arc(arc_rect, *start, *sweep, &p);
				}
			});
			self.active_layer = Some((active_segments, active, layer));
		}
		if let Some((_, _, p)) = &self.active_layer {
			canvas.draw_sprite(p);
		}

		self.value_display.set_value(self.base.value);
		self.value_display.set_colors(inactive, active);
		self.value_display.draw(canvas);
	}
}
