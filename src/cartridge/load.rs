use super::{
    BandaiFcg, Cartridge, Fme7, IremG101, IremH3001, JalecoSs88006, Mapper15, Mapper246, Mapper40,
    Mapper42, Mapper43, Mapper50, Mirroring, Mmc1, Mmc2, Mmc3, Mmc5, Namco163, Namco210, Sunsoft3,
    Sunsoft4, TaitoTc0190, TaitoX1005, TaitoX1017, Vrc1, Vrc2Vrc4, Vrc3, Vrc6,
};
use std::cell::Cell;
use std::fs::File;
use std::io::{Read, Result};

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
        let mapper = (flags7 & 0xF0) | (flags6 >> 4);
        let mapper34_nina001 = mapper == 34 && chr_rom_size > 8192;
        let mapper93_chr_ram_enabled = true;
        let mapper78_hv_mirroring = mapper == 78 && (flags6 & 0x08) != 0;
        let mapper236_chr_ram = mapper == 236 && chr_rom_size == 0;

        let mirroring = if matches!(mapper, 77 | 99) {
            Mirroring::FourScreen
        } else if matches!(mapper, 13 | 38 | 208 | 234) {
            Mirroring::Vertical
        } else if mapper == 235 {
            Mirroring::Horizontal
        } else if mapper == 78 {
            if mapper78_hv_mirroring {
                Mirroring::Horizontal
            } else {
                Mirroring::OneScreenLower
            }
        } else if flags6 & 0x08 != 0 {
            Mirroring::FourScreen
        } else if flags6 & 0x01 != 0 {
            Mirroring::Vertical
        } else {
            Mirroring::Horizontal
        };

        let prg_rom_start = 16;
        let chr_rom_start = prg_rom_start + prg_rom_size;

        let prg_rom = data[prg_rom_start..prg_rom_start + prg_rom_size].to_vec();
        let chr_rom = if mapper == 13 {
            if chr_rom_size > 0 {
                let mut chr = data[chr_rom_start..chr_rom_start + chr_rom_size].to_vec();
                if chr.len() < 0x4000 {
                    chr.resize(0x4000, 0);
                }
                chr
            } else {
                vec![0; 0x4000]
            }
        } else if mapper == 19 && chr_rom_size == 0 {
            vec![0; 0x2000]
        } else if mapper == 63 {
            vec![]
        } else if mapper == 77 {
            if chr_rom_size > 0 {
                data[chr_rom_start..chr_rom_start + chr_rom_size].to_vec()
            } else {
                vec![0; 0x0800]
            }
        } else if mapper == 153 || (matches!(mapper, 18 | 221 | 231) && chr_rom_size == 0) {
            vec![]
        } else if chr_rom_size > 0 {
            data[chr_rom_start..chr_rom_start + chr_rom_size].to_vec()
        } else {
            vec![0; 8192]
        };

        let mmc1 = if mapper == 1 { Some(Mmc1::new()) } else { None };
        let mmc2 = if mapper == 9 || mapper == 10 {
            Some(Mmc2::new())
        } else {
            None
        };
        let mmc3 = if matches!(
            mapper,
            4 | 12
                | 37
                | 44
                | 47
                | 64
                | 74
                | 76
                | 88
                | 95
                | 112
                | 114
                | 115
                | 118
                | 119
                | 123
                | 154
                | 182
                | 189
                | 191
                | 192
                | 194
                | 195
                | 205
                | 206
                | 208
                | 245
                | 248
                | 250
        ) {
            Some(Mmc3::new())
        } else {
            None
        };
        let mmc5 = if mapper == 5 { Some(Mmc5::new()) } else { None };
        let namco163 = if mapper == 19 {
            Some(Namco163::new())
        } else {
            None
        };
        let namco210 = if mapper == 210 {
            Some(Namco210::new(!has_battery))
        } else {
            None
        };
        let jaleco_ss88006 = if mapper == 18 {
            Some(JalecoSs88006::new())
        } else {
            None
        };
        let vrc2_vrc4 = if matches!(mapper, 21 | 22 | 23 | 25) {
            let mut vrc = Vrc2Vrc4::new();
            if mapper == 21 {
                vrc.vrc4_mode = true;
            }
            Some(vrc)
        } else {
            None
        };
        let fme7 = if mapper == 69 {
            Some(Fme7::new())
        } else {
            None
        };
        let mapper40 = if mapper == 40 {
            Some(Mapper40::new())
        } else {
            None
        };
        let mapper42 = if mapper == 42 {
            Some(Mapper42::new())
        } else {
            None
        };
        let mapper43 = if mapper == 43 {
            Some(Mapper43::new())
        } else {
            None
        };
        let mapper50 = if mapper == 50 {
            Some(Mapper50::new())
        } else {
            None
        };
        let bandai_fcg = if matches!(mapper, 16 | 153 | 159) {
            Some(BandaiFcg::new())
        } else {
            None
        };
        let irem_g101 = if mapper == 32 {
            Some(IremG101::new())
        } else {
            None
        };
        let irem_h3001 = if mapper == 65 {
            Some(IremH3001::new())
        } else {
            None
        };
        let vrc1 = if matches!(mapper, 75 | 151) {
            Some(Vrc1::new())
        } else {
            None
        };
        let vrc3 = if matches!(mapper, 73 | 142) {
            Some(Vrc3::new())
        } else {
            None
        };
        let vrc6 = if matches!(mapper, 24 | 26) {
            Some(Vrc6::new())
        } else {
            None
        };
        let mapper15 = if mapper == 15 {
            Some(Mapper15::new())
        } else {
            None
        };
        let taito_tc0190 = if matches!(mapper, 33 | 48) {
            Some(TaitoTc0190::new())
        } else {
            None
        };
        let taito_x1005 = if matches!(mapper, 80 | 207) {
            Some(TaitoX1005::new())
        } else {
            None
        };
        let taito_x1017 = if mapper == 82 {
            Some(TaitoX1017::new())
        } else {
            None
        };
        let mapper246 = if mapper == 246 {
            Some(Mapper246::new())
        } else {
            None
        };
        let sunsoft4 = if mapper == 68 {
            Some(Sunsoft4::new())
        } else {
            None
        };
        let sunsoft3 = if mapper == 67 {
            Some(Sunsoft3::new())
        } else {
            None
        };

        let prg_ram = if mapper == 16 && has_battery {
            vec![0xFF; 256]
        } else if mapper == 159 && has_battery {
            vec![0xFF; 128]
        } else if mapper == 99 {
            vec![0x00; 0x0800]
        } else if mapper == 5 {
            vec![0x00; 0x20000]
        } else if mapper == 19 {
            vec![0x00; 0x2080]
        } else if mapper == 210 && has_battery {
            vec![0x00; 0x0800]
        } else if mapper == 68
            || mapper == 24
            || mapper == 26
            || mapper == 21
            || mapper == 23
            || mapper == 25
            || mapper == 18
            || mapper == 240
            || mapper == 241
            || mapper == 245
            || (mapper == 227 && has_battery)
        {
            vec![0x00; 8192]
        } else if mapper == 80 || mapper == 207 {
            vec![0x00; 128]
        } else if mapper == 82 {
            vec![0x00; 0x1400]
        } else if mapper == 246 {
            vec![0x00; 2048]
        } else if mapper == 225 {
            vec![0x00; 4]
        } else if mapper == 103 {
            vec![0x00; 0x2000]
        } else if mapper == 153 {
            vec![0x00; 0x8000]
        } else if mapper == 1
            || mapper == 32
            || mapper == 4
            || mapper == 73
            || mapper == 74
            || mapper == 118
            || mapper == 119
            || mapper == 192
            || mapper == 194
            || mapper == 15
            || mapper34_nina001
            || mapper == 9
            || mapper == 10
            || mapper == 16
            || mapper == 69
        {
            vec![0x00; 8192]
        } else {
            Vec::new()
        };

        let chr_ram = if mapper == 19 {
            vec![0x00; 0x0800]
        } else if (mapper == 210 && chr_rom_size == 0) || matches!(mapper, 63 | 77 | 153) {
            vec![0x00; 0x2000]
        } else if mapper == 99 {
            vec![0x00; 0x1000]
        } else if matches!(mapper, 21..=26) && chr_rom_size == 0 {
            vec![0x00; 0x2000]
        } else if matches!(mapper, 74 | 191 | 194) {
            vec![0x00; 0x0800]
        } else if mapper == 192 {
            vec![0x00; 0x1000]
        } else if mapper == 12
            || mapper == 44
            || mapper == 119
            || mapper == 115
            || mapper == 189
            || mapper == 205
            || mapper == 248
            || mapper == 195
            || (matches!(
                mapper,
                1 | 4 | 10 | 18 | 48 | 65 | 118 | 221 | 231 | 236 | 245
            ) && chr_rom_size == 0)
        {
            vec![0x00; 8192]
        } else {
            vec![]
        };

        let chr_bank_1 = if mapper == 184 { 4 } else { 1 };

        let mut cart = Cartridge {
            prg_rom,
            chr_rom,
            chr_ram,
            prg_ram,
            has_valid_save_data: false,
            mapper,
            mirroring,
            has_battery,
            chr_bank: 0,
            chr_bank_1,
            prg_bank: if mapper == 208 { 3 } else { 0 },
            mapper34_nina001,
            mapper93_chr_ram_enabled,
            mapper78_hv_mirroring,
            mapper58_nrom128: false,
            mapper59_latch: 0,
            mapper59_locked: false,
            mapper60_game_select: 0,
            mapper61_latch: 0,
            mapper63_latch: 0,
            mapper142_bank_select: 0,
            mapper142_prg_banks: [0; 4],
            mapper137_index: 0,
            mapper137_registers: [0; 8],
            mapper150_index: 0,
            mapper150_registers: [0; 8],
            mapper225_nrom128: false,
            mapper232_outer_bank: 0,
            mapper41_inner_bank: 0,
            mapper233_nrom128: false,
            mapper234_reg0: 0,
            mapper234_reg1: 0,
            mapper235_nrom128: false,
            mapper202_32k_mode: false,
            mapper37_outer_bank: 0,
            mapper44_outer_bank: 0,
            mapper103_prg_ram_disabled: false,
            mapper212_32k_mode: false,
            mapper47_outer_bank: 0,
            mapper12_chr_outer: 0,
            mapper114_override: 0,
            mapper114_chr_outer_bank: 0,
            mapper115_override: 0,
            mapper115_chr_outer_bank: 0,
            mapper123_override: 0,
            mapper205_block: 0,
            mapper226_nrom128: false,
            mapper230_contra_mode: mapper == 230,
            mapper230_nrom128: false,
            mapper228_chip_select: 0,
            mapper228_nrom128: false,
            mapper242_latch: 0,
            mapper243_index: 0,
            mapper243_registers: [0; 8],
            mapper221_mode: 0,
            mapper221_outer_bank: 0,
            mapper221_chr_write_protect: false,
            mapper191_outer_bank: if mapper == 191 && chr_rom_size <= 0x20000 {
                3
            } else {
                0
            },
            mapper195_mode: 0x80,
            mapper208_protection_index: 0,
            mapper208_protection_regs: [0; 4],
            mapper189_prg_bank: 0,
            mapper185_disabled_reads: Cell::new(if mapper == 185 { 2 } else { 0 }),
            mmc1,
            mmc2,
            mmc3,
            mmc5,
            namco163,
            namco210,
            jaleco_ss88006,
            vrc2_vrc4,
            mapper40,
            mapper42,
            mapper43,
            mapper50,
            fme7,
            bandai_fcg,
            irem_g101,
            irem_h3001,
            vrc1,
            vrc3,
            vrc6,
            mapper15,
            sunsoft3,
            sunsoft4,
            taito_tc0190,
            taito_x1005,
            taito_x1017,
            mapper227_latch: 0,
            mapper246,
            mapper236_mode: 0,
            mapper236_outer_bank: 0,
            mapper236_chr_ram,
        };
        if let Some(ref mut bandai) = cart.bandai_fcg {
            bandai.configure_mapper(mapper, has_battery);
        }
        Ok(cart)
    }
}
