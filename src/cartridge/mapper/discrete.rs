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

    /// Mapper 133: simplified Sachen latch variant wired like mapper 79 with
    /// one PRG bit and two CHR bits.
    pub(in crate::cartridge) fn write_prg_mapper133(&mut self, addr: u16, data: u8) {
        if (addr & 0xE100) == 0x4100 {
            let prg_bank_count = (self.prg_rom.len() / 0x8000).max(1);
            let chr_bank_count = (self.chr_rom.len() / 0x2000).max(1);
            self.prg_bank = (((data >> 2) & 0x01) as usize % prg_bank_count) as u8;
            self.chr_bank = ((data & 0x03) as usize % chr_bank_count) as u8;
        }
    }

    /// Mapper 113 (HES NTD-8): 32KB PRG, 8KB CHR, and a mirroring bit on
    /// the same low-address latch family as mapper 79.
    pub(in crate::cartridge) fn write_prg_mapper113(&mut self, addr: u16, data: u8) {
        if (addr & 0xE100) == 0x4100 {
            let prg_bank_count = (self.prg_rom.len() / 0x8000).max(1);
            let chr_bank_count = (self.chr_rom.len() / 0x2000).max(1);
            self.prg_bank = (((data >> 3) & 0x07) as usize % prg_bank_count) as u8;
            self.chr_bank =
                ((((data >> 3) & 0x08) | (data & 0x07)) as usize % chr_bank_count) as u8;
            self.mirroring = if data & 0x80 != 0 {
                Mirroring::Horizontal
            } else {
                Mirroring::Vertical
            };
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

    /// Mapper 147 (Sachen TC-U01-1.5M): low-address latch with two PRG bits
    /// split across D2 and D7, and four CHR bits on D3-D6.
    pub(in crate::cartridge) fn write_prg_mapper147(&mut self, addr: u16, data: u8) {
        if (addr & 0x4103) == 0x4102 {
            let effective = if addr >= 0x8000 {
                data & self.bus_conflict_value_switchable_32k(addr)
            } else {
                data
            };
            let prg_bank_count = (self.prg_rom.len() / 0x8000).max(1);
            let chr_bank_count = (self.chr_rom.len() / 0x2000).max(1);

            self.prg_bank = ((((effective >> 2) & 0x01) | ((effective >> 6) & 0x02)) as usize
                % prg_bank_count) as u8;
            self.chr_bank = (((effective >> 3) & 0x0F) as usize % chr_bank_count) as u8;
        }
    }

    fn update_mapper243_state(&mut self) {
        let prg_bank_count = (self.prg_rom.len() / 0x8000).max(1);
        let chr_bank_count = (self.chr_rom.len() / 0x2000).max(1);

        self.prg_bank = (((self.mapper243_registers[5] as usize) & 0x03) % prg_bank_count) as u8;
        self.chr_bank = ((((self.mapper243_registers[6] & 0x03) << 2)
            | ((self.mapper243_registers[4] & 0x01) << 1)
            | (self.mapper243_registers[2] & 0x01)) as usize
            % chr_bank_count) as u8;
        self.mirroring = match (self.mapper243_registers[7] >> 1) & 0x03 {
            0 => Mirroring::ThreeScreenLower,
            1 => Mirroring::Vertical,
            2 => Mirroring::Horizontal,
            _ => Mirroring::OneScreenUpper,
        };
    }

    /// Mapper 243 (Sachen 74LS374N): low-address index/data register file.
    pub(in crate::cartridge) fn write_prg_mapper243(&mut self, addr: u16, data: u8) {
        match addr & 0xC101 {
            0x4100 => {
                self.mapper243_index = data & 0x07;
            }
            0x4101 => {
                let reg = self.mapper243_index as usize & 0x07;
                self.mapper243_registers[reg] = data & 0x07;
                self.update_mapper243_state();
            }
            _ => {}
        }
    }

    pub(in crate::cartridge) fn read_prg_low_mapper243(&self, addr: u16) -> u8 {
        if (addr & 0xC101) == 0x4101 {
            self.mapper243_registers[self.mapper243_index as usize & 0x07] & 0x07
        } else {
            0
        }
    }
}
