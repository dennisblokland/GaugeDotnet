#!/usr/bin/env bash
# Builds the Rust app for the RG35XX and copies it plus its launcher to the
# device (like ../deploy-arm64.sh does for the C# app). It installs into its
# own folder, so both versions can sit side by side.
set -euo pipefail
cd "$(dirname "$0")"

REMOTE_HOST="${REMOTE_HOST:-192.168.0.234}"
REMOTE_USER="${REMOTE_USER:-root}"
REMOTE_PASS="${REMOTE_PASS:-root}"
BIN_ROOT="${BIN_ROOT:-/mnt/mmc/ports}"
BIN_DIR="${BIN_DIR:-$BIN_ROOT/GaugeRust}"
SCRIPT_DIR="${SCRIPT_DIR:-/mnt/mmc/roms/ports}"
RUN_SCRIPT="${RUN_SCRIPT:-./GaugeRust.sh}"
# Set to 1 to also upload a gauges.json from this folder (otherwise the
# device's existing one is kept, or a default is created on first run).
UPLOAD_CONFIG="${UPLOAD_CONFIG:-0}"

SSH="sshpass -p $REMOTE_PASS ssh -o StrictHostKeyChecking=no"
SCP="sshpass -p $REMOTE_PASS scp -o StrictHostKeyChecking=no"

[ "${SKIP_BUILD:-0}" = 1 ] || ./build-arm64.sh
BIN="./target/aarch64-unknown-linux-gnu/release/gauge"
[ -f "$BIN" ] || { echo "Missing $BIN"; exit 1; }

echo "Uploading $BIN to $REMOTE_USER@$REMOTE_HOST:$BIN_DIR..."
$SSH "$REMOTE_USER@$REMOTE_HOST" "mkdir -p '$BIN_DIR' '$SCRIPT_DIR'"
$SCP "$BIN" "$REMOTE_USER@$REMOTE_HOST:$BIN_DIR/gauge"
if [ "$UPLOAD_CONFIG" = 1 ] && [ -f gauges.json ]; then
  $SCP gauges.json "$REMOTE_USER@$REMOTE_HOST:$BIN_DIR/gauges.json"
fi

RUN_SCRIPT_NAME="$(basename "$RUN_SCRIPT")"
echo "Uploading launcher to $SCRIPT_DIR/$RUN_SCRIPT_NAME..."
$SCP "$RUN_SCRIPT" "$REMOTE_USER@$REMOTE_HOST:$SCRIPT_DIR/$RUN_SCRIPT_NAME"
$SSH "$REMOTE_USER@$REMOTE_HOST" "chmod +x '$BIN_DIR/gauge' '$SCRIPT_DIR/$RUN_SCRIPT_NAME'"

echo "Done. On the device: Ports -> GaugeRust (log: $BIN_DIR/GaugeRust.log)"
