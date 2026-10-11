//! Full-screen states and overlays (ports of `SplashScreen`, `ErrorScreen`,
//! the `GameLoop` exit animation, `FpsCounter` and `BatteryOverlay`).

use std::path::Path;
use std::time::{Duration, Instant};

use gauge_render::canvas::{rect, Paint};
use gauge_render::text::{default_typeface, typeface, TextAlign};
use gauge_render::{Canvas, IRect, Rgba};

const W: f32 = 640.0;
const H: f32 = 480.0;

/// Horizontally centred text, positioned by measuring like the C# screens.
fn centered(canvas: &mut Canvas, text: &str, y: f32, font: &str, size: f32, color: Rgba) {
	let tf = typeface(font);
	let x = (W - tf.measure(text, size)) / 2.0;
	canvas.draw_text(text, x, y, TextAlign::Left, tf, size, &Paint::fill(color));
}

/// One splash frame (`SplashScreen.Render`): title, sweeping progress bar
/// while connecting, full bar plus "Ready" once done.
pub fn draw_splash(canvas: &mut Canvas, t: f64, done: bool, status: &str) {
	if let Some(r) = rect(0.0, 0.0, W, H) {
		canvas.draw_rect_vertical_gradient(r, Rgba::rgb(0x10, 0x10, 0x14), Rgba::BLACK);
	}
	let a = ((t / 0.4).clamp(0.0, 1.0) * 255.0) as u8;
	let race = typeface("Race Sport");
	let title = "GaugeDotnet";
	let title_w = race.measure(title, 64.0);
	let title_y = 210.0;
	let underline_w = title_w + 40.0;
	let accent = Rgba::rgb(0xFF, 0x33, 0x33).with_alpha(a);
	if let Some(r) = rect((W - underline_w) / 2.0, title_y + 14.0, (W + underline_w) / 2.0, title_y + 18.0) {
		canvas.draw_rect(r, &Paint::fill(accent));
	}
	centered(canvas, title, title_y, "Race Sport", 64.0, Rgba::WHITE.with_alpha(a));
	centered(
		canvas,
		"Motorsport Telemetry",
		title_y + 56.0,
		"Race Sport",
		22.0,
		Rgba::rgb(0xCC, 0xCC, 0xCC).with_alpha(a),
	);

	let (bar_w, bar_h) = (260.0, 6.0);
	let (bar_x, bar_y) = ((W - bar_w) / 2.0, 360.0);
	if let Some(r) = rect(bar_x, bar_y, bar_x + bar_w, bar_y + bar_h) {
		canvas.draw_round_rect(r, 3.0, 3.0, &Paint::fill(Rgba::rgb(0x22, 0x22, 0x22).with_alpha(a)));
		if done {
			canvas.draw_round_rect(r, 3.0, 3.0, &Paint::fill(accent));
		}
	}
	if !done {
		let seg_w = 80.0;
		let cycle = (t % 1.6 / 1.6) as f32;
		let seg_x = bar_x + (bar_w + seg_w) * cycle - seg_w;
		let x0 = seg_x.max(bar_x);
		let x1 = (seg_x + seg_w).min(bar_x + bar_w);
		if x1 > x0 {
			if let Some(r) = rect(x0, bar_y, x1, bar_y + bar_h) {
				canvas.draw_round_rect(r, 3.0, 3.0, &Paint::fill(accent));
			}
		}
	}
	let dots = ".".repeat(((t * 3.0) as usize) % 4);
	let text = if done { "Ready".to_string() } else { format!("{status}{dots}") };
	centered(canvas, &text, bar_y + 36.0, "Race Sport", 18.0, Rgba::rgb(0x99, 0x99, 0x99).with_alpha(a));
}

/// The error screen (`ErrorScreen`): red message lines and an exit hint.
pub fn draw_error(canvas: &mut Canvas, message: &str) {
	canvas.clear(Rgba::BLACK);
	let mut y = 200.0;
	for line in message.lines() {
		centered(canvas, line, y, "Race Sport", 24.0, Rgba::rgb(255, 0, 0));
		y += 34.0;
	}
	centered(canvas, "Press any button to exit", y + 20.0, "Race Sport", 24.0, Rgba::WHITE);
}

/// One frame of the exit animation (`GameLoop.RenderExitMessage`), `t` in 0..=1.
pub fn draw_exit(canvas: &mut Canvas, t: f32) {
	canvas.clear(Rgba::BLACK);
	let alpha = (255.0 * t) as u8;
	let (cx, cy) = (W / 2.0, H / 2.0);
	let half = W * 0.38 * t;
	let line = Paint::stroke(Rgba::rgb(0xFF, 0x6B, 0x35).with_alpha(alpha), 2.0);
	canvas.draw_line(cx - half, cy - 52.0, cx + half, cy - 52.0, &line);
	canvas.draw_line(cx - half, cy + 30.0, cx + half, cy + 30.0, &line);
	let tf = default_typeface();
	canvas.draw_text(
		"EXITING",
		cx,
		cy + 10.0,
		TextAlign::Center,
		tf,
		64.0,
		&Paint::fill(Rgba::WHITE.with_alpha(alpha)),
	);
	canvas.draw_text(
		"Goodbye",
		cx,
		cy + 58.0,
		TextAlign::Center,
		tf,
		24.0,
		&Paint::fill(Rgba::rgb(180, 180, 180).with_alpha(alpha)),
	);
}

/// `FpsCounter`: "FPS: 59.94" in the top-left corner, updated once a second.
pub struct FpsCounter {
	since: Instant,
	frames: u32,
	text: String,
}

impl FpsCounter {
	/// Area the counter draws into (for damage tracking).
	pub const AREA: IRect = IRect { x0: 0, y0: 0, x1: 160, y1: 34 };

	pub fn new() -> Self {
		Self { since: Instant::now(), frames: 0, text: "FPS: 0.00".into() }
	}

	pub fn tick(&mut self) {
		self.frames += 1;
		let elapsed = self.since.elapsed().as_secs_f64();
		if elapsed >= 1.0 {
			self.text = format!("FPS: {:.2}", self.frames as f64 / elapsed);
			self.frames = 0;
			self.since = Instant::now();
		}
	}

	pub fn draw(&self, canvas: &mut Canvas) {
		canvas.draw_text(&self.text, 10.0, 25.0, TextAlign::Left, default_typeface(), 20.0, &Paint::fill(Rgba::WHITE));
	}
}

/// `BatteryOverlay`: a battery icon in the top-right corner when charge is
/// at or below 30 %, blinking red at or below 15 %.
pub struct BatteryOverlay {
	simulated: Option<i32>,
	cached: Option<i32>,
	last_read: Option<Instant>,
}

impl BatteryOverlay {
	/// Area the icon draws into (for damage tracking).
	pub const AREA: IRect = IRect { x0: 560, y0: 0, x1: 640, y1: 36 };

	pub fn new(simulated: Option<i32>) -> Self {
		Self { simulated, cached: None, last_read: None }
	}

	fn percent(&mut self) -> Option<i32> {
		if self.simulated.is_some() {
			return self.simulated;
		}
		// sysfs is cheap, but there's no need to read it every frame.
		if self.last_read.map_or(true, |t| t.elapsed() >= Duration::from_secs(30)) {
			self.last_read = Some(Instant::now());
			self.cached = read_system_battery();
		}
		self.cached
	}

	pub fn draw(&mut self, canvas: &mut Canvas) {
		let Some(pct) = self.percent().filter(|p| *p <= 30) else {
			return;
		};
		let critical = pct <= 15;
		let millis =
			std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.subsec_millis());
		if critical && millis > 500 {
			return; // blink
		}
		let fill = if critical { Rgba::rgb(0xFF, 0x20, 0x20) } else { Rgba::rgb(0xFF, 0x8C, 0x00) };
		let outline = Rgba::rgb(0xCC, 0xCC, 0xCC);
		let (body_w, body_h, nub_w, nub_h, pad) = (48.0, 20.0, 5.0, 10.0, 12.0);
		let x = W - body_w - nub_w - pad;
		let y = 8.0;
		if let Some(r) = rect(x, y, x + body_w, y + body_h) {
			canvas.draw_round_rect(r, 3.0, 3.0, &Paint::stroke(outline, 1.5));
		}
		if let Some(r) = rect(x + body_w, y + (body_h - nub_h) / 2.0, x + body_w + nub_w, y + (body_h + nub_h) / 2.0) {
			canvas.draw_round_rect(r, 2.0, 2.0, &Paint::fill(outline));
		}
		let fill_w = (body_w - 4.0) * (pct as f32 / 100.0);
		if fill_w > 0.0 {
			canvas.draw_rect_xywh(x + 2.0, y + 2.0, fill_w, body_h - 4.0, &Paint::fill(fill));
		}
		let label = format!("{pct}%");
		canvas.draw_text(
			&label,
			x + body_w / 2.0,
			y + body_h / 2.0 + 4.0,
			TextAlign::Center,
			default_typeface(),
			11.0,
			&Paint::fill(Rgba::WHITE),
		);
	}
}

fn read_system_battery() -> Option<i32> {
	[
		"/sys/class/power_supply/axp20x-battery/capacity",
		"/sys/class/power_supply/BAT0/capacity",
		"/sys/class/power_supply/BAT1/capacity",
		"/sys/class/power_supply/battery/capacity",
	]
	.iter()
	.filter(|p| Path::new(p).exists())
	.find_map(|p| std::fs::read_to_string(p).ok()?.trim().parse::<i32>().ok())
	.map(|p| p.clamp(0, 100))
}

/// Keeps MuOS from dimming or sleeping while gauges are shown
/// (`ScreenKeepAlive.Poke`); a no-op elsewhere.
pub fn poke_keep_alive() {
	const DIR: &str = "/opt/muos/config/system";
	if Path::new(DIR).is_dir() {
		let _ = std::fs::write(format!("{DIR}/idle_inhibit"), "1");
	}
}
