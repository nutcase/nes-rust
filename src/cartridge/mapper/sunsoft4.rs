use super::super::{Cartridge, Mirroring};

#[derive(Debug, Clone, Copy)]
pub struct Sunsoft4 {
    pub chr_banks: [u8; 4],
    pub nametable_banks: [u8; 2],
    pub control: u8,
    pub prg_bank: u8,
    pub prg_ram_enabled: bool,
    pub nametable_chr_rom: bool,
}

impl Sunsoft4 {
    pub fn new() -> Self {
        Self {
            chr_banks: [0; 4],
            nametable_banks: [0x80; 2],
            control: 0,
            prg_bank: 0,
            prg_ram_enabled: false,
            nametable_chr_rom: false,
        }
    }

    pub fn decode_mirroring(control: u8) -> Mirroring {
        match control & 0x03 {
            0 => Mirroring::Vertical,
            1 => Mirroring::Horizontal,
            2 => Mirroring::OneScreenLower,
            _ => Mirroring::OneScreenUpper,
        }
    }
}

impl Cartridge {
    fn read_chr_bank_2k(&self, bank: u8, offset: usize) -> u8 {
        if self.chr_rom.is_empty() {
            return 0;
        }

        let bank_count = (self.chr_rom.len() / 0x800).max(1);
        let bank = (bank as usize) % bank_count;
        let chr_addr = bank * 0x800 + offset;
        self.chr_rom[chr_addr % self.chr_rom.len()]
    }

    fn write_chr_bank_2k(&mut self, bank: u8, offset: usize, data: u8) {
        if self.chr_rom.is_empty() {
            return;
        }

        let bank_count = (self.chr_rom.len() / 0x800).max(1);
        let bank = (bank as usize) % bank_count;
        let chr_len = self.chr_rom.len();
        let chr_addr = bank * 0x800 + offset;
        self.chr_rom[chr_addr % chr_len] = data;
    }

    pub(in crate::cartridge) fn read_sunsoft4_nametable_chr(
        &self,
        physical_nt: usize,
        offset: usize,
    ) -> u8 {
        if self.chr_rom.is_empty() || offset >= 1024 {
            return 0;
        }

        let Some(sunsoft4) = self.sunsoft4.as_ref() else {
            return 0;
        };
        let bank_count = (self.chr_rom.len() / 0x400).max(1);
        let bank = (sunsoft4.nametable_banks[physical_nt & 1] as usize) % bank_count;
        let chr_addr = bank * 0x400 + offset;
        self.chr_rom[chr_addr % self.chr_rom.len()]
    }

    pub(in crate::cartridge) fn read_prg_sunsoft4(&self, addr: u16) -> u8 {
        let Some(sunsoft4) = self.sunsoft4.as_ref() else {
            return 0;
        };
        if self.prg_rom.is_empty() {
            return 0;
        }

        if addr < 0xC000 {
            let bank_count = (self.prg_rom.len() / 0x4000).max(1);
            let bank = (sunsoft4.prg_bank as usize) % bank_count;
            let offset = bank * 0x4000 + (addr.saturating_sub(0x8000) as usize);
            self.prg_rom[offset % self.prg_rom.len()]
        } else {
            let offset = self.prg_rom.len().saturating_sub(0x4000) + (addr - 0xC000) as usize;
            self.prg_rom[offset % self.prg_rom.len()]
        }
    }

    pub(in crate::cartridge) fn write_prg_sunsoft4(&mut self, addr: u16, data: u8) {
        let mut new_mirroring = None;
        let mut new_prg_bank = None;
        let new_chr_bank;
        if let Some(sunsoft4) = self.sunsoft4.as_mut() {
            match addr & 0xF000 {
                0x8000 => sunsoft4.chr_banks[0] = data,
                0x9000 => sunsoft4.chr_banks[1] = data,
                0xA000 => sunsoft4.chr_banks[2] = data,
                0xB000 => sunsoft4.chr_banks[3] = data,
                0xC000 => sunsoft4.nametable_banks[0] = 0x80 | (data & 0x7F),
                0xD000 => sunsoft4.nametable_banks[1] = 0x80 | (data & 0x7F),
                0xE000 => {
                    sunsoft4.control = data;
                    sunsoft4.nametable_chr_rom = data & 0x10 != 0;
                    new_mirroring = Some(Sunsoft4::decode_mirroring(data));
                }
                0xF000 => {
                    sunsoft4.prg_bank = data & 0x0F;
                    sunsoft4.prg_ram_enabled = data & 0x10 != 0;
                    new_prg_bank = Some(sunsoft4.prg_bank);
                }
                _ => {}
            }
            new_chr_bank = sunsoft4.chr_banks[0];
        } else {
            return;
        }

        if let Some(mirroring) = new_mirroring {
            self.mirroring = mirroring;
        }
        if let Some(prg_bank) = new_prg_bank {
            self.prg_bank = prg_bank;
        }
        self.chr_bank = new_chr_bank;
    }

    pub(in crate::cartridge) fn read_chr_sunsoft4(&self, addr: u16) -> u8 {
        let Some(sunsoft4) = self.sunsoft4.as_ref() else {
            return 0;
        };
        let slot = ((addr as usize) >> 11) & 0x03;
        let offset = (addr as usize) & 0x07FF;
        self.read_chr_bank_2k(sunsoft4.chr_banks[slot], offset)
    }

    pub(in crate::cartridge) fn write_chr_sunsoft4(&mut self, addr: u16, data: u8) {
        let Some(bank) = self
            .sunsoft4
            .as_ref()
            .map(|sunsoft4| sunsoft4.chr_banks[((addr as usize) >> 11) & 0x03])
        else {
            return;
        };
        let offset = (addr as usize) & 0x07FF;
        self.write_chr_bank_2k(bank, offset, data);
    }

    pub(in crate::cartridge) fn read_prg_ram_sunsoft4(&self, addr: u16) -> u8 {
        let Some(prg_ram_enabled) = self
            .sunsoft4
            .as_ref()
            .map(|sunsoft4| sunsoft4.prg_ram_enabled)
        else {
            return 0;
        };
        if !prg_ram_enabled {
            return 0;
        }

        let offset = (addr.saturating_sub(0x6000) as usize) % self.prg_ram.len().max(1);
        self.prg_ram.get(offset).copied().unwrap_or(0)
    }

    pub(in crate::cartridge) fn write_prg_ram_sunsoft4(&mut self, addr: u16, data: u8) {
        let Some(prg_ram_enabled) = self
            .sunsoft4
            .as_ref()
            .map(|sunsoft4| sunsoft4.prg_ram_enabled)
        else {
            return;
        };
        if !prg_ram_enabled || self.prg_ram.is_empty() {
            return;
        }

        let offset = (addr.saturating_sub(0x6000) as usize) % self.prg_ram.len();
        self.prg_ram[offset] = data;
        self.has_valid_save_data = true;
    }
}
