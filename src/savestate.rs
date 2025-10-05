use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{Read, Write};

#[derive(Serialize, Deserialize)]
pub struct SaveState {
    pub version: u32,
    pub timestamp: u64,
    pub cpu_state: CpuSaveState,
    pub ppu_state: PpuSaveState,
    pub apu_state: ApuSaveState,
    pub memory_state: MemoryState,
    pub input_state: InputSaveState,
    pub master_cycles: u64,
    pub frame_count: u64,
    pub rom_checksum: u32,
}

#[derive(Serialize, Deserialize)]
pub struct CpuSaveState {
    pub a: u16,
    pub x: u16,
    pub y: u16,
    pub sp: u16,
    pub dp: u16,
    pub db: u8,
    pub pb: u8,
    pub pc: u16,
    pub p: u8,
    pub emulation_mode: bool,
    pub cycles: u64,
}

#[derive(Serialize, Deserialize)]
pub struct PpuSaveState {
    pub scanline: u16,
    pub dot: u16,
    pub frame_count: u64,
    pub vblank: bool,
    pub hblank: bool,
    pub brightness: u8,
    pub forced_blank: bool,
    pub nmi_enabled: bool,
    pub nmi_pending: bool,
    pub bg_mode: u8,
    pub mosaic_size: u8,
    pub bg_enabled: [bool; 4],
    pub bg_priority: [u8; 4],
    pub bg_scroll_x: [u16; 4],
    pub bg_scroll_y: [u16; 4],
    pub bg_tilemap_address: [u16; 4],
    pub bg_character_address: [u16; 4],
    pub vram: Vec<u8>,
    pub cgram: Vec<u8>,
    pub oam: Vec<u8>,
    pub vram_address: u16,
    pub vram_increment: u16,
    pub cgram_address: u8,
    pub oam_address: u16,
}

#[derive(Serialize, Deserialize)]
pub struct ApuSaveState {
    pub ram: Vec<u8>,
    pub ports: [u8; 4],
    pub dsp_registers: Vec<u8>,
    pub cycle_counter: u64,
    pub timers: Vec<TimerSaveState>,
    pub channels: Vec<SoundChannelSaveState>,
    pub master_volume_left: u8,
    pub master_volume_right: u8,
    pub echo_volume_left: u8,
    pub echo_volume_right: u8,
}

#[derive(Serialize, Deserialize)]
pub struct TimerSaveState {
    pub enabled: bool,
    pub target: u8,
    pub counter: u8,
    pub divider: u16,
    pub divider_target: u16,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct SoundChannelSaveState {
    pub volume_left: u8,
    pub volume_right: u8,
    pub pitch: u16,
    pub sample_start: u16,
    pub sample_loop: u16,
    pub envelope: EnvelopeSaveState,
    pub enabled: bool,
    pub current_sample: u16,
    pub phase: u32,
    pub amplitude: i16,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct EnvelopeSaveState {
    pub attack_rate: u8,
    pub decay_rate: u8,
    pub sustain_level: u8,
    pub release_rate: u8,
    pub current_level: u16,
    pub state: u8, // EnvelopeState as u8
}

#[derive(Serialize, Deserialize)]
pub struct MemoryState {
    pub wram: Vec<u8>,
    pub sram: Vec<u8>,
}

#[derive(Serialize, Deserialize, Default)]
pub struct InputSaveState {
    pub controller1_buttons: u16,
    pub controller2_buttons: u16,
    #[serde(default)]
    pub controller3_buttons: u16,
    #[serde(default)]
    pub controller4_buttons: u16,
    pub controller1_shift_register: u16,
    pub controller2_shift_register: u16,
    #[serde(default)]
    pub controller3_shift_register: u16,
    #[serde(default)]
    pub controller4_shift_register: u16,
    pub controller1_latched_buttons: u16,
    pub controller2_latched_buttons: u16,
    #[serde(default)]
    pub controller3_latched_buttons: u16,
    #[serde(default)]
    pub controller4_latched_buttons: u16,
    pub strobe: bool,
    #[serde(default)]
    pub multitap_enabled: bool,
}

impl SaveState {
    pub const CURRENT_VERSION: u32 = 1;

    pub fn new() -> Self {
        Self {
            version: Self::CURRENT_VERSION,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            cpu_state: CpuSaveState::default(),
            ppu_state: PpuSaveState::default(),
            apu_state: ApuSaveState::default(),
            memory_state: MemoryState::default(),
            input_state: InputSaveState::default(),
            master_cycles: 0,
            frame_count: 0,
            rom_checksum: 0,
        }
    }

    pub fn save_to_file(&self, filename: &str) -> Result<(), String> {
        let compressed_data = self.compress()?;
        let mut file =
            File::create(filename).map_err(|e| format!("Failed to create save file: {}", e))?;

        file.write_all(&compressed_data)
            .map_err(|e| format!("Failed to write save file: {}", e))?;

        Ok(())
    }

    pub fn load_from_file(filename: &str) -> Result<Self, String> {
        let mut file =
            File::open(filename).map_err(|e| format!("Failed to open save file: {}", e))?;

        let mut compressed_data = Vec::new();
        file.read_to_end(&mut compressed_data)
            .map_err(|e| format!("Failed to read save file: {}", e))?;

        Self::decompress(&compressed_data)
    }

    fn compress(&self) -> Result<Vec<u8>, String> {
        let json = serde_json::to_string(self)
            .map_err(|e| format!("Failed to serialize save state: {}", e))?;

        // Simple compression - store as JSON for now
        // In the future, we could add proper compression like zlib
        Ok(json.into_bytes())
    }

    fn decompress(data: &[u8]) -> Result<Self, String> {
        let json = String::from_utf8(data.to_vec())
            .map_err(|e| format!("Invalid save file format: {}", e))?;

        let save_state: SaveState = serde_json::from_str(&json)
            .map_err(|e| format!("Failed to deserialize save state: {}", e))?;

        if save_state.version > Self::CURRENT_VERSION {
            return Err(format!(
                "Save state version {} is not supported (current: {})",
                save_state.version,
                Self::CURRENT_VERSION
            ));
        }

        Ok(save_state)
    }

    pub fn validate_rom_checksum(&self, current_checksum: u32) -> bool {
        self.rom_checksum == current_checksum
    }

    #[allow(dead_code)]
    pub fn get_save_info(&self) -> SaveInfo {
        SaveInfo {
            version: self.version,
            timestamp: self.timestamp,
            frame_count: self.frame_count,
            rom_checksum: self.rom_checksum,
        }
    }
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct SaveInfo {
    pub version: u32,
    pub timestamp: u64,
    pub frame_count: u64,
    pub rom_checksum: u32,
}

impl Default for CpuSaveState {
    fn default() -> Self {
        Self {
            a: 0,
            x: 0,
            y: 0,
            sp: 0x01FF,
            dp: 0,
            db: 0,
            pb: 0,
            pc: 0,
            p: 0x34, // IRQ_DISABLE | MEMORY_8BIT | INDEX_8BIT
            emulation_mode: true,
            cycles: 0,
        }
    }
}

impl Default for PpuSaveState {
    fn default() -> Self {
        Self {
            scanline: 0,
            dot: 0,
            frame_count: 0,
            vblank: false,
            hblank: false,
            brightness: 15,
            forced_blank: true,
            nmi_enabled: false,
            nmi_pending: false,
            bg_mode: 0,
            mosaic_size: 1,
            bg_enabled: [false; 4],
            bg_priority: [0; 4],
            bg_scroll_x: [0; 4],
            bg_scroll_y: [0; 4],
            bg_tilemap_address: [0; 4],
            bg_character_address: [0; 4],
            vram: vec![0; 0x10000],
            cgram: vec![0; 0x200],
            oam: vec![0; 0x220],
            vram_address: 0,
            vram_increment: 1,
            cgram_address: 0,
            oam_address: 0,
        }
    }
}

impl Default for ApuSaveState {
    fn default() -> Self {
        Self {
            ram: vec![0; 0x10000],
            ports: [0; 4],
            dsp_registers: vec![0; 128],
            cycle_counter: 0,
            timers: vec![
                TimerSaveState {
                    enabled: false,
                    target: 0,
                    counter: 0,
                    divider: 0,
                    divider_target: 128,
                },
                TimerSaveState {
                    enabled: false,
                    target: 0,
                    counter: 0,
                    divider: 0,
                    divider_target: 128,
                },
                TimerSaveState {
                    enabled: false,
                    target: 0,
                    counter: 0,
                    divider: 0,
                    divider_target: 16,
                },
            ],
            channels: vec![SoundChannelSaveState::default(); 8],
            master_volume_left: 127,
            master_volume_right: 127,
            echo_volume_left: 0,
            echo_volume_right: 0,
        }
    }
}

impl Default for EnvelopeSaveState {
    fn default() -> Self {
        Self {
            attack_rate: 0,
            decay_rate: 0,
            sustain_level: 0,
            release_rate: 0,
            current_level: 0,
            state: 3, // EnvelopeState::Release
        }
    }
}

impl Default for MemoryState {
    fn default() -> Self {
        Self {
            wram: vec![0; 0x20000],
            sram: vec![0; 0x8000],
        }
    }
}
