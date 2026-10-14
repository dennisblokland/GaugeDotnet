//! Port of `SimulatedMeDevice`: oscillating ECU values for desktop testing,
//! produced as a real [`MeData`] so names and integer truncation match the
//! device path exactly.

use me_data::MeData;

pub struct Simulator {
	t: f64,
}

impl Simulator {
	pub fn new(t: f64) -> Self {
		Self { t }
	}

	/// Advances simulated time; the C# device steps 0.05 every 50 ms (1 unit/s).
	pub fn advance(&mut self, seconds: f64) {
		self.t += seconds;
	}

	/// Same formulas and casts as `SimulatedMeDevice.SimulateAsync`.
	pub fn data(&self) -> MeData {
		let sine = self.t.sin();
		let fast = (self.t * 3.0).sin();
		let round2 = |v: f64| ((v * 100.0).round() / 100.0) as f32;
		MeData {
			rpm: (3500.0 + sine * 3000.0) as u16,
			throttle_position: (50.0 + sine * 50.0) as f32,
			map: (50.0 + sine * 50.0) as f32,
			iat: (35.0 + fast * 10.0) as f32,
			afr_curr1: round2(14.5 + sine * 2.5),
			afr_curr2: round2(14.5 + sine * 2.3),
			afr_target: round2(14.7 + fast * 0.5),
			fuel_eth_perc: 0.0,
			ign_adv_angle: (20.0 + sine * 15.0) as i16,
			ign_dwell: (30.0 + fast * 5.0) as i16,
			pri_inj_pw: (50.0 + sine * 30.0) as i16,
			pri_inj_duty: (40.0 + sine * 35.0) as f32,
			sec_inj_duty: (20.0 + sine * 15.0) as f32,
			sec_inj_angle: (350.0 + fast * 10.0) as f32,
			sec_inj_pw: (5.0 + sine * 3.0) as f32,
			boost_ctrl_duty: (50.0 + sine * 40.0) as f32,
			oil_temp: (90.0 + fast * 15.0) as f32,
			oil_pressure: (400.0 + sine * 200.0) as f32,
			clt: (85.0 + fast * 10.0) as f32,
			vbat: (13.8 + fast * 0.5) as f32,
			gear_pos: ((3.0 + sine * 2.5) as i32).clamp(1, 6) as u8,
			vehicle_speed: (60.0 + sine * 60.0) as u16,
			knock_peak_reading: (100.0 + sine * 50.0) as u16,
			knock_ign_adv_mod: (sine * -2.0) as f32,
			fuel_pressure: (400.0 + sine * 100.0) as u16,
			fuel_temp: (40.0 + fast * 10.0) as u8,
			// Like C#'s `(ushort)` cast on net10, this saturates: negatives give 0.
			knock_evs_cnt: (fast * 5.0) as u16,
			egt1: (700.0 + sine * 200.0) as f32,
			egt2: (680.0 + sine * 180.0) as f32,
			gpt1: (sine * 20.0) as i16,
			gpt2: (sine * 15.0) as i16,
			..MeData::default()
		}
	}

	pub fn read(&self, name: &str) -> Option<f32> {
		self.data().read(name)
	}
}
