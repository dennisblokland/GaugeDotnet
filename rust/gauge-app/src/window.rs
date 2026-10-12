//! SDL2 window (the `GaugeSDL` + `GameLoop` part of the C# app): renders on
//! the CPU each frame and streams the pixels into a texture.

use std::path::Path;
use std::time::{Duration, Instant};

use gauge_render::Canvas;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;

use crate::feed::Feed;
use crate::{load, DrawFn, Loaded};

/// Draws a status banner (connection state / errors) over the bottom of the frame.
fn draw_banner(canvas: &mut Canvas, text: &str) {
	use gauge_render::canvas::{rect, Paint};
	use gauge_render::text::{default_typeface, TextAlign};
	let (w, h) = (canvas.width() as f32, canvas.height() as f32);
	let lines: Vec<&str> = text.lines().collect();
	let size = 22.0;
	let top = h - 16.0 - lines.len() as f32 * (size + 4.0) - 12.0;
	if let Some(r) = rect(0.0, top, w, h) {
		canvas.draw_rect(r, &Paint::fill(gauge_render::Rgba::new(0, 0, 0, 200)));
	}
	for (i, line) in lines.iter().enumerate() {
		let y = top + 12.0 + size + i as f32 * (size + 4.0);
		canvas.draw_text(
			line,
			w / 2.0,
			y,
			TextAlign::Center,
			default_typeface(),
			size,
			&Paint::fill(gauge_render::Rgba::rgb(255, 204, 0)),
		);
	}
}

pub fn run(path: &Path, fullscreen: bool, mut feed: Feed) -> Result<(), String> {
	let (w, h) = crate::dimensions(path)?;
	let mut screens: Vec<DrawFn> = match load(path, w, h)? {
		Loaded::Custom(mut g) => vec![Box::new(move |c, s, reset| {
			if reset {
				g.invalidate();
			}
			g.update_from(|n| s.read(n));
			g.draw(c)
		})],
		Loaded::Screens(screens) => screens
			.into_iter()
			.map(|mut screen| -> DrawFn {
				Box::new(move |c, s, reset| {
					if reset {
						screen.invalidate();
					}
					screen.update(|n| s.read(n));
					screen.draw(c)
				})
			})
			.collect(),
	};

	let mut display = crate::display::Display::open("GaugeDotnet (Rust)", w, h, fullscreen)?;
	let mut canvas = Canvas::new(w, h);
	let mut current = 0usize;
	let mut shown = usize::MAX;
	let mut last_status: Option<String> = None;
	let (mut frames, mut fps_since) = (0u32, Instant::now());

	'main: loop {
		for event in display.events.poll_iter() {
			match event {
				Event::Quit { .. } | Event::KeyDown { keycode: Some(Keycode::Escape), .. } => break 'main,
				Event::KeyDown { keycode: Some(Keycode::Right), .. } => current = (current + 1) % screens.len(),
				Event::KeyDown { keycode: Some(Keycode::Left), .. } => {
					current = (current + screens.len() - 1) % screens.len()
				}
				_ => {}
			}
		}

		let data = feed.tick();
		let status = feed.status();
		// The banner is drawn over the frame, so repaint fully whenever it is
		// shown or changes; otherwise partial redraws would leave it behind.
		let reset = shown != current || status.is_some() || status != last_status;
		shown = current;
		last_status.clone_from(&status);
		let mut damage = screens[current](&mut canvas, &data, reset);
		if let Some(text) = &status {
			draw_banner(&mut canvas, text);
			damage = gauge_render::Damage::full(w, h);
		}
		display.present(&canvas, &damage)?;

		frames += 1;
		if fps_since.elapsed() >= Duration::from_secs(1) {
			let link = status.as_deref().map_or("live".to_string(), |s| s.replace('\n', " "));
			let title =
				format!("GaugeDotnet (Rust) — screen {}/{} — {frames} fps — {link}", current + 1, screens.len());
			display.set_title(&title);
			frames = 0;
			fps_since = Instant::now();
		}
	}
	Ok(())
}
