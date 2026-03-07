use super::super::{Cartridge, Mirroring};

impl Cartridge {
    /// Mapper 76/88/95/154/206: Namco 108 family with two switchable 8KB PRG
    /// banks and the last two 8KB banks fixed.
    pub(in crate::cartridge) fn read_prg_namco108(&self, addr: u16) -> u8 {
        if let Some(ref mmc3) = self.mmc3 {
            let num_8k_banks = self.prg_rom.len() / 0x2000;
            if num_8k_banks < 2 {
                return 0;
            }
            let bank_mask = num_8k_banks - 1;
            let second_last = (num_8k_banks - 2) & bank_mask;
            let last = (num_8k_banks - 1) & bank_mask;

            let (bank, offset) = match addr {
                0x8000..=0x9FFF => ((mmc3.bank_registers[6] as usize) & bank_mask, 0x8000),
                0xA000..=0xBFFF => ((mmc3.bank_registers[7] as usize) & bank_mask, 0xA000),
                0xC000..=0xDFFF => (second_last, 0xC000),
                0xE000..=0xFFFF => (last, 0xE000),
                _ => return 0,
            };

            let rom_addr = bank * 0x2000 + (addr - offset) as usize;
            if rom_addr < self.prg_rom.len() {
                self.prg_rom[rom_addr]
            } else {
                0
            }
        } else {
            0
        }
    }

    pub(in crate::cartridge) fn write_prg_namco108(&mut self, addr: u16, data: u8) {
        if self.mapper == 154 && addr >= 0x8000 {
            self.mirroring = if data & 0x40 != 0 {
                Mirroring::OneScreenUpper
            } else {
                Mirroring::OneScreenLower
            };
        }

        let mut mapper95_banks = None;
        if let Some(ref mut mmc3) = self.mmc3 {
            match addr {
                0x8000..=0x9FFF if (addr & 1) == 0 => {
                    mmc3.bank_select = data & 0x07;
                }
                0x8000..=0x9FFF => {
                    let reg = (mmc3.bank_select & 0x07) as usize;
                    mmc3.bank_registers[reg] =
                        Self::mask_namco108_bank_data(self.mapper, reg, data);
                    if self.mapper == 95 && (reg == 0 || reg == 1) {
                        mapper95_banks = Some(mmc3.bank_registers);
                    }
                }
                _ => {}
            }
        }

        if let Some(bank_registers) = mapper95_banks {
            self.update_mapper95_mirroring(&bank_registers);
        }
    }

    pub(in crate::cartridge) fn read_chr_namco108(&self, addr: u16) -> u8 {
        if let Some(ref mmc3) = self.mmc3 {
            let chr_data = if !self.chr_ram.is_empty() {
                &self.chr_ram
            } else {
                &self.chr_rom
            };
            let num_1k_banks = chr_data.len() / 0x0400;
            if num_1k_banks == 0 {
                return 0;
            }
            let bank_mask = num_1k_banks - 1;
            let bank = self.resolve_chr_bank_namco108(addr, bank_mask, &mmc3.bank_registers);
            let chr_addr = bank * 0x0400 + (addr as usize & 0x03FF);
            if chr_addr < chr_data.len() {
                chr_data[chr_addr]
            } else {
                0
            }
        } else {
            0
        }
    }

    pub(in crate::cartridge) fn write_chr_namco108(&mut self, addr: u16, data: u8) {
        let bank_registers = if let Some(ref mmc3) = self.mmc3 {
            mmc3.bank_registers
        } else {
            return;
        };

        if !self.chr_ram.is_empty() {
            let num_1k_banks = self.chr_ram.len() / 0x0400;
            if num_1k_banks == 0 {
                return;
            }
            let bank_mask = num_1k_banks - 1;
            let bank = self.resolve_chr_bank_namco108(addr, bank_mask, &bank_registers);
            let chr_addr = bank * 0x0400 + (addr as usize & 0x03FF);
            if chr_addr < self.chr_ram.len() {
                self.chr_ram[chr_addr] = data;
            }
        } else if !self.chr_rom.is_empty() {
            let num_1k_banks = self.chr_rom.len() / 0x0400;
            if num_1k_banks == 0 {
                return;
            }
            let bank_mask = num_1k_banks - 1;
            let bank = self.resolve_chr_bank_namco108(addr, bank_mask, &bank_registers);
            let chr_addr = bank * 0x0400 + (addr as usize & 0x03FF);
            if chr_addr < self.chr_rom.len() {
                self.chr_rom[chr_addr] = data;
            }
        }
    }

    fn mask_namco108_bank_data(mapper: u8, reg: usize, data: u8) -> u8 {
        match reg {
            0 | 1 => {
                if mapper == 95 {
                    data & 0x3F
                } else {
                    data & 0x3E
                }
            }
            2..=5 => {
                if mapper == 76 {
                    data & 0x3F
                } else {
                    data & 0x3F
                }
            }
            6 | 7 => data & 0x0F,
            _ => 0,
        }
    }

    fn update_mapper95_mirroring(&mut self, bank_registers: &[u8; 8]) {
        let lower = bank_registers[0] & 0x20 != 0;
        let upper = bank_registers[1] & 0x20 != 0;
        self.mirroring = match (lower, upper) {
            (false, false) => Mirroring::OneScreenLower,
            (true, true) => Mirroring::OneScreenUpper,
            (false, true) => Mirroring::Horizontal,
            (true, false) => Mirroring::HorizontalSwapped,
        };
    }

    fn resolve_chr_bank_namco108(
        &self,
        addr: u16,
        bank_mask: usize,
        bank_registers: &[u8; 8],
    ) -> usize {
        let slot = ((addr >> 10) & 0x07) as usize;

        match self.mapper {
            76 => {
                let bank_2k = bank_registers[2 + (slot / 2)] as usize;
                ((bank_2k << 1) | (slot & 1)) & bank_mask
            }
            88 | 95 | 154 => match slot {
                0 => (bank_registers[0] as usize & !1) & bank_mask,
                1 => ((bank_registers[0] as usize & !1) | 1) & bank_mask,
                2 => (bank_registers[1] as usize & !1) & bank_mask,
                3 => ((bank_registers[1] as usize & !1) | 1) & bank_mask,
                4 => {
                    let high = if self.mapper == 88 || self.mapper == 154 {
                        0x40
                    } else {
                        0
                    };
                    ((bank_registers[2] as usize) | high) & bank_mask
                }
                5 => {
                    let high = if self.mapper == 88 || self.mapper == 154 {
                        0x40
                    } else {
                        0
                    };
                    ((bank_registers[3] as usize) | high) & bank_mask
                }
                6 => {
                    let high = if self.mapper == 88 || self.mapper == 154 {
                        0x40
                    } else {
                        0
                    };
                    ((bank_registers[4] as usize) | high) & bank_mask
                }
                7 => {
                    let high = if self.mapper == 88 || self.mapper == 154 {
                        0x40
                    } else {
                        0
                    };
                    ((bank_registers[5] as usize) | high) & bank_mask
                }
                _ => 0,
            },
            _ => match slot {
                0 => (bank_registers[0] as usize & !1) & bank_mask,
                1 => ((bank_registers[0] as usize & !1) | 1) & bank_mask,
                2 => (bank_registers[1] as usize & !1) & bank_mask,
                3 => ((bank_registers[1] as usize & !1) | 1) & bank_mask,
                4 => (bank_registers[2] as usize) & bank_mask,
                5 => (bank_registers[3] as usize) & bank_mask,
                6 => (bank_registers[4] as usize) & bank_mask,
                7 => (bank_registers[5] as usize) & bank_mask,
                _ => 0,
            },
        }
    }
}
