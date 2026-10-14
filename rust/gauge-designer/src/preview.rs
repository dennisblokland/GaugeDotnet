//! Device-look preview options (overlays, LCD look) and image exports.

use std::path::Path;

use gauge_render::canvas::rect;
use gauge_render::text::{default_typeface, TextAlign};
use gauge_render::{Canvas, CustomGaugeDefinition, ElementRenderer, Paint, Rgba, Values};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PreviewOptions {
	/// The app's FPS counter in the top-left corner.
	pub fps: bool,
	/// The low-battery icon at this charge (it only shows at 30 % or less).
	pub battery: Option<i32>,
	/// The TurboXs old-LCD look applied to the whole frame (an approximation:
	/// TurboXs renders at LCD resolution, this filters the full frame).
	pub lcd: bool,
}

pub fn apply(canvas: &mut Canvas, options: &PreviewOptions) {
	if options.lcd {
		lcd_look(canvas);
	}
	if options.fps {
		// Same text and place as `FpsCounter` in gauge-app.
		canvas.draw_text(
			"FPS: 60.00",
			10.0,
			25.0,
			TextAlign::Left,
			default_typeface(),
			20.0,
			&Paint::fill(Rgba::WHITE),
		);
	}
	if let Some(pct) = options.battery.filter(|p| *p <= 30) {
		draw_battery(canvas, pct);
	}
}

/// `BatteryOverlay::draw` from gauge-app (without the blink).
fn draw_battery(canvas: &mut Canvas, pct: i32) {
	let critical = pct <= 15;
	let fill = if critical { Rgba::rgb(0xFF, 0x20, 0x20) } else { Rgba::rgb(0xFF, 0x8C, 0x00) };
	let outline = Rgba::rgb(0xCC, 0xCC, 0xCC);
	let (body_w, body_h, nub_w, nub_h, pad) = (48.0, 20.0, 5.0, 10.0, 12.0);
	let x = canvas.width() as f32 - body_w - nub_w - pad;
	let y = 8.0;
	if let Some(r) = rect(x, y, x + body_w, y + body_h) {
		canvas.draw_round_rect(r, 3.0, 3.0, &Paint::stroke(outline, 1.5));
	}
	if let Some(r) = rect(x + body_w, y + (body_h - nub_h) / 2.0, x + body_w + nub_w, y + (body_h + nub_h) / 2.0) {
		canvas.draw_round_rect(r, 2.0, 2.0, &Paint::fill(outline));
	}
	let fill_w = (body_w - 4.0) * (pct.clamp(0, 100) as f32 / 100.0);
	if fill_w > 0.0 {
		canvas.draw_rect_xywh(x + 2.0, y + 2.0, fill_w, body_h - 4.0, &Paint::fill(fill));
	}
	canvas.draw_text(
		&format!("{pct}%"),
		x + body_w / 2.0,
		y + body_h / 2.0 + 4.0,
		TextAlign::Center,
		default_typeface(),
		11.0,
		&Paint::fill(Rgba::WHITE),
	);
}

// The constants of the TurboXs LCD mode (gauges/turboxs.rs).
const LCD_CELL: usize = 3;
const LCD_GRID: f32 = 0.85;
const LCD_BLACK: [f32; 3] = [10.0, 14.0, 36.0];
const LCD_WHITE: [f32; 3] = [236.0, 240.0, 255.0];

/// Averages 3×3 cells, maps black/white to the backlight colours and darkens
/// the last row and column of each cell into a pixel grid.
fn lcd_look(canvas: &mut Canvas) {
	let (w, h) = (canvas.width() as usize, canvas.height() as usize);
	let data = canvas.pixmap_mut().data_mut();
	for cy in (0..h).step_by(LCD_CELL) {
		for cx in (0..w).step_by(LCD_CELL) {
			let (mut sum, mut n) = ([0u32; 3], 0u32);
			for y in cy..(cy + LCD_CELL).min(h) {
				for x in cx..(cx + LCD_CELL).min(w) {
					let i = (y * w + x) * 4;
					for c in 0..3 {
						sum[c] += data[i + c] as u32;
					}
					n += 1;
				}
			}
			let tinted: [f32; 3] = std::array::from_fn(|c| {
				let v = sum[c] as f32 / n as f32 / 255.0;
				LCD_BLACK[c] + (LCD_WHITE[c] - LCD_BLACK[c]) * v
			});
			for y in cy..(cy + LCD_CELL).min(h) {
				for x in cx..(cx + LCD_CELL).min(w) {
					let line = y - cy == LCD_CELL - 1 || x - cx == LCD_CELL - 1;
					let i = (y * w + x) * 4;
					for c in 0..3 {
						data[i + c] = (tinted[c] * if line { LCD_GRID } else { 1.0 }).round() as u8;
					}
					data[i + 3] = 255;
				}
			}
		}
	}
}

/// Straight (not premultiplied) RGBA of a canvas.
pub fn to_rgba_image(canvas: &Canvas) -> image::RgbaImage {
	let mut img = image::RgbaImage::from_raw(canvas.width(), canvas.height(), canvas.pixmap().data().to_vec())
		.expect("size matches");
	for p in img.pixels_mut() {
		let a = p[3] as u32;
		if a != 0 && a != 255 {
			for c in 0..3 {
				p[c] = ((p[c] as u32 * 255 + a / 2) / a).min(255) as u8;
			}
		}
	}
	img
}

pub fn export_png(canvas: &Canvas, path: &Path) -> Result<(), String> {
	to_rgba_image(canvas).save(path).map_err(|e| format!("{}: {e}", path.display()))
}

/// Values for frame `i` of `frames` in a sweep from each source's min to its
/// max and back.
pub fn sweep_values(sources: &[(String, f32, f32)], i: usize, frames: usize) -> Values {
	let half = (frames / 2).max(1) as f32;
	let t = if (i as f32) <= half { i as f32 / half } else { (frames - i) as f32 / half };
	sources.iter().map(|(name, min, max)| (name.clone(), min + (max - min) * t.clamp(0.0, 1.0))).collect()
}

/// Renders an animated GIF of every value sweeping min → max → min.
pub fn export_gif(
	def: &CustomGaugeDefinition,
	sources: &[(String, f32, f32)],
	options: &PreviewOptions,
	frames: usize,
	path: &Path,
) -> Result<(), String> {
	use image::codecs::gif::{GifEncoder, Repeat};
	use image::{Delay, Frame};

	let file = std::fs::File::create(path).map_err(|e| format!("{}: {e}", path.display()))?;
	let mut encoder = GifEncoder::new_with_speed(std::io::BufWriter::new(file), 10);
	encoder.set_repeat(Repeat::Infinite).map_err(|e| e.to_string())?;
	let mut renderer = ElementRenderer::new();
	let (w, h) = (def.width.max(1) as u32, def.height.max(1) as u32);
	for i in 0..frames {
		let values = sweep_values(sources, i, frames);
		let mut canvas = Canvas::new(w, h);
		renderer.render(&mut canvas, def, &values, None);
		apply(&mut canvas, options);
		let frame = Frame::from_parts(to_rgba_image(&canvas), 0, 0, Delay::from_numer_denom_ms(40, 1));
		encoder.encode_frame(frame).map_err(|e| e.to_string())?;
	}
	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn sweep_goes_up_and_back() {
		let s = [("Rpm".to_string(), 0.0, 8000.0)];
		assert_eq!(sweep_values(&s, 0, 10)["Rpm"], 0.0);
		assert_eq!(sweep_values(&s, 5, 10)["Rpm"], 8000.0);
		assert_eq!(sweep_values(&s, 9, 10)["Rpm"], 1600.0);
	}

	#[test]
	fn lcd_look_tints_black() {
		let mut canvas = Canvas::new(6, 6);
		canvas.clear(Rgba::BLACK);
		lcd_look(&mut canvas);
		assert_eq!(canvas.pixel(0, 0), Some(Rgba::rgb(10, 14, 36)));
		assert_eq!(canvas.pixel(2, 0).map(|p| p.r), Some(9), "grid line is darker");
	}

	#[test]
	fn exports_an_animated_gif() {
		let path = std::env::temp_dir().join(format!("gauge-designer-{}.gif", std::process::id()));
		let def = crate::templates::rpm_dial();
		export_gif(&def, &[("Rpm".into(), 0.0, 8000.0)], &PreviewOptions::default(), 4, &path).unwrap();
		let decoder =
			image::codecs::gif::GifDecoder::new(std::io::BufReader::new(std::fs::File::open(&path).unwrap())).unwrap();
		use image::AnimationDecoder;
		assert_eq!(decoder.into_frames().count(), 4);
	}
}
