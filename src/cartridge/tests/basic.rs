use super::*;

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

#[test]
fn mapper_5_switches_prg_chr_wram_and_multiplier() {
    let mut cart = make_mmc5_cart();

    cart.write_prg(0x5100, 0x03);
    cart.write_prg(0x5114, 0x81);
    cart.write_prg(0x5115, 0x82);
    cart.write_prg(0x5116, 0x83);
    cart.write_prg(0x5117, 0x9F);

    assert_eq!(cart.read_prg(0x8000), 1);
    assert_eq!(cart.read_prg(0xA000), 2);
    assert_eq!(cart.read_prg(0xC000), 3);
    assert_eq!(cart.read_prg(0xE000), 31);

    cart.write_prg(0x5102, 0x02);
    cart.write_prg(0x5103, 0x01);
    cart.write_prg(0x5113, 0x04);
    cart.write_prg_ram(0x6123, 0x5A);
    assert_eq!(cart.read_prg_ram(0x6123), 0x5A);

    cart.write_prg(0x5101, 0x03);
    cart.write_prg(0x5120, 0x03);
    cart.write_prg(0x5121, 0x04);
    assert_eq!(cart.read_chr(0x0000), 0x83);
    assert_eq!(cart.read_chr_sprite(0x0400, 0), 0x84);

    cart.write_prg(0x5205, 7);
    cart.write_prg(0x5206, 9);
    assert_eq!(cart.read_prg_low(0x5205), 63);
    assert_eq!(cart.read_prg_low(0x5206), 0);

    let snapshot = cart.snapshot_state();

    cart.write_prg(0x5114, 0x87);
    cart.write_prg(0x5113, 0x00);
    cart.write_prg_ram(0x6123, 0x00);
    cart.write_prg(0x5205, 3);
    cart.write_prg(0x5206, 4);

    cart.restore_state(&snapshot);

    assert_eq!(cart.read_prg(0x8000), 1);
    assert_eq!(cart.read_prg_ram(0x6123), 0x5A);
    assert_eq!(cart.read_prg_low(0x5205), 63);
}

#[test]
fn mapper_5_tracks_ppudata_chr_source_and_audio_status() {
    let mut cart = make_mmc5_cart();

    cart.write_prg(0x5101, 0x03);
    cart.write_prg(0x5128, 0x11);
    assert_eq!(cart.read_chr(0x0000), 0x91);

    cart.write_prg(0x5120, 0x03);
    assert_eq!(cart.read_chr(0x0000), 0x83);

    cart.write_prg(0x5015, 0x03);
    cart.write_prg(0x5000, 0xDF);
    cart.write_prg(0x5002, 0x08);
    cart.write_prg(0x5003, 0x18);
    assert_eq!(cart.read_prg_low(0x5015) & 0x01, 0x01);

    let mut non_zero = false;
    for _ in 0..64 {
        if cart.clock_expansion_audio().abs() > f32::EPSILON {
            non_zero = true;
            break;
        }
    }
    assert!(non_zero);

    cart.write_prg(0x5015, 0x00);
    assert_eq!(cart.read_prg_low(0x5015) & 0x03, 0x00);
}

#[test]
fn mapper_19_switches_prg_chr_and_chip_ram_port() {
    let mut cart = make_mapper19_cart();

    cart.write_prg(0xE000, 0x03);
    cart.write_prg(0xE800, 0x04);
    cart.write_prg(0xF000, 0x05);

    assert_eq!(cart.read_prg(0x8000), 3);
    assert_eq!(cart.read_prg(0xA000), 4);
    assert_eq!(cart.read_prg(0xC000), 5);
    assert_eq!(cart.read_prg(0xE000), 63);

    cart.write_prg(0x8000, 0x10);
    cart.write_prg(0x8800, 0x11);
    cart.write_prg(0x9000, 0x12);
    assert_eq!(cart.read_chr(0x0000), 0x90);
    assert_eq!(cart.read_chr(0x0400), 0x91);
    assert_eq!(cart.read_chr(0x0800), 0x92);

    cart.write_prg(0xF800, 0x40);
    cart.write_prg_low(0x4800, 0x5A);
    assert_eq!(cart.read_prg_low(0x4800), 0x5A);

    cart.write_prg(0xF800, 0xC0);
    cart.write_prg_low(0x4800, 0x11);
    assert_eq!(cart.read_prg_low(0x4800), 0x00);
    cart.write_prg(0xF800, 0xC0);
    assert_eq!(cart.read_prg_low(0x4800), 0x11);

    cart.write_prg_low(0x5000, 0x34);
    cart.write_prg_low(0x5800, 0x92);
    let snapshot = cart.snapshot_state();

    cart.write_prg_low(0x5000, 0x00);
    cart.write_prg_low(0x5800, 0x00);
    cart.restore_state(&snapshot);

    assert_eq!(cart.read_prg_low(0x5000), 0x34);
    assert_eq!(cart.read_prg_low(0x5800), 0x92);
}

#[test]
fn mapper_18_switches_prg_chr_and_prg_ram() {
    let mut cart = make_mapper18_cart();

    cart.write_prg(0x8000, 0x05);
    cart.write_prg(0x8001, 0x01);
    cart.write_prg(0x8002, 0x0A);
    cart.write_prg(0x8003, 0x02);
    cart.write_prg(0x9000, 0x03);
    cart.write_prg(0x9001, 0x03);

    assert_eq!(cart.read_prg(0x8000), 0x15);
    assert_eq!(cart.read_prg(0xA000), 0x2A);
    assert_eq!(cart.read_prg(0xC000), 0x33);
    assert_eq!(cart.read_prg(0xE000), 0x3F);

    cart.write_prg(0xA000, 0x04);
    cart.write_prg(0xA001, 0x02);
    cart.write_prg(0xD002, 0x0F);
    cart.write_prg(0xD003, 0x00);

    assert_eq!(cart.read_chr(0x0000), 0xA4);
    assert_eq!(cart.read_chr(0x1C00), 0x8F);

    cart.write_prg_ram(0x6001, 0x99);
    assert_eq!(cart.read_prg_ram(0x6001), 0x00);

    cart.write_prg(0x9002, 0x03);
    cart.write_prg_ram(0x6001, 0x5A);
    assert_eq!(cart.read_prg_ram(0x6001), 0x5A);

    let snapshot = cart.snapshot_state();
    cart.write_prg(0x8000, 0x00);
    cart.write_prg(0x8001, 0x00);
    cart.write_prg_ram(0x6001, 0x00);

    cart.restore_state(&snapshot);

    assert_eq!(cart.read_prg(0x8000), 0x15);
    assert_eq!(cart.read_prg_ram(0x6001), 0x5A);
    assert_eq!(cart.read_chr(0x0000), 0xA4);
}

#[test]
fn mapper_210_switches_prg_chr_and_namco175_ram() {
    let mut cart = make_mapper210_cart(false);

    cart.write_prg(0xE000, 0x03);
    cart.write_prg(0xE800, 0x04);
    cart.write_prg(0xF000, 0x05);

    assert_eq!(cart.read_prg(0x8000), 3);
    assert_eq!(cart.read_prg(0xA000), 4);
    assert_eq!(cart.read_prg(0xC000), 5);
    assert_eq!(cart.read_prg(0xE000), 63);

    cart.write_prg(0x8000, 0x10);
    cart.write_prg(0xB800, 0x17);
    assert_eq!(cart.read_chr(0x0000), 0x90);
    assert_eq!(cart.read_chr(0x1C00), 0x97);

    cart.write_prg_ram(0x6002, 0x55);
    assert_eq!(cart.read_prg_ram(0x6002), 0x00);

    cart.write_prg(0xC000, 0x01);
    cart.write_prg_ram(0x6002, 0x5A);
    assert_eq!(cart.read_prg_ram(0x6002), 0x5A);
    assert_eq!(cart.read_prg_ram(0x6802), 0x5A);

    let snapshot = cart.snapshot_state();
    cart.write_prg(0xE000, 0x00);
    cart.write_prg_ram(0x6002, 0x00);
    cart.restore_state(&snapshot);

    assert_eq!(cart.read_prg(0x8000), 3);
    assert_eq!(cart.read_prg_ram(0x6002), 0x5A);
}

#[test]
fn mapper_21_switches_prg_chr_and_supports_dual_vrc4_decode() {
    let mut cart = make_mapper21_cart();

    cart.write_prg(0x8000, 0x03);
    cart.write_prg(0xA000, 0x04);
    cart.write_prg(0x9000, 0x03);
    cart.write_prg(0xB000, 0x05);
    cart.write_prg(0xB002, 0x01);
    cart.write_prg(0xB040, 0x02);

    assert_eq!(cart.read_prg(0x8000), 3);
    assert_eq!(cart.read_prg(0xA000), 4);
    assert_eq!(cart.read_prg(0xC000), 62);
    assert_eq!(cart.read_prg(0xE000), 63);
    assert_eq!(cart.read_chr(0x0000), 0xA5);
    assert_eq!(cart.mirroring(), Mirroring::OneScreenUpper);

    cart.write_prg(0x9004, 0x02);
    assert_eq!(cart.read_prg(0x8000), 62);
    assert_eq!(cart.read_prg(0xC000), 3);
}

#[test]
fn mapper_22_switches_prg_chr_and_vrc2_mirroring() {
    let mut cart = make_mapper22_cart();

    cart.write_prg(0x8000, 0x03);
    cart.write_prg(0xA000, 0x04);
    cart.write_prg_ram(0x6000, 0x01);
    cart.write_prg(0x9000, 0x01);
    cart.write_prg(0xB000, 0x05);
    cart.write_prg(0xB002, 0x01);
    cart.write_prg(0xB001, 0x07);
    cart.write_prg(0xB003, 0x00);

    assert_eq!(cart.read_prg(0x8000), 3);
    assert_eq!(cart.read_prg(0xA000), 4);
    assert_eq!(cart.read_prg(0xC000), 30);
    assert_eq!(cart.read_prg(0xE000), 31);
    assert_eq!(cart.read_chr(0x0000), 0x8A);
    assert_eq!(cart.read_chr(0x0400), 0x83);
    assert_eq!(cart.read_prg_ram(0x6000), 0x61);
    assert_eq!(cart.mirroring(), Mirroring::Horizontal);

    cart.write_prg(0x9002, 0x00);
    assert_eq!(cart.mirroring(), Mirroring::Vertical);
}

#[test]
fn mapper_23_switches_prg_chr_and_wram() {
    let mut cart = make_mapper23_cart();

    cart.write_prg(0x8000, 0x05);
    cart.write_prg(0xA000, 0x06);

    assert_eq!(cart.read_prg(0x8000), 5);
    assert_eq!(cart.read_prg(0xA000), 6);
    assert_eq!(cart.read_prg(0xC000), 62);
    assert_eq!(cart.read_prg(0xE000), 63);

    cart.write_prg(0x9008, 0x03);
    cart.write_prg_ram(0x6001, 0x5A);
    assert_eq!(cart.read_prg_ram(0x6001), 0x5A);
    assert_eq!(cart.read_prg(0x8000), 62);
    assert_eq!(cart.read_prg(0xC000), 5);

    cart.write_prg(0xB000, 0x0A);
    cart.write_prg(0xB004, 0x01);
    cart.write_prg(0xB008, 0x03);
    cart.write_prg(0xB00C, 0x02);

    assert_eq!(cart.read_chr(0x0000), 0x9A);
    assert_eq!(cart.read_chr(0x0400), 0xA3);

    let snapshot = cart.snapshot_state();
    cart.write_prg(0x8000, 0x00);
    cart.write_prg(0x9008, 0x00);
    cart.write_prg_ram(0x6001, 0x00);
    cart.restore_state(&snapshot);

    assert_eq!(cart.read_prg(0xC000), 5);
    assert_eq!(cart.read_prg_ram(0x6001), 0x5A);
    assert_eq!(cart.read_chr(0x0400), 0xA3);
}

#[test]
fn mapper_24_26_switch_prg_chr_and_wram() {
    fn reg_addr(mapper: u8, reg: u16) -> u16 {
        if mapper == 26 {
            (reg & !0x0003) | (((reg & 0x0001) << 1) | ((reg & 0x0002) >> 1))
        } else {
            reg
        }
    }

    for mapper in [24_u8, 26] {
        let mut cart = make_mapper24_26_cart(mapper);

        cart.write_prg(reg_addr(mapper, 0x8000), 0x03);
        cart.write_prg(reg_addr(mapper, 0xC000), 0x05);
        cart.write_prg_ram(0x6002, 0x11);
        assert_eq!(cart.read_prg_ram(0x6002), 0x00);

        cart.write_prg(reg_addr(mapper, 0xB003), 0x84);
        cart.write_prg(reg_addr(mapper, 0xD000), 0x05);
        cart.write_prg(reg_addr(mapper, 0xD001), 0x06);

        assert_eq!(cart.read_prg(0x8000), 0x06, "mapper {mapper}");
        assert_eq!(cart.read_prg(0xA000), 0x07, "mapper {mapper}");
        assert_eq!(cart.read_prg(0xC000), 0x05, "mapper {mapper}");
        assert_eq!(cart.read_prg(0xE000), 0x3F, "mapper {mapper}");
        assert_eq!(cart.read_chr(0x0000), 0x85, "mapper {mapper}");
        assert_eq!(cart.read_chr(0x0400), 0x86, "mapper {mapper}");
        assert_eq!(cart.mirroring(), Mirroring::Horizontal, "mapper {mapper}");

        cart.write_prg_ram(0x6002, 0x5A);
        assert_eq!(cart.read_prg_ram(0x6002), 0x5A, "mapper {mapper}");
    }
}

#[test]
fn mapper_25_switches_prg_chr_and_wram_with_vrc4d_decode() {
    let mut cart = make_mapper25_cart(false);

    cart.write_prg(0x8008, 0x05);
    cart.write_prg(0xA004, 0x06);

    assert_eq!(cart.read_prg(0x8000), 5);
    assert_eq!(cart.read_prg(0xA000), 6);
    assert_eq!(cart.read_prg(0xC000), 62);
    assert_eq!(cart.read_prg(0xE000), 63);

    cart.write_prg(0x9004, 0x03);
    cart.write_prg_ram(0x6001, 0x5A);
    assert_eq!(cart.read_prg_ram(0x6001), 0x5A);
    assert_eq!(cart.read_prg(0x8000), 62);
    assert_eq!(cart.read_prg(0xC000), 5);

    cart.write_prg(0xB000, 0x0A);
    cart.write_prg(0xB008, 0x01);
    cart.write_prg(0xB004, 0x03);
    cart.write_prg(0xB00C, 0x02);

    assert_eq!(cart.read_chr(0x0000), 0x9A);
    assert_eq!(cart.read_chr(0x0400), 0xA3);

    let snapshot = cart.snapshot_state();
    cart.write_prg(0x8000, 0x00);
    cart.write_prg(0x9004, 0x00);
    cart.write_prg_ram(0x6001, 0x00);
    cart.restore_state(&snapshot);

    assert_eq!(cart.read_prg(0xC000), 5);
    assert_eq!(cart.read_prg_ram(0x6001), 0x5A);
    assert_eq!(cart.read_chr(0x0400), 0xA3);
}

#[test]
fn mapper_64_switches_prg_chr_modes_and_restores_state() {
    let mut cart = make_mapper64_cart();

    cart.write_prg(0x8000, 0x06);
    cart.write_prg(0x8001, 0x03);
    cart.write_prg(0x8000, 0x07);
    cart.write_prg(0x8001, 0x04);
    cart.write_prg(0x8000, 0x0F);
    cart.write_prg(0x8001, 0x05);

    assert_eq!(cart.read_prg(0x8000), 3);
    assert_eq!(cart.read_prg(0xA000), 4);
    assert_eq!(cart.read_prg(0xC000), 5);
    assert_eq!(cart.read_prg(0xE000), 31);

    cart.write_prg(0x8000, 0x46);
    assert_eq!(cart.read_prg(0x8000), 5);
    assert_eq!(cart.read_prg(0xC000), 3);

    cart.write_prg(0x8000, 0x00);
    cart.write_prg(0x8001, 0x0A);
    cart.write_prg(0x8000, 0x01);
    cart.write_prg(0x8001, 0x0C);
    assert_eq!(cart.read_chr(0x0000), 0x8A);
    assert_eq!(cart.read_chr(0x0400), 0x8B);
    assert_eq!(cart.read_chr(0x0800), 0x8C);
    assert_eq!(cart.read_chr(0x0C00), 0x8D);

    cart.write_prg(0x8000, 0x20);
    cart.write_prg(0x8001, 0x10);
    cart.write_prg(0x8000, 0x28);
    cart.write_prg(0x8001, 0x11);
    cart.write_prg(0x8000, 0x21);
    cart.write_prg(0x8001, 0x12);
    cart.write_prg(0x8000, 0x29);
    cart.write_prg(0x8001, 0x13);
    cart.write_prg(0xA000, 0x01);

    assert_eq!(cart.read_chr(0x0000), 0x90);
    assert_eq!(cart.read_chr(0x0400), 0x91);
    assert_eq!(cart.read_chr(0x0800), 0x92);
    assert_eq!(cart.read_chr(0x0C00), 0x93);
    assert_eq!(cart.mirroring(), Mirroring::Horizontal);

    cart.write_prg(0x8000, 0xA2);
    cart.write_prg(0x8001, 0x20);
    cart.write_prg(0x8000, 0xA3);
    cart.write_prg(0x8001, 0x21);
    cart.write_prg(0x8000, 0xA4);
    cart.write_prg(0x8001, 0x22);
    cart.write_prg(0x8000, 0xA5);
    cart.write_prg(0x8001, 0x23);

    assert_eq!(cart.read_chr(0x0000), 0xA0);
    assert_eq!(cart.read_chr(0x0400), 0xA1);
    assert_eq!(cart.read_chr(0x0800), 0xA2);
    assert_eq!(cart.read_chr(0x0C00), 0xA3);
    assert_eq!(cart.read_chr(0x1000), 0x90);
    assert_eq!(cart.read_chr(0x1400), 0x91);

    let snapshot = cart.snapshot_state();
    cart.write_prg(0x8000, 0x06);
    cart.write_prg(0x8001, 0x00);
    cart.write_prg(0xA000, 0x00);
    cart.restore_state(&snapshot);

    assert_eq!(cart.read_prg(0x8000), 3);
    assert_eq!(cart.read_prg(0xC000), 5);
    assert_eq!(cart.read_chr(0x0000), 0xA0);
    assert_eq!(cart.read_chr(0x1000), 0x90);
    assert_eq!(cart.mirroring(), Mirroring::Horizontal);
}

#[test]
fn mapper_11_switches_prg_and_chr_banks() {
    let mut cart = make_simple_bank_cart(11, 4, 16);

    cart.write_prg(0x8000, 0xA1);

    assert_eq!(cart.read_prg(0x8000), 1);
    assert_eq!(cart.read_prg(0xFFFF), 1);
    assert_eq!(cart.read_chr(0x0000), 0x4A);
    assert_eq!(cart.read_chr(0x1FFF), 0x4A);
}

#[test]
fn mapper_66_switches_prg_and_chr_banks() {
    let mut cart = make_simple_bank_cart(66, 4, 4);

    cart.write_prg(0x8000, 0x32);

    assert_eq!(cart.read_prg(0x8000), 3);
    assert_eq!(cart.read_prg(0xBFFF), 3);
    assert_eq!(cart.read_chr(0x0000), 0x42);
    assert_eq!(cart.read_chr(0x1FFF), 0x42);
}

#[test]
fn mapper_34_bnrom_switches_32k_prg_bank() {
    let mut cart = make_simple_bank_cart(34, 4, 1);
    cart.prg_rom[0] = 0xFF;

    cart.write_prg(0x8000, 0x02);

    assert_eq!(cart.read_prg(0x8000), 2);
    assert_eq!(cart.read_prg(0xFFFF), 2);
    assert_eq!(cart.read_chr(0x0000), 0x40);
}

#[test]
fn mapper_34_nina001_switches_prg_and_chr_halves() {
    let mut cart = make_nina001_cart();

    cart.write_prg_ram(0x7FFD, 0x03);
    cart.write_prg_ram(0x7FFE, 0x01);
    cart.write_prg_ram(0x7FFF, 0x02);

    assert_eq!(cart.read_prg(0x8000), 3);
    assert_eq!(cart.read_prg_ram(0x7FFD), 0x03);
    assert_eq!(cart.read_chr(0x0000), 0x51);
    assert_eq!(cart.read_chr(0x1000), 0x52);

    let snapshot = cart.snapshot_state();
    cart.prg_bank = 0;
    cart.chr_bank = 0;
    cart.chr_bank_1 = 1;
    cart.restore_state(&snapshot);

    assert_eq!(cart.prg_bank, 3);
    assert_eq!(cart.chr_bank, 1);
    assert_eq!(cart.chr_bank_1, 2);
}

#[test]
fn mapper_71_switches_low_prg_bank_and_mirroring() {
    let mut cart = make_camerica_cart();

    cart.write_prg(0xC000, 0x03);
    cart.write_prg(0x9000, 0x10);

    assert_eq!(cart.read_prg(0x8000), 3);
    assert_eq!(cart.read_prg(0xBFFF), 3);
    assert_eq!(cart.read_prg(0xC000), 7);
    assert_eq!(cart.mirroring(), Mirroring::OneScreenUpper);
}

#[test]
fn mapper_79_switches_32k_prg_and_chr_banks_via_low_address_latch() {
    let mut cart = make_simple_bank_cart(79, 2, 8);

    cart.write_prg(0x4100, 0x0B);

    assert_eq!(cart.read_prg(0x8000), 1);
    assert_eq!(cart.read_prg(0xFFFF), 1);
    assert_eq!(cart.read_chr(0x0000), 0x43);
}

#[test]
fn mapper_41_switches_outer_and_inner_chr_banks_with_reset() {
    let mut cart = make_simple_bank_cart(41, 8, 16);

    cart.write_prg_ram(0x600C, 0x00);
    cart.prg_rom[4 * 0x8000] = 0x03;
    cart.write_prg(0x8000, 0x03);

    assert_eq!(cart.prg_bank, 4);
    assert_eq!(cart.read_prg(0x8001), 4);
    assert_eq!(cart.read_chr(0x0000), 0x47);
    assert_eq!(cart.mirroring(), Mirroring::Vertical);

    let snapshot = cart.snapshot_state();

    cart.write_prg_ram(0x6008, 0x00);
    assert_eq!(cart.prg_bank, 0);
    assert_eq!(cart.read_chr(0x0000), 0x44);

    cart.restore_state(&snapshot);
    assert_eq!(cart.prg_bank, 4);
    assert_eq!(cart.read_chr(0x0000), 0x47);

    cart.on_reset();
    assert_eq!(cart.prg_bank, 0);
    assert_eq!(cart.read_chr(0x0000), 0x40);
    assert_eq!(cart.mirroring(), Mirroring::Vertical);
}

#[test]
fn mapper_57_switches_prg_chr_and_mirroring_from_address_latch() {
    let mut cart = make_mapper57_cart();

    cart.write_prg(0x8000, 0xAD);

    assert_eq!(cart.read_prg(0x8000), 5);
    assert_eq!(cart.read_prg(0xC000), 7);
    assert_eq!(cart.read_chr(0x0000), 0xA5);
    assert_eq!(cart.mirroring(), Mirroring::Horizontal);

    let snapshot = cart.snapshot_state();

    cart.write_prg(0x8800, 0x66);
    assert_eq!(cart.read_prg(0x8000), 1);
    assert_eq!(cart.read_chr(0x0000), 0xA2);
    assert_eq!(cart.mirroring(), Mirroring::Vertical);

    cart.restore_state(&snapshot);
    assert_eq!(cart.read_prg(0x8000), 5);
    assert_eq!(cart.read_chr(0x0000), 0xA5);
    assert_eq!(cart.mirroring(), Mirroring::Horizontal);
}

#[test]
fn mapper_63_latches_prg_mode_mirroring_and_chr_write_protect() {
    let mut cart = make_mapper63_cart();

    cart.write_prg(0xFFF6, 0x00);
    assert_eq!(cart.read_prg(0x8000), 2);
    assert_eq!(cart.read_prg(0xC000), 2);
    assert_eq!(cart.mirroring(), Mirroring::Horizontal);

    cart.write_chr(0x0123, 0x5A);
    assert_eq!(cart.read_chr(0x0123), 0x5A);

    let snapshot = cart.snapshot_state();

    cart.write_prg(0xFBE9, 0x00);
    assert_eq!(cart.read_prg(0x8000), 4);
    assert_eq!(cart.read_prg(0xC000), 5);
    assert_eq!(cart.mirroring(), Mirroring::Vertical);

    cart.write_chr(0x0123, 0x11);
    assert_eq!(cart.read_chr(0x0123), 0x5A);

    cart.restore_state(&snapshot);
    cart.write_chr(0x0123, 0x33);
    assert_eq!(cart.read_prg(0x8000), 2);
    assert_eq!(cart.read_prg(0xC000), 2);
    assert_eq!(cart.read_chr(0x0123), 0x33);
    assert_eq!(cart.mirroring(), Mirroring::Horizontal);
}

#[test]
fn mapper_99_switches_low_prg_chr_and_shared_ram() {
    let mut cart = make_mapper99_cart();

    assert_eq!(cart.read_prg(0x8000), 0);
    assert_eq!(cart.read_prg(0xA000), 1);
    assert_eq!(cart.read_prg(0xE000), 3);
    assert_eq!(cart.read_chr(0x0000), 0x90);

    cart.write_prg_low(0x4016, 0x04);
    assert_eq!(cart.read_prg(0x8000), 4);
    assert_eq!(cart.read_chr(0x0000), 0x91);
    assert_eq!(cart.read_prg(0xA000), 1);

    cart.write_prg_ram(0x6000, 0x12);
    cart.write_prg_ram(0x6800, 0x34);
    assert_eq!(cart.read_prg_ram(0x6000), 0x34);
    assert_eq!(cart.read_prg_ram(0x7800), 0x34);
}

#[test]
fn mapper_137_register_file_controls_prg_and_chr() {
    let mut cart = make_mapper137_cart();

    cart.write_prg(0x4100, 5);
    cart.write_prg(0x4101, 3);
    cart.write_prg(0x4100, 0);
    cart.write_prg(0x4101, 1);
    cart.write_prg(0x4100, 1);
    cart.write_prg(0x4101, 2);
    cart.write_prg(0x4100, 2);
    cart.write_prg(0x4101, 3);
    cart.write_prg(0x4100, 3);
    cart.write_prg(0x4101, 4);
    cart.write_prg(0x4100, 4);
    cart.write_prg(0x4101, 0x05);
    cart.write_prg(0x4100, 6);
    cart.write_prg(0x4101, 0x01);

    assert_eq!(cart.read_prg(0x8000), 3);
    assert_eq!(cart.read_chr(0x0000), 0xA1);
    assert_eq!(cart.read_chr(0x0400), 0xB2);
    assert_eq!(cart.read_chr(0x0800), 0xA3);
    assert_eq!(cart.read_chr(0x0C00), 0xBC);
    assert_eq!(cart.read_chr(0x1000), 0xBC);
    assert_eq!(cart.read_prg_low(0x4101), 0x01);
}

#[test]
fn mapper_150_register_file_controls_prg_chr_and_mirroring() {
    let mut cart = make_mapper150_cart();

    cart.write_prg(0x4100, 5);
    cart.write_prg(0x4101, 0x02);
    cart.write_prg(0x4100, 4);
    cart.write_prg(0x4101, 0x01);
    cart.write_prg(0x4100, 6);
    cart.write_prg(0x4101, 0x03);
    cart.write_prg(0x4100, 7);
    cart.write_prg(0x4101, 0x04);

    assert_eq!(cart.read_prg(0x8000), 2);
    assert_eq!(cart.read_prg(0xFFFF), 2);
    assert_eq!(cart.read_chr(0x0000), 0xB7);
    assert_eq!(cart.read_prg_low(0x4101), 0x04);
    assert_eq!(cart.mirroring(), Mirroring::Vertical);

    let snapshot = cart.snapshot_state();
    cart.write_prg(0x4100, 5);
    cart.write_prg(0x4101, 0x01);
    assert_eq!(cart.read_prg(0x8000), 1);

    cart.restore_state(&snapshot);
    assert_eq!(cart.read_prg(0x8000), 2);
    assert_eq!(cart.read_chr(0x0000), 0xB7);
    assert_eq!(cart.mirroring(), Mirroring::Vertical);
}

#[test]
fn mapper_151_aliases_vrc1_layout() {
    let mut cart = make_vrc1_cart(151);

    cart.write_prg(0x8000, 0x03);
    cart.write_prg(0xA000, 0x04);
    cart.write_prg(0xC000, 0x05);
    cart.write_prg(0x9000, 0x01);
    cart.write_prg(0xE000, 0x06);
    cart.write_prg(0xF000, 0x07);

    assert_eq!(cart.read_prg(0x8000), 3);
    assert_eq!(cart.read_prg(0xA000), 4);
    assert_eq!(cart.read_prg(0xC000), 5);
    assert_eq!(cart.read_prg(0xE000), 7);
    assert_eq!(cart.read_chr(0x0000), 0x66);
    assert_eq!(cart.read_chr(0x1000), 0x67);
    assert_eq!(cart.mirroring(), Mirroring::Horizontal);
}

#[test]
fn mapper_78_switches_prg_chr_and_one_screen_mirroring() {
    let mut cart = make_mapper78_cart(false);
    cart.prg_rom[0] = 0xFF;

    cart.write_prg(0x8000, 0x9A);

    assert_eq!(cart.read_prg(0x8000), 2);
    assert_eq!(cart.read_prg(0xC000), 7);
    assert_eq!(cart.read_chr(0x0000), 0x79);
    assert_eq!(cart.mirroring(), Mirroring::OneScreenUpper);
}

#[test]
fn mapper_77_switches_32k_prg_and_split_chr_ram() {
    let mut cart = make_mapper77_cart();
    cart.prg_rom[0] = 0xFF;

    cart.write_prg(0x8000, 0x21);

    assert_eq!(cart.read_prg(0x8000), 1);
    assert_eq!(cart.read_prg(0xFFFF), 1);
    assert_eq!(cart.read_chr(0x0000), 0x82);

    cart.write_chr(0x0800, 0x44);
    cart.write_chr(0x1FFF, 0x99);
    assert_eq!(cart.read_chr(0x0800), 0x44);
    assert_eq!(cart.read_chr(0x1FFF), 0x99);

    let snapshot = cart.snapshot_state();
    cart.write_prg(0x8000, 0x00);
    cart.write_chr(0x0800, 0x00);
    cart.restore_state(&snapshot);

    assert_eq!(cart.read_prg(0x8000), 1);
    assert_eq!(cart.read_chr(0x0000), 0x82);
    assert_eq!(cart.read_chr(0x0800), 0x44);
}

#[test]
fn mapper_78_header_variant_uses_horizontal_vertical_mirroring() {
    let mut cart = make_mapper78_cart(true);
    cart.prg_rom[0] = 0xFF;

    cart.write_prg(0x8000, 0x08);
    assert_eq!(cart.mirroring(), Mirroring::Vertical);

    cart.write_prg(0x8000, 0x00);
    assert_eq!(cart.mirroring(), Mirroring::Horizontal);
}

#[test]
fn mapper_94_switches_prg_bank_from_upper_bits() {
    let mut cart = make_uxrom_like_cart(94, 8, 1);
    cart.prg_rom[0] = 0xFF;

    cart.write_prg(0x8000, 0x14);

    assert_eq!(cart.read_prg(0x8000), 5);
    assert_eq!(cart.read_prg(0xC000), 7);
    assert_eq!(cart.read_chr(0x0000), 0x70);
}

#[test]
fn mapper_89_switches_prg_chr_and_one_screen_mirroring() {
    let mut cart = make_uxrom_like_cart(89, 8, 16);
    cart.prg_rom[0] = 0xFF;

    cart.write_prg(0x8000, 0x9D);

    assert_eq!(cart.read_prg(0x8000), 1);
    assert_eq!(cart.read_prg(0xC000), 7);
    assert_eq!(cart.read_chr(0x0000), 0x7D);
    assert_eq!(cart.mirroring(), Mirroring::OneScreenUpper);
}

#[test]
fn mapper_93_switches_prg_and_restores_chr_ram_enable_state() {
    let mut cart = make_uxrom_like_cart(93, 8, 1);
    cart.prg_rom[0] = 0xFF;

    cart.write_chr(0x0010, 0x44);
    assert_eq!(cart.read_chr(0x0010), 0x44);

    cart.write_prg(0x8000, 0x20);
    assert_eq!(cart.read_prg(0x8000), 2);
    assert_eq!(cart.read_chr(0x0010), 0xFF);

    let snapshot = cart.snapshot_state();

    cart.write_prg(0xC000, 0x21);
    assert_eq!(cart.read_chr(0x0010), 0x44);

    cart.restore_state(&snapshot);
    assert_eq!(cart.read_chr(0x0010), 0xFF);
}

#[test]
fn mapper_70_switches_prg_and_chr_banks() {
    let mut cart = make_uxrom_like_cart(70, 8, 16);
    cart.prg_rom[0] = 0xFF;

    cart.write_prg(0x8000, 0x21);

    assert_eq!(cart.read_prg(0x8000), 2);
    assert_eq!(cart.read_prg(0xC000), 7);
    assert_eq!(cart.read_chr(0x0000), 0x71);
    assert_eq!(cart.mirroring(), Mirroring::Horizontal);
}

#[test]
fn mapper_152_switches_prg_chr_and_mirroring() {
    let mut cart = make_uxrom_like_cart(152, 8, 16);
    cart.prg_rom[0] = 0xFF;

    cart.write_prg(0x8000, 0xB2);

    assert_eq!(cart.read_prg(0x8000), 3);
    assert_eq!(cart.read_prg(0xC000), 7);
    assert_eq!(cart.read_chr(0x0000), 0x72);
    assert_eq!(cart.mirroring(), Mirroring::OneScreenUpper);
}

#[test]
fn mapper_146_matches_mapper_79_latch_layout() {
    let mut cart = make_simple_bank_cart(146, 2, 8);

    cart.write_prg(0x5F00, 0x0C);

    assert_eq!(cart.read_prg(0x8000), 1);
    assert_eq!(cart.read_chr(0x0000), 0x44);
}

#[test]
fn mapper_148_switches_32k_prg_and_chr_banks_with_bus_conflicts() {
    let mut cart = make_simple_bank_cart(148, 2, 8);
    cart.prg_rom[0] = 0x09;

    cart.write_prg(0x8000, 0x0B);

    assert_eq!(cart.read_prg(0x8000), 1);
    assert_eq!(cart.read_prg(0xFFFF), 1);
    assert_eq!(cart.read_chr(0x0000), 0x41);
}

#[test]
fn mapper_180_switches_upper_prg_bank_only() {
    let mut cart = make_uxrom_like_cart(180, 8, 1);
    cart.prg_rom[1] = 0xFF;

    cart.write_prg(0xC001, 0x03);

    assert_eq!(cart.read_prg(0x8000), 0);
    assert_eq!(cart.read_prg(0xBFFF), 0);
    assert_eq!(cart.read_prg(0xC000), 3);
    assert_eq!(cart.read_prg(0xFFFF), 3);
}

#[test]
fn mapper_13_switches_upper_chr_ram_page_only() {
    let mut cart = make_split_chr_cart(13, 8, 3);
    cart.prg_rom[0] = 0xFF;

    assert_eq!(cart.read_chr(0x0000), 0x60);
    assert_eq!(cart.read_chr(0x1000), 0x63);

    cart.write_prg(0x8000, 0x01);

    assert_eq!(cart.read_chr(0x0000), 0x60);
    assert_eq!(cart.read_chr(0x1000), 0x61);
}

#[test]
fn mapper_15_handles_all_prg_modes_and_prg_ram() {
    let mut cart = make_mapper15_cart();

    cart.write_prg(0x8000, 0x02);
    assert_eq!(cart.read_prg(0x8000), 2);
    assert_eq!(cart.read_prg(0xC000), 3);
    assert_eq!(cart.mirroring(), Mirroring::Vertical);

    cart.write_prg(0x8001, 0x03);
    assert_eq!(cart.read_prg(0x8000), 3);
    assert_eq!(cart.read_prg(0xC000), 7);

    cart.write_prg(0x8002, 0x01);
    assert_eq!(cart.read_prg(0x8000), 1);
    assert_eq!(cart.read_prg(0xE000), 1);

    cart.write_prg(0x8003, 0x44);
    assert_eq!(cart.read_prg(0x8000), 4);
    assert_eq!(cart.read_prg(0xA000), 4);
    assert_eq!(cart.read_prg(0xC000), 4);
    assert_eq!(cart.mirroring(), Mirroring::Horizontal);

    cart.write_prg_ram(0x6000, 0xA5);
    assert_eq!(cart.read_prg_ram(0x6000), 0xA5);
}
