#!/usr/bin/env bash
set -euo pipefail

# Quick APU handshake tracer (ports $2140-$2143).
#
# Usage:
#   tools/apu_handshake_check.sh roms/<game>.sfc [frames]
#
# Examples:
#   tools/apu_handshake_check.sh roms/mario.sfc 240
#   tools/apu_handshake_check.sh roms/dq3.sfc 300

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")"/.. && pwd)"
cd "$ROOT"

ROM="${1:-}"
if [[ -z "$ROM" ]]; then
  echo "Usage: tools/apu_handshake_check.sh <rom_path> [frames]" >&2
  exit 2
fi

FRAMES="${2:-180}"

# Print concise APU handshake reads/writes, plus port0 writes (kick/index).
QUIET=1 \
HEADLESS=1 \
HEADLESS_FRAMES="$FRAMES" \
TRACE_APU_HANDSHAKE=1 \
TRACE_APU_PORT0=1 \
ALLOW_BAD_CHECKSUM=1 \
cargo run --release --quiet --bin snes_emulator -- "$ROM"

