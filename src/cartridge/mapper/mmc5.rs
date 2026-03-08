use super::super::Cartridge;
use std::cell::Cell;

#[derive(Debug, Clone)]
pub(in crate::cartridge) struct Mmc5 {
    pub(in crate::cartridge) prg_mode: u8,
    pub(in crate::cartridge) chr_mode: u8,
    pub(in crate::cartridge) exram_mode: u8,
    pub(in crate::cartridge) prg_ram_protect_1: u8,
    pub(in crate::cartridge) prg_ram_protect_2: u8,
    pub(in crate::cartridge) nametable_map: [u8; 4],
    pub(in crate::cartridge) fill_tile: u8,
    pub(in crate::cartridge) fill_attr: u8,
    pub(in crate::cartridge) prg_ram_bank: u8,
    pub(in crate::cartridge) prg_banks: [u8; 4],
    pub(in crate::cartridge) chr_upper: u8,
    pub(in crate::cartridge) sprite_chr_banks: [u8; 8],
    pub(in crate::cartridge) bg_chr_banks: [u8; 4],
    pub(in crate::cartridge) exram: Vec<u8>,
    pub(in crate::cartridge) irq_scanline_compare: u8,
    pub(in crate::cartridge) irq_enabled: bool,
    pub(in crate::cartridge) irq_pending: Cell<bool>,
    pub(in crate::cartridge) in_frame: Cell<bool>,
    pub(in crate::cartridge) scanline_counter: Cell<u8>,
    pub(in crate::cartridge) multiplier_a: u8,
    pub(in crate::cartridge) multiplier_b: u8,
    pub(in crate::cartridge) split_control: u8,
    pub(in crate::cartridge) split_scroll: u8,
    pub(in crate::cartridge) split_bank: u8,
    pub(in crate::cartridge) ppu_ctrl: Cell<u8>,
    pub(in crate::cartridge) ppu_mask: Cell<u8>,
    pub(in crate::cartridge) cached_tile_x: Cell<u8>,
    pub(in crate::cartridge) cached_tile_y: Cell<u8>,
    pub(in crate::cartridge) cached_ext_palette: Cell<u8>,
    pub(in crate::cartridge) cached_ext_bank: Cell<u8>,
}

impl Mmc5 {
    pub(in crate::cartridge) fn new() -> Self {
        Self {
            prg_mode: 3,
            chr_mode: 3,
            exram_mode: 0,
            prg_ram_protect_1: 0,
            prg_ram_protect_2: 0,
            nametable_map: [0, 1, 0, 1],
            fill_tile: 0,
            fill_attr: 0,
            prg_ram_bank: 0,
            prg_banks: [0x00, 0x80, 0x80, 0x7F],
            chr_upper: 0,
            sprite_chr_banks: [0; 8],
            bg_chr_banks: [0; 4],
            exram: vec![0; 1024],
            irq_scanline_compare: 0,
            irq_enabled: false,
            irq_pending: Cell::new(false),
            in_frame: Cell::new(false),
            scanline_counter: Cell::new(0),
            multiplier_a: 0,
            multiplier_b: 0,
            split_control: 0,
            split_scroll: 0,
            split_bank: 0,
            ppu_ctrl: Cell::new(0),
            ppu_mask: Cell::new(0),
            cached_tile_x: Cell::new(0),
            cached_tile_y: Cell::new(0),
            cached_ext_palette: Cell::new(0),
            cached_ext_bank: Cell::new(0),
        }
    }

    fn substitutions_enabled(&self) -> bool {
        self.ppu_mask.get() & 0x18 != 0
    }

    fn prg_ram_write_enabled(&self) -> bool {
        self.prg_ram_protect_1 == 0x02 && self.prg_ram_protect_2 == 0x01
    }
}

impl Cartridge {
    fn mmc5_prg_rom_banks_8k(&self) -> usize {
        (self.prg_rom.len() / 0x2000).max(1)
    }

    fn mmc5_prg_ram_banks_8k(&self) -> usize {
        (self.prg_ram.len() / 0x2000).max(1)
    }

    fn mmc5_chr_len(&self) -> usize {
        if !self.chr_rom.is_empty() {
            self.chr_rom.len()
        } else {
            self.chr_ram.len()
        }
    }

    fn mmc5_prg_target(&self, raw_bank: u8, size_8k: usize, offset: usize, rom_only: bool) -> u8 {
        let offset_in_bank = offset & (size_8k * 0x2000 - 1);
        if !rom_only && raw_bank & 0x80 == 0 {
            if self.prg_ram.is_empty() {
                return 0;
            }
            let bank_count = self.mmc5_prg_ram_banks_8k();
            let bank_base = ((raw_bank as usize) & !((size_8k - 1).max(1) - 1)) % bank_count;
            let ram_addr = bank_base * 0x2000 + offset_in_bank;
            return self.prg_ram[ram_addr % self.prg_ram.len()];
        }

        let bank_count = self.mmc5_prg_rom_banks_8k();
        let bank_base = (((raw_bank & 0x7F) as usize) & !((size_8k - 1).max(1) - 1)) % bank_count;
        let rom_addr = bank_base * 0x2000 + offset_in_bank;
        self.prg_rom[rom_addr % self.prg_rom.len()]
    }

    fn write_mmc5_prg_target(
        &mut self,
        raw_bank: u8,
        size_8k: usize,
        offset: usize,
        data: u8,
        rom_only: bool,
    ) {
        let Some(mmc5) = self.mmc5.as_ref() else {
            return;
        };
        if rom_only
            || raw_bank & 0x80 != 0
            || !mmc5.prg_ram_write_enabled()
            || self.prg_ram.is_empty()
        {
            return;
        }

        let offset_in_bank = offset & (size_8k * 0x2000 - 1);
        let bank_count = self.mmc5_prg_ram_banks_8k();
        let bank_base = ((raw_bank as usize) & !((size_8k - 1).max(1) - 1)) % bank_count;
        let ram_addr = bank_base * 0x2000 + offset_in_bank;
        if let Some(slot) = self.prg_ram.get_mut(ram_addr) {
            *slot = data;
        }
    }

    fn mmc5_chr_bank_1k(&self, page: usize, sprite: bool) -> usize {
        let Some(mmc5) = self.mmc5.as_ref() else {
            return page;
        };

        let upper = (mmc5.chr_upper as usize) << 8;
        let raw = if sprite || !mmc5.substitutions_enabled() || (mmc5.ppu_ctrl.get() & 0x20) == 0 {
            match mmc5.chr_mode & 0x03 {
                0 => mmc5.sprite_chr_banks[7] as usize,
                1 => mmc5.sprite_chr_banks[if page < 4 { 3 } else { 7 }] as usize,
                2 => mmc5.sprite_chr_banks[(page | 1) & 7] as usize,
                _ => mmc5.sprite_chr_banks[page & 7] as usize,
            }
        } else {
            mmc5.bg_chr_banks[page & 3] as usize
        };

        let (unit_pages, local_page) =
            if sprite || !mmc5.substitutions_enabled() || (mmc5.ppu_ctrl.get() & 0x20) == 0 {
                match mmc5.chr_mode & 0x03 {
                    0 => (8, page & 7),
                    1 => (4, page & 3),
                    2 => (2, page & 1),
                    _ => (1, 0),
                }
            } else {
                (1, 0)
            };

        (upper | raw) * unit_pages + local_page
    }

    fn read_mmc5_chr_1k(&self, bank_1k: usize, local_offset: usize) -> u8 {
        let len = self.mmc5_chr_len();
        if len == 0 {
            return 0;
        }
        let addr = (bank_1k * 0x0400 + local_offset) % len;
        if !self.chr_rom.is_empty() {
            self.chr_rom[addr]
        } else {
            self.chr_ram[addr]
        }
    }

    fn write_mmc5_chr_1k(&mut self, bank_1k: usize, local_offset: usize, data: u8) {
        let len = self.mmc5_chr_len();
        if len == 0 {
            return;
        }
        let addr = (bank_1k * 0x0400 + local_offset) % len;
        if !self.chr_rom.is_empty() {
            if let Some(slot) = self.chr_rom.get_mut(addr) {
                *slot = data;
            }
        } else if let Some(slot) = self.chr_ram.get_mut(addr) {
            *slot = data;
        }
    }

    fn mmc5_fill_attribute(&self) -> u8 {
        let Some(mmc5) = self.mmc5.as_ref() else {
            return 0;
        };
        let attr = mmc5.fill_attr & 0x03;
        attr | (attr << 2) | (attr << 4) | (attr << 6)
    }

    fn mmc5_exram_palette_attr(&self) -> u8 {
        let Some(mmc5) = self.mmc5.as_ref() else {
            return 0;
        };
        let palette = mmc5.cached_ext_palette.get() & 0x03;
        let tile_x = mmc5.cached_tile_x.get() as usize;
        let tile_y = mmc5.cached_tile_y.get() as usize;
        let block_x = (tile_x & 3) >> 1;
        let block_y = (tile_y & 3) >> 1;
        let shift = (block_y * 2 + block_x) * 2;
        palette << shift
    }

    pub(in crate::cartridge) fn read_prg_mmc5(&self, addr: u16) -> u8 {
        let Some(mmc5) = self.mmc5.as_ref() else {
            return 0;
        };

        match mmc5.prg_mode & 0x03 {
            0 => self.mmc5_prg_target(mmc5.prg_banks[3], 4, (addr - 0x8000) as usize, true),
            1 => {
                if addr < 0xC000 {
                    self.mmc5_prg_target(mmc5.prg_banks[1], 2, (addr - 0x8000) as usize, false)
                } else {
                    self.mmc5_prg_target(mmc5.prg_banks[3], 2, (addr - 0xC000) as usize, true)
                }
            }
            2 => {
                if addr < 0xC000 {
                    self.mmc5_prg_target(mmc5.prg_banks[1], 2, (addr - 0x8000) as usize, false)
                } else if addr < 0xE000 {
                    self.mmc5_prg_target(mmc5.prg_banks[2], 1, (addr - 0xC000) as usize, false)
                } else {
                    self.mmc5_prg_target(mmc5.prg_banks[3], 1, (addr - 0xE000) as usize, true)
                }
            }
            _ => match addr {
                0x8000..=0x9FFF => {
                    self.mmc5_prg_target(mmc5.prg_banks[0], 1, (addr - 0x8000) as usize, false)
                }
                0xA000..=0xBFFF => {
                    self.mmc5_prg_target(mmc5.prg_banks[1], 1, (addr - 0xA000) as usize, false)
                }
                0xC000..=0xDFFF => {
                    self.mmc5_prg_target(mmc5.prg_banks[2], 1, (addr - 0xC000) as usize, false)
                }
                _ => self.mmc5_prg_target(mmc5.prg_banks[3], 1, (addr - 0xE000) as usize, true),
            },
        }
    }

    pub(in crate::cartridge) fn write_prg_mmc5(&mut self, addr: u16, data: u8) {
        match addr {
            0x5100 => {
                if let Some(mmc5) = self.mmc5.as_mut() {
                    mmc5.prg_mode = data & 0x03;
                }
            }
            0x5101 => {
                if let Some(mmc5) = self.mmc5.as_mut() {
                    mmc5.chr_mode = data & 0x03;
                }
            }
            0x5102 => {
                if let Some(mmc5) = self.mmc5.as_mut() {
                    mmc5.prg_ram_protect_1 = data & 0x03;
                }
            }
            0x5103 => {
                if let Some(mmc5) = self.mmc5.as_mut() {
                    mmc5.prg_ram_protect_2 = data & 0x03;
                }
            }
            0x5104 => {
                if let Some(mmc5) = self.mmc5.as_mut() {
                    mmc5.exram_mode = data & 0x03;
                }
            }
            0x5105 => {
                if let Some(mmc5) = self.mmc5.as_mut() {
                    for index in 0..4 {
                        mmc5.nametable_map[index] = (data >> (index * 2)) & 0x03;
                    }
                }
            }
            0x5106 => {
                if let Some(mmc5) = self.mmc5.as_mut() {
                    mmc5.fill_tile = data;
                }
            }
            0x5107 => {
                if let Some(mmc5) = self.mmc5.as_mut() {
                    mmc5.fill_attr = data & 0x03;
                }
            }
            0x5113 => {
                if let Some(mmc5) = self.mmc5.as_mut() {
                    mmc5.prg_ram_bank = data & 0x0F;
                }
            }
            0x5114..=0x5117 => {
                if let Some(mmc5) = self.mmc5.as_mut() {
                    mmc5.prg_banks[(addr - 0x5114) as usize] = data;
                }
            }
            0x5120..=0x5127 => {
                if let Some(mmc5) = self.mmc5.as_mut() {
                    mmc5.sprite_chr_banks[(addr - 0x5120) as usize] = data;
                }
            }
            0x5128..=0x512B => {
                if let Some(mmc5) = self.mmc5.as_mut() {
                    mmc5.bg_chr_banks[(addr - 0x5128) as usize] = data;
                }
            }
            0x5130 => {
                if let Some(mmc5) = self.mmc5.as_mut() {
                    mmc5.chr_upper = data & 0x03;
                }
            }
            0x5200 => {
                if let Some(mmc5) = self.mmc5.as_mut() {
                    mmc5.split_control = data;
                }
            }
            0x5201 => {
                if let Some(mmc5) = self.mmc5.as_mut() {
                    mmc5.split_scroll = data;
                }
            }
            0x5202 => {
                if let Some(mmc5) = self.mmc5.as_mut() {
                    mmc5.split_bank = data;
                }
            }
            0x5203 => {
                if let Some(mmc5) = self.mmc5.as_mut() {
                    mmc5.irq_scanline_compare = data;
                }
            }
            0x5204 => {
                if let Some(mmc5) = self.mmc5.as_mut() {
                    mmc5.irq_enabled = data & 0x80 != 0;
                    if !mmc5.irq_enabled {
                        mmc5.irq_pending.set(false);
                    }
                }
            }
            0x5205 => {
                if let Some(mmc5) = self.mmc5.as_mut() {
                    mmc5.multiplier_a = data;
                }
            }
            0x5206 => {
                if let Some(mmc5) = self.mmc5.as_mut() {
                    mmc5.multiplier_b = data;
                }
            }
            0x5C00..=0x5FFF => {
                if let Some(mmc5) = self.mmc5.as_mut() {
                    if mmc5.exram_mode != 0x03 {
                        let exram_addr = (addr - 0x5C00) as usize;
                        if let Some(slot) = mmc5.exram.get_mut(exram_addr) {
                            *slot = data;
                        }
                    }
                }
            }
            0x6000..=0x7FFF => self.write_prg_ram_mmc5(addr, data),
            0x8000..=0xFFFF => {
                let Some((prg_mode, prg_banks)) = self
                    .mmc5
                    .as_ref()
                    .map(|mmc5| (mmc5.prg_mode & 0x03, mmc5.prg_banks))
                else {
                    return;
                };
                match prg_mode {
                    0 => self.write_mmc5_prg_target(
                        prg_banks[3],
                        4,
                        (addr - 0x8000) as usize,
                        data,
                        true,
                    ),
                    1 => {
                        if addr < 0xC000 {
                            self.write_mmc5_prg_target(
                                prg_banks[1],
                                2,
                                (addr - 0x8000) as usize,
                                data,
                                false,
                            );
                        }
                    }
                    2 => {
                        if addr < 0xC000 {
                            self.write_mmc5_prg_target(
                                prg_banks[1],
                                2,
                                (addr - 0x8000) as usize,
                                data,
                                false,
                            );
                        } else if addr < 0xE000 {
                            self.write_mmc5_prg_target(
                                prg_banks[2],
                                1,
                                (addr - 0xC000) as usize,
                                data,
                                false,
                            );
                        }
                    }
                    _ => match addr {
                        0x8000..=0x9FFF => self.write_mmc5_prg_target(
                            prg_banks[0],
                            1,
                            (addr - 0x8000) as usize,
                            data,
                            false,
                        ),
                        0xA000..=0xBFFF => self.write_mmc5_prg_target(
                            prg_banks[1],
                            1,
                            (addr - 0xA000) as usize,
                            data,
                            false,
                        ),
                        0xC000..=0xDFFF => self.write_mmc5_prg_target(
                            prg_banks[2],
                            1,
                            (addr - 0xC000) as usize,
                            data,
                            false,
                        ),
                        _ => {}
                    },
                }
            }
            _ => {}
        }
    }

    pub(in crate::cartridge) fn read_prg_low_mmc5(&self, addr: u16) -> u8 {
        let Some(mmc5) = self.mmc5.as_ref() else {
            return 0;
        };

        match addr {
            0x5204 => {
                let mut status = 0;
                if mmc5.in_frame.get() {
                    status |= 0x40;
                }
                if mmc5.irq_pending.get() {
                    status |= 0x80;
                    mmc5.irq_pending.set(false);
                }
                status
            }
            0x5205 => {
                let product = (mmc5.multiplier_a as u16) * (mmc5.multiplier_b as u16);
                product as u8
            }
            0x5206 => {
                let product = (mmc5.multiplier_a as u16) * (mmc5.multiplier_b as u16);
                (product >> 8) as u8
            }
            0x5C00..=0x5FFF => match mmc5.exram_mode {
                0x02 | 0x03 => mmc5.exram[(addr - 0x5C00) as usize],
                _ => 0,
            },
            _ => 0,
        }
    }

    pub(in crate::cartridge) fn read_prg_ram_mmc5(&self, addr: u16) -> u8 {
        let Some(mmc5) = self.mmc5.as_ref() else {
            return 0;
        };
        if self.prg_ram.is_empty() {
            return 0;
        }
        let bank = (mmc5.prg_ram_bank as usize) % self.mmc5_prg_ram_banks_8k();
        let ram_addr = bank * 0x2000 + (addr as usize & 0x1FFF);
        self.prg_ram[ram_addr % self.prg_ram.len()]
    }

    pub(in crate::cartridge) fn write_prg_ram_mmc5(&mut self, addr: u16, data: u8) {
        let Some(mmc5) = self.mmc5.as_ref() else {
            return;
        };
        if !mmc5.prg_ram_write_enabled() || self.prg_ram.is_empty() {
            return;
        }
        let bank = (mmc5.prg_ram_bank as usize) % self.mmc5_prg_ram_banks_8k();
        let ram_addr = bank * 0x2000 + (addr as usize & 0x1FFF);
        if let Some(slot) = self.prg_ram.get_mut(ram_addr) {
            *slot = data;
        }
    }

    pub(in crate::cartridge) fn read_chr_mmc5(&self, addr: u16) -> u8 {
        let Some(mmc5) = self.mmc5.as_ref() else {
            return 0;
        };

        if mmc5.substitutions_enabled() && mmc5.exram_mode == 0x01 {
            let bank_4k = mmc5.cached_ext_bank.get() as usize;
            let chr_addr = bank_4k * 0x1000 + (addr as usize & 0x0FFF);
            return self.read_mmc5_chr_1k(chr_addr >> 10, chr_addr & 0x03FF);
        }

        let page = ((addr as usize) >> 10) & 0x07;
        let local_offset = addr as usize & 0x03FF;
        self.read_mmc5_chr_1k(self.mmc5_chr_bank_1k(page, false), local_offset)
    }

    pub(in crate::cartridge) fn read_chr_sprite_mmc5(&self, addr: u16, _sprite_y: u8) -> u8 {
        let page = ((addr as usize) >> 10) & 0x07;
        let local_offset = addr as usize & 0x03FF;
        self.read_mmc5_chr_1k(self.mmc5_chr_bank_1k(page, true), local_offset)
    }

    pub(in crate::cartridge) fn write_chr_mmc5(&mut self, addr: u16, data: u8) {
        let page = ((addr as usize) >> 10) & 0x07;
        let local_offset = addr as usize & 0x03FF;
        self.write_mmc5_chr_1k(self.mmc5_chr_bank_1k(page, false), local_offset, data);
    }

    pub(in crate::cartridge) fn read_nametable_mmc5(
        &self,
        logical_nt: usize,
        offset: usize,
        internal: &[[u8; 1024]; 2],
    ) -> u8 {
        let Some(mmc5) = self.mmc5.as_ref() else {
            return 0;
        };

        let source = mmc5.nametable_map[logical_nt & 3] & 0x03;
        if offset < 960 {
            let tile_x = (offset & 31) as u8;
            let tile_y = (offset / 32) as u8;
            mmc5.cached_tile_x.set(tile_x);
            mmc5.cached_tile_y.set(tile_y);

            let tile = match source {
                0 => internal[0][offset],
                1 => internal[1][offset],
                2 if mmc5.exram_mode <= 0x01 => mmc5.exram[offset],
                3 => mmc5.fill_tile,
                _ => 0,
            };

            if mmc5.substitutions_enabled() && mmc5.exram_mode == 0x01 {
                let exattr = mmc5.exram[offset];
                mmc5.cached_ext_bank.set(exattr & 0x3F);
                mmc5.cached_ext_palette.set((exattr >> 6) & 0x03);
            } else {
                mmc5.cached_ext_bank.set(0);
                mmc5.cached_ext_palette.set(0);
            }

            tile
        } else {
            if mmc5.substitutions_enabled() && mmc5.exram_mode == 0x01 {
                return self.mmc5_exram_palette_attr();
            }
            match source {
                0 => internal[0][offset],
                1 => internal[1][offset],
                2 if mmc5.exram_mode <= 0x01 => mmc5.exram[offset],
                3 => self.mmc5_fill_attribute(),
                _ => 0,
            }
        }
    }

    pub(in crate::cartridge) fn write_nametable_mmc5(
        &mut self,
        logical_nt: usize,
        offset: usize,
        internal: &mut [[u8; 1024]; 2],
        data: u8,
    ) {
        let Some(mmc5) = self.mmc5.as_mut() else {
            return;
        };

        match mmc5.nametable_map[logical_nt & 3] & 0x03 {
            0 => internal[0][offset] = data,
            1 => internal[1][offset] = data,
            2 if mmc5.exram_mode != 0x03 => {
                if let Some(slot) = mmc5.exram.get_mut(offset) {
                    *slot = data;
                }
            }
            _ => {}
        }
    }

    pub(in crate::cartridge) fn resolve_nametable_mmc5(&self, logical_nt: usize) -> usize {
        logical_nt & 3
    }

    pub(crate) fn notify_ppuctrl_mmc5(&mut self, data: u8) {
        if let Some(mmc5) = self.mmc5.as_ref() {
            mmc5.ppu_ctrl.set(data);
        }
    }

    pub(crate) fn notify_ppumask_mmc5(&mut self, data: u8) {
        if let Some(mmc5) = self.mmc5.as_ref() {
            mmc5.ppu_mask.set(data);
            if data & 0x18 == 0 {
                mmc5.in_frame.set(false);
                mmc5.scanline_counter.set(0);
            }
        }
    }

    pub(crate) fn mmc5_scanline_tick(&self) {
        let Some(mmc5) = self.mmc5.as_ref() else {
            return;
        };
        if !mmc5.substitutions_enabled() {
            return;
        }
        let next_scanline = if mmc5.in_frame.get() {
            mmc5.scanline_counter.get().wrapping_add(1)
        } else {
            mmc5.in_frame.set(true);
            0
        };
        mmc5.scanline_counter.set(next_scanline);
        if mmc5.irq_enabled
            && mmc5.irq_scanline_compare != 0
            && next_scanline == mmc5.irq_scanline_compare
        {
            mmc5.irq_pending.set(true);
        }
    }

    pub(crate) fn mmc5_end_frame(&self) {
        if let Some(mmc5) = self.mmc5.as_ref() {
            mmc5.in_frame.set(false);
            mmc5.scanline_counter.set(0);
        }
    }
}
