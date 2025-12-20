#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")"/.. && pwd)"
cd "$ROOT_DIR"

ROM_ARG="${1:-roms/dq3.sfc}"
if [[ ! -f "$ROM_ARG" ]]; then
  echo "[mode7] ROM not found: $ROM_ARG" >&2
  exit 2
fi

echo "[mode7] Building (release) ..."
cargo build --release >/dev/null

echo "[mode7] Running Mode 7 diagnostic (60 frames)"
set +e
OUT=$(HEADLESS=1 HEADLESS_FRAMES=60 MODE7_TEST=1 FORCE_DISPLAY=1 DEBUG_RENDER_METRICS=1 \
      cargo run --release --quiet --bin snes_emulator -- "$ROM_ARG" 2>&1)
status=$?
set -e

echo "$OUT" | rg -n "MODE7_TEST|M7_|RENDER_METRICS|PPU usage" -n || true

if [[ $status -ne 0 ]]; then
  echo "[mode7] FAIL (exit $status)" >&2
  exit $status
fi

echo "[mode7] DONE"
exit 0
