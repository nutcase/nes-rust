mod egui_ui;

use egui_ui::gl_game::GlGameRenderer;
use egui_ui::CheatToolUi;
use nes_emulator::Nes;
use sdl2::audio::{AudioCallback, AudioSpecDesired};
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use egui_sdl2_gl::gl;
use egui_sdl2_gl::DpiScaling;
use egui_sdl2_gl::ShaderVersion;

const SCALE: u32 = 3;
const GAME_W: u32 = 256 * SCALE;
const GAME_H: u32 = 240 * SCALE;
const PANEL_WIDTH_DEFAULT: f32 = 420.0;
const PANEL_WIDTH_MIN: f32 = 300.0;

fn main() -> Result<(), String> {
    let rom_path = match std::env::args().nth(1) {
        Some(p) => p,
        None => {
            // Interactive ROM selection from roms/ directory
            select_rom().map_err(|e| e.to_string())?
        }
    };

    let mut nes = Nes::new();
    nes.load_rom(&rom_path)
        .map_err(|e| format!("Failed to load ROM: {}", e))?;

    let sdl = sdl2::init().map_err(|e| e.to_string())?;
    let audio_subsystem = sdl.audio().map_err(|e| e.to_string())?;
    let video = sdl.video().map_err(|e| e.to_string())?;
    video.text_input().stop();

    let gl_attr = video.gl_attr();
    gl_attr.set_context_profile(sdl2::video::GLProfile::Core);
    gl_attr.set_context_version(3, 2);
    gl_attr.set_double_buffer(true);
    gl_attr.set_multisample_samples(0);

    let mut window = video
        .window("NES Emulator + Cheat Tool", GAME_W, GAME_H)
        .position_centered()
        .resizable()
        .opengl()
        .build()
        .map_err(|e| e.to_string())?;

    let _gl_context = window.gl_create_context().map_err(|e| e.to_string())?;
    window
        .gl_make_current(&_gl_context)
        .map_err(|e| e.to_string())?;

    gl::load_with(|name| video.gl_get_proc_address(name) as *const _);

    // Disable VSync — frame timing is managed manually
    let _ = video.gl_set_swap_interval(sdl2::video::SwapInterval::Immediate);

    let (mut painter, mut egui_state) =
        egui_sdl2_gl::with_sdl2(&window, ShaderVersion::Default, DpiScaling::Default);
    let egui_ctx = egui::Context::default();

    // Audio
    let desired_audio = AudioSpecDesired {
        freq: Some(44_100),
        channels: Some(1),
        samples: Some(4096),
    };

    let audio_buffer = Arc::new(Mutex::new(Vec::<f32>::new()));
    let audio_buffer_clone = audio_buffer.clone();
    let audio_device = audio_subsystem
        .open_playback(None, &desired_audio, |_spec| NesAudioCallback {
            audio_buffer: audio_buffer_clone,
            phase: 0.0,
        })
        .map_err(|e| e.to_string())?;
    audio_device.resume();

    let mut event_pump = sdl.event_pump().map_err(|e| e.to_string())?;
    let mut quit = false;
    let mut pressed: HashSet<Keycode> = HashSet::new();

    let mut game_renderer = GlGameRenderer::new();
    let mut cheat_ui = CheatToolUi::new();
    let mut prev_panel_visible = cheat_ui.panel_visible;
    let mut panel_width_px: u32 = PANEL_WIDTH_DEFAULT as u32;

    let cheat_path = cheat_file_path(&rom_path);

    let frame_duration = Duration::from_nanos(16_666_667);
    let mut last_frame = Instant::now();
    let mut frames_since_save = 0u32;

    while !quit {
        egui_state.input.time = Some(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs_f64(),
        );

        let egui_wants_kb = cheat_ui.panel_visible && egui_ctx.wants_keyboard_input();

        for event in event_pump.poll_iter() {
            // Forward to egui first when panel is visible
            if cheat_ui.panel_visible {
                egui_state.process_input(&window, event.clone(), &mut painter);
            }

            match &event {
                Event::Quit { .. } => {
                    let _ = nes.save_sram();
                    quit = true;
                }
                Event::KeyDown {
                    keycode: Some(code),
                    keymod,
                    repeat: false,
                    ..
                } => {
                    let code = *code;
                    let keymod = *keymod;

                    if code == Keycode::Tab {
                        cheat_ui.panel_visible = !cheat_ui.panel_visible;
                        continue;
                    }

                    if code == Keycode::Escape {
                        let _ = nes.save_sram();
                        quit = true;
                        continue;
                    }

                    // Skip game hotkeys when egui text fields have focus
                    if egui_wants_kb {
                        continue;
                    }

                    let ctrl = keymod
                        .intersects(sdl2::keyboard::Mod::LCTRLMOD | sdl2::keyboard::Mod::RCTRLMOD);

                    if ctrl {
                        // Ctrl+1-4: save state
                        match code {
                            Keycode::Num1 => {
                                let _ = nes.save_state(1, "current_rom");
                            }
                            Keycode::Num2 => {
                                let _ = nes.save_state(2, "current_rom");
                            }
                            Keycode::Num3 => {
                                let _ = nes.save_state(3, "current_rom");
                            }
                            Keycode::Num4 => {
                                let _ = nes.save_state(4, "current_rom");
                            }
                            _ => {
                                pressed.insert(code);
                            }
                        }
                    } else {
                        // Number keys without Ctrl: load state
                        match code {
                            Keycode::Num1 => {
                                let _ = nes.load_state(1);
                            }
                            Keycode::Num2 => {
                                let _ = nes.load_state(2);
                            }
                            Keycode::Num3 => {
                                let _ = nes.load_state(3);
                            }
                            Keycode::Num4 => {
                                let _ = nes.load_state(4);
                            }
                            _ => {
                                pressed.insert(code);
                            }
                        }
                    }
                }
                Event::KeyUp {
                    keycode: Some(code),
                    repeat: false,
                    ..
                } => {
                    if !egui_wants_kb {
                        pressed.remove(code);
                    }
                }
                _ => {}
            }
        }

        // Resize window on panel toggle
        if cheat_ui.panel_visible != prev_panel_visible {
            if cheat_ui.panel_visible {
                cheat_ui.refresh(nes.ram());
            }
            let new_w = if cheat_ui.panel_visible {
                GAME_W + panel_width_px
            } else {
                GAME_W
            };
            let _ = window.set_size(new_w, GAME_H);
            prev_panel_visible = cheat_ui.panel_visible;
        }

        // Update controller state
        let controller = build_controller_state(&pressed);
        nes.set_controller(controller);

        // Emulation: run until frame complete (unless paused)
        if !cheat_ui.paused {
            let mut step_count = 0;
            loop {
                let frame_complete = nes.step();
                if frame_complete {
                    break;
                }
                step_count += 1;
                if step_count > 50000 {
                    break;
                }
            }

            frames_since_save += 1;
            if frames_since_save >= 1800 {
                let _ = nes.save_sram();
                frames_since_save = 0;
            }
        }

        // Apply cheats every frame
        {
            let ram_len = nes.ram().len();
            let mgr = &cheat_ui.cheat_search_ui.manager;
            for entry in &mgr.entries {
                if !entry.enabled {
                    continue;
                }
                let addr = entry.address as usize;
                if addr < ram_len {
                    nes.ram_mut()[addr] = entry.value;
                } else if let Some(sram) = nes.prg_ram_mut() {
                    let sram_addr = addr - ram_len;
                    if sram_addr < sram.len() {
                        sram[sram_addr] = entry.value;
                    }
                }
            }
        }

        // Feed audio
        let audio_samples = nes.get_audio_buffer();
        if !audio_samples.is_empty() {
            if let Ok(mut buffer) = audio_buffer.lock() {
                buffer.extend(audio_samples);
                if buffer.len() > 8192 {
                    buffer.drain(0..2048);
                }
            }
        }

        // Upload game frame to GL texture
        let frame_buf = nes.get_frame_buffer();
        game_renderer.upload_frame_rgb24(frame_buf, 256, 240);

        // Render
        let (win_w, win_h) = window.size();

        unsafe {
            gl::ClearColor(0.0, 0.0, 0.0, 1.0);
            gl::Clear(gl::COLOR_BUFFER_BIT);
        }

        // Draw game quad on the left
        let panel_px = if cheat_ui.panel_visible {
            panel_width_px
        } else {
            0
        };
        let game_vp_w = win_w.saturating_sub(panel_px);
        game_renderer.draw(0, 0, game_vp_w as i32, win_h as i32);

        // Draw panel when visible
        if cheat_ui.panel_visible {
            painter.update_screen_rect((win_w, win_h));
            egui_state.input.screen_rect = Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(win_w as f32, win_h as f32),
            ));

            let mut ram_writes: Vec<(usize, u8)> = Vec::new();
            // Build combined RAM buffer: cpu_ram ++ prg_ram
            let mut combined_ram = nes.ram().to_vec();
            if let Some(sram) = nes.prg_ram() {
                combined_ram.extend_from_slice(sram);
            }
            let live_ram = &combined_ram;

            let full_output = egui_ctx.run(egui_state.input.take(), |ctx| {
                let panel_resp = egui::SidePanel::right("cheat_panel")
                    .resizable(true)
                    .min_width(PANEL_WIDTH_MIN)
                    .default_width(PANEL_WIDTH_DEFAULT)
                    .show(ctx, |ui| {
                        egui::ScrollArea::vertical()
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                cheat_ui.show_panel(
                                    ui,
                                    &mut ram_writes,
                                    live_ram,
                                    Some(&cheat_path),
                                );
                            });
                    });
                let actual_w = panel_resp.response.rect.width() as u32;
                if actual_w != panel_width_px {
                    panel_width_px = actual_w;
                    let new_w = GAME_W + panel_width_px;
                    let _ = window.set_size(new_w, GAME_H);
                }
            });

            if cheat_ui.refresh_requested {
                cheat_ui.refresh(nes.ram());
                cheat_ui.refresh_requested = false;
            }

            let prims = egui_ctx.tessellate(full_output.shapes, full_output.pixels_per_point);
            painter.paint_jobs(None, full_output.textures_delta, prims);
            egui_state.process_output(&window, &full_output.platform_output);

            // Apply hex editor writes
            for (addr, val) in ram_writes {
                let ram = nes.ram_mut();
                if addr < ram.len() {
                    ram[addr] = val;
                }
            }
        }

        window.gl_swap_window();

        // Frame timing
        let now = Instant::now();
        let elapsed = now.duration_since(last_frame);
        if elapsed < frame_duration {
            std::thread::sleep(frame_duration - elapsed);
        }
        last_frame = Instant::now();
    }

    Ok(())
}

fn build_controller_state(pressed: &HashSet<Keycode>) -> u8 {
    let mut state: u8 = 0;
    if pressed.contains(&Keycode::Z) {
        state |= 0x01;
    } // A
    if pressed.contains(&Keycode::X) {
        state |= 0x02;
    } // B
    if pressed.contains(&Keycode::Space) {
        state |= 0x04;
    } // Select
    if pressed.contains(&Keycode::Return) {
        state |= 0x08;
    } // Start
    if pressed.contains(&Keycode::Up) {
        state |= 0x10;
    } // Up
    if pressed.contains(&Keycode::Down) {
        state |= 0x20;
    } // Down
    if pressed.contains(&Keycode::Left) {
        state |= 0x40;
    } // Left
    if pressed.contains(&Keycode::Right) {
        state |= 0x80;
    } // Right
    state
}

fn cheat_file_path(rom_path: &str) -> PathBuf {
    let stem = Path::new(rom_path)
        .file_stem()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("game");
    PathBuf::from("cheats").join(format!("{stem}.json"))
}

fn select_rom() -> Result<String, Box<dyn std::error::Error>> {
    use std::fs;
    use std::io::{self, Write};

    let roms_path = Path::new("roms");
    let mut rom_files = Vec::new();

    if roms_path.exists() && roms_path.is_dir() {
        for entry in fs::read_dir(roms_path)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext == "nes" {
                        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                            rom_files.push((name.to_string(), path.to_string_lossy().to_string()));
                        }
                    }
                }
            }
        }
    }

    if rom_files.is_empty() {
        return Err("No ROM files found in 'roms' directory".into());
    }

    rom_files.sort_by(|a, b| a.0.cmp(&b.0));

    println!("Available ROMs:");
    for (i, (name, _)) in rom_files.iter().enumerate() {
        println!("{}. {}", i + 1, name);
    }

    loop {
        print!("Select ROM (1-{}): ", rom_files.len());
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        if let Ok(choice) = input.trim().parse::<usize>() {
            if choice >= 1 && choice <= rom_files.len() {
                return Ok(rom_files[choice - 1].1.clone());
            }
        }
    }
}

struct NesAudioCallback {
    audio_buffer: Arc<Mutex<Vec<f32>>>,
    phase: f32,
}

impl AudioCallback for NesAudioCallback {
    type Channel = f32;

    fn callback(&mut self, out: &mut [f32]) {
        if let Ok(mut buffer) = self.audio_buffer.lock() {
            for sample in out.iter_mut() {
                if !buffer.is_empty() {
                    let audio_sample = buffer.remove(0);
                    *sample = audio_sample;
                    self.phase = audio_sample;
                } else {
                    self.phase *= 0.98;
                    *sample = self.phase;
                }
            }
        } else {
            for sample in out.iter_mut() {
                self.phase *= 0.98;
                *sample = self.phase;
            }
        }
    }
}
