use std::cell::Cell;

use super::super::{Cartridge, Mirroring};

#[derive(Debug, Clone)]
pub(in crate::cartridge) struct Vrc2Vrc4 {
    pub(in crate::cartridge) prg_banks: [u8; 2],
    pub(in crate::cartridge) chr_banks: [u16; 8],
    pub(in crate::cartridge) wram_enabled: bool,
    pub(in crate::cartridge) prg_swap_mode: bool,
    pub(in crate::cartridge) vrc4_mode: bool,
    pub(in crate::cartridge) latch: u8,
    pub(in crate::cartridge) irq_latch: u8,
    pub(in crate::cartridge) irq_counter: u8,
    pub(in crate::cartridge) irq_enable_after_ack: bool,
    pub(in crate::cartridge) irq_enabled: bool,
    pub(in crate::cartridge) irq_cycle_mode: bool,
    pub(in crate::cartridge) irq_prescaler: i16,
    pub(in crate::cartridge) irq_pending: Cell<bool>,
}

impl Vrc2Vrc4 {
    pub(in crate::cartridge) fn new() -> Self {
        Self {
            prg_banks: [0, 1],
            chr_banks: [0, 1, 2, 3, 4, 5, 6, 7],
            wram_enabled: false,
            prg_swap_mode: false,
            vrc4_mode: false,
            latch: 0,
            irq_latch: 0,
            irq_counter: 0,
            irq_enable_after_ack: false,
            irq_enabled: false,
            irq_cycle_mode: false,
            irq_prescaler: 341,
            irq_pending: Cell::new(false),
        }
    }

    fn clock_irq_counter(&mut self) {
        if self.irq_counter == 0xFF {
            self.irq_counter = self.irq_latch;
            self.irq_pending.set(true);
        } else {
            self.irq_counter = self.irq_counter.wrapping_add(1);
        }
    }

    pub(in crate::cartridge) fn clock_irq_mut(&mut self, cycles: u32) {
        if !self.irq_enabled {
            return;
        }

        for _ in 0..cycles {
            if self.irq_cycle_mode {
                self.clock_irq_counter();
            } else {
                self.irq_prescaler -= 3;
                if self.irq_prescaler <= 0 {
                    self.irq_prescaler += 341;
                    self.clock_irq_counter();
                }
            }
        }
    }
}

impl Cartridge {
    fn vrc2_vrc4_decode_index(&self, addr: u16) -> Option<u8> {
        match self.mapper {
            21 => match addr & 0x00C6 {
                0x000 => Some(0),
                0x002 | 0x040 => Some(1),
                0x004 | 0x080 => Some(2),
                0x006 | 0x0C0 => Some(3),
                _ => None,
            },
            _ => match (self.mapper, addr & 0x000F) {
                (22, 0x0) => Some(0),
                (22, 0x2 | 0x8) => Some(1),
                (22, 0x1 | 0x4) => Some(2),
                (22, 0x3 | 0xC) => Some(3),
                (23, 0x0) | (25, 0x0) => Some(0),
                (23, 0x1 | 0x4) | (25, 0x2 | 0x8) => Some(1),
                (23, 0x2 | 0x8) | (25, 0x1 | 0x4) => Some(2),
                (23, 0x3 | 0xC) | (25, 0x3 | 0xC) => Some(3),
                _ => None,
            },
        }
    }

    fn vrc2_vrc4_uses_alt_vrc4_decode(&self, addr: u16) -> bool {
        self.mapper != 22 && matches!(addr & 0x000F, 0x4 | 0x8 | 0xC)
    }

    fn vrc2_vrc4_decode_chr_index(addr: u16, reg: u8) -> Option<(usize, bool)> {
        let base = match addr & 0xF000 {
            0xB000 => 0,
            0xC000 => 2,
            0xD000 => 4,
            0xE000 => 6,
            _ => return None,
        };
        Some((base + usize::from(reg / 2), reg & 1 != 0))
    }

    fn vrc2_vrc4_chr_data(&self) -> &[u8] {
        if !self.chr_ram.is_empty() {
            &self.chr_ram
        } else {
            &self.chr_rom
        }
    }

    fn vrc2_vrc4_effective_chr_bank(mapper: u8, raw_bank: u16, bank_count: usize) -> usize {
        let bank = if mapper == 22 {
            raw_bank >> 1
        } else {
            raw_bank
        };
        bank as usize % bank_count
    }

    fn vrc2_vrc4_decode_mirroring(vrc4_mode: bool, data: u8) -> Mirroring {
        if vrc4_mode {
            match data & 0x03 {
                0 => Mirroring::Vertical,
                1 => Mirroring::Horizontal,
                2 => Mirroring::OneScreenLower,
                _ => Mirroring::OneScreenUpper,
            }
        } else if data & 0x01 != 0 {
            Mirroring::Horizontal
        } else {
            Mirroring::Vertical
        }
    }

    pub(in crate::cartridge) fn read_prg_mapper21(&self, addr: u16) -> u8 {
        self.read_prg_mapper23(addr)
    }

    pub(in crate::cartridge) fn read_prg_mapper22(&self, addr: u16) -> u8 {
        self.read_prg_mapper23(addr)
    }

    pub(in crate::cartridge) fn read_prg_mapper23(&self, addr: u16) -> u8 {
        if let Some(vrc) = self.vrc2_vrc4.as_ref() {
            if self.prg_rom.is_empty() {
                return 0;
            }

            let bank_count = (self.prg_rom.len() / 0x2000).max(1);
            let second_last = bank_count.saturating_sub(2);
            let last = bank_count.saturating_sub(1);
            let bank = match addr {
                0x8000..=0x9FFF => {
                    if vrc.prg_swap_mode {
                        second_last
                    } else {
                        vrc.prg_banks[0] as usize
                    }
                }
                0xA000..=0xBFFF => vrc.prg_banks[1] as usize,
                0xC000..=0xDFFF => {
                    if vrc.prg_swap_mode {
                        vrc.prg_banks[0] as usize
                    } else {
                        second_last
                    }
                }
                0xE000..=0xFFFF => last,
                _ => return 0,
            } % bank_count;

            let rom_addr = bank * 0x2000 + (addr as usize & 0x1FFF);
            self.prg_rom[rom_addr % self.prg_rom.len()]
        } else {
            0
        }
    }

    pub(in crate::cartridge) fn read_prg_mapper25(&self, addr: u16) -> u8 {
        self.read_prg_mapper23(addr)
    }

    pub(in crate::cartridge) fn write_prg_mapper21(&mut self, addr: u16, data: u8) {
        self.write_prg_mapper23(addr, data);
    }

    pub(in crate::cartridge) fn write_prg_mapper22(&mut self, addr: u16, data: u8) {
        self.write_prg_mapper23(addr, data);
    }

    pub(in crate::cartridge) fn write_prg_mapper23(&mut self, addr: u16, data: u8) {
        let reg = match self.vrc2_vrc4_decode_index(addr) {
            Some(reg) => reg,
            None => return,
        };

        let prg_bank_count = (self.prg_rom.len() / 0x2000).max(1);
        let chr_bank_count = (self.vrc2_vrc4_chr_data().len() / 0x0400).max(1);
        let use_alt_vrc4_decode = self.vrc2_vrc4_uses_alt_vrc4_decode(addr);

        if let Some(vrc) = self.vrc2_vrc4.as_mut() {
            if use_alt_vrc4_decode {
                vrc.vrc4_mode = true;
            }

            match addr & 0xF000 {
                0x8000 => {
                    vrc.prg_banks[0] = ((data & 0x1F) as usize % prg_bank_count) as u8;
                    self.prg_bank = vrc.prg_banks[0];
                }
                0x9000 => {
                    if self.mapper == 22 {
                        self.mirroring = Self::vrc2_vrc4_decode_mirroring(vrc.vrc4_mode, data);
                    } else {
                        match reg {
                            0 => {
                                self.mirroring =
                                    Self::vrc2_vrc4_decode_mirroring(vrc.vrc4_mode, data);
                            }
                            2 => {
                                vrc.vrc4_mode = true;
                                vrc.wram_enabled = data & 0x01 != 0;
                                vrc.prg_swap_mode = data & 0x02 != 0;
                            }
                            3 => {
                                vrc.vrc4_mode = true;
                            }
                            _ => {}
                        }
                    }
                }
                0xA000 => {
                    vrc.prg_banks[1] = ((data & 0x1F) as usize % prg_bank_count) as u8;
                }
                0xB000..=0xE000 => {
                    if let Some((bank_index, high)) = Self::vrc2_vrc4_decode_chr_index(addr, reg) {
                        let bank = &mut vrc.chr_banks[bank_index];
                        if high {
                            let high_mask = if vrc.vrc4_mode { 0x1F } else { 0x0F };
                            *bank = (*bank & 0x000F) | (((data & high_mask) as u16) << 4);
                        } else {
                            *bank = (*bank & !0x000F) | u16::from(data & 0x0F);
                        }
                        *bank %= chr_bank_count as u16;
                        if bank_index == 0 {
                            self.chr_bank = Self::vrc2_vrc4_effective_chr_bank(
                                self.mapper,
                                *bank,
                                chr_bank_count,
                            ) as u8;
                        }
                    }
                }
                0xF000 => {
                    if self.mapper == 22 {
                        return;
                    }
                    vrc.vrc4_mode = true;
                    match reg {
                        0 => {
                            vrc.irq_latch = (vrc.irq_latch & 0xF0) | (data & 0x0F);
                        }
                        1 => {
                            vrc.irq_latch = (vrc.irq_latch & 0x0F) | ((data & 0x0F) << 4);
                        }
                        2 => {
                            vrc.irq_enable_after_ack = data & 0x01 != 0;
                            vrc.irq_enabled = data & 0x02 != 0;
                            vrc.irq_cycle_mode = data & 0x04 != 0;
                            vrc.irq_pending.set(false);
                            vrc.irq_prescaler = 341;
                            if vrc.irq_enabled {
                                vrc.irq_counter = vrc.irq_latch;
                            }
                        }
                        3 => {
                            vrc.irq_pending.set(false);
                            vrc.irq_enabled = vrc.irq_enable_after_ack;
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
    }

    pub(in crate::cartridge) fn write_prg_mapper25(&mut self, addr: u16, data: u8) {
        self.write_prg_mapper23(addr, data);
    }

    pub(in crate::cartridge) fn read_chr_mapper21(&self, addr: u16) -> u8 {
        self.read_chr_mapper23(addr)
    }

    pub(in crate::cartridge) fn read_chr_mapper22(&self, addr: u16) -> u8 {
        self.read_chr_mapper23(addr)
    }

    pub(in crate::cartridge) fn read_chr_mapper23(&self, addr: u16) -> u8 {
        if let Some(vrc) = self.vrc2_vrc4.as_ref() {
            let chr_data = self.vrc2_vrc4_chr_data();
            if chr_data.is_empty() {
                return 0;
            }

            let bank_count = (chr_data.len() / 0x0400).max(1);
            let slot = ((addr >> 10) & 0x07) as usize;
            let bank =
                Self::vrc2_vrc4_effective_chr_bank(self.mapper, vrc.chr_banks[slot], bank_count);
            let chr_addr = bank * 0x0400 + (addr as usize & 0x03FF);
            chr_data[chr_addr % chr_data.len()]
        } else {
            0
        }
    }

    pub(in crate::cartridge) fn read_chr_mapper25(&self, addr: u16) -> u8 {
        self.read_chr_mapper23(addr)
    }

    pub(in crate::cartridge) fn write_chr_mapper21(&mut self, addr: u16, data: u8) {
        self.write_chr_mapper23(addr, data);
    }

    pub(in crate::cartridge) fn write_chr_mapper22(&mut self, addr: u16, data: u8) {
        self.write_chr_mapper23(addr, data);
    }

    pub(in crate::cartridge) fn write_chr_mapper23(&mut self, addr: u16, data: u8) {
        let (bank, chr_len, use_ram) = if let Some(vrc) = self.vrc2_vrc4.as_ref() {
            let chr_data = self.vrc2_vrc4_chr_data();
            if chr_data.is_empty() {
                return;
            }

            let bank_count = (chr_data.len() / 0x0400).max(1);
            let slot = ((addr >> 10) & 0x07) as usize;
            (
                Self::vrc2_vrc4_effective_chr_bank(self.mapper, vrc.chr_banks[slot], bank_count),
                chr_data.len(),
                !self.chr_ram.is_empty(),
            )
        } else {
            return;
        };

        let chr_addr = bank * 0x0400 + (addr as usize & 0x03FF);
        if use_ram {
            self.chr_ram[chr_addr % chr_len] = data;
        } else if !self.chr_rom.is_empty() {
            self.chr_rom[chr_addr % chr_len] = data;
        }
    }

    pub(in crate::cartridge) fn write_chr_mapper25(&mut self, addr: u16, data: u8) {
        self.write_chr_mapper23(addr, data);
    }

    pub(in crate::cartridge) fn read_prg_ram_mapper21(&self, addr: u16) -> u8 {
        self.read_prg_ram_mapper23(addr)
    }

    pub(in crate::cartridge) fn read_prg_ram_mapper22(&self, addr: u16) -> u8 {
        self.read_prg_ram_mapper23(addr)
    }

    pub(in crate::cartridge) fn read_prg_ram_mapper23(&self, addr: u16) -> u8 {
        if let Some(vrc) = self.vrc2_vrc4.as_ref() {
            if self.mapper == 25 && self.has_battery && !self.prg_ram.is_empty() {
                let offset = (addr as usize - 0x6000) % self.prg_ram.len();
                return self.prg_ram[offset];
            }
            if vrc.wram_enabled && !self.prg_ram.is_empty() {
                let offset = (addr as usize - 0x6000) % self.prg_ram.len();
                return self.prg_ram[offset];
            }

            let open_bus = (addr >> 8) as u8;
            if (0x6000..=0x6FFF).contains(&addr) {
                open_bus | (vrc.latch & 0x01)
            } else {
                open_bus
            }
        } else {
            0
        }
    }

    pub(in crate::cartridge) fn read_prg_ram_mapper25(&self, addr: u16) -> u8 {
        self.read_prg_ram_mapper23(addr)
    }

    pub(in crate::cartridge) fn write_prg_ram_mapper21(&mut self, addr: u16, data: u8) {
        self.write_prg_ram_mapper23(addr, data);
    }

    pub(in crate::cartridge) fn write_prg_ram_mapper22(&mut self, addr: u16, data: u8) {
        self.write_prg_ram_mapper23(addr, data);
    }

    pub(in crate::cartridge) fn write_prg_ram_mapper23(&mut self, addr: u16, data: u8) {
        if let Some(vrc) = self.vrc2_vrc4.as_mut() {
            if self.mapper == 25 && self.has_battery && !self.prg_ram.is_empty() {
                let offset = (addr as usize - 0x6000) % self.prg_ram.len();
                self.prg_ram[offset] = data;
                return;
            }
            if vrc.wram_enabled && !self.prg_ram.is_empty() {
                let offset = (addr as usize - 0x6000) % self.prg_ram.len();
                self.prg_ram[offset] = data;
            } else if (0x6000..=0x6FFF).contains(&addr) {
                vrc.latch = data & 0x01;
            }
        }
    }

    pub(in crate::cartridge) fn write_prg_ram_mapper25(&mut self, addr: u16, data: u8) {
        self.write_prg_ram_mapper23(addr, data);
    }

    pub(in crate::cartridge) fn clock_irq_mapper21(&mut self, cycles: u32) {
        self.clock_irq_mapper23(cycles);
    }

    pub(in crate::cartridge) fn clock_irq_mapper23(&mut self, cycles: u32) {
        if let Some(vrc) = self.vrc2_vrc4.as_mut() {
            vrc.clock_irq_mut(cycles);
        }
    }

    pub(in crate::cartridge) fn clock_irq_mapper25(&mut self, cycles: u32) {
        self.clock_irq_mapper23(cycles);
    }
}
