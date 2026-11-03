# ROCKNIX

The app runs on ROCKNIX on the Anbernic RG35XX-SP (H700), next to its muOS
support. Launcher and app detect the OS from `OS_NAME` in `/etc/os-release`, so
there is one build and one launcher.

## Deploy

```bash
REMOTE_HOST=<device ip> ./deploy-arm64.sh
```

The target is detected over ssh; `TARGET=rocknix` or `TARGET=muos` overrides it.
With `ES_SYSTEM=0`, the deploy leaves out the home screen card.

| What | Where |
|---|---|
| Binary, `gauges.json`, log, `rocknix-launcher.sh` | `/storage/GaugeRust` |
| Launcher and Ports entry | `/storage/roms/ports/<App>.sh`, `gamelist.xml`, `images/` |
| "Gauges" home screen card | `/storage/.config/emulationstation/es_systems_gauge.cfg`, `/storage/GaugeMenu` |
| Direct launch from the card | `/storage/.config/emulationstation/scripts/{system,game}-selected/gauge-direct-launch.sh` |
| The card's art | `/storage/roms/_userdata/theme-customizations/art-book-next/artwork/gauges.png` |

All of it is under `/storage`, which survives OS updates. Nothing the system ships
is edited. The deploy also sets two theme options in ES's settings the first time,
restarting ES to do so: **Distribution: RockNIX** and **System Artwork: Custom**. Other
cards keep their art, because the theme uses a custom image only where one exists. If
either option is already set to something else, the deploy leaves it and says so, and
the card keeps the theme's generic art.

To remove it, delete those paths and the entries in `/storage/roms/ports/gamelist.xml`.
Then run `systemctl stop essway`, delete the `subset.distribution` and
`subset.system-view-artwork` lines from `/storage/.config/emulationstation/es_settings.cfg`,
and run `systemctl start essway`.

## Using it

- **Start:** press A on the **Gauges** card on the home screen, which starts the
  app straight away. It is also listed under **Ports**.
- **After quitting from the card:** ES is back on the card's one-item list; B returns to
  the home screen.
- **Quit:** MENU, also during "Connecting". If the app ever hangs, the system's
  hotkey (L1 + SELECT + START) ends it, by force after 5 seconds.
- **Settings:** an optional `GaugeRust.env` next to the binary,
  sourced by the launcher:

| Setting | Default | Effect |
|---|---|---|
| `GAUGE_ARGS` | empty | Extra arguments for `gauge app`, e.g. `--demo` |
| `KEEP_AWAKE` | `1` | Hold the logind idle lock |
| `GPU_FLOOR` | `0` | Hold the GPU at `max_freq` while running |

## How it differs from muOS

**Buttons.** ROCKNIX numbers `/dev/input/js0` differently. The app translates to the
codes written for muOS (`gauge-app/src/input.rs`).

| Button | ROCKNIX | muOS |
|---|---|---|
| B / A | 0 / 1 | 1 / 0 |
| Y / X | 2 / 3 | 3 / 2 |
| L1 / R1 | 4 / 5 | 4 / 5 |
| L2 / R2 | 6 / 7 | 9 / 10 |
| SELECT / START | 8 / 9 | 7 / 6 |
| MENU | 10 | 8 |
| D-pad | buttons 13-16 | axes 3 and 4 |

**Bluetooth.** The muOS bring-up (`rtk_hciattach`, restarting `bluetoothd`) doesn't
run. The launcher starts the system's `bluetooth.service` and stops it on exit if it
was the one that started it. `controllers.bluetooth.enabled` isn't changed. Stopping
the service makes the system write `/storage/roms/backups/bluetooth.tar`, which is its
normal behaviour.

After a cold boot the kernel's `hci_uart_h5` driver is bound to the chip, but `hci0`
doesn't appear, and it was still missing after two minutes. Binding the driver again
creates it and loads the firmware in about 2 seconds, so the launcher does that when
`hci0` is missing and logs `[BT] no hci0: rebinding`. `hci0` then stays, down, until
the next boot.

**Staying awake.** muOS's `idle_inhibit` file doesn't exist here, and `powerstate`
has nothing to do with idling. Two locks are held while the app runs:

- The launcher holds a logind `idle` lock (`systemd-inhibit`).
- SDL holds a Wayland idle inhibitor on the window, which sway reports as
  `inhibit_idle: true`.

The lid and the power button still suspend the device on purpose. A logind `sleep`
lock would disable them too. That means a bumped power button suspends the gauges.

**GPU.** Panfrost instead of the Mali blob, at `/sys/class/devfreq/1800000.gpu`. The
top listed step (648 MHz) is above `max_freq` (600 MHz) unless `enable.gpu-overclock`
is on, so the floor is capped at `max_freq`. Changing the clock floor logs a
`ccu_helper_wait_for_lock` kernel warning. Results are in
[`bench/RESULTS.md`](../bench/RESULTS.md).

**Home screen.** ES loads every `es_systems_*.cfg` from its config folder alongside
the stock list. The theme is read-only, so the card's art goes in its customization
folder (see Deploy), and "GAUGES" is the theme's text for a system with no logo.
`<theme>ports</theme>` would give a second card reading PORTS. A `..` in `<theme>` isn't
an alternative: ES then drops the whole theme for that card. `card-art.py` draws the art.

ES has no setting for starting something straight from the home screen; a card always
opens its list. It does run event scripts, and it reports `game-selected` when the list
opens, so `es-direct-launch.sh` starts the app from there. ES also reports that event
when the app ends and when games are reloaded with the list open, and it queues a
launch per request. So the script only fires once after `system-selected` has reported
the Gauges card, otherwise the app would restart repeatedly. The card holds one app
for this to work.

## Logs

`/storage/GaugeRust/GaugeRust.log`; one previous log is kept as
`.log.1` once it passes 1 MiB.

| Line | Meaning |
|---|---|
| `PLATFORM=rocknix` | OS detection worked |
| `[BT] no hci0: rebinding ...`, `[BT] hci0 present` | Cold boot Bluetooth fix ran |
| `[BT] FAILED: still no hci0` | The adapter didn't come up |
| `[BT] started bluetooth.service`, `Powered: yes` | Bluetooth is up |
| `[BLE] scanning for <mac>` | Looking for the ECU; 60 second limit |
| `[BLE] ME device <mac> connected` | Connected to the ECU |
| `[fps] 60.00 avg over 30 s, slowest frame 23.5 ms` | Frame rate; a large slowest frame is a stutter |
| `[MON] ...` | Once a minute: battery, backlight, output power, clocks, temperature, Bluetooth |
| `[KERNEL] ccu_helper warnings: N, panfrost faults/resets: N` | At start and end; compare them |
