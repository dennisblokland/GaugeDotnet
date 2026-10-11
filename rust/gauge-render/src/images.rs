//! Image and animation-frame cache (`ImageCache` + `GifCache`).

use std::collections::HashMap;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use image::{AnimationDecoder, RgbaImage};
use tiny_skia::{Pixmap, PremultipliedColorU8};

/// Resolves a path from gauge JSON: absolute paths as-is, relative ones
/// against the definition's folder, else next to the executable.
pub fn resolve(path: &str, base_dir: Option<&Path>) -> PathBuf {
	let p = Path::new(path);
	if p.is_absolute() {
		return p.to_path_buf();
	}
	match base_dir {
		Some(dir) => dir.join(p),
		None => std::env::current_exe()
			.ok()
			.and_then(|exe| exe.parent().map(|d| d.join(p)))
			.unwrap_or_else(|| p.to_path_buf()),
	}
}

/// Converts straight RGBA to a premultiplied pixmap.
pub fn pixmap_from_rgba(img: &RgbaImage) -> Option<Pixmap> {
	let mut pixmap = Pixmap::new(img.width(), img.height())?;
	for (dst, src) in pixmap.pixels_mut().iter_mut().zip(img.pixels()) {
		let [r, g, b, a] = src.0;
		let pm = |c: u8| ((c as u16 * a as u16 + 127) / 255) as u8;
		*dst = PremultipliedColorU8::from_rgba(pm(r), pm(g), pm(b), a).expect("premultiplied");
	}
	Some(pixmap)
}

fn decode_still(path: &Path) -> Option<Pixmap> {
	let img = image::open(path).ok()?;
	pixmap_from_rgba(&img.to_rgba8())
}

/// Decodes every frame of an animated GIF/WebP/APNG, fully composited
/// (disposal and blending applied), falling back to a single still frame.
fn decode_frames(path: &Path) -> Option<Vec<Pixmap>> {
	let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
	let open = || std::fs::File::open(path).ok().map(BufReader::new);
	let frames: Option<Vec<image::Frame>> = match ext.as_str() {
		"gif" => image::codecs::gif::GifDecoder::new(open()?).ok().and_then(|d| d.into_frames().collect_frames().ok()),
		"webp" => image::codecs::webp::WebPDecoder::new(open()?).ok().and_then(|d| {
			if d.has_animation() {
				d.into_frames().collect_frames().ok()
			} else {
				None
			}
		}),
		"png" | "apng" => image::codecs::png::PngDecoder::new(open()?).ok().and_then(|d| {
			if d.is_apng().unwrap_or(false) {
				d.apng().ok().and_then(|a| a.into_frames().collect_frames().ok())
			} else {
				None
			}
		}),
		_ => None,
	};
	match frames {
		Some(frames) if !frames.is_empty() => {
			frames.iter().map(|f| pixmap_from_rgba(f.buffer())).collect::<Option<Vec<_>>>()
		}
		_ => decode_still(path).map(|p| vec![p]),
	}
}

/// Caches decoded images by resolved path; failures are cached as `None` so a
/// missing file is not re-read every frame.
#[derive(Default)]
pub struct ImageCache {
	stills: HashMap<PathBuf, Option<Arc<Pixmap>>>,
	animations: HashMap<PathBuf, Option<Arc<Vec<Pixmap>>>>,
}

impl ImageCache {
	pub fn load(&mut self, path: &str, base_dir: Option<&Path>) -> Option<Arc<Pixmap>> {
		let resolved = resolve(path, base_dir);
		self.stills.entry(resolved).or_insert_with_key(|p| decode_still(p).map(Arc::new)).clone()
	}

	pub fn load_frames(&mut self, path: &str, base_dir: Option<&Path>) -> Option<Arc<Vec<Pixmap>>> {
		let resolved = resolve(path, base_dir);
		self.animations.entry(resolved).or_insert_with_key(|p| decode_frames(p).map(Arc::new)).clone()
	}

	/// Read-only lookup of an image previously [`load`](Self::load)ed.
	pub fn get(&self, path: &str, base_dir: Option<&Path>) -> Option<Arc<Pixmap>> {
		self.stills.get(&resolve(path, base_dir)).cloned().flatten()
	}

	/// Read-only lookup of frames previously [`load_frames`](Self::load_frames)ed.
	pub fn get_frames(&self, path: &str, base_dir: Option<&Path>) -> Option<Arc<Vec<Pixmap>>> {
		self.animations.get(&resolve(path, base_dir)).cloned().flatten()
	}

	pub fn clear(&mut self) {
		self.stills.clear();
		self.animations.clear();
	}
}
