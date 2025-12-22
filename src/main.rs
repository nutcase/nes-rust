mod apu;
mod audio;
mod bus;
mod cartridge;
mod cpu;
mod cpu_bus;
mod cpu_core;
mod debug_flags;
mod debugger;
mod dma;
mod emulator;
mod fake_apu;
mod input;
mod ppu;
mod sa1;
mod savestate;
mod shutdown;

use cartridge::Cartridge;
use emulator::Emulator;
use std::env;
use std::path::{Path, PathBuf};
use std::process;

fn resolve_rom_path(arg: &str) -> Result<PathBuf, String> {
    // 1) Direct path
    let direct = PathBuf::from(arg);
    if direct.exists() {
        return Ok(direct);
    }

    // Helper: try with extensions
    fn with_ext(base: &Path, exts: &[&str]) -> Option<PathBuf> {
        for ext in exts {
            let mut p = base.to_path_buf();
            if p.extension().is_none() {
                p.set_extension(ext);
            }
            if p.exists() {
                return Some(p);
            }
        }
        None
    }

    let exts = ["sfc", "smc"];

    // 2) Try arg relative to roms/ (exact or with extension inference)
    let in_roms = Path::new("roms").join(arg);
    if in_roms.exists() {
        return Ok(in_roms);
    }
    if let Some(p) = with_ext(&in_roms, &exts) {
        return Ok(p);
    }

    // 3) Try adding extension to direct argument (for cases like "game" -> "game.sfc")
    if let Some(p) = with_ext(&direct, &exts) {
        return Ok(p);
    }

    // 4) Case-insensitive stem match search under roms/
    let mut matches: Vec<PathBuf> = Vec::new();
    if let Ok(entries) = std::fs::read_dir("roms") {
        let query = arg.to_lowercase();
        for e in entries.flatten() {
            let path = e.path();
            if !path.is_file() {
                continue;
            }
            if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                if !ext.eq_ignore_ascii_case("sfc") && !ext.eq_ignore_ascii_case("smc") {
                    continue;
                }
            } else {
                continue;
            }
            let stem = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_lowercase();
            if stem == query
                || path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .map(|s| s.to_lowercase())
                    .as_deref()
                    == Some(&query)
            {
                matches.push(path);
            }
        }
    }

    if matches.len() == 1 {
        return Ok(matches.remove(0));
    } else if matches.len() > 1 {
        let list = matches
            .iter()
            .map(|p| format!("- {}", p.display()))
            .collect::<Vec<_>>()
            .join("\n");
        return Err(format!(
            "Multiple ROMs matched '{}'. Please specify one:\n{}",
            arg, list
        ));
    }

    // 5) If nothing matched, provide a helpful error including available ROMs
    let available = std::fs::read_dir("roms")
        .ok()
        .into_iter()
        .flat_map(|it| it.flatten())
        .map(|e| e.path())
        .filter(|p| p.is_file())
        .filter(|p| {
            p.extension()
                .and_then(|s| s.to_str())
                .map(|ext| ext.eq_ignore_ascii_case("sfc") || ext.eq_ignore_ascii_case("smc"))
                .unwrap_or(false)
        })
        .map(|p| format!("- {}", p.display()))
        .collect::<Vec<_>>()
        .join("\n");

    Err(if available.is_empty() {
        format!(
            "ROM '{}' not found. Place *.sfc or *.smc files under ./roms or provide a valid path.",
            arg
        )
    } else {
        format!(
            "ROM '{}' not found. Available ROMs under ./roms:\n{}",
            arg, available
        )
    })
}

fn main() {
    // Check if running in headless mode
    let is_headless = std::env::var("HEADLESS")
        .map(|v| v == "1" || v.to_lowercase() == "true")
        .unwrap_or(false);

    if is_headless {
        // In headless mode, we can use a separate thread with larger stack
        // to prevent overflow during long runs
        const STACK_SIZE: usize = 64 * 1024 * 1024; // 64MB (increased from 8MB to handle deep recursion)
        std::thread::Builder::new()
            .stack_size(STACK_SIZE)
            .spawn(|| {
                run_emulator();
            })
            .expect("Failed to spawn emulator thread")
            .join()
            .expect("Emulator thread panicked");
        let code = shutdown::exit_code();
        if code != 0 {
            process::exit(code);
        }
    } else {
        // In GUI mode, must run on main thread for macOS compatibility
        run_emulator();
        let code = shutdown::exit_code();
        if code != 0 {
            process::exit(code);
        }
    }
}

fn run_emulator() {
    let args: Vec<String> = env::args().collect();

    // Minimal CLI flags (optional):
    //   --strict           => STRICT_PPU_TIMING=1
    //   --force-display    => FORCE_DISPLAY=1
    //   --headless         => HEADLESS=1
    //   --frames <N>       => HEADLESS_FRAMES=N
    //   --input-events <S> => scripted controller input (mainly for headless runs)
    //   --help             => usage
    if args.len() < 2 || args.iter().any(|a| a == "--help" || a == "-h") {
        eprintln!(
            "Usage: {} [--strict] [--force-display] [--headless] [--frames N] [--input-events S] <rom>",
            args[0]
        );
        eprintln!("Supported formats: .sfc, .smc");
        return;
    }

    let mut rom_arg_opt: Option<String> = None;
    let mut i = 1;

    while i < args.len() {
        match args[i].as_str() {
            "--strict" => {
                env::set_var("STRICT_PPU_TIMING", "1");
                i += 1;
            }
            "--force-display" => {
                env::set_var("FORCE_DISPLAY", "1");
                i += 1;
            }
            "--headless" => {
                env::set_var("HEADLESS", "1");
                i += 1;
            }
            "--frames" => {
                if i + 1 >= args.len() {
                    eprintln!("--frames requires a value");
                    process::exit(2);
                }
                env::set_var("HEADLESS_FRAMES", &args[i + 1]);
                i += 2;
            }
            "--input-events" => {
                if i + 1 >= args.len() {
                    eprintln!("--input-events requires a value");
                    process::exit(2);
                }
                if let Err(e) = input::install_scripted_input_events(&args[i + 1]) {
                    eprintln!("--input-events: {}", e);
                    process::exit(2);
                }
                i += 2;
            }
            s if s.starts_with('-') => {
                eprintln!("Unknown option: {}", s);
                process::exit(2);
            }
            s => {
                rom_arg_opt = Some(s.to_string());
                i += 1;
                break;
            }
        }
    }
    if rom_arg_opt.is_none() && i < args.len() {
        rom_arg_opt = Some(args[i].clone());
    }
    let rom_arg = match rom_arg_opt {
        Some(s) => s,
        None => {
            eprintln!("ROM argument missing");
            process::exit(2);
        }
    };

    // Install basic Ctrl-C/SIGTERM handler (Unix) to allow clean SRAM flush on exit
    shutdown::install();

    // Resolve ROM path robustly (direct path, roms/, missing extension, case-insensitive)
    let rom_path = match resolve_rom_path(&rom_arg) {
        Ok(p) => p,
        Err(msg) => {
            eprintln!("{}", msg);
            process::exit(1);
        }
    };

    let quiet = debug_flags::quiet();

    if !quiet {
        println!("Loading ROM: {}", rom_path.display());
    }
    let cartridge = match Cartridge::load_from_file(&rom_path) {
        Ok(cart) => {
            if !quiet {
                println!("ROM loaded successfully!");
            }
            let display_title = build_display_title(&cart, &rom_path);
            if !quiet {
                println!("Title: {}", display_title);
                println!("Mapper: {:?}", cart.header.mapper_type);
                println!("ROM Size: {} KB", cart.header.rom_size / 1024);
                println!("RAM Size: {} KB", cart.header.ram_size / 1024);
            }
            // Pass along the chosen display title
            (cart, display_title)
        }
        Err(e) => {
            eprintln!("Failed to load ROM: {}", e);
            process::exit(1);
        }
    };

    if !quiet {
        println!("\nStarting emulator...");
    }

    // Derive .srm path next to the ROM
    let srm_path = {
        let mut p = rom_path.clone();
        p.set_extension("srm");
        p
    };

    match Emulator::new(cartridge.0, cartridge.1, Some(srm_path)) {
        Ok(mut emulator) => {
            if !quiet {
                println!("Emulator initialized successfully!");
            }
            emulator.run();
        }
        Err(e) => {
            eprintln!("Failed to initialize emulator: {}", e);
            process::exit(1);
        }
    }
}

fn build_display_title(cart: &Cartridge, rom_path: &Path) -> String {
    // 1) Explicit override via env
    if let Ok(ov) = std::env::var("OVERRIDE_TITLE") {
        let t = ov.trim();
        if !t.is_empty() {
            return t.to_string();
        }
    }

    // 2) Source preference via env (auto|header|filename)
    let source = std::env::var("TITLE_SOURCE").unwrap_or_else(|_| "auto".into());

    let header_title = normalize_header_title(&cart.header.title);
    let filename_title = rom_path
        .file_stem()
        .and_then(|s| s.to_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| String::from("(Unknown Title)"));

    match source.as_str() {
        "header" => return non_empty_or(filename_title, header_title),
        "filename" => return filename_title,
        _ => {}
    }

    // 3) Auto heuristic: prefer header unless it looks like a product code or garbage
    if looks_like_valid_title(&header_title) {
        header_title
    } else {
        filename_title
    }
}

fn normalize_header_title(raw: &str) -> String {
    let t = raw.trim_matches('\u{0000}').trim();
    // Collapse consecutive spaces
    let mut out = String::with_capacity(t.len());
    let mut prev_space = false;
    for ch in t.chars() {
        let is_space = ch.is_whitespace();
        if is_space {
            if !prev_space {
                out.push(' ');
            }
        } else {
            out.push(ch);
        }
        prev_space = is_space;
    }
    out.trim().to_string()
}

fn looks_like_valid_title(t: &str) -> bool {
    if t.is_empty() {
        return false;
    }
    // Too many placeholders means likely bad decode
    let q = t.chars().filter(|&c| c == '?').count();
    if q * 2 > t.len() {
        return false;
    }
    // Product-code-ish: short, all ASCII alnum, at least one digit
    if t.len() <= 8
        && t.chars().all(|c| c.is_ascii_alphanumeric())
        && t.chars().any(|c| c.is_ascii_digit())
    {
        return false;
    }
    true
}

fn non_empty_or(fallback: String, preferred: String) -> String {
    if preferred.trim().is_empty() {
        fallback
    } else {
        preferred
    }
}
