use std::collections::VecDeque;
use std::time::{Duration, Instant};
use std::sync::{Arc, Mutex};
use nes_emulator::Nes;
use sdl2::pixels::PixelFormatEnum;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::audio::AudioCallback;

fn show_rom_selection() -> Result<String, Box<dyn std::error::Error>> {
    use std::fs;
    use std::path::Path;
    use std::io::{self, Write};

    // Scan roms directory for .nes files
    let roms_path = Path::new("roms");
    let mut rom_files = Vec::new();

    if roms_path.exists() && roms_path.is_dir() {
        for entry in fs::read_dir(roms_path)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                if let Some(extension) = path.extension() {
                    if extension == "nes" {
                        if let Some(file_name) = path.file_name() {
                            if let Some(name_str) = file_name.to_str() {
                                rom_files.push((name_str.to_string(), path.to_string_lossy().to_string()));
                            }
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


fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();

    // Check for command line arguments first
    let args: Vec<String> = std::env::args().collect();
    let selected_rom = if args.len() > 1 {
        args[1].clone()
    } else {
        // Show ROM selection screen
        show_rom_selection()?
    };

    // Initialize SDL2 for emulation
    sdl2::hint::set("SDL_DISABLE_IMMINTRIN_H", "1");
    sdl2::hint::set("SDL_MAC_CTRL_CLICK_EMULATE_RIGHT_CLICK", "0");

    let sdl_context = sdl2::init()?;
    let video_subsystem = sdl_context.video()?;
    video_subsystem.text_input().stop();

    let mut nes = Nes::new();

    if let Err(_e) = nes.load_rom(&selected_rom) {
        std::process::exit(1);
    }

    // Re-initialize audio subsystem for emulation
    let audio_subsystem = sdl_context.audio()?;

    // Create the emulation window
    let window = video_subsystem
        .window("NES Emulator", 256 * 3, 240 * 3)
        .position_centered()
        .resizable()
        .build()?;

    let mut canvas = window.into_canvas().build()?;
    // Set canvas clear color to black instead of default (cyan)
    canvas.set_draw_color(sdl2::pixels::Color::RGB(5, 5, 5));
    let texture_creator = canvas.texture_creator();

    let mut texture = texture_creator
        .create_texture_streaming(PixelFormatEnum::RGB24, 256, 240)?;

    // Set up audio
    let desired_spec = sdl2::audio::AudioSpecDesired {
        freq: Some(44100),
        channels: Some(1), // mono
        samples: Some(4096), // buffer size
    };

    let audio_buffer: Arc<Mutex<VecDeque<f32>>> = Arc::new(Mutex::new(VecDeque::new()));
    let audio_buffer_clone = audio_buffer.clone();

    let audio_device = audio_subsystem.open_playback(None, &desired_spec, |_spec| {
        NesAudioCallback {
            audio_buffer: audio_buffer_clone,
            phase: 0.0,
        }
    })?;

    audio_device.resume();

    let mut event_pump = sdl_context.event_pump()?;

    let frame_duration = Duration::from_nanos(16_666_667); // 60 FPS (1000ms / 60fps)
    let mut last_frame = Instant::now();
    let mut _frame_count = 0;
    let _start_time = Instant::now();
    let mut frames_since_save = 0u32;

    'running: loop {
        // Handle events
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } | Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                    // Save SRAM before quitting
                    if let Err(e) = nes.save_sram() {
                        eprintln!("Failed to save SRAM: {}", e);
                    }
                    break 'running;
                }
                Event::KeyDown { keycode: Some(key), keymod, .. } => {
                    if keymod.contains(sdl2::keyboard::Mod::LCTRLMOD) || keymod.contains(sdl2::keyboard::Mod::RCTRLMOD) {
                        // Ctrl + number keys for save states
                        match key {
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
                            _ => {}
                        }
                    } else {
                        // Number keys without Ctrl for load states
                        match key {
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
                                let controller = map_key_to_controller(key, nes.get_controller());
                                nes.set_controller(controller);
                            }
                        }
                    }
                }
                Event::KeyUp { keycode: Some(key), .. } => {
                    let controller = unmap_key_from_controller(key, nes.get_controller());
                    nes.set_controller(controller);
                }
                _ => {}
            }
        }

        // Run emulation until frame is complete
        let mut step_count = 0;
        loop {
            let frame_complete = nes.step();
            if frame_complete {
                break;
            }
            step_count += 1;

            if step_count > 50000 { // Normal limit for frame completion
                break;
            }
        }

        _frame_count += 1;
        frames_since_save += 1;

        // Save SRAM every 30 seconds (1800 frames at 60 FPS) - reduced frequency
        if frames_since_save >= 1800 {
            let _ = nes.save_sram(); // Only save if valid save data exists
            frames_since_save = 0;
        }

        // Update texture with frame buffer
        texture.with_lock(None, |buffer: &mut [u8], _pitch: usize| {
            let frame_buffer = nes.get_frame_buffer();
            buffer.copy_from_slice(frame_buffer);
        })?;

        // Update audio buffer with improved buffering
        let audio_samples = nes.get_audio_buffer();
        if !audio_samples.is_empty() {
            if let Ok(mut buffer) = audio_buffer.lock() {
                buffer.extend(audio_samples.iter());
                // More conservative buffer management to prevent audio drops
                if buffer.len() > 8192 {
                    drop(buffer.drain(0..2048));
                }
            }
        }

        // Render
        canvas.clear();
        canvas.copy(&texture, None, None)?;
        canvas.present();

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

fn map_key_to_controller(key: Keycode, current: u8) -> u8 {
    match key {
        Keycode::X => current | 0x02,      // B (bit 1)
        Keycode::Z => current | 0x01,      // A (bit 0)
        Keycode::Space => current | 0x04,  // Select (bit 2)
        Keycode::Return => current | 0x08, // Start (bit 3)
        Keycode::Up => current | 0x10,     // Up (bit 4)
        Keycode::Down => current | 0x20,   // Down (bit 5)
        Keycode::Left => current | 0x40,   // Left (bit 6)
        Keycode::Right => current | 0x80,  // Right (bit 7)
        _ => current,
    }
}

fn unmap_key_from_controller(key: Keycode, current: u8) -> u8 {
    match key {
        Keycode::X => current & !0x02,      // B (bit 1)
        Keycode::Z => current & !0x01,      // A (bit 0)
        Keycode::Space => current & !0x04,  // Select (bit 2)
        Keycode::Return => current & !0x08, // Start (bit 3)
        Keycode::Up => current & !0x10,     // Up (bit 4)
        Keycode::Down => current & !0x20,   // Down (bit 5)
        Keycode::Left => current & !0x40,   // Left (bit 6)
        Keycode::Right => current & !0x80,  // Right (bit 7)
        _ => current,
    }
}

struct NesAudioCallback {
    audio_buffer: Arc<Mutex<VecDeque<f32>>>,
    phase: f32,
}

impl AudioCallback for NesAudioCallback {
    type Channel = f32;

    fn callback(&mut self, out: &mut [f32]) {
        if let Ok(mut buffer) = self.audio_buffer.lock() {
            for sample in out.iter_mut() {
                if let Some(audio_sample) = buffer.pop_front() {
                    *sample = audio_sample;
                    self.phase = audio_sample;
                } else {
                    // Gradually fade to silence to avoid clicks
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
