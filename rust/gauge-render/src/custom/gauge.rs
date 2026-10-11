//! Port of `CustomGauge`: tracks the data sources a definition uses and
//! smooths them before rendering.

use std::time::Instant;

use crate::canvas::Canvas;
use crate::custom::{Damage, ElementRenderer, FrameState};
use crate::model::CustomGaugeDefinition;
use crate::{expression, Values};

/// Fraction of the remaining distance covered per 60 Hz frame.
const SMOOTHING_FACTOR: f32 = 0.1;

pub struct CustomGauge {
	definition: CustomGaugeDefinition,
	values: Values,
	targets: Values,
	renderer: ElementRenderer,
	frame: FrameState,
	last_frame: Option<Instant>,
}

impl CustomGauge {
	pub fn new(definition: CustomGaugeDefinition) -> Self {
		let channel_names: Vec<String> =
			definition.calculated_channels.iter().map(|c| c.name.to_ascii_lowercase()).collect();
		let mut sources: Vec<String> = Vec::new();
		let mut add = |name: &str| {
			if !name.is_empty()
				&& !channel_names.contains(&name.to_ascii_lowercase())
				&& !sources.iter().any(|s| s.eq_ignore_ascii_case(name))
			{
				sources.push(name.to_string());
			}
		};
		for element in &definition.elements {
			let base = element.base();
			if let Some(s) = base.source() {
				add(s);
			}
			if base.use_visibility {
				if let Some(s) = base.visibility_source.as_deref() {
					add(s);
				}
			}
		}
		// Unlike the C# version, inputs of calculated channels are tracked even
		// when no element binds them directly, so `Rpm / 1000` works on its own.
		for ch in &definition.calculated_channels {
			for name in expression::identifiers(&ch.expression) {
				add(&name);
			}
		}

		let values: Values = sources.iter().map(|s| (s.clone(), 0.0)).collect();
		Self {
			targets: values.clone(),
			values,
			definition,
			renderer: ElementRenderer::new(),
			frame: FrameState::new(),
			last_frame: None,
		}
	}

	pub fn load(path: impl AsRef<std::path::Path>) -> Result<Self, String> {
		CustomGaugeDefinition::load(path).map(Self::new)
	}

	pub fn definition(&self) -> &CustomGaugeDefinition {
		&self.definition
	}

	pub fn renderer_mut(&mut self) -> &mut ElementRenderer {
		&mut self.renderer
	}

	/// Raw data sources this gauge reads (element bindings, visibility
	/// sources and calculated-channel inputs).
	pub fn data_sources(&self) -> impl Iterator<Item = &str> {
		self.targets.keys().map(String::as_str)
	}

	/// Sets the target value of one source (ignored if the gauge doesn't use it).
	pub fn set_value(&mut self, source: &str, value: f32) {
		if let Some(v) = self.targets.get_mut(source) {
			*v = value;
		} else if let Some((_, v)) = self.targets.iter_mut().find(|(k, _)| k.eq_ignore_ascii_case(source)) {
			*v = value;
		}
	}

	/// Pulls every tracked source from a lookup (`UpdateFromDevice`).
	pub fn update_from(&mut self, mut read: impl FnMut(&str) -> Option<f32>) {
		for (name, target) in self.targets.iter_mut() {
			if let Some(v) = read(name) {
				*target = v;
			}
		}
	}

	/// Jumps the smoothed values straight to their targets.
	pub fn snap_to_targets(&mut self) {
		self.values.clone_from(&self.targets);
	}

	/// Smooths using wall-clock time since the previous draw, then renders.
	/// Only regions that changed since the previous draw are repainted; the
	/// returned [`Damage`] lists them.
	pub fn draw(&mut self, canvas: &mut Canvas) -> Damage {
		let now = Instant::now();
		let dt = self.last_frame.map(|t| now.duration_since(t).as_secs_f32()).unwrap_or(1.0 / 60.0);
		self.last_frame = Some(now);
		self.draw_with_delta(canvas, dt)
	}

	/// Smooths assuming `dt` seconds elapsed, then renders. Frame-rate
	/// independent: the same feel as 10 %/frame at 60 fps.
	pub fn draw_with_delta(&mut self, canvas: &mut Canvas, dt: f32) -> Damage {
		let alpha = 1.0 - (1.0 - SMOOTHING_FACTOR).powf(dt.max(0.0) * 60.0);
		for (name, target) in &self.targets {
			if let Some(v) = self.values.get_mut(name) {
				*v += (target - *v) * alpha;
				// Land exactly on the target once visually there, so a settled
				// gauge stops producing damage.
				if (target - *v).abs() <= 1e-4 * (1.0 + target.abs()) {
					*v = *target;
				}
			}
		}
		self.renderer.render_incremental(canvas, &self.definition, &self.values, Some(&self.targets), &mut self.frame)
	}

	/// Forces the next draw to repaint the whole canvas. Call this when the
	/// canvas was drawn on by something else (e.g. after switching screens).
	pub fn invalidate(&mut self) {
		self.frame.invalidate();
	}

	/// Repaints `r` on the next draw (call for areas an overlay drew over).
	pub fn add_damage(&mut self, r: crate::IRect) {
		self.frame.add_damage(r);
	}

	/// Worker threads used for large repaints (default: cores, max 4).
	pub fn set_threads(&mut self, threads: usize) {
		self.frame.threads = threads.max(1);
	}
}
