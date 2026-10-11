//! Port of `MinMaxGauge`: segmented bar plus persistent min/max markers.

use tiny_skia::{PathBuilder, Pixmap};

use crate::canvas::{rect, Blur, Canvas, Paint};
use crate::color::Rgba;
use crate::format::format_number;
use crate::gauges::{draw_background, render_static, Gauge, GaugeBase, GaugeConfig, SegmentDisplay, SHADOW_BLUR};
use crate::text::{typeface, TextAlign};

const BAR_X: f32 = 70.0;
const BAR_Y: f32 = 210.0;
const BAR_WIDTH: f32 = 500.0;
const BAR_HEIGHT: f32 = 60.0;

pub struct MinMaxGauge {
	base: GaugeBase,
	current_value: f32,
	min_tracked: f32,
	max_tracked: f32,
	peaks_initialized: bool,
	smoothing: bool,
	segment_count: usize,
	segment_width: f32,
	value_display: SegmentDisplay,
	size: (u32, u32),
	static_layer: Option<Pixmap>,
	active_layer: Option<(usize, Rgba, Pixmap)>,
}

impl MinMaxGauge {
	pub fn new(config: &GaugeConfig, width: u32, height: u32) -> Self {
		let base = GaugeBase::new(config);
		let (active, inactive) = base.colors();
		let segment_count = config.segment_count.max(1) as usize;
		Self {
			current_value: base.value,
			min_tracked: base.value,
			max_tracked: base.value,
			peaks_initialized: false,
			smoothing: config.smoothing,
			segment_count,
			segment_width: BAR_WIDTH / segment_count as f32,
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

	pub fn peaks(&self) -> Option<(f32, f32)> {
		self.peaks_initialized.then_some((self.min_tracked, self.max_tracked))
	}

	fn segment_rect(&self, i: usize) -> Option<tiny_skia::Rect> {
		let seg_x = BAR_X + i as f32 * self.segment_width;
		rect(seg_x + 2.0, BAR_Y, seg_x + self.segment_width - 2.0, BAR_Y + BAR_HEIGHT)
	}

	fn update_static(&mut self) {
		let (active, inactive) = self.base.colors();
		let blur = Blur::normal(SHADOW_BLUR);
		let segments: Vec<_> = (0..self.segment_count).filter_map(|i| self.segment_rect(i)).collect();
		let (unit, title) = (self.base.unit.clone(), self.base.title.clone());
		self.static_layer = Some(render_static(self.size.0, self.size.1, |c| {
			draw_background(c);
			for r in segments {
				c.draw_rect(r, &Paint::fill(inactive));
			}
			if let Some(border) = rect(BAR_X - 4.0, BAR_Y - 4.0, BAR_X + BAR_WIDTH + 4.0, BAR_Y + BAR_HEIGHT + 4.0) {
				c.draw_rect_with_blur(border, &Paint::stroke(active, 4.0), blur);
			}
			let font = typeface("Race Sport");
			let label = Paint::fill(active);
			let unit_y = BAR_Y + BAR_HEIGHT + 80.0;
			c.draw_text_with_blur(&unit, 320.0, unit_y, TextAlign::Center, font, 30.0, &label, blur);
			c.draw_text_with_blur(&title, 320.0, unit_y + 40.0, TextAlign::Center, font, 20.0, &label, blur);
		}));
		self.value_display.set_colors(inactive, active);
		self.active_layer = None;
		self.base.static_cache_valid = true;
	}
}

fn draw_marker(canvas: &mut Canvas, x: f32, y: f32, color: Rgba, point_down: bool) {
	let dir = if point_down { 1.0 } else { -1.0 };
	let mut pb = PathBuilder::new();
	pb.move_to(x, y + dir * 12.0);
	pb.line_to(x - 6.0, y);
	pb.line_to(x + 6.0, y);
	pb.close();
	if let Some(path) = pb.finish() {
		canvas.draw_path(&path, &Paint::fill(color).with_blur(Some(Blur::normal(SHADOW_BLUR))));
		canvas.draw_path(&path, &Paint::fill(color));
	}
}

impl Gauge for MinMaxGauge {
	fn base(&self) -> &GaugeBase {
		&self.base
	}

	fn base_mut(&mut self) -> &mut GaugeBase {
		&mut self.base
	}

	fn set_value(&mut self, v: f32) {
		self.base.value = v;
		if !self.peaks_initialized {
			self.min_tracked = v;
			self.max_tracked = v;
			self.peaks_initialized = true;
		} else {
			self.min_tracked = self.min_tracked.min(v);
			self.max_tracked = self.max_tracked.max(v);
		}
	}

	fn reset_saved_state(&mut self) {
		self.current_value = self.base.value;
		self.min_tracked = self.base.value;
		self.max_tracked = self.base.value;
		self.peaks_initialized = false;
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

		let active_segs = (self.base.fraction(self.current_value) * self.segment_count as f32).round() as usize;
		let stale = !matches!(&self.active_layer, Some((n, c, _)) if *n == active_segs && *c == active);
		if stale {
			let rects: Vec<_> = (0..active_segs).filter_map(|i| self.segment_rect(i)).collect();
			let layer = render_static(self.size.0, self.size.1, |c| {
				for r in rects {
					c.draw_rect_with_blur(r, &Paint::fill(active), Blur::normal(SHADOW_BLUR));
				}
			});
			self.active_layer = Some((active_segs, active, layer));
		}
		if let Some((_, _, p)) = &self.active_layer {
			canvas.draw_pixmap(p, 0, 0);
		}

		let range = self.base.max_value - self.base.min_value;
		if range > 0.0 && self.peaks_initialized {
			let font = typeface("Race Sport");
			let min_x = BAR_X + self.base.fraction(self.min_tracked) * BAR_WIDTH;
			draw_marker(canvas, min_x, BAR_Y + BAR_HEIGHT + 6.0, Rgba::DEEP_SKY_BLUE, true);
			let min_label = format!("MIN:{}", format_number(self.min_tracked, "F1"));
			canvas.draw_text(
				&min_label,
				min_x,
				BAR_Y + BAR_HEIGHT + 38.0,
				TextAlign::Center,
				font,
				12.0,
				&Paint::fill(Rgba::DEEP_SKY_BLUE),
			);

			let max_x = BAR_X + self.base.fraction(self.max_tracked) * BAR_WIDTH;
			draw_marker(canvas, max_x, BAR_Y - 6.0, Rgba::ORANGE_RED, false);
			let max_label = format!("MAX:{}", format_number(self.max_tracked, "F1"));
			canvas.draw_text(
				&max_label,
				max_x,
				BAR_Y - 18.0,
				TextAlign::Center,
				font,
				12.0,
				&Paint::fill(Rgba::ORANGE_RED),
			);
		}

		self.value_display.set_value(self.base.value);
		self.value_display.set_colors(inactive, active);
		self.value_display.draw(canvas);
	}
}
