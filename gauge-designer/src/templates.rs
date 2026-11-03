//! Starting points for new gauges, modelled on the built-in screens.

use gauge_render::model::*;

fn base(name: &str, x: f32, y: f32) -> ElementBase {
	ElementBase { name: name.into(), x, y, ..ElementBase::default() }
}

fn bound(name: &str, x: f32, y: f32, source: &str, min: f32, max: f32) -> ElementBase {
	ElementBase {
		data_source: (!source.is_empty()).then(|| source.into()),
		min_value: min,
		max_value: max,
		..base(name, x, y)
	}
}

/// The C# designer's default: an RPM arc dial with ticks, needle and readout.
pub fn rpm_dial() -> CustomGaugeDefinition {
	let elements = vec![
		Element::Arc(ArcElement {
			base: bound("RPM Arc", 320.0, 240.0, "Rpm", 0.0, 8000.0),
			radius: 200.0,
			stroke_width: 30.0,
			color: "#00FFFF".into(),
			track_color: "#0A1A1A".into(),
			..ArcElement::default()
		}),
		Element::Ticks(TickRingElement {
			base: bound("RPM Ticks", 320.0, 240.0, "", 0.0, 8000.0),
			radius: 178.0,
			major_count: 8,
			minor_per_major: 4,
			major_length: 15.0,
			minor_length: 8.0,
			major_width: 2.0,
			minor_width: 1.0,
			color: "#AAAAAA".into(),
			show_labels: true,
			label_font_size: 14.0,
			label_color: "#888888".into(),
			label_offset: 22.0,
			..TickRingElement::default()
		}),
		Element::Needle(NeedleElement {
			base: bound("RPM Needle", 320.0, 240.0, "Rpm", 0.0, 8000.0),
			length: 170.0,
			tail_length: 25.0,
			needle_width: 4.0,
			color: "#FF3333".into(),
			hub_radius: 10.0,
			hub_color: "#CCCCCC".into(),
			..NeedleElement::default()
		}),
		Element::Value(ValueDisplayElement {
			base: bound("RPM Value", 320.0, 340.0, "Rpm", 0.0, 8000.0),
			font_size: 42.0,
			color: "#00FFFF".into(),
			font: "DSEG7 Classic".into(),
			format: "F0".into(),
			..ValueDisplayElement::default()
		}),
		Element::Text(TextElement {
			base: base("Title", 320.0, 390.0),
			text: "RPM".into(),
			font_size: 20.0,
			color: "#666666".into(),
			font: "Race Sport".into(),
			..TextElement::default()
		}),
		Element::Circle(CircleElement {
			base: base("Hub Cap", 320.0, 240.0),
			radius: 8.0,
			fill_color: "#444444".into(),
			stroke_color: "#666666".into(),
			circle_stroke_width: 1.0,
		}),
	];
	with_elements(elements)
}

/// A wide horizontal bar with title and readout (like the built-in Bar).
pub fn bar() -> CustomGaugeDefinition {
	with_elements(vec![
		Element::Text(TextElement {
			base: base("Title", 320.0, 120.0),
			text: "AFR".into(),
			font_size: 40.0,
			color: "#AAAAAA".into(),
			..TextElement::default()
		}),
		Element::Value(ValueDisplayElement {
			base: bound("Value", 320.0, 250.0, "AfrCurr1", 8.0, 18.0),
			font_size: 90.0,
			format: "F2".into(),
			..ValueDisplayElement::default()
		}),
		Element::LinearBar(LinearBarElement {
			base: bound("Bar", 40.0, 330.0, "AfrCurr1", 8.0, 18.0),
			width: 560.0,
			height: 50.0,
			corner_radius: 6.0,
			..LinearBarElement::default()
		}),
	])
}

/// A needle dial with labelled ticks (like the built-in Needle).
pub fn needle_dial() -> CustomGaugeDefinition {
	with_elements(vec![
		Element::Circle(CircleElement {
			base: base("Face", 320.0, 240.0),
			radius: 225.0,
			fill_color: "#101418".into(),
			stroke_color: "#A8A8B4".into(),
			circle_stroke_width: 4.0,
		}),
		Element::ZoneArc(ZoneArcElement {
			base: bound("Zones", 320.0, 240.0, "", 0.0, 140.0),
			radius: 205.0,
			stroke_width: 8.0,
			zone1_color: "#1E3A4A".into(),
			zone2_start: 100.0,
			zone3_start: 120.0,
			..ZoneArcElement::default()
		}),
		Element::Ticks(TickRingElement {
			base: bound("Ticks", 320.0, 240.0, "", 0.0, 140.0),
			radius: 200.0,
			major_count: 8,
			minor_per_major: 4,
			ticks_inside: true,
			label_offset: -40.0,
			label_font_size: 20.0,
			..TickRingElement::default()
		}),
		Element::Needle(NeedleElement {
			base: bound("Needle", 320.0, 240.0, "VehicleSpeed", 0.0, 140.0),
			length: 185.0,
			..NeedleElement::default()
		}),
		Element::Value(ValueDisplayElement {
			base: bound("Value", 320.0, 380.0, "VehicleSpeed", 0.0, 140.0),
			font_size: 40.0,
			color: "#FFFFFF".into(),
			..ValueDisplayElement::default()
		}),
		Element::Text(TextElement {
			base: base("Unit", 320.0, 420.0),
			text: "KM/H".into(),
			font_size: 18.0,
			color: "#888888".into(),
			..TextElement::default()
		}),
	])
}

/// Four boxed readouts in a 2×2 grid (like the built-in Grid).
pub fn grid() -> CustomGaugeDefinition {
	let cells =
		[("RPM", "Rpm", "F0"), ("OIL °C", "OilTemp", "F0"), ("AFR", "AfrCurr1", "F2"), ("BATT V", "Vbat", "F1")];
	let elements = cells
		.iter()
		.enumerate()
		.map(|(i, (label, source, format))| {
			let (x, y) = (160.0 + 320.0 * (i % 2) as f32, 120.0 + 240.0 * (i / 2) as f32);
			Element::LabelValue(LabelValueElement {
				base: bound(label, x, y, source, 0.0, 100.0),
				label: (*label).into(),
				label_font_size: 22.0,
				value_font_size: 64.0,
				value_format: (*format).into(),
				show_box: true,
				box_padding: 24.0,
				box_corner_radius: 10.0,
				..LabelValueElement::default()
			})
		})
		.collect();
	with_elements(elements)
}

fn with_elements(elements: Vec<Element>) -> CustomGaugeDefinition {
	CustomGaugeDefinition { elements, ..CustomGaugeDefinition::default() }
}

/// A named starting point.
pub type Template = (&'static str, fn() -> CustomGaugeDefinition);

pub const TEMPLATES: [Template; 5] = [
	("Blank", CustomGaugeDefinition::default),
	("RPM dial", rpm_dial),
	("Needle dial", needle_dial),
	("Bar", bar),
	("2×2 grid", grid),
];

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn templates_render() {
		for (name, make) in TEMPLATES {
			let def = make();
			let mut canvas = gauge_render::Canvas::new(640, 480);
			gauge_render::ElementRenderer::new().render(&mut canvas, &def, &Default::default(), None);
			// Round-trips through JSON like a saved file.
			let back = CustomGaugeDefinition::from_json(&def.to_json()).unwrap();
			assert_eq!(back.elements.len(), def.elements.len(), "{name}");
		}
	}
}
