//! The eight ECU frames (`ME1_4NET/Frames`). All multi-byte fields are
//! little-endian.

use crate::{DecodeError, Pid};

fn u16le(p: &[u8], i: usize) -> u16 {
	u16::from_le_bytes([p[i], p[i + 1]])
}

fn i16le(p: &[u8], i: usize) -> i16 {
	i16::from_le_bytes([p[i], p[i + 1]])
}

fn need(pid: Pid, payload: &[u8], needed: usize) -> Result<(), DecodeError> {
	if payload.len() < needed {
		Err(DecodeError { pid, needed, got: payload.len() })
	} else {
		Ok(())
	}
}

/// ME1_1 (0x300): rpm, throttle position (%), MAP (kPa), intake air temp (°C).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Me1_1 {
	pub rpm: u16,
	pub throttle_position: f32,
	pub map: f32,
	pub iat: f32,
}

impl Me1_1 {
	pub fn decode(p: &[u8]) -> Result<Self, DecodeError> {
		need(Pid::Me1_1, p, 8)?;
		Ok(Self {
			rpm: u16le(p, 0),
			throttle_position: i16le(p, 2) as f32 * 0.1,
			map: u16le(p, 4) as f32 * 0.01,
			iat: i16le(p, 6) as f32 * 0.1,
		})
	}
}

/// ME1_2 (0x301): rev limit, AFR bank 1/2, lambda trim, AFR target, ethanol %.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Me1_2 {
	pub rpm_hard_limit: i32,
	pub afr_curr1: f32,
	pub afr_curr2: f32,
	pub lambda_trim: i32,
	pub afr_target: f32,
	pub fuel_eth_perc: f32,
}

impl Me1_2 {
	pub fn decode(p: &[u8]) -> Result<Self, DecodeError> {
		need(Pid::Me1_2, p, 8)?;
		// The C# scales AFR in double before narrowing; keep that for identical values.
		let afr = |raw: u8| (raw as f64 * 0.05 + 7.5) as f32;
		Ok(Self {
			rpm_hard_limit: u16le(p, 0) as i32,
			afr_curr1: afr(p[2]),
			afr_curr2: afr(p[3]),
			lambda_trim: u16le(p, 4) as i32,
			afr_target: p[6] as f32,
			fuel_eth_perc: p[7] as f32,
		})
	}
}

/// ME1_3 (0x302): ignition advance, dwell, primary injection angle and pulse width.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Me1_3 {
	pub ign_adv_angle: i16,
	pub ign_dwell: i16,
	pub pri_inj_angle: i16,
	pub pri_inj_pw: i16,
}

impl Me1_3 {
	pub fn decode(p: &[u8]) -> Result<Self, DecodeError> {
		need(Pid::Me1_3, p, 8)?;
		Ok(Self {
			ign_adv_angle: i16le(p, 0),
			ign_dwell: i16le(p, 2),
			pri_inj_angle: i16le(p, 4),
			pri_inj_pw: i16le(p, 6),
		})
	}
}

/// ME1_4 (0x303): injector duties (%), secondary injection angle/pw, boost duty (%).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Me1_4 {
	pub pri_inj_duty: f32,
	pub sec_inj_duty: f32,
	pub sec_inj_angle: f32,
	pub sec_inj_pw: f32,
	pub boost_ctrl_duty: f32,
}

impl Me1_4 {
	pub fn decode(p: &[u8]) -> Result<Self, DecodeError> {
		need(Pid::Me1_4, p, 7)?;
		Ok(Self {
			pri_inj_duty: p[0] as f32 * 0.5,
			sec_inj_duty: p[1] as f32 * 0.5,
			sec_inj_angle: i16le(p, 2) as f32 * 0.1,
			sec_inj_pw: i16le(p, 4) as f32 * 0.1,
			boost_ctrl_duty: p[6] as f32 * 0.5,
		})
	}
}

/// ME1_5 (0x304): oil temp (°C), oil pressure (kPa), coolant (°C), battery (V).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Me1_5 {
	pub oil_temp: f32,
	pub oil_pressure: f32,
	pub clt: f32,
	pub vbat: f32,
}

impl Me1_5 {
	pub fn decode(p: &[u8]) -> Result<Self, DecodeError> {
		need(Pid::Me1_5, p, 8)?;
		Ok(Self {
			oil_temp: i16le(p, 0) as f32 * 0.1,
			oil_pressure: i16le(p, 2) as f32 * 0.1,
			clt: i16le(p, 4) as f32 * 0.1,
			vbat: i16le(p, 6) as f32 * 0.1,
		})
	}
}

/// ME1_6 (0x305): gear, MAP target, vehicle speed, EPS event mask.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Me1_6 {
	pub gear_pos: u8,
	pub map_target: u16,
	pub vehicle_speed: u16,
	pub eps_ev_msk: u16,
}

impl Me1_6 {
	pub fn decode(p: &[u8]) -> Result<Self, DecodeError> {
		need(Pid::Me1_6, p, 7)?;
		Ok(Self { gear_pos: p[0], map_target: u16le(p, 1), vehicle_speed: u16le(p, 3), eps_ev_msk: u16le(p, 5) })
	}
}

/// ME1_7 (0x306): knock peak, knock ignition retard (°), fuel pressure (kPa), fuel temp (°C), knock count.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Me1_7 {
	pub knock_peak_reading: u16,
	pub knock_ign_adv_mod: f32,
	pub fuel_pressure: u16,
	pub fuel_temp: u8,
	pub knock_evs_cnt: u16,
}

impl Me1_7 {
	pub fn decode(p: &[u8]) -> Result<Self, DecodeError> {
		need(Pid::Me1_7, p, 7)?;
		Ok(Self {
			knock_peak_reading: u16le(p, 0),
			knock_ign_adv_mod: (p[2] as i8) as f32 * 0.1,
			fuel_pressure: p[3] as u16 * 5,
			fuel_temp: p[4],
			knock_evs_cnt: u16le(p, 5),
		})
	}
}

/// ME1_8 (0x307): exhaust gas temps (°C), general purpose temps.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Me1_8 {
	pub egt1: f32,
	pub egt2: f32,
	pub gpt1: i16,
	pub gpt2: i16,
}

impl Me1_8 {
	pub fn decode(p: &[u8]) -> Result<Self, DecodeError> {
		need(Pid::Me1_8, p, 8)?;
		Ok(Self {
			egt1: u16le(p, 0) as f32 * 0.1,
			egt2: u16le(p, 2) as f32 * 0.1,
			gpt1: i16le(p, 4),
			gpt2: i16le(p, 6),
		})
	}
}

/// Ports of `ME1_4NET.Tests/Frames/*`, case for case.
#[cfg(test)]
mod tests {
	use super::*;

	/// xUnit's `Assert.Equal(expected, actual, precision)`.
	fn approx(expected: f32, actual: f32, decimals: i32) {
		let tol = 0.5 * 10f32.powi(-decimals);
		assert!((expected - actual).abs() <= tol, "expected {expected}, got {actual} (±{tol})");
	}

	#[test]
	fn me1_1_valid() {
		let f = Me1_1::decode(&[0xB8, 0x0B, 0x2E, 0xFB, 0xC4, 0x09, 0x37, 0x02]).unwrap();
		assert_eq!(f.rpm, 3000);
		approx(-123.4, f.throttle_position, 1);
		approx(25.00, f.map, 2);
		approx(56.7, f.iat, 1);
	}

	#[test]
	fn me1_1_too_short() {
		assert_eq!(Me1_1::decode(&[0; 7]), Err(DecodeError { pid: Pid::Me1_1, needed: 8, got: 7 }));
	}

	#[test]
	fn me1_1_negative_iat_and_throttle() {
		let f = Me1_1::decode(&[0x00, 0x00, 0x9C, 0xFF, 0x00, 0x00, 0x38, 0xFF]).unwrap();
		assert_eq!(f.rpm, 0);
		approx(-10.0, f.throttle_position, 1);
		approx(0.0, f.map, 2);
		approx(-20.0, f.iat, 1);
	}

	#[test]
	fn me1_2_valid() {
		let f = Me1_2::decode(&[0xA0, 0x0F, 0x78, 0x82, 0xC4, 0x09, 0x0E, 0x55]).unwrap();
		assert_eq!(f.rpm_hard_limit, 4000);
		assert_eq!(f.afr_curr1, 13.5);
		assert_eq!(f.afr_curr2, 14.0);
		assert_eq!(f.lambda_trim, 2500);
		assert_eq!(f.afr_target, 14.0);
		assert_eq!(f.fuel_eth_perc, 85.0);
	}

	#[test]
	fn me1_2_too_short() {
		assert!(Me1_2::decode(&[0; 7]).is_err());
	}

	#[test]
	fn me1_2_edge_values() {
		let f = Me1_2::decode(&[0xFF, 0xFF, 0xFF, 0x00, 0x00, 0x00, 0xFF, 0x00]).unwrap();
		assert_eq!(f.rpm_hard_limit, u16::MAX as i32);
		assert_eq!(f.afr_curr1, 20.25);
		assert_eq!(f.afr_curr2, 7.5);
		assert_eq!(f.lambda_trim, 0);
		assert_eq!(f.afr_target, 255.0);
		assert_eq!(f.fuel_eth_perc, 0.0);
	}

	#[test]
	fn me1_3_valid() {
		let f = Me1_3::decode(&[0xE8, 0x03, 0x30, 0xF8, 0xFF, 0x7F, 0x00, 0x80]).unwrap();
		assert_eq!((f.ign_adv_angle, f.ign_dwell, f.pri_inj_angle, f.pri_inj_pw), (1000, -2000, 32767, -32768));
	}

	#[test]
	fn me1_3_too_short() {
		assert!(Me1_3::decode(&[0; 7]).is_err());
	}

	#[test]
	fn me1_3_all_zeros() {
		assert_eq!(Me1_3::decode(&[0; 8]).unwrap(), Me1_3::default());
	}

	#[test]
	fn me1_3_max_min() {
		let f = Me1_3::decode(&[0xFF, 0x7F, 0x00, 0x80, 0xFF, 0xFF, 0x01, 0x00]).unwrap();
		assert_eq!((f.ign_adv_angle, f.ign_dwell, f.pri_inj_angle, f.pri_inj_pw), (i16::MAX, i16::MIN, -1, 1));
	}

	#[test]
	fn me1_4_valid() {
		let f = Me1_4::decode(&[100, 60, 0xAC, 0x0D, 0x32, 0x00, 140]).unwrap();
		assert_eq!(f.pri_inj_duty, 50.0);
		assert_eq!(f.sec_inj_duty, 30.0);
		approx(350.0, f.sec_inj_angle, 1);
		approx(5.0, f.sec_inj_pw, 1);
		assert_eq!(f.boost_ctrl_duty, 70.0);
	}

	#[test]
	fn me1_4_too_short() {
		assert!(Me1_4::decode(&[0; 6]).is_err());
	}

	#[test]
	fn me1_4_all_zeros() {
		assert_eq!(Me1_4::decode(&[0; 7]).unwrap(), Me1_4::default());
	}

	#[test]
	fn me1_4_max_duty() {
		let f = Me1_4::decode(&[0xFF, 0xFF, 0, 0, 0, 0, 0xFF]).unwrap();
		assert_eq!((f.pri_inj_duty, f.sec_inj_duty, f.boost_ctrl_duty), (127.5, 127.5, 127.5));
	}

	#[test]
	fn me1_4_negative_angle() {
		let f = Me1_4::decode(&[0, 0, 0x9C, 0xFF, 0x9C, 0xFF, 0]).unwrap();
		approx(-10.0, f.sec_inj_angle, 1);
		approx(-10.0, f.sec_inj_pw, 1);
	}

	#[test]
	fn me1_5_valid() {
		let f = Me1_5::decode(&[0x84, 0x03, 0xA0, 0x0F, 0x52, 0x03, 0x8A, 0x00]).unwrap();
		approx(90.0, f.oil_temp, 1);
		approx(400.0, f.oil_pressure, 1);
		approx(85.0, f.clt, 1);
		approx(13.8, f.vbat, 1);
	}

	#[test]
	fn me1_5_too_short() {
		assert!(Me1_5::decode(&[0; 7]).is_err());
	}

	#[test]
	fn me1_5_all_zeros() {
		assert_eq!(Me1_5::decode(&[0; 8]).unwrap(), Me1_5::default());
	}

	#[test]
	fn me1_5_negative() {
		let f = Me1_5::decode(&[0x38, 0xFF, 0, 0, 0x38, 0xFF, 0, 0]).unwrap();
		approx(-20.0, f.oil_temp, 1);
		approx(-20.0, f.clt, 1);
	}

	#[test]
	fn me1_6_valid() {
		let f = Me1_6::decode(&[0x12, 0x56, 0x34, 0x9A, 0x78, 0xDE, 0xBC]).unwrap();
		assert_eq!((f.gear_pos, f.map_target, f.vehicle_speed, f.eps_ev_msk), (0x12, 0x3456, 0x789A, 0xBCDE));
	}

	#[test]
	fn me1_6_too_short() {
		assert!(Me1_6::decode(&[1, 2, 3, 4, 5, 6]).is_err());
	}

	#[test]
	fn me1_6_min_and_max() {
		assert_eq!(Me1_6::decode(&[0; 7]).unwrap(), Me1_6::default());
		let f = Me1_6::decode(&[0xFF; 7]).unwrap();
		assert_eq!((f.gear_pos, f.map_target, f.vehicle_speed, f.eps_ev_msk), (0xFF, 0xFFFF, 0xFFFF, 0xFFFF));
	}

	#[test]
	fn me1_7_valid() {
		let f = Me1_7::decode(&[0x34, 0x12, 0xEC, 80, 45, 0x05, 0x00]).unwrap();
		assert_eq!(f.knock_peak_reading, 0x1234);
		approx(-2.0, f.knock_ign_adv_mod, 1);
		assert_eq!((f.fuel_pressure, f.fuel_temp, f.knock_evs_cnt), (400, 45, 5));
	}

	#[test]
	fn me1_7_too_short() {
		assert!(Me1_7::decode(&[1, 2, 3, 4, 5, 6]).is_err());
	}

	#[test]
	fn me1_7_all_zeros() {
		assert_eq!(Me1_7::decode(&[0; 7]).unwrap(), Me1_7::default());
	}

	#[test]
	fn me1_7_max() {
		let f = Me1_7::decode(&[0xFF, 0xFF, 0x7F, 0xFF, 0xFF, 0xFF, 0xFF]).unwrap();
		assert_eq!(f.knock_peak_reading, u16::MAX);
		approx(12.7, f.knock_ign_adv_mod, 1);
		assert_eq!((f.fuel_pressure, f.fuel_temp, f.knock_evs_cnt), (1275, 255, u16::MAX));
	}

	#[test]
	fn me1_8_valid() {
		let f = Me1_8::decode(&[0x58, 0x1B, 0x64, 0x19, 0x64, 0x00, 0xCE, 0xFF]).unwrap();
		approx(700.0, f.egt1, 1);
		approx(650.0, f.egt2, 1);
		assert_eq!((f.gpt1, f.gpt2), (100, -50));
	}

	#[test]
	fn me1_8_too_short() {
		assert!(Me1_8::decode(&[1, 2, 3, 4, 5, 6, 7]).is_err());
	}

	#[test]
	fn me1_8_all_zeros() {
		assert_eq!(Me1_8::decode(&[0; 8]).unwrap(), Me1_8::default());
	}

	#[test]
	fn me1_8_max_egt() {
		let f = Me1_8::decode(&[0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x7F, 0x00, 0x80]).unwrap();
		approx(6553.5, f.egt1, 1);
		approx(6553.5, f.egt2, 1);
		assert_eq!((f.gpt1, f.gpt2), (i16::MAX, i16::MIN));
	}
}
