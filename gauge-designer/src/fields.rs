//! The property table: one line per editable property, per element type.
//!
//! The editor works on an element's JSON form (the `$type` / PascalCase
//! shape the files use), so a property is just its JSON key, a label and a
//! widget kind. Adding a property to the model needs one line here.

use serde_json::Value;

/// How a property is edited.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Kind {
	Text,
	/// Number with a soft range and drag step.
	Float(f32, f32, f32),
	Int(i32, i32),
	/// 0–255 slider (opacity).
	Byte,
	Bool,
	/// `#RRGGBB` / `#AARRGGBB`.
	Color,
	/// Channel name, or none.
	Source,
	Font,
	/// Image path; `true` when the property may be absent (`null`).
	Image(bool),
	/// Visibility limit; `true` for the lower one (off = `float.MinValue`).
	Limit(bool),
	/// .NET numeric format string (`F1`, `0.00`).
	NumberFormat,
	/// .NET date/time format string (`HH:mm`).
	DateFormat,
	/// `BackgroundImageMode` (stored as its number).
	ImageMode,
	/// Section break with a heading.
	Section,
}

#[derive(Clone, Copy, Debug)]
pub struct Field {
	/// JSON key (PascalCase), or the heading of a [`Kind::Section`].
	pub key: &'static str,
	pub label: &'static str,
	pub kind: Kind,
}

const fn f(key: &'static str, label: &'static str, kind: Kind) -> Field {
	Field { key, label, kind }
}

const fn section(title: &'static str) -> Field {
	Field { key: title, label: title, kind: Kind::Section }
}

use Kind::*;

const ANGLE: Kind = Float(-360.0, 360.0, 1.0);
const BIG: Kind = Float(-100_000.0, 100_000.0, 1.0);
const POS: Kind = Float(-640.0, 1280.0, 1.0);

const HEAD: &[Field] = &[f("Name", "Name", Text), f("X", "X", POS), f("Y", "Y", POS)];
const DATA: &[Field] = &[f("DataSource", "Data source", Source)];
const RANGE: &[Field] = &[f("MinValue", "Min value", BIG), f("MaxValue", "Max value", BIG)];
const COMMON: &[Field] = &[
	f("Opacity", "Opacity", Byte),
	section("Visibility"),
	f("UseVisibility", "Use visibility condition", Bool),
	f("VisibilitySource", "Visibility source", Source),
	f("VisibleAbove", "Visible above", Limit(true)),
	f("VisibleBelow", "Visible below", Limit(false)),
];

const ARC: &[Field] = &[
	f("Radius", "Radius", Float(1.0, 400.0, 1.0)),
	f("StrokeWidth", "Stroke width", Float(1.0, 120.0, 1.0)),
	f("StartAngleDeg", "Start angle", ANGLE),
	f("SweepAngleDeg", "Sweep angle", Float(1.0, 360.0, 1.0)),
	f("Color", "Colour", Color),
	f("TrackColor", "Track colour", Color),
	f("ShowTrack", "Show track", Bool),
	f("AntiClockwise", "Anti-clockwise", Bool),
	f("IsDynamic", "Dynamic (value-driven)", Bool),
	f("SegmentCount", "Segments (0 = continuous)", Int(0, 200)),
	f("SegmentGap", "Segment gap", Float(0.0, 40.0, 0.5)),
	section("Conditional colour"),
	f("UseConditionalColor", "Conditional colour", Bool),
	f("WarnThreshold", "Warn threshold", BIG),
	f("WarnColor", "Warn colour", Color),
	f("DangerThreshold", "Danger threshold", BIG),
	f("DangerColor", "Danger colour", Color),
];

const NEEDLE: &[Field] = &[
	f("Length", "Length", Float(1.0, 400.0, 1.0)),
	f("TailLength", "Tail length", Float(0.0, 200.0, 1.0)),
	f("NeedleWidth", "Width", Float(0.5, 40.0, 0.5)),
	f("StartAngleDeg", "Start angle", ANGLE),
	f("SweepAngleDeg", "Sweep angle", Float(1.0, 360.0, 1.0)),
	f("AntiClockwise", "Anti-clockwise", Bool),
	f("Color", "Colour", Color),
	f("ShowHub", "Show hub", Bool),
	f("HubRadius", "Hub radius", Float(0.0, 60.0, 1.0)),
	f("HubColor", "Hub colour", Color),
	section("Needle image"),
	f("ImagePath", "Image", Image(true)),
	f("ImageWidth", "Image width", Float(1.0, 200.0, 1.0)),
	f("ImageLength", "Image length", Float(1.0, 600.0, 1.0)),
];

const TEXT: &[Field] = &[
	f("Text", "Text", Text),
	f("FontSize", "Font size", Float(4.0, 300.0, 1.0)),
	f("Color", "Colour", Color),
	f("Font", "Font", Font),
	section("Box"),
	f("ShowBox", "Show box", Bool),
	f("BoxColor", "Box colour", Color),
	f("BoxPadding", "Box padding", Float(0.0, 80.0, 1.0)),
	f("BoxCornerRadius", "Box corner radius", Float(0.0, 60.0, 1.0)),
];

const VALUE: &[Field] = &[
	f("FontSize", "Font size", Float(4.0, 300.0, 1.0)),
	f("Color", "Colour", Color),
	f("Font", "Font", Font),
	f("Format", "Format (F0, F1, 0.00)", NumberFormat),
	f("Suffix", "Suffix", Text),
];

const TICKS: &[Field] = &[
	f("Radius", "Radius", Float(1.0, 400.0, 1.0)),
	f("StartAngleDeg", "Start angle", ANGLE),
	f("SweepAngleDeg", "Sweep angle", Float(1.0, 360.0, 1.0)),
	f("AntiClockwise", "Anti-clockwise", Bool),
	f("MajorCount", "Major ticks", Int(2, 60)),
	f("MinorPerMajor", "Minor per major", Int(0, 20)),
	f("MajorLength", "Major length", Float(1.0, 100.0, 1.0)),
	f("MinorLength", "Minor length", Float(1.0, 60.0, 1.0)),
	f("MajorWidth", "Major width", Float(0.5, 20.0, 0.5)),
	f("MinorWidth", "Minor width", Float(0.5, 10.0, 0.5)),
	f("Color", "Colour", Color),
	f("ShowTicks", "Show ticks", Bool),
	f("TicksInside", "Ticks inside", Bool),
	section("Labels"),
	f("ShowLabels", "Show labels", Bool),
	f("RadialLabels", "Radial labels", Bool),
	f("LabelFontSize", "Label font size", Float(4.0, 80.0, 1.0)),
	f("LabelColor", "Label colour", Color),
	f("LabelOffset", "Label offset", Float(-100.0, 150.0, 1.0)),
];

const CIRCLE: &[Field] = &[
	f("Radius", "Radius", Float(1.0, 400.0, 1.0)),
	f("FillColor", "Fill colour", Color),
	f("StrokeColor", "Stroke colour", Color),
	f("CircleStrokeWidth", "Stroke width", Float(0.0, 40.0, 0.5)),
];

const RECT: &[Field] = &[
	f("Width", "Width", Float(1.0, 1280.0, 1.0)),
	f("Height", "Height", Float(1.0, 960.0, 1.0)),
	f("FillColor", "Fill colour", Color),
	f("StrokeColor", "Stroke colour", Color),
	f("RectStrokeWidth", "Stroke width", Float(0.0, 40.0, 0.5)),
	f("CornerRadius", "Corner radius", Float(0.0, 200.0, 1.0)),
];

const LINE: &[Field] = &[
	f("X2", "End X", POS),
	f("Y2", "End Y", POS),
	f("LineWidth", "Width", Float(0.5, 60.0, 0.5)),
	f("Color", "Colour", Color),
];

const BAR: &[Field] = &[
	f("Width", "Width", Float(1.0, 1280.0, 1.0)),
	f("Height", "Height", Float(1.0, 960.0, 1.0)),
	f("IsVertical", "Vertical", Bool),
	f("FillColor", "Fill colour", Color),
	f("TrackColor", "Track colour", Color),
	f("BorderColor", "Border colour", Color),
	f("BorderWidth", "Border width", Float(0.0, 20.0, 0.5)),
	f("CornerRadius", "Corner radius", Float(0.0, 100.0, 1.0)),
	f("SegmentCount", "Segments (0 = continuous)", Int(0, 200)),
	f("SegmentGap", "Segment gap", Float(0.0, 40.0, 0.5)),
	section("Conditional colour"),
	f("UseConditionalColor", "Conditional colour", Bool),
	f("WarnThreshold", "Warn threshold", BIG),
	f("WarnColor", "Warn colour", Color),
	f("DangerThreshold", "Danger threshold", BIG),
	f("DangerColor", "Danger colour", Color),
];

const WARNING: &[Field] = &[
	f("Radius", "Radius", Float(1.0, 200.0, 1.0)),
	f("Threshold", "Threshold", BIG),
	f("TriggerAbove", "Trigger above", Bool),
	f("ActiveColor", "Active colour", Color),
	f("InactiveColor", "Inactive colour", Color),
	f("ShowLabel", "Show label", Bool),
	f("Label", "Label", Text),
	f("LabelFontSize", "Label size", Float(4.0, 80.0, 1.0)),
	f("LabelColor", "Label colour", Color),
];

const IMAGE: &[Field] = &[
	f("ImagePath", "Image", Image(false)),
	f("Width", "Width", Float(1.0, 1280.0, 1.0)),
	f("Height", "Height", Float(1.0, 960.0, 1.0)),
	f("Rotation", "Rotation", ANGLE),
];

const ZONE_ARC: &[Field] = &[
	f("Radius", "Radius", Float(1.0, 400.0, 1.0)),
	f("StrokeWidth", "Stroke width", Float(1.0, 120.0, 1.0)),
	f("StartAngleDeg", "Start angle", ANGLE),
	f("SweepAngleDeg", "Sweep angle", Float(1.0, 360.0, 1.0)),
	f("AntiClockwise", "Anti-clockwise", Bool),
	section("Zones"),
	f("Zone1Color", "Zone 1 colour", Color),
	f("ShowZone2", "Show zone 2", Bool),
	f("Zone2Start", "Zone 2 start (value)", BIG),
	f("Zone2Color", "Zone 2 colour", Color),
	f("ShowZone3", "Show zone 3", Bool),
	f("Zone3Start", "Zone 3 start (value)", BIG),
	f("Zone3Color", "Zone 3 colour", Color),
	section("Pointer"),
	f("ShowPointer", "Show pointer", Bool),
	f("PointerColor", "Pointer colour", Color),
	f("PointerWidth", "Pointer width", Float(0.5, 20.0, 0.5)),
];

const GRAPH: &[Field] = &[
	f("Width", "Width", Float(10.0, 1280.0, 1.0)),
	f("Height", "Height", Float(10.0, 960.0, 1.0)),
	f("HistoryDepth", "History depth", Int(2, 1000)),
	f("LineColor", "Line colour", Color),
	f("LineWidth", "Line width", Float(0.5, 20.0, 0.5)),
	f("ShowFill", "Show fill", Bool),
	f("FillColor", "Fill colour", Color),
	f("FillOpacity", "Fill opacity", Byte),
	f("BackColor", "Background colour", Color),
];

const PEAK: &[Field] = &[
	f("Radius", "Radius", Float(1.0, 400.0, 1.0)),
	f("StrokeWidth", "Stroke width", Float(1.0, 120.0, 1.0)),
	f("StartAngleDeg", "Start angle", ANGLE),
	f("SweepAngleDeg", "Sweep angle", Float(1.0, 360.0, 1.0)),
	f("AntiClockwise", "Anti-clockwise", Bool),
	f("MarkerColor", "Marker colour", Color),
	f("MarkerWidth", "Marker width", Float(0.5, 20.0, 0.5)),
	f("DecaySeconds", "Decay seconds (0 = hold)", Float(0.0, 600.0, 0.5)),
];

const LABEL_VALUE: &[Field] = &[
	f("Label", "Label", Text),
	f("LabelFontSize", "Label size", Float(4.0, 120.0, 1.0)),
	f("LabelColor", "Label colour", Color),
	f("LabelFont", "Label font", Font),
	section("Value"),
	f("ValueFontSize", "Value size", Float(4.0, 300.0, 1.0)),
	f("ValueColor", "Value colour", Color),
	f("ValueFont", "Value font", Font),
	f("ValueFormat", "Format", NumberFormat),
	f("ValueSuffix", "Suffix", Text),
	section("Box"),
	f("ShowBox", "Show box", Bool),
	f("BoxColor", "Box colour", Color),
	f("BoxPadding", "Box padding", Float(0.0, 80.0, 1.0)),
	f("BoxCornerRadius", "Box corner radius", Float(0.0, 60.0, 1.0)),
];

const GIF: &[Field] = &[
	f("ImagePath", "Animation (GIF / APNG / WebP)", Image(false)),
	f("Width", "Width", Float(1.0, 1280.0, 1.0)),
	f("Height", "Height", Float(1.0, 960.0, 1.0)),
	f("Rotation", "Rotation", ANGLE),
];

const CLOCK: &[Field] = &[
	f("Format", "Format (HH:mm:ss)", DateFormat),
	f("UseUtc", "Use UTC", Bool),
	f("FontSize", "Font size", Float(4.0, 300.0, 1.0)),
	f("Font", "Font", Font),
	f("Color", "Colour", Color),
	section("Box"),
	f("ShowBox", "Show box", Bool),
	f("BoxColor", "Box colour", Color),
	f("BoxPadding", "Box padding", Float(0.0, 80.0, 1.0)),
	f("BoxCornerRadius", "Box corner radius", Float(0.0, 60.0, 1.0)),
];

/// Canvas (background) properties of the definition itself.
pub const CANVAS: &[Field] = &[
	f("BackgroundColor", "Background colour", Color),
	f("BackgroundImage", "Background image", Image(true)),
	f("BackgroundImageMode", "Image mode", ImageMode),
	f("BackgroundImageOpacity", "Image opacity", Byte),
];

/// Names of `BackgroundImageMode` values, by number.
pub const IMAGE_MODES: [&str; 5] = ["Stretch", "Fill", "Fit", "Center", "Tile"];

/// `$type` names of the element types, with toolbox labels.
pub const TYPES: [(&str, &str); 17] = [
	("arc", "Arc"),
	("needle", "Needle"),
	("ticks", "Ticks"),
	("text", "Text"),
	("value", "Value"),
	("circle", "Circle"),
	("rectangle", "Rect"),
	("line", "Line"),
	("linearbar", "Bar"),
	("warning", "Warning"),
	("image", "Image"),
	("zonearc", "ZoneArc"),
	("graph", "Graph"),
	("labelvalue", "LabelValue"),
	("peak", "Peak"),
	("gif", "Gif"),
	("clock", "Clock"),
];

/// Types bound to a data source (the C# `supportsData` list).
fn has_data(tag: &str) -> bool {
	matches!(
		tag,
		"arc" | "needle" | "value" | "linearbar" | "warning" | "zonearc" | "graph" | "labelvalue" | "peak" | "gif"
	)
}

/// Types with a value range (`hasRange`): the data types plus tick rings.
fn has_range(tag: &str) -> bool {
	has_data(tag) || tag == "ticks"
}

fn specific(tag: &str) -> &'static [Field] {
	match tag {
		"arc" => ARC,
		"needle" => NEEDLE,
		"text" => TEXT,
		"value" => VALUE,
		"ticks" => TICKS,
		"circle" => CIRCLE,
		"rectangle" => RECT,
		"line" => LINE,
		"linearbar" => BAR,
		"warning" => WARNING,
		"image" => IMAGE,
		"zonearc" => ZONE_ARC,
		"graph" => GRAPH,
		"labelvalue" => LABEL_VALUE,
		"peak" => PEAK,
		"gif" => GIF,
		"clock" => CLOCK,
		_ => &[],
	}
}

/// Every property of an element type, in panel order.
pub fn fields_for(tag: &str) -> Vec<Field> {
	let mut out: Vec<Field> = HEAD.to_vec();
	if has_data(tag) {
		out.extend_from_slice(DATA);
	}
	if has_range(tag) {
		out.extend_from_slice(RANGE);
	}
	out.extend_from_slice(COMMON);
	out.push(section("Appearance"));
	out.extend_from_slice(specific(tag));
	out
}

/// The `$type` of an element's JSON form.
pub fn type_tag(value: &Value) -> &str {
	value.get("$type").and_then(Value::as_str).unwrap_or("")
}

/// A string property, treating `null` and missing as empty.
pub fn get_str<'a>(obj: &'a Value, key: &str) -> &'a str {
	obj.get(key).and_then(Value::as_str).unwrap_or("")
}

#[cfg(test)]
mod tests {
	use super::*;
	use gauge_render::model::*;

	fn sample(tag: &str) -> Element {
		serde_json::from_value(serde_json::json!({ "$type": tag })).unwrap()
	}

	/// Every key in the table must exist in the element's JSON, so a typo
	/// can't silently add a property the renderer ignores.
	#[test]
	fn every_field_exists_in_the_model() {
		for (tag, _) in TYPES {
			let value = serde_json::to_value(sample(tag)).unwrap();
			for field in fields_for(tag) {
				if field.kind != Section {
					assert!(value.get(field.key).is_some(), "{tag}.{}", field.key);
				}
			}
		}
		let canvas = serde_json::to_value(CustomGaugeDefinition::default()).unwrap();
		for field in CANVAS {
			assert!(canvas.get(field.key).is_some(), "canvas.{}", field.key);
		}
	}

	/// And every property of the model has a line in the table (Id is internal).
	#[test]
	fn every_model_property_is_editable() {
		for (tag, _) in TYPES {
			let value = serde_json::to_value(sample(tag)).unwrap();
			let fields = fields_for(tag);
			for key in value.as_object().unwrap().keys() {
				if key == "$type" || key == "Id" {
					continue;
				}
				// Elements without a data binding keep the base properties in JSON but don't edit them.
				if !has_range(tag) && matches!(key.as_str(), "DataSource" | "MinValue" | "MaxValue") {
					continue;
				}
				if tag == "ticks" && key == "DataSource" {
					continue;
				}
				assert!(fields.iter().any(|f| f.key == key), "{tag}.{key} has no editor line");
			}
		}
	}

	#[test]
	fn type_list_matches_the_model() {
		for (tag, label) in TYPES {
			assert_eq!(sample(tag).type_label(), label);
		}
	}
}
