use std::fs::File;
use std::io::{Read, Result};

pub struct Cartridge {
    prg_rom: Vec<u8>,
    chr_rom: Vec<u8>,
    mapper: u8,
    mirroring: Mirroring,
    // Mapper 87 specific
    chr_bank: u8,
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
            vec![0; 8192]
        };

        let chr_banks = if chr_rom.len() > 0 { chr_rom.len() / 0x2000 } else { 0 };
        println!("Cartridge loaded - Mapper: {}, PRG ROM: {} bytes, CHR ROM: {} bytes ({} banks)", 
                 mapper, prg_rom.len(), chr_rom.len(), chr_banks);
        
        Ok(Cartridge {
            prg_rom,
            chr_rom,
            mapper,
            mirroring,
            chr_bank: 0,
        })
    }

    pub fn read_prg(&self, addr: u16) -> u8 {
        match self.mapper {
            0 => {
                // NROM mapper - simple direct mapping
                let len = self.prg_rom.len();
                if len == 16384 {
                    self.prg_rom[(addr & 0x3FFF) as usize]
                } else {
                    self.prg_rom[(addr & 0x7FFF) as usize]
                }
            },
            87 => {
                // Mapper 87 - treat as NROM for now (debugging)
                let len = self.prg_rom.len();
                if len == 16384 {
                    self.prg_rom[(addr & 0x3FFF) as usize]
                } else {
                    self.prg_rom[(addr & 0x7FFF) as usize]
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
            87 => {
                // Mapper 87 - CHR bank switching at $6000
                if addr == 0x6000 {
                    // Extract bits 1-0 for CHR bank (2 banks available)
                    self.chr_bank = (data >> 1) & 0x01;
                }
            },
            _ => {} // Unsupported mapper
        }
    }

    pub fn read_chr(&self, addr: u16) -> u8 {
        match self.mapper {
            0 => {
                self.chr_rom[(addr & 0x1FFF) as usize]
            },
            87 => {
                // Mapper 87 - 8KB CHR bank switching
                let bank_addr = (self.chr_bank as usize) * 0x2000 + (addr as usize);
                if bank_addr < self.chr_rom.len() {
                    self.chr_rom[bank_addr]
                } else {
                    0
                }
            },
            _ => {
                self.chr_rom[(addr & 0x1FFF) as usize]
            }
        }
    }

    pub fn write_chr(&mut self, addr: u16, data: u8) {
        match self.mapper {
            0 => {
                self.chr_rom[(addr & 0x1FFF) as usize] = data;
            },
            87 => {
                // Mapper 87 - 8KB CHR bank switching (write)
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
}