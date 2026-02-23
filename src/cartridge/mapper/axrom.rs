use super::super::{Cartridge, Mirroring};

impl Cartridge {
    /// AxROM PRG read - 32KB switchable bank at $8000-$FFFF
    pub(in crate::cartridge) fn read_prg_axrom(&self, addr: u16) -> u8 {
        let bank = self.prg_bank as usize;
        let offset = bank * 0x8000 + (addr - 0x8000) as usize;
        if offset < self.prg_rom.len() {
            self.prg_rom[offset]
        } else {
            self.prg_rom[offset % self.prg_rom.len()]
        }
    }

    /// AxROM PRG write - bits 0-2: 32KB PRG bank, bit 4: nametable select
    pub(in crate::cartridge) fn write_prg_axrom(&mut self, addr: u16, data: u8) {
        if addr >= 0x8000 {
            self.prg_bank = data & 0x07;
            self.mirroring = if data & 0x10 != 0 {
                Mirroring::OneScreenUpper
            } else {
                Mirroring::OneScreenLower
            };
        }
    }
}
