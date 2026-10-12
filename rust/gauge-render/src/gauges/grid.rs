//! Port of `GridGauge`: a 2×2 grid of plain numeric readouts.

use crate::canvas::Sprite;

use crate::canvas::{Canvas, Paint};
use crate::color::Rgba;
use crate::format::format_number;
use crate::gauges::{draw_background, render_static, Gauge, GaugeBase, GaugeConfig, GridCellConfig};
use crate::text::{typeface, TextAlign};

const COLUMNS: usize = 2;
const ROWS: usize = 2;
const VALUE_COLOR: Rgba = Rgba::WHITE;
const LABEL_COLOR: Rgba = Rgba::rgb(120, 120, 120);
const LINE_COLOR: Rgba = Rgba::rgb(60, 60, 60);

pub struct GridGauge {
	base: GaugeBase,
	cells: Vec<GridCellConfig>,
	cell_values: Vec<f32>,
	size: (u32, u32),
	static_layer: Option<Sprite>,
}

impl GridGauge {
	pub fn new(config: &GaugeConfig, width: u32, height: u32) -> Self {
		Self {
			base: GaugeBase::new(config),
			cell_values: vec![0.0; config.cells.len()],
			cells: config.cells.clone(),
			size: (width, height),
			static_layer: None,
		}
	}

	pub fn cell_count(&self) -> usize {
		self.cells.len()
	}

	pub fn cell_config(&self, index: usize) -> &GridCellConfig {
		&self.cells[index]
	}

	pub fn set_cell_value(&mut self, index: usize, value: f32) {
		if let Some(v) = self.cell_values.get_mut(index) {
			*v = value;
		}
	}

	fn cell_size(&self) -> (f32, f32) {
		(self.size.0 as f32 / COLUMNS as f32, self.size.1 as f32 / ROWS as f32)
	}

	fn update_static(&mut self) {
		let (w, h) = (self.size.0 as f32, self.size.1 as f32);
		let (cell_w, cell_h) = self.cell_size();
		let cells = self.cells.clone();
		self.static_layer = Some(render_static(self.size.0, self.size.1, |c| {
			draw_background(c);
			let line = Paint::stroke(LINE_COLOR, 1.0);
			for col in 1..COLUMNS {
				let x = col as f32 * cell_w;
				c.draw_line(x, 0.0, x, h, &line);
			}
			for row in 1..ROWS {
				let y = row as f32 * cell_h;
				c.draw_line(0.0, y, w, y, &line);
			}
			let race = typeface("Race Sport");
			let label = Paint::fill(LABEL_COLOR);
			let (title_size, unit_size) = (cell_h * 0.16, cell_h * 0.14);
			for (i, cell) in cells.iter().enumerate().take(COLUMNS * ROWS) {
				let cx = (i % COLUMNS) as f32 * cell_w + cell_w / 2.0;
				let top = (i / COLUMNS) as f32 * cell_h;
				c.draw_text(&cell.title, cx, top + title_size + 4.0, TextAlign::Center, race, title_size, &label);
				c.draw_text(&cell.unit, cx, top + cell_h - 6.0, TextAlign::Center, race, unit_size, &label);
			}
		}));
		self.base.static_cache_valid = true;
	}
}

impl Gauge for GridGauge {
	fn base(&self) -> &GaugeBase {
		&self.base
	}

	fn base_mut(&mut self) -> &mut GaugeBase {
		&mut self.base
	}

	fn draw(&mut self, canvas: &mut Canvas) {
		if !self.base.static_cache_valid || self.static_layer.is_none() {
			self.update_static();
		}
		if let Some(p) = &self.static_layer {
			canvas.draw_sprite(p);
		}
		let (cell_w, cell_h) = self.cell_size();
		let value_size = cell_h * 0.35;
		let race = typeface("Race Sport");
		let paint = Paint::fill(VALUE_COLOR);
		for (i, cell) in self.cells.iter().enumerate().take(COLUMNS * ROWS) {
			let cx = (i % COLUMNS) as f32 * cell_w + cell_w / 2.0;
			let cy = (i / COLUMNS) as f32 * cell_h + cell_h / 2.0 + value_size * 0.15;
			let text = format_number(self.cell_values[i], &format!("F{}", cell.decimals.max(0)));
			canvas.draw_text(&text, cx, cy, TextAlign::Center, race, value_size, &paint);
		}
	}
}
