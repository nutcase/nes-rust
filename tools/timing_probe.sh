#!/usr/bin/env bash
set -euo pipefail

# Timing probe for PPU write windows. Runs the emulator headless with
# STRICT_PPU_TIMING=1 and surfaces any writes skipped due to timing guards.
#
# Usage:
#   tools/timing_probe.sh [rom_path] [frames]
#
# If rom_path is omitted and exactly one ROM exists under ./roms, it is used.
# Frames default to 240.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")"/.. && pwd)"
cd "$ROOT_DIR"

pick_rom() {
  if [[ $# -gt 0 && -n "${1:-}" ]]; then echo "$1"; return; fi
  shopt -s nullglob
  local arr=(roms/*.sfc roms/*.smc)
  shopt -u nullglob
  if [[ ${#arr[@]} -eq 1 ]]; then echo "${arr[0]}"; return; fi
  echo ""; return 1
}

ROM_PATH=$(pick_rom "${1:-}") || {
  echo "Usage: tools/timing_probe.sh <rom_path> [frames]" >&2
  exit 2
}
FRAMES=${2:-240}

echo "[probe] Building (release)…" >&2
cargo build --release >/dev/null

echo "[probe] Running STRICT_PPU_TIMING on: $ROM_PATH (frames=$FRAMES)" >&2
set +e
OUT=$(STRICT_PPU_TIMING=1 DEBUG_PPU_WRITE=1 HEADLESS=1 HEADLESS_FRAMES="$FRAMES" \
      cargo run --release --quiet -- "$ROM_PATH" 2>&1)
status=$?
set -e

echo "$OUT" | tail -n 40

if [[ $status -ne 0 ]]; then
  echo "[probe] Emulator exited with non-zero status ($status)" >&2
fi

# Summarize strict-timing skips
vr_lo=$(echo "$OUT" | rg -n "PPU TIMING: Skip VMDATAL" | wc -l | awk '{print $1}')
vr_hi=$(echo "$OUT" | rg -n "PPU TIMING: Skip VMDATAH" | wc -l | awk '{print $1}')
cg_wr=$(echo "$OUT" | rg -n "PPU TIMING: Skip CGDATA" | wc -l | awk '{print $1}')
oam_wr=$(echo "$OUT" | rg -n "PPU TIMING: Skip OAMDATA" | wc -l | awk '{print $1}')

echo "[probe] Strict timing skips: VRAM(L/H)=${vr_lo}/${vr_hi} CGRAM=${cg_wr} OAM=${oam_wr}"

if (( vr_lo + vr_hi + cg_wr + oam_wr > 0 )); then
  echo "[probe] Sample violations:"
  echo "$OUT" | rg -n "PPU TIMING: Skip" | head -n 10
fi

exit 0

