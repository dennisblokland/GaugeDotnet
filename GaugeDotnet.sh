#!/bin/bash
# PORTMASTER: GaugeDotnet.zip, GaugeDotnet.sh

export PATH="/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin:/usr/libexec/bluetooth:$PATH"
XDG_DATA_HOME=${XDG_DATA_HOME:-$HOME/.local/share}

if [ -d "/opt/system/Tools/PortMaster/" ]; then
  controlfolder="/opt/system/Tools/PortMaster"
elif [ -d "/opt/tools/PortMaster/" ]; then
  controlfolder="/opt/tools/PortMaster"
elif [ -d "$XDG_DATA_HOME/PortMaster/" ]; then
  controlfolder="$XDG_DATA_HOME/PortMaster"
else
  controlfolder="/roms/ports/PortMaster"
fi

source "$controlfolder/control.txt"
[ -f "${controlfolder}/mod_${CFW_NAME}.txt" ] && source "${controlfolder}/mod_${CFW_NAME}.txt"
get_controls

APP_ROOT="${APP_ROOT:-/mnt/mmc/ports}"
GAMEDIR="${GAMEDIR:-$APP_ROOT/GaugeDotnet}"
APP_NAME="GaugeDotnet"
APP_PATH="$GAMEDIR/$APP_NAME"
LOG_FILE="${LOG_FILE:-$GAMEDIR/$APP_NAME.log}"
BLUETOOTHD_LOG_FILE="${BLUETOOTHD_LOG_FILE:-$GAMEDIR/bluetoothd.log}"
RTK_HCIATTACH_LOG_FILE="${RTK_HCIATTACH_LOG_FILE:-$GAMEDIR/rtk_hciattach.log}"

mkdir -p "$GAMEDIR"
touch "$LOG_FILE"
exec >> "$LOG_FILE" 2>&1

echo ""
echo "===== $APP_NAME launch $(date '+%Y-%m-%d %H:%M:%S') ====="
echo "GAMEDIR=$GAMEDIR"
echo "APP_PATH=$APP_PATH"
echo "PATH=$PATH"
echo "USER=$(id -un 2>/dev/null || echo unknown)"
echo "PWD=$(pwd)"

if [ ! -f "$APP_PATH" ]; then
  echo "Missing binary: $APP_PATH"
  pm_finish
  exit 1
fi

if [ ! -x "$APP_PATH" ]; then
  chmod +x "$APP_PATH"
fi

cd "$GAMEDIR"

# If GPTOKEYB is available, run it for compatibility with PortMaster controls.
if [ -n "${GPTOKEYB:-}" ]; then
  $GPTOKEYB "$APP_NAME" &
fi

# Some handheld images have incomplete locale/ICU setup; force safe defaults for .NET.
export LANG=C
export LC_ALL=C
export DOTNET_SYSTEM_GLOBALIZATION_INVARIANT=1
export DBUS_SYSTEM_BUS_ADDRESS=unix:path=/run/dbus/system_bus_socket

# ── BLE setup (Realtek UART on muOS / Allwinner) ──

# 1. D-Bus
if ! pidof dbus-daemon > /dev/null 2>&1; then
  mkdir -p /run/dbus
  dbus-daemon --system --fork || echo "[BLE] FAILED to start dbus-daemon"
fi

for i in 1 2 3 4 5; do
  [ -S /run/dbus/system_bus_socket ] && break
  echo "[BLE] waiting for D-Bus system socket ($i/5)"
  sleep 1
done

# 2. Load the Realtek Bluetooth power-management kernel module
if ! lsmod | grep -q "^rtl_btlpm"; then
  modprobe /lib/modules/$(uname -r)/kernel/drivers/bluetooth/rtl_btlpm.ko 2>/dev/null \
    || modprobe rtl_btlpm 2>/dev/null \
    || echo "[BLE] WARNING: could not load rtl_btlpm"
fi

# 3. Attach the UART to the HCI subsystem
if ! pgrep -f "rtk_hciattach -n -s 115200 /dev/ttyS1 rtk_h5" > /dev/null; then
  echo "[BLE] starting rtk_hciattach"
  rtk_hciattach -n -s 115200 /dev/ttyS1 rtk_h5 >> "$RTK_HCIATTACH_LOG_FILE" 2>&1 &
else
  echo "[BLE] rtk_hciattach already running"
fi

# 4. Wait for hci0 to appear
for i in 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15; do
  hciconfig hci0 > /dev/null 2>&1 && break
  echo "[BLE] waiting for hci0 ($i/15)"
  sleep 1
done

# 5. Force adapter up
for i in 1 2 3 4 5; do
  hciconfig hci0 up > /dev/null 2>&1 || true
  hciconfig hci0 2>/dev/null | grep -q "UP" && break
  echo "[BLE] waiting for hci0 UP ($i/5)"
  sleep 1
done

# 6. Start bluetoothd
if ! pidof bluetoothd > /dev/null 2>&1; then
  echo "[BLE] starting bluetoothd"
  /usr/libexec/bluetooth/bluetoothd -n -d >> "$BLUETOOTHD_LOG_FILE" 2>&1 &
else
  echo "[BLE] bluetoothd already running"
fi

for i in 1 2 3 4 5; do
  pidof bluetoothd > /dev/null 2>&1 && break
  echo "[BLE] waiting for bluetoothd ($i/5)"
  sleep 1
done

if command -v bluetoothctl > /dev/null 2>&1; then
  echo "[BLE] powering adapter on through bluetoothctl"
  bluetoothctl power on || echo "[BLE] bluetoothctl power on failed"
  sleep 1
fi

echo "[BLE] hci0: $(hciconfig hci0 2>&1 | head -3)"
echo "[BLE] bluetoothd pid: $(pidof bluetoothd 2>/dev/null || echo 'NOT RUNNING')"

echo "[APP] starting $APP_PATH"
"$APP_PATH"
app_status=$?
echo "[APP] exited with status $app_status"
pm_finish
exit "$app_status"
