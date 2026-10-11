//! Port of `SimulatedMeDevice`: oscillating ECU values for desktop testing,
//! keyed by the `DataSourceMapper` names (MEData properties + legacy aliases).

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

	pub fn read(&self, name: &str) -> Option<f32> {
		let sine = self.t.sin();
		let fast = (self.t * 3.0).sin();
		let round2 = |v: f64| (v * 100.0).round() / 100.0;
		let v: f64 = match name.to_ascii_lowercase().as_str() {
			"rpm" => (3500.0 + sine * 3000.0).trunc(),
			"throttleposition" => 50.0 + sine * 50.0,
			"map" => 50.0 + sine * 50.0,
			"iat" => 35.0 + fast * 10.0,
			"afrcurr1" => round2(14.5 + sine * 2.5),
			"afrcurr2" => round2(14.5 + sine * 2.3),
			"afrtarget" => round2(14.7 + fast * 0.5),
			"fuelethperc" => 0.0,
			"ignadvangle" => (20.0 + sine * 15.0).trunc(),
			"igndwell" => (30.0 + fast * 5.0).trunc(),
			"priinjpw" => (50.0 + sine * 30.0).trunc(),
			"priinjduty" | "injectorduty" => 40.0 + sine * 35.0,
			"secinjduty" => 20.0 + sine * 15.0,
			"secinjangle" => 350.0 + fast * 10.0,
			"secinjpw" => 5.0 + sine * 3.0,
			"boostctrlduty" => 50.0 + sine * 40.0,
			"oiltemp" => 90.0 + fast * 15.0,
			"oilpressure" => 400.0 + sine * 200.0,
			"clt" => 85.0 + fast * 10.0,
			"vbat" | "batteryvoltage" => 13.8 + fast * 0.5,
			"gearpos" => ((3.0 + sine * 2.5).trunc()).clamp(1.0, 6.0),
			"vehiclespeed" => (60.0 + sine * 60.0).trunc(),
			"knockpeakreading" | "knocklevel" => (100.0 + sine * 50.0).trunc(),
			"knockignadvmod" => sine * -2.0,
			"fuelpressure" => (400.0 + sine * 100.0).trunc(),
			"fueltemp" => (40.0 + fast * 10.0).trunc(),
			"knockevscnt" => (fast * 5.0).trunc().max(0.0),
			"egt1" => 700.0 + sine * 200.0,
			"egt2" => 680.0 + sine * 180.0,
			"gpt1" => (sine * 20.0).trunc(),
			"gpt2" => (sine * 15.0).trunc(),
			"rpmhardlimit" | "lambdatrim" | "priinjangle" | "maptarget" | "epsevmsk" => 0.0,
			_ => return None,
		};
		Some(v as f32)
	}
}
