//! Port of `ElementRenderer` and the per-element renderers in
//! `Gauges/Custom/Renderers`. Drawing math mirrors the C# code line for line
//! so definitions look the same on both implementations.

use std::collections::{HashMap, VecDeque};
use std::path::Path;
use std::time::Instant;

use chrono::NaiveDateTime;

use crate::canvas::{rect, Blur, Canvas, LineCap, LineJoin, Paint, PaintStyle};
use crate::color::Rgba;
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
#[derive(Default)]
pub struct ElementRenderer {
	images: ImageCache,
	graph_buffers: HashMap<String, VecDeque<f32>>,
	peak_state: HashMap<String, (f32, Instant)>,
	/// When set, clock elements show this time instead of the system clock
	/// (deterministic snapshots, designer previews).
	pub fixed_time: Option<NaiveDateTime>,
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
		let base_dir = definition.base_directory.as_deref();
		canvas.clear(color(&definition.background_color));

		if let Some(bg) = definition.background_image.as_deref().filter(|s| !s.is_empty()) {
			if let Some(img) = self.images.load(bg, base_dir) {
				draw_background(
					canvas,
					&img,
					definition.width as f32,
					definition.height as f32,
					definition.background_image_mode,
					definition.background_image_opacity,
				);
			}
		}

		// Calculated channels are merged into local copies so the caller's maps are untouched.
		let mut owned_values;
		let mut owned_targets;
		let (all_values, all_targets) = if definition.calculated_channels.is_empty() {
			(values, target_values)
		} else {
			owned_values = values.clone();
			owned_targets = target_values.cloned();
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
			(&owned_values, owned_targets.as_ref())
		};

		for element in &definition.elements {
			let base = element.base();
			if base.use_visibility {
				if let Some(src) = base.visibility_source.as_deref().filter(|s| !s.is_empty()) {
					let v = lookup(all_values, src).unwrap_or(0.0);
					if v < base.visible_above || v > base.visible_below {
						continue;
					}
				}
			}

			let source = match all_targets {
				Some(t) if element.uses_target_value() => t,
				_ => all_values,
			};
			let value = base.source().and_then(|s| lookup(source, s)).unwrap_or(0.0);
			self.draw_element(canvas, element, value, base_dir);
		}
	}

	/// Draws one element with the given value (`ElementRenderer.DrawElement`).
	pub fn draw_element(&mut self, canvas: &mut Canvas, element: &Element, value: f32, base_dir: Option<&Path>) {
		let opacity = element.base().opacity;
		let use_layer = opacity < 255;
		if use_layer {
			canvas.save_layer(opacity);
		}

		match element {
			Element::Arc(e) => draw_arc(canvas, e, value),
			Element::Needle(e) => self.draw_needle(canvas, e, value, base_dir),
			Element::Text(e) => draw_text(canvas, e),
			Element::Value(e) => draw_value(canvas, e, value),
			Element::Ticks(e) => draw_ticks(canvas, e),
			Element::Circle(e) => draw_circle(canvas, e),
			Element::Rectangle(e) => draw_rectangle(canvas, e),
			Element::Line(e) => draw_line(canvas, e),
			Element::LinearBar(e) => draw_linear_bar(canvas, e, value),
			Element::Warning(e) => draw_warning(canvas, e, value),
			Element::Image(e) => self.draw_image(canvas, e, base_dir),
			Element::ZoneArc(e) => draw_zone_arc(canvas, e, value),
			Element::Graph(e) => self.draw_graph(canvas, e, value),
			Element::LabelValue(e) => draw_label_value(canvas, e, value),
			Element::Peak(e) => self.draw_peak(canvas, e, value),
			Element::Gif(e) => self.draw_gif(canvas, e, value, base_dir),
			Element::Clock(e) => self.draw_clock(canvas, e),
		}

		if use_layer {
			canvas.restore();
		}
	}

	fn draw_needle(&mut self, canvas: &mut Canvas, n: &NeedleElement, value: f32, base_dir: Option<&Path>) {
		const GLOW_ALPHA: u8 = 50;
		const GLOW_SIGMA: f32 = 6.0;
		const GLOW_STROKE_EXTRA: f32 = 6.0;

		let t = if n.base.has_source() { n.base.normalized(value) } else { 0.0 };
		let angle = sweep_angle(n.start_angle_deg, n.sweep_angle_deg, n.anti_clockwise, t);
		let (x, y) = (n.base.x, n.base.y);

		if let Some(path) = n.image_path.as_deref().filter(|s| !s.is_empty()) {
			if let Some(img) = self.images.load(path, base_dir) {
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

	fn draw_image(&mut self, canvas: &mut Canvas, img: &ImageElement, base_dir: Option<&Path>) {
		if img.image_path.is_empty() {
			return;
		}
		if let Some(bitmap) = self.images.load(&img.image_path, base_dir) {
			draw_rotated_image(canvas, &bitmap, img.base.x, img.base.y, img.width, img.height, img.rotation);
		}
	}

	fn draw_gif(&mut self, canvas: &mut Canvas, gif: &GifElement, value: f32, base_dir: Option<&Path>) {
		if gif.image_path.is_empty() {
			return;
		}
		let Some(frames) = self.images.load_frames(&gif.image_path, base_dir) else {
			return;
		};
		if frames.is_empty() {
			return;
		}
		let index = select_frame(value, gif.base.min_value, gif.base.max_value, frames.len());
		draw_rotated_image(canvas, &frames[index], gif.base.x, gif.base.y, gif.width, gif.height, gif.rotation);
	}

	fn draw_graph(&mut self, canvas: &mut Canvas, g: &GraphElement, value: f32) {
		let (x, y) = (g.base.x, g.base.y);
		let Some(bounds) = rect(x, y, x + g.width, y + g.height) else {
			return;
		};
		canvas.draw_rect(bounds, &Paint::fill(color(&g.back_color)));

		let depth = g.history_depth.max(2) as usize;
		let buffer = self.graph_buffers.entry(g.base.id.clone()).or_default();
		buffer.push_back(value);
		while buffer.len() > depth {
			buffer.pop_front();
		}
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

	fn draw_peak(&mut self, canvas: &mut Canvas, p: &PeakMarkerElement, value: f32) {
		if !p.base.has_source() {
			return;
		}
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

		let t = p.base.normalized(peak);
		let angle = sweep_angle(p.start_angle_deg, p.sweep_angle_deg, p.anti_clockwise, t);
		let (ix, iy) = polar(p.base.x, p.base.y, p.radius - p.stroke_width / 2.0, angle);
		let (ox, oy) = polar(p.base.x, p.base.y, p.radius + p.stroke_width / 2.0, angle);
		canvas.draw_line(ix, iy, ox, oy, &Paint::stroke(color(&p.marker_color), p.marker_width));
	}

	fn draw_clock(&mut self, canvas: &mut Canvas, c: &ClockElement) {
		let now = self.fixed_time.unwrap_or_else(|| {
			if c.use_utc {
				chrono::Utc::now().naive_utc()
			} else {
				chrono::Local::now().naive_local()
			}
		});
		let text =
			format_datetime(&now, &c.format).unwrap_or_else(|| format_datetime(&now, "HH:mm:ss").unwrap_or_default());
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
