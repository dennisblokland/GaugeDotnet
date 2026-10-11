//! Rust port of the GaugeDotnet renderer.
//!
//! * [`custom`] renders designer-built JSON gauges (`custom-gauge.json`) with
//!   all 17 element types, calculated channels, visibility rules and smoothing.
//! * [`gauges`] contains the built-in screens (bar, circular, needle, sweep,
//!   digital, min/max, histogram, grid) and the `gauges.json` screen factory.
//! * [`canvas::Canvas`] is the `SKCanvas`-like drawing surface they share,
//!   rasterised on the CPU with tiny-skia, so frames can be shown through SDL2,
//!   written to PNG, or blitted to any framebuffer.

pub mod blur;
pub mod canvas;
pub mod color;
mod composite;
pub mod custom;
pub mod expression;
pub mod format;
pub mod gauges;
pub mod images;
pub mod model;
pub mod text;

use std::collections::HashMap;

pub use canvas::{Blur, BlurStyle, Canvas, Paint, PaintStyle};
pub use color::Rgba;
pub use custom::{CustomGauge, ElementRenderer};
pub use model::{CustomGaugeDefinition, Element};

/// Named channel values (`Rpm`, `OilTemp`, calculated channels, ...).
pub type Values = HashMap<String, f32>;

/// Looks a channel up by exact name, then case-insensitively (the C#
/// renderer switches to `OrdinalIgnoreCase` once calculated channels exist).
pub fn lookup(values: &Values, name: &str) -> Option<f32> {
	values.get(name).copied().or_else(|| values.iter().find(|(k, _)| k.eq_ignore_ascii_case(name)).map(|(_, v)| *v))
}
