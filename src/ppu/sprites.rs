use super::registers::{PpuRegisters, PpuControl, PpuMask};
use super::memory::PpuMemory;

/// Sprite rendering system
#[derive(Debug, Clone)]
pub struct SpriteRenderer {
    secondary_oam: [u8; 32], // 8 sprites * 4 bytes each
    sprite_count: usize,
}

impl SpriteRenderer {
    pub fn new() -> Self {
        Self {
            secondary_oam: [0xFF; 32],
            sprite_count: 0,
        }
    }
    
    pub fn evaluate_sprites(&mut self, registers: &PpuRegisters, memory: &PpuMemory) {
        self.sprite_count = 0;
        let scanline = registers.scanline;
        
        if scanline < 0 || scanline >= 240 {
            return;
        }
        
        let sprite_height = if registers.control.contains(PpuControl::SPRITE_SIZE) { 16 } else { 8 };
        
        // Find sprites on current scanline (max 8)
        for sprite_idx in 0..64 {
            if self.sprite_count >= 8 {
                // Set sprite overflow flag
                break;
            }
            
            let oam_offset = sprite_idx * 4;
            let sprite_y = memory.oam[oam_offset];
            
            // Skip off-screen sprites
            if sprite_y >= 0xEF {
                continue;
            }
            
            // Check if sprite is on current scanline
            if scanline >= sprite_y as i16 && scanline < (sprite_y as i16 + sprite_height) {
                // Copy sprite data to secondary OAM
                let secondary_offset = self.sprite_count * 4;
                for i in 0..4 {
                    self.secondary_oam[secondary_offset + i] = memory.oam[oam_offset + i];
                }
                self.sprite_count += 1;
            }
        }
    }
    
    pub fn render_pixel(&self, 
                       x: u8, 
                       y: i16, 
                       registers: &PpuRegisters, 
                       cartridge: Option<&crate::cartridge::Cartridge>) -> Option<(u8, bool, bool)> {
        if let Some(cart) = cartridge {
            // Check sprites in order of priority (sprite 0 first)
            for sprite_idx in 0..self.sprite_count {
                let base = sprite_idx * 4;
                let sprite_y = self.secondary_oam[base];
                let tile_id = self.secondary_oam[base + 1];
                let attributes = self.secondary_oam[base + 2];
                let sprite_x = self.secondary_oam[base + 3];
                
                // Check if pixel is within sprite bounds
                if x >= sprite_x && x < sprite_x + 8 {
                    let pixel_x = x - sprite_x;
                    let pixel_y = (y - sprite_y as i16) as u8;
                    
                    let sprite_height = if registers.control.contains(PpuControl::SPRITE_SIZE) { 16 } else { 8 };
                    if pixel_y >= sprite_height {
                        continue;
                    }
                    
                    // Handle sprite flipping
                    let final_x = if attributes & 0x40 != 0 { 7 - pixel_x } else { pixel_x };
                    let final_y = if attributes & 0x80 != 0 { 
                        sprite_height - 1 - pixel_y 
                    } else { 
                        pixel_y 
                    };
                    
                    // Get pattern table and tile
                    let (pattern_table, final_tile_id) = if sprite_height == 16 {
                        // 8x16 sprites
                        let pattern_table = if tile_id & 0x01 != 0 { 0x1000 } else { 0x0000 };
                        let base_tile = tile_id & 0xFE;
                        let final_tile = if final_y >= 8 { base_tile + 1 } else { base_tile };
                        (pattern_table, final_tile)
                    } else {
                        // 8x8 sprites
                        let pattern_table = if registers.control.contains(PpuControl::SPRITE_PATTERN) { 
                            0x1000 
                        } else { 
                            0x0000 
                        };
                        (pattern_table, tile_id)
                    };
                    
                    let pattern_y = final_y % 8;
                    let tile_addr = pattern_table + (final_tile_id as u16 * 16) + pattern_y as u16;
                    
                    if tile_addr + 8 < 0x2000 {
                        let pattern_low = cart.read_chr(tile_addr);
                        let pattern_high = cart.read_chr(tile_addr + 8);
                        
                        let bit = 7 - final_x;
                        let pixel_value = ((pattern_high >> bit) & 1) << 1 | ((pattern_low >> bit) & 1);
                        
                        if pixel_value != 0 {
                            // Get sprite palette
                            let palette_num = attributes & 0x03;
                            let palette_idx = 16 + palette_num * 4 + pixel_value;
                            
                            let is_sprite_0 = sprite_idx == 0;
                            let behind_bg = (attributes & 0x20) != 0;
                            
                            return Some((palette_idx, behind_bg, is_sprite_0));
                        }
                    }
                }
            }
        }
        None
    }
}