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
