use super::super::Cartridge;

impl Cartridge {
    /// Mapper 66 (GxROM): bits 4-5 select a 32KB PRG bank and bits 0-1
    /// select an 8KB CHR bank.
    pub(in crate::cartridge) fn write_prg_gxrom(&mut self, addr: u16, data: u8) {
        if addr >= 0x8000 {
            self.prg_bank = (data >> 4) & 0x03;
            self.chr_bank = data & 0x03;
        }
    }
}
