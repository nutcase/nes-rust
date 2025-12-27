use super::Bus;
use crate::cartridge::MapperType;

impl Bus {
    pub(crate) fn sa1_bwram_addr(&self, offset: u16) -> Option<usize> {
        if self.sa1_bwram.is_empty() || offset < 0x6000 {
            return None;
        }
        let window_offset = (offset - 0x6000) as usize;
        let block = (self.sa1.registers.bwram_select_snes & 0x1F) as usize;
        let base = block << 13; // 8 KB blocks
        let idx = base.wrapping_add(window_offset) % self.sa1_bwram.len();
        Some(idx)
    }

    pub(crate) fn sa1_cpu_bwram_addr(&self, offset: u16) -> Option<usize> {
        if self.sa1_bwram.is_empty() || offset < 0x6000 {
            return None;
        }
        let window_offset = (offset - 0x6000) as usize;

        // Check bit 7 of bwram_select_sa1 for bitmap mode
        let select = self.sa1.registers.bwram_select_sa1;
        if (select & 0x80) != 0 {
            // Bitmap mode: use bits 0-6 to determine the 8KB block
            let block = (select & 0x7F) as usize;
            let base = block << 13; // 8 KB blocks
            let idx = base.wrapping_add(window_offset) % self.sa1_bwram.len();
            Some(idx)
        } else {
            // Normal mode: use bits 0-4 (5-bit block selector)
            let block = (select & 0x1F) as usize;
            let base = block << 13; // 8 KB blocks
            let idx = base.wrapping_add(window_offset) % self.sa1_bwram.len();
            Some(idx)
        }
    }

    /// SA-1専用のROM物理アドレス計算 (MMCバンク考慮)
    ///
    /// SA-1は4つの1MBチャンク（C/D/E/F）をLoROM/HiROM窓にマップする。
    /// デフォルトでは C=0, D=1, E=2, F=3 (1MB単位)。
    pub(crate) fn sa1_phys_addr(&self, bank: u32, offset: u16) -> usize {
        // Current MMC mapping
        let reg = &self.sa1.registers;
        let chunk_index = match bank {
            0x00..=0x1F => reg.mmc_bank_c,
            0x20..=0x3F => reg.mmc_bank_d,
            0x80..=0x9F => reg.mmc_bank_e,
            0xA0..=0xBF => reg.mmc_bank_f,
            0xC0..=0xCF => reg.mmc_bank_c,
            0xD0..=0xDF => reg.mmc_bank_d,
            0xE0..=0xEF => reg.mmc_bank_e,
            0xF0..=0xFF => reg.mmc_bank_f,
            _ => 0,
        } as usize;
        let chunk_base = chunk_index * 0x100000; // 1MB units

        match bank {
            // LoROM style windows (32KB per bank, lower half mirrors upper)
            0x00..=0x1F | 0x20..=0x3F | 0x80..=0x9F | 0xA0..=0xBF => {
                let off = (offset | 0x8000) as usize;
                let bank_lo = (bank & 0x1F) as usize;
                chunk_base + bank_lo * 0x8000 + (off - 0x8000)
            }
            // HiROM mirrors for each chunk
            0xC0..=0xFF => chunk_base + offset as usize,
            _ => chunk_base,
        }
    }

    // Helper method for ROM reading in system banks
    // Dragon Quest 3専用ROM読み取り処理
    pub(crate) fn read_dq3_rom(&mut self, bank: u32, offset: u16) -> u8 {
        // SA-1 LoROM: 32KB banks across 4MB; 0x80-0xFF mirror 0x00-0x7F.
        let rom_addr = self.dq3_phys_addr(bank as u8, offset);

        let value = self.rom[rom_addr % self.rom_size];
        self.mdr = value;
        value
    }

    // エンハンスメント領域の判定 (0x30)
    #[allow(dead_code)]
    pub(crate) fn is_dq3_enhancement_area(&self, bank: u32, _offset: u16) -> bool {
        // エンハンスメントチップ0x30の専用領域
        match bank {
            0x03 | 0x24 | 0x30..=0x37 => true, // エンハンスメント専用バンク
            _ => false,
        }
    }

    // エンハンスメント処理 (0x30)
    pub(crate) fn handle_dq3_enhancement(&self, bank: u32, offset: u16) -> u8 {
        // エンハンスメント機能の実装
        match bank {
            // 標準システムバンク 00-3F の低位アドレス処理
            0x00..=0x3F => {
                // Dragon Quest 3はHiROMベース：全アドレス範囲にROMデータ
                let rom_addr = (bank as usize) * 0x10000 + (offset as usize);

                if rom_addr < self.rom_size {
                    self.rom[rom_addr]
                } else {
                    // ROM範囲外の場合はミラー
                    let mirror_addr = rom_addr % self.rom_size;
                    self.rom[mirror_addr]
                }
            }
            0x03 | 0x24 => {
                // Bank 03/24を適切なROM領域にマップ
                // 4MB ROMでの特殊バンク処理
                if offset < 0x8000 {
                    // 低アドレス領域：特殊マッピング
                    let rom_addr = match bank {
                        0x03 => 0x30000 + (offset as usize),
                        0x24 => 0x240000 + (offset as usize),
                        _ => (bank as usize) * 0x10000 + (offset as usize),
                    };
                    if rom_addr < self.rom_size {
                        self.rom[rom_addr]
                    } else {
                        let mirror_addr = rom_addr % self.rom_size;
                        self.rom[mirror_addr]
                    }
                } else {
                    // 高アドレス領域：ROMデータにマップ
                    let mapped_bank = match bank {
                        0x03 => 0x43, // Bank 03 -> ROM Bank 43
                        0x24 => 0x64, // Bank 24 -> ROM Bank 64
                        _ => bank,
                    };
                    let rom_addr = ((mapped_bank - 0x40) as usize) * 0x10000 + (offset as usize);
                    if rom_addr < self.rom_size {
                        self.rom[rom_addr]
                    } else {
                        // ROM範囲外の場合はミラー
                        let mirror_addr = rom_addr % self.rom_size;
                        self.rom[mirror_addr]
                    }
                }
            }
            0x30..=0x37 => {
                // エンハンスメントチップ専用領域
                let rom_addr = ((bank - 0x30) as usize) * 0x10000 + (offset as usize);
                if rom_addr < self.rom_size {
                    self.rom[rom_addr]
                } else {
                    0xFF
                }
            }
            _ => 0xFF,
        }
    }

    pub(crate) fn read_rom_lohi(&self, bank: u32, offset: u16) -> u8 {
        match self.mapper_type {
            MapperType::LoRom => {
                // LoROM: 32KB banks in upper half. Use 7-bit bank to reach >2MB (e.g., 24/32 Mbit).
                let rom_bank = (bank & 0x7F) as usize;
                let rom_addr = rom_bank * 0x8000 + ((offset - 0x8000) as usize);
                if self.rom_size == 0 {
                    0xFF
                } else {
                    self.rom[rom_addr % self.rom_size]
                }
            }
            MapperType::HiRom => {
                // HiROM: Full 64KB banks
                let rom_addr = (bank as usize) * 0x10000 + (offset as usize);
                if rom_addr < self.rom_size {
                    self.rom[rom_addr]
                } else {
                    0xFF
                }
            }
            MapperType::ExHiRom => {
                // ExHiROM: 00-3F/80-BF high areas map to upper half starting at 0x400000
                // This helper is called only for offset >= 0x8000.
                let rom_addr = 0x400000usize
                    .saturating_add((bank as usize) * 0x10000)
                    .saturating_add(offset as usize);
                if rom_addr < self.rom_size {
                    self.rom[rom_addr]
                } else {
                    0xFF
                }
            }
            MapperType::DragonQuest3 => {
                // ドラクエ3専用マッピング（SA-1 LoROM/HiROMハイブリッド）
                // SA-1 LoROM : 00-1F/80-9F -> バンクC, 20-3F/A0-BF -> バンクD/E/F (32KB窓)
                // SA-1 HiROM: C0-CF/D0-DF/E0-EF/F0-FF -> バンクC/D/E/F (64KB窓)
                // Reuse SA-1 MMC mapping so that CPU vectors (e.g., 00:FFEA/FFEE)
                // resolve to the correct chunk instead of the linear 64KB mapping.
                let rom_addr = self.sa1_phys_addr(bank, offset);
                if offset < 0x8000 {
                    // エンハンスメントチップ/特例（VBMP等）もここで拾う
                    return self.handle_dq3_enhancement(bank, offset);
                }
                if rom_addr >= self.rom_size {
                    // 4MB未満の場合はラップ
                    let mirror_addr = rom_addr % self.rom_size;
                    return self.rom[mirror_addr];
                }
                let value = self.rom[rom_addr];

                // Debug output for bank 08 reads (especially around 0x0000)
                if bank == 0x08 && offset <= 0x0010 {
                    static mut BANK08_DEBUG_COUNT: u32 = 0;
                    unsafe {
                        BANK08_DEBUG_COUNT += 1;
                        if BANK08_DEBUG_COUNT <= 20 {
                            println!(
                                "BANK08 READ: {:02X}:{:04X} -> rom_addr=0x{:06X} -> value=0x{:02X}",
                                bank, offset, rom_addr, value
                            );
                        }
                    }
                }

                if (0xFF98..=0xFFA0).contains(&offset) && crate::debug_flags::debug_reset_area() {
                    println!(
                        "RESET AREA read: bank=0x{:02X}, offset=0x{:04X}, value=0x{:02X}",
                        bank, offset, value
                    );
                }
                value
            }
            _ => 0xFF,
        }
    }

    /// Dragon Quest III / SA-1 用の物理ROMアドレス計算（Fast HiROM, 4MB）
    ///
    /// - ヘッダのマップモードは 0x31 (Fast HiROM) なので、64KB単位で直線的に配置する。
    /// - バンクC0-FFは 00-3F のミラーとして扱い、ROMサイズでラップさせる。
    pub(crate) fn dq3_phys_addr(&self, bank: u8, offset: u16) -> usize {
        let bank_idx = bank as usize;
        let addr = bank_idx * 0x10000 + offset as usize;
        addr % self.rom_size
    }
}
