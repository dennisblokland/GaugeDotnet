//! Custom (designer-built) gauges: JSON definition → elements → canvas.

mod gauge;
mod incremental;
mod renderer;

pub use gauge::CustomGauge;
pub use incremental::{default_threads, Damage, FrameState};
pub use renderer::{is_static, select_frame, ElementRenderer, Painter, StaticCache};
