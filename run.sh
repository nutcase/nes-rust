#!/bin/bash
export LIBRARY_PATH=/opt/homebrew/lib:$LIBRARY_PATH
export C_INCLUDE_PATH=/opt/homebrew/include:$C_INCLUDE_PATH

# Disable IME to avoid macOS error messages
export LANG=C
export LC_ALL=C
export SDL_DISABLE_IMMINTRIN_H=1

# Build the project first
cargo build --release

# Run with ROM selection
./target/release/nes-emulator