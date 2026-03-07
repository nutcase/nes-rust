use super::super::{Cartridge, Mirroring};

#[derive(Debug, Clone)]
pub(in crate::cartridge) struct TaitoTc0190 {
    pub(in crate::cartridge) prg_banks: [u8; 2],
    pub(in crate::cartridge) chr_banks: [u8; 6],
}

#[derive(Debug, Clone)]
pub(in crate::cartridge) struct TaitoX1005 {
    pub(in crate::cartridge) prg_banks: [u8; 3],
    pub(in crate::cartridge) chr_banks: [u8; 6],
    pub(in crate::cartridge) ram_enabled: bool,
}

#[derive(Debug, Clone)]
pub(in crate::cartridge) struct TaitoX1017 {
    pub(in crate::cartridge) prg_banks: [u8; 3],
    pub(in crate::cartridge) chr_banks: [u8; 6],
    pub(in crate::cartridge) ram_enabled: [bool; 3],
    pub(in crate::cartridge) chr_invert: bool,
}

impl TaitoTc0190 {
    pub(in crate::cartridge) fn new() -> Self {
        Self {
            prg_banks: [0, 1],
            chr_banks: [0, 1, 2, 3, 4, 5],
        }
    }
}

impl TaitoX1005 {
    pub(in crate::cartridge) fn new() -> Self {
        Self {
            prg_banks: [0, 1, 2],
            chr_banks: [0, 1, 2, 3, 4, 5],
            ram_enabled: false,
        }
    }
}

impl TaitoX1017 {
    pub(in crate::cartridge) fn new() -> Self {
        Self {
            prg_banks: [0, 1, 2],
            chr_banks: [0, 1, 2, 3, 4, 5],
            ram_enabled: [false; 3],
            chr_invert: false,
        }
    }
}

impl Cartridge {
    fn sync_taito207_mirroring(&mut self) {
        if self.mapper != 207 {
            return;
        }
        if let Some(taito) = self.taito_x1005.as_ref() {
            let top = (taito.chr_banks[0] >> 7) & 1;
            let bottom = (taito.chr_banks[1] >> 7) & 1;
            self.mirroring = match (top, bottom) {
                (0, 0) => Mirroring::OneScreenLower,
                (1, 1) => Mirroring::OneScreenUpper,
                (0, 1) => Mirroring::Horizontal,
                (1, 0) => Mirroring::HorizontalSwapped,
                _ => Mirroring::Horizontal,
            };
        }
    }

    fn read_prg_taito_like(&self, addr: u16, prg_banks: &[u8]) -> u8 {
        if self.prg_rom.is_empty() {
            return 0;
        }

        let bank_count = (self.prg_rom.len() / 0x2000).max(1);
        let second_last = bank_count.saturating_sub(2);
        let last = bank_count.saturating_sub(1);

        let (bank, base) = match addr {
            0x8000..=0x9FFF => (prg_banks.first().copied().unwrap_or(0) as usize, 0x8000),
            0xA000..=0xBFFF => (prg_banks.get(1).copied().unwrap_or(0) as usize, 0xA000),
            0xC000..=0xDFFF => (
                prg_banks
                    .get(2)
                    .copied()
                    .map(usize::from)
                    .unwrap_or(second_last),
                0xC000,
            ),
            0xE000..=0xFFFF => (last, 0xE000),
            _ => return 0,
        };

        let rom_addr = (bank % bank_count) * 0x2000 + (addr - base) as usize;
        self.prg_rom[rom_addr % self.prg_rom.len()]
    }

    fn resolve_taito_chr_bank(
        chr_banks: &[u8; 6],
        addr: u16,
        chr_invert: bool,
        mask_high_mirroring_bits: bool,
    ) -> usize {
        let slot = ((addr >> 10) & 0x07) as usize;
        let adjusted_slot = if chr_invert { slot ^ 4 } else { slot };
        let chr0 = if mask_high_mirroring_bits {
            chr_banks[0] & 0x7F
        } else {
            chr_banks[0]
        };
        let chr1 = if mask_high_mirroring_bits {
            chr_banks[1] & 0x7F
        } else {
            chr_banks[1]
        };

        match adjusted_slot {
            0 => (chr0 as usize) * 2,
            1 => (chr0 as usize) * 2 + 1,
            2 => (chr1 as usize) * 2,
            3 => (chr1 as usize) * 2 + 1,
            4 => chr_banks[2] as usize,
            5 => chr_banks[3] as usize,
            6 => chr_banks[4] as usize,
            _ => chr_banks[5] as usize,
        }
    }

    fn read_chr_taito_like(
        &self,
        addr: u16,
        chr_banks: &[u8; 6],
        chr_invert: bool,
        mask_high_mirroring_bits: bool,
    ) -> u8 {
        let chr_data = if !self.chr_ram.is_empty() {
            &self.chr_ram
        } else {
            &self.chr_rom
        };
        if chr_data.is_empty() {
            return 0;
        }

        let bank_count = (chr_data.len() / 0x0400).max(1);
        let bank =
            Self::resolve_taito_chr_bank(chr_banks, addr, chr_invert, mask_high_mirroring_bits)
                % bank_count;
        let chr_addr = bank * 0x0400 + (addr as usize & 0x03FF);
        chr_data[chr_addr % chr_data.len()]
    }

    fn write_chr_taito_like(
        &mut self,
        addr: u16,
        chr_banks: &[u8; 6],
        chr_invert: bool,
        mask_high_mirroring_bits: bool,
        data: u8,
    ) {
        if !self.chr_ram.is_empty() {
            let bank_count = (self.chr_ram.len() / 0x0400).max(1);
            let bank =
                Self::resolve_taito_chr_bank(chr_banks, addr, chr_invert, mask_high_mirroring_bits)
                    % bank_count;
            let chr_addr = bank * 0x0400 + (addr as usize & 0x03FF);
            let chr_len = self.chr_ram.len();
            self.chr_ram[chr_addr % chr_len] = data;
        } else if !self.chr_rom.is_empty() {
            let bank_count = (self.chr_rom.len() / 0x0400).max(1);
            let bank =
                Self::resolve_taito_chr_bank(chr_banks, addr, chr_invert, mask_high_mirroring_bits)
                    % bank_count;
            let chr_addr = bank * 0x0400 + (addr as usize & 0x03FF);
            let chr_len = self.chr_rom.len();
            self.chr_rom[chr_addr % chr_len] = data;
        }
    }

    pub(in crate::cartridge) fn read_prg_taito_tc0190(&self, addr: u16) -> u8 {
        if let Some(taito) = self.taito_tc0190.as_ref() {
            self.read_prg_taito_like(addr, &[taito.prg_banks[0], taito.prg_banks[1]])
        } else {
            0
        }
    }

    pub(in crate::cartridge) fn write_prg_taito_tc0190(&mut self, addr: u16, data: u8) {
        if let Some(taito) = self.taito_tc0190.as_mut() {
            match addr & 0xF003 {
                0x8000 => {
                    taito.prg_banks[0] = data & 0x3F;
                    self.prg_bank = taito.prg_banks[0];
                    self.mirroring = if data & 0x40 != 0 {
                        Mirroring::Horizontal
                    } else {
                        Mirroring::Vertical
                    };
                }
                0x8001 => {
                    taito.prg_banks[1] = data & 0x3F;
                }
                0x8002 => {
                    taito.chr_banks[0] = data;
                    self.chr_bank = data;
                }
                0x8003 => {
                    taito.chr_banks[1] = data;
                }
                0xA000 => {
                    taito.chr_banks[2] = data;
                }
                0xA001 => {
                    taito.chr_banks[3] = data;
                }
                0xA002 => {
                    taito.chr_banks[4] = data;
                }
                0xA003 => {
                    taito.chr_banks[5] = data;
                }
                _ => {}
            }
        }
    }

    pub(in crate::cartridge) fn read_chr_taito_tc0190(&self, addr: u16) -> u8 {
        if let Some(taito) = self.taito_tc0190.as_ref() {
            self.read_chr_taito_like(addr, &taito.chr_banks, false, false)
        } else {
            0
        }
    }

    pub(in crate::cartridge) fn write_chr_taito_tc0190(&mut self, addr: u16, data: u8) {
        if let Some(taito) = self.taito_tc0190.as_ref() {
            let chr_banks = taito.chr_banks;
            self.write_chr_taito_like(addr, &chr_banks, false, false, data);
        }
    }

    fn taito_x1005_register(addr: u16) -> Option<u8> {
        if (addr & 0xFF70) == 0x7E70 {
            Some((addr & 0x000F) as u8)
        } else {
            None
        }
    }

    pub(in crate::cartridge) fn read_prg_taito_x1005(&self, addr: u16) -> u8 {
        if let Some(taito) = self.taito_x1005.as_ref() {
            self.read_prg_taito_like(addr, &taito.prg_banks)
        } else {
            0
        }
    }

    pub(in crate::cartridge) fn read_chr_taito_x1005(&self, addr: u16) -> u8 {
        if let Some(taito) = self.taito_x1005.as_ref() {
            self.read_chr_taito_like(addr, &taito.chr_banks, false, self.mapper == 207)
        } else {
            0
        }
    }

    pub(in crate::cartridge) fn write_chr_taito_x1005(&mut self, addr: u16, data: u8) {
        if let Some(taito) = self.taito_x1005.as_ref() {
            let chr_banks = taito.chr_banks;
            self.write_chr_taito_like(addr, &chr_banks, false, self.mapper == 207, data);
        }
    }

    pub(in crate::cartridge) fn read_prg_ram_taito_x1005(&self, addr: u16) -> u8 {
        if let Some(taito) = self.taito_x1005.as_ref() {
            if taito.ram_enabled && (0x7F00..=0x7FFF).contains(&addr) && !self.prg_ram.is_empty() {
                let ram_addr = ((addr - 0x7F00) & 0x007F) as usize;
                return self.prg_ram[ram_addr % self.prg_ram.len()];
            }
        }
        0
    }

    pub(in crate::cartridge) fn write_prg_ram_taito_x1005(&mut self, addr: u16, data: u8) {
        if let Some(reg) = Self::taito_x1005_register(addr) {
            if let Some(taito) = self.taito_x1005.as_mut() {
                match reg {
                    0..=5 => {
                        taito.chr_banks[reg as usize] = data;
                        if reg == 0 {
                            self.chr_bank = if self.mapper == 207 {
                                data & 0x7F
                            } else {
                                data
                            };
                        }
                        if reg <= 1 && self.mapper == 207 {
                            self.sync_taito207_mirroring();
                        }
                    }
                    6 => {
                        if self.mapper == 80 {
                            self.mirroring = if data & 0x01 != 0 {
                                Mirroring::Vertical
                            } else {
                                Mirroring::Horizontal
                            };
                        }
                    }
                    8..=10 => {
                        taito.prg_banks[(reg - 8) as usize] = data;
                        if reg == 8 {
                            self.prg_bank = data;
                        }
                        if reg == 10 {
                            taito.ram_enabled = data & 0x08 != 0;
                        }
                    }
                    _ => {}
                }
            }
            return;
        }

        if let Some(taito) = self.taito_x1005.as_ref() {
            if taito.ram_enabled && (0x7F00..=0x7FFF).contains(&addr) && !self.prg_ram.is_empty() {
                let ram_addr = ((addr - 0x7F00) & 0x007F) as usize;
                let ram_len = self.prg_ram.len();
                self.prg_ram[ram_addr % ram_len] = data;
            }
        }
    }

    pub(in crate::cartridge) fn read_prg_taito_x1017(&self, addr: u16) -> u8 {
        if let Some(taito) = self.taito_x1017.as_ref() {
            self.read_prg_taito_like(addr, &taito.prg_banks)
        } else {
            0
        }
    }

    pub(in crate::cartridge) fn read_chr_taito_x1017(&self, addr: u16) -> u8 {
        if let Some(taito) = self.taito_x1017.as_ref() {
            self.read_chr_taito_like(addr, &taito.chr_banks, taito.chr_invert, false)
        } else {
            0
        }
    }

    pub(in crate::cartridge) fn write_chr_taito_x1017(&mut self, addr: u16, data: u8) {
        if let Some(taito) = self.taito_x1017.as_ref() {
            let chr_banks = taito.chr_banks;
            let chr_invert = taito.chr_invert;
            self.write_chr_taito_like(addr, &chr_banks, chr_invert, false, data);
        }
    }

    pub(in crate::cartridge) fn read_prg_ram_taito_x1017(&self, addr: u16) -> u8 {
        if let Some(taito) = self.taito_x1017.as_ref() {
            if self.prg_ram.is_empty() {
                return 0;
            }
            let (enabled, offset) = match addr {
                0x6000..=0x67FF => (taito.ram_enabled[0], (addr - 0x6000) as usize),
                0x6800..=0x6FFF => (taito.ram_enabled[1], (addr - 0x6000) as usize),
                0x7000..=0x73FF => (taito.ram_enabled[2], (addr - 0x6000) as usize),
                _ => return 0,
            };
            if enabled {
                return self.prg_ram.get(offset).copied().unwrap_or(0);
            }
        }
        0
    }

    pub(in crate::cartridge) fn write_prg_ram_taito_x1017(&mut self, addr: u16, data: u8) {
        if let Some(reg) = Self::taito_x1005_register(addr) {
            if let Some(taito) = self.taito_x1017.as_mut() {
                match reg {
                    0..=1 => {
                        taito.chr_banks[reg as usize] = data & 0x7F;
                        if reg == 0 {
                            self.chr_bank = data & 0x7F;
                        }
                    }
                    2..=5 => {
                        taito.chr_banks[reg as usize] = data;
                    }
                    6 => {
                        taito.chr_invert = data & 0x02 != 0;
                        self.mirroring = if data & 0x01 != 0 {
                            Mirroring::Vertical
                        } else {
                            Mirroring::Horizontal
                        };
                    }
                    7 => taito.ram_enabled[0] = data == 0xCA,
                    8 => taito.ram_enabled[1] = data == 0x69,
                    9 => taito.ram_enabled[2] = data == 0x84,
                    10..=12 => {
                        let bank = (data >> 2) & 0x0F;
                        taito.prg_banks[(reg - 10) as usize] = bank;
                        if reg == 10 {
                            self.prg_bank = bank;
                        }
                    }
                    13..=15 => {}
                    _ => {}
                }
            }
            return;
        }

        if let Some(taito) = self.taito_x1017.as_ref() {
            if self.prg_ram.is_empty() {
                return;
            }
            let (enabled, offset) = match addr {
                0x6000..=0x67FF => (taito.ram_enabled[0], (addr - 0x6000) as usize),
                0x6800..=0x6FFF => (taito.ram_enabled[1], (addr - 0x6000) as usize),
                0x7000..=0x73FF => (taito.ram_enabled[2], (addr - 0x6000) as usize),
                _ => return,
            };
            if enabled {
                if let Some(byte) = self.prg_ram.get_mut(offset) {
                    *byte = data;
                }
            }
        }
    }
}
