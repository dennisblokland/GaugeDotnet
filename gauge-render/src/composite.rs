//! Integer pixel helpers for the hot paths (layer composites, cached
//! sprites and glow blits), which are much cheaper than routing them through
//! tiny-skia's general float pipeline.

use tiny_skia::{Mask, Pixmap};

use crate::color::Rgba;

/// `round(v / 255)` for `v <= 255 * 255`.
#[inline(always)]
fn div255_16(v: u16) -> u16 {
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

/// Source-over of one row of premultiplied pixels at full coverage.
///
/// The row loops below are written branch-free on 16-bit lanes so the
/// compiler vectorises them across pixels; per-pixel branches and 32-bit
/// math leave in-order cores (the device's Cortex-A53) one pixel at a time.
/// 16 bits suffice: `div255` of a product of two bytes stays below 2^16.
#[inline(always)]
fn over_row(drow: &mut [u8], srow: &[u8]) {
	// Sprites are mostly fully transparent or fully opaque; whole blocks of
	// those are skipped or copied, which is what the blend would produce.
	const B: usize = 16 * 4;
	let mut dblocks = drow.chunks_exact_mut(B);
	let mut sblocks = srow.chunks_exact(B);
	for (d, s) in (&mut dblocks).zip(&mut sblocks) {
		let (mut all, mut any) = (255u8, 0u8);
		for px in s.chunks_exact(4) {
			all &= px[3];
			any |= px[3];
		}
		if any == 0 {
			continue;
		}
		if all == 255 {
			d.copy_from_slice(s);
		} else {
			over_px(d, s);
		}
	}
	over_px(dblocks.into_remainder(), sblocks.remainder());
}

#[inline(always)]
fn over_px(drow: &mut [u8], srow: &[u8]) {
	for (d, s) in drow.chunks_exact_mut(4).zip(srow.chunks_exact(4)) {
		let inv = 255 - s[3] as u16;
		// A transparent source keeps `d` exactly (`div255(d * 255) == d`),
		// even if its colour bytes are not zero; fully opaque gives `s + 0`.
		// (`on` is arithmetic rather than a compare so the loop vectorises.)
		let on = (s[3] as u16 + 255) >> 8;
		for c in 0..4 {
			d[c] = (s[c] as u16 * on + div255_16(d[c] as u16 * inv)) as u8;
		}
	}
}

/// Source-over of one row scaled by a per-pixel `cover` (0..=255).
#[inline(always)]
fn over_row_cover(drow: &mut [u8], srow: &[u8], cover: &[u8]) {
	for ((d, s), &k) in drow.chunks_exact_mut(4).zip(srow.chunks_exact(4)).zip(cover) {
		let k = k as u16;
		let inv = 255 - div255_16(s[3] as u16 * k);
		// Zero cover or a transparent source leave `d` unchanged.
		let on = (s[3] as u16 + 255) >> 8;
		for c in 0..4 {
			d[c] = (div255_16(s[c] as u16 * on * k) + div255_16(d[c] as u16 * inv)).min(255) as u8;
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
	let mut cover = vec![opacity; if mask.is_some() || opacity != 255 { w } else { 0 }];
	for y in r.y0..r.y1 {
		let so = (y - dy) as usize * src_stride + sx0 * 4;
		let srow = &src[so..so + w * 4];
		let d0 = (y as usize * dst_w + r.x0 as usize) * 4;
		let drow = &mut data[d0..d0 + w * 4];
		match (mask, opacity) {
			(None, 255) => over_row(drow, srow),
			(None, _) => over_row_cover(drow, srow, &cover),
			(Some(m), _) => {
				let mrow = &m[y as usize * dst_w + r.x0 as usize..][..w];
				for (k, &m) in cover.iter_mut().zip(mrow) {
					*k = div255_16(opacity as u16 * m as u16) as u8;
				}
				over_row_cover(drow, srow, &cover);
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

/// Source-over of the solid premultiplied colour `pm` through one row of coverage.
#[inline(always)]
fn over_color_row(drow: &mut [u8], cover: &[u8], pm: [u16; 4]) {
	// Blocks with no coverage (the fringe of a glow) are left alone.
	const B: usize = 16;
	let mut dblocks = drow.chunks_exact_mut(B * 4);
	let mut cblocks = cover.chunks_exact(B);
	for (d, k) in (&mut dblocks).zip(&mut cblocks) {
		if k.iter().fold(0, |a, &k| a | k) != 0 {
			over_color_px(d, k, pm);
		}
	}
	over_color_px(dblocks.into_remainder(), cblocks.remainder(), pm);
}

#[inline(always)]
fn over_color_px(drow: &mut [u8], cover: &[u8], pm: [u16; 4]) {
	for (d, &k) in drow.chunks_exact_mut(4).zip(cover) {
		let k = k as u16;
		// Zero cover leaves `d` unchanged: `0 + div255(d * 255) == d`.
		let inv = 255 - div255_16(pm[3] * k);
		for c in 0..4 {
			d[c] = (div255_16(pm[c] * k) + div255_16(d[c] as u16 * inv)).min(255) as u8;
		}
	}
}

/// Source-over of a solid `color` through an 8-bit coverage buffer: the
/// composite step of a glow, which is one colour times a blurred alpha.
#[cfg(test)]
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
	src_over_color_rows(dst, w, h, dx, dy, color, clip, |row, out| {
		out.copy_from_slice(&alpha[row * stride..][..w as usize]);
	});
}

/// [`src_over_color`] with the coverage produced one row at a time:
/// `fill(row, out)` writes source row `row` (all `w` values) into `out`.
/// Glows stream their upscaled rows straight into the blend this way.
#[allow(clippy::too_many_arguments)]
pub fn src_over_color_rows(
	dst: &mut Pixmap,
	w: u32,
	h: u32,
	dx: i32,
	dy: i32,
	color: Rgba,
	clip: Option<&Mask>,
	mut fill: impl FnMut(usize, &mut [u8]),
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
	let pm = premultiplied(color);
	let (rw, sx0) = (r.width(), (r.x0 - dx) as usize);
	let mut src_row = vec![0u8; w as usize];
	for y in r.y0..r.y1 {
		fill((y - dy) as usize, &mut src_row);
		let cover = &mut src_row[sx0..sx0 + rw];
		let d0 = (y as usize * dst_w + r.x0 as usize) * 4;
		let drow = &mut data[d0..d0 + rw * 4];
		if let Some(m) = mask {
			let mrow = &m[y as usize * dst_w + r.x0 as usize..][..rw];
			for (k, &m) in cover.iter_mut().zip(mrow) {
				*k = div255_16(*k as u16 * m as u16) as u8;
			}
		}
		over_color_row(drow, cover, pm);
	}
}

/// Premultiplied colour channels, alpha last.
fn premultiplied(color: Rgba) -> [u16; 4] {
	let ca = color.a as u16;
	[div255_16(color.r as u16 * ca), div255_16(color.g as u16 * ca), div255_16(color.b as u16 * ca), ca]
}

/// Bilinear upscale of an 8-bit buffer by an integer factor, producing only
/// the `w`×`h` window starting at (`ox`, `oy`) of the upscaled image
/// (sampling pixel centres: `(dst + 0.5) / scale - 0.5`).
#[cfg(test)]
#[allow(clippy::too_many_arguments)]
pub fn upscale_bilinear_a8(src: &[u8], sw: usize, sh: usize, scale: u32, ox: u32, oy: u32, w: u32, h: u32) -> Vec<u8> {
	let mut out = vec![0u8; (w * h) as usize];
	let mut up = Upscaler::new(sw, sh, scale, ox, w);
	for (row, orow) in out.chunks_exact_mut(w as usize).enumerate() {
		up.row(src, oy + row as u32, orow);
	}
	out
}

/// Row-at-a-time form of [`upscale_bilinear_a8`], so a caller can consume
/// each upscaled row while it is still in cache.
///
/// The filter is applied separably: each output row first blends the two
/// source rows it falls between (`v = r0 * (256 - wy) + r1 * wy`), then each
/// output pixel blends two neighbours of `v`. That is the same integer sum as
/// the direct 2×2 form, so the result is bit-identical, with half the
/// multiplies and no 2-D gathers. For scales that divide 128 (the glow's 2
/// and 4), every `scale`-th output column has the same weights, so the
/// horizontal pass runs as a fixed polyphase filter that vectorises.
pub struct Upscaler {
	sw: usize,
	sh: usize,
	scale: i32,
	ox: usize,
	w: usize,
	/// Generic path: per output column, left source column and its weight.
	xs: Vec<(u32, u32)>,
	/// Vertically blended source row with clamped edges: `v[i + 1]` holds
	/// source column `i` clamped to `0..sw`, so `v[0]` repeats column 0 and
	/// the tail repeats the last column.
	v: Vec<u32>,
	/// Source rows and weight `v` currently holds.
	v_key: Option<(usize, usize, u32)>,
	/// Polyphase output covering whole `scale`-wide cells.
	cells: Vec<u8>,
}

impl Upscaler {
	pub fn new(sw: usize, sh: usize, scale: u32, ox: u32, w: u32) -> Self {
		let s = scale as i32;
		let (ox, w) = (ox as usize, w as usize);
		let polyphase = matches!(scale, 2 | 4);
		let xs = if polyphase {
			Vec::new()
		} else {
			(ox as i32..(ox + w) as i32)
				.map(|dx| {
					let fx = upscale_coord(dx, s);
					let x0 = (fx >> 8).clamp(0, sw as i32 - 1);
					let wx = if fx < 0 { 0 } else { (fx & 255) as u32 };
					(x0 as u32, wx)
				})
				.collect()
		};
		// Polyphase reads `v` up to cell index `ceil((ox + w) / s) + 2`.
		let v_len = (sw + 2).max((ox + w).div_ceil(scale as usize) + 3);
		let cells = if polyphase {
			vec![0; ((ox + w).div_ceil(scale as usize) - ox / scale as usize) * scale as usize]
		} else {
			Vec::new()
		};
		Self { sw, sh, scale: s, ox, w, xs, v: vec![0; v_len], v_key: None, cells }
	}

	/// Writes upscaled row `y` (in output coordinates) into `out`.
	pub fn row(&mut self, src: &[u8], y: u32, out: &mut [u8]) {
		let fy = upscale_coord(y as i32, self.scale);
		let y0 = (fy >> 8).clamp(0, self.sh as i32 - 1) as usize;
		let y1 = (y0 + 1).min(self.sh - 1);
		let wy = if fy < 0 { 0 } else { (fy & 255) as u32 };
		if self.v_key != Some((y0, y1, wy)) {
			let sw = self.sw;
			let (r0, r1) = (&src[y0 * sw..][..sw], &src[y1 * sw..][..sw]);
			for ((v, &a), &b) in self.v[1..=sw].iter_mut().zip(r0).zip(r1) {
				*v = a as u32 * (256 - wy) + b as u32 * wy;
			}
			let (first, last) = (self.v[1], self.v[sw]);
			self.v[0] = first;
			self.v[sw + 1..].fill(last);
			self.v_key = Some((y0, y1, wy));
		}
		match self.scale {
			2 => self.polyphase::<2>(out),
			4 => self.polyphase::<4>(out),
			_ => {
				let v = &self.v[..];
				for (o, &(x0, wx)) in out.iter_mut().zip(&self.xs) {
					let x0 = x0 as usize;
					*o = blend_x(v[x0 + 1], v[x0 + 2], wx);
				}
			}
		}
	}

	/// Horizontal pass for a scale `S` dividing 128. Output column
	/// `S * m + p` sits at `256 * m + base(p)` in 8-bit fixed point, so it
	/// blends source columns `m - 1, m` (when `base(p) < 0`) or `m, m + 1`
	/// with weight `base(p) & 255`. Columns outside the source read the
	/// clamped edge copies in `v`, which matches the clamping of the direct
	/// form (both neighbours equal, so the weight drops out).
	#[inline(always)]
	fn polyphase<const S: usize>(&mut self, out: &mut [u8]) {
		let q = 128 / S as i32;
		let mut shift = [0usize; S];
		let mut wx = [0u32; S];
		for p in 0..S {
			let base = (2 * p as i32 + 1) * q - 128;
			// `v[m + shift]` is source column `m - 1 + shift`.
			shift[p] = if base < 0 { 0 } else { 1 };
			wx[p] = (base & 255) as u32;
		}
		let m0 = self.ox / S;
		let n = self.cells.len() / S;
		let v = &self.v[m0..m0 + n + 2];
		for (i, cell) in self.cells.chunks_exact_mut(S).enumerate() {
			let near = [v[i], v[i + 1], v[i + 2]];
			for p in 0..S {
				cell[p] = blend_x(near[shift[p]], near[shift[p] + 1], wx[p]);
			}
		}
		let start = self.ox - m0 * S;
		out.copy_from_slice(&self.cells[start..start + self.w]);
	}
}

/// Final horizontal blend of two vertically blended values.
#[inline(always)]
fn blend_x(a: u32, b: u32, wx: u32) -> u8 {
	((a * (256 - wx) + b * wx + (1 << 15)) >> 16) as u8
}

/// 8-bit fixed point source coordinate of output pixel `d` at scale `s`.
#[inline(always)]
fn upscale_coord(d: i32, s: i32) -> i32 {
	((d * 2 + 1) * 256 / (2 * s)) - 128
}

#[cfg(test)]
mod tests {
	use super::*;
	use tiny_skia::Color;

	/// The original per-pixel implementations, kept as the reference the
	/// vectorised versions must match bit for bit.
	mod reference {
		use super::super::*;

		fn div255(v: u32) -> u32 {
			(v + 128 + ((v + 128) >> 8)) >> 8
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
		pub fn upscale_bilinear_a8(
			src: &[u8],
			sw: usize,
			sh: usize,
			scale: u32,
			ox: u32,
			oy: u32,
			w: u32,
			h: u32,
		) -> Vec<u8> {
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
	}

	/// Small deterministic generator for the equivalence tests.
	struct Lcg(u64);

	impl Lcg {
		fn next(&mut self) -> u32 {
			self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
			(self.0 >> 33) as u32
		}

		/// Bytes biased towards 0 and 255, where the special cases live.
		fn bytes(&mut self, n: usize) -> Vec<u8> {
			(0..n)
				.map(|_| match self.next() % 4 {
					0 => 0,
					1 => 255,
					_ => self.next() as u8,
				})
				.collect()
		}

		fn pixmap(&mut self, w: u32, h: u32) -> Pixmap {
			let mut p = Pixmap::new(w, h).unwrap();
			p.data_mut().copy_from_slice(&self.bytes((w * h * 4) as usize));
			p
		}

		fn mask(&mut self, w: u32, h: u32) -> Mask {
			let mut m = Mask::new(w, h).unwrap();
			m.data_mut().copy_from_slice(&self.bytes((w * h) as usize));
			m
		}
	}

	#[test]
	fn blits_match_reference_bit_for_bit() {
		let mut rng = Lcg(7);
		for case in 0..400 {
			let (dw, dh) = (1 + rng.next() % 80, 1 + rng.next() % 30);
			let (sw, sh) = (1 + rng.next() % 90, 1 + rng.next() % 40);
			let (dx, dy) = ((rng.next() % 60) as i32 - 20, (rng.next() % 50) as i32 - 20);
			let opacity = [255, 0, 1, 128, rng.next() as u8][case % 5];
			let (mut src, mut alpha) = (rng.bytes((sw * sh * 4) as usize), rng.bytes((sw * sh) as usize));
			if case % 2 == 1 {
				// Long transparent / opaque runs, which take the block fast paths.
				for (i, px) in src.chunks_exact_mut(4).enumerate() {
					match (i / 23) % 3 {
						0 => px[3] = 0,
						1 => px[3] = 255,
						_ => {}
					}
				}
				for (i, a) in alpha.iter_mut().enumerate() {
					if (i / 37) % 2 == 0 {
						*a = 0;
					}
				}
			}
			let dst = rng.pixmap(dw, dh);
			let clip = (case % 3 == 0).then(|| rng.mask(dw, dh));
			let color =
				Rgba::new(rng.next() as u8, rng.next() as u8, rng.next() as u8, [255, 1, rng.next() as u8][case % 3]);

			let (mut a, mut b) = (dst.clone(), dst.clone());
			src_over(&mut a, &src, sw as usize * 4, sw, sh, dx, dy, opacity, clip.as_ref());
			reference::src_over(&mut b, &src, sw as usize * 4, sw, sh, dx, dy, opacity, clip.as_ref());
			assert!(a.data() == b.data(), "src_over case {case}");

			let (mut a, mut b) = (dst.clone(), dst.clone());
			src_over_color(&mut a, &alpha, sw as usize, sw, sh, dx, dy, color, clip.as_ref());
			reference::src_over_color(&mut b, &alpha, sw as usize, sw, sh, dx, dy, color, clip.as_ref());
			assert!(a.data() == b.data(), "src_over_color case {case}");
		}
	}

	#[test]
	fn upscale_matches_reference_bit_for_bit() {
		let mut rng = Lcg(11);
		for case in 0..300 {
			let (sw, sh) = (1 + (rng.next() % 30) as usize, 1 + (rng.next() % 30) as usize);
			let scale = [1, 2, 3, 4, 5][case % 5];
			let src = rng.bytes(sw * sh);
			let (fw, fh) = (sw as u32 * scale, sh as u32 * scale);
			let (ox, oy) = (rng.next() % fw, rng.next() % fh);
			let (w, h) = (1 + rng.next() % (fw - ox + 3), 1 + rng.next() % (fh - oy + 3));
			assert_eq!(
				upscale_bilinear_a8(&src, sw, sh, scale, ox, oy, w, h),
				reference::upscale_bilinear_a8(&src, sw, sh, scale, ox, oy, w, h),
				"case {case}"
			);
		}
	}

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
