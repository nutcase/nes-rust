use super::super::Cartridge;
use std::cell::Cell;

const NAMCO163_WRAM_LEN: usize = 0x2000;
const NAMCO163_INTERNAL_RAM_LEN: usize = 0x80;

#[derive(Debug, Clone)]
pub(in crate::cartridge) struct Namco163 {
    pub(in crate::cartridge) chr_banks: [u8; 12],
    pub(in crate::cartridge) prg_banks: [u8; 3],
    pub(in crate::cartridge) sound_disable: bool,
    pub(in crate::cartridge) chr_nt_disabled_low: bool,
    pub(in crate::cartridge) chr_nt_disabled_high: bool,
    pub(in crate::cartridge) wram_write_enable: bool,
    pub(in crate::cartridge) wram_write_protect: u8,
    pub(in crate::cartridge) internal_addr: Cell<u8>,
    pub(in crate::cartridge) internal_auto_increment: bool,
    pub(in crate::cartridge) irq_counter: u16,
    pub(in crate::cartridge) irq_enabled: bool,
    pub(in crate::cartridge) irq_pending: Cell<bool>,
    pub(in crate::cartridge) audio_delay: u8,
    pub(in crate::cartridge) audio_channel_index: u8,
    pub(in crate::cartridge) audio_outputs: [f32; 8],
    pub(in crate::cartridge) audio_current: f32,
}

impl Namco163 {
    pub(in crate::cartridge) fn new() -> Self {
        Self {
            chr_banks: [0; 12],
            prg_banks: [0, 1, 2],
            sound_disable: true,
            chr_nt_disabled_low: false,
            chr_nt_disabled_high: false,
            wram_write_enable: false,
            wram_write_protect: 0x0F,
            internal_addr: Cell::new(0),
            internal_auto_increment: false,
            irq_counter: 0,
            irq_enabled: false,
            irq_pending: Cell::new(false),
            audio_delay: 14,
            audio_channel_index: 0,
            audio_outputs: [0.0; 8],
            audio_current: 0.0,
        }
    }

    fn chip_ram_addr(&self) -> usize {
        NAMCO163_WRAM_LEN + (self.internal_addr.get() as usize & 0x7F)
    }

    fn active_channels(chip_ram: &[u8]) -> u8 {
        (((chip_ram[0x7F] >> 4) & 0x07) + 1).clamp(1, 8)
    }

    fn channel_base(active_channels: u8, channel_index: u8) -> usize {
        0x40 + ((8 - active_channels + channel_index) as usize) * 8
    }

    fn clock_audio_channel(&mut self, chip_ram: &mut [u8]) {
        let active = Self::active_channels(chip_ram);
        let channel_index = self.audio_channel_index.min(active - 1);
        let base = Self::channel_base(active, channel_index);

        let freq = chip_ram[base] as u32
            | ((chip_ram[base + 2] as u32) << 8)
            | (((chip_ram[base + 4] as u32) & 0x03) << 16);
        let mut phase = chip_ram[base + 1] as u32
            | ((chip_ram[base + 3] as u32) << 8)
            | ((chip_ram[base + 5] as u32) << 16);
        let length = 256u32 - (chip_ram[base + 4] as u32 & 0xFC);
        let wave_address = chip_ram[base + 6] as u32;
        let volume = (chip_ram[base + 7] & 0x0F) as i32;

        let sample = if length == 0 || volume == 0 {
            0.0
        } else {
            phase = (phase + freq) % (length << 16);
            let sample_index = (((phase >> 16) + wave_address) & 0xFF) as usize;
            let packed = chip_ram[sample_index >> 1];
            let nibble = if sample_index & 1 == 0 {
                packed & 0x0F
            } else {
                (packed >> 4) & 0x0F
            };

            chip_ram[base + 1] = phase as u8;
            chip_ram[base + 3] = (phase >> 8) as u8;
            chip_ram[base + 5] = (phase >> 16) as u8;

            ((nibble as i32 - 8) * volume) as f32
        };

        self.audio_outputs[channel_index as usize] = sample;
        for index in active as usize..8 {
            self.audio_outputs[index] = 0.0;
        }
        self.audio_current =
            self.audio_outputs[..active as usize].iter().sum::<f32>() / active as f32 / 32.0;
        self.audio_channel_index = if active <= 1 {
            0
        } else {
            (channel_index + 1) % active
        };
    }
}

impl Cartridge {
    fn namco163_chr_rom_bank_count_1k(&self) -> usize {
        (self.chr_rom.len() / 0x0400).max(1)
    }

    fn namco163_prg_rom_bank_count_8k(&self) -> usize {
        (self.prg_rom.len() / 0x2000).max(1)
    }

    fn namco163_ciram_addr(bank: u8, offset: usize) -> usize {
        ((bank as usize) & 1) * 0x0400 + offset
    }

    fn namco163_chr_bank_uses_ciram(&self, slot: usize, bank: u8) -> bool {
        if bank < 0xE0 {
            return false;
        }
        match slot {
            0..=3 => self
                .namco163
                .as_ref()
                .map(|n| !n.chr_nt_disabled_low)
                .unwrap_or(false),
            4..=7 => self
                .namco163
                .as_ref()
                .map(|n| !n.chr_nt_disabled_high)
                .unwrap_or(false),
            _ => true,
        }
    }

    fn read_namco163_chr_bank(&self, bank: u8, offset: usize, slot: usize) -> u8 {
        if self.namco163_chr_bank_uses_ciram(slot, bank) {
            let ciram_addr = Self::namco163_ciram_addr(bank, offset);
            return self.chr_ram.get(ciram_addr).copied().unwrap_or(0);
        }

        let bank_count = self.namco163_chr_rom_bank_count_1k();
        let chr_addr = ((bank as usize % bank_count) * 0x0400) + offset;
        self.chr_rom.get(chr_addr).copied().unwrap_or(0)
    }

    fn write_namco163_chr_bank(&mut self, bank: u8, offset: usize, slot: usize, data: u8) {
        if self.namco163_chr_bank_uses_ciram(slot, bank) {
            let ciram_addr = Self::namco163_ciram_addr(bank, offset);
            if let Some(cell) = self.chr_ram.get_mut(ciram_addr) {
                *cell = data;
            }
        }
    }

    pub(in crate::cartridge) fn read_prg_namco163(&self, addr: u16) -> u8 {
        let Some(namco163) = self.namco163.as_ref() else {
            return 0;
        };
        let bank_count = self.namco163_prg_rom_bank_count_8k();
        let last_bank = bank_count - 1;

        let (bank, base_addr) = match addr {
            0x8000..=0x9FFF => (namco163.prg_banks[0] as usize, 0x8000),
            0xA000..=0xBFFF => (namco163.prg_banks[1] as usize, 0xA000),
            0xC000..=0xDFFF => (namco163.prg_banks[2] as usize, 0xC000),
            0xE000..=0xFFFF => (last_bank, 0xE000),
            _ => return 0,
        };

        let prg_addr = (bank % bank_count) * 0x2000 + (addr - base_addr) as usize;
        self.prg_rom.get(prg_addr).copied().unwrap_or(0)
    }

    pub(in crate::cartridge) fn write_prg_namco163(&mut self, addr: u16, data: u8) {
        let Some(namco163) = self.namco163.as_mut() else {
            return;
        };

        match addr {
            0x8000..=0xDFFF => {
                let index = ((addr - 0x8000) / 0x0800) as usize;
                namco163.chr_banks[index] = data;
            }
            0xE000..=0xE7FF => {
                namco163.sound_disable = data & 0x40 != 0;
                namco163.prg_banks[0] = data & 0x3F;
            }
            0xE800..=0xEFFF => {
                namco163.chr_nt_disabled_low = data & 0x40 != 0;
                namco163.chr_nt_disabled_high = data & 0x80 != 0;
                namco163.prg_banks[1] = data & 0x3F;
            }
            0xF000..=0xF7FF => {
                namco163.prg_banks[2] = data & 0x3F;
            }
            0xF800..=0xFFFF => {
                namco163.wram_write_enable = (data & 0xF0) == 0x40;
                namco163.wram_write_protect = data & 0x0F;
                namco163.internal_auto_increment = data & 0x80 != 0;
                namco163.internal_addr.set(data & 0x7F);
            }
            _ => {}
        }
    }

    pub(in crate::cartridge) fn read_chr_namco163(&self, addr: u16) -> u8 {
        let Some(namco163) = self.namco163.as_ref() else {
            return 0;
        };
        let slot = ((addr as usize) >> 10) & 7;
        let bank = namco163.chr_banks[slot];
        self.read_namco163_chr_bank(bank, addr as usize & 0x03FF, slot)
    }

    pub(in crate::cartridge) fn write_chr_namco163(&mut self, addr: u16, data: u8) {
        let Some(namco163) = self.namco163.as_ref() else {
            return;
        };
        let slot = ((addr as usize) >> 10) & 7;
        let bank = namco163.chr_banks[slot];
        self.write_namco163_chr_bank(bank, addr as usize & 0x03FF, slot, data);
    }

    pub(in crate::cartridge) fn read_prg_low_namco163(&self, addr: u16) -> u8 {
        let Some(namco163) = self.namco163.as_ref() else {
            return 0;
        };

        match addr {
            0x4800..=0x4FFF => {
                let chip_addr = namco163.chip_ram_addr();
                let value = self.prg_ram.get(chip_addr).copied().unwrap_or(0);
                if namco163.internal_auto_increment {
                    namco163
                        .internal_addr
                        .set(namco163.internal_addr.get().wrapping_add(1) & 0x7F);
                }
                value
            }
            0x5000..=0x57FF => namco163.irq_counter as u8,
            0x5800..=0x5FFF => {
                ((namco163.irq_enabled as u8) << 7) | ((namco163.irq_counter >> 8) as u8 & 0x7F)
            }
            _ => 0,
        }
    }

    pub(in crate::cartridge) fn write_prg_low_namco163(&mut self, addr: u16, data: u8) {
        let Some(namco163) = self.namco163.as_mut() else {
            return;
        };

        match addr {
            0x4800..=0x4FFF => {
                let chip_addr = namco163.chip_ram_addr();
                if let Some(cell) = self.prg_ram.get_mut(chip_addr) {
                    *cell = data;
                    if self.has_battery {
                        self.has_valid_save_data = true;
                    }
                }
                if namco163.internal_auto_increment {
                    namco163
                        .internal_addr
                        .set(namco163.internal_addr.get().wrapping_add(1) & 0x7F);
                }
            }
            0x5000..=0x57FF => {
                namco163.irq_counter = (namco163.irq_counter & 0x7F00) | data as u16;
                namco163.irq_pending.set(false);
            }
            0x5800..=0x5FFF => {
                namco163.irq_enabled = data & 0x80 != 0;
                namco163.irq_counter =
                    (namco163.irq_counter & 0x00FF) | (((data & 0x7F) as u16) << 8);
                namco163.irq_pending.set(false);
            }
            _ => {}
        }
    }

    pub(in crate::cartridge) fn read_prg_ram_namco163(&self, addr: u16) -> u8 {
        if self.prg_ram.len() < NAMCO163_WRAM_LEN {
            return 0;
        }
        self.prg_ram[(addr as usize - 0x6000) & 0x1FFF]
    }

    pub(in crate::cartridge) fn write_prg_ram_namco163(&mut self, addr: u16, data: u8) {
        let Some(namco163) = self.namco163.as_ref() else {
            return;
        };
        let window = ((addr as usize - 0x6000) >> 11) & 0x03;
        if !namco163.wram_write_enable || (namco163.wram_write_protect >> window) & 1 != 0 {
            return;
        }
        if self.prg_ram.len() >= NAMCO163_WRAM_LEN {
            self.prg_ram[(addr as usize - 0x6000) & 0x1FFF] = data;
            if self.has_battery {
                self.has_valid_save_data = true;
            }
        }
    }

    pub(in crate::cartridge) fn read_nametable_namco163(
        &self,
        logical_nt: usize,
        offset: usize,
        internal: &[[u8; 1024]; 2],
    ) -> u8 {
        let Some(namco163) = self.namco163.as_ref() else {
            return internal[logical_nt & 1][offset];
        };
        let bank = namco163.chr_banks[8 + (logical_nt & 3)];
        if bank >= 0xE0 {
            let ciram_addr = Self::namco163_ciram_addr(bank, offset);
            self.chr_ram.get(ciram_addr).copied().unwrap_or(0)
        } else {
            let bank_count = self.namco163_chr_rom_bank_count_1k();
            let chr_addr = (bank as usize % bank_count) * 0x0400 + offset;
            self.chr_rom.get(chr_addr).copied().unwrap_or(0)
        }
    }

    pub(in crate::cartridge) fn write_nametable_namco163(
        &mut self,
        logical_nt: usize,
        offset: usize,
        _internal: &mut [[u8; 1024]; 2],
        data: u8,
    ) {
        let Some(namco163) = self.namco163.as_ref() else {
            return;
        };
        let bank = namco163.chr_banks[8 + (logical_nt & 3)];
        if bank >= 0xE0 {
            let ciram_addr = Self::namco163_ciram_addr(bank, offset);
            if let Some(cell) = self.chr_ram.get_mut(ciram_addr) {
                *cell = data;
            }
        }
    }

    pub(in crate::cartridge) fn clock_irq_namco163(&mut self, cycles: u32) {
        let Some(namco163) = self.namco163.as_mut() else {
            return;
        };
        if !namco163.irq_enabled || namco163.irq_pending.get() {
            return;
        }
        let remaining = 0x7FFFu32.saturating_sub(namco163.irq_counter as u32);
        if cycles >= remaining {
            namco163.irq_counter = 0x7FFF;
            namco163.irq_pending.set(true);
        } else {
            namco163.irq_counter = ((namco163.irq_counter as u32 + cycles) & 0x7FFF) as u16;
        }
    }

    pub(in crate::cartridge) fn clock_audio_namco163(&mut self) -> f32 {
        let Some(namco163) = self.namco163.as_mut() else {
            return 0.0;
        };
        if namco163.sound_disable {
            namco163.audio_current = 0.0;
            return 0.0;
        }

        if namco163.audio_delay == 0 {
            namco163.audio_delay = 14;
            if self.prg_ram.len() >= NAMCO163_WRAM_LEN + NAMCO163_INTERNAL_RAM_LEN {
                let chip_ram = &mut self.prg_ram
                    [NAMCO163_WRAM_LEN..NAMCO163_WRAM_LEN + NAMCO163_INTERNAL_RAM_LEN];
                namco163.clock_audio_channel(chip_ram);
            }
        } else {
            namco163.audio_delay -= 1;
        }

        namco163.audio_current
    }
}
