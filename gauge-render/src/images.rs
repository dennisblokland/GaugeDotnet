//! Image and animation-frame cache (`ImageCache` + `GifCache`).

use std::collections::HashMap;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use image::{AnimationDecoder, ImageFormat, RgbaImage};
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

/// The file's format from its first bytes, like Skia's codec sniffing; the
/// extension is only a fallback for content `image` can't recognise.
fn sniff_format(path: &Path) -> Option<ImageFormat> {
	use std::io::Read;
	let mut head = [0u8; 32];
	let n = std::fs::File::open(path).ok()?.read(&mut head).ok()?;
	image::guess_format(&head[..n]).ok().or_else(|| ImageFormat::from_path(path).ok())
}

fn decode_still(path: &Path) -> Option<Pixmap> {
	let file = BufReader::new(std::fs::File::open(path).ok()?);
	let img = image::ImageReader::with_format(file, sniff_format(path)?).decode().ok()?;
	pixmap_from_rgba(&img.to_rgba8())
}

/// Decodes every frame of an animated GIF/WebP/APNG, fully composited
/// (disposal and blending applied), falling back to a single still frame.
fn decode_frames(path: &Path) -> Option<Vec<Pixmap>> {
	let open = || std::fs::File::open(path).ok().map(BufReader::new);
	let frames: Option<Vec<image::Frame>> = match sniff_format(path)? {
		ImageFormat::Gif => {
			image::codecs::gif::GifDecoder::new(open()?).ok().and_then(|d| d.into_frames().collect_frames().ok())
		}
		ImageFormat::WebP => image::codecs::webp::WebPDecoder::new(open()?).ok().and_then(|d| {
			if d.has_animation() {
				d.into_frames().collect_frames().ok()
			} else {
				None
			}
		}),
		ImageFormat::Png => image::codecs::png::PngDecoder::new(open()?).ok().and_then(|d| {
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

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn format_comes_from_content_and_bmp_loads() {
		let dir = std::env::temp_dir().join(format!("gauge-images-{}", std::process::id()));
		std::fs::create_dir_all(&dir).unwrap();
		let img = RgbaImage::from_pixel(3, 2, image::Rgba([255, 0, 0, 255]));
		// A PNG saved as .jpg and a BMP picked for a GIF element both load.
		let png_as_jpg = dir.join("still.jpg");
		img.save_with_format(&png_as_jpg, ImageFormat::Png).unwrap();
		let bmp_as_gif = dir.join("anim.gif");
		image::DynamicImage::ImageRgba8(img).to_rgb8().save_with_format(&bmp_as_gif, ImageFormat::Bmp).unwrap();

		let still = decode_still(&png_as_jpg).expect("png content decodes");
		assert_eq!((still.width(), still.height()), (3, 2));
		let frames = decode_frames(&bmp_as_gif).expect("bmp content decodes");
		assert_eq!(frames.len(), 1);
		assert_eq!(frames[0].pixel(0, 0).unwrap().red(), 255);
		let _ = std::fs::remove_dir_all(dir);
	}
}
