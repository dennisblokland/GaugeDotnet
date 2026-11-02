#!/usr/bin/env bash
# Publishes the C# app and copies it plus its launcher to the device. The target
# OS is detected over ssh (or set TARGET=muos / TARGET=rocknix):
#   REMOTE_HOST=<device ip> ./deploy-arm64.sh
set -euo pipefail

PROJECT="${1:-./src/GaugeDotnet/GaugeDotnet.csproj}"
TARGET="${TARGET:-auto}"
if [ "$TARGET" = rocknix ] && [ -z "${REMOTE_HOST:-}" ]; then
  echo "TARGET=rocknix needs REMOTE_HOST=<device ip>"
  exit 1
fi
REMOTE_HOST="${REMOTE_HOST:-192.168.0.234}"
REMOTE_USER="${REMOTE_USER:-root}"
REMOTE_PASS="${REMOTE_PASS:-root}"
CONFIG="${CONFIG:-Release}"
RUN_SCRIPT="${RUN_SCRIPT:-./GaugeDotnet.sh}"
ART_DIR="${ART_DIR:-./muos}"

PUBLISH_DIR="./publish-arm64"

# An SSH key if the device accepts one, otherwise sshpass for non-interactive
# password auth.
if ssh -n -o BatchMode=yes -o ConnectTimeout=5 "$REMOTE_USER@$REMOTE_HOST" true 2>/dev/null; then
  SSH="ssh"
  SCP="scp"
else
  SSH="sshpass -p $REMOTE_PASS ssh -o StrictHostKeyChecking=no"
  SCP="sshpass -p $REMOTE_PASS scp -o StrictHostKeyChecking=no"
fi

. ./rocknix/install-port.sh
if [ "$TARGET" = auto ]; then
  TARGET=muos
  [ "$(rocknix_is_target $SSH "$REMOTE_USER@$REMOTE_HOST")" = rocknix ] && TARGET=rocknix
fi
echo "Target: $TARGET ($REMOTE_HOST)"

case "$TARGET" in
  rocknix)
    BIN_ROOT="${BIN_ROOT:-/storage}"
    SCRIPT_DIR="${SCRIPT_DIR:-/storage/roms/ports}"
    # No sftp server there, which scp needs by default.
    SCP="$SCP -O"
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
BIN_DIR="${BIN_DIR:-$BIN_ROOT/GaugeDotnet}"

echo "Building $PROJECT for linux-arm64..."
rm -rf "$PUBLISH_DIR"

dotnet publish "$PROJECT" \
  -c "$CONFIG" \
  -r linux-arm64 \
  --self-contained true \
  -o "$PUBLISH_DIR" \
  /p:PublishSingleFile=true

if [ ! -f "$RUN_SCRIPT" ]; then
  echo "Run script not found: $RUN_SCRIPT"
  exit 1
fi

echo "Uploading binaries to $REMOTE_USER@$REMOTE_HOST:$BIN_DIR..."
$SSH "$REMOTE_USER@$REMOTE_HOST" "mkdir -p '$BIN_DIR'"
if [ "$TARGET" = rocknix ]; then
  tar -C "$PUBLISH_DIR" -cf - . | $SSH "$REMOTE_USER@$REMOTE_HOST" "tar -xf - -C '$BIN_DIR'"
else
  $SCP -r "$PUBLISH_DIR"/. "$REMOTE_USER@$REMOTE_HOST:$BIN_DIR/"
fi

RUN_SCRIPT_NAME="$(basename "$RUN_SCRIPT")"
echo "Uploading launcher to $REMOTE_USER@$REMOTE_HOST:$SCRIPT_DIR/$RUN_SCRIPT_NAME..."
$SSH "$REMOTE_USER@$REMOTE_HOST" "mkdir -p '$SCRIPT_DIR'"
$SCP "$RUN_SCRIPT" "$REMOTE_USER@$REMOTE_HOST:$SCRIPT_DIR/$RUN_SCRIPT_NAME"

echo "Setting executable permissions..."
APP_NAME="$(basename "$PROJECT" .csproj)"
$SSH "$REMOTE_USER@$REMOTE_HOST" "chmod +x '$BIN_DIR/$APP_NAME' '$SCRIPT_DIR/$RUN_SCRIPT_NAME' || true"

if [ "$TARGET" = rocknix ]; then
  $SCP ./rocknix/launcher.sh "$REMOTE_USER@$REMOTE_HOST:$BIN_DIR/rocknix-launcher.sh"
  echo "Adding ${RUN_SCRIPT_NAME%.sh} to the ROCKNIX home screen..."
  # The Gauges card starts one app directly, and that is the Rust one.
  ES_SYSTEM=0 rocknix_install_menus "${RUN_SCRIPT_NAME%.sh}" "$ART_DIR" $SSH "$REMOTE_USER@$REMOTE_HOST"
  echo "Done. On the device: Ports -> ${RUN_SCRIPT_NAME%.sh} (log: $BIN_DIR/GaugeDotnet.log)"
  exit 0
fi

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

echo "Done."
echo "Run on device:"
echo "ssh $REMOTE_USER@$REMOTE_HOST"
echo "cd $SCRIPT_DIR && ./$RUN_SCRIPT_NAME"