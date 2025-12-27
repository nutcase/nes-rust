# Repository Guidelines

## Project Structure & Module Organization
Rust 2021 NES emulator. Entry point is `src/main.rs` (SDL2 loop, input, save states). Hardware modules live in `src/bus.rs`, `src/cpu/`, `src/ppu/`, `src/apu/`, `src/cartridge/`, and `src/memory/`. Persistence helpers are in `src/save_state.rs` and `src/sram.rs`. Tests sit beside modules (e.g., `src/cpu/tests.rs`, `src/ppu/tests.rs`, `src/ppu/additional_tests.rs`), and build artifacts land in `target/`.

## Architecture Overview & Reading Order
Start with `src/main.rs` (loop + SDL2), then read `src/bus.rs` for the memory map. Next, `src/cpu/mod.rs`, `src/ppu/mod.rs`, and `src/apu/mod.rs` cover timing-critical logic; mapper logic lives in `src/cartridge/mod.rs`.

## Build, Test, and Development Commands
- `cargo build --release` — build the binary.
- `./run.sh` — macOS helper that exports SDL2 paths, builds, then launches the emulator.
- `./target/release/nes-emulator <path-to-rom.nes>` — run a specific ROM.
- `cargo test` — run tests.
- `cargo test <test_name>` — run one test (example: `cargo test test_lda_immediate`).
- `cargo fmt` — format code.
- `cargo clippy` — run the Rust linter.

## Coding Style & Naming Conventions
Use Rust 2021 defaults. No repo `rustfmt`/`clippy` config; if you run `cargo fmt` or `cargo clippy`, keep fixes scoped to your change and only address relevant warnings. Follow `snake_case` for modules/functions, `CamelCase` for types/traits (for example `CpuBus`), and `SCREAMING_SNAKE_CASE` for constants.

## Testing Guidelines
Tests use the built-in harness under `#[cfg(test)]` and follow `test_*` naming. Add tests near the code you touch and focus on cycle accuracy, flags, and timing edges. Avoid external ROM dependencies; prefer small in-memory programs and explicit assertions.

## Commit & Pull Request Guidelines
Commit history follows `<type>: <short summary>` (examples: `feat: support NES memory mappers`, `fix: unify split-screen logic`). Keep summaries imperative and scoped. No CI is configured, so PRs should list local tests run, confirm `cargo clippy` passes, and note the ROM used for manual checks. Do not add ROMs to the repo.

## Dependencies & Local Setup
SDL2 is required. On macOS, `run.sh` sets Homebrew SDL2 paths; on Linux, install `libsdl2-dev`. Ensure SDL2 headers/libraries are discoverable on other platforms. Keep local ROMs outside the repo and pass their paths when running.

## Compatibility & Save Data
Save states are serialized via `serde`/`bincode` in `src/save_state.rs`; adding or reordering fields can break old saves, so call it out in your PR. SRAM saves are written as `.sav` next to the ROM via `src/sram.rs`; changes to naming or paths affect existing saves.
