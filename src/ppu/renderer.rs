use super::registers::{PpuRegisters, PpuMask, PpuStatus};
use super::memory::PpuMemory;
use super::background::BackgroundRenderer;
use super::sprites::SpriteRenderer;

/// Main PPU renderer that coordinates background and sprites
#[derive(Debug, Clone)]
pub struct PpuRenderer {
    pub background: BackgroundRenderer,
    pub sprites: SpriteRenderer,
    pub frame_buffer: Vec<u8>,
}

impl PpuRenderer {
    pub fn new() -> Self {
        Self {
            background: BackgroundRenderer::new(),
            sprites: SpriteRenderer::new(),
            frame_buffer: vec![0x40; 256 * 240 * 3], // Initialize with gray
        }
    }
    
    pub fn render_pixel(&mut self, 
                       registers: &mut PpuRegisters, 
                       memory: &PpuMemory, 
                       cartridge: Option<&crate::cartridge::Cartridge>) {
        let x = registers.cycle.saturating_sub(1);
        let y = registers.scanline;
        
        if x >= 256 || y < 0 || y >= 240 {
            return;
        }
        
        let mut final_color = memory.palettes[0]; // Background color
        let mut bg_pixel = 0u8;
        let mut sprite_0_hit = false;
        
        // Simplified background rendering without scrolling
        if registers.mask.contains(PpuMask::BG_ENABLE) {
            if let Some(cart) = cartridge {
                let tile_x = (x / 8) as u8;
                let tile_y = (y / 8) as u8;
                let fine_x = (x % 8) as u8;
                let fine_y = (y % 8) as u8;
                
                if tile_x < 32 && tile_y < 30 {
                    // Read from nametable 0 (0x2000)
                    let nt_addr = 0x2000 + ((tile_y as u16) << 5) + (tile_x as u16);
                    let tile_id = memory.read_nametable(nt_addr, cart.mirroring());
                    
                    // Get pattern table data
                    let pattern_table = if registers.control.contains(super::registers::PpuControl::BG_PATTERN) { 0x1000 } else { 0x0000 };
                    let tile_addr = pattern_table + (tile_id as u16 * 16) + fine_y as u16;
                    
                    let pattern_low = cart.read_chr(tile_addr);
                    let pattern_high = cart.read_chr(tile_addr + 8);
                    
                    let bit = 7 - fine_x;
                    let pixel_val = (1 & (pattern_low >> bit)) | ((1 & (pattern_high >> bit)) << 1);
                    
                    if pixel_val != 0 {
                        // Get attribute data for palette
                        let attr_x = tile_x / 4;
                        let attr_y = tile_y / 4;
                        let attr_addr = 0x23C0 + (attr_y as u16 * 8) + attr_x as u16;
                        let attr_byte = memory.read_nametable(attr_addr, cart.mirroring());
                        
                        let quad_x = (tile_x % 4) / 2;
                        let quad_y = (tile_y % 4) / 2;
                        let shift = (quad_y * 2 + quad_x) * 2;
                        let palette_select = (attr_byte >> shift) & 0x03;
                        
                        let palette_idx = (palette_select as usize * 4) + pixel_val as usize;
                        if palette_idx < 16 {
                            final_color = memory.palettes[palette_idx];
                            bg_pixel = pixel_val;
                        }
                    }
                }
            }
        }
        
        // Render sprites
        if registers.mask.contains(PpuMask::SPRITE_ENABLE) {
            if let Some((sprite_palette_idx, behind_bg, is_sprite_0)) = 
                self.sprites.render_pixel(x as u8, y, registers, cartridge) {
                
                // Check sprite 0 hit
                if is_sprite_0 && bg_pixel != 0 && x < 255 {
                    // Additional checks for accurate sprite 0 hit
                    let left_clip_sprite = !registers.mask.contains(PpuMask::SPRITE_LEFT_ENABLE) && x < 8;
                    let left_clip_bg = !registers.mask.contains(PpuMask::BG_LEFT_ENABLE) && x < 8;
                    
                    if !left_clip_sprite && !left_clip_bg {
                        sprite_0_hit = true;
                    }
                }
                
                // Determine final pixel color
                if !behind_bg || bg_pixel == 0 {
                    if (sprite_palette_idx as usize) < 32 {
                        final_color = memory.palettes[sprite_palette_idx as usize];
                    }
                }
            }
        }
        
        // Set sprite 0 hit flag
        if sprite_0_hit {
            registers.status.insert(PpuStatus::SPRITE_0_HIT);
        }
        
        // Convert NES color to RGB
        let rgb = get_nes_color(final_color);
        let pixel_index = ((y as usize * 256) + x as usize) * 3;
        
        if pixel_index + 2 < self.frame_buffer.len() {
            self.frame_buffer[pixel_index] = rgb.0;
            self.frame_buffer[pixel_index + 1] = rgb.1;
            self.frame_buffer[pixel_index + 2] = rgb.2;
        }
    }
    
    pub fn get_frame_buffer(&self) -> &[u8] {
        &self.frame_buffer
    }
}

fn get_nes_color(index: u8) -> (u8, u8, u8) {
    let palette = [
        (0x80, 0x80, 0x80), (0x00, 0x3D, 0xA6), (0x00, 0x12, 0xB0), (0x44, 0x00, 0x96),
        (0xA1, 0x00, 0x5E), (0xC7, 0x00, 0x28), (0xBA, 0x06, 0x00), (0x8C, 0x17, 0x00),
        (0x5C, 0x2F, 0x00), (0x10, 0x45, 0x00), (0x05, 0x4A, 0x00), (0x00, 0x47, 0x2E),
        (0x00, 0x41, 0x66), (0x00, 0x00, 0x00), (0x05, 0x05, 0x05), (0x05, 0x05, 0x05),
        (0xC7, 0xC7, 0xC7), (0x00, 0x77, 0xFF), (0x21, 0x55, 0xFF), (0x82, 0x37, 0xFA),
        (0xEB, 0x2F, 0xB5), (0xFF, 0x29, 0x50), (0xFF, 0x22, 0x00), (0xD6, 0x32, 0x00),
        (0xC4, 0x62, 0x00), (0x35, 0x80, 0x00), (0x05, 0x8F, 0x00), (0x00, 0x8A, 0x55),
        (0x00, 0x99, 0xCC), (0x21, 0x21, 0x21), (0x09, 0x09, 0x09), (0x09, 0x09, 0x09),
        (0xFF, 0xFF, 0xFF), (0x0F, 0xD7, 0xFF), (0x69, 0xA2, 0xFF), (0xD4, 0x80, 0xFF),
        (0xFF, 0x45, 0xF3), (0xFF, 0x61, 0x8B), (0xFF, 0x88, 0x33), (0xFF, 0x9C, 0x12),
        (0xFA, 0xBC, 0x20), (0x9F, 0xE3, 0x0E), (0x2B, 0xF0, 0x35), (0x0C, 0xF0, 0xA4),
        (0x05, 0xFB, 0xFF), (0x5E, 0x5E, 0x5E), (0x0D, 0x0D, 0x0D), (0x0D, 0x0D, 0x0D),
        (0xFF, 0xFF, 0xFF), (0xA6, 0xFC, 0xFF), (0xB3, 0xEC, 0xFF), (0xDA, 0xAB, 0xEB),
        (0xFF, 0xA8, 0xF9), (0xFF, 0xAB, 0xB3), (0xFF, 0xD2, 0xB0), (0xFF, 0xEF, 0xA6),
        (0xFF, 0xF7, 0x9C), (0xD7, 0xFF, 0xB3), (0xC6, 0xFF, 0xC2), (0xC6, 0xFF, 0xD7),
        (0xC4, 0xFF, 0xFF), (0xB9, 0xB9, 0xB9), (0xA4, 0xA4, 0xA4), (0xA4, 0xA4, 0xA4),
    ];
    
    palette.get(index as usize).copied().unwrap_or((0, 0, 0))
}