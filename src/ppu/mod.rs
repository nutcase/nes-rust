use bitflags::bitflags;

#[cfg(test)]
mod tests;

// NES Color Palette (RGB values)
const PALETTE_COLORS: [(u8, u8, u8); 64] = [
    (84, 84, 84), (0, 30, 116), (8, 16, 144), (48, 0, 136), (68, 0, 100), (92, 0, 48), (84, 4, 0), (60, 24, 0),
    (32, 42, 0), (8, 58, 0), (0, 64, 0), (0, 60, 0), (0, 50, 60), (0, 0, 0), (0, 0, 0), (0, 0, 0),
    (152, 150, 152), (8, 76, 196), (48, 50, 236), (92, 30, 228), (136, 20, 176), (160, 20, 100), (152, 34, 32), (120, 60, 0),
    (84, 90, 0), (40, 114, 0), (8, 124, 0), (0, 118, 40), (0, 102, 120), (0, 0, 0), (0, 0, 0), (0, 0, 0),
    (236, 238, 236), (76, 154, 236), (120, 124, 236), (176, 98, 236), (228, 84, 236), (236, 88, 180), (236, 106, 100), (212, 136, 32),
    (160, 170, 0), (116, 196, 0), (76, 208, 32), (56, 204, 108), (56, 180, 204), (60, 60, 60), (0, 0, 0), (0, 0, 0),
    (236, 238, 236), (168, 204, 236), (188, 188, 236), (212, 178, 236), (236, 174, 236), (236, 174, 212), (236, 180, 176), (228, 196, 144),
    (204, 210, 120), (180, 222, 120), (168, 226, 144), (152, 226, 180), (160, 214, 228), (160, 162, 160), (0, 0, 0), (0, 0, 0),
];

bitflags! {
    #[derive(Debug, Clone, Copy)]
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
    #[derive(Debug, Clone, Copy)]
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
    #[derive(Debug, Clone, Copy)]
    pub struct PpuStatus: u8 {
        const SPRITE_OVERFLOW = 0b00100000;
        const SPRITE_0_HIT = 0b01000000;
        const VBLANK = 0b10000000;
    }
}

pub struct Ppu {
    #[cfg(test)]
    pub control: PpuControl,
    #[cfg(not(test))]
    control: PpuControl,
    
    #[cfg(test)]
    pub mask: PpuMask,
    #[cfg(not(test))]
    mask: PpuMask,
    
    #[cfg(test)]
    pub status: PpuStatus,
    #[cfg(not(test))]
    status: PpuStatus,
    
    #[cfg(test)]
    pub oam_addr: u8,
    #[cfg(not(test))]
    oam_addr: u8,
    
    #[cfg(test)]
    pub v: u16,
    #[cfg(not(test))]
    v: u16,
    
    #[cfg(test)]
    pub t: u16,
    #[cfg(not(test))]
    t: u16,
    
    #[cfg(test)]
    pub x: u8,
    #[cfg(not(test))]
    x: u8,
    
    #[cfg(test)]
    pub w: bool,
    #[cfg(not(test))]
    w: bool,
    
    #[cfg(test)]
    pub cycle: u16,
    #[cfg(not(test))]
    cycle: u16,
    
    #[cfg(test)]
    pub scanline: i16,
    #[cfg(not(test))]
    scanline: i16,
    
    frame: u64,
    
    // Flag to preserve forced rendering from being overwritten (disabled for natural rendering)
    force_rendered_frame: bool,
    
    #[cfg(test)]
    pub nametable: [[u8; 1024]; 2],
    #[cfg(not(test))]
    nametable: [[u8; 1024]; 2],
    
    #[cfg(test)]
    pub palette: [u8; 32],
    #[cfg(not(test))]
    palette: [u8; 32],
    
    #[cfg(test)]
    pub oam: [u8; 256],
    #[cfg(not(test))]
    oam: [u8; 256],
    
    buffer: Vec<u8>,
    
    // PPU $2007 read buffer for CHR-ROM reads
    read_buffer: u8,
    
    // NMI suppression for race condition handling
    nmi_suppressed: bool,

    // VBlank flag management
    vblank_flag_set_this_frame: bool,

    // Pending NMI from edge-triggered NMI_ENABLE write during VBlank
    pending_nmi: bool,

    // Set to true when the PPU completes a full frame (scanline wraps from 260 to -1)
    pub frame_complete: bool,

    // Cached rendering_enabled flag — updated on $2001 write
    rendering_enabled: bool,

    // Per-scanline sprite cache: (sprite_num, y, tile_id, attributes, x)
    scanline_sprites: [(u8, u8, u8, u8, u8); 8],
    scanline_sprite_count: u8,
}

impl Ppu {
    pub fn new() -> Self {
        let ppu = Ppu {
            control: PpuControl::empty(),
            mask: PpuMask::empty(),
            // NES-accurate: VBlank is often set at power-on
            // Many games expect this for proper initialization
            status: PpuStatus::VBLANK,
            oam_addr: 0,
            
            v: 0,
            t: 0,
            x: 0,
            w: false,
            
            cycle: 0,
            scanline: -1,
            frame: 0,
            force_rendered_frame: false,
            
            nametable: [[0; 1024]; 2],
            palette: [0x0F; 32], // Initialize with black (0x0F)
            oam: [0xFF; 256],    // Initialize OAM with 0xFF (sprites off-screen)
            
            buffer: {
                let mut buf = Vec::new();
                // Initialize with dark background
                for _ in 0..(256 * 240) {
                    buf.push(5); // R - Dark gray
                    buf.push(5); // G - Dark gray
                    buf.push(5); // B - Dark gray
                }
                buf
            },
            read_buffer: 0,
            nmi_suppressed: false,
            vblank_flag_set_this_frame: false,
            pending_nmi: false,
            frame_complete: false,
            rendering_enabled: false,
            scanline_sprites: [(0, 0, 0, 0, 0); 8],
            scanline_sprite_count: 0,
        };

        ppu
    }

    #[inline]
    pub fn step(&mut self, cartridge: Option<&crate::cartridge::Cartridge>) -> bool {
        let mut nmi = false;

        // Check for edge-triggered NMI from $2000 write
        if self.pending_nmi {
            self.pending_nmi = false;
            nmi = true;
        }

        match self.scanline {
            -1 => {
                // Pre-render scanline - clear flags at cycle 1
                if self.cycle == 1 {
                    self.vblank_flag_set_this_frame = false;
                    self.status.remove(PpuStatus::VBLANK);
                    self.status.remove(PpuStatus::SPRITE_0_HIT);
                    self.status.remove(PpuStatus::SPRITE_OVERFLOW);
                }
                
                // Copy horizontal scroll bits from t to v at cycle 257
                if self.cycle == 257 && self.rendering_enabled {
                    self.v = (self.v & !0x041F) | (self.t & 0x041F);
                }

                // Update vertical scroll during pre-render scanline
                if self.cycle >= 280 && self.cycle <= 304 {
                    if self.rendering_enabled {
                        // Copy vertical scroll bits from t to v
                        self.v = (self.v & !0x7BE0) | (self.t & 0x7BE0);
                    }
                }
            }
            0..=239 => {
                // Visible scanlines

                // Evaluate sprites for this scanline at cycle 0
                if self.cycle == 0 {
                    self.evaluate_scanline_sprites(cartridge);
                }

                if self.cycle >= 1 && self.cycle <= 256 {
                    self.render_pixel(cartridge);

                    // Increment coarse X every 8 pixels
                    if self.cycle & 7 == 0 {
                        self.increment_coarse_x();
                    }
                }

                // Increment Y at cycle 256
                if self.cycle == 256 {
                    self.increment_y();
                }

                // Copy horizontal scroll bits from t to v at cycle 257
                if self.cycle == 257 && self.rendering_enabled {
                    self.v = (self.v & !0x041F) | (self.t & 0x041F);
                }
                
                // Removed automatic force rendering to fix frame synchronization issues
            }
            240 => {
                // Post-render scanline - no sprite evaluation needed here anymore
                // Sprite evaluation is now done at the start of each visible scanline
            }
            241 => {
                if self.cycle == 1 {
                    self.vblank_flag_set_this_frame = true;
                    self.status.insert(PpuStatus::VBLANK);

                    if self.control.contains(PpuControl::NMI_ENABLE) && !self.nmi_suppressed {
                        nmi = true;
                    }

                    self.nmi_suppressed = false;
                }
            }
            242..=260 => {
                // Keep VBlank flag set during VBlank period
                // VBlank period runs from scanline 241 to 260
            }
            _ => {}
        }

        self.cycle += 1;

        // Odd-frame cycle skip: on pre-render scanline of odd frames,
        // skip the last cycle (340) when rendering is enabled
        let cycle_limit = if self.scanline == -1
            && self.rendering_enabled
            && (self.frame & 1) == 1
        {
            340
        } else {
            341
        };

        if self.cycle >= cycle_limit {
            self.cycle = 0;
            self.scanline += 1;

            if self.scanline >= 261 {
                self.scanline = -1;
                self.frame += 1;
                self.frame_complete = true;

                // Reset force rendering flag for next frame
                self.force_rendered_frame = false;
            }
        }

        nmi
    }

    pub fn force_full_render(&mut self, cartridge: Option<&crate::cartridge::Cartridge>) {
        if let Some(cart) = cartridge {
            for tile_y in 0..30 {
                for tile_x in 0..32 {
                    let nametable_offset = tile_y * 32 + tile_x;
                    let tile_index = self.nametable[0][nametable_offset];

                    let pattern_table_base = if self.control.contains(PpuControl::BG_PATTERN) { 0x1000 } else { 0x0000 };
                    let tile_addr = pattern_table_base + (tile_index as u16) * 16;

                    let mut tile_data = [0u8; 16];
                    for i in 0..16 {
                        tile_data[i] = cart.read_chr(tile_addr + i as u16);
                    }

                    for pixel_y in 0..8 {
                        let low_byte = tile_data[pixel_y];
                        let high_byte = tile_data[pixel_y + 8];

                        for pixel_x in 0..8 {
                            let bit = 7 - pixel_x;
                            let low_bit = (low_byte >> bit) & 1;
                            let high_bit = (high_byte >> bit) & 1;
                            let pixel_value = (high_bit << 1) | low_bit;

                            let screen_x = tile_x * 8 + pixel_x;
                            let screen_y = tile_y * 8 + pixel_y;

                            if screen_x < 256 && screen_y < 240 {
                                let buffer_index = (screen_y * 256 + screen_x) * 3;
                                if buffer_index + 2 < self.buffer.len() {
                                    let palette_index = if pixel_value == 0 {
                                        self.palette[0]
                                    } else {
                                        self.palette[pixel_value as usize]
                                    };
                                    let color = PALETTE_COLORS[palette_index as usize];

                                    self.buffer[buffer_index] = color.0;
                                    self.buffer[buffer_index + 1] = color.1;
                                    self.buffer[buffer_index + 2] = color.2;
                                }
                            }
                        }
                    }
                }
            }
        }

        self.force_rendered_frame = true;
    }

    // rendering_enabled is now a cached field updated on $2001 write

    #[inline]
    fn resolve_nametable(&self, logical_nt: usize, cartridge: Option<&crate::cartridge::Cartridge>) -> usize {
        if let Some(cart) = cartridge {
            match cart.mirroring() {
                crate::cartridge::Mirroring::Vertical => match logical_nt & 3 {
                    0 | 2 => 0,
                    1 | 3 => 1,
                    _ => 0,
                },
                crate::cartridge::Mirroring::Horizontal => match logical_nt & 3 {
                    0 | 1 => 0,
                    2 | 3 => 1,
                    _ => 0,
                },
                crate::cartridge::Mirroring::FourScreen => logical_nt & 1,
                crate::cartridge::Mirroring::OneScreenLower => 0,
                crate::cartridge::Mirroring::OneScreenUpper => 1,
            }
        } else {
            logical_nt & 1
        }
    }

    #[inline]
    fn increment_coarse_x(&mut self) {
        if !self.rendering_enabled { return; }
        if (self.v & 0x001F) == 31 {
            self.v &= !0x001F;   // coarse X = 0
            self.v ^= 0x0400;    // toggle horizontal nametable
        } else {
            self.v += 1;
        }
    }

    #[inline]
    fn increment_y(&mut self) {
        if !self.rendering_enabled { return; }
        if (self.v & 0x7000) != 0x7000 {
            // fine Y < 7, just increment
            self.v += 0x1000;
        } else {
            // fine Y overflow
            self.v &= !0x7000; // fine Y = 0
            let mut coarse_y = (self.v & 0x03E0) >> 5;
            if coarse_y == 29 {
                coarse_y = 0;
                self.v ^= 0x0800; // toggle vertical nametable
            } else if coarse_y == 31 {
                coarse_y = 0; // wrap without NT toggle
            } else {
                coarse_y += 1;
            }
            self.v = (self.v & !0x03E0) | (coarse_y << 5);
        }
    }

    #[inline]
    fn render_pixel(&mut self, cartridge: Option<&crate::cartridge::Cartridge>) {
        let x = self.cycle - 1;
        let y = self.scanline;

        if x >= 256 || y < 0 || y >= 240 {
            return;
        }

        let mut bg_color = self.palette[0]; // Default background color
        let mut bg_pixel = 0u8; // Background pixel value (0 = transparent, 1-3 = palette entries)

        // Render background if enabled
        if self.mask.contains(PpuMask::BG_ENABLE) {
            // Left column clipping
            if !self.mask.contains(PpuMask::BG_LEFT_ENABLE) && x < 8 {
                // bg_color stays palette[0], bg_pixel stays 0
            } else if let Some(cart) = cartridge {
                // Extract scroll state from v register
                let fine_y = ((self.v >> 12) & 7) as u16;
                let coarse_y = ((self.v >> 5) & 0x1F) as usize;
                let logical_nt = ((self.v >> 10) & 3) as usize;
                let coarse_x = (self.v & 0x1F) as usize;

                // Calculate which tile column pixel to render using fine X scroll
                let pixel_col = (x & 7) as u8;
                let scrolled_col = pixel_col + self.x;
                let (tile_cx, tile_nt, tile_fx) = if scrolled_col >= 8 {
                    // Need the next tile
                    let next_cx = if coarse_x == 31 { 0 } else { coarse_x + 1 };
                    let next_nt = if coarse_x == 31 { logical_nt ^ 1 } else { logical_nt };
                    (next_cx, next_nt, scrolled_col - 8)
                } else {
                    (coarse_x, logical_nt, scrolled_col)
                };

                let physical_nt = self.resolve_nametable(tile_nt, cartridge);
                let nt_addr = coarse_y * 32 + tile_cx;
                let tile_id = if nt_addr < 1024 {
                    self.nametable[physical_nt][nt_addr]
                } else {
                    0
                };

                let pattern_table = if self.control.contains(PpuControl::BG_PATTERN) { 0x1000u16 } else { 0x0000u16 };
                let tile_addr = pattern_table + (tile_id as u16 * 16) + fine_y;

                if tile_addr < 0x2000 {
                    let low_byte = cart.read_chr(tile_addr);
                    let high_byte = cart.read_chr(tile_addr + 8);
                    let pixel_bit = 7 - tile_fx;
                    let low_bit = (low_byte >> pixel_bit) & 1;
                    let high_bit = (high_byte >> pixel_bit) & 1;
                    let pixel_value = (high_bit << 1) | low_bit;

                    bg_pixel = pixel_value;

                    if pixel_value != 0 {
                        // Attribute table lookup using scroll coordinates
                        let attr_x = tile_cx >> 2;
                        let attr_y = coarse_y >> 2;
                        let attr_offset = 960 + attr_y * 8 + attr_x;
                        let attr_byte = if attr_offset < 1024 {
                            self.nametable[physical_nt][attr_offset]
                        } else {
                            0
                        };

                        let block_x = (tile_cx & 3) >> 1;
                        let block_y = (coarse_y & 3) >> 1;
                        let shift = (block_y * 2 + block_x) * 2;
                        let palette_num = (attr_byte >> shift) & 0x03;

                        let palette_idx = (palette_num as usize * 4) + pixel_value as usize;
                        if palette_idx < 16 {
                            bg_color = self.palette[palette_idx];
                        }
                    }
                }
            }
        }

        // Check for sprite rendering
        let mut sprite_result = None;
        let mut sprite_0_hit = false;

        if self.mask.contains(PpuMask::SPRITE_ENABLE) {
            // Left column clipping for sprites
            if !self.mask.contains(PpuMask::SPRITE_LEFT_ENABLE) && x < 8 {
                // Skip sprite rendering in left 8 pixels
            } else {
                sprite_result = self.render_sprites(x as u8, y as u8, cartridge, &mut sprite_0_hit);

                // Set sprite 0 hit flag if needed
                if sprite_0_hit && bg_pixel != 0 {
                    self.status.insert(PpuStatus::SPRITE_0_HIT);
                }
            }
        }

        // Determine final pixel color
        let final_color = if let Some((sprite_color, priority_behind_bg)) = sprite_result {
            if priority_behind_bg && bg_pixel != 0 {
                bg_color
            } else {
                sprite_color
            }
        } else {
            bg_color
        };

        let pixel_index = ((y as usize * 256) + x as usize) * 3;

        let mut masked_color = final_color & 0x3F;
        if self.mask.contains(PpuMask::GRAYSCALE) {
            masked_color &= 0x30;
        }
        let color = PALETTE_COLORS[masked_color as usize];
        // Safety: x is 0..255 and y is 0..239 (guarded above), buffer is 256*240*3
        let dest = &mut self.buffer[pixel_index..pixel_index + 3];
        dest[0] = color.0;
        dest[1] = color.1;
        dest[2] = color.2;
    }

    fn evaluate_scanline_sprites(&mut self, _cartridge: Option<&crate::cartridge::Cartridge>) {
        self.scanline_sprite_count = 0;
        if self.scanline < 0 || self.scanline >= 240 {
            return;
        }

        let sprite_height: u16 = if self.control.contains(PpuControl::SPRITE_SIZE) { 16 } else { 8 };
        let current_scanline = self.scanline as u16;

        for sprite_num in 0u8..64 {
            let base = sprite_num as usize * 4;
            let sprite_y = self.oam[base];

            if sprite_y >= 0xEF { continue; }

            let sprite_top = sprite_y as u16 + 1;
            let sprite_bottom = sprite_top + sprite_height;

            if current_scanline >= sprite_top && current_scanline < sprite_bottom {
                let idx = self.scanline_sprite_count as usize;
                if idx >= 8 {
                    self.status.insert(PpuStatus::SPRITE_OVERFLOW);
                    break;
                }
                self.scanline_sprites[idx] = (
                    sprite_num,
                    sprite_y,
                    self.oam[base + 1],
                    self.oam[base + 2],
                    self.oam[base + 3],
                );
                self.scanline_sprite_count += 1;
            }
        }
    }

    #[inline]
    fn render_sprites(&self, x: u8, y: u8, cartridge: Option<&crate::cartridge::Cartridge>, sprite_0_hit: &mut bool) -> Option<(u8, bool)> {
        if let Some(cart) = cartridge {
            let sprite_size: u8 = if self.control.contains(PpuControl::SPRITE_SIZE) { 16 } else { 8 };
            let count = self.scanline_sprite_count as usize;

            for i in 0..count {
                let (sprite_num, sprite_y, tile_id, attributes, sprite_x) = self.scanline_sprites[i];

                // Check if pixel is within sprite horizontal bounds
                if x < sprite_x || (x as u16) >= sprite_x as u16 + 8 {
                    continue;
                }

                let sprite_top = sprite_y as u16 + 1;
                let mut pixel_x = x - sprite_x;
                let mut pixel_y = (y as u16 - sprite_top) as u8;

                // Handle horizontal flip
                if attributes & 0x40 != 0 {
                    pixel_x = 7 - pixel_x;
                }

                // Handle vertical flip
                if attributes & 0x80 != 0 {
                    pixel_y = (sprite_size - 1) - pixel_y;
                }

                // Calculate pattern table address
                let (pattern_table, actual_tile_id) = if sprite_size == 16 {
                    let pattern_table: u16 = if tile_id & 0x01 != 0 { 0x1000 } else { 0x0000 };
                    let actual_tile_id = tile_id & 0xFE;
                    (pattern_table, actual_tile_id)
                } else {
                    let pattern_table: u16 = if self.control.contains(PpuControl::SPRITE_PATTERN) { 0x1000 } else { 0x0000 };
                    (pattern_table, tile_id)
                };

                // For 8x16 sprites, select top or bottom half
                let final_tile_id = if sprite_size == 16 && pixel_y >= 8 {
                    actual_tile_id + 1
                } else {
                    actual_tile_id
                };

                let pattern_fine_y = (pixel_y & 7) as u16;
                let tile_addr = pattern_table + (final_tile_id as u16 * 16) + pattern_fine_y;

                // Read pattern data
                if tile_addr + 8 < 0x2000 {
                    let low_byte = cart.read_chr(tile_addr);
                    let high_byte = cart.read_chr(tile_addr + 8);
                    let pixel_bit = 7 - pixel_x;
                    let low_bit = (low_byte >> pixel_bit) & 1;
                    let high_bit = (high_byte >> pixel_bit) & 1;
                    let pixel_value = (high_bit << 1) | low_bit;

                    if pixel_value != 0 {
                        if sprite_num == 0 && x != 255 {
                            *sprite_0_hit = true;
                        }

                        let palette_num = attributes & 0x03;
                        let palette_idx = 16 + palette_num * 4 + pixel_value;

                        let color_index = if (palette_idx as usize) < 32 {
                            self.palette[palette_idx as usize]
                        } else {
                            self.palette[16]
                        };

                        let priority_behind_bg = (attributes & 0x20) != 0;
                        return Some((color_index, priority_behind_bg));
                    }
                }
            }
        }
        None
    }

    pub fn read_register(&mut self, addr: u16, cartridge: Option<&crate::cartridge::Cartridge>) -> u8 {
        match addr {
            0x2002 => {
                let status = self.status.bits();

                // Clear VBlank flag after read
                self.status.remove(PpuStatus::VBLANK);

                // Reset write toggle
                self.w = false;

                // NMI suppression: reading $2002 on the exact cycle VBlank is set
                if self.scanline == 241 && self.cycle == 1 {
                    self.nmi_suppressed = true;
                }

                status
            }
            0x2004 => self.oam[self.oam_addr as usize],
            0x2007 => {
                // Super Mario Bros title screen fix: Proper $2007 read implementation
                let data = if self.v >= 0x3F00 {
                    // Palette RAM: Immediate read (no buffering)
                    let palette_addr = (self.v & 0x1F) as usize;
                    // Proper NES palette mirroring for reads
                    let mirrored_addr = match palette_addr {
                        0x10 => 0x00, // $3F10 mirrors $3F00
                        0x14 => 0x04, // $3F14 mirrors $3F04
                        0x18 => 0x08, // $3F18 mirrors $3F08
                        0x1C => 0x0C, // $3F1C mirrors $3F0C
                        _ => palette_addr & 0x1F
                    };
                    // Also fill read_buffer with nametable data "underneath" the palette
                    let nt_addr = (self.v & 0x2FFF) as usize;
                    if nt_addr >= 0x2000 {
                        let offset_in_nt = nt_addr - 0x2000;
                        let logical_nt = (offset_in_nt >> 10) & 3;
                        let table = self.resolve_nametable(logical_nt, cartridge);
                        let offset = offset_in_nt & 0x3FF;
                        self.read_buffer = if offset < 1024 {
                            self.nametable[table][offset]
                        } else {
                            0
                        };
                    }
                    self.palette[mirrored_addr]
                } else {
                    // All other memory: Buffered read (crucial for SMB)
                    let old_buffer = self.read_buffer;
                    
                    // Update buffer with new data
                    let effective_v = if self.v >= 0x3000 && self.v < 0x3F00 {
                        self.v - 0x1000 // $3000-$3EFF mirrors $2000-$2EFF
                    } else {
                        self.v
                    };
                    if effective_v >= 0x2000 && effective_v < 0x3000 {
                        // Nametable read with proper mirroring
                        let addr = (effective_v - 0x2000) as usize;
                        let logical_nt = (addr >> 10) & 3;
                        let table = self.resolve_nametable(logical_nt, cartridge);
                        let offset = addr & 0x3FF;
                        self.read_buffer = if offset < 1024 {
                            self.nametable[table][offset]
                        } else {
                            0
                        };
                    } else if effective_v < 0x2000 {
                        // CHR-ROM/CHR-RAM read
                        if let Some(cart) = cartridge {
                            self.read_buffer = cart.read_chr(effective_v);
                        } else {
                            self.read_buffer = 0;
                        }
                    } else {
                        self.read_buffer = 0;
                    }
                    
                    old_buffer
                };
                
                // CRITICAL: Increment VRAM address AFTER read
                let increment = if self.control.contains(PpuControl::VRAM_INCREMENT) { 32 } else { 1 };
                self.v = self.v.wrapping_add(increment) & 0x3FFF;

                data
            }
            _ => 0
        }
    }

    pub fn write_register(&mut self, addr: u16, data: u8, cartridge: Option<&crate::cartridge::Cartridge>) -> Option<(u16, u8)> {
        match addr {
            0x2000 => {
                let old_nmi_enable = self.control.contains(PpuControl::NMI_ENABLE);
                self.control = PpuControl::from_bits_truncate(data);

                // Update nametable select bits in t register
                self.t = (self.t & 0xF3FF) | ((data as u16 & 0x03) << 10);

                // NMI edge detection: 0->1 while VBlank is set triggers immediate NMI
                let new_nmi_enable = self.control.contains(PpuControl::NMI_ENABLE);
                if !old_nmi_enable && new_nmi_enable && self.status.contains(PpuStatus::VBLANK) {
                    self.pending_nmi = true;
                }

            }
            0x2001 => {
                self.mask = PpuMask::from_bits_truncate(data);
                self.rendering_enabled = self.mask.contains(PpuMask::BG_ENABLE)
                    || self.mask.contains(PpuMask::SPRITE_ENABLE);
            }
            0x2003 => {
                self.oam_addr = data;
            }
            0x2004 => {
                self.oam[self.oam_addr as usize] = data;
                self.oam_addr = self.oam_addr.wrapping_add(1);
            }
            0x2005 => {
                if !self.w {
                    self.x = data & 0x07;
                    self.t = (self.t & 0xFFE0) | ((data as u16) >> 3);
                    self.w = true;
                } else {
                    self.t = (self.t & 0x8C1F) | (((data as u16) & 0x07) << 12) | (((data as u16) >> 3) << 5);
                    self.w = false;
                }
            }
            0x2006 => {
                if !self.w {
                    self.t = (self.t & 0x00FF) | (((data & 0x3F) as u16) << 8);
                    self.w = true;
                } else {
                    self.t = (self.t & 0xFF00) | data as u16;
                    self.v = self.t;
                    self.w = false;
                }
            }
            0x2007 => {
                let write_v = if self.v >= 0x3000 && self.v < 0x3F00 {
                    self.v - 0x1000
                } else {
                    self.v
                };
                if write_v >= 0x3F00 {
                    // Palette write
                    let palette_addr = (write_v & 0x1F) as usize;
                    let mirrored_addr = match palette_addr {
                        0x10 => 0x00,
                        0x14 => 0x04,
                        0x18 => 0x08,
                        0x1C => 0x0C,
                        _ => palette_addr & 0x1F
                    };
                    self.palette[mirrored_addr] = data;
                } else if write_v >= 0x2000 && write_v < 0x3000 {
                    // Nametable write
                    let addr = (write_v - 0x2000) as usize;
                    let nt_index = (addr >> 10) & 3;
                    let offset = addr & 0x3FF;

                    if offset < 1024 {
                        let physical_nt = if let Some(cart) = cartridge {
                            match cart.mirroring() {
                                crate::cartridge::Mirroring::Vertical => match nt_index {
                                    0 | 2 => 0,
                                    1 | 3 => 1,
                                    _ => 0,
                                },
                                crate::cartridge::Mirroring::Horizontal => match nt_index {
                                    0 | 1 => 0,
                                    2 | 3 => 1,
                                    _ => 0,
                                },
                                crate::cartridge::Mirroring::FourScreen => nt_index & 1,
                                crate::cartridge::Mirroring::OneScreenLower => 0,
                                crate::cartridge::Mirroring::OneScreenUpper => 1,
                            }
                        } else {
                            nt_index & 1
                        };

                        self.nametable[physical_nt][offset] = data;
                    }
                } else if write_v < 0x2000 {
                    // CHR write (for CHR RAM)
                    let chr_addr = write_v;
                    let increment = if self.control.contains(PpuControl::VRAM_INCREMENT) { 32 } else { 1 };
                    self.v = self.v.wrapping_add(increment) & 0x3FFF;
                    return Some((chr_addr, data));
                }

                let increment = if self.control.contains(PpuControl::VRAM_INCREMENT) { 32 } else { 1 };
                self.v = self.v.wrapping_add(increment) & 0x3FFF;
            }
            _ => {}
        }
        None
    }

    pub fn get_vram_write_addr(&self) -> u16 {
        self.v
    }

    pub fn get_buffer(&self) -> &[u8] {
        &self.buffer
    }
}

impl Ppu {
    // Save state getters
    pub fn get_control(&self) -> PpuControl {
        self.control
    }
    
    pub fn get_control_bits(&self) -> u8 {
        self.control.bits()
    }
    
    pub fn get_mask(&self) -> PpuMask {
        self.mask
    }
    
    pub fn get_mask_bits(&self) -> u8 {
        self.mask.bits()
    }
    
    pub fn get_status(&self) -> PpuStatus {
        self.status
    }
    
    pub fn get_status_bits(&self) -> u8 {
        self.status.bits()
    }
    
    pub fn get_vram_addr(&self) -> u16 {
        self.v
    }
    
    pub fn get_oam_addr(&self) -> u8 {
        self.oam_addr
    }
    
    pub fn get_palette(&self) -> [u8; 32] {
        self.palette
    }
    
    pub fn get_nametable(&self) -> [[u8; 1024]; 2] {
        self.nametable
    }
    
    pub fn get_oam(&self) -> [u8; 256] {
        self.oam
    }
    
    
    pub fn get_current_vram_address(&self) -> u16 {
        self.v
    }
    
    
    
    
    
    pub fn debug_get_control(&self) -> u8 {
        self.control.bits()
    }
    
    pub fn debug_get_mask(&self) -> u8 {
        self.mask.bits()
    }
    
    
    
    // Save state setters
    pub fn set_palette(&mut self, palette: [u8; 32]) {
        self.palette = palette;
    }
    
    pub fn set_nametable(&mut self, nametable: [[u8; 1024]; 2]) {
        self.nametable = nametable;
    }
    
    pub fn set_oam(&mut self, oam: [u8; 256]) {
        self.oam = oam;
    }

    pub fn write_oam_data(&mut self, addr: u8, data: u8) {
        self.oam[addr as usize] = data;
    }
    
    
    

    pub fn get_palette_value(&self, index: usize) -> u8 {
        if index < self.palette.len() {
            self.palette[index]
        } else {
            0
        }
    }
    
}