use super::super::{Cartridge, Mirroring};

impl Cartridge {
    /// Mapper 71 (Camerica): 16KB switchable bank at $8000-$BFFF with
    /// the last 16KB fixed at $C000-$FFFF. Some boards also expose a
    /// one-screen mirroring register at $9000-$9FFF.
    pub(in crate::cartridge) fn write_prg_camerica(&mut self, addr: u16, data: u8) {
        match addr {
            0x9000..=0x9FFF => {
                self.mirroring = if data & 0x10 != 0 {
                    Mirroring::OneScreenUpper
                } else {
                    Mirroring::OneScreenLower
                };
            }
            0xC000..=0xFFFF => {
                let bank_count = (self.prg_rom.len() / 0x4000).max(1);
                self.prg_bank = (data as usize % bank_count) as u8;
            }
            _ => {}
        }
    }
}
