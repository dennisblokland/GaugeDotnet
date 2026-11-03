#!/usr/bin/env bash
# Builds the app for the RG35XX and copies it plus its launcher to the device.
#
# The target OS is detected over ssh (or set TARGET=muos / TARGET=rocknix):
#   REMOTE_HOST=<device ip> ./deploy-arm64.sh
set -euo pipefail
cd "$(dirname "$0")"

TARGET="${TARGET:-auto}"
if [ "$TARGET" = rocknix ] && [ -z "${REMOTE_HOST:-}" ]; then
  echo "TARGET=rocknix needs REMOTE_HOST=<device ip>"
  exit 1
fi
REMOTE_HOST="${REMOTE_HOST:-192.168.0.234}"
REMOTE_USER="${REMOTE_USER:-root}"
REMOTE_PASS="${REMOTE_PASS:-root}"
RUN_SCRIPT="${RUN_SCRIPT:-./GaugeRust.sh}"
ART_DIR="${ART_DIR:-muos}"
# Set to 1 to also upload a gauges.json from this folder (otherwise the
# device's existing one is kept, or a default is created on first run).
UPLOAD_CONFIG="${UPLOAD_CONFIG:-0}"

# An SSH key if the device accepts one. Otherwise a password through sshpass
# when it's installed, or plain ssh (set a key up with `ssh-copy-id root@<host>`).
if ssh -n -o BatchMode=yes -o ConnectTimeout=5 "$REMOTE_USER@$REMOTE_HOST" true 2>/dev/null; then
  SSH="ssh"
elif command -v sshpass >/dev/null && [ -n "$REMOTE_PASS" ]; then
  SSH="sshpass -p $REMOTE_PASS ssh -o StrictHostKeyChecking=no"
else
  SSH="ssh"
fi
SSH_HOST=($SSH "$REMOTE_USER@$REMOTE_HOST")

. rocknix/install-port.sh
if [ "$TARGET" = auto ]; then
  TARGET=muos
  [ "$(rocknix_is_target "${SSH_HOST[@]}")" = rocknix ] && TARGET=rocknix
fi
echo "Target: $TARGET ($REMOTE_HOST)"

case "$TARGET" in
  rocknix)
    BIN_ROOT="${BIN_ROOT:-/storage}"
    SCRIPT_DIR="${SCRIPT_DIR:-/storage/roms/ports}"
    ;;
  muos)
    BIN_ROOT="${BIN_ROOT:-/mnt/mmc/ports}"
    SCRIPT_DIR="${SCRIPT_DIR:-/mnt/mmc/roms/ports}"
    ;;
  *)
    echo "Unknown TARGET=$TARGET (muos or rocknix)"
    exit 1
    ;;
esac
BIN_DIR="${BIN_DIR:-$BIN_ROOT/GaugeRust}"

[ "${SKIP_BUILD:-0}" = 1 ] || ./build-arm64.sh
BIN="./target/aarch64-unknown-linux-gnu/release/gauge"
[ -f "$BIN" ] || { echo "Missing $BIN"; exit 1; }

# Copied with cat rather than scp, which needs an sftp server on newer OpenSSH.
# The rename means this also works while the old binary is running.
upload() { # local-file remote-file
  "${SSH_HOST[@]}" "cat > '$2.new' && mv -f '$2.new' '$2'" < "$1"
}

echo "Uploading $BIN to $REMOTE_USER@$REMOTE_HOST:$BIN_DIR..."
"${SSH_HOST[@]}" "mkdir -p '$BIN_DIR' '$SCRIPT_DIR'"
upload "$BIN" "$BIN_DIR/gauge"
if [ "$UPLOAD_CONFIG" = 1 ] && [ -f gauges.json ]; then
  upload gauges.json "$BIN_DIR/gauges.json"
fi

RUN_SCRIPT_NAME="$(basename "$RUN_SCRIPT")"
echo "Uploading launcher to $SCRIPT_DIR/$RUN_SCRIPT_NAME..."
upload "$RUN_SCRIPT" "$SCRIPT_DIR/$RUN_SCRIPT_NAME"
[ "$TARGET" != rocknix ] || upload rocknix/launcher.sh "$BIN_DIR/rocknix-launcher.sh"
"${SSH_HOST[@]}" "chmod +x '$BIN_DIR/gauge' '$SCRIPT_DIR/$RUN_SCRIPT_NAME'"

ART_NAME="${RUN_SCRIPT_NAME%.sh}"
if [ "$TARGET" = rocknix ]; then
  echo "Adding $ART_NAME to the ROCKNIX home screen..."
  rocknix_install_menus "$ART_NAME" "$ART_DIR" "${SSH_HOST[@]}"
elif [ -f "$ART_DIR/$ART_NAME.png" ]; then
  # muOS artwork for the Ports list: box art, preview and description, matched
  # to the launcher by name.
  CATALOGUE_DIR="${CATALOGUE_DIR:-/run/muos/storage/info/catalogue/External - Ports}"
  echo "Uploading muOS artwork to $CATALOGUE_DIR..."
  "${SSH_HOST[@]}" "mkdir -p '$CATALOGUE_DIR/box' '$CATALOGUE_DIR/preview' '$CATALOGUE_DIR/text'"
  for kind in box preview; do
    "${SSH_HOST[@]}" "cat > '$CATALOGUE_DIR/$kind/$ART_NAME.png'" < "$ART_DIR/$ART_NAME.png"
  done
  if [ -f "$ART_DIR/$ART_NAME.txt" ]; then
    "${SSH_HOST[@]}" "cat > '$CATALOGUE_DIR/text/$ART_NAME.txt'" < "$ART_DIR/$ART_NAME.txt"
  fi
fi

echo "Done. On the device: Ports -> GaugeRust, or the Gauge system on ROCKNIX (log: $BIN_DIR/GaugeRust.log)"
