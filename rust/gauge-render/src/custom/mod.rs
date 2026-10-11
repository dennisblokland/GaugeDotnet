//! Custom (designer-built) gauges: JSON definition → elements → canvas.

mod gauge;
mod renderer;

pub use gauge::CustomGauge;
pub use renderer::{select_frame, ElementRenderer};
