//! Saving: the C# designer's image packaging. Images are copied into
//! `images/` next to the JSON and paths are made relative, so the folder can
//! go to the device as is. Stills are re-encoded to `image_N.png`;
//! animations (GIF, APNG, WebP) are copied byte for byte with their extension.

use std::path::{Path, PathBuf};

use gauge_render::images::resolve;
use gauge_render::model::*;

/// An image property of the definition.
enum Slot {
	Background,
	Needle(usize),
	Image(usize),
	Gif(usize),
}

fn slots(def: &CustomGaugeDefinition) -> Vec<(Slot, String)> {
	let mut out = Vec::new();
	if let Some(p) = def.background_image.as_deref().filter(|p| !p.is_empty()) {
		out.push((Slot::Background, p.to_string()));
	}
	for (i, element) in def.elements.iter().enumerate() {
		match element {
			Element::Needle(n) => {
				if let Some(p) = n.image_path.as_deref().filter(|p| !p.is_empty()) {
					out.push((Slot::Needle(i), p.to_string()));
				}
			}
			Element::Image(img) if !img.image_path.is_empty() => out.push((Slot::Image(i), img.image_path.clone())),
			Element::Gif(g) if !g.image_path.is_empty() => out.push((Slot::Gif(i), g.image_path.clone())),
			_ => {}
		}
	}
	out
}

fn set_slot(def: &mut CustomGaugeDefinition, slot: &Slot, path: String) {
	match *slot {
		Slot::Background => def.background_image = Some(path),
		Slot::Needle(i) => {
			if let Some(Element::Needle(n)) = def.elements.get_mut(i) {
				n.image_path = Some(path);
			}
		}
		Slot::Image(i) => {
			if let Some(Element::Image(img)) = def.elements.get_mut(i) {
				img.image_path = path;
			}
		}
		Slot::Gif(i) => {
			if let Some(Element::Gif(g)) = def.elements.get_mut(i) {
				g.image_path = path;
			}
		}
	}
}

/// Result of a save: the definition as written, plus problems that didn't stop it.
pub struct Saved {
	pub definition: CustomGaugeDefinition,
	pub warnings: Vec<String>,
}

/// Writes `def` to `json_path`, packaging its images into `images/`.
pub fn save(def: &CustomGaugeDefinition, json_path: &Path) -> Result<Saved, String> {
	let save_dir = json_path.parent().map(Path::to_path_buf).unwrap_or_else(|| PathBuf::from("."));
	let images_dir = save_dir.join("images");
	let mut export = def.clone();
	let mut warnings = Vec::new();

	// Read every source first: a source may itself be images/image_N.png
	// from the last save, and is about to be overwritten.
	let mut prepared: Vec<(Slot, String, Vec<u8>)> = Vec::new();
	let mut index = 0;
	for (slot, path) in slots(def) {
		index += 1;
		let source = resolve(&path, def.base_directory.as_deref());
		let bytes = match std::fs::read(&source) {
			Ok(b) => b,
			Err(e) => {
				warnings.push(format!("{}: {e}; path kept as is", source.display()));
				continue;
			}
		};
		let (name, data) = if matches!(slot, Slot::Gif(_)) {
			let ext = source.extension().and_then(|e| e.to_str()).filter(|e| !e.is_empty()).unwrap_or("gif");
			(format!("image_{index}.{ext}"), bytes)
		} else {
			match to_png(&bytes) {
				Ok(png) => (format!("image_{index}.png"), png),
				Err(e) => {
					warnings.push(format!("{}: {e}; path kept as is", source.display()));
					continue;
				}
			}
		};
		prepared.push((slot, name, data));
	}

	if !prepared.is_empty() {
		std::fs::create_dir_all(&images_dir).map_err(|e| format!("{}: {e}", images_dir.display()))?;
	}
	for (slot, name, data) in prepared {
		let dest = images_dir.join(&name);
		std::fs::write(&dest, data).map_err(|e| format!("{}: {e}", dest.display()))?;
		// Forward slashes, so the path works on the device whatever OS saved it.
		set_slot(&mut export, &slot, format!("images/{name}"));
	}

	write_atomic(json_path, export.to_json().as_bytes())?;
	export.base_directory = std::fs::canonicalize(&save_dir).ok().or(Some(save_dir));
	Ok(Saved { definition: export, warnings })
}

/// Re-encodes any still image the renderer reads as PNG (`SaveImageFromCache`).
fn to_png(bytes: &[u8]) -> Result<Vec<u8>, String> {
	let img = image::load_from_memory(bytes).map_err(|e| e.to_string())?;
	let mut out = std::io::Cursor::new(Vec::new());
	img.write_to(&mut out, image::ImageFormat::Png).map_err(|e| e.to_string())?;
	Ok(out.into_inner())
}

/// Temp file, then rename: a crash mid-save never leaves a truncated file.
pub fn write_atomic(path: &Path, data: &[u8]) -> Result<(), String> {
	let tmp = path.with_extension("json.tmp");
	std::fs::write(&tmp, data).map_err(|e| format!("{}: {e}", tmp.display()))?;
	std::fs::rename(&tmp, path).map_err(|e| format!("{}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
	use super::*;

	fn tmp_dir(name: &str) -> PathBuf {
		let dir = std::env::temp_dir().join(format!("gauge-designer-{name}-{}", std::process::id()));
		let _ = std::fs::remove_dir_all(&dir);
		std::fs::create_dir_all(&dir).unwrap();
		dir
	}

	fn write_jpeg(path: &Path) {
		image::RgbImage::from_pixel(4, 4, image::Rgb([200, 10, 10])).save(path).unwrap();
	}

	#[test]
	fn packages_images_like_the_csharp_designer() {
		let src = tmp_dir("src");
		let out = tmp_dir("out");
		write_jpeg(&src.join("bg.jpg"));
		write_jpeg(&src.join("needle.jpg"));
		std::fs::write(src.join("anim.gif"), b"GIF89a not decoded, just copied").unwrap();

		let mut def = CustomGaugeDefinition {
			background_image: Some(src.join("bg.jpg").display().to_string()),
			..Default::default()
		};
		def.elements.push(Element::Circle(CircleElement::default()));
		def.elements
			.push(Element::Needle(NeedleElement { image_path: Some("needle.jpg".into()), ..Default::default() }));
		def.elements.push(Element::Gif(GifElement { image_path: "anim.gif".into(), ..Default::default() }));
		def.base_directory = Some(src.clone());

		let saved = save(&def, &out.join("g.json")).unwrap();
		assert!(saved.warnings.is_empty(), "{:?}", saved.warnings);
		let back = CustomGaugeDefinition::load(out.join("g.json")).unwrap();
		assert_eq!(back.background_image.as_deref(), Some("images/image_1.png"));
		let Element::Needle(n) = &back.elements[1] else { panic!() };
		assert_eq!(n.image_path.as_deref(), Some("images/image_2.png"));
		let Element::Gif(g) = &back.elements[2] else { panic!() };
		assert_eq!(g.image_path, "images/image_3.gif");
		assert!(image::open(out.join("images/image_2.png")).is_ok(), "stills become real PNGs");
		assert_eq!(std::fs::read(out.join("images/image_3.gif")).unwrap(), b"GIF89a not decoded, just copied");

		// Saving the saved file again in place keeps working (sources are read first).
		let saved_again = save(&saved.definition, &out.join("g.json")).unwrap();
		assert!(saved_again.warnings.is_empty());
		assert!(image::open(out.join("images/image_1.png")).is_ok());
	}

	#[test]
	fn missing_images_keep_their_path() {
		let out = tmp_dir("missing");
		let mut def = CustomGaugeDefinition::default();
		def.elements.push(Element::Image(ImageElement { image_path: "/nowhere/x.png".into(), ..Default::default() }));
		let saved = save(&def, &out.join("g.json")).unwrap();
		assert_eq!(saved.warnings.len(), 1);
		let Element::Image(i) = &saved.definition.elements[0] else { panic!() };
		assert_eq!(i.image_path, "/nowhere/x.png");
	}
}
