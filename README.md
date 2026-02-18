# NES Emulator

A cycle-accurate NES (Nintendo Entertainment System / Famicom) emulator written in Rust.

## Features

- Cycle-accurate CPU (6502) and PPU emulation
- Audio support with pulse, triangle, and noise channels
- Multiple mapper support (0, 1, 2, 3, 87)
- Save states (4 slots)
- Battery-backed SRAM persistence
- SDL2-based graphics and audio
- Cheat tool with memory search, hex viewer, and egui side panel

## Requirements

- Rust (2021 edition)
- SDL2 library

### macOS (Homebrew)

```bash
brew install sdl2
```

### Ubuntu/Debian

```bash
sudo apt-get install libsdl2-dev
```

## Building

```bash
cargo build --release
```

## Running

```bash
# Using the run script (macOS, cheat UI enabled)
./run.sh

# With a specific ROM
./run.sh roms/game.nes

# Without cheat UI (plain SDL2 version)
cargo build --release
./target/release/nes-emulator <path-to-rom.nes>
```

## Controls

| NES Button | Keyboard |
|------------|----------|
| D-Pad      | Arrow Keys |
| A          | Z |
| B          | X |
| Start      | Enter |
| Select     | Space |

### Save States

- **Save**: Ctrl + 1/2/3/4
- **Load**: 1/2/3/4

### Cheat Panel

- **Toggle panel**: Tab
- **Pause emulation**: Pause checkbox in panel

The cheat panel provides two tabs:

- **Hex Viewer** — Browse and edit CPU RAM / SRAM in real time
- **Cheat Search** — Snapshot-based memory search to find and freeze values (lives, health, etc.)

Cheats are saved/loaded as JSON in the `cheats/` directory.

## Supported Mappers

| Mapper | Name | Examples |
|--------|------|----------|
| 0 | NROM | Super Mario Bros, Donkey Kong |
| 1 | MMC1/SxROM | The Legend of Zelda, Metroid |
| 2 | UxROM | Mega Man, Castlevania |
| 3 | CNROM | Solomon's Key |
| 87 | - | Variant of Mapper 3 |

## Architecture

The emulator follows the NES hardware architecture:

- **CPU**: MOS 6502 processor (unofficial opcodes supported)
- **PPU**: Picture Processing Unit for graphics rendering
- **APU**: Audio Processing Unit for sound generation
- **Bus**: Central interconnect with memory-mapped I/O

### Timing

- CPU clock: ~1.79 MHz
- PPU clock: 3× CPU clock
- Frame rate: 60 FPS (NTSC)

## License

This project is for educational purposes.
