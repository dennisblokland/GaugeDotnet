//! Where live values come from: the simulator (desktop / `DemoMode`) or the
//! ECU over Bluetooth.

use std::time::Instant;

use me_data::MeData;

use crate::sim::Simulator;

pub enum Feed {
	Sim {
		sim: Simulator,
		last: Instant,
	},
	#[cfg(feature = "ble")]
	Ble(me_ble::BleLink),
}

impl Feed {
	pub fn simulated() -> Self {
		eprintln!("[SimulatedMeDevice] Connected (simulated)");
		Feed::Sim { sim: Simulator::new(0.0), last: Instant::now() }
	}

	/// Latest values for this frame.
	pub fn tick(&mut self) -> MeData {
		match self {
			Feed::Sim { sim, last } => {
				let now = Instant::now();
				sim.advance(now.duration_since(*last).as_secs_f64());
				*last = now;
				sim.data()
			}
			#[cfg(feature = "ble")]
			Feed::Ble(link) => link.snapshot(),
		}
	}

	/// Start-up progress for the splash screen: `(finished, error message)`.
	pub fn startup_state(&self) -> (bool, Option<String>) {
		match self {
			Feed::Sim { .. } => (true, None),
			#[cfg(feature = "ble")]
			Feed::Ble(link) => match link.status() {
				me_ble::LinkStatus::Connected => (true, None),
				me_ble::LinkStatus::Failed(msg) => (true, Some(msg)),
				_ => (false, None),
			},
		}
	}

	/// A message to show over the gauges while there is no live data.
	pub fn status(&self) -> Option<String> {
		match self {
			Feed::Sim { .. } => None,
			#[cfg(feature = "ble")]
			Feed::Ble(link) => {
				use me_ble::LinkStatus::*;
				match link.status() {
					Connected => None,
					Starting | InitializingHardware => Some("Starting Bluetooth...".into()),
					Scanning => Some("Searching for ME device...".into()),
					Connecting => Some("Connecting to ME device...".into()),
					Reconnecting => Some("Connection lost, reconnecting...".into()),
					Disconnected => Some("ME device disconnected".into()),
					Failed(msg) => Some(msg),
				}
			}
		}
	}
}
