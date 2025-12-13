#!/usr/bin/env bash
set -euo pipefail

# Simple runner for CPU test ROMs that print status to APU $2140 (blargg style).
# Usage:
#   tools/run_cpu_tests.sh roms/tests/*.sfc
# or
#   tools/run_cpu_tests.sh roms/tests

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")"/.. && pwd)"
cd "$ROOT"

collect() {
  if [[ $# -eq 0 ]]; then echo ""; return; fi
  if [[ -d "$1" ]]; then
    find "$1" -type f \( -name '*.sfc' -o -name '*.smc' \) | sort
  else
    printf '%s\n' "$@"
  fi
}

ROM_LIST=( $(collect "$@") ) || true
if [[ ${#ROM_LIST[@]} -eq 0 ]]; then
  echo "No test ROMs given." >&2
  echo "Usage: tools/run_cpu_tests.sh <rom_dir_or_files>" >&2
  exit 2
fi

pass=0; fail=0; unknown=0
for rom in "${ROM_LIST[@]}"; do
  echo "==> RUN: $rom"
  base="$(basename "$rom")"
  is_cputest=0
  if [[ "$base" == *cputest* || "$base" == *CPUTEST* || "$base" == *65c816* || "$base" == *65C816* ]]; then
    is_cputest=1
  fi

  # Run headless.
  # - Most blargg-style test ROMs print to APU $2140 -> TESTROM_APU_PRINT=1
  # - cputest-full.sfc halts at a known loop after printing "Success"/"Failed" -> CPU_TEST_MODE=1
  set +e
if [[ $is_cputest -eq 1 ]]; then
  OUT=$(CPU_TEST_MODE=1 CPU_TEST_AUTO_FRAMES=${CPU_TEST_AUTO_FRAMES:-2000} \
          HEADLESS=1 HEADLESS_FRAMES=${HEADLESS_FRAMES:-2000} QUIET=1 \
          ALLOW_BAD_CHECKSUM=1 \
          cargo run --release --quiet --bin snes_emulator -- "$rom" 2>&1)
else
  OUT=$(TESTROM_APU_PRINT=1 HEADLESS=1 HEADLESS_FRAMES=${HEADLESS_FRAMES:-7200} QUIET=1 \
          ALLOW_BAD_CHECKSUM=1 \
          cargo run --release --quiet --bin snes_emulator -- "$rom" 2>&1)
fi
  rc=$?
  set -e
  echo "$OUT" | tail -n 50
  if echo "$OUT" | rg -q "\[CPUTEST\] PASS"; then
    echo "[RESULT] PASS: $rom"
    pass=$((pass+1))
  elif echo "$OUT" | rg -q "\[CPUTEST\] FAIL"; then
    echo "[RESULT] FAIL: $rom"
    fail=$((fail+1))
  elif echo "$OUT" | rg -q "\[TESTROM\] PASS"; then
    echo "[RESULT] PASS: $rom"
    pass=$((pass+1))
  elif echo "$OUT" | rg -q "\[TESTROM\] FAIL"; then
    echo "[RESULT] FAIL: $rom"
    fail=$((fail+1))
  else
    echo "[RESULT] UNKNOWN: $rom (no PASS/FAIL signature)"
    unknown=$((unknown+1))
  fi
done

echo "\nSummary: PASS=$pass FAIL=$fail UNKNOWN=$unknown (total=$((pass+fail+unknown)))"
[[ $fail -eq 0 ]] || exit 1
exit 0
