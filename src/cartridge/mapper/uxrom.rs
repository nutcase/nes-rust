use super::super::Cartridge;

impl Cartridge {
    /// UxROM PRG read - 16KB switchable + 16KB fixed
    pub(in crate::cartridge) fn read_prg_uxrom(&self, addr: u16, rom_addr: u16) -> u8 {
        if addr < 0xC000 {
            // Switchable 16KB bank at $8000-$BFFF
            let offset = (self.prg_bank as usize) * 0x4000 + (rom_addr as usize);
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
    }

    /// Mapper 58: multicart board that can behave as either NROM-256
    /// (32KB switchable) or NROM-128 (16KB mirrored) depending on the
    /// latched address mode bit.
    pub(in crate::cartridge) fn read_prg_mapper58(&self, addr: u16) -> u8 {
        if self.mapper58_nrom128 {
            let bank_count = (self.prg_rom.len() / 0x4000).max(1);
            let bank = (self.prg_bank as usize) % bank_count;
            let offset = bank * 0x4000 + ((addr - 0x8000) as usize & 0x3FFF);
            self.prg_rom[offset % self.prg_rom.len()]
        } else {
            let bank_count = (self.prg_rom.len() / 0x8000).max(1);
            let bank = ((self.prg_bank as usize) >> 1) % bank_count;
            let offset = bank * 0x8000 + (addr - 0x8000) as usize;
            self.prg_rom[offset % self.prg_rom.len()]
        }
    }

    /// UxROM PRG write - bank switching with bus conflicts
    pub(in crate::cartridge) fn write_prg_uxrom(&mut self, addr: u16, data: u8) {
        if addr >= 0x8000 {
            // Bus conflicts: AND written value with ROM value
            let rom_offset = if addr < 0xC000 {
                (self.prg_bank as usize) * 0x4000 + ((addr - 0x8000) as usize)
            } else {
                self.prg_rom.len() - 0x4000 + ((addr - 0xC000) as usize)
            };

            let rom_value = if rom_offset < self.prg_rom.len() {
                self.prg_rom[rom_offset]
            } else {
                0xFF
            };

            let effective_value = data & rom_value;
            self.prg_bank = effective_value & 0x07;
        }
    }

    /// Mapper 180 (UNROM-180): fixed first 16KB bank at $8000 and
    /// switchable 16KB bank at $C000.
    pub(in crate::cartridge) fn read_prg_uxrom_inverted(&self, addr: u16, rom_addr: u16) -> u8 {
        if addr < 0xC000 {
            let offset = (addr - 0x8000) as usize;
            if offset < self.prg_rom.len() {
                self.prg_rom[offset]
            } else {
                0
            }
        } else {
            let bank_count = (self.prg_rom.len() / 0x4000).max(1);
            let bank = (self.prg_bank as usize) % bank_count;
            let offset = bank * 0x4000 + ((rom_addr - 0x4000) as usize);
            if offset < self.prg_rom.len() {
                self.prg_rom[offset]
            } else {
                0
            }
        }
    }

    pub(in crate::cartridge) fn write_prg_uxrom_inverted(&mut self, addr: u16, data: u8) {
        if addr >= 0x8000 {
            let rom_offset = if addr < 0xC000 {
                (addr - 0x8000) as usize
            } else {
                (self.prg_bank as usize) * 0x4000 + ((addr - 0xC000) as usize)
            };

            let rom_value = if rom_offset < self.prg_rom.len() {
                self.prg_rom[rom_offset]
            } else {
                0xFF
            };

            let effective_value = data & rom_value;
            let bank_count = (self.prg_rom.len() / 0x4000).max(1);
            self.prg_bank = (effective_value as usize % bank_count) as u8;
        }
    }

    /// Mapper 97 (Irem TAM-S1): fixed last 16KB at $8000, switchable 16KB at
    /// $C000, plus a mirroring control bit on the write register.
    pub(in crate::cartridge) fn read_prg_fixed_last_switch_high(
        &self,
        addr: u16,
        rom_addr: u16,
    ) -> u8 {
        if addr < 0xC000 {
            let last_bank_offset = self.prg_rom.len().saturating_sub(0x4000);
            let offset = last_bank_offset + (addr - 0x8000) as usize;
            self.prg_rom[offset % self.prg_rom.len()]
        } else {
            let bank_count = (self.prg_rom.len() / 0x4000).max(1);
            let bank = (self.prg_bank as usize) % bank_count;
            let offset = bank * 0x4000 + (rom_addr - 0x4000) as usize;
            self.prg_rom[offset % self.prg_rom.len()]
        }
    }

    pub(in crate::cartridge) fn write_prg_mapper97(&mut self, addr: u16, data: u8) {
        if addr >= 0x8000 {
            let bank_count = (self.prg_rom.len() / 0x4000).max(1);
            self.prg_bank = ((data & 0x1F) as usize % bank_count) as u8;
            self.mirroring = if data & 0x80 != 0 {
                crate::cartridge::Mirroring::Vertical
            } else {
                crate::cartridge::Mirroring::Horizontal
            };
        }
    }

    /// Mapper 81: the four low address bits on writes to $8000-$FFFF latch
    /// both the 16KB PRG bank at $8000 and the 8KB CHR bank.
    pub(in crate::cartridge) fn write_prg_mapper81(&mut self, addr: u16) {
        if addr >= 0x8000 {
            self.prg_bank = ((addr >> 2) & 0x03) as u8;
            self.chr_bank = (addr & 0x03) as u8;
        }
    }

    /// Mapper 58 latches bank bits directly from the CPU address.
    pub(in crate::cartridge) fn write_prg_mapper58(&mut self, addr: u16) {
        if addr >= 0x8000 {
            let prg_bank_count_16k = (self.prg_rom.len() / 0x4000).max(1);
            let chr_bank_count = (self.chr_rom.len() / 0x2000).max(1);

            self.prg_bank = ((addr as usize & 0x07) % prg_bank_count_16k) as u8;
            self.chr_bank = (((addr as usize >> 3) & 0x07) % chr_bank_count) as u8;
            self.mapper58_nrom128 = addr & 0x40 != 0;
            self.mirroring = if addr & 0x80 != 0 {
                crate::cartridge::Mirroring::Horizontal
            } else {
                crate::cartridge::Mirroring::Vertical
            };
        }
    }

    /// UxROM CHR read - 8KB CHR RAM
    pub(in crate::cartridge) fn read_chr_uxrom(&self, addr: u16) -> u8 {
        let chr_addr = (addr & 0x1FFF) as usize;
        if chr_addr < self.chr_rom.len() {
            self.chr_rom[chr_addr]
        } else {
            0
        }
    }

    /// UxROM CHR write - 8KB CHR RAM (writable)
    pub(in crate::cartridge) fn write_chr_uxrom(&mut self, addr: u16, data: u8) {
        let chr_addr = (addr & 0x1FFF) as usize;
        if chr_addr < self.chr_rom.len() {
            self.chr_rom[chr_addr] = data;
        }
    }
}
