//! Port of `DigitalGauge`: a large framed segment readout.

use crate::canvas::Sprite;

use crate::canvas::{rect, Blur, Canvas, Paint};
use crate::format::format_number;
use crate::gauges::{draw_background, render_static, Gauge, GaugeBase, GaugeConfig, SegmentDisplay, SHADOW_BLUR};
use crate::text::{typeface, TextAlign};

const CENTER_X: f32 = 320.0;
const CENTER_Y: f32 = 220.0;

pub struct DigitalGauge {
	base: GaugeBase,
	value_display: SegmentDisplay,
	size: (u32, u32),
	static_layer: Option<Sprite>,
}

impl DigitalGauge {
	pub fn new(config: &GaugeConfig, width: u32, height: u32) -> Self {
		let base = GaugeBase::new(config);
		let (active, inactive) = base.colors();
		Self {
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
		let (w, h) = self.size;
		self.static_layer = Some(render_static(w, h, |c| {
			draw_background(c);
			let Some(border) = rect(80.0, CENTER_Y - 60.0, w as f32 - 80.0, CENTER_Y + 60.0) else {
				return;
			};
			let frame = Paint::stroke(active, 3.0);
			c.draw_round_rect(border, 8.0, 8.0, &frame.with_blur(Some(Blur::normal(SHADOW_BLUR))));
			c.draw_round_rect(border, 8.0, 8.0, &frame);

			// Corner accents.
			let accent = Paint::stroke(inactive, 2.0);
			let len = 20.0;
			let (l, t, r, b) =
				(border.left() - 10.0, border.top() - 10.0, border.right() + 10.0, border.bottom() + 10.0);
			for (x, y, dx, dy) in [(l, t, 1.0, 1.0), (r, t, -1.0, 1.0), (l, b, 1.0, -1.0), (r, b, -1.0, -1.0)] {
				c.draw_line(x, y, x + dx * len, y, &accent);
				c.draw_line(x, y, x, y + dy * len, &accent);
			}

			let race = typeface("Race Sport");
			let label = Paint::fill(active);
			let glow = Blur::normal(SHADOW_BLUR);
			c.draw_text_with_blur(&base.unit, CENTER_X, CENTER_Y + 110.0, TextAlign::Center, race, 30.0, &label, glow);
			c.draw_text_with_blur(&base.title, CENTER_X, CENTER_Y + 155.0, TextAlign::Center, race, 20.0, &label, glow);

			let range = Paint::fill(inactive);
			let min = format_number(base.min_value, "");
			let max = format_number(base.max_value, "");
			c.draw_text(&min, border.left() + 10.0, border.top() - 15.0, TextAlign::Left, race, 14.0, &range);
			c.draw_text(&max, border.right() - 10.0, border.top() - 15.0, TextAlign::Right, race, 14.0, &range);
		}));
		self.value_display.set_colors(inactive, active);
		self.base.static_cache_valid = true;
	}
}

impl Gauge for DigitalGauge {
	fn base(&self) -> &GaugeBase {
		&self.base
	}

	fn base_mut(&mut self) -> &mut GaugeBase {
		&mut self.base
	}

	fn draw(&mut self, canvas: &mut Canvas) {
		let (active, inactive) = self.base.colors();
		if !self.base.static_cache_valid || self.static_layer.is_none() {
			self.update_static();
		}
		if let Some(p) = &self.static_layer {
			canvas.draw_sprite(p);
		}
		self.value_display.set_value(self.base.value);
		self.value_display.set_colors(inactive, active);
		self.value_display.draw(canvas);
	}
}
