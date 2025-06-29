use super::registers::PpuRegisters;
use super::memory::PpuMemory;

/// Background rendering with shift registers
#[derive(Debug, Clone)]
pub struct BackgroundRenderer {
    // Shift registers for current tile being rendered
    tile_shift_low: u16,
    tile_shift_high: u16,
    attr_shift_low: u16,
    attr_shift_high: u16,
    
    // Next tile data waiting to be loaded
    next_tile_id: u8,
    next_tile_attr: u8,
    next_tile_low: u8,
    next_tile_high: u8,
}

impl BackgroundRenderer {
    pub fn new() -> Self {
        Self {
            tile_shift_low: 0,
            tile_shift_high: 0,
            attr_shift_low: 0,
            attr_shift_high: 0,
            next_tile_id: 0,
            next_tile_attr: 0,
            next_tile_low: 0,
            next_tile_high: 0,
        }
    }
    
    pub fn shift_registers(&mut self) {
        self.tile_shift_low <<= 1;
        self.tile_shift_high <<= 1;
        self.attr_shift_low <<= 1;
        self.attr_shift_high <<= 1;
    }
    
    pub fn load_shift_registers(&mut self) {
        // Load new tile data into lower 8 bits
        self.tile_shift_low = (self.tile_shift_low & 0xFF00) | self.next_tile_low as u16;
        self.tile_shift_high = (self.tile_shift_high & 0xFF00) | self.next_tile_high as u16;
        
        // Load attribute data
        let attr_bits = self.next_tile_attr & 0x03;
        let attr_low_byte = if attr_bits & 1 != 0 { 0xFF } else { 0x00 };
        let attr_high_byte = if attr_bits & 2 != 0 { 0xFF } else { 0x00 };
        
        self.attr_shift_low = (self.attr_shift_low & 0xFF00) | attr_low_byte;
        self.attr_shift_high = (self.attr_shift_high & 0xFF00) | attr_high_byte;
    }
    
    pub fn get_pixel(&self, fine_x: u8) -> (u8, u8) {
        if fine_x >= 8 {
            return (0, 0); // Should not happen
        }
        
        let bit_position = 15 - fine_x;
        
        // Get tile pixel
        let pixel_low = (self.tile_shift_low >> bit_position) & 1;
        let pixel_high = (self.tile_shift_high >> bit_position) & 1;
        let pixel_value = (pixel_high << 1) | pixel_low;
        
        // Get attribute bits
        let attr_low = (self.attr_shift_low >> bit_position) & 1;
        let attr_high = (self.attr_shift_high >> bit_position) & 1;
        let palette_select = (attr_high << 1) | attr_low;
        
        (pixel_value as u8, palette_select as u8)
    }
    
    pub fn fetch_tile_data(&mut self, 
                          registers: &PpuRegisters, 
                          memory: &PpuMemory, 
                          cartridge: Option<&crate::cartridge::Cartridge>) {
        if let Some(cart) = cartridge {
            // Extract address components
            let coarse_x = registers.v & 0x1F;
            let coarse_y = (registers.v >> 5) & 0x1F;
            let fine_y = (registers.v >> 12) & 0x07;
            let nt_x = (registers.v >> 10) & 0x01;
            let nt_y = (registers.v >> 11) & 0x01;
            
            // Fetch tile ID
            let nt_addr = 0x2000 | (nt_y << 11) | (nt_x << 10) | (coarse_y << 5) | coarse_x;
            self.next_tile_id = memory.read_nametable(nt_addr, cart.mirroring());
            
            // Fetch attribute
            let attr_addr = 0x23C0 | (nt_y << 11) | (nt_x << 10) | ((coarse_y >> 2) << 3) | (coarse_x >> 2);
            let attr_byte = memory.read_nametable(attr_addr, cart.mirroring());
            let shift = ((coarse_y & 2) << 1) | (coarse_x & 2);
            self.next_tile_attr = (attr_byte >> shift) & 0x03;
            
            // Fetch pattern data
            let pattern_table = if registers.control.contains(super::registers::PpuControl::BG_PATTERN) { 
                0x1000 
            } else { 
                0x0000 
            };
            let tile_addr = pattern_table + (self.next_tile_id as u16 * 16) + fine_y;
            
            if tile_addr + 8 < 0x2000 {
                self.next_tile_low = cart.read_chr(tile_addr);
                self.next_tile_high = cart.read_chr(tile_addr + 8);
            }
        }
    }
}