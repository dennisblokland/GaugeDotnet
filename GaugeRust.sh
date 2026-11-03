#!/bin/bash
# PORTMASTER: GaugeRust.zip, GaugeRust.sh
# Launcher for the device app (`gauge app`).
# Runs on muOS (through PortMaster) and on ROCKNIX; the OS is detected below.

export LANG=C
export LC_ALL=C
export PATH="/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin:/usr/libexec/bluetooth:$PATH"
XDG_DATA_HOME=${XDG_DATA_HOME:-$HOME/.local/share}

PLATFORM=portmaster
grep -qs '^OS_NAME="\?ROCKNIX' /etc/os-release && PLATFORM=rocknix

if [ "$PLATFORM" = rocknix ]; then
  # Gives us set_kill and the system's environment. PortMaster isn't used:
  # buttons come straight from /dev/input/js0.
  . /etc/profile
  export LANG=C LC_ALL=C
  export HOME="${HOME:-/storage}"
  # ES passes these on; the defaults let the launcher also run from ssh.
  export XDG_RUNTIME_DIR="${XDG_RUNTIME_DIR:-/var/run/0-runtime-dir}"
  export WAYLAND_DISPLAY="${WAYLAND_DISPLAY:-wayland-1}"
  export SWAYSOCK="${SWAYSOCK:-$XDG_RUNTIME_DIR/sway-ipc.0.sock}"
  export SDL_VIDEODRIVER="${SDL_VIDEODRIVER:-wayland}"
  # Everything of ours lives under /storage, which survives OS updates.
  APP_ROOT="${APP_ROOT:-/storage}"
else
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
  APP_ROOT="${APP_ROOT:-/mnt/mmc/ports}"
fi
type pm_finish >/dev/null 2>&1 || pm_finish() { :; }

GAMEDIR="${GAMEDIR:-$APP_ROOT/GaugeRust}"
APP_NAME="gauge"
APP_PATH="$GAMEDIR/$APP_NAME"
LOG_FILE="${LOG_FILE:-$GAMEDIR/GaugeRust.log}"

mkdir -p "$GAMEDIR"
# One previous log is kept; there is no serial console to read this on, so it
# has to survive a restart, but it must not fill the card.
if [ -f "$LOG_FILE" ] && [ "$(wc -c < "$LOG_FILE")" -gt 1048576 ]; then
  mv -f "$LOG_FILE" "$LOG_FILE.1"
fi
touch "$LOG_FILE"
exec >> "$LOG_FILE" 2>&1

echo ""
echo "===== GaugeRust launch $(date '+%Y-%m-%d %H:%M:%S') ====="
echo "PLATFORM=$PLATFORM"
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
[ -x "$APP_PATH" ] || chmod +x "$APP_PATH"

cd "$GAMEDIR"

# Optional local settings, e.g. GAUGE_ARGS="--demo", GPU_FLOOR=1, KEEP_AWAKE=0.
GAUGE_ARGS=""
[ -f "$GAMEDIR/GaugeRust.env" ] && . "$GAMEDIR/GaugeRust.env"

# Buttons are read from /dev/input/js0 directly; gptokeyb is only started for
# PortMaster's hotkeys (its key presses are ignored while js0 is in use).
if [ -n "${GPTOKEYB:-}" ]; then
  $GPTOKEYB "$APP_NAME" &
fi

export DBUS_SYSTEM_BUS_ADDRESS=unix:path=/run/dbus/system_bus_socket

if [ "$PLATFORM" = rocknix ]; then
  . "$GAMEDIR/rocknix-launcher.sh"
  rocknix_prepare "$GAMEDIR/gauges.json"
fi

# The renderer draws on the CPU, so the GPU clock isn't held up by default
# (GPU_FLOOR=1 on ROCKNIX, which measured no gain: see bench/RESULTS.md).

# gauges.json is read from (and created in) $GAMEDIR, next to the binary.
echo "[APP] starting $APP_PATH app --fullscreen $GAUGE_ARGS"
# shellcheck disable=SC2086 # GAUGE_ARGS is a list of flags
"$APP_PATH" app --fullscreen $GAUGE_ARGS
app_status=$?
echo "[APP] exited with status $app_status"
pm_finish
exit "$app_status"
