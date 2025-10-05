#!/usr/bin/env python3
"""Quick VRAM summary tool.

Usage:
    python tools/vram_summary.py dumps/snapshot.vram

Prints non-zero ranges and bucketed counts so we can eyeball whether
VRAM banks contain meaningful tile data.
"""
from __future__ import annotations

import argparse
import pathlib
from collections import defaultdict

CHUNK = 0x200  # 512 bytes (256 words)


def summarise(data: bytes) -> None:
    total = len(data)
    if total % CHUNK:
        print(f"warning: VRAM dump length {total:#x} not multiple of chunk {CHUNK:#x}")

    # Track non-zero regions and simple histogram per 4KB
    non_zero_ranges = []
    start = None
    for i, b in enumerate(data):
        if b and start is None:
            start = i
        elif not b and start is not None:
            non_zero_ranges.append((start, i))
            start = None
    if start is not None:
        non_zero_ranges.append((start, len(data)))

    h = defaultdict(int)
    for i in range(0, len(data), CHUNK):
        chunk = data[i : i + CHUNK]
        if any(chunk):
            h[i // 0x800] += 1  # bucket per 2KB words (~tile page)

    print(f"size: {total} bytes")
    if not non_zero_ranges:
        print("all zeros")
        return

    print("first non-zero ranges (up to 8):")
    for start, end in non_zero_ranges[:8]:
        span = end - start
        print(f"  {start:#06x} .. {end:#06x} ({span} bytes)")

    print("bucketed 2KB pages with data:")
    for page in sorted(h):
        print(f"  page {page:#04x}: {h[page]} non-zero 512B chunks")


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("vram", type=pathlib.Path)
    args = ap.parse_args()
    data = args.vram.read_bytes()
    summarise(data)


if __name__ == "__main__":
    main()
