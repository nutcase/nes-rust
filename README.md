# NES Emulator

A cycle-accurate NES (Nintendo Entertainment System / Famicom) emulator written in Rust.

## Features

- Cycle-accurate CPU (6502) and PPU emulation
- Audio support with pulse, triangle, noise, and DMC channels
- Multiple mapper support (90 mapper IDs including MMC1/MMC3/FME-7, Bandai FCG, Sunsoft-4, Taito boards, VRC1, and multicart boards)
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
| 13 | CPROM | Videomation |
| 15 | K-1029/K-1030P | 100-in-1 Contra Function 16 |
| 16 | Bandai FCG | Dragon Ball Z II |
| 33 | Taito TC0190/TC0350 | Akira, Don Doko Don |
| 34 | BNROM / NINA-001 | Deadly Towers, Impossible Mission II |
| 38 | Bit Corp. UNL-PCI556 | Crime Busters |
| 46 | Rumble Station multicart | Rumble Station 15-in-1 |
| 58 | GK 4-in-1 / multicart board | Super 700-in-1, Game Star 4-in-1 |
| 66 | GxROM/MHROM | Dragon Spirit, Bart vs. the Space Mutants |
| 68 | Sunsoft-4 | After Burner |
| 69 | Sunsoft FME-7 | Batman: Return of the Joker, Gimmick! |
| 70 | Jaleco JF-11/JF-14 | City Connection, Rod Land |
| 71 | Camerica / BF9093 | Fire Hawk, Micro Machines |
| 72 | Jaleco JF-17/JF-19 | Pinball Quest |
| 74 | Pirate MMC3 / TxROM variant | AV Kyuukyoku Mahjong 2 |
| 75 | VRC1 | Ganbare Goemon Gaiden, Esper Dream 2 |
| 76 | Namcot 3446 | Megami Tensei II |
| 78 | Irem 74HC161/32 | Holy Diver, Cosmo Carrier |
| 79 | AVE NINA-03/NINA-06 | - |
| 80 | Taito X1-005 | Bakushou!! Jinsei Gekijou |
| 81 | NTDEC N715021 | Super Gun |
| 82 | Taito X1-017 | Kyuukyoku Harikiri Koushien |
| 86 | Jaleco JF-13 | Moero!! Pro Yakyuu '88 Ketteiban |
| 87 | - | Variant of Mapper 3 |
| 88 | Namcot 3443 | Devil Man |
| 89 | Sunsoft-2 | - |
| 92 | Jaleco JF-17/JF-19 (alt PRG wiring) | Moero!! Pro Tennis |
| 93 | Sunsoft-2 variant | - |
| 94 | UN1ROM | Senjou no Ookami, Higemaru Makaijima |
| 95 | Namcot 3425 | Dragon Buster |
| 97 | Irem TAM-S1 | Kaiketsu Yanchamaru |
| 101 | JF-10 (bad iNES mapper) | Urusei Yatsura - Lum no Wedding Bell |
| 107 | Magicseries | Magic Dragon |
| 113 | HES NTD-8 | HES 6-in-1, HES 4-in-1 |
| 118 | TxSROM/TLSROM | Goal! 2, Ys III |
| 119 | TQROM | Pinbot (PRG1), High Speed |
| 133 | Sachen / Joy Van (72-pin latch variant) | Jovial Race |
| 140 | Jaleco JF-11/JF-14 variant | Bio Senshi Dan |
| 144 | Death Race board | Death Race |
| 145 | Sachen SA-72007 | - |
| 146 | AVE 301/301? (79-compatible) | - |
| 147 | Sachen TC-U01-1.5M | - |
| 148 | Sachen SA-008-A | - |
| 152 | Jaleco JF-17/JF-19 | Moero!! Pro Soccer, Goal! Two |
| 154 | Namcot 3453 | Devil Man (UNL variants) |
| 180 | UNROM-180 | Crazy Climber |
| 184 | Sunsoft-1 | - |
| 191 | Xiangfeng 4-in-1 / MMC3 variant | Du Shen |
| 192 | Waixing FS308 | Ying Lie Qun Xia Zhuan |
| 194 | Pirate MMC3 variant | Dai-2-Ji Super Robot Taisen (As) |
| 195 | Waixing FS303 | Columbus Chinese, Captain Tsubasa II Chinese |
| 200 | 1200-in-1 multicart | 1200-in-1 |
| 201 | 21-in-1 multicart | 21-in-1 |
| 202 | 150-in-1 pirate multicart | 150-in-1 |
| 203 | 35-in-1 / 64-in-1 multicart | 35-in-1 |
| 206 | DxROM/Namco 108 | Faxanadu, Dragon Spirit |
| 207 | Taito X1-005 variant | Fudou Myouou Den |
| 208 | MMC3-like pirate board | Street Fighter IV (pirate) |
| 212 | Super HIK multicart | 9999999-in-1 |
| 213 | Duplicate of 58 | 150-in-1 multicarts |
| 221 | NTDEC 821202C | 76-in-1, Super 42-in-1 |
| 225 | 72-in-1 multicart board | 72-in-1 |
| 226 | 76-in-1 / multicart board | 76-in-1 |
| 227 | 120-in-1 multicart / FW-01 | 1992 Contra 120-in-1, Chinese RPG variants |
| 228 | Action 52 / Cheetahmen II | Action 52, Cheetahmen II |
| 229 | 31-in-1 multicart | 31-in-1 |
| 230 | 22-in-1 multicart / Contra switcher | 22-in-1 |
| 231 | 20-in-1 multicart | 20-in-1 |
| 232 | Camerica BF9096 / Quattro | Quattro Adventure, Quattro Sports |
| 233 | 42-in-1 multicart (Disch notes) | 42-in-1 |
| 234 | Maxi 15 / AVE board | Maxi 15 |
| 235 | Golden Game 150-in-1 | 150-in-1 Contra Function 16 |
| 236 | Realtec 8031/8155/8099 | 35-in-1, 56-in-1, 68-in-1 |
| 240 | Gen Ke Le Zhuan / Sheng Huo Lie Zhuan | Jing Ke Xin Zhuan, Sheng Huo Lie Zhuan |
| 241 | BxROM with WRAM | Education Computer 26-in-1 |
| 242 | Waixing FS005/FS306 multicart | Waixing multicarts |
| 243 | Sachen SA-020A | Honey Peach |
| 245 | Waixing F003 | Dragon Quest IV (Waixing) |
| 246 | G0151-1 | Feng Shen Bang |
| 250 | Nitra MMC3 wiring | Time Diver Avenger, Queen Bee V |
| 255 | 110-in-1 / 255 multicart | 110-in-1 |

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
