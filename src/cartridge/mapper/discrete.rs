use super::super::{Cartridge, Mirroring};

impl Cartridge {
    fn bus_conflict_value_fixed_last_16k(&self, addr: u16) -> u8 {
        if self.prg_rom.is_empty() {
            return 0xFF;
        }

        let offset = if addr < 0xC000 {
            (self.prg_bank as usize) * 0x4000 + (addr.saturating_sub(0x8000) as usize)
        } else {
            self.prg_rom.len().saturating_sub(0x4000) + (addr.saturating_sub(0xC000) as usize)
        };

        self.prg_rom[offset % self.prg_rom.len()]
    }

    fn bus_conflict_value_switchable_32k(&self, addr: u16) -> u8 {
        if self.prg_rom.is_empty() {
            return 0xFF;
        }

        let offset = (self.prg_bank as usize) * 0x8000 + (addr.saturating_sub(0x8000) as usize);
        self.prg_rom[offset % self.prg_rom.len()]
    }

    /// Mapper 79 / 146: AVE NINA-03/NINA-06 latch at addresses matching
    /// 010x xxx1 xxxx xxxx, selecting a 32KB PRG bank and an 8KB CHR bank.
    pub(in crate::cartridge) fn write_prg_mapper79_146(&mut self, addr: u16, data: u8) {
        if (addr & 0xE100) == 0x4100 {
            let prg_bank_count = (self.prg_rom.len() / 0x8000).max(1);
            let chr_bank_count = (self.chr_rom.len() / 0x2000).max(1);
            self.prg_bank = (((data >> 3) & 0x01) as usize % prg_bank_count) as u8;
            self.chr_bank = ((data & 0x07) as usize % chr_bank_count) as u8;
        }
    }

    /// Mapper 89 (Sunsoft-2): fixed last PRG bank, switchable low 16KB PRG,
    /// switchable 8KB CHR, and one-screen mirroring control.
    pub(in crate::cartridge) fn write_prg_mapper89(&mut self, addr: u16, data: u8) {
        if addr >= 0x8000 {
            let effective = data & self.bus_conflict_value_fixed_last_16k(addr);
            let prg_bank_count = (self.prg_rom.len() / 0x4000).max(1);
            let chr_bank_count = (self.chr_rom.len() / 0x2000).max(1);

            self.prg_bank = (((effective >> 4) & 0x07) as usize % prg_bank_count) as u8;
            self.chr_bank =
                ((((effective >> 4) & 0x08) | (effective & 0x07)) as usize % chr_bank_count) as u8;
            self.mirroring = if effective & 0x08 != 0 {
                Mirroring::OneScreenUpper
            } else {
                Mirroring::OneScreenLower
            };
        }
    }

    /// Mapper 93 (Sunsoft-2 variant): fixed last PRG bank with a gated
    /// CHR-RAM data path. When disabled, CHR reads return a simple open-bus
    /// approximation because the PPU open-bus latch is not modeled.
    pub(in crate::cartridge) fn write_prg_mapper93(&mut self, addr: u16, data: u8) {
        if addr >= 0x8000 {
            let effective = data & self.bus_conflict_value_fixed_last_16k(addr);
            let prg_bank_count = (self.prg_rom.len() / 0x4000).max(1);
            self.prg_bank = (((effective >> 4) & 0x07) as usize % prg_bank_count) as u8;
            self.mapper93_chr_ram_enabled = effective & 0x01 != 0;
        }
    }

    pub(in crate::cartridge) fn read_chr_mapper93(&self, addr: u16) -> u8 {
        if self.mapper93_chr_ram_enabled {
            self.read_chr_uxrom(addr)
        } else {
            0xFF
        }
    }

    pub(in crate::cartridge) fn write_chr_mapper93(&mut self, addr: u16, data: u8) {
        if self.mapper93_chr_ram_enabled {
            self.write_chr_uxrom(addr, data);
        }
    }

    /// Mapper 148 (Sachen SA-008-A): bus-conflict variant of the NINA-03
    /// style 32KB PRG / 8KB CHR latch.
    pub(in crate::cartridge) fn write_prg_mapper148(&mut self, addr: u16, data: u8) {
        if addr >= 0x8000 {
            let effective = data & self.bus_conflict_value_switchable_32k(addr);
            let prg_bank_count = (self.prg_rom.len() / 0x8000).max(1);
            let chr_bank_count = (self.chr_rom.len() / 0x2000).max(1);
            self.prg_bank = (((effective >> 3) & 0x01) as usize % prg_bank_count) as u8;
            self.chr_bank = ((effective & 0x07) as usize % chr_bank_count) as u8;
        }
    }
}
