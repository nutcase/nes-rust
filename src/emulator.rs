#![allow(unreachable_patterns)]
// Allow dev-only helpers to coexist without warnings in release
// Allow dev-only helpers to coexist without warnings in release
// Allow dev-only helpers to coexist without warnings in release
// cleaned: was stray inner attributes
// #![allow(dead_code)]
// #![allow(static_mut_refs)]
use minifb::{Key, Window, WindowOptions};
use std::io::BufWriter;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use crate::audio::AudioSystem;
use crate::bus::Bus;
use crate::cartridge::Cartridge;
use crate::cpu::Cpu;
use crate::debugger::Debugger;
use crate::savestate::*;
use crate::shutdown;

fn write_framebuffer_ppm(
    path: &std::path::Path,
    fb: &[u32],
    width: usize,
    height: usize,
) -> Result<(), std::io::Error> {
    let mut ppm = Vec::with_capacity(width * height * 3 + 32);
    ppm.extend_from_slice(format!("P6\n{} {}\n255\n", width, height).as_bytes());
    for &px in fb.iter().take(width * height) {
        let r = ((px >> 16) & 0xFF) as u8;
        let g = ((px >> 8) & 0xFF) as u8;
        let b = (px & 0xFF) as u8;
        ppm.extend_from_slice(&[r, g, b]);
    }
    std::fs::write(path, &ppm)
}

fn write_framebuffer_png(
    path: &std::path::Path,
    fb: &[u32],
    width: usize,
    height: usize,
) -> Result<(), String> {
    let file = std::fs::File::create(path).map_err(|e| e.to_string())?;
    let w = BufWriter::new(file);

    let mut encoder = png::Encoder::new(w, width as u32, height as u32);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().map_err(|e| e.to_string())?;

    let mut rgba = Vec::with_capacity(width * height * 4);
    for &px in fb.iter().take(width * height) {
        let r = ((px >> 16) & 0xFF) as u8;
        let g = ((px >> 8) & 0xFF) as u8;
        let b = (px & 0xFF) as u8;
        let a = ((px >> 24) & 0xFF) as u8;
        rgba.extend_from_slice(&[r, g, b, a]);
    }
    writer
        .write_image_data(&rgba)
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn write_framebuffer_image(
    path: &std::path::Path,
    fb: &[u32],
    width: usize,
    height: usize,
) -> Result<(), String> {
    match path.extension().and_then(|s| s.to_str()) {
        Some(ext) if ext.eq_ignore_ascii_case("ppm") => {
            write_framebuffer_ppm(path, fb, width, height).map_err(|e| e.to_string())
        }
        Some(ext) if ext.eq_ignore_ascii_case("png") => write_framebuffer_png(path, fb, width, height),
        _ => write_framebuffer_png(path, fb, width, height),
    }
}

#[derive(Debug, Clone)]
pub struct PerformanceStats {
    fps: f64,
    frame_time_avg: Duration,
    frame_time_min: Duration,
    frame_time_max: Duration,
    #[allow(dead_code)]
    cpu_usage: f64,
    dropped_frames: u64,
    total_frames: u64,
    last_fps_update: Instant,
    frame_times: Vec<Duration>,
    // Component-level timing
    cpu_time_total: Duration,
    ppu_time_total: Duration,
    dma_time_total: Duration,
    sa1_time_total: Duration,
    // Timing samples for current second
    cpu_time_samples: Vec<Duration>,
    ppu_time_samples: Vec<Duration>,
    dma_time_samples: Vec<Duration>,
    sa1_time_samples: Vec<Duration>,
}

impl PerformanceStats {
    fn new() -> Self {
        Self {
            fps: 60.0,
            frame_time_avg: Duration::from_secs_f64(1.0 / 60.0),
            frame_time_min: Duration::from_secs_f64(1.0 / 60.0),
            frame_time_max: Duration::from_secs_f64(1.0 / 60.0),
            cpu_usage: 0.0,
            dropped_frames: 0,
            total_frames: 0,
            last_fps_update: Instant::now(),
            frame_times: Vec::with_capacity(60),
            cpu_time_total: Duration::ZERO,
            ppu_time_total: Duration::ZERO,
            dma_time_total: Duration::ZERO,
            sa1_time_total: Duration::ZERO,
            cpu_time_samples: Vec::with_capacity(60),
            ppu_time_samples: Vec::with_capacity(60),
            dma_time_samples: Vec::with_capacity(60),
            sa1_time_samples: Vec::with_capacity(60),
        }
    }

    fn update(&mut self, frame_time: Duration) {
        self.total_frames += 1;
        self.frame_times.push(frame_time);

        // Update min/max
        if frame_time < self.frame_time_min {
            self.frame_time_min = frame_time;
        }
        if frame_time > self.frame_time_max {
            self.frame_time_max = frame_time;
        }

        // Keep only the last 60 frame times for averaging
        if self.frame_times.len() > 60 {
            self.frame_times.remove(0);
        }

        let now = Instant::now();
        if now.duration_since(self.last_fps_update) >= Duration::from_secs(1) {
            // Calculate FPS and average frame time
            if !self.frame_times.is_empty() {
                let total_time: Duration = self.frame_times.iter().sum();
                self.frame_time_avg = total_time / self.frame_times.len() as u32;
                self.fps = 1.0 / self.frame_time_avg.as_secs_f64();
            }

            // Reset min/max for next second
            self.frame_time_min = Duration::from_secs_f64(1.0 / 60.0);
            self.frame_time_max = Duration::from_secs_f64(1.0 / 60.0);

            // Clear component timing samples for next second
            self.cpu_time_samples.clear();
            self.ppu_time_samples.clear();
            self.dma_time_samples.clear();
            self.sa1_time_samples.clear();

            self.last_fps_update = now;
        }
    }

    fn add_cpu_time(&mut self, time: Duration) {
        self.cpu_time_total += time;
        self.cpu_time_samples.push(time);
    }

    fn add_ppu_time(&mut self, time: Duration) {
        self.ppu_time_total += time;
        self.ppu_time_samples.push(time);
    }

    #[allow(dead_code)]
    fn add_dma_time(&mut self, time: Duration) {
        self.dma_time_total += time;
        self.dma_time_samples.push(time);
    }

    fn add_sa1_time(&mut self, time: Duration) {
        self.sa1_time_total += time;
        self.sa1_time_samples.push(time);
    }

    fn should_skip_frame(&self, target_fps: f64) -> bool {
        self.fps < target_fps * 0.85 // Skip if running more than 15% slower
    }

    #[allow(dead_code)]
    fn get_cpu_usage_percent(&self) -> f64 {
        self.cpu_usage * 100.0
    }
}

const SCREEN_WIDTH: usize = 256;
const SCREEN_HEIGHT: usize = 224;
#[allow(dead_code)]
const MASTER_CLOCK_NTSC: f64 = 21_477_272.0;
// 実機は CPU:PPU=6:4（=3:2）。
// ここでは「master clock からの分周」を使って CPUサイクル→PPUドット数へ変換する。
const CPU_CLOCK_DIVIDER: u64 = 6;
const PPU_CLOCK_DIVIDER: u64 = 4;

pub struct Emulator {
    cpu: Cpu,
    bus: Bus,
    window: Option<Window>,
    frame_buffer: Vec<u32>,
    master_cycles: u64,
    // Pending "stall" time in master cycles (e.g., MDMA); CPU is halted while PPU/APU advance.
    pending_stall_master_cycles: u64,
    // CPU->PPU 変換時の端数（master cycles を PPU_CLOCK_DIVIDER で割った余り; 0..PPU_CLOCK_DIVIDER-1）
    ppu_cycle_accum: u8,
    // APU step batching: pending CPU cycles to apply to the APU.
    apu_cycle_debt: u32,
    // master->CPU conversion remainder for APU batching (0..CPU_CLOCK_DIVIDER-1)
    apu_master_cycle_accum: u8,
    apu_step_batch: u32,
    apu_step_force: u32,
    next_frame_time: Instant,
    target_frame_duration: Duration,
    rom_checksum: u32,
    frame_count: u64,
    // Performance optimization fields
    frame_skip_count: u8,
    max_frame_skip: u8,
    adaptive_timing: bool,
    performance_stats: PerformanceStats,
    audio_system: AudioSystem,
    present_every_auto: u64,
    // NMI handling
    #[allow(dead_code)]
    nmi_triggered_this_flag: bool,
    debugger: Debugger,
    #[allow(dead_code)]
    rom_title: String,
    black_screen_streak: u32,
    black_screen_reported: bool,
    headless: bool,
    headless_max_frames: u64,
    srm_path: Option<PathBuf>,
    srm_autosave_every: Option<u64>,
    srm_last_autosave_frame: u64,
    boot_fallback_applied: bool,
    #[allow(dead_code)]
    palette_fallback_applied: bool,
}

impl Emulator {
    pub fn new(
        cartridge: Cartridge,
        display_title: String,
        srm_path: Option<PathBuf>,
    ) -> Result<Self, String> {
        let quiet = crate::debug_flags::quiet();
        let rom = cartridge.rom.clone();
        let mut bus = Bus::new_with_mapper(
            cartridge.rom,
            cartridge.header.mapper_type.clone(),
            cartridge.header.ram_size,
        );
        // CPUテストROM用の補助（通常ROMでは無効）
        // - 65C816 TEST: cputest-full.sfc 等
        // - 明示的に有効化したい場合は CPU_TEST_MODE=1
        let title_up = display_title.to_ascii_uppercase();
        let cpu_test_env = std::env::var_os("CPU_TEST_MODE").is_some();
        if cpu_test_env
            || title_up.contains("CPU TEST")
            || title_up.contains("CPUTEST")
            || title_up.contains("65C816 TEST")
        {
            bus.enable_cpu_test_mode();
        }
        if (crate::debug_flags::mapper() || crate::debug_flags::boot_verbose()) && !quiet {
            println!("Mapper: {:?}", cartridge.header.mapper_type);
        }
        let mut cpu = Cpu::new();

        // SNESのリセットベクターは0x00FFFCにある
        let reset_vector_lo = bus.read_u8(0x00FFFC) as u16;
        let reset_vector_hi = bus.read_u8(0x00FFFD) as u16;
        let reset_vector = (reset_vector_hi << 8) | reset_vector_lo;
        if crate::debug_flags::boot_verbose() && !quiet {
            println!(
                "Reset vector: 0x{:04X} (lo=0x{:02X}, hi=0x{:02X})",
                reset_vector, reset_vector_lo, reset_vector_hi
            );
        }

        // リセットベクターが無効な場合、デバッグ情報を表示
        if reset_vector == 0x0000 || reset_vector == 0xFFFF {
            if crate::debug_flags::boot_verbose() && !quiet {
                println!("WARNING: Invalid reset vector detected!");
                println!(
                    "ROM info: title='{}', mapper={:?}, size={}KB",
                    cartridge.header.title,
                    cartridge.header.mapper_type,
                    cartridge.header.rom_size / 1024
                );
                println!("Memory around reset vector (0xFFFC-0xFFFF):");
                for addr in 0xFFFC..=0xFFFF {
                    let val = bus.read_u8(addr);
                    println!("  0x{:04X}: 0x{:02X}", addr, val);
                }
            }
        }

        cpu.reset(reset_vector);

        // Initialize stack area to prevent 0xFFFF values
        cpu.init_stack(&mut bus);

        // --- Optional override via env: FORCE_MAPPER=lorom|hirom|exhirom ---
        if let Ok(val) = std::env::var("FORCE_MAPPER") {
            use crate::cartridge::MapperType;
            let forced = match val.to_lowercase().as_str() {
                "lorom" => Some(MapperType::LoRom),
                "hirom" => Some(MapperType::HiRom),
                "exhirom" => Some(MapperType::ExHiRom),
                _ => None,
            };
            if let Some(m) = forced {
                if !quiet {
                    println!("FORCE_MAPPER applied: {:?}", m);
                }
                bus.set_mapper_type(m);
                let lo = bus.read_u8(0x00FFFC) as u16;
                let hi = bus.read_u8(0x00FFFD) as u16;
                let rv = ((hi << 8) | lo) as u16;
                cpu.reset(rv);
                cpu.init_stack(&mut bus);
            }
        }

        // --- Runtime mapper self-check: probe candidates and pick the healthiest ---
        let disable_autocorrect = std::env::var("DISABLE_MAPPER_AUTOCORRECT")
            .map(|v| v == "1" || v.to_lowercase() == "true")
            .unwrap_or(false);
        use crate::cartridge::MapperType;
        fn sample_non_ff(bus: &mut Bus, addr: u32, n: usize) -> usize {
            let mut cnt = 0usize;
            for off in 0..(n as u32) {
                if bus.read_u8(addr.wrapping_add(off)) != 0xFF {
                    cnt += 1;
                }
            }
            cnt
        }
        fn score_mapper(bus: &mut Bus, mapper: MapperType) -> (usize, u16) {
            let mut score = 0usize;
            bus.set_mapper_type(mapper);
            // reset vector
            let rv_lo = bus.read_u8(0x00FFFC) as u16;
            let rv_hi = bus.read_u8(0x00FFFD) as u16;
            let rv = ((rv_hi << 8) | rv_lo) as u16;
            // sample around reset in bank 00
            score += sample_non_ff(bus, (0x00u32 << 16) | (rv as u32), 32);
            // sample high regions in common code banks
            for &bank in &[0x00u8, 0x80u8, 0x85u8, 0xC0u8] {
                let base = ((bank as u32) << 16) | 0xFF80;
                score += sample_non_ff(bus, base, 0x80);
            }
            (score, rv)
        }
        if !disable_autocorrect {
            let current_mapper = bus.get_mapper_type();
            // Build candidate set
            let mut candidates = vec![current_mapper];
            if !candidates.contains(&MapperType::LoRom) {
                candidates.push(MapperType::LoRom);
            }
            if !candidates.contains(&MapperType::HiRom) {
                candidates.push(MapperType::HiRom);
            }
            if !candidates.contains(&MapperType::ExHiRom) {
                candidates.push(MapperType::ExHiRom);
            }

            // Skip auto-correct for special mappers (non Lo/Hi/ExHiROM)
            if !matches!(
                current_mapper,
                MapperType::LoRom | MapperType::HiRom | MapperType::ExHiRom
            ) {
                if !quiet {
                    println!(
                        "Mapper auto-correct skipped for special mapper: {:?}",
                        current_mapper
                    );
                }
            } else {
                let mut best = current_mapper;
                let mut best_score = 0usize;
                let mut cur_score = 0usize;
                let mut best_rv: u16 = reset_vector;
                for cand in candidates.into_iter() {
                    let (s, rv) = score_mapper(&mut bus, cand);
                    if crate::debug_flags::mapper() {
                        println!("Mapper score {:?}: {} (reset=0x{:04X})", cand, s, rv);
                    }
                    if cand == current_mapper {
                        cur_score = s;
                    }
                    if s > best_score {
                        best_score = s;
                        best = cand;
                        best_rv = rv;
                    }
                }
                // Adopt best only if it clearly beats current (margin to avoid mis-picks)
                let force_best = std::env::var("FORCE_MAPPER_BEST")
                    .map(|v| v == "1" || v.to_lowercase() == "true")
                    .unwrap_or(false);
                if best != current_mapper
                    && (force_best || best_score >= cur_score.saturating_add(100))
                {
                    if !quiet {
                        println!(
                            "Mapper auto-correct: {:?} -> {:?} (best score={}, cur score={}), reset=0x{:04X}",
                            current_mapper, best, best_score, cur_score, best_rv
                        );
                    }
                    bus.set_mapper_type(best);
                    cpu.reset(best_rv);
                    cpu.init_stack(&mut bus);
                } else {
                    // Keep current mapper
                    bus.set_mapper_type(current_mapper);
                }
            }
        } else if !quiet {
            println!("Mapper auto-correct disabled by env");
        }

        // Optional ROM byte dump for boot diagnosis
        if std::env::var("DUMP_BOOT_BYTES")
            .map(|v| v == "1" || v.to_lowercase() == "true")
            .unwrap_or(false)
        {
            fn dump_range(bus: &mut Bus, base: u32, len: usize) {
                print!(
                    "DUMP {:02X}:{:04X}-{:04X}: ",
                    (base >> 16) & 0xFF,
                    base as u16,
                    (base as u16).wrapping_add(len as u16)
                );
                for i in 0..len as u32 {
                    print!("{:02X} ", bus.read_u8(base + i));
                }
                println!("");
            }
            let pc_reset = ((0x00u32) << 16) | (cpu.pc as u32);
            if !quiet {
                println!("Boot PC after mapper: {:02X}:{:04X}", cpu.pb, cpu.pc);
            }
            dump_range(&mut bus, pc_reset.wrapping_sub(8) & 0x00FFFF, 32);
            dump_range(&mut bus, 0x00FFFC, 4);
        }

        // Headless via env
        let headless_env = std::env::var("HEADLESS")
            .map(|v| v == "1" || v.to_lowercase() == "true")
            .unwrap_or(false);
        let headless_max_frames: u64 = std::env::var("HEADLESS_FRAMES")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(300);

        // Try to create a window unless headless requested. If it fails (e.g., no GUI), fallback to headless.
        let mut window_opt = if headless_env {
            None
        } else {
            match Window::new(
                "SNES Emulator",
                SCREEN_WIDTH,
                SCREEN_HEIGHT,
                WindowOptions {
                    resize: true,
                    scale: minifb::Scale::X2,
                    ..WindowOptions::default()
                },
            ) {
                Ok(w) => Some(w),
                Err(e) => {
                    if !quiet {
                        println!("WINDOW: creation failed ({}). Falling back to headless.", e);
                    }
                    None
                }
            }
        };

        // Use caller-provided display title (already normalized/fallback applied)
        let rom_title = if display_title.trim().is_empty() {
            String::from("(Unknown Title)")
        } else {
            display_title
        };
        if let Some(w) = &mut window_opt {
            w.set_title(&format!("SNES Emulator - {}", rom_title));
        }

        let frame_buffer = vec![0; SCREEN_WIDTH * SCREEN_HEIGHT];
        let target_frame_duration = Duration::from_secs_f64(1.0 / 60.0);

        // Attempt to load existing SRAM from disk (if provided), unless disabled
        let ignore_sram = std::env::var("IGNORE_SRAM")
            .map(|v| v == "1" || v.to_lowercase() == "true")
            .unwrap_or(false);
        if !ignore_sram {
            if let Some(ref path) = srm_path {
                if let Ok(bytes) = std::fs::read(path) {
                    let load_len = bytes.len().min(bus.sram_size());
                    if load_len > 0 {
                        bus.sram_mut()[..load_len].copy_from_slice(&bytes[..load_len]);
                        bus.clear_sram_dirty();
                        if !quiet {
                            println!("SRAM loaded: {} bytes from {}", load_len, path.display());
                        }
                    }
                }
            }
        } else if !quiet {
            println!("SRAM load skipped (IGNORE_SRAM=1)");
        }

        // Calculate ROM checksum for save state validation
        let rom_checksum = calculate_checksum(&rom);

        // Initialize audio system (silent when headless/no-audio to avoid device errors)
        let audio_off = std::env::var("NO_AUDIO")
            .map(|v| v == "1" || v.to_lowercase() == "true")
            .unwrap_or(false);
        let headless_final = headless_env || window_opt.is_none();
        let audio_system = if headless_final || audio_off {
            if !quiet {
                println!("HEADLESS: using silent audio backend (no device init)");
            }
            AudioSystem::new_silent()
        } else {
            let mut asys =
                AudioSystem::new().map_err(|e| format!("Failed to initialize audio: {}", e))?;
            // Connect audio system to APU
            let apu_handle = bus.get_apu_shared();
            asys.set_apu(apu_handle);
            asys.start();
            asys
        };

        // Enable multitap via env (MULTITAP=1)
        let multitap = std::env::var("MULTITAP")
            .map(|v| v == "1" || v.to_lowercase() == "true")
            .unwrap_or(false);
        if multitap {
            bus.get_input_system_mut().set_multitap_enabled(true);
            if !quiet {
                println!("Input: Multitap enabled (controllers 3/4 active)");
            }
        }

        // Allow game to configure NMI/IRQ via $4200

        // Optional SRAM autosave interval (frames); 0/empty disables
        let srm_autosave_every = std::env::var("SRAM_AUTOSAVE_FRAMES")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .filter(|&v| v > 0);

        // Adaptive timing / frame skip control (allow disabling via env)
        let adaptive_timing_env = std::env::var("ADAPTIVE_TIMING")
            .ok()
            .map(|v| !(v == "0" || v.to_lowercase() == "false"))
            .unwrap_or(true);
        let disable_skip_env = std::env::var("DISABLE_FRAME_SKIP")
            .map(|v| v == "1" || v.to_lowercase() == "true")
            .unwrap_or(false);
        let max_frame_skip: u8 = std::env::var("MAX_FRAME_SKIP")
            .ok()
            .and_then(|v| v.parse::<u8>().ok())
            .map(|v| v.min(10))
            .unwrap_or(2);
        let apu_step_batch: u32 = std::env::var("APU_STEP_BATCH")
            .ok()
            .and_then(|v| v.parse::<u32>().ok())
            .filter(|&v| v > 0)
            .unwrap_or(8);
        let apu_step_force: u32 = std::env::var("APU_STEP_FORCE")
            .ok()
            .and_then(|v| v.parse::<u32>().ok())
            .filter(|&v| v > 0)
            .unwrap_or(2048);
        Ok(Emulator {
            cpu,
            bus,
            window: window_opt,
            frame_buffer,
            master_cycles: 0,
            pending_stall_master_cycles: 0,
            ppu_cycle_accum: 0,
            apu_cycle_debt: 0,
            apu_master_cycle_accum: 0,
            apu_step_batch,
            apu_step_force,
            next_frame_time: Instant::now() + target_frame_duration,
            target_frame_duration,
            rom_checksum,
            frame_count: 0,
            frame_skip_count: 0,
            max_frame_skip, // Allow skipping up to N frames for performance
            adaptive_timing: adaptive_timing_env && !disable_skip_env,
            performance_stats: PerformanceStats::new(),
            audio_system,
            present_every_auto: 2,
            nmi_triggered_this_flag: false,
            debugger: Debugger::new(),
            rom_title,
            black_screen_streak: 0,
            black_screen_reported: false,
            headless: headless_final,
            headless_max_frames,
            srm_path,
            srm_autosave_every,
            srm_last_autosave_frame: 0,
            boot_fallback_applied: false,
            palette_fallback_applied: false,
        })
    }

    pub fn run(&mut self) {
        let quiet = crate::debug_flags::quiet();
        // 起動直後にテストパターンを約2秒間（120フレーム）表示（Dragon Quest III修正のため有効化）
        if std::env::var("FORCE_TEST_PATTERN")
            .map(|v| v == "1")
            .unwrap_or(false)
        {
            self.bus.get_ppu_mut().force_test_pattern();
            let frame_delay = std::time::Duration::from_millis(16);
            for _ in 0..120 {
                self.render();
                if !self.headless {
                    std::thread::sleep(frame_delay);
                }
            }
        }
        let mut show_stats = false;
        let mut stats_timer = Instant::now();

        if !self.headless {
            if crate::debug_flags::force_display() {
                println!(
                    "FORCE_DISPLAY: active (ignoring forced blank and brightness in renderer)"
                );
            }
        }

        let headless_fast_render = std::env::var("HEADLESS_FAST_RENDER")
            .map(|v| v == "1" || v.to_lowercase() == "true")
            .unwrap_or(false);
        let headless_fast_render_last: u64 = std::env::var("HEADLESS_FAST_RENDER_LAST")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(1);
        let headless_fast_render_from = self
            .headless_max_frames
            .saturating_sub(headless_fast_render_last.max(1));
        let present_every_env: Option<u64> = std::env::var("PRESENT_EVERY")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .filter(|&v| v > 0);
        let auto_present = std::env::var("AUTO_PRESENT")
            .map(|v| v != "0" && v.to_lowercase() != "false")
            .unwrap_or(true);

        if self.headless {
            if !quiet {
                println!(
                    "HEADLESS mode: running {} frames without window",
                    self.headless_max_frames
                );
            }
            while self.frame_count < self.headless_max_frames && !shutdown::should_quit() {
                let frame_start = Instant::now();
                self.apply_scripted_input_for_headless();
                if headless_fast_render {
                    let enable = self.frame_count >= headless_fast_render_from;
                    self.bus
                        .get_ppu_mut()
                        .set_framebuffer_rendering_enabled(enable);
                }
                if std::env::var("MODE7_TEST")
                    .map(|v| v == "1" || v.to_lowercase() == "true")
                    .unwrap_or(false)
                {
                    self.run_mode7_diag_frame();
                } else {
                    self.run_frame();
                }
                // Headlessでもレンダーパイプを通し、フォールバック描画/テストパターンを反映させる。
                // ただし HEADLESS_FAST_RENDER=1 の場合は、最後の数フレームのみ描画して高速化する。
                if !headless_fast_render || self.frame_count >= headless_fast_render_from {
                    self.render();
                }
                // CPUテストROM: 終了状態（PASS/FAIL）に到達したら早期終了する
                self.maybe_quit_on_cpu_test_result();
                if shutdown::should_quit() {
                    break;
                }
                // Periodic minimal palette injection to ensure visibility until game loads CGRAM
                self.maybe_inject_min_palette_periodic();
                // Debug: periodically dump CPU PC/PB to identify headless stalls (HEADLESS_PC_DUMP=1)
                if std::env::var_os("HEADLESS_PC_DUMP").is_some() && self.frame_count % 120 == 0 {
                    let cpu = self.cpu.core.state();
                    let ppu = self.bus.get_ppu();
                    let wram_flag = self.bus.wram().get(0x0122).copied().unwrap_or(0);
                    println!(
                        "[HEADLESS-PC] frame={} PB={:02X} PC={:04X} A={:04X} X={:04X} Y={:04X} SP={:04X} P={:02X} E={} DB={:02X} DP={:04X} NMI(en={},latched={},vblank={}) WRAM[0122]={:02X}",
                        self.frame_count,
                        cpu.pb,
                        cpu.pc,
                        cpu.a,
                        cpu.x,
                        cpu.y,
                        cpu.sp,
                        cpu.p.bits(),
                        cpu.emulation_mode,
                        cpu.db,
                        cpu.dp,
                        ppu.nmi_enabled,
                        ppu.nmi_latched,
                        ppu.is_vblank(),
                        wram_flag
                    );
                }
                let frame_time = frame_start.elapsed();
                self.performance_stats.update(frame_time);
                self.frame_count += 1;
                self.maybe_dump_framebuffer_at();
                self.maybe_dump_mem_at();

                // Periodic SRAM autosave (optional)
                self.maybe_autosave_sram();
                if self.frame_count == 60
                    || self.frame_count == 120
                    || self.frame_count == 180
                    || self.frame_count == 370
                {
                    {
                        let ppu = self.bus.get_ppu();
                        if !quiet {
                            println!(
                                "PPU usage @frame {}: VRAM {}/{} CGRAM {}/{} OAM {}/{}",
                                self.frame_count,
                                ppu.vram_usage(),
                                0x10000,
                                ppu.cgram_usage(),
                                0x200,
                                ppu.oam_usage(),
                                0x220
                            );
                        }
                        // CGRAM head dump (first few colors)
                        let head = ppu.dump_cgram_head(8);
                        if !head.is_empty() && !quiet {
                            let hex: Vec<String> =
                                head.iter().map(|c| format!("{:04X}", c)).collect();
                            println!("CGRAM head: [{}]", hex.join(", "));
                        }
                    }
                    // Print VRAM FG summary and reset its counters (separate mutable borrow)
                    let summary = { self.bus.get_ppu_mut().take_vram_write_summary() };
                    if !quiet {
                        println!("VRAM summary: {}", summary);
                    }
                    // DMA dest summary (consumes internal counters)
                    let dma_sum = { self.bus.take_dma_dest_summary() };
                    if !quiet {
                        println!("{}", dma_sum);
                    }
                    // HDMA activity summary (consumes counters)
                    let hdma_sum = { self.bus.take_hdma_summary() };
                    if !quiet {
                        println!("{}", hdma_sum);
                    }
                    // Render metrics (consumes counters)
                    let rm = { self.bus.get_ppu_mut().take_render_metrics_summary() };
                    if !quiet {
                        println!("{}", rm);
                    }

                    // Headless visibility metric (counter-based, no screenshots). Separate borrow.
                    let vis_check = std::env::var("HEADLESS_VIS_CHECK")
                        .map(|v| v == "1" || v.to_lowercase() == "true")
                        .unwrap_or(!quiet);
                    if vis_check {
                        let (non_black, first_non_black, sample0, sample128, sample256) = {
                            let fb = self.bus.get_ppu().get_framebuffer();
                            let nb = fb.iter().filter(|&&px| px != 0xFF000000).count();
                            let first = fb
                                .iter()
                                .position(|&px| px != 0xFF000000)
                                .unwrap_or(usize::MAX);
                            let s0 = if fb.len() > 0 { fb[0] } else { 0 };
                            let s128 = if fb.len() > 128 { fb[128] } else { 0 };
                            let s256 = if fb.len() > 256 { fb[256] } else { 0 };
                            (nb, first, s0, s128, s256)
                        };
                        let ppu = self.bus.get_ppu();
                        let brightness = ppu.brightness;
                        let screen_display = ppu.screen_display;
                        let tm = ppu.get_main_screen_designation();
                        let bg_mode = ppu.get_bg_mode();
                        println!(
                            "VISIBILITY: frame={} non_black_pixels={} first_non_black_idx={} brightness={} forced_blank={} INIDISP=0x{:02X} TM=0x{:02X} mode={}",
                            self.frame_count, non_black, first_non_black, brightness, (screen_display & 0x80) != 0, screen_display, tm, bg_mode
                        );
                        // Optional: dump small VRAM/OAM/CGRAM slices for early frames (debug)
                        if std::env::var_os("DUMP_VRAM_HEAD").is_some() && self.frame_count <= 4 {
                            let vram = ppu.dump_vram_head(64);
                            let cgram = ppu.dump_cgram_head(16);
                            let oam = ppu.dump_oam_head(32);
                            println!("VRAM[0..64]: {:02X?}", vram);
                            println!("CGRAM[0..16]: {:04X?}", cgram);
                            println!("OAM[0..32]: {:02X?}", oam);
                        }
                        // Debug TM bits for frames with graphics
                        if non_black > 0 {
                            let bg1_en = (tm & 0x01) != 0;
                            let bg2_en = (tm & 0x02) != 0;
                            let bg3_en = (tm & 0x04) != 0;
                            let bg4_en = (tm & 0x08) != 0;
                            let obj_en = (tm & 0x10) != 0;
                            println!(
                                "  TM bits: BG1={} BG2={} BG3={} BG4={} OBJ={}",
                                bg1_en, bg2_en, bg3_en, bg4_en, obj_en
                            );
                        }
                        if !quiet {
                            println!(
                                "FB SAMPLE: [0]=0x{:08X} [128]=0x{:08X} [256]=0x{:08X}",
                                sample0, sample128, sample256
                            );
                        }
                        // Small top-left region inspection (16x16)
                        let (tl_nonblack, tl_total) = {
                            let fb = self.bus.get_ppu().get_framebuffer();
                            let mut cnt = 0usize;
                            let w = 256usize;
                            let h = 224usize;
                            let rw = 16usize;
                            let rh = 16usize;
                            for y in 0..rh.min(h) {
                                for x in 0..rw.min(w) {
                                    let idx = y * w + x;
                                    if idx < fb.len() && fb[idx] != 0xFF000000 {
                                        cnt += 1;
                                    }
                                }
                            }
                            (cnt, (rw.min(w) * rh.min(h)) as usize)
                        };
                        if !quiet {
                            println!("FB TOPLEFT: non_black={}/{}", tl_nonblack, tl_total);
                        }
                        // Sample first 10 non-black pixels
                        if non_black > 0 && !quiet {
                            let fb = self.bus.get_ppu().get_framebuffer();
                            let samples: Vec<_> = fb
                                .iter()
                                .enumerate()
                                .filter(|(_, &px)| px != 0xFF000000)
                                .take(10)
                                .map(|(idx, &px)| {
                                    let x = idx % 256;
                                    let y = idx / 256;
                                    format!("({},{})=0x{:08X}", x, y, px)
                                })
                                .collect();
                            println!("NON-BLACK SAMPLES: {}", samples.join(", "));
                        }
                    }
                    // Optional per-frame CPU/SA-1 PC dump (HEADLESS_LOG_CPUPC=1)
                    if std::env::var("HEADLESS_LOG_CPUPC")
                        .map(|v| v == "1" || v.to_lowercase() == "true")
                        .unwrap_or(false)
                    {
                        println!(
                            "CPU PC: {:02X}:{:04X} A={:04X} X={:04X} Y={:04X} SP={:04X} P={:02X} icount={}",
                            self.cpu.pb,
                            self.cpu.pc,
                            self.cpu.a,
                            self.cpu.x,
                            self.cpu.y,
                            self.cpu.sp,
                            self.cpu.p.bits(),
                            self.cpu.debug_instruction_count
                        );
                        let sa1 = self.bus.sa1();
                        println!(
                            "SA1 PC: {:02X}:{:04X} A={:04X} X={:04X} Y={:04X} SP={:04X} P={:02X} icount={}",
                            sa1.cpu.pb,
                            sa1.cpu.pc,
                            sa1.cpu.a,
                            sa1.cpu.x,
                            sa1.cpu.y,
                            sa1.cpu.sp,
                            sa1.cpu.p.bits(),
                            sa1.cpu.debug_instruction_count
                        );
                    }
                }
                // Compatibility boot fallback (headless)
                self.maybe_auto_unblank();
            }
            // Final init summary (used by tools/smoke.sh; can be disabled for CPU test runs)
            let headless_summary = std::env::var("HEADLESS_SUMMARY")
                .map(|v| v == "1" || v.to_lowercase() == "true")
                .unwrap_or(true);
            if headless_summary {
                let (nmi_w, mdma_w, hdma_w, dma_cfg) = self.bus.get_init_counters();
                let (imp_w, vwl, vwh, cg, oam) = self.bus.get_ppu().get_init_counters();
                println!(
                    "INIT summary: $4200 writes={} MDMAEN!=0={} HDMAEN!=0={} DMAreg={} PPU important={} VRAM L/H={}/{} CGRAM={} OAM={}",
                    nmi_w, mdma_w, hdma_w, dma_cfg, imp_w, vwl, vwh, cg, oam
                );
                println!("{}", self.bus.get_dma_config_summary());
                // OBJ (sprite) timing summary
                let obj_sum = { self.bus.get_ppu_mut().take_obj_summary() };
                println!("{}", obj_sum);
            }

            // Optional framebuffer dump for headless debugging (PPM, 256x224)
            if std::env::var("HEADLESS_DUMP_FRAME")
                .map(|v| v == "1" || v.to_lowercase() == "true")
                .unwrap_or(false)
            {
                let fb = self.bus.get_ppu().get_framebuffer();
                let _ = std::fs::create_dir_all("logs");
                let out_path = std::env::var("HEADLESS_DUMP_FRAME_PATH")
                    .ok()
                    .filter(|s| !s.trim().is_empty())
                    .unwrap_or_else(|| "logs/headless_fb.png".to_string());
                if let Err(e) = write_framebuffer_image(
                    std::path::Path::new(&out_path),
                    fb,
                    256,
                    224,
                ) {
                    eprintln!("Failed to dump framebuffer: {}", e);
                } else {
                    println!("Framebuffer dumped to {}", out_path);
                }
            }

            // Optional VRAM/CGRAM/OAM dump (binary) for headless debugging
            if std::env::var("HEADLESS_DUMP_MEM")
                .map(|v| v == "1" || v.to_lowercase() == "true")
                .unwrap_or(false)
            {
                let ppu = self.bus.get_ppu();
                let bwram = self.bus.sa1_bwram_slice();
                let _ = std::fs::create_dir_all("logs");
                // Dump WRAM as well for CPU test debugging
                if let Err(e) = std::fs::write("logs/wram.bin", self.bus.wram()) {
                    eprintln!("Failed to dump WRAM: {}", e);
                } else {
                    println!(
                        "WRAM dumped to logs/wram.bin ({} bytes)",
                        self.bus.wram().len()
                    );
                }
                if let Err(e) = std::fs::write("logs/vram.bin", ppu.get_vram()) {
                    eprintln!("Failed to dump VRAM: {}", e);
                } else {
                    println!(
                        "VRAM dumped to logs/vram.bin ({} bytes)",
                        ppu.get_vram().len()
                    );
                }
                if let Err(e) = std::fs::write("logs/cgram.bin", ppu.get_cgram()) {
                    eprintln!("Failed to dump CGRAM: {}", e);
                } else {
                    println!(
                        "CGRAM dumped to logs/cgram.bin ({} bytes)",
                        ppu.get_cgram().len()
                    );
                }
                if let Err(e) = std::fs::write("logs/oam.bin", ppu.get_oam()) {
                    eprintln!("Failed to dump OAM: {}", e);
                } else {
                    println!("OAM dumped to logs/oam.bin ({} bytes)", ppu.get_oam().len());
                }
                if !bwram.is_empty() {
                    if let Err(e) = std::fs::write("logs/bwram.bin", bwram) {
                        eprintln!("Failed to dump BW-RAM: {}", e);
                    } else {
                        println!("BWRAM dumped to logs/bwram.bin ({} bytes)", bwram.len());
                    }
                }
            }

            // Optional PPU register/state dump for headless debugging
            if std::env::var("HEADLESS_DUMP_PPU_STATE")
                .map(|v| v == "1" || v.to_lowercase() == "true")
                .unwrap_or(false)
            {
                self.bus.get_ppu().debug_ppu_state();
            }
            println!(
                "HEADLESS mode finished ({} / {} frames)",
                self.frame_count, self.headless_max_frames
            );
            // Optional WRAM dump after headless run
            if let Some(path) = std::env::var_os("DUMP_WRAM") {
                let path = std::path::PathBuf::from(path);
                match std::fs::write(&path, self.bus.wram()) {
                    Ok(_) => {
                        if !quiet {
                            println!(
                                "[dump_wram] wrote WRAM ({} bytes) to {}",
                                self.bus.wram().len(),
                                path.display()
                            );
                        }
                    }
                    Err(e) => eprintln!("[dump_wram] failed to write {}: {}", path.display(), e),
                }
            }
            self.save_sram_if_dirty();
            return;
        }

        while self.window.as_ref().map(|w| w.is_open()).unwrap_or(false)
            && !self
                .window
                .as_ref()
                .map(|w| w.is_key_down(Key::Escape))
                .unwrap_or(false)
        {
            if shutdown::should_quit() {
                break;
            }
            let frame_start = Instant::now();

            // Handle performance toggles
            if self
                .window
                .as_ref()
                .map(|w| w.is_key_pressed(Key::F1, minifb::KeyRepeat::No))
                .unwrap_or(false)
            {
                show_stats = !show_stats;
                println!(
                    "Performance stats: {}",
                    if show_stats { "ON" } else { "OFF" }
                );
            }

            if self
                .window
                .as_ref()
                .map(|w| w.is_key_pressed(Key::F2, minifb::KeyRepeat::No))
                .unwrap_or(false)
            {
                self.adaptive_timing = !self.adaptive_timing;
                println!(
                    "Adaptive timing: {}",
                    if self.adaptive_timing { "ON" } else { "OFF" }
                );
            }

            if self
                .window
                .as_ref()
                .map(|w| w.is_key_pressed(Key::F3, minifb::KeyRepeat::No))
                .unwrap_or(false)
            {
                let enabled = !self.audio_system.is_enabled();
                self.audio_system.set_enabled(enabled);
                println!("Audio: {}", if enabled { "ON" } else { "OFF" });
            }

            // F8: Force PPU test pattern (debug)
            // T: Force PPU test pattern (debug)
            if self
                .window
                .as_ref()
                .map(|w| w.is_key_pressed(Key::T, minifb::KeyRepeat::No))
                .unwrap_or(false)
            {
                self.bus.get_ppu_mut().force_test_pattern();
                println!("PPU: Forced test pattern (debug via 'T')");
            }

            // Volume controls:
            // - F4: volume down (Shift+F4: up)
            // - F6: volume up (Shift+F6: down)
            let f4 = self
                .window
                .as_ref()
                .map(|w| w.is_key_pressed(Key::F4, minifb::KeyRepeat::No))
                .unwrap_or(false);
            let f6 = self
                .window
                .as_ref()
                .map(|w| w.is_key_pressed(Key::F6, minifb::KeyRepeat::No))
                .unwrap_or(false);
            if f4 || f6 {
                let shift = self
                    .window
                    .as_ref()
                    .map(|w| w.is_key_down(Key::LeftShift) || w.is_key_down(Key::RightShift))
                    .unwrap_or(false);
                let cur = self.audio_system.get_volume();
                let inc = (f6 && !shift) || (f4 && shift);
                let new_v = if inc {
                    (cur + 0.1).min(1.0)
                } else {
                    (cur - 0.1).max(0.0)
                };
                self.audio_system.set_volume(new_v);
                println!("Volume: {:.0}%", new_v * 100.0);
            }

            // Check if we should skip this frame for performance
            let should_skip_frame = self.adaptive_timing
                && self.frame_skip_count < self.max_frame_skip
                && self.performance_stats.should_skip_frame(60.0);

            let force_render_for_title_screen = std::env::var("FORCE_RENDER_ALL")
                .map(|v| v == "1" || v.to_lowercase() == "true")
                .unwrap_or(false);
            let final_should_skip = should_skip_frame && !force_render_for_title_screen;

            if crate::debug_flags::render_verbose() {
                static mut FRAME_SKIP_DEBUG: u32 = 0;
                let fsd = unsafe {
                    FRAME_SKIP_DEBUG = FRAME_SKIP_DEBUG.wrapping_add(1);
                    FRAME_SKIP_DEBUG
                };
                if fsd <= 5 {
                    println!("🎬 FRAME SKIP DEBUG[{}]: adaptive_timing={}, should_skip_frame={}, final_should_skip={}", 
                            fsd, self.adaptive_timing, should_skip_frame, final_should_skip);
                }
            }

            if final_should_skip {
                self.frame_skip_count += 1;
                self.performance_stats.dropped_frames += 1;
                if crate::debug_flags::render_verbose() && !crate::debug_flags::quiet() {
                    println!("🎬 SKIPPING FRAME: skip_count={}", self.frame_skip_count);
                }
            } else {
                self.frame_skip_count = 0;
            }

            // Present/render cadence control (screen update can be expensive).
            let mut present_every = present_every_env.unwrap_or(2);
            if present_every_env.is_none() && auto_present {
                let fps = self.performance_stats.fps;
                let current = self.present_every_auto;
                let next = match current {
                    2 => {
                        if fps < 43.0 {
                            3
                        } else {
                            2
                        }
                    }
                    3 => {
                        if fps < 33.0 {
                            4
                        } else if fps > 50.0 {
                            2
                        } else {
                            3
                        }
                    }
                    4 => {
                        if fps > 40.0 {
                            3
                        } else {
                            4
                        }
                    }
                    _ => 2,
                };
                if next != current {
                    self.present_every_auto = next;
                    if !crate::debug_flags::quiet() {
                        println!(
                            "AUTO_PRESENT: fps={:.1} -> present_every={}",
                            fps, self.present_every_auto
                        );
                    }
                }
                present_every = self.present_every_auto.max(1);
            }

            let should_present = !final_should_skip
                && (present_every == 1 || (self.frame_count % present_every == 0));

            // Disable per-dot framebuffer rendering when we are not presenting to save time.
            if self.bus.get_ppu().framebuffer_rendering_enabled() != should_present {
                self.bus
                    .get_ppu_mut()
                    .set_framebuffer_rendering_enabled(should_present);
            }

            // Pump input/events early so auto-joypad reads see the current state.
            self.handle_input();
            self.handle_save_states();
            self.handle_debugger_input();

            // Always run emulation logic
            self.run_frame();

            // Only render/present when scheduled
            if should_present {
                self.render();
            }

            let frame_time = frame_start.elapsed();
            self.performance_stats.update(frame_time);

            if !self.headless {
                // Keep audio playback stable even when frames are skipped.
                if self.audio_system.is_enabled() || !should_skip_frame {
                    self.sync_frame_rate();
                }
            }

            self.frame_count += 1;
            self.maybe_dump_framebuffer_at();
            self.maybe_dump_mem_at();

            // Periodic SRAM autosave (optional)
            self.maybe_autosave_sram();

            // Display performance stats periodically
            if show_stats && stats_timer.elapsed() >= Duration::from_secs(2) {
                self.print_performance_stats();
                stats_timer = Instant::now();
            }

            // Compatibility boot fallback (windowed, permissive)
            // Use the same heuristic as headless to auto-unblank if a title sticks on forced blank.
            self.maybe_auto_unblank();
            // Optional hard override: force unblank between configured frames regardless of heuristics
            self.maybe_force_unblank();
            // Periodic minimal palette injection in windowed mode too
            self.maybe_inject_min_palette_periodic();

            // Optional visual fallback: draw PPU test pattern if still nothing visible by a threshold
            if std::env::var("BOOT_TEST_PATTERN")
                .map(|v| v == "1" || v.to_lowercase() == "true")
                .unwrap_or(false)
            {
                if self.frame_count >= 150 {
                    let non_black = {
                        let fb = self.bus.get_ppu().get_framebuffer();
                        fb.iter()
                            .filter(|&&px| px != 0xFF000000 && px != 0x00000000)
                            .count()
                    };
                    if non_black == 0 {
                        println!(
                            "VISUAL FALLBACK: Applying PPU test pattern (BOOT_TEST_PATTERN=1)"
                        );
                        self.bus.get_ppu_mut().force_test_pattern();
                    }
                }
            }

            // Early boot visibility: print simple PPU usage snapshots
            if self.frame_count == 60 || self.frame_count == 180 {
                let ppu = self.bus.get_ppu();
                println!(
                    "PPU usage @frame {}: VRAM {}/{} CGRAM {}/{} OAM {}/{}",
                    self.frame_count,
                    ppu.vram_usage(),
                    0x10000,
                    ppu.cgram_usage(),
                    0x200,
                    ppu.oam_usage(),
                    0x220
                );
            }
        }
        // Save SRAM on exit (window closed or Esc)
        self.save_sram_if_dirty();
    }

    // Force unblank regardless of game state (debug/compat aid)
    fn maybe_force_unblank(&mut self) {
        let env_enabled = std::env::var("BOOT_FORCE_UNBLANK")
            .map(|v| v == "1" || v.to_lowercase() == "true")
            .unwrap_or(false);
        if !env_enabled {
            return;
        }

        let force_always = std::env::var("BOOT_FORCE_UNBLANK_ALWAYS")
            .map(|v| v == "1" || v.to_lowercase() == "true")
            .unwrap_or(false);

        let from: u64 = std::env::var("BOOT_FORCE_UNBLANK_FROM")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(50); // Start earlier (was 90)
        let to: u64 = std::env::var("BOOT_FORCE_UNBLANK_TO")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(1000); // Extended range (was 600)

        if self.frame_count < from || self.frame_count > to {
            return;
        }

        let (imp, vwl, vwh, cg, oam) = self.bus.get_ppu().get_init_counters();
        let forced_blank = {
            let ppu = self.bus.get_ppu();
            ppu.is_forced_blank() || ppu.current_brightness() == 0
        };

        if !forced_blank && !force_always {
            return;
        }

        // Require minimal activity before unblanking to avoid premature intervention
        let has_activity = (vwl + vwh) > 100 || cg > 0 || oam > 0;
        if !force_always && !has_activity && self.frame_count < 200 {
            return;
        }

        let ppu = self.bus.get_ppu_mut();
        if crate::debug_flags::boot_verbose() || crate::debug_flags::compat() {
            println!(
                "🔆 FORCE-UNBLANK: frame={} (imp={} VRAM L/H={}/{} CGRAM={} OAM={})",
                self.frame_count, imp, vwl, vwh, cg, oam
            );
        }

        self.boot_fallback_applied = true;

        // Enable BG1 and set brightness to max (unblank). Write directly to fields to bypass IGNORE_INIDISP_CPU.
        ppu.screen_display = 0x0F;
        ppu.brightness = 0x0F;
        ppu.write(0x2C, 0x01); // TM: BG1 on
                               // Disable color math to avoid unintended global gray (halve/add) on fallback frames
        ppu.write(0x30, 0x00); // CGWSEL: clear
        ppu.write(0x31, 0x00); // CGADSUB: no layers selected
                               // Reset fixed color to black
        ppu.write(0x32, 0x00); // component=0 (no-op/blue=0)
        ppu.write(0x32, 0x20); // set green=0 (component=010) with intensity 0
        ppu.write(0x32, 0x40); // set red=0   (component=100) with intensity 0
                               // If CGRAM is still empty, inject minimal palette for visibility
        if ppu.cgram_usage() == 0 {
            ppu.write(0x21, 0x00); // CGADD=0
            ppu.write(0x22, 0xFF);
            ppu.write(0x22, 0x7F); // White
            ppu.write(0x22, 0x00);
            ppu.write(0x22, 0x7C); // Blue
            ppu.write(0x22, 0x1F);
            ppu.write(0x22, 0x00); // Red
            ppu.write(0x22, 0xE0);
            ppu.write(0x22, 0x03); // Green
        }

        // If somehow brightness is still 0 while forced blank is off, bump it.
        if ppu.current_brightness() == 0 && !ppu.is_forced_blank() {
            ppu.screen_display = 0x0F;
            ppu.brightness = 0x0F;
        }
    }

    fn save_sram_if_dirty(&mut self) {
        let no_sram_save = std::env::var("NO_SRAM_SAVE")
            .map(|v| v == "1" || v.to_lowercase() == "true")
            .unwrap_or(false);
        if no_sram_save {
            return;
        }
        if let Some(ref path) = self.srm_path {
            if self.bus.is_sram_dirty() {
                if let Err(e) = std::fs::write(path, self.bus.sram()) {
                    eprintln!("Failed to save SRAM to {}: {}", path.display(), e);
                } else {
                    println!(
                        "SRAM saved to {} ({} bytes)",
                        path.display(),
                        self.bus.sram().len()
                    );
                    self.bus.clear_sram_dirty();
                }
            }
        }
    }

    fn maybe_autosave_sram(&mut self) {
        if let (Some(every), Some(ref path)) = (self.srm_autosave_every, self.srm_path.as_ref()) {
            if self
                .frame_count
                .saturating_sub(self.srm_last_autosave_frame)
                >= every
                && self.bus.is_sram_dirty()
            {
                let tmp = {
                    let mut p = path.to_path_buf();
                    p.set_extension("srm.tmp");
                    p
                };
                let write_ok =
                    std::fs::write(&tmp, self.bus.sram()).and_then(|_| std::fs::rename(&tmp, path));
                match write_ok {
                    Ok(_) => {
                        println!(
                            "SRAM autosaved to {} (every {} frames)",
                            path.display(),
                            every
                        );
                        self.srm_last_autosave_frame = self.frame_count;
                        // Keep dirty true; we will still flush on exit.
                    }
                    Err(e) => eprintln!("SRAM autosave failed ({}): {}", path.display(), e),
                }
            }
        }
    }

    // Auto-unblank helper gated by env controls. Runs at specific frame thresholds.
    fn maybe_auto_unblank(&mut self) {
        if self.boot_fallback_applied {
            return;
        }
        let enabled = std::env::var("COMPAT_BOOT_FALLBACK")
            .map(|v| v == "1" || v.to_lowercase() == "true")
            .unwrap_or(false);
        if !enabled {
            return;
        }
        let threshold: u64 = std::env::var("COMPAT_AUTO_UNBLANK_FRAME")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(120);
        let second: u64 = threshold.saturating_mul(2);
        let third: u64 = threshold.saturating_mul(3);
        if !(self.frame_count == threshold
            || self.frame_count == second
            || self.frame_count == third)
        {
            return;
        }

        // Heuristics: plenty of VRAM writes and zero CGRAM writes yet
        let (_imp, vwl, vwh, cg, oam) = self.bus.get_ppu().get_init_counters();
        let vram_min: u64 = std::env::var("COMPAT_AUTO_UNBLANK_VRAM_MIN")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(4096);
        let cgram_max: u64 = std::env::var("COMPAT_AUTO_UNBLANK_CGRAM_MAX")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        let minimal_activity = (vwl + vwh) > 0 || cg > 0 || oam > 0;
        if cg > cgram_max || (vwl + vwh) <= vram_min {
            // Heuristics did not pass. As a last resort, if we've waited long enough and
            // the display is still blank while there is at least some activity, unblank anyway.
            let late_fallback = self.frame_count >= third && minimal_activity;
            if !late_fallback {
                return;
            }
        }

        // If the framebuffer already contains visible pixels, prefer unblanking.
        let (forced_blank, brightness, non_black_pixels) = {
            let ppu = self.bus.get_ppu();
            let fb = ppu.get_framebuffer();
            let nb = fb
                .iter()
                .take(256 * 224)
                .filter(|&&px| px != 0xFF000000)
                .count();
            ((ppu.screen_display & 0x80) != 0, ppu.brightness, nb)
        };
        if !forced_blank {
            return;
        }

        // Also require that the game touched OAM a bit (sprites prepped), but keep this lenient
        // Keep permissive: allow unblank even if framebuffer is still black
        if oam == 0 && non_black_pixels == 0 && self.frame_count < third {
            return;
        }

        if crate::debug_flags::boot_verbose() || crate::debug_flags::compat() {
            println!(
                "COMPAT: Auto-unblank at frame {} (VRAM L/H={} / {}, CGRAM={}, OAM={}, non_black={} ; brightness={}).",
                self.frame_count, vwl, vwh, cg, oam, non_black_pixels, brightness
            );
            println!("        Forcing INIDISP=0x0F, TM=BG1 (fallback)");
        }
        let ppu_mut = self.bus.get_ppu_mut();
        ppu_mut.write(0x2C, 0x01); // TM: BG1 on
        ppu_mut.write(0x00, 0x0F); // INIDISP: brightness 15, unblank
                                   // Also disable color math to avoid global gray when palette is not ready yet
        ppu_mut.write(0x30, 0x00); // CGWSEL: clear
        ppu_mut.write(0x31, 0x00); // CGADSUB: no layers selected

        let do_palette = std::env::var("COMPAT_INJECT_MIN_PALETTE")
            .map(|v| v == "1" || v.to_lowercase() == "true")
            .unwrap_or(false);
        if do_palette {
            if crate::debug_flags::boot_verbose() || crate::debug_flags::compat() {
                println!("COMPAT: Injecting minimal CGRAM palette (fallback)");
            }
            ppu_mut.write(0x21, 0x00); // CGADD=0
                                       // Color 0: White (backdrop visible)
            ppu_mut.write(0x22, 0xFF);
            ppu_mut.write(0x22, 0x7F);
            // Color 1: Blue
            ppu_mut.write(0x22, 0x00);
            ppu_mut.write(0x22, 0x7C);
            // Color 2: Red
            ppu_mut.write(0x22, 0x1F);
            ppu_mut.write(0x22, 0x00);
            // Color 3: Green
            ppu_mut.write(0x22, 0xE0);
            ppu_mut.write(0x22, 0x03);
        }
        self.boot_fallback_applied = true;
    }

    fn run_frame(&mut self) {
        // Run exactly one NTSC PPU frame worth of master cycles:
        // 341 dots/line * 262 lines/frame * 4 master cycles/dot.
        //
        // Using MASTER_CLOCK_NTSC/60.0 causes drift vs the PPU’s actual frame length
        // (NTSC SNES is ~60.0988Hz). That drift shows up as tearing/corrupted headless
        // frame dumps because we present mid-scanline in a rotating phase.
        const DOTS_PER_LINE_NTSC: u64 = 341;
        const SCANLINES_PER_FRAME_NTSC: u64 = 262;
        let cycles_per_frame = DOTS_PER_LINE_NTSC
            .saturating_mul(SCANLINES_PER_FRAME_NTSC)
            .saturating_mul(PPU_CLOCK_DIVIDER);
        let start_cycles = self.master_cycles;

        static mut FRAME_COUNT: u32 = 0;
        let frame_count = unsafe {
            FRAME_COUNT = FRAME_COUNT.wrapping_add(1);
            FRAME_COUNT
        };
        if frame_count <= 2 && !crate::debug_flags::quiet() {
            println!(
                "Frame {}: cycles_per_frame={}, start_cycles={}",
                frame_count, cycles_per_frame, start_cycles
            );
        }

        // --- NMI 抑止ガード（デフォルトOFF） ---
        // 特殊なROMの初期化用に環境変数でのみ有効化する。
        let nmi_guard_frames: u32 = std::env::var("NMI_GUARD_FRAMES")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);
        if frame_count <= nmi_guard_frames {
            self.bus.get_ppu_mut().nmi_enabled = false;
            // pending NMI を必ずクリア
            let _ = self.bus.read_u8(0x4210);
        } else if frame_count == nmi_guard_frames + 1 {
            self.bus.get_ppu_mut().nmi_enabled = true;
            let _ = self.bus.read_u8(0x4210);
        }
        // Optional: dump S-CPU PC for early-frame debugging (enable via SHOW_PC=1)
        if std::env::var_os("SHOW_PC").is_some() && frame_count <= 16 {
            println!(
                "[pc] frame={} S-CPU PC=${:02X}:{:04X} P=0x{:02X} I={}",
                frame_count,
                self.cpu.pb,
                self.cpu.pc,
                self.cpu.p.bits(),
                (self.cpu.p.bits() & 0x04) != 0
            );
        }
        // Debug: S-CPU P/Iフラグを冒頭で確認
        if std::env::var_os("DEBUG_CPU_FLAGS").is_some() && frame_count <= 8 {
            println!(
                "[cpu-flags] frame={} PC={:02X}:{:04X} P=0x{:02X} I={}",
                frame_count,
                self.cpu.pb,
                self.cpu.pc,
                self.cpu.p.bits(),
                (self.cpu.p.bits() & 0x04) != 0
            );
        }

        // Debug hack: force clear I-flag on early frames if requested (env FORCE_CLI_FRAMES=N)
        if let Some(n) = std::env::var("FORCE_CLI_FRAMES")
            .ok()
            .and_then(|v| v.parse::<u32>().ok())
        {
            if frame_count <= n {
                let mut p = self.cpu.p;
                p.remove(crate::cpu::StatusFlags::IRQ_DISABLE);
                self.cpu.p = p;
                self.cpu.core.state_mut().p = p;
                if std::env::var_os("DEBUG_CPU_FLAGS").is_some() {
                    println!(
                        "[cpu-flags] forced CLI at frame={} PC={:02X}:{:04X}",
                        frame_count, self.cpu.pb, self.cpu.pc
                    );
                }
            }
        }

        // Debug: Show frame progress every 5 frames
        if frame_count % 5 == 0 && frame_count > 2 {
            if crate::debug_flags::render_verbose() {
                println!("Frame progress: {}/10", frame_count);
            }
        }

        // Debug: Track loop iterations to detect infinite loops
        let mut loop_iterations = 0u64;
        // ループ検出の許容回数を環境変数で調整できるようにする。
        // ・初期フレーム(<=3): 5,000,000（VBlank待ちなどで多少重くても落とさない）
        // ・重いトレース有効時: 50,000,000 まで許容（WATCH_PC/TRACE_4210/TRACE_BRANCH など）
        // ・通常: 1,000,000
        // LOOP_GUARD_MAX を指定するとその値を上書きする（デバッグ用）。
        let tracing_heavy = std::env::var_os("TRACE_4210").is_some()
            || std::env::var_os("WATCH_PC").is_some()
            || std::env::var_os("WATCH_PC_FLOW").is_some()
            || std::env::var_os("TRACE_BRANCH").is_some();
        let default_max = if tracing_heavy {
            50_000_000
        } else if frame_count <= 3 {
            5_000_000
        } else {
            1_000_000
        };
        let max_iterations: u64 = std::env::var("LOOP_GUARD_MAX")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(default_max);

        // Optional stall detector: TRACE_STALL=<N> logs when同一PCがN回連続
        let stall_threshold: u32 = std::env::var("TRACE_STALL")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);
        let mut stall_pc: u32 = 0;
        let mut stall_count: u32 = 0;
        let mut stall_ring: [u32; 16] = [0; 16];
        let mut stall_ring_pos: usize = 0;
        let mut stall_last_diff: u32 = 0;

        // Debug: force SA-1 IRQ to S-CPU each frame if requested
        if crate::debug_flags::sa1_force_irq_each_frame() && self.bus.is_sa1_active() {
            self.bus.sa1_mut().registers.interrupt_pending |= crate::sa1::Sa1::IRQ_LINE_BIT;
        }
        // Debug: force S-CPU IRQ for first N frames if requested
        if let Some(n) = crate::debug_flags::force_scpu_irq_frames() {
            if frame_count <= n {
                self.cpu.trigger_irq(&mut self.bus);
            }
        }

        // PERF_VERBOSE=1 enables expensive per-instruction timing/profiling.
        // Default is off to keep headless regression tests fast.
        let perf_verbose = std::env::var("PERF_VERBOSE")
            .map(|v| v == "1" || v.to_lowercase() == "true")
            .unwrap_or(false);
        // CPU_BATCH is opt-in; reading env per instruction is expensive, so cache per frame.
        let batch_exec = std::env::var("CPU_BATCH")
            .map(|v| v == "1" || v.to_lowercase() == "true")
            .unwrap_or(false);
        // Instruction trace is extremely expensive; keep it opt-in.
        let trace_exec = std::env::var("TRACE_EXEC")
            .map(|v| v == "1" || v.to_lowercase() == "true")
            .unwrap_or(false);
        // These are checked in the inner loop; cache once per frame.
        let trace_pc_ffff = std::env::var_os("TRACE_PC_FFFF").is_some();
        let trace_pc_frame = std::env::var_os("TRACE_PC_FRAME").is_some();
        let trace_loop_cycles = std::env::var_os("TRACE_LOOP_CYCLES").is_some();
        let trace_pc_ffff_once = std::env::var_os("TRACE_PC_FFFF_ONCE").is_some();

        // SMW compatibility hack (debug-only): cache env read to avoid per-instruction overhead.
        let smw_force_bbaa = std::env::var("SMW_FORCE_BBAA")
            .map(|v| v == "1" || v.to_lowercase() == "true")
            .unwrap_or(false);

        while self.master_cycles - start_cycles < cycles_per_frame {
            loop_iterations += 1;
            if loop_iterations > max_iterations {
                eprintln!(
                    "FATAL: Frame {} exceeded {} loop iterations! Possible infinite loop.",
                    frame_count, max_iterations
                );
                eprintln!(
                    "  master_cycles={}, start_cycles={}, target={}",
                    self.master_cycles, start_cycles, cycles_per_frame
                );
                eprintln!(
                    "  CPU PC={:02X}:{:04X}",
                    self.cpu.get_pc() >> 16,
                    self.cpu.get_pc() & 0xFFFF
                );
                eprintln!(
                    "  CPU waiting_for_irq={}, stopped={}",
                    self.cpu.core.state().waiting_for_irq,
                    self.cpu.core.state().stopped
                );

                // Print last 10 loop iteration details
                if frame_count >= 997 {
                    eprintln!("\n  Collecting final diagnostics...");
                    for i in 0..10 {
                        let pc = self.cpu.get_pc();
                        let opcode = self.bus.read_u8(pc);
                        let cpu_cycles = self.cpu.step(&mut self.bus);
                        eprintln!(
                            "    Loop {}: PC={:02X}:{:04X} opcode=0x{:02X} cycles={}",
                            loop_iterations + i + 1,
                            pc >> 16,
                            pc & 0xFFFF,
                            opcode,
                            cpu_cycles
                        );
                        if cpu_cycles == 0 {
                            eprintln!("    WARNING: CPU returned 0 cycles!");
                            break;
                        }
                    }
                }
                std::process::exit(1);
            }

            // If a previous instruction triggered a DMA stall, the CPU is halted while time
            // continues to advance. Consume that stall budget here before running more CPU.
            if self.pending_stall_master_cycles > 0 {
                let remaining = cycles_per_frame.saturating_sub(self.master_cycles - start_cycles);
                let consume = self.pending_stall_master_cycles.min(remaining);
                self.advance_time_without_cpu(consume);
                self.pending_stall_master_cycles -= consume;
                self.pending_stall_master_cycles = self
                    .pending_stall_master_cycles
                    .saturating_add(self.bus.take_pending_stall_master_cycles());
                continue;
            }

            // デバッガのブレークポイントチェック
            let pc = self.cpu.get_pc();

            // Debug: when PC gets stuck at $FFFF (seen in SMW init), dump state a few times
            if trace_pc_ffff && pc == 0x00FFFF {
                use std::sync::atomic::{AtomicU32, Ordering};
                static COUNT_FFFF: AtomicU32 = AtomicU32::new(0);
                let n = COUNT_FFFF.fetch_add(1, Ordering::Relaxed);
                if n < 8 {
                    let st = self.cpu.core.state();
                    println!(
                        "[PCFFFF] frame={} count={} wait_irq={} stopped={} A={:04X} X={:04X} Y={:04X} SP={:04X} P={:02X} DB={:02X} DP={:04X}",
                        frame_count,
                        n,
                        st.waiting_for_irq,
                        st.stopped,
                        st.a,
                        st.x,
                        st.y,
                        st.sp,
                        st.p.bits(),
                        st.db,
                        st.dp
                    );
                }
            }
            if self.debugger.check_breakpoint(pc) {
                return; // ブレークポイントでフレーム処理を中断
            }

            if stall_threshold > 0 {
                stall_ring[stall_ring_pos] = pc;
                stall_ring_pos = (stall_ring_pos + 1) & 0x0F;
                if pc == stall_pc {
                    stall_count = stall_count.saturating_add(1);
                    if stall_count == stall_threshold {
                        let mut recent = Vec::new();
                        for i in 0..stall_ring.len() {
                            let idx = (stall_ring_pos + i) & 0x0F;
                            recent.push(format!(
                                "{:02X}:{:04X}",
                                stall_ring[idx] >> 16,
                                stall_ring[idx] & 0xFFFF
                            ));
                        }
                        println!(
                            "[STALL] frame={} PC={:02X}:{:04X} last_diff={:02X}:{:04X} A={:04X} X={:04X} Y={:04X} SP={:04X} P={:02X} DB={:02X} DP={:04X} recent=[{}]",
                            frame_count,
                            self.cpu.pb,
                            self.cpu.pc,
                            stall_last_diff >> 16,
                            stall_last_diff & 0xFFFF,
                            self.cpu.a,
                            self.cpu.x,
                            self.cpu.y,
                            self.cpu.sp,
                            self.cpu.p.bits(),
                            self.cpu.db,
                            self.cpu.dp,
                            recent.join(", ")
                        );
                        stall_count = 0;
                    }
                } else {
                    stall_last_diff = stall_pc;
                    stall_pc = pc;
                    stall_count = 0;
                }
            }

            // Optional: per-frame PC trace for coarse progress
            if trace_pc_frame {
                static mut LAST_LOGGED_FRAME: u32 = 0;
                // Avoid spamming: log once per frame at loop top
                unsafe {
                    if LAST_LOGGED_FRAME != frame_count {
                        LAST_LOGGED_FRAME = frame_count;
                        println!(
                            "[frame_pc] frame={} PC={:02X}:{:04X} A=0x{:04X} X=0x{:04X} Y=0x{:04X} P=0x{:02X} JOYBUSY={}",
                            frame_count,
                            pc >> 16,
                            pc & 0xFFFF,
                            self.cpu.a,
                            self.cpu.x,
                            self.cpu.y,
                            self.cpu.p.bits(),
                            self.bus.joy_busy_counter()
                        );
                    }
                }
            }

            // burn-in-test.sfc: annotate the ROM-side OBJ overflow checks with current PPU timing.
            // (This helps distinguish "PPU flag wrong" vs "check happens before scanline".)
            if crate::debug_flags::trace_burnin_obj_checks()
                && pc >> 16 == 0x00
                && matches!(
                    pc & 0xFFFF,
                    0x9AC4 | 0x9AEC | 0x9B61 | 0x9B8E | 0x9BD0 | 0x9BD8
                )
            {
                let ppu_frame = self.bus.get_ppu().get_frame();
                let ppu_sl = self.bus.get_ppu().scanline;
                let ppu_cyc = self.bus.get_ppu().get_cycle();
                let ppu_vblank = self.bus.get_ppu().is_vblank() as u8;
                let hvbjoy = self.bus.read_u8(0x4212);
                let stat77 = self.bus.read_u8(0x213E);
                println!(
                    "[BURNIN-OBJ-CHECK-CTX] PC=00:{:04X} frame={} sl={} cyc={} vblank={} hvbjoy={:02X} stat77={:02X}",
                    pc & 0xFFFF,
                    ppu_frame,
                    ppu_sl,
                    ppu_cyc,
                    ppu_vblank,
                    hvbjoy,
                    stat77
                );
            }

            // Watch a specific address read/write (S-CPU side) if requested
            if let Some(watch) = crate::debug_flags::watch_addr() {
                let wbank = (watch >> 16) as u8;
                let woff = (watch & 0xFFFF) as u16;
                let val = self.bus.read_u8(((wbank as u32) << 16) | woff as u32);
                println!(
                    "[watch] frame={} addr={:02X}:{:04X} val={:02X} PC={:02X}:{:04X}",
                    frame_count, wbank, woff, val, self.cpu.pb, self.cpu.pc
                );
            }

            // デバッガが一時停止中かチェック
            if self.debugger.is_paused() && !self.debugger.should_step() {
                return; // 一時停止中
            }

            // SMW (LoROM) bootstrap guard: seed 7E:BBAA with 0xBBAA to escape the early self-check loop.
            if smw_force_bbaa {
                let addr = 0x00BBAAusize;
                // Cover both WRAM banks 7E/7F in case DB is set later.
                for bank in &[0x7E0000u32, 0x7F0000u32] {
                    self.bus.write_u8(bank + addr as u32, 0xAA);
                    self.bus.write_u8(bank + addr as u32 + 1, 0xBB);
                }
            }

            // Use batch execution for better performance (デバッグモードでない場合)
            let remaining_cycles = cycles_per_frame - (self.master_cycles - start_cycles);
            let mut batch_cycles = if self.debugger.is_paused() {
                1 // デバッグモードでは1命令ずつ実行
            } else {
                (remaining_cycles / CPU_CLOCK_DIVIDER).min(32) as u8
            };
            // 端数で0になってしまうとフレーム末尾で永久ループに入るため、最低1サイクルは回す
            if batch_cycles == 0 {
                batch_cycles = 1;
            }

            // Optional: record instruction trace (debugger/TRACE_EXEC=1)
            let record_trace = trace_exec || self.debugger.is_paused();
            let need_opcode = record_trace || trace_loop_cycles || trace_pc_ffff_once;
            let opcode = if need_opcode { self.bus.read_u8(pc) } else { 0 };
            if record_trace {
                let operands = self.fetch_operands(pc, opcode);
                self.debugger
                    .record_trace(&self.cpu, &self.bus, opcode, &operands);
            }

            let before_pc = pc;
            // Batch execution is a performance optimization but it breaks timing-sensitive
            // software (e.g., official burn-in HV latch tests) because PPU/APU are stepped only
            // once per batch. Keep it opt-in.
            let cpu_cycles = if perf_verbose {
                let cpu_start = Instant::now();
                let cycles = if batch_exec
                    && batch_cycles > 8
                    && self.adaptive_timing
                    && !self.debugger.is_paused()
                {
                    self.cpu.step_multiple(&mut self.bus, batch_cycles)
                } else {
                    self.cpu.step(&mut self.bus)
                };
                let cpu_time = cpu_start.elapsed();
                self.performance_stats.add_cpu_time(cpu_time);
                cycles
            } else if batch_exec
                && batch_cycles > 8
                && self.adaptive_timing
                && !self.debugger.is_paused()
            {
                self.cpu.step_multiple(&mut self.bus, batch_cycles)
            } else {
                self.cpu.step(&mut self.bus)
            };
            if trace_loop_cycles && loop_iterations < 20 {
                println!(
                    "[loop] iter={} cpu_cycles={} master_cycles={} pc={:02X}:{:04X}",
                    loop_iterations + 1,
                    cpu_cycles,
                    self.master_cycles,
                    before_pc >> 16,
                    before_pc & 0xFFFF
                );
            }
            let after_pc = self.cpu.get_pc();
            if trace_pc_ffff_once && before_pc != 0x00FF_FF && after_pc == 0x00FF_FF {
                static mut LAST_GOOD_PC: u32 = 0;
                unsafe {
                    println!(
                        "[PCFFFF-TRANS] frame={} from {:02X}:{:04X} opcode={:02X} A={:04X} X={:04X} Y={:04X} SP={:04X} P={:02X} DB={:02X} DP={:04X} last_good={:02X}:{:04X}",
                        frame_count,
                        before_pc >> 16,
                        before_pc & 0xFFFF,
                        opcode,
                        self.cpu.a,
                        self.cpu.x,
                        self.cpu.y,
                        self.cpu.sp,
                        self.cpu.p.bits(),
                        self.cpu.db,
                        self.cpu.dp,
                        LAST_GOOD_PC >> 16,
                        LAST_GOOD_PC & 0xFFFF
                    );
                    LAST_GOOD_PC = before_pc;
                }
            } else if trace_pc_ffff_once {
                static mut LAST_GOOD_PC: u32 = 0;
                unsafe {
                    LAST_GOOD_PC = before_pc;
                }
            }

            if perf_verbose {
                // Measure SA-1 execution time
                let sa1_start = Instant::now();
                self.bus.run_sa1_scheduler(cpu_cycles);
                // Process any pending SA-1 DMA/CC-DMA transfers after SA-1 execution
                self.bus.process_sa1_dma();
                let sa1_time = sa1_start.elapsed();
                self.performance_stats.add_sa1_time(sa1_time);
            } else {
                self.bus.run_sa1_scheduler(cpu_cycles);
                // Process any pending SA-1 DMA/CC-DMA transfers after SA-1 execution
                self.bus.process_sa1_dma();
            }

            // CPU:PPU=6:4 の比率で進める。端数は master cycles の余りとして保持してロスを防ぐ。
            let master = (cpu_cycles as u64)
                .saturating_mul(CPU_CLOCK_DIVIDER)
                .saturating_add(self.ppu_cycle_accum as u64);
            let mut ppu_cycles = (master / PPU_CLOCK_DIVIDER) as u16;
            self.ppu_cycle_accum = (master % PPU_CLOCK_DIVIDER) as u8;
            // CPUがビジーループで止まっていても時間を進めるため、最低1サイクルは進める
            if ppu_cycles == 0 {
                ppu_cycles = 1;
            }
            if perf_verbose {
                // Measure PPU rendering time
                let ppu_start = Instant::now();
                self.step_ppu(ppu_cycles, true);
                let ppu_time = ppu_start.elapsed();
                self.performance_stats.add_ppu_time(ppu_time);
            } else {
                self.step_ppu(ppu_cycles, true);
            }

            // APU も更新
            let apu_cycles = cpu_cycles; // APUはCPUと同じクロック
            if !self.bus.is_fake_apu() {
                self.apu_cycle_debt = self
                    .apu_cycle_debt
                    .saturating_add(apu_cycles as u32);
                if self.apu_cycle_debt >= self.apu_step_batch {
                    self.step_apu_debt(false);
                }
                if self.apu_cycle_debt >= self.apu_step_force {
                    self.step_apu_debt(true);
                }
            }

            // NMI/IRQ は CPU 側の poll_nmi/service_nmi/service_irq で処理する。
            // ここで手動トリガ/クリアすると IRQ のレベル維持が崩れるため触らない。

            self.master_cycles += (cpu_cycles as u64) * CPU_CLOCK_DIVIDER;

            // Drain any time consumed by DMA stalls that occurred during this instruction slice.
            // The CPU should remain halted for that duration, but PPU/APU continue to advance.
            self.pending_stall_master_cycles = self
                .pending_stall_master_cycles
                .saturating_add(self.bus.take_pending_stall_master_cycles());
        }

        // Frame完了時に主要レジスタのサマリを出力（デバッグ用）
        self.maybe_dump_register_summary(frame_count);

        // Flush any remaining APU cycles at end-of-frame to keep audio in sync.
        self.step_apu_debt(true);

        if self.audio_system.is_enabled() && !self.bus.is_fake_apu() {
            let audio_system = &mut self.audio_system;
            self.bus.with_apu_mut(|apu| {
                audio_system.mix_frame_from_apu(apu);
            });
        }
    }

    /// Advance emulated time without executing any S-CPU instructions.
    ///
    /// Used to model stalls such as general DMA (MDMA), where the S-CPU is halted while
    /// the PPU/APU (and SA-1) continue to run.
    fn advance_time_without_cpu(&mut self, master_cycles: u64) {
        if master_cycles == 0 {
            return;
        }

        // Step SA-1 scheduler (if present) during the stall.
        // Use S-CPU cycle equivalents as a rough proxy for elapsed time.
        let mut sa1_cycles = master_cycles / CPU_CLOCK_DIVIDER;
        while sa1_cycles > 0 {
            let chunk = sa1_cycles.min(u8::MAX as u64) as u8;
            self.bus.run_sa1_scheduler(chunk);
            self.bus.process_sa1_dma();
            sa1_cycles -= chunk as u64;
        }

        // Step PPU: PPU clock is master/4.
        //
        // IMPORTANT: Preserve master->PPU fractional remainder across CPU and stall paths.
        // Otherwise, repeated small stalls (e.g., slow memory extra master cycles) will
        // systematically drop remainder and cause video timing drift (tearing in dumps).
        let master_with_rem = master_cycles.saturating_add(self.ppu_cycle_accum as u64);
        let mut ppu_cycles = master_with_rem / PPU_CLOCK_DIVIDER;
        self.ppu_cycle_accum = (master_with_rem % PPU_CLOCK_DIVIDER) as u8;
        while ppu_cycles > 0 {
            let chunk = ppu_cycles.min(u16::MAX as u64) as u16;
            self.step_ppu(chunk, false);
            ppu_cycles -= chunk as u64;
        }

        // Step APU for the same elapsed master time.
        if !self.bus.is_fake_apu() {
            let total = master_cycles.saturating_add(self.apu_master_cycle_accum as u64);
            let apu_cpu_cycles = (total / CPU_CLOCK_DIVIDER) as u32;
            self.apu_master_cycle_accum = (total % CPU_CLOCK_DIVIDER) as u8;
            self.apu_cycle_debt = self.apu_cycle_debt.saturating_add(apu_cpu_cycles);
            if self.apu_cycle_debt >= self.apu_step_batch {
                self.step_apu_debt(false);
            }
            if self.apu_cycle_debt >= self.apu_step_force {
                self.step_apu_debt(true);
            }
        }

        self.master_cycles = self.master_cycles.saturating_add(master_cycles);
    }

    fn step_apu_debt(&mut self, _force: bool) {
        if self.bus.is_fake_apu() {
            self.apu_cycle_debt = 0;
            return;
        }
        let debt = self.apu_cycle_debt;
        if debt == 0 {
            return;
        }

        let step_fn = |apu: &mut crate::apu::Apu| {
            let mut remaining = debt;
            while remaining > 0 {
                let chunk = remaining.min(u8::MAX as u32) as u8;
                apu.step(chunk);
                remaining -= chunk as u32;
            }
        };

        // Always step APU when we have debt; this keeps handshakes progressing reliably.
        // The audio thread will opportunistically try_lock instead of blocking the emulator.
        self.bus.with_apu_mut(step_fn);
        self.apu_cycle_debt = 0;
    }

    /// 主要PPUレジスタの変遷を出力（回帰検出用）
    fn maybe_dump_register_summary(&self, frame: u32) {
        // 環境変数で制御
        let enabled = std::env::var("DUMP_REGISTER_SUMMARY")
            .map(|v| v == "1" || v.to_lowercase() == "true")
            .unwrap_or(false);

        if !enabled {
            return;
        }

        // 特定フレームのみ出力（環境変数で指定可能）
        let target_frames: Vec<u32> = std::env::var("DUMP_REGISTER_FRAMES")
            .ok()
            .and_then(|s| {
                s.split(',')
                    .filter_map(|n| n.trim().parse::<u32>().ok())
                    .collect::<Vec<_>>()
                    .into()
            })
            .unwrap_or_else(|| vec![60, 120, 180, 240, 300, 360, 500, 1000]);

        if !target_frames.contains(&frame) {
            return;
        }

        let ppu = self.bus.get_ppu();
        println!("\n━━━━ REGISTER SUMMARY @ Frame {} ━━━━", frame);
        println!(
            "  INIDISP:    0x{:02X} (blank={} brightness={})",
            if ppu.is_forced_blank() {
                0x80 | ppu.current_brightness()
            } else {
                ppu.current_brightness()
            },
            if ppu.is_forced_blank() { "ON " } else { "OFF" },
            ppu.current_brightness()
        );
        println!(
            "  TM (main):  0x{:02X} (BG1={} BG2={} BG3={} BG4={} OBJ={})",
            ppu.get_main_screen_designation(),
            (ppu.get_main_screen_designation() & 0x01) != 0,
            (ppu.get_main_screen_designation() & 0x02) != 0,
            (ppu.get_main_screen_designation() & 0x04) != 0,
            (ppu.get_main_screen_designation() & 0x08) != 0,
            (ppu.get_main_screen_designation() & 0x10) != 0
        );
        println!("  BG mode:    {}", ppu.get_bg_mode());
        let setini = ppu.get_setini();
        println!(
            "  SETINI:     0x{:02X} (interlace={} obj_interlace={} pseudo_hires={} overscan={} extbg={})",
            setini,
            (setini & 0x01) != 0,
            (setini & 0x02) != 0,
            (setini & 0x08) != 0,
            (setini & 0x04) != 0,
            (setini & 0x40) != 0
        );

        let main_tm = ppu.get_main_screen_designation();
        if (main_tm & 0x01) != 0 {
            let (tile_base, map_base, tile_16, screen_size) = ppu.get_bg_config(1);
            let size_desc = match screen_size {
                0 => "32x32",
                1 => "64x32",
                2 => "32x64",
                3 => "64x64",
                _ => "???",
            };
            println!(
                "  BG1 config: tile_base=0x{:04X} map_base=0x{:04X} tile_size={} screen={}",
                tile_base,
                map_base,
                if tile_16 { "16x16" } else { "8x8" },
                size_desc
            );
        }

        if (main_tm & 0x08) != 0 {
            let (tile_base, map_base, tile_16, screen_size) = ppu.get_bg_config(4);
            let size_desc = match screen_size {
                0 => "32x32",
                1 => "64x32",
                2 => "32x64",
                3 => "64x64",
                _ => "???",
            };
            println!(
                "  BG4 config: tile_base=0x{:04X} map_base=0x{:04X} tile_size={} screen={}",
                tile_base,
                map_base,
                if tile_16 { "16x16" } else { "8x8" },
                size_desc
            );
        }

        // BG3 configuration
        if (main_tm & 0x04) != 0 {
            // BG3 enabled
            let (tile_base, map_base, tile_16, screen_size) = ppu.get_bg_config(3);
            let size_desc = match screen_size {
                0 => "32x32",
                1 => "64x32",
                2 => "32x64",
                3 => "64x64",
                _ => "???",
            };
            println!(
                "  BG3 config: tile_base=0x{:04X} map_base=0x{:04X} tile_size={} screen={}",
                tile_base,
                map_base,
                if tile_16 { "16x16" } else { "8x8" },
                size_desc
            );

            // Check actual data in tile and map regions
            let (tile_nonzero, tile_samples) = ppu.analyze_vram_region(tile_base, 512);
            let (map_nonzero, map_samples) = ppu.analyze_vram_region(map_base, 512);
            println!(
                "    └─ Tile data @ 0x{:04X}: {} nonzero bytes, samples: {:02X?}...",
                tile_base,
                tile_nonzero,
                &tile_samples[..tile_samples.len().min(8)]
            );
            println!("    └─ Map  data @ 0x{:04X}: {} nonzero bytes (512 words checked), samples: {:02X?}...",
                map_base, map_nonzero, &map_samples[..map_samples.len().min(8)]
            );
        }

        // VRAM analysis
        let (vram_nonzero, vram_unique, vram_samples) = ppu.analyze_vram_content();
        println!(
            "  VRAM usage: {}/{} bytes ({:.1}%)",
            vram_nonzero,
            65536,
            (vram_nonzero as f64 / 65536.0) * 100.0
        );
        if vram_nonzero > 0 {
            println!(
                "    └─ {} unique values, samples: {:?}...",
                vram_unique,
                &vram_samples[..vram_samples.len().min(5)]
            );

            // Show VRAM distribution by 4KB blocks
            let distribution = ppu.get_vram_distribution();
            println!("    └─ Distribution by 4KB blocks (word addresses):");
            for (word_addr, count) in distribution.iter() {
                println!("       0x{:04X}: {} bytes", word_addr, count);
            }
        }

        println!(
            "  CGRAM usage: {}/{} bytes ({:.1}%)",
            ppu.cgram_usage(),
            512,
            (ppu.cgram_usage() as f64 / 512.0) * 100.0
        );
        println!(
            "  OAM usage:  {}/{} bytes ({:.1}%)",
            ppu.oam_usage(),
            544,
            (ppu.oam_usage() as f64 / 544.0) * 100.0
        );

        // フレームバッファの統計
        let fb = ppu.get_framebuffer();
        let non_black = fb
            .iter()
            .take(256 * 224)
            .filter(|&&px| px != 0xFF000000 && px != 0x00000000)
            .count();
        println!(
            "  Non-black pixels: {} ({:.1}%)",
            non_black,
            (non_black as f64 / (256.0 * 224.0)) * 100.0
        );
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
    }

    fn step_ppu(&mut self, cycles: u16, apply_dram_refresh: bool) {
        // Step in bounded slices so we don't miss per-scanline events when a single call
        // advances across HBlank and/or multiple scanlines (e.g., during MDMA stalls).
        //
        // In particular, the official burn-in tests rely on accurate HV-timer behavior.
        // If we step across a scanline boundary in one lump, we must:
        // - Run HDMA exactly at HBlank entry for that scanline (visible lines only)
        // - Tick scanline-based timers on every scanline advance
        // - Attribute HV-timer H-match to the correct scanline (before wrap)
        let mut remaining = cycles;
        const DOTS_PER_LINE: u16 = 341;
        const FIRST_HBLANK_DOT: u16 = 22 + 256; // visible starts at 22, width=256

        while remaining > 0 {
            let old_scanline = self.bus.get_ppu().scanline;
            let old_cycle = self.bus.get_ppu().get_cycle();
            let was_hblank = self.bus.get_ppu().is_hblank();

            // Compute a slice that won't cross HBlank entry or scanline wrap.
            //
            // IMPORTANT:
            // PPU's HBlank flag flips while *processing* the first HBlank dot (FIRST_HBLANK_DOT).
            // If we slice exactly up to FIRST_HBLANK_DOT, the PPU will not have executed that dot
            // yet, so `is_hblank()` stays false and we can miss the HDMA start-of-HBlank event.
            //
            // To reliably catch the transition, ensure we step at least 1 dot into HBlank
            // (i.e., end at >= FIRST_HBLANK_DOT+1) before checking `is_hblank()`.
            let mut slice = remaining.min(DOTS_PER_LINE.saturating_sub(old_cycle).max(1));
            if !was_hblank {
                if old_cycle < FIRST_HBLANK_DOT {
                    slice = slice.min((FIRST_HBLANK_DOT + 1).saturating_sub(old_cycle).max(1));
                } else if old_cycle == FIRST_HBLANK_DOT {
                    slice = slice.min(1);
                }
            }

            self.bus.get_ppu_mut().step(slice);
            remaining -= slice;

            let new_scanline = self.bus.get_ppu().scanline;
            let new_cycle = self.bus.get_ppu().get_cycle();
            let is_hblank = self.bus.get_ppu().is_hblank();

            // Update H/V timer progress for the segment we just stepped.
            // If we wrapped to the next scanline, attribute the segment to the old scanline.
            if old_scanline == new_scanline {
                self.bus.tick_timers_hv(old_cycle, new_cycle, old_scanline);
            } else {
                self.bus
                    .tick_timers_hv(old_cycle, DOTS_PER_LINE, old_scanline);
            }

            // H-Blank入りでHDMA実行
            //
            // SNES HDMA runs at the start of HBlank for *every* scanline (including scanline 0,
            // which is always hidden in blanking).  Some titles (notably Mode 7 perspective)
            // rely on the scanline-0 transfer to set up the first visible line.
            if !was_hblank && is_hblank {
                // Guard a few dots at HBlank head for HDMA operations
                self.bus.get_ppu_mut().on_hblank_start_guard();
                self.bus.hdma_hblank();
            }

            // スキャンライン変更時はタイマを進める
            if old_scanline != new_scanline {
                // DRAM refresh stalls the S-CPU for ~40 master cycles every scanline.
                // Model this as extra time that advances PPU/APU without CPU execution.
                if apply_dram_refresh {
                    const DRAM_REFRESH_MASTER_CYCLES: u64 = 40;
                    self.pending_stall_master_cycles = self
                        .pending_stall_master_cycles
                        .saturating_add(DRAM_REFRESH_MASTER_CYCLES);
                }
                self.bus.tick_timers();
                // JOYBUSYの更新
                self.bus.on_scanline_advance();
                // Frame start (scanline counter wrapped to 0)
                if new_scanline < old_scanline {
                    self.bus.on_frame_start();
                }
                // VBlank突入検知
                let vblank_start = self.bus.get_ppu().get_visible_height().saturating_add(1);
                if old_scanline < vblank_start && new_scanline >= vblank_start {
                    self.bus.on_vblank_start();
                    if std::env::var_os("TRACE_VBLANK_PC").is_some() {
                        let pc = self.cpu.get_pc();
                        println!(
                            "[TRACE_VBLANK_PC] frame={} PC={:02X}:{:04X} sl={} cyc={}",
                            self.bus.get_ppu().get_frame(),
                            (pc >> 16) as u8,
                            (pc & 0xFFFF) as u16,
                            new_scanline,
                            self.bus.get_ppu().get_cycle()
                        );
                    }
                }
            }
        }
    }

    #[allow(dead_code)]
    fn handle_nmi(&mut self) {
        self.cpu.trigger_nmi(&mut self.bus);
    }

    fn render(&mut self) {
        static mut RENDER_DEBUG_COUNT: u32 = 0;
        let rdc = unsafe {
            RENDER_DEBUG_COUNT = RENDER_DEBUG_COUNT.wrapping_add(1);
            RENDER_DEBUG_COUNT
        };
        if crate::debug_flags::render_verbose() && rdc <= 5 {
            println!("🎬 EMULATOR RENDER[{}]: Starting render function", rdc);
        }

        let ppu = self.bus.get_ppu();
        let ppu_framebuffer = ppu.get_framebuffer();

        if crate::debug_flags::render_verbose() && rdc <= 5 {
            println!(
                "🎬 COPYING FRAMEBUFFER: {} pixels from PPU to emulator",
                ppu_framebuffer.len()
            );
            if !ppu_framebuffer.is_empty() {
                println!(
                    "🎬 FIRST PIXELS: [0]=0x{:08X}, [1]=0x{:08X}, [256]=0x{:08X}",
                    ppu_framebuffer[0],
                    if ppu_framebuffer.len() > 1 {
                        ppu_framebuffer[1]
                    } else {
                        0
                    },
                    if ppu_framebuffer.len() > 256 {
                        ppu_framebuffer[256]
                    } else {
                        0
                    }
                );
            }
            let ppu_non_black = ppu_framebuffer
                .iter()
                .filter(|&&px| px != 0x00000000 && px != 0xFF000000 && (px & 0x00FFFFFF) != 0)
                .count();
            let white_pixels = ppu_framebuffer
                .iter()
                .filter(|&&px| px == 0xFFFFFFFF || (px & 0x00FFFFFF) == 0x00FFFFFF)
                .count();
            let start = 3320.min(ppu_framebuffer.len());
            let end = 3350.min(ppu_framebuffer.len());
            let actual_sample: Vec<u32> = ppu_framebuffer[start..end].iter().cloned().collect();
            println!(
                "🎨 PPU FRAMEBUFFER[{}]: Non-black pixels: {}/{}, White pixels: {}",
                rdc,
                ppu_non_black,
                ppu_framebuffer.len(),
                white_pixels
            );
            println!(
                "   🔍 PPU Sample around pos 3328: {:?}",
                &actual_sample[..10.min(actual_sample.len())]
            );
            if !ppu_framebuffer.is_empty() {
                println!(
                    "🎨 PPU SAMPLE[{}]: [0]=0x{:08X}, [128]=0x{:08X}, [256]=0x{:08X}",
                    rdc,
                    ppu_framebuffer[0],
                    if ppu_framebuffer.len() > 128 {
                        ppu_framebuffer[128]
                    } else {
                        0
                    },
                    if ppu_framebuffer.len() > 256 {
                        ppu_framebuffer[256]
                    } else {
                        0
                    }
                );
            }
        }

        let len = self.frame_buffer.len().min(ppu_framebuffer.len());
        if len > 0 {
            self.frame_buffer[..len].copy_from_slice(&ppu_framebuffer[..len]);
        }
        if len < self.frame_buffer.len() {
            self.frame_buffer[len..].fill(0xFF000000);
        }

        // ROM title overlay removed for clean display

        if let Some(w) = &mut self.window {
            if crate::debug_flags::render_verbose() && rdc <= 5 {
                println!(
                    "🖼️  WINDOW UPDATE[{}]: Updating window with {}x{} buffer",
                    rdc, SCREEN_WIDTH, SCREEN_HEIGHT
                );
                let non_black_pixels = self
                    .frame_buffer
                    .iter()
                    .filter(|&&pixel| pixel != 0xFF000000 && pixel != 0x00000000)
                    .count();
                println!(
                    "🖼️  WINDOW UPDATE[{}]: Non-black pixels in framebuffer: {}/{}",
                    rdc,
                    non_black_pixels,
                    self.frame_buffer.len()
                );
                println!(
                    "🖼️  WINDOW UPDATE[{}]: Sample pixels: [0]=0x{:08X}, [128]=0x{:08X}, [256]=0x{:08X}",
                    rdc,
                    if !self.frame_buffer.is_empty() { self.frame_buffer[0] } else { 0 },
                    if self.frame_buffer.len() > 128 { self.frame_buffer[128] } else { 0 },
                    if self.frame_buffer.len() > 256 { self.frame_buffer[256] } else { 0 }
                );
            }

            w.update_with_buffer(&self.frame_buffer, SCREEN_WIDTH, SCREEN_HEIGHT)
                .expect("Failed to update window buffer");

            if crate::debug_flags::render_verbose() && rdc <= 5 {
                println!(
                    "🖼️  WINDOW UPDATE[{}]: Window buffer updated successfully",
                    rdc
                );
            }
        }

        self.maybe_log_black_screen();
    }

    fn maybe_log_black_screen(&mut self) {
        let enabled = std::env::var("DEBUG_BLACK_SCREEN")
            .map(|v| v == "1" || v.to_lowercase() == "true")
            .unwrap_or(false);
        if !enabled {
            return;
        }
        let threshold = std::env::var("DEBUG_BLACK_SCREEN_FRAMES")
            .ok()
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(60);
        let all_black = self
            .frame_buffer
            .iter()
            .all(|&px| px == 0xFF000000 || px == 0x00000000);
        if all_black {
            self.black_screen_streak = self.black_screen_streak.saturating_add(1);
            if !self.black_screen_reported && self.black_screen_streak >= threshold {
                self.black_screen_reported = true;
                println!(
                    "[BLACK_SCREEN] frame={} streak={} (threshold={})",
                    self.frame_count, self.black_screen_streak, threshold
                );
                self.bus.get_ppu().debug_ppu_state();
            }
        } else {
            self.black_screen_streak = 0;
            self.black_screen_reported = false;
        }
    }

    // Headless-only: render a full Mode 7 diagnostic frame without running CPU
    fn run_mode7_diag_frame(&mut self) {
        let ppu = self.bus.get_ppu_mut();
        // One-time setup on first frame
        if self.frame_count == 0 {
            println!("MODE7_TEST: configuring PPU for Mode 7 diagnostic");
            // Forced blank during VRAM/CGRAM setup so writes are accepted regardless of timing.
            ppu.write(0x00, 0x8F);
            // Mode 7
            ppu.write(0x05, 0x07);
            // EXTBG on/off per env (default on)
            let extbg = std::env::var("M7_EXTBG")
                .map(|v| v == "1" || v.to_lowercase() == "true")
                .unwrap_or(true);
            ppu.write(0x33, if extbg { 0x40 } else { 0x00 });
            // Main screen: BG1, and BG2 as well when EXTBG is enabled.
            ppu.write(0x2C, if extbg { 0x03 } else { 0x01 });
            // M7SEL: from env flags; defaults R=1 (fill), F=1 (char0), flips off
            let r = std::env::var("M7_R")
                .map(|v| v == "1" || v.to_lowercase() == "true")
                .unwrap_or(true);
            let f = std::env::var("M7_F")
                .map(|v| v == "1" || v.to_lowercase() == "true")
                .unwrap_or(true);
            let flipx = std::env::var("M7_FLIPX")
                .map(|v| v == "1" || v.to_lowercase() == "true")
                .unwrap_or(false);
            let flipy = std::env::var("M7_FLIPY")
                .map(|v| v == "1" || v.to_lowercase() == "true")
                .unwrap_or(false);
            let mut m7sel: u8 = 0;
            if r {
                m7sel |= 0x80;
            }
            if f {
                m7sel |= 0x40;
            }
            if flipy {
                m7sel |= 0x02;
            }
            if flipx {
                m7sel |= 0x01;
            }
            ppu.write(0x1A, m7sel);
            // Matrix from angle/scale; Center at (128,128)
            let w16 = |ppu: &mut crate::ppu::Ppu, reg: u16, val: i16| {
                let lo = (val as u16 & 0x00FF) as u8;
                let hi = ((val as u16 >> 8) & 0xFF) as u8;
                ppu.write(reg, lo);
                ppu.write(reg, hi);
            };
            let scale = std::env::var("M7_SCALE")
                .ok()
                .and_then(|s| s.parse::<f32>().ok())
                .unwrap_or(1.25);
            let angle_deg = std::env::var("M7_ANGLE_DEG")
                .ok()
                .and_then(|s| s.parse::<f32>().ok())
                .unwrap_or(0.0);
            let theta = angle_deg.to_radians();
            let cos_t = theta.cos();
            let sin_t = theta.sin();
            let s256 = (scale * 256.0) as f32;
            let a = (s256 * cos_t).round() as i16;
            let b = (s256 * -sin_t).round() as i16;
            let c = (s256 * sin_t).round() as i16;
            let d = (s256 * cos_t).round() as i16;
            w16(ppu, 0x1B, a); // A
            w16(ppu, 0x1C, b); // B
            w16(ppu, 0x1D, c); // C
            w16(ppu, 0x1E, d); // D
            w16(ppu, 0x1F, 128); // center X (13-bit signed integer)
            w16(ppu, 0x20, 128); // center Y (13-bit signed integer)
                                 // Palette: 256 entries gradient (covers both EXTBG and non-EXTBG cases)
            ppu.write(0x21, 0x00);
            for i in 0..256u16 {
                let r = ((i >> 1) & 0x1F) as u16;
                let g = ((i >> 1) & 0x1F) as u16;
                let b = (i & 0x1F) as u16;
                let col = (r << 10) | (g << 5) | b;
                ppu.write(0x22, (col & 0xFF) as u8);
                ppu.write(0x22, ((col >> 8) as u8) & 0x7F);
            }

            // Mode 7 VRAM layout (words 0x0000..0x3FFF):
            // - Low byte: tilemap (128x128 bytes)
            // - High byte: tile data (256 tiles * 64 bytes)
            //
            // Fill tilemap with tile #1, and define tile #0/#1 with distinct gradients.
            // Configure VMAIN: increment after HIGH (bit7=1), inc=1
            ppu.write(0x15, 0x80);
            // VMADD = 0x0000 (word address)
            ppu.write(0x16, 0x00);
            ppu.write(0x17, 0x00);
            for w in 0..(128 * 128) {
                let lo = 0x01u8; // map: tile #1 everywhere
                let hi = if w < 64 {
                    // tile0: 0..63 (BG1)
                    w as u8
                } else if w < 128 {
                    // tile1: 128..191 (BG2 when EXTBG)
                    128u8.wrapping_add((w - 64) as u8)
                } else {
                    0u8
                };
                ppu.write(0x18, lo);
                ppu.write(0x19, hi); // increments word address
            }
            // Restore VMAIN to default (inc after LOW, inc=1)
            ppu.write(0x15, 0x00);
            // Unblank at full brightness
            ppu.write(0x00, 0x0F);
            println!(
                "MODE7_TEST: scale={:.2} angle_deg={:.1} EXTBG={} R={} F={} flips=({},{}) z:OBJ[3,2,1,0]=[{},{},{},{}] BG1={} BG2={}",
                scale, angle_deg, extbg, r, f, flipx, flipy,
                crate::debug_flags::m7_z_obj3(), crate::debug_flags::m7_z_obj2(),
                crate::debug_flags::m7_z_obj1(), crate::debug_flags::m7_z_obj0(),
                crate::debug_flags::m7_z_bg1(), crate::debug_flags::m7_z_bg2()
            );
        }
        // Step the PPU through one NTSC frame (approx 262 scanlines * 341 dots)
        let total_dots = 262u32 * 341u32;
        for _ in 0..total_dots {
            self.bus.get_ppu_mut().step(1u16);
        }
    }

    fn handle_input(&mut self) {
        let mut key_states = crate::input::KeyStates::default();

        if self.headless {
            return;
        }

        if let Some(w) = &mut self.window {
            let _ = w.update();
        }

        // キーボード入力を KeyStates に変換
        if let Some(w) = &self.window {
            key_states.up = w.is_key_down(Key::Up);
            key_states.down = w.is_key_down(Key::Down);
            key_states.left = w.is_key_down(Key::Left);
            key_states.right = w.is_key_down(Key::Right);
            key_states.b = w.is_key_down(Key::Z);
            key_states.a = w.is_key_down(Key::X);
            key_states.y = w.is_key_down(Key::A);
            key_states.x = w.is_key_down(Key::S);
            key_states.l = w.is_key_down(Key::Q);
            key_states.r = w.is_key_down(Key::W);
            key_states.start = w.is_key_down(Key::Enter)
                || w.is_key_down(Key::NumPadEnter)
                || w.is_key_down(Key::Space);
            key_states.select = w.is_key_down(Key::LeftShift) || w.is_key_down(Key::RightShift);
        }

        // 入力システムに渡す
        self.bus
            .get_input_system_mut()
            .handle_key_input(&key_states);

        // デバッグ: フレーム開始時に WRAM を強制書き換え（複数指定可、カンマ区切り）
        // 例: WRAM_POKE=7E:E95C:01,7E:E95D:00
        if let Ok(pokes) = std::env::var("WRAM_POKE") {
            for ent in pokes.split(',') {
                if ent.trim().is_empty() {
                    continue;
                }
                if let Some((bank, off, val)) = ent.trim().split_once(':').and_then(|(b, rest)| {
                    rest.split_once(':').and_then(|(a, v)| {
                        let bank = u8::from_str_radix(b, 16).ok()?;
                        let off = u16::from_str_radix(a, 16).ok()?;
                        let val = u8::from_str_radix(v, 16).ok()?;
                        Some((bank, off, val))
                    })
                }) {
                    let addr = ((bank as u32) << 16) | off as u32;
                    self.bus.write_u8(addr, val);
                    if std::env::var_os("TRACE_WRAM_POKE").is_some() {
                        println!("[WRAM_POKE] {:02X}:{:04X} <= {:02X}", bank, off, val);
                    }
                }
            }
        }

        // Handle audio controls
        self.handle_audio_controls();
    }

    fn apply_scripted_input_for_headless(&mut self) {
        if !self.headless {
            return;
        }
        let mask = crate::input::scripted_input_mask_for_frame(self.frame_count);
        self.bus
            .get_input_system_mut()
            .controller1
            .set_buttons(mask);
    }

    fn maybe_quit_on_cpu_test_result(&mut self) {
        if !self.bus.is_cpu_test_mode() {
            return;
        }
        let Some(result) = self.bus.take_cpu_test_result() else {
            return;
        };
        match result {
            crate::bus::CpuTestResult::Pass { test_idx } => {
                println!("[CPUTEST] PASS (test_idx=0x{:04X})", test_idx);
                crate::shutdown::request_quit();
            }
            crate::bus::CpuTestResult::Fail { test_idx } => {
                println!("[CPUTEST] FAIL (test_idx=0x{:04X})", test_idx);
                crate::shutdown::request_quit_with_code(1);
            }
            crate::bus::CpuTestResult::InvalidOrder { test_idx } => {
                println!(
                    "[CPUTEST] FAIL (msg=\"Invalid test order\" test_idx=0x{:04X})",
                    test_idx
                );
                crate::shutdown::request_quit_with_code(1);
            }
        }
    }

    // Periodically inject a minimal visible palette until the game loads enough CGRAM
    fn maybe_inject_min_palette_periodic(&mut self) {
        let enabled = std::env::var("COMPAT_PERIODIC_MIN_PALETTE")
            .map(|v| v == "1" || v.to_lowercase() == "true")
            .unwrap_or(false);
        if !enabled {
            return;
        }
        // Only when CGRAM is still tiny
        let need_help = { self.bus.get_ppu().cgram_usage() < 32 };
        if !need_help {
            return;
        }
        // Every 30 frames, inject a few colors
        if self.frame_count % 30 != 0 {
            return;
        }
        let ppu = self.bus.get_ppu_mut();
        // Ensure BG1 on and unblank, color math off
        ppu.write(0x2C, 0x01);
        ppu.write(0x00, 0x0F);
        ppu.write(0x30, 0x00);
        ppu.write(0x31, 0x00);
        // Inject colors 0..7
        ppu.write(0x21, 0x00);
        // 0: White, 1: Blue, 2: Red, 3: Green, 4..7: Gray steps
        // White
        ppu.write(0x22, 0xFF);
        ppu.write(0x22, 0x7F);
        // Blue
        ppu.write(0x22, 0x00);
        ppu.write(0x22, 0x7C);
        // Red
        ppu.write(0x22, 0x1F);
        ppu.write(0x22, 0x00);
        // Green
        ppu.write(0x22, 0xE0);
        ppu.write(0x22, 0x03);
        // Gray tones
        for lvl in [0x10u8, 0x20, 0x30, 0x3A] {
            ppu.write(0x22, lvl);
            ppu.write(0x22, 0x3F);
        }
    }

    fn sync_frame_rate(&mut self) {
        let now = Instant::now();
        if now < self.next_frame_time {
            std::thread::sleep(self.next_frame_time - now);
        }
        let now = Instant::now();
        self.next_frame_time += self.target_frame_duration;
        if now > self.next_frame_time {
            self.next_frame_time = now + self.target_frame_duration;
        }
    }

    fn maybe_dump_framebuffer_at(&mut self) {
        #[derive(Clone)]
        struct DumpCfg {
            frame: u64,
            quit: bool,
        }

        static CFG: OnceLock<Option<DumpCfg>> = OnceLock::new();
        let Some(cfg) = CFG
            .get_or_init(|| {
                let frame = std::env::var("DUMP_FRAME_AT")
                    .ok()
                    .and_then(|v| v.parse::<u64>().ok())?;
                let quit = std::env::var("DUMP_FRAME_QUIT")
                    .map(|v| v == "1" || v.to_lowercase() == "true")
                    .unwrap_or(false);
                Some(DumpCfg { frame, quit })
            })
            .clone()
        else {
            return;
        };

        if self.frame_count != cfg.frame {
            return;
        }

        let out_path = std::env::var("DUMP_FRAME_PATH")
            .ok()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| format!("logs/frame_{:05}.png", self.frame_count));
        let fb = self.bus.get_ppu().get_framebuffer();
        let _ = std::fs::create_dir_all("logs");
        if let Err(e) = write_framebuffer_image(std::path::Path::new(&out_path), fb, 256, 224) {
            eprintln!("DUMP_FRAME_AT: failed to write {}: {}", out_path, e);
        } else if !crate::debug_flags::quiet() {
            println!("DUMP_FRAME_AT: wrote {}", out_path);
        }

        if cfg.quit {
            crate::shutdown::request_quit();
        }
    }

    fn maybe_dump_mem_at(&mut self) {
        #[derive(Clone)]
        struct DumpCfg {
            frame: u64,
            prefix: String,
            quit: bool,
            ppu_state: bool,
        }

        static CFG: OnceLock<Option<DumpCfg>> = OnceLock::new();
        let Some(cfg) = CFG
            .get_or_init(|| {
                let frame = std::env::var("DUMP_MEM_AT")
                    .ok()
                    .and_then(|v| v.parse::<u64>().ok())?;
                let prefix = std::env::var("DUMP_MEM_PREFIX")
                    .ok()
                    .filter(|s| !s.trim().is_empty())
                    .unwrap_or_else(|| format!("logs/mem_{:05}", frame));
                let quit = std::env::var("DUMP_MEM_QUIT")
                    .map(|v| v == "1" || v.to_lowercase() == "true")
                    .unwrap_or(false);
                let ppu_state = std::env::var("DUMP_MEM_PPU_STATE")
                    .map(|v| v == "1" || v.to_lowercase() == "true")
                    .unwrap_or(false);
                Some(DumpCfg {
                    frame,
                    prefix,
                    quit,
                    ppu_state,
                })
            })
            .clone()
        else {
            return;
        };

        if self.frame_count != cfg.frame {
            return;
        }

        let _ = std::fs::create_dir_all("logs");

        let ppu = self.bus.get_ppu();
        let bwram = self.bus.sa1_bwram_slice();
        let write_bin = |suffix: &str, data: &[u8]| -> Result<(), std::io::Error> {
            let path = format!("{}_{}.bin", cfg.prefix, suffix);
            std::fs::write(path, data)
        };

        let mut ok = true;
        if let Err(e) = write_bin("wram", self.bus.wram()) {
            eprintln!("DUMP_MEM_AT: failed to write WRAM: {}", e);
            ok = false;
        }
        if let Err(e) = write_bin("vram", ppu.get_vram()) {
            eprintln!("DUMP_MEM_AT: failed to write VRAM: {}", e);
            ok = false;
        }
        if let Err(e) = write_bin("cgram", ppu.get_cgram()) {
            eprintln!("DUMP_MEM_AT: failed to write CGRAM: {}", e);
            ok = false;
        }
        if let Err(e) = write_bin("oam", ppu.get_oam()) {
            eprintln!("DUMP_MEM_AT: failed to write OAM: {}", e);
            ok = false;
        }
        if !bwram.is_empty() {
            if let Err(e) = write_bin("bwram", bwram) {
                eprintln!("DUMP_MEM_AT: failed to write BW-RAM: {}", e);
                ok = false;
            }
        }
        if std::env::var_os("DUMP_MEM_APU_STATE").is_some() {
            let mut apu_dump = String::new();
            self.bus.with_apu_mut(|apu| {
                let (smp_pc, smp_psw, smp_a, smp_x, smp_y, smp_sp, smp_stopped) =
                    if let Some(smp) = apu.inner.smp.as_ref() {
                        (
                            smp.reg_pc,
                            smp.get_psw(),
                            smp.reg_a,
                            smp.reg_x,
                            smp.reg_y,
                            smp.reg_sp,
                            smp.is_stopped() as u8,
                        )
                    } else {
                        (0, 0, 0, 0, 0, 0, 1)
                    };
                let mut code = [0u8; 16];
                if smp_stopped == 0 {
                    for (i, b) in code.iter_mut().enumerate() {
                        *b = apu
                            .inner
                            .read_u8(smp_pc.wrapping_add(i as u16) as u32);
                    }
                }
                let _ = std::fmt::Write::write_fmt(
                    &mut apu_dump,
                    format_args!(
                        "apu_cycles={}\nboot_state={}\nport_latch=[{:02X} {:02X} {:02X} {:02X}]\ninner_ports=[{:02X} {:02X} {:02X} {:02X}]\nsmp_pc={:04X} psw={:02X} A={:02X} X={:02X} Y={:02X} SP={:02X} stopped={}\ncode={:02X?}\n",
                        apu.total_smp_cycles,
                        apu.handshake_state_str(),
                        apu.port_latch[0],
                        apu.port_latch[1],
                        apu.port_latch[2],
                        apu.port_latch[3],
                        apu.inner.cpu_read_port(0),
                        apu.inner.cpu_read_port(1),
                        apu.inner.cpu_read_port(2),
                        apu.inner.cpu_read_port(3),
                        smp_pc,
                        smp_psw,
                        smp_a,
                        smp_x,
                        smp_y,
                        smp_sp,
                        smp_stopped,
                        code
                    ),
                );
            });
            let apu_path = format!("{}_apu.txt", cfg.prefix);
            if let Err(e) = std::fs::write(&apu_path, apu_dump) {
                eprintln!("DUMP_MEM_AT: failed to write APU state: {}", e);
                ok = false;
            }
        }
        // Always dump S-CPU core state when DUMP_MEM_AT is active (debug aid).
        let cpu_path = format!("{}_cpu.txt", cfg.prefix);
        let cpu_dump = format!(
            "pc={:02X}:{:04X}\nA={:04X} X={:04X} Y={:04X} SP={:04X}\nDB={:02X} DP={:04X}\nP={:02X} emu={} M8={} X8={}\n",
            self.cpu.pb,
            self.cpu.pc,
            self.cpu.a,
            self.cpu.x,
            self.cpu.y,
            self.cpu.sp,
            self.cpu.db,
            self.cpu.dp,
            self.cpu.p.bits(),
            self.cpu.emulation_mode,
            self.cpu.p.contains(crate::cpu::StatusFlags::MEMORY_8BIT),
            self.cpu.p.contains(crate::cpu::StatusFlags::INDEX_8BIT),
        );
        if let Err(e) = std::fs::write(&cpu_path, cpu_dump) {
            eprintln!("DUMP_MEM_AT: failed to write CPU state: {}", e);
            ok = false;
        }
        if ok && !crate::debug_flags::quiet() {
            println!("DUMP_MEM_AT: wrote {}_*", cfg.prefix);
        }
        if cfg.ppu_state {
            self.bus.get_ppu().debug_ppu_state();
        }

        if cfg.quit {
            crate::shutdown::request_quit();
        }
    }

    fn print_performance_stats(&self) {
        println!("╔═══════════════════════════════════════════╗");
        println!("║        Performance Statistics             ║");
        println!("╠═══════════════════════════════════════════╣");
        println!(
            "║ FPS: {:.1}                                  ║",
            self.performance_stats.fps
        );
        println!(
            "║ Frame Time: {:.2}ms (avg)                  ║",
            self.performance_stats.frame_time_avg.as_secs_f64() * 1000.0
        );
        println!(
            "║   Min: {:.2}ms  Max: {:.2}ms               ║",
            self.performance_stats.frame_time_min.as_secs_f64() * 1000.0,
            self.performance_stats.frame_time_max.as_secs_f64() * 1000.0
        );
        println!(
            "║ Dropped: {} / {} ({:.1}%)                  ║",
            self.performance_stats.dropped_frames,
            self.performance_stats.total_frames,
            (self.performance_stats.dropped_frames as f64
                / self.performance_stats.total_frames.max(1) as f64)
                * 100.0
        );

        // Show component timing if PERF_VERBOSE is enabled
        let verbose = std::env::var("PERF_VERBOSE").unwrap_or_default() == "1";
        if verbose && self.performance_stats.total_frames > 0 {
            println!("╠═══════════════════════════════════════════╣");
            println!("║ Component Timing (per frame avg)         ║");
            println!("╠═══════════════════════════════════════════╣");

            let frames = self.performance_stats.total_frames as f64;
            let cpu_avg = self.performance_stats.cpu_time_total.as_secs_f64() * 1000.0 / frames;
            let ppu_avg = self.performance_stats.ppu_time_total.as_secs_f64() * 1000.0 / frames;
            let dma_avg = self.performance_stats.dma_time_total.as_secs_f64() * 1000.0 / frames;
            let sa1_avg = self.performance_stats.sa1_time_total.as_secs_f64() * 1000.0 / frames;

            println!("║ CPU:  {:.3}ms                             ║", cpu_avg);
            println!("║ PPU:  {:.3}ms                             ║", ppu_avg);
            println!("║ DMA:  {:.3}ms                             ║", dma_avg);
            println!("║ SA-1: {:.3}ms                             ║", sa1_avg);

            let total_component = cpu_avg + ppu_avg + dma_avg + sa1_avg;
            let frame_avg = self.performance_stats.frame_time_avg.as_secs_f64() * 1000.0;
            let other = frame_avg - total_component;
            println!("║ Other:{:.3}ms                             ║", other);
        }

        println!("╠═══════════════════════════════════════════╣");
        println!(
            "║ Total Frames: {}                           ║",
            self.frame_count
        );
        println!(
            "║ Adaptive Timing: {}                         ║",
            if self.adaptive_timing { "ON " } else { "OFF" }
        );
        println!("╚═══════════════════════════════════════════╝");

        if !verbose {
            println!("(Set PERF_VERBOSE=1 for component-level timing)");
        }
    }

    // Performance optimization methods
    #[allow(dead_code)]
    pub fn set_frame_skip(&mut self, max_skip: u8) {
        self.max_frame_skip = max_skip.min(5); // Cap at 5 frames max
    }

    #[allow(dead_code)]
    pub fn set_adaptive_timing(&mut self, enabled: bool) {
        self.adaptive_timing = enabled;
        if enabled {
            println!("Adaptive timing enabled - frame skipping may occur for performance");
        } else {
            println!("Adaptive timing disabled - consistent frame rate with potential slowdown");
        }
    }

    #[allow(dead_code)]
    pub fn get_performance_stats(&self) -> &PerformanceStats {
        &self.performance_stats
    }

    // Optimized rendering with reduced frequency for performance
    #[allow(dead_code)]
    fn should_render_frame(&self) -> bool {
        // Always render if adaptive timing is off
        if !self.adaptive_timing {
            return true;
        }

        // Skip rendering occasionally if performance is good
        if self.performance_stats.fps > 58.0 {
            self.frame_count % 2 == 0 // Render every other frame when running well
        } else {
            true // Always render when struggling
        }
    }

    #[allow(dead_code)]
    fn handle_save_states(&mut self) {
        if self.headless {
            return;
        }
        if self
            .window
            .as_ref()
            .map(|w| w.is_key_pressed(Key::F5, minifb::KeyRepeat::No))
            .unwrap_or(false)
        {
            match self.quick_save() {
                Ok(_) => println!("Quick save completed successfully"),
                Err(e) => println!("Failed to save: {}", e),
            }
        }

        if self
            .window
            .as_ref()
            .map(|w| w.is_key_pressed(Key::F9, minifb::KeyRepeat::No))
            .unwrap_or(false)
        {
            match self.quick_load() {
                Ok(_) => println!("Quick load completed successfully"),
                Err(e) => println!("Failed to load: {}", e),
            }
        }
    }

    #[allow(dead_code)]
    fn handle_debugger_input(&mut self) {
        // F10: Pause/Resume
        if self.headless {
            return;
        }
        if self
            .window
            .as_ref()
            .map(|w| w.is_key_pressed(Key::F10, minifb::KeyRepeat::No))
            .unwrap_or(false)
        {
            if self.debugger.is_paused() {
                self.debugger.resume();
            } else {
                self.debugger.pause();
                self.debugger.print_cpu_state(&self.cpu);
            }
        }

        // F11: Step Instruction
        if self
            .window
            .as_ref()
            .map(|w| w.is_key_pressed(Key::F11, minifb::KeyRepeat::No))
            .unwrap_or(false)
        {
            let shift = self
                .window
                .as_ref()
                .map(|w| w.is_key_down(Key::LeftShift) || w.is_key_down(Key::RightShift))
                .unwrap_or(false);
            if shift {
                // Shift+F11: Step Over
                self.debugger.step_over();
                println!("Step Over");
            } else {
                // F11: Step Instruction
                self.debugger.step_instruction();
                self.debugger.print_cpu_state(&self.cpu);
            }
        }

        // F12: Show Debug Info
        if self
            .window
            .as_ref()
            .map(|w| w.is_key_pressed(Key::F12, minifb::KeyRepeat::No))
            .unwrap_or(false)
        {
            self.debugger.print_cpu_state(&self.cpu);
            self.debugger.print_trace(10);
            self.debugger.list_breakpoints();

            // Show memory at current PC
            let pc = self.cpu.get_pc();
            self.debugger.print_memory(&mut self.bus, pc, 64);
        }

        // D: Show PPU State (debug)
        if self
            .window
            .as_ref()
            .map(|w| w.is_key_pressed(Key::D, minifb::KeyRepeat::No))
            .unwrap_or(false)
        {
            self.bus.get_ppu().debug_ppu_state();
        }

        // P: Inject a minimal visible palette into CGRAM (developer aid)
        if self
            .window
            .as_ref()
            .map(|w| w.is_key_pressed(Key::P, minifb::KeyRepeat::No))
            .unwrap_or(false)
        {
            let ppu = self.bus.get_ppu_mut();
            println!("DEV: Injecting minimal CGRAM palette (P pressed)");
            ppu.write(0x21, 0x00); // CGADD=0
                                   // Color 0: White
            ppu.write(0x22, 0xFF);
            ppu.write(0x22, 0x7F);
            // Color 1: Blue
            ppu.write(0x22, 0x00);
            ppu.write(0x22, 0x7C);
            // Color 2: Red
            ppu.write(0x22, 0x1F);
            ppu.write(0x22, 0x00);
            // Color 3: Green
            ppu.write(0x22, 0xE0);
            ppu.write(0x22, 0x03);
        }

        // U: Unblank display immediately (developer aid)
        if self
            .window
            .as_ref()
            .map(|w| w.is_key_pressed(Key::U, minifb::KeyRepeat::No))
            .unwrap_or(false)
        {
            let inject_palette = self
                .window
                .as_ref()
                .map(|w| !(w.is_key_down(Key::LeftShift) || w.is_key_down(Key::RightShift)))
                .unwrap_or(true);
            let ppu = self.bus.get_ppu_mut();
            println!(
                "DEV: Forcing unblank (U pressed): TM=BG1, INIDISP=0x0F{}",
                if inject_palette {
                    ", with minimal palette"
                } else {
                    ""
                }
            );
            ppu.write(0x2C, 0x01); // TM: BG1 on
            ppu.write(0x00, 0x0F); // INIDISP: brightness 15, unblank
            if inject_palette {
                ppu.write(0x21, 0x00); // CGADD=0
                                       // Color 0: White
                ppu.write(0x22, 0xFF);
                ppu.write(0x22, 0x7F);
                // Color 1: Blue
                ppu.write(0x22, 0x00);
                ppu.write(0x22, 0x7C);
                // Color 2: Red
                ppu.write(0x22, 0x1F);
                ppu.write(0x22, 0x00);
                // Color 3: Green
                ppu.write(0x22, 0xE0);
                ppu.write(0x22, 0x03);
            }
        }
    }

    fn fetch_operands(&mut self, pc: u32, opcode: u8) -> Vec<u8> {
        // オペコードに基づいてオペランドのサイズを決定
        let operand_size = self.get_operand_size(opcode);
        let mut operands = Vec::new();

        for i in 1..=operand_size {
            operands.push(self.bus.read_u8(pc + i as u32));
        }

        operands
    }

    fn get_operand_size(&self, opcode: u8) -> u8 {
        // 簡易的なオペランドサイズ判定（完全実装は別途必要）
        match opcode {
            0x00 | 0x08 | 0x0A | 0x0B | 0x18 | 0x1A | 0x1B | 0x28 | 0x2A | 0x2B | 0x38 | 0x3A
            | 0x3B | 0x40 | 0x48 | 0x4A | 0x4B | 0x58 | 0x5A | 0x5B | 0x60 | 0x68 | 0x6A | 0x6B
            | 0x78 | 0x7A | 0x7B | 0x88 | 0x8A | 0x8B | 0x98 | 0x9A | 0x9B | 0xA8 | 0xAA | 0xAB
            | 0xB8 | 0xBA | 0xBB | 0xC8 | 0xCA | 0xCB | 0xD8 | 0xDA | 0xDB | 0xE8 | 0xEA | 0xEB
            | 0xF8 | 0xFA | 0xFB => 0, // 暗黙のオペランド

            0x10 | 0x30 | 0x50 | 0x70 | 0x80 | 0x90 | 0xB0 | 0xD0 | 0xF0 => 1, // 相対アドレス

            0x04 | 0x05 | 0x06 | 0x07 | 0x14 | 0x15 | 0x16 | 0x17 | 0x24 | 0x25 | 0x26 | 0x27
            | 0x34 | 0x35 | 0x36 | 0x37 | 0x44 | 0x45 | 0x46 | 0x47 | 0x54 | 0x55 | 0x56 | 0x57
            | 0x64 | 0x65 | 0x66 | 0x67 | 0x74 | 0x75 | 0x76 | 0x77 | 0x84 | 0x85 | 0x86 | 0x87
            | 0x94 | 0x95 | 0x96 | 0x97 | 0xA4 | 0xA5 | 0xA6 | 0xA7 | 0xB4 | 0xB5 | 0xB6 | 0xB7
            | 0xC4 | 0xC5 | 0xC6 | 0xC7 | 0xC2 | 0xE2 | 0xE4 | 0xE5 | 0xE6 | 0xE7 | 0xF4 => 1, // ダイレクトページ

            0x09 | 0x29 | 0x49 | 0x69 | 0x89 | 0xA0 | 0xA2 | 0xA9 | 0xC0 | 0xC9 | 0xE0 | 0xE9 => {
                // 即値（Mビット/Xビットに依存）
                if opcode & 0xF0 == 0xA0 || opcode & 0xF0 == 0xC0 || opcode & 0xF0 == 0xE0 {
                    // インデックスレジスタ操作
                    if self.cpu.p.bits() & 0x10 != 0 {
                        1
                    } else {
                        2
                    }
                } else {
                    // アキュムレータ操作
                    if self.cpu.p.bits() & 0x20 != 0 {
                        1
                    } else {
                        2
                    }
                }
            }

            0x0C | 0x0D | 0x0E | 0x1C | 0x1D | 0x1E | 0x2C | 0x2D | 0x2E | 0x3C | 0x3D | 0x3E
            | 0x4C | 0x4D | 0x4E | 0x5C | 0x5D | 0x5E | 0x6C | 0x6D | 0x6E | 0x7C | 0x7D | 0x7E
            | 0x8C | 0x8D | 0x8E | 0x9C | 0x9D | 0xAC | 0xAD | 0xAE | 0xBC | 0xBD | 0xBE | 0xCC
            | 0xCD | 0xCE | 0xDC | 0xDD | 0xDE | 0xEC | 0xED | 0xEE | 0xFC | 0xFD | 0xFE | 0x20 => {
                2
            } // 絶対アドレス

            0x0F | 0x1F | 0x2F | 0x3F | 0x4F | 0x5F | 0x6F | 0x7F | 0x8F | 0x9F | 0xAF | 0xBF
            | 0xCF | 0xDF | 0xEF | 0xFF | 0x22 | 0x5C => 3, // ロングアドレス

            _ => 1, // デフォルト
        }
    }

    fn handle_audio_controls(&mut self) {
        // Toggle audio on/off with F3
        if self.headless {
            return;
        }
        if self
            .window
            .as_ref()
            .map(|w| w.is_key_pressed(Key::F3, minifb::KeyRepeat::No))
            .unwrap_or(false)
        {
            let new_state = !self.audio_system.is_enabled();
            self.audio_system.set_enabled(new_state);
            println!("Audio: {}", if new_state { "ON" } else { "OFF" });
        }

        // Volume controls with F4/F6
        if self
            .window
            .as_ref()
            .map(|w| w.is_key_pressed(Key::F4, minifb::KeyRepeat::No))
            .unwrap_or(false)
        {
            let current_volume = self.audio_system.get_volume();
            let new_volume = (current_volume - 0.1).max(0.0);
            self.audio_system.set_volume(new_volume);
            println!("Volume: {:.0}%", new_volume * 100.0);
        }

        if self
            .window
            .as_ref()
            .map(|w| w.is_key_pressed(Key::F6, minifb::KeyRepeat::No))
            .unwrap_or(false)
        {
            let current_volume = self.audio_system.get_volume();
            let new_volume = (current_volume + 0.1).min(1.0);
            self.audio_system.set_volume(new_volume);
            println!("Volume: {:.0}%", new_volume * 100.0);
        }
    }

    pub fn quick_save(&mut self) -> Result<(), String> {
        let save_state = self.create_save_state();
        save_state.save_to_file("quicksave.sav")
    }

    pub fn quick_load(&mut self) -> Result<(), String> {
        let save_state = SaveState::load_from_file("quicksave.sav")?;

        if !save_state.validate_rom_checksum(self.rom_checksum) {
            return Err("Save state is from a different ROM".to_string());
        }

        self.load_save_state(save_state);
        Ok(())
    }

    #[allow(dead_code)]
    pub fn save_to_slot(&mut self, slot: u8) -> Result<(), String> {
        let filename = format!("save_slot_{}.sav", slot);
        let save_state = self.create_save_state();
        save_state.save_to_file(&filename)
    }

    #[allow(dead_code)]
    pub fn load_from_slot(&mut self, slot: u8) -> Result<(), String> {
        let filename = format!("save_slot_{}.sav", slot);
        let save_state = SaveState::load_from_file(&filename)?;

        if !save_state.validate_rom_checksum(self.rom_checksum) {
            return Err("Save state is from a different ROM".to_string());
        }

        self.load_save_state(save_state);
        Ok(())
    }

    fn create_save_state(&self) -> SaveState {
        let mut save_state = SaveState::new();

        // CPU state
        let cpu_state = self.cpu.get_state();
        save_state.cpu_state = CpuSaveState {
            a: cpu_state.a,
            x: cpu_state.x,
            y: cpu_state.y,
            sp: cpu_state.sp,
            dp: cpu_state.dp,
            db: cpu_state.db,
            pb: cpu_state.pb,
            pc: cpu_state.pc,
            p: cpu_state.p,
            emulation_mode: cpu_state.emulation_mode,
            cycles: cpu_state.cycles,
        };

        // Set metadata
        save_state.master_cycles = self.master_cycles;
        save_state.frame_count = self.frame_count;
        save_state.rom_checksum = self.rom_checksum;

        // PPU/APU/Memory/Input
        save_state.ppu_state = self.bus.get_ppu().to_save_state();
        if let Ok(apu) = self.bus.get_apu_shared().lock() {
            save_state.apu_state = apu.to_save_state();
        }
        let (wram, sram) = self.bus.snapshot_memory();
        save_state.memory_state = crate::savestate::MemoryState { wram, sram };
        save_state.input_state = self.bus.get_input_system().to_save_state();

        save_state
    }

    fn load_save_state(&mut self, save_state: SaveState) {
        // Restore CPU state
        let cpu_state = crate::cpu::CpuState {
            a: save_state.cpu_state.a,
            x: save_state.cpu_state.x,
            y: save_state.cpu_state.y,
            sp: save_state.cpu_state.sp,
            dp: save_state.cpu_state.dp,
            db: save_state.cpu_state.db,
            pb: save_state.cpu_state.pb,
            pc: save_state.cpu_state.pc,
            p: save_state.cpu_state.p,
            emulation_mode: save_state.cpu_state.emulation_mode,
            cycles: save_state.cpu_state.cycles,
        };

        self.cpu.set_state(cpu_state);
        self.master_cycles = save_state.master_cycles;
        self.frame_count = save_state.frame_count;

        // Restore PPU/APU/Memory/Input
        {
            let ppu = self.bus.get_ppu_mut();
            ppu.load_from_save_state(&save_state.ppu_state);
        }
        if let Ok(mut apu) = self.bus.get_apu_shared().lock() {
            apu.load_from_save_state(&save_state.apu_state);
        }
        self.bus
            .restore_memory(&save_state.memory_state.wram, &save_state.memory_state.sram);
        self.bus
            .get_input_system_mut()
            .load_from_save_state(&save_state.input_state);
    }

    // デバッガインターフェース
    #[allow(dead_code)]
    pub fn add_breakpoint(&mut self, address: u32) {
        self.debugger.add_breakpoint(address);
    }

    #[allow(dead_code)]
    pub fn remove_breakpoint(&mut self, address: u32) {
        self.debugger.remove_breakpoint(address);
    }

    #[allow(dead_code)]
    pub fn toggle_pause(&mut self) {
        if self.debugger.is_paused() {
            self.debugger.resume();
        } else {
            self.debugger.pause();
        }
    }

    #[allow(dead_code)]
    pub fn step_instruction(&mut self) {
        self.debugger.step_instruction();
    }

    #[allow(dead_code)]
    pub fn step_over(&mut self) {
        self.debugger.step_over();
    }

    #[allow(dead_code)]
    pub fn step_out(&mut self) {
        self.debugger.step_out();
    }

    #[allow(dead_code)]
    pub fn is_debugging(&self) -> bool {
        self.debugger.is_paused()
    }

    #[allow(dead_code)]
    pub fn get_save_info(&self, filename: &str) -> Result<SaveInfo, String> {
        let save_state = SaveState::load_from_file(filename)?;
        Ok(save_state.get_save_info())
    }

    /// Dump用: 現在の WRAM スナップショットを返す（ヘッドレス終了後などに利用）
    #[allow(dead_code)]
    pub fn wram(&self) -> &[u8] {
        self.bus.wram()
    }
}

fn calculate_checksum(data: &[u8]) -> u32 {
    data.iter()
        .fold(0u32, |acc, &byte| acc.wrapping_add(byte as u32))
}

// --- Minimal 5x7 font overlay for ASCII text ---
// Each glyph is 5x7, stored as 7 bytes (rows), LSB left.
#[cfg(feature = "dev")]
static FONT_5X7: [[u8; 7]; 39] = [
    // 'A'..'Z'
    [0x1E, 0x11, 0x11, 0x1F, 0x11, 0x11, 0x11], // A
    [0x1E, 0x11, 0x1E, 0x11, 0x11, 0x11, 0x1E], // B
    [0x1F, 0x10, 0x10, 0x10, 0x10, 0x10, 0x1F], // C
    [0x1E, 0x11, 0x11, 0x11, 0x11, 0x11, 0x1E], // D
    [0x1F, 0x10, 0x1E, 0x10, 0x10, 0x10, 0x1F], // E
    [0x1F, 0x10, 0x1E, 0x10, 0x10, 0x10, 0x10], // F
    [0x1F, 0x10, 0x10, 0x17, 0x11, 0x11, 0x1F], // G
    [0x11, 0x11, 0x1F, 0x11, 0x11, 0x11, 0x11], // H
    [0x1F, 0x04, 0x04, 0x04, 0x04, 0x04, 0x1F], // I
    [0x01, 0x01, 0x01, 0x01, 0x11, 0x11, 0x1F], // J
    [0x11, 0x12, 0x14, 0x18, 0x14, 0x12, 0x11], // K
    [0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x1F], // L
    [0x11, 0x1B, 0x15, 0x11, 0x11, 0x11, 0x11], // M
    [0x11, 0x19, 0x15, 0x13, 0x11, 0x11, 0x11], // N
    [0x0E, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0E], // O
    [0x1E, 0x11, 0x11, 0x1E, 0x10, 0x10, 0x10], // P
    [0x0E, 0x11, 0x11, 0x11, 0x15, 0x12, 0x0D], // Q
    [0x1E, 0x11, 0x11, 0x1E, 0x14, 0x12, 0x11], // R
    [0x0F, 0x10, 0x10, 0x0E, 0x01, 0x01, 0x1E], // S
    [0x1F, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04], // T
    [0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0E], // U
    [0x11, 0x11, 0x11, 0x0A, 0x0A, 0x04, 0x04], // V
    [0x11, 0x11, 0x11, 0x11, 0x15, 0x1B, 0x11], // W
    [0x11, 0x11, 0x0A, 0x04, 0x0A, 0x11, 0x11], // X
    [0x11, 0x11, 0x0A, 0x04, 0x04, 0x04, 0x04], // Y
    [0x1F, 0x01, 0x02, 0x04, 0x08, 0x10, 0x1F], // Z
    // '0'..'9'
    [0x0E, 0x11, 0x13, 0x15, 0x19, 0x11, 0x0E], // 0
    [0x04, 0x0C, 0x04, 0x04, 0x04, 0x04, 0x0E], // 1
    [0x0E, 0x11, 0x01, 0x0E, 0x10, 0x10, 0x1F], // 2
    [0x1F, 0x01, 0x02, 0x06, 0x01, 0x11, 0x0E], // 3
    [0x02, 0x06, 0x0A, 0x12, 0x1F, 0x02, 0x02], // 4
    [0x1F, 0x10, 0x1E, 0x01, 0x01, 0x11, 0x0E], // 5
    [0x06, 0x08, 0x10, 0x1E, 0x11, 0x11, 0x0E], // 6
    [0x1F, 0x01, 0x02, 0x04, 0x08, 0x08, 0x08], // 7
    [0x0E, 0x11, 0x11, 0x0E, 0x11, 0x11, 0x0E], // 8
    [0x0E, 0x11, 0x11, 0x0F, 0x01, 0x02, 0x0C], // 9
    // space, '-', ':'
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00], // space (index 36)
    [0x00, 0x00, 0x00, 0x1F, 0x00, 0x00, 0x00], // -     (index 37)
    [0x00, 0x00, 0x04, 0x00, 0x04, 0x00, 0x00], // :     (index 38)
];

#[cfg(feature = "dev")]
fn glyph_index(ch: char) -> Option<usize> {
    match ch {
        'A'..='Z' => Some((ch as u8 - b'A') as usize), // 0..25
        '0'..='9' => Some(26 + (ch as u8 - b'0') as usize), // 26..35
        ' ' => Some(36),                               // 36
        '-' => Some(37),                               // 37
        ':' => Some(38),                               // 38
        _ => None,
    }
}

#[cfg(feature = "dev")]
fn draw_text(buf: &mut [u32], w: usize, h: usize, x: usize, y: usize, s: &str, color: u32) {
    let mut cx = x;
    let cy = y;
    for ch in s.chars() {
        let ch = ch.to_ascii_uppercase();
        if ch == '\n' {
            break;
        }
        if let Some(idx) = glyph_index(ch) {
            draw_glyph(buf, w, h, cx, cy, &FONT_5X7[idx], color);
            // simple shadow
            draw_glyph(buf, w, h, cx + 1, cy + 1, &FONT_5X7[idx], 0x80000000);
        }
        cx += 6; // 5px glyph + 1px spacing
        if cx + 5 >= w {
            break;
        }
    }
}

#[cfg(feature = "dev")]
fn draw_glyph(buf: &mut [u32], w: usize, h: usize, x: usize, y: usize, rows: &[u8; 7], color: u32) {
    for (ry, row) in rows.iter().enumerate() {
        if y + ry >= h {
            break;
        }
        for rx in 0..5 {
            if x + rx >= w {
                break;
            }
            if (row >> rx) & 1 == 1 {
                let idx = (y + ry) * w + (x + rx);
                if idx < buf.len() {
                    buf[idx] = color;
                }
            }
        }
    }
}
// cleaned: stray inner attributes
// #![allow(dead_code)]
// #![allow(static_mut_refs)]
