//! Damage-tracked, optionally multithreaded rendering of custom gauges.
//!
//! Each frame only the regions whose content changed are repainted:
//! 1. every dynamic element gets a fingerprint (value, clock text, peak,
//!    graph history) from [`ElementRenderer::element_key`];
//! 2. for elements whose fingerprint changed, a bounds-only pass
//!    ([`Canvas::recorder`]) finds where they draw now, and the damage is
//!    their old bounds plus their new bounds;
//! 3. each damage rect is re-rendered from scratch (static sprites and every
//!    element that overlaps it, in order) into a small tile canvas, and
//!    pasted into the frame. Large rects are split into horizontal bands,
//!    and all tiles are shared out over worker threads.
//!
//! The result matches a full render because each tile replays the complete
//! draw list for its area.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use crate::canvas::Canvas;
use crate::composite::IRect;
use crate::custom::renderer::{partial_damage, Painter, Segment, StaticCache};
use crate::custom::ElementRenderer;
use crate::model::CustomGaugeDefinition;
use crate::Values;

/// Regions of the frame that changed in one draw.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Damage {
	pub rects: Vec<IRect>,
}

impl Damage {
	pub fn full(width: u32, height: u32) -> Self {
		Self { rects: vec![IRect { x0: 0, y0: 0, x1: width as i32, y1: height as i32 }] }
	}

	pub fn is_empty(&self) -> bool {
		self.rects.is_empty()
	}

	/// Total repainted pixels.
	pub fn area(&self) -> usize {
		self.rects.iter().map(|r| r.area()).sum()
	}
}

/// Smallest area worth splitting across threads (a band per ~32k pixels).
const MIN_PIXELS_PER_THREAD: usize = 32 * 1024;
/// More damage rects than this are merged down (each costs a tile setup).
const MAX_RECTS: usize = 8;

/// What [`ElementRenderer::render_incremental`] remembers between frames.
pub struct FrameState {
	cache: StaticCache,
	keys: Vec<Option<u64>>,
	bounds: Vec<Option<IRect>>,
	values: Vec<Option<f32>>,
	valid: bool,
	size: (u32, u32),
	/// Areas the caller drew over since the last frame (overlays), repainted next frame.
	extra_damage: Vec<IRect>,
	/// Worker threads for large repaints (1 = render on the calling thread).
	pub threads: usize,
}

impl Default for FrameState {
	fn default() -> Self {
		Self {
			cache: StaticCache::new(),
			keys: Vec::new(),
			bounds: Vec::new(),
			values: Vec::new(),
			valid: false,
			size: (0, 0),
			extra_damage: Vec::new(),
			threads: default_threads(),
		}
	}
}

/// Available cores, capped at 4 (the RG35XX has four).
pub fn default_threads() -> usize {
	std::thread::available_parallelism().map(|n| n.get().min(4)).unwrap_or(1)
}

impl FrameState {
	pub fn new() -> Self {
		Self::default()
	}

	/// Forces the next frame to repaint everything, e.g. after something else
	/// drew on the canvas (switching screens) or the definition changed.
	pub fn invalidate(&mut self) {
		self.valid = false;
	}

	/// Marks an area to repaint on the next frame, e.g. where an overlay
	/// (FPS counter, battery icon) was drawn over the gauge.
	pub fn add_damage(&mut self, r: IRect) {
		self.extra_damage.push(r);
	}

	/// Also drops the pre-rendered static runs (after editing the definition).
	pub fn invalidate_all(&mut self) {
		self.valid = false;
		self.cache.invalidate();
	}
}

/// Merges damage rects where that wastes little area: a pair is merged
/// when its bounding box costs at most 25 % more pixels than the two rects
/// (always true for heavily overlapping rects), cheapest pair first. Past
/// `MAX_RECTS`, the cheapest pairs are merged regardless.
fn merge(mut rects: Vec<IRect>) -> Vec<IRect> {
	loop {
		let mut best: Option<(i64, usize, usize)> = None;
		for i in 0..rects.len() {
			for j in i + 1..rects.len() {
				let (a, b) = (rects[i], rects[j]);
				let separate = a.area() as i64 + b.area() as i64;
				let overlap = a.intersect(b).map_or(0, |o| o.area() as i64);
				let waste = a.union(b).area() as i64 - (separate - overlap);
				if best.map_or(true, |(w, _, _)| waste < w) {
					best = Some((waste, i, j));
				}
			}
		}
		let Some((waste, i, j)) = best else {
			return rects;
		};
		let separate = (rects[i].area() + rects[j].area()) as i64;
		if waste * 4 > separate && rects.len() <= MAX_RECTS {
			return rects;
		}
		rects[i] = rects[i].union(rects[j]);
		rects.swap_remove(j);
	}
}

/// Renders the full draw list clipped to `tile` into a tile-sized pixmap.
fn render_tile(
	painter: &Painter,
	segments: &[Segment],
	resolved: &[Option<f32>],
	bounds: &[Option<IRect>],
	tile: IRect,
) -> tiny_skia::Pixmap {
	let mut c = Canvas::new(tile.width() as u32, tile.height() as u32);
	c.set_origin(tile.x0, tile.y0);
	c.translate(-tile.x0 as f32, -tile.y0 as f32);
	let elements = &painter.definition().elements;
	for segment in segments {
		match segment {
			Segment::Static { sprite: Some(sprite), .. } => c.draw_sprite(sprite),
			Segment::Static { sprite: None, .. } => {}
			Segment::Dynamic(i) => {
				let overlaps = bounds[*i].is_some_and(|b| b.intersect(tile).is_some());
				if let (true, Some(v)) = (overlaps, resolved[*i]) {
					painter.draw_element(&mut c, &elements[*i], v);
				}
			}
		}
	}
	c.into_pixmap()
}

fn split_bands(r: IRect, threads: usize) -> Vec<IRect> {
	let n = threads.min(r.area() / MIN_PIXELS_PER_THREAD).min(r.height() / 16).max(1);
	let h = r.height().div_ceil(n) as i32;
	(0..n as i32)
		.map(|k| IRect { y0: r.y0 + k * h, y1: (r.y0 + (k + 1) * h).min(r.y1), ..r })
		.filter(|b| b.y0 < b.y1)
		.collect()
}

impl ElementRenderer {
	/// Like [`ElementRenderer::render_cached`], but repaints only what changed
	/// since the previous call with the same `state`, and returns the damaged
	/// regions (e.g. to upload only those to a texture).
	///
	/// The canvas must be the same one passed last time, untouched in
	/// between (call [`FrameState::invalidate`] otherwise), with an identity
	/// transform.
	pub fn render_incremental(
		&mut self,
		canvas: &mut Canvas,
		definition: &CustomGaugeDefinition,
		values: &Values,
		target_values: Option<&Values>,
		state: &mut FrameState,
	) -> Damage {
		let size = (canvas.width(), canvas.height());
		let resolved = self.prepare(definition, values, target_values);
		let n = definition.elements.len();
		let full = !state.valid || state.size != size || state.keys.len() != n;
		if full {
			state.keys = vec![None; n];
			state.bounds = vec![None; n];
			state.values = vec![None; n];
			state.size = size;
		}

		let painter = self.painter(definition);
		state.cache.ensure(&painter, size);

		let mut damage = Vec::new();
		for segment in &state.cache.segments {
			let Segment::Dynamic(i) = *segment else {
				continue;
			};
			let element = &definition.elements[i];
			let key = self.element_key(element, resolved[i]);
			if state.keys[i] == Some(key) {
				continue;
			}
			let new_bounds = resolved[i].and_then(|v| {
				let mut rec = Canvas::recorder(size.0, size.1);
				painter.draw_element(&mut rec, element, v);
				rec.recorded().and_then(|r| r.clamp_to(size.0, size.1))
			});
			// Arcs and bars that only moved their fill damage just the
			// changed strip; anything else damages its old and new bounds.
			let partial = match (state.values[i], resolved[i]) {
				(Some(old), Some(new)) if !full => partial_damage(element, old, new, size),
				_ => None,
			};
			match partial {
				Some(strip) => damage.extend(strip),
				None => {
					damage.extend(state.bounds[i]);
					damage.extend(new_bounds);
				}
			}
			state.keys[i] = Some(key);
			state.bounds[i] = new_bounds;
			state.values[i] = resolved[i];
		}

		damage.append(&mut state.extra_damage);
		let rects = if full {
			Damage::full(size.0, size.1).rects
		} else {
			merge(damage.into_iter().filter_map(|r| r.clamp_to(size.0, size.1)).collect())
		};
		let segments = &state.cache.segments;
		let bounds = &state.bounds;
		let threads = state.threads.max(1);
		// All tiles of all damage rects form one work queue for the threads.
		let tiles: Vec<IRect> = rects.iter().flat_map(|&r| split_bands(r, threads)).collect();
		if threads == 1 || tiles.len() == 1 {
			for &t in &tiles {
				let tile = render_tile(&painter, segments, &resolved, bounds, t);
				canvas.replace_pixels(&tile, t.x0, t.y0);
			}
		} else {
			let next = AtomicUsize::new(0);
			let done: Vec<Mutex<Option<tiny_skia::Pixmap>>> = tiles.iter().map(|_| Mutex::new(None)).collect();
			std::thread::scope(|s| {
				for _ in 0..threads.min(tiles.len()) {
					s.spawn(|| loop {
						let k = next.fetch_add(1, Ordering::Relaxed);
						let Some(&t) = tiles.get(k) else {
							break;
						};
						let tile = render_tile(&painter, segments, &resolved, bounds, t);
						*done[k].lock().expect("tile slot") = Some(tile);
					});
				}
			});
			for (t, slot) in tiles.iter().zip(done) {
				if let Some(tile) = slot.into_inner().expect("tile slot") {
					canvas.replace_pixels(&tile, t.x0, t.y0);
				}
			}
		}
		state.valid = true;
		Damage { rects }
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn merges_only_when_cheap() {
		let a = IRect { x0: 0, y0: 0, x1: 10, y1: 10 };
		let b = IRect { x0: 11, y0: 0, x1: 20, y1: 10 };
		let far = IRect { x0: 100, y0: 100, x1: 110, y1: 110 };
		let m = merge(vec![a, far, b]);
		assert_eq!(m.len(), 2, "{m:?}");
		assert!(m.contains(&IRect { x0: 0, y0: 0, x1: 20, y1: 10 }));
		// Two diagonal rects would waste half the union: kept apart.
		let c = IRect { x0: 0, y0: 0, x1: 50, y1: 50 };
		let d = IRect { x0: 50, y0: 50, x1: 100, y1: 100 };
		assert_eq!(merge(vec![c, d]).len(), 2);
		// Overlapping duplicates always merge.
		assert_eq!(merge(vec![c, c]), vec![c]);
	}

	#[test]
	fn bands_cover_rect() {
		let r = IRect { x0: 5, y0: 7, x1: 605, y1: 407 };
		let bands = split_bands(r, 4);
		assert_eq!(bands.len(), 4);
		assert_eq!(bands.first().unwrap().y0, 7);
		assert_eq!(bands.last().unwrap().y1, 407);
		assert!(bands.windows(2).all(|w| w[0].y1 == w[1].y0));
		assert_eq!(split_bands(IRect { x0: 0, y0: 0, x1: 20, y1: 20 }, 4).len(), 1);
	}
}
