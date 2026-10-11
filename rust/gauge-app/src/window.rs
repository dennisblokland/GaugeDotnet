//! SDL2 window (the `GaugeSDL` + `GameLoop` part of the C# app): renders on
//! the CPU each frame and streams the pixels into a texture.

use std::path::Path;
use std::time::{Duration, Instant};

use gauge_render::Canvas;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::PixelFormatEnum;

use crate::sim::Simulator;
use crate::{load, DrawFn, Loaded};

pub fn run(path: &Path, fullscreen: bool) -> Result<(), String> {
	let (w, h) = crate::dimensions(path)?;
	let mut screens: Vec<DrawFn> = match load(path, w, h)? {
		Loaded::Custom(mut g) => vec![Box::new(move |c, s| {
			g.update_from(|n| s.read(n));
			g.draw(c);
		})],
		Loaded::Screens(screens) => screens
			.into_iter()
			.map(|mut screen| -> DrawFn {
				Box::new(move |c, s| {
					screen.update(|n| s.read(n));
					screen.draw(c);
				})
			})
			.collect(),
	};

	let sdl = sdl2::init()?;
	let video = sdl.video()?;
	let mut builder = video.window("GaugeDotnet (Rust)", w, h);
	builder.position_centered();
	if fullscreen {
		builder.fullscreen();
	}
	let window = builder.build().map_err(|e| e.to_string())?;
	let mut sdl_canvas = window.into_canvas().present_vsync().build().map_err(|e| e.to_string())?;
	let creator = sdl_canvas.texture_creator();
	// tiny-skia stores bytes as R,G,B,A; frames are opaque so premultiplication is moot.
	let mut texture = creator.create_texture_streaming(PixelFormatEnum::RGBA32, w, h).map_err(|e| e.to_string())?;
	let mut events = sdl.event_pump()?;
	let _joystick = sdl.joystick().ok().and_then(|j| j.open(0).ok());

	let mut canvas = Canvas::new(w, h);
	let mut sim = Simulator::new(0.0);
	let mut current = 0usize;
	let mut last = Instant::now();
	let (mut frames, mut fps_since) = (0u32, Instant::now());

	'main: loop {
		for event in events.poll_iter() {
			match event {
				Event::Quit { .. } | Event::KeyDown { keycode: Some(Keycode::Escape), .. } => break 'main,
				Event::KeyDown { keycode: Some(Keycode::Right), .. } => current = (current + 1) % screens.len(),
				Event::KeyDown { keycode: Some(Keycode::Left), .. } => {
					current = (current + screens.len() - 1) % screens.len()
				}
				Event::JoyHatMotion { state, .. } => match state {
					sdl2::joystick::HatState::Right => current = (current + 1) % screens.len(),
					sdl2::joystick::HatState::Left => current = (current + screens.len() - 1) % screens.len(),
					_ => {}
				},
				_ => {}
			}
		}

		let now = Instant::now();
		sim.advance(now.duration_since(last).as_secs_f64());
		last = now;

		screens[current](&mut canvas, &sim);
		texture.update(None, canvas.pixmap().data(), w as usize * 4).map_err(|e| e.to_string())?;
		sdl_canvas.copy(&texture, None, None)?;
		sdl_canvas.present();

		frames += 1;
		if fps_since.elapsed() >= Duration::from_secs(1) {
			let title = format!("GaugeDotnet (Rust) — screen {}/{} — {frames} fps", current + 1, screens.len());
			let _ = sdl_canvas.window_mut().set_title(&title);
			frames = 0;
			fps_since = Instant::now();
		}
	}
	Ok(())
}
