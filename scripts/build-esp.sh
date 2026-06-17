#!/usr/bin/env bash
# build-esp.sh — build m5Tui for the Cardputer-Adv (ESP32-S3).
#
# Usage:
#   ./scripts/build-esp.sh                # release build
#   ./scripts/build-esp.sh --debug        # debug build
#   ./scripts/build-esp.sh --flash PORT   # build + flash to PORT (/dev/ttyACM0)
#   ./scripts/build-esp.sh --monitor PORT # build + flash + open serial monitor
#
# Requires espup (https://github.com/esp-rs/espup) installed and sourced:
#   espup install
#   . $HOME/export-esp.sh
# On a fresh machine run this once before building.

set -euo pipefail

# Resolve the repo root regardless of where the script is invoked.
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$REPO_ROOT"

PROFILE="release"
FLASH_PORT=""
MONITOR_PORT=""
while [ $# -gt 0 ]; do
  case "$1" in
    --debug) PROFILE="debug" ;;
    --flash) FLASH_PORT="${2:-}"; shift ;;
    --monitor) MONITOR_PORT="${2:-}"; shift ;;
    -h|--help)
      sed -n '2,16p' "$0"; exit 0 ;;
    *) echo "unknown arg: $1" >&2; exit 1 ;;
  esac
  shift
done

# Make espup's toolchain visible if the caller hasn't sourced it.
if [ -f "$HOME/export-esp.sh" ]; then
  # shellcheck disable=SC1091
  . "$HOME/export-esp.sh"
fi

# Ensure the target is installed (espup installs it, but a developer
# may have sourced the file before the first build).
if ! rustup target list --installed | grep -q 'xtensa-esp32s3-espidf'; then
  rustup target add xtensa-esp32s3-espidf
fi

# Build.
FEATURES="--features m5tui-device/device"
case "$PROFILE" in
  release) CARGO_FLAGS="--release" ;;
  debug)   CARGO_FLAGS="" ;;
  *) echo "bad profile: $PROFILE" >&2; exit 1 ;;
esac

echo "==> cargo check $CARGO_FLAGS $FEATURES --target xtensa-esp32s3-espidf -p m5tui-bin"
cargo check $CARGO_FLAGS $FEATURES --target xtensa-esp32s3-espidf -p m5tui-bin
if [ -n "$FLASH_PORT" ]; then
  echo "==> cargo build $CARGO_FLAGS $FEATURES --target xtensa-esp32s3-espidf -p m5tui-bin"
  cargo build $CARGO_FLAGS $FEATURES --target xtensa-esp32s3-espidf -p m5tui-bin
  BIN="$REPO_ROOT/target/xtensa-esp32s3-espidf/$PROFILE/m5tui-bin"
  if [ ! -f "$BIN" ]; then
    echo "ERROR: built binary not found at $BIN" >&2
    exit 1
  fi
  echo "==> espflash flash $FLASH_PORT $BIN"
  espflash flash "$FLASH_PORT" "$BIN"
  if [ -n "$MONITOR_PORT" ]; then
    echo "==> espflash monitor $MONITOR_PORT"
    exec espflash monitor "$MONITOR_PORT"
  fi
fi

echo "OK. Built for xtensa-esp32s3-espidf ($PROFILE)."
