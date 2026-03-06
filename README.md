# NES Emulator

A cycle-accurate NES (Nintendo Entertainment System / Famicom) emulator written in Rust.

## Features

- Cycle-accurate CPU (6502) and PPU emulation
- Audio support with pulse, triangle, noise, and DMC channels
- Multiple mapper support (0, 1, 2, 3, 4, 7, 9, 10, 11, 16, 34, 66, 69, 70, 71, 87, 152, 180)
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
| 4 | MMC3/TxROM | Super Mario Bros. 3, Kirby's Adventure |
| 7 | AxROM | Battletoads |
| 9 | MMC2 | Punch-Out!! |
| 10 | MMC4 | Fire Emblem Gaiden |
| 11 | Color Dreams | Crystal Mines, Menace Beach |
| 16 | Bandai FCG | Dragon Ball Z II |
| 34 | BNROM / NINA-001 | Deadly Towers, Impossible Mission II |
| 66 | GxROM/MHROM | Dragon Spirit, Bart vs. the Space Mutants |
| 69 | Sunsoft FME-7 | Batman: Return of the Joker, Gimmick! |
| 70 | Jaleco JF-11/JF-14 | City Connection, Rod Land |
| 71 | Camerica / BF9093 | Fire Hawk, Micro Machines |
| 87 | - | Variant of Mapper 3 |
| 152 | Jaleco JF-17/JF-19 | Moero!! Pro Soccer, Goal! Two |
| 180 | UNROM-180 | Crazy Climber |

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
