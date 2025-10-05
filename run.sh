#!/usr/bin/env bash
set -euo pipefail

# Simple launcher for the SNES emulator.
# Usage:
#   ./run.sh [rom_file_or_path]
# If no argument is given and exactly one ROM exists in ./roms with .sfc/.smc,
# that ROM will be used automatically. Otherwise, a usage hint is printed.

project_root_dir() {
  # Resolve to project root (directory containing this script)
  cd "$(dirname "${BASH_SOURCE[0]}")" >/dev/null 2>&1
  pwd
}

ROOT_DIR=$(project_root_dir)
cd "$ROOT_DIR"

pick_rom_if_needed() {
  # If a non-empty argument is provided, use it as-is
  if [[ $# -gt 0 && -n "${1}" ]]; then
    echo "$1"
    return 0
  fi

  shopt -s nullglob
  # Collect ROM candidates from roms/ with typical extensions
  local candidates=(roms/*.sfc roms/*.smc)
  shopt -u nullglob

  if [[ ${#candidates[@]} -eq 0 ]]; then
    echo "" 1>&2
    echo "No ROM found under ./roms (expected *.sfc or *.smc)." 1>&2
    echo "Usage: ./run.sh <rom_filename_or_path>" 1>&2
    exit 1
  elif [[ ${#candidates[@]} -eq 1 ]]; then
    echo "${candidates[0]}"
  else
    echo "" 1>&2
    echo "Multiple ROMs found. Please specify one explicitly:" 1>&2
    for f in "${candidates[@]}"; do
      echo "  $f" 1>&2
    done
    echo "" 1>&2
    echo "Usage: ./run.sh <rom_filename_or_path>" 1>&2
    exit 1
  fi
}

if [[ $# -gt 0 ]]; then
  ROM_ARG=$(pick_rom_if_needed "$1")
else
  ROM_ARG=$(pick_rom_if_needed)
fi

echo "Building emulator (release)..."
cargo build --release

BIN=./target/release/snes_emulator
if [[ ! -x "$BIN" ]]; then
  echo "Build did not produce $BIN" 1>&2
  exit 1
fi

# Prepare logs directory and logfile
mkdir -p logs
ts=$(date +"%Y%m%d_%H%M%S")
LOGFILE="logs/run_${ts}.log"

echo "Running: $BIN $ROM_ARG"
echo "Logging to: $LOGFILE (and console via tee)"

# Default: suppress heavy init trace unless explicitly enabled
export INIT_TRACE=${INIT_TRACE:-0}

if [[ -n "${QUIET:-}" ]]; then
  echo "QUIET mode enabled: filtering noisy log lines" >&2
  # Default noisy patterns to drop (can override/extend via QUIET_PATTERNS)
  # QUIET levels:
  # 1 (default): core noise only
  # 2: +PPU register chatter
  # 3: +all remaining debug (very quiet)
  level=${QUIET:-1}
  case "$level" in
    1) DEFAULT_QUIET_PATTERNS='^WARNING: read_u24|^JML from NMI|^NMI |^\\$4200 ' ;;
    2) DEFAULT_QUIET_PATTERNS='^WARNING: read_u24|^JML from NMI|^NMI |^\\$4200 |^PPU: ' ;;
    *) DEFAULT_QUIET_PATTERNS='^.|^$' ;; # drop almost everything
  esac
  # Category toggles (1 = drop, 0 = keep). Defaults drop most noisy.
  QUIET_DMA=${QUIET_DMA:-1}
  QUIET_PPU=${QUIET_PPU:-1}
  QUIET_MAPPER=${QUIET_MAPPER:-1}
  QUIET_TRACE=${QUIET_TRACE:-1}
  if [[ "$QUIET_DMA" == "1" ]]; then
    DEFAULT_QUIET_PATTERNS="$DEFAULT_QUIET_PATTERNS|^DMA Transfer|^MDMAEN set"
  fi
  if [[ "$QUIET_PPU" == "1" ]]; then
    DEFAULT_QUIET_PATTERNS="$DEFAULT_QUIET_PATTERNS|^PPU: "
  fi
  if [[ "$QUIET_MAPPER" == "1" ]]; then
    DEFAULT_QUIET_PATTERNS="$DEFAULT_QUIET_PATTERNS|^Mapper score"
  fi
  if [[ "$QUIET_TRACE" == "1" ]]; then
    DEFAULT_QUIET_PATTERNS="$DEFAULT_QUIET_PATTERNS|^TRACE|^ADDR\["
  fi
  QUIET_PATTERNS_REGEX=${QUIET_PATTERNS:-$DEFAULT_QUIET_PATTERNS}
  # Run emulator, filter, tee
  "$BIN" "$ROM_ARG" 2>&1 \
    | rg -v -e "$QUIET_PATTERNS_REGEX" \
    | tee "$LOGFILE"
else
  # Run emulator and tee output to logfile
  "$BIN" "$ROM_ARG" 2>&1 | tee "$LOGFILE"
fi

# Keep a handy symlink to the latest log
ln -sf "$(basename "$LOGFILE")" logs/latest.log
