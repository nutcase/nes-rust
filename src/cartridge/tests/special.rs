use super::*;

#[test]
fn mapper_206_uses_namco108_bank_layout() {
    let mut cart = make_namco108_cart(206, 8, 16);

    cart.write_prg(0x8000, 0x06);
    cart.write_prg(0x8001, 0x03);
    cart.write_prg(0x8000, 0x07);
    cart.write_prg(0x8001, 0x04);
    cart.write_prg(0x8000, 0x00);
    cart.write_prg(0x8001, 0x06);
    cart.write_prg(0x8000, 0x01);
    cart.write_prg(0x8001, 0x08);
    cart.write_prg(0x8000, 0x02);
    cart.write_prg(0x8001, 0x0A);
    cart.write_prg(0x8000, 0x03);
    cart.write_prg(0x8001, 0x0B);
    cart.write_prg(0x8000, 0x04);
    cart.write_prg(0x8001, 0x0C);
    cart.write_prg(0x8000, 0x05);
    cart.write_prg(0x8001, 0x0D);

    assert_eq!(cart.read_prg(0x8000), 3);
    assert_eq!(cart.read_prg(0xA000), 4);
    assert_eq!(cart.read_prg(0xC000), 6);
    assert_eq!(cart.read_prg(0xE000), 7);
    assert_eq!(cart.read_chr(0x0000), 0x26);
    assert_eq!(cart.read_chr(0x0400), 0x27);
    assert_eq!(cart.read_chr(0x0800), 0x28);
    assert_eq!(cart.read_chr(0x1000), 0x2A);
    assert_eq!(cart.read_chr(0x1C00), 0x2D);
}

#[test]
fn mapper_208_switches_prg_protection_and_chr_banks() {
    let mut cart = make_mapper208_cart();

    assert_eq!(cart.read_prg(0x8000), 3);
    cart.write_prg(0x4800, 0x20);
    assert_eq!(cart.read_prg(0x8000), 0);
    assert_eq!(cart.mirroring(), Mirroring::Horizontal);

    cart.write_prg_ram(0x6800, 0x11);
    assert_eq!(cart.read_prg(0x8000), 3);
    assert_eq!(cart.mirroring(), Mirroring::Vertical);

    cart.write_prg(0x5000, 0x09);
    cart.write_prg(0x5800, 0xAA);
    cart.write_prg(0x5801, 0x55);
    assert_eq!(cart.read_prg_low(0x5800), 0xE3);
    assert_eq!(cart.read_prg_low(0x5801), 0x1C);

    cart.write_prg(0x8000, 0x00);
    cart.write_prg(0x8001, 0x06);
    cart.write_prg(0x8000, 0x02);
    cart.write_prg(0x8001, 0x05);
    assert_eq!(cart.read_chr(0x0000), 0x56);
    assert_eq!(cart.read_chr(0x0400), 0x57);
    assert_eq!(cart.read_chr(0x1000), 0x55);

    let snapshot = cart.snapshot_state();
    cart.write_prg(0x4800, 0x20);
    cart.write_prg(0x5000, 0x0A);
    cart.write_prg(0x5800, 0x00);
    cart.restore_state(&snapshot);

    assert_eq!(cart.read_prg(0x8000), 3);
    assert_eq!(cart.read_prg_low(0x5800), 0xE3);
    assert_eq!(cart.read_chr(0x1000), 0x55);
}

#[test]
fn mapper_250_uses_address_lines_for_register_select_and_data() {
    let mut cart = make_mmc3_mixed_chr_cart(250, 8, 16, 0);

    cart.write_prg(0x8006, 0xFF);
    cart.write_prg(0x8403, 0xAA);
    cart.write_prg(0x8007, 0x11);
    cart.write_prg(0x8404, 0x22);
    cart.write_prg(0x8000, 0xFE);
    cart.write_prg(0x8406, 0x55);
    cart.write_prg(0xA001, 0x00);

    assert_eq!(cart.read_prg(0x8000), 3);
    assert_eq!(cart.read_prg(0xA000), 4);
    assert_eq!(cart.read_prg(0xC000), 6);
    assert_eq!(cart.read_prg(0xE000), 7);
    assert_eq!(cart.read_chr(0x0000), 0x56);
    assert_eq!(cart.read_chr(0x0400), 0x57);
    assert_eq!(cart.mirroring(), Mirroring::Horizontal);

    let snapshot = cart.snapshot_state();
    cart.write_prg(0x8006, 0x00);
    cart.write_prg(0x8401, 0x00);
    cart.write_prg(0xA000, 0x00);
    cart.restore_state(&snapshot);

    assert_eq!(cart.read_prg(0x8000), 3);
    assert_eq!(cart.read_prg(0xA000), 4);
    assert_eq!(cart.read_prg(0xC000), 6);
    assert_eq!(cart.read_chr(0x0000), 0x56);
    assert_eq!(cart.read_chr(0x0400), 0x57);
    assert_eq!(cart.mirroring(), Mirroring::Horizontal);
}

#[test]
fn mapper_74_uses_chr_ram_for_banks_8_and_9() {
    let mut cart = make_mmc3_mixed_chr_cart(74, 8, 16, 0x0800);

    cart.write_prg(0x8000, 0x00);
    cart.write_prg(0x8001, 0x08);
    cart.write_prg(0x8000, 0x01);
    cart.write_prg(0x8001, 0x02);
    cart.write_prg(0x8000, 0x02);
    cart.write_prg(0x8001, 0x04);

    assert_eq!(cart.read_chr(0x0000), 0x00);
    assert_eq!(cart.read_chr(0x0400), 0x00);
    cart.write_chr(0x0000, 0xAA);
    cart.write_chr(0x0400, 0xBB);
    assert_eq!(cart.read_chr(0x0000), 0xAA);
    assert_eq!(cart.read_chr(0x0400), 0xBB);
    assert_eq!(cart.read_chr(0x1000), 0x54);

    cart.write_chr(0x1000, 0x11);
    assert_eq!(cart.read_chr(0x1000), 0x54);
}

#[test]
fn mapper_119_switches_between_chr_rom_and_chr_ram() {
    let mut cart = make_mmc3_mixed_chr_cart(119, 8, 32, 0x2000);

    cart.write_prg(0x8000, 0x00);
    cart.write_prg(0x8001, 0x40);
    cart.write_prg(0x8000, 0x01);
    cart.write_prg(0x8001, 0x03);
    cart.write_prg(0x8000, 0x02);
    cart.write_prg(0x8001, 0x05);

    assert_eq!(cart.read_chr(0x0000), 0x00);
    assert_eq!(cart.read_chr(0x0400), 0x00);
    cart.write_chr(0x0000, 0xC1);
    cart.write_chr(0x0400, 0xC2);
    assert_eq!(cart.read_chr(0x0000), 0xC1);
    assert_eq!(cart.read_chr(0x0400), 0xC2);

    assert_eq!(cart.read_chr(0x0800), 0x52);
    assert_eq!(cart.read_chr(0x0C00), 0x53);
    assert_eq!(cart.read_chr(0x1000), 0x55);
    cart.write_chr(0x1000, 0x99);
    assert_eq!(cart.read_chr(0x1000), 0x55);
}

#[test]
fn mapper_118_uses_chr_bank_bits_for_nametable_mapping() {
    let mut cart = make_mmc3_mixed_chr_cart(118, 8, 16, 0);
    let mut ppu = crate::ppu::Ppu::new();
    ppu.nametable[0][0] = 0x11;
    ppu.nametable[1][0] = 0x22;

    cart.write_prg(0x8000, 0x00);
    cart.write_prg(0x8001, 0x80);
    cart.write_prg(0x8000, 0x01);
    cart.write_prg(0x8001, 0x00);

    ppu.v = 0x2000;
    let _ = ppu.read_register(0x2007, Some(&cart));
    ppu.v = 0x2000;
    let _ = ppu.read_register(0x2007, Some(&cart));
    assert_eq!(ppu.read_register(0x2007, Some(&cart)), 0x22);

    ppu.v = 0x2800;
    let _ = ppu.read_register(0x2007, Some(&cart));
    ppu.v = 0x2800;
    let _ = ppu.read_register(0x2007, Some(&cart));
    assert_eq!(ppu.read_register(0x2007, Some(&cart)), 0x11);

    ppu.v = 0x2400;
    ppu.write_register(0x2007, 0x77, Some(&cart));
    assert_eq!(ppu.nametable[1][0], 0x77);
    assert_eq!(ppu.nametable[0][0], 0x11);
}

#[test]
fn mapper_192_uses_chr_ram_for_banks_8_through_11() {
    let mut cart = make_mmc3_mixed_chr_cart(192, 8, 16, 0x1000);

    cart.write_prg(0x8000, 0x00);
    cart.write_prg(0x8001, 0x08);
    cart.write_prg(0x8000, 0x01);
    cart.write_prg(0x8001, 0x0A);
    cart.write_prg(0x8000, 0x02);
    cart.write_prg(0x8001, 0x04);

    assert_eq!(cart.read_chr(0x0000), 0x00);
    assert_eq!(cart.read_chr(0x0400), 0x00);
    assert_eq!(cart.read_chr(0x0800), 0x00);
    assert_eq!(cart.read_chr(0x0C00), 0x00);
    cart.write_chr(0x0000, 0xA1);
    cart.write_chr(0x0400, 0xA2);
    cart.write_chr(0x0800, 0xA3);
    cart.write_chr(0x0C00, 0xA4);
    assert_eq!(cart.read_chr(0x0000), 0xA1);
    assert_eq!(cart.read_chr(0x0400), 0xA2);
    assert_eq!(cart.read_chr(0x0800), 0xA3);
    assert_eq!(cart.read_chr(0x0C00), 0xA4);
    assert_eq!(cart.read_chr(0x1000), 0x54);

    cart.write_chr(0x1000, 0x11);
    assert_eq!(cart.read_chr(0x1000), 0x54);
}

#[test]
fn mapper_194_uses_chr_ram_for_banks_0_and_1() {
    let mut cart = make_mmc3_mixed_chr_cart(194, 8, 16, 0x0800);

    cart.write_prg(0x8000, 0x00);
    cart.write_prg(0x8001, 0x00);
    cart.write_prg(0x8000, 0x01);
    cart.write_prg(0x8001, 0x02);
    cart.write_prg(0x8000, 0x02);
    cart.write_prg(0x8001, 0x04);

    assert_eq!(cart.read_chr(0x0000), 0x00);
    assert_eq!(cart.read_chr(0x0400), 0x00);
    cart.write_chr(0x0000, 0xB1);
    cart.write_chr(0x0400, 0xB2);
    assert_eq!(cart.read_chr(0x0000), 0xB1);
    assert_eq!(cart.read_chr(0x0400), 0xB2);

    assert_eq!(cart.read_chr(0x0800), 0x52);
    assert_eq!(cart.read_chr(0x1000), 0x54);
    cart.write_chr(0x0800, 0xCC);
    assert_eq!(cart.read_chr(0x0800), 0x52);
}

#[test]
fn mapper_191_switches_fixed_prg_banks_and_chr_mode() {
    let mut cart = make_mmc3_mixed_chr_cart(191, 32, 256, 0x0800);

    cart.write_prg(0x8000, 0x06);
    cart.write_prg(0x8001, 0x03);
    cart.write_prg(0x8000, 0x07);
    cart.write_prg(0x8001, 0x04);
    cart.write_prg(0x8000, 0x02);
    cart.write_prg(0x8001, 0x05);

    assert_eq!(cart.read_prg(0x8000), 3);
    assert_eq!(cart.read_prg(0xA000), 4);
    assert_eq!(cart.read_prg(0xC000), 24);
    assert_eq!(cart.read_prg(0xE000), 25);
    assert_eq!(cart.read_chr(0x1000), 0xD5);

    cart.write_prg(0x90AA, 0x03);
    cart.write_prg(0x8000, 0x02);
    cart.write_prg(0x8001, 0x80);

    assert_eq!(cart.read_prg(0xC000), 30);
    assert_eq!(cart.read_prg(0xE000), 31);
    assert_eq!(cart.read_chr(0x1000), 0x00);
    cart.write_chr(0x1000, 0x5E);
    assert_eq!(cart.read_chr(0x1000), 0x5E);

    let snapshot = cart.snapshot_state();
    cart.write_prg(0x90AA, 0x00);
    cart.write_prg(0x8000, 0x02);
    cart.write_prg(0x8001, 0x05);
    cart.restore_state(&snapshot);

    assert_eq!(cart.read_prg(0xC000), 30);
    assert_eq!(cart.read_prg(0xE000), 31);
    assert_eq!(cart.read_chr(0x1000), 0x5E);
}

#[test]
fn mapper_195_switches_chr_ram_windows_via_ppu_writes() {
    let mut cart = make_mmc3_mixed_chr_cart(195, 8, 256, 0x2000);

    cart.write_prg(0x8000, 0x00);
    cart.write_prg(0x8001, 0x28);
    cart.write_prg(0x8000, 0x01);
    cart.write_prg(0x8001, 0x2A);
    cart.write_prg(0x8000, 0x02);
    cart.write_prg(0x8001, 0x10);

    assert_eq!(cart.read_chr(0x0000), 0x00);
    assert_eq!(cart.read_chr(0x0400), 0x00);
    cart.write_chr(0x0000, 0x61);
    cart.write_chr(0x0400, 0x62);
    assert_eq!(cart.read_chr(0x0000), 0x61);
    assert_eq!(cart.read_chr(0x0400), 0x62);
    assert_eq!(cart.read_chr(0x1000), 0x60);

    cart.write_prg(0x8000, 0x00);
    cart.write_prg(0x8001, 0x0A);
    assert_eq!(cart.read_chr(0x0000), 0x5A);
    cart.write_chr(0x0000, 0x00);
    cart.write_chr(0x0000, 0x71);
    cart.write_chr(0x0400, 0x72);
    assert_eq!(cart.read_chr(0x0000), 0x71);
    assert_eq!(cart.read_chr(0x0400), 0x72);
    assert_eq!(cart.read_chr(0x0800), 0x7A);

    let snapshot = cart.snapshot_state();

    cart.write_prg(0x8000, 0x02);
    cart.write_prg(0x8001, 0xCA);
    cart.write_chr(0x1000, 0x00);
    assert_eq!(cart.read_chr(0x0000), 0x5A);

    cart.restore_state(&snapshot);
    assert_eq!(cart.read_chr(0x0000), 0x71);
    assert_eq!(cart.read_chr(0x0400), 0x72);
}

#[test]
fn mapper_33_switches_prg_chr_and_mirroring() {
    let mut cart = make_taito_tc0190_cart();

    cart.write_prg(0x8000, 0x43);
    cart.write_prg(0x8001, 0x05);
    cart.write_prg(0x8002, 0x04);
    cart.write_prg(0x8003, 0x06);
    cart.write_prg(0xA000, 0x0A);
    cart.write_prg(0xA001, 0x0B);
    cart.write_prg(0xA002, 0x0C);
    cart.write_prg(0xA003, 0x0D);

    assert_eq!(cart.read_prg(0x8000), 3);
    assert_eq!(cart.read_prg(0xA000), 5);
    assert_eq!(cart.read_prg(0xC000), 14);
    assert_eq!(cart.read_prg(0xE000), 15);
    assert_eq!(cart.read_chr(0x0000), 0x48);
    assert_eq!(cart.read_chr(0x0400), 0x49);
    assert_eq!(cart.read_chr(0x0800), 0x4C);
    assert_eq!(cart.read_chr(0x0C00), 0x4D);
    assert_eq!(cart.read_chr(0x1000), 0x4A);
    assert_eq!(cart.read_chr(0x1400), 0x4B);
    assert_eq!(cart.read_chr(0x1800), 0x4C);
    assert_eq!(cart.read_chr(0x1C00), 0x4D);
    assert_eq!(cart.mirroring(), Mirroring::Horizontal);

    let snapshot = cart.snapshot_state();
    cart.write_prg(0x8000, 0x01);
    cart.write_prg(0x8002, 0x00);
    cart.restore_state(&snapshot);

    assert_eq!(cart.read_prg(0x8000), 3);
    assert_eq!(cart.read_chr(0x0000), 0x48);
    assert_eq!(cart.mirroring(), Mirroring::Horizontal);
}

#[test]
fn mapper_80_switches_prg_chr_mirroring_and_internal_ram() {
    let mut cart = make_taito_x1005_cart();

    cart.write_prg_ram(0x7EF0, 0x03);
    cart.write_prg_ram(0x7EF1, 0x04);
    cart.write_prg_ram(0x7EF2, 0x0A);
    cart.write_prg_ram(0x7EF3, 0x0B);
    cart.write_prg_ram(0x7EF4, 0x0C);
    cart.write_prg_ram(0x7EF5, 0x0D);
    cart.write_prg_ram(0x7EF6, 0x01);
    cart.write_prg_ram(0x7EF8, 0x02);
    cart.write_prg_ram(0x7EF9, 0x03);
    cart.write_prg_ram(0x7EFA, 0x0C);

    assert_eq!(cart.read_prg(0x8000), 2);
    assert_eq!(cart.read_prg(0xA000), 3);
    assert_eq!(cart.read_prg(0xC000), 12);
    assert_eq!(cart.read_prg(0xE000), 15);
    assert_eq!(cart.read_chr(0x0000), 0x86);
    assert_eq!(cart.read_chr(0x0400), 0x87);
    assert_eq!(cart.read_chr(0x0800), 0x88);
    assert_eq!(cart.read_chr(0x0C00), 0x89);
    assert_eq!(cart.read_chr(0x1000), 0x8A);
    assert_eq!(cart.read_chr(0x1400), 0x8B);
    assert_eq!(cart.read_chr(0x1800), 0x8C);
    assert_eq!(cart.read_chr(0x1C00), 0x8D);
    assert_eq!(cart.mirroring(), Mirroring::Vertical);

    assert_eq!(cart.read_prg_ram(0x7F20), 0x00);
    cart.write_prg_ram(0x7F20, 0x5A);
    assert_eq!(cart.read_prg_ram(0x7F20), 0x5A);
    assert_eq!(cart.read_prg_ram(0x7FA0), 0x5A);

    let snapshot = cart.snapshot_state();
    cart.write_prg_ram(0x7EF8, 0x00);
    cart.write_prg_ram(0x7EFA, 0x00);
    cart.write_prg_ram(0x7F20, 0x00);
    cart.restore_state(&snapshot);

    assert_eq!(cart.read_prg(0xC000), 12);
    assert_eq!(cart.read_prg_ram(0x7FA0), 0x5A);
    assert_eq!(cart.read_chr(0x1000), 0x8A);
    assert_eq!(cart.mirroring(), Mirroring::Vertical);
}

#[test]
fn mapper_82_switches_prg_chr_and_segmented_ram() {
    let mut cart = make_taito_x1017_cart();

    assert_eq!(cart.read_prg_ram(0x6000), 0x00);
    assert_eq!(cart.read_prg_ram(0x6800), 0x00);
    assert_eq!(cart.read_prg_ram(0x7000), 0x00);

    cart.write_prg_ram(0x7EF0, 0x01);
    cart.write_prg_ram(0x7EF1, 0x02);
    cart.write_prg_ram(0x7EF2, 0x0A);
    cart.write_prg_ram(0x7EF3, 0x0B);
    cart.write_prg_ram(0x7EF4, 0x0C);
    cart.write_prg_ram(0x7EF5, 0x0D);
    cart.write_prg_ram(0x7EF6, 0x03);
    cart.write_prg_ram(0x7EF7, 0xCA);
    cart.write_prg_ram(0x7EF8, 0x69);
    cart.write_prg_ram(0x7EF9, 0x84);
    cart.write_prg_ram(0x7EFA, 0x0C);
    cart.write_prg_ram(0x7EFB, 0x10);
    cart.write_prg_ram(0x7EFC, 0x14);

    assert_eq!(cart.read_prg(0x8000), 3);
    assert_eq!(cart.read_prg(0xA000), 4);
    assert_eq!(cart.read_prg(0xC000), 5);
    assert_eq!(cart.read_prg(0xE000), 15);
    assert_eq!(cart.read_chr(0x0000), 0x9A);
    assert_eq!(cart.read_chr(0x0400), 0x9B);
    assert_eq!(cart.read_chr(0x0800), 0x9C);
    assert_eq!(cart.read_chr(0x0C00), 0x9D);
    assert_eq!(cart.read_chr(0x1000), 0x92);
    assert_eq!(cart.read_chr(0x1400), 0x93);
    assert_eq!(cart.read_chr(0x1800), 0x94);
    assert_eq!(cart.read_chr(0x1C00), 0x95);
    assert_eq!(cart.mirroring(), Mirroring::Vertical);

    cart.write_prg_ram(0x6000, 0xA1);
    cart.write_prg_ram(0x6800, 0xB2);
    cart.write_prg_ram(0x7000, 0xC3);
    assert_eq!(cart.read_prg_ram(0x6000), 0xA1);
    assert_eq!(cart.read_prg_ram(0x6800), 0xB2);
    assert_eq!(cart.read_prg_ram(0x7000), 0xC3);

    let snapshot = cart.snapshot_state();
    cart.write_prg_ram(0x7EF7, 0x00);
    cart.write_prg_ram(0x7EFA, 0x00);
    cart.write_prg_ram(0x6000, 0x00);
    cart.restore_state(&snapshot);

    assert_eq!(cart.read_prg(0x8000), 3);
    assert_eq!(cart.read_prg_ram(0x6000), 0xA1);
    assert_eq!(cart.read_chr(0x0000), 0x9A);
    assert_eq!(cart.mirroring(), Mirroring::Vertical);
}

#[test]
fn mapper_207_uses_chr_bank_bits_for_nametable_mapping() {
    let mut cart = make_taito_x1005_cart();
    let mut ppu = crate::ppu::Ppu::new();
    cart.mapper = 207;
    ppu.nametable[0][0] = 0x11;
    ppu.nametable[1][0] = 0x22;

    cart.write_prg_ram(0x7EF0, 0x81);
    cart.write_prg_ram(0x7EF1, 0x02);

    assert_eq!(cart.read_chr(0x0000), 0x82);
    assert_eq!(cart.read_chr(0x0400), 0x83);
    assert_eq!(cart.read_chr(0x0800), 0x84);
    assert_eq!(cart.read_chr(0x0C00), 0x85);
    assert_eq!(cart.mirroring(), Mirroring::HorizontalSwapped);

    ppu.v = 0x2000;
    let _ = ppu.read_register(0x2007, Some(&cart));
    ppu.v = 0x2000;
    let _ = ppu.read_register(0x2007, Some(&cart));
    assert_eq!(ppu.read_register(0x2007, Some(&cart)), 0x22);

    ppu.v = 0x2800;
    let _ = ppu.read_register(0x2007, Some(&cart));
    ppu.v = 0x2800;
    let _ = ppu.read_register(0x2007, Some(&cart));
    assert_eq!(ppu.read_register(0x2007, Some(&cart)), 0x11);

    ppu.v = 0x2400;
    ppu.write_register(0x2007, 0x77, Some(&cart));
    assert_eq!(ppu.nametable[1][0], 0x77);
    assert_eq!(ppu.nametable[0][0], 0x11);
}

#[test]
fn mapper_246_switches_prg_chr_and_vector_reads() {
    let mut cart = make_mapper246_cart();

    cart.write_prg_ram(0x6000, 0x01);
    cart.write_prg_ram(0x6001, 0x02);
    cart.write_prg_ram(0x6002, 0x03);
    cart.write_prg_ram(0x6003, 0x04);
    cart.write_prg_ram(0x6004, 0x05);
    cart.write_prg_ram(0x6005, 0x06);
    cart.write_prg_ram(0x6006, 0x07);
    cart.write_prg_ram(0x6007, 0x08);

    assert_eq!(cart.read_prg(0x8000), 1);
    assert_eq!(cart.read_prg(0xA000), 2);
    assert_eq!(cart.read_prg(0xC000), 3);
    assert_eq!(cart.read_prg(0xE000), 4);
    assert_eq!(cart.read_prg(0xFFFC), 20);
    assert_eq!(cart.read_chr(0x0000), 0x45);
    assert_eq!(cart.read_chr(0x0800), 0x46);
    assert_eq!(cart.read_chr(0x1000), 0x47);
    assert_eq!(cart.read_chr(0x1800), 0x48);

    cart.write_prg_ram(0x6800, 0xA5);
    assert_eq!(cart.read_prg_ram(0x6800), 0xA5);

    let snapshot = cart.snapshot_state();
    cart.write_prg_ram(0x6003, 0x00);
    cart.restore_state(&snapshot);
    assert_eq!(cart.read_prg(0xE000), 4);
    assert_eq!(cart.read_prg(0xFFFC), 20);
}

#[test]
fn mapper_236_chr_rom_variant_switches_prg_chr_and_modes() {
    let mut cart = make_mapper236_cart(false);

    cart.write_prg(0x801A, 0);
    assert_eq!(cart.read_chr(0x0000), 0x8A);
    assert_eq!(cart.mirroring(), Mirroring::Horizontal);

    cart.write_prg(0xC00B, 0);
    assert_eq!(cart.read_prg(0x8000), 11);
    assert_eq!(cart.read_prg(0xC000), 15);

    cart.write_prg(0xC02D, 0);
    assert_eq!(cart.read_prg(0x8000), 12);
    assert_eq!(cart.read_prg(0xC000), 13);

    let snapshot = cart.snapshot_state();
    cart.write_prg(0x8001, 0);
    cart.write_prg(0xC003, 0);
    cart.restore_state(&snapshot);

    assert_eq!(cart.read_prg(0x8000), 12);
    assert_eq!(cart.read_prg(0xC000), 13);
    assert_eq!(cart.read_chr(0x0000), 0x8A);
    assert_eq!(cart.mirroring(), Mirroring::Horizontal);
}

#[test]
fn mapper_236_chr_ram_variant_switches_outer_and_inner_prg_banks() {
    let mut cart = make_mapper236_cart(true);

    cart.write_prg(0x8015, 0);
    cart.write_prg(0xC002, 0);
    assert_eq!(cart.read_prg(0x8000), 42);
    assert_eq!(cart.read_prg(0xC000), 47);
    assert_eq!(cart.mirroring(), Mirroring::Horizontal);

    cart.write_chr(0x0123, 0x5A);
    assert_eq!(cart.read_chr(0x0123), 0x5A);

    let snapshot = cart.snapshot_state();
    cart.write_prg(0xC026, 0);
    assert_eq!(cart.read_prg(0x8000), 46);
    assert_eq!(cart.read_prg(0xC000), 47);

    cart.write_prg(0x8000, 0);
    cart.write_chr(0x0123, 0x00);
    cart.restore_state(&snapshot);

    assert_eq!(cart.read_prg(0x8000), 42);
    assert_eq!(cart.read_prg(0xC000), 47);
    assert_eq!(cart.read_chr(0x0123), 0x5A);
    assert_eq!(cart.mirroring(), Mirroring::Horizontal);
}

#[test]
fn mapper_221_switches_prg_modes_and_write_protects_chr_ram() {
    let mut cart = make_mapper221_cart();

    cart.write_prg(0x8005, 0);
    cart.write_prg(0xC003, 0);
    assert_eq!(cart.read_prg(0x8000), 11);
    assert_eq!(cart.read_prg(0xC000), 11);
    assert_eq!(cart.mirroring(), Mirroring::Horizontal);

    cart.write_prg(0x8006, 0);
    cart.write_prg(0xC005, 0);
    assert_eq!(cart.read_prg(0x8000), 12);
    assert_eq!(cart.read_prg(0xC000), 13);
    assert_eq!(cart.mirroring(), Mirroring::Vertical);

    cart.write_prg(0x8107, 0);
    cart.write_prg(0xC002, 0);
    assert_eq!(cart.read_prg(0x8000), 10);
    assert_eq!(cart.read_prg(0xC000), 15);
    assert_eq!(cart.mirroring(), Mirroring::Horizontal);

    cart.write_chr(0x0010, 0x5A);
    cart.write_prg(0xC00A, 0);
    cart.write_chr(0x0010, 0x00);
    assert_eq!(cart.read_chr(0x0010), 0x5A);

    let snapshot = cart.snapshot_state();
    cart.write_prg(0x8006, 0);
    cart.write_prg(0xC001, 0);
    cart.write_chr(0x0010, 0x11);
    assert_eq!(cart.read_chr(0x0010), 0x11);

    cart.restore_state(&snapshot);
    assert_eq!(cart.read_prg(0x8000), 10);
    assert_eq!(cart.read_prg(0xC000), 15);
    assert_eq!(cart.read_chr(0x0010), 0x5A);

    cart.write_chr(0x0010, 0x33);
    assert_eq!(cart.read_chr(0x0010), 0x5A);
}

#[test]
fn mapper_243_uses_indexed_register_file_for_prg_chr_and_mirroring() {
    let mut cart = make_simple_bank_cart(243, 4, 16);

    cart.write_prg(0x4100, 0x05);
    cart.write_prg(0x4101, 0x02);
    cart.write_prg(0x4100, 0x02);
    cart.write_prg(0x4101, 0x01);
    cart.write_prg(0x4100, 0x04);
    cart.write_prg(0x4101, 0x01);
    cart.write_prg(0x4100, 0x06);
    cart.write_prg(0x4101, 0x02);
    cart.write_prg(0x4100, 0x07);
    cart.write_prg(0x4101, 0x00);

    cart.write_prg(0x4100, 0x05);
    assert_eq!(cart.read_prg_low(0x4101), 0x02);
    assert_eq!(cart.read_prg(0x8000), 2);
    assert_eq!(cart.read_prg(0xFFFF), 2);
    assert_eq!(cart.read_chr(0x0000), 0x4B);
    assert_eq!(cart.mirroring(), Mirroring::ThreeScreenLower);

    let snapshot = cart.snapshot_state();
    cart.write_prg(0x4100, 0x07);
    cart.write_prg(0x4101, 0x06);
    cart.restore_state(&snapshot);

    assert_eq!(cart.read_prg_low(0x4101), 0x02);
    assert_eq!(cart.read_prg(0x8000), 2);
    assert_eq!(cart.read_chr(0x0000), 0x4B);
    assert_eq!(cart.mirroring(), Mirroring::ThreeScreenLower);
}

#[test]
fn mapper_245_uses_chr_register_high_bit_for_prg_bank_group() {
    let mut cart = make_mapper245_cart();

    cart.write_prg(0x8000, 0x00);
    cart.write_prg(0x8001, 0x02);
    cart.write_prg(0x8000, 0x06);
    cart.write_prg(0x8001, 0x05);
    cart.write_prg(0x8000, 0x07);
    cart.write_prg(0x8001, 0x06);

    assert_eq!(cart.read_prg(0x8000), 37);
    assert_eq!(cart.read_prg(0xA000), 38);
    assert_eq!(cart.read_prg(0xC000), 62);
    assert_eq!(cart.read_prg(0xE000), 63);

    cart.write_chr(0x0123, 0x5A);
    assert_eq!(cart.read_chr(0x0123), 0x5A);
    cart.write_prg_ram(0x6000, 0xA5);
    assert_eq!(cart.read_prg_ram(0x6000), 0xA5);
}

#[test]
fn mapper_68_switches_prg_chr_nametables_and_prg_ram() {
    let mut cart = make_sunsoft4_cart();

    cart.write_prg(0x8000, 0x04);
    cart.write_prg(0x9000, 0x05);
    cart.write_prg(0xA000, 0x06);
    cart.write_prg(0xB000, 0x07);
    cart.write_prg(0xC000, 0x02);
    cart.write_prg(0xD000, 0x03);
    cart.write_prg(0xE000, 0x11);
    cart.write_prg(0xF000, 0x12);

    assert_eq!(cart.read_prg(0x8000), 2);
    assert_eq!(cart.read_chr(0x0000), 8);
    assert_eq!(cart.read_chr(0x0800), 10);
    assert_eq!(cart.read_chr(0x1000), 12);
    assert_eq!(cart.read_chr(0x1800), 14);
    assert_eq!(cart.mirroring(), Mirroring::Horizontal);
    assert!(!cart.nametable_writes_to_internal_vram());

    assert_eq!(cart.read_nametable_byte(0, 0, &[[0; 1024]; 2]), 0x82);
    assert_eq!(cart.read_nametable_byte(1, 0, &[[0; 1024]; 2]), 0x83);

    assert_eq!(cart.read_prg_ram(0x6000), 0x00);
    cart.write_prg_ram(0x6000, 0x5A);
    assert_eq!(cart.read_prg_ram(0x6000), 0x5A);

    let snapshot = cart.snapshot_state();
    cart.write_prg(0x8000, 0x00);
    cart.write_prg(0xC000, 0x00);
    cart.write_prg(0xE000, 0x02);
    cart.write_prg(0xF000, 0x00);

    cart.restore_state(&snapshot);
    assert_eq!(cart.read_prg(0x8000), 2);
    assert_eq!(cart.read_chr(0x1000), 12);
    assert_eq!(cart.read_nametable_byte(0, 0, &[[0; 1024]; 2]), 0x82);
    assert_eq!(cart.mirroring(), Mirroring::Horizontal);
    assert_eq!(cart.read_prg_ram(0x6000), 0x5A);
}

#[test]
fn mapper_68_ppu_reads_chr_rom_nametables() {
    let mut cart = make_sunsoft4_cart();
    let mut ppu = crate::ppu::Ppu::new();
    ppu.nametable[0][0] = 0x11;
    ppu.nametable[1][0] = 0x22;

    cart.write_prg(0xC000, 0x02);
    cart.write_prg(0xD000, 0x03);
    cart.write_prg(0xE000, 0x10);

    ppu.v = 0x2000;
    let _ = ppu.read_register(0x2007, Some(&cart));
    ppu.v = 0x2000;
    let _ = ppu.read_register(0x2007, Some(&cart));
    let rom_nt0 = ppu.read_register(0x2007, Some(&cart));
    assert_eq!(rom_nt0, 0x82);

    ppu.v = 0x2400;
    let _ = ppu.read_register(0x2007, Some(&cart));
    ppu.v = 0x2400;
    let _ = ppu.read_register(0x2007, Some(&cart));
    let rom_nt1 = ppu.read_register(0x2007, Some(&cart));
    assert_eq!(rom_nt1, 0x83);

    ppu.v = 0x2000;
    ppu.write_register(0x2007, 0x99, Some(&cart));
    assert_eq!(ppu.nametable[0][0], 0x11);
}
