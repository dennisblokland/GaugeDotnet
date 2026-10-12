//! A small `SKCanvas`-like immediate-mode canvas on top of tiny-skia.
//!
//! It supplies the pieces the C# renderer relies on that tiny-skia lacks out of
//! the box: a save/restore stack with transforms and clips, opacity layers
//! (`SaveLayer`), arcs, round rects, text and blur mask filters (glows).

use std::rc::Rc;

use tiny_skia::{
	BlendMode, FillRule, FilterQuality, Mask, Path, PathBuilder, PathStroker, Pattern, Pixmap, PixmapPaint, Rect,
	SpreadMode, Stroke, Transform,
};

pub use tiny_skia::{LineCap, LineJoin};

use crate::blur::gaussian_blur_a8;
use crate::color::Rgba;
use crate::composite::{copy_rows, opaque_bounds, src_over, src_over_color, upscale_bilinear_a8, IRect};
use crate::text::{TextAlign, Typeface};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaintStyle {
	Fill,
	Stroke,
}

/// Mirrors `SKBlurStyle` for the two styles the gauges use.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlurStyle {
	/// Blur inside and outside the shape.
	Normal,
	/// Blur only outside the shape (used by the segment display glow).
	Outer,
}

/// Equivalent of `SKMaskFilter.CreateBlur(style, sigma)`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Blur {
	pub sigma: f32,
	pub style: BlurStyle,
}

impl Blur {
	pub const fn normal(sigma: f32) -> Self {
		Self { sigma, style: BlurStyle::Normal }
	}

	pub const fn outer(sigma: f32) -> Self {
		Self { sigma, style: BlurStyle::Outer }
	}
}

/// Equivalent of the subset of `SKPaint` used by the gauges.
#[derive(Clone, Copy, Debug)]
pub struct Paint {
	pub color: Rgba,
	pub style: PaintStyle,
	pub stroke_width: f32,
	pub cap: LineCap,
	pub join: LineJoin,
	pub blur: Option<Blur>,
	pub anti_alias: bool,
}

impl Default for Paint {
	fn default() -> Self {
		Self {
			color: Rgba::WHITE,
			style: PaintStyle::Fill,
			stroke_width: 1.0,
			cap: LineCap::Butt,
			join: LineJoin::Miter,
			blur: None,
			anti_alias: true,
		}
	}
}

impl Paint {
	pub fn fill(color: Rgba) -> Self {
		Self { color, ..Self::default() }
	}

	pub fn stroke(color: Rgba, width: f32) -> Self {
		Self { color, style: PaintStyle::Stroke, stroke_width: width, ..Self::default() }
	}

	pub fn with_cap(mut self, cap: LineCap) -> Self {
		self.cap = cap;
		self
	}

	pub fn with_join(mut self, join: LineJoin) -> Self {
		self.join = join;
		self
	}

	pub fn with_blur(mut self, blur: Option<Blur>) -> Self {
		self.blur = blur;
		self
	}

	pub fn with_color(mut self, color: Rgba) -> Self {
		self.color = color;
		self
	}

	fn to_skia(self) -> tiny_skia::Paint<'static> {
		let mut p = tiny_skia::Paint::default();
		p.set_color(self.color.to_skia());
		p.anti_alias = self.anti_alias;
		p
	}

	fn to_stroke(self) -> Stroke {
		Stroke { width: self.stroke_width, line_cap: self.cap, line_join: self.join, ..Stroke::default() }
	}
}

#[derive(Clone)]
struct State {
	transform: Transform,
	clip: Option<Rc<Mask>>,
	pushes_layer: bool,
}

struct Layer {
	pixmap: Pixmap,
	opacity: u8,
	/// Device area drawn into, so restore only composites (and clears) that.
	dirty: Option<IRect>,
}

pub struct Canvas {
	base: Pixmap,
	layers: Vec<Layer>,
	/// Transparent full-size pixmaps recycled between `save_layer` calls.
	layer_pool: Vec<Pixmap>,
	state: State,
	stack: Vec<State>,
	/// Logical size (a recording canvas has no real pixels).
	size: (u32, u32),
	/// Position of this canvas's pixel (0, 0) on the full frame. Used to keep
	/// downscaled blur grids aligned when a frame is rendered in tiles.
	origin: (i32, i32),
	/// Bounds-only mode: draw calls just accumulate `recorded`.
	recording: bool,
	recorded: Option<IRect>,
}

/// Integer offset when `ts` is a pure whole-pixel translation.
fn integer_translation(ts: &Transform) -> Option<(i32, i32)> {
	let whole = |v: f32| (v.fract() == 0.0).then_some(v as i32);
	if ts.sx == 1.0 && ts.sy == 1.0 && ts.kx == 0.0 && ts.ky == 0.0 {
		Some((whole(ts.tx)?, whole(ts.ty)?))
	} else {
		None
	}
}

/// Maximum distance (device px) between a curve and its flattened polyline.
const FLATTEN_TOLERANCE: f32 = 0.05;

/// Transforms `path` to device space and replaces curves with line segments
/// (Wang's formula). Rasterising lines is invariant to how tiny-skia clips a
/// path at the canvas edge, whereas clipped curves are re-split and land on
/// slightly different samples. Flattening up front therefore makes a tile of
/// a frame render exactly like the same area of a full-frame render.
fn flatten_device(path: &Path, ts: Transform) -> Option<Path> {
	use tiny_skia::PathSegment;
	let path = path.clone().transform(ts)?;
	let mut pb = PathBuilder::with_capacity(path.len() * 2, path.len() * 2);
	let mut last = tiny_skia::Point::zero();
	let steps = |d: f32, factor: f32| ((factor * d / FLATTEN_TOLERANCE).sqrt().ceil() as usize).clamp(1, 256);
	for seg in path.segments() {
		match seg {
			PathSegment::MoveTo(p) => {
				pb.move_to(p.x, p.y);
				last = p;
			}
			PathSegment::LineTo(p) => {
				pb.line_to(p.x, p.y);
				last = p;
			}
			PathSegment::QuadTo(c, p) => {
				let d = ((last.x - 2.0 * c.x + p.x).powi(2) + (last.y - 2.0 * c.y + p.y).powi(2)).sqrt();
				let n = steps(d, 0.25);
				for i in 1..=n {
					let t = i as f32 / n as f32;
					let mt = 1.0 - t;
					pb.line_to(
						mt * mt * last.x + 2.0 * mt * t * c.x + t * t * p.x,
						mt * mt * last.y + 2.0 * mt * t * c.y + t * t * p.y,
					);
				}
				last = p;
			}
			PathSegment::CubicTo(c1, c2, p) => {
				let dd = |a: f32, b: f32, c: f32| a - 2.0 * b + c;
				let d1 = dd(last.x, c1.x, c2.x).hypot(dd(last.y, c1.y, c2.y));
				let d2 = dd(c1.x, c2.x, p.x).hypot(dd(c1.y, c2.y, p.y));
				let n = steps(d1.max(d2), 0.75);
				for i in 1..=n {
					let t = i as f32 / n as f32;
					let mt = 1.0 - t;
					let (a, b, c, e) = (mt * mt * mt, 3.0 * mt * mt * t, 3.0 * mt * t * t, t * t * t);
					pb.line_to(a * last.x + b * c1.x + c * c2.x + e * p.x, a * last.y + b * c1.y + c * c2.y + e * p.y);
				}
				last = p;
			}
			PathSegment::Close => pb.close(),
		}
	}
	pb.finish()
}

/// Sorted rect from left/top/right/bottom (Skia sorts rects before drawing).
pub fn rect(l: f32, t: f32, r: f32, b: f32) -> Option<Rect> {
	Rect::from_ltrb(l.min(r), t.min(b), l.max(r), t.max(b))
}

/// Arcs are built from cubic segments at fixed angular steps from the start
/// angle, so lengthening an arc leaves its existing segments bit-identical
/// (incremental redraw relies on this: only the changed wedge repaints).
pub const ARC_SEGMENT_DEG: f32 = 15.0;

/// Cubic-bezier approximation of an oval arc, angles in degrees, clockwise
/// from +x in y-down space (Skia's `DrawArc` convention, `useCenter = false`).
pub fn arc_path(oval: Rect, start_deg: f32, sweep_deg: f32) -> Option<Path> {
	if sweep_deg == 0.0 || !sweep_deg.is_finite() {
		return None;
	}
	if sweep_deg.abs() >= 360.0 {
		return PathBuilder::from_oval(oval);
	}
	let cx = (oval.left() + oval.right()) / 2.0;
	let cy = (oval.top() + oval.bottom()) / 2.0;
	let rx = oval.width() / 2.0;
	let ry = oval.height() / 2.0;
	let sign = sweep_deg.signum();
	let total = sweep_deg.abs();

	let point = |deg: f32| {
		let a = deg.to_radians();
		(cx + rx * a.cos(), cy + ry * a.sin())
	};
	let mut pb = PathBuilder::new();
	let (sx, sy) = point(start_deg);
	pb.move_to(sx, sy);
	let mut done = 0.0;
	while done < total {
		let step = ARC_SEGMENT_DEG.min(total - done);
		let a0 = (start_deg + sign * done).to_radians();
		let a1 = (start_deg + sign * (done + step)).to_radians();
		let k = 4.0 / 3.0 * ((a1 - a0) / 4.0).tan();
		let (s0, c0) = a0.sin_cos();
		let (s1, c1) = a1.sin_cos();
		pb.cubic_to(
			cx + rx * (c0 - k * s0),
			cy + ry * (s0 + k * c0),
			cx + rx * (c1 + k * s1),
			cy + ry * (s1 - k * c1),
			cx + rx * c1,
			cy + ry * s1,
		);
		done += step;
	}
	pb.finish()
}

/// Rounded rectangle path (`SKCanvas.DrawRoundRect` / `SKPath.AddRoundRect`).
pub fn round_rect_path(r: Rect, rx: f32, ry: f32) -> Option<Path> {
	let rx = rx.min(r.width() / 2.0);
	let ry = ry.min(r.height() / 2.0);
	if rx <= 0.0 || ry <= 0.0 {
		return Some(PathBuilder::from_rect(r));
	}
	const K: f32 = 0.552_284_8;
	let (l, t, rr, b) = (r.left(), r.top(), r.right(), r.bottom());
	let mut pb = PathBuilder::new();
	pb.move_to(l + rx, t);
	pb.line_to(rr - rx, t);
	pb.cubic_to(rr - rx + rx * K, t, rr, t + ry - ry * K, rr, t + ry);
	pb.line_to(rr, b - ry);
	pb.cubic_to(rr, b - ry + ry * K, rr - rx + rx * K, b, rr - rx, b);
	pb.line_to(l + rx, b);
	pb.cubic_to(l + rx - rx * K, b, l, b - ry + ry * K, l, b - ry);
	pb.line_to(l, t + ry);
	pb.cubic_to(l, t + ry - ry * K, l + rx - rx * K, t, l + rx, t);
	pb.close();
	pb.finish()
}

/// A pre-rendered layer trimmed to its visible pixels, for static caches.
/// Blitting a sprite touches only its content, and fully opaque sprites are
/// copied row by row with no blending.
pub struct Sprite {
	pixmap: Option<Pixmap>,
	x: i32,
	y: i32,
	opaque: bool,
}

impl Sprite {
	/// Trims `pixmap` (positioned at the origin) to its non-transparent area.
	pub fn from_pixmap(pixmap: Pixmap) -> Self {
		let Some(b) = opaque_bounds(&pixmap) else {
			return Self { pixmap: None, x: 0, y: 0, opaque: false };
		};
		let full = b.x0 == 0 && b.y0 == 0 && b.x1 == pixmap.width() as i32 && b.y1 == pixmap.height() as i32;
		let cropped = if full {
			pixmap
		} else {
			let r = tiny_skia::IntRect::from_xywh(b.x0, b.y0, b.width() as u32, b.height() as u32).expect("non-empty");
			pixmap.clone_rect(r).expect("bounds inside pixmap")
		};
		let opaque = cropped.data().chunks_exact(4).all(|px| px[3] == 255);
		Self { pixmap: Some(cropped), x: b.x0, y: b.y0, opaque }
	}

	/// Renders `draw` (in canvas coordinates) into just the device region
	/// `x, y, width, height`. Cheaper than [`Sprite::render`]
	/// for caches that are rebuilt often.
	pub fn render_in(x: i32, y: i32, width: u32, height: u32, draw: impl FnOnce(&mut Canvas)) -> Self {
		let mut c = Canvas::new(width.max(1), height.max(1));
		c.set_origin(x, y);
		c.translate(-x as f32, -y as f32);
		draw(&mut c);
		// The region is already tight, so skip trimming (it is rebuilt often).
		Self { pixmap: Some(c.into_pixmap()), x, y, opaque: false }
	}

	/// Device rect covered by the sprite (`None` if fully transparent).
	pub fn bounds(&self) -> Option<IRect> {
		let p = self.pixmap.as_ref()?;
		Some(IRect { x0: self.x, y0: self.y, x1: self.x + p.width() as i32, y1: self.y + p.height() as i32 })
	}

	/// Renders `draw` onto a transparent `width`×`height` canvas and trims it.
	pub fn render(width: u32, height: u32, draw: impl FnOnce(&mut Canvas)) -> Self {
		let mut c = Canvas::new(width, height);
		draw(&mut c);
		Self::from_pixmap(c.into_pixmap())
	}
}

impl Canvas {
	/// Creates a transparent canvas. Panics if a dimension is zero.
	pub fn new(width: u32, height: u32) -> Self {
		Self::from_pixmap(Pixmap::new(width, height).expect("canvas size must be non-zero"))
	}

	pub fn from_pixmap(pixmap: Pixmap) -> Self {
		Self {
			size: (pixmap.width(), pixmap.height()),
			base: pixmap,
			layers: Vec::new(),
			layer_pool: Vec::new(),
			state: State { transform: Transform::identity(), clip: None, pushes_layer: false },
			stack: Vec::new(),
			origin: (0, 0),
			recording: false,
			recorded: None,
		}
	}

	/// A canvas that rasterises nothing and only records the device bounds of
	/// what would be drawn (see [`Canvas::recorded`]). Bounds are conservative:
	/// clips are ignored and strokes/glows are padded.
	pub fn recorder(width: u32, height: u32) -> Self {
		let mut c = Self::from_pixmap(Pixmap::new(1, 1).expect("1x1"));
		c.size = (width, height);
		c.recording = true;
		c
	}

	/// Device bounds accumulated by a [`Canvas::recorder`].
	pub fn recorded(&self) -> Option<IRect> {
		self.recorded
	}

	/// Declares where this canvas's pixel (0, 0) sits on the full frame, for
	/// canvases that render one tile of a larger frame.
	pub fn set_origin(&mut self, x: i32, y: i32) {
		self.origin = (x, y);
	}

	pub fn width(&self) -> u32 {
		self.size.0
	}

	pub fn height(&self) -> u32 {
		self.size.1
	}

	/// The finished frame, writable (for compositors that assemble frames
	/// from cached layers).
	pub fn pixmap_mut(&mut self) -> &mut Pixmap {
		&mut self.base
	}

	/// Replaces the pixels at (`x`, `y`) with `src` (no blending, no
	/// transform), e.g. to paste a separately rendered tile.
	pub fn replace_pixels(&mut self, src: &Pixmap, x: i32, y: i32) {
		copy_rows(&mut self.base, src.data(), src.width() as usize * 4, src.width(), src.height(), x, y);
	}

	/// The finished frame (premultiplied RGBA8). Unbalanced layers are not included.
	pub fn pixmap(&self) -> &Pixmap {
		&self.base
	}

	pub fn into_pixmap(mut self) -> Pixmap {
		self.restore_to_count(0);
		self.base
	}

	pub fn save_png(&self, path: impl AsRef<std::path::Path>) -> Result<(), String> {
		self.base.save_png(path).map_err(|e| e.to_string())
	}

	fn parts(&mut self) -> (&mut Pixmap, &State) {
		let target = match self.layers.last_mut() {
			Some(layer) => &mut layer.pixmap,
			None => &mut self.base,
		};
		(target, &self.state)
	}

	// ---- state stack -------------------------------------------------------

	pub fn save(&mut self) {
		self.stack.push(self.state.clone());
		self.state.pushes_layer = false;
	}

	/// `SaveLayer` with a paint of the given alpha: everything until the
	/// matching `restore` is composited at `opacity`.
	pub fn save_layer(&mut self, opacity: u8) {
		self.save();
		if self.recording {
			return;
		}
		let pixmap = match self.layer_pool.pop() {
			Some(p) => p,
			None => Pixmap::new(self.width(), self.height()).expect("non-zero size"),
		};
		self.layers.push(Layer { pixmap, opacity, dirty: None });
		self.state.pushes_layer = true;
	}

	pub fn restore(&mut self) {
		let Some(prev) = self.stack.pop() else {
			return;
		};
		if self.state.pushes_layer {
			if let Some(mut layer) = self.layers.pop() {
				let (w, h) = (self.width(), self.height());
				if let Some(d) = layer.dirty.and_then(|d| d.clamp_to(w, h)) {
					let stride = w as usize * 4;
					let start = d.y0 as usize * stride + d.x0 as usize * 4;
					let (dw, dh) = ((d.x1 - d.x0) as u32, (d.y1 - d.y0) as u32);
					let target = match self.layers.last_mut() {
						Some(l) => &mut l.pixmap,
						None => &mut self.base,
					};
					src_over(target, &layer.pixmap.data()[start..], stride, dw, dh, d.x0, d.y0, layer.opacity, None);
					self.mark_dirty(d);
					// Leave the pooled pixmap transparent again.
					let data = layer.pixmap.data_mut();
					for y in d.y0..d.y1 {
						let row = y as usize * stride;
						data[row + d.x0 as usize * 4..row + d.x1 as usize * 4].fill(0);
					}
				}
				self.layer_pool.push(layer.pixmap);
			}
		}
		self.state = prev;
	}

	/// Records that device area `r` of the current layer was drawn into.
	fn mark_dirty(&mut self, r: IRect) {
		if self.recording {
			self.recorded = Some(self.recorded.map_or(r, |d| d.union(r)));
		}
		if let Some(layer) = self.layers.last_mut() {
			layer.dirty = Some(layer.dirty.map_or(r, |d| d.union(r)));
		}
	}

	/// Marks the device bounds of a user-space rect, grown by `pad`.
	fn mark_dirty_user(&mut self, l: f32, t: f32, r: f32, b: f32, pad: f32) {
		if self.layers.is_empty() && !self.recording {
			return;
		}
		let mut pts = [
			tiny_skia::Point::from_xy(l - pad, t - pad),
			tiny_skia::Point::from_xy(r + pad, t - pad),
			tiny_skia::Point::from_xy(r + pad, b + pad),
			tiny_skia::Point::from_xy(l - pad, b + pad),
		];
		self.state.transform.map_points(&mut pts);
		let (mut x0, mut y0, mut x1, mut y1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
		for p in pts {
			x0 = x0.min(p.x);
			y0 = y0.min(p.y);
			x1 = x1.max(p.x);
			y1 = y1.max(p.y);
		}
		self.mark_dirty(IRect::from_bounds(x0 - 1.0, y0 - 1.0, x1 + 1.0, y1 + 1.0));
	}

	pub fn save_count(&self) -> usize {
		self.stack.len()
	}

	pub fn restore_to_count(&mut self, count: usize) {
		while self.stack.len() > count {
			self.restore();
		}
	}

	pub fn translate(&mut self, dx: f32, dy: f32) {
		self.state.transform = self.state.transform.pre_translate(dx, dy);
	}

	pub fn rotate_degrees(&mut self, degrees: f32) {
		self.state.transform = self.state.transform.pre_concat(Transform::from_rotate(degrees));
	}

	pub fn scale(&mut self, sx: f32, sy: f32) {
		self.state.transform = self.state.transform.pre_scale(sx, sy);
	}

	/// `SKCanvas.Skew`: `x' = x + kx * y`, `y' = y + ky * x`.
	pub fn skew(&mut self, kx: f32, ky: f32) {
		self.state.transform = self.state.transform.pre_concat(Transform::from_skew(kx, ky));
	}

	pub fn transform(&self) -> Transform {
		self.state.transform
	}

	pub fn clip_rect(&mut self, r: Rect) {
		self.clip_path(&PathBuilder::from_rect(r));
	}

	pub fn clip_path(&mut self, path: &Path) {
		if self.recording {
			return;
		}
		let ts = self.state.transform;
		let mask = match &self.state.clip {
			Some(existing) => {
				let mut m = (**existing).clone();
				m.intersect_path(path, FillRule::Winding, true, ts);
				m
			}
			None => {
				let mut m = Mask::new(self.width(), self.height()).expect("non-zero size");
				m.fill_path(path, FillRule::Winding, true, ts);
				m
			}
		};
		self.state.clip = Some(Rc::new(mask));
	}

	// ---- drawing -----------------------------------------------------------

	/// Fills the current clip with `color`, replacing what is there (`SKCanvas.Clear`).
	pub fn clear(&mut self, color: Rgba) {
		let (w, h) = (self.width() as i32, self.height() as i32);
		self.mark_dirty(IRect { x0: 0, y0: 0, x1: w, y1: h });
		if self.recording {
			return;
		}
		let (target, state) = self.parts();
		match &state.clip {
			None => target.fill(color.to_skia()),
			Some(mask) => {
				let mut p = tiny_skia::Paint::default();
				p.set_color(color.to_skia());
				p.blend_mode = BlendMode::Source;
				let full = Rect::from_xywh(0.0, 0.0, target.width() as f32, target.height() as f32).unwrap();
				target.fill_rect(full, &p, Transform::identity(), Some(mask));
			}
		}
	}

	pub fn draw_path(&mut self, path: &Path, paint: &Paint) {
		if let Some(blur) = paint.blur.filter(|b| b.sigma > 0.0) {
			self.draw_blurred(path, paint, blur);
			return;
		}
		if !self.layers.is_empty() || self.recording {
			let b = path.bounds();
			let pad = match paint.style {
				PaintStyle::Fill => 0.0,
				// Miter joins can poke out further than half the width.
				PaintStyle::Stroke => paint.stroke_width * 2.0,
			};
			self.mark_dirty_user(b.left(), b.top(), b.right(), b.bottom(), pad);
			if self.recording {
				return;
			}
		}
		let ts = self.state.transform;
		// Strokes become filled outlines first (hairlines excepted), so every
		// shape is filled from a flattened device-space path.
		let outline = match paint.style {
			PaintStyle::Stroke if paint.stroke_width > 0.0 => {
				path.stroke(&paint.to_stroke(), PathStroker::compute_resolution_scale(&ts))
			}
			_ => Some(path.clone()),
		};
		let (target, state) = self.parts();
		let clip = state.clip.as_deref();
		let sp = paint.to_skia();
		match (paint.style, outline.and_then(|o| flatten_device(&o, ts))) {
			(PaintStyle::Stroke, _) if paint.stroke_width <= 0.0 => {
				target.stroke_path(path, &sp, &paint.to_stroke(), ts, clip)
			}
			(_, Some(device)) => target.fill_path(&device, &sp, FillRule::Winding, Transform::identity(), clip),
			_ => {}
		}
	}

	/// Renders the shape into an offscreen buffer, blurs it and composites the
	/// result: the CPU equivalent of drawing with a blur `SKMaskFilter`.
	fn draw_blurred(&mut self, path: &Path, paint: &Paint, blur: Blur) {
		let ts = self.state.transform;
		let outline = match paint.style {
			PaintStyle::Fill => Some(path.clone()),
			PaintStyle::Stroke => {
				let res_scale = PathStroker::compute_resolution_scale(&ts);
				path.stroke(&paint.to_stroke(), res_scale)
			}
		};
		let Some(device_path) = outline.and_then(|p| flatten_device(&p, ts)) else {
			return;
		};

		// Like Skia, large blurs run on a downscaled copy: a wide gaussian has
		// no high frequencies left to lose, and this cuts the cost ~scale².
		let scale: u32 = if blur.sigma >= 6.0 {
			4
		} else if blur.sigma >= 3.0 {
			2
		} else {
			1
		};

		// Reach of the glow: ~3 sigma, plus what downscaling adds (coarse
		// rasterisation and the bilinear upscale each spread one low-res pixel).
		let pad = (blur.sigma * 3.0).ceil() + 3.0 * scale as f32;
		let b = device_path.bounds();
		let (cw, ch) = (self.width() as f32, self.height() as f32);
		let mut x0 = (b.left() - pad).floor().max(-pad);
		let mut y0 = (b.top() - pad).floor().max(-pad);
		// Snap the low-res grid to frame coordinates so tiles of one frame
		// produce the same glow as a single full render.
		x0 -= ((x0 as i32 + self.origin.0).rem_euclid(scale as i32)) as f32;
		y0 -= ((y0 as i32 + self.origin.1).rem_euclid(scale as i32)) as f32;
		let x1 = (b.right() + pad).ceil().min(cw + pad);
		let y1 = (b.bottom() + pad).ceil().min(ch + pad);
		if x1 <= x0 || y1 <= y0 || x1 <= 0.0 || y1 <= 0.0 || x0 >= cw || y0 >= ch {
			return;
		}
		let (w, h) = ((x1 - x0) as u32, (y1 - y0) as u32);
		if self.recording {
			self.mark_dirty(IRect::from_bounds(x0, y0, x1, y1));
			return;
		}
		// A glow is a single colour, so only its coverage is rasterised and blurred.
		let (lw, lh) = (w.div_ceil(scale) + 1, h.div_ceil(scale) + 1);
		let Some(mut low) = Mask::new(lw, lh) else {
			return;
		};
		let inv = 1.0 / scale as f32;
		let to_low = Transform::from_row(inv, 0.0, 0.0, inv, -x0 * inv, -y0 * inv);
		low.fill_path(&device_path, FillRule::Winding, paint.anti_alias, to_low);
		gaussian_blur_a8(low.data_mut(), lw as usize, lh as usize, blur.sigma * inv);

		// Only the part of the region that lands on the canvas is upscaled
		// and composited (the region extends `pad` beyond the canvas edges).
		let Some(vis) = IRect::from_bounds(x0, y0, x1, y1).clamp_to(self.width(), self.height()) else {
			return;
		};
		let (ox, oy) = ((vis.x0 as f32 - x0) as u32, (vis.y0 as f32 - y0) as u32);
		let (vw, vh) = (vis.width() as u32, vis.height() as u32);
		let mut glow = if scale == 1 {
			let mut g = Vec::with_capacity((vw * vh) as usize);
			for row in low.data().chunks_exact(lw as usize).skip(oy as usize).take(vh as usize) {
				g.extend_from_slice(&row[ox as usize..(ox + vw) as usize]);
			}
			g
		} else {
			upscale_bilinear_a8(low.data(), lw as usize, lh as usize, scale, ox, oy, vw, vh)
		};

		if blur.style == BlurStyle::Outer {
			// Knock out the shape itself, using a full-resolution coverage mask.
			if let Some(mut shape) = Mask::new(vw, vh) {
				let to_vis = Transform::from_translate(-(vis.x0 as f32), -(vis.y0 as f32));
				shape.fill_path(&device_path, FillRule::Winding, true, to_vis);
				for (g, &m) in glow.iter_mut().zip(shape.data()) {
					*g = ((*g as u32 * (255 - m as u32) + 127) / 255) as u8;
				}
			}
		}

		self.mark_dirty(vis);
		let (target, state) = self.parts();
		src_over_color(target, &glow, vw as usize, vw, vh, vis.x0, vis.y0, paint.color, state.clip.as_deref());
	}

	/// `DrawLine`: always stroked, whatever the paint style.
	pub fn draw_line(&mut self, x0: f32, y0: f32, x1: f32, y1: f32, paint: &Paint) {
		let mut pb = PathBuilder::new();
		pb.move_to(x0, y0);
		pb.line_to(x1, y1);
		if let Some(path) = pb.finish() {
			let p = Paint { style: PaintStyle::Stroke, ..*paint };
			self.draw_path(&path, &p);
		}
	}

	/// `DrawArc(oval, start, sweep, useCenter: false, paint)`.
	pub fn draw_arc(&mut self, oval: Rect, start_deg: f32, sweep_deg: f32, paint: &Paint) {
		if let Some(path) = arc_path(oval, start_deg, sweep_deg) {
			self.draw_path(&path, paint);
		}
	}

	pub fn draw_circle(&mut self, cx: f32, cy: f32, radius: f32, paint: &Paint) {
		if let Some(path) = PathBuilder::from_circle(cx, cy, radius) {
			self.draw_path(&path, paint);
		}
	}

	pub fn draw_rect(&mut self, r: Rect, paint: &Paint) {
		self.draw_path(&PathBuilder::from_rect(r), paint);
	}

	/// `DrawRect(x, y, w, h)` convenience; empty or inverted sizes are sorted like Skia.
	pub fn draw_rect_xywh(&mut self, x: f32, y: f32, w: f32, h: f32, paint: &Paint) {
		if let Some(r) = rect(x, y, x + w, y + h) {
			self.draw_rect(r, paint);
		}
	}

	pub fn draw_round_rect(&mut self, r: Rect, rx: f32, ry: f32, paint: &Paint) {
		if let Some(path) = round_rect_path(r, rx, ry) {
			self.draw_path(&path, paint);
		}
	}

	/// Fills `r` with a vertical linear gradient from `top` to `bottom`.
	pub fn draw_rect_vertical_gradient(&mut self, r: Rect, top: Rgba, bottom: Rgba) {
		let Some(shader) = tiny_skia::LinearGradient::new(
			tiny_skia::Point::from_xy(0.0, r.top()),
			tiny_skia::Point::from_xy(0.0, r.bottom()),
			vec![tiny_skia::GradientStop::new(0.0, top.to_skia()), tiny_skia::GradientStop::new(1.0, bottom.to_skia())],
			SpreadMode::Pad,
			Transform::identity(),
		) else {
			return;
		};
		self.mark_dirty_user(r.left(), r.top(), r.right(), r.bottom(), 0.0);
		if self.recording {
			return;
		}
		let paint = tiny_skia::Paint { shader, anti_alias: true, ..tiny_skia::Paint::default() };
		let (target, state) = self.parts();
		target.fill_rect(r, &paint, state.transform, state.clip.as_deref());
	}

	/// Draws `image` scaled into `dest` with the given opacity (0..=1).
	pub fn draw_image(&mut self, image: &Pixmap, dest: Rect, opacity: f32) {
		let sx = dest.width() / image.width() as f32;
		let sy = dest.height() / image.height() as f32;
		let shader = Pattern::new(
			image.as_ref(),
			SpreadMode::Pad,
			FilterQuality::Bilinear,
			opacity.clamp(0.0, 1.0),
			Transform::from_row(sx, 0.0, 0.0, sy, dest.left(), dest.top()),
		);
		let paint = tiny_skia::Paint { shader, anti_alias: true, ..tiny_skia::Paint::default() };
		self.mark_dirty_user(dest.left(), dest.top(), dest.right(), dest.bottom(), 0.0);
		if self.recording {
			return;
		}
		let (target, state) = self.parts();
		target.fill_rect(dest, &paint, state.transform, state.clip.as_deref());
	}

	/// Draws a cached [`Sprite`] where it was rendered.
	pub fn draw_sprite(&mut self, sprite: &Sprite) {
		let Some(p) = &sprite.pixmap else {
			return;
		};
		let (x, y) = (sprite.x, sprite.y);
		let (w, h) = (p.width(), p.height());
		self.mark_dirty_user(x as f32, y as f32, (x + w as i32) as f32, (y + h as i32) as f32, 0.0);
		if self.recording {
			return;
		}
		let (target, state) = self.parts();
		match integer_translation(&state.transform) {
			Some((tx, ty)) if sprite.opaque && state.clip.is_none() => {
				copy_rows(target, p.data(), w as usize * 4, w, h, x + tx, y + ty)
			}
			Some((tx, ty)) => {
				src_over(target, p.data(), w as usize * 4, w, h, x + tx, y + ty, 255, state.clip.as_deref())
			}
			None => {
				target.draw_pixmap(x, y, p.as_ref(), &PixmapPaint::default(), state.transform, state.clip.as_deref())
			}
		}
	}

	/// Blits a same-scale pixmap at an integer offset (used for static caches).
	pub fn draw_pixmap(&mut self, image: &Pixmap, x: i32, y: i32) {
		let (x0, y0) = (x as f32, y as f32);
		self.mark_dirty_user(x0, y0, x0 + image.width() as f32, y0 + image.height() as f32, 0.0);
		if self.recording {
			return;
		}
		let (target, state) = self.parts();
		if let Some((tx, ty)) = integer_translation(&state.transform) {
			let stride = image.width() as usize * 4;
			let (w, h) = (image.width(), image.height());
			src_over(target, image.data(), stride, w, h, x + tx, y + ty, 255, state.clip.as_deref());
		} else {
			target.draw_pixmap(x, y, image.as_ref(), &PixmapPaint::default(), state.transform, state.clip.as_deref());
		}
	}

	/// `DrawText(text, x, y, align, font, paint)`; `y` is the baseline.
	#[allow(clippy::too_many_arguments)]
	pub fn draw_text(
		&mut self,
		text: &str,
		x: f32,
		y: f32,
		align: TextAlign,
		typeface: &Typeface,
		size: f32,
		paint: &Paint,
	) {
		if let Some(path) = typeface.text_path(text, x, y, size, align) {
			let p = Paint { style: PaintStyle::Fill, ..*paint };
			self.draw_path(&path, &p);
		}
	}

	/// The C# `DrawTextWithBlur` extension: blurred pass, then a crisp pass.
	#[allow(clippy::too_many_arguments)]
	pub fn draw_text_with_blur(
		&mut self,
		text: &str,
		x: f32,
		y: f32,
		align: TextAlign,
		typeface: &Typeface,
		size: f32,
		paint: &Paint,
		blur: Blur,
	) {
		if let Some(path) = typeface.text_path(text, x, y, size, align) {
			let p = Paint { style: PaintStyle::Fill, ..*paint };
			self.draw_path(&path, &p.with_blur(Some(blur)));
			self.draw_path(&path, &p);
		}
	}

	/// The C# `DrawRectWithBlur` extension: blurred pass, then a crisp pass.
	pub fn draw_rect_with_blur(&mut self, r: Rect, paint: &Paint, blur: Blur) {
		let path = PathBuilder::from_rect(r);
		self.draw_path(&path, &paint.with_blur(Some(blur)));
		self.draw_path(&path, paint);
	}

	/// Reads one unpremultiplied pixel from the finished frame (for tests/tools).
	pub fn pixel(&self, x: u32, y: u32) -> Option<Rgba> {
		let c = self.base.pixel(x, y)?.demultiply();
		Some(Rgba::new(c.red(), c.green(), c.blue(), c.alpha()))
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn arc_endpoints_follow_skia_convention() {
		let oval = rect(0.0, 0.0, 200.0, 200.0).unwrap();
		// 0deg = +x (right), 90deg = +y (down).
		let p = arc_path(oval, 0.0, 90.0).unwrap();
		let pts = p.points();
		let first = pts.first().unwrap();
		let last = pts.last().unwrap();
		assert!((first.x - 200.0).abs() < 0.01 && (first.y - 100.0).abs() < 0.01);
		assert!((last.x - 100.0).abs() < 0.01 && (last.y - 200.0).abs() < 0.01);
	}

	#[test]
	fn layer_opacity_is_applied_on_restore() {
		let mut c = Canvas::new(10, 10);
		c.clear(Rgba::BLACK);
		c.save_layer(128);
		c.draw_rect(rect(0.0, 0.0, 10.0, 10.0).unwrap(), &Paint::fill(Rgba::WHITE));
		c.restore();
		let px = c.pixel(5, 5).unwrap();
		assert!((px.r as i32 - 128).abs() <= 2, "{px:?}");
	}

	#[test]
	fn clip_rect_limits_drawing() {
		let mut c = Canvas::new(20, 20);
		c.save();
		c.clip_rect(rect(0.0, 0.0, 10.0, 20.0).unwrap());
		c.draw_rect(rect(0.0, 0.0, 20.0, 20.0).unwrap(), &Paint::fill(Rgba::WHITE));
		c.restore();
		assert_eq!(c.pixel(5, 5).unwrap().a, 255);
		assert_eq!(c.pixel(15, 5).unwrap().a, 0);
	}

	#[test]
	fn outer_blur_leaves_shape_interior_empty() {
		let mut c = Canvas::new(60, 60);
		let r = rect(20.0, 20.0, 40.0, 40.0).unwrap();
		c.draw_rect(r, &Paint::fill(Rgba::WHITE).with_blur(Some(Blur::outer(4.0))));
		assert_eq!(c.pixel(30, 30).unwrap().a, 0);
		assert!(c.pixel(18, 30).unwrap().a > 0);
	}
}
