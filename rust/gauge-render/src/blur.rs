//! Gaussian blur approximation used to emulate Skia's `SKMaskFilter.CreateBlur`.
//!
//! Three successive box blurs approximate a gaussian with the given sigma
//! (standard deviation), which is what Skia's blur mask filter takes.

/// Box widths whose successive application approximates a gaussian of `sigma`.
fn boxes_for_gauss(sigma: f32, n: usize) -> Vec<usize> {
	let w_ideal = ((12.0 * sigma * sigma / n as f32) + 1.0).sqrt();
	let mut wl = w_ideal.floor() as i32;
	if wl % 2 == 0 {
		wl -= 1;
	}
	let wl = wl.max(1);
	let wu = wl + 2;
	let m_ideal =
		(12.0 * sigma * sigma - (n as i32 * wl * wl) as f32 - (4 * n as i32 * wl) as f32 - (3 * n as i32) as f32)
			/ (-4 * wl - 4) as f32;
	let m = m_ideal.round().max(0.0) as usize;
	(0..n).map(|i| if i < m { wl as usize } else { wu as usize }).collect()
}

/// Blurs a premultiplied RGBA8 buffer in place. Pixels outside the buffer are
/// treated as transparent, so callers should pad by roughly `3 * sigma`.
pub fn gaussian_blur_rgba(data: &mut [u8], width: usize, height: usize, sigma: f32) {
	if sigma <= 0.0 || width == 0 || height == 0 {
		return;
	}
	let mut tmp = vec![0u8; data.len()];
	for size in boxes_for_gauss(sigma, 3) {
		let r = (size - 1) / 2;
		if r == 0 {
			continue;
		}
		box_blur(data, &mut tmp, width, height, r, true);
		box_blur(&tmp, data, width, height, r, false);
	}
}

fn box_blur(src: &[u8], dst: &mut [u8], width: usize, height: usize, r: usize, horizontal: bool) {
	let (len, lines, step, line_step) =
		if horizontal { (width, height, 4, width * 4) } else { (height, width, width * 4, 4) };
	let inv = (1u64 << 24) / (2 * r as u64 + 1);

	for line in 0..lines {
		let base = line * line_step;
		let mut sum = [0u32; 4];
		for i in 0..=r.min(len - 1) {
			let o = base + i * step;
			for c in 0..4 {
				sum[c] += src[o + c] as u32;
			}
		}
		for i in 0..len {
			let o = base + i * step;
			for c in 0..4 {
				dst[o + c] = ((sum[c] as u64 * inv + (1 << 23)) >> 24).min(255) as u8;
			}
			let add = i + r + 1;
			if add < len {
				let ao = base + add * step;
				for c in 0..4 {
					sum[c] += src[ao + c] as u32;
				}
			}
			if i >= r {
				let so = base + (i - r) * step;
				for c in 0..4 {
					sum[c] -= src[so + c] as u32;
				}
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn blur_spreads_and_roughly_preserves_energy() {
		let (w, h) = (41, 41);
		let mut buf = vec![0u8; w * h * 4];
		for y in 18..23 {
			for x in 18..23 {
				let o = (y * w + x) * 4;
				buf[o..o + 4].copy_from_slice(&[255, 255, 255, 255]);
			}
		}
		let before: u32 = buf.iter().skip(3).step_by(4).map(|&a| a as u32).sum();
		gaussian_blur_rgba(&mut buf, w, h, 3.0);
		let after: u32 = buf.iter().skip(3).step_by(4).map(|&a| a as u32).sum();
		assert!(buf[(20 * w + 12) * 4 + 3] > 0, "blur should spread outwards");
		let diff = (before as i64 - after as i64).abs() as f64 / before as f64;
		assert!(diff < 0.05, "energy drift {diff}");
	}
}
