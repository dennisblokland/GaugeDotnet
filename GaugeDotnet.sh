#!/bin/bash
# PORTMASTER: GaugeDotnet.zip, GaugeDotnet.sh

export LANG=en_US.UTF-8
export LC_ALL=en_US.UTF-8

export PATH="/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin:/usr/libexec/bluetooth:$PATH"
XDG_DATA_HOME=${XDG_DATA_HOME:-$HOME/.local/share}

PLATFORM=portmaster
grep -qs '^OS_NAME="\?ROCKNIX' /etc/os-release && PLATFORM=rocknix

if [ "$PLATFORM" = rocknix ]; then
  # Gives us set_kill and the system's environment. PortMaster isn't used:
  # buttons come straight from /dev/input/js0.
  . /etc/profile
  export HOME="${HOME:-/storage}"
  # ES passes these on; the defaults let the launcher also run from ssh.
  export XDG_RUNTIME_DIR="${XDG_RUNTIME_DIR:-/var/run/0-runtime-dir}"
  export WAYLAND_DISPLAY="${WAYLAND_DISPLAY:-wayland-1}"
  export SWAYSOCK="${SWAYSOCK:-$XDG_RUNTIME_DIR/sway-ipc.0.sock}"
  export SDL_VIDEODRIVER="${SDL_VIDEODRIVER:-wayland}"
  # Everything of ours lives under /storage, which survives OS updates.
  APP_ROOT="${APP_ROOT:-/storage}"
  pm_finish() { :; }
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

  source "$controlfolder/control.txt"
  [ -f "${controlfolder}/mod_${CFW_NAME}.txt" ] && source "${controlfolder}/mod_${CFW_NAME}.txt"
  get_controls

  APP_ROOT="${APP_ROOT:-/mnt/mmc/ports}"
fi

GAMEDIR="${GAMEDIR:-$APP_ROOT/GaugeDotnet}"
APP_NAME="GaugeDotnet"
APP_PATH="$GAMEDIR/$APP_NAME"
LOG_FILE="${LOG_FILE:-$GAMEDIR/$APP_NAME.log}"

mkdir -p "$GAMEDIR"
# One previous log is kept; there is no serial console to read this on, so it
# has to survive a restart, but it must not fill the card.
if [ -f "$LOG_FILE" ] && [ "$(wc -c < "$LOG_FILE")" -gt 1048576 ]; then
  mv -f "$LOG_FILE" "$LOG_FILE.1"
fi
touch "$LOG_FILE"
exec >> "$LOG_FILE" 2>&1

echo ""
echo "===== $APP_NAME launch $(date '+%Y-%m-%d %H:%M:%S') ====="
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

# Optional local settings on ROCKNIX, e.g. GPU_FLOOR=0, KEEP_AWAKE=0.
if [ "$PLATFORM" = rocknix ]; then
  [ -f "$GAMEDIR/GaugeDotnet.env" ] && . "$GAMEDIR/GaugeDotnet.env"
  . "$GAMEDIR/rocknix-launcher.sh"
  rocknix_prepare "$GAMEDIR/gauges.json"
fi

# muOS only (rocknix-launcher.sh has its own GPU_FLOOR, and this path doesn't
# exist there). The Mali's simple_ondemand governor never raises the GPU clock
# for Skia's GL work, which costs 20-40% per frame (bench/RESULTS.md). Hold it at
# its top step while the app runs, then restore the previous floor.
GPU_DEVFREQ=/sys/class/devfreq/gpu
muos_gpu_min_prev=""
if [ "$PLATFORM" != rocknix ] && [ -w "$GPU_DEVFREQ/min_freq" ] && [ -r "$GPU_DEVFREQ/available_frequencies" ]; then
  muos_gpu_min_prev="$(cat "$GPU_DEVFREQ/min_freq")"
  gpu_max="$(tr ' ' '\n' < "$GPU_DEVFREQ/available_frequencies" | sort -n | tail -n 1)"
  echo "$gpu_max" > "$GPU_DEVFREQ/min_freq" && echo "[APP] GPU clock held at $gpu_max"
fi

echo "[APP] starting $APP_PATH"
"$APP_PATH"
app_status=$?
echo "[APP] exited with status $app_status"

if [ -n "$muos_gpu_min_prev" ]; then
  echo "$muos_gpu_min_prev" > "$GPU_DEVFREQ/min_freq"
fi
pm_finish
exit "$app_status"
