#!/usr/bin/env bash
set -euo pipefail

# Headless runner for roms/tests/burn-in-test.sfc (Nintendo burn-in test).
#
# Usage:
#   tools/burnin.sh [rom_path]
#
# This script:
# - Boots the ROM
# - Navigates to menu item 5 (BURN-IN TEST) via AUTO_INPUT_EVENTS
# - Runs for a fixed number of frames
# - Dumps the final framebuffer (PPM) and checks that the PASS/FAIL column contains no red pixels

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")"/.. && pwd)"
cd "$ROOT_DIR"

ROM_ARG="${1:-roms/tests/burn-in-test.sfc}"
if [[ ! -f "$ROM_ARG" ]]; then
  echo "[burnin] ROM not found: $ROM_ARG" >&2
  exit 2
fi

echo "[burnin] Building (release) ..."
if ! cargo build --release >/dev/null 2>&1; then
  # Re-run with output for troubleshooting
  cargo build --release
  exit 1
fi

BIN=./target/release/snes_emulator
if [[ ! -x "$BIN" ]]; then
  echo "[burnin] Build did not produce $BIN" >&2
  exit 1
fi

# Note: With more accurate HV latch/APU timing, the full burn-in suite can take
# several thousand frames to finish. Default to a higher value so the final
# framebuffer is likely to be the results screen.
FRAMES=${HEADLESS_FRAMES:-8000}
TAIL_LINES=${BURNIN_TAIL_LINES:-60}
TMP_LOG="$(mktemp -t snes-burnin.XXXXXX.log)"
trap '[[ -n "$TMP_LOG" && -f "$TMP_LOG" && -z "${BURNIN_KEEP_LOG:-}" ]] && rm -f "$TMP_LOG"' EXIT

# Menu navigation:
# - SELECT cycles through items 1..5
# - START begins the selected test
AUTO_EVENTS=${AUTO_INPUT_EVENTS:-'10-12:SELECT;30-32:SELECT;50-52:SELECT;70-72:SELECT;90-110:START'}

rm -f logs/headless_fb.ppm

echo "[burnin] Running headless (${FRAMES} frames): $ROM_ARG"
set +e
AUTO_INPUT_EVENTS="$AUTO_EVENTS" \
HEADLESS=1 \
HEADLESS_FAST_RENDER="${HEADLESS_FAST_RENDER:-1}" \
HEADLESS_FAST_RENDER_LAST="${HEADLESS_FAST_RENDER_LAST:-1}" \
HEADLESS_FRAMES="${FRAMES}" \
HEADLESS_DUMP_FRAME=1 \
HEADLESS_SUMMARY=0 \
QUIET=1 \
"$BIN" "$ROM_ARG" 2>&1 \
  | tee "$TMP_LOG" \
  | tail -n "$TAIL_LINES"
status=${PIPESTATUS[0]}
set -e

if [[ $status -ne 0 ]]; then
  echo "[burnin] FAIL: emulator returned non-zero exit ($status)" >&2
  exit "$status"
fi

if [[ ! -f logs/headless_fb.ppm ]]; then
  echo "[burnin] FAIL: framebuffer dump not found (logs/headless_fb.ppm)" >&2
  exit 1
fi

python3 - <<'PY'
from pathlib import Path

path = Path("logs/headless_fb.ppm")
data = path.read_bytes()
if data[:2] != b"P6":
    raise SystemExit("[burnin] FAIL: not a P6 PPM")

i = data.find(b"\n") + 1
tokens = []
while len(tokens) < 3:
    if data[i:i+1] == b"#":
        i = data.find(b"\n", i) + 1
        continue
    while data[i:i+1].isspace():
        i += 1
    j = i
    while j < len(data) and not data[j:j+1].isspace():
        j += 1
    tokens.append(data[i:j].decode())
    i = j

w, h, _maxv = map(int, tokens)
i = data.find(b"\n", i) + 1
pix = data[i:]
if len(pix) < w * h * 3:
    raise SystemExit("[burnin] FAIL: truncated pixel data")

def get(x: int, y: int):
    idx = (y * w + x) * 3
    return pix[idx], pix[idx + 1], pix[idx + 2]

# The PASS/FAIL status column is on the right side. Sample a fixed region that:
# - covers all rows (including the early "DMA MEMORY" row), and
# - excludes the bottom sprite strip (Mario row).
x0, x1 = 200, 255
y0, y1 = 40, 200
red = green = other = 0
for y in range(y0, y1):
    for x in range(x0, x1):
        r, g, b = get(x, y)
        if r == 0 and g == 0 and b == 0:
            continue
        if r > 200 and g < 80 and b < 80:
            red += 1
        elif g > 200 and r < 80 and b < 80:
            green += 1
        else:
            other += 1

print(f"[burnin] status region: red={red} green={green} other={other}")
if green == 0:
    raise SystemExit("[burnin] FAIL: no green PASS pixels detected (did not reach results screen?)")
if red != 0:
    raise SystemExit("[burnin] FAIL: red FAIL pixels detected")
print("[burnin] RESULT: PASS")
PY

if [[ -n "${BURNIN_KEEP_LOG:-}" ]]; then
  echo "[burnin] log kept at $TMP_LOG"
else
  rm -f "$TMP_LOG"
  TMP_LOG=""
fi
