//! Gamepad and keyboard input (port of `RG35XX.Core.GamePads`,
//! `LinuxGamePadReader`, `KeyBus` and `InputHandler`).
//!
//! On the device, buttons come straight from the kernel joystick device
//! (`/dev/input/js0`), like the C# release build. When there is no joystick
//! (desktop), SDL keyboard events are mapped to the same keys, like the C#
//! debug build. Only one source is used, so PortMaster's gptokeyb (which
//! turns buttons into key presses) doesn't produce double inputs.

use std::collections::VecDeque;
use std::fs::File;
use std::io::Read;
use std::sync::mpsc::{channel, Receiver};

use sdl2::event::Event;
use sdl2::keyboard::Keycode;

/// A joystick event code: `(number << 16) | (value as u16)`, exactly the
/// C# `GamepadKey` enum values. Two names share codes (`UP_DOWN_UP` =
/// `L1_UP`, `LEFT_RIGHT_UP` = `Y_UP`), as in C#.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GamepadKey(pub i32);

#[allow(dead_code)]
impl GamepadKey {
	pub const NONE: Self = Self(-1);
	pub const L1_DOWN: Self = Self(262145);
	pub const L2_DOWN: Self = Self(589825);
	pub const R1_DOWN: Self = Self(327681);
	pub const R2_DOWN: Self = Self(655361);
	pub const L1_UP: Self = Self(262144);
	pub const L2_UP: Self = Self(589824);
	pub const R1_UP: Self = Self(327680);
	pub const R2_UP: Self = Self(655360);
	pub const UP: Self = Self(294913);
	pub const DOWN: Self = Self(294911);
	pub const LEFT: Self = Self(229377);
	pub const RIGHT: Self = Self(229375);
	pub const MENU_DOWN: Self = Self(524289);
	pub const MENU_UP: Self = Self(720897);
	pub const SELECT_DOWN: Self = Self(458753);
	pub const START_DOWN: Self = Self(393217);
	pub const SELECT_UP: Self = Self(458752);
	pub const START_UP: Self = Self(393216);
	pub const A_DOWN: Self = Self(1);
	pub const A_UP: Self = Self(0);
	pub const B_DOWN: Self = Self(65537);
	pub const B_UP: Self = Self(65536);
	pub const X_DOWN: Self = Self(131073);
	pub const Y_DOWN: Self = Self(196609);
	pub const X_UP: Self = Self(131072);
	pub const Y_UP: Self = Self(196608);
	pub const UP_DOWN_UP: Self = Self(262144);
	pub const LEFT_RIGHT_UP: Self = Self(196608);

	/// Decodes a Linux `struct js_event` (time u32, value i16, type u8, number u8).
	pub fn from_js_event(event: &[u8; 8]) -> Self {
		let value = u16::from_le_bytes([event[4], event[5]]);
		let number = event[7];
		Self(((number as i32) << 16) | value as i32)
	}

	/// A button press or d-pad push. Releases have a zero value, except the
	/// keyboard's `MENU_UP`, which C# gives a code of its own.
	pub fn is_press(self) -> bool {
		self != Self::NONE && self != Self::MENU_UP && self.0 & 0xFFFF != 0
	}

	/// The C# `KeyBus` keyboard mapping (desktop builds).
	pub fn from_keycode(key: Keycode, down: bool) -> Option<Self> {
		Some(match (key, down) {
			(Keycode::A, true) => Self::A_DOWN,
			(Keycode::A, false) => Self::A_UP,
			(Keycode::Return, true) => Self::START_DOWN,
			(Keycode::Return, false) => Self::START_UP,
			(Keycode::B, true) => Self::B_DOWN,
			(Keycode::B, false) => Self::B_UP,
			(Keycode::Y, true) => Self::Y_DOWN,
			(Keycode::Y, false) => Self::Y_UP,
			(Keycode::X, true) => Self::X_DOWN,
			(Keycode::X, false) => Self::X_UP,
			(Keycode::Up, true) => Self::UP,
			(Keycode::Down, true) => Self::DOWN,
			(Keycode::Up | Keycode::Down, false) => Self::UP_DOWN_UP,
			(Keycode::Left, true) => Self::LEFT,
			(Keycode::Right, true) => Self::RIGHT,
			(Keycode::Left | Keycode::Right, false) => Self::LEFT_RIGHT_UP,
			(Keycode::Escape, true) => Self::MENU_DOWN,
			(Keycode::Escape, false) => Self::MENU_UP,
			(Keycode::Comma, true) => Self::L1_DOWN,
			(Keycode::Comma, false) => Self::L1_UP,
			(Keycode::Period, true) => Self::R1_DOWN,
			(Keycode::Period, false) => Self::R1_UP,
			_ => return None,
		})
	}
}

/// Reads `/dev/input/js0` on a background thread (`LinuxGamePadReader`).
struct JoystickReader {
	rx: Receiver<GamepadKey>,
}

impl JoystickReader {
	fn open(path: &str) -> Option<Self> {
		let mut file = match File::open(path) {
			Ok(f) => f,
			Err(e) => {
				eprintln!("[input] no gamepad at {path} ({e}); using the keyboard");
				return None;
			}
		};
		let (tx, rx) = channel();
		std::thread::Builder::new()
			.name("gamepad".into())
			.spawn(move || {
				let mut buf = [0u8; 8];
				while file.read_exact(&mut buf).is_ok() {
					// Skip the synthetic "initial state" events (JS_EVENT_INIT).
					if buf[6] & 0x80 != 0 {
						continue;
					}
					if tx.send(GamepadKey::from_js_event(&buf)).is_err() {
						break;
					}
				}
			})
			.ok()?;
		Some(Self { rx })
	}
}

/// Per-frame input (`InputHandler`): at most one gamepad key per poll, the
/// last SDL key pressed this frame, and whether the window was closed.
pub struct Input {
	joystick: Option<JoystickReader>,
	keyboard: VecDeque<GamepadKey>,
	pub key: GamepadKey,
	pub sdl_key: Option<Keycode>,
	pub quit: bool,
}

impl Input {
	pub fn new() -> Self {
		Self {
			joystick: JoystickReader::open("/dev/input/js0"),
			keyboard: VecDeque::new(),
			key: GamepadKey::NONE,
			sdl_key: None,
			quit: false,
		}
	}

	pub fn poll(&mut self, events: &mut sdl2::EventPump) {
		self.sdl_key = None;
		self.quit = false;
		for event in events.poll_iter() {
			match event {
				Event::Quit { .. } => self.quit = true,
				Event::KeyDown { keycode: Some(k), repeat, .. } => {
					if !repeat {
						self.sdl_key = Some(k);
					}
					if self.joystick.is_none() {
						self.keyboard.extend(GamepadKey::from_keycode(k, true));
					}
				}
				Event::KeyUp { keycode: Some(k), .. } if self.joystick.is_none() => {
					self.keyboard.extend(GamepadKey::from_keycode(k, false));
				}
				_ => {}
			}
		}
		// One key per frame, but releases never hold back a press queued
		// behind them (a tap is a press + release in the same frame).
		self.key = GamepadKey::NONE;
		loop {
			let next = match &self.joystick {
				Some(j) => j.rx.try_recv().ok(),
				None => self.keyboard.pop_front(),
			};
			let Some(key) = next else { break };
			self.key = key;
			if key.is_press() {
				break;
			}
		}
	}

	/// Any button or key pressed this poll (for "press any button" screens).
	pub fn any_pressed(&self) -> bool {
		self.sdl_key.is_some() || self.key.is_press()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn js(value: i16, number: u8) -> [u8; 8] {
		let v = value.to_le_bytes();
		[0, 0, 0, 0, v[0], v[1], 1, number]
	}

	#[test]
	fn joystick_events_map_to_csharp_codes() {
		assert_eq!(GamepadKey::from_js_event(&js(1, 0)), GamepadKey::A_DOWN);
		assert_eq!(GamepadKey::from_js_event(&js(0, 0)), GamepadKey::A_UP);
		assert_eq!(GamepadKey::from_js_event(&js(1, 5)), GamepadKey::R1_DOWN);
		assert_eq!(GamepadKey::from_js_event(&js(1, 8)), GamepadKey::MENU_DOWN);
		assert_eq!(GamepadKey::from_js_event(&js(1, 7)), GamepadKey::SELECT_DOWN);
		// D-pad axes: -32767 / 32767 on axis 3 (left/right) and 4 (up/down).
		assert_eq!(GamepadKey::from_js_event(&js(-32767, 3)), GamepadKey::LEFT);
		assert_eq!(GamepadKey::from_js_event(&js(32767, 3)), GamepadKey::RIGHT);
		assert_eq!(GamepadKey::from_js_event(&js(-32767, 4)), GamepadKey::UP);
		assert_eq!(GamepadKey::from_js_event(&js(32767, 4)), GamepadKey::DOWN);
		assert_eq!(GamepadKey::from_js_event(&js(0, 3)), GamepadKey::LEFT_RIGHT_UP);
	}

	#[test]
	fn keyboard_mapping_matches_keybus() {
		assert_eq!(GamepadKey::from_keycode(Keycode::Escape, true), Some(GamepadKey::MENU_DOWN));
		assert_eq!(GamepadKey::from_keycode(Keycode::Period, true), Some(GamepadKey::R1_DOWN));
		assert_eq!(GamepadKey::from_keycode(Keycode::Right, true), Some(GamepadKey::RIGHT));
		assert_eq!(GamepadKey::from_keycode(Keycode::Right, false), Some(GamepadKey::LEFT_RIGHT_UP));
		assert_eq!(GamepadKey::from_keycode(Keycode::Q, true), None);
		assert!(GamepadKey::MENU_DOWN.is_press() && GamepadKey::LEFT.is_press());
		assert!(
			!GamepadKey::MENU_UP.is_press() && !GamepadKey::A_UP.is_press() && !GamepadKey::LEFT_RIGHT_UP.is_press()
		);
	}
}
