#!/bin/bash
# ROCKNIX side of the launcher, sourced by GaugeRust.sh and deployed next to
# the app. Anything changed here is undone by rocknix_restore,
# which runs however the launcher ends.
#
# Uses from the launcher: APP_NAME, GAMEDIR. Settings, usually from the app's
# .env file: GAUGE_ARGS, KEEP_AWAKE (default 1), GPU_FLOOR (default 0).

bt_started=0
inhibit_pid=""
monitor_pid=""
gpu_devfreq=""
gpu_min_prev=""

kernel_trouble_counts() {
  echo "[KERNEL] ccu_helper warnings: $(dmesg | grep -c 'WARNING.*ccu_helper_wait_for_lock')," \
    "panfrost faults/resets: $(dmesg | grep -ciE 'panfrost.*(fault|timeout|reset)')"
}

rocknix_ensure_hci0() {
  # After a cold boot the kernel's H5 driver is bound to the chip but hci0 never
  # appears, and nothing in the system changes that. Binding it again does, and
  # loads the firmware. hci0 stays afterwards (down, once the service stops).
  [ -e /sys/class/bluetooth/hci0 ] && return
  local driver=/sys/bus/serial/drivers/hci_uart_h5 dev name i bound=0
  for dev in "$driver"/serial*; do
    [ -e "$dev" ] || continue
    bound=1
    name="$(basename "$dev")"
    echo "[BT] no hci0: rebinding hci_uart_h5 to $name"
    echo "$name" > "$driver/unbind"
    sleep 1
    echo "$name" > "$driver/bind"
  done
  if [ "$bound" = 0 ]; then
    # Not bound at all: the driver only accepts the Bluetooth chip's device.
    for dev in /sys/bus/serial/devices/*; do
      name="$(basename "$dev")"
      echo "$name" > "$driver/bind" 2>/dev/null &&
        echo "[BT] no hci0: bound hci_uart_h5 to $name"
    done
  fi
  for i in 1 2 3 4 5 6 7 8 9 10; do
    [ -e /sys/class/bluetooth/hci0 ] && { echo "[BT] hci0 present"; return; }
    sleep 1
  done
  echo "[BT] FAILED: still no hci0"
}

rocknix_bluetooth_up() {
  rocknix_ensure_hci0
  # The kernel attaches the controller itself and bluetooth.service is the
  # system's own bluetoothd, so only that service is started. The system's
  # controllers.bluetooth.enabled setting stays as the user left it.
  if systemctl -q is-active bluetooth; then
    echo "[BT] bluetooth.service already running; leaving it alone"
  elif systemctl start bluetooth; then
    bt_started=1
    echo "[BT] started bluetooth.service"
  else
    echo "[BT] FAILED to start bluetooth.service"
    systemctl status --no-pager bluetooth | tail -n 8
    return
  fi
  local i
  for i in 1 2 3 4 5 6 7 8 9 10; do
    timeout 3 bluetoothctl show 2>/dev/null | grep -q "Powered: yes" && break
    [ "$i" = 3 ] && timeout 3 bluetoothctl power on
    sleep 1
  done
  timeout 3 bluetoothctl show | grep -E "Controller|Powered|PowerState|Discovering"
  hciconfig hci0 | sed -n '1p;3p'
}

rocknix_keep_awake() {
  if [ "${KEEP_AWAKE:-1}" != 1 ]; then
    echo "[AWAKE] KEEP_AWAKE=${KEEP_AWAKE}: no idle lock taken"
    return
  fi
  # Runs on its own rather than wrapping the app, so that a logind problem
  # can't stop the gauges from starting. Deliberately "idle" and not "sleep":
  # a sleep lock would also disable the lid and the power button.
  systemd-inhibit --what=idle --who="$APP_NAME" --why="Gauges on screen" --mode=block \
    sleep 2147483647 2>/dev/null &
  inhibit_pid=$!
  sleep 0.3
  systemd-inhibit --list --no-legend --no-pager | grep "^$APP_NAME " ||
    echo "[AWAKE] WARNING: idle lock not held"
}

rocknix_gpu_floor() {
  [ "${GPU_FLOOR:-0}" = 1 ] || return
  local d
  for d in /sys/class/devfreq/*.gpu; do
    [ -w "$d/min_freq" ] && gpu_devfreq="$d"
  done
  [ -n "$gpu_devfreq" ] || { echo "[GPU] no devfreq device; floor not set"; return; }
  # The top step in the list can be above max_freq (it is an overclock step
  # unless enable.gpu-overclock is on), so never ask for more than max_freq.
  local top max
  top="$(tr ' ' '\n' < "$gpu_devfreq/available_frequencies" | grep . | sort -n | tail -n 1)"
  max="$(cat "$gpu_devfreq/max_freq")"
  [ "$top" -gt "$max" ] && top="$max"
  gpu_min_prev="$(cat "$gpu_devfreq/min_freq")"
  echo "$top" > "$gpu_devfreq/min_freq" && echo "[GPU] floor $top (was $gpu_min_prev)"
}

rocknix_monitor() {
  # One line a minute: if the screen goes dark or Bluetooth drops in the car,
  # this says when and what state things were in.
  local bl backlight
  backlight="$(ls -d /sys/class/backlight/* 2>/dev/null | head -n 1)"
  while sleep 60; do
    bl="$(cat "$backlight/bl_power" 2>/dev/null)/$(cat "$backlight/brightness" 2>/dev/null)"
    echo "[MON] $(date '+%H:%M:%S')" \
      "bat=$(cat /sys/class/power_supply/battery/capacity 2>/dev/null)%/$(cat /sys/class/power_supply/battery/status 2>/dev/null)" \
      "bl_power/brightness=$bl" \
      "output_power=$(swaymsg -t get_outputs -r 2>/dev/null | grep -m1 '"power"' | tr -d ' ,')" \
      "cpu=$(cat /sys/devices/system/cpu/cpu0/cpufreq/scaling_cur_freq 2>/dev/null)" \
      "gpu=$(cat /sys/class/devfreq/*.gpu/cur_freq 2>/dev/null)" \
      "temp=$(cat /sys/class/thermal/thermal_zone0/temp 2>/dev/null)" \
      "bt=$(systemctl is-active bluetooth 2>/dev/null)"
  done
}

rocknix_restore() {
  trap - EXIT
  [ -n "$monitor_pid" ] && kill "$monitor_pid" 2>/dev/null
  if [ -n "$gpu_min_prev" ]; then
    echo "$gpu_min_prev" > "$gpu_devfreq/min_freq" && echo "[GPU] floor restored to $gpu_min_prev"
  fi
  if [ -n "$inhibit_pid" ]; then
    # Ending its child ends systemd-inhibit, which releases the lock.
    pkill -P "$inhibit_pid" sleep 2>/dev/null
    kill "$inhibit_pid" 2>/dev/null
    echo "[AWAKE] idle lock released"
  fi
  if [ "$bt_started" = 1 ]; then
    systemctl stop bluetooth && echo "[BT] stopped bluetooth.service (we started it)"
  fi
  kernel_trouble_counts
  echo "===== $APP_NAME end $(date '+%Y-%m-%d %H:%M:%S') ====="
}

rocknix_is_demo() { # config-file
  case " ${GAUGE_ARGS:-} " in *" --demo "*) return 0 ;; esac
  grep -qsiE '"DemoMode"[[:space:]]*:[[:space:]]*true' "$1"
}

# Call just before starting the app.
rocknix_prepare() { # config-file
  trap rocknix_restore EXIT
  # The system's quit hotkey (default L1+SELECT+START) sends SIGTERM to the
  # app; these are for when the launcher itself is signalled.
  trap 'exit 143' TERM HUP INT
  set_kill set "$APP_NAME"
  echo "OS=$(. /etc/os-release; echo "$OS_NAME $OS_VERSION") kernel=$(uname -r)" \
    "SDL_VIDEODRIVER=$SDL_VIDEODRIVER WAYLAND_DISPLAY=$WAYLAND_DISPLAY"
  kernel_trouble_counts
  if rocknix_is_demo "$1"; then
    echo "[BT] demo mode: Bluetooth not started"
  else
    rocknix_bluetooth_up
  fi
  rocknix_keep_awake
  rocknix_gpu_floor
  rocknix_monitor &
  monitor_pid=$!
}
