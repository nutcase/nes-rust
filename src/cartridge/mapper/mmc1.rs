use super::super::{Cartridge, Mirroring};

#[derive(Debug, Clone)]
pub(in crate::cartridge) struct Mmc1 {
    pub(in crate::cartridge) shift_register: u8,
    pub(in crate::cartridge) shift_count: u8,
    pub(in crate::cartridge) control: u8,
    pub(in crate::cartridge) chr_bank_0: u8,
    pub(in crate::cartridge) chr_bank_1: u8,
    pub(in crate::cartridge) prg_bank: u8,
    pub(in crate::cartridge) prg_ram_disable: bool,
}

impl Mmc1 {
    pub(in crate::cartridge) fn new() -> Self {
        Mmc1 {
            shift_register: 0x10,
            shift_count: 0,
            control: 0x0C, // Default: 16KB PRG mode, last bank fixed
            chr_bank_0: 0,
            chr_bank_1: 0,
            prg_bank: 0,
            prg_ram_disable: false,
        }
    }
}

impl Cartridge {
    /// MMC1 PRG read with 4 banking modes + SUROM support
    pub(in crate::cartridge) fn read_prg_mmc1(&self, addr: u16, rom_addr: u16) -> u8 {
        if let Some(ref mmc1) = self.mmc1 {
            let prg_mode = (mmc1.control >> 2) & 0x03;
            let prg_size = self.prg_rom.len() / 0x4000; // Number of 16KB banks

            // SUROM support: Use CHR bank bit 4 for PRG bank extension
            let prg_bank_hi = if prg_size > 16 {
                // For SUROM: CHR bank 0 bit 4 selects which 256KB half
                ((mmc1.chr_bank_0 >> 4) & 0x01) as usize
            } else {
                0
            };

            match prg_mode {
                0 | 1 => {
                    // 32KB mode: switch 32KB at $8000
                    let bank_lo = ((mmc1.prg_bank & 0x0E) >> 1) as usize;
                    let bank = (prg_bank_hi << 3) | bank_lo;
                    let max_banks = self.prg_rom.len() / 0x8000;
                    let safe_bank = bank % max_banks;
                    let offset = safe_bank * 0x8000 + (rom_addr as usize);
                    if offset < self.prg_rom.len() {
                        self.prg_rom[offset]
                    } else {
                        0
                    }
                }
                2 => {
                    // Fix first bank at $8000, switch 16KB at $C000
                    if addr < 0xC000 {
                        let offset = (prg_bank_hi * 0x40000) + (rom_addr as usize);
                        if offset < self.prg_rom.len() {
                            self.prg_rom[offset]
                        } else {
                            0
                        }
                    } else {
                        let bank_lo = (mmc1.prg_bank & 0x0F) as usize;
                        let bank = (prg_bank_hi << 4) | bank_lo;
                        let max_banks = self.prg_rom.len() / 0x4000;
                        let safe_bank = bank % max_banks;
                        let offset = safe_bank * 0x4000 + ((addr - 0xC000) as usize);
                        if offset < self.prg_rom.len() {
                            self.prg_rom[offset]
                        } else {
                            0
                        }
                    }
                }
                3 | _ => {
                    // Switch 16KB at $8000, fix last bank at $C000 (default after reset)
                    if addr < 0xC000 {
                        let bank_lo = (mmc1.prg_bank & 0x0F) as usize;
                        let bank = (prg_bank_hi << 4) | bank_lo;
                        let max_banks = self.prg_rom.len() / 0x4000;
                        let safe_bank = bank % max_banks;
                        let offset = safe_bank * 0x4000 + (rom_addr as usize);
                        if offset < self.prg_rom.len() {
                            self.prg_rom[offset]
                        } else {
                            0
                        }
                    } else {
                        // Fixed last bank at $C000
                        // For SUROM (512KB), the "last bank" depends on CHR bank 0 bit 4
                        let last_bank_offset = if self.prg_rom.len() > 0x40000 {
                            // SUROM: last bank of the selected 256KB region
                            let base = prg_bank_hi * 0x40000;
                            base + 0x3C000
                        } else {
                            // Standard MMC1: last bank of entire ROM
                            self.prg_rom.len() - 0x4000
                        };
                        let offset = last_bank_offset + ((addr - 0xC000) as usize);
                        if offset < self.prg_rom.len() {
                            self.prg_rom[offset]
                        } else {
                            0
                        }
                    }
                }
            }
        } else {
            0
        }
    }

    /// MMC1 PRG write - shift register + register decode
    pub(in crate::cartridge) fn write_prg_mmc1(&mut self, addr: u16, data: u8) {
        if let Some(ref mut mmc1) = self.mmc1 {
            // Check for reset (bit 7 set)
            if data & 0x80 != 0 {
                mmc1.shift_register = 0x10;
                mmc1.shift_count = 0;
                mmc1.control |= 0x0C;
                return;
            }

            // Shift in bit 0 (LSB first)
            mmc1.shift_register >>= 1;
            if data & 0x01 != 0 {
                mmc1.shift_register |= 0x10;
            }
            mmc1.shift_count = mmc1.shift_count.saturating_add(1);

            // After 5 writes, update the target register
            if mmc1.shift_count >= 5 {
                let register_data = mmc1.shift_register & 0x1F;

                let register_select = match addr {
                    0x8000..=0x9FFF => 0, // Control
                    0xA000..=0xBFFF => 1, // CHR bank 0
                    0xC000..=0xDFFF => 2, // CHR bank 1
                    0xE000..=0xFFFF => 3, // PRG bank
                    _ => return,
                };

                match register_select {
                    0 => {
                        mmc1.control = register_data;
                        self.mirroring = match register_data & 0x03 {
                            0 => Mirroring::OneScreenLower,
                            1 => Mirroring::OneScreenUpper,
                            2 => Mirroring::Vertical,
                            3 => Mirroring::Horizontal,
                            _ => self.mirroring,
                        };
                    }
                    1 => {
                        mmc1.chr_bank_0 = register_data;
                    }
                    2 => {
                        mmc1.chr_bank_1 = register_data;
                    }
                    3 => {
                        mmc1.prg_bank = register_data & 0x0F;
                        mmc1.prg_ram_disable = (register_data & 0x10) != 0;
                    }
                    _ => {}
                }

                mmc1.shift_register = 0x10;
                mmc1.shift_count = 0;
            }
        }
    }

    /// MMC1 CHR read - 8KB/4KB modes, CHR-RAM/ROM
    pub(in crate::cartridge) fn read_chr_mmc1(&self, addr: u16) -> u8 {
        if let Some(ref mmc1) = self.mmc1 {
            let chr_mode = (mmc1.control >> 4) & 0x01;

            if chr_mode == 0 {
                // 8KB mode: use CHR bank 0, ignore CHR bank 1
                let bank = (mmc1.chr_bank_0 & 0x1E) >> 1;
                let offset = (bank as usize) * 0x2000 + (addr as usize);

                if !self.chr_ram.is_empty() {
                    if offset < self.chr_ram.len() {
                        self.chr_ram[offset]
                    } else {
                        0
                    }
                } else if offset < self.chr_rom.len() {
                    self.chr_rom[offset]
                } else {
                    0
                }
            } else {
                // 4KB mode: separate banks for each 4KB region
                let (bank, local_addr) = if addr < 0x1000 {
                    (mmc1.chr_bank_0, addr as usize)
                } else {
                    (mmc1.chr_bank_1, (addr - 0x1000) as usize)
                };
                let offset = (bank as usize) * 0x1000 + local_addr;

                if !self.chr_ram.is_empty() {
                    if offset < self.chr_ram.len() {
                        self.chr_ram[offset]
                    } else {
                        0
                    }
                } else if offset < self.chr_rom.len() {
                    self.chr_rom[offset]
                } else {
                    0
                }
            }
        } else {
            0
        }
    }

    /// MMC1 CHR write - CHR-RAM/ROM with bank switching
    pub(in crate::cartridge) fn write_chr_mmc1(&mut self, addr: u16, data: u8) {
        if let Some(ref mmc1) = self.mmc1 {
            let chr_mode = (mmc1.control >> 4) & 0x01;

            if chr_mode == 0 {
                // 8KB mode
                let bank = (mmc1.chr_bank_0 & 0x1E) >> 1;
                let offset = (bank as usize) * 0x2000 + (addr as usize);

                if !self.chr_ram.is_empty() {
                    if offset < self.chr_ram.len() {
                        self.chr_ram[offset] = data;
                    }
                } else if offset < self.chr_rom.len() {
                    self.chr_rom[offset] = data;
                }
            } else {
                // 4KB mode
                let (bank, local_addr) = if addr < 0x1000 {
                    (mmc1.chr_bank_0, addr as usize)
                } else {
                    (mmc1.chr_bank_1, (addr - 0x1000) as usize)
                };
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
    }

    /// MMC1 PRG-RAM read
    pub(in crate::cartridge) fn read_prg_ram_mmc1(&self, addr: u16) -> u8 {
        if !self.prg_ram.is_empty() {
            if let Some(ref mmc1) = self.mmc1 {
                let e_bit_clear = (mmc1.control & 0x10) == 0;
                let r_bit_clear = !mmc1.prg_ram_disable;

                // Only block if both bits indicate disable
                if !e_bit_clear && !r_bit_clear {
                    return 0x00;
                }
            }

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

    /// MMC1 PRG-RAM write
    pub(in crate::cartridge) fn write_prg_ram_mmc1(&mut self, addr: u16, data: u8) {
        if !self.prg_ram.is_empty() {
            if let Some(ref mmc1) = self.mmc1 {
                let e_bit_clear = (mmc1.control & 0x10) == 0;
                let r_bit_clear = !mmc1.prg_ram_disable;

                if !e_bit_clear && !r_bit_clear {
                    return;
                }
            }

            let ram_addr = (addr - 0x6000) as usize;
            if ram_addr < self.prg_ram.len() {
                self.prg_ram[ram_addr] = data;

                if addr == 0x60B7 && data == 0x5A {
                    self.has_valid_save_data = true;
                }
            }
        }
    }
}
