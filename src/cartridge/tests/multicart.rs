use super::*;

#[test]
fn mapper_240_switches_prg_chr_and_exposes_prg_ram() {
    let mut cart = make_simple_bank_cart(240, 4, 4);
    cart.prg_ram = vec![0; 0x2000];

    cart.write_prg(0x4800, 0x21);
    assert_eq!(cart.read_prg(0x8000), 2);
    assert_eq!(cart.read_chr(0x0000), 0x41);

    cart.write_prg(0x4100, 0x32);
    assert_eq!(cart.read_prg(0x8000), 3);
    assert_eq!(cart.read_chr(0x0000), 0x42);

    cart.write_prg_ram(0x6000, 0xA5);
    assert_eq!(cart.read_prg_ram(0x6000), 0xA5);
}

#[test]
fn mapper_213_matches_mapper_58_address_latch_behavior() {
    let mut cart = make_uxrom_like_cart(213, 16, 8);

    cart.write_prg(0x80DA, 0);

    assert_eq!(cart.read_prg(0x8000), 2);
    assert_eq!(cart.read_prg(0xC000), 2);
    assert_eq!(cart.read_chr(0x0000), 0x73);
    assert_eq!(cart.mirroring(), Mirroring::Horizontal);

    let snapshot = cart.snapshot_state();
    cart.write_prg(0x8000, 0);
    cart.restore_state(&snapshot);

    assert_eq!(cart.read_prg(0x8000), 2);
    assert_eq!(cart.read_prg(0xC000), 2);
    assert_eq!(cart.read_chr(0x0000), 0x73);
}

#[test]
fn mapper_241_switches_32k_prg_from_4800_window_and_exposes_wram() {
    let mut cart = make_simple_bank_cart(241, 4, 1);
    cart.prg_ram = vec![0; 0x2000];

    cart.write_prg(0x4800, 0x02);
    assert_eq!(cart.read_prg(0x8000), 2);
    assert_eq!(cart.read_prg(0xFFFF), 2);
    assert_eq!(cart.read_chr(0x0000), 0x40);

    cart.write_prg_ram(0x6000, 0x5A);
    assert_eq!(cart.read_prg_ram(0x6000), 0x5A);
}

#[test]
fn mapper_230_toggles_between_multicart_and_contra_modes_on_reset() {
    let mut cart = make_mapper230_cart();

    cart.on_reset();
    cart.write_prg(0x8000, 0x23);
    assert_eq!(cart.read_prg(0x8000), 11);
    assert_eq!(cart.read_prg(0xC000), 11);
    assert_eq!(cart.mirroring(), Mirroring::Horizontal);

    let snapshot = cart.snapshot_state();

    cart.on_reset();
    cart.write_prg(0x8000, 0x05);
    assert_eq!(cart.read_prg(0x8000), 5);
    assert_eq!(cart.read_prg(0xC000), 7);
    assert_eq!(cart.mirroring(), Mirroring::Vertical);

    cart.restore_state(&snapshot);
    assert_eq!(cart.read_prg(0x8000), 11);
    assert_eq!(cart.read_prg(0xC000), 11);
    assert_eq!(cart.mirroring(), Mirroring::Horizontal);
}

#[test]
fn mapper_200_latches_mirrored_16k_prg_chr_and_mirroring_from_address() {
    let mut cart = make_uxrom_like_cart(200, 16, 16);

    cart.write_prg(0x800B, 0);

    assert_eq!(cart.read_prg(0x8000), 11);
    assert_eq!(cart.read_prg(0xC000), 11);
    assert_eq!(cart.read_chr(0x0000), 0x7B);
    assert_eq!(cart.read_chr(0x1FFF), 0x7B);
    assert_eq!(cart.mirroring(), Mirroring::Horizontal);

    cart.write_prg(0x8002, 0);
    assert_eq!(cart.read_prg(0x8000), 2);
    assert_eq!(cart.mirroring(), Mirroring::Vertical);
}

#[test]
fn mapper_201_uses_low_address_byte_for_prg_and_chr_bank() {
    let mut cart = make_simple_bank_cart(201, 8, 8);

    cart.write_prg(0x80C5, 0);

    assert_eq!(cart.read_prg(0x8000), 5);
    assert_eq!(cart.read_prg(0xFFFF), 5);
    assert_eq!(cart.read_chr(0x0000), 0x45);
    assert_eq!(cart.read_chr(0x1FFF), 0x45);
}

#[test]
fn mapper_202_switches_between_mirrored_16k_and_32k_modes() {
    let mut cart = make_uxrom_like_cart(202, 16, 8);

    cart.write_prg(0x800C, 0);
    assert_eq!(cart.read_prg(0x8000), 6);
    assert_eq!(cart.read_prg(0xC000), 6);
    assert_eq!(cart.read_chr(0x0000), 0x76);
    assert_eq!(cart.mirroring(), Mirroring::Vertical);

    cart.write_prg(0x8009, 0);
    assert_eq!(cart.read_prg(0x8000), 8);
    assert_eq!(cart.read_prg(0xC000), 9);
    assert_eq!(cart.read_chr(0x0000), 0x74);
    assert_eq!(cart.mirroring(), Mirroring::Horizontal);

    let snapshot = cart.snapshot_state();
    cart.write_prg(0x8000, 0);
    cart.restore_state(&snapshot);

    assert_eq!(cart.read_prg(0x8000), 8);
    assert_eq!(cart.read_prg(0xC000), 9);
    assert_eq!(cart.mirroring(), Mirroring::Horizontal);
}

#[test]
fn mapper_203_uses_data_latch_for_mirrored_prg_and_chr_bank() {
    let mut cart = make_uxrom_like_cart(203, 16, 4);

    cart.write_prg(0x8000, 0x1D);

    assert_eq!(cart.read_prg(0x8000), 7);
    assert_eq!(cart.read_prg(0xC000), 7);
    assert_eq!(cart.read_chr(0x0000), 0x71);
    assert_eq!(cart.read_chr(0x1FFF), 0x71);
}

#[test]
fn mapper_229_switches_shared_bank_and_special_cases_bank_zero() {
    let mut cart = make_uxrom_like_cart(229, 16, 8);

    cart.write_prg(0x8020, 0);
    assert_eq!(cart.read_prg(0x8000), 0);
    assert_eq!(cart.read_prg(0xC000), 1);
    assert_eq!(cart.read_chr(0x0000), 0x70);
    assert_eq!(cart.mirroring(), Mirroring::Horizontal);

    cart.write_prg(0x8005, 0);
    assert_eq!(cart.read_prg(0x8000), 5);
    assert_eq!(cart.read_prg(0xC000), 5);
    assert_eq!(cart.read_chr(0x0000), 0x75);
    assert_eq!(cart.mirroring(), Mirroring::Vertical);
}

#[test]
fn mapper_212_switches_between_mirrored_16k_and_32k_modes() {
    let mut cart = make_uxrom_like_cart(212, 8, 8);

    cart.write_prg(0x800D, 0);
    assert_eq!(cart.read_prg(0x8000), 5);
    assert_eq!(cart.read_prg(0xC000), 5);
    assert_eq!(cart.read_chr(0x0000), 0x75);
    assert_eq!(cart.mirroring(), Mirroring::Horizontal);
    assert_eq!(cart.read_prg_ram(0x6000), 0x80);

    cart.write_prg(0xC006, 0);
    assert_eq!(cart.read_prg(0x8000), 6);
    assert_eq!(cart.read_prg(0xC000), 7);
    assert_eq!(cart.read_chr(0x0000), 0x76);
    assert_eq!(cart.mirroring(), Mirroring::Vertical);
}

#[test]
fn mapper_226_handles_mode_mirroring_high_bit_and_restore() {
    let mut cart = make_uxrom_like_cart(226, 80, 1);

    cart.write_prg(0x8000, 0x03);
    assert_eq!(cart.read_prg(0x8000), 2);
    assert_eq!(cart.read_prg(0xC000), 3);
    assert_eq!(cart.mirroring(), Mirroring::Horizontal);

    cart.write_prg(0x8001, 0x01);
    assert_eq!(cart.read_prg(0x8000), 66);
    assert_eq!(cart.read_prg(0xC000), 67);

    cart.write_prg(0x8000, 0x63);
    assert_eq!(cart.read_prg(0x8000), 67);
    assert_eq!(cart.read_prg(0xC000), 67);
    assert_eq!(cart.mirroring(), Mirroring::Vertical);

    let snapshot = cart.snapshot_state();
    cart.write_prg(0x8000, 0x00);
    cart.write_prg(0x8001, 0x00);
    cart.restore_state(&snapshot);

    assert_eq!(cart.read_prg(0x8000), 67);
    assert_eq!(cart.read_prg(0xC000), 67);
    assert_eq!(cart.mirroring(), Mirroring::Vertical);
}

#[test]
fn mapper_232_selects_inner_page_inside_64k_block() {
    let mut cart = make_uxrom_like_cart(232, 16, 1);

    cart.write_prg(0x8000, 0x10);
    cart.write_prg(0xC000, 0x01);

    assert_eq!(cart.read_prg(0x8000), 9);
    assert_eq!(cart.read_prg(0xC000), 11);
    assert_eq!(cart.read_chr(0x0000), 0x70);

    let snapshot = cart.snapshot_state();
    cart.write_prg(0x8000, 0x00);
    cart.write_prg(0xC000, 0x00);
    cart.restore_state(&snapshot);

    assert_eq!(cart.read_prg(0x8000), 9);
    assert_eq!(cart.read_prg(0xC000), 11);
}

#[test]
fn mapper_233_switches_prg_chr_and_mirroring_modes() {
    let mut cart = make_uxrom_like_cart(233, 32, 32);

    cart.write_prg(0x8000, 0x85);
    assert_eq!(cart.read_prg(0x8000), 4);
    assert_eq!(cart.read_prg(0xC000), 5);
    assert_eq!(cart.read_chr(0x0000), 0x75);
    assert_eq!(cart.mirroring(), Mirroring::Horizontal);

    cart.write_prg(0x8000, 0xE3);
    assert_eq!(cart.read_prg(0x8000), 3);
    assert_eq!(cart.read_prg(0xC000), 3);
    assert_eq!(cart.read_chr(0x0000), 0x73);
    assert_eq!(cart.mirroring(), Mirroring::OneScreenUpper);

    let snapshot = cart.snapshot_state();
    cart.write_prg(0x8000, 0x05);
    assert_eq!(cart.read_prg(0x8000), 4);
    assert_eq!(cart.read_prg(0xC000), 5);
    assert_eq!(cart.read_chr(0x0000), 0x75);
    assert_eq!(cart.mirroring(), Mirroring::ThreeScreenLower);

    cart.restore_state(&snapshot);
    assert_eq!(cart.read_prg(0x8000), 3);
    assert_eq!(cart.read_prg(0xC000), 3);
    assert_eq!(cart.read_chr(0x0000), 0x73);
    assert_eq!(cart.mirroring(), Mirroring::OneScreenUpper);
}

#[test]
fn mapper_234_latches_outer_and_inner_banks_from_cpu_reads() {
    let mut cart = make_simple_bank_cart(234, 16, 64);
    cart.prg_rom[0x7F80] = 0xC3;
    cart.prg_rom[2 * 0x8000 + 0x7FE8] = 0x51;
    cart.prg_rom[3 * 0x8000 + 0x7F80] = 0x00;
    cart.prg_rom[3 * 0x8000 + 0x7FE8] = 0x20;

    assert_eq!(cart.read_prg_cpu(0xFF80), 0xC3);
    assert_eq!(cart.read_prg(0x8000), 2);
    assert_eq!(cart.mirroring(), Mirroring::Horizontal);

    assert_eq!(cart.read_prg_cpu(0xFFE8), 0x51);
    assert_eq!(cart.read_prg(0x8000), 3);
    assert_eq!(cart.read_prg(0xFFFF), 3);
    assert_eq!(cart.read_chr(0x0000), 0x4D);

    let snapshot = cart.snapshot_state();

    assert_eq!(cart.read_prg_cpu(0xFF80), 0x00);
    assert_eq!(cart.read_prg(0x8000), 3);
    assert_eq!(cart.read_chr(0x0000), 0x4D);

    assert_eq!(cart.read_prg_cpu(0xFFE8), 0x20);
    assert_eq!(cart.read_prg(0x8000), 2);
    assert_eq!(cart.read_chr(0x0000), 0x4A);

    cart.restore_state(&snapshot);
    assert_eq!(cart.read_prg(0x8000), 3);
    assert_eq!(cart.read_chr(0x0000), 0x4D);
    assert_eq!(cart.mirroring(), Mirroring::Horizontal);
}

#[test]
fn mapper_235_selects_chip_page_and_mirroring_modes() {
    let mut cart = make_mapper235_cart();

    cart.write_prg(0x8203, 0);
    assert_eq!(cart.read_prg(0x8000), 70);
    assert_eq!(cart.read_prg(0xC000), 71);
    assert_eq!(cart.mirroring(), Mirroring::Horizontal);

    cart.write_prg(0x9E02, 0);
    assert_eq!(cart.read_prg(0x8000), 69);
    assert_eq!(cart.read_prg(0xC000), 69);
    assert_eq!(cart.mirroring(), Mirroring::OneScreenLower);

    cart.write_chr(0x0123, 0x5A);
    assert_eq!(cart.read_chr(0x0123), 0x5A);

    let snapshot = cart.snapshot_state();

    cart.write_prg(0x8100, 0);
    assert_eq!(cart.read_prg(0x8000), 0xFF);
    assert_eq!(cart.read_prg(0xC000), 0xFF);

    cart.restore_state(&snapshot);
    assert_eq!(cart.read_prg(0x8000), 69);
    assert_eq!(cart.read_prg(0xC000), 69);
    assert_eq!(cart.mirroring(), Mirroring::OneScreenLower);
}

#[test]
fn mapper_227_switches_between_unrom_and_nrom_modes() {
    let mut cart = make_mapper227_cart();

    cart.write_prg(0x822E, 0);
    assert_eq!(cart.read_prg(0x8000), 11);
    assert_eq!(cart.read_prg(0xC000), 15);
    assert_eq!(cart.mirroring(), Mirroring::Horizontal);
    cart.write_chr(0x0010, 0x33);
    assert_eq!(cart.read_chr(0x0010), 0x33);

    cart.write_prg(0x80F5, 0);
    assert_eq!(cart.read_prg(0x8000), 28);
    assert_eq!(cart.read_prg(0xC000), 29);
    assert_eq!(cart.mirroring(), Mirroring::Vertical);
    cart.write_chr(0x0010, 0x77);
    assert_eq!(cart.read_chr(0x0010), 0x33);

    let snapshot = cart.snapshot_state();
    cart.write_prg(0x8000, 0);
    cart.restore_state(&snapshot);
    assert_eq!(cart.read_prg(0x8000), 28);
    assert_eq!(cart.read_prg(0xC000), 29);
}

#[test]
fn mapper_225_switches_prg_chr_and_exposes_low_nibble_ram() {
    let mut cart = make_mapper225_cart(225);

    cart.write_prg(0xC085, 0);
    assert_eq!(cart.read_prg(0x8000), 66);
    assert_eq!(cart.read_prg(0xC000), 67);
    assert_eq!(cart.read_chr(0x0000), 0xC5);
    assert_eq!(cart.mirroring(), Mirroring::Vertical);

    cart.write_prg(0x5802, 0x3C);
    assert_eq!(cart.read_prg_low(0x5802), 0x0C);

    cart.write_prg(0xF081, 0);
    assert_eq!(cart.read_prg(0x8000), 66);
    assert_eq!(cart.read_prg(0xC000), 66);
    assert_eq!(cart.read_chr(0x0000), 0xC1);
    assert_eq!(cart.mirroring(), Mirroring::Horizontal);

    let snapshot = cart.snapshot_state();
    cart.write_prg(0x8000, 0);
    cart.restore_state(&snapshot);

    assert_eq!(cart.read_prg(0x8000), 66);
    assert_eq!(cart.read_prg(0xC000), 66);
    assert_eq!(cart.read_prg_low(0x5802), 0x0C);
    assert_eq!(cart.mirroring(), Mirroring::Horizontal);
}

#[test]
fn mapper_228_selects_prg_chip_and_chr_bank() {
    let mut cart = make_mapper228_cart();

    cart.write_prg(0xB885, 0x02);
    assert_eq!(cart.read_prg(0x8000), 66);
    assert_eq!(cart.read_prg(0xC000), 67);
    assert_eq!(cart.read_chr(0x0000), 0x36);
    assert_eq!(cart.mirroring(), Mirroring::Horizontal);

    let snapshot = cart.snapshot_state();
    cart.write_prg(0x9000, 0);
    assert_eq!(cart.read_prg(0x8000), 0);
    assert_eq!(cart.read_prg(0xC000), 0);
    cart.restore_state(&snapshot);

    assert_eq!(cart.read_prg(0x8000), 66);
    assert_eq!(cart.read_prg(0xC000), 67);
    assert_eq!(cart.read_chr(0x0000), 0x36);
}

#[test]
fn mapper_242_switches_prg_modes_and_write_protects_chr_ram() {
    let mut cart = make_mapper242_cart();

    cart.write_prg(0x824E, 0);
    assert_eq!(cart.read_prg(0x8000), 19);
    assert_eq!(cart.read_prg(0xC000), 23);
    assert_eq!(cart.mirroring(), Mirroring::Horizontal);
    cart.write_chr(0x0000, 0x5A);
    assert_eq!(cart.read_chr(0x0000), 0x5A);

    cart.write_prg(0x80B9, 0);
    assert_eq!(cart.read_prg(0x8000), 14);
    assert_eq!(cart.read_prg(0xC000), 15);
    assert_eq!(cart.mirroring(), Mirroring::Vertical);
    cart.write_chr(0x0000, 0x11);
    assert_eq!(cart.read_chr(0x0000), 0x5A);

    let snapshot = cart.snapshot_state();
    cart.write_prg(0x8000, 0);
    cart.restore_state(&snapshot);

    assert_eq!(cart.read_prg(0x8000), 14);
    assert_eq!(cart.read_prg(0xC000), 15);
    assert_eq!(cart.read_chr(0x0000), 0x5A);
}

#[test]
fn mapper_255_matches_225_bank_switching_without_low_ram() {
    let mut cart = make_mapper225_cart(255);

    cart.write_prg(0xC085, 0);
    assert_eq!(cart.read_prg(0x8000), 66);
    assert_eq!(cart.read_prg(0xC000), 67);
    assert_eq!(cart.read_chr(0x0000), 0xC5);

    cart.write_prg(0xF081, 0);
    assert_eq!(cart.read_prg(0x8000), 66);
    assert_eq!(cart.read_prg(0xC000), 66);
    assert_eq!(cart.read_prg_low(0x5802), 0x00);
    assert_eq!(cart.mirroring(), Mirroring::Horizontal);
}

#[test]
fn mapper_231_latches_prg_and_preserves_chr_ram_in_save_state() {
    let mut cart = make_mapper231_cart();

    cart.write_prg(0x80A0, 0);
    assert_eq!(cart.read_prg(0x8000), 0);
    assert_eq!(cart.read_prg(0xC000), 1);
    assert_eq!(cart.mirroring(), Mirroring::Horizontal);

    cart.write_prg(0x801E, 0);
    assert_eq!(cart.read_prg(0x8000), 30);
    assert_eq!(cart.read_prg(0xC000), 30);
    assert_eq!(cart.mirroring(), Mirroring::Vertical);

    cart.write_chr(0x0123, 0x5A);
    let snapshot = cart.snapshot_state();
    cart.write_prg(0x80A0, 0);
    cart.write_chr(0x0123, 0x00);
    cart.restore_state(&snapshot);

    assert_eq!(cart.read_prg(0x8000), 30);
    assert_eq!(cart.read_prg(0xC000), 30);
    assert_eq!(cart.read_chr(0x0123), 0x5A);
    assert_eq!(cart.mirroring(), Mirroring::Vertical);
}
