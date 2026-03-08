# NES Emulator (Rust)

Rust implementation of a Nintendo Entertainment System / Famicom emulator (6502 + PPU + APU + mapper layer).

Current focus is compatibility-first execution with broad mapper coverage, SDL front-ends, save states, and cheat/debug tooling for rapid iteration.

## Implemented
- 6502 CPU core with official opcodes, broad unofficial opcode coverage, IRQ/NMI handling, and JAM/KIL halt behaviour.
- PPU background + sprite rendering pipeline, sprite 0 hit / overflow, odd-frame timing, mirroring control, and mapper-driven nametable routing.
- APU pulse/triangle/noise/DMC path plus cartridge expansion audio currently used by Sunsoft 5B, Namco 163, and VRC6 boards.
- Cartridge loader with battery-backed SRAM, save-state integration, and support for 138 iNES mapper IDs.
- Plain SDL front-end (`cargo run --`) and cheat-panel front-end (`./run.sh` or `cargo run --example nes_emulator --features cheat-ui`).
- Headless frame runner for scripted capture/regression work (`headless_test`).

## Quick Start
Preferred launcher:
```bash
./run.sh
./run.sh "roms/<game>.nes"
```

Direct CLI:
```bash
cargo run -- roms/<game>.nes
cargo run --example nes_emulator --features cheat-ui -- roms/<game>.nes
cargo run --bin headless_test -- roms/<game>.nes --frames 120 --capture 0
```

- If no ROM path is provided, both SDL front-ends scan `roms/` and show a selector.
- SRAM saves are written as `<rom>.sav` next to the ROM.
- Save states are written under `states/<rom_stem>.slotN.sav`.
- Cheat files are written under `cheats/<rom_stem>.json` when using the cheat UI.

## SDL Front-Ends
Plain SDL (`cargo run --`):
- D-pad: arrow keys
- A / B: `Z` / `X`
- Start / Select: `Enter` / `Space`
- Save state: `Ctrl + 1..4`
- Load state: `1..4`

Cheat UI (`./run.sh` or `cargo run --example nes_emulator --features cheat-ui`):
- Same game controls and save/load hotkeys as the plain SDL front-end
- Toggle cheat panel: `Tab`
- Tabs: `Hex Viewer`, `Cheat Search`
- Pause emulation: panel checkbox
- The cheat panel accepts ASCII text input only; IME composition is intentionally disabled while it is focused

## Build Notes
- SDL2 is required.
- On macOS, `.cargo/config.toml` adds Homebrew library search paths for `/opt/homebrew/lib` and `/usr/local/lib`.
- `run.sh` also exports Homebrew include/library paths and builds the `cheat-ui` example before launch.

Install SDL2:
```bash
brew install sdl2
sudo apt-get install libsdl2-dev
```

## Development Commands
```bash
cargo fmt
cargo clippy
cargo test
cargo run -- roms/<game>.nes
cargo run --example nes_emulator --features cheat-ui -- roms/<game>.nes
cargo run --bin headless_test -- roms/<game>.nes --frames 300 --capture 120
```

## Known Limitations
- Mapper coverage is broad but still incomplete, and NES 2.0 submapper handling is still limited.
- Exact timing for some rare boards and expansion-audio edge cases is still being refined.
- Compatibility is strongest on iNES ROMs covered by the mapper table below; unsupported boards will not boot correctly.

## Mapper Coverage

| Mapper | Name | Examples |
|--------|------|----------|
| 0 | NROM | Super Mario Bros, Donkey Kong |
| 1 | MMC1/SxROM | The Legend of Zelda, Metroid |
| 2 | UxROM | Mega Man, Castlevania |
| 3 | CNROM | Solomon's Key |
| 4 | MMC3/TxROM | Super Mario Bros. 3, Kirby's Adventure |
| 5 | MMC5/ExROM | Castlevania III: Dracula's Curse, Metal Slader Glory |
| 7 | AxROM | Battletoads |
| 9 | MMC2 | Punch-Out!! |
| 10 | MMC4 | Fire Emblem Gaiden |
| 11 | Color Dreams | Crystal Mines, Menace Beach |
| 12 | MMC3 with split CHR outer bits | Shanghai II |
| 13 | CPROM | Videomation |
| 15 | K-1029/K-1030P | 100-in-1 Contra Function 16 |
| 16 | Bandai FCG | Dragon Ball Z II |
| 18 | Jaleco SS 88006 | Magic John, Pizza Pop! |
| 19 | Namco 163 | Battle Fleet, Dokuganryuu Masamune |
| 21 | VRC4a / VRC4c | Wai Wai World 2, Ganbare Goemon Gaiden 2 |
| 22 | VRC2a | Ganbare Pennant Race, TwinBee 3 |
| 23 | VRC2b / VRC4e | Contra, Tiny Toon Adventures |
| 24 | VRC6a | Akumajou Densetsu |
| 25 | VRC2c / VRC4b / VRC4d | Ganbare Goemon Gaiden, Gradius II |
| 26 | VRC6b | Madara, Esper Dream 2 |
| 32 | Irem G-101 | Major League, Kid Niki 3 |
| 33 | Taito TC0190/TC0350 | Akira, Don Doko Don |
| 34 | BNROM / NINA-001 | Deadly Towers, Impossible Mission II |
| 37 | MMC3 multicart | Super Mario Bros. + Tetris + Nintendo World Cup |
| 38 | Bit Corp. UNL-PCI556 | Crime Busters |
| 40 | NTDEC 2722 / SMB2J conversion | Super Mario Bros. II+, 1990 Super Mario Bros. 4 |
| 41 | Caltron 6-in-1 | Caltron 6-in-1 |
| 42 | Mario Baby / pirate board | Baby Mario, Ai Senshi Nicol |
| 43 | SMB2j pirate conversion | Mario Baby, SMB2j pirates |
| 44 | MMC3 multicart variant | Super 8-in-1 |
| 46 | Rumble Station multicart | Rumble Station 15-in-1 |
| 47 | MMC3 multicart (Nintendo QJ) | Super Spike V'Ball + Nintendo World Cup |
| 48 | Taito TC0690 / X1-005 variant | The Jetsons: Cogswell's Caper! |
| 50 | N-32 / Romeo SMB2J conversion | Super Mario Bros. (JU) (Alt Levels) |
| 57 | GK 6-in-1 / HES 6-in-1 | GK 6-in-1, HES 6-in-1 |
| 58 | GK 4-in-1 / multicart board | Super 700-in-1, Game Star 4-in-1 |
| 59 | T3H53 multicart board | T3H53, Super HIK 4-in-1 |
| 60 | Reset-based 4-in-1 multicart | 4-in-1 multicarts |
| 61 | 20-in-1 / N-32 multicart | 20-in-1 |
| 63 | 16K/32K discrete board with CHR-RAM protect | Modded multicarts |
| 64 | RAMBO-1 | Klax, Shinobi |
| 65 | Irem H-3001 | Daiku no Gen-san 2, Kaiketsu Yanchamaru 3 |
| 66 | GxROM/MHROM | Dragon Spirit, Bart vs. the Space Mutants |
| 67 | Sunsoft-3 | Fantasy Zone 2, Batman: Return of the Joker (Sunsoft-3) |
| 68 | Sunsoft-4 | After Burner |
| 69 | Sunsoft FME-7 | Batman: Return of the Joker, Gimmick! |
| 70 | Jaleco JF-11/JF-14 | City Connection, Rod Land |
| 71 | Camerica / BF9093 | Fire Hawk, Micro Machines |
| 72 | Jaleco JF-17/JF-19 | Pinball Quest |
| 73 | VRC3 | Salamander |
| 74 | Pirate MMC3 / TxROM variant | AV Kyuukyoku Mahjong 2 |
| 75 | VRC1 | Ganbare Goemon Gaiden, Esper Dream 2 |
| 76 | Namcot 3446 | Megami Tensei II |
| 77 | Irem LROG017 | Napoleon Senki |
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
| 99 | Vs. System | Vs. Excitebike, Vs. Super Mario Bros. |
| 101 | JF-10 (bad iNES mapper) | Urusei Yatsura - Lum no Wedding Bell |
| 103 | Fighting Hero / HES bootleg board | Fighting Hero |
| 107 | Magicseries | Magic Dragon |
| 112 | Asder / NTDEC board | Asder 20-in-1, Supervision 16-in-1 |
| 113 | HES NTD-8 | HES 6-in-1, HES 4-in-1 |
| 114 | MMC3 + NROM switch multicart | 1000000-in-1 |
| 115 | MMC3 + NROM switch multicart | 15-in-1 pirates |
| 118 | TxSROM/TLSROM | Goal! 2, Ys III |
| 119 | TQROM | Pinbot (PRG1), High Speed |
| 123 | MMC3 + NROM switch multicart | 700-in-1 multicarts |
| 133 | Sachen / Joy Van (72-pin latch variant) | Jovial Race |
| 137 | Sachen 8259D | He Jian Ka Zhan |
| 140 | Jaleco JF-11/JF-14 variant | Bio Senshi Dan |
| 142 | KS7032 / VRC3-variant pirate board | - |
| 144 | Death Race board | Death Race |
| 145 | Sachen SA-72007 | - |
| 146 | AVE 301/301? (79-compatible) | - |
| 147 | Sachen TC-U01-1.5M | - |
| 148 | Sachen SA-008-A | - |
| 150 | Sachen SA-015 | - |
| 151 | VRC1 alias (should be mapper 75) | - |
| 152 | Jaleco JF-17/JF-19 | Moero!! Pro Soccer, Goal! Two |
| 153 | Bandai FCG-2 | Famicom Jump 2 |
| 154 | Namcot 3453 | Devil Man (UNL variants) |
| 159 | Bandai FCG + X24C01 | Dragon Ball Z: Kyoushuu! Saiya-jin, Knight Gundam Monogatari |
| 180 | UNROM-180 | Crazy Climber |
| 182 | Duplicate of 114 | Super 700-in-1 multicarts |
| 184 | Sunsoft-1 | - |
| 185 | CNROM with CHR-disable protection | Spy vs Spy, Mighty Bomb Jack |
| 189 | MMC3 CHR + 32KB PRG pirate board | Street Fighter Zero 2 '97, Super Mario Fighter III |
| 191 | Xiangfeng 4-in-1 / MMC3 variant | Du Shen |
| 192 | Waixing FS308 | Ying Lie Qun Xia Zhuan |
| 194 | Pirate MMC3 variant | Dai-2-Ji Super Robot Taisen (As) |
| 195 | Waixing FS303 | Columbus Chinese, Captain Tsubasa II Chinese |
| 200 | 1200-in-1 multicart | 1200-in-1 |
| 201 | 21-in-1 multicart | 21-in-1 |
| 202 | 150-in-1 pirate multicart | 150-in-1 |
| 203 | 35-in-1 / 64-in-1 multicart | 35-in-1 |
| 205 | MMC3 multicart variant | 64-in-1 / multicart boards |
| 206 | DxROM/Namco 108 | Faxanadu, Dragon Spirit |
| 207 | Taito X1-005 variant | Fudou Myouou Den |
| 208 | MMC3-like pirate board | Street Fighter IV (pirate) |
| 210 | Namco 175 / Namco 340 | Family Circuit '91, Dream Master |
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
| 248 | Alias of 115 | MMC3 multicarts |
| 250 | Nitra MMC3 wiring | Time Diver Avenger, Queen Bee V |
| 255 | 110-in-1 / 255 multicart | 110-in-1 |

## License

This project is for educational purposes.
