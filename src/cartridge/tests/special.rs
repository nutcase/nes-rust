use super::*;

#[test]
fn mapper_5_uses_fill_mode_exram_attributes_and_scanline_irq() {
    let mut cart = make_mmc5_cart();
    let mut ppu = crate::ppu::Ppu::new();

    ppu.nametable[0][0] = 0x21;
    ppu.nametable[1][0] = 0x42;

    cart.write_prg(0x5105, 0b11_10_01_00);
    cart.write_prg(0x5106, 0x66);
    cart.write_prg(0x5107, 0x03);
    cart.write_prg(0x5C00, 0x33);

    assert_eq!(cart.resolve_nametable(0), Some(0));
    assert_eq!(cart.resolve_nametable(3), Some(3));
    assert_eq!(cart.read_nametable_byte(0, 0, &ppu.nametable), 0x21);
    assert_eq!(cart.read_nametable_byte(1, 0, &ppu.nametable), 0x42);
    assert_eq!(cart.read_nametable_byte(2, 0, &ppu.nametable), 0x33);
    assert_eq!(cart.read_nametable_byte(3, 0, &ppu.nametable), 0x66);
    assert_eq!(cart.read_nametable_byte(3, 960, &ppu.nametable), 0xFF);

    cart.write_prg(0x5104, 0x01);
    cart.write_prg(0x5105, 0x00);
    cart.write_prg(0x5C00, 0b10_001011);
    cart.notify_ppumask_mmc5(0x18);
    ppu.nametable[0][0] = 0x04;

    assert_eq!(cart.read_nametable_byte(0, 0, &ppu.nametable), 0x04);
    assert_eq!(cart.read_nametable_byte(0, 960, &ppu.nametable), 0x02);
    assert_eq!(cart.read_chr(0x0040), 0xAC);

    cart.write_prg(0x5203, 0x02);
    cart.write_prg(0x5204, 0x80);
    cart.mmc5_scanline_tick();
    cart.mmc5_scanline_tick();
    assert!(!cart.irq_pending());
    cart.mmc5_scanline_tick();
    assert!(cart.irq_pending());
    assert_eq!(cart.read_prg_low(0x5204), 0xC0);
    assert_eq!(cart.read_prg_low(0x5204), 0x40);

    let snapshot = cart.snapshot_state();
    cart.write_prg(0x5C00, 0x00);
    cart.notify_ppumask_mmc5(0x00);
    cart.restore_state(&snapshot);

    assert_eq!(cart.read_prg_low(0x5204), 0x40);
    assert_eq!(cart.read_prg_low(0x5C00), 0x00);
    cart.write_prg(0x5104, 0x02);
    assert_eq!(cart.read_prg_low(0x5C00), 0b10_001011);
}

#[test]
fn mapper_19_uses_nametable_alias_irq_and_audio() {
    let mut cart = make_mapper19_cart();
    let ppu = crate::ppu::Ppu::new();

    cart.write_prg(0xC000, 0xE0);
    cart.write_prg(0xC800, 0xE1);
    cart.write_prg(0xD000, 0x07);
    cart.write_prg(0xD800, 0xE0);

    cart.write_nametable_byte(0, 0x012, &mut [[0; 1024]; 2], 0x44);
    cart.write_nametable_byte(1, 0x012, &mut [[0; 1024]; 2], 0x55);
    assert_eq!(cart.read_nametable_byte(0, 0x012, &ppu.nametable), 0x44);
    assert_eq!(cart.read_nametable_byte(1, 0x012, &ppu.nametable), 0x55);
    assert_eq!(cart.read_nametable_byte(2, 0x012, &ppu.nametable), 0x87);

    cart.write_prg(0x8000, 0xE0);
    cart.write_prg(0x8800, 0xE1);
    cart.write_chr(0x0012, 0x66);
    cart.write_chr(0x0412, 0x77);
    assert_eq!(cart.read_chr(0x0012), 0x66);
    assert_eq!(cart.read_chr(0x0412), 0x77);

    cart.write_prg_low(0x5000, 0xFD);
    cart.write_prg_low(0x5800, 0xFF);
    cart.clock_irq_counter_cycles(1);
    assert!(!cart.irq_pending());
    cart.clock_irq_counter_cycles(1);
    assert!(cart.irq_pending());
    assert_eq!(cart.read_prg_low(0x5800), 0xFF);
    cart.acknowledge_irq();
    assert!(!cart.irq_pending());

    cart.write_prg(0xE000, 0x03);
    cart.write_prg(0xF800, 0x40);
    for (addr, value) in [
        (0x00, 0x10),
        (0x01, 0x00),
        (0x02, 0x00),
        (0x03, 0x00),
        (0x04, 0xFC),
        (0x05, 0x00),
        (0x06, 0x00),
        (0x07, 0x0F),
        (0x7F, 0x70),
    ] {
        cart.write_prg(0xF800, 0x40 | addr);
        cart.write_prg_low(0x4800, value);
    }
    cart.write_prg(0xF800, 0x40);
    cart.write_prg_low(0x4800, 0x98);

    let mut non_zero = false;
    for _ in 0..64 {
        if cart.clock_expansion_audio().abs() > f32::EPSILON {
            non_zero = true;
            break;
        }
    }
    assert!(non_zero);
}

#[test]
fn mapper_18_uses_irq_width_control_and_mirroring() {
    let mut cart = make_mapper18_cart();

    cart.write_prg(0xF002, 0x00);
    assert_eq!(cart.mirroring(), Mirroring::Horizontal);
    cart.write_prg(0xF002, 0x01);
    assert_eq!(cart.mirroring(), Mirroring::Vertical);
    cart.write_prg(0xF002, 0x02);
    assert_eq!(cart.mirroring(), Mirroring::OneScreenLower);
    cart.write_prg(0xF002, 0x03);
    assert_eq!(cart.mirroring(), Mirroring::OneScreenUpper);

    cart.write_prg(0xE000, 0x0);
    cart.write_prg(0xE001, 0x0);
    cart.write_prg(0xE002, 0x0);
    cart.write_prg(0xE003, 0x1);
    cart.write_prg(0xF000, 0);
    cart.write_prg(0xF001, 0x03);
    cart.clock_irq_counter_cycles(1);
    assert!(cart.irq_pending());

    cart.write_prg(0xF000, 0);
    assert!(!cart.irq_pending());

    cart.write_prg(0xE000, 0x2);
    cart.write_prg(0xE001, 0x0);
    cart.write_prg(0xE002, 0x0);
    cart.write_prg(0xE003, 0x0);
    cart.write_prg(0xF000, 0);
    cart.write_prg(0xF001, 0x09);
    cart.clock_irq_counter_cycles(2);
    assert!(!cart.irq_pending());
    cart.clock_irq_counter_cycles(1);
    assert!(cart.irq_pending());
}

#[test]
fn mapper_210_uses_namco340_mirroring_control() {
    let mut cart = make_mapper210_cart(true);

    cart.write_prg(0xE000, 0x00);
    assert_eq!(cart.mirroring(), Mirroring::OneScreenLower);
    cart.write_prg(0xE000, 0x40);
    assert_eq!(cart.mirroring(), Mirroring::Vertical);
    cart.write_prg(0xE000, 0x80);
    assert_eq!(cart.mirroring(), Mirroring::OneScreenUpper);
    cart.write_prg(0xE000, 0xC0);
    assert_eq!(cart.mirroring(), Mirroring::Horizontal);

    cart.write_prg(0xE800, 0x06);
    cart.write_prg(0xF000, 0x07);
    assert_eq!(cart.read_prg(0xA000), 6);
    assert_eq!(cart.read_prg(0xC000), 7);

    let snapshot = cart.snapshot_state();
    cart.write_prg(0xE000, 0x01);
    cart.write_prg(0xE800, 0x00);
    cart.restore_state(&snapshot);

    assert_eq!(cart.mirroring(), Mirroring::Horizontal);
    assert_eq!(cart.read_prg(0xA000), 6);
}

#[test]
fn mapper_21_uses_vrc4_irq_and_restores_state() {
    let mut cart = make_mapper21_cart();

    cart.write_prg(0x8000, 0x02);
    cart.write_prg(0x9000, 0x03);
    cart.write_prg(0xF000, 0x0E);
    cart.write_prg(0xF040, 0x0F);
    cart.write_prg(0xF004, 0x07);
    cart.clock_irq_counter_cycles(1);
    assert!(!cart.irq_pending());
    cart.clock_irq_counter_cycles(1);
    assert!(cart.irq_pending());

    let snapshot = cart.snapshot_state();
    cart.write_prg(0x8000, 0x00);
    cart.write_prg(0x9000, 0x00);
    cart.write_prg(0xF004, 0x00);
    cart.restore_state(&snapshot);

    assert_eq!(cart.read_prg(0x8000), 2);
    assert_eq!(cart.mirroring(), Mirroring::OneScreenUpper);
    cart.acknowledge_irq();
    assert!(!cart.irq_pending());
    cart.clock_irq_counter_cycles(1);
    assert!(!cart.irq_pending());
    cart.clock_irq_counter_cycles(1);
    assert!(cart.irq_pending());
}

#[test]
fn mapper_22_ignores_vrc4_irq_registers_and_restores_state() {
    let mut cart = make_mapper22_cart();

    cart.write_prg(0x8000, 0x02);
    cart.write_prg(0x9000, 0x01);
    cart.write_prg(0xF000, 0x0E);
    cart.write_prg(0xF002, 0x07);
    cart.clock_irq_counter_cycles(16);
    assert!(!cart.irq_pending());

    let snapshot = cart.snapshot_state();
    cart.write_prg(0x8000, 0x00);
    cart.write_prg(0x9000, 0x00);
    cart.write_prg_ram(0x6000, 0x00);
    cart.restore_state(&snapshot);

    assert_eq!(cart.read_prg(0x8000), 2);
    assert_eq!(cart.read_prg_ram(0x6000), 0x60);
    assert_eq!(cart.mirroring(), Mirroring::Horizontal);
    assert!(!cart.irq_pending());
}

#[test]
fn mapper_23_uses_vrc2_latch_and_vrc4_irq() {
    let mut cart = make_mapper23_cart();

    cart.write_prg_ram(0x6000, 0x01);
    assert_eq!(cart.read_prg_ram(0x6000), 0x61);
    assert_eq!(cart.read_prg_ram(0x7000), 0x70);

    cart.write_prg(0x9000, 0xFF);
    assert_eq!(cart.mirroring(), Mirroring::Horizontal);

    cart.write_prg(0xF000, 0x0E);
    cart.write_prg(0xF004, 0x0F);
    cart.write_prg(0xF008, 0x07);
    cart.clock_irq_counter_cycles(1);
    assert!(!cart.irq_pending());
    cart.clock_irq_counter_cycles(1);
    assert!(cart.irq_pending());

    cart.acknowledge_irq();
    assert!(!cart.irq_pending());

    let snapshot = cart.snapshot_state();
    cart.write_prg(0x9008, 0x03);
    cart.write_prg(0xF008, 0x00);
    cart.restore_state(&snapshot);

    assert_eq!(cart.read_prg_ram(0x6000), 0x61);
    cart.clock_irq_counter_cycles(1);
    assert!(!cart.irq_pending());
    cart.clock_irq_counter_cycles(1);
    assert!(cart.irq_pending());
}

#[test]
fn mapper_26_uses_vrc6_irq_audio_and_restores_state() {
    fn reg_addr(reg: u16) -> u16 {
        (reg & !0x0003) | (((reg & 0x0001) << 1) | ((reg & 0x0002) >> 1))
    }

    let mut cart = make_mapper24_26_cart(26);

    cart.write_prg(reg_addr(0x8000), 0x02);
    cart.write_prg(reg_addr(0xB003), 0x8C);
    cart.write_prg_ram(0x6002, 0x77);

    cart.write_prg(reg_addr(0xF000), 0xFE);
    cart.write_prg(reg_addr(0xF001), 0x07);
    cart.clock_irq_counter_cycles(1);
    assert!(!cart.irq_pending());
    cart.clock_irq_counter_cycles(1);
    assert!(cart.irq_pending());

    cart.write_prg(reg_addr(0x9000), 0x8F);
    cart.write_prg(reg_addr(0x9001), 0x00);
    cart.write_prg(reg_addr(0x9002), 0x80);
    cart.write_prg(reg_addr(0x9003), 0x00);

    let mut non_zero = false;
    for _ in 0..16 {
        if cart.clock_expansion_audio().abs() > f32::EPSILON {
            non_zero = true;
            break;
        }
    }
    assert!(non_zero);

    let snapshot = cart.snapshot_state();

    cart.write_prg(reg_addr(0x8000), 0x00);
    cart.write_prg(reg_addr(0xB003), 0x00);
    cart.write_prg_ram(0x6002, 0x00);
    cart.write_prg(reg_addr(0x9002), 0x00);
    cart.acknowledge_irq();

    cart.restore_state(&snapshot);

    assert_eq!(cart.read_prg(0x8000), 0x04);
    assert_eq!(cart.read_prg_ram(0x6002), 0x77);
    assert_eq!(cart.mirroring(), Mirroring::OneScreenUpper);
    assert!(cart.irq_pending());

    let mut restored_non_zero = false;
    for _ in 0..16 {
        if cart.clock_expansion_audio().abs() > f32::EPSILON {
            restored_non_zero = true;
            break;
        }
    }
    assert!(restored_non_zero);
}

#[test]
fn mapper_25_supports_vrc2c_battery_ram_and_vrc4d_irq() {
    let mut cart = make_mapper25_cart(true);

    cart.write_prg_ram(0x6002, 0x77);
    assert_eq!(cart.read_prg_ram(0x6002), 0x77);

    cart.write_prg(0x9000, 0x01);
    assert_eq!(cart.mirroring(), Mirroring::Horizontal);

    cart.write_prg(0xF000, 0x0E);
    cart.write_prg(0xF008, 0x0F);
    cart.write_prg(0xF004, 0x07);
    cart.clock_irq_counter_cycles(1);
    assert!(!cart.irq_pending());
    cart.clock_irq_counter_cycles(1);
    assert!(cart.irq_pending());

    cart.acknowledge_irq();
    assert!(!cart.irq_pending());

    let snapshot = cart.snapshot_state();
    cart.write_prg_ram(0x6002, 0x00);
    cart.write_prg(0xF004, 0x00);
    cart.restore_state(&snapshot);

    assert_eq!(cart.read_prg_ram(0x6002), 0x77);
    cart.clock_irq_counter_cycles(1);
    assert!(!cart.irq_pending());
    cart.clock_irq_counter_cycles(1);
    assert!(cart.irq_pending());
}

#[test]
fn mapper_64_supports_scanline_and_cycle_irq_modes() {
    let mut cart = make_mapper64_cart();

    cart.write_prg(0xC000, 0x00);
    cart.write_prg(0xC001, 0x00);
    cart.write_prg(0xE001, 0x00);

    cart.clock_irq_counter();
    assert!(!cart.irq_pending());
    cart.clock_irq_counter_cycles(3);
    assert!(!cart.irq_pending());
    cart.clock_irq_counter_cycles(1);
    assert!(cart.irq_pending());

    cart.write_prg(0xE000, 0x00);
    assert!(!cart.irq_pending());

    cart.write_prg(0xC001, 0x01);
    cart.write_prg(0xE001, 0x00);
    cart.clock_irq_counter();
    cart.clock_irq_counter();
    assert!(!cart.irq_pending());

    cart.clock_irq_counter_cycles(7);
    assert!(!cart.irq_pending());

    let snapshot = cart.snapshot_state();
    cart.clock_irq_counter_cycles(1);
    assert!(cart.irq_pending());

    cart.write_prg(0xE000, 0x00);
    cart.restore_state(&snapshot);
    assert!(!cart.irq_pending());
    cart.clock_irq_counter_cycles(1);
    assert!(cart.irq_pending());
}

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
fn mapper_40_switches_c000_bank_and_fixed_cycle_irq() {
    let mut cart = make_mapper40_cart();

    cart.write_prg(0xE000, 0x03);

    assert_eq!(cart.read_prg_ram(0x6000), 6);
    assert_eq!(cart.read_prg(0x8000), 4);
    assert_eq!(cart.read_prg(0xA000), 5);
    assert_eq!(cart.read_prg(0xC000), 3);
    assert_eq!(cart.read_prg(0xE000), 7);

    cart.write_prg(0xA000, 0x00);
    cart.clock_irq_counter_cycles(4095);
    assert!(!cart.irq_pending());
    cart.clock_irq_counter_cycles(1);
    assert!(cart.irq_pending());

    let snapshot = cart.snapshot_state();
    cart.write_prg(0x8000, 0x00);
    assert!(!cart.irq_pending());

    cart.restore_state(&snapshot);
    assert!(cart.irq_pending());
}

#[test]
fn mapper_50_uses_scrambled_c000_bank_and_fixed_cycle_irq() {
    let mut cart = make_mapper50_cart();

    cart.write_prg(0x4020, 0x07);

    assert_eq!(cart.read_prg_ram(0x6000), 15);
    assert_eq!(cart.read_prg(0x8000), 8);
    assert_eq!(cart.read_prg(0xA000), 9);
    assert_eq!(cart.read_prg(0xC000), 7);
    assert_eq!(cart.read_prg(0xE000), 11);

    cart.write_prg(0x4120, 0x01);
    cart.clock_irq_counter_cycles(4096);
    assert!(cart.irq_pending());

    let snapshot = cart.snapshot_state();
    cart.write_prg(0x4120, 0x00);
    assert!(!cart.irq_pending());

    cart.restore_state(&snapshot);
    assert!(cart.irq_pending());
}

#[test]
fn mapper_59_latches_address_modes_and_unlocks_on_reset() {
    let mut cart = make_mapper59_cart();

    cart.write_prg(0x80BD, 0x00);
    assert_eq!(cart.read_prg(0x8000), 3);
    assert_eq!(cart.read_prg(0xC000), 3);
    assert_eq!(cart.read_chr(0x0000), 0x45);
    assert_eq!(cart.mirroring(), Mirroring::Horizontal);

    let snapshot = cart.snapshot_state();
    cart.write_prg(0x8122, 0x00);
    assert_eq!(cart.read_prg(0x8000), 0);
    cart.restore_state(&snapshot);

    assert_eq!(cart.read_prg(0x8000), 3);
    assert_eq!(cart.read_chr(0x0000), 0x45);
    assert_eq!(cart.mirroring(), Mirroring::Horizontal);

    cart.write_prg(0x8222, 0x00);
    assert_eq!(cart.read_prg(0x8000), 2);
    assert_eq!(cart.read_prg(0xC000), 3);
    assert_eq!(cart.read_chr(0x0000), 0x42);
    assert_eq!(cart.mirroring(), Mirroring::Vertical);

    cart.write_prg(0x80BD, 0x00);
    assert_eq!(cart.read_prg(0x8000), 2);
    assert_eq!(cart.read_chr(0x0000), 0x42);

    cart.on_reset();
    cart.write_prg(0x80BD, 0x00);
    assert_eq!(cart.read_prg(0x8000), 3);
    assert_eq!(cart.read_prg(0xC000), 3);
    assert_eq!(cart.read_chr(0x0000), 0x45);
    assert_eq!(cart.mirroring(), Mirroring::Horizontal);
}

#[test]
fn mapper_60_cycles_through_four_nrom_games_on_reset() {
    let mut cart = make_mapper60_cart();

    assert_eq!(cart.read_prg(0x8000), 0);
    assert_eq!(cart.read_prg(0xC000), 0);
    assert_eq!(cart.read_chr(0x0000), 0x50);

    cart.on_reset();
    assert_eq!(cart.read_prg(0x8000), 1);
    assert_eq!(cart.read_prg(0xC000), 1);
    assert_eq!(cart.read_chr(0x0000), 0x51);

    cart.on_reset();
    assert_eq!(cart.read_prg(0x8000), 2);
    assert_eq!(cart.read_prg(0xC000), 2);
    assert_eq!(cart.read_chr(0x0000), 0x52);

    let snapshot = cart.snapshot_state();

    cart.on_reset();
    assert_eq!(cart.read_prg(0x8000), 3);
    assert_eq!(cart.read_prg(0xC000), 3);
    assert_eq!(cart.read_chr(0x0000), 0x53);

    cart.on_reset();
    assert_eq!(cart.read_prg(0x8000), 0);
    assert_eq!(cart.read_prg(0xC000), 0);
    assert_eq!(cart.read_chr(0x0000), 0x50);

    cart.restore_state(&snapshot);
    assert_eq!(cart.read_prg(0x8000), 2);
    assert_eq!(cart.read_prg(0xC000), 2);
    assert_eq!(cart.read_chr(0x0000), 0x52);
}

#[test]
fn mapper_61_latches_prg_chr_and_mirroring_modes() {
    let mut cart = make_mapper61_cart();

    cart.write_prg(0x89B5, 0x00);
    assert_eq!(cart.read_prg(0x8000), 11);
    assert_eq!(cart.read_prg(0xC000), 11);
    assert_eq!(cart.read_chr(0x0000), 0x49);
    assert_eq!(cart.mirroring(), Mirroring::Horizontal);

    let snapshot = cart.snapshot_state();

    cart.write_prg(0x83C2, 0x00);
    assert_eq!(cart.read_prg(0x8000), 4);
    assert_eq!(cart.read_prg(0xC000), 5);
    assert_eq!(cart.read_chr(0x0000), 0x43);
    assert_eq!(cart.mirroring(), Mirroring::Horizontal);

    cart.restore_state(&snapshot);
    assert_eq!(cart.read_prg(0x8000), 11);
    assert_eq!(cart.read_prg(0xC000), 11);
    assert_eq!(cart.read_chr(0x0000), 0x49);
}

#[test]
fn mapper_77_routes_two_nametables_to_chr_ram_and_two_to_internal_vram() {
    let mut cart = make_mapper77_cart();
    let mut ppu = crate::ppu::Ppu::new();

    ppu.v = 0x2000;
    ppu.write_register(0x2007, 0x55, Some(&mut cart));
    ppu.v = 0x2400;
    ppu.write_register(0x2007, 0x66, Some(&mut cart));
    ppu.v = 0x2800;
    ppu.write_register(0x2007, 0x77, Some(&mut cart));
    ppu.v = 0x2C00;
    ppu.write_register(0x2007, 0x88, Some(&mut cart));

    assert_eq!(cart.read_nametable_byte(0, 0, &ppu.nametable), 0x55);
    assert_eq!(cart.read_nametable_byte(1, 0, &ppu.nametable), 0x66);
    assert_eq!(cart.read_nametable_byte(2, 0, &ppu.nametable), 0x77);
    assert_eq!(cart.read_nametable_byte(3, 0, &ppu.nametable), 0x88);
    assert_eq!(ppu.nametable[0][0], 0x77);
    assert_eq!(ppu.nametable[1][0], 0x88);
}

#[test]
fn mapper_99_uses_cartridge_four_screen_nametables() {
    let mut cart = make_mapper99_cart();
    let mut ppu = crate::ppu::Ppu::new();

    ppu.v = 0x2000;
    ppu.write_register(0x2007, 0x11, Some(&mut cart));
    ppu.v = 0x2400;
    ppu.write_register(0x2007, 0x22, Some(&mut cart));
    ppu.v = 0x2800;
    ppu.write_register(0x2007, 0x33, Some(&mut cart));
    ppu.v = 0x2C00;
    ppu.write_register(0x2007, 0x44, Some(&mut cart));

    assert_eq!(cart.read_nametable_byte(0, 0, &ppu.nametable), 0x11);
    assert_eq!(cart.read_nametable_byte(1, 0, &ppu.nametable), 0x22);
    assert_eq!(cart.read_nametable_byte(2, 0, &ppu.nametable), 0x33);
    assert_eq!(cart.read_nametable_byte(3, 0, &ppu.nametable), 0x44);
    assert_eq!(ppu.nametable[0][0], 0);
    assert_eq!(ppu.nametable[1][0], 0);
}

#[test]
fn mapper_137_custom_mirroring_and_state_restore() {
    let mut cart = make_mapper137_cart();

    cart.write_prg(0x4100, 7);
    cart.write_prg(0x4101, 0x00);
    assert_eq!(cart.resolve_nametable(0), Some(0));
    assert_eq!(cart.resolve_nametable(1), Some(1));
    assert_eq!(cart.resolve_nametable(2), Some(1));
    assert_eq!(cart.resolve_nametable(3), Some(1));

    let snapshot = cart.snapshot_state();

    cart.write_prg(0x4100, 7);
    cart.write_prg(0x4101, 0x06);
    assert_eq!(cart.resolve_nametable(0), None);
    assert_eq!(cart.mirroring(), Mirroring::OneScreenUpper);

    cart.restore_state(&snapshot);
    assert_eq!(cart.resolve_nametable(0), Some(0));
    assert_eq!(cart.resolve_nametable(3), Some(1));
    assert_eq!(cart.read_prg_low(0x4101), 0x00);
}

#[test]
fn mapper_67_switches_chr_prg_mirroring_and_cycle_irq() {
    let mut cart = make_mapper67_cart();

    cart.write_prg(0x8800, 0x01);
    cart.write_prg(0x9800, 0x02);
    cart.write_prg(0xA800, 0x03);
    cart.write_prg(0xB800, 0x04);
    cart.write_prg(0xE800, 0x03);
    cart.write_prg(0xF800, 0x03);

    assert_eq!(cart.read_prg(0x8000), 3);
    assert_eq!(cart.read_prg(0xC000), 7);
    assert_eq!(cart.read_chr(0x0000), 0x71);
    assert_eq!(cart.read_chr(0x0800), 0x72);
    assert_eq!(cart.read_chr(0x1000), 0x73);
    assert_eq!(cart.read_chr(0x1800), 0x74);
    assert_eq!(cart.mirroring(), Mirroring::OneScreenUpper);

    cart.write_prg(0xC800, 0x00);
    cart.write_prg(0xC800, 0x02);
    cart.write_prg(0xD800, 0x10);
    cart.clock_irq_counter_cycles(2);
    assert!(!cart.irq_pending());
    cart.clock_irq_counter_cycles(1);
    assert!(cart.irq_pending());

    let snapshot = cart.snapshot_state();
    cart.write_prg(0x8000, 0x00);
    assert!(!cart.irq_pending());

    cart.restore_state(&snapshot);
    assert!(cart.irq_pending());
    assert_eq!(cart.read_prg(0x8000), 3);
    assert_eq!(cart.read_chr(0x1800), 0x74);
}

#[test]
fn mapper_185_disables_chr_for_initial_probe_reads_after_reset() {
    let mut cart = make_mapper185_cart();

    cart.write_prg(0x8000, 0x02);
    assert_eq!(cart.read_chr(0x0000), 0);

    let snapshot = cart.snapshot_state();
    assert_eq!(cart.read_chr(0x0000), 0);
    assert_eq!(cart.read_chr(0x0000), 0x62);

    cart.restore_state(&snapshot);
    assert_eq!(cart.read_chr(0x0000), 0);
    assert_eq!(cart.read_chr(0x0000), 0x62);

    cart.on_reset();
    assert_eq!(cart.read_chr(0x0000), 0);
    assert_eq!(cart.read_chr(0x0000), 0);
    assert_eq!(cart.read_chr(0x0000), 0x62);
}

#[test]
fn mapper_189_uses_low_address_prg_bank_writes_with_mmc3_chr() {
    let mut cart = make_mapper189_cart();

    cart.write_prg(0x4100, 0xA4);
    cart.write_prg(0x8000, 0x00);
    cart.write_prg(0x8001, 0x04);
    cart.write_prg(0x8000, 0x02);
    cart.write_prg(0x8001, 0x07);

    assert_eq!(cart.read_prg(0x8000), 14);
    assert_eq!(cart.read_prg(0xE000), 14);
    assert_eq!(cart.read_chr(0x0000), 0x64);
    assert_eq!(cart.read_chr(0x0400), 0x65);
    assert_eq!(cart.read_chr(0x1000), 0x67);

    let snapshot = cart.snapshot_state();

    cart.write_prg_ram(0x6000, 0x93);
    cart.write_prg(0x8000, 0x02);
    cart.write_prg(0x8001, 0x01);
    assert_eq!(cart.read_prg(0x8000), 11);
    assert_eq!(cart.read_chr(0x1000), 0x61);

    cart.restore_state(&snapshot);
    assert_eq!(cart.read_prg(0x8000), 14);
    assert_eq!(cart.read_chr(0x0000), 0x64);
    assert_eq!(cart.read_chr(0x0400), 0x65);
    assert_eq!(cart.read_chr(0x1000), 0x67);
}

#[test]
fn mapper_73_switches_prg_and_handles_16bit_and_8bit_irq_modes() {
    let mut cart = make_vrc3_cart();

    cart.write_prg(0xF000, 0x03);
    assert_eq!(cart.read_prg(0x8000), 3);
    assert_eq!(cart.read_prg(0xC000), 7);

    cart.write_prg_ram(0x6000, 0xA5);
    assert_eq!(cart.read_prg_ram(0x6000), 0xA5);

    cart.write_prg(0x8000, 0x0E);
    cart.write_prg(0x9000, 0x0F);
    cart.write_prg(0xA000, 0x0F);
    cart.write_prg(0xB000, 0x0F);
    cart.write_prg(0xC000, 0x02);
    cart.clock_irq_counter_cycles(1);
    assert!(!cart.irq_pending());
    cart.clock_irq_counter_cycles(1);
    assert!(cart.irq_pending());

    let snapshot = cart.snapshot_state();
    cart.write_prg(0xD000, 0x00);
    assert!(!cart.irq_pending());

    cart.restore_state(&snapshot);
    assert!(cart.irq_pending());

    cart.write_prg(0xD000, 0x00);
    cart.write_prg(0x8000, 0x0E);
    cart.write_prg(0x9000, 0x0F);
    cart.write_prg(0xA000, 0x02);
    cart.write_prg(0xB000, 0x01);
    cart.write_prg(0xC000, 0x06);
    cart.clock_irq_counter_cycles(1);
    assert!(!cart.irq_pending());
    cart.clock_irq_counter_cycles(1);
    assert!(cart.irq_pending());
    assert_eq!(cart.vrc3.as_ref().unwrap().irq_counter, 0x12FE);
}

#[test]
fn mapper_142_switches_four_8k_prg_slots_and_uses_vrc3_irq() {
    let mut cart = make_mapper142_cart();

    cart.write_prg(0xE000, 0x01);
    cart.write_prg(0xF000, 0x03);
    cart.write_prg(0xE000, 0x02);
    cart.write_prg(0xF000, 0x04);
    cart.write_prg(0xE000, 0x03);
    cart.write_prg(0xF000, 0x05);
    cart.write_prg(0xE000, 0x04);
    cart.write_prg(0xF000, 0x06);

    assert_eq!(cart.read_prg_ram(0x6000), 6);
    assert_eq!(cart.read_prg(0x8000), 3);
    assert_eq!(cart.read_prg(0xA000), 4);
    assert_eq!(cart.read_prg(0xC000), 5);
    assert_eq!(cart.read_prg(0xE000), 15);

    cart.write_chr(0x0123, 0xA5);
    assert_eq!(cart.read_chr(0x0123), 0xA5);

    cart.write_prg(0x8000, 0x0E);
    cart.write_prg(0x9000, 0x0F);
    cart.write_prg(0xA000, 0x0F);
    cart.write_prg(0xB000, 0x0F);
    cart.write_prg(0xC000, 0x02);
    cart.clock_irq_counter_cycles(1);
    assert!(!cart.irq_pending());
    cart.clock_irq_counter_cycles(1);
    assert!(cart.irq_pending());

    let snapshot = cart.snapshot_state();
    cart.write_prg(0xD000, 0x00);
    cart.write_prg(0xE000, 0x01);
    cart.write_prg(0xF000, 0x00);
    assert!(!cart.irq_pending());
    assert_eq!(cart.read_prg(0x8000), 0);

    cart.restore_state(&snapshot);
    assert!(cart.irq_pending());
    assert_eq!(cart.read_prg(0x8000), 3);
    assert_eq!(cart.read_prg_ram(0x6000), 6);
}

#[test]
fn mapper_32_switches_prg_chr_and_prg_mode() {
    let mut cart = make_mapper32_cart();

    cart.write_prg(0x8000, 0x03);
    cart.write_prg(0xA000, 0x04);
    for index in 0..8 {
        cart.write_prg(0xB000 + index as u16, 0x08 + index as u8);
    }

    assert_eq!(cart.read_prg(0x8000), 3);
    assert_eq!(cart.read_prg(0xA000), 4);
    assert_eq!(cart.read_prg(0xC000), 6);
    assert_eq!(cart.read_prg(0xE000), 7);
    assert_eq!(cart.read_chr(0x0000), 0x38);
    assert_eq!(cart.read_chr(0x1C00), 0x3F);

    cart.write_prg(0x9000, 0x03);
    assert_eq!(cart.read_prg(0x8000), 6);
    assert_eq!(cart.read_prg(0xC000), 3);
    assert_eq!(cart.mirroring(), Mirroring::Horizontal);

    let snapshot = cart.snapshot_state();
    cart.write_prg(0x8000, 0x00);
    cart.write_prg(0x9000, 0x00);
    cart.restore_state(&snapshot);

    assert_eq!(cart.read_prg(0x8000), 6);
    assert_eq!(cart.read_prg(0xC000), 3);
    assert_eq!(cart.read_chr(0x0000), 0x38);
    assert_eq!(cart.mirroring(), Mirroring::Horizontal);
}

#[test]
fn mapper_42_switches_low_prg_bank_and_counts_cycle_irq() {
    let mut cart = make_mapper42_cart();

    cart.write_prg(0xE000, 0x27);

    assert_eq!(cart.read_prg_ram(0x6000), 7);
    assert_eq!(cart.read_prg(0x8000), 12);
    assert_eq!(cart.read_prg(0xA000), 13);
    assert_eq!(cart.read_prg(0xC000), 14);
    assert_eq!(cart.read_prg(0xE000), 15);
    assert_eq!(cart.mirroring(), Mirroring::Vertical);

    cart.clock_irq_counter_cycles(24_575);
    assert!(!cart.irq_pending());
    cart.clock_irq_counter_cycles(1);
    assert!(cart.irq_pending());

    let snapshot = cart.snapshot_state();
    cart.write_prg(0xE000, 0x10);
    assert_eq!(cart.read_prg_ram(0x6000), 0);
    assert_eq!(cart.mirroring(), Mirroring::Horizontal);
    assert!(!cart.irq_pending());

    cart.restore_state(&snapshot);
    assert_eq!(cart.read_prg_ram(0x6000), 7);
    assert!(cart.irq_pending());
}

#[test]
fn mapper_43_maps_split_prg_layout_and_12bit_irq() {
    let mut cart = make_mapper43_cart();

    assert_eq!(cart.read_prg_low(0x5000), 0xF2);
    assert_eq!(cart.read_prg_ram(0x6000), 2);
    assert_eq!(cart.read_prg(0x8000), 1);
    assert_eq!(cart.read_prg(0xA000), 0);
    assert_eq!(cart.read_prg(0xE000), 0xE8);

    cart.write_prg(0x4022, 0x01);
    assert_eq!(cart.read_prg(0xC000), 3);
    cart.write_prg(0x4022, 0x05);
    assert_eq!(cart.read_prg(0xC000), 7);

    cart.write_prg(0x4122, 0x01);
    cart.clock_irq_counter_cycles(4095);
    assert!(!cart.irq_pending());
    cart.clock_irq_counter_cycles(1);
    assert!(cart.irq_pending());

    let snapshot = cart.snapshot_state();
    cart.write_prg(0x4122, 0x00);
    assert!(!cart.irq_pending());

    cart.restore_state(&snapshot);
    assert!(cart.irq_pending());
    assert_eq!(cart.read_prg(0xC000), 7);
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
fn mapper_65_switches_prg_chr_and_cycle_irq() {
    let mut cart = make_mapper65_cart();

    cart.write_prg(0x8000, 0x03);
    cart.write_prg(0xA000, 0x04);
    for index in 0..8 {
        cart.write_prg(0xB000 + index as u16, 0x08 + index as u8);
    }

    assert_eq!(cart.read_prg(0x8000), 3);
    assert_eq!(cart.read_prg(0xA000), 4);
    assert_eq!(cart.read_prg(0xC000), 14);
    assert_eq!(cart.read_prg(0xE000), 15);
    assert_eq!(cart.read_chr(0x0000), 0x98);
    assert_eq!(cart.read_chr(0x1C00), 0x9F);

    cart.write_prg(0x9000, 0x80);
    cart.write_prg(0x9001, 0x80);
    assert_eq!(cart.read_prg(0x8000), 14);
    assert_eq!(cart.read_prg(0xC000), 3);
    assert_eq!(cart.mirroring(), Mirroring::Horizontal);

    cart.write_prg(0x9005, 0x00);
    cart.write_prg(0x9006, 0x02);
    cart.write_prg(0x9004, 0x00);
    cart.write_prg(0x9003, 0x80);
    cart.clock_irq_counter_cycles(1);
    assert!(!cart.irq_pending());
    cart.clock_irq_counter_cycles(1);
    assert!(cart.irq_pending());

    let snapshot = cart.snapshot_state();
    cart.write_prg(0x9003, 0x00);
    assert!(!cart.irq_pending());

    cart.restore_state(&snapshot);
    assert!(cart.irq_pending());
    assert_eq!(cart.read_prg(0xC000), 3);
}

#[test]
fn mapper_103_switches_bank_ram_overlay_and_mirroring() {
    let mut cart = make_mapper103_cart();

    cart.write_prg(0x8000, 0x05);
    cart.write_prg(0xF000, 0x10);
    assert_eq!(cart.read_prg_ram(0x6000), 5);
    assert_eq!(cart.read_prg(0x8000), 0xA1);
    assert_eq!(cart.read_prg(0xB800), 0xB2);
    assert_eq!(cart.read_prg(0xD800), 0xC3);

    cart.write_prg(0xE000, 0x08);
    assert_eq!(cart.mirroring(), Mirroring::Horizontal);

    cart.write_prg(0xF000, 0x00);
    cart.write_prg_ram(0x6000, 0x5A);
    cart.write_prg(0xB800, 0xA5);
    assert_eq!(cart.read_prg_ram(0x6000), 0xA5);
    assert_eq!(cart.read_prg(0xB800), 0xA5);

    let snapshot = cart.snapshot_state();
    cart.write_prg(0xF000, 0x10);
    cart.write_prg_ram(0x6000, 0x00);
    cart.restore_state(&snapshot);

    assert_eq!(cart.read_prg_ram(0x6000), 0xA5);
    assert_eq!(cart.read_prg(0xB800), 0xA5);
    assert_eq!(cart.mirroring(), Mirroring::Horizontal);
}

#[test]
fn mapper_153_uses_outer_prg_bank_chr_ram_and_wram_enable() {
    let mut cart = make_mapper153_cart();

    cart.write_prg(0x8000, 0x01);
    cart.write_prg(0x8008, 0x03);
    cart.write_prg(0x8009, 0x03);
    cart.write_prg(0x800D, 0x40);

    assert_eq!(cart.read_prg(0x8000), 19);
    assert_eq!(cart.read_prg(0xC000), 31);
    assert_eq!(cart.mirroring(), Mirroring::OneScreenUpper);

    cart.write_prg_ram(0x6000, 0x5A);
    assert_eq!(cart.read_prg_ram(0x6000), 0x5A);

    cart.write_chr(0x1234, 0x77);
    assert_eq!(cart.read_chr(0x1234), 0x77);

    cart.write_prg(0x800B, 0x01);
    cart.write_prg(0x800C, 0x00);
    cart.write_prg(0x800A, 0x01);
    cart.clock_irq_counter_cycles(1);
    assert!(!cart.irq_pending());
    cart.clock_irq_counter_cycles(1);
    assert!(cart.irq_pending());

    let snapshot = cart.snapshot_state();
    cart.write_prg(0x800D, 0x00);
    assert_eq!(cart.read_prg_ram(0x6000), 0);

    cart.restore_state(&snapshot);
    assert_eq!(cart.read_prg_ram(0x6000), 0x5A);
    assert!(cart.irq_pending());
}

#[test]
fn mapper_159_uses_x24c01_eeprom_and_bandai_banks() {
    let mut cart = make_mapper159_cart();

    cart.write_prg(0x8000, 0x02);
    cart.write_prg(0x8001, 0x03);
    cart.write_prg(0x8008, 0x05);
    cart.write_prg(0x8009, 0x01);

    assert_eq!(cart.read_prg(0x8000), 5);
    assert_eq!(cart.read_prg(0xC000), 31);
    assert_eq!(cart.read_chr(0x0000), 0x82);
    assert_eq!(cart.read_chr(0x0400), 0x83);
    assert_eq!(cart.mirroring(), Mirroring::Horizontal);
    assert_eq!(cart.read_prg_ram(0x6000), 0x10);

    cart.write_prg(0x800B, 0x01);
    cart.write_prg(0x800C, 0x00);
    cart.write_prg(0x800A, 0x01);
    cart.clock_irq_counter_cycles(1);
    assert!(!cart.irq_pending());
    cart.clock_irq_counter_cycles(1);
    assert!(cart.irq_pending());

    let snapshot = cart.snapshot_state();
    cart.write_prg(0x8000, 0x00);
    cart.write_prg(0x8008, 0x00);
    cart.write_prg(0x8009, 0x00);
    cart.restore_state(&snapshot);

    assert_eq!(cart.read_prg(0x8000), 5);
    assert_eq!(cart.read_chr(0x0000), 0x82);
    assert_eq!(cart.mirroring(), Mirroring::Horizontal);
    assert!(cart.irq_pending());
}

#[test]
fn mapper_37_selects_prg_and_chr_windows_from_prg_ram_latch() {
    let mut cart = make_mmc3_mixed_chr_cart(37, 32, 256, 0);

    cart.write_prg(0x8000, 0x06);
    cart.write_prg(0x8001, 0x05);
    cart.write_prg(0x8000, 0x07);
    cart.write_prg(0x8001, 0x04);
    cart.write_prg(0x8000, 0x02);
    cart.write_prg(0x8001, 0x05);

    assert_eq!(cart.read_prg(0x8000), 5);
    assert_eq!(cart.read_prg(0xA000), 4);
    assert_eq!(cart.read_prg(0xC000), 6);
    assert_eq!(cart.read_prg(0xE000), 7);
    assert_eq!(cart.read_chr(0x1000), 0x55);

    cart.write_prg_ram(0x6000, 0x03);
    assert_eq!(cart.read_prg(0x8000), 13);
    assert_eq!(cart.read_prg(0xA000), 12);
    assert_eq!(cart.read_prg(0xC000), 14);
    assert_eq!(cart.read_prg(0xE000), 15);
    assert_eq!(cart.read_chr(0x1000), 0x55);

    cart.write_prg_ram(0x6000, 0x04);
    assert_eq!(cart.read_prg(0x8000), 21);
    assert_eq!(cart.read_prg(0xA000), 20);
    assert_eq!(cart.read_prg(0xC000), 30);
    assert_eq!(cart.read_prg(0xE000), 31);
    assert_eq!(cart.read_chr(0x1000), 0xD5);

    let snapshot = cart.snapshot_state();
    cart.write_prg_ram(0x6000, 0x00);
    cart.restore_state(&snapshot);
    assert_eq!(cart.read_prg(0x8000), 21);
    assert_eq!(cart.read_chr(0x1000), 0xD5);
}

#[test]
fn mapper_47_switches_128k_blocks_only_when_prg_ram_is_writable() {
    let mut cart = make_mmc3_mixed_chr_cart(47, 32, 256, 0);

    cart.write_prg(0x8000, 0x06);
    cart.write_prg(0x8001, 0x05);
    cart.write_prg(0x8000, 0x07);
    cart.write_prg(0x8001, 0x04);
    cart.write_prg(0x8000, 0x02);
    cart.write_prg(0x8001, 0x05);

    cart.write_prg(0xA001, 0x00);
    cart.write_prg_ram(0x6000, 0x01);
    assert_eq!(cart.read_prg(0x8000), 5);
    assert_eq!(cart.read_chr(0x1000), 0x55);

    cart.write_prg(0xA001, 0x80);
    cart.write_prg_ram(0x6000, 0x01);
    assert_eq!(cart.read_prg(0x8000), 21);
    assert_eq!(cart.read_prg(0xA000), 20);
    assert_eq!(cart.read_prg(0xC000), 30);
    assert_eq!(cart.read_prg(0xE000), 31);
    assert_eq!(cart.read_chr(0x1000), 0xD5);

    let snapshot = cart.snapshot_state();
    cart.write_prg_ram(0x6000, 0x00);
    cart.restore_state(&snapshot);
    assert_eq!(cart.read_prg(0x8000), 21);
    assert_eq!(cart.read_chr(0x1000), 0xD5);
}

#[test]
fn mapper_48_uses_taito_banking_and_delayed_irq() {
    let mut cart = make_mapper48_cart();

    cart.write_prg(0x8000, 0x03);
    cart.write_prg(0x8001, 0x04);
    cart.write_prg(0x8002, 0x05);
    cart.write_prg(0x8003, 0x06);
    cart.write_prg(0xA000, 0x07);
    cart.write_prg(0xA001, 0x08);
    cart.write_prg(0xA002, 0x09);
    cart.write_prg(0xA003, 0x0A);

    assert_eq!(cart.read_prg(0x8000), 3);
    assert_eq!(cart.read_prg(0xA000), 4);
    assert_eq!(cart.read_prg(0xC000), 14);
    assert_eq!(cart.read_prg(0xE000), 15);
    assert_eq!(cart.read_chr(0x0000), 0x8A);
    assert_eq!(cart.read_chr(0x0400), 0x8B);
    assert_eq!(cart.read_chr(0x0800), 0x8C);
    assert_eq!(cart.read_chr(0x0C00), 0x8D);
    assert_eq!(cart.read_chr(0x1000), 0x87);
    assert_eq!(cart.read_chr(0x1C00), 0x8A);

    cart.write_prg(0xE000, 0x40);
    assert_eq!(cart.mirroring(), Mirroring::Horizontal);

    cart.write_prg(0xC000, 0xFE);
    cart.write_prg(0xC001, 0x00);
    cart.write_prg(0xC002, 0x00);
    cart.clock_irq_counter();
    assert!(!cart.irq_pending());
    cart.clock_irq_counter();
    assert!(!cart.irq_pending());
    cart.clock_irq_counter_cycles(3);
    assert!(!cart.irq_pending());

    let snapshot = cart.snapshot_state();
    cart.clock_irq_counter_cycles(1);
    assert!(cart.irq_pending());

    cart.write_prg(0xC003, 0x00);
    assert!(!cart.irq_pending());

    cart.restore_state(&snapshot);
    assert!(!cart.irq_pending());
    cart.clock_irq_counter_cycles(1);
    assert!(cart.irq_pending());
}

#[test]
fn mapper_114_scrambles_registers_and_supports_override_modes() {
    let mut cart = make_mapper114_cart(114);

    cart.write_prg(0xA000, 0x04);
    cart.write_prg(0xC000, 0x03);
    cart.write_prg(0xA000, 0x05);
    cart.write_prg(0xC000, 0x04);
    cart.write_prg(0xA000, 0x06);
    cart.write_prg(0xC000, 0x05);
    cart.write_prg_ram(0x6001, 0x01);

    assert_eq!(cart.read_prg(0x8000), 3);
    assert_eq!(cart.read_prg(0xA000), 4);
    assert_eq!(cart.read_prg(0xC000), 30);
    assert_eq!(cart.read_prg(0xE000), 31);
    assert_eq!(cart.read_chr(0x1000), 0x85);

    cart.write_prg_ram(0x6000, 0x83);
    assert_eq!(cart.read_prg(0x8000), 6);
    assert_eq!(cart.read_prg(0xC000), 6);

    cart.write_prg_ram(0x6000, 0xC2);
    assert_eq!(cart.read_prg(0x8000), 4);
    assert_eq!(cart.read_prg(0xC000), 6);

    let snapshot = cart.snapshot_state();
    cart.write_prg_ram(0x6000, 0x00);
    cart.write_prg_ram(0x6001, 0x00);
    cart.restore_state(&snapshot);
    assert_eq!(cart.read_prg(0x8000), 4);
    assert_eq!(cart.read_prg(0xC000), 6);
    assert_eq!(cart.read_chr(0x1000), 0x85);
}

#[test]
fn mapper_123_uses_scrambled_bank_select_and_5800_override() {
    let mut cart = make_mapper123_cart();

    cart.write_prg(0x8000, 0x04);
    cart.write_prg(0x8001, 0x03);
    cart.write_prg(0x8000, 0x05);
    cart.write_prg(0x8001, 0x04);
    cart.write_prg(0x8000, 0x06);
    cart.write_prg(0x8001, 0x05);

    assert_eq!(cart.read_prg(0x8000), 3);
    assert_eq!(cart.read_prg(0xA000), 4);
    assert_eq!(cart.read_prg(0xC000), 30);
    assert_eq!(cart.read_prg(0xE000), 31);
    assert_eq!(cart.read_chr(0x1000), 0x65);

    cart.write_prg(0x5800, 0x40);
    assert_eq!(cart.read_prg(0x8000), 0);
    assert_eq!(cart.read_prg(0xC000), 0);

    cart.write_prg(0x5800, 0x42);
    assert_eq!(cart.read_prg(0x8000), 0);
    assert_eq!(cart.read_prg(0xC000), 2);

    let snapshot = cart.snapshot_state();
    cart.write_prg(0x5800, 0x00);
    cart.restore_state(&snapshot);
    assert_eq!(cart.read_prg(0x8000), 0);
    assert_eq!(cart.read_prg(0xC000), 2);
}

#[test]
fn mapper_182_aliases_114_and_uses_mmc3a_irq_behavior() {
    let mut cart = make_mapper114_cart(182);

    cart.write_prg(0xA001, 0x00);
    cart.write_prg(0xC001, 0x00);
    cart.write_prg(0xE001, 0x00);
    cart.clock_irq_counter();
    cart.clock_irq_counter();
    assert!(!cart.irq_pending());

    cart.write_prg(0xA001, 0x01);
    cart.write_prg(0xC001, 0x00);
    cart.write_prg(0xE001, 0x00);
    cart.clock_irq_counter();
    assert!(!cart.irq_pending());
    cart.clock_irq_counter();
    assert!(cart.irq_pending());

    cart.write_prg(0xE000, 0x00);
    assert!(!cart.irq_pending());
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
    ppu.write_register(0x2007, 0x77, Some(&mut cart));
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
    ppu.write_register(0x2007, 0x77, Some(&mut cart));
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
    ppu.write_register(0x2007, 0x99, Some(&mut cart));
    assert_eq!(ppu.nametable[0][0], 0x11);
}

#[test]
fn mapper_112_uses_hardwired_prg_layout_and_2k_chr_banks() {
    let mut cart = make_namco108_cart(112, 8, 32);

    cart.write_prg(0x8000, 0x00);
    cart.write_prg(0xA000, 0x03);
    cart.write_prg(0x8000, 0x01);
    cart.write_prg(0xA000, 0x04);
    cart.write_prg(0x8000, 0x02);
    cart.write_prg(0xA000, 0x05);
    cart.write_prg(0x8000, 0x03);
    cart.write_prg(0xA000, 0x06);
    cart.write_prg(0x8000, 0x04);
    cart.write_prg(0xA000, 0x07);
    cart.write_prg(0x8000, 0x05);
    cart.write_prg(0xA000, 0x08);
    cart.write_prg(0x8000, 0x06);
    cart.write_prg(0xA000, 0x09);
    cart.write_prg(0x8000, 0x07);
    cart.write_prg(0xA000, 0x0A);
    cart.write_prg(0xE000, 0x01);

    assert_eq!(cart.read_prg(0x8000), 3);
    assert_eq!(cart.read_prg(0xA000), 4);
    assert_eq!(cart.read_prg(0xC000), 6);
    assert_eq!(cart.read_prg(0xE000), 7);
    assert_eq!(cart.read_chr(0x0000), 0x2A);
    assert_eq!(cart.read_chr(0x0400), 0x2B);
    assert_eq!(cart.read_chr(0x0800), 0x2C);
    assert_eq!(cart.read_chr(0x0C00), 0x2D);
    assert_eq!(cart.read_chr(0x1000), 0x27);
    assert_eq!(cart.read_chr(0x1400), 0x28);
    assert_eq!(cart.read_chr(0x1800), 0x29);
    assert_eq!(cart.read_chr(0x1C00), 0x2A);
    assert_eq!(cart.mirroring(), Mirroring::Horizontal);

    let snapshot = cart.snapshot_state();
    cart.write_prg(0x8000, 0x00);
    cart.write_prg(0xA000, 0x00);
    cart.write_prg(0xE000, 0x00);
    cart.restore_state(&snapshot);

    assert_eq!(cart.read_prg(0x8000), 3);
    assert_eq!(cart.read_chr(0x1000), 0x27);
    assert_eq!(cart.mirroring(), Mirroring::Horizontal);
}

#[test]
fn mapper_115_supports_outer_chr_and_nrom_override_modes() {
    let mut cart = make_mapper115_cart(115);

    cart.write_prg(0x8000, 0x06);
    cart.write_prg(0x8001, 0x03);
    cart.write_prg(0x8000, 0x07);
    cart.write_prg(0x8001, 0x04);
    cart.write_prg(0x8000, 0x02);
    cart.write_prg(0x8001, 0x05);

    assert_eq!(cart.read_prg(0x8000), 3);
    assert_eq!(cart.read_prg(0xA000), 4);

    cart.write_prg_ram(0x6001, 0x01);
    assert_eq!(cart.read_chr(0x1000), 0x85);

    cart.write_prg_ram(0x6000, 0x83);
    assert_eq!(cart.read_prg(0x8000), 6);
    assert_eq!(cart.read_prg(0xC000), 6);

    cart.write_prg_ram(0x6000, 0xA2);
    assert_eq!(cart.read_prg(0x8000), 4);
    assert_eq!(cart.read_prg(0xC000), 6);
    assert_eq!(cart.read_prg_ram(0x6002), 0);

    let snapshot = cart.snapshot_state();
    cart.write_prg_ram(0x6000, 0x00);
    cart.write_prg_ram(0x6001, 0x00);
    cart.restore_state(&snapshot);

    assert_eq!(cart.read_prg(0x8000), 4);
    assert_eq!(cart.read_prg(0xC000), 6);
    assert_eq!(cart.read_chr(0x1000), 0x85);
}

#[test]
fn mapper_248_aliases_115_behavior() {
    let mut cart = make_mapper115_cart(248);

    cart.write_prg(0x8000, 0x06);
    cart.write_prg(0x8001, 0x01);
    cart.write_prg(0x8000, 0x07);
    cart.write_prg(0x8001, 0x02);
    cart.write_prg(0x8000, 0x02);
    cart.write_prg(0x8001, 0x04);
    cart.write_prg_ram(0x6001, 0x01);
    cart.write_prg_ram(0x6000, 0x81);

    assert_eq!(cart.read_prg(0x8000), 2);
    assert_eq!(cart.read_prg(0xC000), 2);
    assert_eq!(cart.read_chr(0x1000), 0x84);
    assert_eq!(cart.read_prg_ram(0x6002), 0);
}

#[test]
fn mapper_205_selects_outer_prg_chr_blocks_and_restores_state() {
    let mut cart = make_mapper205_cart();

    cart.write_prg(0x8000, 0x06);
    cart.write_prg(0x8001, 0x13);
    cart.write_prg(0x8000, 0x07);
    cart.write_prg(0x8001, 0x14);
    cart.write_prg(0x8000, 0x00);
    cart.write_prg(0x8001, 0x86);
    cart.write_prg(0x8000, 0x01);
    cart.write_prg(0x8001, 0x88);
    cart.write_prg(0x8000, 0x02);
    cart.write_prg(0x8001, 0x8A);
    cart.write_prg(0x8000, 0x03);
    cart.write_prg(0x8001, 0x8B);
    cart.write_prg(0x8000, 0x04);
    cart.write_prg(0x8001, 0x8C);
    cart.write_prg(0x8000, 0x05);
    cart.write_prg(0x8001, 0x8D);

    assert_eq!(cart.read_prg(0x8000), 19);
    assert_eq!(cart.read_prg(0xA000), 20);
    assert_eq!(cart.read_prg(0xC000), 30);
    assert_eq!(cart.read_prg(0xE000), 31);
    assert_eq!(cart.read_chr(0x0000), 0x46);
    assert_eq!(cart.read_chr(0x0400), 0x47);
    assert_eq!(cart.read_chr(0x0800), 0x48);
    assert_eq!(cart.read_chr(0x1000), 0x4A);
    assert_eq!(cart.read_chr(0x1C00), 0x4D);

    cart.write_prg_ram(0x6000, 0x02);
    assert_eq!(cart.read_prg(0x8000), 35);
    assert_eq!(cart.read_prg(0xA000), 36);
    assert_eq!(cart.read_prg(0xC000), 46);
    assert_eq!(cart.read_prg(0xE000), 47);
    assert_eq!(cart.read_chr(0x0000), 0x86);
    assert_eq!(cart.read_chr(0x1000), 0x8A);
    assert_eq!(cart.read_chr(0x1C00), 0x8D);

    cart.write_prg_ram(0x6000, 0x03);
    assert_eq!(cart.read_prg(0x8000), 51);
    assert_eq!(cart.read_prg(0xA000), 52);
    assert_eq!(cart.read_prg(0xC000), 62);
    assert_eq!(cart.read_prg(0xE000), 63);
    assert_eq!(cart.read_chr(0x0000), 0xC6);
    assert_eq!(cart.read_chr(0x1000), 0xCA);
    assert_eq!(cart.read_chr(0x1C00), 0xCD);

    let snapshot = cart.snapshot_state();
    cart.write_prg_ram(0x6000, 0x00);
    cart.restore_state(&snapshot);

    assert_eq!(cart.read_prg(0x8000), 51);
    assert_eq!(cart.read_chr(0x1000), 0xCA);
}

#[test]
fn mapper_12_uses_split_outer_chr_bits_with_mmc3_prg_layout() {
    let mut cart = make_mapper12_cart();

    cart.write_prg(0x8000, 0x06);
    cart.write_prg(0x8001, 0x03);
    cart.write_prg(0x8000, 0x07);
    cart.write_prg(0x8001, 0x04);
    cart.write_prg(0x8000, 0x00);
    cart.write_prg(0x8001, 0x04);
    cart.write_prg(0x8000, 0x01);
    cart.write_prg(0x8001, 0x06);
    cart.write_prg(0x8000, 0x02);
    cart.write_prg(0x8001, 0x08);
    cart.write_prg(0x8000, 0x03);
    cart.write_prg(0x8001, 0x09);
    cart.write_prg(0x8000, 0x04);
    cart.write_prg(0x8001, 0x0A);
    cart.write_prg(0x8000, 0x05);
    cart.write_prg(0x8001, 0x0B);
    cart.write_prg(0xA001, 0x11);

    assert_eq!(cart.read_prg(0x8000), 3);
    assert_eq!(cart.read_prg(0xA000), 4);
    assert_eq!(cart.read_prg(0xC000), 30);
    assert_eq!(cart.read_prg(0xE000), 31);
    assert_eq!(cart.read_chr(0x0000), 0x84);
    assert_eq!(cart.read_chr(0x0400), 0x85);
    assert_eq!(cart.read_chr(0x0800), 0x86);
    assert_eq!(cart.read_chr(0x0C00), 0x87);
    assert_eq!(cart.read_chr(0x1000), 0x88);
    assert_eq!(cart.read_chr(0x1C00), 0x8B);

    let snapshot = cart.snapshot_state();
    cart.write_prg(0xA001, 0x00);
    cart.restore_state(&snapshot);

    assert_eq!(cart.read_chr(0x0000), 0x84);
    assert_eq!(cart.read_chr(0x1000), 0x88);
}

#[test]
fn mapper_44_switches_outer_prg_chr_windows() {
    let mut cart = make_mapper44_cart();

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
    cart.write_prg(0xA000, 0x01);

    assert_eq!(cart.read_prg(0x8000), 3);
    assert_eq!(cart.read_prg(0xA000), 4);
    assert_eq!(cart.read_prg(0xC000), 14);
    assert_eq!(cart.read_prg(0xE000), 15);
    assert_eq!(cart.read_chr(0x0000), 0x06);
    assert_eq!(cart.read_chr(0x1000), 0x0A);
    assert_eq!(cart.mirroring(), Mirroring::Horizontal);

    cart.write_prg(0xA001, 0x02);
    assert_eq!(cart.read_prg(0x8000), 35);
    assert_eq!(cart.read_prg(0xA000), 36);
    assert_eq!(cart.read_prg(0xC000), 46);
    assert_eq!(cart.read_prg(0xE000), 47);
    assert_eq!(cart.read_chr(0x0000), 0x46);
    assert_eq!(cart.read_chr(0x1000), 0x4A);

    cart.write_prg(0xA001, 0x07);
    assert_eq!(cart.read_prg(0x8000), 115);
    assert_eq!(cart.read_prg(0xA000), 116);
    assert_eq!(cart.read_prg(0xC000), 126);
    assert_eq!(cart.read_prg(0xE000), 127);
    assert_eq!(cart.read_chr(0x0000), 0xE6);
    assert_eq!(cart.read_chr(0x1000), 0xEA);

    let snapshot = cart.snapshot_state();
    cart.write_prg(0xA001, 0x00);
    cart.restore_state(&snapshot);

    assert_eq!(cart.read_prg(0x8000), 115);
    assert_eq!(cart.read_chr(0x1000), 0xEA);
    assert_eq!(cart.mirroring(), Mirroring::Horizontal);
}
