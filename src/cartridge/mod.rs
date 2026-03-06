mod mapper;

use mapper::{BandaiFcg, Fme7, Mmc1, Mmc2, Mmc3};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{Read, Result};

pub struct Cartridge {
    prg_rom: Vec<u8>,
    chr_rom: Vec<u8>,
    chr_ram: Vec<u8>, // CHR-RAM for MMC1 and other mappers
    prg_ram: Vec<u8>, // Battery-backed SRAM for save data
    has_valid_save_data: bool,
    mapper: u8,
    mirroring: Mirroring,
    has_battery: bool,
    chr_bank: u8,
    prg_bank: u8,
    mmc1: Option<Mmc1>,
    mmc2: Option<Mmc2>,
    mmc3: Option<Mmc3>,
    fme7: Option<Fme7>,
    bandai_fcg: Option<BandaiFcg>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Mirroring {
    Horizontal,
    Vertical,
    FourScreen,
    OneScreenLower,
    OneScreenUpper,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mmc1State {
    pub shift_register: u8,
    pub shift_count: u8,
    pub control: u8,
    pub chr_bank_0: u8,
    pub chr_bank_1: u8,
    pub prg_bank: u8,
    pub prg_ram_disable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mmc2State {
    pub prg_bank: u8,
    pub chr_bank_0_fd: u8,
    pub chr_bank_0_fe: u8,
    pub chr_bank_1_fd: u8,
    pub chr_bank_1_fe: u8,
    pub latch_0: bool,
    pub latch_1: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mmc3State {
    pub bank_select: u8,
    pub bank_registers: [u8; 8],
    pub irq_latch: u8,
    pub irq_counter: u8,
    pub irq_reload: bool,
    pub irq_enabled: bool,
    pub irq_pending: bool,
    pub prg_ram_enabled: bool,
    pub prg_ram_write_protect: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fme7State {
    pub command: u8,
    pub chr_banks: [u8; 8],
    pub prg_banks: [u8; 3],
    pub prg_bank_6000: u8,
    pub prg_ram_enabled: bool,
    pub prg_ram_select: bool,
    pub irq_counter: u16,
    pub irq_counter_enabled: bool,
    pub irq_enabled: bool,
    pub irq_pending: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BandaiFcgState {
    pub chr_banks: [u8; 8],
    pub prg_bank: u8,
    pub irq_counter: u16,
    pub irq_latch: u16,
    pub irq_enabled: bool,
    pub irq_pending: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CartridgeState {
    pub mapper: u8,
    pub mirroring: Mirroring,
    pub prg_bank: u8,
    pub chr_bank: u8,
    pub prg_ram: Vec<u8>,
    pub chr_ram: Vec<u8>,
    pub has_valid_save_data: bool,
    pub mmc1: Option<Mmc1State>,
    pub mmc2: Option<Mmc2State>,
    #[serde(default)]
    pub mmc3: Option<Mmc3State>,
    #[serde(default)]
    pub fme7: Option<Fme7State>,
    #[serde(default)]
    pub bandai_fcg: Option<BandaiFcgState>,
}

impl Cartridge {
    pub fn load(path: &str) -> Result<Self> {
        let mut file = File::open(path)?;
        let mut data = Vec::new();
        file.read_to_end(&mut data)?;

        if &data[0..4] != b"NES\x1a" {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Invalid NES file format",
            ));
        }

        let prg_rom_size = data[4] as usize * 16384;
        let chr_rom_size = data[5] as usize * 8192;
        let flags6 = data[6];
        let flags7 = data[7];

        let has_battery = (flags6 & 0x02) != 0;

        let mirroring = if flags6 & 0x08 != 0 {
            Mirroring::FourScreen
        } else if flags6 & 0x01 != 0 {
            Mirroring::Vertical
        } else {
            Mirroring::Horizontal
        };

        let mapper = (flags7 & 0xF0) | (flags6 >> 4);

        let prg_rom_start = 16;
        let chr_rom_start = prg_rom_start + prg_rom_size;

        let prg_rom = data[prg_rom_start..prg_rom_start + prg_rom_size].to_vec();
        let chr_rom = if chr_rom_size > 0 {
            data[chr_rom_start..chr_rom_start + chr_rom_size].to_vec()
        } else {
            // CHR RAM - initialize with zeros
            vec![0; 8192]
        };

        let mmc1 = if mapper == 1 { Some(Mmc1::new()) } else { None };
        let mmc2 = if mapper == 9 || mapper == 10 {
            Some(Mmc2::new())
        } else {
            None
        };
        let mmc3 = if mapper == 4 { Some(Mmc3::new()) } else { None };
        let fme7 = if mapper == 69 {
            Some(Fme7::new())
        } else {
            None
        };
        let bandai_fcg = if mapper == 16 {
            Some(BandaiFcg::new())
        } else {
            None
        };

        // Initialize PRG-RAM for mappers that support it
        let prg_ram = if mapper == 1
            || mapper == 4
            || mapper == 9
            || mapper == 10
            || mapper == 16
            || mapper == 69
        {
            vec![0x00; 8192]
        } else {
            Vec::new()
        };

        let chr_ram = if (mapper == 1 || mapper == 4 || mapper == 10) && chr_rom_size == 0 {
            vec![0x00; 8192]
        } else {
            vec![]
        };

        let cartridge = Cartridge {
            prg_rom,
            chr_rom,
            chr_ram,
            prg_ram,
            has_valid_save_data: false,
            mapper,
            mirroring,
            has_battery,
            chr_bank: 0,
            prg_bank: 0,
            mmc1,
            mmc2,
            mmc3,
            fme7,
            bandai_fcg,
        };

        Ok(cartridge)
    }

    pub fn read_prg(&self, addr: u16) -> u8 {
        let rom_addr = addr - 0x8000;
        match self.mapper {
            0 | 3 | 87 => self.read_prg_nrom(rom_addr),
            1 => self.read_prg_mmc1(addr, rom_addr),
            2 => self.read_prg_uxrom(addr, rom_addr),
            4 => self.read_prg_mmc3(addr),
            7 => self.read_prg_axrom(addr),
            9 | 10 => self.read_prg_mmc2(addr, rom_addr),
            16 => self.read_prg_bandai(addr),
            69 => self.read_prg_fme7(addr),
            _ => 0,
        }
    }

    pub fn write_prg(&mut self, addr: u16, data: u8) {
        match self.mapper {
            0 => {}
            1 => self.write_prg_mmc1(addr, data),
            2 => self.write_prg_uxrom(addr, data),
            3 => self.write_prg_cnrom(addr, data),
            4 => self.write_prg_mmc3(addr, data),
            7 => self.write_prg_axrom(addr, data),
            9 | 10 => self.write_prg_mmc2(addr, data),
            16 => self.write_prg_bandai(addr, data),
            69 => self.write_prg_fme7(addr, data),
            87 => self.write_prg_mapper87(addr, data),
            _ => {}
        }
    }

    #[inline]
    pub fn read_chr(&self, addr: u16) -> u8 {
        match self.mapper {
            0 => self.read_chr_nrom(addr),
            1 => self.read_chr_mmc1(addr),
            2 | 7 => self.read_chr_uxrom(addr),
            3 | 87 => self.read_chr_cnrom(addr),
            4 => self.read_chr_mmc3(addr),
            9 | 10 => self.read_chr_mmc2(addr),
            16 => self.read_chr_bandai(addr),
            69 => self.read_chr_fme7(addr),
            _ => {
                let chr_addr = (addr & 0x1FFF) as usize;
                if chr_addr < self.chr_rom.len() {
                    self.chr_rom[chr_addr]
                } else {
                    0
                }
            }
        }
    }

    pub fn read_chr_sprite(&self, addr: u16, _sprite_y: u8) -> u8 {
        self.read_chr(addr)
    }

    pub fn write_chr(&mut self, addr: u16, data: u8) {
        match self.mapper {
            0 => self.write_chr_nrom(addr, data),
            1 => self.write_chr_mmc1(addr, data),
            2 | 7 => self.write_chr_uxrom(addr, data),
            3 | 87 => self.write_chr_cnrom(addr, data),
            4 => self.write_chr_mmc3(addr, data),
            9 | 10 => self.write_chr_mmc2(addr, data),
            16 => self.write_chr_bandai(addr, data),
            69 => self.write_chr_fme7(addr, data),
            _ => {
                self.chr_rom[(addr & 0x1FFF) as usize] = data;
            }
        }
    }

    pub fn read_prg_ram(&self, addr: u16) -> u8 {
        match self.mapper {
            1 => self.read_prg_ram_mmc1(addr),
            4 => self.read_prg_ram_mmc3(addr),
            9 | 10 => self.read_prg_ram_mmc2(addr),
            16 => self.read_prg_ram_bandai(addr),
            69 => self.read_prg_ram_fme7(addr),
            _ => 0,
        }
    }

    pub fn write_prg_ram(&mut self, addr: u16, data: u8) {
        match self.mapper {
            87 => self.write_prg_mapper87(addr, data),
            1 => self.write_prg_ram_mmc1(addr, data),
            4 => self.write_prg_ram_mmc3(addr, data),
            9 | 10 => self.write_prg_ram_mmc2(addr, data),
            16 => self.write_prg_ram_bandai(addr, data),
            69 => self.write_prg_ram_fme7(addr, data),
            _ => {}
        }
    }

    pub fn mirroring(&self) -> Mirroring {
        self.mirroring
    }

    pub fn chr_rom_size(&self) -> usize {
        self.chr_rom.len()
    }

    pub fn mapper_number(&self) -> u8 {
        self.mapper
    }

    pub fn prg_rom_size(&self) -> usize {
        self.prg_rom.len()
    }

    pub fn get_prg_bank(&self) -> u8 {
        if let Some(ref mmc1) = self.mmc1 {
            mmc1.prg_bank
        } else {
            self.prg_bank
        }
    }

    pub fn get_chr_bank(&self) -> u8 {
        if let Some(ref mmc1) = self.mmc1 {
            mmc1.chr_bank_0
        } else {
            self.chr_bank
        }
    }

    pub fn set_prg_bank(&mut self, bank: u8) {
        self.prg_bank = bank;
        if let Some(ref mut mmc1) = self.mmc1 {
            mmc1.prg_bank = bank & 0x0F;
        }
    }

    pub fn set_chr_bank(&mut self, bank: u8) {
        self.chr_bank = bank;
        if let Some(ref mut mmc1) = self.mmc1 {
            mmc1.chr_bank_0 = bank;
            mmc1.chr_bank_1 = bank;
        }
    }

    pub fn has_battery_save(&self) -> bool {
        self.has_battery && !self.prg_ram.is_empty()
    }

    pub fn get_sram_data(&self) -> Option<&[u8]> {
        if self.has_battery && !self.prg_ram.is_empty() && self.has_valid_save_data {
            Some(&self.prg_ram)
        } else {
            None
        }
    }

    pub fn set_sram_data(&mut self, data: Vec<u8>) {
        if self.has_battery && data.len() == self.prg_ram.len() {
            self.prg_ram = data;
            self.has_valid_save_data = true;
        }
    }

    /// Direct reference to PRG-RAM (returns None if empty).
    pub fn prg_ram_ref(&self) -> Option<&[u8]> {
        if self.prg_ram.is_empty() {
            None
        } else {
            Some(&self.prg_ram)
        }
    }

    /// Mutable reference to PRG-RAM (returns None if empty).
    pub fn prg_ram_mut(&mut self) -> Option<&mut [u8]> {
        if self.prg_ram.is_empty() {
            None
        } else {
            Some(&mut self.prg_ram)
        }
    }

    pub fn snapshot_state(&self) -> CartridgeState {
        let mmc1 = self.mmc1.as_ref().map(|m| Mmc1State {
            shift_register: m.shift_register,
            shift_count: m.shift_count,
            control: m.control,
            chr_bank_0: m.chr_bank_0,
            chr_bank_1: m.chr_bank_1,
            prg_bank: m.prg_bank,
            prg_ram_disable: m.prg_ram_disable,
        });

        let mmc2 = self.mmc2.as_ref().map(|m| Mmc2State {
            prg_bank: m.prg_bank,
            chr_bank_0_fd: m.chr_bank_0_fd,
            chr_bank_0_fe: m.chr_bank_0_fe,
            chr_bank_1_fd: m.chr_bank_1_fd,
            chr_bank_1_fe: m.chr_bank_1_fe,
            latch_0: m.latch_0.get(),
            latch_1: m.latch_1.get(),
        });

        let mmc3 = self.mmc3.as_ref().map(|m| Mmc3State {
            bank_select: m.bank_select,
            bank_registers: m.bank_registers,
            irq_latch: m.irq_latch,
            irq_counter: m.irq_counter,
            irq_reload: m.irq_reload,
            irq_enabled: m.irq_enabled,
            irq_pending: m.irq_pending.get(),
            prg_ram_enabled: m.prg_ram_enabled,
            prg_ram_write_protect: m.prg_ram_write_protect,
        });

        let fme7 = self.fme7.as_ref().map(|f| Fme7State {
            command: f.command,
            chr_banks: f.chr_banks,
            prg_banks: f.prg_banks,
            prg_bank_6000: f.prg_bank_6000,
            prg_ram_enabled: f.prg_ram_enabled,
            prg_ram_select: f.prg_ram_select,
            irq_counter: f.irq_counter,
            irq_counter_enabled: f.irq_counter_enabled,
            irq_enabled: f.irq_enabled,
            irq_pending: f.irq_pending.get(),
        });

        let bandai_fcg = self.bandai_fcg.as_ref().map(|b| BandaiFcgState {
            chr_banks: b.chr_banks,
            prg_bank: b.prg_bank,
            irq_counter: b.irq_counter,
            irq_latch: b.irq_latch,
            irq_enabled: b.irq_enabled,
            irq_pending: b.irq_pending.get(),
        });

        CartridgeState {
            mapper: self.mapper,
            mirroring: self.mirroring,
            prg_bank: self.get_prg_bank(),
            chr_bank: self.get_chr_bank(),
            prg_ram: self.prg_ram.clone(),
            chr_ram: self.chr_ram.clone(),
            has_valid_save_data: self.has_valid_save_data,
            mmc1,
            mmc2,
            mmc3,
            fme7,
            bandai_fcg,
        }
    }

    pub fn restore_state(&mut self, state: &CartridgeState) {
        if state.mapper != self.mapper {
            return;
        }

        self.mirroring = state.mirroring;
        self.set_prg_bank(state.prg_bank);
        self.set_chr_bank(state.chr_bank);
        self.has_valid_save_data = state.has_valid_save_data;

        let prg_len = self.prg_ram.len().min(state.prg_ram.len());
        if prg_len > 0 {
            self.prg_ram[..prg_len].copy_from_slice(&state.prg_ram[..prg_len]);
        }

        let chr_len = self.chr_ram.len().min(state.chr_ram.len());
        if chr_len > 0 {
            self.chr_ram[..chr_len].copy_from_slice(&state.chr_ram[..chr_len]);
        }

        if let (Some(ref mut mmc1), Some(saved)) = (self.mmc1.as_mut(), state.mmc1.as_ref()) {
            mmc1.shift_register = saved.shift_register;
            mmc1.shift_count = saved.shift_count;
            mmc1.control = saved.control;
            mmc1.chr_bank_0 = saved.chr_bank_0;
            mmc1.chr_bank_1 = saved.chr_bank_1;
            mmc1.prg_bank = saved.prg_bank;
            mmc1.prg_ram_disable = saved.prg_ram_disable;
        }

        if let (Some(ref mut mmc2), Some(saved)) = (self.mmc2.as_mut(), state.mmc2.as_ref()) {
            mmc2.prg_bank = saved.prg_bank;
            mmc2.chr_bank_0_fd = saved.chr_bank_0_fd;
            mmc2.chr_bank_0_fe = saved.chr_bank_0_fe;
            mmc2.chr_bank_1_fd = saved.chr_bank_1_fd;
            mmc2.chr_bank_1_fe = saved.chr_bank_1_fe;
            mmc2.latch_0.set(saved.latch_0);
            mmc2.latch_1.set(saved.latch_1);
        }

        if let (Some(ref mut mmc3), Some(saved)) = (self.mmc3.as_mut(), state.mmc3.as_ref()) {
            mmc3.bank_select = saved.bank_select;
            mmc3.bank_registers = saved.bank_registers;
            mmc3.irq_latch = saved.irq_latch;
            mmc3.irq_counter = saved.irq_counter;
            mmc3.irq_reload = saved.irq_reload;
            mmc3.irq_enabled = saved.irq_enabled;
            mmc3.irq_pending.set(saved.irq_pending);
            mmc3.prg_ram_enabled = saved.prg_ram_enabled;
            mmc3.prg_ram_write_protect = saved.prg_ram_write_protect;
        }

        if let (Some(ref mut fme7), Some(saved)) = (self.fme7.as_mut(), state.fme7.as_ref()) {
            fme7.command = saved.command;
            fme7.chr_banks = saved.chr_banks;
            fme7.prg_banks = saved.prg_banks;
            fme7.prg_bank_6000 = saved.prg_bank_6000;
            fme7.prg_ram_enabled = saved.prg_ram_enabled;
            fme7.prg_ram_select = saved.prg_ram_select;
            fme7.irq_counter = saved.irq_counter;
            fme7.irq_counter_enabled = saved.irq_counter_enabled;
            fme7.irq_enabled = saved.irq_enabled;
            fme7.irq_pending.set(saved.irq_pending);
        }

        if let (Some(ref mut bandai), Some(saved)) =
            (self.bandai_fcg.as_mut(), state.bandai_fcg.as_ref())
        {
            bandai.chr_banks = saved.chr_banks;
            bandai.prg_bank = saved.prg_bank;
            bandai.irq_counter = saved.irq_counter;
            bandai.irq_latch = saved.irq_latch;
            bandai.irq_enabled = saved.irq_enabled;
            bandai.irq_pending.set(saved.irq_pending);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_mmc1_cart() -> Cartridge {
        Cartridge {
            prg_rom: vec![0; 0x8000],
            chr_rom: vec![0; 0x2000],
            chr_ram: vec![0; 0x2000],
            prg_ram: vec![0; 0x2000],
            has_valid_save_data: true,
            mapper: 1,
            mirroring: Mirroring::Vertical,
            has_battery: true,
            chr_bank: 0,
            prg_bank: 0,
            mmc1: Some(Mmc1::new()),
            mmc2: None,
            mmc3: None,
            fme7: None,
            bandai_fcg: None,
        }
    }

    #[test]
    fn snapshot_and_restore_keeps_mmc1_and_ram_state() {
        let mut cart = make_mmc1_cart();
        {
            let mmc1 = cart.mmc1.as_mut().unwrap();
            mmc1.shift_register = 0x1B;
            mmc1.shift_count = 3;
            mmc1.control = 0x12;
            mmc1.chr_bank_0 = 7;
            mmc1.chr_bank_1 = 9;
            mmc1.prg_bank = 5;
            mmc1.prg_ram_disable = true;
        }
        cart.prg_ram[0x10] = 0xAA;
        cart.chr_ram[0x20] = 0x55;

        let snapshot = cart.snapshot_state();

        cart.mirroring = Mirroring::Horizontal;
        cart.has_valid_save_data = false;
        cart.prg_ram.fill(0);
        cart.chr_ram.fill(0);
        {
            let mmc1 = cart.mmc1.as_mut().unwrap();
            mmc1.shift_register = 0x10;
            mmc1.shift_count = 0;
            mmc1.control = 0x0C;
            mmc1.chr_bank_0 = 0;
            mmc1.chr_bank_1 = 0;
            mmc1.prg_bank = 0;
            mmc1.prg_ram_disable = false;
        }

        cart.restore_state(&snapshot);

        assert_eq!(cart.mirroring, Mirroring::Vertical);
        assert!(cart.has_valid_save_data);
        assert_eq!(cart.prg_ram[0x10], 0xAA);
        assert_eq!(cart.chr_ram[0x20], 0x55);

        let mmc1 = cart.mmc1.as_ref().unwrap();
        assert_eq!(mmc1.shift_register, 0x1B);
        assert_eq!(mmc1.shift_count, 3);
        assert_eq!(mmc1.control, 0x12);
        assert_eq!(mmc1.chr_bank_0, 7);
        assert_eq!(mmc1.chr_bank_1, 9);
        assert_eq!(mmc1.prg_bank, 5);
        assert!(mmc1.prg_ram_disable);
    }

    #[test]
    fn restore_state_ignores_mapper_mismatch() {
        let mut cart = make_mmc1_cart();
        let mut state = cart.snapshot_state();
        state.mapper = 2;

        cart.prg_ram[0] = 0x11;
        cart.restore_state(&state);
        assert_eq!(cart.prg_ram[0], 0x11);
    }
}
