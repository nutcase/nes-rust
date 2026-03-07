use super::super::{Cartridge, Mirroring};

impl Cartridge {
    fn mapper227_outer_bank(&self) -> usize {
        (((self.mapper227_latch >> 8) as usize & 0x01) << 2)
            | ((self.mapper227_latch >> 5) as usize & 0x03)
    }

    fn mapper227_inner_bank(&self) -> usize {
        (self.mapper227_latch as usize >> 2) & 0x07
    }

    /// Mapper 227: address-latched multicart with UNROM-like and NROM-like
    /// modes over a 1 MiB PRG ROM and fixed 8 KiB CHR-RAM.
    pub(in crate::cartridge) fn read_prg_mapper227(&self, addr: u16) -> u8 {
        if self.prg_rom.is_empty() {
            return 0;
        }

        let outer_base = self.mapper227_outer_bank() * 8;
        let inner_bank = self.mapper227_inner_bank();
        let nrom_mode = self.mapper227_latch & 0x0080 != 0;
        let mode_32k = self.mapper227_latch & 0x0001 != 0;
        let fixed_bank = if self.mapper227_latch & 0x0200 != 0 {
            7
        } else {
            0
        };
        let upper_half = addr >= 0xC000;
        let bank_count = (self.prg_rom.len() / 0x4000).max(1);

        let bank = if nrom_mode {
            if mode_32k {
                outer_base + (inner_bank & !1) + usize::from(upper_half)
            } else {
                outer_base + inner_bank
            }
        } else if upper_half {
            outer_base + fixed_bank
        } else if mode_32k {
            outer_base + (inner_bank & !1)
        } else {
            outer_base + inner_bank
        };

        let offset = (bank % bank_count) * 0x4000 + ((addr - 0x8000) as usize & 0x3FFF);
        self.prg_rom[offset % self.prg_rom.len()]
    }

    pub(in crate::cartridge) fn sync_mapper234_state(&mut self) {
        let prg_bank_count = (self.prg_rom.len() / 0x8000).max(1);
        let chr_bank_count = (self.chr_rom.len() / 0x2000).max(1);

        self.mirroring = if self.mapper234_reg0 & 0x80 != 0 {
            Mirroring::Horizontal
        } else {
            Mirroring::Vertical
        };

        let (prg_bank, chr_bank) = if self.mapper234_reg0 & 0x40 != 0 {
            let outer = ((self.mapper234_reg0 >> 1) as usize) & 0x07;
            let prg = (outer << 1) | ((self.mapper234_reg1 as usize) & 0x01);
            let chr = (outer << 3) | (((self.mapper234_reg1 >> 4) as usize) & 0x07);
            (prg, chr)
        } else {
            let outer = (self.mapper234_reg0 as usize) & 0x0F;
            let prg = outer;
            let chr = (outer << 2) | (((self.mapper234_reg1 >> 4) as usize) & 0x03);
            (prg, chr)
        };

        self.prg_bank = (prg_bank % prg_bank_count) as u8;
        self.chr_bank = (chr_bank % chr_bank_count) as u8;
    }

    pub(in crate::cartridge) fn apply_mapper234_value(&mut self, addr: u16, value: u8) {
        match addr {
            0xFF80..=0xFF9F => {
                if self.mapper234_reg0 & 0x3F == 0 {
                    self.mapper234_reg0 = value;
                    self.sync_mapper234_state();
                }
            }
            0xFFE8..=0xFFF7 => {
                self.mapper234_reg1 = value;
                self.sync_mapper234_state();
            }
            _ => {}
        }
    }

    /// NROM PRG read - 16KB/32KB mirroring (shared by Mapper 0/3/87)
    pub(in crate::cartridge) fn read_prg_nrom(&self, rom_addr: u16) -> u8 {
        let len = self.prg_rom.len();
        if len == 16384 {
            // 16KB PRG: Mirror at 0xC000
            self.prg_rom[(rom_addr & 0x3FFF) as usize]
        } else {
            // 32KB PRG: Direct mapping
            self.prg_rom[(rom_addr & 0x7FFF) as usize]
        }
    }

    /// Mapper 200: switchable 16KB PRG bank mirrored into both CPU halves.
    pub(in crate::cartridge) fn read_prg_mapper200(&self, addr: u16) -> u8 {
        if self.prg_rom.is_empty() {
            return 0;
        }

        let bank_count = (self.prg_rom.len() / 0x4000).max(1);
        let bank = (self.prg_bank as usize) % bank_count;
        let offset = bank * 0x4000 + ((addr - 0x8000) as usize & 0x3FFF);
        self.prg_rom[offset % self.prg_rom.len()]
    }

    /// Mapper 229: usually mirrors a single 16KB PRG bank into both CPU
    /// halves, except bank 0 behaves like NROM-256 with banks 0 and 1.
    pub(in crate::cartridge) fn read_prg_mapper229(&self, addr: u16) -> u8 {
        if self.prg_rom.is_empty() {
            return 0;
        }

        let bank_count = (self.prg_rom.len() / 0x4000).max(1);
        let bank = (self.prg_bank as usize) % bank_count;
        let selected_bank = if bank == 0 && addr >= 0xC000 && bank_count > 1 {
            1
        } else {
            bank
        };
        let offset = selected_bank * 0x4000 + ((addr - 0x8000) as usize & 0x3FFF);
        self.prg_rom[offset % self.prg_rom.len()]
    }

    /// Mapper 221: one outer 128KB/1MB bank plus a 3-bit inner bank feeding
    /// one of three PRG layouts: mirrored 16KB, paired 32KB, or UNROM.
    pub(in crate::cartridge) fn read_prg_mapper221(&self, addr: u16) -> u8 {
        if self.prg_rom.is_empty() {
            return 0;
        }

        let bank_count = (self.prg_rom.len() / 0x4000).max(1);
        let outer_base = (self.mapper221_outer_bank as usize) * 8;
        let inner_bank = (self.prg_bank as usize) & 0x07;
        let bank = match self.mapper221_mode {
            0 => outer_base + inner_bank,
            1 => outer_base + (inner_bank & !1) + usize::from(addr >= 0xC000),
            _ => {
                if addr >= 0xC000 {
                    outer_base + 7
                } else {
                    outer_base + inner_bank
                }
            }
        } % bank_count;
        let offset = bank * 0x4000 + ((addr - 0x8000) as usize & 0x3FFF);
        self.prg_rom[offset % self.prg_rom.len()]
    }

    /// Mapper 231: lower 16KB uses the latched bank with bit 0 forced low,
    /// while upper 16KB uses the full latched 5-bit bank.
    pub(in crate::cartridge) fn read_prg_mapper231(&self, addr: u16) -> u8 {
        if self.prg_rom.is_empty() {
            return 0;
        }

        let bank_count = (self.prg_rom.len() / 0x4000).max(1);
        let bank = if addr < 0xC000 {
            (self.prg_bank as usize) & 0x1E
        } else {
            self.prg_bank as usize
        } % bank_count;
        let offset = bank * 0x4000 + ((addr - 0x8000) as usize & 0x3FFF);
        self.prg_rom[offset % self.prg_rom.len()]
    }

    /// Mapper 233: Disch-notes 42-in-1 board with a 5-bit page register, a
    /// PRG mode bit selecting mirrored 16KB or paired 32KB, and a custom
    /// "three-screen lower" mirroring mode.
    pub(in crate::cartridge) fn read_prg_mapper233(&self, addr: u16) -> u8 {
        if self.prg_rom.is_empty() || addr < 0x8000 {
            return 0;
        }

        let bank_count = (self.prg_rom.len() / 0x4000).max(1);
        let bank = if self.mapper233_nrom128 {
            self.prg_bank as usize
        } else {
            ((self.prg_bank as usize) & !1) | usize::from(addr >= 0xC000)
        } % bank_count;
        let offset = bank * 0x4000 + ((addr - 0x8000) as usize & 0x3FFF);
        self.prg_rom[offset % self.prg_rom.len()]
    }

    pub(in crate::cartridge) fn write_prg_mapper233(&mut self, addr: u16, data: u8) {
        if addr < 0x8000 {
            return;
        }

        let bank = data & 0x1F;
        self.set_prg_bank(bank);
        self.set_chr_bank(bank);
        self.mapper233_nrom128 = data & 0x20 != 0;
        self.mirroring = match data >> 6 {
            0 => Mirroring::ThreeScreenLower,
            1 => Mirroring::Vertical,
            2 => Mirroring::Horizontal,
            _ => Mirroring::OneScreenUpper,
        };
    }

    /// Mapper 234: reads use the current 32KB PRG bank, while CPU reads from
    /// the high register area latch the value that was observed on the bus.
    pub(in crate::cartridge) fn read_prg_mapper234(&self, addr: u16) -> u8 {
        self.read_prg_axrom(addr)
    }

    /// Mapper 212: address latch choosing either a mirrored 16KB bank or a
    /// 32KB bank, plus a shared 8KB CHR bank and mirroring bit.
    pub(in crate::cartridge) fn read_prg_mapper212(&self, addr: u16) -> u8 {
        if self.prg_rom.is_empty() {
            return 0;
        }

        if self.mapper212_32k_mode {
            let bank_count = (self.prg_rom.len() / 0x8000).max(1);
            let bank = ((self.prg_bank as usize) >> 1) % bank_count;
            let offset = bank * 0x8000 + (addr - 0x8000) as usize;
            self.prg_rom[offset % self.prg_rom.len()]
        } else {
            let bank_count = (self.prg_rom.len() / 0x4000).max(1);
            let bank = (self.prg_bank as usize) % bank_count;
            let offset = bank * 0x4000 + ((addr - 0x8000) as usize & 0x3FFF);
            self.prg_rom[offset % self.prg_rom.len()]
        }
    }

    /// Mapper 226: multicart with a 7-bit PRG register and a mode bit
    /// selecting either 32KB PRG or mirrored 16KB PRG.
    pub(in crate::cartridge) fn read_prg_mapper226(&self, addr: u16) -> u8 {
        if self.prg_rom.is_empty() {
            return 0;
        }

        if self.mapper226_nrom128 {
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

    /// Mapper 230: reset-driven multicart with a dedicated Contra mode on
    /// the first 128 KiB PRG chip and a 512 KiB multicart mode on the second.
    pub(in crate::cartridge) fn read_prg_mapper230(&self, addr: u16) -> u8 {
        if self.prg_rom.is_empty() || addr < 0x8000 {
            return 0;
        }

        let offset_in_bank = (addr - 0x8000) as usize & 0x3FFF;
        if self.mapper230_contra_mode {
            let chip0_len = self.prg_rom.len().min(0x20000);
            let bank_count = (chip0_len / 0x4000).max(1);
            let bank = if addr < 0xC000 {
                (self.prg_bank as usize & 0x07) % bank_count
            } else {
                bank_count.saturating_sub(1)
            };
            return self.prg_rom[bank * 0x4000 + offset_in_bank];
        }

        let chip_base = 0x20000.min(self.prg_rom.len());
        let chip1_len = self.prg_rom.len().saturating_sub(chip_base);
        if chip1_len == 0 {
            return 0;
        }

        let bank_count = (chip1_len / 0x4000).max(1);
        let page = self.prg_bank as usize & 0x1F;
        let bank = if self.mapper230_nrom128 {
            page % bank_count
        } else {
            ((page & !1) | usize::from(addr >= 0xC000)) % bank_count
        };
        let offset = chip_base + bank * 0x4000 + offset_in_bank;
        self.prg_rom[offset % self.prg_rom.len()]
    }

    /// Mapper 235: multicart that either maps a full 32KB page or mirrors one
    /// 16KB half into both CPU slots. Invalid ROM socket selections read back
    /// as open bus.
    pub(in crate::cartridge) fn read_prg_mapper235(&self, addr: u16) -> u8 {
        if self.prg_rom.is_empty() {
            return 0xFF;
        }

        if self.prg_bank == u8::MAX {
            return 0xFF;
        }

        if self.mapper235_nrom128 {
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

    /// Mapper 202: address latch choosing either a mirrored 16KB bank or a
    /// 32KB bank, plus a shared 8KB CHR bank and mirroring bit.
    pub(in crate::cartridge) fn read_prg_mapper202(&self, addr: u16) -> u8 {
        if self.prg_rom.is_empty() {
            return 0;
        }

        if self.mapper202_32k_mode {
            let bank_count = (self.prg_rom.len() / 0x8000).max(1);
            let bank = (self.prg_bank as usize) % bank_count;
            let offset = bank * 0x8000 + (addr - 0x8000) as usize;
            self.prg_rom[offset % self.prg_rom.len()]
        } else {
            let bank_count = (self.prg_rom.len() / 0x4000).max(1);
            let bank = (self.prg_bank as usize) % bank_count;
            let offset = bank * 0x4000 + ((addr - 0x8000) as usize & 0x3FFF);
            self.prg_rom[offset % self.prg_rom.len()]
        }
    }

    /// Mapper 225/255: 16KB mirrored or 32KB PRG banks selected from the
    /// latched address, with a shared high bit applied to both PRG and CHR.
    pub(in crate::cartridge) fn read_prg_mapper225(&self, addr: u16) -> u8 {
        if self.prg_rom.is_empty() {
            return 0;
        }

        if self.mapper225_nrom128 {
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

    fn mapper228_chip_base(&self) -> Option<usize> {
        const CHIP_SIZE: usize = 0x80000;

        let chip = self.mapper228_chip_select as usize;
        let chip_count = self.prg_rom.len() / CHIP_SIZE;
        if chip_count == 3 {
            match chip {
                0 | 1 => Some(chip * CHIP_SIZE),
                2 => None,
                3 => Some(2 * CHIP_SIZE),
                _ => None,
            }
        } else if chip < chip_count {
            Some(chip * CHIP_SIZE)
        } else {
            None
        }
    }

    /// Mapper 228: Action 52 / Cheetahmen II. Selects one PRG chip and either
    /// a mirrored 16KB bank or a paired 32KB bank inside that chip.
    pub(in crate::cartridge) fn read_prg_mapper228(&self, addr: u16) -> u8 {
        const CHIP_SIZE: usize = 0x80000;

        let Some(chip_base) = self.mapper228_chip_base() else {
            return 0;
        };

        let remaining = self.prg_rom.len().saturating_sub(chip_base);
        let chip_len = CHIP_SIZE.min(remaining).max(0x4000);
        let bank_count = (chip_len / 0x4000).max(1);
        let offset_in_bank = (addr - 0x8000) as usize & 0x3FFF;
        let bank = if self.mapper228_nrom128 {
            (self.prg_bank as usize) % bank_count
        } else {
            (((self.prg_bank as usize) & !1) | usize::from(addr >= 0xC000)) % bank_count
        };
        let offset = chip_base + bank * 0x4000 + offset_in_bank;
        self.prg_rom.get(offset).copied().unwrap_or(0)
    }

    /// Mapper 242: Waixing multicart with several PRG modes and optional
    /// CHR-RAM write protection driven by the address latch.
    pub(in crate::cartridge) fn read_prg_mapper242(&self, addr: u16) -> u8 {
        if self.prg_rom.is_empty() {
            return 0;
        }

        let reg = self.mapper242_latch as usize;
        let inner_bank = (reg >> 2) & 0x07;
        let outer_bank = (reg >> 5) & 0x03;
        let fixed_bank = if reg & 0x0200 != 0 { 7 } else { 0 };
        let nrom_mode = reg & 0x0080 != 0;
        let mode_32k = reg & 0x0001 != 0;
        let upper_half = addr >= 0xC000;

        let (chip_base, bank_count, bank16) = if self.prg_rom.len() > 0x80000 && reg & 0x0400 == 0 {
            let chip_base = 0x80000;
            let bank_count = (self.prg_rom.len().saturating_sub(chip_base) / 0x4000).max(1);
            let bank16 = if nrom_mode {
                if mode_32k {
                    (inner_bank & !1) | usize::from(upper_half)
                } else {
                    inner_bank
                }
            } else if upper_half {
                fixed_bank
            } else if mode_32k {
                inner_bank & !1
            } else {
                inner_bank
            };
            (chip_base, bank_count, bank16)
        } else {
            let chip_base = 0;
            let bank_count = (self.prg_rom.len() / 0x4000).max(1);
            let bank16 = outer_bank * 8
                + if nrom_mode {
                    if mode_32k {
                        (inner_bank & !1) | usize::from(upper_half)
                    } else {
                        inner_bank
                    }
                } else if upper_half {
                    fixed_bank
                } else if mode_32k {
                    inner_bank & !1
                } else {
                    inner_bank
                };
            (chip_base, bank_count, bank16)
        };

        let offset_in_bank = (addr - 0x8000) as usize & 0x3FFF;
        let bank = bank16 % bank_count;
        let offset = chip_base + bank * 0x4000 + offset_in_bank;
        self.prg_rom[offset % self.prg_rom.len()]
    }

    /// NROM CHR read - 8KB CHR ROM direct mapping
    pub(in crate::cartridge) fn read_chr_nrom(&self, addr: u16) -> u8 {
        let chr_addr = (addr & 0x1FFF) as usize;
        if chr_addr < self.chr_rom.len() {
            self.chr_rom[chr_addr]
        } else {
            0
        }
    }

    /// NROM CHR write
    pub(in crate::cartridge) fn write_chr_nrom(&mut self, addr: u16, data: u8) {
        let chr_addr = (addr & 0x1FFF) as usize;
        if chr_addr < self.chr_rom.len() {
            self.chr_rom[chr_addr] = data;
        }
    }

    pub(in crate::cartridge) fn read_chr_mapper221(&self, addr: u16) -> u8 {
        let chr_addr = (addr & 0x1FFF) as usize;
        if chr_addr < self.chr_ram.len() {
            self.chr_ram[chr_addr]
        } else {
            0
        }
    }

    pub(in crate::cartridge) fn write_chr_mapper221(&mut self, addr: u16, data: u8) {
        if self.mapper221_chr_write_protect {
            return;
        }

        let chr_addr = (addr & 0x1FFF) as usize;
        if chr_addr < self.chr_ram.len() {
            self.chr_ram[chr_addr] = data;
        }
    }

    pub(in crate::cartridge) fn read_chr_mapper231(&self, addr: u16) -> u8 {
        let chr_addr = (addr & 0x1FFF) as usize;
        if chr_addr < self.chr_ram.len() {
            self.chr_ram[chr_addr]
        } else {
            0
        }
    }

    pub(in crate::cartridge) fn write_chr_mapper231(&mut self, addr: u16, data: u8) {
        let chr_addr = (addr & 0x1FFF) as usize;
        if chr_addr < self.chr_ram.len() {
            self.chr_ram[chr_addr] = data;
        }
    }

    /// Mapper 13 (CPROM): fixed 4KB CHR-RAM at $0000 and a switchable
    /// 4KB CHR-RAM page at $1000 selected by bits 0-1.
    pub(in crate::cartridge) fn write_prg_cprom(&mut self, addr: u16, data: u8) {
        if addr >= 0x8000 {
            let rom_value = if self.prg_rom.is_empty() {
                0xFF
            } else {
                self.prg_rom[(addr - 0x8000) as usize % self.prg_rom.len()]
            };
            self.chr_bank = (data & rom_value) & 0x03;
        }
    }

    /// Mapper 200: simple address-latch NROM-128 multicart. iNES does not
    /// encode the NES 2.0 submapper, so this uses the common submapper 0
    /// wiring where A0-A3 select both PRG/CHR bank and A3 controls mirroring.
    pub(in crate::cartridge) fn write_prg_mapper200(&mut self, addr: u16) {
        if addr >= 0x8000 {
            let bank = (addr & 0x000F) as usize;
            let prg_bank_count = (self.prg_rom.len() / 0x4000).max(1);
            let chr_bank_count = (self.chr_rom.len() / 0x2000).max(1);

            self.prg_bank = (bank % prg_bank_count) as u8;
            self.chr_bank = (bank % chr_bank_count) as u8;
            self.mirroring = if addr & 0x0008 != 0 {
                Mirroring::Horizontal
            } else {
                Mirroring::Vertical
            };
        }
    }

    /// Mapper 201: address-latch NROM-256 multicart where the low byte of the
    /// write address selects both the 32KB PRG bank and the 8KB CHR bank.
    pub(in crate::cartridge) fn write_prg_mapper201(&mut self, addr: u16) {
        if addr >= 0x8000 {
            let bank = (addr & 0x00FF) as usize;
            let prg_bank_count = (self.prg_rom.len() / 0x8000).max(1);
            let chr_bank_count = (self.chr_rom.len() / 0x2000).max(1);

            self.prg_bank = (bank % prg_bank_count) as u8;
            self.chr_bank = (bank % chr_bank_count) as u8;
        }
    }

    /// Mapper 203: a data latch where bits 7-2 select a mirrored 16KB PRG
    /// bank and bits 1-0 select an 8KB CHR bank.
    pub(in crate::cartridge) fn write_prg_mapper203(&mut self, addr: u16, data: u8) {
        if addr >= 0x8000 {
            let prg_bank_count = (self.prg_rom.len() / 0x4000).max(1);
            let chr_bank_count = (self.chr_rom.len() / 0x2000).max(1);
            self.prg_bank = (((data >> 2) as usize) % prg_bank_count) as u8;
            self.chr_bank = ((data as usize & 0x03) % chr_bank_count) as u8;
        }
    }

    pub(in crate::cartridge) fn write_prg_mapper227(&mut self, addr: u16) {
        if addr >= 0x8000 {
            self.mapper227_latch = addr & 0x07FF;
            self.mirroring = if addr & 0x0002 != 0 {
                Mirroring::Horizontal
            } else {
                Mirroring::Vertical
            };
            self.prg_bank = (self.mapper227_outer_bank() * 8 + self.mapper227_inner_bank()) as u8;
        }
    }

    /// Mapper 234: writes behave like bus-conflicted reads of the same ROM
    /// location, so the latched value is the written byte AND the ROM byte.
    pub(in crate::cartridge) fn write_prg_mapper234(&mut self, addr: u16, data: u8) {
        if addr >= 0x8000 {
            let effective = data & self.read_prg_mapper234(addr);
            self.apply_mapper234_value(addr, effective);
        }
    }

    /// Mapper 235 latches address bits instead of written data.
    pub(in crate::cartridge) fn write_prg_mapper235(&mut self, addr: u16, _data: u8) {
        if addr >= 0x8000 {
            let chip_bits = ((addr >> 8) & 0x03) as u8;
            let page = (addr & 0x001F) as usize;
            let chip_base = usize::from((chip_bits >> 1) & 0x01) * (0x100000 / 0x4000);
            let bank16 = chip_base + page * 2 + usize::from(addr & 0x1000 != 0);
            let bank_count = (self.prg_rom.len() / 0x4000).max(1);

            self.mapper235_nrom128 = addr & 0x0800 != 0;
            self.mirroring = if addr & 0x0400 != 0 {
                Mirroring::OneScreenLower
            } else if addr & 0x2000 != 0 {
                Mirroring::Vertical
            } else {
                Mirroring::Horizontal
            };

            self.prg_bank = if chip_bits & 0x01 != 0 || bank16 >= bank_count {
                u8::MAX
            } else {
                bank16 as u8
            };
        }
    }

    pub(in crate::cartridge) fn write_chr_mapper227(&mut self, addr: u16, data: u8) {
        let write_protected = !self.has_battery && self.mapper227_latch & 0x0080 != 0;
        if !write_protected {
            self.write_chr_uxrom(addr, data);
        }
    }

    pub(in crate::cartridge) fn read_prg_low_mapper225(&self, addr: u16) -> u8 {
        if (0x5800..=0x5FFF).contains(&addr) && !self.prg_ram.is_empty() {
            self.prg_ram[(addr as usize) & 0x03] & 0x0F
        } else {
            0
        }
    }

    /// Mapper 225/255: bank bits come entirely from the write address.
    /// Mapper 225 optionally exposes four 4-bit latches at $5800-$5FFF.
    pub(in crate::cartridge) fn write_prg_mapper225(&mut self, addr: u16, data: u8) {
        if (0x5800..=0x5FFF).contains(&addr) && self.mapper == 225 && !self.prg_ram.is_empty() {
            self.prg_ram[(addr as usize) & 0x03] = data & 0x0F;
            return;
        }

        if addr >= 0x8000 {
            let high_bit = if addr & 0x4000 != 0 { 0x40 } else { 0x00 };
            let prg_bank_count = (self.prg_rom.len() / 0x4000).max(1);
            let chr_bank_count = (self.chr_rom.len() / 0x2000).max(1);

            self.prg_bank = ((((addr as usize >> 6) & 0x3F) | high_bit) % prg_bank_count) as u8;
            self.chr_bank = ((((addr as usize) & 0x3F) | high_bit) % chr_bank_count) as u8;
            self.mapper225_nrom128 = addr & 0x1000 != 0;
            self.mirroring = if addr & 0x2000 != 0 {
                Mirroring::Horizontal
            } else {
                Mirroring::Vertical
            };
        }
    }

    /// Mapper 202: bits A3-A1 select the bank, A0 selects mirroring, and the
    /// A3/A0 combination decides whether PRG is mirrored 16KB or direct 32KB.
    pub(in crate::cartridge) fn write_prg_mapper202(&mut self, addr: u16) {
        if addr >= 0x8000 {
            let bank = ((addr >> 1) & 0x07) as usize;
            let prg_bank_count_16k = (self.prg_rom.len() / 0x4000).max(1);
            let chr_bank_count = (self.chr_rom.len() / 0x2000).max(1);

            self.prg_bank = (bank % prg_bank_count_16k) as u8;
            self.chr_bank = (bank % chr_bank_count) as u8;
            self.mirroring = if addr & 0x0001 != 0 {
                Mirroring::Horizontal
            } else {
                Mirroring::Vertical
            };
            self.mapper202_32k_mode = (addr & 0x0009) == 0x0009;
        }
    }

    /// Mapper 228: select mirroring, a PRG chip, a 16KB PRG bank, and a 6-bit
    /// CHR bank. The low CHR bits come from the written data.
    pub(in crate::cartridge) fn write_prg_mapper228(&mut self, addr: u16, data: u8) {
        if addr >= 0x8000 {
            let chr_bank_count = (self.chr_rom.len() / 0x2000).max(1);

            self.mapper228_chip_select = ((addr >> 11) & 0x03) as u8;
            self.prg_bank = ((addr >> 6) & 0x1F) as u8;
            self.mapper228_nrom128 = addr & 0x0020 != 0;
            self.chr_bank =
                ((((addr & 0x000F) << 2) | u16::from(data & 0x03)) as usize % chr_bank_count) as u8;
            self.mirroring = if addr & 0x2000 != 0 {
                Mirroring::Horizontal
            } else {
                Mirroring::Vertical
            };
        }
    }

    /// Mapper 229: address latch controlling a shared PRG/CHR bank number
    /// and nametable mirroring.
    pub(in crate::cartridge) fn write_prg_mapper229(&mut self, addr: u16) {
        if addr >= 0x8000 {
            let bank = (addr & 0x001F) as usize;
            let prg_bank_count = (self.prg_rom.len() / 0x4000).max(1);
            let chr_bank_count = (self.chr_rom.len() / 0x2000).max(1);

            self.prg_bank = (bank % prg_bank_count) as u8;
            self.chr_bank = (bank % chr_bank_count) as u8;
            self.mirroring = if addr & 0x0020 != 0 {
                Mirroring::Horizontal
            } else {
                Mirroring::Vertical
            };
        }
    }

    /// Mapper 221: outer register comes from the write address in
    /// $8000-$BFFF and chooses mirroring plus one of the three PRG modes.
    pub(in crate::cartridge) fn write_prg_mapper221(&mut self, addr: u16) {
        if (0x8000..=0xBFFF).contains(&addr) {
            self.mapper221_outer_bank = (((addr >> 2) & 0x07) | (((addr >> 9) & 0x01) << 3)) as u8;
            self.mapper221_mode = if addr & 0x0002 == 0 {
                0
            } else if addr & 0x0100 == 0 {
                1
            } else {
                2
            };
            self.mirroring = if addr & 0x0001 != 0 {
                Mirroring::Horizontal
            } else {
                Mirroring::Vertical
            };
        } else if addr >= 0xC000 {
            self.prg_bank = (addr & 0x0007) as u8;
            self.mapper221_chr_write_protect = addr & 0x0008 != 0;
        }
    }

    /// Mapper 231: address latch controlling horizontal/vertical mirroring
    /// and a 5-bit PRG bank where bit 5 of the address becomes the low bank bit.
    pub(in crate::cartridge) fn write_prg_mapper231(&mut self, addr: u16) {
        if addr >= 0x8000 {
            self.prg_bank = ((addr & 0x001E) | ((addr >> 5) & 0x01)) as u8;
            self.mirroring = if addr & 0x0080 != 0 {
                Mirroring::Horizontal
            } else {
                Mirroring::Vertical
            };
        }
    }

    /// Mapper 212: write address selects PRG/CHR bank, mirroring, and whether
    /// the CPU sees a 16KB mirrored bank or a 32KB bank.
    pub(in crate::cartridge) fn write_prg_mapper212(&mut self, addr: u16) {
        if addr >= 0x8000 {
            let bank = (addr & 0x0007) as usize;
            let chr_bank_count = (self.chr_rom.len() / 0x2000).max(1);
            let prg_bank_count = (self.prg_rom.len() / 0x4000).max(1);

            self.prg_bank = (bank % prg_bank_count) as u8;
            self.chr_bank = (bank % chr_bank_count) as u8;
            self.mirroring = if addr & 0x0008 != 0 {
                Mirroring::Horizontal
            } else {
                Mirroring::Vertical
            };
            self.mapper212_32k_mode = addr & 0x4000 != 0;
        }
    }

    pub(in crate::cartridge) fn write_prg_mapper242(&mut self, addr: u16) {
        if addr >= 0x8000 {
            self.mapper242_latch = addr & 0x07FF;
            self.mirroring = if addr & 0x0002 != 0 {
                Mirroring::Horizontal
            } else {
                Mirroring::Vertical
            };
        }
    }

    /// Mapper 226 uses two write addresses: even selects the low PRG bits,
    /// mirroring, and mode; odd selects the high PRG bit.
    pub(in crate::cartridge) fn write_prg_mapper226(&mut self, addr: u16, data: u8) {
        if addr >= 0x8000 {
            let bank_count_16k = (self.prg_rom.len() / 0x4000).max(1);
            if addr & 0x0001 == 0 {
                let low_bits = (data & 0x1F) as usize | (((data >> 7) as usize) << 5);
                let high_bit = self.prg_bank as usize & 0x40;
                self.prg_bank = ((high_bit | low_bits) % bank_count_16k) as u8;
                self.mapper226_nrom128 = data & 0x20 != 0;
                self.mirroring = if data & 0x40 != 0 {
                    Mirroring::Vertical
                } else {
                    Mirroring::Horizontal
                };
            } else {
                let low_bits = self.prg_bank as usize & 0x3F;
                let high_bit = ((data & 0x01) as usize) << 6;
                self.prg_bank = ((high_bit | low_bits) % bank_count_16k) as u8;
            }
        }
    }

    pub(in crate::cartridge) fn write_prg_mapper230(&mut self, addr: u16, data: u8) {
        if addr < 0x8000 {
            return;
        }

        if self.mapper230_contra_mode {
            self.prg_bank = data & 0x07;
            self.mirroring = Mirroring::Vertical;
            return;
        }

        self.prg_bank = data & 0x1F;
        self.mapper230_nrom128 = data & 0x20 != 0;
        self.mirroring = if data & 0x40 != 0 {
            Mirroring::Vertical
        } else {
            Mirroring::Horizontal
        };
    }

    pub(in crate::cartridge) fn read_chr_cprom(&self, addr: u16) -> u8 {
        if self.chr_rom.is_empty() {
            return 0;
        }

        let bank = if addr < 0x1000 {
            0usize
        } else {
            (self.chr_bank as usize) % (self.chr_rom.len() / 0x1000).max(1)
        };
        let offset = if addr < 0x1000 {
            addr as usize
        } else {
            (addr as usize) - 0x1000
        };
        let chr_addr = bank * 0x1000 + offset;
        self.chr_rom[chr_addr % self.chr_rom.len()]
    }

    pub(in crate::cartridge) fn write_chr_cprom(&mut self, addr: u16, data: u8) {
        if self.chr_rom.is_empty() {
            return;
        }

        let bank_count = (self.chr_rom.len() / 0x1000).max(1);
        let bank = if addr < 0x1000 {
            0usize
        } else {
            (self.chr_bank as usize) % bank_count
        };
        let offset = if addr < 0x1000 {
            addr as usize
        } else {
            (addr as usize) - 0x1000
        };
        let chr_len = self.chr_rom.len();
        let chr_addr = bank * 0x1000 + offset;
        self.chr_rom[chr_addr % chr_len] = data;
    }

    pub(in crate::cartridge) fn write_chr_mapper242(&mut self, addr: u16, data: u8) {
        if self.mapper242_latch & 0x0080 == 0 {
            self.write_chr_nrom(addr, data);
        }
    }
}
