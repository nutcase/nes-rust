use std::fs::File;
use std::io::{Read, Result};

pub struct Cartridge {
    prg_rom: Vec<u8>,
    chr_rom: Vec<u8>,
    mapper: u8,
    mirroring: Mirroring,
    // Mapper 3 and 87 specific
    chr_bank: u8,
    prg_bank: u8,  // Add PRG bank for proper mapper support
    // Game-specific flags
    is_goonies: bool,
    // MMC1 specific
    mmc1: Option<Mmc1>,
    // Debug counters
    chr_read_count: u32,
    chr_write_count: u32,
}

#[derive(Debug, Clone)]
struct Mmc1 {
    shift_register: u8,
    shift_count: u8,
    control: u8,
    chr_bank_0: u8,
    chr_bank_1: u8,
    prg_bank: u8,
    prg_ram_disable: bool,
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
        let chr_rom = if chr_rom_size > 0 {
            data[chr_rom_start..chr_rom_start + chr_rom_size].to_vec()
        } else {
            // CHR RAM - initialize with zeros
            vec![0; 8192]
        };
        
        // Normal CHR ROM handling - no special processing for now

        
        
        // Detect Goonies by ROM size and mapper
        let is_goonies = (mapper == 3 || mapper == 87) && 
                         prg_rom.len() == 32768 && 
                         chr_rom.len() == 16384;
        
        
        let mmc1 = if mapper == 1 {
            Some(Mmc1 {
                shift_register: 0x10,
                shift_count: 0,
                control: 0x0C, // Default: 16KB PRG mode, last bank fixed
                chr_bank_0: 0,
                chr_bank_1: 0,
                prg_bank: 0,
                prg_ram_disable: false,
            })
        } else {
            None
        };

        Ok(Cartridge {
            prg_rom,
            chr_rom,
            mapper,
            mirroring,
            chr_bank: 0,
            prg_bank: 0,
            is_goonies,
            mmc1,
            chr_read_count: 0,
            chr_write_count: 0,
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
            2 => {
                // UxROM mapper - 16KB switchable + 16KB fixed
                if addr < 0xC000 {
                    // Switchable 16KB bank at $8000-$BFFF
                    let bank = self.prg_bank;
                    let offset = (bank as usize) * 0x4000 + (rom_addr as usize);
                    if offset < self.prg_rom.len() {
                        self.prg_rom[offset]
                    } else {
                        0
                    }
                } else {
                    // Fixed last 16KB bank at $C000-$FFFF
                    let last_bank_offset = self.prg_rom.len() - 0x4000;
                    let offset = last_bank_offset + ((addr - 0xC000) as usize);
                    if offset < self.prg_rom.len() {
                        self.prg_rom[offset]
                    } else {
                        0
                    }
                }
            },
            1 => {
                // MMC1 mapper
                if let Some(ref mmc1) = self.mmc1 {
                    let prg_mode = (mmc1.control >> 2) & 0x03;
                    let prg_size = self.prg_rom.len() / 0x4000; // Number of 16KB banks
                    
                    match prg_mode {
                        0 | 1 => {
                            // 32KB mode: switch 32KB at $8000
                            let bank = (mmc1.prg_bank & 0x0E) >> 1; // Use bits 1-4, ignore bit 0
                            let offset = (bank as usize) * 0x8000 + (rom_addr as usize);
                            if offset < self.prg_rom.len() {
                                self.prg_rom[offset]
                            } else {
                                0
                            }
                        },
                        2 => {
                            // Fix first bank at $8000, switch 16KB at $C000
                            if addr < 0xC000 {
                                // Fixed first bank (bank 0)
                                let offset = rom_addr as usize;
                                if offset < self.prg_rom.len() {
                                    self.prg_rom[offset]
                                } else {
                                    0
                                }
                            } else {
                                // Switchable bank at $C000
                                let bank = mmc1.prg_bank & 0x0F;
                                let offset = (bank as usize) * 0x4000 + ((addr - 0xC000) as usize);
                                if offset < self.prg_rom.len() {
                                    self.prg_rom[offset]
                                } else {
                                    0
                                }
                            }
                        },
                        3 | _ => {
                            // Switch 16KB at $8000, fix last bank at $C000
                            // This is the default mode after reset
                            if addr < 0xC000 {
                                // Switchable bank at $8000
                                let bank = mmc1.prg_bank & 0x0F;
                                let offset = (bank as usize) * 0x4000 + (rom_addr as usize);
                                if offset < self.prg_rom.len() {
                                    self.prg_rom[offset]
                                } else {
                                    0
                                }
                            } else {
                                // Fixed last bank at $C000
                                let last_bank_offset = self.prg_rom.len() - 0x4000;
                                let offset = last_bank_offset + ((addr - 0xC000) as usize);
                                if offset < self.prg_rom.len() {
                                    self.prg_rom[offset]
                                } else {
                                    0
                                }
                            }
                        },
                        _ => 0
                    }
                } else {
                    0
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
            2 => {
                // UxROM mapper - bank switching
                if addr >= 0x8000 {
                    // Bus conflicts: AND written value with ROM value
                    let rom_offset = if addr < 0xC000 {
                        // Switchable bank
                        (self.prg_bank as usize) * 0x4000 + ((addr - 0x8000) as usize)
                    } else {
                        // Fixed last bank
                        self.prg_rom.len() - 0x4000 + ((addr - 0xC000) as usize)
                    };
                    
                    let rom_value = if rom_offset < self.prg_rom.len() {
                        self.prg_rom[rom_offset]
                    } else {
                        0xFF
                    };
                    
                    // Bus conflict resolution
                    let effective_value = data & rom_value;
                    self.prg_bank = effective_value & 0x07; // 3 bits for 8 banks max
                }
            },
            1 => {
                // MMC1 mapper - shift register handling
                if let Some(ref mut mmc1) = self.mmc1 {
                    // Check for reset (bit 7 set)
                    if data & 0x80 != 0 {
                        mmc1.shift_register = 0x10;
                        mmc1.shift_count = 0;
                        mmc1.control |= 0x0C; // Set bits 2-3 for 16KB PRG mode
                        return;
                    }
                    
                    // Shift in bit 0
                    mmc1.shift_register >>= 1;
                    if data & 0x01 != 0 {
                        mmc1.shift_register |= 0x10;
                    }
                    mmc1.shift_count += 1;
                    
                    // After 5 writes, update the target register
                    if mmc1.shift_count >= 5 {
                        let register_data = mmc1.shift_register & 0x1F;
                        
                        match (addr >> 13) & 0x03 {
                            0 => {
                                // Control register ($8000-$9FFF)
                                mmc1.control = register_data;
                                // Update mirroring based on control bits 0-1
                                self.mirroring = match register_data & 0x03 {
                                    0 => Mirroring::Horizontal, // One-screen, lower bank
                                    1 => Mirroring::Horizontal, // One-screen, upper bank  
                                    2 => Mirroring::Vertical,   // Vertical mirroring
                                    3 => Mirroring::Horizontal, // Horizontal mirroring
                                    _ => self.mirroring,
                                };
                            },
                            1 => {
                                // CHR bank 0 ($A000-$BFFF)
                                mmc1.chr_bank_0 = register_data;
                            },
                            2 => {
                                // CHR bank 1 ($C000-$DFFF)
                                mmc1.chr_bank_1 = register_data;
                            },
                            3 => {
                                // PRG bank ($E000-$FFFF)
                                mmc1.prg_bank = register_data & 0x0F;
                                mmc1.prg_ram_disable = (register_data & 0x10) != 0;
                            },
                            _ => {}
                        }
                        
                        // Reset shift register
                        mmc1.shift_register = 0x10;
                        mmc1.shift_count = 0;
                    }
                }
            },
            3 => {
                // Mapper 3 (CNROM) - CHR bank switching
                // Register at $8000-$FFFF
                // CNROM has bus conflicts - ROM value and CPU value must match
                if addr >= 0x8000 {
                    let old_bank = self.chr_bank;
                    
                    // Read ROM value at the write address to handle bus conflicts
                    let rom_value = if (addr as usize) < self.prg_rom.len() {
                        self.prg_rom[addr as usize]
                    } else {
                        // Handle mirrored addresses
                        let mirrored_addr = (addr - 0x8000) % (self.prg_rom.len() as u16);
                        self.prg_rom[mirrored_addr as usize]
                    };
                    
                    // Bus conflict: use AND of written value and ROM value
                    let effective_value = data & rom_value;
                    self.chr_bank = effective_value & 0x03; // Select one of 4 possible 8KB CHR banks
                    
                    // CHR bank switching is working correctly - disable debug logging
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
            2 => {
                // UxROM mapper - CHR RAM (8KB)
                let chr_addr = (addr & 0x1FFF) as usize;
                if chr_addr < self.chr_rom.len() {
                    self.chr_rom[chr_addr]
                } else {
                    0
                }
            },
            1 => {
                // MMC1 mapper - CHR bank switching
                if let Some(ref mmc1) = self.mmc1 {
                    let chr_mode = (mmc1.control >> 4) & 0x01;
                    
                    if chr_mode == 0 {
                        // 8KB mode: use CHR bank 0, ignore CHR bank 1
                        let bank = (mmc1.chr_bank_0 & 0x1E) >> 1; // Use even banks only
                        let offset = (bank as usize) * 0x2000 + (addr as usize);
                        if offset < self.chr_rom.len() {
                            self.chr_rom[offset]
                        } else {
                            0
                        }
                    } else {
                        // 4KB mode: separate banks for each 4KB region
                        if addr < 0x1000 {
                            // Pattern table 0: CHR bank 0
                            let bank = mmc1.chr_bank_0;
                            let offset = (bank as usize) * 0x1000 + (addr as usize);
                            if offset < self.chr_rom.len() {
                                self.chr_rom[offset]
                            } else {
                                0
                            }
                        } else {
                            // Pattern table 1: CHR bank 1
                            let bank = mmc1.chr_bank_1;
                            let offset = (bank as usize) * 0x1000 + ((addr - 0x1000) as usize);
                            if offset < self.chr_rom.len() {
                                self.chr_rom[offset]
                            } else {
                                0
                            }
                        }
                    }
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

    // Sprite-specific CHR read that handles mapper-specific requirements
    pub fn read_chr_sprite(&self, addr: u16, _sprite_y: u8) -> u8 {
        // For now, use normal CHR read for all sprites
        // Goonies status sprite issue needs further investigation
        self.read_chr(addr)
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
            2 => {
                // UxROM mapper - CHR RAM (8KB, writable)
                let chr_addr = (addr & 0x1FFF) as usize;
                if chr_addr < self.chr_rom.len() {
                    self.chr_rom[chr_addr] = data;
                }
            },
            1 => {
                // MMC1 mapper - CHR bank switching (write)
                if let Some(ref mmc1) = self.mmc1 {
                    let chr_mode = (mmc1.control >> 4) & 0x01;
                    
                    if chr_mode == 0 {
                        // 8KB mode: use CHR bank 0, ignore CHR bank 1
                        let bank = (mmc1.chr_bank_0 & 0x1E) >> 1; // Use even banks only
                        let offset = (bank as usize) * 0x2000 + (addr as usize);
                        if offset < self.chr_rom.len() {
                            self.chr_rom[offset] = data;
                        }
                    } else {
                        // 4KB mode: separate banks for each 4KB region
                        if addr < 0x1000 {
                            // Pattern table 0: CHR bank 0
                            let bank = mmc1.chr_bank_0;
                            let offset = (bank as usize) * 0x1000 + (addr as usize);
                            if offset < self.chr_rom.len() {
                                self.chr_rom[offset] = data;
                            }
                        } else {
                            // Pattern table 1: CHR bank 1
                            let bank = mmc1.chr_bank_1;
                            let offset = (bank as usize) * 0x1000 + ((addr - 0x1000) as usize);
                            if offset < self.chr_rom.len() {
                                self.chr_rom[offset] = data;
                            }
                        }
                    }
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
    
    // Save state methods
    pub fn get_prg_bank(&self) -> u8 {
        self.prg_bank
    }
    
    pub fn get_chr_bank(&self) -> u8 {
        self.chr_bank
    }
    
    pub fn set_prg_bank(&mut self, bank: u8) {
        self.prg_bank = bank;
    }
    
    pub fn set_chr_bank(&mut self, bank: u8) {
        self.chr_bank = bank;
    }
}