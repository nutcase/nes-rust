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
    
    // APU state (basic)
    pub apu_frame_counter: u8,
    pub apu_frame_interrupt: bool,
    
    // Additional metadata
    pub rom_filename: String,
    pub timestamp: u64,
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
        let save_state = bincode::deserialize(&data)?;
        println!("Save state loaded from: {}", filename);
        Ok(save_state)
    }
}