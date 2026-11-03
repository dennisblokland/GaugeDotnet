//! `gauge-designer`: desktop editor for custom gauges. The canvas is drawn
//! by `gauge-render`, the renderer the RG35XX runs, so the preview matches
//! the device pixel for pixel.

mod app;
mod config_editor;
mod cost;
mod deploy;
mod document;
mod fields;
mod geometry;
mod package;
mod preview;
mod props;
mod templates;
mod validate;

use std::path::PathBuf;

const USAGE: &str = "usage: gauge-designer [custom-gauge.json] [--config gauges.json]";

fn main() -> eframe::Result {
	let mut gauge = None;
	let mut config = None;
	let mut args = std::env::args().skip(1);
	while let Some(arg) = args.next() {
		match arg.as_str() {
			"--config" => config = args.next().map(PathBuf::from),
			"-h" | "--help" => {
				println!("{USAGE}");
				return Ok(());
			}
			_ if gauge.is_none() && !arg.starts_with('-') => gauge = Some(PathBuf::from(arg)),
			_ => {
				eprintln!("{USAGE}");
				std::process::exit(2);
			}
		}
	}
	let options = eframe::NativeOptions {
		viewport: eframe::egui::ViewportBuilder::default()
			.with_inner_size([1400.0, 860.0])
			.with_min_inner_size([900.0, 560.0])
			.with_title("Gauge Designer"),
		..Default::default()
	};
	eframe::run_native(
		"Gauge Designer",
		options,
		Box::new(move |_cc| Ok(Box::new(app::DesignerApp::new(gauge, config)))),
	)
}
