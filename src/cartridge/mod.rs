mod load;
mod mapper;
mod state;

use mapper::{
    BandaiFcg, Fme7, Mapper15, Mapper246, Mmc1, Mmc2, Mmc3, Sunsoft4, TaitoTc0190, TaitoX1005,
    TaitoX1017, Vrc1,
};
use serde::{Deserialize, Serialize};
pub use state::*;

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
    chr_bank_1: u8,
    prg_bank: u8,
    mapper34_nina001: bool,
    mapper93_chr_ram_enabled: bool,
    mapper78_hv_mirroring: bool,
    mapper58_nrom128: bool,
    mapper225_nrom128: bool,
    mapper232_outer_bank: u8,
    mapper233_nrom128: bool,
    mapper234_reg0: u8,
    mapper234_reg1: u8,
    mapper235_nrom128: bool,
    mapper202_32k_mode: bool,
    mapper212_32k_mode: bool,
    mapper226_nrom128: bool,
    mapper230_contra_mode: bool,
    mapper230_nrom128: bool,
    mapper228_chip_select: u8,
    mapper228_nrom128: bool,
    mapper242_latch: u16,
    mapper243_index: u8,
    mapper243_registers: [u8; 8],
    mapper221_mode: u8,
    mapper221_outer_bank: u8,
    mapper221_chr_write_protect: bool,
    mapper191_outer_bank: u8,
    mapper195_mode: u8,
    mapper208_protection_index: u8,
    mapper208_protection_regs: [u8; 4],
    mmc1: Option<Mmc1>,
    mmc2: Option<Mmc2>,
    mmc3: Option<Mmc3>,
    fme7: Option<Fme7>,
    bandai_fcg: Option<BandaiFcg>,
    vrc1: Option<Vrc1>,
    mapper15: Option<Mapper15>,
    sunsoft4: Option<Sunsoft4>,
    taito_tc0190: Option<TaitoTc0190>,
    taito_x1005: Option<TaitoX1005>,
    taito_x1017: Option<TaitoX1017>,
    mapper227_latch: u16,
    mapper246: Option<Mapper246>,
    mapper236_mode: u8,
    mapper236_outer_bank: u8,
    mapper236_chr_ram: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Mirroring {
    Horizontal,
    HorizontalSwapped,
    ThreeScreenLower,
    Vertical,
    FourScreen,
    OneScreenLower,
    OneScreenUpper,
}

impl Cartridge {
    pub fn read_prg(&self, addr: u16) -> u8 {
        let rom_addr = addr - 0x8000;
        match self.mapper {
            0 | 3 | 13 | 87 | 101 | 184 => self.read_prg_nrom(rom_addr),
            1 => self.read_prg_mmc1(addr, rom_addr),
            208 => self.read_prg_mapper208(addr),
            15 => self.read_prg_mapper15(addr),
            33 => self.read_prg_taito_tc0190(addr),
            221 => self.read_prg_mapper221(addr),
            225 | 255 => self.read_prg_mapper225(addr),
            230 => self.read_prg_mapper230(addr),
            227 => self.read_prg_mapper227(addr),
            231 => self.read_prg_mapper231(addr),
            236 => self.read_prg_mapper236(addr),
            232 => self.read_prg_mapper232(addr),
            233 => self.read_prg_mapper233(addr),
            234 => self.read_prg_mapper234(addr),
            235 => self.read_prg_mapper235(addr),
            246 => self.read_prg_mapper246(addr),
            200 | 203 => self.read_prg_mapper200(addr),
            202 => self.read_prg_mapper202(addr),
            212 => self.read_prg_mapper212(addr),
            226 => self.read_prg_mapper226(addr),
            228 => self.read_prg_mapper228(addr),
            242 => self.read_prg_mapper242(addr),
            2 | 70 | 71 | 72 | 81 | 86 | 89 | 93 | 152 => self.read_prg_uxrom(addr, rom_addr),
            34 | 38 | 46 | 79 | 113 | 133 | 140 | 144 | 146 | 147 | 148 | 201 | 240 | 241 | 243 => {
                self.read_prg_axrom(addr)
            }
            58 | 213 => self.read_prg_mapper58(addr),
            229 => self.read_prg_mapper229(addr),
            92 | 180 => self.read_prg_uxrom_inverted(addr, rom_addr),
            97 => self.read_prg_fixed_last_switch_high(addr, rom_addr),
            191 => self.read_prg_mapper191(addr),
            4 | 74 | 118 | 119 | 192 | 194 | 195 | 250 => self.read_prg_mmc3(addr),
            245 => self.read_prg_mapper245(addr),
            68 => self.read_prg_sunsoft4(addr),
            75 => self.read_prg_vrc1(addr),
            80 | 207 => self.read_prg_taito_x1005(addr),
            82 => self.read_prg_taito_x1017(addr),
            76 | 88 | 95 | 154 | 206 => self.read_prg_namco108(addr),
            7 | 11 | 66 | 107 => self.read_prg_axrom(addr),
            78 | 94 => self.read_prg_uxrom(addr, rom_addr),
            9 | 10 => self.read_prg_mmc2(addr, rom_addr),
            16 => self.read_prg_bandai(addr),
            69 => self.read_prg_fme7(addr),
            _ => 0,
        }
    }

    pub fn read_prg_cpu(&mut self, addr: u16) -> u8 {
        let value = self.read_prg(addr);
        if self.mapper == 234 {
            self.apply_mapper234_value(addr, value);
        }
        value
    }

    pub fn write_prg(&mut self, addr: u16, data: u8) {
        match self.mapper {
            0 => {}
            1 => self.write_prg_mmc1(addr, data),
            208 => self.write_prg_mapper208(addr, data),
            15 => self.write_prg_mapper15(addr, data),
            33 => self.write_prg_taito_tc0190(addr, data),
            221 => self.write_prg_mapper221(addr),
            2 => self.write_prg_uxrom(addr, data),
            225 | 255 => self.write_prg_mapper225(addr, data),
            230 => self.write_prg_mapper230(addr, data),
            227 => self.write_prg_mapper227(addr),
            231 => self.write_prg_mapper231(addr),
            236 => self.write_prg_mapper236(addr, data),
            232 => self.write_prg_mapper232(addr, data),
            233 => self.write_prg_mapper233(addr, data),
            234 => self.write_prg_mapper234(addr, data),
            235 => self.write_prg_mapper235(addr, data),
            3 => self.write_prg_cnrom(addr, data),
            13 => self.write_prg_cprom(addr, data),
            34 if self.mapper34_nina001 => {}
            34 => self.write_prg_bnrom(addr, data),
            46 => self.write_prg_mapper46_inner(addr, data),
            191 => self.write_prg_mapper191(addr, data),
            4 | 74 | 118 | 119 | 192 | 194 | 195 | 245 => self.write_prg_mmc3(addr, data),
            250 => self.write_prg_mapper250(addr, data),
            58 | 213 => self.write_prg_mapper58(addr),
            72 | 92 => self.write_prg_mapper72_92(addr, data),
            68 => self.write_prg_sunsoft4(addr, data),
            75 => self.write_prg_vrc1(addr, data),
            76 | 88 | 95 | 154 | 206 => self.write_prg_namco108(addr, data),
            7 => self.write_prg_axrom(addr, data),
            200 => self.write_prg_mapper200(addr),
            201 => self.write_prg_mapper201(addr),
            202 => self.write_prg_mapper202(addr),
            203 => self.write_prg_mapper203(addr, data),
            212 => self.write_prg_mapper212(addr),
            226 => self.write_prg_mapper226(addr, data),
            228 => self.write_prg_mapper228(addr, data),
            229 => self.write_prg_mapper229(addr),
            242 => self.write_prg_mapper242(addr),
            243 => self.write_prg_mapper243(addr, data),
            241 => self.write_prg_mapper241(addr, data),
            79 | 146 => self.write_prg_mapper79_146(addr, data),
            133 => self.write_prg_mapper133(addr, data),
            113 => self.write_prg_mapper113(addr, data),
            144 => self.write_prg_mapper144(addr, data),
            145 => self.write_prg_mapper145(addr, data),
            11 => self.write_prg_color_dreams(addr, data),
            89 => self.write_prg_mapper89(addr, data),
            93 => self.write_prg_mapper93(addr, data),
            70 => self.write_prg_mapper70(addr, data),
            71 => self.write_prg_camerica(addr, data),
            81 => self.write_prg_mapper81(addr),
            97 => self.write_prg_mapper97(addr, data),
            78 => self.write_prg_mapper78(addr, data),
            80 => {}
            86 => self.write_prg_mapper86(addr, data),
            94 => self.write_prg_mapper94(addr, data),
            107 => self.write_prg_mapper107(addr, data),
            147 => self.write_prg_mapper147(addr, data),
            148 => self.write_prg_mapper148(addr, data),
            152 => self.write_prg_mapper152(addr, data),
            180 => self.write_prg_uxrom_inverted(addr, data),
            240 => self.write_prg_mapper240(addr, data),
            9 | 10 => self.write_prg_mmc2(addr, data),
            16 => self.write_prg_bandai(addr, data),
            66 => self.write_prg_gxrom(addr, data),
            69 => self.write_prg_fme7(addr, data),
            87 => self.write_prg_mapper87(addr, data),
            _ => {}
        }
    }

    #[inline]
    pub fn read_chr(&self, addr: u16) -> u8 {
        match self.mapper {
            0 | 97 | 226 | 241 | 242 => self.read_chr_nrom(addr),
            1 => self.read_chr_mmc1(addr),
            2 | 7 | 15 | 71 | 94 | 180 | 227 | 230 | 232 | 235 => self.read_chr_uxrom(addr),
            221 => self.read_chr_mapper221(addr),
            231 => self.read_chr_mapper231(addr),
            236 => self.read_chr_mapper236(addr),
            75 => self.read_chr_vrc1(addr),
            13 => self.read_chr_cprom(addr),
            34 if self.mapper34_nina001 => self.read_chr_nina001(addr),
            184 => self.read_chr_split_4k(addr),
            33 => self.read_chr_taito_tc0190(addr),
            34 => self.read_chr_nrom(addr),
            3 | 11 | 38 | 46 | 58 | 66 | 70 | 72 | 78 | 79 | 81 | 86 | 89 | 92 | 101 | 107
            | 113 | 133 | 140 | 144 | 145 | 146 | 147 | 148 | 152 | 200 | 201 | 202 | 203 | 212
            | 213 | 225 | 228 | 229 | 233 | 234 | 240 | 243 | 255 | 87 => self.read_chr_cnrom(addr),
            93 => self.read_chr_mapper93(addr),
            4 | 118 | 208 | 250 => self.read_chr_mmc3(addr),
            74 => self.read_chr_mapper74(addr),
            119 => self.read_chr_mapper119(addr),
            191 => self.read_chr_mapper191(addr),
            192 => self.read_chr_mapper192(addr),
            194 => self.read_chr_mapper194(addr),
            195 => self.read_chr_mapper195(addr),
            245 => self.read_chr_mapper245(addr),
            246 => self.read_chr_mapper246(addr),
            68 => self.read_chr_sunsoft4(addr),
            80 | 207 => self.read_chr_taito_x1005(addr),
            82 => self.read_chr_taito_x1017(addr),
            76 | 88 | 95 | 154 | 206 => self.read_chr_namco108(addr),
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
            0 | 97 | 226 | 241 => self.write_chr_nrom(addr, data),
            1 => self.write_chr_mmc1(addr, data),
            2 | 7 | 15 | 71 | 94 | 180 | 230 | 232 | 235 => self.write_chr_uxrom(addr, data),
            221 => self.write_chr_mapper221(addr, data),
            227 => self.write_chr_mapper227(addr, data),
            231 => self.write_chr_mapper231(addr, data),
            236 => self.write_chr_mapper236(addr, data),
            75 => self.write_chr_vrc1(addr, data),
            13 => self.write_chr_cprom(addr, data),
            34 if self.mapper34_nina001 => self.write_chr_nina001(addr, data),
            184 => self.write_chr_split_4k(addr, data),
            33 => self.write_chr_taito_tc0190(addr, data),
            34 => self.write_chr_nrom(addr, data),
            3 | 11 | 38 | 46 | 58 | 66 | 70 | 72 | 78 | 79 | 81 | 86 | 89 | 92 | 101 | 107
            | 113 | 133 | 140 | 144 | 145 | 146 | 147 | 148 | 152 | 200 | 201 | 202 | 203 | 212
            | 213 | 225 | 228 | 229 | 233 | 234 | 240 | 243 | 255 | 87 => {
                self.write_chr_cnrom(addr, data)
            }
            242 => self.write_chr_mapper242(addr, data),
            93 => self.write_chr_mapper93(addr, data),
            4 | 118 | 208 | 250 => self.write_chr_mmc3(addr, data),
            74 => self.write_chr_mapper74(addr, data),
            119 => self.write_chr_mapper119(addr, data),
            191 => self.write_chr_mapper191(addr, data),
            192 => self.write_chr_mapper192(addr, data),
            194 => self.write_chr_mapper194(addr, data),
            195 => self.write_chr_mapper195(addr, data),
            245 => self.write_chr_mapper245(addr, data),
            68 => self.write_chr_sunsoft4(addr, data),
            80 | 207 => self.write_chr_taito_x1005(addr, data),
            82 => self.write_chr_taito_x1017(addr, data),
            246 => {}
            76 | 88 | 95 | 154 | 206 => self.write_chr_namco108(addr, data),
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
            15 => self.read_prg_ram_mapper15(addr),
            80 | 207 => self.read_prg_ram_taito_x1005(addr),
            82 => self.read_prg_ram_taito_x1017(addr),
            227 => {
                let offset = (addr - 0x6000) as usize;
                if offset < self.prg_ram.len() {
                    self.prg_ram[offset]
                } else {
                    0
                }
            }
            212 => 0x80,
            246 => self.read_prg_ram_mapper246(addr),
            241 => {
                let offset = (addr - 0x6000) as usize;
                if offset < self.prg_ram.len() {
                    self.prg_ram[offset]
                } else {
                    0
                }
            }
            34 if self.mapper34_nina001 => {
                let offset = (addr - 0x6000) as usize;
                if offset < self.prg_ram.len() {
                    self.prg_ram[offset]
                } else {
                    0
                }
            }
            4 | 74 | 118 | 119 | 192 | 194 | 245 | 250 => self.read_prg_ram_mmc3(addr),
            68 => self.read_prg_ram_sunsoft4(addr),
            240 => {
                let offset = (addr - 0x6000) as usize;
                if offset < self.prg_ram.len() {
                    self.prg_ram[offset]
                } else {
                    0
                }
            }
            9 | 10 => self.read_prg_ram_mmc2(addr),
            16 => self.read_prg_ram_bandai(addr),
            69 => self.read_prg_ram_fme7(addr),
            _ => 0,
        }
    }

    pub fn read_prg_low(&self, addr: u16) -> u8 {
        match self.mapper {
            208 => self.read_prg_low_mapper208(addr),
            225 => self.read_prg_low_mapper225(addr),
            243 => self.read_prg_low_mapper243(addr),
            _ => 0,
        }
    }

    pub fn write_prg_ram(&mut self, addr: u16, data: u8) {
        match self.mapper {
            46 => self.write_prg_mapper46_outer(addr, data),
            38 => self.write_prg_mapper38(addr, data),
            87 => self.write_prg_mapper87(addr, data),
            86 => self.write_prg_mapper86(addr, data),
            101 => self.write_prg_mapper101(addr, data),
            15 => self.write_prg_ram_mapper15(addr, data),
            208 => self.write_prg_ram_mapper208(addr, data),
            80 | 207 => self.write_prg_ram_taito_x1005(addr, data),
            82 => self.write_prg_ram_taito_x1017(addr, data),
            140 => self.write_prg_mapper140(addr, data),
            1 => self.write_prg_ram_mmc1(addr, data),
            34 if self.mapper34_nina001 => self.write_prg_ram_nina001(addr, data),
            184 => self.write_prg_mapper184(addr, data),
            227 => {
                let offset = (addr - 0x6000) as usize;
                if offset < self.prg_ram.len() {
                    self.prg_ram[offset] = data;
                }
            }
            4 | 74 | 118 | 119 | 192 | 194 | 245 | 250 => self.write_prg_ram_mmc3(addr, data),
            68 => self.write_prg_ram_sunsoft4(addr, data),
            246 => self.write_prg_ram_mapper246(addr, data),
            241 => {
                let offset = (addr - 0x6000) as usize;
                if offset < self.prg_ram.len() {
                    self.prg_ram[offset] = data;
                }
            }
            240 => {
                let offset = (addr - 0x6000) as usize;
                if offset < self.prg_ram.len() {
                    self.prg_ram[offset] = data;
                }
            }
            9 | 10 => self.write_prg_ram_mmc2(addr, data),
            16 => self.write_prg_ram_bandai(addr, data),
            69 => self.write_prg_ram_fme7(addr, data),
            _ => {}
        }
    }

    pub fn mirroring(&self) -> Mirroring {
        self.mirroring
    }

    pub fn on_reset(&mut self) {
        if self.mapper == 230 {
            self.mapper230_contra_mode = !self.mapper230_contra_mode;
            if self.mapper230_contra_mode {
                self.mirroring = Mirroring::Vertical;
            }
        }
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
        } else if let Some(ref vrc1) = self.vrc1 {
            vrc1.prg_banks[0]
        } else if let Some(ref sunsoft4) = self.sunsoft4 {
            sunsoft4.prg_bank
        } else if let Some(ref taito_tc0190) = self.taito_tc0190 {
            taito_tc0190.prg_banks[0]
        } else if let Some(ref taito_x1005) = self.taito_x1005 {
            taito_x1005.prg_banks[0]
        } else if let Some(ref taito_x1017) = self.taito_x1017 {
            taito_x1017.prg_banks[0]
        } else {
            self.prg_bank
        }
    }

    pub fn get_chr_bank(&self) -> u8 {
        if let Some(ref mmc1) = self.mmc1 {
            mmc1.chr_bank_0
        } else if let Some(ref vrc1) = self.vrc1 {
            vrc1.chr_bank_0
        } else if let Some(ref sunsoft4) = self.sunsoft4 {
            sunsoft4.chr_banks[0]
        } else if let Some(ref taito_tc0190) = self.taito_tc0190 {
            taito_tc0190.chr_banks[0]
        } else if let Some(ref taito_x1005) = self.taito_x1005 {
            taito_x1005.chr_banks[0]
        } else if let Some(ref taito_x1017) = self.taito_x1017 {
            taito_x1017.chr_banks[0]
        } else {
            self.chr_bank
        }
    }

    pub fn set_prg_bank(&mut self, bank: u8) {
        self.prg_bank = bank;
        if let Some(ref mut mmc1) = self.mmc1 {
            mmc1.prg_bank = bank & 0x0F;
        }
        if let Some(ref mut vrc1) = self.vrc1 {
            vrc1.prg_banks[0] = bank & 0x0F;
        }
        if let Some(ref mut sunsoft4) = self.sunsoft4 {
            sunsoft4.prg_bank = bank & 0x0F;
        }
        if let Some(ref mut taito_tc0190) = self.taito_tc0190 {
            taito_tc0190.prg_banks[0] = bank;
        }
        if let Some(ref mut taito_x1005) = self.taito_x1005 {
            taito_x1005.prg_banks[0] = bank;
        }
        if let Some(ref mut taito_x1017) = self.taito_x1017 {
            taito_x1017.prg_banks[0] = bank;
        }
    }

    pub fn set_chr_bank(&mut self, bank: u8) {
        self.chr_bank = bank;
        if let Some(ref mut mmc1) = self.mmc1 {
            mmc1.chr_bank_0 = bank;
            mmc1.chr_bank_1 = bank;
        }
        if let Some(ref mut vrc1) = self.vrc1 {
            vrc1.chr_bank_0 = bank & 0x1F;
        }
        if let Some(ref mut sunsoft4) = self.sunsoft4 {
            sunsoft4.chr_banks[0] = bank;
        }
        if let Some(ref mut taito_tc0190) = self.taito_tc0190 {
            taito_tc0190.chr_banks[0] = bank;
        }
        if let Some(ref mut taito_x1005) = self.taito_x1005 {
            taito_x1005.chr_banks[0] = bank;
        }
        if let Some(ref mut taito_x1017) = self.taito_x1017 {
            taito_x1017.chr_banks[0] = bank;
        }
    }

    pub fn read_nametable_byte(
        &self,
        physical_nt: usize,
        offset: usize,
        internal: &[[u8; 1024]; 2],
    ) -> u8 {
        if offset >= 1024 {
            return 0;
        }

        if let Some(sunsoft4) = self.sunsoft4.as_ref() {
            if sunsoft4.nametable_chr_rom {
                return self.read_sunsoft4_nametable_chr(physical_nt & 1, offset);
            }
        }

        internal[physical_nt & 1][offset]
    }

    pub fn resolve_nametable(&self, logical_nt: usize) -> Option<usize> {
        if self.mapper == 118 {
            if let Some(mmc3) = self.mmc3.as_ref() {
                let chr_a12_invert = (mmc3.bank_select >> 7) & 1;
                let physical_nt = if chr_a12_invert == 0 {
                    match logical_nt & 3 {
                        0 | 1 => (mmc3.bank_registers[0] >> 7) as usize,
                        2 | 3 => (mmc3.bank_registers[1] >> 7) as usize,
                        _ => 0,
                    }
                } else {
                    match logical_nt & 3 {
                        0 => (mmc3.bank_registers[2] >> 7) as usize,
                        1 => (mmc3.bank_registers[3] >> 7) as usize,
                        2 => (mmc3.bank_registers[4] >> 7) as usize,
                        3 => (mmc3.bank_registers[5] >> 7) as usize,
                        _ => 0,
                    }
                };
                return Some(physical_nt & 1);
            }
        }

        if self.mapper == 207 {
            if let Some(taito) = self.taito_x1005.as_ref() {
                let physical_nt = match logical_nt & 3 {
                    0 | 1 => (taito.chr_banks[0] >> 7) as usize,
                    2 | 3 => (taito.chr_banks[1] >> 7) as usize,
                    _ => 0,
                };
                return Some(physical_nt & 1);
            }
        }

        None
    }

    pub fn nametable_writes_to_internal_vram(&self) -> bool {
        self.sunsoft4
            .as_ref()
            .map(|sunsoft4| !sunsoft4.nametable_chr_rom)
            .unwrap_or(true)
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
}

#[cfg(test)]
mod tests;
