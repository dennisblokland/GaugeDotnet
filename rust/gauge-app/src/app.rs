//! The device app (`Program` + `GameLoop`): load `gauges.json`, bring up
//! Bluetooth and connect to the ECU behind a splash screen (or simulate in
//! `DemoMode`), then run the gauge screens until MENU is pressed.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use gauge_render::gauges::{self, AppConfig, Screen};
use gauge_render::{Canvas, Damage};
use me_data::MeData;

use crate::display::Display;
use crate::editor::ConfigEditor;
use crate::feed::Feed;
use crate::input::{GamepadKey, Input};
use crate::screens::{self, BatteryOverlay, FpsCounter};

const WIDTH: u32 = 640;
const HEIGHT: u32 = 480;
/// Values are pushed to the current screen at most every 50 ms (`GameLoop.UpdateGaugeData`).
const DATA_INTERVAL: Duration = Duration::from_millis(50);
const KEEP_ALIVE_INTERVAL: Duration = Duration::from_secs(3);
const SPLASH_MIN: Duration = Duration::from_millis(1500);

pub struct AppOptions {
	/// `gauges.json`; defaults to the one next to the executable.
	pub config: Option<PathBuf>,
	pub fullscreen: bool,
	/// Force simulated data even if the config says otherwise.
	pub demo: bool,
	/// Run the RG35XX Bluetooth bring-up (`None` = only when `rtk_hciattach` exists).
	pub init_bt: Option<bool>,
}

fn default_config_path() -> PathBuf {
	std::env::current_exe()
		.ok()
		.and_then(|exe| exe.parent().map(|d| d.join("gauges.json")))
		.unwrap_or_else(|| PathBuf::from("gauges.json"))
}

/// The RG35XX's Bluetooth bring-up only makes sense where its tools exist;
/// on a desktop it would restart the system's bluetoothd.
fn on_rg35xx() -> bool {
	std::process::Command::new("sh")
		.args(["-c", "command -v rtk_hciattach"])
		.output()
		.map(|o| o.status.success())
		.unwrap_or(false)
}

/// Builds the screens. One that fails to load becomes a placeholder showing
/// the error, so screen `i` is still `config.screens[i]`.
fn build_screens(config: &AppConfig, dir: Option<&Path>) -> Vec<Screen> {
	gauges::build_screens(config, WIDTH, HEIGHT, dir)
		.into_iter()
		.enumerate()
		.map(|(i, s)| {
			s.unwrap_or_else(|e| {
				eprintln!("[app] screen {i} failed to load: {e}");
				Screen::Failed(e)
			})
		})
		.collect()
}

pub fn run(options: AppOptions) -> Result<(), String> {
	let config_path = options.config.unwrap_or_else(default_config_path);
	let config = AppConfig::load_or_create(&config_path);
	eprintln!("[app] loaded {} screen(s) from {}", config.screens.len(), config_path.display());
	let demo = options.demo || config.demo_mode;

	let mut display = Display::open("GaugeDotnet", WIDTH, HEIGHT, options.fullscreen)?;
	let mut canvas = Canvas::new(WIDTH, HEIGHT);
	let mut input = Input::new();

	let feed = if demo {
		eprintln!("[app] DemoMode: simulated ECU, no Bluetooth");
		Feed::simulated()
	} else {
		start_ble(&config, options.init_bt.unwrap_or_else(on_rg35xx))?
	};

	// Splash until connected or failed (at least 1.5 s), like SplashScreen.ShowUntil.
	let started = Instant::now();
	loop {
		input.poll(&mut display.events);
		if input.quit {
			return Ok(());
		}
		let (done, error) = feed.startup_state();
		if let Some(message) = error {
			return show_error(&mut display, &mut canvas, &mut input, &message);
		}
		if done && started.elapsed() >= SPLASH_MIN {
			break;
		}
		screens::draw_splash(&mut canvas, started.elapsed().as_secs_f64(), done, "Connecting");
		display.present_full(&canvas)?;
		std::thread::sleep(Duration::from_millis(16));
	}

	GameLoop::new(config, Some(config_path), feed).run(&mut display, &mut canvas, &mut input)?;

	for i in 0..=20 {
		screens::draw_exit(&mut canvas, i as f32 / 20.0);
		display.present_full(&canvas)?;
		std::thread::sleep(Duration::from_millis(18));
	}
	std::thread::sleep(Duration::from_millis(600));
	Ok(())
}

#[cfg(feature = "ble")]
fn start_ble(config: &AppConfig, init_hardware: bool) -> Result<Feed, String> {
	Ok(Feed::Ble(me_ble::BleLink::start(me_ble::LinkOptions {
		mac: config.device_mac_address.clone(),
		init_hardware,
		..Default::default()
	})))
}

#[cfg(not(feature = "ble"))]
fn start_ble(_: &AppConfig, _: bool) -> Result<Feed, String> {
	Err("This build has no Bluetooth support.\nSet DemoMode or rebuild with --features ble.".into())
}

/// Shows an error until any button is pressed (`ErrorScreen.ShowOn`).
fn show_error(display: &mut Display, canvas: &mut Canvas, input: &mut Input, message: &str) -> Result<(), String> {
	eprintln!("[app] {}", message.replace('\n', " "));
	screens::draw_error(canvas, message);
	display.present_full(canvas)?;
	loop {
		input.poll(&mut display.events);
		if input.quit || input.any_pressed() {
			return Ok(());
		}
		std::thread::sleep(Duration::from_millis(50));
	}
}

struct GameLoop {
	config: AppConfig,
	/// Where the editor saves; `None` keeps edits in memory (tests).
	config_path: Option<PathBuf>,
	config_dir: Option<PathBuf>,
	editor: Option<ConfigEditor>,
	screens: Vec<Screen>,
	current: usize,
	shown: Option<usize>,
	feed: Feed,
	data: MeData,
	fps: FpsCounter,
	battery: BatteryOverlay,
	last_update: Option<Instant>,
	last_keep_alive: Option<Instant>,
}

impl GameLoop {
	fn new(config: AppConfig, config_path: Option<PathBuf>, feed: Feed) -> Self {
		// Custom definition files are relative to the config's folder.
		let config_dir = config_path
			.as_deref()
			.and_then(|p| std::fs::canonicalize(p).ok())
			.and_then(|p| p.parent().map(Path::to_path_buf));
		Self {
			screens: build_screens(&config, config_dir.as_deref()),
			battery: BatteryOverlay::new(config.simulate_battery_percent),
			config,
			config_path,
			config_dir,
			editor: None,
			current: 0,
			shown: None,
			feed,
			data: MeData::default(),
			fps: FpsCounter::new(),
			last_update: None,
			last_keep_alive: None,
		}
	}

	fn run(&mut self, display: &mut Display, canvas: &mut Canvas, input: &mut Input) -> Result<(), String> {
		while self.step(display, canvas, input)? {}
		Ok(())
	}

	/// One frame: input, navigation, data, render. `false` once the user quits.
	fn step(&mut self, display: &mut Display, canvas: &mut Canvas, input: &mut Input) -> Result<bool, String> {
		input.poll(&mut display.events);
		if input.quit {
			return Ok(false);
		}
		if let Some(editor) = &mut self.editor {
			editor.handle_input(&mut self.config, self.config_path.as_deref(), input.key, input.keyboard_key());
			if editor.active {
				editor.draw(&self.config, canvas);
				display.present_full(canvas)?;
				return Ok(true);
			}
			self.close_editor();
			// The key that closed the editor is used up (on a keyboard, Escape
			// is both B and MENU), so just show the gauges again.
			self.render(display, canvas)?;
			return Ok(true);
		}
		if input.key == GamepadKey::MENU_DOWN {
			return Ok(false);
		}
		if input.key == GamepadKey::SELECT_DOWN || input.sdl_key == Some(sdl2::keyboard::Keycode::Tab) {
			self.editor = Some(ConfigEditor::new());
			return Ok(true);
		}
		self.handle_reset(input);
		self.handle_navigation(input.key);
		self.update_data();
		self.render(display, canvas)?;
		Ok(true)
	}

	/// Rebuilds the screens if the editor changed the config (`HandleConfigEditor`).
	fn close_editor(&mut self) {
		if self.editor.take().is_some_and(|e| e.changed) {
			self.screens = build_screens(&self.config, self.config_dir.as_deref());
			self.current = self.current.min(self.screens.len().saturating_sub(1));
			self.last_update = None;
		}
		// The editor painted over everything.
		self.shown = None;
	}

	/// R1 (or `r`) clears the current screen's saved state (peaks, history).
	fn handle_reset(&mut self, input: &Input) {
		if input.key == GamepadKey::R1_DOWN || input.sdl_key == Some(sdl2::keyboard::Keycode::R) {
			if let Some(screen) = self.screens.get_mut(self.current) {
				screen.reset_saved_state();
			}
		}
	}

	fn handle_navigation(&mut self, key: GamepadKey) {
		let n = self.screens.len();
		if n > 1 && key == GamepadKey::RIGHT {
			self.current = (self.current + 1) % n;
		} else if n > 1 && key == GamepadKey::LEFT {
			self.current = (self.current + n - 1) % n;
		}
	}

	fn update_data(&mut self) {
		let now = Instant::now();
		if self.last_keep_alive.map_or(true, |t| now - t >= KEEP_ALIVE_INTERVAL) {
			self.last_keep_alive = Some(now);
			screens::poke_keep_alive();
		}
		if self.last_update.is_some_and(|t| now - t < DATA_INTERVAL) {
			return;
		}
		self.last_update = Some(now);
		self.data = self.feed.tick();
		if self.feed.status().is_none() {
			if let Some(screen) = self.screens.get_mut(self.current) {
				let data = self.data;
				screen.update(|name| data.read(name));
			}
		}
	}

	fn render(&mut self, display: &mut Display, canvas: &mut Canvas) -> Result<(), String> {
		let damage = match self.screens.get_mut(self.current) {
			Some(screen) => {
				if self.shown != Some(self.current) {
					screen.invalidate();
					self.shown = Some(self.current);
				}
				// The overlays below are redrawn every frame, so their areas
				// must be repainted under them too.
				screen.add_damage(FpsCounter::AREA);
				screen.add_damage(BatteryOverlay::AREA);
				let mut damage = screen.draw(canvas);
				damage.rects.extend([FpsCounter::AREA, BatteryOverlay::AREA]);
				damage
			}
			None => {
				canvas.clear(gauge_render::Rgba::BLACK);
				Damage::full(WIDTH, HEIGHT)
			}
		};
		self.fps.tick();
		self.fps.draw(canvas);
		self.battery.draw(canvas);
		display.present(canvas, &damage)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use gauge_render::gauges::{GaugeConfig, GaugeType, ScreenConfig};
	use sdl2::keyboard::Keycode;

	fn screen(t: GaugeType) -> ScreenConfig {
		ScreenConfig {
			gauge: GaugeConfig {
				gauge_type: t,
				data_source: "Rpm".into(),
				max_value: 8000.0,
				..GaugeConfig::default()
			},
			custom_definition_file: None,
		}
	}

	/// Drives the real loop through SDL's offscreen driver with synthetic keys.
	#[test]
	fn game_loop_navigates_resets_and_exits() {
		std::env::set_var("SDL_VIDEODRIVER", "offscreen");
		let Ok(mut display) = Display::open("test", WIDTH, HEIGHT, false) else {
			eprintln!("SDL offscreen driver unavailable; skipping");
			return;
		};
		let mut canvas = Canvas::new(WIDTH, HEIGHT);
		let mut input = Input::new();
		let config = AppConfig {
			screens: vec![screen(GaugeType::Needle), screen(GaugeType::MinMax), screen(GaugeType::Digital)],
			simulate_battery_percent: Some(25),
			..AppConfig::default()
		};
		let mut game = GameLoop::new(config, None, Feed::simulated());

		assert!(game.step(&mut display, &mut canvas, &mut input).unwrap());
		assert_eq!(game.current, 0);
		// Gauge drawn, FPS text top-left, battery icon (25 % = warning) top-right.
		assert!(canvas.pixmap().pixels().iter().filter(|p| p.green() > 150).count() > 1000);
		assert!((0..30).any(|y| (8..60).any(|x| canvas.pixel(x, y).unwrap().r > 200)), "FPS text");
		let orange = (0..36).any(|y| {
			(570..640).any(|x| {
				let p = canvas.pixel(x, y).unwrap();
				p.r > 200 && p.g > 100 && p.g < 160 && p.b < 40
			})
		});
		assert!(orange, "battery warning icon");

		display.push_key(Keycode::Right);
		assert!(game.step(&mut display, &mut canvas, &mut input).unwrap());
		assert_eq!(game.current, 1);
		display.push_key(Keycode::Left);
		game.step(&mut display, &mut canvas, &mut input).unwrap();
		display.push_key(Keycode::Left);
		game.step(&mut display, &mut canvas, &mut input).unwrap();
		assert_eq!(game.current, 2, "wraps around");

		// Tab opens the editor: delete screen 1 (Delete = X), then Escape closes it
		// without quitting, and the screens are rebuilt.
		for key in [Keycode::Tab, Keycode::Delete, Keycode::Escape] {
			display.push_key(key);
			assert!(game.step(&mut display, &mut canvas, &mut input).unwrap());
			if key == Keycode::Delete {
				assert!(game.editor.is_some());
				assert!((60..300).any(|x| canvas.pixel(x, 80).is_some_and(|p| p.g > 200 && p.b > 200)), "editor drawn");
			}
		}
		assert!(game.editor.is_none());
		assert_eq!(game.screens.len(), 2);
		assert_eq!(game.current, 1, "clamped to the last screen");

		display.push_key(Keycode::Escape);
		assert!(!game.step(&mut display, &mut canvas, &mut input).unwrap(), "MENU exits");
	}
}
