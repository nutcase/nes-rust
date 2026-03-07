use super::*;

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
        bandai_fcg: None,
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
