//! Port of `HistogramGauge`: bar history sampled every `IntervalMs`.

use std::time::{Duration, Instant};

use crate::canvas::Sprite;

use crate::canvas::{rect, Blur, Canvas, Paint};
use crate::color::Rgba;
use crate::gauges::{draw_background, render_static, Gauge, GaugeBase, GaugeConfig, SegmentDisplay, SHADOW_BLUR};
use crate::text::{typeface, TextAlign};

const CHART_X: f32 = 70.0;
const CHART_Y: f32 = 150.0;
const CHART_WIDTH: f32 = 500.0;
const CHART_HEIGHT: f32 = 200.0;

pub struct HistogramGauge {
	base: GaugeBase,
	interval: Duration,
	/// Ring buffer of samples; `head` is the next write slot.
	data: Vec<f32>,
	count: usize,
	head: usize,
	last_update: Instant,
	blur: Blur,
	value_display: SegmentDisplay,
	size: (u32, u32),
	static_layer: Option<Sprite>,
	/// Bumped whenever the samples change; the bar layer is rebuilt only then.
	version: u64,
	bars_layer: Option<(u64, Rgba, Sprite)>,
}

impl HistogramGauge {
	pub fn new(config: &GaugeConfig, width: u32, height: u32) -> Self {
		let base = GaugeBase::new(config);
		let (active, inactive) = base.colors();
		let max_points = config.max_data_points.max(1) as usize;
		Self {
			interval: Duration::from_millis(config.interval_ms.max(0) as u64),
			data: vec![base.value; max_points],
			count: max_points,
			head: 0,
			last_update: Instant::now(),
			blur: Blur::normal(SHADOW_BLUR / 2.0),
			value_display: SegmentDisplay::new(
				width,
				height,
				320.0,
				100.0,
				inactive,
				active,
				SHADOW_BLUR,
				config.decimals.max(0) as usize,
				70.0,
			),
			size: (width, height),
			static_layer: None,
			version: 0,
			bars_layer: None,
			base,
		}
	}

	/// Samples oldest → newest.
	pub fn samples(&self) -> impl Iterator<Item = f32> + '_ {
		let n = self.data.len();
		(0..self.count).map(move |i| self.data[(self.head + n - self.count + i) % n])
	}

	fn update_static(&mut self) {
		let (active, inactive) = self.base.colors();
		let base = self.base.clone();
		let blur = self.blur;
		self.static_layer = Some(render_static(self.size.0, self.size.1, |c| {
			draw_background(c);
			if let Some(border) = rect(CHART_X, CHART_Y, CHART_X + CHART_WIDTH, CHART_Y + CHART_HEIGHT) {
				let p = Paint::stroke(active, 4.0);
				c.draw_rect(border, &p.with_blur(Some(blur)));
				c.draw_rect(border, &p);
			}
			let label = Paint::fill(active);
			c.draw_text_with_blur(
				&base.unit,
				320.0,
				CHART_Y + CHART_HEIGHT + 40.0,
				TextAlign::Center,
				typeface("Race Sport"),
				30.0,
				&label,
				blur,
			);
			c.draw_text_with_blur(
				&base.title,
				320.0,
				CHART_Y + CHART_HEIGHT + 80.0,
				TextAlign::Center,
				typeface("DSEG14 Classic"),
				20.0,
				&label,
				blur,
			);
		}));
		self.value_display.set_colors(inactive, active);
		self.base.static_cache_valid = true;
	}
}

impl Gauge for HistogramGauge {
	fn base(&self) -> &GaugeBase {
		&self.base
	}

	fn base_mut(&mut self) -> &mut GaugeBase {
		&mut self.base
	}

	fn set_value(&mut self, v: f32) {
		self.base.value = v;
		let now = Instant::now();
		if now.duration_since(self.last_update) >= self.interval {
			let n = self.data.len();
			self.data[self.head] = v;
			self.head = (self.head + 1) % n;
			self.count = (self.count + 1).min(n);
			self.last_update = now;
			self.version += 1;
		}
	}

	fn reset_saved_state(&mut self) {
		self.version += 1;
		self.data.fill(0.0);
		self.count = self.data.len();
		self.head = 0;
		self.last_update = Instant::now();
	}

	fn draw(&mut self, canvas: &mut Canvas) {
		let (active, inactive) = self.base.colors();
		if !self.base.static_cache_valid || self.static_layer.is_none() {
			self.update_static();
		}
		if let Some(p) = &self.static_layer {
			canvas.draw_sprite(p);
		}

		let stale = !matches!(&self.bars_layer, Some((v, c, _)) if *v == self.version && *c == active);
		if stale {
			let bar_w = CHART_WIDTH / self.data.len() as f32;
			let range = self.base.max_value - self.base.min_value;
			let bars: Vec<(f32, f32, f32)> = self
				.samples()
				.enumerate()
				.map(|(i, v)| {
					let pct = if range > 0.0 { ((v - self.base.min_value) / range).clamp(0.0, 1.0) } else { 0.0 };
					let h = pct * CHART_HEIGHT;
					(CHART_X + i as f32 * bar_w + 2.0, CHART_Y + CHART_HEIGHT - h, h)
				})
				.collect();
			let blur = self.blur;
			let pad = (blur.sigma * 3.0) as i32 + 2;
			let layer = Sprite::render_in(
				CHART_X as i32 - pad,
				CHART_Y as i32 - pad,
				(CHART_WIDTH as i32 + 2 * pad) as u32,
				(CHART_HEIGHT as i32 + 2 * pad) as u32,
				|c| {
					// Glow pass for all bars, then a crisp pass.
					for paint in [Paint::fill(active).with_blur(Some(blur)), Paint::fill(active)] {
						for &(x, y, h) in &bars {
							if h > 0.0 {
								c.draw_rect_xywh(x, y, bar_w - 4.0, h, &paint);
							}
						}
					}
				},
			);
			self.bars_layer = Some((self.version, active, layer));
		}
		if let Some((_, _, layer)) = &self.bars_layer {
			canvas.draw_sprite(layer);
		}

		self.value_display.set_value(self.base.value);
		self.value_display.set_colors(inactive, active);
		self.value_display.draw(canvas);
	}
}
