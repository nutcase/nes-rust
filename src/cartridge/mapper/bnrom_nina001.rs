use super::super::Cartridge;

impl Cartridge {
    /// Mapper 34 exposes two different boards in iNES:
    /// BNROM uses a single 32KB PRG bank register at $8000-$FFFF,
    /// while NINA-001 puts PRG/CHR registers in the $7FFD-$7FFF window.
    pub(in crate::cartridge) fn write_prg_bnrom(&mut self, addr: u16, data: u8) {
        if addr >= 0x8000 {
            let current_bank = self.prg_bank as usize;
            let offset = current_bank * 0x8000 + (addr - 0x8000) as usize;
            let rom_value = if self.prg_rom.is_empty() {
                0xFF
            } else {
                self.prg_rom[offset % self.prg_rom.len()]
            };
            let effective = data & rom_value;
            let bank_count = (self.prg_rom.len() / 0x8000).max(1);
            self.prg_bank = (effective as usize % bank_count) as u8;
        }
    }

    pub(in crate::cartridge) fn write_prg_ram_nina001(&mut self, addr: u16, data: u8) {
        if (0x6000..=0x7FFF).contains(&addr) && !self.prg_ram.is_empty() {
            let index = (addr - 0x6000) as usize % self.prg_ram.len();
            self.prg_ram[index] = data;
            self.has_valid_save_data = true;
        }

        match addr {
            0x7FFD => {
                let bank_count = (self.prg_rom.len() / 0x8000).max(1);
                self.prg_bank = (data as usize % bank_count) as u8;
            }
            0x7FFE => {
                let bank_count = (self.chr_rom.len() / 0x1000).max(1);
                self.chr_bank = (data as usize % bank_count) as u8;
            }
            0x7FFF => {
                let bank_count = (self.chr_rom.len() / 0x1000).max(1);
                self.chr_bank_1 = (data as usize % bank_count) as u8;
            }
            _ => {}
        }
    }

    pub(in crate::cartridge) fn read_chr_nina001(&self, addr: u16) -> u8 {
        if self.chr_rom.is_empty() {
            return 0;
        }

        let bank_count = (self.chr_rom.len() / 0x1000).max(1);
        let (bank, offset) = if addr < 0x1000 {
            ((self.chr_bank as usize) % bank_count, addr as usize)
        } else {
            (
                (self.chr_bank_1 as usize) % bank_count,
                (addr as usize) - 0x1000,
            )
        };
        let chr_addr = bank * 0x1000 + offset;
        self.chr_rom[chr_addr % self.chr_rom.len()]
    }

    pub(in crate::cartridge) fn write_chr_nina001(&mut self, addr: u16, data: u8) {
        if self.chr_rom.is_empty() {
            return;
        }

        let bank_count = (self.chr_rom.len() / 0x1000).max(1);
        let (bank, offset) = if addr < 0x1000 {
            ((self.chr_bank as usize) % bank_count, addr as usize)
        } else {
            (
                (self.chr_bank_1 as usize) % bank_count,
                (addr as usize) - 0x1000,
            )
        };
        let chr_len = self.chr_rom.len();
        let chr_addr = bank * 0x1000 + offset;
        self.chr_rom[chr_addr % chr_len] = data;
    }
}
