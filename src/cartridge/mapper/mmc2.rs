use std::cell::Cell;

use super::super::{Cartridge, Mirroring};

#[derive(Debug, Clone)]
pub(in crate::cartridge) struct Mmc2 {
    pub(in crate::cartridge) prg_bank: u8,
    pub(in crate::cartridge) chr_bank_0_fd: u8,
    pub(in crate::cartridge) chr_bank_0_fe: u8,
    pub(in crate::cartridge) chr_bank_1_fd: u8,
    pub(in crate::cartridge) chr_bank_1_fe: u8,
    pub(in crate::cartridge) latch_0: Cell<bool>, // false=FD, true=FE
    pub(in crate::cartridge) latch_1: Cell<bool>,
}

impl Mmc2 {
    pub(in crate::cartridge) fn new() -> Self {
        Mmc2 {
            prg_bank: 0,
            chr_bank_0_fd: 0,
            chr_bank_0_fe: 0,
            chr_bank_1_fd: 0,
            chr_bank_1_fe: 0,
            latch_0: Cell::new(true),  // FE selected initially
            latch_1: Cell::new(true),  // FE selected initially
        }
    }
}

impl Cartridge {
    /// MMC2/MMC4 PRG read
    /// MMC2 (mapper 9): 8KB switchable ($8000-$9FFF) + 24KB fixed ($A000-$FFFF)
    /// MMC4 (mapper 10): 16KB switchable ($8000-$BFFF) + 16KB fixed ($C000-$FFFF)
    pub(in crate::cartridge) fn read_prg_mmc2(&self, addr: u16, rom_addr: u16) -> u8 {
        if let Some(ref mmc2) = self.mmc2 {
            if self.mapper == 9 {
                // MMC2: 8KB switchable + 24KB fixed (last 3 x 8KB banks)
                if addr < 0xA000 {
                    let offset = (mmc2.prg_bank as usize) * 0x2000 + (rom_addr as usize);
                    if offset < self.prg_rom.len() {
                        self.prg_rom[offset]
                    } else {
                        0
                    }
                } else {
                    let fixed_start = self.prg_rom.len() - 0x6000;
                    let offset = fixed_start + ((addr - 0xA000) as usize);
                    if offset < self.prg_rom.len() {
                        self.prg_rom[offset]
                    } else {
                        0
                    }
                }
            } else {
                // MMC4 (mapper 10): 16KB switchable + 16KB fixed
                if addr < 0xC000 {
                    let offset = (mmc2.prg_bank as usize) * 0x4000 + (rom_addr as usize);
                    if offset < self.prg_rom.len() {
                        self.prg_rom[offset]
                    } else {
                        0
                    }
                } else {
                    let last_bank_offset = self.prg_rom.len() - 0x4000;
                    let offset = last_bank_offset + ((addr - 0xC000) as usize);
                    if offset < self.prg_rom.len() {
                        self.prg_rom[offset]
                    } else {
                        0
                    }
                }
            }
        } else {
            0
        }
    }

    /// MMC2/MMC4 PRG write - register decode at $A000-$FFFF
    pub(in crate::cartridge) fn write_prg_mmc2(&mut self, addr: u16, data: u8) {
        if let Some(ref mut mmc2) = self.mmc2 {
            match addr {
                0xA000..=0xAFFF => {
                    mmc2.prg_bank = data & 0x0F;
                }
                0xB000..=0xBFFF => {
                    mmc2.chr_bank_0_fd = data & 0x1F;
                }
                0xC000..=0xCFFF => {
                    mmc2.chr_bank_0_fe = data & 0x1F;
                }
                0xD000..=0xDFFF => {
                    mmc2.chr_bank_1_fd = data & 0x1F;
                }
                0xE000..=0xEFFF => {
                    mmc2.chr_bank_1_fe = data & 0x1F;
                }
                0xF000..=0xFFFF => {
                    self.mirroring = if data & 0x01 != 0 {
                        Mirroring::Horizontal
                    } else {
                        Mirroring::Vertical
                    };
                }
                _ => {}
            }
        }
    }

    /// MMC2/MMC4 CHR read with latch mechanism
    pub(in crate::cartridge) fn read_chr_mmc2(&self, addr: u16) -> u8 {
        if let Some(ref mmc2) = self.mmc2 {
            let bank = if addr < 0x1000 {
                if mmc2.latch_0.get() {
                    mmc2.chr_bank_0_fe
                } else {
                    mmc2.chr_bank_0_fd
                }
            } else {
                if mmc2.latch_1.get() {
                    mmc2.chr_bank_1_fe
                } else {
                    mmc2.chr_bank_1_fd
                }
            };

            let local_addr = (addr & 0x0FFF) as usize;
            let offset = (bank as usize) * 0x1000 + local_addr;

            let data = if !self.chr_ram.is_empty() {
                if offset < self.chr_ram.len() {
                    self.chr_ram[offset]
                } else {
                    0
                }
            } else if offset < self.chr_rom.len() {
                self.chr_rom[offset]
            } else {
                0
            };

            // Update latches AFTER the read based on the address fetched
            match addr {
                0x0FD8..=0x0FDF => mmc2.latch_0.set(false), // FD
                0x0FE8..=0x0FEF => mmc2.latch_0.set(true),  // FE
                0x1FD8..=0x1FDF => mmc2.latch_1.set(false), // FD
                0x1FE8..=0x1FEF => mmc2.latch_1.set(true),  // FE
                _ => {}
            }

            data
        } else {
            0
        }
    }

    /// MMC2/MMC4 CHR write (CHR-RAM)
    pub(in crate::cartridge) fn write_chr_mmc2(&mut self, addr: u16, data: u8) {
        if let Some(ref mmc2) = self.mmc2 {
            let bank = if addr < 0x1000 {
                if mmc2.latch_0.get() {
                    mmc2.chr_bank_0_fe
                } else {
                    mmc2.chr_bank_0_fd
                }
            } else {
                if mmc2.latch_1.get() {
                    mmc2.chr_bank_1_fe
                } else {
                    mmc2.chr_bank_1_fd
                }
            };

            let local_addr = (addr & 0x0FFF) as usize;
            let offset = (bank as usize) * 0x1000 + local_addr;

            if !self.chr_ram.is_empty() {
                if offset < self.chr_ram.len() {
                    self.chr_ram[offset] = data;
                }
            } else if offset < self.chr_rom.len() {
                self.chr_rom[offset] = data;
            }
        }
    }

    /// MMC2/MMC4 PRG-RAM read ($6000-$7FFF)
    pub(in crate::cartridge) fn read_prg_ram_mmc2(&self, addr: u16) -> u8 {
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

    /// MMC2/MMC4 PRG-RAM write ($6000-$7FFF)
    pub(in crate::cartridge) fn write_prg_ram_mmc2(&mut self, addr: u16, data: u8) {
        if !self.prg_ram.is_empty() {
            let ram_addr = (addr - 0x6000) as usize;
            if ram_addr < self.prg_ram.len() {
                self.prg_ram[ram_addr] = data;
            }
        }
    }
}
