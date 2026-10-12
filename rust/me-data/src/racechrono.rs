//! RaceChrono BLE CAN-bus protocol, as spoken by the ECU's BLE bridge.
//!
//! The peripheral exposes service `0x1FF8` with a notify characteristic
//! (`0x0001`) that streams CAN frames as `[can_id: u32 LE][payload…]`, and a
//! write characteristic (`0x0002`) for PID filter commands.

use crate::{decode, DecodeError, Frame, Pid};

/// Service `00001FF8-0000-1000-8000-00805F9B34FB`.
pub const SERVICE_UUID: u128 = 0x0000_1FF8_0000_1000_8000_0080_5F9B_34FB;
/// Notify characteristic carrying CAN frames.
pub const CAN_BUS_CHARACTERISTIC_UUID: u128 = 0x0000_0001_0000_1000_8000_0080_5F9B_34FB;
/// Write characteristic for PID filter commands.
pub const PID_CHARACTERISTIC_UUID: u128 = 0x0000_0002_0000_1000_8000_0080_5F9B_34FB;

/// "Allow all PIDs" (command `0x01`) with a 50 ms update interval (big-endian
/// u16). Must be re-sent after every (re)connect: the peripheral forgets it.
pub const ALLOW_ALL_PIDS: [u8; 3] = [0x01, (50u16 >> 8) as u8, (50u16 & 0xFF) as u8];

/// Decodes one notification. `None` for packets that are too short to carry
/// an id or that belong to a CAN id we don't know (they are ignored, as in
/// `MeDevice.DataReceived`); `Some(Err)` when a known frame is truncated.
pub fn parse_notification(packet: &[u8]) -> Option<Result<Frame, DecodeError>> {
	if packet.len() < 2 {
		return None;
	}
	let can_id = u16::from_le_bytes([packet[0], packet[1]]);
	let pid = Pid::from_id(can_id & 0x0FFF)?;
	let payload = packet.get(4..).unwrap_or(&[]);
	Some(decode(pid, payload))
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn parses_known_frames() {
		// can_id 0x300 (LE u32), rpm 3000.
		let packet = [0x00, 0x03, 0x00, 0x00, 0xB8, 0x0B, 0, 0, 0, 0, 0, 0];
		let Some(Ok(Frame::Me1_1(f))) = parse_notification(&packet) else { panic!() };
		assert_eq!(f.rpm, 3000);
	}

	#[test]
	fn masks_high_id_bits_like_csharp() {
		let packet = [0x04, 0xF3, 0x00, 0x00, 0x84, 0x03, 0, 0, 0, 0, 0, 0];
		assert!(matches!(parse_notification(&packet), Some(Ok(Frame::Me1_5(_)))));
	}

	#[test]
	fn ignores_unknown_and_tiny_packets() {
		assert_eq!(parse_notification(&[0x00]), None);
		assert_eq!(parse_notification(&[0x99, 0x01, 0, 0, 1, 2, 3, 4, 5, 6, 7, 8]), None);
	}

	#[test]
	fn truncated_known_frame_is_an_error() {
		assert!(matches!(parse_notification(&[0x00, 0x03, 0, 0, 1, 2]), Some(Err(_))));
		assert!(matches!(parse_notification(&[0x00, 0x03]), Some(Err(_))));
	}

	#[test]
	fn uuids_match_the_csharp_strings() {
		assert_eq!(format!("{SERVICE_UUID:032X}"), "00001FF800001000800000805F9B34FB");
		assert_eq!(ALLOW_ALL_PIDS, [0x01, 0x00, 0x32]);
	}
}
