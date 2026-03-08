mod load;
mod mapper;
mod state;

use mapper::{
    BandaiFcg, Fme7, IremG101, IremH3001, JalecoSs88006, Mapper15, Mapper246, Mapper40, Mapper42,
    Mapper43, Mapper50, Mmc1, Mmc2, Mmc3, Mmc5, Namco163, Namco210, Sunsoft3, Sunsoft4,
    TaitoTc0190, TaitoX1005, TaitoX1017, Vrc1, Vrc2Vrc4, Vrc3, Vrc6,
};
use serde::{Deserialize, Serialize};
pub use state::*;
use std::cell::Cell;

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
    mapper59_latch: u16,
    mapper59_locked: bool,
    mapper60_game_select: u8,
    mapper61_latch: u16,
    mapper63_latch: u16,
    mapper142_bank_select: u8,
    mapper142_prg_banks: [u8; 4],
    mapper137_index: u8,
    mapper137_registers: [u8; 8],
    mapper150_index: u8,
    mapper150_registers: [u8; 8],
    mapper225_nrom128: bool,
    mapper232_outer_bank: u8,
    mapper41_inner_bank: u8,
    mapper233_nrom128: bool,
    mapper234_reg0: u8,
    mapper234_reg1: u8,
    mapper235_nrom128: bool,
    mapper202_32k_mode: bool,
    mapper37_outer_bank: u8,
    mapper44_outer_bank: u8,
    mapper103_prg_ram_disabled: bool,
    mapper212_32k_mode: bool,
    mapper47_outer_bank: u8,
    mapper12_chr_outer: u8,
    mapper114_override: u8,
    mapper114_chr_outer_bank: u8,
    mapper115_override: u8,
    mapper115_chr_outer_bank: u8,
    mapper123_override: u8,
    mapper205_block: u8,
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
    mapper189_prg_bank: u8,
    mapper185_disabled_reads: Cell<u8>,
    mmc1: Option<Mmc1>,
    mmc2: Option<Mmc2>,
    mmc3: Option<Mmc3>,
    mmc5: Option<Mmc5>,
    namco163: Option<Namco163>,
    namco210: Option<Namco210>,
    jaleco_ss88006: Option<JalecoSs88006>,
    vrc2_vrc4: Option<Vrc2Vrc4>,
    mapper40: Option<Mapper40>,
    mapper42: Option<Mapper42>,
    mapper43: Option<Mapper43>,
    mapper50: Option<Mapper50>,
    fme7: Option<Fme7>,
    bandai_fcg: Option<BandaiFcg>,
    irem_g101: Option<IremG101>,
    irem_h3001: Option<IremH3001>,
    vrc1: Option<Vrc1>,
    vrc3: Option<Vrc3>,
    vrc6: Option<Vrc6>,
    mapper15: Option<Mapper15>,
    sunsoft3: Option<Sunsoft3>,
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
            210 => self.read_prg_mapper210(addr),
            21 => self.read_prg_mapper21(addr),
            22 => self.read_prg_mapper22(addr),
            23 => self.read_prg_mapper23(addr),
            24 | 26 => self.read_prg_vrc6(addr),
            25 => self.read_prg_mapper25(addr),
            18 => self.read_prg_mapper18(addr),
            19 => self.read_prg_namco163(addr),
            5 => self.read_prg_mmc5(addr),
            64 => self.read_prg_mapper64(addr),
            0 | 3 | 13 | 87 | 101 | 184 | 185 => self.read_prg_nrom(rom_addr),
            59 => self.read_prg_mapper59(addr),
            60 => self.read_prg_mapper60(addr),
            61 => self.read_prg_mapper61(addr),
            63 => self.read_prg_mapper63(addr),
            99 => self.read_prg_mapper99(addr),
            142 => self.read_prg_mapper142(addr),
            150 => self.read_prg_axrom(addr),
            12 => self.read_prg_mmc3(addr),
            37 => self.read_prg_mapper37(addr),
            44 => self.read_prg_mapper44(addr),
            32 => self.read_prg_mapper32(addr),
            40 => self.read_prg_mapper40(addr),
            42 => self.read_prg_mapper42(addr),
            43 => self.read_prg_mapper43(addr),
            47 => self.read_prg_mapper47(addr),
            48 => self.read_prg_taito_tc0190(addr),
            112 => self.read_prg_namco108(addr),
            114 | 182 => self.read_prg_mapper114(addr),
            115 | 248 => self.read_prg_mapper115(addr),
            123 => self.read_prg_mapper123(addr),
            205 => self.read_prg_mapper205(addr),
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
            2 | 57 | 70 | 71 | 72 | 73 | 81 | 86 | 89 | 93 | 152 => {
                self.read_prg_uxrom(addr, rom_addr)
            }
            34 | 38 | 41 | 46 | 77 | 79 | 113 | 133 | 140 | 144 | 146 | 147 | 148 | 201 | 240
            | 241 | 243 => self.read_prg_axrom(addr),
            50 => self.read_prg_mapper50(addr),
            58 | 213 => self.read_prg_mapper58(addr),
            65 => self.read_prg_mapper65(addr),
            67 => self.read_prg_sunsoft3(addr),
            229 => self.read_prg_mapper229(addr),
            92 | 180 => self.read_prg_uxrom_inverted(addr, rom_addr),
            97 => self.read_prg_fixed_last_switch_high(addr, rom_addr),
            103 => self.read_prg_mapper103(addr),
            137 => self.read_prg_axrom(addr),
            191 => self.read_prg_mapper191(addr),
            4 | 74 | 118 | 119 | 192 | 194 | 195 | 250 => self.read_prg_mmc3(addr),
            189 => self.read_prg_axrom(addr),
            245 => self.read_prg_mapper245(addr),
            68 => self.read_prg_sunsoft4(addr),
            75 | 151 => self.read_prg_vrc1(addr),
            80 | 207 => self.read_prg_taito_x1005(addr),
            82 => self.read_prg_taito_x1017(addr),
            76 | 88 | 95 | 154 | 206 => self.read_prg_namco108(addr),
            7 | 11 | 66 | 107 => self.read_prg_axrom(addr),
            78 | 94 => self.read_prg_uxrom(addr, rom_addr),
            9 | 10 => self.read_prg_mmc2(addr, rom_addr),
            16 | 153 | 159 => self.read_prg_bandai(addr),
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
            210 => self.write_prg_mapper210(addr, data),
            21 => self.write_prg_mapper21(addr, data),
            22 => self.write_prg_mapper22(addr, data),
            23 => self.write_prg_mapper23(addr, data),
            24 | 26 => self.write_prg_vrc6(addr, data),
            25 => self.write_prg_mapper25(addr, data),
            18 => self.write_prg_mapper18(addr, data),
            19 => self.write_prg_namco163(addr, data),
            5 => self.write_prg_mmc5(addr, data),
            64 => self.write_prg_mapper64(addr, data),
            32 => self.write_prg_mapper32(addr, data),
            40 => self.write_prg_mapper40(addr, data),
            42 => self.write_prg_mapper42(addr, data),
            43 => self.write_prg_mapper43(addr, data),
            1 => self.write_prg_mmc1(addr, data),
            50 => self.write_prg_mapper50(addr, data),
            73 => self.write_prg_vrc3(addr, data),
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
            41 => self.write_prg_mapper41(addr, data),
            232 => self.write_prg_mapper232(addr, data),
            233 => self.write_prg_mapper233(addr, data),
            234 => self.write_prg_mapper234(addr, data),
            235 => self.write_prg_mapper235(addr, data),
            3 | 185 => self.write_prg_cnrom(addr, data),
            13 => self.write_prg_cprom(addr, data),
            34 if self.mapper34_nina001 => {}
            34 => self.write_prg_bnrom(addr, data),
            46 => self.write_prg_mapper46_inner(addr, data),
            191 => self.write_prg_mapper191(addr, data),
            4 | 37 | 47 | 74 | 115 | 118 | 119 | 192 | 194 | 195 | 205 | 245 | 248 => {
                self.write_prg_mmc3(addr, data)
            }
            12 => self.write_prg_mapper12(addr, data),
            44 => self.write_prg_mapper44(addr, data),
            48 => self.write_prg_mapper48(addr, data),
            114 | 182 => self.write_prg_mapper114(addr, data),
            123 => self.write_prg_mapper123(addr, data),
            250 => self.write_prg_mapper250(addr, data),
            57 => self.write_prg_mapper57(addr, data),
            59 => self.write_prg_mapper59(addr),
            60 => {}
            61 => self.write_prg_mapper61(addr),
            63 => self.write_prg_mapper63(addr),
            137 => self.write_prg_mapper137(addr, data),
            142 => self.write_prg_mapper142(addr, data),
            150 => self.write_prg_mapper150(addr, data),
            58 | 213 => self.write_prg_mapper58(addr),
            65 => self.write_prg_mapper65(addr, data),
            67 => self.write_prg_sunsoft3(addr, data),
            72 | 92 => self.write_prg_mapper72_92(addr, data),
            68 => self.write_prg_sunsoft4(addr, data),
            75 | 151 => self.write_prg_vrc1(addr, data),
            76 | 88 | 95 | 154 | 206 | 112 => self.write_prg_namco108(addr, data),
            7 => self.write_prg_axrom(addr, data),
            77 => self.write_prg_mapper77(addr, data),
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
            189 => self.write_prg_mapper189(addr, data),
            89 => self.write_prg_mapper89(addr, data),
            93 => self.write_prg_mapper93(addr, data),
            70 => self.write_prg_mapper70(addr, data),
            71 => self.write_prg_camerica(addr, data),
            81 => self.write_prg_mapper81(addr),
            97 => self.write_prg_mapper97(addr, data),
            103 => self.write_prg_mapper103(addr, data),
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
            16 | 153 | 159 => self.write_prg_bandai(addr, data),
            66 => self.write_prg_gxrom(addr, data),
            69 => self.write_prg_fme7(addr, data),
            87 => self.write_prg_mapper87(addr, data),
            _ => {}
        }
    }

    #[inline]
    pub fn read_chr(&self, addr: u16) -> u8 {
        match self.mapper {
            210 => self.read_chr_mapper210(addr),
            21 => self.read_chr_mapper21(addr),
            22 => self.read_chr_mapper22(addr),
            23 => self.read_chr_mapper23(addr),
            24 | 26 => self.read_chr_vrc6(addr),
            25 => self.read_chr_mapper25(addr),
            18 => self.read_chr_mapper18(addr),
            19 => self.read_chr_namco163(addr),
            5 => self.read_chr_mmc5(addr),
            0 | 43 | 97 | 103 | 226 | 241 | 242 => self.read_chr_nrom(addr),
            1 => self.read_chr_mmc1(addr),
            2 | 7 | 15 | 40 | 42 | 50 | 71 | 73 | 94 | 180 | 227 | 230 | 232 | 235 => {
                self.read_chr_uxrom(addr)
            }
            32 => self.read_chr_mapper32(addr),
            65 => self.read_chr_mapper65(addr),
            221 => self.read_chr_mapper221(addr),
            231 => self.read_chr_mapper231(addr),
            236 => self.read_chr_mapper236(addr),
            75 | 151 => self.read_chr_vrc1(addr),
            13 => self.read_chr_cprom(addr),
            34 if self.mapper34_nina001 => self.read_chr_nina001(addr),
            184 => self.read_chr_split_4k(addr),
            33 => self.read_chr_taito_tc0190(addr),
            34 => self.read_chr_nrom(addr),
            3 | 11 | 38 | 41 | 46 | 57 | 58 | 59 | 60 | 61 | 66 | 70 | 72 | 78 | 79 | 81 | 86
            | 89 | 92 | 101 | 107 | 113 | 133 | 140 | 144 | 145 | 146 | 147 | 148 | 152 | 200
            | 201 | 202 | 203 | 212 | 213 | 225 | 228 | 229 | 233 | 234 | 240 | 243 | 255 | 87 => {
                self.read_chr_cnrom(addr)
            }
            63 => self.read_chr_mapper63(addr),
            64 => self.read_chr_mapper64(addr),
            99 => self.read_chr_mapper99(addr),
            137 => self.read_chr_mapper137(addr),
            142 => self.read_chr_nrom(addr),
            150 => self.read_chr_cnrom(addr),
            77 => self.read_chr_mapper77(addr),
            67 => self.read_chr_sunsoft3(addr),
            185 => self.read_chr_mapper185(addr),
            189 => self.read_chr_mmc3(addr),
            93 => self.read_chr_mapper93(addr),
            4 | 118 | 208 | 250 => self.read_chr_mmc3(addr),
            12 => self.read_chr_mapper12(addr),
            37 => self.read_chr_mapper37(addr),
            44 => self.read_chr_mapper44(addr),
            47 => self.read_chr_mapper47(addr),
            48 => self.read_chr_taito_tc0190(addr),
            114 | 182 => self.read_chr_mapper114(addr),
            115 | 248 => self.read_chr_mapper115(addr),
            123 => self.read_chr_mmc3(addr),
            205 => self.read_chr_mapper205(addr),
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
            76 | 88 | 95 | 154 | 206 | 112 => self.read_chr_namco108(addr),
            9 | 10 => self.read_chr_mmc2(addr),
            16 | 153 | 159 => self.read_chr_bandai(addr),
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
        if self.mapper == 5 {
            self.read_chr_sprite_mmc5(addr, _sprite_y)
        } else {
            self.read_chr(addr)
        }
    }

    pub fn write_chr(&mut self, addr: u16, data: u8) {
        match self.mapper {
            210 => self.write_chr_mapper210(addr, data),
            21 => self.write_chr_mapper21(addr, data),
            22 => self.write_chr_mapper22(addr, data),
            23 => self.write_chr_mapper23(addr, data),
            24 | 26 => self.write_chr_vrc6(addr, data),
            25 => self.write_chr_mapper25(addr, data),
            18 => self.write_chr_mapper18(addr, data),
            19 => self.write_chr_namco163(addr, data),
            5 => self.write_chr_mmc5(addr, data),
            0 | 43 | 97 | 103 | 226 | 241 => self.write_chr_nrom(addr, data),
            1 => self.write_chr_mmc1(addr, data),
            2 | 7 | 15 | 40 | 42 | 50 | 71 | 73 | 94 | 180 | 230 | 232 | 235 => {
                self.write_chr_uxrom(addr, data)
            }
            32 => self.write_chr_mapper32(addr, data),
            65 => self.write_chr_mapper65(addr, data),
            221 => self.write_chr_mapper221(addr, data),
            227 => self.write_chr_mapper227(addr, data),
            231 => self.write_chr_mapper231(addr, data),
            236 => self.write_chr_mapper236(addr, data),
            75 | 151 => self.write_chr_vrc1(addr, data),
            13 => self.write_chr_cprom(addr, data),
            34 if self.mapper34_nina001 => self.write_chr_nina001(addr, data),
            184 => self.write_chr_split_4k(addr, data),
            33 => self.write_chr_taito_tc0190(addr, data),
            34 => self.write_chr_nrom(addr, data),
            3 | 11 | 38 | 41 | 46 | 57 | 58 | 59 | 60 | 61 | 66 | 70 | 72 | 78 | 79 | 81 | 86
            | 89 | 92 | 101 | 107 | 113 | 133 | 140 | 144 | 145 | 146 | 147 | 148 | 152 | 200
            | 201 | 202 | 203 | 212 | 213 | 225 | 228 | 229 | 233 | 234 | 240 | 243 | 255 | 87 => {
                self.write_chr_cnrom(addr, data)
            }
            63 => self.write_chr_mapper63(addr, data),
            64 => self.write_chr_mapper64(addr, data),
            99 => {}
            137 => self.write_chr_mapper137(addr, data),
            142 => self.write_chr_nrom(addr, data),
            150 => self.write_chr_cnrom(addr, data),
            77 => self.write_chr_mapper77(addr, data),
            67 => self.write_chr_sunsoft3(addr, data),
            185 => self.write_chr_mapper185(addr, data),
            189 => self.write_chr_mmc3(addr, data),
            242 => self.write_chr_mapper242(addr, data),
            93 => self.write_chr_mapper93(addr, data),
            4 | 118 | 208 | 250 => self.write_chr_mmc3(addr, data),
            12 => self.write_chr_mapper12(addr, data),
            37 => self.write_chr_mapper37(addr, data),
            44 => self.write_chr_mapper44(addr, data),
            47 => self.write_chr_mapper47(addr, data),
            48 => self.write_chr_taito_tc0190(addr, data),
            114 | 182 => self.write_chr_mapper114(addr, data),
            115 | 248 => self.write_chr_mapper115(addr, data),
            123 => self.write_chr_mmc3(addr, data),
            205 => self.write_chr_mapper205(addr, data),
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
            76 | 88 | 95 | 154 | 206 | 112 => self.write_chr_namco108(addr, data),
            9 | 10 => self.write_chr_mmc2(addr, data),
            16 | 153 | 159 => self.write_chr_bandai(addr, data),
            69 => self.write_chr_fme7(addr, data),
            _ => {
                self.chr_rom[(addr & 0x1FFF) as usize] = data;
            }
        }
    }

    pub fn read_prg_ram(&self, addr: u16) -> u8 {
        match self.mapper {
            210 => self.read_prg_ram_mapper210(addr),
            21 => self.read_prg_ram_mapper21(addr),
            22 => self.read_prg_ram_mapper22(addr),
            23 => self.read_prg_ram_mapper23(addr),
            24 | 26 => self.read_prg_ram_vrc6(addr),
            25 => self.read_prg_ram_mapper25(addr),
            18 => self.read_prg_ram_mapper18(addr),
            19 => self.read_prg_ram_namco163(addr),
            5 => self.read_prg_ram_mmc5(addr),
            1 => self.read_prg_ram_mmc1(addr),
            15 => self.read_prg_ram_mapper15(addr),
            40 => self.read_prg_ram_mapper40(addr),
            42 => self.read_prg_ram_mapper42(addr),
            43 => self.read_prg_ram_mapper43(addr),
            50 => self.read_prg_ram_mapper50(addr),
            73 => self.read_prg_ram_vrc3(addr),
            99 => self.read_prg_ram_mapper99(addr),
            142 => self.read_prg_ram_mapper142(addr),
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
            115 | 248 => self.read_prg_ram_mapper115(addr),
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
            16 | 153 | 159 => self.read_prg_ram_bandai(addr),
            103 => self.read_prg_ram_mapper103(addr),
            69 => self.read_prg_ram_fme7(addr),
            _ => 0,
        }
    }

    pub fn read_prg_low(&self, addr: u16) -> u8 {
        match self.mapper {
            19 => self.read_prg_low_namco163(addr),
            5 => self.read_prg_low_mmc5(addr),
            43 => self.read_prg_low_mapper43(addr),
            137 => self.read_prg_low_mapper137(addr),
            150 => self.read_prg_low_mapper150(addr),
            208 => self.read_prg_low_mapper208(addr),
            225 => self.read_prg_low_mapper225(addr),
            243 => self.read_prg_low_mapper243(addr),
            _ => 0,
        }
    }

    pub fn write_prg_low(&mut self, addr: u16, data: u8) {
        if self.mapper == 99 {
            self.write_prg_low_mapper99(addr, data);
        } else if self.mapper == 5 {
            self.write_prg_mmc5(addr, data);
        } else if self.mapper == 19 {
            self.write_prg_low_namco163(addr, data);
        }
    }

    pub fn write_prg_ram(&mut self, addr: u16, data: u8) {
        match self.mapper {
            210 => self.write_prg_ram_mapper210(addr, data),
            21 => self.write_prg_ram_mapper21(addr, data),
            22 => self.write_prg_ram_mapper22(addr, data),
            23 => self.write_prg_ram_mapper23(addr, data),
            24 | 26 => self.write_prg_ram_vrc6(addr, data),
            25 => self.write_prg_ram_mapper25(addr, data),
            18 => self.write_prg_ram_mapper18(addr, data),
            19 => self.write_prg_ram_namco163(addr, data),
            5 => self.write_prg_ram_mmc5(addr, data),
            46 => self.write_prg_mapper46_outer(addr, data),
            41 => self.write_prg_ram_mapper41(addr),
            38 => self.write_prg_mapper38(addr, data),
            87 => self.write_prg_mapper87(addr, data),
            86 => self.write_prg_mapper86(addr, data),
            101 => self.write_prg_mapper101(addr, data),
            15 => self.write_prg_ram_mapper15(addr, data),
            73 => self.write_prg_ram_vrc3(addr, data),
            99 => self.write_prg_ram_mapper99(addr, data),
            208 => self.write_prg_ram_mapper208(addr, data),
            37 => self.write_prg_ram_mapper37(addr, data),
            47 => self.write_prg_ram_mapper47(addr, data),
            114 | 182 => self.write_prg_ram_mapper114(addr, data),
            115 | 248 => self.write_prg_ram_mapper115(addr, data),
            205 => self.write_prg_ram_mapper205(addr, data),
            80 | 207 => self.write_prg_ram_taito_x1005(addr, data),
            82 => self.write_prg_ram_taito_x1017(addr, data),
            140 => self.write_prg_mapper140(addr, data),
            189 => self.write_prg_ram_mapper189(data),
            1 => self.write_prg_ram_mmc1(addr, data),
            103 => self.write_prg_ram_mapper103(addr, data),
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
            16 | 153 | 159 => self.write_prg_ram_bandai(addr, data),
            69 => self.write_prg_ram_fme7(addr, data),
            _ => {}
        }
    }

    pub fn mirroring(&self) -> Mirroring {
        self.mirroring
    }

    pub fn on_reset(&mut self) {
        if self.mapper == 41 {
            self.prg_bank = 0;
            self.chr_bank = 0;
            self.mapper41_inner_bank = 0;
            self.mirroring = Mirroring::Vertical;
        }
        if self.mapper == 59 {
            self.mapper59_locked = false;
        }
        if self.mapper == 63 {
            self.mapper63_latch = 0;
            self.mirroring = Mirroring::Vertical;
        }
        if self.mapper == 60 {
            self.advance_mapper60_game();
        }
        if self.mapper == 185 {
            self.mapper185_disabled_reads.set(2);
        }
        if self.mapper == 230 {
            self.mapper230_contra_mode = !self.mapper230_contra_mode;
            if self.mapper230_contra_mode {
                self.mirroring = Mirroring::Vertical;
            }
        }
        if self.mapper == 5 {
            self.mmc5_end_frame();
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
        } else if let Some(ref g101) = self.irem_g101 {
            g101.prg_banks[0]
        } else if let Some(ref h3001) = self.irem_h3001 {
            h3001.prg_banks[0]
        } else if let Some(ref mapper210) = self.namco210 {
            mapper210.prg_banks[0]
        } else if let Some(ref vrc2_vrc4) = self.vrc2_vrc4 {
            vrc2_vrc4.prg_banks[0]
        } else if let Some(ref vrc6) = self.vrc6 {
            vrc6.prg_bank_16k
        } else if let Some(ref mapper18) = self.jaleco_ss88006 {
            mapper18.prg_banks[0]
        } else if let Some(ref vrc1) = self.vrc1 {
            vrc1.prg_banks[0]
        } else if let Some(ref sunsoft3) = self.sunsoft3 {
            sunsoft3.prg_bank
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
        } else if let Some(ref g101) = self.irem_g101 {
            g101.chr_banks[0]
        } else if let Some(ref h3001) = self.irem_h3001 {
            h3001.chr_banks[0]
        } else if let Some(ref mapper210) = self.namco210 {
            mapper210.chr_banks[0]
        } else if let Some(ref vrc2_vrc4) = self.vrc2_vrc4 {
            vrc2_vrc4.chr_banks[0] as u8
        } else if let Some(ref vrc6) = self.vrc6 {
            vrc6.chr_banks[0]
        } else if let Some(ref mapper18) = self.jaleco_ss88006 {
            mapper18.chr_banks[0]
        } else if let Some(ref vrc1) = self.vrc1 {
            vrc1.chr_bank_0
        } else if let Some(ref sunsoft3) = self.sunsoft3 {
            sunsoft3.chr_banks[0]
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
        if let Some(ref mut g101) = self.irem_g101 {
            g101.prg_banks[0] = bank;
        }
        if let Some(ref mut h3001) = self.irem_h3001 {
            h3001.prg_banks[0] = bank;
        }
        if let Some(ref mut mapper210) = self.namco210 {
            mapper210.prg_banks[0] = bank;
        }
        if let Some(ref mut vrc2_vrc4) = self.vrc2_vrc4 {
            vrc2_vrc4.prg_banks[0] = bank & 0x1F;
        }
        if let Some(ref mut vrc6) = self.vrc6 {
            vrc6.prg_bank_16k = bank & 0x0F;
        }
        if let Some(ref mut mapper18) = self.jaleco_ss88006 {
            mapper18.prg_banks[0] = bank;
        }
        if let Some(ref mut vrc1) = self.vrc1 {
            vrc1.prg_banks[0] = bank & 0x0F;
        }
        if let Some(ref mut sunsoft3) = self.sunsoft3 {
            sunsoft3.prg_bank = bank & 0x0F;
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
        if let Some(ref mut g101) = self.irem_g101 {
            g101.chr_banks[0] = bank;
        }
        if let Some(ref mut h3001) = self.irem_h3001 {
            h3001.chr_banks[0] = bank;
        }
        if let Some(ref mut mapper210) = self.namco210 {
            mapper210.chr_banks[0] = bank;
        }
        if let Some(ref mut vrc2_vrc4) = self.vrc2_vrc4 {
            vrc2_vrc4.chr_banks[0] = bank as u16;
        }
        if let Some(ref mut vrc6) = self.vrc6 {
            vrc6.chr_banks[0] = bank;
        }
        if let Some(ref mut mapper18) = self.jaleco_ss88006 {
            mapper18.chr_banks[0] = bank;
        }
        if let Some(ref mut vrc1) = self.vrc1 {
            vrc1.chr_bank_0 = bank & 0x1F;
        }
        if let Some(ref mut sunsoft3) = self.sunsoft3 {
            sunsoft3.chr_banks[0] = bank;
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

        if self.mapper == 19 {
            return self.read_nametable_namco163(physical_nt, offset, internal);
        }

        if self.mapper == 5 {
            return self.read_nametable_mmc5(physical_nt, offset, internal);
        }

        if let Some(sunsoft4) = self.sunsoft4.as_ref() {
            if sunsoft4.nametable_chr_rom {
                return self.read_sunsoft4_nametable_chr(physical_nt & 1, offset);
            }
        }

        if self.mapper == 77 {
            if physical_nt < 2 {
                let chr_addr = 0x1800 + physical_nt * 0x0400 + offset;
                return self.chr_ram.get(chr_addr).copied().unwrap_or(0);
            }
            return internal[(physical_nt - 2) & 1][offset];
        }

        if self.mapper == 99 {
            let chr_addr = ((physical_nt & 3) * 0x0400) + offset;
            return self.chr_ram.get(chr_addr).copied().unwrap_or(0);
        }

        internal[physical_nt & 1][offset]
    }

    pub fn write_nametable_byte(
        &mut self,
        physical_nt: usize,
        offset: usize,
        internal: &mut [[u8; 1024]; 2],
        data: u8,
    ) {
        if offset >= 1024 {
            return;
        }

        if self.mapper == 19 {
            self.write_nametable_namco163(physical_nt, offset, internal, data);
            return;
        }

        if self.mapper == 5 {
            self.write_nametable_mmc5(physical_nt, offset, internal, data);
            return;
        }

        if let Some(sunsoft4) = self.sunsoft4.as_ref() {
            if sunsoft4.nametable_chr_rom {
                return;
            }
        }

        if self.mapper == 77 && physical_nt < 2 {
            let chr_addr = 0x1800 + physical_nt * 0x0400 + offset;
            if let Some(slot) = self.chr_ram.get_mut(chr_addr) {
                *slot = data;
            }
            return;
        }

        if self.mapper == 99 {
            let chr_addr = ((physical_nt & 3) * 0x0400) + offset;
            if let Some(slot) = self.chr_ram.get_mut(chr_addr) {
                *slot = data;
            }
            return;
        }

        let internal_nt = if self.mapper == 77 && physical_nt >= 2 {
            (physical_nt - 2) & 1
        } else {
            physical_nt & 1
        };
        internal[internal_nt][offset] = data;
    }

    pub fn resolve_nametable(&self, logical_nt: usize) -> Option<usize> {
        if self.mapper == 19 {
            return Some(logical_nt & 3);
        }

        if self.mapper == 5 {
            return Some(self.resolve_nametable_mmc5(logical_nt));
        }

        if self.mapper == 77 {
            return Some(logical_nt & 3);
        }

        if self.mapper == 99 {
            return Some(logical_nt & 3);
        }

        if self.mapper == 137 {
            if (self.mapper137_registers[7] >> 1) & 0x03 == 0 {
                return Some(match logical_nt & 3 {
                    0 => 0,
                    _ => 1,
                });
            }
            return None;
        }

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
