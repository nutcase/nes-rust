/// PPU memory management
#[derive(Debug, Clone)]
pub struct PpuMemory {
    pub nametables: [[u8; 1024]; 2],
    pub palettes: [u8; 32],
    pub oam: [u8; 256],
}

impl PpuMemory {
    pub fn new() -> Self {
        Self {
            nametables: [[0; 1024]; 2],
            palettes: [0x0F; 32], // Initialize with black
            oam: [0xFF; 256],     // Initialize OAM with 0xFF (sprites off-screen)
        }
    }
    
    pub fn read_nametable(&self, addr: u16, mirroring: crate::cartridge::Mirroring) -> u8 {
        let addr = addr & 0x0FFF; // Mask to nametable space
        let table_select = self.get_nametable_index(addr, mirroring);
        let offset = (addr % 0x400) as usize;
        
        if offset < 1024 {
            self.nametables[table_select][offset]
        } else {
            0
        }
    }
    
    pub fn write_nametable(&mut self, addr: u16, data: u8, mirroring: crate::cartridge::Mirroring) {
        let addr = addr & 0x0FFF;
        let table_select = self.get_nametable_index(addr, mirroring);
        let offset = (addr % 0x400) as usize;
        
        if offset < 1024 {
            self.nametables[table_select][offset] = data;
        }
    }
    
    pub fn read_palette(&self, addr: u16) -> u8 {
        let addr = (addr & 0x1F) as usize;
        // Handle mirroring: $3F10/$3F14/$3F18/$3F1C mirror $3F00/$3F04/$3F08/$3F0C
        let mirrored_addr = if addr >= 16 && addr % 4 == 0 {
            addr - 16
        } else {
            addr
        };
        self.palettes[mirrored_addr]
    }
    
    pub fn write_palette(&mut self, addr: u16, data: u8) {
        let addr = (addr & 0x1F) as usize;
        let mirrored_addr = if addr >= 16 && addr % 4 == 0 {
            addr - 16
        } else {
            addr
        };
        self.palettes[mirrored_addr] = data;
    }
    
    fn get_nametable_index(&self, addr: u16, mirroring: crate::cartridge::Mirroring) -> usize {
        let table = (addr / 0x400) % 4;
        match mirroring {
            crate::cartridge::Mirroring::Horizontal => {
                // $2000=$2800, $2400=$2C00
                ((table / 2) % 2) as usize
            },
            crate::cartridge::Mirroring::Vertical => {
                // $2000=$2400, $2800=$2C00
                (table % 2) as usize
            },
            crate::cartridge::Mirroring::FourScreen => {
                // All four nametables are separate (but we only have 2)
                (table % 2) as usize
            }
        }
    }
}