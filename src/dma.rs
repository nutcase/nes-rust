// SNES DMA and HDMA implementation
use crate::debug_flags;

#[derive(Debug, Clone)]
pub struct DmaChannel {
    pub control: u8,      // DMA制御レジスタ ($43X0)
    pub dest_address: u8, // 転送先アドレス ($43X1) - PPUレジスタ
    pub src_address: u32, // 転送元アドレス ($43X2-$43X4)
    pub size: u16,        // 転送サイズ ($43X5-$43X6)
    pub dasb: u8,         // Indirect HDMA bank / DMA reg ($43X7)
    pub a2a: u16,         // HDMA table current address ($43X8-$43X9)
    pub nltr: u8,         // HDMA line counter/reload ($43XA)
    pub unused: u8,       // Unused shared byte ($43XB and $43XF)

    // HDMA関連
    pub hdma_table_addr: u32,   // HDMAテーブルアドレス ($43X2-$43X4)
    pub hdma_line_counter: u8,  // HDMAライン残数 ($43X0の下位7ビット)
    pub hdma_repeat_flag: bool, // HDMAリピートフラグ ($43X0の7ビット目)
    pub hdma_do_transfer: bool, // HDMAがこのラインで転送するか（repeat=0時の「最初の1回」制御）
    pub hdma_enabled: bool,     // HDMAが有効か
    pub hdma_terminated: bool,  // HDMAが終了したか
    // HDMAデータ（リピート用ラッチ）
    pub hdma_latched: [u8; 4],
    pub hdma_latched_len: u8,
    // HDMA indirect addressing support
    pub hdma_indirect: bool,
    pub hdma_indirect_addr: u32,
    pub configured: bool,

    // Debug/config tracking (for INIT summaries)
    pub cfg_ctrl: bool,
    pub cfg_dest: bool,
    pub cfg_src: bool,
    pub cfg_size: bool,
}

impl Default for DmaChannel {
    fn default() -> Self {
        Self::new()
    }
}

impl DmaChannel {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            // Power-on defaults per SNESdev wiki:
            // - DMAPn  = $FF
            // - BBADn  = $FF
            // - A1Tn   = $FFFFFF
            // - DASn   = $FFFF
            control: 0xFF,
            dest_address: 0xFF,
            src_address: 0x00FF_FFFF,
            size: 0xFFFF,
            dasb: 0xFF,
            a2a: 0xFFFF,
            nltr: 0xFF,
            unused: 0xFF,
            hdma_table_addr: 0,
            hdma_line_counter: 0,
            hdma_repeat_flag: false,
            hdma_do_transfer: false,
            hdma_enabled: false,
            hdma_terminated: false,
            hdma_latched: [0; 4],
            hdma_latched_len: 0,
            hdma_indirect: false,
            hdma_indirect_addr: 0,
            configured: false,
            cfg_ctrl: false,
            cfg_dest: false,
            cfg_src: false,
            cfg_size: false,
        }
    }

    #[allow(dead_code)]
    pub fn reset(&mut self) {
        *self = Self::new();
    }

    // DMA転送方向を取得
    #[allow(dead_code)]
    pub fn is_ppu_to_cpu(&self) -> bool {
        self.control & 0x80 != 0
    }

    // DMA転送単位を取得
    pub fn get_transfer_unit(&self) -> u8 {
        self.control & 0x07
    }

    // アドレス増減設定を取得
    pub fn get_address_mode(&self) -> u8 {
        (self.control >> 3) & 0x03
    }
}

#[derive(Debug)]
pub struct DmaController {
    pub channels: [DmaChannel; 8],
    pub dma_enable: u8,  // DMA有効チャンネル ($420B)
    pub hdma_enable: u8, // HDMA有効チャンネル ($420C)
}

impl DmaController {
    pub fn new() -> Self {
        Self {
            channels: Default::default(),
            dma_enable: 0,
            hdma_enable: 0,
        }
    }

    #[inline]
    #[allow(dead_code)]
    fn hdma_transfer_len(unit: u8) -> u8 {
        match unit & 0x07 {
            0 => 1,
            1 => 2,
            2 => 2,
            3 => 4,
            4 => 4,
            5 => 4,
            6 => 2,
            7 => 4,
            _ => 1,
        }
    }

    #[inline]
    #[allow(dead_code)]
    fn hdma_dest_offset(unit: u8, base: u8, index: u8) -> u8 {
        let i = index as u8;
        match unit & 0x07 {
            0 => base,                            // A
            1 => base.wrapping_add(i & 1),        // A, B
            2 => base,                            // A, A
            3 => base.wrapping_add((i >> 1) & 1), // A, A, B, B
            4 => base.wrapping_add(i & 3),        // A, B, C, D
            5 => base.wrapping_add(i & 1),        // A,B,A,B (undocumented)
            6 => base,                            // A,A (undocumented)
            7 => base.wrapping_add((i >> 1) & 1), // A,A,B,B (undocumented)
            _ => base,
        }
    }

    #[allow(dead_code)]
    pub fn reset(&mut self) {
        for channel in &mut self.channels {
            channel.reset();
        }
        self.dma_enable = 0;
        self.hdma_enable = 0;
    }

    // DMAレジスタ書き込み
    pub fn write(&mut self, addr: u16, value: u8) {
        // Lightweight debug hook: dump early DMA register writes when TRACE_DMA_REG is set.
        if std::env::var_os("TRACE_DMA_REG").is_some() {
            use std::sync::atomic::{AtomicU32, Ordering};
            static COUNT: AtomicU32 = AtomicU32::new(0);
            let n = COUNT.fetch_add(1, Ordering::Relaxed);
            if n < 256 {
                let ch = ((addr.saturating_sub(0x4300)) >> 4) as u8;
                let reg = addr & 0x0F;
                println!(
                    "[DMA-REG] W ${:04X} ch{} reg=${:X} val={:02X}",
                    addr + 0x0000,
                    ch,
                    reg,
                    value
                );
            }
        }
        match addr {
            0x420B => {
                self.dma_enable = value;
                if (debug_flags::dma() || debug_flags::dma_reg()) && !debug_flags::quiet() {
                    use std::sync::atomic::{AtomicU32, Ordering};
                    static EN_LOG: AtomicU32 = AtomicU32::new(0);
                    let n = EN_LOG.fetch_add(1, Ordering::Relaxed);
                    if n < 64 {
                        println!("[DMA-EN] $420B MDMAEN=0x{:02X}", value);
                    }
                }
            }
            0x420C => {
                self.hdma_enable = value;
                // HDMAを有効化
                for i in 0..8 {
                    if value & (1 << i) != 0 {
                        // 未設定チャンネルは有効化しない（デフォルト値の暴走を防ぐ）
                        if !self.channels[i].configured {
                            self.channels[i].hdma_enabled = false;
                        } else {
                            self.channels[i].hdma_enabled = true;
                            self.channels[i].hdma_terminated = false;
                            // 初期化処理
                            self.init_hdma_channel(i);
                        }
                    } else {
                        self.channels[i].hdma_enabled = false;
                    }
                }
            }
            0x4300..=0x43FF => {
                // チャンネル別レジスタ
                let channel = ((addr - 0x4300) >> 4) as usize;
                let reg = (addr & 0x0F) as u8;

                if channel < 8 {
                    if channel == 1 && debug_flags::dma_reg() {
                        use std::sync::atomic::{AtomicU32, Ordering};
                        static COUNT1: AtomicU32 = AtomicU32::new(0);
                        let n = COUNT1.fetch_add(1, Ordering::Relaxed);
                        if n < 32 {
                            println!(
                                "[DMA1-REG] W ${:04X} reg=${:X} val={:02X}",
                                addr + 0x0000,
                                reg,
                                value
                            );
                        }
                    }
                    match reg {
                        0x00 => {
                            self.channels[channel].control = value;
                            // bit6: HDMA indirect addressing
                            self.channels[channel].hdma_indirect = (value & 0x40) != 0;
                            self.channels[channel].configured = true;
                            self.channels[channel].cfg_ctrl = true;
                            if debug_flags::dma_reg() {
                                println!(
                                    "DMA ch{} control=0x{:02X} (unit={}, addr_mode={})",
                                    channel,
                                    value,
                                    self.channels[channel].get_transfer_unit(),
                                    self.channels[channel].get_address_mode()
                                );
                            }
                        }
                        0x01 => {
                            // B-bus destination ($43x1) — use value as-is.
                            self.channels[channel].dest_address = value;
                            self.channels[channel].configured = true;
                            self.channels[channel].cfg_dest = true;
                            if debug_flags::dma_reg() {
                                use std::sync::atomic::{AtomicU32, Ordering};
                                static DEST_LOG: AtomicU32 = AtomicU32::new(0);
                                let n = DEST_LOG.fetch_add(1, Ordering::Relaxed);
                                if n < 64 {
                                    println!(
                                        "[DMA-DEST-REG] ch{} BBAD=$21{:02X} (reg=${:04X})",
                                        channel, value, addr
                                    );
                                }
                                // Lightweight trace for graphics-related destinations
                                if matches!(value, 0x18 | 0x19 | 0x22 | 0x04) {
                                    static DEST_TRACE: AtomicU32 = AtomicU32::new(0);
                                    let n = DEST_TRACE.fetch_add(1, Ordering::Relaxed);
                                    if n < 32 {
                                        println!(
                                            "[DMA-DEST] ch{} dest=$21{:02X} (graphics path)",
                                            channel, value
                                        );
                                    }
                                }
                            }
                            if (debug_flags::dma_reg() || debug_flags::cgram_dma()) && value == 0x22
                            {
                                println!("DMA ch{} configured for CGRAM ($2122)", channel);
                            }
                            if debug_flags::dma_reg() {
                                println!("DMA ch{} dest=$21{:02X}", channel, value);
                            }
                        }
                        0x02 => {
                            self.channels[channel].src_address =
                                (self.channels[channel].src_address & 0xFFFF00) | value as u32;
                            self.channels[channel].configured = true;
                            self.channels[channel].cfg_src = true;
                            if debug_flags::dma_reg() {
                                println!("DMA ch{} src.lo=0x{:02X}", channel, value);
                            }
                        }
                        0x03 => {
                            self.channels[channel].src_address =
                                (self.channels[channel].src_address & 0xFF00FF)
                                    | ((value as u32) << 8);
                            self.channels[channel].configured = true;
                            self.channels[channel].cfg_src = true;
                            if debug_flags::dma_reg() {
                                println!("DMA ch{} src.mid=0x{:02X}", channel, value);
                            }
                        }
                        0x04 => {
                            self.channels[channel].src_address =
                                (self.channels[channel].src_address & 0x00FFFF)
                                    | ((value as u32) << 16);
                            self.channels[channel].configured = true;
                            self.channels[channel].cfg_src = true;
                            // If HDMA is using A2A, update only the bank portion for subsequent table reads.
                            let low = self.channels[channel].hdma_table_addr & 0x0000_FFFF;
                            self.channels[channel].hdma_table_addr =
                                (self.channels[channel].src_address & 0xFF00_0000) | low;
                            if debug_flags::dma_reg() {
                                println!("DMA ch{} src.bank=0x{:02X}", channel, value);
                            }
                        }
                        0x05 => {
                            self.channels[channel].size =
                                (self.channels[channel].size & 0xFF00) | value as u16;
                            self.channels[channel].configured = true;
                            self.channels[channel].cfg_size = true;
                            if (debug_flags::dma_reg() || debug_flags::cgram_dma())
                                && self.channels[channel].dest_address == 0x22
                            {
                                println!(
                                    "DMA ch{} CGRAM size.lo set -> size={} bytes",
                                    channel, self.channels[channel].size
                                );
                            }
                            if debug_flags::dma_reg() {
                                println!("DMA ch{} size.lo=0x{:02X}", channel, value);
                            }
                        }
                        0x06 => {
                            self.channels[channel].size =
                                (self.channels[channel].size & 0x00FF) | ((value as u16) << 8);
                            self.channels[channel].configured = true;
                            self.channels[channel].cfg_size = true;
                            if (debug_flags::dma_reg() || debug_flags::cgram_dma())
                                && self.channels[channel].dest_address == 0x22
                            {
                                println!(
                                    "DMA ch{} CGRAM size.hi set -> size={} bytes",
                                    channel, self.channels[channel].size
                                );
                            }
                            if debug_flags::dma_reg() {
                                static mut DMA_SIZE_LOG_CNT2: u32 = 0;
                                unsafe {
                                    DMA_SIZE_LOG_CNT2 += 1;
                                    if DMA_SIZE_LOG_CNT2 <= 16 {
                                        println!(
                                            "DMA ch{} size.hi=0x{:02X} (size={})",
                                            channel, value, self.channels[channel].size
                                        );
                                    }
                                }
                            }
                        }
                        0x07 => {
                            // DASBn ($43x7): Indirect HDMA bank. RW8.
                            self.channels[channel].dasb = value;
                        }
                        0x08 => {
                            // A2AnL ($43x8): HDMA table current address low. RW8.
                            self.channels[channel].a2a =
                                (self.channels[channel].a2a & 0xFF00) | value as u16;
                            // Mirror into internal HDMA table pointer (bank from A1Bn/src_address).
                            let bank = self.channels[channel].src_address & 0xFF0000;
                            self.channels[channel].hdma_table_addr =
                                bank | (self.channels[channel].a2a as u32);
                        }
                        0x09 => {
                            // A2AnH ($43x9): HDMA table current address high. RW8.
                            self.channels[channel].a2a =
                                (self.channels[channel].a2a & 0x00FF) | ((value as u16) << 8);
                            // Mirror into internal HDMA table pointer (bank from A1Bn/src_address).
                            let bank = self.channels[channel].src_address & 0xFF0000;
                            self.channels[channel].hdma_table_addr =
                                bank | (self.channels[channel].a2a as u32);
                        }
                        0x0A => {
                            // NLTRn ($43xA): HDMA reload flag + line counter. RW8.
                            self.channels[channel].nltr = value;
                        }
                        0x0B | 0x0F => {
                            // UNUSEDn ($43xB/$43xF): shared RW8 byte with no effect on DMA/HDMA.
                            self.channels[channel].unused = value;
                        }
                        0x0C..=0x0E => {
                            // Unused holes: ignore writes (open bus on read).
                        }
                        _ => {}
                    }

                    // HDMAテーブルアドレスはsrc_addressと同じ
                    self.channels[channel].hdma_table_addr = self.channels[channel].src_address;
                }
            }
            _ => {}
        }
    }

    // DMAレジスタ読み込み
    pub fn read(&self, addr: u16) -> u8 {
        match addr {
            0x420B => self.dma_enable,
            0x420C => self.hdma_enable,
            0x2200..=0x23FF => {
                // TODO: route to SA-1 DMA/CC-DMA registers once implemented.
                0
            }
            0x4300..=0x43FF => {
                let channel = ((addr - 0x4300) >> 4) as usize;
                let reg = (addr & 0x0F) as u8;

                if channel < 8 {
                    match reg {
                        0x00 => self.channels[channel].control,
                        0x01 => self.channels[channel].dest_address,
                        0x02 => (self.channels[channel].src_address & 0xFF) as u8,
                        0x03 => ((self.channels[channel].src_address >> 8) & 0xFF) as u8,
                        0x04 => ((self.channels[channel].src_address >> 16) & 0xFF) as u8,
                        0x05 => (self.channels[channel].size & 0xFF) as u8,
                        0x06 => ((self.channels[channel].size >> 8) & 0xFF) as u8,
                        0x07 => self.channels[channel].dasb,
                        0x08 => (self.channels[channel].a2a & 0xFF) as u8,
                        0x09 => ((self.channels[channel].a2a >> 8) & 0xFF) as u8,
                        0x0A => self.channels[channel].nltr,
                        0x0B | 0x0F => self.channels[channel].unused,
                        _ => 0xFF,
                    }
                } else {
                    0xFF
                }
            }
            _ => 0xFF,
        }
    }

    // HDMAチャンネル初期化
    fn init_hdma_channel(&mut self, channel: usize) {
        // HDMAテーブルから最初のエントリを読み込み
        self.channels[channel].hdma_line_counter = 0;
        self.channels[channel].hdma_repeat_flag = false;
        self.channels[channel].hdma_do_transfer = false;
        self.channels[channel].hdma_latched = [0; 4];
        self.channels[channel].hdma_latched_len = 0;
        // テーブル開始点は src_address
        self.channels[channel].hdma_table_addr = self.channels[channel].src_address;
        // 初回は次の scanline で load_hdma_entry が走る
    }

    // HDMA実行（スキャンライン毎に呼び出し）
    #[allow(dead_code)]
    pub fn hdma_scanline(&mut self, bus: &mut crate::bus::Bus) {
        for i in 0..8 {
            if !self.channels[i].hdma_enabled || self.channels[i].hdma_terminated {
                continue;
            }

            if self.channels[i].hdma_line_counter == 0 {
                // 新しいHDMAエントリを読み込み
                if !self.load_hdma_entry(i, bus) {
                    self.channels[i].hdma_terminated = true;
                    continue;
                }
            }

            // HDMA転送実行
            self.perform_hdma_transfer(i, bus);

            // ライン カウンターをデクリメント
            self.channels[i].hdma_line_counter =
                self.channels[i].hdma_line_counter.saturating_sub(1);
        }
    }

    // HDMAエントリ読み込み
    #[allow(dead_code)]
    fn load_hdma_entry(&mut self, channel: usize, bus: &mut crate::bus::Bus) -> bool {
        let table_addr = self.channels[channel].hdma_table_addr;

        // エントリの最初のバイトを読み込み（ライン数）
        let line_count = bus.read_u8(table_addr);

        if line_count == 0 {
            // 終了マーカー
            return false;
        }

        self.channels[channel].hdma_line_counter = line_count & 0x7F;
        self.channels[channel].hdma_repeat_flag = (line_count & 0x80) != 0;

        // テーブルアドレスを進める
        self.channels[channel].hdma_table_addr = table_addr + 1;

        // 転送単位に応じてデータサイズ（1/2/4）
        let data_size = match self.channels[channel].get_transfer_unit() {
            0 | 2 | 6 => 1,
            1 | 3 | 7 => 2,
            4 | 5 => 4,
            _ => 1,
        } as usize;

        // 間接アドレスモード: 行頭で 16bit アドレスを取得
        if self.channels[channel].hdma_indirect {
            let lo = bus.read_u8(self.channels[channel].hdma_table_addr) as u32;
            let hi = bus.read_u8(self.channels[channel].hdma_table_addr + 1) as u32;
            // バンクは src_address のバンクを使用
            let bank = (self.channels[channel].src_address & 0xFF0000) as u32;
            self.channels[channel].hdma_indirect_addr = bank | (hi << 8) | lo;
            self.channels[channel].hdma_table_addr += 2;
        }

        // 行頭でラッチ更新
        if !self.channels[channel].hdma_repeat_flag {
            // 新規データを取得してラッチ
            for i in 0..data_size.min(4) {
                let byte = if self.channels[channel].hdma_indirect {
                    bus.read_u8(self.channels[channel].hdma_indirect_addr + i as u32)
                } else {
                    bus.read_u8(self.channels[channel].hdma_table_addr + i as u32)
                };
                self.channels[channel].hdma_latched[i] = byte;
            }
            self.channels[channel].hdma_latched_len = data_size as u8;
            if self.channels[channel].hdma_indirect {
                self.channels[channel].hdma_indirect_addr += data_size as u32;
            } else {
                self.channels[channel].hdma_table_addr += data_size as u32;
            }
        } else {
            // リピートの場合、以前のラッチをそのまま使用（初回は 0 にならないように data_size をセット）
            if self.channels[channel].hdma_latched_len == 0 {
                self.channels[channel].hdma_latched_len = data_size as u8;
            }
        }

        true
    }

    // HDMA転送実行
    #[allow(dead_code)]
    fn perform_hdma_transfer(&mut self, channel: usize, bus: &mut crate::bus::Bus) {
        let dest_base = self.channels[channel].dest_address;
        let unit = self.channels[channel].get_transfer_unit();
        const ENABLE_DMA_REG_LOG: bool = false;
        let count = Self::hdma_transfer_len(unit) as usize;

        // 転送データは、repeat=1 ならラッチ値、repeat=0 なら最新ラッチ（既に load で更新済）
        for i in 0..count {
            let b = self.channels[channel].hdma_latched[i.min(3)];
            // Bバス宛先をモードに応じて決定
            let dest_off = Self::hdma_dest_offset(unit, dest_base, i as u8);
            let dest_addr = 0x2100u32 + dest_off as u32;
            if dest_off <= 0x33 || (dest_off >= 0x40 && dest_off <= 0x43) {
                bus.write_u8(dest_addr, b);
            }
        }
    }
}
// cleaned: stray inner attribute
// #![allow(dead_code)]
