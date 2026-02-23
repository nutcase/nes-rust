use std::cell::Cell;

use super::super::{Cartridge, Mirroring};

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
                0xE000..=0xFFFF => {
                    (last, (addr - 0xE000) as usize)
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

            let (bank_1k, local_offset) = self.resolve_chr_bank_mmc3(addr, chr_a12_invert, bank_mask, mmc3);

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

    pub(in crate::cartridge) fn write_chr_mmc3(&mut self, addr: u16, data: u8) {
        if !self.chr_ram.is_empty() {
            if let Some(ref mmc3) = self.mmc3 {
                let chr_a12_invert = (mmc3.bank_select >> 7) & 1;
                let num_1k_banks = self.chr_ram.len() / 0x0400;
                if num_1k_banks == 0 {
                    return;
                }
                let bank_mask = num_1k_banks - 1;

                let (bank_1k, local_offset) = self.resolve_chr_bank_mmc3(addr, chr_a12_invert, bank_mask, mmc3);

                let chr_addr = bank_1k * 0x0400 + local_offset;
                if chr_addr < self.chr_ram.len() {
                    self.chr_ram[chr_addr] = data;
                }
            }
        }
    }

    fn resolve_chr_bank_mmc3(
        &self,
        addr: u16,
        chr_a12_invert: u8,
        bank_mask: usize,
        mmc3: &Mmc3,
    ) -> (usize, usize) {
        // CHR A12 inversion swaps the 2KB and 1KB regions:
        // invert=0: R0,R1 at $0000-$0FFF (2KB each), R2-R5 at $1000-$1FFF (1KB each)
        // invert=1: R2-R5 at $0000-$0FFF (1KB each), R0,R1 at $1000-$1FFF (2KB each)
        let slot = (addr >> 10) & 7; // 0-7, each 1KB slot
        let adjusted_slot = if chr_a12_invert != 0 {
            slot ^ 4 // swap upper and lower halves
        } else {
            slot
        };

        let bank_1k = match adjusted_slot {
            0 => (mmc3.bank_registers[0] as usize & !1) & bank_mask,       // R0 low
            1 => ((mmc3.bank_registers[0] as usize & !1) | 1) & bank_mask, // R0 high
            2 => (mmc3.bank_registers[1] as usize & !1) & bank_mask,       // R1 low
            3 => ((mmc3.bank_registers[1] as usize & !1) | 1) & bank_mask, // R1 high
            4 => (mmc3.bank_registers[2] as usize) & bank_mask,
            5 => (mmc3.bank_registers[3] as usize) & bank_mask,
            6 => (mmc3.bank_registers[4] as usize) & bank_mask,
            7 => (mmc3.bank_registers[5] as usize) & bank_mask,
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
            mmc3.irq_pending.get()
        } else {
            false
        }
    }

    pub fn acknowledge_irq(&self) {
        if let Some(ref mmc3) = self.mmc3 {
            mmc3.irq_pending.set(false);
        }
    }
}
