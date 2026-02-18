mod mapper;

use mapper::Mmc1;
use std::fs::File;
use std::io::{Read, Result};

pub struct Cartridge {
    prg_rom: Vec<u8>,
    chr_rom: Vec<u8>,
    chr_ram: Vec<u8>,  // CHR-RAM for MMC1 and other mappers
    prg_ram: Vec<u8>,  // Battery-backed SRAM for save data
    has_valid_save_data: bool,
    mapper: u8,
    mirroring: Mirroring,
    has_battery: bool,
    chr_bank: u8,
    prg_bank: u8,
    is_goonies: bool,
    mmc1: Option<Mmc1>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Mirroring {
    Horizontal,
    Vertical,
    FourScreen,
    OneScreenLower,
    OneScreenUpper,
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

        let has_battery = (flags6 & 0x02) != 0;

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

        // Detect Goonies by ROM size and mapper
        let is_goonies = (mapper == 3 || mapper == 87) &&
                         prg_rom.len() == 32768 &&
                         chr_rom.len() == 16384;

        let mmc1 = if mapper == 1 {
            Some(Mmc1::new())
        } else {
            None
        };

        // Initialize PRG-RAM for mappers that support it
        let prg_ram = if mapper == 1 {
            vec![0x00; 8192]
        } else {
            Vec::new()
        };

        let chr_ram = if mapper == 1 && chr_rom_size == 0 {
            vec![0x00; 8192]
        } else {
            vec![]
        };

        let cartridge = Cartridge {
            prg_rom,
            chr_rom,
            chr_ram,
            prg_ram,
            has_valid_save_data: false,
            mapper,
            mirroring,
            has_battery,
            chr_bank: 0,
            prg_bank: 0,
            is_goonies,
            mmc1,
        };

        Ok(cartridge)
    }

    pub fn read_prg(&self, addr: u16) -> u8 {
        let rom_addr = addr - 0x8000;
        match self.mapper {
            0 | 3 | 87 => self.read_prg_nrom(rom_addr),
            1 => self.read_prg_mmc1(addr, rom_addr),
            2 => self.read_prg_uxrom(addr, rom_addr),
            _ => 0,
        }
    }

    pub fn write_prg(&mut self, addr: u16, data: u8) {
        match self.mapper {
            0 => {}
            1 => self.write_prg_mmc1(addr, data),
            2 => self.write_prg_uxrom(addr, data),
            3 => self.write_prg_cnrom(addr, data),
            87 => self.write_prg_mapper87(addr, data),
            _ => {}
        }
    }

    #[inline]
    pub fn read_chr(&self, addr: u16) -> u8 {
        match self.mapper {
            0 => self.read_chr_nrom(addr),
            1 => self.read_chr_mmc1(addr),
            2 => self.read_chr_uxrom(addr),
            3 | 87 => self.read_chr_cnrom(addr),
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

    pub fn read_chr_sprite(&self, addr: u16, _sprite_y: u8) -> u8 {
        self.read_chr(addr)
    }

    pub fn write_chr(&mut self, addr: u16, data: u8) {
        match self.mapper {
            0 => self.write_chr_nrom(addr, data),
            1 => self.write_chr_mmc1(addr, data),
            2 => self.write_chr_uxrom(addr, data),
            3 | 87 => self.write_chr_cnrom(addr, data),
            _ => {
                self.chr_rom[(addr & 0x1FFF) as usize] = data;
            }
        }
    }

    pub fn read_prg_ram(&self, addr: u16) -> u8 {
        match self.mapper {
            1 => self.read_prg_ram_mmc1(addr),
            _ => 0,
        }
    }

    pub fn write_prg_ram(&mut self, addr: u16, data: u8) {
        match self.mapper {
            87 => self.write_prg_mapper87(addr, data),
            1 => self.write_prg_ram_mmc1(addr, data),
            _ => {}
        }
    }

    pub fn mirroring(&self) -> Mirroring {
        self.mirroring
    }

    pub fn is_goonies(&self) -> bool {
        self.is_goonies
    }

    pub fn goonies_check_ce7x_loop(&self, _pc: u16, _sp: u8, _cycles: u64) -> Option<(u16, u8)> {
        if !self.is_goonies {
            return None;
        }
        None
    }

    pub fn goonies_check_abnormal_brk(&self, _pc: u16, _sp: u8, _cycles: u64) -> Option<(u16, u8)> {
        if !self.is_goonies {
            return None;
        }
        None
    }

    pub fn chr_rom_size(&self) -> usize {
        self.chr_rom.len()
    }

    pub fn mapper_number(&self) -> u8 {
        self.mapper
    }

    pub fn prg_rom_size(&self) -> usize {
        self.prg_rom.len()
    }

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

    pub fn has_battery_save(&self) -> bool {
        self.has_battery && !self.prg_ram.is_empty()
    }

    pub fn get_sram_data(&self) -> Option<&[u8]> {
        if self.has_battery && !self.prg_ram.is_empty() && self.has_valid_save_data {
            Some(&self.prg_ram)
        } else {
            None
        }
    }

    pub fn set_sram_data(&mut self, data: Vec<u8>) {
        if self.has_battery && data.len() == self.prg_ram.len() {
            self.prg_ram = data;
            self.has_valid_save_data = true;
        }
    }

    /// Direct reference to PRG-RAM (returns None if empty).
    pub fn prg_ram_ref(&self) -> Option<&[u8]> {
        if self.prg_ram.is_empty() {
            None
        } else {
            Some(&self.prg_ram)
        }
    }

    /// Mutable reference to PRG-RAM (returns None if empty).
    pub fn prg_ram_mut(&mut self) -> Option<&mut [u8]> {
        if self.prg_ram.is_empty() {
            None
        } else {
            Some(&mut self.prg_ram)
        }
    }
}
