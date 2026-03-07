use std::cell::Cell;

use super::super::{Cartridge, Mirroring};

const MAPPER208_PROTECTION_LUT: [u8; 256] = [
    0x59, 0x59, 0x59, 0x59, 0x59, 0x59, 0x59, 0x59, 0x59, 0x49, 0x19, 0x09, 0x59, 0x49, 0x19, 0x09,
    0x59, 0x59, 0x59, 0x59, 0x59, 0x59, 0x59, 0x59, 0x51, 0x41, 0x11, 0x01, 0x51, 0x41, 0x11, 0x01,
    0x59, 0x59, 0x59, 0x59, 0x59, 0x59, 0x59, 0x59, 0x59, 0x49, 0x19, 0x09, 0x59, 0x49, 0x19, 0x09,
    0x59, 0x59, 0x59, 0x59, 0x59, 0x59, 0x59, 0x59, 0x51, 0x41, 0x11, 0x01, 0x51, 0x41, 0x11, 0x01,
    0x00, 0x10, 0x40, 0x50, 0x00, 0x10, 0x40, 0x50, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x08, 0x18, 0x48, 0x58, 0x08, 0x18, 0x48, 0x58, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x10, 0x40, 0x50, 0x00, 0x10, 0x40, 0x50, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x08, 0x18, 0x48, 0x58, 0x08, 0x18, 0x48, 0x58, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x59, 0x59, 0x59, 0x59, 0x59, 0x59, 0x59, 0x59, 0x58, 0x48, 0x18, 0x08, 0x58, 0x48, 0x18, 0x08,
    0x59, 0x59, 0x59, 0x59, 0x59, 0x59, 0x59, 0x59, 0x50, 0x40, 0x10, 0x00, 0x50, 0x40, 0x10, 0x00,
    0x59, 0x59, 0x59, 0x59, 0x59, 0x59, 0x59, 0x59, 0x58, 0x48, 0x18, 0x08, 0x58, 0x48, 0x18, 0x08,
    0x59, 0x59, 0x59, 0x59, 0x59, 0x59, 0x59, 0x59, 0x50, 0x40, 0x10, 0x00, 0x50, 0x40, 0x10, 0x00,
    0x01, 0x11, 0x41, 0x51, 0x01, 0x11, 0x41, 0x51, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x09, 0x19, 0x49, 0x59, 0x09, 0x19, 0x49, 0x59, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x01, 0x11, 0x41, 0x51, 0x01, 0x11, 0x41, 0x51, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x09, 0x19, 0x49, 0x59, 0x09, 0x19, 0x49, 0x59, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
];

#[derive(Debug, Clone)]
pub(in crate::cartridge) struct Mmc3 {
    pub(in crate::cartridge) bank_select: u8,
    pub(in crate::cartridge) bank_registers: [u8; 8],
    pub(in crate::cartridge) irq_latch: u8,
    pub(in crate::cartridge) irq_counter: u8,
    pub(in crate::cartridge) irq_reload: bool,
    pub(in crate::cartridge) irq_enabled: bool,
    pub(in crate::cartridge) irq_pending: Cell<bool>,
    pub(in crate::cartridge) prg_ram_enabled: bool,
    pub(in crate::cartridge) prg_ram_write_protect: bool,
}

impl Mmc3 {
    pub(in crate::cartridge) fn new() -> Self {
        Mmc3 {
            bank_select: 0,
            bank_registers: [0; 8],
            irq_latch: 0,
            irq_counter: 0,
            irq_reload: false,
            irq_enabled: false,
            irq_pending: Cell::new(false),
            prg_ram_enabled: true,
            prg_ram_write_protect: false,
        }
    }

    pub(in crate::cartridge) fn clock_irq_mut(&mut self) {
        let counter_was_zero = self.irq_counter == 0;
        if counter_was_zero || self.irq_reload {
            self.irq_counter = self.irq_latch;
            self.irq_reload = false;
        } else {
            self.irq_counter -= 1;
        }

        if self.irq_counter == 0 && self.irq_enabled {
            self.irq_pending.set(true);
        }
    }
}

impl Cartridge {
    fn mapper191_outer_bank_writable(&self) -> bool {
        self.chr_rom.len() > 0x20000
    }

    fn mapper191_effective_outer_bank(&self) -> usize {
        if self.mapper191_outer_bank_writable() {
            (self.mapper191_outer_bank & 0x03) as usize
        } else {
            3
        }
    }

    fn mapper195_mode_for_bank(raw_bank: usize) -> Option<u8> {
        match raw_bank {
            0x00..=0x03 => Some(0x82),
            0x0A..=0x0B => Some(0xC8),
            0x28..=0x2B => Some(0x80),
            0x46..=0x47 => Some(0xC0),
            0x4C..=0x4F => Some(0x88),
            0x64..=0x67 => Some(0x8A),
            0x7C..=0x7D => Some(0xC2),
            0xCA => Some(0xCA),
            _ => None,
        }
    }

    fn resolve_mapper195_chr_bank(&self, raw_bank: usize) -> (bool, usize) {
        match self.mapper195_mode {
            0x80 => {
                if (0x28..=0x2B).contains(&raw_bank) {
                    (true, raw_bank & 0x07)
                } else {
                    (false, raw_bank)
                }
            }
            0x82 => {
                if (0x00..=0x03).contains(&raw_bank) {
                    (true, raw_bank & 0x07)
                } else {
                    (false, raw_bank)
                }
            }
            0x88 => {
                if (0x4C..=0x4F).contains(&raw_bank) {
                    (true, raw_bank & 0x07)
                } else {
                    (false, raw_bank)
                }
            }
            0x8A => {
                if (0x64..=0x67).contains(&raw_bank) {
                    (true, raw_bank & 0x07)
                } else {
                    (false, raw_bank)
                }
            }
            0xC0 => {
                if (0x46..=0x47).contains(&raw_bank) {
                    (true, raw_bank & 0x07)
                } else {
                    (false, raw_bank)
                }
            }
            0xC2 => {
                if (0x7C..=0x7D).contains(&raw_bank) {
                    (true, raw_bank & 0x07)
                } else {
                    (false, raw_bank)
                }
            }
            0xC8 => {
                if (0x0A..=0x0B).contains(&raw_bank) {
                    (true, raw_bank & 0x07)
                } else {
                    (false, raw_bank)
                }
            }
            0xCA => (false, raw_bank),
            _ => (false, raw_bank),
        }
    }

    fn read_chr_mixed_bank(bank_data: &[u8], bank: usize, local_offset: usize) -> u8 {
        if bank_data.is_empty() {
            return 0;
        }

        let bank_count = (bank_data.len() / 0x0400).max(1);
        let chr_addr = ((bank % bank_count) * 0x0400 + local_offset) % bank_data.len();
        bank_data[chr_addr]
    }

    fn write_chr_mixed_bank(bank_data: &mut [u8], bank: usize, local_offset: usize, data: u8) {
        if bank_data.is_empty() {
            return;
        }

        let bank_count = (bank_data.len() / 0x0400).max(1);
        let chr_addr = ((bank % bank_count) * 0x0400 + local_offset) % bank_data.len();
        bank_data[chr_addr] = data;
    }

    fn resolve_mixed_chr_bank(&self, raw_bank: usize) -> (bool, usize) {
        match self.mapper {
            74 => match raw_bank {
                8 | 9 => (true, raw_bank - 8),
                _ => (false, raw_bank),
            },
            119 => {
                if raw_bank & 0x40 != 0 {
                    (true, raw_bank & 0x07)
                } else {
                    (false, raw_bank & 0x3F)
                }
            }
            191 => {
                if self.mapper191_effective_outer_bank() != 3 {
                    (false, raw_bank | 0x80)
                } else if raw_bank & 0x80 != 0 {
                    (true, raw_bank & 0x01)
                } else {
                    (false, raw_bank & 0x7F)
                }
            }
            192 => match raw_bank {
                8..=11 => (true, raw_bank - 8),
                _ => (false, raw_bank),
            },
            194 => match raw_bank {
                0 | 1 => (true, raw_bank),
                _ => (false, raw_bank),
            },
            195 => self.resolve_mapper195_chr_bank(raw_bank),
            _ => (false, raw_bank),
        }
    }

    fn read_chr_mixed_mmc3(&self, addr: u16) -> u8 {
        if let Some(ref mmc3) = self.mmc3 {
            let (raw_bank, local_offset) = self.resolve_chr_bank_raw_mmc3(addr, mmc3);
            let (is_ram, bank) = self.resolve_mixed_chr_bank(raw_bank);
            if is_ram {
                return Self::read_chr_mixed_bank(&self.chr_ram, bank, local_offset);
            }

            Self::read_chr_mixed_bank(&self.chr_rom, bank, local_offset)
        } else {
            0
        }
    }

    fn write_chr_mixed_mmc3(&mut self, addr: u16, data: u8) {
        if let Some(ref mmc3) = self.mmc3 {
            let (raw_bank, local_offset) = self.resolve_chr_bank_raw_mmc3(addr, mmc3);
            let (is_ram, bank) = self.resolve_mixed_chr_bank(raw_bank);
            if is_ram {
                Self::write_chr_mixed_bank(&mut self.chr_ram, bank, local_offset, data);
            } else if self.mapper == 195 {
                if let Some(mode) = Self::mapper195_mode_for_bank(raw_bank) {
                    self.mapper195_mode = mode;
                }
            }
        }
    }

    fn mapper245_prg_bank_base(mmc3: &Mmc3) -> usize {
        let high_bit_source = if (mmc3.bank_select & 0x80) == 0 {
            mmc3.bank_registers[0]
        } else {
            mmc3.bank_registers[2]
        };

        ((high_bit_source >> 1) as usize & 0x01) << 5
    }

    fn mapper208_apply_prg_and_mirroring(&mut self, data: u8) {
        let bank_count = (self.prg_rom.len() / 0x8000).max(1);
        let bank = ((((data >> 4) & 0x01) << 1) | (data & 0x01)) as usize;
        self.prg_bank = (bank % bank_count) as u8;
        self.mirroring = if data & 0x20 != 0 {
            Mirroring::Horizontal
        } else {
            Mirroring::Vertical
        };
    }

    pub(in crate::cartridge) fn read_prg_mapper208(&self, addr: u16) -> u8 {
        if self.prg_rom.is_empty() {
            return 0;
        }

        let bank_count = (self.prg_rom.len() / 0x8000).max(1);
        let bank = (self.prg_bank as usize) % bank_count;
        let offset = bank * 0x8000 + (addr - 0x8000) as usize;
        self.prg_rom[offset % self.prg_rom.len()]
    }

    pub(in crate::cartridge) fn read_prg_low_mapper208(&self, addr: u16) -> u8 {
        match addr {
            0x5800..=0x5FFF => self.mapper208_protection_regs[(addr as usize) & 0x03],
            _ => 0,
        }
    }

    pub(in crate::cartridge) fn read_prg_mmc3(&self, addr: u16) -> u8 {
        if let Some(ref mmc3) = self.mmc3 {
            let num_8k_banks = self.prg_rom.len() / 0x2000;
            if num_8k_banks == 0 {
                return 0;
            }
            let bank_mask = num_8k_banks - 1;
            let prg_mode = (mmc3.bank_select >> 6) & 1;
            let second_last = (num_8k_banks - 2) & bank_mask;
            let last = (num_8k_banks - 1) & bank_mask;

            let (bank, offset) = match addr {
                0x8000..=0x9FFF => {
                    let bank = if prg_mode == 0 {
                        (mmc3.bank_registers[6] as usize) & bank_mask
                    } else {
                        second_last
                    };
                    (bank, (addr - 0x8000) as usize)
                }
                0xA000..=0xBFFF => {
                    let bank = (mmc3.bank_registers[7] as usize) & bank_mask;
                    (bank, (addr - 0xA000) as usize)
                }
                0xC000..=0xDFFF => {
                    let bank = if prg_mode == 0 {
                        second_last
                    } else {
                        (mmc3.bank_registers[6] as usize) & bank_mask
                    };
                    (bank, (addr - 0xC000) as usize)
                }
                0xE000..=0xFFFF => (last, (addr - 0xE000) as usize),
                _ => return 0,
            };

            let rom_addr = bank * 0x2000 + offset;
            if rom_addr < self.prg_rom.len() {
                self.prg_rom[rom_addr]
            } else {
                0
            }
        } else {
            0
        }
    }

    pub(in crate::cartridge) fn read_prg_mapper191(&self, addr: u16) -> u8 {
        if let Some(ref mmc3) = self.mmc3 {
            let num_8k_banks = self.prg_rom.len() / 0x2000;
            if num_8k_banks == 0 {
                return 0;
            }
            let bank_mask = num_8k_banks - 1;
            let prg_mode = (mmc3.bank_select >> 6) & 1;
            let fixed_base = (0x18 + self.mapper191_effective_outer_bank() * 2) & bank_mask;
            let second_last = fixed_base;
            let last = (fixed_base + 1) & bank_mask;

            let (bank, offset) = match addr {
                0x8000..=0x9FFF => {
                    let bank = if prg_mode == 0 {
                        (mmc3.bank_registers[6] as usize) & bank_mask
                    } else {
                        second_last
                    };
                    (bank, (addr - 0x8000) as usize)
                }
                0xA000..=0xBFFF => {
                    let bank = (mmc3.bank_registers[7] as usize) & bank_mask;
                    (bank, (addr - 0xA000) as usize)
                }
                0xC000..=0xDFFF => {
                    let bank = if prg_mode == 0 {
                        second_last
                    } else {
                        (mmc3.bank_registers[6] as usize) & bank_mask
                    };
                    (bank, (addr - 0xC000) as usize)
                }
                0xE000..=0xFFFF => (last, (addr - 0xE000) as usize),
                _ => return 0,
            };

            let rom_addr = bank * 0x2000 + offset;
            self.prg_rom.get(rom_addr).copied().unwrap_or(0)
        } else {
            0
        }
    }

    pub(in crate::cartridge) fn write_prg_mapper191(&mut self, addr: u16, data: u8) {
        if (addr & 0xF0FF) == 0x90AA {
            if self.mapper191_outer_bank_writable() {
                self.mapper191_outer_bank = data & 0x03;
            }
            return;
        }

        self.write_prg_mmc3(addr, data);
    }

    /// Mapper 245: MMC3-like PRG layout with an extra PRG bank bit borrowed
    /// from the currently active CHR bank register. CHR itself is plain 8KB
    /// CHR-RAM, so the bank registers only influence PRG selection.
    pub(in crate::cartridge) fn read_prg_mapper245(&self, addr: u16) -> u8 {
        if let Some(ref mmc3) = self.mmc3 {
            let num_8k_banks = self.prg_rom.len() / 0x2000;
            if num_8k_banks == 0 {
                return 0;
            }

            let prg_mode = (mmc3.bank_select >> 6) & 1;
            let base = Self::mapper245_prg_bank_base(mmc3);
            let second_last = (base | 30) % num_8k_banks;
            let last = (base | 31) % num_8k_banks;

            let (bank, offset) = match addr {
                0x8000..=0x9FFF => {
                    let bank = if prg_mode == 0 {
                        (base | (mmc3.bank_registers[6] as usize & 0x1F)) % num_8k_banks
                    } else {
                        second_last
                    };
                    (bank, (addr - 0x8000) as usize)
                }
                0xA000..=0xBFFF => {
                    let bank = (base | (mmc3.bank_registers[7] as usize & 0x1F)) % num_8k_banks;
                    (bank, (addr - 0xA000) as usize)
                }
                0xC000..=0xDFFF => {
                    let bank = if prg_mode == 0 {
                        second_last
                    } else {
                        (base | (mmc3.bank_registers[6] as usize & 0x1F)) % num_8k_banks
                    };
                    (bank, (addr - 0xC000) as usize)
                }
                0xE000..=0xFFFF => (last, (addr - 0xE000) as usize),
                _ => return 0,
            };

            let rom_addr = bank * 0x2000 + offset;
            self.prg_rom.get(rom_addr).copied().unwrap_or(0)
        } else {
            0
        }
    }

    pub(in crate::cartridge) fn write_prg_mmc3(&mut self, addr: u16, data: u8) {
        if let Some(ref mut mmc3) = self.mmc3 {
            let even = (addr & 1) == 0;
            match addr {
                0x8000..=0x9FFF => {
                    if even {
                        // Bank Select
                        mmc3.bank_select = data;
                    } else {
                        // Bank Data
                        let reg = (mmc3.bank_select & 0x07) as usize;
                        mmc3.bank_registers[reg] = data;
                    }
                }
                0xA000..=0xBFFF => {
                    if even {
                        // Mirroring
                        self.mirroring = if data & 0x01 != 0 {
                            Mirroring::Horizontal
                        } else {
                            Mirroring::Vertical
                        };
                    } else {
                        // PRG-RAM protect
                        mmc3.prg_ram_write_protect = (data & 0x40) != 0;
                        mmc3.prg_ram_enabled = (data & 0x80) != 0;
                    }
                }
                0xC000..=0xDFFF => {
                    if even {
                        // IRQ Latch
                        mmc3.irq_latch = data;
                    } else {
                        // IRQ Reload
                        mmc3.irq_reload = true;
                    }
                }
                0xE000..=0xFFFF => {
                    if even {
                        // IRQ Disable
                        mmc3.irq_enabled = false;
                        mmc3.irq_pending.set(false);
                    } else {
                        // IRQ Enable
                        mmc3.irq_enabled = true;
                    }
                }
                _ => {}
            }
        }
    }

    pub(in crate::cartridge) fn write_prg_mapper250(&mut self, addr: u16, _data: u8) {
        if !(0x8000..=0xFFFF).contains(&addr) {
            return;
        }

        let synthetic_addr = (addr & 0xE000) | if (addr & 0x0400) != 0 { 1 } else { 0 };
        let synthetic_data = (addr & 0x00FF) as u8;
        self.write_prg_mmc3(synthetic_addr, synthetic_data);
    }

    pub(in crate::cartridge) fn write_prg_mapper208(&mut self, addr: u16, data: u8) {
        match addr {
            0x4800..=0x4FFF => self.mapper208_apply_prg_and_mirroring(data),
            0x5000..=0x57FF => {
                self.mapper208_protection_index = data;
            }
            0x5800..=0x5FFF => {
                let lut = MAPPER208_PROTECTION_LUT[self.mapper208_protection_index as usize];
                self.mapper208_protection_regs[(addr as usize) & 0x03] = data ^ lut;
            }
            0x8000..=0x9FFF | 0xC000..=0xFFFF => self.write_prg_mmc3(addr, data),
            _ => {}
        }
    }

    pub(in crate::cartridge) fn write_prg_ram_mapper208(&mut self, addr: u16, data: u8) {
        if (0x6800..=0x6FFF).contains(&addr) {
            self.mapper208_apply_prg_and_mirroring(data);
        }
    }

    pub(in crate::cartridge) fn read_chr_mmc3(&self, addr: u16) -> u8 {
        if let Some(ref mmc3) = self.mmc3 {
            let chr_a12_invert = (mmc3.bank_select >> 7) & 1;
            let num_1k_banks = if !self.chr_ram.is_empty() {
                self.chr_ram.len() / 0x0400
            } else {
                self.chr_rom.len() / 0x0400
            };
            if num_1k_banks == 0 {
                return 0;
            }
            let bank_mask = num_1k_banks - 1;

            let (bank_1k, local_offset) =
                self.resolve_chr_bank_mmc3(addr, chr_a12_invert, bank_mask, mmc3);

            let chr_addr = bank_1k * 0x0400 + local_offset;
            if !self.chr_ram.is_empty() {
                if chr_addr < self.chr_ram.len() {
                    self.chr_ram[chr_addr]
                } else {
                    0
                }
            } else if chr_addr < self.chr_rom.len() {
                self.chr_rom[chr_addr]
            } else {
                0
            }
        } else {
            0
        }
    }

    pub(in crate::cartridge) fn read_chr_mapper74(&self, addr: u16) -> u8 {
        self.read_chr_mixed_mmc3(addr)
    }

    pub(in crate::cartridge) fn write_chr_mapper74(&mut self, addr: u16, data: u8) {
        self.write_chr_mixed_mmc3(addr, data)
    }

    pub(in crate::cartridge) fn read_chr_mapper119(&self, addr: u16) -> u8 {
        self.read_chr_mixed_mmc3(addr)
    }

    pub(in crate::cartridge) fn write_chr_mapper119(&mut self, addr: u16, data: u8) {
        self.write_chr_mixed_mmc3(addr, data)
    }

    pub(in crate::cartridge) fn read_chr_mapper191(&self, addr: u16) -> u8 {
        self.read_chr_mixed_mmc3(addr)
    }

    pub(in crate::cartridge) fn write_chr_mapper191(&mut self, addr: u16, data: u8) {
        self.write_chr_mixed_mmc3(addr, data)
    }

    pub(in crate::cartridge) fn read_chr_mapper192(&self, addr: u16) -> u8 {
        self.read_chr_mixed_mmc3(addr)
    }

    pub(in crate::cartridge) fn write_chr_mapper192(&mut self, addr: u16, data: u8) {
        self.write_chr_mixed_mmc3(addr, data)
    }

    pub(in crate::cartridge) fn read_chr_mapper194(&self, addr: u16) -> u8 {
        self.read_chr_mixed_mmc3(addr)
    }

    pub(in crate::cartridge) fn write_chr_mapper194(&mut self, addr: u16, data: u8) {
        self.write_chr_mixed_mmc3(addr, data)
    }

    pub(in crate::cartridge) fn read_chr_mapper195(&self, addr: u16) -> u8 {
        self.read_chr_mixed_mmc3(addr)
    }

    pub(in crate::cartridge) fn write_chr_mapper195(&mut self, addr: u16, data: u8) {
        self.write_chr_mixed_mmc3(addr, data)
    }

    pub(in crate::cartridge) fn write_chr_mmc3(&mut self, addr: u16, data: u8) {
        if !self.chr_ram.is_empty() {
            if let Some(ref mmc3) = self.mmc3 {
                let chr_a12_invert = (mmc3.bank_select >> 7) & 1;
                let num_1k_banks = self.chr_ram.len() / 0x0400;
                if num_1k_banks == 0 {
                    return;
                }
                let bank_mask = num_1k_banks - 1;

                let (bank_1k, local_offset) =
                    self.resolve_chr_bank_mmc3(addr, chr_a12_invert, bank_mask, mmc3);

                let chr_addr = bank_1k * 0x0400 + local_offset;
                if chr_addr < self.chr_ram.len() {
                    self.chr_ram[chr_addr] = data;
                }
            }
        }
    }

    pub(in crate::cartridge) fn read_chr_mapper245(&self, addr: u16) -> u8 {
        let chr_addr = (addr & 0x1FFF) as usize;
        if chr_addr < self.chr_ram.len() {
            self.chr_ram[chr_addr]
        } else if chr_addr < self.chr_rom.len() {
            self.chr_rom[chr_addr]
        } else {
            0
        }
    }

    pub(in crate::cartridge) fn write_chr_mapper245(&mut self, addr: u16, data: u8) {
        let chr_addr = (addr & 0x1FFF) as usize;
        if chr_addr < self.chr_ram.len() {
            self.chr_ram[chr_addr] = data;
        }
    }

    fn resolve_chr_bank_mmc3(
        &self,
        addr: u16,
        _chr_a12_invert: u8,
        bank_mask: usize,
        mmc3: &Mmc3,
    ) -> (usize, usize) {
        let (raw_bank, local_offset) = self.resolve_chr_bank_raw_mmc3(addr, mmc3);
        (raw_bank & bank_mask, local_offset)
    }

    fn resolve_chr_bank_raw_mmc3(&self, addr: u16, mmc3: &Mmc3) -> (usize, usize) {
        // CHR A12 inversion swaps the 2KB and 1KB regions:
        // invert=0: R0,R1 at $0000-$0FFF (2KB each), R2-R5 at $1000-$1FFF (1KB each)
        // invert=1: R2-R5 at $0000-$0FFF (1KB each), R0,R1 at $1000-$1FFF (2KB each)
        let chr_a12_invert = (mmc3.bank_select >> 7) & 1;
        let slot = (addr >> 10) & 7; // 0-7, each 1KB slot
        let adjusted_slot = if chr_a12_invert != 0 {
            slot ^ 4 // swap upper and lower halves
        } else {
            slot
        };

        let bank_1k = match adjusted_slot {
            0 => mmc3.bank_registers[0] as usize & !1,       // R0 low
            1 => (mmc3.bank_registers[0] as usize & !1) | 1, // R0 high
            2 => mmc3.bank_registers[1] as usize & !1,       // R1 low
            3 => (mmc3.bank_registers[1] as usize & !1) | 1, // R1 high
            4 => mmc3.bank_registers[2] as usize,
            5 => mmc3.bank_registers[3] as usize,
            6 => mmc3.bank_registers[4] as usize,
            7 => mmc3.bank_registers[5] as usize,
            _ => 0,
        };

        let local_offset = (addr & 0x03FF) as usize;
        (bank_1k, local_offset)
    }

    pub(in crate::cartridge) fn read_prg_ram_mmc3(&self, addr: u16) -> u8 {
        if let Some(ref mmc3) = self.mmc3 {
            if !mmc3.prg_ram_enabled {
                return 0;
            }
        }
        if !self.prg_ram.is_empty() {
            let ram_addr = (addr - 0x6000) as usize;
            if ram_addr < self.prg_ram.len() {
                self.prg_ram[ram_addr]
            } else {
                0
            }
        } else {
            0
        }
    }

    pub(in crate::cartridge) fn write_prg_ram_mmc3(&mut self, addr: u16, data: u8) {
        if let Some(ref mmc3) = self.mmc3 {
            if !mmc3.prg_ram_enabled || mmc3.prg_ram_write_protect {
                return;
            }
        }
        if !self.prg_ram.is_empty() {
            let ram_addr = (addr - 0x6000) as usize;
            if ram_addr < self.prg_ram.len() {
                self.prg_ram[ram_addr] = data;
            }
        }
    }

    pub fn clock_irq_counter(&mut self) {
        if let Some(ref mut mmc3) = self.mmc3 {
            mmc3.clock_irq_mut();
        }
    }

    pub fn irq_pending(&self) -> bool {
        if let Some(ref mmc3) = self.mmc3 {
            if mmc3.irq_pending.get() {
                return true;
            }
        }
        if let Some(ref fme7) = self.fme7 {
            if fme7.irq_pending.get() {
                return true;
            }
        }
        if let Some(ref bandai) = self.bandai_fcg {
            if bandai.irq_pending.get() {
                return true;
            }
        }
        false
    }

    pub fn acknowledge_irq(&self) {
        if let Some(ref mmc3) = self.mmc3 {
            mmc3.irq_pending.set(false);
        }
        if let Some(ref fme7) = self.fme7 {
            fme7.irq_pending.set(false);
        }
        if let Some(ref bandai) = self.bandai_fcg {
            bandai.irq_pending.set(false);
        }
    }

    pub fn clock_irq_counter_cycles(&mut self, cycles: u32) {
        if let Some(ref mut fme7) = self.fme7 {
            for _ in 0..cycles {
                fme7.clock_irq_mut();
            }
        }
        if let Some(ref mut bandai) = self.bandai_fcg {
            for _ in 0..cycles {
                bandai.clock_irq_mut();
            }
        }
    }

    /// Clock Sunsoft 5B expansion audio one CPU cycle and return output sample.
    pub fn clock_expansion_audio(&mut self) -> f32 {
        if let Some(ref mut fme7) = self.fme7 {
            fme7.audio.clock()
        } else {
            0.0
        }
    }
}
