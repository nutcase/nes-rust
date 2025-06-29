mod registers;
mod memory;
mod background;
mod sprites;
mod renderer;

use registers::{PpuRegisters, PpuControl, PpuMask, PpuStatus};
use memory::PpuMemory;
use renderer::PpuRenderer;

/// New modular PPU implementation
pub struct NewPpu {
    registers: PpuRegisters,
    memory: PpuMemory,
    renderer: PpuRenderer,
}

impl NewPpu {
    pub fn new() -> Self {
        Self {
            registers: PpuRegisters::new(),
            memory: PpuMemory::new(),
            renderer: PpuRenderer::new(),
        }
    }
    
    pub fn step(&mut self, cartridge: Option<&crate::cartridge::Cartridge>) -> bool {
        let mut nmi = false;
        
        match self.registers.scanline {
            -1 => {
                // Pre-render scanline
                if self.registers.cycle == 1 {
                    self.registers.status.remove(PpuStatus::VBLANK);
                    self.registers.status.remove(PpuStatus::SPRITE_0_HIT);
                }
                
                // Update vertical scroll
                if self.registers.cycle >= 280 && self.registers.cycle <= 304 {
                    if self.rendering_enabled() {
                        self.registers.v = (self.registers.v & 0x841F) | (self.registers.t & 0x7BE0);
                    }
                }
            }
            0..=239 => {
                // Visible scanlines
                if self.registers.cycle == 0 {
                    // Sprite evaluation at the start of each scanline
                    self.renderer.sprites.evaluate_sprites(&self.registers, &self.memory);
                }
                
                if self.registers.cycle >= 1 && self.registers.cycle <= 256 {
                    // Background fetches and rendering
                    if self.rendering_enabled() {
                        // Shift registers every cycle
                        self.renderer.background.shift_registers();
                        
                        // Fetch new tile data every 8 cycles
                        if (self.registers.cycle - 1) % 8 == 0 {
                            self.renderer.background.load_shift_registers();
                            self.renderer.background.fetch_tile_data(&self.registers, &self.memory, cartridge);
                        }
                    }
                    
                    // Render pixel
                    self.renderer.render_pixel(&mut self.registers, &self.memory, cartridge);
                }
                
                // Update horizontal scroll
                if self.registers.cycle == 257 && self.rendering_enabled() {
                    self.registers.v = (self.registers.v & 0xFBE0) | (self.registers.t & 0x041F);
                }
            }
            240 => {
                // Post-render scanline (idle)
            }
            241 => {
                // VBlank start
                if self.registers.cycle == 1 {
                    self.registers.status.insert(PpuStatus::VBLANK);
                    if self.registers.control.contains(PpuControl::NMI_ENABLE) {
                        nmi = true;
                    }
                }
            }
            242..=260 => {
                // VBlank period
            }
            _ => {}
        }
        
        // Advance timing
        self.registers.cycle += 1;
        if self.registers.cycle >= 341 {
            self.registers.cycle = 0;
            self.registers.scanline += 1;
            
            if self.registers.scanline >= 261 {
                self.registers.scanline = -1;
                self.registers.frame += 1;
            }
        }
        
        nmi
    }
    
    pub fn read_register(&mut self, addr: u16) -> u8 {
        match addr {
            0x2002 => {
                let status = self.registers.status.bits();
                self.registers.w = false;
                self.registers.status.remove(PpuStatus::VBLANK);
                status
            }
            0x2004 => self.memory.oam[self.registers.oam_addr as usize],
            0x2007 => {
                let data = if self.registers.v >= 0x3F00 {
                    self.memory.read_palette(self.registers.v)
                } else if self.registers.v >= 0x2000 && self.registers.v < 0x3000 {
                    if let Some(cart) = None as Option<&crate::cartridge::Cartridge> {
                        self.memory.read_nametable(self.registers.v, cart.mirroring())
                    } else {
                        0
                    }
                } else {
                    0
                };
                
                self.registers.increment_vram_addr();
                data
            }
            _ => 0
        }
    }
    
    pub fn write_register(&mut self, addr: u16, data: u8) {
        match addr {
            0x2000 => {
                self.registers.control = PpuControl::from_bits_truncate(data);
                self.registers.t = (self.registers.t & 0xF3FF) | ((data as u16 & 0x03) << 10);
            }
            0x2001 => {
                self.registers.mask = PpuMask::from_bits_truncate(data);
            }
            0x2003 => {
                self.registers.oam_addr = data;
            }
            0x2004 => {
                self.memory.oam[self.registers.oam_addr as usize] = data;
                self.registers.oam_addr = self.registers.oam_addr.wrapping_add(1);
            }
            0x2005 => {
                if !self.registers.w {
                    self.registers.t = (self.registers.t & 0xFFE0) | (data as u16 >> 3);
                    self.registers.x = data & 0x07;
                    self.registers.w = true;
                } else {
                    self.registers.t = (self.registers.t & 0x8FFF) | ((data as u16 & 0x07) << 12);
                    self.registers.t = (self.registers.t & 0xFC1F) | ((data as u16 & 0xF8) << 2);
                    self.registers.w = false;
                }
            }
            0x2006 => {
                if !self.registers.w {
                    self.registers.t = (self.registers.t & 0x80FF) | ((data as u16 & 0x3F) << 8);
                    self.registers.w = true;
                } else {
                    self.registers.t = (self.registers.t & 0xFF00) | data as u16;
                    self.registers.v = self.registers.t;
                    self.registers.w = false;
                }
            }
            0x2007 => {
                if self.registers.v >= 0x3F00 {
                    self.memory.write_palette(self.registers.v, data);
                } else if self.registers.v >= 0x2000 && self.registers.v < 0x3000 {
                    if let Some(cart) = None as Option<&crate::cartridge::Cartridge> {
                        self.memory.write_nametable(self.registers.v, data, cart.mirroring());
                    }
                }
                
                self.registers.increment_vram_addr();
            }
            _ => {}
        }
    }
    
    pub fn get_buffer(&self) -> &[u8] {
        self.renderer.get_frame_buffer()
    }
    
    fn rendering_enabled(&self) -> bool {
        self.registers.mask.contains(PpuMask::BG_ENABLE) || 
        self.registers.mask.contains(PpuMask::SPRITE_ENABLE)
    }
}