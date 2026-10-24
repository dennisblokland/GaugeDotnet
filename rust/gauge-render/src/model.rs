//! Custom gauge definition model, JSON-compatible with the C# `GaugeElement`
//! hierarchy (`"$type"` discriminator, PascalCase properties) so files saved
//! by the Avalonia designer load unchanged.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

fn new_id() -> String {
	static COUNTER: AtomicU64 = AtomicU64::new(0);
	let seed =
		std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(0);
	let n = COUNTER.fetch_add(1, Ordering::Relaxed);
	let mixed = (seed ^ n.wrapping_mul(0x9E37_79B9_7F4A_7C15)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
	format!("{:08x}", (mixed >> 32) as u32)
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase", default)]
pub struct CalculatedChannel {
	pub name: String,
	pub expression: String,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BackgroundImageMode {
	#[default]
	Stretch,
	Fill,
	Fit,
	Center,
	Tile,
}

impl BackgroundImageMode {
	const ALL: [Self; 5] = [Self::Stretch, Self::Fill, Self::Fit, Self::Center, Self::Tile];

	fn name(self) -> &'static str {
		match self {
			Self::Stretch => "Stretch",
			Self::Fill => "Fill",
			Self::Fit => "Fit",
			Self::Center => "Center",
			Self::Tile => "Tile",
		}
	}
}

// System.Text.Json writes enums as numbers by default; accept names too.
impl<'de> Deserialize<'de> for BackgroundImageMode {
	fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
		#[derive(Deserialize)]
		#[serde(untagged)]
		enum Raw {
			Num(i64),
			Str(String),
		}
		Ok(match Raw::deserialize(d)? {
			Raw::Num(n) => Self::ALL.get(n as usize).copied().unwrap_or_default(),
			Raw::Str(s) => Self::ALL.into_iter().find(|m| m.name().eq_ignore_ascii_case(&s)).unwrap_or_default(),
		})
	}
}

impl Serialize for BackgroundImageMode {
	fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
		s.serialize_u8(*self as u8)
	}
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase", default)]
pub struct CustomGaugeDefinition {
	pub width: i32,
	pub height: i32,
	pub background_color: String,
	pub background_image: Option<String>,
	pub background_image_mode: BackgroundImageMode,
	pub background_image_opacity: u8,
	pub elements: Vec<Element>,
	pub calculated_channels: Vec<CalculatedChannel>,
	/// Directory relative image paths resolve against (the JSON file's folder).
	#[serde(skip)]
	pub base_directory: Option<PathBuf>,
}

impl Default for CustomGaugeDefinition {
	fn default() -> Self {
		Self {
			width: 640,
			height: 480,
			background_color: "#000000".into(),
			background_image: None,
			background_image_mode: BackgroundImageMode::Stretch,
			background_image_opacity: 255,
			elements: Vec::new(),
			calculated_channels: Vec::new(),
			base_directory: None,
		}
	}
}

impl CustomGaugeDefinition {
	pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
		serde_json::from_str(json)
	}

	/// Loads a definition and records its folder as the image base directory.
	pub fn load(path: impl AsRef<Path>) -> Result<Self, String> {
		let path = path.as_ref();
		let json = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
		let mut def =
			Self::from_json(json.trim_start_matches('\u{feff}')).map_err(|e| format!("{}: {e}", path.display()))?;
		def.base_directory = std::fs::canonicalize(path).ok().and_then(|p| p.parent().map(Path::to_path_buf));
		Ok(def)
	}

	pub fn to_json(&self) -> String {
		serde_json::to_string_pretty(self).expect("definition serialises")
	}
}

/// Properties shared by every element (`GaugeElement`).
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase", default)]
pub struct ElementBase {
	pub id: String,
	pub name: String,
	pub x: f32,
	pub y: f32,
	pub data_source: Option<String>,
	pub min_value: f32,
	pub max_value: f32,
	pub opacity: u8,
	pub use_visibility: bool,
	pub visibility_source: Option<String>,
	pub visible_above: f32,
	pub visible_below: f32,
}

impl Default for ElementBase {
	fn default() -> Self {
		Self {
			id: new_id(),
			name: String::new(),
			x: 320.0,
			y: 240.0,
			data_source: None,
			min_value: 0.0,
			max_value: 100.0,
			opacity: 255,
			use_visibility: false,
			visibility_source: None,
			visible_above: f32::MIN,
			visible_below: f32::MAX,
		}
	}
}

impl ElementBase {
	/// The bound data source, treating `""` as unbound like `string.IsNullOrEmpty`.
	pub fn source(&self) -> Option<&str> {
		self.data_source.as_deref().filter(|s| !s.is_empty())
	}

	pub fn has_source(&self) -> bool {
		self.source().is_some()
	}

	/// `Math.Clamp((value - Min) / (Max - Min), 0, 1)`, 0 when the range is empty.
	pub fn normalized(&self, value: f32) -> f32 {
		let range = self.max_value - self.min_value;
		if range > 0.0 {
			((value - self.min_value) / range).clamp(0.0, 1.0)
		} else {
			0.0
		}
	}
}

macro_rules! element {
	($(#[$meta:meta])* $name:ident { $($field:ident : $ty:ty = $default:expr),* $(,)? }) => {
		$(#[$meta])*
		#[derive(Clone, Debug, Serialize, Deserialize)]
		#[serde(rename_all = "PascalCase", default)]
		pub struct $name {
			#[serde(flatten)]
			pub base: ElementBase,
			$(pub $field: $ty,)*
		}

		impl Default for $name {
			fn default() -> Self {
				Self { base: ElementBase::default(), $($field: $default,)* }
			}
		}
	};
}

element!(
	/// Sweep arc with track, glow and value-driven fill.
	ArcElement {
		radius: f32 = 200.0,
		stroke_width: f32 = 30.0,
		start_angle_deg: f32 = 135.0,
		sweep_angle_deg: f32 = 270.0,
		color: String = "#00FFFF".into(),
		track_color: String = "#0A1A1A".into(),
		show_track: bool = true,
		is_dynamic: bool = true,
		anti_clockwise: bool = false,
		use_conditional_color: bool = false,
		warn_threshold: f32 = 80.0,
		warn_color: String = "#FFCC00".into(),
		danger_threshold: f32 = 95.0,
		danger_color: String = "#FF3333".into(),
		segment_count: i32 = 0,
		segment_gap: f32 = 2.0,
	}
);

element!(
	/// Rotating needle with tail and hub, optionally drawn from an image.
	NeedleElement {
		length: f32 = 180.0,
		tail_length: f32 = 25.0,
		needle_width: f32 = 4.0,
		start_angle_deg: f32 = 135.0,
		sweep_angle_deg: f32 = 270.0,
		anti_clockwise: bool = false,
		color: String = "#FF3333".into(),
		show_hub: bool = true,
		hub_radius: f32 = 10.0,
		hub_color: String = "#CCCCCC".into(),
		image_path: Option<String> = None,
		image_width: f32 = 20.0,
		image_length: f32 = 180.0,
	}
);

element!(
	/// Static label.
	TextElement {
		text: String = "Label".into(),
		font_size: f32 = 24.0,
		color: String = "#FFFFFF".into(),
		font: String = "Race Sport".into(),
		show_box: bool = false,
		box_color: String = "#1A1A1A".into(),
		box_padding: f32 = 6.0,
		box_corner_radius: f32 = 4.0,
	}
);

element!(
	/// Formatted numeric readout.
	ValueDisplayElement {
		font_size: f32 = 48.0,
		color: String = "#00FFFF".into(),
		font: String = "DSEG7 Classic".into(),
		format: String = "F0".into(),
		suffix: String = String::new(),
	}
);

element!(
	/// Major/minor tick marks with labels.
	TickRingElement {
		radius: f32 = 200.0,
		start_angle_deg: f32 = 135.0,
		sweep_angle_deg: f32 = 270.0,
		anti_clockwise: bool = false,
		major_count: i32 = 8,
		minor_per_major: i32 = 4,
		major_length: f32 = 20.0,
		minor_length: f32 = 10.0,
		major_width: f32 = 3.0,
		minor_width: f32 = 1.0,
		color: String = "#FFFFFF".into(),
		show_ticks: bool = true,
		ticks_inside: bool = false,
		show_labels: bool = true,
		radial_labels: bool = false,
		label_font_size: f32 = 14.0,
		label_color: String = "#CCCCCC".into(),
		label_offset: f32 = 25.0,
	}
);

element!(CircleElement {
	radius: f32 = 20.0,
	fill_color: String = "#333333".into(),
	stroke_color: String = "#666666".into(),
	circle_stroke_width: f32 = 2.0,
});

element!(RectangleElement {
	width: f32 = 100.0,
	height: f32 = 60.0,
	fill_color: String = "#222222".into(),
	stroke_color: String = "#666666".into(),
	rect_stroke_width: f32 = 1.0,
	corner_radius: f32 = 0.0,
});

element!(LineElement { x2: f32 = 420.0, y2: f32 = 240.0, line_width: f32 = 2.0, color: String = "#666666".into() });

element!(
	/// Horizontal or vertical fill bar.
	LinearBarElement {
		width: f32 = 200.0,
		height: f32 = 20.0,
		is_vertical: bool = false,
		fill_color: String = "#00FFFF".into(),
		track_color: String = "#0A1A1A".into(),
		border_color: String = "#444444".into(),
		border_width: f32 = 1.0,
		corner_radius: f32 = 2.0,
		use_conditional_color: bool = false,
		warn_threshold: f32 = 80.0,
		warn_color: String = "#FFCC00".into(),
		danger_threshold: f32 = 95.0,
		danger_color: String = "#FF3333".into(),
		segment_count: i32 = 0,
		segment_gap: f32 = 2.0,
	}
);

element!(
	/// Threshold-triggered lamp with label.
	WarningIndicatorElement {
		radius: f32 = 12.0,
		threshold: f32 = 80.0,
		trigger_above: bool = true,
		active_color: String = "#FF3333".into(),
		inactive_color: String = "#331111".into(),
		label: String = "WARN".into(),
		label_font_size: f32 = 12.0,
		label_color: String = "#FFFFFF".into(),
		show_label: bool = true,
	}
);

element!(ImageElement {
	image_path: String = String::new(),
	width: f32 = 100.0,
	height: f32 = 100.0,
	rotation: f32 = 0.0,
});

element!(
	/// Compound label + value; X/Y is the centre.
	LabelValueElement {
		label: String = "LABEL".into(),
		label_font_size: f32 = 14.0,
		label_color: String = "#888888".into(),
		label_font: String = "Race Sport".into(),
		value_font_size: f32 = 48.0,
		value_color: String = "#00FFFF".into(),
		value_font: String = "DSEG7 Classic".into(),
		value_format: String = "F0".into(),
		value_suffix: String = String::new(),
		show_box: bool = false,
		box_color: String = "#1A1A1A".into(),
		box_padding: f32 = 8.0,
		box_corner_radius: f32 = 4.0,
	}
);

element!(
	/// Arc with up to three coloured bands and an optional value pointer.
	ZoneArcElement {
		radius: f32 = 200.0,
		stroke_width: f32 = 30.0,
		start_angle_deg: f32 = 135.0,
		sweep_angle_deg: f32 = 270.0,
		anti_clockwise: bool = false,
		zone1_color: String = "#00CC00".into(),
		zone2_start: f32 = 70.0,
		zone2_color: String = "#FFCC00".into(),
		show_zone2: bool = true,
		zone3_start: f32 = 90.0,
		zone3_color: String = "#FF3333".into(),
		show_zone3: bool = true,
		show_pointer: bool = false,
		pointer_color: String = "#FFFFFF".into(),
		pointer_width: f32 = 3.0,
	}
);

element!(
	/// Rolling time-series plot.
	GraphElement {
		width: f32 = 200.0,
		height: f32 = 100.0,
		history_depth: i32 = 60,
		line_color: String = "#00FFFF".into(),
		fill_color: String = "#00FFFF".into(),
		fill_opacity: u8 = 40,
		back_color: String = "#0A1A1A".into(),
		line_width: f32 = 2.0,
		show_fill: bool = true,
	}
);

element!(
	/// Tick at the rolling peak value.
	PeakMarkerElement {
		radius: f32 = 200.0,
		stroke_width: f32 = 30.0,
		start_angle_deg: f32 = 135.0,
		sweep_angle_deg: f32 = 270.0,
		anti_clockwise: bool = false,
		marker_color: String = "#FFFFFF".into(),
		marker_width: f32 = 3.0,
		decay_seconds: f32 = 0.0,
	}
);

element!(
	/// Multi-frame image whose frame is picked by the value.
	GifElement {
		image_path: String = String::new(),
		width: f32 = 100.0,
		height: f32 = 100.0,
		rotation: f32 = 0.0,
	}
);

element!(
	/// Wall-clock time formatted with a .NET date format string.
	ClockElement {
		format: String = "HH:mm:ss".into(),
		use_utc: bool = false,
		font_size: f32 = 32.0,
		color: String = "#FFFFFF".into(),
		font: String = "DSEG7 Classic".into(),
		show_box: bool = false,
		box_color: String = "#1A1A1A".into(),
		box_padding: f32 = 6.0,
		box_corner_radius: f32 = 4.0,
	}
);

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "$type")]
pub enum Element {
	#[serde(rename = "arc")]
	Arc(ArcElement),
	#[serde(rename = "needle")]
	Needle(NeedleElement),
	#[serde(rename = "text")]
	Text(TextElement),
	#[serde(rename = "value")]
	Value(ValueDisplayElement),
	#[serde(rename = "ticks")]
	Ticks(TickRingElement),
	#[serde(rename = "circle")]
	Circle(CircleElement),
	#[serde(rename = "rectangle")]
	Rectangle(RectangleElement),
	#[serde(rename = "line")]
	Line(LineElement),
	#[serde(rename = "linearbar")]
	LinearBar(LinearBarElement),
	#[serde(rename = "warning")]
	Warning(WarningIndicatorElement),
	#[serde(rename = "image")]
	Image(ImageElement),
	#[serde(rename = "zonearc")]
	ZoneArc(ZoneArcElement),
	#[serde(rename = "graph")]
	Graph(GraphElement),
	#[serde(rename = "labelvalue")]
	LabelValue(LabelValueElement),
	#[serde(rename = "peak")]
	Peak(PeakMarkerElement),
	#[serde(rename = "gif")]
	Gif(GifElement),
	#[serde(rename = "clock")]
	Clock(ClockElement),
}

macro_rules! each_element {
	($self:expr, $e:ident => $body:expr) => {
		match $self {
			Element::Arc($e) => $body,
			Element::Needle($e) => $body,
			Element::Text($e) => $body,
			Element::Value($e) => $body,
			Element::Ticks($e) => $body,
			Element::Circle($e) => $body,
			Element::Rectangle($e) => $body,
			Element::Line($e) => $body,
			Element::LinearBar($e) => $body,
			Element::Warning($e) => $body,
			Element::Image($e) => $body,
			Element::ZoneArc($e) => $body,
			Element::Graph($e) => $body,
			Element::LabelValue($e) => $body,
			Element::Peak($e) => $body,
			Element::Gif($e) => $body,
			Element::Clock($e) => $body,
		}
	};
}

impl Element {
	pub fn base(&self) -> &ElementBase {
		each_element!(self, e => &e.base)
	}

	pub fn base_mut(&mut self) -> &mut ElementBase {
		each_element!(self, e => &mut e.base)
	}

	/// Display name used by the designer (`TypeLabel`).
	pub fn type_label(&self) -> &'static str {
		match self {
			Element::Arc(_) => "Arc",
			Element::Needle(_) => "Needle",
			Element::Text(_) => "Text",
			Element::Value(_) => "Value",
			Element::Ticks(_) => "Ticks",
			Element::Circle(_) => "Circle",
			Element::Rectangle(_) => "Rect",
			Element::Line(_) => "Line",
			Element::LinearBar(_) => "Bar",
			Element::Warning(_) => "Warning",
			Element::Image(_) => "Image",
			Element::ZoneArc(_) => "ZoneArc",
			Element::Graph(_) => "Graph",
			Element::LabelValue(_) => "LabelValue",
			Element::Peak(_) => "Peak",
			Element::Gif(_) => "Gif",
			Element::Clock(_) => "Clock",
		}
	}

	/// Text-like elements snap to the target value instead of the smoothed one.
	pub fn uses_target_value(&self) -> bool {
		matches!(self, Element::Value(_) | Element::Text(_) | Element::Warning(_))
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn parses_designer_json() {
		let json = r##"{
			"Width": 640, "Height": 480, "BackgroundColor": "#101010",
			"BackgroundImageMode": 2,
			"Elements": [
				{ "$type": "arc", "Id": "a1", "X": 320, "Y": 240, "DataSource": "Rpm",
				  "MaxValue": 8000, "AntiClockwise": true, "VisibleAbove": -3.4028235E+38 },
				{ "$type": "zonearc", "Zone2Start": 60, "ShowPointer": true },
				{ "$type": "clock", "Format": "HH:mm", "UseUtc": true },
				{ "X": 1, "$type": "text", "Text": "RPM", "Opacity": 128 }
			],
			"CalculatedChannels": [ { "Name": "RpmK", "Expression": "Rpm / 1000" } ]
		}"##;
		let def = CustomGaugeDefinition::from_json(json).unwrap();
		assert_eq!(def.background_image_mode, BackgroundImageMode::Fit);
		assert_eq!(def.elements.len(), 4);
		let Element::Arc(arc) = &def.elements[0] else { panic!() };
		assert_eq!(arc.base.id, "a1");
		assert_eq!(arc.base.max_value, 8000.0);
		assert_eq!(arc.base.visible_above, f32::MIN);
		assert!(arc.anti_clockwise);
		assert_eq!(arc.radius, 200.0, "missing properties keep C# defaults");
		let Element::ZoneArc(z) = &def.elements[1] else { panic!() };
		assert_eq!(z.zone2_start, 60.0);
		assert!(z.show_pointer);
		let Element::Text(t) = &def.elements[3] else { panic!() };
		assert_eq!(t.base.opacity, 128);
		assert_eq!(t.base.x, 1.0);
		assert_eq!(def.calculated_channels[0].name, "RpmK");
	}

	#[test]
	fn round_trips_through_json() {
		let mut def = CustomGaugeDefinition::default();
		def.elements.push(Element::Needle(NeedleElement::default()));
		def.background_image_mode = BackgroundImageMode::Tile;
		let json = def.to_json();
		assert!(json.contains("\"$type\": \"needle\""));
		assert!(json.contains("\"BackgroundImageMode\": 4"));
		let back = CustomGaugeDefinition::from_json(&json).unwrap();
		assert_eq!(back.elements[0].base().id, def.elements[0].base().id);
	}

	#[test]
	fn mode_accepts_names() {
		let def = CustomGaugeDefinition::from_json(r#"{"BackgroundImageMode":"center"}"#).unwrap();
		assert_eq!(def.background_image_mode, BackgroundImageMode::Center);
	}
}
