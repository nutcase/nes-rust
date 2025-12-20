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

pass=0; fail=0; skip=0; unknown=0
include_manual="${RUN_CPU_TESTS_INCLUDE_MANUAL:-0}"
for rom in "${ROM_LIST[@]}"; do
  echo "==> RUN: $rom"
  base="$(basename "$rom")"

  # Some ROMs under roms/tests are manual/visual suites and don't emit APU PASS/FAIL signatures.
  if [[ "$include_manual" != "1" ]]; then
    if [[ "$base" == *"burn-in"* || "$base" == *"burnin"* ]]; then
      echo "[RESULT] SKIP: $rom (manual burn-in test ROM; set RUN_CPU_TESTS_INCLUDE_MANUAL=1 to run)"
      skip=$((skip+1))
      continue
    fi
    if [[ "$base" == *"wrmpyb-in-flight"* ]]; then
      echo "[RESULT] SKIP: $rom (visual test ROM; no APU PASS/FAIL signature)"
      skip=$((skip+1))
      continue
    fi
  fi

  is_cputest=0
  if [[ "$base" == *cputest* || "$base" == *CPUTEST* || "$base" == *65c816* || "$base" == *65C816* ]]; then
    is_cputest=1
  fi

  # Run headless.
  # - Most blargg-style test ROMs print to APU $2140 -> TESTROM_APU_PRINT=1
  # - cputest-full.sfc halts at a known loop after printing "Success"/"Failed" -> CPU_TEST_MODE=1
  set +e
if [[ $is_cputest -eq 1 ]]; then
  OUT=$(CPU_TEST_MODE=1 \
          HEADLESS=1 HEADLESS_FRAMES=${HEADLESS_FRAMES:-2000} QUIET=1 \
          HEADLESS_VIS_CHECK=0 HEADLESS_SUMMARY=0 \
          ALLOW_BAD_CHECKSUM=1 \
          RUSTFLAGS="-Awarnings ${RUSTFLAGS:-}" \
          cargo run --release --quiet --bin snes_emulator -- "$rom" 2>&1)
else
  OUT=$(TESTROM_APU_PRINT=1 HEADLESS=1 HEADLESS_FRAMES=${HEADLESS_FRAMES:-7200} QUIET=1 \
          HEADLESS_VIS_CHECK=0 HEADLESS_SUMMARY=0 \
          ALLOW_BAD_CHECKSUM=1 \
          RUSTFLAGS="-Awarnings ${RUSTFLAGS:-}" \
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
    if [[ $rc -ne 0 ]]; then
      echo "[RESULT] FAIL: $rom (emulator returned exit=$rc)"
      fail=$((fail+1))
    else
      echo "[RESULT] UNKNOWN: $rom (no PASS/FAIL signature)"
      unknown=$((unknown+1))
    fi
  fi
done

echo "\nSummary: PASS=$pass FAIL=$fail SKIP=$skip UNKNOWN=$unknown (total=$((pass+fail+skip+unknown)))"
[[ $fail -eq 0 ]] || exit 1
exit 0
