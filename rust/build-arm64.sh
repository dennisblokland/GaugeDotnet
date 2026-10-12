#!/usr/bin/env bash
# Cross-builds the device app (SDL window + Bluetooth) for the RG35XX (aarch64).
#
# Links against the arm64 libSDL2 from Ubuntu's archive (downloaded once into
# target/sdl-arm64). At run time the binary uses the device's own libSDL2,
# exactly like the C# app; libdbus is compiled in (ble-vendored), so nothing
# else is needed beyond libc.
#
# Needs: rustup target aarch64-unknown-linux-gnu, gcc-aarch64-linux-gnu, curl, dpkg-deb.
set -euo pipefail
cd "$(dirname "$0")"

TARGET=aarch64-unknown-linux-gnu
SDL_DEB_URL="${SDL_DEB_URL:-http://ports.ubuntu.com/ubuntu-ports/pool/main/libs/libsdl2/libsdl2-2.0-0_2.30.0+dfsg-1ubuntu3.1_arm64.deb}"
SDL_DIR="$PWD/target/sdl-arm64"
SDL_LIB="$SDL_DIR/lib"

if [ ! -f "$SDL_LIB/libSDL2.so" ]; then
  echo "Fetching arm64 libSDL2 to link against..."
  mkdir -p "$SDL_DIR/deb" "$SDL_LIB"
  curl -fsSL -o "$SDL_DIR/libsdl2.deb" "$SDL_DEB_URL"
  dpkg-deb -x "$SDL_DIR/libsdl2.deb" "$SDL_DIR/deb"
  so="$(find "$SDL_DIR/deb" -name 'libSDL2-2.0.so.0*' -type f | head -n1)"
  cp "$so" "$SDL_LIB/libSDL2-2.0.so.0"
  ln -sf libSDL2-2.0.so.0 "$SDL_LIB/libSDL2.so"
fi

export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER="${CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER:-aarch64-linux-gnu-gcc}"
export CC_aarch64_unknown_linux_gnu="${CC_aarch64_unknown_linux_gnu:-aarch64-linux-gnu-gcc}"
# libSDL2's own dependencies (X11, Wayland, ...) exist only on the device, so
# allow them to stay unresolved here. The rpath lets a libSDL2 placed next to
# the binary take precedence if the device ever lacks one.
export RUSTFLAGS="${RUSTFLAGS:-} -L $SDL_LIB -C link-arg=-Wl,--allow-shlib-undefined -C link-arg=-Wl,-rpath,\$ORIGIN"

cargo build --release --target "$TARGET" -p gauge-app --features sdl,ble-vendored

OUT="$PWD/target/$TARGET/release/gauge"
echo "Built $OUT"
