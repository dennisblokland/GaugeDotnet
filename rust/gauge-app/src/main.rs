//! `gauge`: renders GaugeDotnet gauge definitions with the Rust renderer.

#[cfg(feature = "sdl")]
mod app;
#[cfg(feature = "sdl")]
mod display;
#[cfg(feature = "sdl")]
mod feed;
#[cfg(feature = "sdl")]
mod input;
#[cfg(feature = "sdl")]
mod screens;
mod sim;
#[cfg(feature = "sdl")]
mod window;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Instant;

use gauge_render::gauges::{self, AppConfig, GaugeConfig, GaugeType, Screen};
use gauge_render::{Canvas, CustomGauge, CustomGaugeDefinition};

use sim::Simulator;

const USAGE: &str = "\
gauge — GaugeDotnet renderer (Rust)

USAGE:
  gauge app [--config gauges.json] [--fullscreen] [--demo] [--init-bt | --no-init-bt]
      The on-device app (port of the C# GaugeDotnet): loads gauges.json (next
      to the executable by default, created if missing), connects to the ECU
      over Bluetooth behind a splash screen (or simulates in DemoMode / --demo),
      then shows the screens. D-pad left/right switches screens, R1 resets
      peaks/history, MENU exits. On a desktop the keyboard stands in:
      arrows, '.', Esc. Bluetooth bring-up runs automatically where
      rtk_hciattach exists. Needs --features sdl (and ble for the ECU).
  gauge render <file.json> [-o out.png] [--set NAME=VALUE]... [--time T] [--frames N]
      Render a custom gauge definition, or every screen of a gauges.json
      (written as out-0.png, out-1.png, ...). Values come from the simulated
      ECU at time T (default 1.2) unless overridden with --set.
  gauge builtin <type> [-o out.png] [--value V] [--min V] [--max V]
                [--title S] [--unit S] [--color #RRGGBB] [--decimals N] [--segments N]
      Render a built-in gauge: bar, circular, needle, digital, sweep, minmax, histogram.
  gauge bench <file.json> [--frames N] [--warmup N] [--threads N] [--data-hz HZ] [--full]
      Time the draw call of a custom definition, or of every gauges.json screen.
      --data-hz sets how often new values arrive (default 60 = every frame);
      --full repaints the whole frame every time (no damage tracking).
  gauge run <file.json> [--fullscreen] [--ble [--mac AA:BB:CC:DD:EE:FF] [--init-bt]]
      Live SDL2 window (build with --features sdl). Simulated data by default;
      --ble reads the ECU over Bluetooth (build with --features ble). The MAC
      comes from --mac or DeviceMacAddress in gauges.json; without one the
      first ME device found is used. --init-bt runs the RG35XX Bluetooth
      bring-up first. Left/Right (or D-pad) switch screens, Esc quits.
  gauge ble-scan [--seconds N]
      List nearby ME devices (RaceChrono BLE service).
  gauge ble-monitor [--mac AA:BB:CC:DD:EE:FF] [--init-bt] [--seconds N]
      Connect to the ECU and print live decoded values.
";

struct Args {
	positional: Vec<String>,
	options: HashMap<String, String>,
	sets: Vec<(String, f32)>,
	flags: Vec<String>,
}

fn parse_args(raw: impl Iterator<Item = String>) -> Result<Args, String> {
	let mut args = Args { positional: Vec::new(), options: HashMap::new(), sets: Vec::new(), flags: Vec::new() };
	let mut it = raw.peekable();
	while let Some(a) = it.next() {
		match a.as_str() {
			"--fullscreen" | "--full" | "--ble" | "--init-bt" | "--no-init-bt" | "--demo" | "-h" | "--help" => {
				args.flags.push(a)
			}
			"--set" => {
				let kv = it.next().ok_or("--set needs NAME=VALUE")?;
				let (k, v) = kv.split_once('=').ok_or_else(|| format!("bad --set '{kv}'"))?;
				let v: f32 = v.parse().map_err(|_| format!("bad number in --set '{kv}'"))?;
				args.sets.push((k.to_string(), v));
			}
			s if s.starts_with('-') => {
				let v = it.next().ok_or_else(|| format!("{s} needs a value"))?;
				args.options.insert(s.trim_start_matches('-').to_string(), v);
			}
			_ => args.positional.push(a),
		}
	}
	Ok(args)
}

impl Args {
	fn opt<T: std::str::FromStr>(&self, names: &[&str], default: T) -> Result<T, String> {
		for n in names {
			if let Some(v) = self.options.get(*n) {
				return v.parse().map_err(|_| format!("invalid value for --{n}: {v}"));
			}
		}
		Ok(default)
	}

	fn output(&self) -> PathBuf {
		PathBuf::from(self.options.get("o").or(self.options.get("output")).map(String::as_str).unwrap_or("out.png"))
	}
}

/// Feeds simulated data to one screen and draws it; `true` forces a full
/// repaint (after switching screens). Returns what changed.
#[cfg_attr(not(feature = "sdl"), allow(dead_code))]
pub type DrawFn = Box<dyn FnMut(&mut Canvas, &me_data::MeData, bool) -> gauge_render::Damage>;

/// What a JSON file contains.
pub enum Loaded {
	Custom(Box<CustomGauge>),
	Screens(Vec<Screen>),
}

pub fn load(path: &Path, width: u32, height: u32) -> Result<Loaded, String> {
	let json = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
	if is_app_config(&json) {
		let config = AppConfig::from_json(&json).map_err(|e| format!("{}: {e}", path.display()))?;
		let dir = std::fs::canonicalize(path).ok().and_then(|p| p.parent().map(Path::to_path_buf));
		let mut screens = Vec::new();
		for (i, s) in gauges::build_screens(&config, width, height, dir.as_deref()).into_iter().enumerate() {
			match s {
				Ok(s) => screens.push(s),
				Err(e) => eprintln!("screen {i}: {e}"),
			}
		}
		if screens.is_empty() {
			return Err("no usable screens in config".into());
		}
		Ok(Loaded::Screens(screens))
	} else {
		Ok(Loaded::Custom(Box::new(CustomGauge::load(path)?)))
	}
}

/// `gauges.json` has a top-level `Screens` array; custom definitions have `Elements`.
pub fn is_app_config(json: &str) -> bool {
	serde_json::from_str::<serde_json::Value>(json.trim_start_matches('\u{feff}'))
		.ok()
		.and_then(|v| v.as_object().map(|o| o.contains_key("Screens") && !o.contains_key("Elements")))
		.unwrap_or(false)
}

fn reader<'a>(sim: &'a Simulator, sets: &'a [(String, f32)]) -> impl FnMut(&str) -> Option<f32> + 'a {
	move |name: &str| {
		sets.iter().find(|(k, _)| k.eq_ignore_ascii_case(name)).map(|(_, v)| *v).or_else(|| sim.read(name))
	}
}

fn cmd_render(args: &Args) -> Result<(), String> {
	let file = args.positional.get(1).ok_or("render needs a JSON file")?;
	let time: f64 = args.opt(&["time", "t"], 1.2)?;
	let frames: u32 = args.opt(&["frames"], 90)?;
	let out = args.output();
	let (w, h) = dimensions(Path::new(file))?;

	// Run the simulation up to `time` so graphs, peaks and smoothing have history.
	let mut sim = Simulator::new(time - frames as f64 / 30.0);
	match load(Path::new(file), w, h)? {
		Loaded::Custom(mut gauge) => {
			let mut canvas = Canvas::new(w, h);
			for _ in 0..frames.max(1) {
				sim.advance(1.0 / 30.0);
				gauge.update_from(reader(&sim, &args.sets));
				gauge.draw_with_delta(&mut canvas, 1.0 / 30.0);
			}
			gauge.snap_to_targets();
			gauge.draw_with_delta(&mut canvas, 0.0);
			canvas.save_png(&out)?;
			println!("wrote {}", out.display());
		}
		Loaded::Screens(mut screens) => {
			for (i, screen) in screens.iter_mut().enumerate() {
				let mut canvas = Canvas::new(w, h);
				let mut sim = Simulator::new(time - frames as f64 / 30.0);
				for _ in 0..frames.max(1) {
					sim.advance(1.0 / 30.0);
					screen.update(reader(&sim, &args.sets));
					match &mut *screen {
						Screen::Custom(g) => g.draw_with_delta(&mut canvas, 1.0 / 30.0),
						other => other.draw(&mut canvas),
					};
				}
				if let Screen::Custom(g) = screen {
					g.snap_to_targets();
					g.draw_with_delta(&mut canvas, 0.0);
				}
				let path = numbered(&out, i);
				canvas.save_png(&path)?;
				println!("wrote {}", path.display());
			}
		}
	}
	Ok(())
}

fn numbered(out: &Path, i: usize) -> PathBuf {
	let stem = out.file_stem().and_then(|s| s.to_str()).unwrap_or("out");
	let ext = out.extension().and_then(|s| s.to_str()).unwrap_or("png");
	out.with_file_name(format!("{stem}-{i}.{ext}"))
}

/// Canvas size: the custom definition's Width/Height, else 640×480.
fn dimensions(path: &Path) -> Result<(u32, u32), String> {
	let json = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
	if is_app_config(&json) {
		return Ok((640, 480));
	}
	let def = CustomGaugeDefinition::from_json(&json).map_err(|e| format!("{}: {e}", path.display()))?;
	Ok((def.width.max(1) as u32, def.height.max(1) as u32))
}

fn cmd_builtin(args: &Args) -> Result<(), String> {
	let kind = args.positional.get(1).ok_or("builtin needs a gauge type")?;
	let gauge_type = GaugeType::parse(kind).ok_or_else(|| format!("unknown gauge type '{kind}'"))?;
	let value: f32 = args.opt(&["value", "v"], 65.0)?;
	let config = GaugeConfig {
		gauge_type,
		min_value: args.opt(&["min"], 0.0)?,
		max_value: args.opt(&["max"], 100.0)?,
		title: args.opt(&["title"], "TITLE".to_string())?,
		unit: args.opt(&["unit"], "UNIT".to_string())?,
		color_hex: args.opt(&["color"], "#00FFFF".to_string())?,
		decimals: args.opt(&["decimals"], 0)?,
		segment_count: args.opt(&["segments"], if gauge_type == GaugeType::Circular { 64 } else { 32 })?,
		initial_value: value,
		interval_ms: 0,
		..GaugeConfig::default()
	};
	let mut gauge = gauges::create_gauge(&config, 640, 480);
	let mut canvas = Canvas::new(640, 480);
	// Feed a wobble for history-based gauges, then settle on the requested value.
	for i in 0..60 {
		let wobble = (i as f32 * 0.3).sin() * (config.max_value - config.min_value) * 0.15;
		gauge.set_value(value + wobble * (1.0 - i as f32 / 60.0));
		gauge.draw(&mut canvas);
	}
	gauge.set_value(value);
	for _ in 0..120 {
		gauge.draw(&mut canvas);
	}
	let out = args.output();
	canvas.save_png(&out)?;
	println!("wrote {}", out.display());
	Ok(())
}

fn cmd_bench(args: &Args) -> Result<(), String> {
	let file = args.positional.get(1).ok_or("bench needs a JSON file")?;
	let frames: u32 = args.opt(&["frames"], 600)?;
	let warmup: u32 = args.opt(&["warmup"], 300)?;
	let threads: usize = args.opt(&["threads"], gauge_render::custom::default_threads())?;
	// How often new data arrives; 60 = every frame (worst case for caching).
	let data_hz: f64 = args.opt(&["data-hz"], 60.0)?;
	// Repaint everything every frame (worst case: what a screen switch costs).
	let force_full = args.flags.iter().any(|f| f == "--full");
	let (w, h) = dimensions(Path::new(file))?;
	let mut canvas = Canvas::new(w, h);
	let frame_px = (w * h) as f64;

	// Same protocol as bench/csharp: fixed 60 Hz steps, warm-up, then only the draw is timed.
	let mut run = |label: String, screen: &mut dyn FnMut(&mut Canvas, &Simulator, bool) -> usize| {
		let mut sim = Simulator::new(0.0);
		let mut since_data = f64::MAX;
		// Advances the simulation one frame; true when new data is due.
		let mut data_due = |sim: &mut Simulator| {
			sim.advance(1.0 / 60.0);
			since_data += 1.0 / 60.0;
			let due = since_data >= 1.0 / data_hz - 1e-9;
			if due {
				since_data = 0.0;
			}
			due
		};
		for _ in 0..warmup {
			if data_due(&mut sim) {
				screen(&mut canvas, &sim, true);
			}
			screen(&mut canvas, &sim, false);
		}
		let mut total = std::time::Duration::ZERO;
		let mut damaged = 0usize;
		for _ in 0..frames {
			if data_due(&mut sim) {
				screen(&mut canvas, &sim, true);
			}
			let start = Instant::now();
			damaged += screen(&mut canvas, &sim, false);
			total += start.elapsed();
		}
		let ms = total.as_secs_f64() * 1000.0 / frames as f64;
		let pct = damaged as f64 * 100.0 / frames as f64 / frame_px;
		println!("{label:<28} {ms:>9.3} {:>7.0} {pct:>8.1}%", 1000.0 / ms);
	};

	println!("Rust / tiny-skia (CPU raster), {threads} thread(s), data at {data_hz} Hz");
	println!("{:<28} {:>9} {:>7} {:>9}", "screen", "ms/frame", "fps", "repainted");
	match load(Path::new(file), w, h)? {
		Loaded::Custom(mut g) => {
			g.set_threads(threads);
			run("Custom".into(), &mut |c, s, update| {
				if update {
					g.update_from(|n| s.read(n));
					0
				} else {
					if force_full {
						g.invalidate();
					}
					g.draw_with_delta(c, 1.0 / 60.0).area()
				}
			})
		}
		Loaded::Screens(mut screens) => {
			let json = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
			let config = AppConfig::from_json(&json).map_err(|e| e.to_string())?;
			for (screen, cfg) in screens.iter_mut().zip(&config.screens) {
				let g = &cfg.gauge;
				let label = match (g.gauge_type, &cfg.custom_definition_file) {
					(GaugeType::Custom, Some(f)) => format!("Custom ({f})"),
					(GaugeType::Grid, _) => "Grid (4 cells)".into(),
					(t, _) => format!("{t:?} ({})", g.data_source),
				};
				if let Screen::Custom(g) = screen {
					g.set_threads(threads);
				}
				screen.invalidate();
				run(label, &mut |c, s, update| {
					if update {
						screen.update(|n| s.read(n));
						0
					} else if let Screen::Custom(g) = screen {
						if force_full {
							g.invalidate();
						}
						g.draw_with_delta(c, 1.0 / 60.0).area()
					} else {
						screen.draw(c).area()
					}
				});
			}
		}
	}
	Ok(())
}

/// MAC from `--mac`, else `DeviceMacAddress` when the file is a gauges.json.
#[cfg_attr(not(feature = "ble"), allow(dead_code))]
fn configured_mac(args: &Args, file: Option<&String>) -> Option<String> {
	if let Some(mac) = args.options.get("mac") {
		return Some(mac.clone());
	}
	let json = std::fs::read_to_string(file?).ok()?;
	if !is_app_config(&json) {
		return None;
	}
	AppConfig::from_json(&json).ok()?.device_mac_address.filter(|m| !m.trim().is_empty())
}

#[cfg(feature = "ble")]
fn ble_link(args: &Args, file: Option<&String>) -> me_ble::BleLink {
	me_ble::BleLink::start(me_ble::LinkOptions {
		mac: configured_mac(args, file),
		init_hardware: args.flags.iter().any(|f| f == "--init-bt"),
		..Default::default()
	})
}

#[cfg(feature = "sdl")]
fn cmd_run(args: &Args) -> Result<(), String> {
	let file = args.positional.get(1).ok_or("run needs a JSON file")?;
	let feed = if args.flags.iter().any(|f| f == "--ble") {
		#[cfg(feature = "ble")]
		{
			feed::Feed::Ble(ble_link(args, Some(file)))
		}
		#[cfg(not(feature = "ble"))]
		return Err("this build has no Bluetooth support; rebuild with `--features ble`".into());
	} else {
		feed::Feed::simulated()
	};
	window::run(Path::new(file), args.flags.iter().any(|f| f == "--fullscreen"), feed)
}

#[cfg(feature = "sdl")]
fn cmd_app(args: &Args) -> Result<(), String> {
	let flag = |f: &str| args.flags.iter().any(|x| x == f);
	app::run(app::AppOptions {
		config: args.options.get("config").map(PathBuf::from),
		fullscreen: flag("--fullscreen"),
		demo: flag("--demo"),
		init_bt: if flag("--init-bt") {
			Some(true)
		} else if flag("--no-init-bt") {
			Some(false)
		} else {
			None
		},
	})
}

#[cfg(not(feature = "sdl"))]
fn cmd_app(_: &Args) -> Result<(), String> {
	Err("this build has no window support; rebuild with `--features sdl`".into())
}

#[cfg(not(feature = "sdl"))]
fn cmd_run(_: &Args) -> Result<(), String> {
	Err("this build has no window support; rebuild with `--features sdl`".into())
}

#[cfg(feature = "ble")]
fn cmd_ble_scan(args: &Args) -> Result<(), String> {
	let seconds: u64 = args.opt(&["seconds"], 10)?;
	println!("Scanning {seconds} s for ME devices...");
	let found = me_ble::scan_blocking(std::time::Duration::from_secs(seconds)).map_err(|e| e.to_string())?;
	if found.is_empty() {
		println!("No ME devices found.");
	}
	for d in found {
		let rssi = d.rssi.map_or("?".to_string(), |r| format!("{r} dBm"));
		println!("{}  {:<20} {rssi}", d.address, d.name.as_deref().unwrap_or("(no name)"));
	}
	Ok(())
}

#[cfg(feature = "ble")]
fn cmd_ble_monitor(args: &Args) -> Result<(), String> {
	let seconds: u64 = args.opt(&["seconds"], 0)?;
	let link = ble_link(args, None);
	let start = Instant::now();
	let mut last_status = None;
	loop {
		let status = link.status();
		if Some(&status) != last_status.as_ref() {
			println!("status: {status:?}");
			last_status = Some(status.clone());
		}
		if let me_ble::LinkStatus::Failed(msg) = &status {
			return Err(msg.replace('\n', " "));
		}
		if status == me_ble::LinkStatus::Connected {
			let d = link.snapshot();
			println!(
				"rpm {:5}  map {:6.1}  tps {:5.1}  afr {:5.2}  clt {:5.1}  oil {:5.1}/{:6.1}  vbat {:4.1}  gear {}  speed {}",
				d.rpm, d.map, d.throttle_position, d.afr_curr1, d.clt, d.oil_temp, d.oil_pressure, d.vbat, d.gear_pos, d.vehicle_speed
			);
		}
		if seconds > 0 && start.elapsed().as_secs() >= seconds {
			return Ok(());
		}
		std::thread::sleep(std::time::Duration::from_millis(500));
	}
}

#[cfg(not(feature = "ble"))]
fn cmd_ble_scan(_: &Args) -> Result<(), String> {
	Err("this build has no Bluetooth support; rebuild with `--features ble`".into())
}

#[cfg(not(feature = "ble"))]
fn cmd_ble_monitor(_: &Args) -> Result<(), String> {
	cmd_ble_scan(&Args { positional: vec![], options: HashMap::new(), sets: vec![], flags: vec![] })
}

fn main() -> ExitCode {
	let args = match parse_args(std::env::args().skip(1)) {
		Ok(a) => a,
		Err(e) => {
			eprintln!("error: {e}\n\n{USAGE}");
			return ExitCode::from(2);
		}
	};
	let result = match args.positional.first().map(String::as_str) {
		Some("render") => cmd_render(&args),
		Some("builtin") => cmd_builtin(&args),
		Some("bench") => cmd_bench(&args),
		Some("app") => cmd_app(&args),
		Some("run") => cmd_run(&args),
		Some("ble-scan") => cmd_ble_scan(&args),
		Some("ble-monitor") => cmd_ble_monitor(&args),
		_ => {
			print!("{USAGE}");
			return if args.flags.iter().any(|f| f == "-h" || f == "--help") {
				ExitCode::SUCCESS
			} else {
				ExitCode::from(2)
			};
		}
	};
	match result {
		Ok(()) => ExitCode::SUCCESS,
		Err(e) => {
			eprintln!("error: {e}");
			ExitCode::FAILURE
		}
	}
}
