use bitflags::bitflags;

#[cfg(test)]
mod tests;

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
    sprite_0_hit_line: i16, // Track which scanline sprite 0 hit occurred
    scroll_change_line: i16, // Track scroll register changes for split-screen
    stable_split_line: i16, // Stable split point to prevent flickering
    frame_since_scroll_change: u16, // Frames since last mid-frame scroll change
    
    // PPU $2007 read buffer for CHR-ROM reads
    read_buffer: u8,
}

impl Ppu {
    pub fn new() -> Self {
        let mut ppu = Ppu {
            control: PpuControl::empty(),
            mask: PpuMask::empty(),
            status: PpuStatus::from_bits_truncate(0x80), // Start with VBLANK set
            oam_addr: 0,
            
            v: 0,
            t: 0,
            x: 0,
            w: false,
            
            cycle: 0,
            scanline: -1,
            frame: 0,
            
            nametable: [[0; 1024]; 2],
            palette: [0x0F; 32], // Initialize with black (0x0F)
            oam: [0xFF; 256],    // Initialize OAM with 0xFF (sprites off-screen)
            
            buffer: vec![0x40; 256 * 240 * 3], // Initialize with gray instead of black
            sprite_0_hit_line: -1,
            scroll_change_line: -1,
            stable_split_line: -1,
            frame_since_scroll_change: 0,
            read_buffer: 0,
        };
        
        // Set initial palette to grayscale
        ppu.palette[0] = 0x0F;  // Background color
        
        // Initialize buffer to black - let game rendering take over
        for pixel in ppu.buffer.chunks_mut(3) {
            pixel[0] = 0x0F; // Black
            pixel[1] = 0x0F;
            pixel[2] = 0x0F;
        }
        
        ppu
    }

    pub fn step(&mut self, cartridge: Option<&crate::cartridge::Cartridge>) -> bool {
        let mut nmi = false;

        // Handle scanline processing
        
        // Handle scanline 241

        match self.scanline {
            -1 => {
                // Pre-render scanline - clear VBlank flag at cycle 1
                if self.cycle == 1 {
                    self.status.remove(PpuStatus::VBLANK);
                    self.status.remove(PpuStatus::SPRITE_0_HIT);
                    // Reset split-screen tracking per frame
                    self.sprite_0_hit_line = -1;
                    
                    // Gradually decay split-screen detection to prevent stuck splits
                    if self.frame_since_scroll_change > 0 {
                        self.frame_since_scroll_change -= 1;
                        if self.frame_since_scroll_change == 0 {
                            self.scroll_change_line = -1;
                            self.stable_split_line = -1;
                        }
                    }
                }
                
                // Update vertical scroll during pre-render scanline
                if self.cycle >= 280 && self.cycle <= 304 {
                    if self.mask.contains(PpuMask::BG_ENABLE) || self.mask.contains(PpuMask::SPRITE_ENABLE) {
                        // Copy vertical scroll bits from t to v
                        self.v = (self.v & 0x841F) | (self.t & 0x7BE0);
                    }
                }
            }
            0..=239 => {
                // Visible scanlines
                
                // Update horizontal scroll at end of visible pixels
                if self.cycle == 257 && (self.mask.contains(PpuMask::BG_ENABLE) || self.mask.contains(PpuMask::SPRITE_ENABLE)) {
                    // Copy horizontal scroll bits from t to v
                    self.v = (self.v & 0xFBE0) | (self.t & 0x041F);
                }
                
                if self.cycle >= 1 && self.cycle <= 256 {
                    self.render_pixel(cartridge);
                }
            }
            240 => {
                // Post-render scanline
                if self.cycle == 1 {
                    self.evaluate_sprites();
                }
            }
            241 => {
                // Set VBlank flag for entire scanline 241 for better compatibility
                if self.cycle == 1 {
                    self.status.insert(PpuStatus::VBLANK);
                    if self.control.contains(PpuControl::NMI_ENABLE) {
                        nmi = true;
                    }
                }
            }
            242..=260 => {
                // Keep VBlank flag set during VBlank period
                // VBlank period runs from scanline 241 to 260
            }
            _ => {}
        }

        self.cycle += 1;
        if self.cycle >= 341 {
            self.cycle = 0;
            self.scanline += 1;
            
            // Handle frame completion 
            if self.scanline >= 261 {
                self.scanline = -1;
                self.frame += 1;
                
                // Don't reset scroll_change_line here - let the frame counter handle it
            }
        }

        nmi
    }

    fn render_pixel(&mut self, cartridge: Option<&crate::cartridge::Cartridge>) {
        let x = self.cycle - 1;
        let y = self.scanline;
        
        if x >= 256 || y < 0 || y >= 240 {
            return;
        }
        
        // Simple background rendering with scrolling
        let mut bg_color = self.palette[0];
        let mut bg_pixel = 0;
        
        
        if self.mask.contains(PpuMask::BG_ENABLE) && cartridge.is_some() {
            // Universal sprite 0 split-screen detection (works for all games)
            let sprite_0_y = self.oam[0] as i16;
            let apply_scroll = if sprite_0_y >= 15 && sprite_0_y <= 50 {
                // Sprite 0 detected in status area range - use split-screen
                let split_line = sprite_0_y + 8;
                y > split_line
            } else {
                // No sprite 0 in status area - normal scrolling
                true
            };
            
            let (pixel_x, pixel_y) = if apply_scroll {
                // Extract scroll values from PPU registers
                let coarse_x = self.v & 0x1F;
                let coarse_y = (self.v >> 5) & 0x1F; 
                let fine_x = self.x as u16;
                let fine_y = (self.v >> 12) & 0x07;
                let nt_x = (self.v >> 10) & 0x01;
                let nt_y = (self.v >> 11) & 0x01;
                
                // Calculate scroll offset
                let scroll_x = (nt_x * 256) + (coarse_x * 8) + fine_x;
                let scroll_y = (nt_y * 240) + (coarse_y * 8) + fine_y;
                
                // Current pixel position with scroll applied
                ((x as u16 + scroll_x) % 512, (y as u16 + scroll_y) % 480)
            } else {
                // No scrolling for status area
                (x as u16, y as u16)
            };
            
            // Determine which physical nametable to use
            let physical_nt_x = (pixel_x / 256) as u16;
            let physical_nt_y = (pixel_y / 240) as u16;
            
            // For static screens (like title screens), use PPU CONTROL nametable selection
            let nt_select = if apply_scroll {
                // During scrolling, use mirroring-based selection
                if let Some(cart) = cartridge {
                    match cart.mirroring() {
                        crate::cartridge::Mirroring::Horizontal => {
                            // Horizontal mirroring: top mirrors to bottom
                            physical_nt_y % 2
                        },
                        crate::cartridge::Mirroring::Vertical => {
                            // Vertical mirroring: left mirrors to right
                            physical_nt_x % 2
                        },
                        crate::cartridge::Mirroring::FourScreen => {
                            // Four screen: direct mapping (limited to 2 nametables)
                            (physical_nt_y * 2 + physical_nt_x) % 2
                        }
                    }
                } else {
                    0
                }
            } else {
                // For status area: force nametable 0 for stability
                if sprite_0_y >= 15 && sprite_0_y <= 50 && y < sprite_0_y + 8 {
                    0u16  // Status area: force nametable 0
                } else {
                    (self.control.bits() & 0x03) as u16
                }
            } as usize % 2;
            
            // Local coordinates within the nametable
            let local_x = pixel_x % 256;
            let local_y = pixel_y % 240;
            let local_tile_x = (local_x / 8) as usize;
            let local_tile_y = (local_y / 8) as usize;
            
            if local_tile_x < 32 && local_tile_y < 30 && nt_select < 2 {
                let nametable_addr = local_tile_y * 32 + local_tile_x;
                let tile_id = self.nametable[nt_select][nametable_addr];
                
                
                
                
                if let Some(cart) = cartridge {
                    let pattern_table = if self.control.contains(PpuControl::BG_PATTERN) { 0x1000 } else { 0x0000 };
                    let pattern_fine_y = (local_y % 8) as u16;
                    let tile_addr = pattern_table + (tile_id as u16 * 16) + pattern_fine_y;
                    
                    // Ensure tile_addr is within valid range for CHR ROM
                    if tile_addr < 0x2000 {
                        let pattern_low = cart.read_chr(tile_addr);
                        let pattern_high = cart.read_chr(tile_addr + 8);
                        
                        let pattern_fine_x = (local_x % 8) as u8;
                        let pixel_bit = 7 - pattern_fine_x;
                        let pixel_value = ((pattern_high >> pixel_bit) & 1) << 1 | ((pattern_low >> pixel_bit) & 1);
                        
                        // Skip background rendering on split line to avoid black line (universal fix)
                        let skip_bg = sprite_0_y >= 15 && sprite_0_y <= 50 && y == sprite_0_y + 8;
                        
                        if !skip_bg {
                            bg_pixel = pixel_value;
                        }
                        
                        if pixel_value != 0 && !skip_bg {
                            // Get attribute byte for palette selection
                            let attr_x = local_tile_x / 4;
                            let attr_y = local_tile_y / 4;
                            let attr_offset = 0x3C0 + (attr_y * 8 + attr_x);
                            let attr_byte = if attr_offset < 1024 {
                                self.nametable[nt_select][attr_offset]
                            } else {
                                0
                            };
                            
                            // Determine which 2x2 block within the 4x4 area
                            let block_x = (local_tile_x % 4) / 2;
                            let block_y = (local_tile_y % 4) / 2;
                            let shift = (block_y * 2 + block_x) * 2;
                            let palette_num = (attr_byte >> shift) & 0x03;
                            
                            // Background palette index: palette_num * 4 + pixel_value
                            let palette_idx = (palette_num as usize * 4) + pixel_value as usize;
                            if palette_idx < 16 {
                                bg_color = self.palette[palette_idx];
                                
                            }
                        }
                    }
                }
            }
        }
        
        // Check for sprite rendering
        let mut sprite_result = None;
        let mut sprite_0_hit = false;
        
        if self.mask.contains(PpuMask::SPRITE_ENABLE) {
            sprite_result = self.render_sprites(x as u8, y as u8, cartridge, &mut sprite_0_hit);
            
            // Set sprite 0 hit flag if needed
            if sprite_0_hit && bg_pixel != 0 {
                self.status.insert(PpuStatus::SPRITE_0_HIT);
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
        
        let color = get_nes_color(final_color);
        let pixel_index = ((y as usize * 256) + x as usize) * 3;
        
        
        if pixel_index + 2 < self.buffer.len() {
            self.buffer[pixel_index] = color.0;
            self.buffer[pixel_index + 1] = color.1;
            self.buffer[pixel_index + 2] = color.2;
        }
    }

    fn evaluate_sprites(&mut self) {
        // Clear overflow flag at start of frame
        self.status.remove(PpuStatus::SPRITE_OVERFLOW);
    }

    fn render_sprites(&self, x: u8, y: u8, cartridge: Option<&crate::cartridge::Cartridge>, sprite_0_hit: &mut bool) -> Option<(u8, bool)> {
        if let Some(cart) = cartridge {
            // Check all 64 sprites but limit to 8 per scanline (hardware limit)
            let mut sprites_on_scanline = 0;
            for sprite_num in 0..64 {
                let base = sprite_num * 4;
                if base + 3 >= 256 { break; }
                
                let sprite_y = self.oam[base];
                let tile_id = self.oam[base + 1];
                let attributes = self.oam[base + 2];
                let sprite_x = self.oam[base + 3];
                
                // Skip off-screen sprites
                if sprite_y >= 0xEF { continue; }
                
                let sprite_size = if self.control.contains(PpuControl::SPRITE_SIZE) { 16 } else { 8 };
                
                // Check if sprite is on current scanline
                if y >= sprite_y && y < sprite_y + sprite_size {
                    sprites_on_scanline += 1;
                    
                    // NES hardware limit: max 8 sprites per scanline
                    if sprites_on_scanline > 8 {
                        break;
                    }
                    
                    // Check if pixel is within sprite horizontal bounds
                    if x >= sprite_x && x < sprite_x + 8 {
                    
                    let mut pixel_x = x - sprite_x;
                    let mut pixel_y = y - sprite_y;
                    
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
                        // 8x16 sprites: bit 0 of tile_id selects pattern table
                        let pattern_table = if tile_id & 0x01 != 0 { 0x1000 } else { 0x0000 };
                        let actual_tile_id = tile_id & 0xFE; // Use even tile number
                        (pattern_table, actual_tile_id)
                    } else {
                        // 8x8 sprites: use control register
                        let pattern_table = if self.control.contains(PpuControl::SPRITE_PATTERN) { 0x1000 } else { 0x0000 };
                        (pattern_table, tile_id)
                    };
                    
                    // For 8x16 sprites, select top or bottom half
                    let final_tile_id = if sprite_size == 16 && pixel_y >= 8 {
                        actual_tile_id + 1
                    } else {
                        actual_tile_id
                    };
                    
                    let pattern_fine_y = (pixel_y % 8) as u16;
                    let tile_addr = pattern_table + (final_tile_id as u16 * 16) + pattern_fine_y;
                    
                    // Read pattern data
                    if tile_addr + 8 < 0x2000 {
                        // Use cartridge's sprite-specific CHR read
                        let pattern_low = cart.read_chr_sprite(tile_addr, sprite_y);
                        let pattern_high = cart.read_chr_sprite(tile_addr + 8, sprite_y);
                        
                        let pixel_bit = 7 - pixel_x;
                        let pixel_value = ((pattern_high >> pixel_bit) & 1) << 1 | ((pattern_low >> pixel_bit) & 1);
                        
                        if pixel_value != 0 {
                            if sprite_num == 0 {
                                *sprite_0_hit = true;
                                
                                // Normal sprite 0 rendering
                                let palette_num = attributes & 0x03;
                                let palette_idx = 16 + palette_num * 4 + pixel_value;
                                
                                let color_index = if (palette_idx as usize) < 32 {
                                    self.palette[palette_idx as usize]
                                } else {
                                    self.palette[16]
                                };
                                
                                let priority_behind_bg = (attributes & 0x20) != 0;
                                return Some((color_index, priority_behind_bg));
                            } else {
                                // Normal sprite rendering
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
                }
            }
        }
        None
    }

    pub fn read_register(&mut self, addr: u16, cartridge: Option<&crate::cartridge::Cartridge>) -> u8 {
        match addr {
            0x2002 => {
                
                
                let status = self.status.bits();
                self.w = false;
                
                // Clear VBlank after read
                self.status.remove(PpuStatus::VBLANK);
                
                status
            }
            0x2004 => self.oam[self.oam_addr as usize],
            0x2007 => {
                // Super Mario Bros title screen fix: Proper $2007 read implementation
                let data = if self.v >= 0x3F00 {
                    // Palette RAM: Immediate read (no buffering)
                    let palette_addr = (self.v & 0x1F) as usize;
                    let mirrored_addr = if palette_addr >= 16 && palette_addr % 4 == 0 {
                        palette_addr - 16
                    } else {
                        palette_addr
                    };
                    self.palette[mirrored_addr]
                } else {
                    // All other memory: Buffered read (crucial for SMB)
                    let old_buffer = self.read_buffer;
                    
                    // Update buffer with new data
                    if self.v >= 0x2000 && self.v < 0x3000 {
                        // Nametable read
                        let addr = (self.v - 0x2000) as usize;
                        let table = (addr / 0x400) % 2;
                        let offset = addr % 0x400;
                        self.read_buffer = if offset < 1024 {
                            self.nametable[table][offset]
                        } else {
                            0
                        };
                    } else if self.v < 0x2000 {
                        // CHR-ROM read (CRITICAL for Super Mario Bros title screen!)
                        // This is what was missing - SMB reads title data from CHR-ROM
                        if let Some(cart) = cartridge {
                            self.read_buffer = cart.read_chr(self.v);
                            
                            // CHR-ROM read successful
                        } else {
                            self.read_buffer = 0;
                        }
                    } else {
                        self.read_buffer = 0;
                    }
                    
                    old_buffer
                };
                
                // CRITICAL: Increment VRAM address AFTER read (this was missing!)
                let increment = if self.control.contains(PpuControl::VRAM_INCREMENT) { 32 } else { 1 };
                self.v = self.v.wrapping_add(increment);
                
                data
            }
            _ => 0
        }
    }

    pub fn write_register(&mut self, addr: u16, data: u8, cartridge: Option<&crate::cartridge::Cartridge>) -> Option<(u16, u8)> {
        match addr {
            0x2000 => {
                // PPU CONTROL register handling
                
                self.control = PpuControl::from_bits_truncate(data);
                self.t = (self.t & 0xF3FF) | ((data as u16 & 0x03) << 10);
            }
            0x2001 => {
                // PPU MASK register handling
                self.mask = PpuMask::from_bits_truncate(data);
            }
            0x2003 => {
                self.oam_addr = data;
            }
            0x2004 => {
                self.oam[self.oam_addr as usize] = data;
                self.oam_addr = self.oam_addr.wrapping_add(1);
            }
            0x2005 => {
                // Mid-frame scroll detection for split-screen effects
                // Only detect during specific scanline ranges to avoid false positives
                if self.scanline >= 10 && self.scanline <= 50 && self.scroll_change_line == -1 {
                    self.scroll_change_line = self.scanline;
                    // Set frame counter to maintain split detection for multiple frames
                    self.frame_since_scroll_change = 120; // Keep split active for 2 seconds
                }
                
                if !self.w {
                    self.t = (self.t & 0xFFE0) | (data as u16 >> 3);
                    self.x = data & 0x07;
                    self.w = true;
                } else {
                    self.t = (self.t & 0x8FFF) | ((data as u16 & 0x07) << 12);
                    self.t = (self.t & 0xFC1F) | ((data as u16 & 0xF8) << 2);
                    self.w = false;
                }
            }
            0x2006 => {
                if !self.w {
                    self.t = (self.t & 0x80FF) | ((data as u16 & 0x3F) << 8);
                    self.w = true;
                } else {
                    self.t = (self.t & 0xFF00) | data as u16;
                    self.v = self.t;
                    self.w = false;
                    
                }
            }
            0x2007 => {
                if self.v >= 0x3F00 {
                    let palette_addr = (self.v & 0x1F) as usize;
                    let mirrored_addr = if palette_addr >= 16 && palette_addr % 4 == 0 {
                        palette_addr - 16
                    } else {
                        palette_addr
                    };
                    self.palette[mirrored_addr] = data;
                } else if self.v >= 0x2000 && self.v < 0x3000 {
                    let addr = (self.v - 0x2000) as usize;
                    // Proper nametable mirroring
                    let nt_index = (addr / 0x400) % 4; // 0-3 for NT0-NT3
                    let offset = addr % 0x400;
                    
                    if offset < 1024 {
                        // Map logical nametables to physical based on cartridge mirroring
                        let physical_nt = if let Some(cart) = cartridge {
                            match cart.mirroring() {
                                crate::cartridge::Mirroring::Vertical => match nt_index {
                                    0 => 0, // NT0 -> Physical NT0
                                    1 => 1, // NT1 -> Physical NT1
                                    2 => 0, // NT2 -> Physical NT0 (mirrors NT0)
                                    3 => 1, // NT3 -> Physical NT1 (mirrors NT1)
                                    _ => 0,
                                },
                                crate::cartridge::Mirroring::Horizontal => match nt_index {
                                    0 => 0, // NT0 -> Physical NT0
                                    1 => 0, // NT1 -> Physical NT0 (mirrors NT0)
                                    2 => 1, // NT2 -> Physical NT1
                                    3 => 1, // NT3 -> Physical NT1 (mirrors NT2)
                                    _ => 0,
                                },
                                crate::cartridge::Mirroring::FourScreen => nt_index % 2,
                            }
                        } else {
                            nt_index % 2
                        };
                        
                        // Write to nametable
                        
                        self.nametable[physical_nt][offset] = data;
                    }
                } else if self.v < 0x2000 {
                    // CHR writes to cartridge (for CHR RAM)
                    // Return CHR write info for bus to handle
                    let chr_addr = self.v;
                    let increment = if self.control.contains(PpuControl::VRAM_INCREMENT) { 32 } else { 1 };
                    self.v = self.v.wrapping_add(increment);
                    return Some((chr_addr, data));
                }
                
                let increment = if self.control.contains(PpuControl::VRAM_INCREMENT) { 32 } else { 1 };
                self.v = self.v.wrapping_add(increment);
            }
            _ => {}
        }
        None
    }

    pub fn get_buffer(&self) -> &[u8] {
        &self.buffer
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

impl Ppu {
    // Save state getters
    pub fn get_control(&self) -> u8 {
        self.control.bits()
    }
    
    pub fn get_mask(&self) -> u8 {
        self.mask.bits()
    }
    
    pub fn get_status(&self) -> u8 {
        self.status.bits()
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
}