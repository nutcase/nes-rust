use super::super::Cartridge;

impl Cartridge {
    /// UxROM PRG read - 16KB switchable + 16KB fixed
    pub(in crate::cartridge) fn read_prg_uxrom(&self, addr: u16, rom_addr: u16) -> u8 {
        if addr < 0xC000 {
            // Switchable 16KB bank at $8000-$BFFF
            let offset = (self.prg_bank as usize) * 0x4000 + (rom_addr as usize);
            if offset < self.prg_rom.len() {
                self.prg_rom[offset]
            } else {
                0
            }
        } else {
            // Fixed last 16KB bank at $C000-$FFFF
            let last_bank_offset = self.prg_rom.len() - 0x4000;
            let offset = last_bank_offset + ((addr - 0xC000) as usize);
            if offset < self.prg_rom.len() {
                self.prg_rom[offset]
            } else {
                0
            }
        }
    }

    /// UxROM PRG write - bank switching with bus conflicts
    pub(in crate::cartridge) fn write_prg_uxrom(&mut self, addr: u16, data: u8) {
        if addr >= 0x8000 {
            // Bus conflicts: AND written value with ROM value
            let rom_offset = if addr < 0xC000 {
                (self.prg_bank as usize) * 0x4000 + ((addr - 0x8000) as usize)
            } else {
                self.prg_rom.len() - 0x4000 + ((addr - 0xC000) as usize)
            };

            let rom_value = if rom_offset < self.prg_rom.len() {
                self.prg_rom[rom_offset]
            } else {
                0xFF
            };

            let effective_value = data & rom_value;
            self.prg_bank = effective_value & 0x07;
        }
    }

    /// UxROM CHR read - 8KB CHR RAM
    pub(in crate::cartridge) fn read_chr_uxrom(&self, addr: u16) -> u8 {
        let chr_addr = (addr & 0x1FFF) as usize;
        if chr_addr < self.chr_rom.len() {
            self.chr_rom[chr_addr]
        } else {
            0
        }
    }

    /// UxROM CHR write - 8KB CHR RAM (writable)
    pub(in crate::cartridge) fn write_chr_uxrom(&mut self, addr: u16, data: u8) {
        let chr_addr = (addr & 0x1FFF) as usize;
        if chr_addr < self.chr_rom.len() {
            self.chr_rom[chr_addr] = data;
        }
    }
}
