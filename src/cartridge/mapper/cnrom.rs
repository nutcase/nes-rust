use super::super::Cartridge;

impl Cartridge {
    /// Mapper 3 (CNROM) PRG write - CHR bank switching with bus conflicts
    pub(in crate::cartridge) fn write_prg_cnrom(&mut self, addr: u16, data: u8) {
        if addr >= 0x8000 {
            // Read ROM value at the write address to handle bus conflicts
            let rom_value = if (addr as usize) < self.prg_rom.len() {
                self.prg_rom[addr as usize]
            } else {
                let mirrored_addr = (addr - 0x8000) % (self.prg_rom.len() as u16);
                self.prg_rom[mirrored_addr as usize]
            };

            // Bus conflict: use AND of written value and ROM value
            let effective_value = data & rom_value;
            self.chr_bank = effective_value & 0x03;
        }
    }

    /// Mapper 87 PRG write - CHR bank switching at $6000-$7FFF
    pub(in crate::cartridge) fn write_prg_mapper87(&mut self, addr: u16, data: u8) {
        if addr >= 0x6000 && addr <= 0x7FFF {
            // Swap bits 0 and 1
            self.chr_bank = ((data & 0x01) << 1) | ((data & 0x02) >> 1);
        }
    }

    /// CNROM/Mapper 87 CHR read - 8KB CHR bank switching
    pub(in crate::cartridge) fn read_chr_cnrom(&self, addr: u16) -> u8 {
        let bank_addr = (self.chr_bank as usize) * 0x2000 + (addr as usize);
        if bank_addr < self.chr_rom.len() {
            self.chr_rom[bank_addr]
        } else {
            0
        }
    }

    /// CNROM/Mapper 87 CHR write
    pub(in crate::cartridge) fn write_chr_cnrom(&mut self, addr: u16, data: u8) {
        let bank_addr = (self.chr_bank as usize) * 0x2000 + (addr as usize);
        if bank_addr < self.chr_rom.len() {
            self.chr_rom[bank_addr] = data;
        }
    }
}
