#!/usr/bin/env bash
set -euo pipefail

PROJECT="${1:-./src/GaugeDotnet/GaugeDotnet.csproj}"
REMOTE_HOST="${REMOTE_HOST:-192.168.0.234}"
REMOTE_USER="${REMOTE_USER:-root}"
REMOTE_PASS="${REMOTE_PASS:-root}"
BIN_ROOT="${BIN_ROOT:-/mnt/mmc/ports}"
BIN_DIR="${BIN_DIR:-$BIN_ROOT/GaugeDotnet}"
SCRIPT_DIR="${SCRIPT_DIR:-/mnt/mmc/roms/ports}"
CONFIG="${CONFIG:-Release}"
RUN_SCRIPT="${RUN_SCRIPT:-./GaugeDotnet.sh}"

PUBLISH_DIR="./publish-arm64"

# Use sshpass for non-interactive password auth
SSH="sshpass -p $REMOTE_PASS ssh -o StrictHostKeyChecking=no"
SCP="sshpass -p $REMOTE_PASS scp -o StrictHostKeyChecking=no"

echo "Building $PROJECT for linux-arm64..."
rm -rf "$PUBLISH_DIR"

dotnet publish "$PROJECT" \
  -c "$CONFIG" \
  -r linux-arm64 \
  --self-contained true \
  -o "$PUBLISH_DIR" \
  /p:PublishSingleFile=false

if [ ! -f "$RUN_SCRIPT" ]; then
  echo "Run script not found: $RUN_SCRIPT"
  exit 1
fi

echo "Uploading binaries to $REMOTE_USER@$REMOTE_HOST:$BIN_DIR..."
$SSH "$REMOTE_USER@$REMOTE_HOST" "mkdir -p '$BIN_DIR'"
$SCP -r "$PUBLISH_DIR"/. "$REMOTE_USER@$REMOTE_HOST:$BIN_DIR/"

RUN_SCRIPT_NAME="$(basename "$RUN_SCRIPT")"
echo "Uploading launcher to $REMOTE_USER@$REMOTE_HOST:$SCRIPT_DIR/$RUN_SCRIPT_NAME..."
$SSH "$REMOTE_USER@$REMOTE_HOST" "mkdir -p '$SCRIPT_DIR'"
$SCP "$RUN_SCRIPT" "$REMOTE_USER@$REMOTE_HOST:$SCRIPT_DIR/$RUN_SCRIPT_NAME"

echo "Setting executable permissions..."
APP_NAME="$(basename "$PROJECT" .csproj)"
$SSH "$REMOTE_USER@$REMOTE_HOST" "chmod +x '$BIN_DIR/$APP_NAME' '$SCRIPT_DIR/$RUN_SCRIPT_NAME' || true"

echo "Done."
echo "Run on device:"
echo "ssh $REMOTE_USER@$REMOTE_HOST"
echo "cd $SCRIPT_DIR && ./$RUN_SCRIPT_NAME"