use std::fs::File;
use std::io::{Read, Result};

pub struct Cartridge {
    prg_rom: Vec<u8>,
    chr_rom: Vec<u8>,
    mapper: u8,
    mirroring: Mirroring,
    // Mapper 87 specific
    chr_bank: u8,
    // Game-specific flags
    is_goonies: bool,
}

#[derive(Debug, Clone, Copy)]
pub enum Mirroring {
    Horizontal,
    Vertical,
    FourScreen,
}

impl Cartridge {
    pub fn load(path: &str) -> Result<Self> {
        let mut file = File::open(path)?;
        let mut data = Vec::new();
        file.read_to_end(&mut data)?;

        if &data[0..4] != b"NES\x1a" {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Invalid NES file format",
            ));
        }

        let prg_rom_size = data[4] as usize * 16384;
        let chr_rom_size = data[5] as usize * 8192;
        let flags6 = data[6];
        let flags7 = data[7];

        let mirroring = if flags6 & 0x08 != 0 {
            Mirroring::FourScreen
        } else if flags6 & 0x01 != 0 {
            Mirroring::Vertical
        } else {
            Mirroring::Horizontal
        };

        let mapper = (flags7 & 0xF0) | (flags6 >> 4);

        let prg_rom_start = 16;
        let chr_rom_start = prg_rom_start + prg_rom_size;

        let prg_rom = data[prg_rom_start..prg_rom_start + prg_rom_size].to_vec();
        let mut chr_rom = if chr_rom_size > 0 {
            data[chr_rom_start..chr_rom_start + chr_rom_size].to_vec()
        } else {
            // CHR RAM - initialize with zeros
            vec![0; 8192]
        };
        
        // Normal CHR ROM handling - no special processing for now

        let chr_banks = if chr_rom.len() > 0 { chr_rom.len() / 0x2000 } else { 0 };
        println!("Cartridge loaded - Mapper: {}, PRG ROM: {} bytes, CHR ROM: {} bytes ({} banks)", 
                 mapper, prg_rom.len(), chr_rom.len(), chr_banks);
        println!("  iNES header: CHR size field = {} ({}*8KB = {}KB)", 
                 chr_rom_size / 8192, chr_rom_size / 8192, chr_rom_size / 1024);
        println!("  Mirroring: {:?}", mirroring);
        
        
        // Detect Goonies by ROM size and mapper
        let is_goonies = (mapper == 3 || mapper == 87) && 
                         prg_rom.len() == 32768 && 
                         chr_rom.len() == 16384;
        
        
        Ok(Cartridge {
            prg_rom,
            chr_rom,
            mapper,
            mirroring,
            chr_bank: 0,
            is_goonies,
        })
    }

    pub fn read_prg(&self, addr: u16) -> u8 {
        // Normal reset vector handling
        
        // Convert CPU address (0x8000-0xFFFF) to PRG ROM offset
        let rom_addr = addr - 0x8000;
        
        match self.mapper {
            0 | 3 | 87 => {
                // NROM, CNROM, and Mapper 87 - simple direct mapping
                let len = self.prg_rom.len();
                if len == 16384 {
                    // 16KB PRG: Mirror at 0xC000
                    self.prg_rom[(rom_addr & 0x3FFF) as usize]
                } else {
                    // 32KB PRG: Direct mapping
                    self.prg_rom[(rom_addr & 0x7FFF) as usize]
                }
            },
            _ => {
                // Unsupported mapper
                0
            }
        }
    }

    pub fn write_prg(&mut self, addr: u16, data: u8) {
        match self.mapper {
            0 => {
                // NROM has no bank switching
            },
            3 => {
                // Mapper 3 (CNROM) - CHR bank switching
                // Register at $8000-$FFFF
                // Unlike Mapper 87, CNROM has bus conflicts
                if addr >= 0x8000 {
                    // CNROM uses bits 0-1 directly for CHR bank selection
                    self.chr_bank = data & 0x03; // Select one of 4 possible 8KB CHR banks
                }
            },
            87 => {
                // Mapper 87 - CHR bank switching
                // Register at $6000-$7FFF
                if addr >= 0x6000 && addr <= 0x7FFF {
                    // Swap bits 0 and 1 as per nen-emulator implementation
                    self.chr_bank = ((data & 0x01) << 1) | ((data & 0x02) >> 1);
                }
            },
            _ => {} // Unsupported mapper
        }
    }

    pub fn read_chr(&self, addr: u16) -> u8 {
        match self.mapper {
            0 => {
                // NROM mapper - 8KB CHR ROM handling
                let chr_addr = if self.chr_rom.len() == 0x2000 {
                    // 8KB CHR ROM: Direct mapping, no mirroring
                    // Pattern table 0 (0x0000-0x0FFF): CHR[0x0000-0x0FFF]
                    // Pattern table 1 (0x1000-0x1FFF): CHR[0x1000-0x1FFF]
                    (addr & 0x1FFF) as usize
                } else {
                    // Larger CHR ROM: Normal access to both pattern tables
                    (addr & 0x1FFF) as usize
                };
                
                
                if chr_addr < self.chr_rom.len() {
                    self.chr_rom[chr_addr]
                } else {
                    0
                }
            },
            3 | 87 => {
                // Mapper 3 (CNROM) and Mapper 87 - 8KB CHR bank switching
                let bank_addr = (self.chr_bank as usize) * 0x2000 + (addr as usize);
                
                
                if bank_addr < self.chr_rom.len() {
                    self.chr_rom[bank_addr]
                } else {
                    0
                }
            },
            _ => {
                let chr_addr = (addr & 0x1FFF) as usize;
                if chr_addr < self.chr_rom.len() {
                    self.chr_rom[chr_addr]
                } else {
                    0
                }
            }
        }
    }
    
    

    pub fn write_chr(&mut self, addr: u16, data: u8) {
        match self.mapper {
            0 => {
                // NROM mapper - 8KB CHR ROM handling
                let chr_addr = if self.chr_rom.len() == 0x2000 {
                    // 8KB CHR ROM: Direct mapping, no mirroring
                    // Pattern table 0 (0x0000-0x0FFF): CHR[0x0000-0x0FFF]
                    // Pattern table 1 (0x1000-0x1FFF): CHR[0x1000-0x1FFF]
                    (addr & 0x1FFF) as usize
                } else {
                    // Larger CHR ROM: Normal access to both pattern tables
                    (addr & 0x1FFF) as usize
                };
                
                if chr_addr < self.chr_rom.len() {
                    self.chr_rom[chr_addr] = data;
                }
            },
            3 | 87 => {
                // Mapper 3 (CNROM) and Mapper 87 - 8KB CHR bank switching (write)
                let bank_addr = (self.chr_bank as usize) * 0x2000 + (addr as usize);
                if bank_addr < self.chr_rom.len() {
                    self.chr_rom[bank_addr] = data;
                }
            },
            _ => {
                self.chr_rom[(addr & 0x1FFF) as usize] = data;
            }
        }
    }

    pub fn mirroring(&self) -> Mirroring {
        self.mirroring
    }
    
    pub fn is_goonies(&self) -> bool {
        self.is_goonies
    }
    
    pub fn chr_rom_size(&self) -> usize {
        self.chr_rom.len()
    }
    
    pub fn mapper_number(&self) -> u8 {
        self.mapper
    }
    
    // Goonies-specific CHR read with status bar handling
    pub fn read_chr_goonies(&self, addr: u16, is_status_sprite: bool) -> u8 {
        if self.mapper == 87 {
            // For Goonies status bar sprites, always use bank 1 (status data is in bank 1)
            // For game area sprites, use current bank
            let forced_bank = if is_status_sprite { 1 } else { self.chr_bank };
            let bank_addr = (forced_bank as usize) * 0x2000 + (addr as usize);
            
            
            if bank_addr < self.chr_rom.len() {
                self.chr_rom[bank_addr]
            } else {
                0
            }
        } else {
            // Fall back to normal CHR read for other mappers
            self.read_chr(addr)
        }
    }
}