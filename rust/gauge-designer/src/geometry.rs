//! Element bounds, hit testing and resizing for the editor. Port of the C#
//! designer's `Rendering/ElementRenderer.cs` (`GetBounds`, `HitTest`,
//! `SetBounds`), plus moving and the angle handles the C# designer lacked.

use gauge_render::format::format_datetime;
use gauge_render::model::*;
use gauge_render::text::typeface;

/// Axis-aligned rectangle in canvas pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bounds {
	pub left: f32,
	pub top: f32,
	pub right: f32,
	pub bottom: f32,
}

impl Bounds {
	pub fn new(left: f32, top: f32, right: f32, bottom: f32) -> Self {
		Self { left, top, right, bottom }
	}

	pub fn around(cx: f32, cy: f32, half_w: f32, half_h: f32) -> Self {
		Self::new(cx - half_w, cy - half_h, cx + half_w, cy + half_h)
	}

	pub fn width(&self) -> f32 {
		self.right - self.left
	}

	pub fn height(&self) -> f32 {
		self.bottom - self.top
	}

	pub fn center_x(&self) -> f32 {
		(self.left + self.right) / 2.0
	}

	pub fn center_y(&self) -> f32 {
		(self.top + self.bottom) / 2.0
	}

	pub fn inflate(&self, d: f32) -> Self {
		Self::new(self.left - d, self.top - d, self.right + d, self.bottom + d)
	}

	pub fn contains(&self, x: f32, y: f32) -> bool {
		x >= self.left && x <= self.right && y >= self.top && y <= self.bottom
	}

	pub fn intersects(&self, o: &Bounds) -> bool {
		self.left <= o.right && o.left <= self.right && self.top <= o.bottom && o.top <= self.bottom
	}

	pub fn union(&self, o: &Bounds) -> Self {
		Self::new(self.left.min(o.left), self.top.min(o.top), self.right.max(o.right), self.bottom.max(o.bottom))
	}

	/// Rectangle spanned by two corners, in any order.
	pub fn from_corners(ax: f32, ay: f32, bx: f32, by: f32) -> Self {
		Self::new(ax.min(bx), ay.min(by), ax.max(bx), ay.max(by))
	}
}

fn text_box(x: f32, y: f32, text: &str, font: &str, size: f32) -> Bounds {
	let width = typeface(font).measure(text, size);
	Bounds::new(x - width / 2.0, y - size, x + width / 2.0, y + size * 0.3)
}

fn ring(cx: f32, cy: f32, radius: f32, stroke: f32) -> Bounds {
	let r = radius + stroke / 2.0;
	Bounds::around(cx, cy, r, r)
}

/// Sample text used to size a clock (the current time in its format).
pub fn clock_sample(clock: &ClockElement) -> String {
	let now = chrono::Local::now().naive_local();
	format_datetime(&now, &clock.format).unwrap_or_else(|| "88:88:88".into())
}

/// `GetBounds`: the box the selection outline and resize handles use.
pub fn bounds(element: &Element) -> Bounds {
	let b = element.base();
	let (x, y) = (b.x, b.y);
	match element {
		Element::Arc(a) => ring(x, y, a.radius, a.stroke_width),
		Element::ZoneArc(z) => ring(x, y, z.radius, z.stroke_width),
		Element::Peak(p) => ring(x, y, p.radius, p.stroke_width),
		Element::Needle(n) => Bounds::around(x, y, n.length, n.length),
		Element::Text(t) => text_box(x, y, &t.text, &t.font, t.font_size),
		Element::Clock(c) => text_box(x, y, &clock_sample(c), &c.font, c.font_size),
		Element::Value(v) => text_box(x, y, &format!("8888{}", v.suffix), &v.font, v.font_size),
		Element::Ticks(t) => Bounds::around(x, y, t.radius, t.radius),
		Element::Circle(c) => Bounds::around(x, y, c.radius, c.radius),
		Element::Rectangle(r) => Bounds::new(x, y, x + r.width, y + r.height),
		Element::Line(l) => Bounds::new(
			x.min(l.x2) - l.line_width,
			y.min(l.y2) - l.line_width,
			x.max(l.x2) + l.line_width,
			y.max(l.y2) + l.line_width,
		),
		Element::LinearBar(bar) => Bounds::new(x, y, x + bar.width, y + bar.height),
		Element::Warning(w) => Bounds::new(
			x - w.radius,
			y - w.radius,
			x + w.radius,
			y + w.radius + if w.show_label { w.label_font_size + 8.0 } else { 0.0 },
		),
		Element::Image(i) => Bounds::new(x, y, x + i.width, y + i.height),
		Element::Gif(g) => Bounds::new(x, y, x + g.width, y + g.height),
		Element::Graph(g) => Bounds::new(x, y, x + g.width, y + g.height),
		Element::LabelValue(lv) => {
			let half_h = (lv.label_font_size + 4.0 + lv.value_font_size) / 2.0 + lv.box_padding;
			Bounds::around(x, y, lv.value_font_size * 3.0, half_h)
		}
	}
}

fn in_ring(cx: f32, cy: f32, px: f32, py: f32, inner: f32, outer: f32) -> bool {
	let dist = ((px - cx).powi(2) + (py - cy).powi(2)).sqrt();
	dist >= inner && dist <= outer
}

/// `HitTest`: ring-shaped elements are only hit near their ring, so the
/// needle and value inside a dial stay clickable.
pub fn hit_test(element: &Element, px: f32, py: f32) -> bool {
	let b = element.base();
	match element {
		Element::Arc(a) => {
			in_ring(b.x, b.y, px, py, a.radius - a.stroke_width - 10.0, a.radius + a.stroke_width + 10.0)
		}
		Element::ZoneArc(z) => {
			in_ring(b.x, b.y, px, py, z.radius - z.stroke_width - 10.0, z.radius + z.stroke_width + 10.0)
		}
		Element::Peak(p) => {
			in_ring(b.x, b.y, px, py, p.radius - p.stroke_width - 10.0, p.radius + p.stroke_width + 10.0)
		}
		Element::Ticks(t) => in_ring(b.x, b.y, px, py, t.radius - t.major_length - 30.0, t.radius + 10.0),
		_ => bounds(element).inflate(8.0).contains(px, py),
	}
}

/// `SetBounds`: maps a target rectangle onto the element's size properties.
/// Round elements take the largest circle that fits; text takes the height.
pub fn set_bounds(element: &mut Element, r: Bounds) {
	let (cx, cy) = (r.center_x(), r.center_y());
	let (w, h) = (r.width().max(2.0), r.height().max(2.0));
	let radius = (w.min(h) / 2.0).max(5.0);
	let small_radius = (w.min(h) / 2.0).max(2.0);
	let text_size = (h / 1.3).max(6.0);
	match element {
		Element::Arc(a) => (a.base.x, a.base.y, a.radius) = (cx, cy, radius),
		Element::ZoneArc(z) => (z.base.x, z.base.y, z.radius) = (cx, cy, radius),
		Element::Peak(p) => (p.base.x, p.base.y, p.radius) = (cx, cy, radius),
		Element::Ticks(t) => (t.base.x, t.base.y, t.radius) = (cx, cy, radius),
		Element::Circle(c) => (c.base.x, c.base.y, c.radius) = (cx, cy, small_radius),
		Element::Warning(wi) => (wi.base.x, wi.base.y, wi.radius) = (cx, cy, small_radius),
		Element::Needle(n) => (n.base.x, n.base.y, n.length) = (cx, cy, radius),
		Element::Rectangle(e) => (e.base.x, e.base.y, e.width, e.height) = (r.left, r.top, w, h),
		Element::LinearBar(e) => (e.base.x, e.base.y, e.width, e.height) = (r.left, r.top, w, h),
		Element::Image(e) => (e.base.x, e.base.y, e.width, e.height) = (r.left, r.top, w, h),
		Element::Gif(e) => (e.base.x, e.base.y, e.width, e.height) = (r.left, r.top, w, h),
		Element::Graph(e) => (e.base.x, e.base.y, e.width, e.height) = (r.left, r.top, w, h),
		Element::Line(l) => {
			// Keep the line's direction (the C# designer always made it run
			// top-left to bottom-right).
			let pad = l.line_width;
			let (left, right) = (r.left + pad, (r.right - pad).max(r.left + pad));
			let (top, bottom) = (r.top + pad, (r.bottom - pad).max(r.top + pad));
			let (x0, x1) = if l.base.x <= l.x2 { (left, right) } else { (right, left) };
			let (y0, y1) = if l.base.y <= l.y2 { (top, bottom) } else { (bottom, top) };
			(l.base.x, l.base.y, l.x2, l.y2) = (x0, y0, x1, y1);
		}
		Element::Text(t) => t.font_size = text_size,
		Element::Value(v) => v.font_size = text_size,
		Element::Clock(c) => c.font_size = text_size,
		Element::LabelValue(lv) => {
			lv.value_font_size = (h * 0.6).max(6.0);
			lv.label_font_size = (h * 0.25).max(6.0);
		}
	}
}

/// Resizes by dragging `corner` (0 TL, 1 TR, 2 BL, 3 BR) of `start` to
/// (`mx`, `my`); the opposite corner stays where it was.
pub fn resize_from_corner(element: &mut Element, start: Bounds, corner: usize, mx: f32, my: f32) {
	let anchor_x = if corner == 0 || corner == 2 { start.right } else { start.left };
	let anchor_y = if corner == 0 || corner == 1 { start.bottom } else { start.top };
	set_bounds(element, Bounds::from_corners(anchor_x, anchor_y, mx, my));
	// SetBounds may not reproduce the rect (round elements stay square, text
	// keeps its width), so shift the element until the anchor is back.
	let actual = bounds(element);
	let ax = if corner == 0 || corner == 2 { actual.right } else { actual.left };
	let ay = if corner == 0 || corner == 1 { actual.bottom } else { actual.top };
	translate(element, anchor_x - ax, anchor_y - ay);
}

/// Moves an element. Lines move both ends (the C# designer only moved the start).
pub fn translate(element: &mut Element, dx: f32, dy: f32) {
	let b = element.base_mut();
	b.x += dx;
	b.y += dy;
	if let Element::Line(l) = element {
		l.x2 += dx;
		l.y2 += dy;
	}
}

/// What an angle handle edits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AngleHandle {
	/// Rotates the sweep (moves `StartAngleDeg`, keeps the sweep).
	Start,
	/// Moves the end of the sweep (changes `SweepAngleDeg`).
	End,
	/// `Rotation` of an image or GIF.
	Rotation,
}

/// A dial's sweep: centre, radius, start angle, sweep and direction (+1 / -1).
struct Sweep {
	cx: f32,
	cy: f32,
	radius: f32,
	start: f32,
	sweep: f32,
	sign: f32,
}

fn sweep_of(element: &Element) -> Option<Sweep> {
	let b = element.base();
	let (radius, start, sweep, acw) = match element {
		Element::Arc(a) => (a.radius, a.start_angle_deg, a.sweep_angle_deg, a.anti_clockwise),
		Element::ZoneArc(z) => (z.radius, z.start_angle_deg, z.sweep_angle_deg, z.anti_clockwise),
		Element::Peak(p) => (p.radius, p.start_angle_deg, p.sweep_angle_deg, p.anti_clockwise),
		Element::Ticks(t) => (t.radius, t.start_angle_deg, t.sweep_angle_deg, t.anti_clockwise),
		Element::Needle(n) => (n.length, n.start_angle_deg, n.sweep_angle_deg, n.anti_clockwise),
		_ => return None,
	};
	Some(Sweep { cx: b.x, cy: b.y, radius, start, sweep, sign: if acw { -1.0 } else { 1.0 } })
}

fn polar(cx: f32, cy: f32, radius: f32, deg: f32) -> (f32, f32) {
	let a = deg.to_radians();
	(cx + a.cos() * radius, cy + a.sin() * radius)
}

/// Images and GIFs rotate around the centre of their box: (cx, cy, half height, rotation).
fn rotation_of(element: &Element) -> Option<(f32, f32, f32, f32)> {
	let (x, y, w, h, rotation) = match element {
		Element::Image(i) => (i.base.x, i.base.y, i.width, i.height, i.rotation),
		Element::Gif(g) => (g.base.x, g.base.y, g.width, g.height, g.rotation),
		_ => return None,
	};
	Some((x + w / 2.0, y + h / 2.0, h / 2.0, rotation))
}

/// Distance of the rotation handle above an image's top edge.
const ROTATION_HANDLE_OFFSET: f32 = 24.0;

/// Angle handles of an element, with their canvas positions. Dials get a
/// start and an end handle on their radius; images a rotation handle above them.
pub fn angle_handles(element: &Element) -> Vec<(AngleHandle, f32, f32)> {
	if let Some(s) = sweep_of(element) {
		let (sx, sy) = polar(s.cx, s.cy, s.radius, s.start);
		let (ex, ey) = polar(s.cx, s.cy, s.radius, s.start + s.sweep * s.sign);
		return vec![(AngleHandle::Start, sx, sy), (AngleHandle::End, ex, ey)];
	}
	if let Some((cx, cy, half_h, rotation)) = rotation_of(element) {
		let (hx, hy) = polar(cx, cy, half_h + ROTATION_HANDLE_OFFSET, rotation - 90.0);
		return vec![(AngleHandle::Rotation, hx, hy)];
	}
	Vec::new()
}

/// Centre the angle handles turn around.
pub fn angle_center(element: &Element) -> Option<(f32, f32)> {
	sweep_of(element).map(|s| (s.cx, s.cy)).or_else(|| rotation_of(element).map(|r| (r.0, r.1)))
}

/// Drags `handle` to (`px`, `py`). Angles snap to multiples of `snap_deg`
/// (use 1 for whole degrees).
pub fn apply_angle(element: &mut Element, handle: AngleHandle, px: f32, py: f32, snap_deg: f32) {
	let Some((cx, cy)) = angle_center(element) else { return };
	let raw = (py - cy).atan2(px - cx).to_degrees();
	let snap = |deg: f32| (deg / snap_deg).round() * snap_deg;
	match handle {
		AngleHandle::Start => {
			let start = snap(raw).rem_euclid(360.0);
			set_sweep(element, Some(start), None);
		}
		AngleHandle::End => {
			let Some(s) = sweep_of(element) else { return };
			let mut sweep = snap(((raw - s.start) * s.sign).rem_euclid(360.0));
			if sweep < 1.0 {
				// Past the start: a full circle if it was more than half, else the minimum.
				sweep = if s.sweep > 180.0 { 360.0 } else { 1.0 };
			}
			set_sweep(element, None, Some(sweep));
		}
		AngleHandle::Rotation => {
			let mut rotation = snap(raw + 90.0).rem_euclid(360.0);
			if rotation > 180.0 {
				rotation -= 360.0;
			}
			match element {
				Element::Image(i) => i.rotation = rotation,
				Element::Gif(g) => g.rotation = rotation,
				_ => {}
			}
		}
	}
}

fn set_sweep(element: &mut Element, start: Option<f32>, sweep: Option<f32>) {
	let (s, w) = match element {
		Element::Arc(a) => (&mut a.start_angle_deg, &mut a.sweep_angle_deg),
		Element::ZoneArc(z) => (&mut z.start_angle_deg, &mut z.sweep_angle_deg),
		Element::Peak(p) => (&mut p.start_angle_deg, &mut p.sweep_angle_deg),
		Element::Ticks(t) => (&mut t.start_angle_deg, &mut t.sweep_angle_deg),
		Element::Needle(n) => (&mut n.start_angle_deg, &mut n.sweep_angle_deg),
		_ => return,
	};
	if let Some(v) = start {
		*s = v;
	}
	if let Some(v) = sweep {
		*w = v;
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn all_types() -> Vec<Element> {
		vec![
			Element::Arc(ArcElement::default()),
			Element::Needle(NeedleElement::default()),
			Element::Text(TextElement::default()),
			Element::Value(ValueDisplayElement::default()),
			Element::Ticks(TickRingElement::default()),
			Element::Circle(CircleElement::default()),
			Element::Rectangle(RectangleElement::default()),
			Element::Line(LineElement::default()),
			Element::LinearBar(LinearBarElement::default()),
			Element::Warning(WarningIndicatorElement::default()),
			Element::Image(ImageElement::default()),
			Element::ZoneArc(ZoneArcElement::default()),
			Element::Graph(GraphElement::default()),
			Element::LabelValue(LabelValueElement::default()),
			Element::Peak(PeakMarkerElement::default()),
			Element::Gif(GifElement::default()),
			Element::Clock(ClockElement::default()),
		]
	}

	#[test]
	fn every_type_has_bounds_and_is_hit_on_its_outline() {
		for el in all_types() {
			let b = bounds(&el);
			assert!(b.width() > 0.0 && b.height() > 0.0, "{}", el.type_label());
			// Every element is hit somewhere on the right edge of its box.
			let hit = (0..=20).any(|i| hit_test(&el, b.right, b.top + b.height() * i as f32 / 20.0));
			assert!(hit, "{}", el.type_label());
		}
	}

	#[test]
	fn arcs_are_hit_on_the_ring_only() {
		let arc = Element::Arc(ArcElement::default()); // r 200, stroke 30, at 320,240
		assert!(hit_test(&arc, 520.0, 240.0));
		assert!(!hit_test(&arc, 320.0, 240.0), "the centre stays free for the needle");
	}

	#[test]
	fn resizing_keeps_the_opposite_corner() {
		for mut el in all_types() {
			if matches!(el, Element::Text(_) | Element::Value(_) | Element::Clock(_) | Element::LabelValue(_)) {
				continue; // text scales with the height only
			}
			let start = bounds(&el);
			resize_from_corner(&mut el, start, 3, start.right + 40.0, start.bottom + 40.0);
			let after = bounds(&el);
			assert!((after.left - start.left).abs() < 0.01, "{} left", el.type_label());
			assert!((after.top - start.top).abs() < 0.01, "{} top", el.type_label());
			assert!(after.width() > start.width(), "{} grew", el.type_label());
		}
	}

	#[test]
	fn rect_resize_from_top_left() {
		let mut el = Element::Rectangle(RectangleElement::default()); // 320,240 100x60
		let start = bounds(&el);
		resize_from_corner(&mut el, start, 0, 300.0, 200.0);
		let Element::Rectangle(r) = &el else { unreachable!() };
		assert_eq!((r.base.x, r.base.y, r.width, r.height), (300.0, 200.0, 120.0, 100.0));
	}

	#[test]
	fn moving_a_line_moves_both_ends() {
		let mut el = Element::Line(LineElement::default());
		translate(&mut el, 10.0, 5.0);
		let Element::Line(l) = &el else { unreachable!() };
		assert_eq!((l.base.x, l.base.y, l.x2, l.y2), (330.0, 245.0, 430.0, 245.0));
	}

	#[test]
	fn angle_handles_follow_the_sweep() {
		let mut el = Element::Arc(ArcElement::default()); // start 135, sweep 270
		let handles = angle_handles(&el);
		assert_eq!(handles.len(), 2);
		// Drag the end handle to straight down (90°): sweep = 90 - 135 = -45 → 315.
		apply_angle(&mut el, AngleHandle::End, 320.0, 440.0, 1.0);
		let Element::Arc(a) = &el else { unreachable!() };
		assert_eq!(a.sweep_angle_deg, 315.0);
		// Drag the start handle to the right (0°).
		apply_angle(&mut el, AngleHandle::Start, 500.0, 240.0, 1.0);
		let Element::Arc(a) = &el else { unreachable!() };
		assert_eq!((a.start_angle_deg, a.sweep_angle_deg), (0.0, 315.0));
	}

	#[test]
	fn image_rotation_handle() {
		let mut el = Element::Image(ImageElement::default()); // 320,240 100x100 → centre 370,290
		let (_, hx, hy) = angle_handles(&el)[0];
		assert!((hx - 370.0).abs() < 0.01 && hy < 240.0);
		apply_angle(&mut el, AngleHandle::Rotation, 470.0, 290.0, 1.0); // to the right: 90°
		let Element::Image(i) = &el else { unreachable!() };
		assert_eq!(i.rotation, 90.0);
	}
}
