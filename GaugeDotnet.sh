#!/bin/bash
# PORTMASTER: GaugeDotnet.zip, GaugeDotnet.sh

export LANG=en_US.UTF-8
export LC_ALL=en_US.UTF-8

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

echo "[APP] starting $APP_PATH"
"$APP_PATH"
app_status=$?
echo "[APP] exited with status $app_status"
pm_finish
exit "$app_status"
