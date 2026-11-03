//! Motorsport Electronics ECU data: CAN frame decoding (port of `ME1_4NET`),
//! the aggregated [`MeData`] snapshot, the data-source name mapping used by
//! gauge configs (`DataSourceMapper`) and the RaceChrono BLE packet format the
//! ECU streams over.
//!
//! Decoding is bit-for-bit identical to the C# library, including where it
//! scales in `double` (ME1_2's AFR) versus `float` (everything else).

mod frames;
mod mapper;
pub mod racechrono;

pub use frames::*;
pub use mapper::{DATA_SOURCE_NAMES, LEGACY_ALIASES};

use std::fmt;

/// CAN identifiers of the frames the ECU broadcasts (`Pid`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u16)]
pub enum Pid {
	Me1_1 = 0x300,
	Me1_2 = 0x301,
	Me1_3 = 0x302,
	Me1_4 = 0x303,
	Me1_5 = 0x304,
	Me1_6 = 0x305,
	Me1_7 = 0x306,
	Me1_8 = 0x307,
}

impl Pid {
	pub const ALL: [Pid; 8] =
		[Pid::Me1_1, Pid::Me1_2, Pid::Me1_3, Pid::Me1_4, Pid::Me1_5, Pid::Me1_6, Pid::Me1_7, Pid::Me1_8];

	/// The PID for a CAN identifier, if it is one of ours.
	pub fn from_id(id: u16) -> Option<Pid> {
		Pid::ALL.into_iter().find(|p| *p as u16 == id)
	}
}

/// A frame payload was shorter than its layout requires.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DecodeError {
	pub pid: Pid,
	pub needed: usize,
	pub got: usize,
}

impl fmt::Display for DecodeError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "payload too short for {:?}: need {} bytes, got {}", self.pid, self.needed, self.got)
	}
}

impl std::error::Error for DecodeError {}

/// One decoded frame (`ICanFrame`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Frame {
	Me1_1(Me1_1),
	Me1_2(Me1_2),
	Me1_3(Me1_3),
	Me1_4(Me1_4),
	Me1_5(Me1_5),
	Me1_6(Me1_6),
	Me1_7(Me1_7),
	Me1_8(Me1_8),
}

/// Decodes a payload for the given PID (`CanDecoder.Decode`).
pub fn decode(pid: Pid, payload: &[u8]) -> Result<Frame, DecodeError> {
	Ok(match pid {
		Pid::Me1_1 => Frame::Me1_1(Me1_1::decode(payload)?),
		Pid::Me1_2 => Frame::Me1_2(Me1_2::decode(payload)?),
		Pid::Me1_3 => Frame::Me1_3(Me1_3::decode(payload)?),
		Pid::Me1_4 => Frame::Me1_4(Me1_4::decode(payload)?),
		Pid::Me1_5 => Frame::Me1_5(Me1_5::decode(payload)?),
		Pid::Me1_6 => Frame::Me1_6(Me1_6::decode(payload)?),
		Pid::Me1_7 => Frame::Me1_7(Me1_7::decode(payload)?),
		Pid::Me1_8 => Frame::Me1_8(Me1_8::decode(payload)?),
	})
}

/// Latest value of every ECU channel (`MEData`). Plain data: share it behind
/// a lock and copy it out for each frame (`Snapshot`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MeData {
	// ME1_1
	pub rpm: u16,
	pub throttle_position: f32,
	pub map: f32,
	pub iat: f32,
	// ME1_2
	pub rpm_hard_limit: i32,
	pub afr_curr1: f32,
	pub afr_curr2: f32,
	pub lambda_trim: i32,
	pub afr_target: f32,
	pub fuel_eth_perc: f32,
	// ME1_3
	pub ign_adv_angle: i16,
	pub ign_dwell: i16,
	pub pri_inj_angle: i16,
	pub pri_inj_pw: i16,
	// ME1_4
	pub pri_inj_duty: f32,
	pub sec_inj_duty: f32,
	pub sec_inj_angle: f32,
	pub sec_inj_pw: f32,
	pub boost_ctrl_duty: f32,
	// ME1_5
	pub oil_temp: f32,
	pub oil_pressure: f32,
	pub clt: f32,
	pub vbat: f32,
	// ME1_6
	pub gear_pos: u8,
	pub map_target: u16,
	pub vehicle_speed: u16,
	pub eps_ev_msk: u16,
	// ME1_7
	pub knock_peak_reading: u16,
	pub knock_ign_adv_mod: f32,
	pub fuel_pressure: u16,
	pub fuel_temp: u8,
	pub knock_evs_cnt: u16,
	// ME1_8
	pub egt1: f32,
	pub egt2: f32,
	pub gpt1: i16,
	pub gpt2: i16,
}

impl MeData {
	/// Copies a frame's fields in, leaving other groups untouched (`Apply`).
	pub fn apply(&mut self, frame: &Frame) {
		match *frame {
			Frame::Me1_1(f) => {
				self.rpm = f.rpm;
				self.throttle_position = f.throttle_position;
				self.map = f.map;
				self.iat = f.iat;
			}
			Frame::Me1_2(f) => {
				self.rpm_hard_limit = f.rpm_hard_limit;
				self.afr_curr1 = f.afr_curr1;
				self.afr_curr2 = f.afr_curr2;
				self.lambda_trim = f.lambda_trim;
				self.afr_target = f.afr_target;
				self.fuel_eth_perc = f.fuel_eth_perc;
			}
			Frame::Me1_3(f) => {
				self.ign_adv_angle = f.ign_adv_angle;
				self.ign_dwell = f.ign_dwell;
				self.pri_inj_angle = f.pri_inj_angle;
				self.pri_inj_pw = f.pri_inj_pw;
			}
			Frame::Me1_4(f) => {
				self.pri_inj_duty = f.pri_inj_duty;
				self.sec_inj_duty = f.sec_inj_duty;
				self.sec_inj_angle = f.sec_inj_angle;
				self.sec_inj_pw = f.sec_inj_pw;
				self.boost_ctrl_duty = f.boost_ctrl_duty;
			}
			Frame::Me1_5(f) => {
				self.oil_temp = f.oil_temp;
				self.oil_pressure = f.oil_pressure;
				self.clt = f.clt;
				self.vbat = f.vbat;
			}
			Frame::Me1_6(f) => {
				self.gear_pos = f.gear_pos;
				self.map_target = f.map_target;
				self.vehicle_speed = f.vehicle_speed;
				self.eps_ev_msk = f.eps_ev_msk;
			}
			Frame::Me1_7(f) => {
				self.knock_peak_reading = f.knock_peak_reading;
				self.knock_ign_adv_mod = f.knock_ign_adv_mod;
				self.fuel_pressure = f.fuel_pressure;
				self.fuel_temp = f.fuel_temp;
				self.knock_evs_cnt = f.knock_evs_cnt;
			}
			Frame::Me1_8(f) => {
				self.egt1 = f.egt1;
				self.egt2 = f.egt2;
				self.gpt1 = f.gpt1;
				self.gpt2 = f.gpt2;
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn decode_dispatches_each_pid() {
		assert!(matches!(decode(Pid::Me1_1, &[0; 8]), Ok(Frame::Me1_1(_))));
		assert!(matches!(decode(Pid::Me1_2, &[0; 8]), Ok(Frame::Me1_2(_))));
		assert!(matches!(decode(Pid::Me1_3, &[0; 8]), Ok(Frame::Me1_3(_))));
		assert!(matches!(decode(Pid::Me1_4, &[0; 7]), Ok(Frame::Me1_4(_))));
		assert!(matches!(decode(Pid::Me1_5, &[0; 8]), Ok(Frame::Me1_5(_))));
		assert!(matches!(decode(Pid::Me1_6, &[0; 7]), Ok(Frame::Me1_6(_))));
		assert!(matches!(decode(Pid::Me1_7, &[0; 7]), Ok(Frame::Me1_7(_))));
		assert!(matches!(decode(Pid::Me1_8, &[0; 8]), Ok(Frame::Me1_8(_))));
	}

	#[test]
	fn unknown_ids_have_no_pid() {
		// C#: Decode((Pid)0xFFFF, ...) throws KeyNotFoundException.
		assert_eq!(Pid::from_id(0xFFFF), None);
		assert_eq!(Pid::from_id(0x308), None);
		assert_eq!(Pid::from_id(0x305), Some(Pid::Me1_6));
	}

	#[test]
	fn decode_me1_1_payload_values() {
		let Ok(Frame::Me1_1(f)) = decode(Pid::Me1_1, &[0xDC, 0x05, 0, 0, 0, 0, 0, 0]) else { panic!() };
		assert_eq!(f.rpm, 1500);
	}

	#[test]
	fn decode_me1_6_payload_values() {
		let Ok(Frame::Me1_6(f)) = decode(Pid::Me1_6, &[0x04, 0x90, 0x01, 0xC8, 0x00, 0x00, 0x00]) else { panic!() };
		assert_eq!((f.gear_pos, f.map_target, f.vehicle_speed), (4, 400, 200));
	}

	fn approx(expected: f32, actual: f32, decimals: i32) {
		let tol = 0.5 * 10f32.powi(-decimals);
		assert!((expected - actual).abs() <= tol, "expected {expected}, got {actual} (±{tol})");
	}

	fn applied(payload: &[u8], pid: Pid) -> MeData {
		let mut d = MeData::default();
		d.apply(&decode(pid, payload).unwrap());
		d
	}

	#[test]
	fn apply_me1_1() {
		let d = applied(&[0xB8, 0x0B, 0xE8, 0x03, 0xF4, 0x01, 0xFA, 0x00], Pid::Me1_1);
		assert_eq!(d.rpm, 3000);
		approx(100.0, d.throttle_position, 1);
		approx(5.0, d.map, 2);
		approx(25.0, d.iat, 1);
	}

	#[test]
	fn apply_me1_2() {
		let d = applied(&[0xA0, 0x0F, 0x78, 0x82, 0xC4, 0x09, 0x0E, 0x55], Pid::Me1_2);
		assert_eq!(d.rpm_hard_limit, 4000);
		assert_eq!(d.afr_curr1, 13.5);
		assert_eq!(d.lambda_trim, 2500);
		assert_eq!(d.fuel_eth_perc, 85.0);
	}

	#[test]
	fn apply_me1_3() {
		let d = applied(&[0xE8, 0x03, 0x30, 0xF8, 0xFF, 0x7F, 0x00, 0x80], Pid::Me1_3);
		assert_eq!((d.ign_adv_angle, d.ign_dwell, d.pri_inj_angle, d.pri_inj_pw), (1000, -2000, i16::MAX, i16::MIN));
	}

	#[test]
	fn apply_me1_4() {
		let d = applied(&[100, 60, 0xAC, 0x0D, 0x32, 0x00, 140], Pid::Me1_4);
		assert_eq!(d.pri_inj_duty, 50.0);
		assert_eq!(d.sec_inj_duty, 30.0);
		approx(350.0, d.sec_inj_angle, 1);
		approx(5.0, d.sec_inj_pw, 1);
		assert_eq!(d.boost_ctrl_duty, 70.0);
	}

	#[test]
	fn apply_me1_5() {
		let d = applied(&[0x84, 0x03, 0xA0, 0x0F, 0x52, 0x03, 0x8A, 0x00], Pid::Me1_5);
		approx(90.0, d.oil_temp, 1);
		approx(400.0, d.oil_pressure, 1);
		approx(85.0, d.clt, 1);
		approx(13.8, d.vbat, 1);
	}

	#[test]
	fn apply_me1_6() {
		let d = applied(&[0x03, 0x50, 0x00, 0x90, 0x01, 0x00, 0x00], Pid::Me1_6);
		assert_eq!((d.gear_pos, d.map_target, d.vehicle_speed, d.eps_ev_msk), (3, 80, 400, 0));
	}

	#[test]
	fn apply_me1_7() {
		let d = applied(&[0x34, 0x12, 0xEC, 80, 45, 0x05, 0x00], Pid::Me1_7);
		assert_eq!(d.knock_peak_reading, 0x1234);
		approx(-2.0, d.knock_ign_adv_mod, 1);
		assert_eq!((d.fuel_pressure, d.fuel_temp, d.knock_evs_cnt), (400, 45, 5));
	}

	#[test]
	fn apply_me1_8() {
		let d = applied(&[0x58, 0x1B, 0x64, 0x19, 0x64, 0x00, 0xCE, 0xFF], Pid::Me1_8);
		approx(700.0, d.egt1, 1);
		approx(650.0, d.egt2, 1);
		assert_eq!((d.gpt1, d.gpt2), (100, -50));
	}

	#[test]
	fn apply_keeps_other_groups() {
		let mut d = MeData::default();
		d.apply(&decode(Pid::Me1_1, &[0xB8, 0x0B, 0, 0, 0, 0, 0, 0]).unwrap());
		d.apply(&decode(Pid::Me1_5, &[0x84, 0x03, 0, 0, 0, 0, 0, 0]).unwrap());
		assert_eq!(d.rpm, 3000);
		approx(90.0, d.oil_temp, 1);
	}

	#[test]
	fn apply_same_frame_overwrites() {
		let mut d = MeData::default();
		d.apply(&decode(Pid::Me1_1, &[0xB8, 0x0B, 0, 0, 0, 0, 0, 0]).unwrap());
		assert_eq!(d.rpm, 3000);
		d.apply(&decode(Pid::Me1_1, &[0xD0, 0x07, 0, 0, 0, 0, 0, 0]).unwrap());
		assert_eq!(d.rpm, 2000);
	}
}
