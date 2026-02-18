use std::collections::HashMap;
use std::io::Write;
use nes_emulator::Nes;

struct Args {
    rom_path: String,
    max_frames: u32,
    inputs: HashMap<u32, u8>,
    captures: Vec<u32>,
    capture_dir: String,
    all_frames: bool,
}

impl Args {
    fn should_capture(&self, frame: u32) -> bool {
        self.all_frames || self.captures.contains(&frame)
    }
}

fn parse_buttons(s: &str) -> u8 {
    let mut val = 0u8;
    for name in s.split(',') {
        val |= match name.trim() {
            "A" => 0x01,
            "B" => 0x02,
            "Select" => 0x04,
            "Start" => 0x08,
            "Up" => 0x10,
            "Down" => 0x20,
            "Left" => 0x40,
            "Right" => 0x80,
            "" => 0x00,
            other => {
                eprintln!("Unknown button: {}", other);
                0
            }
        };
    }
    val
}

fn parse_args() -> Args {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: headless_test <rom_path> [options]");
        eprintln!();
        eprintln!("Options:");
        eprintln!("  --frames <N>              Run N frames (default: 300)");
        eprintln!("  --input <frame>:<buttons>  Set controller input at frame");
        eprintln!("                             buttons: A,B,Select,Start,Up,Down,Left,Right");
        eprintln!("                             Example: --input 60:Start --input 65:");
        eprintln!("  --capture <frame>          Capture screenshot at frame");
        eprintln!("  --capture-dir <dir>        Capture output directory (default: /tmp)");
        eprintln!("  --all-frames               Capture every frame");
        std::process::exit(1);
    }

    let rom_path = args[1].clone();
    let mut max_frames = 300u32;
    let mut inputs = HashMap::new();
    let mut captures = Vec::new();
    let mut capture_dir = "/tmp".to_string();
    let mut all_frames = false;

    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--frames" => {
                i += 1;
                max_frames = args[i].parse().expect("Invalid --frames value");
            }
            "--input" => {
                i += 1;
                let parts: Vec<&str> = args[i].splitn(2, ':').collect();
                if parts.len() != 2 {
                    eprintln!("Invalid --input format, expected frame:buttons");
                    std::process::exit(1);
                }
                let frame: u32 = parts[0].parse().expect("Invalid frame number");
                let buttons = parse_buttons(parts[1]);
                inputs.insert(frame, buttons);
            }
            "--capture" => {
                i += 1;
                let frame: u32 = args[i].parse().expect("Invalid --capture frame number");
                captures.push(frame);
            }
            "--capture-dir" => {
                i += 1;
                capture_dir = args[i].clone();
            }
            "--all-frames" => {
                all_frames = true;
            }
            other => {
                eprintln!("Unknown option: {}", other);
                std::process::exit(1);
            }
        }
        i += 1;
    }

    Args {
        rom_path,
        max_frames,
        inputs,
        captures,
        capture_dir,
        all_frames,
    }
}

fn save_ppm(frame: u32, buffer: &[u8], dir: &str) {
    std::fs::create_dir_all(dir).expect("Failed to create capture directory");
    let path = format!("{}/frame_{:04}.ppm", dir, frame);
    let mut file = std::fs::File::create(&path).expect("Failed to create PPM file");

    // PPM P6 header: width=256, height=240
    write!(file, "P6\n256 240\n255\n").expect("Failed to write PPM header");
    file.write_all(buffer).expect("Failed to write PPM data");
}

fn main() {
    let args = parse_args();

    eprintln!("Loading ROM: {}", args.rom_path);
    let mut nes = Nes::new();
    nes.load_rom(&args.rom_path).expect("Failed to load ROM");

    eprintln!("Running {} frames...", args.max_frames);
    let mut frame_count = 0u32;
    while frame_count < args.max_frames {
        // Apply input changes at frame start
        if let Some(&buttons) = args.inputs.get(&frame_count) {
            nes.set_controller(buttons);
            eprintln!("Frame {}: controller = 0x{:02X}", frame_count, buttons);
        }

        // Run one frame
        loop {
            if nes.step() {
                break;
            }
        }

        // Capture if requested
        if args.should_capture(frame_count) {
            save_ppm(frame_count, nes.get_frame_buffer(), &args.capture_dir);
            eprintln!("Frame {}: captured", frame_count);
        }

        frame_count += 1;
    }

    eprintln!("Done. {} frames executed.", frame_count);
}
