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
	TurboXs,
}

impl GaugeType {
	pub const ALL: [GaugeType; 10] = [
		Self::Bar,
		Self::Circular,
		Self::Histogram,
		Self::Needle,
		Self::Digital,
		Self::Sweep,
		Self::MinMax,
		Self::Grid,
		Self::Custom,
		Self::TurboXs,
	];

	/// The C# enum name (also its JSON spelling).
	pub fn name(self) -> &'static str {
		match self {
			Self::Bar => "Bar",
			Self::Circular => "Circular",
			Self::Histogram => "Histogram",
			Self::Needle => "Needle",
			Self::Digital => "Digital",
			Self::Sweep => "Sweep",
			Self::MinMax => "MinMax",
			Self::Grid => "Grid",
			Self::Custom => "Custom",
			Self::TurboXs => "TurboXs",
		}
	}

	pub fn parse(s: &str) -> Option<Self> {
		Self::ALL.into_iter().find(|t| t.name().eq_ignore_ascii_case(s))
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
	/// TurboXs: `Peak`, `Min`, `Text` or `None` (the status block under the title).
	pub status_mode: String,
	/// TurboXs: first status line; empty picks `Peak:`, `Min:` or `Datalog:`.
	pub status_label: String,
	/// TurboXs: second status line in `Text` mode.
	pub status_text: String,
	/// TurboXs: old-LCD look (low resolution, pixel grid, backlight tint, ghosting).
	pub lcd: bool,
	/// TurboXs: share of the previous frame that lingers in Lcd mode (0 = none, up to 0.9).
	pub lcd_ghosting: f32,
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
			status_mode: "Peak".into(),
			status_label: String::new(),
			status_text: "Stop".into(),
			lcd: false,
			lcd_ghosting: 0.2,
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
	/// Most screens a config may hold (`AppConfig.MaxScreens`).
	pub const MAX_SCREENS: usize = 10;

	pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
		serde_json::from_str(json.trim_start_matches('\u{feff}'))
	}

	pub fn to_json(&self) -> String {
		serde_json::to_string_pretty(self).expect("config serialises")
	}

	/// The config written when none exists (`ConfigService.CreateDefault`).
	pub fn default_config() -> Self {
		Self {
			screens: vec![ScreenConfig {
				gauge: GaugeConfig {
					gauge_type: GaugeType::Bar,
					data_source: "AfrCurr1".into(),
					color_hex: "#00FFFF".into(),
					title: "AFR".into(),
					unit: String::new(),
					min_value: 8.0,
					max_value: 18.0,
					initial_value: 14.7,
					decimals: 2,
					segment_count: 32,
					smoothing: true,
					..GaugeConfig::default()
				},
				custom_definition_file: None,
			}],
			..Self::default()
		}
	}

	/// Loads `path` like `ConfigService.Load`: a missing or empty config is
	/// replaced by the default (and written back), and extra screens beyond
	/// [`Self::MAX_SCREENS`] are dropped. A config that exists but can't be
	/// parsed (say, truncated by a power cut) is first copied to
	/// `gauges.json.bad` so the user's settings can still be recovered.
	pub fn load_or_create(path: &std::path::Path) -> Self {
		let loaded = match std::fs::read_to_string(path) {
			Ok(json) => match Self::from_json(&json) {
				Ok(c) => Some(c),
				Err(e) => {
					eprintln!("[config] {}: {e}; using defaults", path.display());
					backup_bad(path);
					None
				}
			},
			Err(_) => None,
		};
		let mut config = match loaded {
			Some(c) if !c.screens.is_empty() => c,
			_ => {
				let c = Self::default_config();
				if let Err(e) = c.save(path) {
					eprintln!("[config] could not write {}: {e}", path.display());
				}
				c
			}
		};
		config.screens.truncate(Self::MAX_SCREENS);
		config
	}

	/// Writes `path` atomically: the JSON goes to `<path>.tmp`, is fsynced,
	/// then renamed over `path`, and the directory is fsynced. A power cut
	/// on the SD card leaves either the old or the new file, never a
	/// truncated one.
	pub fn save(&self, path: &std::path::Path) -> std::io::Result<()> {
		use std::io::Write;
		let tmp = sibling(path, ".tmp");
		let result = (|| {
			let mut file = std::fs::File::create(&tmp)?;
			file.write_all(self.to_json().as_bytes())?;
			file.sync_all()?;
			drop(file);
			std::fs::rename(&tmp, path)
		})();
		if result.is_err() {
			let _ = std::fs::remove_file(&tmp);
		}
		result?;
		// Makes the rename itself durable. Not every filesystem lets a
		// directory be opened or synced, and the data is already safe.
		let dir = path.parent().filter(|d| !d.as_os_str().is_empty()).unwrap_or(std::path::Path::new("."));
		if let Ok(d) = std::fs::File::open(dir) {
			let _ = d.sync_all();
		}
		Ok(())
	}
}

/// `path` with `suffix` appended to its file name (`gauges.json.tmp`).
fn sibling(path: &std::path::Path, suffix: &str) -> std::path::PathBuf {
	let mut name = path.file_name().unwrap_or_default().to_os_string();
	name.push(suffix);
	path.with_file_name(name)
}

/// Keeps an unparsable config as `gauges.json.bad` before the defaults
/// replace it.
fn backup_bad(path: &std::path::Path) {
	let bad = sibling(path, ".bad");
	match std::fs::copy(path, &bad) {
		Ok(_) => eprintln!("[config] kept the unreadable config as {}", bad.display()),
		Err(e) => eprintln!("[config] could not back up {}: {e}", path.display()),
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn load_or_create_writes_default_and_caps_screens() {
		let dir = std::env::temp_dir().join(format!("gauge-config-{}", std::process::id()));
		std::fs::create_dir_all(&dir).unwrap();
		let path = dir.join("gauges.json");
		let _ = std::fs::remove_file(&path);

		let c = AppConfig::load_or_create(&path);
		assert_eq!(c.screens.len(), 1);
		assert_eq!(c.screens[0].gauge.data_source, "AfrCurr1");
		let written = std::fs::read_to_string(&path).unwrap();
		assert!(written.contains("\"Type\": \"Bar\""), "{written}");

		let mut many = AppConfig::default_config();
		many.screens = (0..14).map(|_| many.screens[0].clone()).collect();
		many.save(&path).unwrap();
		assert_eq!(AppConfig::load_or_create(&path).screens.len(), AppConfig::MAX_SCREENS);

		std::fs::write(&path, "{ not json").unwrap();
		assert_eq!(AppConfig::load_or_create(&path).screens.len(), 1);
		let bad = dir.join("gauges.json.bad");
		assert_eq!(std::fs::read_to_string(&bad).unwrap(), "{ not json", "the broken config is kept");
		assert!(!dir.join("gauges.json.tmp").exists(), "save leaves no temp file");
		let _ = std::fs::remove_dir_all(dir);
	}

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
