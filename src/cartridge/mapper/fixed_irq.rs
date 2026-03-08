use std::cell::Cell;

use super::super::Cartridge;

#[derive(Debug, Clone)]
pub(in crate::cartridge) struct Mapper40 {
    pub(in crate::cartridge) irq_counter: u16,
    pub(in crate::cartridge) irq_enabled: bool,
    pub(in crate::cartridge) irq_pending: Cell<bool>,
}

impl Mapper40 {
    pub(in crate::cartridge) fn new() -> Self {
        Self {
            irq_counter: 0,
            irq_enabled: false,
            irq_pending: Cell::new(false),
        }
    }

    pub(in crate::cartridge) fn clock_irq_mut(&mut self, cycles: u32) {
        if !self.irq_enabled {
            return;
        }

        let next = self.irq_counter as u32 + cycles;
        if next >= 4096 {
            self.irq_counter = 4096;
            self.irq_enabled = false;
            self.irq_pending.set(true);
        } else {
            self.irq_counter = next as u16;
        }
    }
}

#[derive(Debug, Clone)]
pub(in crate::cartridge) struct Mapper50 {
    pub(in crate::cartridge) irq_counter: u16,
    pub(in crate::cartridge) irq_enabled: bool,
    pub(in crate::cartridge) irq_pending: Cell<bool>,
}

impl Mapper50 {
    pub(in crate::cartridge) fn new() -> Self {
        Self {
            irq_counter: 0,
            irq_enabled: false,
            irq_pending: Cell::new(false),
        }
    }

    pub(in crate::cartridge) fn clock_irq_mut(&mut self, cycles: u32) {
        if !self.irq_enabled {
            return;
        }

        let next = self.irq_counter as u32 + cycles;
        if next >= 4096 {
            self.irq_counter = 4096;
            self.irq_enabled = false;
            self.irq_pending.set(true);
        } else {
            self.irq_counter = next as u16;
        }
    }
}

#[derive(Debug, Clone)]
pub(in crate::cartridge) struct Mapper42 {
    pub(in crate::cartridge) irq_counter: u16,
    pub(in crate::cartridge) irq_enabled: bool,
    pub(in crate::cartridge) irq_pending: Cell<bool>,
}

impl Mapper42 {
    pub(in crate::cartridge) fn new() -> Self {
        Self {
            irq_counter: 0,
            irq_enabled: false,
            irq_pending: Cell::new(false),
        }
    }

    pub(in crate::cartridge) fn clock_irq_mut(&mut self, cycles: u32) {
        if !self.irq_enabled {
            return;
        }

        let next = ((self.irq_counter as u32 + cycles) & 0x7FFF) as u16;
        self.irq_counter = next;
        self.irq_pending.set(next >= 0x6000);
    }
}

#[derive(Debug, Clone)]
pub(in crate::cartridge) struct Mapper43 {
    pub(in crate::cartridge) irq_counter: u16,
    pub(in crate::cartridge) irq_enabled: bool,
    pub(in crate::cartridge) irq_pending: Cell<bool>,
}

impl Mapper43 {
    pub(in crate::cartridge) fn new() -> Self {
        Self {
            irq_counter: 0,
            irq_enabled: false,
            irq_pending: Cell::new(false),
        }
    }

    pub(in crate::cartridge) fn clock_irq_mut(&mut self, cycles: u32) {
        if !self.irq_enabled {
            return;
        }

        let next = self.irq_counter as u32 + cycles;
        if next >= 0x1000 {
            self.irq_counter = (next & 0x0FFF) as u16;
            self.irq_pending.set(true);
        } else {
            self.irq_counter = next as u16;
        }
    }
}

impl Cartridge {
    const MAPPER43_C000_BANKS: [u8; 8] = [4, 3, 4, 4, 4, 7, 5, 6];

    fn read_prg_8k_bank(&self, bank: usize, base: u16, addr: u16) -> u8 {
        if self.prg_rom.is_empty() || addr < base {
            return 0;
        }

        let bank_count = (self.prg_rom.len() / 0x2000).max(1);
        let offset = (bank % bank_count) * 0x2000 + (addr - base) as usize;
        self.prg_rom[offset % self.prg_rom.len()]
    }

    pub(in crate::cartridge) fn read_prg_mapper40(&self, addr: u16) -> u8 {
        let bank_count = (self.prg_rom.len() / 0x2000).max(1);
        let fixed_6000 = bank_count.saturating_sub(2);
        let fixed_8000 = bank_count.saturating_sub(4);
        let fixed_a000 = bank_count.saturating_sub(3);
        let fixed_e000 = bank_count.saturating_sub(1);

        match addr {
            0x8000..=0x9FFF => self.read_prg_8k_bank(fixed_8000, 0x8000, addr),
            0xA000..=0xBFFF => self.read_prg_8k_bank(fixed_a000, 0xA000, addr),
            0xC000..=0xDFFF => self.read_prg_8k_bank(self.prg_bank as usize, 0xC000, addr),
            0xE000..=0xFFFF => self.read_prg_8k_bank(fixed_e000, 0xE000, addr),
            _ => self.read_prg_8k_bank(fixed_6000, 0x6000, addr),
        }
    }

    pub(in crate::cartridge) fn read_prg_ram_mapper40(&self, addr: u16) -> u8 {
        let bank_count = (self.prg_rom.len() / 0x2000).max(1);
        let fixed_6000 = bank_count.saturating_sub(2);
        self.read_prg_8k_bank(fixed_6000, 0x6000, addr)
    }

    pub(in crate::cartridge) fn read_prg_mapper42(&self, addr: u16) -> u8 {
        let bank_count = (self.prg_rom.len() / 0x2000).max(1);
        let fixed_8000 = bank_count.saturating_sub(4);
        let fixed_a000 = bank_count.saturating_sub(3);
        let fixed_c000 = bank_count.saturating_sub(2);
        let fixed_e000 = bank_count.saturating_sub(1);

        match addr {
            0x8000..=0x9FFF => self.read_prg_8k_bank(fixed_8000, 0x8000, addr),
            0xA000..=0xBFFF => self.read_prg_8k_bank(fixed_a000, 0xA000, addr),
            0xC000..=0xDFFF => self.read_prg_8k_bank(fixed_c000, 0xC000, addr),
            0xE000..=0xFFFF => self.read_prg_8k_bank(fixed_e000, 0xE000, addr),
            _ => self.read_prg_8k_bank(self.prg_bank as usize, 0x6000, addr),
        }
    }

    pub(in crate::cartridge) fn read_prg_ram_mapper42(&self, addr: u16) -> u8 {
        self.read_prg_8k_bank(self.prg_bank as usize, 0x6000, addr)
    }

    pub(in crate::cartridge) fn read_prg_low_mapper43(&self, addr: u16) -> u8 {
        if self.prg_rom.len() <= 0x10000 {
            return 0;
        }

        match addr {
            0x5000..=0x5FFF => {
                let base = 0x10000;
                let offset = (addr as usize - 0x5000) & 0x07FF;
                self.prg_rom[(base + offset) % self.prg_rom.len()]
            }
            _ => 0,
        }
    }

    pub(in crate::cartridge) fn read_prg_mapper43(&self, addr: u16) -> u8 {
        match addr {
            0x8000..=0x9FFF => self.read_prg_8k_bank(1, 0x8000, addr),
            0xA000..=0xBFFF => self.read_prg_8k_bank(0, 0xA000, addr),
            0xC000..=0xDFFF => {
                let bank = Self::MAPPER43_C000_BANKS[(self.prg_bank & 0x07) as usize] as usize;
                self.read_prg_8k_bank(bank, 0xC000, addr)
            }
            0xE000..=0xFFFF => {
                if self.prg_rom.len() > 0x12000 {
                    let offset = 0x12000 + (addr - 0xE000) as usize;
                    self.prg_rom[offset % self.prg_rom.len()]
                } else {
                    0
                }
            }
            _ => 0,
        }
    }

    pub(in crate::cartridge) fn read_prg_ram_mapper43(&self, addr: u16) -> u8 {
        self.read_prg_8k_bank(2, 0x6000, addr)
    }

    pub(in crate::cartridge) fn write_prg_mapper40(&mut self, addr: u16, data: u8) {
        if let Some(mapper40) = self.mapper40.as_mut() {
            match addr & 0xE000 {
                0x8000 => {
                    mapper40.irq_enabled = false;
                    mapper40.irq_counter = 0;
                    mapper40.irq_pending.set(false);
                }
                0xA000 => {
                    mapper40.irq_enabled = true;
                    mapper40.irq_counter = 0;
                    mapper40.irq_pending.set(false);
                }
                0xE000 => {
                    let bank_count = (self.prg_rom.len() / 0x2000).max(1);
                    self.prg_bank = (((data as usize) & 0x07) % bank_count) as u8;
                }
                _ => {}
            }
        }
    }

    pub(in crate::cartridge) fn read_prg_mapper50(&self, addr: u16) -> u8 {
        match addr {
            0x8000..=0x9FFF => self.read_prg_8k_bank(0x08, 0x8000, addr),
            0xA000..=0xBFFF => self.read_prg_8k_bank(0x09, 0xA000, addr),
            0xC000..=0xDFFF => self.read_prg_8k_bank(self.prg_bank as usize, 0xC000, addr),
            0xE000..=0xFFFF => self.read_prg_8k_bank(0x0B, 0xE000, addr),
            _ => self.read_prg_8k_bank(0x0F, 0x6000, addr),
        }
    }

    pub(in crate::cartridge) fn read_prg_ram_mapper50(&self, addr: u16) -> u8 {
        self.read_prg_8k_bank(0x0F, 0x6000, addr)
    }

    pub(in crate::cartridge) fn write_prg_mapper50(&mut self, addr: u16, data: u8) {
        match addr & 0xD160 {
            0x4120 => {
                if let Some(mapper50) = self.mapper50.as_mut() {
                    mapper50.irq_pending.set(false);
                    mapper50.irq_enabled = data & 0x01 != 0;
                    if !mapper50.irq_enabled {
                        mapper50.irq_counter = 0;
                    }
                }
            }
            0x4020 => {
                let bank_count = (self.prg_rom.len() / 0x2000).max(1);
                let bank = ((data & 0x01) << 2)
                    | ((data & 0x02) >> 1)
                    | ((data & 0x04) >> 1)
                    | (data & 0x08);
                self.prg_bank = (bank as usize % bank_count) as u8;
            }
            _ => {}
        }
    }

    pub(in crate::cartridge) fn write_prg_mapper42(&mut self, addr: u16, data: u8) {
        if addr < 0x8000 {
            return;
        }

        let bank_count = (self.prg_rom.len() / 0x2000).max(1);
        self.prg_bank = ((data as usize & 0x0F) % bank_count) as u8;
        self.mirroring = if data & 0x10 != 0 {
            crate::cartridge::Mirroring::Horizontal
        } else {
            crate::cartridge::Mirroring::Vertical
        };

        if let Some(mapper42) = self.mapper42.as_mut() {
            if data & 0x20 != 0 {
                mapper42.irq_enabled = true;
            } else {
                mapper42.irq_enabled = false;
                mapper42.irq_counter = 0;
                mapper42.irq_pending.set(false);
            }
        }
    }

    pub(in crate::cartridge) fn write_prg_mapper43(&mut self, addr: u16, data: u8) {
        match addr {
            0x4022 => {
                self.prg_bank = data & 0x07;
            }
            0x4122 | 0x8122 => {
                if let Some(mapper43) = self.mapper43.as_mut() {
                    if data & 0x01 != 0 {
                        mapper43.irq_enabled = true;
                    } else {
                        mapper43.irq_enabled = false;
                        mapper43.irq_counter = 0;
                        mapper43.irq_pending.set(false);
                    }
                }
            }
            _ => {}
        }
    }
}
