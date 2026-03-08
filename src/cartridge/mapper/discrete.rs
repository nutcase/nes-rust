use super::super::{Cartridge, Mirroring};

impl Cartridge {
    pub(in crate::cartridge) fn sync_mapper41_chr_bank(&mut self) {
        let outer_bank = self.chr_bank >> 2;
        self.chr_bank = (outer_bank << 2)
            | if self.prg_bank & 0x04 != 0 {
                self.mapper41_inner_bank & 0x03
            } else {
                0
            };
    }

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

    /// Mapper 41: Caltron 6-in-1 outer register lives in $6000-$67FF and
    /// selects 32KB PRG, 32KB CHR outer bank, mirroring, and whether writes
    /// to $8000-$FFFF update the hidden inner 8KB CHR bank latch.
    pub(in crate::cartridge) fn write_prg_ram_mapper41(&mut self, addr: u16) {
        if !(0x6000..=0x67FF).contains(&addr) {
            return;
        }

        let prg_bank_count = (self.prg_rom.len() / 0x8000).max(1);
        let outer_chr_bank_count = (self.chr_rom.len() / 0x2000).max(1).div_ceil(4);
        let outer_bank = (((addr >> 3) & 0x03) as usize % outer_chr_bank_count.max(1)) as u8;

        self.prg_bank = ((addr as usize & 0x07) % prg_bank_count) as u8;
        self.chr_bank = outer_bank << 2;
        self.mirroring = if addr & 0x20 != 0 {
            Mirroring::Horizontal
        } else {
            Mirroring::Vertical
        };
        self.sync_mapper41_chr_bank();
    }

    pub(in crate::cartridge) fn write_prg_mapper41(&mut self, addr: u16, data: u8) {
        if addr < 0x8000 || self.prg_bank & 0x04 == 0 {
            return;
        }

        let effective = data & self.read_prg_axrom(addr);
        self.mapper41_inner_bank = effective & 0x03;
        self.sync_mapper41_chr_bank();
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

    fn mapper137_chr_bank_1k(&self, slot: usize) -> usize {
        match slot & 3 {
            0 => (self.mapper137_registers[0] & 0x07) as usize,
            1 => {
                (((self.mapper137_registers[4] & 0x01) << 4) | (self.mapper137_registers[1] & 0x07))
                    as usize
            }
            2 => {
                ((((self.mapper137_registers[4] >> 1) & 0x01) << 4)
                    | (self.mapper137_registers[2] & 0x07)) as usize
            }
            _ => {
                ((((self.mapper137_registers[4] >> 2) & 0x01) << 4)
                    | ((self.mapper137_registers[6] & 0x01) << 3)
                    | (self.mapper137_registers[3] & 0x07)) as usize
            }
        }
    }

    pub(in crate::cartridge) fn update_mapper137_state(&mut self) {
        let prg_bank_count = (self.prg_rom.len() / 0x8000).max(1);
        self.prg_bank = ((self.mapper137_registers[5] as usize & 0x07) % prg_bank_count) as u8;
        self.chr_bank = self.mapper137_chr_bank_1k(0) as u8;
        self.mirroring = match (self.mapper137_registers[7] >> 1) & 0x03 {
            1 => Mirroring::Horizontal,
            2 => Mirroring::Vertical,
            3 => Mirroring::OneScreenUpper,
            _ => Mirroring::Vertical,
        };
    }

    /// Mapper 137 (Sachen 8259D): $4100 selects one of eight 3-bit registers
    /// and $4101 writes the selected value. The low 4KB of CHR uses four 1KB
    /// banks while the upper 4KB is fixed to the last 4KB of CHR-ROM.
    pub(in crate::cartridge) fn write_prg_mapper137(&mut self, addr: u16, data: u8) {
        match addr & 0x4101 {
            0x4100 => {
                self.mapper137_index = data & 0x07;
            }
            0x4101 => {
                let reg = self.mapper137_index as usize & 0x07;
                self.mapper137_registers[reg] = data & 0x07;
                self.update_mapper137_state();
            }
            _ => {}
        }
    }

    pub(in crate::cartridge) fn read_prg_low_mapper137(&self, addr: u16) -> u8 {
        if (addr & 0x4101) == 0x4101 {
            self.mapper137_registers[self.mapper137_index as usize & 0x07] & 0x07
        } else {
            0
        }
    }

    pub(in crate::cartridge) fn read_chr_mapper137(&self, addr: u16) -> u8 {
        if self.chr_rom.is_empty() {
            return 0;
        }

        let chr_addr = if addr < 0x1000 {
            let slot = (addr as usize) / 0x0400;
            let bank_count = (self.chr_rom.len() / 0x0400).max(1);
            let bank = self.mapper137_chr_bank_1k(slot) % bank_count;
            bank * 0x0400 + (addr as usize & 0x03FF)
        } else {
            self.chr_rom.len().saturating_sub(0x1000) + (addr as usize & 0x0FFF)
        };
        self.chr_rom[chr_addr % self.chr_rom.len()]
    }

    pub(in crate::cartridge) fn write_chr_mapper137(&mut self, addr: u16, data: u8) {
        if self.chr_rom.is_empty() {
            return;
        }

        let chr_addr = if addr < 0x1000 {
            let slot = (addr as usize) / 0x0400;
            let bank_count = (self.chr_rom.len() / 0x0400).max(1);
            let bank = self.mapper137_chr_bank_1k(slot) % bank_count;
            bank * 0x0400 + (addr as usize & 0x03FF)
        } else {
            self.chr_rom.len().saturating_sub(0x1000) + (addr as usize & 0x0FFF)
        };
        let chr_addr = chr_addr % self.chr_rom.len();
        self.chr_rom[chr_addr] = data;
    }

    pub(in crate::cartridge) fn update_mapper150_state(&mut self) {
        let prg_bank_count = (self.prg_rom.len() / 0x8000).max(1);
        let chr_bank_count = (self.chr_rom.len() / 0x2000).max(1);

        self.prg_bank = ((self.mapper150_registers[5] as usize & 0x03) % prg_bank_count) as u8;
        self.chr_bank = ((((self.mapper150_registers[4] & 0x01) << 2)
            | (self.mapper150_registers[6] & 0x03)) as usize
            % chr_bank_count) as u8;
        self.mirroring = match (self.mapper150_registers[7] >> 1) & 0x03 {
            0 => Mirroring::ThreeScreenLower,
            1 => Mirroring::Horizontal,
            2 => Mirroring::Vertical,
            _ => Mirroring::OneScreenUpper,
        };
    }

    /// Mapper 150 (Sachen SA-015): same low-address register-file layout as
    /// mapper 243, but with a 2-bit 32KB PRG bank and a 3-bit 8KB CHR bank.
    pub(in crate::cartridge) fn write_prg_mapper150(&mut self, addr: u16, data: u8) {
        match addr & 0xC101 {
            0x4100 => {
                self.mapper150_index = data & 0x07;
            }
            0x4101 => {
                let reg = self.mapper150_index as usize & 0x07;
                self.mapper150_registers[reg] = data & 0x07;
                self.update_mapper150_state();
            }
            _ => {}
        }
    }

    pub(in crate::cartridge) fn read_prg_low_mapper150(&self, addr: u16) -> u8 {
        if (addr & 0xC101) == 0x4101 {
            self.mapper150_registers[self.mapper150_index as usize & 0x07] & 0x07
        } else {
            0
        }
    }
}
