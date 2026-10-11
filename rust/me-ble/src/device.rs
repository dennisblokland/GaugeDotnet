//! BLE connection to the ECU's RaceChrono bridge (port of `BleManager` +
//! `MeDevice`), on btleplug: the same Rust library the C# app reached through
//! the `VaettirNet.Btleplug` bindings.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use btleplug::api::{
	BDAddr, Central, CentralEvent, Characteristic, Manager as _, Peripheral as _, ScanFilter, WriteType,
};
use btleplug::platform::{Adapter, Manager, Peripheral};
use futures::StreamExt;
use me_data::racechrono::{self, ALLOW_ALL_PIDS};
use me_data::MeData;
use tokio::sync::watch;
use uuid::Uuid;

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Result<T> = std::result::Result<T, Error>;

const SERVICE: Uuid = Uuid::from_u128(racechrono::SERVICE_UUID);
const CAN_BUS: Uuid = Uuid::from_u128(racechrono::CAN_BUS_CHARACTERISTIC_UUID);
const PID_FILTER: Uuid = Uuid::from_u128(racechrono::PID_CHARACTERISTIC_UUID);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConnectionState {
	Disconnected,
	Connected,
	Reconnecting,
}

/// Parses `AA:BB:CC:DD:EE:FF`, `AA-BB-…` or `AABBCCDDEEFF` (like the C# `ParseIdentifier`).
pub fn parse_mac(text: &str) -> Option<BDAddr> {
	let hex: String = text.trim().chars().filter(|c| *c != ':' && *c != '-').collect();
	if hex.len() != 12 {
		return None;
	}
	let n = u64::from_str_radix(&hex, 16).ok()?;
	BDAddr::try_from(n).ok()
}

/// First Bluetooth adapter.
pub async fn adapter() -> Result<Adapter> {
	let manager = Manager::new().await?;
	manager.adapters().await?.into_iter().next().ok_or_else(|| "no Bluetooth adapter found".into())
}

async fn advertises_service(p: &Peripheral) -> bool {
	match p.properties().await {
		Ok(Some(props)) => props.services.contains(&SERVICE),
		_ => false,
	}
}

/// Scans for ME devices advertising the RaceChrono service and returns the
/// first match: the device with `address` if given, else any (`ScanAsync` /
/// `ConnectByAddressAsync`). Runs until found; wrap it in a timeout.
pub async fn find(adapter: &Adapter, address: Option<BDAddr>) -> Result<Peripheral> {
	let mut events = adapter.events().await?;
	adapter.start_scan(ScanFilter { services: vec![SERVICE] }).await?;
	let matches = |p: &Peripheral| address.map_or(true, |a| p.address() == a);

	// Devices BlueZ already knows don't always re-announce themselves.
	for p in adapter.peripherals().await? {
		if matches(&p) && (address.is_some() || advertises_service(&p).await) {
			let _ = adapter.stop_scan().await;
			return Ok(p);
		}
	}
	while let Some(event) = events.next().await {
		if let CentralEvent::DeviceDiscovered(id) | CentralEvent::DeviceUpdated(id) = event {
			let Ok(p) = adapter.peripheral(&id).await else { continue };
			eprintln!("[BLE] discovered candidate {}", p.address());
			if matches(&p) && (address.is_some() || advertises_service(&p).await) {
				let _ = adapter.stop_scan().await;
				return Ok(p);
			}
		}
	}
	Err("scan ended without finding the ME device".into())
}

/// A device seen during [`scan_all`].
#[derive(Clone, Debug)]
pub struct Found {
	pub address: BDAddr,
	pub name: Option<String>,
	pub rssi: Option<i16>,
}

/// Lists every device advertising the RaceChrono service for `duration`.
pub async fn scan_all(adapter: &Adapter, duration: Duration) -> Result<Vec<Found>> {
	adapter.start_scan(ScanFilter { services: vec![SERVICE] }).await?;
	tokio::time::sleep(duration).await;
	let _ = adapter.stop_scan().await;
	let mut found = Vec::new();
	for p in adapter.peripherals().await? {
		if let Ok(Some(props)) = p.properties().await {
			if props.services.contains(&SERVICE) {
				found.push(Found { address: p.address(), name: props.local_name, rssi: props.rssi });
			}
		}
	}
	Ok(found)
}

/// A connected (or reconnecting) ECU.
pub struct MeDevice {
	adapter: Adapter,
	peripheral: Peripheral,
	data: Arc<Mutex<MeData>>,
	state: watch::Sender<ConnectionState>,
}

impl MeDevice {
	pub fn new(adapter: Adapter, peripheral: Peripheral, data: Arc<Mutex<MeData>>) -> Self {
		Self { adapter, peripheral, data, state: watch::channel(ConnectionState::Disconnected).0 }
	}

	pub fn address(&self) -> BDAddr {
		self.peripheral.address()
	}

	pub fn state(&self) -> watch::Receiver<ConnectionState> {
		self.state.subscribe()
	}

	/// Connects and starts streaming, retrying up to three times: BlueZ
	/// sometimes aborts a connect that races the scan teardown.
	pub async fn connect(&self) -> Result<()> {
		let mut last: Option<Error> = None;
		for attempt in 1..=3u64 {
			match self.connect_once().await {
				Ok(()) => {
					self.state.send_replace(ConnectionState::Connected);
					return Ok(());
				}
				Err(e) => {
					eprintln!("[BLE] connect attempt {attempt}/3 failed: {e}");
					let _ = self.peripheral.disconnect().await;
					last = Some(e);
					if attempt < 3 {
						tokio::time::sleep(Duration::from_millis(500 * attempt)).await;
					}
				}
			}
		}
		Err(last.unwrap_or_else(|| "failed to connect to ME device".into()))
	}

	async fn connect_once(&self) -> Result<()> {
		if !self.peripheral.is_connected().await? {
			self.peripheral.connect().await?;
		}
		self.initialize_session().await
	}

	/// Subscribes to CAN frames and enables all PIDs. The peripheral forgets
	/// both on disconnect, so this runs after every (re)connect.
	async fn initialize_session(&self) -> Result<()> {
		self.peripheral.discover_services().await?;
		let can = self.characteristic(CAN_BUS)?;
		let pid = self.characteristic(PID_FILTER)?;
		// Re-arm the CCCD cleanly on reconnects.
		let _ = self.peripheral.unsubscribe(&can).await;
		self.peripheral.subscribe(&can).await?;
		if self.peripheral.write(&pid, &ALLOW_ALL_PIDS, WriteType::WithResponse).await.is_err() {
			self.peripheral.write(&pid, &ALLOW_ALL_PIDS, WriteType::WithoutResponse).await?;
		}
		Ok(())
	}

	fn characteristic(&self, uuid: Uuid) -> Result<Characteristic> {
		self.peripheral
			.characteristics()
			.into_iter()
			.find(|c| c.uuid == uuid && c.service_uuid == SERVICE)
			.ok_or_else(|| format!("characteristic {uuid} not found on ME device").into())
	}

	/// Streams frames into the shared data until `stop` fires, reconnecting
	/// with a 1 s → 10 s backoff whenever the link drops (`AttemptReconnect`).
	pub async fn run(&self, mut stop: watch::Receiver<bool>) -> Result<()> {
		let id = self.peripheral.id();
		loop {
			let mut notifications = self.peripheral.notifications().await?;
			let mut events = self.adapter.events().await?;
			// Pump frames until the link drops or we're told to stop.
			loop {
				tokio::select! {
					_ = stop.changed() => {
						let _ = self.peripheral.disconnect().await;
						self.state.send_replace(ConnectionState::Disconnected);
						return Ok(());
					}
					n = notifications.next() => match n {
						Some(n) if n.uuid == CAN_BUS => self.on_packet(&n.value),
						Some(_) => {}
						None => break,
					},
					e = events.next() => match e {
						Some(CentralEvent::DeviceDisconnected(d)) if d == id => break,
						Some(_) => {}
						None => break,
					},
				}
			}

			eprintln!("[BLE] lost device {}, attempting reconnect", self.address());
			self.state.send_replace(ConnectionState::Reconnecting);
			let mut delay = 1;
			loop {
				match self.connect_once().await {
					Ok(()) => {
						eprintln!("[BLE] device {} reconnected", self.address());
						self.state.send_replace(ConnectionState::Connected);
						break;
					}
					Err(e) => {
						eprintln!("[BLE] could not reconnect ({e}), trying again in {delay} s");
						tokio::select! {
							_ = stop.changed() => {
								self.state.send_replace(ConnectionState::Disconnected);
								return Ok(());
							}
							_ = tokio::time::sleep(Duration::from_secs(delay)) => {}
						}
						delay = (delay + 1).min(10);
					}
				}
			}
		}
	}

	fn on_packet(&self, packet: &[u8]) {
		match racechrono::parse_notification(packet) {
			Some(Ok(frame)) => {
				if let Ok(mut d) = self.data.lock() {
					d.apply(&frame);
				}
			}
			Some(Err(e)) => eprintln!("[BLE] bad frame: {e}"),
			None => {}
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn parses_mac_formats() {
		let want = BDAddr::from([0x20, 0xE7, 0xC8, 0xB9, 0x20, 0x2A]);
		assert_eq!(parse_mac("20:E7:C8:B9:20:2A"), Some(want));
		assert_eq!(parse_mac(" 20-e7-c8-b9-20-2a "), Some(want));
		assert_eq!(parse_mac("20E7C8B9202A"), Some(want));
		assert_eq!(parse_mac("20:E7:C8"), None);
		assert_eq!(parse_mac("ZZ:E7:C8:B9:20:2A"), None);
	}

	#[test]
	fn uuids_are_racechrono() {
		assert_eq!(SERVICE.to_string(), "00001ff8-0000-1000-8000-00805f9b34fb");
		assert_eq!(CAN_BUS.to_string(), "00000001-0000-1000-8000-00805f9b34fb");
		assert_eq!(PID_FILTER.to_string(), "00000002-0000-1000-8000-00805f9b34fb");
	}
}
