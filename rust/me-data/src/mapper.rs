//! Data-source names used by gauge configs (`DataSourceMapper`): every
//! numeric `MEData` property by its C# name, plus legacy aliases. Lookups are
//! case-insensitive, like the C# `OrdinalIgnoreCase` dictionary.

use crate::MeData;

/// Every channel name, in `MEData` declaration order.
pub const DATA_SOURCE_NAMES: [&str; 36] = [
	"Rpm",
	"ThrottlePosition",
	"Map",
	"Iat",
	"RpmHardLimit",
	"AfrCurr1",
	"AfrCurr2",
	"LambdaTrim",
	"AfrTarget",
	"FuelEthPerc",
	"IgnAdvAngle",
	"IgnDwell",
	"PriInjAngle",
	"PriInjPw",
	"PriInjDuty",
	"SecInjDuty",
	"SecInjAngle",
	"SecInjPw",
	"BoostCtrlDuty",
	"OilTemp",
	"OilPressure",
	"Clt",
	"Vbat",
	"GearPos",
	"MapTarget",
	"VehicleSpeed",
	"EpsEvMsk",
	"KnockPeakReading",
	"KnockIgnAdvMod",
	"FuelPressure",
	"FuelTemp",
	"KnockEvsCnt",
	"Egt1",
	"Egt2",
	"Gpt1",
	"Gpt2",
];

/// Backwards-compatible names kept for existing gauge configs: (alias, channel).
pub const LEGACY_ALIASES: [(&str, &str); 3] =
	[("BatteryVoltage", "Vbat"), ("KnockLevel", "KnockPeakReading"), ("InjectorDuty", "PriInjDuty")];

impl MeData {
	/// Reads a channel by config name (`DataSourceMapper.ReadValue`). Unknown
	/// names return `None` instead of throwing, so a gauge bound to something
	/// else (e.g. a calculated channel) cannot take the update loop down.
	pub fn read(&self, name: &str) -> Option<f32> {
		let name = LEGACY_ALIASES.iter().find(|(alias, _)| alias.eq_ignore_ascii_case(name)).map_or(name, |(_, c)| c);
		let v = match name.to_ascii_lowercase().as_str() {
			"rpm" => self.rpm as f32,
			"throttleposition" => self.throttle_position,
			"map" => self.map,
			"iat" => self.iat,
			"rpmhardlimit" => self.rpm_hard_limit as f32,
			"afrcurr1" => self.afr_curr1,
			"afrcurr2" => self.afr_curr2,
			"lambdatrim" => self.lambda_trim as f32,
			"afrtarget" => self.afr_target,
			"fuelethperc" => self.fuel_eth_perc,
			"ignadvangle" => self.ign_adv_angle as f32,
			"igndwell" => self.ign_dwell as f32,
			"priinjangle" => self.pri_inj_angle as f32,
			"priinjpw" => self.pri_inj_pw as f32,
			"priinjduty" => self.pri_inj_duty,
			"secinjduty" => self.sec_inj_duty,
			"secinjangle" => self.sec_inj_angle,
			"secinjpw" => self.sec_inj_pw,
			"boostctrlduty" => self.boost_ctrl_duty,
			"oiltemp" => self.oil_temp,
			"oilpressure" => self.oil_pressure,
			"clt" => self.clt,
			"vbat" => self.vbat,
			"gearpos" => self.gear_pos as f32,
			"maptarget" => self.map_target as f32,
			"vehiclespeed" => self.vehicle_speed as f32,
			"epsevmsk" => self.eps_ev_msk as f32,
			"knockpeakreading" => self.knock_peak_reading as f32,
			"knockignadvmod" => self.knock_ign_adv_mod,
			"fuelpressure" => self.fuel_pressure as f32,
			"fueltemp" => self.fuel_temp as f32,
			"knockevscnt" => self.knock_evs_cnt as f32,
			"egt1" => self.egt1,
			"egt2" => self.egt2,
			"gpt1" => self.gpt1 as f32,
			"gpt2" => self.gpt2 as f32,
			_ => return None,
		};
		Some(v)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn every_name_and_alias_resolves() {
		let d = MeData::default();
		for name in DATA_SOURCE_NAMES {
			assert_eq!(d.read(name), Some(0.0), "{name}");
			assert!(d.read(&name.to_uppercase()).is_some(), "{name} case-insensitive");
		}
		for (alias, _) in LEGACY_ALIASES {
			assert!(d.read(alias).is_some(), "{alias}");
		}
		assert_eq!(d.read("RpmK"), None);
	}

	#[test]
	fn reads_values_and_aliases() {
		let d = MeData {
			rpm: 6500,
			vbat: 13.8,
			knock_peak_reading: 120,
			pri_inj_duty: 42.5,
			gpt2: -7,
			..Default::default()
		};
		assert_eq!(d.read("rpm"), Some(6500.0));
		assert_eq!(d.read("BatteryVoltage"), Some(13.8));
		assert_eq!(d.read("KnockLevel"), Some(120.0));
		assert_eq!(d.read("InjectorDuty"), Some(42.5));
		assert_eq!(d.read("Gpt2"), Some(-7.0));
	}
}
