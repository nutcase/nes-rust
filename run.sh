#!/bin/bash
export LIBRARY_PATH=/opt/homebrew/lib:$LIBRARY_PATH
export C_INCLUDE_PATH=/opt/homebrew/include:$C_INCLUDE_PATH
export DYLD_LIBRARY_PATH=/opt/homebrew/lib:$DYLD_LIBRARY_PATH

# Disable IME to avoid macOS error messages
export LANG=C
export LC_ALL=C
export SDL_DISABLE_IMMINTRIN_H=1

# Build the cheat-ui example
cargo build --release --example nes_emulator --features cheat-ui

# Run with ROM selection (pass any arguments through)
./target/release/examples/nes_emulator "$@"