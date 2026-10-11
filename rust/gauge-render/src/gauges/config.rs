//! `gauges.json` model (`AppConfig`, `ScreenConfig`, `GaugeConfig`).

use serde::{Deserialize, Deserializer, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub enum GaugeType {
	#[default]
	Bar,
	Circular,
	Histogram,
	Needle,
	Digital,
	Sweep,
	MinMax,
	Grid,
	Custom,
}

impl GaugeType {
	pub const ALL: [GaugeType; 9] = [
		Self::Bar,
		Self::Circular,
		Self::Histogram,
		Self::Needle,
		Self::Digital,
		Self::Sweep,
		Self::MinMax,
		Self::Grid,
		Self::Custom,
	];

	pub fn parse(s: &str) -> Option<Self> {
		Self::ALL.into_iter().find(|t| format!("{t:?}").eq_ignore_ascii_case(s))
	}
}

/// Mirrors `TolerantGaugeTypeConverter`: names (any case) or numbers, with
/// unknown values falling back to `Bar` instead of failing the whole file.
impl<'de> Deserialize<'de> for GaugeType {
	fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
		let v = serde_json::Value::deserialize(d)?;
		Ok(match v {
			serde_json::Value::String(s) => Self::parse(&s).unwrap_or_default(),
			serde_json::Value::Number(n) => {
				n.as_u64().and_then(|n| Self::ALL.get(n as usize).copied()).unwrap_or_default()
			}
			_ => Self::default(),
		})
	}
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase", default)]
pub struct GridCellConfig {
	pub data_source: String,
	pub title: String,
	pub unit: String,
	pub decimals: i32,
}

impl Default for GridCellConfig {
	fn default() -> Self {
		Self { data_source: "Rpm".into(), title: String::new(), unit: String::new(), decimals: 0 }
	}
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase", default)]
pub struct GaugeConfig {
	#[serde(rename = "Type")]
	pub gauge_type: GaugeType,
	pub data_source: String,
	pub color_hex: String,
	pub title: String,
	pub unit: String,
	pub min_value: f32,
	pub max_value: f32,
	pub initial_value: f32,
	pub segment_count: i32,
	pub smoothing: bool,
	pub decimals: i32,
	pub max_data_points: i32,
	pub interval_ms: i32,
	pub cells: Vec<GridCellConfig>,
}

impl Default for GaugeConfig {
	fn default() -> Self {
		Self {
			gauge_type: GaugeType::Bar,
			data_source: "AfrCurr1".into(),
			color_hex: "#00FFFF".into(),
			title: String::new(),
			unit: String::new(),
			min_value: 0.0,
			max_value: 100.0,
			initial_value: 0.0,
			segment_count: 32,
			smoothing: true,
			decimals: 0,
			max_data_points: 50,
			interval_ms: 1000,
			cells: Vec::new(),
		}
	}
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase", default)]
pub struct ScreenConfig {
	pub gauge: GaugeConfig,
	pub custom_definition_file: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase", default)]
pub struct AppConfig {
	pub demo_mode: bool,
	pub simulate_battery_percent: Option<i32>,
	pub device_mac_address: Option<String>,
	pub screens: Vec<ScreenConfig>,
}

impl Default for AppConfig {
	fn default() -> Self {
		Self {
			demo_mode: false,
			simulate_battery_percent: None,
			device_mac_address: Some("20:E7:C8:B9:20:2A".into()),
			screens: Vec::new(),
		}
	}
}

impl AppConfig {
	pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
		serde_json::from_str(json.trim_start_matches('\u{feff}'))
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn tolerant_gauge_type() {
		let cfg = AppConfig::from_json(
			r#"{"Screens":[
				{"Gauge":{"Type":"needle","MaxValue":8000}},
				{"Gauge":{"Type":3}},
				{"Gauge":{"Type":"Legacy"}},
				{"Gauge":{"Type":"Custom"},"CustomDefinitionFile":"custom-gauge.json"}
			]}"#,
		)
		.unwrap();
		let types: Vec<_> = cfg.screens.iter().map(|s| s.gauge.gauge_type).collect();
		assert_eq!(types, [GaugeType::Needle, GaugeType::Needle, GaugeType::Bar, GaugeType::Custom]);
		assert_eq!(cfg.screens[0].gauge.max_value, 8000.0);
		assert_eq!(cfg.screens[0].gauge.segment_count, 32);
		assert_eq!(cfg.screens[3].custom_definition_file.as_deref(), Some("custom-gauge.json"));
	}
}
