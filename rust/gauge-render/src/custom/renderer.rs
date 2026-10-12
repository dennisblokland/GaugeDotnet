//! Port of `ElementRenderer` and the per-element renderers in
//! `Gauges/Custom/Renderers`. Drawing math mirrors the C# code line for line
//! so definitions look the same on both implementations.

use std::borrow::Cow;
use std::collections::{HashMap, VecDeque};
use std::ops::Range;
use std::path::Path;
use std::time::Instant;

use chrono::NaiveDateTime;

use crate::canvas::{rect, Blur, Canvas, LineCap, LineJoin, Paint, PaintStyle, Sprite, ARC_SEGMENT_DEG};
use crate::color::Rgba;
use crate::composite::IRect;
use crate::format::{format_datetime, format_number};
use crate::images::ImageCache;
use crate::model::*;
use crate::text::{default_typeface, typeface, TextAlign};
use crate::{expression, lookup, Values};

fn color(hex: &str) -> Rgba {
	Rgba::parse_or_black(hex)
}

/// Angle (degrees) for a normalised value along a sweep.
fn sweep_angle(start: f32, sweep: f32, anti_clockwise: bool, t: f32) -> f32 {
	let sign = if anti_clockwise { -1.0 } else { 1.0 };
	start + t * sweep * sign
}

fn polar(cx: f32, cy: f32, radius: f32, angle_deg: f32) -> (f32, f32) {
	let a = angle_deg.to_radians();
	(cx + a.cos() * radius, cy + a.sin() * radius)
}

fn circle_rect(cx: f32, cy: f32, r: f32) -> Option<tiny_skia::Rect> {
	rect(cx - r, cy - r, cx + r, cy + r)
}

/// Renders custom gauge definitions. Holds the state the C# version keeps in
/// static caches: decoded images, graph history and peak-hold values (keyed
/// by element `Id`).
///
/// A frame is two steps: [`prepare`](Self::prepare) resolves values and
/// advances per-element state once, then a [`Painter`] draws. Painting is
/// read-only, so it can be repeated (bounds passes, several dirty regions)
/// or shared between threads.
#[derive(Default)]
pub struct ElementRenderer {
	images: ImageCache,
	/// History per graph `Id`, plus how many samples were ever pushed.
	graph_buffers: HashMap<String, (VecDeque<f32>, u64)>,
	peak_state: HashMap<String, (f32, Instant)>,
	/// When set, clock elements show this time instead of the system clock
	/// (deterministic snapshots, designer previews).
	pub fixed_time: Option<NaiveDateTime>,
	/// Local and UTC time sampled by the last `prepare`.
	frame_time: Option<(NaiveDateTime, NaiveDateTime)>,
}

impl ElementRenderer {
	pub fn new() -> Self {
		Self::default()
	}

	/// Drops cached images and per-element history (`ClearImageCache`).
	pub fn clear_caches(&mut self) {
		self.images.clear();
		self.graph_buffers.clear();
		self.peak_state.clear();
	}

	/// Draws a whole definition. `values` drive visual elements (normally the
	/// smoothed values); `target_values`, when given, drive text-like elements
	/// so numbers snap instead of rolling.
	pub fn render(
		&mut self,
		canvas: &mut Canvas,
		definition: &CustomGaugeDefinition,
		values: &Values,
		target_values: Option<&Values>,
	) {
		let resolved = self.prepare(definition, values, target_values);
		let painter = self.painter(definition);
		painter.draw_backdrop(canvas);
		for (element, value) in definition.elements.iter().zip(&resolved) {
			if let Some(v) = value {
				painter.draw_element(canvas, element, *v);
			}
		}
	}

	/// Same output as [`ElementRenderer::render`], but runs of elements that
	/// cannot change between frames (no data source, no visibility rule) are
	/// rendered once into sprites held by `cache`, together with the
	/// background. Use one cache per definition and call
	/// [`StaticCache::invalidate`] after editing that definition.
	pub fn render_cached(
		&mut self,
		canvas: &mut Canvas,
		definition: &CustomGaugeDefinition,
		values: &Values,
		target_values: Option<&Values>,
		cache: &mut StaticCache,
	) {
		let resolved = self.prepare(definition, values, target_values);
		let painter = self.painter(definition);
		cache.ensure(&painter, (canvas.width(), canvas.height()));
		for segment in &cache.segments {
			match segment {
				Segment::Static { sprite, .. } => {
					if let Some(sprite) = sprite {
						canvas.draw_sprite(sprite);
					}
				}
				Segment::Dynamic(i) => {
					if let Some(v) = resolved[*i] {
						painter.draw_element(canvas, &definition.elements[*i], v);
					}
				}
			}
		}
	}

	/// Starts a frame: merges calculated channels, applies visibility rules
	/// and picks each element's value (`None` = hidden), then advances graph
	/// history, peak hold and the clock, and loads any images. Call once per
	/// frame, before painting.
	pub fn prepare(
		&mut self,
		definition: &CustomGaugeDefinition,
		values: &Values,
		target_values: Option<&Values>,
	) -> Vec<Option<f32>> {
		let (all_values, all_targets) = merge_calculated(definition, values, target_values);
		let base_dir = definition.base_directory.as_deref();
		self.frame_time = Some(match self.fixed_time {
			Some(t) => (t, t),
			None => (chrono::Local::now().naive_local(), chrono::Utc::now().naive_utc()),
		});
		if let Some(bg) = definition.background_image.as_deref().filter(|s| !s.is_empty()) {
			self.images.load(bg, base_dir);
		}

		let mut resolved = Vec::with_capacity(definition.elements.len());
		for element in &definition.elements {
			let base = element.base();
			if base.use_visibility {
				if let Some(src) = base.visibility_source.as_deref().filter(|s| !s.is_empty()) {
					let v = lookup(&all_values, src).unwrap_or(0.0);
					if v < base.visible_above || v > base.visible_below {
						resolved.push(None);
						continue;
					}
				}
			}
			let source = match all_targets.as_deref() {
				Some(t) if element.uses_target_value() => t,
				_ => &all_values,
			};
			let value = base.source().and_then(|s| lookup(source, s)).unwrap_or(0.0);
			self.prepare_element(element, value, base_dir);
			resolved.push(Some(value));
		}
		resolved
	}

	/// Advances the per-element state that drawing reads.
	fn prepare_element(&mut self, element: &Element, value: f32, base_dir: Option<&Path>) {
		match element {
			Element::Needle(n) => {
				if let Some(path) = n.image_path.as_deref().filter(|s| !s.is_empty()) {
					self.images.load(path, base_dir);
				}
			}
			Element::Image(img) if !img.image_path.is_empty() => {
				self.images.load(&img.image_path, base_dir);
			}
			Element::Gif(gif) if !gif.image_path.is_empty() => {
				self.images.load_frames(&gif.image_path, base_dir);
			}
			Element::Graph(g) => {
				let depth = g.history_depth.max(2) as usize;
				let (buffer, pushes) = self.graph_buffers.entry(g.base.id.clone()).or_default();
				buffer.push_back(value);
				*pushes += 1;
				while buffer.len() > depth {
					buffer.pop_front();
				}
			}
			Element::Peak(p) if p.base.has_source() => {
				let now = Instant::now();
				let (mut peak, mut seen) = *self.peak_state.entry(p.base.id.clone()).or_insert((value, now));
				if p.decay_seconds > 0.0 && now.duration_since(seen).as_secs_f32() > p.decay_seconds {
					peak = value;
					seen = now;
				}
				if value > peak {
					peak = value;
					seen = now;
				}
				self.peak_state.insert(p.base.id.clone(), (peak, seen));
			}
			_ => {}
		}
	}

	/// Read-only drawing view for this frame.
	pub fn painter<'a>(&'a self, definition: &'a CustomGaugeDefinition) -> Painter<'a> {
		Painter { renderer: self, definition }
	}

	/// Draws one element outside a frame (prepares and paints it).
	pub fn draw_element(&mut self, canvas: &mut Canvas, element: &Element, value: f32, base_dir: Option<&Path>) {
		if self.frame_time.is_none() || self.fixed_time.is_none() {
			let now = self.fixed_time.unwrap_or_else(|| chrono::Local::now().naive_local());
			self.frame_time = Some((now, self.fixed_time.unwrap_or_else(|| chrono::Utc::now().naive_utc())));
		}
		self.prepare_element(element, value, base_dir);
		let definition =
			CustomGaugeDefinition { base_directory: base_dir.map(Path::to_path_buf), ..Default::default() };
		Painter { renderer: self, definition: &definition }.draw_element(canvas, element, value);
	}

	/// A fingerprint of everything that decides how an element looks this
	/// frame (its value, plus clock text, peak value or graph history), used
	/// to skip redrawing elements that did not change.
	pub fn element_key(&self, element: &Element, value: Option<f32>) -> u64 {
		use std::hash::{Hash, Hasher};
		let mut h = std::collections::hash_map::DefaultHasher::new();
		value.map(f32::to_bits).hash(&mut h);
		match element {
			Element::Clock(c) => self.clock_text(c).hash(&mut h),
			Element::Peak(p) => self.peak_state.get(&p.base.id).map(|(v, _)| v.to_bits()).hash(&mut h),
			Element::Graph(g) => self.graph_buffers.get(&g.base.id).map(|(_, n)| *n).hash(&mut h),
			_ => {}
		}
		h.finish()
	}

	fn clock_text(&self, c: &ClockElement) -> String {
		let (local, utc) = self.frame_time.unwrap_or_else(|| {
			let now = chrono::Local::now().naive_local();
			(now, now)
		});
		let now = if c.use_utc { utc } else { local };
		format_datetime(&now, &c.format).unwrap_or_else(|| format_datetime(&now, "HH:mm:ss").unwrap_or_default())
	}
}

/// Immutable drawing view of an [`ElementRenderer`] for one frame. `Sync`,
/// so several threads can paint different tiles of the same frame.
#[derive(Clone, Copy)]
pub struct Painter<'a> {
	renderer: &'a ElementRenderer,
	definition: &'a CustomGaugeDefinition,
}

impl<'a> Painter<'a> {
	pub fn definition(&self) -> &'a CustomGaugeDefinition {
		self.definition
	}

	fn base_dir(&self) -> Option<&'a Path> {
		self.definition.base_directory.as_deref()
	}

	/// Background colour and image.
	pub fn draw_backdrop(&self, canvas: &mut Canvas) {
		let d = self.definition;
		canvas.clear(color(&d.background_color));
		if let Some(bg) = d.background_image.as_deref().filter(|s| !s.is_empty()) {
			if let Some(img) = self.renderer.images.get(bg, self.base_dir()) {
				draw_background(
					canvas,
					&img,
					d.width as f32,
					d.height as f32,
					d.background_image_mode,
					d.background_image_opacity,
				);
			}
		}
	}

	/// Draws one element with the given value (`ElementRenderer.DrawElement`).
	pub fn draw_element(&self, canvas: &mut Canvas, element: &Element, value: f32) {
		let opacity = element.base().opacity;
		let use_layer = opacity < 255;
		if use_layer {
			canvas.save_layer(opacity);
		}

		match element {
			Element::Arc(e) => draw_arc(canvas, e, value),
			Element::Needle(e) => self.draw_needle(canvas, e, value),
			Element::Text(e) => draw_text(canvas, e),
			Element::Value(e) => draw_value(canvas, e, value),
			Element::Ticks(e) => draw_ticks(canvas, e),
			Element::Circle(e) => draw_circle(canvas, e),
			Element::Rectangle(e) => draw_rectangle(canvas, e),
			Element::Line(e) => draw_line(canvas, e),
			Element::LinearBar(e) => draw_linear_bar(canvas, e, value),
			Element::Warning(e) => draw_warning(canvas, e, value),
			Element::Image(e) => self.draw_image(canvas, e),
			Element::ZoneArc(e) => draw_zone_arc(canvas, e, value),
			Element::Graph(e) => self.draw_graph(canvas, e),
			Element::LabelValue(e) => draw_label_value(canvas, e, value),
			Element::Peak(e) => self.draw_peak(canvas, e),
			Element::Gif(e) => self.draw_gif(canvas, e, value),
			Element::Clock(e) => self.draw_clock(canvas, e),
		}

		if use_layer {
			canvas.restore();
		}
	}

	fn draw_needle(&self, canvas: &mut Canvas, n: &NeedleElement, value: f32) {
		const GLOW_ALPHA: u8 = 50;
		const GLOW_SIGMA: f32 = 6.0;
		const GLOW_STROKE_EXTRA: f32 = 6.0;

		let t = if n.base.has_source() { n.base.normalized(value) } else { 0.0 };
		let angle = sweep_angle(n.start_angle_deg, n.sweep_angle_deg, n.anti_clockwise, t);
		let (x, y) = (n.base.x, n.base.y);

		if let Some(path) = n.image_path.as_deref().filter(|s| !s.is_empty()) {
			if let Some(img) = self.renderer.images.get(path, self.base_dir()) {
				canvas.save();
				canvas.translate(x, y);
				canvas.rotate_degrees(angle + 90.0);
				if let Some(dest) = rect(-n.image_width / 2.0, -n.image_length, n.image_width / 2.0, 0.0) {
					canvas.draw_image(&img, dest, 1.0);
				}
				canvas.restore();
				if n.show_hub {
					canvas.draw_circle(x, y, n.hub_radius, &Paint::fill(color(&n.hub_color)));
				}
				return;
			}
		}

		let (tip_x, tip_y) = polar(x, y, n.length, angle);
		let (tail_x, tail_y) = polar(x, y, -n.tail_length, angle);
		let needle_color = color(&n.color);

		let glow = Paint::stroke(needle_color.with_alpha(GLOW_ALPHA), n.needle_width + GLOW_STROKE_EXTRA)
			.with_cap(LineCap::Round)
			.with_blur(Some(Blur::normal(GLOW_SIGMA)));
		canvas.draw_line(tail_x, tail_y, tip_x, tip_y, &glow);
		let crisp = Paint::stroke(needle_color, n.needle_width).with_cap(LineCap::Round);
		canvas.draw_line(tail_x, tail_y, tip_x, tip_y, &crisp);

		if n.show_hub {
			canvas.draw_circle(x, y, n.hub_radius, &Paint::fill(color(&n.hub_color)));
		}
	}

	fn draw_image(&self, canvas: &mut Canvas, img: &ImageElement) {
		if img.image_path.is_empty() {
			return;
		}
		if let Some(bitmap) = self.renderer.images.get(&img.image_path, self.base_dir()) {
			draw_rotated_image(canvas, &bitmap, img.base.x, img.base.y, img.width, img.height, img.rotation);
		}
	}

	fn draw_gif(&self, canvas: &mut Canvas, gif: &GifElement, value: f32) {
		if gif.image_path.is_empty() {
			return;
		}
		let Some(frames) = self.renderer.images.get_frames(&gif.image_path, self.base_dir()) else {
			return;
		};
		if frames.is_empty() {
			return;
		}
		let index = select_frame(value, gif.base.min_value, gif.base.max_value, frames.len());
		draw_rotated_image(canvas, &frames[index], gif.base.x, gif.base.y, gif.width, gif.height, gif.rotation);
	}

	fn draw_graph(&self, canvas: &mut Canvas, g: &GraphElement) {
		let (x, y) = (g.base.x, g.base.y);
		let Some(bounds) = rect(x, y, x + g.width, y + g.height) else {
			return;
		};
		canvas.draw_rect(bounds, &Paint::fill(color(&g.back_color)));

		let depth = g.history_depth.max(2) as usize;
		let Some((buffer, _)) = self.renderer.graph_buffers.get(&g.base.id) else {
			return;
		};
		if buffer.len() < 2 {
			return;
		}
		let range = g.base.max_value - g.base.min_value;
		if range <= 0.0 {
			return;
		}

		let count = buffer.len();
		let step_x = g.width / (depth - 1) as f32;
		let point = |i: usize, sample: f32| {
			let px = x + (depth - count + i) as f32 * step_x;
			let t = ((sample - g.base.min_value) / range).clamp(0.0, 1.0);
			(px, y + g.height - t * g.height)
		};

		let mut line = tiny_skia::PathBuilder::new();
		for (i, &s) in buffer.iter().enumerate() {
			let (px, py) = point(i, s);
			if i == 0 {
				line.move_to(px, py);
			} else {
				line.line_to(px, py);
			}
		}

		canvas.save();
		canvas.clip_rect(bounds);

		if g.show_fill {
			let mut fill = line.clone();
			let last_x = x + (depth - 1) as f32 * step_x;
			let first_x = x + (depth - count) as f32 * step_x;
			fill.line_to(last_x, y + g.height);
			fill.line_to(first_x, y + g.height);
			fill.close();
			if let Some(path) = fill.finish() {
				canvas.draw_path(&path, &Paint::fill(color(&g.fill_color).with_alpha(g.fill_opacity)));
			}
		}

		if let Some(path) = line.finish() {
			let stroke =
				Paint::stroke(color(&g.line_color), g.line_width).with_cap(LineCap::Round).with_join(LineJoin::Round);
			canvas.draw_path(&path, &stroke);
		}
		canvas.restore();
	}

	fn draw_peak(&self, canvas: &mut Canvas, p: &PeakMarkerElement) {
		if !p.base.has_source() {
			return;
		}
		let Some(&(peak, _)) = self.renderer.peak_state.get(&p.base.id) else {
			return;
		};
		let t = p.base.normalized(peak);
		let angle = sweep_angle(p.start_angle_deg, p.sweep_angle_deg, p.anti_clockwise, t);
		let (ix, iy) = polar(p.base.x, p.base.y, p.radius - p.stroke_width / 2.0, angle);
		let (ox, oy) = polar(p.base.x, p.base.y, p.radius + p.stroke_width / 2.0, angle);
		canvas.draw_line(ix, iy, ox, oy, &Paint::stroke(color(&p.marker_color), p.marker_width));
	}

	fn draw_clock(&self, canvas: &mut Canvas, c: &ClockElement) {
		let text = self.renderer.clock_text(c);
		draw_boxed_text(
			canvas,
			&text,
			c.base.x,
			c.base.y,
			&c.font,
			c.font_size,
			color(&c.color),
			c.show_box.then(|| (color(&c.box_color), c.box_padding, c.box_corner_radius)),
		);
	}
}

/// Merges calculated channels into copies of the value maps (the caller's
/// maps are untouched); borrows them unchanged when there are none.
fn merge_calculated<'a>(
	definition: &CustomGaugeDefinition,
	values: &'a Values,
	targets: Option<&'a Values>,
) -> (Cow<'a, Values>, Option<Cow<'a, Values>>) {
	if definition.calculated_channels.is_empty() {
		return (Cow::Borrowed(values), targets.map(Cow::Borrowed));
	}
	let mut owned_values = values.clone();
	let mut owned_targets = targets.cloned();
	for ch in &definition.calculated_channels {
		if ch.name.is_empty() || ch.expression.is_empty() {
			continue;
		}
		let result = expression::evaluate(&ch.expression, &owned_values);
		owned_values.insert(ch.name.clone(), result);
		if let Some(t) = owned_targets.as_mut() {
			t.insert(ch.name.clone(), result);
		}
	}
	(Cow::Owned(owned_values), owned_targets.map(Cow::Owned))
}

/// Whether an element draws the same pixels every frame: no visibility rule,
/// and either no data source or one it ignores. Clocks and graphs change on
/// their own.
pub fn is_static(element: &Element) -> bool {
	let base = element.base();
	if base.use_visibility && base.visibility_source.as_deref().is_some_and(|s| !s.is_empty()) {
		return false;
	}
	match element {
		Element::Clock(_) | Element::Graph(_) => false,
		Element::Text(_) => true,
		Element::ZoneArc(z) => !(z.show_pointer && base.has_source()),
		_ => !base.has_source(),
	}
}

pub(crate) enum Segment {
	/// A run of static elements, pre-rendered on first use.
	Static { range: Range<usize>, backdrop: bool, sprite: Option<Sprite> },
	/// An element redrawn every frame.
	Dynamic(usize),
}

/// Pre-rendered static runs of one definition, for [`ElementRenderer::render_cached`].
#[derive(Default)]
pub struct StaticCache {
	pub(crate) segments: Vec<Segment>,
	size: (u32, u32),
}

impl StaticCache {
	pub fn new() -> Self {
		Self::default()
	}

	/// Drops the pre-rendered runs (after the definition or its images change).
	pub fn invalidate(&mut self) {
		self.segments.clear();
	}

	/// Builds the segment plan and renders static runs if needed.
	pub(crate) fn ensure(&mut self, painter: &Painter, size: (u32, u32)) {
		if self.size != size || self.segments.is_empty() {
			self.build(painter.definition(), size);
		}
		let elements = &painter.definition().elements;
		for segment in &mut self.segments {
			if let Segment::Static { range, backdrop, sprite: sprite @ None } = segment {
				*sprite = Some(Sprite::render(size.0, size.1, |c| {
					if *backdrop {
						painter.draw_backdrop(c);
					}
					for element in &elements[range.clone()] {
						painter.draw_element(c, element, 0.0);
					}
				}));
			}
		}
	}

	fn build(&mut self, definition: &CustomGaugeDefinition, size: (u32, u32)) {
		self.size = size;
		self.segments.clear();
		let n = definition.elements.len();
		let mut i = 0;
		let mut first = true;
		while i < n || first {
			let start = i;
			while i < n && is_static(&definition.elements[i]) {
				i += 1;
			}
			// The first run always exists: it carries the background.
			if first || i > start {
				self.segments.push(Segment::Static { range: start..i, backdrop: first, sprite: None });
			}
			first = false;
			if i < n {
				self.segments.push(Segment::Dynamic(i));
				i += 1;
			}
		}
	}
}

/// Fill colour an arc or bar uses at `value` (conditional colours).
fn conditional_fill<'a>(
	base: &ElementBase,
	normal: &'a str,
	on: bool,
	warn: (f32, &'a str),
	danger: (f32, &'a str),
	value: f32,
) -> &'a str {
	if on && base.has_source() {
		if value >= danger.0 {
			return danger.1;
		} else if value >= warn.0 {
			return warn.1;
		}
	}
	normal
}

/// For elements whose change between two values is confined to a small
/// region (the wedge between an arc's old and new fill angle, the strip
/// between a bar's old and new fill edge), the device bounds of that region,
/// including glow. `None` when the whole element must be repainted.
pub(crate) fn partial_damage(element: &Element, old: f32, new: f32, size: (u32, u32)) -> Option<Option<IRect>> {
	let mut rec = Canvas::recorder(size.0, size.1);
	match element {
		Element::Arc(arc) if arc.is_dynamic && arc.base.has_source() => {
			let fill = |v| {
				conditional_fill(
					&arc.base,
					&arc.color,
					arc.use_conditional_color,
					(arc.warn_threshold, &arc.warn_color),
					(arc.danger_threshold, &arc.danger_color),
					v,
				)
			};
			// A colour change, or the fill appearing/disappearing, repaints the whole arc.
			let (t0, t1) = (arc.base.normalized(old), arc.base.normalized(new));
			if fill(old) != fill(new) || t0 == 0.0 || t1 == 0.0 {
				return None;
			}
			let oval = circle_rect(arc.base.x, arc.base.y, arc.radius)?;
			let sign = if arc.anti_clockwise { -1.0 } else { 1.0 };
			// The arc's last cubic segment (from the previous ARC_SEGMENT_DEG
			// boundary) is rebuilt when the end moves, so start the wedge there.
			let lo = arc.sweep_angle_deg * t0.min(t1);
			let lo = (lo / ARC_SEGMENT_DEG).floor() * ARC_SEGMENT_DEG;
			let hi = arc.sweep_angle_deg * t0.max(t1);
			let a0 = arc.start_angle_deg + lo * sign;
			let sweep = (hi - lo) * sign;
			// Same geometry as the glow pass (the widest one), wedge only.
			let glow = Paint::stroke(Rgba::WHITE, arc.stroke_width + 12.0).with_blur(Some(Blur::normal(8.0)));
			rec.draw_arc(oval, a0, sweep, &glow);
		}
		Element::LinearBar(bar) => {
			let fill = |v| {
				conditional_fill(
					&bar.base,
					&bar.fill_color,
					bar.use_conditional_color,
					(bar.warn_threshold, &bar.warn_color),
					(bar.danger_threshold, &bar.danger_color),
					v,
				)
			};
			let (t0, t1) = (bar.base.normalized(old), bar.base.normalized(new));
			if fill(old) != fill(new) || t0 == 0.0 || t1 == 0.0 {
				return None;
			}
			let (x, y) = (bar.base.x, bar.base.y);
			let (lo, hi) = (t0.min(t1), t0.max(t1));
			let strip = if bar.is_vertical {
				rect(x, y + bar.height * (1.0 - hi), x + bar.width, y + bar.height * (1.0 - lo))
			} else {
				rect(x + bar.width * lo, y, x + bar.width * hi, y + bar.height)
			}?;
			rec.draw_rect(strip, &Paint::fill(Rgba::WHITE).with_blur(Some(Blur::normal(6.0))));
		}
		_ => return None,
	}
	Some(rec.recorded().and_then(|r| r.clamp_to(size.0, size.1)))
}

/// Frame index for a value mapped `min..max` → `0..frame_count-1`.
pub fn select_frame(value: f32, min: f32, max: f32, frame_count: usize) -> usize {
	if frame_count <= 1 {
		return 0;
	}
	let t = if max > min { (value - min) / (max - min) } else { 0.0 };
	let idx = (t.clamp(0.0, 1.0) * (frame_count - 1) as f32).round() as usize;
	idx.min(frame_count - 1)
}

fn draw_rotated_image(canvas: &mut Canvas, img: &tiny_skia::Pixmap, x: f32, y: f32, w: f32, h: f32, rotation: f32) {
	canvas.save();
	if rotation != 0.0 {
		canvas.translate(x + w / 2.0, y + h / 2.0);
		canvas.rotate_degrees(rotation);
		canvas.translate(-(x + w / 2.0), -(y + h / 2.0));
	}
	if let Some(dest) = rect(x, y, x + w, y + h) {
		canvas.draw_image(img, dest, 1.0);
	}
	canvas.restore();
}

fn draw_background(
	canvas: &mut Canvas,
	img: &tiny_skia::Pixmap,
	cw: f32,
	ch: f32,
	mode: BackgroundImageMode,
	alpha: u8,
) {
	let opacity = alpha as f32 / 255.0;
	let (iw, ih) = (img.width() as f32, img.height() as f32);
	let full = rect(0.0, 0.0, cw, ch);
	let centered = |w: f32, h: f32| {
		let ox = (cw - w) / 2.0;
		let oy = (ch - h) / 2.0;
		rect(ox, oy, ox + w, oy + h)
	};
	match mode {
		BackgroundImageMode::Stretch => {
			if let Some(dest) = full {
				canvas.draw_image(img, dest, opacity);
			}
		}
		BackgroundImageMode::Fit => {
			let scale = (cw / iw).min(ch / ih);
			if let Some(dest) = centered(iw * scale, ih * scale) {
				canvas.draw_image(img, dest, opacity);
			}
		}
		BackgroundImageMode::Fill | BackgroundImageMode::Center | BackgroundImageMode::Tile => {
			let Some(clip) = full else { return };
			canvas.save();
			canvas.clip_rect(clip);
			match mode {
				BackgroundImageMode::Fill => {
					let scale = (cw / iw).max(ch / ih);
					if let Some(dest) = centered(iw * scale, ih * scale) {
						canvas.draw_image(img, dest, opacity);
					}
				}
				BackgroundImageMode::Center => {
					if let Some(dest) = centered(iw, ih) {
						canvas.draw_image(img, dest, opacity);
					}
				}
				_ => {
					let mut ty = 0.0;
					while ty < ch {
						let mut tx = 0.0;
						while tx < cw {
							if let Some(dest) = rect(tx, ty, tx + iw, ty + ih) {
								canvas.draw_image(img, dest, opacity);
							}
							tx += iw;
						}
						ty += ih;
					}
				}
			}
			canvas.restore();
		}
	}
}

fn draw_arc(canvas: &mut Canvas, arc: &ArcElement, value: f32) {
	const GLOW_ALPHA: u8 = 60;
	const GLOW_SIGMA: f32 = 8.0;
	const GLOW_STROKE_EXTRA: f32 = 12.0;

	let Some(oval) = circle_rect(arc.base.x, arc.base.y, arc.radius) else {
		return;
	};
	let sign = if arc.anti_clockwise { -1.0 } else { 1.0 };

	if arc.show_track {
		let track = Paint::stroke(color(&arc.track_color), arc.stroke_width);
		canvas.draw_arc(oval, arc.start_angle_deg, arc.sweep_angle_deg * sign, &track);
	}

	let mut fill_sweep = arc.sweep_angle_deg;
	if arc.is_dynamic && arc.base.has_source() {
		fill_sweep = arc.sweep_angle_deg * arc.base.normalized(value);
	}
	fill_sweep *= sign;

	let mut fill_color = &arc.color;
	if arc.use_conditional_color && arc.base.has_source() {
		if value >= arc.danger_threshold {
			fill_color = &arc.danger_color;
		} else if value >= arc.warn_threshold {
			fill_color = &arc.warn_color;
		}
	}
	let fill = color(fill_color);

	canvas.draw_arc(oval, arc.start_angle_deg, fill_sweep, &Paint::stroke(fill, arc.stroke_width));
	if fill_sweep != 0.0 {
		let glow = Paint::stroke(fill.with_alpha(GLOW_ALPHA), arc.stroke_width + GLOW_STROKE_EXTRA)
			.with_blur(Some(Blur::normal(GLOW_SIGMA)));
		canvas.draw_arc(oval, arc.start_angle_deg, fill_sweep, &glow);
	}
}

fn draw_zone_arc(canvas: &mut Canvas, zone: &ZoneArcElement, value: f32) {
	let Some(oval) = circle_rect(zone.base.x, zone.base.y, zone.radius) else {
		return;
	};
	let sign = if zone.anti_clockwise { -1.0 } else { 1.0 };
	let sweep = zone.sweep_angle_deg * sign;
	let range = zone.base.max_value - zone.base.min_value;
	let t2 = if range > 0.0 { ((zone.zone2_start - zone.base.min_value) / range).clamp(0.0, 1.0) } else { 0.5 };
	let t3 = if range > 0.0 { ((zone.zone3_start - zone.base.min_value) / range).clamp(0.0, 1.0) } else { 0.85 };
	let band = |c: &str| Paint::stroke(color(c), zone.stroke_width);

	let z1_end = if zone.show_zone2 {
		t2
	} else if zone.show_zone3 {
		t3
	} else {
		1.0
	};
	canvas.draw_arc(oval, zone.start_angle_deg, sweep * z1_end, &band(&zone.zone1_color));

	if zone.show_zone2 {
		let z2_end = if zone.show_zone3 { t3 } else { 1.0 };
		canvas.draw_arc(oval, zone.start_angle_deg + t2 * sweep, (z2_end - t2) * sweep, &band(&zone.zone2_color));
	}
	if zone.show_zone3 {
		let z3_start = if zone.show_zone2 { t3 } else { t2 };
		canvas.draw_arc(
			oval,
			zone.start_angle_deg + z3_start * sweep,
			(1.0 - z3_start) * sweep,
			&band(&zone.zone3_color),
		);
	}

	if zone.show_pointer && zone.base.has_source() {
		let t = zone.base.normalized(value);
		let angle = zone.start_angle_deg + t * sweep;
		let (ix, iy) = polar(zone.base.x, zone.base.y, zone.radius - zone.stroke_width / 2.0 - 4.0, angle);
		let (ox, oy) = polar(zone.base.x, zone.base.y, zone.radius + zone.stroke_width / 2.0 + 4.0, angle);
		let p = Paint::stroke(color(&zone.pointer_color), zone.pointer_width).with_cap(LineCap::Round);
		canvas.draw_line(ix, iy, ox, oy, &p);
	}
}

/// Centred text with an optional background box sized like the C# renderer
/// (`FontSize * 1.3` tall, box top at `y - FontSize - padding`).
#[allow(clippy::too_many_arguments)]
fn draw_boxed_text(
	canvas: &mut Canvas,
	text: &str,
	x: f32,
	y: f32,
	font: &str,
	size: f32,
	text_color: Rgba,
	boxed: Option<(Rgba, f32, f32)>,
) {
	let tf = typeface(font);
	if let Some((box_color, padding, corner)) = boxed {
		let w = tf.measure(text, size);
		let bx = x - w / 2.0 - padding;
		let by = y - size - padding;
		if let Some(r) = rect(bx, by, bx + w + padding * 2.0, by + size * 1.3 + padding * 2.0) {
			let paint = Paint::fill(box_color);
			if corner > 0.0 {
				canvas.draw_round_rect(r, corner, corner, &paint);
			} else {
				canvas.draw_rect(r, &paint);
			}
		}
	}
	canvas.draw_text(text, x, y, TextAlign::Center, tf, size, &Paint::fill(text_color));
}

fn draw_text(canvas: &mut Canvas, t: &TextElement) {
	draw_boxed_text(
		canvas,
		&t.text,
		t.base.x,
		t.base.y,
		&t.font,
		t.font_size,
		color(&t.color),
		t.show_box.then(|| (color(&t.box_color), t.box_padding, t.box_corner_radius)),
	);
}

fn draw_value(canvas: &mut Canvas, d: &ValueDisplayElement, value: f32) {
	let text = format_number(value, &d.format) + &d.suffix;
	canvas.draw_text(
		&text,
		d.base.x,
		d.base.y,
		TextAlign::Center,
		typeface(&d.font),
		d.font_size,
		&Paint::fill(color(&d.color)),
	);
}

fn draw_label_value(canvas: &mut Canvas, lv: &LabelValueElement, value: f32) {
	let value_text = format_number(value, &lv.value_format) + &lv.value_suffix;
	let label_tf = typeface(&lv.label_font);
	let value_tf = typeface(&lv.value_font);
	let label_w = label_tf.measure(&lv.label, lv.label_font_size);
	let value_w = value_tf.measure(&value_text, lv.value_font_size);

	let gap = 4.0;
	let total_h = lv.label_font_size + gap + lv.value_font_size;
	let total_w = label_w.max(value_w);
	let (x, y) = (lv.base.x, lv.base.y);

	if lv.show_box {
		let bx = x - total_w / 2.0 - lv.box_padding;
		let by = y - total_h / 2.0 - lv.box_padding;
		if let Some(r) = rect(bx, by, bx + total_w + lv.box_padding * 2.0, by + total_h + lv.box_padding * 2.0) {
			let paint = Paint::fill(color(&lv.box_color));
			if lv.box_corner_radius > 0.0 {
				canvas.draw_round_rect(r, lv.box_corner_radius, lv.box_corner_radius, &paint);
			} else {
				canvas.draw_rect(r, &paint);
			}
		}
	}

	let label_y = y - total_h / 2.0 + lv.label_font_size;
	let value_y = y + total_h / 2.0;
	canvas.draw_text(
		&lv.label,
		x,
		label_y,
		TextAlign::Center,
		label_tf,
		lv.label_font_size,
		&Paint::fill(color(&lv.label_color)),
	);
	canvas.draw_text(
		&value_text,
		x,
		value_y,
		TextAlign::Center,
		value_tf,
		lv.value_font_size,
		&Paint::fill(color(&lv.value_color)),
	);
}

fn draw_ticks(canvas: &mut Canvas, t: &TickRingElement) {
	let total = t.major_count;
	if total <= 0 {
		return;
	}
	let (cx, cy) = (t.base.x, t.base.y);
	let tick_color = color(&t.color);
	let label_color = color(&t.label_color);
	let dir = if t.ticks_inside { 1.0 } else { -1.0 };

	for i in 0..=total {
		let frac = i as f32 / total as f32;
		let angle = sweep_angle(t.start_angle_deg, t.sweep_angle_deg, t.anti_clockwise, frac);

		if t.show_ticks {
			let (ox, oy) = polar(cx, cy, t.radius, angle);
			let (ix, iy) = polar(cx, cy, t.radius + dir * t.major_length, angle);
			canvas.draw_line(ox, oy, ix, iy, &Paint::stroke(tick_color, t.major_width));
		}

		if t.show_labels {
			let label_value = t.base.min_value + frac * (t.base.max_value - t.base.min_value);
			let label = format_number(label_value, "F0");
			let dist = t.radius + dir * (t.major_length + t.label_offset);
			let (lx, ly) = polar(cx, cy, dist, angle);
			let paint = Paint::fill(label_color);
			let tf = default_typeface();
			if t.radial_labels {
				canvas.save();
				canvas.translate(lx, ly);
				canvas.rotate_degrees(angle + 90.0);
				canvas.draw_text(
					&label,
					0.0,
					t.label_font_size / 3.0,
					TextAlign::Center,
					tf,
					t.label_font_size,
					&paint,
				);
				canvas.restore();
			} else {
				canvas.draw_text(
					&label,
					lx,
					ly + t.label_font_size / 3.0,
					TextAlign::Center,
					tf,
					t.label_font_size,
					&paint,
				);
			}
		}

		if i < total && t.minor_per_major > 0 && t.show_ticks {
			let paint = Paint::stroke(tick_color, t.minor_width);
			for j in 1..=t.minor_per_major {
				let mt = frac + (j as f32 / (t.minor_per_major + 1) as f32) / total as f32;
				let m_angle = sweep_angle(t.start_angle_deg, t.sweep_angle_deg, t.anti_clockwise, mt);
				let (ox, oy) = polar(cx, cy, t.radius, m_angle);
				let (ix, iy) = polar(cx, cy, t.radius + dir * t.minor_length, m_angle);
				canvas.draw_line(ox, oy, ix, iy, &paint);
			}
		}
	}
}

fn draw_circle(canvas: &mut Canvas, c: &CircleElement) {
	canvas.draw_circle(c.base.x, c.base.y, c.radius, &Paint::fill(color(&c.fill_color)));
	if c.circle_stroke_width > 0.0 {
		canvas.draw_circle(c.base.x, c.base.y, c.radius, &Paint::stroke(color(&c.stroke_color), c.circle_stroke_width));
	}
}

fn draw_rect_or_round(canvas: &mut Canvas, r: tiny_skia::Rect, corner: f32, paint: &Paint) {
	if corner > 0.0 {
		canvas.draw_round_rect(r, corner, corner, paint);
	} else {
		canvas.draw_rect(r, paint);
	}
}

fn draw_rectangle(canvas: &mut Canvas, r: &RectangleElement) {
	let Some(bounds) = rect(r.base.x, r.base.y, r.base.x + r.width, r.base.y + r.height) else {
		return;
	};
	draw_rect_or_round(canvas, bounds, r.corner_radius, &Paint::fill(color(&r.fill_color)));
	if r.rect_stroke_width > 0.0 {
		draw_rect_or_round(
			canvas,
			bounds,
			r.corner_radius,
			&Paint::stroke(color(&r.stroke_color), r.rect_stroke_width),
		);
	}
}

fn draw_line(canvas: &mut Canvas, l: &LineElement) {
	let p = Paint::stroke(color(&l.color), l.line_width).with_cap(LineCap::Round);
	canvas.draw_line(l.base.x, l.base.y, l.x2, l.y2, &p);
}

fn draw_linear_bar(canvas: &mut Canvas, bar: &LinearBarElement, value: f32) {
	const GLOW_ALPHA: u8 = 60;
	const GLOW_SIGMA: f32 = 6.0;

	let t = bar.base.normalized(value);
	let (x, y) = (bar.base.x, bar.base.y);
	let Some(track) = rect(x, y, x + bar.width, y + bar.height) else {
		return;
	};
	draw_rect_or_round(canvas, track, bar.corner_radius, &Paint::fill(color(&bar.track_color)));

	let (fl, ft, fr, fb) = if bar.is_vertical {
		let h = bar.height * t;
		(x, y + bar.height - h, x + bar.width, y + bar.height)
	} else {
		(x, y, x + bar.width * t, y + bar.height)
	};

	let mut fill_hex = &bar.fill_color;
	if bar.use_conditional_color && bar.base.has_source() {
		if value >= bar.danger_threshold {
			fill_hex = &bar.danger_color;
		} else if value >= bar.warn_threshold {
			fill_hex = &bar.warn_color;
		}
	}

	if fr - fl > 0.0 && fb - ft > 0.0 {
		if let Some(fill_rect) = rect(fl, ft, fr, fb) {
			let fill = color(fill_hex);
			canvas.save();
			if bar.corner_radius > 0.0 {
				if let Some(clip) = crate::canvas::round_rect_path(track, bar.corner_radius, bar.corner_radius) {
					canvas.clip_path(&clip);
				}
			}
			canvas.draw_rect(fill_rect, &Paint::fill(fill));
			canvas.draw_rect(
				fill_rect,
				&Paint::fill(fill.with_alpha(GLOW_ALPHA)).with_blur(Some(Blur::normal(GLOW_SIGMA))),
			);
			canvas.restore();
		}
	}

	if bar.border_width > 0.0 {
		draw_rect_or_round(
			canvas,
			track,
			bar.corner_radius,
			&Paint::stroke(color(&bar.border_color), bar.border_width),
		);
	}
}

fn draw_warning(canvas: &mut Canvas, w: &WarningIndicatorElement, value: f32) {
	const GLOW_ALPHA: u8 = 80;
	const GLOW_SIGMA: f32 = 10.0;
	const GLOW_RADIUS_EXTRA: f32 = 4.0;

	let active = if w.trigger_above { value >= w.threshold } else { value <= w.threshold };
	let c = color(if active { &w.active_color } else { &w.inactive_color });
	let (x, y) = (w.base.x, w.base.y);
	canvas.draw_circle(x, y, w.radius, &Paint::fill(c));

	if active {
		let glow = Paint::fill(color(&w.active_color).with_alpha(GLOW_ALPHA)).with_blur(Some(Blur::normal(GLOW_SIGMA)));
		canvas.draw_circle(x, y, w.radius + GLOW_RADIUS_EXTRA, &glow);
	}

	if w.show_label {
		let ly = y + w.radius + w.label_font_size + 4.0;
		let paint = Paint { style: PaintStyle::Fill, ..Paint::fill(color(&w.label_color)) };
		canvas.draw_text(&w.label, x, ly, TextAlign::Center, default_typeface(), w.label_font_size, &paint);
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn frame_selection_matches_csharp() {
		assert_eq!(select_frame(50.0, 0.0, 100.0, 1), 0);
		assert_eq!(select_frame(0.0, 0.0, 100.0, 5), 0);
		assert_eq!(select_frame(100.0, 0.0, 100.0, 5), 4);
		assert_eq!(select_frame(50.0, 0.0, 100.0, 5), 2);
		assert_eq!(select_frame(500.0, 0.0, 100.0, 5), 4);
		assert_eq!(select_frame(10.0, 5.0, 5.0, 5), 0);
	}
}
