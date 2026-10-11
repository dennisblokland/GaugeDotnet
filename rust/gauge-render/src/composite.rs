//! Integer pixel helpers for the hot paths (layer composites, cached
//! sprites and glow blits), which are much cheaper than routing them through
//! tiny-skia's general float pipeline.

use tiny_skia::{Mask, Pixmap};

use crate::color::Rgba;

#[inline(always)]
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

	pub fn intersect(self, o: IRect) -> Option<IRect> {
		let r = IRect { x0: self.x0.max(o.x0), y0: self.y0.max(o.y0), x1: self.x1.min(o.x1), y1: self.y1.min(o.y1) };
		(r.x0 < r.x1 && r.y0 < r.y1).then_some(r)
	}

	pub fn area(self) -> usize {
		self.width() * self.height()
	}

	pub fn width(self) -> usize {
		(self.x1 - self.x0) as usize
	}

	pub fn height(self) -> usize {
		(self.y1 - self.y0) as usize
	}
}

/// Bounds of the non-transparent pixels of a pixmap.
pub fn opaque_bounds(p: &Pixmap) -> Option<IRect> {
	let w = p.width() as usize;
	let data = p.data();
	let (mut x0, mut y0, mut x1, mut y1) = (usize::MAX, usize::MAX, 0, 0);
	for (y, row) in data.chunks_exact(w * 4).enumerate() {
		let mut first = None;
		let mut last = 0;
		for (x, px) in row.chunks_exact(4).enumerate() {
			if px[3] != 0 {
				first.get_or_insert(x);
				last = x;
			}
		}
		if let Some(f) = first {
			x0 = x0.min(f);
			x1 = x1.max(last + 1);
			y0 = y0.min(y);
			y1 = y + 1;
		}
	}
	(x0 != usize::MAX).then_some(IRect { x0: x0 as i32, y0: y0 as i32, x1: x1 as i32, y1: y1 as i32 })
}

/// Visible part of a `w`×`h` source placed at (`dx`, `dy`) on a `dw`×`dh` target.
fn visible(dx: i32, dy: i32, w: u32, h: u32, dw: u32, dh: u32) -> Option<IRect> {
	IRect { x0: dx, y0: dy, x1: dx + w as i32, y1: dy + h as i32 }.clamp_to(dw, dh)
}

#[inline(always)]
fn blend_px(d: &mut [u8], s: &[u8]) {
	let sa = s[3] as u32;
	if sa == 255 {
		d.copy_from_slice(s);
	} else if sa != 0 {
		let inv = 255 - sa;
		for c in 0..4 {
			d[c] = (s[c] as u32 + div255(d[c] as u32 * inv)) as u8;
		}
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
	let Some(r) = visible(dx, dy, src_w, src_h, dst.width(), dst.height()) else {
		return;
	};
	let dst_w = dst.width() as usize;
	let mask = clip.map(|m| m.data());
	let data = dst.data_mut();
	let (w, sx0) = (r.width(), (r.x0 - dx) as usize);
	for y in r.y0..r.y1 {
		let so = (y - dy) as usize * src_stride + sx0 * 4;
		let srow = &src[so..so + w * 4];
		let d0 = (y as usize * dst_w + r.x0 as usize) * 4;
		let drow = &mut data[d0..d0 + w * 4];
		match (mask, opacity) {
			(None, 255) => {
				for (d, s) in drow.chunks_exact_mut(4).zip(srow.chunks_exact(4)) {
					blend_px(d, s);
				}
			}
			_ => {
				let mrow = mask.map(|m| &m[y as usize * dst_w + r.x0 as usize..][..w]);
				for (i, (d, s)) in drow.chunks_exact_mut(4).zip(srow.chunks_exact(4)).enumerate() {
					if s[3] == 0 {
						continue;
					}
					let mut cover = opacity as u32;
					if let Some(m) = mrow {
						cover = div255(cover * m[i] as u32);
					}
					if cover == 0 {
						continue;
					}
					let a = div255(s[3] as u32 * cover);
					let inv = 255 - a;
					for c in 0..4 {
						d[c] = (div255(s[c] as u32 * cover) + div255(d[c] as u32 * inv)).min(255) as u8;
					}
				}
			}
		}
	}
}

/// Copies opaque rows straight into `dst` (no blending needed).
pub fn copy_rows(dst: &mut Pixmap, src: &[u8], src_stride: usize, src_w: u32, src_h: u32, dx: i32, dy: i32) {
	let Some(r) = visible(dx, dy, src_w, src_h, dst.width(), dst.height()) else {
		return;
	};
	let dst_w = dst.width() as usize;
	let data = dst.data_mut();
	let (w, sx0) = (r.width() * 4, (r.x0 - dx) as usize * 4);
	for y in r.y0..r.y1 {
		let so = (y - dy) as usize * src_stride + sx0;
		let d0 = (y as usize * dst_w + r.x0 as usize) * 4;
		data[d0..d0 + w].copy_from_slice(&src[so..so + w]);
	}
}

/// Source-over of a solid `color` through an 8-bit coverage buffer: the
/// composite step of a glow, which is one colour times a blurred alpha.
#[allow(clippy::too_many_arguments)]
pub fn src_over_color(
	dst: &mut Pixmap,
	alpha: &[u8],
	stride: usize,
	w: u32,
	h: u32,
	dx: i32,
	dy: i32,
	color: Rgba,
	clip: Option<&Mask>,
) {
	if color.a == 0 {
		return;
	}
	let Some(r) = visible(dx, dy, w, h, dst.width(), dst.height()) else {
		return;
	};
	let dst_w = dst.width() as usize;
	let mask = clip.map(|m| m.data());
	let data = dst.data_mut();
	let ca = color.a as u32;
	// Premultiplied colour channels.
	let pm = [div255(color.r as u32 * ca), div255(color.g as u32 * ca), div255(color.b as u32 * ca), ca];
	let (rw, sx0) = (r.width(), (r.x0 - dx) as usize);
	for y in r.y0..r.y1 {
		let arow = &alpha[(y - dy) as usize * stride + sx0..][..rw];
		let d0 = (y as usize * dst_w + r.x0 as usize) * 4;
		let drow = &mut data[d0..d0 + rw * 4];
		let mrow = mask.map(|m| &m[y as usize * dst_w + r.x0 as usize..][..rw]);
		for (i, (d, &a)) in drow.chunks_exact_mut(4).zip(arow).enumerate() {
			let mut cover = a as u32;
			if let Some(m) = mrow {
				cover = div255(cover * m[i] as u32);
			}
			if cover == 0 {
				continue;
			}
			let inv = 255 - div255(ca * cover);
			for c in 0..4 {
				d[c] = (div255(pm[c] * cover) + div255(d[c] as u32 * inv)).min(255) as u8;
			}
		}
	}
}

/// Bilinear upscale of an 8-bit buffer by an integer factor, producing only
/// the `w`×`h` window starting at (`ox`, `oy`) of the upscaled image
/// (sampling pixel centres: `(dst + 0.5) / scale - 0.5`).
#[allow(clippy::too_many_arguments)]
pub fn upscale_bilinear_a8(src: &[u8], sw: usize, sh: usize, scale: u32, ox: u32, oy: u32, w: u32, h: u32) -> Vec<u8> {
	let (sw, sh) = (sw as i32, sh as i32);
	let mut out = vec![0u8; (w * h) as usize];
	let s = scale as i32;
	// 8-bit fixed point source coordinates.
	let coord = |d: i32| ((d * 2 + 1) * 256 / (2 * s)) - 128;
	let xs: Vec<(usize, usize, u32)> = (ox as i32..(ox + w) as i32)
		.map(|dx| {
			let fx = coord(dx);
			let x0 = (fx >> 8).clamp(0, sw - 1);
			let x1 = (x0 + 1).min(sw - 1);
			let wx = if fx < 0 { 0 } else { (fx & 255) as u32 };
			(x0 as usize, x1 as usize, wx)
		})
		.collect();
	for (row, orow) in out.chunks_exact_mut(w as usize).enumerate() {
		let fy = coord(oy as i32 + row as i32);
		let y0 = (fy >> 8).clamp(0, sh - 1);
		let y1 = (y0 + 1).min(sh - 1);
		let wy = if fy < 0 { 0 } else { (fy & 255) as u32 };
		let r0 = &src[y0 as usize * sw as usize..][..sw as usize];
		let r1 = &src[y1 as usize * sw as usize..][..sw as usize];
		for (o, &(x0, x1, wx)) in orow.iter_mut().zip(&xs) {
			let top = r0[x0] as u32 * (256 - wx) + r0[x1] as u32 * wx;
			let bot = r1[x0] as u32 * (256 - wx) + r1[x1] as u32 * wx;
			*o = ((top * (256 - wy) + bot * wy + (1 << 15)) >> 16) as u8;
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
	fn colour_blit_matches_premultiplied_over() {
		let mut dst = Pixmap::new(2, 1).unwrap();
		dst.fill(Color::BLACK);
		src_over_color(&mut dst, &[255, 128], 2, 2, 1, 0, 0, Rgba::rgb(200, 100, 0), None);
		assert_eq!(dst.pixel(0, 0).unwrap().red(), 200);
		assert!((dst.pixel(1, 0).unwrap().red() as i32 - 100).abs() <= 1);
	}

	#[test]
	fn upscale_keeps_flat_value() {
		let out = upscale_bilinear_a8(&[90; 9], 3, 3, 4, 0, 0, 12, 12);
		assert!(out.iter().all(|&v| v == 90));
		let win = upscale_bilinear_a8(&[0, 40, 80, 0, 40, 80], 3, 2, 2, 2, 1, 3, 2);
		let full = upscale_bilinear_a8(&[0, 40, 80, 0, 40, 80], 3, 2, 2, 0, 0, 6, 4);
		assert_eq!(win, [full[8], full[9], full[10], full[14], full[15], full[16]]);
		assert!(out.iter().all(|&v| v == 90));
	}

	#[test]
	fn finds_content_bounds() {
		let mut p = Pixmap::new(10, 10).unwrap();
		p.fill_rect(
			tiny_skia::Rect::from_xywh(2.0, 3.0, 4.0, 5.0).unwrap(),
			&tiny_skia::Paint::default(),
			tiny_skia::Transform::identity(),
			None,
		);
		assert_eq!(opaque_bounds(&p), Some(IRect { x0: 2, y0: 3, x1: 6, y1: 8 }));
		assert_eq!(opaque_bounds(&Pixmap::new(4, 4).unwrap()), None);
	}
}
