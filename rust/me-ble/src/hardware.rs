//! RG35XX Bluetooth bring-up (port of `RG35XX.Libraries/BluetoothHardwareInit`).
//!
//! The stock firmware doesn't start the Realtek UART controller or BlueZ, so
//! this loads the driver, attaches the controller, (re)starts `bluetoothd`
//! and clears connections left over from a previous run. Every step shells
//! out exactly like the C# version and only logs on failure.

use std::path::Path;
use std::process::{Command, Stdio};
use std::thread::sleep;
use std::time::Duration;

fn log(msg: &str) {
	eprintln!("[BLE] {msg}");
}

/// Runs a command, returning (exit code, stdout); (-1, "") if it can't start.
fn run(cmd: &str, args: &[&str]) -> (i32, String) {
	match Command::new(cmd).args(args).stdin(Stdio::null()).stderr(Stdio::piped()).output() {
		Ok(out) => (out.status.code().unwrap_or(-1), String::from_utf8_lossy(&out.stdout).into_owned()),
		Err(e) => {
			log(&format!("'{cmd} {}' failed: {e}", args.join(" ")));
			(-1, String::new())
		}
	}
}

fn is_running(name: &str) -> bool {
	run("pgrep", &["-x", name]).0 == 0
}

fn is_module_loaded(name: &str) -> bool {
	std::fs::read_to_string("/proc/modules")
		.map(|m| m.lines().any(|l| l.starts_with(&format!("{name} "))))
		.unwrap_or(false)
}

fn start_background(cmd: &str, args: &[&str]) {
	if let Err(e) = Command::new(cmd).args(args).stdin(Stdio::null()).spawn() {
		log(&format!("failed to start '{cmd}': {e}"));
	}
}

/// Starts a daemon detached from our session with its output discarded.
fn start_background_silent(cmd: &str, args: &str) {
	let shell = format!("setsid {cmd} {args} </dev/null >/dev/null 2>&1 &");
	if let Err(e) = Command::new("/bin/sh").args(["-c", &shell]).spawn() {
		log(&format!("failed to start '{cmd}' silently: {e}"));
	}
}

/// Full bring-up sequence (`BluetoothHardwareInit.InitializeAsync`). Blocking;
/// takes a few seconds on a cold boot.
pub fn initialize() {
	ensure_dbus();
	ensure_rtl_btlpm();
	ensure_rtk_hciattach();
	wait_for_hci0();
	drop_stale_le_links();
	force_hci0_up();
	ensure_bluetoothd();
	log("powering adapter on through bluetoothctl");
	run("bluetoothctl", &["power", "on"]);
	sleep(Duration::from_secs(1));
	disconnect_stale_bluez_devices();

	let status = run("hciconfig", &["hci0"]).1.lines().next().unwrap_or("").to_string();
	log(&format!("hci0: {status}"));
	log(&format!("bluetoothd: {}", if is_running("bluetoothd") { "running" } else { "NOT RUNNING" }));
}

fn ensure_dbus() {
	if is_running("dbus-daemon") {
		return;
	}
	let _ = std::fs::create_dir_all("/run/dbus");
	run("dbus-daemon", &["--system", "--fork"]);
	for i in 1..=5 {
		if Path::new("/run/dbus/system_bus_socket").exists() {
			return;
		}
		log(&format!("waiting for D-Bus system socket ({i}/5)"));
		sleep(Duration::from_secs(1));
	}
}

fn ensure_rtl_btlpm() {
	if is_module_loaded("rtl_btlpm") {
		return;
	}
	let release = run("uname", &["-r"]).1.trim().to_string();
	let ko = format!("/lib/modules/{release}/kernel/drivers/bluetooth/rtl_btlpm.ko");
	if Path::new(&ko).exists() {
		run("modprobe", &[&ko]);
	} else {
		run("modprobe", &["rtl_btlpm"]);
	}
}

fn ensure_rtk_hciattach() {
	if run("pgrep", &["-f", "rtk_hciattach"]).0 == 0 {
		log("rtk_hciattach already running");
		return;
	}
	log("starting rtk_hciattach");
	start_background("rtk_hciattach", &["-n", "-s", "115200", "/dev/ttyS1", "rtk_h5"]);
}

fn wait_for_hci0() {
	for i in 1..=15 {
		if run("hciconfig", &["hci0"]).0 == 0 {
			return;
		}
		log(&format!("waiting for hci0 ({i}/15)"));
		sleep(Duration::from_secs(1));
	}
}

/// `hcitool con` lines look like `< LE 11:22:33:44:55:66 handle 64 state 1 lm SLAVE`.
fn parse_le_links(output: &str) -> Vec<(String, String)> {
	output
		.lines()
		.filter(|l| l.contains("LE") && l.contains("handle"))
		.filter_map(|l| {
			let parts: Vec<&str> = l.split_whitespace().collect();
			let handle = parts.get(parts.iter().position(|p| *p == "handle")? + 1)?;
			let mac = parts.iter().find(|p| p.matches(':').count() == 5).copied().unwrap_or("?");
			Some((mac.to_string(), handle.to_string()))
		})
		.collect()
}

fn drop_stale_le_links() {
	let (code, output) = run("hcitool", &["con"]);
	if code != 0 {
		return;
	}
	for (mac, handle) in parse_le_links(&output) {
		log(&format!("dropping stale LE link {mac} (handle {handle})"));
		// 0x13 = Remote User Terminated
		run("hcitool", &["ledc", &handle, "0x13"]);
	}
}

fn force_hci0_up() {
	for i in 1..=5 {
		run("hciconfig", &["hci0", "up"]);
		if run("hciconfig", &["hci0"]).1.contains("UP") {
			return;
		}
		log(&format!("waiting for hci0 UP ({i}/5)"));
		sleep(Duration::from_secs(1));
	}
}

/// Always restarts bluetoothd cleanly, silenced (a previous launch may have
/// used `-d` and inherited our console).
fn ensure_bluetoothd() {
	if is_running("bluetoothd") {
		log("killing existing bluetoothd for clean relaunch");
		run("pkill", &["-f", "bluetoothd"]);
		for _ in 0..10 {
			if !is_running("bluetoothd") {
				break;
			}
			sleep(Duration::from_millis(200));
		}
	}
	log("starting bluetoothd (silent)");
	start_background_silent("/usr/libexec/bluetooth/bluetoothd", "-n");
	for i in 1..=5 {
		if is_running("bluetoothd") {
			return;
		}
		log(&format!("waiting for bluetoothd ({i}/5)"));
		sleep(Duration::from_secs(1));
	}
}

/// `bluetoothctl devices Connected` lines look like `Device AA:BB:CC:DD:EE:FF Name`.
fn parse_device_macs(output: &str) -> Vec<String> {
	output
		.lines()
		.filter_map(|l| {
			let mut parts = l.split_whitespace();
			(parts.next()? == "Device").then_some(())?;
			let mac = parts.next()?;
			(mac.matches(':').count() == 5).then(|| mac.to_string())
		})
		.collect()
}

fn disconnect_stale_bluez_devices() {
	let (code, output) = run("bluetoothctl", &["devices", "Connected"]);
	if code != 0 {
		return;
	}
	for mac in parse_device_macs(&output) {
		log(&format!("dropping stale bluez session for {mac}"));
		run("bluetoothctl", &["disconnect", &mac]);
	}
}

/// Whether BlueZ has the device in its cache.
pub fn is_device_cached(mac: &str) -> bool {
	let (code, output) = run("bluetoothctl", &["info", mac.trim()]);
	code == 0 && output.contains("Device ")
}

/// Removes a cached device so a scan re-announces it. BlueZ only updates a
/// known device's properties instead of announcing it, which a scan can miss.
pub fn evict_cached_device(mac: &str) -> bool {
	if mac.trim().is_empty() || !is_device_cached(mac) {
		return false;
	}
	log(&format!("evicting cached bluez entry for {} so the scan sees it", mac.trim()));
	run("bluetoothctl", &["remove", mac.trim()]);
	true
}

/// Disconnects a device BlueZ still believes is connected from a prior run.
pub fn disconnect_cached_device(mac: &str) -> bool {
	if mac.trim().is_empty() {
		return false;
	}
	let (code, output) = run("bluetoothctl", &["info", mac.trim()]);
	if code != 0 || !output.contains("Connected: yes") {
		return false;
	}
	log(&format!("bluez still has {} connected; forcing disconnect", mac.trim()));
	run("bluetoothctl", &["disconnect", mac.trim()]);
	true
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn parses_hcitool_connections() {
		let out = "Connections:\n\t< LE 11:22:33:44:55:66 handle 64 state 1 lm SLAVE\n\t> ACL 00:00:00:00:00:01 handle 3 state 1";
		assert_eq!(parse_le_links(out), vec![("11:22:33:44:55:66".to_string(), "64".to_string())]);
	}

	#[test]
	fn parses_bluetoothctl_devices() {
		let out = "Device 20:E7:C8:B9:20:2A ME221\nDevice bogus\nController 00:11:22:33:44:55 rg35xx";
		assert_eq!(parse_device_macs(out), vec!["20:E7:C8:B9:20:2A".to_string()]);
	}
}
