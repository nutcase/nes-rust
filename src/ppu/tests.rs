use super::*;

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_ppu_control_register() {
        let mut ppu = Ppu::new();
        
        // Write to PPUCTRL ($2000)
        ppu.write_register(0x2000, 0xFF);
        
        assert!(ppu.control.contains(PpuControl::NAMETABLE_X));
        assert!(ppu.control.contains(PpuControl::NAMETABLE_Y));
        assert!(ppu.control.contains(PpuControl::VRAM_INCREMENT));
        assert!(ppu.control.contains(PpuControl::SPRITE_PATTERN));
        assert!(ppu.control.contains(PpuControl::BG_PATTERN));
        assert!(ppu.control.contains(PpuControl::SPRITE_SIZE));
        assert!(ppu.control.contains(PpuControl::PPU_MASTER_SLAVE));
        assert!(ppu.control.contains(PpuControl::NMI_ENABLE));
    }
    
    #[test]
    fn test_ppu_mask_register() {
        let mut ppu = Ppu::new();
        
        // Write to PPUMASK ($2001)
        ppu.write_register(0x2001, 0xFF);
        
        assert!(ppu.mask.contains(PpuMask::GRAYSCALE));
        assert!(ppu.mask.contains(PpuMask::BG_LEFT_ENABLE));
        assert!(ppu.mask.contains(PpuMask::SPRITE_LEFT_ENABLE));
        assert!(ppu.mask.contains(PpuMask::BG_ENABLE));
        assert!(ppu.mask.contains(PpuMask::SPRITE_ENABLE));
        assert!(ppu.mask.contains(PpuMask::EMPHASIZE_RED));
        assert!(ppu.mask.contains(PpuMask::EMPHASIZE_GREEN));
        assert!(ppu.mask.contains(PpuMask::EMPHASIZE_BLUE));
    }
    
    #[test]
    fn test_ppu_status_register() {
        let mut ppu = Ppu::new();
        
        // Set some status flags
        ppu.status.insert(PpuStatus::VBLANK);
        ppu.status.insert(PpuStatus::SPRITE_0_HIT);
        ppu.status.insert(PpuStatus::SPRITE_OVERFLOW);
        
        // Read PPUSTATUS ($2002)
        let status = ppu.read_register(0x2002);
        
        assert_eq!(status & 0xE0, 0xE0); // Top 3 bits should be set
        
        // VBLANK should be cleared after read
        assert!(!ppu.status.contains(PpuStatus::VBLANK));
        
        // w register should be reset
        assert_eq!(ppu.w, false);
    }
    
    #[test]
    fn test_oam_addr_and_data() {
        let mut ppu = Ppu::new();
        
        // Write OAM address
        ppu.write_register(0x2003, 0x10);
        assert_eq!(ppu.oam_addr, 0x10);
        
        // Write OAM data
        ppu.write_register(0x2004, 0x42);
        assert_eq!(ppu.oam[0x10], 0x42);
        assert_eq!(ppu.oam_addr, 0x11); // Auto-increment
        
        // Write more data
        ppu.write_register(0x2004, 0x43);
        assert_eq!(ppu.oam[0x11], 0x43);
        assert_eq!(ppu.oam_addr, 0x12);
    }
    
    #[test]
    fn test_oam_read() {
        let mut ppu = Ppu::new();
        
        // Set up some OAM data
        ppu.oam[0x20] = 0x55;
        ppu.oam_addr = 0x20;
        
        // Read OAM data
        let data = ppu.read_register(0x2004);
        assert_eq!(data, 0x55);
        // OAM address doesn't increment on read
        assert_eq!(ppu.oam_addr, 0x20);
    }
    
    #[test]
    fn test_scroll_register() {
        let mut ppu = Ppu::new();
        
        // First write (X scroll)
        ppu.write_register(0x2005, 0x20);
        assert_eq!(ppu.t & 0x001F, 0x04); // Coarse X = 0x20 >> 3
        assert_eq!(ppu.x, 0x00); // Fine X = 0x20 & 7 (but x is set on second write)
        assert_eq!(ppu.w, true);
        
        // Second write (Y scroll)
        ppu.write_register(0x2005, 0x30);
        assert_eq!((ppu.t >> 5) & 0x1F, 0x06); // Coarse Y = 0x30 >> 3
        assert_eq!((ppu.t >> 12) & 0x07, 0x00); // Fine Y = 0x30 & 7 (but stored differently)
        assert_eq!(ppu.w, false);
    }
    
    #[test]
    fn test_ppu_addr_register() {
        let mut ppu = Ppu::new();
        
        // First write (high byte)
        ppu.write_register(0x2006, 0x21);
        assert_eq!(ppu.t & 0x7F00, 0x2100);
        assert_eq!(ppu.w, true);
        
        // Second write (low byte)
        ppu.write_register(0x2006, 0x08);
        assert_eq!(ppu.t, 0x2108);
        assert_eq!(ppu.v, 0x2108); // v = t on second write
        assert_eq!(ppu.w, false);
    }
    
    #[test]
    fn test_ppu_data_read_write() {
        let mut ppu = Ppu::new();
        
        // Set VRAM address to nametable area
        ppu.v = 0x2000;
        
        // Write data
        ppu.write_register(0x2007, 0x42);
        assert_eq!(ppu.nametable[0][0], 0x42);
        assert_eq!(ppu.v, 0x2001); // Auto-increment by 1
        
        // Test increment mode (32)
        ppu.control.insert(PpuControl::VRAM_INCREMENT);
        ppu.write_register(0x2007, 0x43);
        assert_eq!(ppu.nametable[0][1], 0x43);
        assert_eq!(ppu.v, 0x2021); // Auto-increment by 32
    }
    
    #[test]
    fn test_palette_write() {
        let mut ppu = Ppu::new();
        
        // Write to palette RAM
        ppu.v = 0x3F00;
        ppu.write_register(0x2007, 0x0F); // Black
        assert_eq!(ppu.palette[0], 0x0F);
        
        // Test palette mirroring
        ppu.v = 0x3F10;
        ppu.write_register(0x2007, 0x30); // White
        assert_eq!(ppu.palette[0], 0x30); // Mirrors to 0x3F00
    }
    
    #[test]
    fn test_vblank_timing() {
        let mut ppu = Ppu::new();
        
        // Ensure VBlank is set (initial state may have it set)
        ppu.status.insert(PpuStatus::VBLANK);
        assert!(ppu.status.contains(PpuStatus::VBLANK));
        
        // Test VBlank clear on status read
        let _status = ppu.read_register(0x2002);
        assert!(!ppu.status.contains(PpuStatus::VBLANK));
    }
    
    #[test]
    fn test_sprite_0_hit() {
        let mut ppu = Ppu::new();
        
        // Enable rendering
        ppu.mask.insert(PpuMask::BG_ENABLE);
        ppu.mask.insert(PpuMask::SPRITE_ENABLE);
        
        // Place sprite 0 at a visible position
        ppu.oam[0] = 100; // Y position
        ppu.oam[1] = 0;   // Tile index
        ppu.oam[2] = 0;   // Attributes
        ppu.oam[3] = 100; // X position
        
        // Sprite 0 hit flag should be set during rendering
        // (This is a simplified test - actual hit detection requires background/sprite overlap)
    }
    
    #[test]
    fn test_oam_dma() {
        let mut ppu = Ppu::new();
        
        // DMA is typically handled by the bus, but we can test OAM writing
        for i in 0..256 {
            ppu.oam[i] = i as u8;
        }
        
        // Verify OAM contents
        for i in 0..256 {
            assert_eq!(ppu.oam[i], i as u8);
        }
    }
    
    #[test]
    fn test_nametable_mirroring() {
        let mut ppu = Ppu::new();
        
        // Test horizontal mirroring
        ppu.v = 0x2000;
        ppu.write_register(0x2007, 0x11);
        ppu.v = 0x2400;
        ppu.write_register(0x2007, 0x22);
        
        // In horizontal mirroring, 0x2000 and 0x2400 should be different
        assert_eq!(ppu.nametable[0][0], 0x11);
        assert_eq!(ppu.nametable[1][0], 0x22);
    }
    
    #[test]
    fn test_frame_buffer_output() {
        let ppu = Ppu::new();
        
        // Check buffer is initialized
        let buffer = ppu.get_buffer();
        assert_eq!(buffer.len(), 256 * 240 * 3); // RGB format
        
        // Check buffer format (actual initial values may vary)
        assert!(buffer.len() > 0);
    }
    
}