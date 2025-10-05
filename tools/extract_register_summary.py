#!/usr/bin/env python3
"""Extract register summary blocks from emulator log into CSV/JSON for analysis."""
from __future__ import annotations

import argparse
import json
import pathlib
import re

BLOCK_RE = re.compile(r"^━━━━ REGISTER SUMMARY @ Frame (?P<frame>\d+) ━━━━$")
INDENT_RE = re.compile(r"^\s{2}([A-Za-z0-9()\s]+):\s+(.*)$")
VRAM_BUCKET_RE = re.compile(r"^\s{7}0x([0-9A-F]{4}):\s+(\d+) bytes$")


def parse_block(lines: list[str]) -> dict[str, object]:
    data: dict[str, object] = {}
    buckets: dict[str, int] = {}
    state = None
    for line in lines:
        if line.strip().startswith("└─") or line.strip().startswith("┌"):
            continue
        if "Distribution by 4KB blocks" in line:
            state = "vram_blocks"
            continue
        if state == "vram_blocks":
            if VRAM_BUCKET_RE.search(line):
                addr, count = VRAM_BUCKET_RE.findall(line)[0]
                buckets[f"0x{addr}"] = int(count)
                continue
            else:
                state = None
        match = INDENT_RE.match(line)
        if match:
            key, value = match.groups()
            data[key.strip()] = value.strip()
    if buckets:
        data["VRAM block distribution"] = buckets
    return data


def extract(path: pathlib.Path) -> list[dict[str, object]]:
    summaries: list[dict[str, object]] = []
    with path.open("r", encoding="utf-8", errors="ignore") as fh:
        current: list[str] = []
        frame: int | None = None
        for raw in fh:
            line = raw.rstrip("\n")
            match = BLOCK_RE.match(line)
            if match:
                if current and frame is not None:
                    block = parse_block(current)
                    block["frame"] = frame
                    summaries.append(block)
                current = []
                frame = int(match.group("frame"))
            elif frame is not None:
                if line.startswith("━━━━━━━━") and current:
                    block = parse_block(current)
                    block["frame"] = frame
                    summaries.append(block)
                    current = []
                    frame = None
                else:
                    current.append(line)
    return summaries


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("log", type=pathlib.Path)
    parser.add_argument("--json", dest="json_path", type=pathlib.Path)
    args = parser.parse_args()
    summaries = extract(args.log)
    output = json.dumps(summaries, indent=2)
    if args.json_path:
        args.json_path.write_text(output + "\n", encoding="utf-8")
    else:
        print(output)


if __name__ == "__main__":
    main()
