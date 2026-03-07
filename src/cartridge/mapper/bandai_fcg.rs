use std::cell::Cell;

use super::super::{Cartridge, Mirroring};

#[derive(Debug, Clone, Copy)]
enum BandaiEepromNext {
    ReceiveAddress,
    ReceiveData,
    SendData,
}

#[derive(Debug, Clone, Copy)]
enum BandaiEepromPhase {
    Idle,
    ReceivingControl,
    ReceivingAddress,
    ReceivingData,
    AckPending(BandaiEepromNext),
    AckLow(BandaiEepromNext),
    Sending { byte: u8, bit_index: u8 },
    WaitAckPending,
    WaitAck,
}

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
    eeprom_phase: BandaiEepromPhase,
    eeprom_address: u8,
    eeprom_shift: u8,
    eeprom_bits: u8,
    eeprom_prev_scl: bool,
    eeprom_prev_sda: bool,
    eeprom_data_out: bool,
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
            eeprom_phase: BandaiEepromPhase::Idle,
            eeprom_address: 0,
            eeprom_shift: 0,
            eeprom_bits: 0,
            eeprom_prev_scl: false,
            eeprom_prev_sda: true,
            eeprom_data_out: true,
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

    fn eeprom_start(&mut self) {
        self.eeprom_phase = BandaiEepromPhase::ReceivingControl;
        self.eeprom_shift = 0;
        self.eeprom_bits = 0;
        self.eeprom_data_out = true;
    }

    fn eeprom_stop(&mut self) {
        self.eeprom_phase = BandaiEepromPhase::Idle;
        self.eeprom_data_out = true;
        self.eeprom_shift = 0;
        self.eeprom_bits = 0;
    }

    fn eeprom_begin_send(&mut self, byte: u8) {
        self.eeprom_phase = BandaiEepromPhase::Sending { byte, bit_index: 7 };
        self.eeprom_data_out = (byte & 0x80) != 0;
    }

    fn eeprom_transition_after_ack(&mut self, next: BandaiEepromNext, storage: &[u8]) {
        self.eeprom_data_out = true;
        match next {
            BandaiEepromNext::ReceiveAddress => {
                self.eeprom_phase = BandaiEepromPhase::ReceivingAddress;
                self.eeprom_shift = 0;
                self.eeprom_bits = 0;
            }
            BandaiEepromNext::ReceiveData => {
                self.eeprom_phase = BandaiEepromPhase::ReceivingData;
                self.eeprom_shift = 0;
                self.eeprom_bits = 0;
            }
            BandaiEepromNext::SendData => {
                let byte = storage[self.eeprom_address as usize % storage.len()];
                self.eeprom_begin_send(byte);
            }
        }
    }

    fn eeprom_process_received_byte(&mut self, byte: u8, storage: &mut [u8], dirty: &mut bool) {
        match self.eeprom_phase {
            BandaiEepromPhase::ReceivingControl => {
                // 24C02 fixed device address 1010_000x.
                if (byte >> 1) == 0x50 {
                    let next = if byte & 0x01 == 0 {
                        BandaiEepromNext::ReceiveAddress
                    } else {
                        BandaiEepromNext::SendData
                    };
                    self.eeprom_phase = BandaiEepromPhase::AckPending(next);
                } else {
                    self.eeprom_phase = BandaiEepromPhase::Idle;
                }
            }
            BandaiEepromPhase::ReceivingAddress => {
                self.eeprom_address = byte;
                self.eeprom_phase = BandaiEepromPhase::AckPending(BandaiEepromNext::ReceiveData);
            }
            BandaiEepromPhase::ReceivingData => {
                let index = self.eeprom_address as usize % storage.len();
                if storage[index] != byte {
                    storage[index] = byte;
                    *dirty = true;
                }
                self.eeprom_address = self.eeprom_address.wrapping_add(1);
                self.eeprom_phase = BandaiEepromPhase::AckPending(BandaiEepromNext::ReceiveData);
            }
            _ => {}
        }
    }

    fn eeprom_clock_control(&mut self, control: u8, storage: &mut [u8], dirty: &mut bool) {
        if storage.is_empty() {
            self.eeprom_data_out = true;
            return;
        }

        let read_enabled = (control & 0x80) != 0;
        let sda = if read_enabled {
            true
        } else {
            (control & 0x40) != 0
        };
        let scl = (control & 0x20) != 0;

        if self.eeprom_prev_scl && scl {
            if self.eeprom_prev_sda && !sda {
                self.eeprom_start();
            } else if !self.eeprom_prev_sda && sda {
                self.eeprom_stop();
            }
        }

        if !self.eeprom_prev_scl && scl {
            match self.eeprom_phase {
                BandaiEepromPhase::ReceivingControl
                | BandaiEepromPhase::ReceivingAddress
                | BandaiEepromPhase::ReceivingData => {
                    self.eeprom_shift = (self.eeprom_shift << 1) | u8::from(sda);
                    self.eeprom_bits += 1;
                    if self.eeprom_bits == 8 {
                        let byte = self.eeprom_shift;
                        self.eeprom_shift = 0;
                        self.eeprom_bits = 0;
                        self.eeprom_process_received_byte(byte, storage, dirty);
                    }
                }
                BandaiEepromPhase::Sending { byte, bit_index } => {
                    if bit_index == 0 {
                        self.eeprom_phase = BandaiEepromPhase::WaitAckPending;
                    } else {
                        self.eeprom_phase = BandaiEepromPhase::Sending {
                            byte,
                            bit_index: bit_index - 1,
                        };
                    }
                }
                BandaiEepromPhase::WaitAckPending => {}
                BandaiEepromPhase::WaitAck => {
                    if !sda {
                        self.eeprom_address = self.eeprom_address.wrapping_add(1);
                        let byte = storage[self.eeprom_address as usize % storage.len()];
                        self.eeprom_begin_send(byte);
                    } else {
                        self.eeprom_phase = BandaiEepromPhase::Idle;
                        self.eeprom_data_out = true;
                    }
                }
                BandaiEepromPhase::AckPending(_)
                | BandaiEepromPhase::AckLow(_)
                | BandaiEepromPhase::Idle => {}
            }
        }

        if self.eeprom_prev_scl && !scl {
            match self.eeprom_phase {
                BandaiEepromPhase::AckPending(next) => {
                    self.eeprom_phase = BandaiEepromPhase::AckLow(next);
                    self.eeprom_data_out = false;
                }
                BandaiEepromPhase::AckLow(next) => {
                    self.eeprom_transition_after_ack(next, storage);
                }
                BandaiEepromPhase::Sending { byte, bit_index } => {
                    self.eeprom_data_out = ((byte >> bit_index) & 0x01) != 0;
                }
                BandaiEepromPhase::WaitAckPending => {
                    self.eeprom_phase = BandaiEepromPhase::WaitAck;
                    self.eeprom_data_out = true;
                }
                _ => {}
            }
        }

        self.eeprom_prev_scl = scl;
        self.eeprom_prev_sda = sda;
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
        let Cartridge {
            bandai_fcg,
            prg_ram,
            has_valid_save_data,
            mirroring,
            ..
        } = self;
        if let Some(ref mut bandai) = bandai_fcg {
            let reg = addr & 0x0F;
            match reg {
                0x00..=0x07 => {
                    bandai.chr_banks[reg as usize] = data;
                }
                0x08 => {
                    bandai.prg_bank = data & 0x0F;
                }
                0x09 => {
                    *mirroring = match data & 0x03 {
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
                    bandai.eeprom_clock_control(data, prg_ram, has_valid_save_data);
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
        if let Some(ref bandai) = self.bandai_fcg {
            if self.has_battery {
                return if bandai.eeprom_data_out { 0x10 } else { 0 };
            }
        }

        let ram_addr = (addr - 0x6000) as usize;
        if ram_addr < self.prg_ram.len() {
            self.prg_ram[ram_addr]
        } else {
            0
        }
    }

    pub(in crate::cartridge) fn write_prg_ram_bandai(&mut self, addr: u16, data: u8) {
        if self.has_battery {
            return;
        }
        let ram_addr = (addr - 0x6000) as usize;
        if ram_addr < self.prg_ram.len() {
            self.prg_ram[ram_addr] = data;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EEPROM_READ: u8 = 0x80;
    const EEPROM_SDA: u8 = 0x40;
    const EEPROM_SCL: u8 = 0x20;

    fn make_bandai_eeprom_cart() -> Cartridge {
        Cartridge {
            prg_rom: vec![0; 0x8000],
            chr_rom: vec![0; 0x2000],
            chr_ram: vec![],
            prg_ram: vec![0xFF; 256],
            has_valid_save_data: false,
            mapper: 16,
            mirroring: Mirroring::Horizontal,
            has_battery: true,
            chr_bank: 0,
            chr_bank_1: 1,
            prg_bank: 0,
            mapper34_nina001: false,
            mapper93_chr_ram_enabled: true,
            mapper78_hv_mirroring: false,
            mapper58_nrom128: false,
            mapper225_nrom128: false,
            mapper232_outer_bank: 0,
            mapper233_nrom128: false,
            mapper234_reg0: 0,
            mapper234_reg1: 0,
            mapper235_nrom128: false,
            mapper202_32k_mode: false,
            mapper212_32k_mode: false,
            mapper226_nrom128: false,
            mapper230_contra_mode: false,
            mapper230_nrom128: false,
            mapper228_chip_select: 0,
            mapper228_nrom128: false,
            mapper242_latch: 0,
            mapper243_index: 0,
            mapper243_registers: [0; 8],
            mapper221_mode: 0,
            mapper221_outer_bank: 0,
            mapper221_chr_write_protect: false,
            mapper191_outer_bank: 0,
            mapper195_mode: 0x80,
            mapper208_protection_index: 0,
            mapper208_protection_regs: [0; 4],
            mmc1: None,
            mmc2: None,
            mmc3: None,
            fme7: None,
            bandai_fcg: Some(BandaiFcg::new()),
            vrc1: None,
            mapper15: None,
            sunsoft4: None,
            taito_tc0190: None,
            taito_x1005: None,
            taito_x1017: None,
            mapper227_latch: 0,
            mapper246: None,
            mapper236_mode: 0,
            mapper236_outer_bank: 0,
            mapper236_chr_ram: false,
        }
    }

    fn drive(cart: &mut Cartridge, read: bool, sda: bool, scl: bool) {
        let mut data = 0;
        if read {
            data |= EEPROM_READ;
        }
        if sda {
            data |= EEPROM_SDA;
        }
        if scl {
            data |= EEPROM_SCL;
        }
        cart.write_prg_bandai(0x800D, data);
    }

    fn start(cart: &mut Cartridge) {
        drive(cart, false, true, false);
        drive(cart, false, true, true);
        drive(cart, false, false, true);
        drive(cart, false, false, false);
    }

    fn stop(cart: &mut Cartridge) {
        drive(cart, false, false, false);
        drive(cart, false, false, true);
        drive(cart, false, true, true);
        drive(cart, false, true, false);
    }

    fn write_bit(cart: &mut Cartridge, bit: bool) {
        drive(cart, false, bit, false);
        drive(cart, false, bit, true);
        drive(cart, false, bit, false);
    }

    fn read_bit(cart: &mut Cartridge) -> bool {
        drive(cart, true, true, false);
        drive(cart, true, true, true);
        let bit = cart.read_prg_ram_bandai(0x6000) & 0x10 != 0;
        drive(cart, true, true, false);
        bit
    }

    fn write_byte(cart: &mut Cartridge, byte: u8) -> bool {
        for shift in (0..8).rev() {
            write_bit(cart, ((byte >> shift) & 1) != 0);
        }
        !read_bit(cart)
    }

    fn read_byte(cart: &mut Cartridge, ack: bool) -> u8 {
        let mut byte = 0;
        for _ in 0..8 {
            byte = (byte << 1) | u8::from(read_bit(cart));
        }
        write_bit(cart, !ack);
        byte
    }

    #[test]
    fn bandai_eeprom_round_trips_a_byte() {
        let mut cart = make_bandai_eeprom_cart();

        start(&mut cart);
        assert!(write_byte(&mut cart, 0xA0));
        assert!(write_byte(&mut cart, 0x2A));
        assert!(write_byte(&mut cart, 0x5C));
        stop(&mut cart);

        start(&mut cart);
        assert!(write_byte(&mut cart, 0xA0));
        assert!(write_byte(&mut cart, 0x2A));
        start(&mut cart);
        assert!(write_byte(&mut cart, 0xA1));
        let value = read_byte(&mut cart, false);
        stop(&mut cart);

        assert_eq!(value, 0x5C);
        assert_eq!(cart.prg_ram[0x2A], 0x5C);
        assert!(cart.has_valid_save_data);
    }

    #[test]
    fn bandai_eeprom_idle_line_reads_high() {
        let mut cart = make_bandai_eeprom_cart();
        drive(&mut cart, true, true, true);
        assert_eq!(cart.read_prg_ram_bandai(0x6000) & 0x10, 0x10);
    }
}
