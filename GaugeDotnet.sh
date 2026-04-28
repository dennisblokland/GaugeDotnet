#!/bin/bash
# PORTMASTER: GaugeDotnet.zip, GaugeDotnet.sh

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

source $controlfolder/control.txt
[ -f "${controlfolder}/mod_${CFW_NAME}.txt" ] && source "${controlfolder}/mod_${CFW_NAME}.txt"
get_controls

APP_ROOT="${APP_ROOT:-/mnt/mmc/ports}"
GAMEDIR="${GAMEDIR:-$APP_ROOT/GaugeDotnet}"
APP_NAME="GaugeDotnet"
APP_PATH="$GAMEDIR/$APP_NAME"

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

# ── BLE setup (Realtek UART on muOS / Allwinner) ──

# 1. D-Bus
if ! pidof dbus-daemon > /dev/null 2>&1; then
  mkdir -p /run/dbus
  dbus-daemon --system --fork || echo "[BLE] FAILED to start dbus-daemon"
fi

# 2. Load the Realtek Bluetooth power-management kernel module
if ! lsmod | grep -q "^rtl_btlpm"; then
  modprobe /lib/modules/$(uname -r)/kernel/drivers/bluetooth/rtl_btlpm.ko 2>/dev/null \
    || modprobe rtl_btlpm 2>/dev/null \
    || echo "[BLE] WARNING: could not load rtl_btlpm"
fi

# 3. Attach the UART to the HCI subsystem
if ! pgrep -f "rtk_hciattach -n -s 115200 /dev/ttyS1 rtk_h5" > /dev/null; then
  rtk_hciattach -n -s 115200 /dev/ttyS1 rtk_h5 &
  sleep 3
fi

# 4. Wait for hci0 to come up
for i in 1 2 3 4 5; do
  hciconfig hci0 2>/dev/null | grep -q "UP" && break
  sleep 1
done

# 5. Force adapter up if still down
if hciconfig hci0 2>/dev/null | grep -q "DOWN"; then
  hciconfig hci0 up
  sleep 1
fi

# 6. Start bluetoothd
if ! pidof bluetoothd > /dev/null 2>&1; then
  /usr/libexec/bluetooth/bluetoothd -n -d &
  sleep 2
fi

echo "[BLE] hci0: $(hciconfig hci0 2>&1 | head -3)"
echo "[BLE] bluetoothd pid: $(pidof bluetoothd 2>/dev/null || echo 'NOT RUNNING')"

"$APP_PATH"
pm_finish