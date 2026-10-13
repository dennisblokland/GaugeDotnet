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
ART_DIR="${ART_DIR:-../muos}"
# Set to 1 to also upload a gauges.json from this folder (otherwise the
# device's existing one is kept, or a default is created on first run).
UPLOAD_CONFIG="${UPLOAD_CONFIG:-0}"

# Password login through sshpass when it's installed; otherwise plain ssh,
# which uses your SSH key (set one up once with `ssh-copy-id root@<host>`).
if command -v sshpass >/dev/null && [ -n "$REMOTE_PASS" ]; then
  SSH="sshpass -p $REMOTE_PASS ssh -o StrictHostKeyChecking=no"
  SCP="sshpass -p $REMOTE_PASS scp -o StrictHostKeyChecking=no"
else
  SSH="ssh"
  SCP="scp"
fi

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

# muOS artwork for the Ports list: box art, preview and description, matched
# to the launcher by name.
CATALOGUE_DIR="${CATALOGUE_DIR:-/run/muos/storage/info/catalogue/External - Ports}"
ART_NAME="${RUN_SCRIPT_NAME%.sh}"
if [ -f "$ART_DIR/$ART_NAME.png" ]; then
  echo "Uploading muOS artwork to $CATALOGUE_DIR..."
  $SSH "$REMOTE_USER@$REMOTE_HOST" "mkdir -p '$CATALOGUE_DIR/box' '$CATALOGUE_DIR/preview' '$CATALOGUE_DIR/text'"
  for kind in box preview; do
    $SSH "$REMOTE_USER@$REMOTE_HOST" "cat > '$CATALOGUE_DIR/$kind/$ART_NAME.png'" < "$ART_DIR/$ART_NAME.png"
  done
  if [ -f "$ART_DIR/$ART_NAME.txt" ]; then
    $SSH "$REMOTE_USER@$REMOTE_HOST" "cat > '$CATALOGUE_DIR/text/$ART_NAME.txt'" < "$ART_DIR/$ART_NAME.txt"
  fi
fi

echo "Done. On the device: Ports -> GaugeRust (log: $BIN_DIR/GaugeRust.log)"
