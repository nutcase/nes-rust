use crate::cartridge::CartridgeState;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct SaveState {
    // CPU state
    pub cpu_a: u8,
    pub cpu_x: u8,
    pub cpu_y: u8,
    pub cpu_pc: u16,
    pub cpu_sp: u8,
    pub cpu_status: u8,
    pub cpu_cycles: u64,

    // PPU state
    pub ppu_control: u8,
    pub ppu_mask: u8,
    pub ppu_status: u8,
    pub ppu_oam_addr: u8,
    pub ppu_scroll_x: u8,
    pub ppu_scroll_y: u8,
    pub ppu_addr: u16,
    pub ppu_data_buffer: u8,
    pub ppu_w: bool,
    pub ppu_t: u16,
    pub ppu_v: u16,
    pub ppu_x: u8,
    pub ppu_scanline: i16,
    pub ppu_cycle: u16,
    pub ppu_frame: u64,

    // PPU memory
    pub ppu_palette: [u8; 32],
    pub ppu_nametable: Vec<u8>, // Flattened nametable data
    pub ppu_oam: Vec<u8>,

    // Main RAM
    pub ram: Vec<u8>,

    // Cartridge state
    pub cartridge_prg_bank: u8,
    pub cartridge_chr_bank: u8,
    #[serde(default)]
    pub cartridge_state: Option<CartridgeState>,

    // APU state (basic)
    pub apu_frame_counter: u8,
    pub apu_frame_interrupt: bool,

    // Additional metadata
    pub rom_filename: String,
    pub timestamp: u64,
}

#[derive(Serialize, Deserialize)]
struct LegacySaveState {
    cpu_a: u8,
    cpu_x: u8,
    cpu_y: u8,
    cpu_pc: u16,
    cpu_sp: u8,
    cpu_status: u8,
    cpu_cycles: u64,
    ppu_control: u8,
    ppu_mask: u8,
    ppu_status: u8,
    ppu_oam_addr: u8,
    ppu_scroll_x: u8,
    ppu_scroll_y: u8,
    ppu_addr: u16,
    ppu_data_buffer: u8,
    ppu_w: bool,
    ppu_t: u16,
    ppu_v: u16,
    ppu_x: u8,
    ppu_scanline: i16,
    ppu_cycle: u16,
    ppu_frame: u64,
    ppu_palette: [u8; 32],
    ppu_nametable: Vec<u8>,
    ppu_oam: Vec<u8>,
    ram: Vec<u8>,
    cartridge_prg_bank: u8,
    cartridge_chr_bank: u8,
    apu_frame_counter: u8,
    apu_frame_interrupt: bool,
    rom_filename: String,
    timestamp: u64,
}

impl From<LegacySaveState> for SaveState {
    fn from(legacy: LegacySaveState) -> Self {
        SaveState {
            cpu_a: legacy.cpu_a,
            cpu_x: legacy.cpu_x,
            cpu_y: legacy.cpu_y,
            cpu_pc: legacy.cpu_pc,
            cpu_sp: legacy.cpu_sp,
            cpu_status: legacy.cpu_status,
            cpu_cycles: legacy.cpu_cycles,
            ppu_control: legacy.ppu_control,
            ppu_mask: legacy.ppu_mask,
            ppu_status: legacy.ppu_status,
            ppu_oam_addr: legacy.ppu_oam_addr,
            ppu_scroll_x: legacy.ppu_scroll_x,
            ppu_scroll_y: legacy.ppu_scroll_y,
            ppu_addr: legacy.ppu_addr,
            ppu_data_buffer: legacy.ppu_data_buffer,
            ppu_w: legacy.ppu_w,
            ppu_t: legacy.ppu_t,
            ppu_v: legacy.ppu_v,
            ppu_x: legacy.ppu_x,
            ppu_scanline: legacy.ppu_scanline,
            ppu_cycle: legacy.ppu_cycle,
            ppu_frame: legacy.ppu_frame,
            ppu_palette: legacy.ppu_palette,
            ppu_nametable: legacy.ppu_nametable,
            ppu_oam: legacy.ppu_oam,
            ram: legacy.ram,
            cartridge_prg_bank: legacy.cartridge_prg_bank,
            cartridge_chr_bank: legacy.cartridge_chr_bank,
            cartridge_state: None,
            apu_frame_counter: legacy.apu_frame_counter,
            apu_frame_interrupt: legacy.apu_frame_interrupt,
            rom_filename: legacy.rom_filename,
            timestamp: legacy.timestamp,
        }
    }
}

impl SaveState {
    pub fn save_to_file(&self, filename: &str) -> Result<(), Box<dyn std::error::Error>> {
        let data = bincode::serialize(self)?;
        std::fs::write(filename, data)?;
        println!("Save state written to: {}", filename);
        Ok(())
    }

    pub fn load_from_file(filename: &str) -> Result<SaveState, Box<dyn std::error::Error>> {
        let data = std::fs::read(filename)?;
        if let Ok(save_state) = bincode::deserialize::<SaveState>(&data) {
            println!("Save state loaded from: {}", filename);
            return Ok(save_state);
        }

        let legacy = bincode::deserialize::<LegacySaveState>(&data)?;
        println!("Save state loaded from: {} (legacy format)", filename);
        Ok(legacy.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_legacy_save_state_defaults_new_fields() {
        let legacy = LegacySaveState {
            cpu_a: 1,
            cpu_x: 2,
            cpu_y: 3,
            cpu_pc: 0x8000,
            cpu_sp: 0xFD,
            cpu_status: 0x24,
            cpu_cycles: 123,
            ppu_control: 0,
            ppu_mask: 0,
            ppu_status: 0,
            ppu_oam_addr: 0,
            ppu_scroll_x: 0,
            ppu_scroll_y: 0,
            ppu_addr: 0,
            ppu_data_buffer: 0,
            ppu_w: false,
            ppu_t: 0,
            ppu_v: 0,
            ppu_x: 0,
            ppu_scanline: 0,
            ppu_cycle: 0,
            ppu_frame: 0,
            ppu_palette: [0; 32],
            ppu_nametable: vec![0; 2048],
            ppu_oam: vec![0; 256],
            ram: vec![0; 0x800],
            cartridge_prg_bank: 0,
            cartridge_chr_bank: 0,
            apu_frame_counter: 0,
            apu_frame_interrupt: false,
            rom_filename: "legacy".to_string(),
            timestamp: 0,
        };

        let encoded = bincode::serialize(&legacy).expect("serialize legacy save");
        let mut path = std::env::temp_dir();
        path.push(format!(
            "nes_legacy_state_{}.sav",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system time")
                .as_nanos()
        ));
        std::fs::write(&path, encoded).expect("write legacy save");
        let decoded = SaveState::load_from_file(path.to_str().expect("utf-8 path"))
            .expect("load legacy save");
        let _ = std::fs::remove_file(path);

        assert!(decoded.cartridge_state.is_none());
    }
}
