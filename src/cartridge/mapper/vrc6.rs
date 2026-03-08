use std::cell::Cell;

use super::super::{Cartridge, Mirroring};

const VRC6_AUDIO_SCALE: f32 = 0.35 / 63.0;

#[derive(Debug, Clone)]
pub(in crate::cartridge) struct Vrc6Pulse {
    pub(in crate::cartridge) volume: u8,
    pub(in crate::cartridge) duty: u8,
    pub(in crate::cartridge) ignore_duty: bool,
    pub(in crate::cartridge) period: u16,
    pub(in crate::cartridge) enabled: bool,
    pub(in crate::cartridge) step: u8,
    pub(in crate::cartridge) divider: u16,
}

impl Vrc6Pulse {
    fn new() -> Self {
        Self {
            volume: 0,
            duty: 0,
            ignore_duty: false,
            period: 0,
            enabled: false,
            step: 15,
            divider: 0,
        }
    }

    fn write_control(&mut self, data: u8) {
        self.volume = data & 0x0F;
        self.duty = (data >> 4) & 0x07;
        self.ignore_duty = data & 0x80 != 0;
    }

    fn write_period_low(&mut self, data: u8) {
        self.period = (self.period & 0x0F00) | data as u16;
    }

    fn write_period_high(&mut self, data: u8) {
        self.period = (self.period & 0x00FF) | (((data & 0x0F) as u16) << 8);
        self.enabled = data & 0x80 != 0;
        if !self.enabled {
            self.step = 15;
        }
    }

    fn effective_period(&self, shift: u8) -> u16 {
        (self.period >> shift).max(1)
    }

    fn clock(&mut self, shift: u8, halt: bool) {
        if halt || !self.enabled {
            return;
        }

        if self.divider == 0 {
            self.divider = self.effective_period(shift);
            self.step = self.step.wrapping_sub(1) & 0x0F;
        } else {
            self.divider -= 1;
        }
    }

    fn output(&self) -> u8 {
        if !self.enabled {
            return 0;
        }
        if self.ignore_duty || self.step <= self.duty {
            self.volume
        } else {
            0
        }
    }
}

#[derive(Debug, Clone)]
pub(in crate::cartridge) struct Vrc6Saw {
    pub(in crate::cartridge) rate: u8,
    pub(in crate::cartridge) period: u16,
    pub(in crate::cartridge) enabled: bool,
    pub(in crate::cartridge) step: u8,
    pub(in crate::cartridge) divider: u16,
    pub(in crate::cartridge) accumulator: u8,
}

impl Vrc6Saw {
    fn new() -> Self {
        Self {
            rate: 0,
            period: 0,
            enabled: false,
            step: 0,
            divider: 0,
            accumulator: 0,
        }
    }

    fn write_rate(&mut self, data: u8) {
        self.rate = data & 0x3F;
    }

    fn write_period_low(&mut self, data: u8) {
        self.period = (self.period & 0x0F00) | data as u16;
    }

    fn write_period_high(&mut self, data: u8) {
        self.period = (self.period & 0x00FF) | (((data & 0x0F) as u16) << 8);
        self.enabled = data & 0x80 != 0;
        if !self.enabled {
            self.step = 0;
            self.accumulator = 0;
        }
    }

    fn effective_period(&self, shift: u8) -> u16 {
        (self.period >> shift).max(1)
    }

    fn clock(&mut self, shift: u8, halt: bool) {
        if halt || !self.enabled {
            return;
        }

        if self.divider == 0 {
            self.divider = self.effective_period(shift);
            self.step = (self.step + 1) % 14;
            if self.step == 0 {
                self.accumulator = 0;
            } else if self.step & 1 == 0 {
                self.accumulator = self.accumulator.wrapping_add(self.rate);
            }
        } else {
            self.divider -= 1;
        }
    }

    fn output(&self) -> u8 {
        if self.enabled {
            self.accumulator >> 3
        } else {
            0
        }
    }
}

#[derive(Debug, Clone)]
pub(in crate::cartridge) struct Vrc6 {
    pub(in crate::cartridge) prg_bank_16k: u8,
    pub(in crate::cartridge) prg_bank_8k: u8,
    pub(in crate::cartridge) chr_banks: [u8; 8],
    pub(in crate::cartridge) banking_control: u8,
    pub(in crate::cartridge) irq_latch: u8,
    pub(in crate::cartridge) irq_counter: u8,
    pub(in crate::cartridge) irq_enable_after_ack: bool,
    pub(in crate::cartridge) irq_enabled: bool,
    pub(in crate::cartridge) irq_cycle_mode: bool,
    pub(in crate::cartridge) irq_prescaler: i16,
    pub(in crate::cartridge) irq_pending: Cell<bool>,
    pub(in crate::cartridge) audio_halt: bool,
    pub(in crate::cartridge) audio_freq_shift: u8,
    pub(in crate::cartridge) pulse1: Vrc6Pulse,
    pub(in crate::cartridge) pulse2: Vrc6Pulse,
    pub(in crate::cartridge) saw: Vrc6Saw,
}

impl Vrc6 {
    pub(in crate::cartridge) fn new() -> Self {
        Self {
            prg_bank_16k: 0,
            prg_bank_8k: 0,
            chr_banks: [0; 8],
            banking_control: 0,
            irq_latch: 0,
            irq_counter: 0,
            irq_enable_after_ack: false,
            irq_enabled: false,
            irq_cycle_mode: false,
            irq_prescaler: 341,
            irq_pending: Cell::new(false),
            audio_halt: false,
            audio_freq_shift: 0,
            pulse1: Vrc6Pulse::new(),
            pulse2: Vrc6Pulse::new(),
            saw: Vrc6Saw::new(),
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

    pub(in crate::cartridge) fn clock_audio(&mut self) -> f32 {
        self.pulse1.clock(self.audio_freq_shift, self.audio_halt);
        self.pulse2.clock(self.audio_freq_shift, self.audio_halt);
        self.saw.clock(self.audio_freq_shift, self.audio_halt);

        let mix =
            self.pulse1.output() as f32 + self.pulse2.output() as f32 + self.saw.output() as f32;
        mix * VRC6_AUDIO_SCALE
    }
}

impl Cartridge {
    fn vrc6_normalize_addr(&self, addr: u16) -> u16 {
        let low = addr & 0x0003;
        let low = if self.mapper == 26 {
            ((low & 0x0001) << 1) | ((low & 0x0002) >> 1)
        } else {
            low
        };
        (addr & 0xF000) | low
    }

    fn vrc6_chr_data(&self) -> &[u8] {
        if !self.chr_ram.is_empty() {
            &self.chr_ram
        } else {
            &self.chr_rom
        }
    }

    pub(in crate::cartridge) fn vrc6_apply_banking_control(&mut self, data: u8) {
        self.mirroring = match data & 0x0C {
            0x00 => Mirroring::Vertical,
            0x04 => Mirroring::Horizontal,
            0x08 => Mirroring::OneScreenLower,
            _ => Mirroring::OneScreenUpper,
        };
    }

    pub(in crate::cartridge) fn read_prg_vrc6(&self, addr: u16) -> u8 {
        let Some(vrc6) = self.vrc6.as_ref() else {
            return 0;
        };
        let bank_count_16k = (self.prg_rom.len() / 0x4000).max(1);
        let bank_count_8k = (self.prg_rom.len() / 0x2000).max(1);

        let prg_addr = match addr {
            0x8000..=0xBFFF => {
                let bank = vrc6.prg_bank_16k as usize % bank_count_16k;
                bank * 0x4000 + (addr as usize - 0x8000)
            }
            0xC000..=0xDFFF => {
                let bank = vrc6.prg_bank_8k as usize % bank_count_8k;
                bank * 0x2000 + (addr as usize - 0xC000)
            }
            0xE000..=0xFFFF => {
                let bank = bank_count_8k - 1;
                bank * 0x2000 + (addr as usize - 0xE000)
            }
            _ => return 0,
        };
        self.prg_rom.get(prg_addr).copied().unwrap_or(0)
    }

    pub(in crate::cartridge) fn write_prg_vrc6(&mut self, addr: u16, data: u8) {
        let normalized = self.vrc6_normalize_addr(addr);
        let Some(vrc6) = self.vrc6.as_mut() else {
            return;
        };
        let mut prg_bank = None;
        let mut chr_bank = None;
        let mut apply_banking_control = None;

        match normalized & 0xF003 {
            0x8000..=0x8003 => {
                vrc6.prg_bank_16k = data & 0x0F;
                prg_bank = Some(vrc6.prg_bank_16k);
            }
            0x9000 => vrc6.pulse1.write_control(data),
            0x9001 => vrc6.pulse1.write_period_low(data),
            0x9002 => vrc6.pulse1.write_period_high(data),
            0x9003 => {
                vrc6.audio_halt = data & 0x01 != 0;
                vrc6.audio_freq_shift = if data & 0x04 != 0 {
                    8
                } else if data & 0x02 != 0 {
                    4
                } else {
                    0
                };
            }
            0xA000 => vrc6.pulse2.write_control(data),
            0xA001 => vrc6.pulse2.write_period_low(data),
            0xA002 => vrc6.pulse2.write_period_high(data),
            0xB000 => vrc6.saw.write_rate(data),
            0xB001 => vrc6.saw.write_period_low(data),
            0xB002 => vrc6.saw.write_period_high(data),
            0xB003 => {
                vrc6.banking_control = data;
                apply_banking_control = Some(data);
            }
            0xC000..=0xC003 => vrc6.prg_bank_8k = data & 0x1F,
            0xD000..=0xD003 => {
                let index = (normalized & 0x0003) as usize;
                vrc6.chr_banks[index] = data;
                if index == 0 {
                    chr_bank = Some(data);
                }
            }
            0xE000..=0xE003 => {
                let index = 4 + (normalized & 0x0003) as usize;
                vrc6.chr_banks[index] = data;
            }
            0xF000 => vrc6.irq_latch = data,
            0xF001 => {
                vrc6.irq_enable_after_ack = data & 0x01 != 0;
                vrc6.irq_enabled = data & 0x02 != 0;
                vrc6.irq_cycle_mode = data & 0x04 != 0;
                vrc6.irq_pending.set(false);
                vrc6.irq_prescaler = 341;
                if vrc6.irq_enabled {
                    vrc6.irq_counter = vrc6.irq_latch;
                }
            }
            0xF002 => {
                vrc6.irq_pending.set(false);
                vrc6.irq_enabled = vrc6.irq_enable_after_ack;
            }
            _ => {}
        }

        if let Some(bank) = prg_bank {
            self.prg_bank = bank;
        }
        if let Some(bank) = chr_bank {
            self.chr_bank = bank;
        }
        if let Some(control) = apply_banking_control {
            self.vrc6_apply_banking_control(control);
        }
    }

    pub(in crate::cartridge) fn read_chr_vrc6(&self, addr: u16) -> u8 {
        let Some(vrc6) = self.vrc6.as_ref() else {
            return 0;
        };
        let chr_data = self.vrc6_chr_data();
        if chr_data.is_empty() {
            return 0;
        }

        let bank_count = (chr_data.len() / 0x0400).max(1);
        let slot = ((addr as usize) >> 10) & 7;
        let bank = vrc6.chr_banks[slot] as usize % bank_count;
        let chr_addr = bank * 0x0400 + (addr as usize & 0x03FF);
        chr_data[chr_addr % chr_data.len()]
    }

    pub(in crate::cartridge) fn write_chr_vrc6(&mut self, addr: u16, data: u8) {
        if self.chr_ram.is_empty() {
            return;
        }
        let chr_ram_len = self.chr_ram.len();
        let bank_count = (chr_ram_len / 0x0400).max(1);
        let slot = ((addr as usize) >> 10) & 7;
        let bank = {
            let Some(vrc6) = self.vrc6.as_ref() else {
                return;
            };
            vrc6.chr_banks[slot] as usize % bank_count
        };
        let chr_addr = bank * 0x0400 + (addr as usize & 0x03FF);
        if let Some(cell) = self.chr_ram.get_mut(chr_addr % chr_ram_len) {
            *cell = data;
        }
    }

    pub(in crate::cartridge) fn read_prg_ram_vrc6(&self, addr: u16) -> u8 {
        let Some(vrc6) = self.vrc6.as_ref() else {
            return 0;
        };
        if vrc6.banking_control & 0x80 == 0 {
            return 0;
        }
        let offset = (addr as usize).saturating_sub(0x6000);
        self.prg_ram.get(offset).copied().unwrap_or(0)
    }

    pub(in crate::cartridge) fn write_prg_ram_vrc6(&mut self, addr: u16, data: u8) {
        let wram_enabled = self
            .vrc6
            .as_ref()
            .map(|vrc6| vrc6.banking_control & 0x80 != 0)
            .unwrap_or(false);
        if !wram_enabled {
            return;
        }
        let offset = (addr as usize).saturating_sub(0x6000);
        if let Some(cell) = self.prg_ram.get_mut(offset) {
            *cell = data;
        }
    }

    pub(in crate::cartridge) fn clock_irq_vrc6(&mut self, cycles: u32) {
        if let Some(vrc6) = self.vrc6.as_mut() {
            vrc6.clock_irq_mut(cycles);
        }
    }

    pub(in crate::cartridge) fn clock_audio_vrc6(&mut self) -> f32 {
        if let Some(vrc6) = self.vrc6.as_mut() {
            vrc6.clock_audio()
        } else {
            0.0
        }
    }
}
