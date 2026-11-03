//! SDL2 output (the `GaugeSDL` part of the C# app). Frames are rendered on
//! the CPU into a [`Canvas`] and streamed into a texture; only damaged rects
//! are uploaded.

use gauge_render::{Canvas, Damage};
use sdl2::pixels::PixelFormatEnum;
use sdl2::render::{Texture, TextureCreator, WindowCanvas};
use sdl2::video::WindowContext;
use sdl2::{EventPump, Sdl};

pub struct Display {
	_sdl: Sdl,
	canvas: WindowCanvas,
	/// Leaked once (the display lives for the whole program) so the struct can
	/// hold the texture that borrows it.
	_creator: &'static TextureCreator<WindowContext>,
	texture: Texture<'static>,
	pub events: EventPump,
	pub width: u32,
	pub height: u32,
}

impl Display {
	pub fn open(title: &str, width: u32, height: u32, fullscreen: bool) -> Result<Self, String> {
		let sdl = sdl2::init()?;
		let video = sdl.video()?;
		let mut builder = video.window(title, width, height);
		builder.position_centered();
		if fullscreen {
			builder.fullscreen();
		}
		let window = builder.build().map_err(|e| e.to_string())?;
		let canvas = window.into_canvas().present_vsync().build().map_err(|e| e.to_string())?;
		// The display lives for the whole program; leaking its single texture
		// creator lets the struct hold the texture without self-references.
		let creator: &'static TextureCreator<WindowContext> = Box::leak(Box::new(canvas.texture_creator()));
		// tiny-skia stores bytes as R,G,B,A; frames are opaque so premultiplication is moot.
		let texture =
			creator.create_texture_streaming(PixelFormatEnum::RGBA32, width, height).map_err(|e| e.to_string())?;
		let events = sdl.event_pump()?;
		Ok(Self { _sdl: sdl, canvas, _creator: creator, texture, events, width, height })
	}

	/// Uploads the damaged parts of `frame` and presents it (vsync-paced).
	pub fn present(&mut self, frame: &Canvas, damage: &Damage) -> Result<(), String> {
		let pitch = self.width as usize * 4;
		let pixels = frame.pixmap().data();
		for r in &damage.rects {
			let rect = sdl2::rect::Rect::new(r.x0, r.y0, r.width() as u32, r.height() as u32);
			let start = r.y0 as usize * pitch + r.x0 as usize * 4;
			self.texture.update(Some(rect), &pixels[start..], pitch).map_err(|e| e.to_string())?;
		}
		self.canvas.copy(&self.texture, None, None)?;
		self.canvas.present();
		Ok(())
	}

	/// Uploads and presents the whole frame.
	pub fn present_full(&mut self, frame: &Canvas) -> Result<(), String> {
		self.present(frame, &Damage::full(self.width, self.height))
	}

	/// Injects a key press and release (tests drive the app with these).
	#[cfg(test)]
	pub fn push_key(&mut self, key: sdl2::keyboard::Keycode) {
		use sdl2::event::Event;
		let ev = self._sdl.event().expect("event subsystem");
		for down in [true, false] {
			let e = if down {
				Event::KeyDown {
					timestamp: 0,
					window_id: 0,
					keycode: Some(key),
					scancode: None,
					keymod: sdl2::keyboard::Mod::NOMOD,
					repeat: false,
				}
			} else {
				Event::KeyUp {
					timestamp: 0,
					window_id: 0,
					keycode: Some(key),
					scancode: None,
					keymod: sdl2::keyboard::Mod::NOMOD,
					repeat: false,
				}
			};
			ev.push_event(e).expect("push key event");
		}
	}

	pub fn set_title(&mut self, title: &str) {
		let _ = self.canvas.window_mut().set_title(title);
	}
}
