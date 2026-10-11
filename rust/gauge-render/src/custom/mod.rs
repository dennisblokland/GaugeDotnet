//! Custom (designer-built) gauges: JSON definition → elements → canvas.

mod gauge;
mod renderer;

pub use gauge::CustomGauge;
pub use renderer::{is_static, select_frame, ElementRenderer, StaticCache};
