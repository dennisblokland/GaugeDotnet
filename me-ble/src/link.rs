//! Synchronous handle around the async BLE stack, for render loops: start it,
//! read [`BleLink::snapshot`] every frame, check [`BleLink::status`].

use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use me_data::MeData;
use tokio::sync::watch;

use crate::device::{self, ConnectionState, MeDevice};
use crate::hardware;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LinkStatus {
	Starting,
	InitializingHardware,
	Scanning,
	Connecting,
	Connected,
	Reconnecting,
	Disconnected,
	/// Startup failed; the message is meant for the error screen.
	Failed(String),
}

#[derive(Clone, Debug)]
pub struct LinkOptions {
	/// ECU MAC address (`DeviceMacAddress`); `None` connects to the first ME device found.
	pub mac: Option<String>,
	/// Run the RG35XX Bluetooth bring-up first.
	pub init_hardware: bool,
	/// How long clearing BlueZ's cache and scanning may take before giving
	/// up (the C# app uses 60 s).
	pub scan_timeout: Duration,
}

impl Default for LinkOptions {
	fn default() -> Self {
		Self { mac: None, init_hardware: false, scan_timeout: Duration::from_secs(60) }
	}
}

/// Connection to the ECU running on a background thread.
pub struct BleLink {
	data: Arc<Mutex<MeData>>,
	status: Arc<Mutex<LinkStatus>>,
	stop: watch::Sender<bool>,
	thread: Option<JoinHandle<()>>,
}

impl BleLink {
	pub fn start(options: LinkOptions) -> Self {
		let data = Arc::new(Mutex::new(MeData::default()));
		let status = Arc::new(Mutex::new(LinkStatus::Starting));
		let (stop, stop_rx) = watch::channel(false);
		let (d, s) = (data.clone(), status.clone());
		let thread = std::thread::Builder::new()
			.name("ble".into())
			.spawn(move || {
				let rt = match tokio::runtime::Builder::new_multi_thread().worker_threads(1).enable_all().build() {
					Ok(rt) => rt,
					Err(e) => {
						set(&s, LinkStatus::Failed(format!("Bluetooth runtime error:\n{e}")));
						return;
					}
				};
				let mut stop_main = stop_rx.clone();
				rt.block_on(async {
					tokio::select! {
						_ = stop_main.changed() => {}
						_ = run(options, d, s.clone(), stop_rx) => {}
					}
				});
			})
			.expect("spawn BLE thread");
		Self { data, status, stop, thread: Some(thread) }
	}

	/// Copy of the latest ECU values (`MEData.Snapshot`).
	pub fn snapshot(&self) -> MeData {
		self.data.lock().map(|d| *d).unwrap_or_default()
	}

	pub fn status(&self) -> LinkStatus {
		self.status.lock().map(|s| s.clone()).unwrap_or(LinkStatus::Disconnected)
	}

	pub fn is_connected(&self) -> bool {
		self.status() == LinkStatus::Connected
	}
}

impl Drop for BleLink {
	fn drop(&mut self) {
		self.stop.send_replace(true);
		if let Some(t) = self.thread.take() {
			let _ = t.join();
		}
	}
}

fn set(status: &Mutex<LinkStatus>, s: LinkStatus) {
	if let Ok(mut g) = status.lock() {
		*g = s;
	}
}

async fn run(
	options: LinkOptions,
	data: Arc<Mutex<MeData>>,
	status: Arc<Mutex<LinkStatus>>,
	stop: watch::Receiver<bool>,
) {
	let fail = |msg: String| {
		eprintln!("[BLE] {}", msg.replace('\n', " "));
		set(&status, LinkStatus::Failed(msg));
	};

	let mac = options.mac.as_deref().map(str::trim).filter(|m| !m.is_empty());
	let address = match mac.map(device::parse_mac) {
		Some(None) => return fail("Invalid Bluetooth MAC\naddress in config.".into()),
		Some(Some(a)) => Some(a),
		None => None,
	};

	if options.init_hardware {
		set(&status, LinkStatus::InitializingHardware);
		let _ = tokio::task::spawn_blocking(hardware::initialize).await;
	}

	// Like C#'s 60 s CancelAfter, the timeout covers clearing the cache too.
	let found = tokio::time::timeout(options.scan_timeout, async {
		// A cached device updates silently instead of re-announcing; start
		// clean. C# does this whenever a MAC is configured.
		if let Some(mac) = mac.map(str::to_string) {
			let _ = tokio::task::spawn_blocking(move || {
				hardware::disconnect_cached_device(&mac);
				hardware::evict_cached_device(&mac);
			})
			.await;
		}
		let adapter = device::adapter().await?;
		set(&status, LinkStatus::Scanning);
		eprintln!("[BLE] scanning for {}", mac.unwrap_or("any ME device"));
		let peripheral = device::find(&adapter, address).await?;
		Ok::<_, device::Error>((adapter, peripheral))
	})
	.await;
	let (adapter, peripheral) = match found {
		Err(_) => return fail("Timed out while scanning\nfor ME device.".into()),
		Ok(Err(e)) => return fail(format!("Bluetooth error:\n{e}")),
		Ok(Ok(found)) => found,
	};

	set(&status, LinkStatus::Connecting);
	let device = MeDevice::new(adapter, peripheral, data);
	if let Err(e) = device.connect().await {
		return fail(format!("Bluetooth error:\n{e}"));
	}
	eprintln!("[BLE] ME device {} connected", device.address());
	set(&status, LinkStatus::Connected);

	// Mirror connection changes into the status.
	let mut state = device.state();
	let mirror = status.clone();
	tokio::spawn(async move {
		while state.changed().await.is_ok() {
			let s = match *state.borrow() {
				ConnectionState::Connected => LinkStatus::Connected,
				ConnectionState::Reconnecting => LinkStatus::Reconnecting,
				ConnectionState::Disconnected => LinkStatus::Disconnected,
			};
			set(&mirror, s);
		}
	});

	if let Err(e) = device.run(stop).await {
		fail(format!("Bluetooth error:\n{e}"));
	}
}
