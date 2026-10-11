//! Port of `BarGauge`: horizontal segmented bar with a segment readout.

use crate::canvas::Sprite;

use crate::canvas::{rect, Blur, Canvas, Paint};
use crate::color::Rgba;
use crate::gauges::{draw_background, render_static, Gauge, GaugeBase, GaugeConfig, SegmentDisplay, SHADOW_BLUR};
use crate::text::{typeface, TextAlign};

const X: f32 = 70.0;
const Y: f32 = 210.0;
const WIDTH: f32 = 500.0;
const HEIGHT: f32 = 60.0;

const GLOW_PAD: i32 = (SHADOW_BLUR * 3.0) as i32 + 2;

pub struct BarGauge {
	base: GaugeBase,
	current_value: f32,
	smoothing: bool,
	segment_count: usize,
	segment_width: f32,
	value_display: SegmentDisplay,
	size: (u32, u32),
	static_layer: Option<Sprite>,
	/// Active segments are cached by (count, colour): their glow is the expensive part.
	active_layer: Option<(usize, Rgba, Sprite)>,
}

impl BarGauge {
	pub fn new(config: &GaugeConfig, width: u32, height: u32) -> Self {
		let base = GaugeBase::new(config);
		let (active, inactive) = base.colors();
		let segment_count = config.segment_count.max(1) as usize;
		Self {
			current_value: base.value,
			smoothing: config.smoothing,
			segment_count,
			segment_width: WIDTH / segment_count as f32,
			value_display: SegmentDisplay::new(
				width,
				height,
				320.0,
				120.0,
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

	fn segment_rect(&self, i: usize) -> Option<tiny_skia::Rect> {
		let seg_x = X + i as f32 * self.segment_width;
		rect(seg_x + 2.0, Y, seg_x + self.segment_width - 2.0, Y + HEIGHT)
	}

	fn update_static(&mut self) {
		let (active, inactive) = self.base.colors();
		let blur = Blur::normal(SHADOW_BLUR);
		let segments: Vec<_> = (0..self.segment_count).filter_map(|i| self.segment_rect(i)).collect();
		let (unit, title) = (self.base.unit.clone(), self.base.title.clone());
		self.static_layer = Some(render_static(self.size.0, self.size.1, |c| {
			draw_background(c);
			let seg_paint = Paint::fill(inactive);
			for r in segments {
				c.draw_rect(r, &seg_paint);
			}
			if let Some(border) = rect(X - 4.0, Y - 4.0, X + WIDTH + 4.0, Y + HEIGHT + 4.0) {
				c.draw_rect_with_blur(border, &Paint::stroke(active, 4.0), blur);
			}
			let font = typeface("Race Sport");
			let label = Paint::fill(active);
			let unit_y = Y + HEIGHT + 80.0;
			c.draw_text_with_blur(&unit, 320.0, unit_y, TextAlign::Center, font, 30.0, &label, blur);
			c.draw_text_with_blur(&title, 320.0, unit_y + 40.0, TextAlign::Center, font, 20.0, &label, blur);
		}));
		self.value_display.set_colors(inactive, active);
		self.active_layer = None;
		self.base.static_cache_valid = true;
	}
}

impl Gauge for BarGauge {
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

		let active_segs = (self.base.fraction(self.current_value) * self.segment_count as f32).round() as usize;
		let stale = !matches!(&self.active_layer, Some((n, c, _)) if *n == active_segs && *c == active);
		if stale {
			let rects: Vec<_> = (0..active_segs).filter_map(|i| self.segment_rect(i)).collect();
			let (rx, ry, rw, rh) = (
				X as i32 - GLOW_PAD,
				Y as i32 - GLOW_PAD,
				(WIDTH as i32 + 2 * GLOW_PAD) as u32,
				(HEIGHT as i32 + 2 * GLOW_PAD) as u32,
			);
			let layer = Sprite::render_in(rx, ry, rw, rh, |c| {
				let paint = Paint::fill(active);
				for r in rects {
					c.draw_rect_with_blur(r, &paint, Blur::normal(SHADOW_BLUR));
				}
			});
			self.active_layer = Some((active_segs, active, layer));
		}
		if let Some((_, _, p)) = &self.active_layer {
			canvas.draw_sprite(p);
		}

		self.value_display.set_value(self.base.value);
		self.value_display.set_colors(inactive, active);
		self.value_display.draw(canvas);
	}
}
