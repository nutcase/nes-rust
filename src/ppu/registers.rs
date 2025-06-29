use bitflags::bitflags;

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct PpuControl: u8 {
        const NAMETABLE_X = 0b00000001;
        const NAMETABLE_Y = 0b00000010;
        const VRAM_INCREMENT = 0b00000100;
        const SPRITE_PATTERN = 0b00001000;
        const BG_PATTERN = 0b00010000;
        const SPRITE_SIZE = 0b00100000;
        const PPU_MASTER_SLAVE = 0b01000000;
        const NMI_ENABLE = 0b10000000;
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct PpuMask: u8 {
        const GRAYSCALE = 0b00000001;
        const BG_LEFT_ENABLE = 0b00000010;
        const SPRITE_LEFT_ENABLE = 0b00000100;
        const BG_ENABLE = 0b00001000;
        const SPRITE_ENABLE = 0b00010000;
        const EMPHASIZE_RED = 0b00100000;
        const EMPHASIZE_GREEN = 0b01000000;
        const EMPHASIZE_BLUE = 0b10000000;
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct PpuStatus: u8 {
        const SPRITE_OVERFLOW = 0b00100000;
        const SPRITE_0_HIT = 0b01000000;
        const VBLANK = 0b10000000;
    }
}

/// PPU registers and internal state
#[derive(Debug, Clone)]
pub struct PpuRegisters {
    pub control: PpuControl,
    pub mask: PpuMask,
    pub status: PpuStatus,
    pub oam_addr: u8,
    
    // Scroll registers (Loopy registers)
    pub v: u16,  // Current VRAM address
    pub t: u16,  // Temporary VRAM address
    pub x: u8,   // Fine X scroll
    pub w: bool, // Write toggle
    
    // Timing
    pub cycle: u16,
    pub scanline: i16,
    pub frame: u64,
}

impl PpuRegisters {
    pub fn new() -> Self {
        Self {
            control: PpuControl::empty(),
            mask: PpuMask::empty(),
            status: PpuStatus::VBLANK, // Start with VBLANK set
            oam_addr: 0,
            v: 0,
            t: 0,
            x: 0,
            w: false,
            cycle: 0,
            scanline: -1,
            frame: 0,
        }
    }
    
    pub fn increment_vram_addr(&mut self) {
        let increment = if self.control.contains(PpuControl::VRAM_INCREMENT) { 32 } else { 1 };
        self.v = self.v.wrapping_add(increment);
    }
    
    pub fn get_nametable_select(&self) -> (u8, u8) {
        let nt_x = (self.control.bits() & PpuControl::NAMETABLE_X.bits()) >> 0;
        let nt_y = (self.control.bits() & PpuControl::NAMETABLE_Y.bits()) >> 1;
        (nt_x, nt_y)
    }
}