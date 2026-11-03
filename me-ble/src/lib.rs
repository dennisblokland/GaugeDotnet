//! Bluetooth LE link to the Motorsport Electronics ECU (port of the C#
//! `BleManager`, `MeDevice` and `BluetoothHardwareInit`).
//!
//! Frames arrive over the RaceChrono BLE protocol, are decoded by
//! [`me_data`] and accumulated into a shared [`me_data::MeData`].
//! [`BleLink`] wraps it all for synchronous callers such as a render loop.

pub mod device;
pub mod hardware;
mod link;

pub use device::{parse_mac, ConnectionState, Found, MeDevice};

/// Blocking [`device::scan_all`] on a throwaway runtime (for CLI tools).
pub fn scan_blocking(duration: std::time::Duration) -> device::Result<Vec<Found>> {
	let rt = tokio::runtime::Builder::new_current_thread().enable_all().build()?;
	rt.block_on(async {
		let adapter = device::adapter().await?;
		device::scan_all(&adapter, duration).await
	})
}
pub use link::{BleLink, LinkOptions, LinkStatus};
