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

# Run without args to get interactive ROM selection from roms/ directory
./target/release/nes-emulator
```

**macOS SDL2 Setup:** Install SDL2 via Homebrew (`brew install sdl2`). The `run.sh` script sets up the required library paths (`LIBRARY_PATH`, `DYLD_LIBRARY_PATH` for `/opt/homebrew/lib`).

## Architecture Overview

This is a cycle-accurate NES (Famicom) emulator written in Rust using SDL2 for rendering and audio.

### Emulation Loop

The main emulation loop lives in `Nes::step()` (`src/main.rs`). Each step:
1. Checks for DMA in progress (OAM DMA takes priority over CPU)
2. Executes one CPU instruction via `Cpu::step()`
3. Runs PPU for `cpu_cycles * 3` cycles (PPU clock is 3x CPU clock)
4. Fires NMI to CPU if PPU triggered it during VBlank
5. Checks APU frame IRQ
6. Counts cycles; returns `true` when a full frame is complete (29,830 CPU cycles)

### Core Components

```
Nes (main.rs)  ──>  Bus (bus.rs)  ──>  CPU, PPU, APU, Memory, Cartridge
```

- **CPU** (`src/cpu/mod.rs`): 6502 processor with all official + unofficial opcodes. ~2600 lines. Uses `CpuBus` trait (defined at bottom of file) for memory access. Status flags use `bitflags!` crate (`StatusFlags`).
- **PPU** (`src/ppu/mod.rs`): Monolithic implementation with Loopy scrolling registers (`v`, `t`, `x`, `w`). The `ppu/` directory also contains an unused refactored version split into submodules (`registers.rs`, `memory.rs`, `background.rs`, `sprites.rs`, `renderer.rs`) via `new_mod.rs`/`complex_mod.rs` — the active code is all in `mod.rs`. Fields are `pub` under `#[cfg(test)]` for test access.
- **APU** (`src/apu/mod.rs`): Pulse (x2), triangle, noise channels. Outputs at 44.1kHz with high-pass and low-pass filters. Audio is buffered and consumed by SDL2 callback via `Arc<Mutex<Vec<f32>>>`.
- **Bus** (`src/bus.rs`): Central interconnect implementing `CpuBus` trait. Handles memory-mapped I/O routing, OAM DMA, and controller reads.
- **Cartridge** (`src/cartridge/mod.rs`): iNES ROM loading with thin dispatch methods. Mapper implementations live in `src/cartridge/mapper/` (nrom.rs, mmc1.rs, uxrom.rs, cnrom.rs). Supports mappers 0/NROM, 1/MMC1, 2/UxROM, 3/CNROM, 87.
- **Memory** (`src/memory/mod.rs`): 2KB RAM with mirroring. Simple read/write with address masking (`addr & 0x7FF`).

### Key Traits

- `CpuBus` (`src/cpu/mod.rs:2592`): Primary interface between CPU and bus — `read()`, `write()`, plus game-specific protection hooks.
- `CpuBusWithTick`: Extension trait adding `tick()` for fine-grained PPU synchronization (defined but not widely used).

### Memory Map

- `$0000-$1FFF`: 2KB RAM (mirrored every 2KB)
- `$2000-$2007`: PPU registers (mirrored through `$3FFF`)
- `$4000-$4017`: APU and Controller registers
- `$6000-$7FFF`: PRG-RAM (cartridge battery-backed SRAM)
- `$8000-$FFFF`: PRG-ROM (cartridge, bank-switched by mapper)

### Persistence

- **Save states** (`src/save_state.rs`): Serialized with serde/bincode to `save_state_{slot}.sav` files.
- **SRAM** (`src/sram.rs`): Battery-backed saves persisted to `.sav` files next to the ROM. Auto-saved every 30 seconds (1800 frames).

### Testing

CPU tests use a `TestBus` mock (`src/cpu/tests.rs`) that implements `CpuBus` with flat 64KB memory. PPU tests access internal state via `#[cfg(test)]` public fields. Test files:
- `src/cpu/tests.rs`, `src/cpu/addressing_tests.rs`, `src/cpu/additional_tests.rs`
- `src/ppu/tests.rs`, `src/ppu/additional_tests.rs`

### Controls

- **D-Pad**: Arrow keys
- **A/B**: Z / X
- **Start/Select**: Enter / Space
- **Save State**: Ctrl+1-4
- **Load State**: 1-4
