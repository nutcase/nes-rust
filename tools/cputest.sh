#!/usr/bin/env bash
set -euo pipefail

# Headless runner for roms/tests/cputest-full.sfc (65c816 CPU test ROM).
#
# Usage:
#   tools/cputest.sh [rom_path]
#
# Notes:
# - Relies on the emulator's built-in PASS/FAIL auto-exit (CPU_TEST_MODE=1).
# - Builds once (release) and runs the produced binary to avoid compile-time warnings in logs.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")"/.. && pwd)"
cd "$ROOT_DIR"

ROM_ARG="${1:-roms/tests/cputest-full.sfc}"
if [[ ! -f "$ROM_ARG" ]]; then
  echo "[cputest] ROM not found: $ROM_ARG" >&2
  exit 2
fi

echo "[cputest] Building (release) ..."
if ! cargo build --release >/dev/null 2>&1; then
  # Re-run with output for troubleshooting
  cargo build --release
  exit 1
fi

BIN=./target/release/snes_emulator
if [[ ! -x "$BIN" ]]; then
  echo "[cputest] Build did not produce $BIN" >&2
  exit 1
fi

FRAMES=${HEADLESS_FRAMES:-200000}
TAIL_LINES=${CPUTEST_TAIL_LINES:-80}
TMP_LOG="$(mktemp -t snes-cputest.XXXXXX.log)"
trap '[[ -n "$TMP_LOG" && -f "$TMP_LOG" && -z "${CPUTEST_KEEP_LOG:-}" ]] && rm -f "$TMP_LOG"' EXIT

echo "[cputest] Running headless (${FRAMES} frames max): $ROM_ARG"
set +e
#
# Note: Developers often keep DEBUG_* / TRACE_* env vars in their shell; those can
# dramatically slow headless regressions. Default to a clean env for reproducible speed.
# Set CPUTEST_CLEAN_ENV=0 to inherit the current environment.
if [[ "${CPUTEST_CLEAN_ENV:-1}" == "0" ]]; then
  CPU_TEST_MODE=1 \
  HEADLESS=1 \
  HEADLESS_FAST_RENDER="${HEADLESS_FAST_RENDER:-1}" \
  HEADLESS_FAST_RENDER_LAST="${HEADLESS_FAST_RENDER_LAST:-1}" \
  HEADLESS_FRAMES="${FRAMES}" \
  HEADLESS_SUMMARY=0 \
  QUIET=1 \
  "$BIN" "$ROM_ARG" 2>&1 \
    | tee "$TMP_LOG" \
    | tail -n "$TAIL_LINES"
else
  env -i \
    PATH="$PATH" \
    HOME="${HOME:-/}" \
    CPU_TEST_MODE=1 \
    HEADLESS=1 \
    HEADLESS_FAST_RENDER="${HEADLESS_FAST_RENDER:-1}" \
    HEADLESS_FAST_RENDER_LAST="${HEADLESS_FAST_RENDER_LAST:-1}" \
    HEADLESS_FRAMES="${FRAMES}" \
    HEADLESS_SUMMARY=0 \
    QUIET=1 \
    "$BIN" "$ROM_ARG" 2>&1 \
      | tee "$TMP_LOG" \
      | tail -n "$TAIL_LINES"
fi
status=${PIPESTATUS[0]}
set -e

if [[ $status -eq 0 ]]; then
  echo "[cputest] RESULT: PASS"
  exit 0
fi

echo "[cputest] RESULT: FAIL (exit=$status)" >&2
if [[ -n "${CPUTEST_KEEP_LOG:-}" ]]; then
  echo "[cputest] log kept at $TMP_LOG" >&2
else
  rm -f "$TMP_LOG"
  TMP_LOG=""
fi
exit "$status"
