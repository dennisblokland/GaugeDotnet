//! Frame-cost meter: times each element with the device renderer.

use std::time::Instant;

use gauge_render::custom::is_static;
use gauge_render::{Canvas, CustomGaugeDefinition, ElementRenderer, Values};

/// Device time ÷ desktop time for a full repaint, single-threaded: the
/// median over the built-in screens in `bench/RESULTS.md` (RG35XX
/// Cortex-A53 at 1.5 GHz against a desktop x86 core). A rough guide only.
pub const DEVICE_FACTOR: f64 = 3.7;

/// 60 fps frame budget, ms.
pub const FRAME_BUDGET_MS: f64 = 1000.0 / 60.0;

#[derive(Clone, Debug, Default)]
pub struct CostReport {
	/// (element id, ms, redrawn when its value changes).
	pub elements: Vec<(String, f64, bool)>,
	pub backdrop_ms: f64,
}

impl CostReport {
	/// Everything, as on a screen switch.
	pub fn full_ms(&self) -> f64 {
		self.backdrop_ms + self.elements.iter().map(|e| e.1).sum::<f64>()
	}

	/// Elements redrawn every frame while values move (static runs are cached as sprites).
	pub fn dynamic_ms(&self) -> f64 {
		self.elements.iter().filter(|e| e.2).map(|e| e.1).sum()
	}

	pub fn max_ms(&self) -> f64 {
		self.elements.iter().map(|e| e.1).fold(self.backdrop_ms, f64::max)
	}
}

const SAMPLES: usize = 5;

/// Times each visible element. Every sample runs on a fresh thread, so
/// glow memoisation (per thread) can't hide the cost of a changing value.
pub fn measure(def: &CustomGaugeDefinition, values: &Values) -> CostReport {
	let mut renderer = ElementRenderer::new();
	let resolved = renderer.prepare(def, values, None);
	let painter = renderer.painter(def);
	let (w, h) = (def.width.max(1) as u32, def.height.max(1) as u32);

	let time = |draw: &(dyn Fn(&mut Canvas) + Sync)| -> f64 {
		let mut samples: Vec<f64> = (0..SAMPLES)
			.map(|_| {
				std::thread::scope(|s| {
					s.spawn(|| {
						let mut canvas = Canvas::new(w, h);
						let start = Instant::now();
						draw(&mut canvas);
						start.elapsed().as_secs_f64() * 1000.0
					})
					.join()
					.unwrap_or(0.0)
				})
			})
			.collect();
		samples.sort_by(f64::total_cmp);
		samples[SAMPLES / 2]
	};

	let backdrop_ms = time(&|c: &mut Canvas| painter.draw_backdrop(c));
	let elements = def
		.elements
		.iter()
		.zip(&resolved)
		.filter_map(|(element, value)| {
			let v = (*value)?;
			let ms = time(&|c: &mut Canvas| painter.draw_element(c, element, v));
			Some((element.base().id.clone(), ms, !is_static(element)))
		})
		.collect();
	CostReport { elements, backdrop_ms }
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn measures_every_element() {
		let def = crate::templates::rpm_dial();
		let report = measure(&def, &Values::new());
		assert_eq!(report.elements.len(), def.elements.len());
		assert!(report.full_ms() > 0.0);
		assert!(report.dynamic_ms() <= report.full_ms());
	}
}
