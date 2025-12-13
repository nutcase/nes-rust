#[path = "../apu.rs"]
mod apu;
#[path = "../audio.rs"]
mod audio;
#[path = "../bus.rs"]
mod bus;
#[path = "../cartridge.rs"]
mod cartridge;
#[path = "../cpu.rs"]
mod cpu;
#[path = "../cpu_bus.rs"]
mod cpu_bus;
#[path = "../cpu_core.rs"]
mod cpu_core;
#[path = "../debug_flags.rs"]
mod debug_flags;
#[path = "../debugger.rs"]
mod debugger;
#[path = "../dma.rs"]
mod dma;
#[path = "../emulator.rs"]
mod emulator;
#[path = "../fake_apu.rs"]
mod fake_apu;
#[path = "../input.rs"]
mod input;
#[path = "../ppu.rs"]
mod ppu;
#[path = "../sa1.rs"]
mod sa1;
#[path = "../savestate.rs"]
mod savestate;
#[path = "../shutdown.rs"]
mod shutdown;

use crate::cartridge::Cartridge;
use crate::emulator::Emulator;
use std::env;
use std::path::PathBuf;

// 簡易 WRAM ダンプツール
// 使い方:
//   cargo run --release --bin dump_wram -- roms/tests/cputest-full.sfc --frames 120 --start 0x0000 --len 0x0200
// 環境変数でも指定可能: WRAM_START, WRAM_LEN, HEADLESS_FRAMES

fn parse_u32_hex_or_dec(s: &str) -> Option<u32> {
    let s = s.trim();
    if let Some(stripped) = s.strip_prefix("0x") {
        u32::from_str_radix(stripped, 16).ok()
    } else {
        u32::from_str_radix(s, 10).ok()
    }
}

fn parse_args() -> (PathBuf, u64, u32, u32, bool) {
    let mut args = std::env::args().skip(1);
    let mut rom: Option<PathBuf> = None;
    let mut frames: Option<u64> = None;
    let mut start: Option<u32> = None;
    let mut len: Option<u32> = None;
    let mut nonzero_only = false;

    while let Some(a) = args.next() {
        match a.as_str() {
            "--frames" => {
                if let Some(v) = args.next() {
                    frames = parse_u32_hex_or_dec(&v).map(|n| n as u64);
                }
            }
            "--start" => {
                if let Some(v) = args.next() {
                    start = parse_u32_hex_or_dec(&v);
                }
            }
            "--len" | "--length" => {
                if let Some(v) = args.next() {
                    len = parse_u32_hex_or_dec(&v);
                }
            }
            "--nonzero" | "--nz" => {
                nonzero_only = true;
            }
            _ => {
                if rom.is_none() {
                    rom = Some(PathBuf::from(&a));
                }
            }
        }
    }

    let rom = rom.expect("ROM path is required");
    let frames = frames
        .or_else(|| {
            env::var("HEADLESS_FRAMES")
                .ok()
                .and_then(|s| s.parse().ok())
        })
        .unwrap_or(120);
    let start = start
        .or_else(|| {
            env::var("WRAM_START")
                .ok()
                .and_then(|s| parse_u32_hex_or_dec(&s))
        })
        .unwrap_or(0x0000);
    let len = len
        .or_else(|| {
            env::var("WRAM_LEN")
                .ok()
                .and_then(|s| parse_u32_hex_or_dec(&s))
        })
        .unwrap_or(0x0200);

    (rom, frames, start, len, nonzero_only)
}

fn resolve_rom_path(p: &PathBuf) -> PathBuf {
    if p.exists() {
        return p.clone();
    }
    let alt = PathBuf::from("roms").join(p);
    if alt.exists() {
        return alt;
    }
    panic!("ROM not found: {:?} (also tried {:?})", p, alt);
}

fn main() {
    // 静かに動かす
    env::set_var("HEADLESS", "1");
    env::set_var("QUIET", "1");

    let (rom_arg, frames, start, len, nonzero_only) = parse_args();
    env::set_var("HEADLESS_FRAMES", frames.to_string());

    let rom_path = resolve_rom_path(&rom_arg);
    let cart = Cartridge::load_from_file(&rom_path).expect("failed to load ROM");
    let title = rom_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("(dump)")
        .to_string();
    let mut emu = Emulator::new(cart, title, None).expect("failed to init emulator");

    emu.run();

    let wram = emu.wram();
    if start as usize >= wram.len() {
        eprintln!(
            "WRAM start 0x{:05X} is outside WRAM size (0x{:05X})",
            start,
            wram.len()
        );
        return;
    }
    let end = ((start + len) as usize).min(wram.len());
    println!(
        "WRAM dump: start=0x{:05X} len=0x{:04X} (frames={}){}",
        start,
        end - start as usize,
        frames,
        if nonzero_only { " [nonzero only]" } else { "" }
    );

    let mut addr = start as usize;
    while addr < end {
        let line_end = (addr + 16).min(end);
        let has_nz = wram[addr..line_end].iter().any(|&b| b != 0);
        if !nonzero_only || has_nz {
            print!("{:05X}:", addr);
            for i in addr..line_end {
                print!(" {:02X}", wram[i]);
            }
            println!();
        }
        addr = line_end;
    }
}
