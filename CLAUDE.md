# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build and Run Commands

```bash
# Build release version
cargo build --release

# Run tests
cargo test

# Run a single test
cargo test <test_name>

# Build and run (macOS with Homebrew SDL2)
./run.sh

# Run with a specific ROM
./target/release/nes-emulator <path-to-rom.nes>
```

**macOS SDL2 Setup:** Install SDL2 via Homebrew (`brew install sdl2`). The `run.sh` script sets up the required library paths.

## Architecture Overview

This is a cycle-accurate NES (Famicom) emulator written in Rust.

### Core Components

```
┌─────────────────────────────────────────────────────┐
│  Nes (main.rs)                                      │
│  Entry point, SDL2 rendering, input, save states    │
└──────────────────────┬──────────────────────────────┘
                       │
                       ▼
              ┌────────────────┐
              │  Bus (bus.rs)  │  Central interconnect
              ├────────────────┤
              │ Memory  (2KB)  │
              │ PPU            │
              │ APU            │
              │ Cartridge      │
              │ Controller     │
              └────────────────┘
```

- **CPU** (`src/cpu/mod.rs`): 6502 processor with full instruction set including unofficial opcodes. Implements `CpuBus` trait for memory access.
- **PPU** (`src/ppu/`): Picture Processing Unit with Loopy scrolling registers, sprite evaluation, and background rendering. Runs at 3x CPU clock rate.
- **APU** (`src/apu/mod.rs`): Audio with pulse, triangle, and noise channels. Outputs at 44.1kHz.
- **Bus** (`src/bus.rs`): Memory-mapped I/O connecting all components. Handles DMA transfers.
- **Cartridge** (`src/cartridge/mod.rs`): ROM loading and mapper implementations (0, 1, 2, 3, 87).

### Memory Map

- `$0000-$1FFF`: 2KB RAM (mirrored)
- `$2000-$2007`: PPU registers
- `$4000-$4017`: APU/Controller registers
- `$6000-$7FFF`: PRG-RAM (cartridge save)
- `$8000-$FFFF`: PRG-ROM

### Timing

- CPU cycles per frame: 29,830
- PPU cycles = CPU cycles × 3
- Target: 60 FPS

### Key Traits

- `CpuBus`: Interface between CPU and bus for memory read/write operations

## Controls

- **D-Pad**: Arrow keys
- **A Button**: Z
- **B Button**: X
- **Start**: Enter
- **Select**: Space
- **Save State**: Ctrl+1-4
- **Load State**: 1-4
