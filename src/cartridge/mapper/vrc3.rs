use std::cell::Cell;

use super::super::Cartridge;

#[derive(Debug, Clone)]
pub(in crate::cartridge) struct Vrc3 {
    pub(in crate::cartridge) irq_reload: u16,
    pub(in crate::cartridge) irq_counter: u16,
    pub(in crate::cartridge) irq_enable_on_ack: bool,
    pub(in crate::cartridge) irq_enabled: bool,
    pub(in crate::cartridge) irq_mode_8bit: bool,
    pub(in crate::cartridge) irq_pending: Cell<bool>,
}

impl Vrc3 {
    pub(in crate::cartridge) fn new() -> Self {
        Self {
            irq_reload: 0,
            irq_counter: 0,
            irq_enable_on_ack: false,
            irq_enabled: false,
            irq_mode_8bit: false,
            irq_pending: Cell::new(false),
        }
    }

    fn write_latch_nibble(&mut self, nibble: usize, data: u8) {
        let shift = (nibble as u16) * 4;
        let mask = !(0x000Fu16 << shift);
        self.irq_reload = (self.irq_reload & mask) | (((data & 0x0F) as u16) << shift);
    }

    fn write_control(&mut self, data: u8) {
        self.irq_mode_8bit = data & 0x04 != 0;
        self.irq_enable_on_ack = data & 0x01 != 0;
        self.irq_enabled = data & 0x02 != 0;
        self.irq_pending.set(false);
        if self.irq_enabled {
            self.irq_counter = self.irq_reload;
        }
    }

    fn acknowledge(&mut self) {
        self.irq_pending.set(false);
        self.irq_enabled = self.irq_enable_on_ack;
    }

    pub(in crate::cartridge) fn clock_irq_mut(&mut self, cycles: u32) {
        if !self.irq_enabled {
            return;
        }

        for _ in 0..cycles {
            if self.irq_mode_8bit {
                let low = (self.irq_counter & 0x00FF) as u8;
                if low == 0xFF {
                    self.irq_counter = (self.irq_counter & 0xFF00) | (self.irq_reload & 0x00FF);
                    self.irq_pending.set(true);
                } else {
                    self.irq_counter = (self.irq_counter & 0xFF00) | (low.wrapping_add(1) as u16);
                }
            } else if self.irq_counter == 0xFFFF {
                self.irq_counter = self.irq_reload;
                self.irq_pending.set(true);
            } else {
                self.irq_counter = self.irq_counter.wrapping_add(1);
            }
        }
    }
}

impl Cartridge {
    pub(in crate::cartridge) fn read_prg_mapper142(&self, addr: u16) -> u8 {
        if self.prg_rom.is_empty() || addr < 0x8000 {
            return 0;
        }

        let bank_count = (self.prg_rom.len() / 0x2000).max(1);
        let bank = match addr {
            0x8000..=0x9FFF => self.mapper142_prg_banks[0] as usize,
            0xA000..=0xBFFF => self.mapper142_prg_banks[1] as usize,
            0xC000..=0xDFFF => self.mapper142_prg_banks[2] as usize,
            _ => bank_count.saturating_sub(1),
        } % bank_count;
        let offset = bank * 0x2000 + ((addr - 0x8000) as usize & 0x1FFF);
        self.prg_rom[offset % self.prg_rom.len()]
    }

    pub(in crate::cartridge) fn read_prg_ram_mapper142(&self, addr: u16) -> u8 {
        if self.prg_rom.is_empty() || !(0x6000..=0x7FFF).contains(&addr) {
            return 0;
        }

        let bank_count = (self.prg_rom.len() / 0x2000).max(1);
        let bank = (self.mapper142_prg_banks[3] as usize) % bank_count;
        let offset = bank * 0x2000 + ((addr - 0x6000) as usize & 0x1FFF);
        self.prg_rom[offset % self.prg_rom.len()]
    }

    pub(in crate::cartridge) fn write_prg_vrc3(&mut self, addr: u16, data: u8) {
        match addr & 0xF000 {
            0x8000 => {
                if let Some(vrc3) = self.vrc3.as_mut() {
                    vrc3.write_latch_nibble(0, data);
                }
            }
            0x9000 => {
                if let Some(vrc3) = self.vrc3.as_mut() {
                    vrc3.write_latch_nibble(1, data);
                }
            }
            0xA000 => {
                if let Some(vrc3) = self.vrc3.as_mut() {
                    vrc3.write_latch_nibble(2, data);
                }
            }
            0xB000 => {
                if let Some(vrc3) = self.vrc3.as_mut() {
                    vrc3.write_latch_nibble(3, data);
                }
            }
            0xC000 => {
                if let Some(vrc3) = self.vrc3.as_mut() {
                    vrc3.write_control(data);
                }
            }
            0xD000 => {
                if let Some(vrc3) = self.vrc3.as_mut() {
                    vrc3.acknowledge();
                }
            }
            0xF000 => {
                let bank_count = (self.prg_rom.len() / 0x4000).max(1);
                self.prg_bank = (((data as usize) & 0x07) % bank_count) as u8;
            }
            _ => {}
        }
    }

    pub(in crate::cartridge) fn write_prg_mapper142(&mut self, addr: u16, data: u8) {
        match addr & 0xF000 {
            0x8000 => {
                if let Some(vrc3) = self.vrc3.as_mut() {
                    vrc3.write_latch_nibble(0, data);
                }
            }
            0x9000 => {
                if let Some(vrc3) = self.vrc3.as_mut() {
                    vrc3.write_latch_nibble(1, data);
                }
            }
            0xA000 => {
                if let Some(vrc3) = self.vrc3.as_mut() {
                    vrc3.write_latch_nibble(2, data);
                }
            }
            0xB000 => {
                if let Some(vrc3) = self.vrc3.as_mut() {
                    vrc3.write_latch_nibble(3, data);
                }
            }
            0xC000 => {
                if let Some(vrc3) = self.vrc3.as_mut() {
                    vrc3.write_control(data);
                }
            }
            0xD000 => {
                if let Some(vrc3) = self.vrc3.as_mut() {
                    vrc3.acknowledge();
                }
            }
            0xE000 => {
                self.mapper142_bank_select = data & 0x07;
            }
            0xF000 => {
                if let Some(slot) = self.mapper142_bank_select.checked_sub(1) {
                    if let Some(bank) = self.mapper142_prg_banks.get_mut(slot as usize) {
                        *bank = data & 0x0F;
                        if slot == 0 {
                            self.prg_bank = *bank;
                        }
                    }
                }
            }
            _ => {}
        }
    }

    pub(in crate::cartridge) fn read_prg_ram_vrc3(&self, addr: u16) -> u8 {
        let offset = (addr - 0x6000) as usize;
        self.prg_ram.get(offset).copied().unwrap_or(0)
    }

    pub(in crate::cartridge) fn write_prg_ram_vrc3(&mut self, addr: u16, data: u8) {
        let offset = (addr - 0x6000) as usize;
        if let Some(slot) = self.prg_ram.get_mut(offset) {
            *slot = data;
        }
    }
}
