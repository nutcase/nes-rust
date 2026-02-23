use std::cell::Cell;

use super::super::{Cartridge, Mirroring};

/// YM2149F-compatible (Sunsoft 5B) expansion audio DAC volume table.
/// Pre-scaled for NES mixing: ~0.12 max per channel, 3 channels total ~0.36.
/// Logarithmic curve (~3 dB per step) matching the hardware DAC.
const AY_VOLUME: [f32; 16] = [
    0.0,
    0.00095, 0.00134, 0.00190, 0.00268, 0.00379, 0.00535, 0.00755,
    0.01067, 0.01506, 0.02128, 0.03006, 0.04247, 0.05999, 0.08474, 0.11973,
];

/// Sunsoft 5B expansion audio (YM2149F / AY-3-8910 compatible).
/// 3 square wave channels + noise generator + envelope generator.
#[derive(Debug, Clone)]
pub(in crate::cartridge) struct Sunsoft5BAudio {
    register_select: u8,

    // Tone generators (3 channels)
    tone_period: [u16; 3],   // 12-bit period
    tone_counter: [u16; 3],
    tone_output: [bool; 3],  // Current square wave state

    // Noise generator
    noise_period: u8,        // 5-bit period
    noise_counter: u8,
    noise_lfsr: u32,         // 17-bit LFSR
    noise_output: bool,

    // Mixer (register 7): bits 0-2 = tone disable A/B/C, bits 3-5 = noise disable A/B/C
    mixer: u8,

    // Per-channel volume (bits 0-3 = volume, bit 4 = envelope mode)
    volume: [u8; 3],

    // Envelope generator
    envelope_period: u16,
    envelope_counter: u16,
    envelope_shape: u8,
    envelope_volume: u8,     // 0-15
    envelope_holding: bool,
    envelope_up: bool,       // true = attack (counting up)

    // CPU clock prescaler (divides by 16)
    prescaler: u8,

    // Cached output (only changes at prescaler tick)
    last_output: f32,
}

impl Sunsoft5BAudio {
    pub(in crate::cartridge) fn new() -> Self {
        Sunsoft5BAudio {
            register_select: 0,
            tone_period: [0; 3],
            tone_counter: [1; 3],
            tone_output: [false; 3],
            noise_period: 0,
            noise_counter: 1,
            noise_lfsr: 1,
            noise_output: false,
            mixer: 0xFF, // All outputs disabled by default
            volume: [0; 3],
            envelope_period: 0,
            envelope_counter: 1,
            envelope_shape: 0,
            envelope_volume: 0,
            envelope_holding: true,
            envelope_up: false,
            prescaler: 0,
            last_output: 0.0,
        }
    }

    pub(in crate::cartridge) fn write_select(&mut self, data: u8) {
        self.register_select = data & 0x0F;
    }

    pub(in crate::cartridge) fn write_data(&mut self, data: u8) {
        match self.register_select {
            0 => self.tone_period[0] = (self.tone_period[0] & 0xF00) | data as u16,
            1 => self.tone_period[0] = (self.tone_period[0] & 0x0FF) | ((data as u16 & 0x0F) << 8),
            2 => self.tone_period[1] = (self.tone_period[1] & 0xF00) | data as u16,
            3 => self.tone_period[1] = (self.tone_period[1] & 0x0FF) | ((data as u16 & 0x0F) << 8),
            4 => self.tone_period[2] = (self.tone_period[2] & 0xF00) | data as u16,
            5 => self.tone_period[2] = (self.tone_period[2] & 0x0FF) | ((data as u16 & 0x0F) << 8),
            6 => self.noise_period = data & 0x1F,
            7 => self.mixer = data,
            8 => self.volume[0] = data & 0x1F,
            9 => self.volume[1] = data & 0x1F,
            10 => self.volume[2] = data & 0x1F,
            11 => self.envelope_period = (self.envelope_period & 0xFF00) | data as u16,
            12 => self.envelope_period = (self.envelope_period & 0x00FF) | ((data as u16) << 8),
            13 => {
                self.envelope_shape = data & 0x0F;
                // Reset envelope on shape write
                self.envelope_up = (data & 0x04) != 0; // ATT bit
                self.envelope_volume = if self.envelope_up { 0 } else { 15 };
                self.envelope_holding = false;
                self.envelope_counter = self.envelope_period.max(1);
            }
            _ => {}
        }
    }

    /// Clock one CPU cycle. Returns expansion audio output.
    pub(in crate::cartridge) fn clock(&mut self) -> f32 {
        self.prescaler += 1;
        if self.prescaler >= 16 {
            self.prescaler = 0;
            self.clock_internal();
            self.last_output = self.compute_output();
        }
        self.last_output
    }

    fn clock_internal(&mut self) {
        // Clock tone generators
        for ch in 0..3 {
            if self.tone_counter[ch] > 0 {
                self.tone_counter[ch] -= 1;
            }
            if self.tone_counter[ch] == 0 {
                self.tone_counter[ch] = self.tone_period[ch].max(1);
                self.tone_output[ch] = !self.tone_output[ch];
            }
        }

        // Clock noise generator (17-bit LFSR, taps at bits 0 and 3)
        if self.noise_counter > 0 {
            self.noise_counter -= 1;
        }
        if self.noise_counter == 0 {
            self.noise_counter = self.noise_period.max(1);
            let feedback = (self.noise_lfsr ^ (self.noise_lfsr >> 3)) & 1;
            self.noise_lfsr = (self.noise_lfsr >> 1) | (feedback << 16);
            self.noise_output = (self.noise_lfsr & 1) != 0;
        }

        // Clock envelope generator
        if !self.envelope_holding {
            if self.envelope_counter > 0 {
                self.envelope_counter -= 1;
            }
            if self.envelope_counter == 0 {
                self.envelope_counter = self.envelope_period.max(1);
                self.step_envelope();
            }
        }
    }

    fn step_envelope(&mut self) {
        if self.envelope_up {
            if self.envelope_volume < 15 {
                self.envelope_volume += 1;
            } else {
                self.handle_envelope_boundary();
            }
        } else {
            if self.envelope_volume > 0 {
                self.envelope_volume -= 1;
            } else {
                self.handle_envelope_boundary();
            }
        }
    }

    fn handle_envelope_boundary(&mut self) {
        let cont = (self.envelope_shape & 0x08) != 0;
        let alt = (self.envelope_shape & 0x02) != 0;
        let hold = (self.envelope_shape & 0x01) != 0;

        if !cont {
            // CONT=0: one cycle then hold at 0
            self.envelope_volume = 0;
            self.envelope_holding = true;
        } else if hold {
            // CONT=1, HOLD=1: hold at current or opposite end
            if alt {
                self.envelope_volume = if self.envelope_up { 0 } else { 15 };
            }
            self.envelope_holding = true;
        } else if alt {
            // CONT=1, HOLD=0, ALT=1: reverse direction
            self.envelope_up = !self.envelope_up;
        } else {
            // CONT=1, HOLD=0, ALT=0: repeat from start
            self.envelope_volume = if self.envelope_up { 0 } else { 15 };
        }
    }

    fn compute_output(&self) -> f32 {
        let mut total = 0.0f32;

        for ch in 0..3 {
            let tone_disable = (self.mixer >> ch) & 1 != 0;
            let noise_disable = (self.mixer >> (ch + 3)) & 1 != 0;

            // Channel gate: (tone OR tone_disabled) AND (noise OR noise_disabled)
            let gate = (self.tone_output[ch] || tone_disable)
                && (self.noise_output || noise_disable);

            if gate {
                let vol_reg = self.volume[ch];
                let vol_level = if vol_reg & 0x10 != 0 {
                    // Envelope mode
                    self.envelope_volume
                } else {
                    vol_reg & 0x0F
                };
                total += AY_VOLUME[vol_level as usize];
            }
        }

        total
    }
}

#[derive(Debug, Clone)]
pub(in crate::cartridge) struct Fme7 {
    pub(in crate::cartridge) command: u8,
    pub(in crate::cartridge) chr_banks: [u8; 8],
    pub(in crate::cartridge) prg_banks: [u8; 3],
    pub(in crate::cartridge) prg_bank_6000: u8,
    pub(in crate::cartridge) prg_ram_enabled: bool,
    pub(in crate::cartridge) prg_ram_select: bool,
    pub(in crate::cartridge) irq_counter: u16,
    pub(in crate::cartridge) irq_counter_enabled: bool,
    pub(in crate::cartridge) irq_enabled: bool,
    pub(in crate::cartridge) irq_pending: Cell<bool>,
    pub(in crate::cartridge) audio: Sunsoft5BAudio,
}

impl Fme7 {
    pub(in crate::cartridge) fn new() -> Self {
        Fme7 {
            command: 0,
            chr_banks: [0; 8],
            prg_banks: [0; 3],
            prg_bank_6000: 0,
            prg_ram_enabled: false,
            prg_ram_select: false,
            irq_counter: 0,
            irq_counter_enabled: false,
            irq_enabled: false,
            irq_pending: Cell::new(false),
            audio: Sunsoft5BAudio::new(),
        }
    }

    pub(in crate::cartridge) fn clock_irq_mut(&mut self) {
        if self.irq_counter_enabled {
            let old = self.irq_counter;
            self.irq_counter = old.wrapping_sub(1);
            if old == 0 && self.irq_enabled {
                self.irq_pending.set(true);
            }
        }
    }
}

impl Cartridge {
    pub(in crate::cartridge) fn read_prg_fme7(&self, addr: u16) -> u8 {
        if let Some(ref fme7) = self.fme7 {
            let num_8k_banks = self.prg_rom.len() / 0x2000;
            if num_8k_banks == 0 {
                return 0;
            }
            let bank_mask = num_8k_banks - 1;

            let (bank, offset) = match addr {
                0x8000..=0x9FFF => {
                    let bank = (fme7.prg_banks[0] as usize) & bank_mask;
                    (bank, (addr - 0x8000) as usize)
                }
                0xA000..=0xBFFF => {
                    let bank = (fme7.prg_banks[1] as usize) & bank_mask;
                    (bank, (addr - 0xA000) as usize)
                }
                0xC000..=0xDFFF => {
                    let bank = (fme7.prg_banks[2] as usize) & bank_mask;
                    (bank, (addr - 0xC000) as usize)
                }
                0xE000..=0xFFFF => {
                    let bank = (num_8k_banks - 1) & bank_mask;
                    (bank, (addr - 0xE000) as usize)
                }
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

    pub(in crate::cartridge) fn write_prg_fme7(&mut self, addr: u16, data: u8) {
        if let Some(ref mut fme7) = self.fme7 {
            match addr {
                0x8000..=0x9FFF => {
                    fme7.command = data & 0x0F;
                }
                0xA000..=0xBFFF => {
                    match fme7.command {
                        0..=7 => {
                            fme7.chr_banks[fme7.command as usize] = data;
                        }
                        8 => {
                            fme7.prg_ram_enabled = (data & 0x80) != 0;
                            fme7.prg_ram_select = (data & 0x40) != 0;
                            fme7.prg_bank_6000 = data & 0x3F;
                        }
                        9 => {
                            fme7.prg_banks[0] = data & 0x3F;
                        }
                        0xA => {
                            fme7.prg_banks[1] = data & 0x3F;
                        }
                        0xB => {
                            fme7.prg_banks[2] = data & 0x3F;
                        }
                        0xC => {
                            self.mirroring = match data & 0x03 {
                                0 => Mirroring::Vertical,
                                1 => Mirroring::Horizontal,
                                2 => Mirroring::OneScreenLower,
                                3 => Mirroring::OneScreenUpper,
                                _ => unreachable!(),
                            };
                        }
                        0xD => {
                            fme7.irq_counter_enabled = (data & 0x80) != 0;
                            fme7.irq_enabled = (data & 0x01) != 0;
                            // Writing to command D clears pending IRQ
                            fme7.irq_pending.set(false);
                        }
                        0xE => {
                            let high = (fme7.irq_counter & 0xFF00) as u16;
                            fme7.irq_counter = high | (data as u16);
                        }
                        0xF => {
                            let low = (fme7.irq_counter & 0x00FF) as u16;
                            fme7.irq_counter = ((data as u16) << 8) | low;
                        }
                        _ => {}
                    }
                }
                // Sunsoft 5B expansion audio registers
                0xC000..=0xDFFF => {
                    fme7.audio.write_select(data);
                }
                0xE000..=0xFFFF => {
                    fme7.audio.write_data(data);
                }
                _ => {}
            }
        }
    }

    pub(in crate::cartridge) fn read_chr_fme7(&self, addr: u16) -> u8 {
        if let Some(ref fme7) = self.fme7 {
            let slot = ((addr >> 10) & 7) as usize;
            let bank = fme7.chr_banks[slot] as usize;
            let offset = (addr & 0x03FF) as usize;

            let chr_addr = bank * 0x0400 + offset;

            if !self.chr_ram.is_empty() {
                if chr_addr < self.chr_ram.len() {
                    self.chr_ram[chr_addr]
                } else {
                    self.chr_ram[chr_addr % self.chr_ram.len()]
                }
            } else if chr_addr < self.chr_rom.len() {
                self.chr_rom[chr_addr]
            } else if !self.chr_rom.is_empty() {
                self.chr_rom[chr_addr % self.chr_rom.len()]
            } else {
                0
            }
        } else {
            0
        }
    }

    pub(in crate::cartridge) fn write_chr_fme7(&mut self, addr: u16, data: u8) {
        if !self.chr_ram.is_empty() {
            if let Some(ref fme7) = self.fme7 {
                let slot = ((addr >> 10) & 7) as usize;
                let bank = fme7.chr_banks[slot] as usize;
                let offset = (addr & 0x03FF) as usize;
                let chr_addr = bank * 0x0400 + offset;
                if chr_addr < self.chr_ram.len() {
                    self.chr_ram[chr_addr] = data;
                }
            }
        }
    }

    pub(in crate::cartridge) fn read_prg_ram_fme7(&self, addr: u16) -> u8 {
        if let Some(ref fme7) = self.fme7 {
            if !fme7.prg_ram_enabled {
                return 0;
            }
            if fme7.prg_ram_select {
                // RAM mode
                let ram_addr = (addr - 0x6000) as usize;
                if ram_addr < self.prg_ram.len() {
                    self.prg_ram[ram_addr]
                } else {
                    0
                }
            } else {
                // ROM mode - map PRG-ROM bank here
                let num_8k_banks = self.prg_rom.len() / 0x2000;
                if num_8k_banks == 0 {
                    return 0;
                }
                let bank = (fme7.prg_bank_6000 as usize) % num_8k_banks;
                let offset = (addr - 0x6000) as usize;
                let rom_addr = bank * 0x2000 + offset;
                if rom_addr < self.prg_rom.len() {
                    self.prg_rom[rom_addr]
                } else {
                    0
                }
            }
        } else {
            0
        }
    }

    pub(in crate::cartridge) fn write_prg_ram_fme7(&mut self, addr: u16, data: u8) {
        if let Some(ref fme7) = self.fme7 {
            if !fme7.prg_ram_enabled || !fme7.prg_ram_select {
                return;
            }
            let ram_addr = (addr - 0x6000) as usize;
            if ram_addr < self.prg_ram.len() {
                self.prg_ram[ram_addr] = data;
            }
        }
    }
}
