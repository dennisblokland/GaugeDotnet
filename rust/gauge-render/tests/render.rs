use std::path::PathBuf;

use gauge_render::gauges::{self, AppConfig, GaugeConfig, GaugeType, Screen};
use gauge_render::{Canvas, CustomGauge, CustomGaugeDefinition, ElementRenderer, Rgba, Values};

fn examples_dir() -> PathBuf {
	PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../examples")
}

fn temp_dir(name: &str) -> PathBuf {
	let dir = std::env::temp_dir().join(format!("gauge-render-{name}-{}", std::process::id()));
	std::fs::create_dir_all(&dir).unwrap();
	dir
}

fn values(pairs: &[(&str, f32)]) -> Values {
	pairs.iter().map(|(k, v)| (k.to_string(), *v)).collect()
}

fn close(a: Rgba, b: Rgba, tol: i32) -> bool {
	(a.r as i32 - b.r as i32).abs() <= tol
		&& (a.g as i32 - b.g as i32).abs() <= tol
		&& (a.b as i32 - b.b as i32).abs() <= tol
}

#[test]
fn showcase_example_renders_arc_fill_and_track() {
	let mut gauge = CustomGauge::load(examples_dir().join("showcase.json")).unwrap();
	gauge.renderer_mut().fixed_time = chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap().and_hms_opt(12, 34, 0);
	gauge.set_value("Rpm", 4000.0);
	gauge.snap_to_targets();
	let mut canvas = Canvas::new(640, 480);
	gauge.draw_with_delta(&mut canvas, 0.0);

	// Background colour.
	assert!(close(canvas.pixel(2, 2).unwrap(), Rgba::parse("#05080A").unwrap(), 2));
	// Arc (centre 230,240, r=175, starts at 135°): filled at 180° (left), track at 0° (right).
	assert!(close(canvas.pixel(230 - 175, 240).unwrap(), Rgba::rgb(0, 255, 255), 30));
	let track = canvas.pixel(230 + 175, 240).unwrap();
	assert!(track.g < 80, "unfilled side should show the dark track, got {track:?}");
}

#[test]
fn calculated_channel_inputs_are_tracked() {
	let gauge = CustomGauge::load(examples_dir().join("showcase.json")).unwrap();
	let sources: Vec<&str> = gauge.data_sources().collect();
	for s in ["Rpm", "AfrCurr1", "OilTemp", "Clt"] {
		assert!(sources.iter().any(|x| x.eq_ignore_ascii_case(s)), "missing {s} in {sources:?}");
	}
	assert!(!sources.iter().any(|x| x.eq_ignore_ascii_case("RpmK")), "calculated channels are not inputs");
}

#[test]
fn visibility_and_opacity() {
	let json = r##"{ "BackgroundColor": "#000000", "Elements": [
		{ "$type": "rectangle", "X": 0, "Y": 0, "Width": 50, "Height": 50, "FillColor": "#FFFFFF",
		  "RectStrokeWidth": 0, "Opacity": 128 },
		{ "$type": "rectangle", "X": 100, "Y": 0, "Width": 50, "Height": 50, "FillColor": "#FF0000",
		  "RectStrokeWidth": 0, "UseVisibility": true, "VisibilitySource": "Speed", "VisibleAbove": 10, "VisibleBelow": 20 }
	]}"##;
	let def = CustomGaugeDefinition::from_json(json).unwrap();
	let mut r = ElementRenderer::new();

	let mut canvas = Canvas::new(200, 100);
	r.render(&mut canvas, &def, &values(&[("Speed", 5.0)]), None);
	let half = canvas.pixel(25, 25).unwrap();
	assert!((half.r as i32 - 128).abs() <= 2, "opacity 128 over black: {half:?}");
	assert_eq!(canvas.pixel(125, 25).unwrap().r, 0, "hidden below VisibleAbove");

	r.render(&mut canvas, &def, &values(&[("speed", 15.0)]), None);
	assert_eq!(canvas.pixel(125, 25).unwrap(), Rgba::rgb(255, 0, 0), "visible inside range");
}

#[test]
fn images_and_gif_frames_load_relative_to_definition() {
	use image::codecs::gif::GifEncoder;
	use image::{Delay, Frame, Rgba as Px, RgbaImage};

	let dir = temp_dir("images");
	RgbaImage::from_pixel(4, 4, Px([0, 0, 255, 255])).save(dir.join("blue.png")).unwrap();
	{
		let file = std::fs::File::create(dir.join("anim.gif")).unwrap();
		let mut enc = GifEncoder::new(file);
		for color in [[255, 0, 0, 255], [0, 255, 0, 255], [255, 255, 0, 255]] {
			let frame =
				Frame::from_parts(RgbaImage::from_pixel(4, 4, Px(color)), 0, 0, Delay::from_numer_denom_ms(100, 1));
			enc.encode_frame(frame).unwrap();
		}
	}
	let json = r##"{ "Elements": [
		{ "$type": "image", "X": 0, "Y": 0, "Width": 40, "Height": 40, "ImagePath": "blue.png" },
		{ "$type": "gif", "X": 50, "Y": 0, "Width": 40, "Height": 40, "ImagePath": "anim.gif",
		  "DataSource": "Gear", "MinValue": 0, "MaxValue": 2 }
	]}"##;
	std::fs::write(dir.join("def.json"), json).unwrap();
	let def = CustomGaugeDefinition::load(dir.join("def.json")).unwrap();
	let mut r = ElementRenderer::new();
	let mut canvas = Canvas::new(100, 50);

	r.render(&mut canvas, &def, &values(&[("Gear", 0.0)]), None);
	assert!(close(canvas.pixel(20, 20).unwrap(), Rgba::rgb(0, 0, 255), 2));
	assert!(close(canvas.pixel(70, 20).unwrap(), Rgba::rgb(255, 0, 0), 8));

	r.render(&mut canvas, &def, &values(&[("Gear", 1.0)]), None);
	assert!(close(canvas.pixel(70, 20).unwrap(), Rgba::rgb(0, 255, 0), 8));

	r.render(&mut canvas, &def, &values(&[("Gear", 9.0)]), None);
	assert!(close(canvas.pixel(70, 20).unwrap(), Rgba::rgb(255, 255, 0), 8));
	let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn every_builtin_gauge_draws_something() {
	for gauge_type in GaugeType::ALL {
		let config = GaugeConfig {
			gauge_type,
			title: "T".into(),
			unit: "U".into(),
			initial_value: 60.0,
			interval_ms: 0,
			cells: vec![Default::default(); 4],
			..GaugeConfig::default()
		};
		let mut gauge = gauges::create_gauge(&config, 640, 480);
		let mut canvas = Canvas::new(640, 480);
		for _ in 0..5 {
			gauge.set_value(60.0);
			gauge.draw(&mut canvas);
		}
		let lit = canvas.pixmap().pixels().iter().filter(|p| p.green() > 100).count();
		assert!(lit > 500, "{gauge_type:?} drew only {lit} bright pixels");
		gauge.set_color_hex("#FF0000");
		gauge.draw(&mut canvas);
	}
}

#[test]
fn example_config_builds_all_screens() {
	let dir = examples_dir();
	let config = AppConfig::from_json(&std::fs::read_to_string(dir.join("gauges.json")).unwrap()).unwrap();
	let screens = gauges::build_screens(&config, 640, 480, Some(&dir));
	assert_eq!(screens.len(), config.screens.len());
	let mut canvas = Canvas::new(640, 480);
	for screen in screens {
		let mut screen = screen.unwrap();
		screen.update(|_| Some(42.0));
		screen.draw(&mut canvas);
		if let Screen::Custom(g) = &screen {
			assert!(g.definition().elements.len() > 10);
		}
	}
}

#[test]
fn cached_static_runs_match_uncached_render() {
	use gauge_render::custom::StaticCache;

	let def = CustomGaugeDefinition::load(examples_dir().join("showcase.json")).unwrap();
	let time = chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap().and_hms_opt(8, 15, 0);
	let (mut plain, mut cached) = (ElementRenderer::new(), ElementRenderer::new());
	plain.fixed_time = time;
	cached.fixed_time = time;
	let mut cache = StaticCache::new();
	let (mut a, mut b) = (Canvas::new(640, 480), Canvas::new(640, 480));

	for rpm in [1000.0, 4500.0, 7200.0] {
		let v = values(&[("Rpm", rpm), ("AfrCurr1", 14.2), ("OilTemp", 112.0), ("Clt", 80.0), ("BoostCtrlDuty", 60.0)]);
		plain.render(&mut a, &def, &v, Some(&v));
		cached.render_cached(&mut b, &def, &v, Some(&v), &mut cache);
		let max_diff =
			a.pixmap().data().iter().zip(b.pixmap().data()).map(|(x, y)| (*x as i32 - *y as i32).abs()).max().unwrap();
		assert!(max_diff <= 3, "rpm {rpm}: cached render differs by {max_diff}");
	}
}
