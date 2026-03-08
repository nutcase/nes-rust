use super::*;
use std::cell::Cell;

fn base_cartridge(
    mapper: u8,
    prg_rom: Vec<u8>,
    chr_rom: Vec<u8>,
    chr_ram: Vec<u8>,
    prg_ram: Vec<u8>,
    mirroring: Mirroring,
) -> Cartridge {
    Cartridge {
        prg_rom,
        chr_rom,
        chr_ram,
        prg_ram,
        has_valid_save_data: false,
        mapper,
        mirroring,
        has_battery: false,
        chr_bank: 0,
        chr_bank_1: 1,
        prg_bank: 0,
        mapper34_nina001: false,
        mapper93_chr_ram_enabled: true,
        mapper78_hv_mirroring: false,
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
        mapper189_prg_bank: 0,
        mapper185_disabled_reads: Cell::new(0),
        mmc1: None,
        mmc2: None,
        mmc3: None,
        mmc5: None,
        namco163: None,
        namco210: None,
        jaleco_ss88006: None,
        vrc2_vrc4: None,
        mapper40: None,
        mapper42: None,
        mapper43: None,
        mapper50: None,
        fme7: None,
        bandai_fcg: None,
        irem_g101: None,
        irem_h3001: None,
        vrc1: None,
        vrc3: None,
        vrc6: None,
        mapper15: None,
        sunsoft3: None,
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

fn make_mmc1_cart() -> Cartridge {
    let mut cart = base_cartridge(
        1,
        vec![0; 0x8000],
        vec![0; 0x2000],
        vec![0; 0x2000],
        vec![0; 0x2000],
        Mirroring::Vertical,
    );
    cart.has_valid_save_data = true;
    cart.has_battery = true;
    cart.mmc1 = Some(Mmc1::new());
    cart
}

fn make_mmc5_cart() -> Cartridge {
    let mut prg_rom = vec![0; 32 * 0x2000];
    for bank in 0..32 {
        prg_rom[bank * 0x2000..(bank + 1) * 0x2000].fill(bank as u8);
    }

    let mut chr_rom = vec![0; 128 * 0x0400];
    for bank in 0..128 {
        chr_rom[bank * 0x0400..(bank + 1) * 0x0400].fill(0x80 | (bank as u8));
    }

    let mut cart = base_cartridge(
        5,
        prg_rom,
        chr_rom,
        vec![],
        vec![0; 0x20000],
        Mirroring::Horizontal,
    );
    cart.mmc5 = Some(Mmc5::new());
    cart
}

fn make_mapper19_cart() -> Cartridge {
    let mut prg_rom = vec![0; 64 * 0x2000];
    for bank in 0..64 {
        prg_rom[bank * 0x2000..(bank + 1) * 0x2000].fill(bank as u8);
    }

    let mut chr_rom = vec![0; 256 * 0x0400];
    for bank in 0..256 {
        chr_rom[bank * 0x0400..(bank + 1) * 0x0400].fill(0x80 | bank as u8);
    }

    let mut cart = base_cartridge(
        19,
        prg_rom,
        chr_rom,
        vec![0; 0x0800],
        vec![0; 0x2080],
        Mirroring::Horizontal,
    );
    cart.namco163 = Some(Namco163::new());
    cart
}

fn make_mapper18_cart() -> Cartridge {
    let mut prg_rom = vec![0; 64 * 0x2000];
    for bank in 0..64 {
        prg_rom[bank * 0x2000..(bank + 1) * 0x2000].fill(bank as u8);
    }

    let mut chr_rom = vec![0; 256 * 0x0400];
    for bank in 0..256 {
        chr_rom[bank * 0x0400..(bank + 1) * 0x0400].fill(0x80 | bank as u8);
    }

    let mut cart = base_cartridge(
        18,
        prg_rom,
        chr_rom,
        vec![],
        vec![0; 0x2000],
        Mirroring::Horizontal,
    );
    cart.jaleco_ss88006 = Some(JalecoSs88006::new());
    cart
}

fn make_mapper22_cart() -> Cartridge {
    let mut prg_rom = vec![0; 32 * 0x2000];
    for bank in 0..32 {
        prg_rom[bank * 0x2000..(bank + 1) * 0x2000].fill(bank as u8);
    }

    let mut chr_rom = vec![0; 128 * 0x0400];
    for bank in 0..128 {
        chr_rom[bank * 0x0400..(bank + 1) * 0x0400].fill(0x80 | bank as u8);
    }

    let mut cart = base_cartridge(22, prg_rom, chr_rom, vec![], vec![], Mirroring::Vertical);
    cart.vrc2_vrc4 = Some(Vrc2Vrc4::new());
    cart
}

fn make_mapper21_cart() -> Cartridge {
    let mut prg_rom = vec![0; 64 * 0x2000];
    for bank in 0..64 {
        prg_rom[bank * 0x2000..(bank + 1) * 0x2000].fill(bank as u8);
    }

    let mut chr_rom = vec![0; 256 * 0x0400];
    for bank in 0..256 {
        chr_rom[bank * 0x0400..(bank + 1) * 0x0400].fill(0x80 | bank as u8);
    }

    let mut cart = base_cartridge(
        21,
        prg_rom,
        chr_rom,
        vec![],
        vec![0; 0x2000],
        Mirroring::Vertical,
    );
    let mut vrc = Vrc2Vrc4::new();
    vrc.vrc4_mode = true;
    cart.vrc2_vrc4 = Some(vrc);
    cart
}

fn make_mapper210_cart(namco340: bool) -> Cartridge {
    let mut prg_rom = vec![0; 64 * 0x2000];
    for bank in 0..64 {
        prg_rom[bank * 0x2000..(bank + 1) * 0x2000].fill(bank as u8);
    }

    let mut chr_rom = vec![0; 256 * 0x0400];
    for bank in 0..256 {
        chr_rom[bank * 0x0400..(bank + 1) * 0x0400].fill(0x80 | bank as u8);
    }

    let mut cart = base_cartridge(
        210,
        prg_rom,
        chr_rom,
        vec![],
        if namco340 { vec![] } else { vec![0; 0x0800] },
        if namco340 {
            Mirroring::Horizontal
        } else {
            Mirroring::Vertical
        },
    );
    cart.has_battery = !namco340;
    cart.namco210 = Some(Namco210::new(namco340));
    cart
}

fn make_mapper23_cart() -> Cartridge {
    let mut prg_rom = vec![0; 64 * 0x2000];
    for bank in 0..64 {
        prg_rom[bank * 0x2000..(bank + 1) * 0x2000].fill(bank as u8);
    }

    let mut chr_rom = vec![0; 128 * 0x0400];
    for bank in 0..128 {
        chr_rom[bank * 0x0400..(bank + 1) * 0x0400].fill(0x80 | bank as u8);
    }

    let mut cart = base_cartridge(
        23,
        prg_rom,
        chr_rom,
        vec![],
        vec![0; 0x2000],
        Mirroring::Vertical,
    );
    cart.vrc2_vrc4 = Some(Vrc2Vrc4::new());
    cart
}

fn make_mapper24_26_cart(mapper: u8) -> Cartridge {
    let mut prg_rom = vec![0; 64 * 0x2000];
    for bank in 0..64 {
        prg_rom[bank * 0x2000..(bank + 1) * 0x2000].fill(bank as u8);
    }

    let mut chr_rom = vec![0; 256 * 0x0400];
    for bank in 0..256 {
        chr_rom[bank * 0x0400..(bank + 1) * 0x0400].fill(0x80 | bank as u8);
    }

    let mut cart = base_cartridge(
        mapper,
        prg_rom,
        chr_rom,
        vec![],
        vec![0; 0x2000],
        Mirroring::Vertical,
    );
    cart.vrc6 = Some(Vrc6::new());
    cart
}

fn make_mapper25_cart(has_battery: bool) -> Cartridge {
    let mut prg_rom = vec![0; 64 * 0x2000];
    for bank in 0..64 {
        prg_rom[bank * 0x2000..(bank + 1) * 0x2000].fill(bank as u8);
    }

    let mut chr_rom = vec![0; 128 * 0x0400];
    for bank in 0..128 {
        chr_rom[bank * 0x0400..(bank + 1) * 0x0400].fill(0x80 | bank as u8);
    }

    let mut cart = base_cartridge(
        25,
        prg_rom,
        chr_rom,
        vec![],
        vec![0; 0x2000],
        Mirroring::Vertical,
    );
    cart.has_battery = has_battery;
    cart.vrc2_vrc4 = Some(Vrc2Vrc4::new());
    cart
}

fn make_mapper64_cart() -> Cartridge {
    let mut prg_rom = vec![0; 32 * 0x2000];
    for bank in 0..32 {
        prg_rom[bank * 0x2000..(bank + 1) * 0x2000].fill(bank as u8);
    }

    let mut chr_rom = vec![0; 256 * 0x0400];
    for bank in 0..256 {
        chr_rom[bank * 0x0400..(bank + 1) * 0x0400].fill(0x80 | bank as u8);
    }

    let mut cart = base_cartridge(64, prg_rom, chr_rom, vec![], vec![], Mirroring::Vertical);
    cart.mmc3 = Some(Mmc3::new());
    cart
}

fn make_simple_bank_cart(mapper: u8, prg_banks_32k: usize, chr_banks_8k: usize) -> Cartridge {
    let mut prg_rom = vec![0; prg_banks_32k * 0x8000];
    for bank in 0..prg_banks_32k {
        let fill = bank as u8;
        prg_rom[bank * 0x8000..(bank + 1) * 0x8000].fill(fill);
    }

    let mut chr_rom = vec![0; chr_banks_8k * 0x2000];
    for bank in 0..chr_banks_8k {
        let fill = 0x40 | bank as u8;
        chr_rom[bank * 0x2000..(bank + 1) * 0x2000].fill(fill);
    }

    base_cartridge(
        mapper,
        prg_rom,
        chr_rom,
        vec![],
        vec![],
        Mirroring::Horizontal,
    )
}

fn make_camerica_cart() -> Cartridge {
    let mut prg_rom = vec![0; 8 * 0x4000];
    for bank in 0..8 {
        prg_rom[bank * 0x4000..(bank + 1) * 0x4000].fill(bank as u8);
    }

    base_cartridge(
        71,
        prg_rom,
        vec![0; 0x2000],
        vec![],
        vec![],
        Mirroring::Horizontal,
    )
}

fn make_uxrom_like_cart(mapper: u8, prg_banks_16k: usize, chr_banks_8k: usize) -> Cartridge {
    let mut prg_rom = vec![0; prg_banks_16k * 0x4000];
    for bank in 0..prg_banks_16k {
        prg_rom[bank * 0x4000..(bank + 1) * 0x4000].fill(bank as u8);
    }

    let mut chr_rom = vec![0; chr_banks_8k * 0x2000];
    for bank in 0..chr_banks_8k {
        chr_rom[bank * 0x2000..(bank + 1) * 0x2000].fill(0x70 | bank as u8);
    }

    base_cartridge(
        mapper,
        prg_rom,
        chr_rom,
        vec![],
        vec![],
        Mirroring::Horizontal,
    )
}

fn make_mapper78_cart(hv_mirroring: bool) -> Cartridge {
    let mut cart = make_uxrom_like_cart(78, 8, 16);
    cart.mapper78_hv_mirroring = hv_mirroring;
    cart.mirroring = if hv_mirroring {
        Mirroring::Horizontal
    } else {
        Mirroring::OneScreenLower
    };
    cart
}

fn make_mapper40_cart() -> Cartridge {
    let mut prg_rom = vec![0; 8 * 0x2000];
    for bank in 0..8 {
        prg_rom[bank * 0x2000..(bank + 1) * 0x2000].fill(bank as u8);
    }

    let mut cart = base_cartridge(
        40,
        prg_rom,
        vec![0; 0x2000],
        vec![],
        vec![],
        Mirroring::Horizontal,
    );
    cart.mapper40 = Some(Mapper40::new());
    cart
}

fn make_mapper42_cart() -> Cartridge {
    let mut prg_rom = vec![0; 16 * 0x2000];
    for bank in 0..16 {
        prg_rom[bank * 0x2000..(bank + 1) * 0x2000].fill(bank as u8);
    }

    let mut cart = base_cartridge(
        42,
        prg_rom,
        vec![0; 0x2000],
        vec![],
        vec![],
        Mirroring::Vertical,
    );
    cart.mapper42 = Some(Mapper42::new());
    cart
}

fn make_mapper43_cart() -> Cartridge {
    let mut prg_rom = vec![0; 0x14000];
    for bank in 0..8 {
        prg_rom[bank * 0x2000..(bank + 1) * 0x2000].fill(bank as u8);
    }
    prg_rom[0x10000..0x10800].fill(0xF2);
    prg_rom[0x10800..0x12000].fill(0xF2);
    prg_rom[0x12000..0x14000].fill(0xE8);

    let mut cart = base_cartridge(
        43,
        prg_rom,
        vec![0; 0x2000],
        vec![],
        vec![],
        Mirroring::Vertical,
    );
    cart.mapper43 = Some(Mapper43::new());
    cart
}

fn make_mapper50_cart() -> Cartridge {
    let mut prg_rom = vec![0; 16 * 0x2000];
    for bank in 0..16 {
        prg_rom[bank * 0x2000..(bank + 1) * 0x2000].fill(bank as u8);
    }

    let mut cart = base_cartridge(
        50,
        prg_rom,
        vec![0; 0x2000],
        vec![],
        vec![],
        Mirroring::Horizontal,
    );
    cart.mapper50 = Some(Mapper50::new());
    cart
}

fn make_mapper32_cart() -> Cartridge {
    let mut prg_rom = vec![0; 8 * 0x2000];
    for bank in 0..8 {
        prg_rom[bank * 0x2000..(bank + 1) * 0x2000].fill(bank as u8);
    }

    let mut chr_rom = vec![0; 16 * 0x0400];
    for bank in 0..16 {
        chr_rom[bank * 0x0400..(bank + 1) * 0x0400].fill(0x30 | bank as u8);
    }

    let mut cart = base_cartridge(32, prg_rom, chr_rom, vec![], vec![], Mirroring::Vertical);
    cart.irem_g101 = Some(IremG101::new());
    cart
}

fn make_vrc3_cart() -> Cartridge {
    let mut prg_rom = vec![0; 8 * 0x4000];
    for bank in 0..8 {
        prg_rom[bank * 0x4000..(bank + 1) * 0x4000].fill(bank as u8);
    }

    let mut cart = base_cartridge(
        73,
        prg_rom,
        vec![0; 0x2000],
        vec![],
        vec![0; 0x2000],
        Mirroring::Vertical,
    );
    cart.vrc3 = Some(Vrc3::new());
    cart
}

fn make_mapper65_cart() -> Cartridge {
    let mut prg_rom = vec![0; 16 * 0x2000];
    for bank in 0..16 {
        prg_rom[bank * 0x2000..(bank + 1) * 0x2000].fill(bank as u8);
    }

    let mut chr_rom = vec![0; 16 * 0x0400];
    for bank in 0..16 {
        chr_rom[bank * 0x0400..(bank + 1) * 0x0400].fill(0x90 | bank as u8);
    }

    let mut cart = base_cartridge(65, prg_rom, chr_rom, vec![], vec![], Mirroring::Vertical);
    cart.irem_h3001 = Some(IremH3001::new());
    cart
}

fn make_mapper103_cart() -> Cartridge {
    let mut prg_rom = vec![0; 0x20000];
    for bank in 0..12 {
        prg_rom[bank * 0x2000..(bank + 1) * 0x2000].fill(bank as u8);
    }
    prg_rom[0x18000..0x1B800].fill(0xA1);
    prg_rom[0x1B800..0x1D800].fill(0xB2);
    prg_rom[0x1D800..0x20000].fill(0xC3);

    base_cartridge(
        103,
        prg_rom,
        vec![0; 0x2000],
        vec![],
        vec![0; 0x2000],
        Mirroring::Vertical,
    )
}

fn make_mapper153_cart() -> Cartridge {
    let mut prg_rom = vec![0; 32 * 0x4000];
    for bank in 0..32 {
        prg_rom[bank * 0x4000..(bank + 1) * 0x4000].fill(bank as u8);
    }

    let mut cart = base_cartridge(
        153,
        prg_rom,
        vec![],
        vec![0; 0x2000],
        vec![0; 0x8000],
        Mirroring::Vertical,
    );
    cart.has_battery = true;
    cart.bandai_fcg = Some(BandaiFcg::new());
    if let Some(ref mut bandai) = cart.bandai_fcg {
        bandai.configure_mapper(153, true);
    }
    cart
}

fn make_mapper159_cart() -> Cartridge {
    let mut prg_rom = vec![0; 32 * 0x4000];
    for bank in 0..32 {
        prg_rom[bank * 0x4000..(bank + 1) * 0x4000].fill(bank as u8);
    }

    let mut chr_rom = vec![0; 128 * 0x0400];
    for bank in 0..128 {
        chr_rom[bank * 0x0400..(bank + 1) * 0x0400].fill(0x80 | bank as u8);
    }

    let mut cart = base_cartridge(
        159,
        prg_rom,
        chr_rom,
        vec![],
        vec![0xFF; 0x80],
        Mirroring::Vertical,
    );
    cart.has_battery = true;
    cart.bandai_fcg = Some(BandaiFcg::new());
    if let Some(ref mut bandai) = cart.bandai_fcg {
        bandai.configure_mapper(159, true);
    }
    cart
}

fn make_mapper57_cart() -> Cartridge {
    let mut prg_rom = vec![0; 8 * 0x4000];
    for bank in 0..8 {
        prg_rom[bank * 0x4000..(bank + 1) * 0x4000].fill(bank as u8);
    }

    let mut chr_rom = vec![0; 16 * 0x2000];
    for bank in 0..16 {
        chr_rom[bank * 0x2000..(bank + 1) * 0x2000].fill(0xA0 | bank as u8);
    }

    base_cartridge(57, prg_rom, chr_rom, vec![], vec![], Mirroring::Vertical)
}

fn make_mapper63_cart() -> Cartridge {
    let mut prg_rom = vec![0; 8 * 0x4000];
    for bank in 0..8 {
        prg_rom[bank * 0x4000..(bank + 1) * 0x4000].fill(bank as u8);
    }

    base_cartridge(
        63,
        prg_rom,
        vec![],
        vec![0; 0x2000],
        vec![],
        Mirroring::Vertical,
    )
}

fn make_mapper77_cart() -> Cartridge {
    let mut prg_rom = vec![0; 4 * 0x8000];
    for bank in 0..4 {
        prg_rom[bank * 0x8000..(bank + 1) * 0x8000].fill(bank as u8);
    }

    let mut chr_rom = vec![0; 4 * 0x0800];
    for bank in 0..4 {
        chr_rom[bank * 0x0800..(bank + 1) * 0x0800].fill(0x80 | bank as u8);
    }

    base_cartridge(
        77,
        prg_rom,
        chr_rom,
        vec![0; 0x2000],
        vec![],
        Mirroring::FourScreen,
    )
}

fn make_mapper99_cart() -> Cartridge {
    let mut prg_rom = vec![0; 5 * 0x2000];
    for bank in 0..5 {
        prg_rom[bank * 0x2000..(bank + 1) * 0x2000].fill(bank as u8);
    }

    let mut chr_rom = vec![0; 2 * 0x2000];
    for bank in 0..2 {
        chr_rom[bank * 0x2000..(bank + 1) * 0x2000].fill(0x90 | bank as u8);
    }

    base_cartridge(
        99,
        prg_rom,
        chr_rom,
        vec![0; 0x1000],
        vec![0; 0x0800],
        Mirroring::FourScreen,
    )
}

fn make_mapper137_cart() -> Cartridge {
    let mut prg_rom = vec![0; 8 * 0x8000];
    for bank in 0..8 {
        prg_rom[bank * 0x8000..(bank + 1) * 0x8000].fill(bank as u8);
    }

    let mut chr_rom = vec![0; 32 * 0x0400];
    for bank in 0..32 {
        chr_rom[bank * 0x0400..(bank + 1) * 0x0400].fill(0xA0 | bank as u8);
    }

    base_cartridge(137, prg_rom, chr_rom, vec![], vec![], Mirroring::Vertical)
}

fn make_mapper142_cart() -> Cartridge {
    let mut prg_rom = vec![0; 16 * 0x2000];
    for bank in 0..16 {
        prg_rom[bank * 0x2000..(bank + 1) * 0x2000].fill(bank as u8);
    }

    let mut cart = base_cartridge(
        142,
        prg_rom,
        vec![0; 0x2000],
        vec![],
        vec![],
        Mirroring::Horizontal,
    );
    cart.vrc3 = Some(Vrc3::new());
    cart
}

fn make_mapper150_cart() -> Cartridge {
    let mut prg_rom = vec![0; 4 * 0x8000];
    for bank in 0..4 {
        prg_rom[bank * 0x8000..(bank + 1) * 0x8000].fill(bank as u8);
    }

    let mut chr_rom = vec![0; 8 * 0x2000];
    for bank in 0..8 {
        chr_rom[bank * 0x2000..(bank + 1) * 0x2000].fill(0xB0 | bank as u8);
    }

    base_cartridge(150, prg_rom, chr_rom, vec![], vec![], Mirroring::Vertical)
}

fn make_vrc1_cart(mapper: u8) -> Cartridge {
    let mut prg_rom = vec![0; 8 * 0x2000];
    for bank in 0..8 {
        prg_rom[bank * 0x2000..(bank + 1) * 0x2000].fill(bank as u8);
    }

    let mut chr_rom = vec![0; 16 * 0x1000];
    for bank in 0..16 {
        chr_rom[bank * 0x1000..(bank + 1) * 0x1000].fill(0x60 | bank as u8);
    }

    let mut cart = base_cartridge(
        mapper,
        prg_rom,
        chr_rom,
        vec![],
        vec![],
        Mirroring::Vertical,
    );
    cart.vrc1 = Some(Vrc1::new());
    cart
}

fn make_nina001_cart() -> Cartridge {
    let mut prg_rom = vec![0; 4 * 0x8000];
    for bank in 0..4 {
        prg_rom[bank * 0x8000..(bank + 1) * 0x8000].fill(bank as u8);
    }

    let mut chr_rom = vec![0; 4 * 0x1000];
    for bank in 0..4 {
        chr_rom[bank * 0x1000..(bank + 1) * 0x1000].fill(0x50 | bank as u8);
    }

    let mut cart = base_cartridge(
        34,
        prg_rom,
        chr_rom,
        vec![],
        vec![0; 0x2000],
        Mirroring::Horizontal,
    );
    cart.mapper34_nina001 = true;
    cart
}

fn make_split_chr_cart(mapper: u8, chr_banks_4k: usize, upper_bank: u8) -> Cartridge {
    let mut chr_rom = vec![0; chr_banks_4k * 0x1000];
    for bank in 0..chr_banks_4k {
        chr_rom[bank * 0x1000..(bank + 1) * 0x1000].fill(0x60 | bank as u8);
    }

    let chr_bank = if mapper == 13 { upper_bank } else { 0 };
    let mut cart = base_cartridge(
        mapper,
        vec![0; 0x8000],
        chr_rom,
        vec![],
        vec![],
        Mirroring::Horizontal,
    );
    cart.chr_bank = chr_bank;
    cart.chr_bank_1 = upper_bank;
    cart
}

fn make_namco108_cart(mapper: u8, prg_banks_8k: usize, chr_banks_1k: usize) -> Cartridge {
    let mut prg_rom = vec![0; prg_banks_8k * 0x2000];
    for bank in 0..prg_banks_8k {
        prg_rom[bank * 0x2000..(bank + 1) * 0x2000].fill(bank as u8);
    }

    let mut chr_rom = vec![0; chr_banks_1k * 0x0400];
    for bank in 0..chr_banks_1k {
        chr_rom[bank * 0x0400..(bank + 1) * 0x0400].fill(0x20u8.wrapping_add(bank as u8));
    }

    let mut cart = base_cartridge(
        mapper,
        prg_rom,
        chr_rom,
        vec![],
        vec![],
        Mirroring::Vertical,
    );
    cart.mmc3 = Some(Mmc3::new());
    cart
}

fn make_mmc3_mixed_chr_cart(
    mapper: u8,
    prg_banks_8k: usize,
    chr_banks_1k: usize,
    chr_ram_size: usize,
) -> Cartridge {
    let mut prg_rom = vec![0; prg_banks_8k * 0x2000];
    for bank in 0..prg_banks_8k {
        prg_rom[bank * 0x2000..(bank + 1) * 0x2000].fill(bank as u8);
    }

    let mut chr_rom = vec![0; chr_banks_1k * 0x0400];
    for bank in 0..chr_banks_1k {
        chr_rom[bank * 0x0400..(bank + 1) * 0x0400].fill(0x50u8.wrapping_add(bank as u8));
    }

    let mut cart = base_cartridge(
        mapper,
        prg_rom,
        chr_rom,
        vec![0; chr_ram_size],
        vec![0; 0x2000],
        Mirroring::Vertical,
    );
    cart.mmc3 = Some(Mmc3::new());
    cart
}

fn make_sunsoft4_cart() -> Cartridge {
    let mut prg_rom = vec![0; 4 * 0x4000];
    for bank in 0..4 {
        prg_rom[bank * 0x4000..(bank + 1) * 0x4000].fill(bank as u8);
    }

    let mut chr_rom = vec![0; 0x40000];
    for bank in 0..(chr_rom.len() / 0x0400) {
        chr_rom[bank * 0x0400..(bank + 1) * 0x0400].fill(bank as u8);
    }

    let mut cart = base_cartridge(
        68,
        prg_rom,
        chr_rom,
        vec![],
        vec![0; 0x2000],
        Mirroring::Vertical,
    );
    cart.sunsoft4 = Some(Sunsoft4::new());
    cart
}

fn make_mapper15_cart() -> Cartridge {
    let mut prg_rom = vec![0; 16 * 0x4000];
    for bank in 0..16 {
        prg_rom[bank * 0x4000..(bank + 1) * 0x4000].fill(bank as u8);
    }

    let mut cart = base_cartridge(
        15,
        prg_rom,
        vec![0; 0x2000],
        vec![],
        vec![0; 0x2000],
        Mirroring::Vertical,
    );
    cart.mapper15 = Some(Mapper15::new());
    cart
}

fn make_mapper225_cart(mapper: u8) -> Cartridge {
    let mut prg_rom = vec![0; 128 * 0x4000];
    for bank in 0..128 {
        prg_rom[bank * 0x4000..(bank + 1) * 0x4000].fill(bank as u8);
    }

    let mut chr_rom = vec![0; 128 * 0x2000];
    for bank in 0..128 {
        chr_rom[bank * 0x2000..(bank + 1) * 0x2000].fill(0x80u8.wrapping_add(bank as u8));
    }

    base_cartridge(
        mapper,
        prg_rom,
        chr_rom,
        vec![],
        if mapper == 225 { vec![0; 4] } else { vec![] },
        Mirroring::Vertical,
    )
}

fn make_mapper228_cart() -> Cartridge {
    let mut prg_rom = vec![0; 3 * 0x80000];
    for bank in 0..(prg_rom.len() / 0x4000) {
        prg_rom[bank * 0x4000..(bank + 1) * 0x4000].fill(bank as u8);
    }

    let mut chr_rom = vec![0; 64 * 0x2000];
    for bank in 0..64 {
        chr_rom[bank * 0x2000..(bank + 1) * 0x2000].fill(0x20u8.wrapping_add(bank as u8));
    }

    base_cartridge(228, prg_rom, chr_rom, vec![], vec![], Mirroring::Vertical)
}

fn make_mapper242_cart() -> Cartridge {
    let mut prg_rom = vec![0; 32 * 0x4000];
    for bank in 0..32 {
        prg_rom[bank * 0x4000..(bank + 1) * 0x4000].fill(bank as u8);
    }

    base_cartridge(
        242,
        prg_rom,
        vec![0; 0x2000],
        vec![],
        vec![],
        Mirroring::Vertical,
    )
}

fn make_mapper245_cart() -> Cartridge {
    let mut prg_rom = vec![0; 64 * 0x2000];
    for bank in 0..64 {
        prg_rom[bank * 0x2000..(bank + 1) * 0x2000].fill(bank as u8);
    }

    let mut cart = base_cartridge(
        245,
        prg_rom,
        vec![],
        vec![0; 0x2000],
        vec![0; 0x2000],
        Mirroring::Vertical,
    );
    cart.mmc3 = Some(Mmc3::new());
    cart
}

fn make_mapper235_cart() -> Cartridge {
    let mut prg_rom = vec![0; 128 * 0x4000];
    for bank in 0..128 {
        prg_rom[bank * 0x4000..(bank + 1) * 0x4000].fill(bank as u8);
    }

    base_cartridge(
        235,
        prg_rom,
        vec![0; 0x2000],
        vec![],
        vec![],
        Mirroring::Horizontal,
    )
}

fn make_mapper227_cart() -> Cartridge {
    let mut prg_rom = vec![0; 64 * 0x4000];
    for bank in 0..64 {
        prg_rom[bank * 0x4000..(bank + 1) * 0x4000].fill(bank as u8);
    }

    base_cartridge(
        227,
        prg_rom,
        vec![0; 0x2000],
        vec![],
        vec![],
        Mirroring::Vertical,
    )
}

fn make_mapper246_cart() -> Cartridge {
    let mut prg_rom = vec![0; 64 * 0x2000];
    for bank in 0..64 {
        prg_rom[bank * 0x2000..(bank + 1) * 0x2000].fill(bank as u8);
    }

    let mut chr_rom = vec![0; 64 * 0x0800];
    for bank in 0..64 {
        chr_rom[bank * 0x0800..(bank + 1) * 0x0800].fill(0x40 | bank as u8);
    }

    let mut cart = base_cartridge(
        246,
        prg_rom,
        chr_rom,
        vec![],
        vec![0; 0x800],
        Mirroring::Vertical,
    );
    cart.mapper246 = Some(Mapper246::new());
    cart
}

fn make_mapper236_cart(chr_ram_variant: bool) -> Cartridge {
    let prg_bank_count = if chr_ram_variant { 64 } else { 16 };
    let mut prg_rom = vec![0; prg_bank_count * 0x4000];
    for bank in 0..prg_bank_count {
        prg_rom[bank * 0x4000..(bank + 1) * 0x4000].fill(bank as u8);
    }

    let chr_rom = if chr_ram_variant {
        vec![]
    } else {
        let mut chr_rom = vec![0; 16 * 0x2000];
        for bank in 0..16 {
            chr_rom[bank * 0x2000..(bank + 1) * 0x2000].fill(0x80 | bank as u8);
        }
        chr_rom
    };

    let mut cart = base_cartridge(
        236,
        prg_rom,
        chr_rom,
        if chr_ram_variant {
            vec![0; 0x2000]
        } else {
            vec![]
        },
        vec![],
        Mirroring::Vertical,
    );
    cart.mapper236_chr_ram = chr_ram_variant;
    cart
}

fn make_mapper231_cart() -> Cartridge {
    let mut prg_rom = vec![0; 32 * 0x4000];
    for bank in 0..32 {
        prg_rom[bank * 0x4000..(bank + 1) * 0x4000].fill(bank as u8);
    }

    base_cartridge(
        231,
        prg_rom,
        vec![],
        vec![0; 0x2000],
        vec![],
        Mirroring::Vertical,
    )
}

fn make_mapper230_cart() -> Cartridge {
    let mut prg_rom = vec![0; 40 * 0x4000];
    for bank in 0..40 {
        prg_rom[bank * 0x4000..(bank + 1) * 0x4000].fill(bank as u8);
    }

    let mut cart = base_cartridge(
        230,
        prg_rom,
        vec![0; 0x2000],
        vec![],
        vec![],
        Mirroring::Vertical,
    );
    cart.mapper230_contra_mode = true;
    cart
}

fn make_mapper221_cart() -> Cartridge {
    let mut prg_rom = vec![0; 64 * 0x4000];
    for bank in 0..64 {
        prg_rom[bank * 0x4000..(bank + 1) * 0x4000].fill(bank as u8);
    }

    base_cartridge(
        221,
        prg_rom,
        vec![],
        vec![0; 0x2000],
        vec![],
        Mirroring::Vertical,
    )
}

fn make_taito_tc0190_cart() -> Cartridge {
    let mut prg_rom = vec![0; 16 * 0x2000];
    for bank in 0..16 {
        prg_rom[bank * 0x2000..(bank + 1) * 0x2000].fill(bank as u8);
    }

    let mut chr_rom = vec![0; 32 * 0x0400];
    for bank in 0..32 {
        chr_rom[bank * 0x0400..(bank + 1) * 0x0400].fill(0x40u8.wrapping_add(bank as u8));
    }

    let mut cart = base_cartridge(33, prg_rom, chr_rom, vec![], vec![], Mirroring::Vertical);
    cart.taito_tc0190 = Some(TaitoTc0190::new());
    cart
}

fn make_mapper48_cart() -> Cartridge {
    let mut prg_rom = vec![0; 16 * 0x2000];
    for bank in 0..16 {
        prg_rom[bank * 0x2000..(bank + 1) * 0x2000].fill(bank as u8);
    }

    let mut chr_rom = vec![0; 32 * 0x0400];
    for bank in 0..32 {
        chr_rom[bank * 0x0400..(bank + 1) * 0x0400].fill(0x80u8.wrapping_add(bank as u8));
    }

    let mut cart = base_cartridge(48, prg_rom, chr_rom, vec![], vec![], Mirroring::Vertical);
    cart.taito_tc0190 = Some(TaitoTc0190::new());
    cart
}

fn make_mapper114_cart(mapper: u8) -> Cartridge {
    let mut prg_rom = vec![0; 32 * 0x2000];
    for bank in 0..32 {
        prg_rom[bank * 0x2000..(bank + 1) * 0x2000].fill(bank as u8);
    }

    let mut chr_rom = vec![0; 512 * 0x0400];
    for bank in 0..512 {
        let fill = ((bank & 0xFF) as u8) ^ (((bank >> 8) as u8) << 7);
        chr_rom[bank * 0x0400..(bank + 1) * 0x0400].fill(fill);
    }

    let mut cart = base_cartridge(
        mapper,
        prg_rom,
        chr_rom,
        vec![],
        vec![0; 0x2000],
        Mirroring::Vertical,
    );
    cart.mmc3 = Some(Mmc3::new());
    cart
}

fn make_mapper123_cart() -> Cartridge {
    let mut prg_rom = vec![0; 32 * 0x2000];
    for bank in 0..32 {
        prg_rom[bank * 0x2000..(bank + 1) * 0x2000].fill(bank as u8);
    }

    let mut chr_rom = vec![0; 64 * 0x0400];
    for bank in 0..64 {
        chr_rom[bank * 0x0400..(bank + 1) * 0x0400].fill(0x60u8.wrapping_add(bank as u8));
    }

    let mut cart = base_cartridge(
        123,
        prg_rom,
        chr_rom,
        vec![],
        vec![0; 0x2000],
        Mirroring::Vertical,
    );
    cart.mmc3 = Some(Mmc3::new());
    cart
}

fn make_mapper115_cart(mapper: u8) -> Cartridge {
    let mut prg_rom = vec![0; 64 * 0x2000];
    for bank in 0..64 {
        prg_rom[bank * 0x2000..(bank + 1) * 0x2000].fill(bank as u8);
    }

    let mut chr_rom = vec![0; 512 * 0x0400];
    for bank in 0..512 {
        let fill = ((bank & 0xFF) as u8) ^ (((bank >> 8) as u8) << 7);
        chr_rom[bank * 0x0400..(bank + 1) * 0x0400].fill(fill);
    }

    let mut cart = base_cartridge(
        mapper,
        prg_rom,
        chr_rom,
        vec![],
        vec![],
        Mirroring::Vertical,
    );
    cart.mmc3 = Some(Mmc3::new());
    cart
}

fn make_mapper205_cart() -> Cartridge {
    let mut prg_rom = vec![0; 64 * 0x2000];
    for bank in 0..64 {
        prg_rom[bank * 0x2000..(bank + 1) * 0x2000].fill(bank as u8);
    }

    let mut chr_rom = vec![0; 512 * 0x0400];
    for bank in 0..512 {
        let fill = ((bank & 0x3F) as u8) | ((((bank >> 7) & 0x03) as u8) << 6);
        chr_rom[bank * 0x0400..(bank + 1) * 0x0400].fill(fill);
    }

    let mut cart = base_cartridge(205, prg_rom, chr_rom, vec![], vec![], Mirroring::Vertical);
    cart.mmc3 = Some(Mmc3::new());
    cart
}

fn make_mapper12_cart() -> Cartridge {
    let mut prg_rom = vec![0; 32 * 0x2000];
    for bank in 0..32 {
        prg_rom[bank * 0x2000..(bank + 1) * 0x2000].fill(bank as u8);
    }

    let mut chr_rom = vec![0; 512 * 0x0400];
    for bank in 0..512 {
        let fill = ((bank & 0xFF) as u8) ^ (((bank >> 8) as u8) << 7);
        chr_rom[bank * 0x0400..(bank + 1) * 0x0400].fill(fill);
    }

    let mut cart = base_cartridge(12, prg_rom, chr_rom, vec![], vec![], Mirroring::Vertical);
    cart.mmc3 = Some(Mmc3::new());
    cart
}

fn make_mapper59_cart() -> Cartridge {
    let mut prg_rom = vec![0; 8 * 0x4000];
    for bank in 0..8 {
        prg_rom[bank * 0x4000..(bank + 1) * 0x4000].fill(bank as u8);
    }

    let mut chr_rom = vec![0; 8 * 0x2000];
    for bank in 0..8 {
        chr_rom[bank * 0x2000..(bank + 1) * 0x2000].fill(0x40 | bank as u8);
    }

    base_cartridge(59, prg_rom, chr_rom, vec![], vec![], Mirroring::Vertical)
}

fn make_mapper60_cart() -> Cartridge {
    let mut prg_rom = vec![0; 4 * 0x4000];
    for bank in 0..4 {
        prg_rom[bank * 0x4000..(bank + 1) * 0x4000].fill(bank as u8);
    }

    let mut chr_rom = vec![0; 4 * 0x2000];
    for bank in 0..4 {
        chr_rom[bank * 0x2000..(bank + 1) * 0x2000].fill(0x50 | bank as u8);
    }

    base_cartridge(60, prg_rom, chr_rom, vec![], vec![], Mirroring::Horizontal)
}

fn make_mapper61_cart() -> Cartridge {
    let mut prg_rom = vec![0; 32 * 0x4000];
    for bank in 0..32 {
        prg_rom[bank * 0x4000..(bank + 1) * 0x4000].fill(bank as u8);
    }

    let mut chr_rom = vec![0; 16 * 0x2000];
    for bank in 0..16 {
        chr_rom[bank * 0x2000..(bank + 1) * 0x2000].fill(0x40 | bank as u8);
    }

    base_cartridge(61, prg_rom, chr_rom, vec![], vec![], Mirroring::Vertical)
}

fn make_mapper67_cart() -> Cartridge {
    let mut prg_rom = vec![0; 8 * 0x4000];
    for bank in 0..8 {
        prg_rom[bank * 0x4000..(bank + 1) * 0x4000].fill(bank as u8);
    }

    let mut chr_rom = vec![0; 8 * 0x0800];
    for bank in 0..8 {
        chr_rom[bank * 0x0800..(bank + 1) * 0x0800].fill(0x70 | bank as u8);
    }

    let mut cart = base_cartridge(67, prg_rom, chr_rom, vec![], vec![], Mirroring::Vertical);
    cart.sunsoft3 = Some(Sunsoft3::new());
    cart
}

fn make_mapper185_cart() -> Cartridge {
    let mut chr_rom = vec![0; 4 * 0x2000];
    for bank in 0..4 {
        chr_rom[bank * 0x2000..(bank + 1) * 0x2000].fill(0x60 | bank as u8);
    }

    let cart = base_cartridge(
        185,
        vec![0xFF; 0x8000],
        chr_rom,
        vec![],
        vec![],
        Mirroring::Vertical,
    );
    cart.mapper185_disabled_reads.set(2);
    cart
}

fn make_mapper189_cart() -> Cartridge {
    let mut prg_rom = vec![0; 16 * 0x8000];
    for bank in 0..16 {
        prg_rom[bank * 0x8000..(bank + 1) * 0x8000].fill(bank as u8);
    }

    let mut chr_rom = vec![0; 16 * 0x0400];
    for bank in 0..16 {
        chr_rom[bank * 0x0400..(bank + 1) * 0x0400].fill(0x60 | bank as u8);
    }

    let mut cart = base_cartridge(189, prg_rom, chr_rom, vec![], vec![], Mirroring::Vertical);
    cart.mmc3 = Some(Mmc3::new());
    cart
}

fn make_mapper44_cart() -> Cartridge {
    let mut prg_rom = vec![0; 128 * 0x2000];
    for bank in 0..128 {
        prg_rom[bank * 0x2000..(bank + 1) * 0x2000].fill(bank as u8);
    }

    let mut chr_rom = vec![0; 1024 * 0x0400];
    for bank in 0..1024 {
        let fill = ((bank & 0x1F) as u8) | ((((bank >> 7) & 0x07) as u8) << 5);
        chr_rom[bank * 0x0400..(bank + 1) * 0x0400].fill(fill);
    }

    let mut cart = base_cartridge(44, prg_rom, chr_rom, vec![], vec![], Mirroring::Vertical);
    cart.mmc3 = Some(Mmc3::new());
    cart
}

fn make_taito_x1005_cart() -> Cartridge {
    let mut prg_rom = vec![0; 16 * 0x2000];
    for bank in 0..16 {
        prg_rom[bank * 0x2000..(bank + 1) * 0x2000].fill(bank as u8);
    }

    let mut chr_rom = vec![0; 32 * 0x0400];
    for bank in 0..32 {
        chr_rom[bank * 0x0400..(bank + 1) * 0x0400].fill(0x80u8.wrapping_add(bank as u8));
    }

    let mut cart = base_cartridge(
        80,
        prg_rom,
        chr_rom,
        vec![],
        vec![0; 128],
        Mirroring::Horizontal,
    );
    cart.taito_x1005 = Some(TaitoX1005::new());
    cart
}

fn make_taito_x1017_cart() -> Cartridge {
    let mut prg_rom = vec![0; 16 * 0x2000];
    for bank in 0..16 {
        prg_rom[bank * 0x2000..(bank + 1) * 0x2000].fill(bank as u8);
    }

    let mut chr_rom = vec![0; 32 * 0x0400];
    for bank in 0..32 {
        chr_rom[bank * 0x0400..(bank + 1) * 0x0400].fill(0x90 | bank as u8);
    }

    let mut cart = base_cartridge(
        82,
        prg_rom,
        chr_rom,
        vec![],
        vec![0; 0x1400],
        Mirroring::Horizontal,
    );
    cart.has_battery = true;
    cart.taito_x1017 = Some(TaitoX1017::new());
    cart
}

fn make_mapper208_cart() -> Cartridge {
    let mut prg_rom = vec![0; 4 * 0x8000];
    for bank in 0..4 {
        prg_rom[bank * 0x8000..(bank + 1) * 0x8000].fill(bank as u8);
    }

    let mut chr_rom = vec![0; 16 * 0x0400];
    for bank in 0..16 {
        chr_rom[bank * 0x0400..(bank + 1) * 0x0400].fill(0x50u8.wrapping_add(bank as u8));
    }

    let mut cart = base_cartridge(208, prg_rom, chr_rom, vec![], vec![], Mirroring::Vertical);
    cart.prg_bank = 3;
    cart.mmc3 = Some(Mmc3::new());
    cart
}

mod basic;
mod multicart;
mod special;
