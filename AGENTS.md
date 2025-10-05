# Repository Guidelines

## Mission
Be able to load and correctly execute the game ROMs in the ROMs folder.

## Project Structure & Module Organization
-   `src/` — Rust modules for the emulator:
    -   `cpu.rs`, `ppu.rs`, `apu.rs`, `dma.rs`, `bus.rs`, `cartridge.rs`, `emulator.rs`, `audio.rs`, `input.rs`, `debugger.rs`, `savestate.rs`.
-   `roms/` — local ROMs (ignored). Use `.sfc`/`.smc`.
-   `tools/` — helper scripts (e.g., `tools/smoke.sh`).
-   `run.sh` — convenience launcher (build + run + log piping).
-   `logs/` — runtime logs per run.
-   `Cargo.toml` — crate metadata and dependencies.

## Build, Test, and Development Commands
-   Build (optimized): `cargo build --release`
-   Run with ROM:
    -   `cargo run --release -- roms/<game>.sfc`
    -   `./run.sh <rom_filename_or_path>`
-   Headless smoke test (quick regression):
    -   `tools/smoke.sh roms/<game>.sfc`
    -   Useful env: `HEADLESS=1 HEADLESS_FRAMES=180`, `QUIET=1`.
-   Debug‑reduced console output: `QUIET=1 ./run.sh <rom>`
-   Check logs
    -   After running `run.sh`, check `logs/latest.log` to confirm whether it's executing correctly.

## Coding Style & Naming Conventions
-   Rust 2021; 4‑space indent; prefer small, focused functions.
-   Names: modules/files `snake_case`; types `UpperCamelCase`; functions/vars `snake_case`; constants `SCREAMING_SNAKE_CASE`.
-   Avoid ad‑hoc game‑specific hacks; gate experimental logs behind env flags.
-   Formatting/lint (recommended): `cargo fmt`, `cargo clippy -- -D warnings`.

## Testing Guidelines
-   Primary: `tools/smoke.sh` verifies DMA/VRAM/OAM activity and clean exit.
-   Headless checks for boot/title: `HEADLESS=1 HEADLESS_FRAMES=120–300 cargo run --release -- <rom>`.
-   Prefer assertions via counters/log lines (e.g., “VRAM L/H=…”, “CGRAM=…”) over screenshot baselines.
-   Place additional scripts under `tools/`; name by concern (`smoke_ppu_init.sh`, `smoke_dma_vram.sh`).

## Commit & Pull Request Guidelines
-   Commits: imperative, present tense; include scope where useful (e.g., `ppu: fix CGRAM staging`).
-   Describe motivation and effect; paste minimal before/after logs or screenshots when relevant.
-   PRs: clear summary, repro steps, flags used (`DEBUG_*`, `HEADLESS`, `QUIET`), linked issues.

## Architecture & Debug Tips
-   Core flow: `Emulator` drives `Cpu` ↔ `Bus` (maps `Cartridge`); `Ppu`/`Apu`/`Dma` hang off the bus; `audio` mixes samples.
-   Helpful flags: `HEADLESS`, `HEADLESS_FRAMES`, `QUIET`, `DEBUG_BOOT`, `DEBUG_DMA`, `DEBUG_DMA_REG`.
-   Keep behavior faithful to SNES timing/ports; prefer feature flags over permanent special‑case paths.

日本語で回答してください
