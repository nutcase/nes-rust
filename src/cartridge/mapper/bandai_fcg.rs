use std::cell::Cell;

use super::super::{Cartridge, Mirroring};

/// Bandai FCG / LZ93D50 (Mapper 16).
/// Used by Dragon Ball Z series and other Bandai games.
/// Features: 8x1KB CHR banking, 16KB PRG banking, CPU-cycle IRQ counter.
#[derive(Debug, Clone)]
pub(in crate::cartridge) struct BandaiFcg {
    pub(in crate::cartridge) chr_banks: [u8; 8],
    pub(in crate::cartridge) prg_bank: u8,
    pub(in crate::cartridge) irq_counter: u16,
    pub(in crate::cartridge) irq_latch: u16,
    pub(in crate::cartridge) irq_enabled: bool,
    pub(in crate::cartridge) irq_pending: Cell<bool>,
}

impl BandaiFcg {
    pub(in crate::cartridge) fn new() -> Self {
        BandaiFcg {
            chr_banks: [0; 8],
            prg_bank: 0,
            irq_counter: 0,
            irq_latch: 0,
            irq_enabled: false,
            irq_pending: Cell::new(false),
        }
    }

    pub(in crate::cartridge) fn clock_irq_mut(&mut self) {
        if self.irq_enabled {
            if self.irq_counter == 0 {
                self.irq_pending.set(true);
                self.irq_enabled = false;
            } else {
                self.irq_counter -= 1;
            }
        }
    }
}

impl Cartridge {
    pub(in crate::cartridge) fn read_prg_bandai(&self, addr: u16) -> u8 {
        if let Some(ref bandai) = self.bandai_fcg {
            let num_16k_banks = self.prg_rom.len() / 0x4000;
            if num_16k_banks == 0 {
                return 0;
            }

            let (bank, offset) = match addr {
                0x8000..=0xBFFF => {
                    let bank = (bandai.prg_bank as usize) % num_16k_banks;
                    (bank, (addr - 0x8000) as usize)
                }
                0xC000..=0xFFFF => {
                    let bank = num_16k_banks - 1;
                    (bank, (addr - 0xC000) as usize)
                }
                _ => return 0,
            };

            let rom_addr = bank * 0x4000 + offset;
            if rom_addr < self.prg_rom.len() {
                self.prg_rom[rom_addr]
            } else {
                0
            }
        } else {
            0
        }
    }

    pub(in crate::cartridge) fn write_prg_bandai(&mut self, addr: u16, data: u8) {
        if let Some(ref mut bandai) = self.bandai_fcg {
            let reg = addr & 0x0F;
            match reg {
                0x00..=0x07 => {
                    bandai.chr_banks[reg as usize] = data;
                }
                0x08 => {
                    bandai.prg_bank = data & 0x0F;
                }
                0x09 => {
                    self.mirroring = match data & 0x03 {
                        0 => Mirroring::Vertical,
                        1 => Mirroring::Horizontal,
                        2 => Mirroring::OneScreenLower,
                        3 => Mirroring::OneScreenUpper,
                        _ => unreachable!(),
                    };
                }
                0x0A => {
                    bandai.irq_pending.set(false);
                    bandai.irq_enabled = (data & 0x01) != 0;
                    bandai.irq_counter = bandai.irq_latch;
                }
                0x0B => {
                    bandai.irq_latch = (bandai.irq_latch & 0xFF00) | (data as u16);
                }
                0x0C => {
                    bandai.irq_latch = (bandai.irq_latch & 0x00FF) | ((data as u16) << 8);
                }
                0x0D => {
                    // EEPROM I/O - not implemented
                }
                _ => {}
            }
        }
    }

    pub(in crate::cartridge) fn read_chr_bandai(&self, addr: u16) -> u8 {
        if let Some(ref bandai) = self.bandai_fcg {
            let slot = ((addr >> 10) & 7) as usize;
            let bank = bandai.chr_banks[slot] as usize;
            let offset = (addr & 0x03FF) as usize;

            let chr_addr = bank * 0x0400 + offset;

            if chr_addr < self.chr_rom.len() {
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

    pub(in crate::cartridge) fn write_chr_bandai(&mut self, addr: u16, _data: u8) {
        // CHR-ROM is read-only for Bandai FCG
        let _ = addr;
    }

    pub(in crate::cartridge) fn read_prg_ram_bandai(&self, addr: u16) -> u8 {
        let ram_addr = (addr - 0x6000) as usize;
        if ram_addr < self.prg_ram.len() {
            self.prg_ram[ram_addr]
        } else {
            0
        }
    }

    pub(in crate::cartridge) fn write_prg_ram_bandai(&mut self, addr: u16, data: u8) {
        let ram_addr = (addr - 0x6000) as usize;
        if ram_addr < self.prg_ram.len() {
            self.prg_ram[ram_addr] = data;
        }
    }
}
