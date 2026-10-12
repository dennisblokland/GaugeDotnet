#!/bin/bash
# PORTMASTER: GaugeRust.zip, GaugeRust.sh
# Launcher for the Rust port (`gauge app`); mirrors ../GaugeDotnet.sh.

export LANG=C
export LC_ALL=C
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

if [ -f "$controlfolder/control.txt" ]; then
  source "$controlfolder/control.txt"
  [ -f "${controlfolder}/mod_${CFW_NAME}.txt" ] && source "${controlfolder}/mod_${CFW_NAME}.txt"
  get_controls
fi
type pm_finish >/dev/null 2>&1 || pm_finish() { :; }

APP_ROOT="${APP_ROOT:-/mnt/mmc/ports}"
GAMEDIR="${GAMEDIR:-$APP_ROOT/GaugeRust}"
APP_NAME="gauge"
APP_PATH="$GAMEDIR/$APP_NAME"
LOG_FILE="${LOG_FILE:-$GAMEDIR/GaugeRust.log}"

mkdir -p "$GAMEDIR"
touch "$LOG_FILE"
exec >> "$LOG_FILE" 2>&1

echo ""
echo "===== GaugeRust launch $(date '+%Y-%m-%d %H:%M:%S') ====="
echo "APP_PATH=$APP_PATH"

if [ ! -f "$APP_PATH" ]; then
  echo "Missing binary: $APP_PATH"
  pm_finish
  exit 1
fi
[ -x "$APP_PATH" ] || chmod +x "$APP_PATH"

cd "$GAMEDIR"

# Buttons are read from /dev/input/js0 directly; gptokeyb is only started for
# PortMaster's hotkeys (its key presses are ignored while js0 is in use).
if [ -n "${GPTOKEYB:-}" ]; then
  $GPTOKEYB "$APP_NAME" &
fi

export DBUS_SYSTEM_BUS_ADDRESS=unix:path=/run/dbus/system_bus_socket

# gauges.json is read from (and created in) $GAMEDIR, next to the binary.
echo "[APP] starting $APP_PATH app --fullscreen"
"$APP_PATH" app --fullscreen
app_status=$?
echo "[APP] exited with status $app_status"
pm_finish
exit "$app_status"
