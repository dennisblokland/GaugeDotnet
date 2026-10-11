//! Integer pixel helpers for the hot paths (layer composites and glow
//! blits), which are much cheaper than routing them through tiny-skia's
//! general float pipeline.

use tiny_skia::{Mask, Pixmap};

#[inline]
fn div255(v: u32) -> u32 {
	(v + 128 + ((v + 128) >> 8)) >> 8
}

/// Integer device-space rectangle, `x0..x1` × `y0..y1`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IRect {
	pub x0: i32,
	pub y0: i32,
	pub x1: i32,
	pub y1: i32,
}

impl IRect {
	pub fn from_bounds(l: f32, t: f32, r: f32, b: f32) -> Self {
		Self { x0: l.floor() as i32, y0: t.floor() as i32, x1: r.ceil() as i32, y1: b.ceil() as i32 }
	}

	pub fn union(self, o: IRect) -> IRect {
		IRect { x0: self.x0.min(o.x0), y0: self.y0.min(o.y0), x1: self.x1.max(o.x1), y1: self.y1.max(o.y1) }
	}

	pub fn clamp_to(self, w: u32, h: u32) -> Option<IRect> {
		let r = IRect { x0: self.x0.max(0), y0: self.y0.max(0), x1: self.x1.min(w as i32), y1: self.y1.min(h as i32) };
		(r.x0 < r.x1 && r.y0 < r.y1).then_some(r)
	}
}

/// Premultiplied source-over of a `src_w`×`src_h` RGBA buffer onto `dst`
/// with its top-left at (`dx`, `dy`), scaled by `opacity` and the clip mask.
#[allow(clippy::too_many_arguments)]
pub fn src_over(
	dst: &mut Pixmap,
	src: &[u8],
	src_stride: usize,
	src_w: u32,
	src_h: u32,
	dx: i32,
	dy: i32,
	opacity: u8,
	clip: Option<&Mask>,
) {
	if opacity == 0 {
		return;
	}
	let Some(r) =
		(IRect { x0: dx, y0: dy, x1: dx + src_w as i32, y1: dy + src_h as i32 }).clamp_to(dst.width(), dst.height())
	else {
		return;
	};
	let dst_w = dst.width() as usize;
	let mask = clip.map(|m| m.data());
	let data = dst.data_mut();
	for y in r.y0..r.y1 {
		let sy = (y - dy) as usize;
		let drow = y as usize * dst_w;
		for x in r.x0..r.x1 {
			let so = sy * src_stride + (x - dx) as usize * 4;
			let sa = src[so + 3] as u32;
			if sa == 0 {
				continue;
			}
			let mut cover = opacity as u32;
			if let Some(m) = mask {
				cover = div255(cover * m[drow + x as usize] as u32);
				if cover == 0 {
					continue;
				}
			}
			let d = (drow + x as usize) * 4;
			if cover == 255 && sa == 255 {
				data[d..d + 4].copy_from_slice(&src[so..so + 4]);
				continue;
			}
			let a = if cover == 255 { sa } else { div255(sa * cover) };
			let inv = 255 - a;
			for c in 0..4 {
				let s = if cover == 255 { src[so + c] as u32 } else { div255(src[so + c] as u32 * cover) };
				data[d + c] = (s + div255(data[d + c] as u32 * inv)).min(255) as u8;
			}
		}
	}
}

/// Bilinear upscale of `src` by an integer factor into a `w`×`h` buffer,
/// sampling pixel centres (`(dst + 0.5) / scale - 0.5`).
pub fn upscale_bilinear(src: &Pixmap, scale: u32, w: u32, h: u32) -> Vec<u8> {
	let (sw, sh) = (src.width() as i32, src.height() as i32);
	let sdata = src.data();
	let mut out = vec![0u8; (w * h * 4) as usize];
	let s = scale as i32;
	// 8-bit fixed point source coordinates.
	let coord = |d: i32| ((d * 2 + 1) * 256 / (2 * s)) - 128;
	let xs: Vec<(usize, usize, u32)> = (0..w as i32)
		.map(|dx| {
			let fx = coord(dx);
			let x0 = (fx >> 8).clamp(0, sw - 1);
			let x1 = (x0 + 1).min(sw - 1);
			let wx = if fx < 0 { 0 } else { (fx & 255) as u32 };
			(x0 as usize * 4, x1 as usize * 4, wx)
		})
		.collect();
	let stride = sw as usize * 4;
	for dy in 0..h as i32 {
		let fy = coord(dy);
		let y0 = (fy >> 8).clamp(0, sh - 1);
		let y1 = (y0 + 1).min(sh - 1);
		let wy = if fy < 0 { 0 } else { (fy & 255) as u32 };
		let (r0, r1) = (y0 as usize * stride, y1 as usize * stride);
		let orow = dy as usize * w as usize * 4;
		for (dx, &(x0, x1, wx)) in xs.iter().enumerate() {
			let o = orow + dx * 4;
			for c in 0..4 {
				let top = sdata[r0 + x0 + c] as u32 * (256 - wx) + sdata[r0 + x1 + c] as u32 * wx;
				let bot = sdata[r1 + x0 + c] as u32 * (256 - wx) + sdata[r1 + x1 + c] as u32 * wx;
				out[o + c] = ((top * (256 - wy) + bot * wy + (1 << 15)) >> 16) as u8;
			}
		}
	}
	out
}

#[cfg(test)]
mod tests {
	use super::*;
	use tiny_skia::Color;

	#[test]
	fn src_over_blends_half_opacity() {
		let mut dst = Pixmap::new(4, 4).unwrap();
		dst.fill(Color::BLACK);
		let src = vec![255u8; 2 * 2 * 4];
		src_over(&mut dst, &src, 8, 2, 2, 1, 1, 128, None);
		let p = dst.pixel(1, 1).unwrap();
		assert!((p.red() as i32 - 128).abs() <= 1);
		assert_eq!(dst.pixel(0, 0).unwrap().red(), 0);
		assert_eq!(dst.pixel(3, 3).unwrap().red(), 0);
	}

	#[test]
	fn upscale_keeps_flat_colour() {
		let mut src = Pixmap::new(3, 3).unwrap();
		src.fill(Color::from_rgba8(200, 100, 50, 255));
		let out = upscale_bilinear(&src, 4, 12, 12);
		assert!(out.chunks(4).all(|p| p == [200, 100, 50, 255]));
	}
}
