#!/bin/sh
set -eu
: "${WEBOS_SDK:?Set WEBOS_SDK to the relocated native webOS SDK}"
export CARGO_TARGET_ARMV7_UNKNOWN_LINUX_GNUEABI_LINKER="$WEBOS_SDK/bin/arm-webos-linux-gnueabi-gcc"
if [ ! -x "$CARGO_TARGET_ARMV7_UNKNOWN_LINUX_GNUEABI_LINKER" ]; then
  echo 'Missing webOS ARM soft-float cross compiler' >&2
  exit 1
fi
cd "$(dirname "$0")"
"${CARGO:-cargo}" test --locked
"${CARGO:-cargo}" build --locked --release --target armv7-unknown-linux-gnueabi --features sdl
"$WEBOS_SDK/bin/arm-webos-linux-gnueabi-readelf" -h target/armv7-unknown-linux-gnueabi/release/aerial-native-probe
