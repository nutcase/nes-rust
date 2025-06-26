mod cpu;
mod ppu;
mod apu;
mod memory;
mod cartridge;
mod bus;

use std::time::{Duration, Instant};
use std::sync::{Arc, Mutex};
use cpu::Cpu;
use bus::Bus;
use cartridge::Cartridge;
use sdl2::pixels::PixelFormatEnum;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::audio::AudioCallback;

const CPU_CYCLES_PER_FRAME: u32 = 29830; // Increased for better compatibility

pub struct Nes {
    cpu: Cpu,
    bus: Bus,
    cpu_cycles: u32,
}

impl Nes {
    pub fn new() -> Self {
        Nes {
            cpu: Cpu::new(),
            bus: Bus::new(),
            cpu_cycles: 0,
        }
    }

    pub fn load_rom(&mut self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let cartridge = Cartridge::load(path)?;
        self.bus.load_cartridge(cartridge);
        self.cpu.reset(&mut self.bus);
        Ok(())
    }

    pub fn step(&mut self) -> bool {
        let mut total_cycles = 0u32;
        
        // If DMA is in progress, don't execute CPU instruction
        if self.bus.is_dma_in_progress() {
            // DMA takes 1 cycle to process
            let dma_completed = self.bus.step_dma();
            total_cycles = 1;
            if dma_completed {
                // DMA completed
            }
        } else {
            // Normal CPU execution
            let cpu_cycles = self.cpu.step(&mut self.bus);
            
            // Safety check for zero cycles
            if cpu_cycles == 0 {
                return false;
            }
            
            total_cycles = cpu_cycles as u32;
        }
        
        let mut nmi_triggered = false;
        for _ in 0..total_cycles * 3 {
            let nmi = self.bus.step_ppu();
            if nmi {
                nmi_triggered = true;
            }
        }
        
        if nmi_triggered {
            self.cpu.nmi(&mut self.bus);
        }
        
        // APU runs at CPU clock rate
        for _ in 0..total_cycles {
            self.bus.step_apu();
        }
        
        self.cpu_cycles += total_cycles;
        
        if self.cpu_cycles >= CPU_CYCLES_PER_FRAME {
            self.cpu_cycles -= CPU_CYCLES_PER_FRAME;
            true
        } else {
            false
        }
    }

    pub fn get_frame_buffer(&self) -> &[u8] {
        self.bus.get_ppu_buffer()
    }

    pub fn get_audio_buffer(&mut self) -> Vec<f32> {
        self.bus.get_audio_buffer()
    }

    pub fn set_controller(&mut self, controller: u8) {
        self.bus.set_controller(controller);
    }
    
    pub fn get_controller(&self) -> u8 {
        self.bus.controller
    }
}

fn show_rom_selection() -> Result<String, Box<dyn std::error::Error>> {
    use std::fs;
    use std::path::Path;
    use std::io::{self, Write};
    
    println!("=== NES Emulator ROM Selection ===");
    
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
    
    println!("Available ROM files:");
    for (i, (name, _)) in rom_files.iter().enumerate() {
        println!("  {}. {}", i + 1, name);
    }
    
    loop {
        print!("Select ROM (1-{}): ", rom_files.len());
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        
        if let Ok(choice) = input.trim().parse::<usize>() {
            if choice >= 1 && choice <= rom_files.len() {
                println!("Selected: {}", rom_files[choice - 1].0);
                return Ok(rom_files[choice - 1].1.clone());
            }
        }
        
        println!("Invalid selection. Please enter a number between 1 and {}", rom_files.len());
    }
}


fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();
        
    println!("Starting NES emulator...");

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
    
    if let Err(e) = nes.load_rom(&selected_rom) {
        eprintln!("Failed to load ROM: {}", e);
        std::process::exit(1);
    }

    println!("ROM loaded successfully: {}", selected_rom);
    println!("Initializing SDL2 subsystems...");
    
    // Re-initialize audio subsystem for emulation
    let audio_subsystem = sdl_context.audio()?;
    
    // Create the emulation window
    let window = video_subsystem
        .window("NES Emulator", 256 * 3, 240 * 3)
        .position_centered()
        .resizable()
        .build()?;
    
    let mut canvas = window.into_canvas().build()?;
    let texture_creator = canvas.texture_creator();
    
    let mut texture = texture_creator
        .create_texture_streaming(PixelFormatEnum::RGB24, 256, 240)?;
    
    // Set up audio
    let desired_spec = sdl2::audio::AudioSpecDesired {
        freq: Some(44100),
        channels: Some(1), // mono
        samples: Some(4096), // buffer size
    };
    
    let audio_buffer = Arc::new(Mutex::new(Vec::<f32>::new()));
    let audio_buffer_clone = audio_buffer.clone();
    
    let audio_device = audio_subsystem.open_playback(None, &desired_spec, |spec| {
        println!("Audio spec: {:?}", spec);
        NesAudioCallback {
            audio_buffer: audio_buffer_clone,
            phase: 0.0,
        }
    })?;
    
    audio_device.resume();
    println!("Audio device started");
    
    let mut event_pump = sdl_context.event_pump()?;
    println!("Starting main emulation loop...");
    
    let frame_duration = Duration::from_nanos(16_666_667); // 60 FPS (1000ms / 60fps)
    let mut last_frame = Instant::now();
    let mut frame_count = 0;
    let _start_time = Instant::now();
    
    'running: loop {
        // Handle events
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } | Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                    break 'running;
                }
                Event::KeyDown { keycode: Some(key), .. } => {
                    let controller = map_key_to_controller(key, nes.get_controller());
                    nes.set_controller(controller);
                }
                Event::KeyUp { keycode: Some(key), .. } => {
                    let controller = unmap_key_from_controller(key, nes.get_controller());
                    nes.set_controller(controller);
                }
                _ => {}
            }
        }
        
        // Skip long-running game check - remove verbose logging
        
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
        
        // Update texture with frame buffer
        texture.with_lock(None, |buffer: &mut [u8], _pitch: usize| {
            let frame_buffer = nes.get_frame_buffer();
            buffer.copy_from_slice(frame_buffer);
        })?;
        
        // Update audio buffer with improved buffering
        let audio_samples = nes.get_audio_buffer();
        if !audio_samples.is_empty() {
            if let Ok(mut buffer) = audio_buffer.lock() {
                buffer.extend(audio_samples);
                // More conservative buffer management to prevent audio drops
                if buffer.len() > 8192 {
                    buffer.drain(0..2048);
                }
            }
        }
        
        // Render
        canvas.clear();
        canvas.copy(&texture, None, None)?;
        canvas.present();
        
        frame_count += 1;
        
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
        Keycode::Space => current | 0x04,  // Select (bit 2) - changed from RShift to Space
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
        Keycode::Space => current & !0x04,  // Select (bit 2) - changed from RShift to Space
        Keycode::Return => current & !0x08, // Start (bit 3)
        Keycode::Up => current & !0x10,     // Up (bit 4)
        Keycode::Down => current & !0x20,   // Down (bit 5)
        Keycode::Left => current & !0x40,   // Left (bit 6)
        Keycode::Right => current & !0x80,  // Right (bit 7)
        _ => current,
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
                    // Simple, direct output
                    *sample = audio_sample;
                    self.phase = audio_sample;
                } else {
                    // Gradually fade to silence to avoid clicks
                    self.phase *= 0.98;
                    *sample = self.phase;
                }
            }
        } else {
            // If we can't lock the buffer, gradually fade to silence
            for sample in out.iter_mut() {
                self.phase *= 0.98;
                *sample = self.phase;
            }
        }
    }
}
