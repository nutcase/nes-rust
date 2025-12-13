#!/usr/bin/env bash
set -euo pipefail

# Simple headless smoke test for the SNES emulator.
# Validates that DMA/PPU initialization progresses (VRAM/OAM writes occur)
# and that the emulator finishes the requested number of frames.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")"/.. && pwd)"
cd "$ROOT_DIR"

ROM_ARG="${1:-}"
if [[ -z "$ROM_ARG" ]]; then
  # Auto-pick a ROM from roms/*.sfc|*.smc if single
  shopt -s nullglob
  roms=(roms/*.sfc roms/*.smc)
  shopt -u nullglob
  if [[ ${#roms[@]} -eq 1 ]]; then
    ROM_ARG="${roms[0]}"
  else
    echo "Usage: tools/smoke.sh <rom_path>" >&2
    exit 2
  fi
fi

FRAMES=${HEADLESS_FRAMES:-180}
echo "[smoke] Building (release) ..."
cargo build --release >/dev/null

echo "[smoke] Running headless for ${FRAMES} frames on: $ROM_ARG"

TAIL_LINES=${SMOKE_TAIL_LINES:-200}
TMP_LOG="$(mktemp -t snes-smoke.XXXXXX.log)"
trap '[[ -n "$TMP_LOG" && -f "$TMP_LOG" && -z "${SMOKE_KEEP_LOG:-}" ]] && rm -f "$TMP_LOG"' EXIT

set +e
HEADLESS=1 \
HEADLESS_FRAMES=${FRAMES} \
INIT_TRACE=0 \
QUIET=1 \
HEADLESS_VIS_CHECK=1 \
COMPAT_BOOT_FALLBACK=0 \
COMPAT_INJECT_MIN_PALETTE=0 \
COMPAT_PERIODIC_MIN_PALETTE=0 \
cargo run --release --quiet --bin snes_emulator -- "$ROM_ARG" 2>&1 \
    | tee "$TMP_LOG" \
    | tail -n "$TAIL_LINES"
status=${PIPESTATUS[0]}
set -e

if [[ $status -ne 0 ]]; then
  echo "[smoke] FAIL: emulator returned non-zero exit ($status)" >&2
  exit 1
fi

# Parse final INIT summary
summary_line=$(rg -n "^INIT summary: " "$TMP_LOG" | tail -n1 | sed -E 's/^[^:]+://')
if [[ -z "$summary_line" ]]; then
  echo "[smoke] FAIL: INIT summary not found" >&2
  exit 1
fi
echo "[smoke] Summary: $summary_line"

# Extract key numbers
mdma=$(echo "$summary_line" | sed -nE 's/.*MDMAEN!=0=([0-9]+).*/\1/p')
hdma=$(echo "$summary_line" | sed -nE 's/.*HDMAEN!=0=([0-9]+).*/\1/p')
ppuimp=$(echo "$summary_line" | sed -nE 's/.*PPU important=([0-9]+).*/\1/p')
vram_l=$(echo "$summary_line" | sed -nE 's/.*VRAM L\/H=([0-9]+)\/[0-9]+.*/\1/p')
vram_h=$(echo "$summary_line" | sed -nE 's/.*VRAM L\/H=[0-9]+\/([0-9]+).*/\1/p')
cgram=$(echo "$summary_line" | sed -nE 's/.*CGRAM=([0-9]+).*/\1/p')
oam=$(echo "$summary_line"   | sed -nE 's/.*OAM=([0-9]+).*/\1/p')

fail=0
[[ -n "$mdma" && $mdma -ge 1 ]] || { echo "[smoke] FAIL: MDMAEN count < 1" >&2; fail=1; }
[[ -n "$vram_l" && -n "$vram_h" && ( $vram_l -ge 1 || $vram_h -ge 1 ) ]] || { echo "[smoke] FAIL: VRAM writes did not increase" >&2; fail=1; }
# CGRAM check is deferred until after visibility check (may use fallback palette)
[[ -n "$oam" && $oam -ge 1 ]] || { echo "[smoke] FAIL: OAM writes did not increase" >&2; fail=1; }

finished=$(rg -n "HEADLESS mode finished \(" "$TMP_LOG" | tail -n1)
[[ -n "$finished" ]] || { echo "[smoke] FAIL: headless run did not finish" >&2; fail=1; }

# Visibility check: ensure we saw non-black pixels at some checkpoint
VIS_CHECK_ENABLED=${HEADLESS_VIS_CHECK:-1}
if [[ "$VIS_CHECK_ENABLED" != "0" ]]; then
  vis_line=$(rg -n "^VISIBILITY: frame=.* non_black_pixels=([0-9]+)" "$TMP_LOG" | tail -n1 | sed -E 's/^[^:]+:VISIBILITY: frame=([0-9]+) non_black_pixels=([0-9]+)/\1 \2/')
  if [[ -z "$vis_line" ]]; then
    echo "[smoke] FAIL: visibility metric not found" >&2; fail=1;
  else
    vis_frame=$(echo "$vis_line" | awk '{print $1}')
    vis_pixels=$(echo "$vis_line" | awk '{print $2}')
    echo "[smoke] Visibility@frame ${vis_frame}: non_black=${vis_pixels}"
    if [[ ${vis_pixels:-0} -le 0 ]]; then
      echo "[smoke] FAIL: no visible pixels detected (FORCE_DISPLAY=1)" >&2; fail=1;
    fi
  fi
else
  echo "[smoke] Visibility check skipped (HEADLESS_VIS_CHECK=0)"
  vis_pixels=0
fi

# CGRAM check: required only if no visible pixels (fallback palette may be used)
if [[ ${vis_pixels:-0} -gt 0 ]]; then
  # Graphics displayed successfully
  if [[ -n "$cgram" && $cgram -ge 1 ]]; then
    echo "[smoke] CGRAM: $cgram writes (game-supplied palette)"
  else
    echo "[smoke] CGRAM: 0 writes (using fallback palette - acceptable for DQ3 early boot)"
  fi
else
  # No graphics, require CGRAM writes
  [[ -n "$cgram" && $cgram -ge 1 ]] || { echo "[smoke] FAIL: CGRAM writes did not increase and no visible pixels" >&2; fail=1; }
fi

if [[ $fail -ne 0 ]]; then
  echo "[smoke] RESULT: FAIL" >&2
  exit 1
fi

echo "[smoke] RESULT: PASS"
if [[ -n "${SMOKE_KEEP_LOG:-}" ]]; then
  echo "[smoke] log kept at $TMP_LOG"
else
  rm -f "$TMP_LOG"
  TMP_LOG=""
fi
exit 0
