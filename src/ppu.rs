#![allow(static_mut_refs)]
// Logging controls (runtime via env — see debug_flags)
const IMPORTANT_WRITE_LIMIT: u32 = 10; // How many important writes to print
use std::sync::OnceLock;

pub struct Ppu {
    vram: Vec<u8>,
    cgram: Vec<u8>,
    oam: Vec<u8>,
    // DQ3専用: INIDISP への DMA/HDMA を無視するフラグ
    dq3_block_inidisp: bool,
    /// 強制ブランクを無視して描画するデバッグ／DQ3用のオーバーライド
    force_display_override: bool,

    pub scanline: u16,
    // Current dot within the scanline (0..=340 approx). This is our dot counter.
    cycle: u16,
    frame: u64,
    // Latched H/V counters (set by reading $2137 or by WRIO latch via $4201 bit7 transition).
    hv_latched_h: u16,
    hv_latched_v: u16,
    // Pending external latch via WRIO ($4201 bit7 1->0). Fires after a 1-dot delay.
    wio_latch_pending_dots: u8,
    ophct_second: bool,
    opvct_second: bool,

    bg_mode: u8,
    // Mode 1 only: BG3 priority enable ($2105 bit3). Used by z-rank model.
    mode1_bg3_priority: bool,
    bg_mosaic: u8,
    mosaic_size: u8, // モザイクサイズ（1-16）

    bg1_tile_base: u16,
    bg2_tile_base: u16,
    bg3_tile_base: u16,
    bg4_tile_base: u16,

    bg1_tilemap_base: u16,
    bg2_tilemap_base: u16,
    bg3_tilemap_base: u16,
    bg4_tilemap_base: u16,

    bg1_hscroll: u16,
    bg1_vscroll: u16,
    bg2_hscroll: u16,
    bg2_vscroll: u16,
    bg3_hscroll: u16,
    bg3_vscroll: u16,
    bg4_hscroll: u16,
    bg4_vscroll: u16,

    // BG tile size flags (false=8x8, true=16x16)
    bg_tile_16: [bool; 4],
    // BG screen sizes: 0=32x32, 1=64x32, 2=32x64, 3=64x64
    bg_screen_size: [u8; 4],

    // Scroll register latches shared across BG1..BG4 ($210D..$2114).
    // See SNESdev wiki: BGnHOFS/BGnVOFS behavior uses shared latches.
    bgofs_latch: u8,
    bghofs_latch: u8,

    main_screen_designation: u8,
    main_screen_designation_last_nonzero: u8, // Remember last non-zero value for rendering
    sub_screen_designation: u8,
    tmw_mask: u8, // $212E: window mask enables for main screen (bits: BG1..BG4,OBJ)
    tsw_mask: u8, // $212F: window mask enables for sub screen

    pub screen_display: u8,
    pub brightness: u8,

    vram_addr: u16,
    vram_increment: u16,
    vram_mapping: u8,
    // VRAM read latch for $2139/$213A (VMDATAREAD)
    vram_read_buf_lo: u8,
    vram_read_buf_hi: u8,

    cgram_addr: u8,          // CGRAM word address (0..255)
    cgram_second: bool,      // false: next $2122 is low; true: next $2122 is high
    cgram_read_second: bool, // false: next $213B returns low; true: next returns high then increments
    cgram_latch_lo: u8,      // latched low byte (not committed until high arrives)
    oam_addr: u16,

    // スプライト関連の追加フィールド
    sprite_overflow: bool,  // スプライトオーバーフローフラグ
    sprite_time_over: bool, // スプライトタイムオーバーフラグ
    // STAT77 flags are sticky until end of VBlank.
    sprite_overflow_latched: bool,
    sprite_time_over_latched: bool,
    sprites_on_line_count: u8, // 現在のスキャンラインのスプライト数

    // スプライト関連
    sprite_size: u8,         // スプライトサイズ設定
    sprite_name_base: u16,   // スプライトタイル名ベースアドレス
    sprite_name_select: u16, // スプライト名テーブル選択

    // ウィンドウ関連
    window1_left: u8,        // Window 1の左端
    window1_right: u8,       // Window 1の右端
    window2_left: u8,        // Window 2の左端
    window2_right: u8,       // Window 2の右端
    window_bg_mask: [u8; 4], // BG1-4のウィンドウマスク設定
    window_obj_mask: u8,     // スプライトのウィンドウマスク設定
    window_color_mask: u8,   // カラーウィンドウマスク
    // Window logic (WBGLOG/WOBJLOG): 0=OR,1=AND,2=XOR,3=XNOR
    bg_window_logic: [u8; 4],
    obj_window_logic: u8,
    color_window_logic: u8,

    // カラー演算関連
    // Color math registers
    cgwsel: u8,                 // $2130: Color Window Select (gating + subscreen/fixed)
    cgadsub: u8,                // $2131: Addition/Subtraction + halve + layer enables
    color_math_designation: u8, // legacy alias (CGADSUB layer mask)
    color_math_control: u8,     // legacy alias (CGWSEL)
    fixed_color: u16,           // 固定色データ（$2132）

    // Mode 7関連
    m7sel: u8,           // $211A: Mode 7 settings (repeat/fill/flip)
    mode7_matrix_a: i16, // Mode 7変換行列A ($211B)
    mode7_matrix_b: i16, // Mode 7変換行列B ($211C)
    mode7_matrix_c: i16, // Mode 7変換行列C ($211D)
    mode7_matrix_d: i16, // Mode 7変換行列D ($211E)
    mode7_center_x: i16, // Mode 7回転中心X ($211F)
    mode7_center_y: i16, // Mode 7回転中心Y ($2120)

    // Mode 7 乗算結果キャッシュ ($2134-$2136)
    mode7_mul_result: u32, // 24bit 有効（下位3バイト）

    // Mode 7 register write latches (two-write: low then high)
    m7_latch_low: [u8; 6],
    m7_latch_second: [bool; 6],

    framebuffer: Vec<u32>,
    subscreen_buffer: Vec<u32>, // サブスクリーン用バッファ

    // SETINI ($2133)
    setini: u8,
    pseudo_hires: bool,
    extbg: bool,
    interlace: bool,
    // H/V counter latch enable (mirrors $4201 bit7) and STAT78 latch flag.
    wio_latch_enable: bool,
    stat78_latch_flag: bool,
    // STAT78 "interlace field" bit (toggles every VBlank).
    interlace_field: bool,
    // SETINI bits
    overscan: bool,
    obj_interlace: bool,

    pub nmi_enabled: bool,
    pub nmi_flag: bool,
    pub nmi_latched: bool,
    /// 同一VBlank中にRDNMIが読まれたか（読まれたら再セットしないためのフラグ）
    pub rdnmi_read_in_vblank: bool,

    v_blank: bool,
    h_blank: bool,

    // Lightweight VRAM write diagnostics (headless summaries)
    vram_write_buckets: [u32; 8], // counts per 0x1000-word region (0x0000..0x7000)
    vram_write_low_count: u32,
    vram_write_high_count: u32,
    vram_last_vmain: u8,
    // Strict timing: reject counters
    vram_rejects: u32,
    cgram_rejects: u32,
    oam_rejects: u32,
    // Gap-block counters (per summary interval)
    vram_gap_blocks: u32,
    cgram_gap_blocks: u32,
    oam_gap_blocks: u32,
    oam_data_gap_ticks: u16,
    // First per-frame rejection logs (to avoid spam when DEBUG_TIMING_REJECTS)
    last_reject_frame_vram: u64,
    last_reject_frame_cgram: u64,
    last_reject_frame_oam: u64,

    // Run-wide counters for headless init summary
    important_writes_count: u32,
    vram_writes_total_low: u64,
    vram_writes_total_high: u64,
    cgram_writes_total: u64,
    oam_writes_total: u64,
    // OAMDATA write latch (low table uses 16-bit word staging)
    oam_write_latch: u8,
    // $2103 bit7: priority rotation enable
    oam_priority_rotation_enabled: bool,
    // OBJ timing metrics per frame
    obj_overflow_lines: u32,
    obj_time_over_lines: u32,
    // OAM evaluation rotation base (sprite index 0..127). Derived from $2102/$2103.
    oam_eval_base: u8,

    // Dot-level OBJ pipeline state (per visible scanline)
    line_sprites: Vec<SpriteData>,
    sprite_tile_entry_counts: [u8; 256],
    sprite_tile_budget_remaining: i16,
    sprite_draw_disabled: bool,
    sprite_timeover_stop_x: u16, // when time-over triggers, tiles starting at >= stop_x are forbidden

    // --- Dot-level window/color-math gating (per visible scanline) ---
    line_window_prepared: bool,
    color_window_lut: [u8; 256], // 1: inside color window per $2125(COL)
    main_bg_window_lut: [[u8; 256]; 4], // 1: BG masked on main at x
    sub_bg_window_lut: [[u8; 256]; 4], // 1: BG masked on sub at x
    main_obj_window_lut: [u8; 256], // 1: OBJ masked on main at x
    sub_obj_window_lut: [u8; 256], // 1: OBJ masked on sub at x

    // internal OAM byte address (internal_oamadd, 10-bit)
    oam_internal_addr: u16,

    // --- HBlank head HDMA phase guard ---
    // A tiny sub-window after HBlank starts where only HDMA should be active; MDMA is held off.
    hdma_head_busy_until: u16,

    // --- Latched (timed-commit) display-affecting registers ---
    // These are optionally used when STRICT_PPU_TIMING is enabled to apply
    // register effects at well-defined scanline boundaries instead of mid-line.
    latched_inidisp: Option<u8>, // mirrors $2100 (forced blank + brightness)
    latched_tm: Option<u8>,      // $212C main screen designation
    latched_ts: Option<u8>,      // $212D sub  screen designation
    latched_tmw: Option<u8>,     // $212E window mask enable (main)
    latched_tsw: Option<u8>,     // $212F window mask enable (sub)
    latched_cgwsel: Option<u8>,  // $2130 color window select
    latched_cgadsub: Option<u8>, // $2131 color math control
    latched_fixed_color: Option<u16>, // $2132 fixed color
    latched_setini: Option<u8>,  // $2133 SETINI (pseudo hires, EXTBG, interlace)
    // --- Latched control (address) registers for safe commit ---
    latched_vmadd_lo: Option<u8>, // $2116 VMADDL (low byte)
    latched_vmadd_hi: Option<u8>, // $2117 VMADDH (high byte)
    latched_cgadd: Option<u8>,    // $2121 CGADD
    latched_vmain: Option<u8>,    // $2115 VMAIN
    // Deferred effect for VMAIN (after commit)
    vmain_effect_pending: Option<u8>,
    vmain_effect_ticks: u16,
    // Deferred effect for CGADD
    cgadd_effect_pending: Option<u8>,
    cgadd_effect_ticks: u16,
    // Data write gap after VMAIN effect (MDMA/CPU only)
    vmain_data_gap_ticks: u16,
    // Data write gap after CGADD effect (MDMA/CPU only)
    cgram_data_gap_ticks: u16,
    latched_wbglog: Option<u8>,  // $212A window logic BG1..BG4
    latched_wobjlog: Option<u8>, // $212B window logic OBJ/COL

    // --- Optional per-frame render metrics (for regression/debug) ---
    dbg_clip_inside: u64,
    dbg_clip_outside: u64,
    dbg_math_add: u64,
    dbg_math_sub: u64,
    dbg_math_add_half: u64,
    dbg_math_sub_half: u64,
    dbg_masked_bg: u64,
    dbg_masked_obj: u64,
    dbg_math_obj_add: u64,
    dbg_math_obj_sub: u64,
    dbg_math_obj_add_half: u64,
    dbg_math_obj_sub_half: u64,
    dbg_clip_obj_inside: u64,
    dbg_clip_obj_outside: u64,
    // Mode 7 metrics
    dbg_m7_wrap: u64,
    dbg_m7_clip: u64,
    dbg_m7_fill: u64,
    dbg_m7_bg1: u64,
    dbg_m7_bg2: u64,
    dbg_m7_edge: u64,
    // Window logic usage counters (optional)
    dbg_win_xor_applied: u64,
    dbg_win_xnor_applied: u64,
    // Color math blocked by CGADSUB counters
    dbg_math_blocked: u64,
    dbg_math_blocked_obj: u64,
    dbg_math_blocked_backdrop: u64,

    // Distinguish CPU vs MDMA vs HDMA register writes (0=CPU,1=MDMA,2=HDMA)
    write_ctx: u8,
    debug_dma_channel: Option<u8>, // active MDMA/HDMA channel for debug logs
    // burn-in-test.sfc: arm narrow VRAM clobber tracing after DMA MEMORY begins
    burnin_vram_trace_armed: bool,
    burnin_vram_trace_cnt_2118: u32,
    burnin_vram_trace_cnt_2119: u32,
}

#[derive(Debug, Clone)]
struct SpriteData {
    x: u16,
    y: u8,
    tile: u16,
    palette: u8,
    priority: u8,
    flip_x: bool,
    flip_y: bool,
    size: SpriteSize,
}

#[derive(Debug, Clone)]
enum SpriteSize {
    Small, // BGモードによって 8x8 または 16x16
    Large, // BGモードによって 16x16, 32x32, または 64x64
}

impl Ppu {
    #[inline]
    fn sprite_x_signed(x_raw: u16) -> i16 {
        // OBJ X is 9-bit and treated as signed on the screen:
        // 0..255 => 0..255, 256..511 => -256..-1.
        let x9 = (x_raw & 0x01FF) as i16;
        if (x9 & 0x0100) != 0 {
            x9 - 512
        } else {
            x9
        }
    }

    #[inline]
    fn force_display_active(&self) -> bool {
        self.force_display_override || crate::debug_flags::force_display()
    }

    // --- Coarse NTSC timing helpers ---
    #[inline]
    fn first_visible_dot(&self) -> u16 {
        // SNES visible area starts at H=22 (0..339).
        22
    }
    #[inline]
    fn dots_per_line(&self) -> u16 {
        341
    }
    #[inline]
    fn first_hblank_dot(&self) -> u16 {
        // Visible width is 256 pixels. Visible starts at H=22, so HBlank begins at 22+256=278.
        self.first_visible_dot() + 256
    }
    #[inline]
    fn last_dot_index(&self) -> u16 {
        self.dots_per_line() - 1
    }
    #[inline]
    pub fn get_visible_height(&self) -> u16 {
        // デバッグ用に表示高さを短くして早めにVBlankへ入れるオプション。
        // 環境変数 PPU_VIS_HEIGHT を指定するとその値を使う（例: 200）。
        static OVERRIDE: OnceLock<Option<u16>> = OnceLock::new();
        let override_val = *OVERRIDE.get_or_init(|| {
            std::env::var("PPU_VIS_HEIGHT")
                .ok()
                .and_then(|v| v.parse::<u16>().ok())
                .filter(|v| *v >= 160 && *v <= 239)
        });
        if let Some(v) = override_val {
            return v;
        }

        if self.overscan {
            239
        } else {
            224
        }
    }
    #[inline]
    fn fixed8_floor(val: i64) -> i32 {
        // Floor division by 256 for signed 8.8 fixed
        if val >= 0 {
            (val >> 8) as i32
        } else {
            -(((-val + 255) >> 8) as i32)
        }
    }

    #[inline]
    fn write_bghofs(&mut self, bg_num: usize, value: u8) {
        // BGnHOFS ($210D/$210F/$2111/$2113)
        // SNESdev wiki: BGnHOFS = (value<<8) | (bgofs_latch & ~7) | (bghofs_latch & 7)
        let lo = (self.bgofs_latch & !0x07) | (self.bghofs_latch & 0x07);
        let ofs = (((value as u16) << 8) | (lo as u16)) & 0x03FF;
        match bg_num {
            0 => self.bg1_hscroll = ofs,
            1 => self.bg2_hscroll = ofs,
            2 => self.bg3_hscroll = ofs,
            _ => self.bg4_hscroll = ofs,
        }
        self.bgofs_latch = value;
        self.bghofs_latch = value;
    }

    #[inline]
    fn write_bgvofs(&mut self, bg_num: usize, value: u8) {
        // BGnVOFS ($210E/$2110/$2112/$2114)
        // SNESdev wiki: BGnVOFS = (value<<8) | bgofs_latch
        let ofs = (((value as u16) << 8) | (self.bgofs_latch as u16)) & 0x03FF;
        match bg_num {
            0 => self.bg1_vscroll = ofs,
            1 => self.bg2_vscroll = ofs,
            2 => self.bg3_vscroll = ofs,
            _ => self.bg4_vscroll = ofs,
        }
        self.bgofs_latch = value;
    }

    pub fn new() -> Self {
        Self {
            vram: vec![0; 0x10000],
            cgram: vec![0; 0x200],
            oam: vec![0; 0x220],

            dq3_block_inidisp: false,
            force_display_override: false,

            scanline: 0,
            cycle: 0,
            frame: 0,
            hv_latched_h: 0,
            hv_latched_v: 0,
            wio_latch_pending_dots: 0,
            ophct_second: false,
            opvct_second: false,

            bg_mode: 0,
            mode1_bg3_priority: false,
            bg_mosaic: 0,
            mosaic_size: 1,

            bg1_tile_base: 0,
            bg2_tile_base: 0,
            bg3_tile_base: 0,
            bg4_tile_base: 0,

            bg1_tilemap_base: 0,
            bg2_tilemap_base: 0,
            bg3_tilemap_base: 0,
            bg4_tilemap_base: 0,

            bg1_hscroll: 0,
            bg1_vscroll: 0,
            bg2_hscroll: 0,
            bg2_vscroll: 0,
            bg3_hscroll: 0,
            bg3_vscroll: 0,
            bg4_hscroll: 0,
            bg4_vscroll: 0,

            bg_tile_16: [false; 4],
            bg_screen_size: [0; 4],

            bgofs_latch: 0,
            bghofs_latch: 0,

            main_screen_designation: 0x1F, // 初期は全BG/Spriteレイヤー有効
            main_screen_designation_last_nonzero: 0x1F,
            sub_screen_designation: 0,
            tmw_mask: 0,
            tsw_mask: 0,

            screen_display: 0x80, // forced blank on by default (初期状態は画面非表示)
            brightness: 0,        // 初期明度を0に設定

            vram_addr: 0,
            vram_increment: 1,
            vram_mapping: 0,
            vram_read_buf_lo: 0,
            vram_read_buf_hi: 0,

            cgram_addr: 0,
            cgram_second: false,
            cgram_read_second: false,
            cgram_latch_lo: 0,
            oam_addr: 0,

            sprite_overflow: false,
            sprite_time_over: false,
            sprite_overflow_latched: false,
            sprite_time_over_latched: false,
            sprites_on_line_count: 0,

            // スプライト関連初期化
            sprite_size: 0,
            sprite_name_base: 0,
            sprite_name_select: 0,

            // ウィンドウ関連初期化
            window1_left: 0,
            window1_right: 0,
            window2_left: 0,
            window2_right: 0,
            window_bg_mask: [0; 4],
            window_obj_mask: 0,
            window_color_mask: 0,
            bg_window_logic: [0; 4],
            obj_window_logic: 0,
            color_window_logic: 0,

            // カラー演算関連初期化
            cgwsel: 0,
            cgadsub: 0,
            color_math_designation: 0,
            color_math_control: 0,
            fixed_color: 0,

            // Mode 7関連初期化（単位行列）
            m7sel: 0,
            mode7_matrix_a: 256, // 1.0 in fixed point (8.8)
            mode7_matrix_b: 0,
            mode7_matrix_c: 0,
            mode7_matrix_d: 256, // 1.0 in fixed point (8.8)
            mode7_center_x: 0,
            mode7_center_y: 0,
            mode7_mul_result: 0,

            m7_latch_low: [0; 6],
            m7_latch_second: [false; 6],

            framebuffer: vec![0; 256 * 224],
            subscreen_buffer: vec![0; 256 * 224],

            setini: 0,
            pseudo_hires: false,
            extbg: false,
            interlace: false,
            wio_latch_enable: false,
            stat78_latch_flag: false,
            interlace_field: false,
            overscan: false,
            obj_interlace: false,

            nmi_enabled: false,
            // 実機ではリセット直後に RDNMI フラグ(bit7)が1の状態から始まるため、初期値をtrueにしておく。
            nmi_flag: true,
            nmi_latched: false,
            rdnmi_read_in_vblank: false,

            v_blank: false,
            h_blank: false,

            vram_write_buckets: [0; 8],
            vram_write_low_count: 0,
            vram_write_high_count: 0,
            vram_last_vmain: 0,
            vram_rejects: 0,
            cgram_rejects: 0,
            oam_rejects: 0,
            vram_gap_blocks: 0,
            cgram_gap_blocks: 0,
            oam_gap_blocks: 0,
            oam_data_gap_ticks: 0,
            last_reject_frame_vram: u64::MAX,
            last_reject_frame_cgram: u64::MAX,
            last_reject_frame_oam: u64::MAX,

            important_writes_count: 0,
            vram_writes_total_low: 0,
            vram_writes_total_high: 0,
            cgram_writes_total: 0,
            oam_writes_total: 0,
            oam_write_latch: 0,
            oam_priority_rotation_enabled: false,
            obj_overflow_lines: 0,
            obj_time_over_lines: 0,
            oam_eval_base: 0,
            line_sprites: Vec::new(),
            sprite_tile_entry_counts: [0; 256],
            sprite_tile_budget_remaining: 0,
            sprite_draw_disabled: false,
            sprite_timeover_stop_x: 256,
            line_window_prepared: false,
            color_window_lut: [0; 256],
            main_bg_window_lut: [[0; 256]; 4],
            sub_bg_window_lut: [[0; 256]; 4],
            main_obj_window_lut: [0; 256],
            sub_obj_window_lut: [0; 256],
            oam_internal_addr: 0,
            hdma_head_busy_until: 0,

            // Latched display regs (disabled by default)
            latched_inidisp: None,
            latched_tm: None,
            latched_ts: None,
            latched_tmw: None,
            latched_tsw: None,
            latched_cgwsel: None,
            latched_cgadsub: None,
            latched_fixed_color: None,
            latched_setini: None,
            latched_vmadd_lo: None,
            latched_vmadd_hi: None,
            latched_cgadd: None,
            latched_vmain: None,
            vmain_effect_pending: None,
            vmain_effect_ticks: 0,
            cgadd_effect_pending: None,
            cgadd_effect_ticks: 0,
            vmain_data_gap_ticks: 0,
            cgram_data_gap_ticks: 0,
            latched_wbglog: None,
            latched_wobjlog: None,

            dbg_clip_inside: 0,
            dbg_clip_outside: 0,
            dbg_math_add: 0,
            dbg_math_sub: 0,
            dbg_math_add_half: 0,
            dbg_math_sub_half: 0,
            dbg_masked_bg: 0,
            dbg_masked_obj: 0,
            dbg_math_obj_add: 0,
            dbg_math_obj_sub: 0,
            dbg_math_obj_add_half: 0,
            dbg_math_obj_sub_half: 0,
            dbg_clip_obj_inside: 0,
            dbg_clip_obj_outside: 0,
            dbg_m7_wrap: 0,
            dbg_m7_clip: 0,
            dbg_m7_fill: 0,
            dbg_m7_bg1: 0,
            dbg_m7_bg2: 0,
            dbg_m7_edge: 0,

            dbg_win_xor_applied: 0,
            dbg_win_xnor_applied: 0,
            dbg_math_blocked: 0,
            dbg_math_blocked_obj: 0,
            dbg_math_blocked_backdrop: 0,

            write_ctx: 0,
            debug_dma_channel: None,
            burnin_vram_trace_armed: false,
            burnin_vram_trace_cnt_2118: 0,
            burnin_vram_trace_cnt_2119: 0,
        }
    }

    pub fn step(&mut self, cycles: u16) {
        // Per-CPU-cycle PPU stepping (approx 1 CPU cycle -> 1 PPU dot)
        let dots_per_line = self.dots_per_line();
        let first_hblank = self.first_hblank_dot();
        let first_visible = self.first_visible_dot();
        for _ in 0..cycles {
            // Advance any deferred control effects before processing this dot
            self.tick_deferred_ctrl_effects();
            let x = self.cycle;
            let y = self.scanline;
            let vis_last = self.get_visible_height();
            let vblank_start = vis_last.saturating_add(1);

            // Update HBlank state from dot counters.
            //
            // Official burn-in tests (HVBJOY/VH FLAG) expect $4212 bit6 (HBlank) to be set only
            // for the right-side blanking period. Do not treat the pre-visible dots as "HBlank"
            // for this flag.
            let hblank_now = x >= first_hblank;
            if hblank_now != self.h_blank {
                self.h_blank = hblank_now;
                if hblank_now && x == first_hblank {
                    // Entering right-side HBlank; guard a few dots at HBlank head for HDMA operations only.
                    let guard = crate::debug_flags::hblank_hdma_guard_dots();
                    self.hdma_head_busy_until = first_hblank.saturating_add(guard);
                }
            }

            // Start-of-line duties
            if x == 0 {
                // Commit latched regs at the beginning of each scanline
                self.commit_latched_display_regs();
                if y >= 1 && y <= vis_last {
                    // Prepare window LUTs at line start (OBJ list is prepared during previous HBlank)
                    self.prepare_line_window_luts();
                }
            }

            // After guard period, commit any pending control registers (VMADD/CGADD)
            if self.h_blank && x == self.hdma_head_busy_until {
                self.commit_pending_ctrl_if_any();
            }

            // Visible pixel render
            if !self.v_blank && y >= 1 && y <= vis_last && x >= first_visible && x < first_hblank {
                let fb_x = (x - first_visible) as usize;
                let fb_y = (y - 1) as usize;
                if fb_y < 224 {
                    self.update_obj_time_over_at_x(fb_x as u16);
                    self.render_dot(fb_x, fb_y);
                }
            }

            // Advance dot; end-of-line at DOTS_PER_LINE
            self.cycle += 1;
            if self.cycle >= dots_per_line {
                // End of scanline
                self.cycle = 0;
                self.h_blank = false; // dot 0 is not treated as HBlank for HVBJOY
                self.scanline = self.scanline.wrapping_add(1);

                // VBlank transitions
                // 通常: 可視領域終了の次のラインでVBlank突入
                if !self.v_blank && self.scanline == vblank_start {
                    if crate::debug_flags::boot_verbose() {
                        println!("📺 ENTERING VBLANK at scanline {}", self.scanline);
                    }
                    self.enter_vblank();
                } else if self.scanline == 262 {
                    // NTSC frame end (coarse). Wrap to next frame.
                    if crate::debug_flags::boot_verbose() {
                        println!("📺 FRAME END: scanline 262, resetting to 0");
                    }
                    self.exit_vblank();
                    self.scanline = 0;
                    self.frame = self.frame.wrapping_add(1);
                    // Prepare first visible line sprites ahead (scanline 0)
                    self.prepare_line_obj_pipeline(0);
                } else {
                    // Prepare next visible scanline sprites during HBlank end
                    let ny = self.scanline;
                    if ny >= 1 && ny <= vis_last {
                        let vy = ny - 1;
                        self.prepare_line_obj_pipeline(vy);
                    }
                }
            }

            // External HV latch via WRIO ($4201 bit7 1->0): latch occurs 1 dot later than $2137.
            // (See Super Famicom Development Wiki "Timing".)
            if self.wio_latch_pending_dots > 0 {
                self.wio_latch_pending_dots = self.wio_latch_pending_dots.saturating_sub(1);
                if self.wio_latch_pending_dots == 0 {
                    self.latch_hv_counters();
                }
            }
        }
    }

    pub fn latch_hv_counters(&mut self) {
        // Latch current H/V counters. Writing $2137 always updates the latched values.
        // STAT78 bit6 (latch flag) is set until $213F is read (which clears it).
        // H/V counters are 9-bit values on real hardware.
        self.hv_latched_h = self.cycle & 0x01FF;
        self.hv_latched_v = self.scanline & 0x01FF;
        // STAT78 latch flag: set when counters are latched.
        self.stat78_latch_flag = true;
        // Reset OPHCT/OPVCT read selectors so the next read returns the low byte.
        self.ophct_second = false;
        self.opvct_second = false;

        if crate::debug_flags::trace_burnin_ext_latch() {
            use std::sync::atomic::{AtomicU32, Ordering};
            static CNT: AtomicU32 = AtomicU32::new(0);
            let n = CNT.fetch_add(1, Ordering::Relaxed);
            if n < 1024 && !crate::debug_flags::quiet() {
                println!(
                    "[BURNIN-EXT][LATCH] sl={} cyc={} -> OPHCT={:03} OPVCT={:03} flag={} wio_en={}",
                    self.scanline,
                    self.cycle,
                    self.hv_latched_h,
                    self.hv_latched_v,
                    self.stat78_latch_flag as u8,
                    self.wio_latch_enable as u8
                );
            }
        }
    }

    pub fn request_wrio_hv_latch(&mut self) {
        // WRIO ($4201) external latch is documented as latching 1 dot later than a $2137 read.
        // We schedule the latch so it fires after the next dot advances.
        self.wio_latch_pending_dots = 1;
        if crate::debug_flags::trace_burnin_ext_latch() && !crate::debug_flags::quiet() {
            use std::sync::atomic::{AtomicU32, Ordering};
            static CNT: AtomicU32 = AtomicU32::new(0);
            let n = CNT.fetch_add(1, Ordering::Relaxed);
            if n < 256 {
                println!(
                    "[BURNIN-EXT][WRIO-LATCH-REQ] sl={} cyc={} pending_dots={}",
                    self.scanline, self.cycle, self.wio_latch_pending_dots
                );
            }
        }
    }

    pub fn set_wio_latch_enable(&mut self, enabled: bool) {
        self.wio_latch_enable = enabled;
    }

    // Render one pixel at the current (x,y)
    fn render_dot(&mut self, x: usize, y: usize) {
        // Debug at start of each scanline - only when not forced blank
        if x == 0 && crate::debug_flags::debug_render_dot() {
            static mut LINE_DEBUG_COUNT: u32 = 0;
            unsafe {
                let fblank = (self.screen_display & 0x80) != 0;
                if LINE_DEBUG_COUNT < 10 && (!fblank || LINE_DEBUG_COUNT < 3) {
                    LINE_DEBUG_COUNT += 1;
                    let effective = self.effective_main_screen_designation();
                    println!("🎬 RENDER_DOT[{}]: y={} main=0x{:02X} effective=0x{:02X} last_nz=0x{:02X} mode={} bright={} fblank={}",
                        LINE_DEBUG_COUNT, y, self.main_screen_designation, effective,
                        self.main_screen_designation_last_nonzero, self.bg_mode,
                        self.brightness, fblank);
                }
            }

            // Periodic CGRAM contents check (frames 1, 10, 30, 60, 100)
            static mut CGRAM_CHECK_COUNT: u32 = 0;
            unsafe {
                if y == 0 {
                    let frame = self.frame;
                    let should_check = match frame {
                        1 | 10 | 30 | 60 | 100 => true,
                        _ => false,
                    };
                    if should_check && CGRAM_CHECK_COUNT < 5 {
                        CGRAM_CHECK_COUNT += 1;
                        let mut nonzero_count = 0;
                        let mut first_colors = Vec::new();
                        for i in 0..256 {
                            let lo = self.cgram[i * 2] as u16;
                            let hi = (self.cgram[i * 2 + 1] & 0x7F) as u16;
                            let color = (hi << 8) | lo;
                            if color != 0 {
                                nonzero_count += 1;
                                if first_colors.len() < 8 {
                                    first_colors.push((i, color));
                                }
                            }
                        }
                        println!(
                            "🎨 CGRAM CHECK (frame {}): {} non-zero colors out of 256",
                            frame, nonzero_count
                        );
                        for (idx, color) in &first_colors {
                            // Convert 15-bit BGR color to RGB for display
                            let r = ((color & 0x001F) as u32) << 3;
                            let g = (((color >> 5) & 0x001F) as u32) << 3;
                            let b = (((color >> 10) & 0x001F) as u32) << 3;
                            let rgb = (r << 16) | (g << 8) | b;
                            println!("   Color[{}] = 0x{:04X} (RGB: 0x{:06X})", idx, color, rgb);
                        }
                    }
                }
            }
        }

        // Use existing per-pixel composition with color math and windows.
        let (mut main_color, mut main_layer_id) =
            self.render_main_screen_pixel_with_layer(x as u16, y as u16);
        // If main pixel is transparent, treat as backdrop for color math decisions
        if main_color == 0 {
            main_color = self.cgram_to_rgb(0);
            main_layer_id = 5; // Backdrop layer id
        }
        let (sub_color, sub_layer_id) = self.render_sub_screen_pixel_with_layer(x as u16, y as u16);
        let hires_out = self.pseudo_hires || matches!(self.bg_mode, 5 | 6);
        let final_color = if hires_out {
            let even_mix =
                self.apply_color_math_screens(main_color, sub_color, main_layer_id, x as u16);
            let odd_mix =
                self.apply_color_math_screens(sub_color, main_color, sub_layer_id, x as u16);
            Self::average_rgb(even_mix, odd_mix)
        } else {
            self.apply_color_math_screens(main_color, sub_color, main_layer_id, x as u16)
        };

        let pixel_offset = y * 256 + x;
        let final_brightness_color = self.apply_brightness(final_color);
        if pixel_offset < self.framebuffer.len() {
            self.framebuffer[pixel_offset] = final_brightness_color;
        }
        if pixel_offset < self.subscreen_buffer.len() {
            self.subscreen_buffer[pixel_offset] = sub_color;
        }
    }

    fn enter_vblank(&mut self) {
        self.v_blank = true;
        // STAT78 field flag toggles every VBlank.
        self.interlace_field = !self.interlace_field;
        self.rdnmi_read_in_vblank = false; // 新しいVBlankでリセット
                                           // RDNMIフラグ（$4210 bit7）はNMI許可に関わらずVBlank突入で立つ。
                                           // 読み出しでクリアされるが、VBlank中は常に再セットされる挙動に近づける。
        self.nmi_flag = true;
        // NMIパルスは許可時のみCPUへ届ける。ラッチを使って多重発火を防ぐ。
        if self.nmi_enabled && !self.nmi_latched {
            self.nmi_latched = true; // ensure one NMI per VBlank
        }
        if std::env::var_os("TRACE_VBLANK").is_some() {
            use std::sync::atomic::{AtomicU32, Ordering};
            static COUNT: AtomicU32 = AtomicU32::new(0);
            let n = COUNT.fetch_add(1, Ordering::Relaxed);
            if n < 8 {
                println!(
                    "[TRACE_VBLANK] frame={} scanline={} nmi_flag={} nmi_en={} latched={}",
                    self.frame, self.scanline, self.nmi_flag, self.nmi_enabled, self.nmi_latched
                );
            }
        }
    }

    fn exit_vblank(&mut self) {
        self.v_blank = false;
        // VBlankが終わったらRDNMIフラグも必ず下げる
        self.nmi_flag = false;
        self.nmi_latched = false;
        self.rdnmi_read_in_vblank = false;
        // STAT77 flags are reset at end of VBlank.
        self.sprite_overflow_latched = false;
        self.sprite_time_over_latched = false;
    }

    // Returns true if we're currently in the active display area (not V/HBlank)
    #[inline]
    fn in_active_display(&self) -> bool {
        let vis_last = self.get_visible_height();
        let v_vis = self.scanline >= 1 && self.scanline <= vis_last;
        let h_vis = self.cycle >= self.first_visible_dot() && self.cycle < self.first_hblank_dot();
        v_vis && h_vis && !self.v_blank && !self.h_blank
    }

    #[inline]
    fn is_vram_write_safe_dot(&self) -> bool {
        // VRAM data port ($2118/$2119) writes are only effective during:
        // - forced blank (INIDISP bit7), or
        // - VBlank, or
        // - a small HBlank window for DMA/HDMA (timing-sensitive titles rely on this)
        //
        // NOTE: Even when the write is ignored, VMADD still increments based on VMAIN. The
        // caller must apply the increment regardless of the return value here.
        self.can_write_vram_now()
    }

    #[inline]
    fn can_read_vram_now(&self) -> bool {
        // SNESdev wiki: VRAM reads via $2139/$213A are only valid during VBlank or forced blank.
        if (self.screen_display & 0x80) != 0 {
            return true;
        }
        let vblank_start = self.get_visible_height().saturating_add(1);
        self.v_blank || self.scanline >= vblank_start
    }

    // Centralized timing gates for graphics register writes.
    // These are coarse approximations meant to be refined over time.
    #[inline]
    fn can_write_vram_now(&self) -> bool {
        let strict = crate::debug_flags::strict_ppu_timing();
        // During forced blank (INIDISP bit7), VRAM is accessible at any time.
        if (self.screen_display & 0x80) != 0 {
            return true;
        }
        let vblank_start = self.get_visible_height().saturating_add(1);
        if self.v_blank || self.scanline >= vblank_start {
            // Optional VBlank head/tail sub-windows for MDMA/CPU
            if strict && self.write_ctx != 2 {
                let head = crate::debug_flags::vram_vblank_head();
                let tail = crate::debug_flags::vram_vblank_tail();
                let last = self.last_dot_index();
                if head > 0 && (self.scanline == vblank_start) && (self.cycle as u16) < head {
                    return false;
                }
                if tail > 0
                    && (self.scanline == 261)
                    && (self.cycle as u16) > last.saturating_sub(tail)
                {
                    return false;
                }
            }
            return true;
        }
        // Outside of VBlank/forced blank, VRAM data port writes are ignored on real hardware
        // (even during HBlank). See: SNESdev wiki ($2118/$2119 timing notes).
        //
        // We keep the STRICT_PPU_TIMING flag for future refinements, but the fundamental rule
        // is the same in both modes.
        let _ = strict;
        false
    }

    #[inline]
    fn can_write_cgram_now(&self) -> bool {
        if !crate::debug_flags::strict_ppu_timing() {
            return true;
        }
        // During forced blank (INIDISP bit7), CGRAM is accessible at any time.
        if (self.screen_display & 0x80) != 0 {
            return true;
        }
        let vblank_start = self.get_visible_height().saturating_add(1);
        if self.v_blank || self.scanline >= vblank_start {
            // Optional: enforce CGRAM MDMA/CPU head/tail guard in VBlank
            if self.write_ctx != 2 {
                let head = crate::debug_flags::cgram_vblank_head();
                let tail = crate::debug_flags::cgram_vblank_tail();
                let last = self.last_dot_index();
                if head > 0 && (self.scanline == vblank_start) && (self.cycle as u16) < head {
                    return false;
                }
                if tail > 0
                    && (self.scanline == 261)
                    && (self.cycle as u16) > last.saturating_sub(tail)
                {
                    return false;
                }
            }
            return true;
        }
        if !self.h_blank {
            return false;
        }
        // Block CGRAM data writes immediately after CGADD effect for MDMA/CPU
        if self.cgram_data_gap_ticks > 0 && self.write_ctx != 2 {
            return false;
        }
        // Permit CGRAM HDMA during HBlank (for per-scanline gradients) within a safe sub-window
        if self.write_ctx == 2 {
            let x = self.cycle as u16;
            let hb = self.first_hblank_dot();
            let last = self.last_dot_index();
            let head = hb.saturating_add(crate::debug_flags::cgram_hdma_head());
            let tail = crate::debug_flags::cgram_hdma_tail();
            return x >= head && x <= (last.saturating_sub(tail));
        }
        false
    }

    #[inline]
    fn can_write_oam_now(&self) -> bool {
        if !crate::debug_flags::strict_ppu_timing() {
            return true;
        }
        // During forced blank (INIDISP bit7), OAM is accessible at any time.
        if (self.screen_display & 0x80) != 0 {
            return true;
        }
        let vblank_start = self.get_visible_height().saturating_add(1);
        if self.v_blank || self.scanline >= vblank_start {
            // Optional: enforce OAM gap in VBlank for MDMA/CPU
            if crate::debug_flags::oam_gap_in_vblank()
                && self.oam_data_gap_ticks > 0
                && self.write_ctx != 2
            {
                return false;
            }
            if self.write_ctx != 2 {
                let head = crate::debug_flags::oam_vblank_head();
                let tail = crate::debug_flags::oam_vblank_tail();
                let last = self.last_dot_index();
                if head > 0 && (self.scanline == vblank_start) && (self.cycle as u16) < head {
                    return false;
                }
                if tail > 0
                    && (self.scanline == 261)
                    && (self.cycle as u16) > last.saturating_sub(tail)
                {
                    return false;
                }
            }
            return true;
        }
        if !self.h_blank {
            return false;
        }
        // Allow only HDMA to OAM during HBlank; avoid edges
        if self.write_ctx == 2 {
            let x = self.cycle as u16;
            let hb = self.first_hblank_dot();
            let last = self.last_dot_index();
            let head = hb.saturating_add(crate::debug_flags::oam_hdma_head());
            let tail = crate::debug_flags::oam_hdma_tail();
            return x >= head && x <= (last.saturating_sub(tail));
        }
        false
    }

    // Public helper for Bus-side VRAM MDMA gating to avoid data loss
    pub fn is_hblank_safe_for_vram_mdma(&self) -> bool {
        if !self.h_blank {
            return false;
        }
        let x = self.cycle as u16;
        let hb = self.first_hblank_dot();
        let last = self.last_dot_index();
        let head = hb.saturating_add(crate::debug_flags::vram_mdma_head());
        let tail = crate::debug_flags::vram_mdma_tail();
        let start = head.max(self.hdma_head_busy_until);
        x >= start && x <= (last.saturating_sub(tail))
    }

    // Apply any latched display-affecting registers at the start of a scanline.
    fn commit_latched_display_regs(&mut self) {
        let mut any = false;
        if let Some(v) = self.latched_inidisp.take() {
            self.screen_display = v;
            self.brightness = v & 0x0F;
            any = true;
        }
        if let Some(v) = self.latched_tm.take() {
            self.main_screen_designation = v;
            any = true;
        }
        if let Some(v) = self.latched_ts.take() {
            self.sub_screen_designation = v;
            any = true;
        }
        if let Some(v) = self.latched_tmw.take() {
            self.tmw_mask = v & 0x1F;
            any = true;
        }
        if let Some(v) = self.latched_tsw.take() {
            self.tsw_mask = v & 0x1F;
            any = true;
        }
        if let Some(v) = self.latched_cgwsel.take() {
            self.cgwsel = v;
            self.color_math_control = v;
            any = true;
        }
        if let Some(v) = self.latched_cgadsub.take() {
            self.cgadsub = v;
            self.color_math_designation = v;
            any = true;
        }
        if let Some(v) = self.latched_fixed_color.take() {
            self.fixed_color = v;
            any = true;
        }
        if let Some(v) = self.latched_setini.take() {
            self.setini = v;
            self.pseudo_hires = (v & 0x08) != 0;
            self.extbg = (v & 0x40) != 0;
            self.overscan = (v & 0x04) != 0;
            self.obj_interlace = (v & 0x02) != 0;
            self.interlace = (v & 0x01) != 0;
            any = true;
        }
        if let Some(v) = self.latched_wbglog.take() {
            self.bg_window_logic[0] = (v >> 0) & 0x03;
            self.bg_window_logic[1] = (v >> 2) & 0x03;
            self.bg_window_logic[2] = (v >> 4) & 0x03;
            self.bg_window_logic[3] = (v >> 6) & 0x03;
            any = true;
        }
        if let Some(v) = self.latched_wobjlog.take() {
            self.obj_window_logic = (v >> 0) & 0x03;
            self.color_window_logic = (v >> 2) & 0x03;
            any = true;
        }
        if any && crate::debug_flags::boot_verbose() {
            println!("PPU: latched regs committed at line {}", self.scanline);
        }
    }

    // Determine if it is safe to commit VMADD (VRAM address) now
    fn can_commit_vmadd_now(&self) -> bool {
        if !crate::debug_flags::strict_ppu_timing() {
            return true;
        }
        // During forced blank (INIDISP bit7), VRAM control regs are writable at any time.
        if (self.screen_display & 0x80) != 0 {
            return true;
        }
        let vblank_start = self.get_visible_height().saturating_add(1);
        if self.v_blank || self.scanline >= vblank_start {
            return true;
        }
        if !self.h_blank {
            return false;
        }
        let x = self.cycle as u16;
        let hb = self.first_hblank_dot();
        let last = self.last_dot_index();
        let head = hb
            .saturating_add(crate::debug_flags::vmadd_ctrl_head())
            .max(self.hdma_head_busy_until);
        let tail = crate::debug_flags::vmadd_ctrl_tail();
        x >= head && x <= (last.saturating_sub(tail))
    }

    // Determine if it is safe to commit CGADD (CGRAM address) now
    fn can_commit_cgadd_now(&self) -> bool {
        if !crate::debug_flags::strict_ppu_timing() {
            return true;
        }
        // During forced blank (INIDISP bit7), CGRAM control regs are writable at any time.
        if (self.screen_display & 0x80) != 0 {
            return true;
        }
        let vblank_start = self.get_visible_height().saturating_add(1);
        if self.v_blank || self.scanline >= vblank_start {
            return true;
        }
        if !self.h_blank {
            return false;
        }
        let x = self.cycle as u16;
        let hb = self.first_hblank_dot();
        let last = self.last_dot_index();
        let head = hb
            .saturating_add(crate::debug_flags::cgadd_ctrl_head())
            .max(self.hdma_head_busy_until);
        let tail = crate::debug_flags::cgadd_ctrl_tail();
        x >= head && x <= (last.saturating_sub(tail))
    }

    // Determine if it is safe to commit VMAIN (VRAM control) now
    fn can_commit_vmain_now(&self) -> bool {
        // Reuse VMADD control margins
        self.can_commit_vmadd_now()
    }

    // Commit pending control registers if safe
    fn commit_pending_ctrl_if_any(&mut self) {
        // VMADD
        if self.latched_vmadd_lo.is_some() || self.latched_vmadd_hi.is_some() {
            if self.can_commit_vmadd_now() {
                let mut changed = false;
                if let Some(lo) = self.latched_vmadd_lo.take() {
                    self.vram_addr = (self.vram_addr & 0xFF00) | (lo as u16);
                    changed = true;
                }
                if let Some(hi) = self.latched_vmadd_hi.take() {
                    self.vram_addr = (self.vram_addr & 0x00FF) | ((hi as u16) << 8);
                    changed = true;
                }
                if changed {
                    // SNESdev wiki: On VMADD write, vram_latch = [VMADD]
                    self.reload_vram_read_latch();
                }
            }
        }
        // CGADD
        if self.latched_cgadd.is_some() && self.can_commit_cgadd_now() {
            if let Some(v) = self.latched_cgadd.take() {
                self.cgadd_effect_pending = Some(v);
                self.cgadd_effect_ticks = crate::debug_flags::cgadd_effect_delay_dots();
            }
        }
        // VMAIN
        if let Some(v) = self.latched_vmain.take() {
            if self.can_commit_vmain_now() {
                // Defer the visible effect by a small number of dots
                self.vmain_effect_pending = Some(v);
                self.vmain_effect_ticks = crate::debug_flags::vmain_effect_delay_dots();
            } else {
                // Put back if still unsafe
                self.latched_vmain = Some(v);
            }
        }
    }

    // Tick and apply deferred control effects (called each dot)
    fn tick_deferred_ctrl_effects(&mut self) {
        if self.vmain_effect_pending.is_some() {
            if self.vmain_effect_ticks > 0 {
                self.vmain_effect_ticks -= 1;
            }
            if self.vmain_effect_ticks == 0 {
                if let Some(v) = self.vmain_effect_pending.take() {
                    self.vram_mapping = v;
                    self.vram_last_vmain = v;
                    // Update increment now that mapping took effect
                    match v & 0x03 {
                        0 => self.vram_increment = 1,
                        1 => self.vram_increment = 32,
                        2 | 3 => self.vram_increment = 128,
                        _ => {}
                    }
                    if crate::debug_flags::ppu_write() {
                        let inc = match v & 0x03 {
                            0 => 1,
                            1 => 32,
                            _ => 128,
                        };
                        let fg = (v >> 2) & 0x03;
                        let inc_on_high = (v & 0x80) != 0;
                        println!(
                            "VMAIN applied: 0x{:02X} (inc={}, FGmode={}, inc_on_{})",
                            v,
                            inc,
                            fg,
                            if inc_on_high { "HIGH" } else { "LOW" }
                        );
                    }
                    // Start a small MDMA/CPU gap after VMAIN effect
                    self.vmain_data_gap_ticks = crate::debug_flags::vram_gap_after_vmain();
                }
            }
        }
        if self.vmain_data_gap_ticks > 0 {
            self.vmain_data_gap_ticks -= 1;
        }
        if self.oam_data_gap_ticks > 0 {
            self.oam_data_gap_ticks -= 1;
        }
        if self.cgadd_effect_pending.is_some() {
            if self.cgadd_effect_ticks > 0 {
                self.cgadd_effect_ticks -= 1;
            }
            if self.cgadd_effect_ticks == 0 {
                if let Some(v) = self.cgadd_effect_pending.take() {
                    self.cgram_addr = v;
                    self.cgram_second = false;
                    self.cgram_read_second = false;
                    if crate::debug_flags::ppu_write() {
                        println!("CGADD applied: 0x{:02X}", v);
                    }
                    // Start a small MDMA/CPU gap after CGADD effect
                    self.cgram_data_gap_ticks = crate::debug_flags::cgram_gap_after_cgadd();
                }
            }
        }
    }

    #[allow(dead_code)]
    fn render_scanline(&mut self) {
        if crate::debug_flags::boot_verbose() {
            // Debug scanline rendering
            static mut SCANLINE_DEBUG_COUNT: u32 = 0;
            unsafe {
                SCANLINE_DEBUG_COUNT += 1;
                if SCANLINE_DEBUG_COUNT <= 10 || SCANLINE_DEBUG_COUNT % 1000 == 0 {
                    println!(
                        "🖼️ SCANLINE RENDER[{}]: line={}, brightness={}, forced_blank={}",
                        SCANLINE_DEBUG_COUNT,
                        self.scanline,
                        self.brightness,
                        (self.screen_display & 0x80) != 0
                    );
                }
            }
        }

        // 画面表示が有効でなくても、テストパターンを表示
        let y = self.scanline as usize;

        if crate::debug_flags::boot_verbose() {
            static mut SCANLINE_CHECK_COUNT: u32 = 0;
            unsafe {
                SCANLINE_CHECK_COUNT += 1;
                if SCANLINE_CHECK_COUNT <= 5 {
                    println!(
                        "🔍 SCANLINE CHECK: y={}, scanline={}, condition y >= 224: {}",
                        y,
                        self.scanline,
                        y >= 224
                    );
                }
            }
        }

        if y >= 224 {
            if crate::debug_flags::boot_verbose() {
                println!("🚫 SCANLINE SKIPPED: y={} >= 224, returning early", y);
            }
            return;
        }

        // Render pixels for scanline y

        // Use game-provided screen designation as-is.

        // Debug: Check main screen designation during rendering
        if crate::debug_flags::render_verbose() && !crate::debug_flags::quiet() {
            static mut RENDER_DEBUG_COUNT: u32 = 0;
            unsafe {
                if RENDER_DEBUG_COUNT < 10 {
                    RENDER_DEBUG_COUNT += 1;
                    let effective = self.effective_main_screen_designation();
                    println!("🎬 RENDER[{}]: y={} main_screen=0x{:02X} effective=0x{:02X} last_nonzero=0x{:02X} bg_mode={} brightness={} forced_blank={}",
                        RENDER_DEBUG_COUNT, y, self.main_screen_designation, effective,
                        self.main_screen_designation_last_nonzero, self.bg_mode,
                        self.brightness, (self.screen_display & 0x80) != 0);
                }
            }
        }

        // CRITICAL DEBUG: Verify we reach this point
        // Process 256 pixels for this scanline

        // Debug: Report pixel loop entry
        if crate::debug_flags::boot_verbose() {
            static mut PIXEL_LOOP_REPORTED: bool = false;
            unsafe {
                if !PIXEL_LOOP_REPORTED {
                    println!("🖼️ PIXEL LOOP: Starting pixel rendering for line {}", y);
                    PIXEL_LOOP_REPORTED = true;
                }
            }
        }

        // Render all 256 pixels
        for x in 0..256 {
            // メインスクリーンとサブスクリーンを個別に描画（レイヤID付き）
            let (main_color, main_layer_id) =
                self.render_main_screen_pixel_with_layer(x as u16, y as u16);
            let (sub_color, sub_layer_id) =
                self.render_sub_screen_pixel_with_layer(x as u16, y as u16);

            let final_color = if self.pseudo_hires {
                // 疑似ハイレゾ: 512pxを256pxに折りたたむ近似として、
                // main→sub と sub→main の両方の合成結果を平均化。
                let even_mix =
                    self.apply_color_math_screens(main_color, sub_color, main_layer_id, x as u16);
                let odd_mix =
                    self.apply_color_math_screens(sub_color, main_color, sub_layer_id, x as u16);
                Self::average_rgb(even_mix, odd_mix)
            } else {
                // カラー演算適用（対象レイヤに限定）
                self.apply_color_math_screens(main_color, sub_color, main_layer_id, x as u16)
            };

            let pixel_offset = y * 256 + x;

            // Debug render_scanline sanity
            if crate::debug_flags::boot_verbose() {
                static mut RENDER_SCANLINE_CALLS: u32 = 0;
                static mut REAL_GRAPHICS_SHOWN: bool = false;
                unsafe {
                    RENDER_SCANLINE_CALLS += 1;
                    if !REAL_GRAPHICS_SHOWN && x == 0 && y == 0 {
                        println!(
                            "🎮 RENDER_SCANLINE[{}]: x={}, y={}, first final_color=0x{:08X}",
                            RENDER_SCANLINE_CALLS, x, y, final_color
                        );
                        REAL_GRAPHICS_SHOWN = true;
                    } else if RENDER_SCANLINE_CALLS <= 100 && x == 0 {
                        println!(
                            "📺 SCANLINE PIXEL[{}]: y={}, first_final_color=0x{:08X}",
                            RENDER_SCANLINE_CALLS, y, final_color
                        );
                    }
                }
            }

            // 画面の明度（INIDISP）を適用
            let final_brightness_color = self.apply_brightness(final_color);
            self.framebuffer[pixel_offset] = final_brightness_color;

            // Debug white pixel writing to framebuffer
            if crate::debug_flags::boot_verbose() {
                static mut WHITE_PIXEL_DEBUG: u32 = 0;
                if final_brightness_color != 0xFF000000 {
                    unsafe {
                        WHITE_PIXEL_DEBUG += 1;
                        if WHITE_PIXEL_DEBUG <= 10 {
                            println!(
                                "🖼️ FRAMEBUFFER[{}]: pos={} final=0x{:08X} (brightness={})",
                                WHITE_PIXEL_DEBUG,
                                pixel_offset,
                                final_brightness_color,
                                self.brightness
                            );
                        }
                    }
                }
            }
            self.subscreen_buffer[pixel_offset] = sub_color;

            // Debug framebuffer writes
            if crate::debug_flags::boot_verbose() {
                static mut FRAMEBUFFER_DEBUG_COUNT: u32 = 0;
                unsafe {
                    FRAMEBUFFER_DEBUG_COUNT += 1;
                    if FRAMEBUFFER_DEBUG_COUNT <= 5 {
                        println!(
                            "🖼️ FRAMEBUFFER[{}]: pos={} final=0x{:08X} (brightness={})",
                            FRAMEBUFFER_DEBUG_COUNT,
                            pixel_offset,
                            final_brightness_color,
                            self.brightness
                        );
                    }
                }
            }
        }
    }

    #[allow(dead_code)]
    fn get_pixel_color(&mut self, x: u16, y: u16) -> u32 {
        // Respect forced blank: when set, output black regardless of scene state
        let mut forced_blank = (self.screen_display & 0x80) != 0;
        if self.force_display_active() {
            forced_blank = false;
        }
        if forced_blank {
            return 0xFF000000;
        }

        if crate::debug_flags::boot_verbose() {
            static mut EMERGENCY_DEBUG_COUNT: u32 = 0;
            static mut PIXEL_CALL_COUNT: u32 = 0;
            unsafe {
                PIXEL_CALL_COUNT += 1;
                EMERGENCY_DEBUG_COUNT += 1;
                if EMERGENCY_DEBUG_COUNT <= 3 {
                    println!(
                        "🔍 GET_PIXEL_COLOR CALLED[{}]: x={}, y={}, forced_blank={}, brightness={}",
                        EMERGENCY_DEBUG_COUNT, x, y, forced_blank, self.brightness
                    );
                    println!(
                        "   📊 Total get_pixel_color calls: {} (from render_scanline)",
                        PIXEL_CALL_COUNT
                    );
                }
            }
        }

        // Dragon Quest III Data Analysis: Check if game has loaded graphics data
        if x == 0 && y == 1 {
            if crate::debug_flags::boot_verbose() {
                static mut DATA_ANALYSIS_DONE: bool = false;
                unsafe {
                    if !DATA_ANALYSIS_DONE {
                        DATA_ANALYSIS_DONE = true;
                        // Check CGRAM for color data
                        let mut non_black_colors = 0;
                        let mut sample_colors = Vec::new();
                        for i in 1..16 {
                            let addr = i * 2;
                            if addr + 1 < self.cgram.len() {
                                let color = ((self.cgram[addr + 1] as u16) << 8)
                                    | (self.cgram[addr] as u16);
                                if color != 0 {
                                    non_black_colors += 1;
                                    if sample_colors.len() < 5 {
                                        sample_colors.push(format!("#{}: 0x{:04X}", i, color));
                                    }
                                }
                            }
                        }
                        println!(
                            "🎨 DQ3 CGRAM ANALYSIS: {} non-black colors out of 15",
                            non_black_colors
                        );
                        if !sample_colors.is_empty() {
                            println!("   Sample colors: {}", sample_colors.join(", "));
                        }
                        // Check VRAM for tile data
                        let mut non_zero_vram = 0;
                        for i in (0..std::cmp::min(0x1000, self.vram.len())).step_by(4) {
                            if self.vram[i] != 0 {
                                non_zero_vram += 1;
                            }
                        }
                        println!(
                            "🗂️ DQ3 VRAM ANALYSIS: {} non-zero bytes in first 0x1000 bytes",
                            non_zero_vram
                        );
                        // Check BG settings
                        println!(
                            "🖼️ DQ3 BG SETTINGS: mode={}, tile16=[{},{},{},{}]",
                            self.bg_mode,
                            self.bg_tile_16[0],
                            self.bg_tile_16[1],
                            self.bg_tile_16[2],
                            self.bg_tile_16[3]
                        );
                        println!("   BG bases: tilemap=[0x{:04X}, 0x{:04X}, 0x{:04X}, 0x{:04X}], tile=[0x{:04X}, 0x{:04X}, 0x{:04X}, 0x{:04X}]",
                                self.bg1_tilemap_base, self.bg2_tilemap_base, self.bg3_tilemap_base, self.bg4_tilemap_base,
                                self.bg1_tile_base, self.bg2_tile_base, self.bg3_tile_base, self.bg4_tile_base);
                    }
                }
            }
        }

        // BGとスプライトの情報を取得 - Use main BG pixel function for proper graphics
        let (bg_color, bg_priority, bg_id) = self.get_main_bg_pixel(x, y);
        let (sprite_color, sprite_priority) = self.get_sprite_pixel(x, y);

        // Emergency test pattern removed - now showing actual DQ3 graphics

        // Debug pixel color generation (first few pixels only)
        if crate::debug_flags::boot_verbose() {
            static mut PIXEL_DEBUG_COUNT: u32 = 0;
            unsafe {
                PIXEL_DEBUG_COUNT += 1;
                if PIXEL_DEBUG_COUNT <= 10 && x < 3 && y < 3 {
                    println!("🎨 PIXEL[{},{}]: bg_color=0x{:08X}, bg_priority={}, sprite_color=0x{:08X}, sprite_priority={}", 
                            x, y, bg_color, bg_priority, sprite_color, sprite_priority);
                    // Check if CGRAM has any non-black data for palette colors 1-15
                    let non_zero_colors = (1..16)
                        .map(|i| {
                            let addr = i * 2;
                            if addr + 1 < self.cgram.len() {
                                let color = ((self.cgram[addr + 1] as u16) << 8)
                                    | (self.cgram[addr] as u16);
                                color != 0
                            } else {
                                false
                            }
                        })
                        .filter(|&x| x)
                        .count();
                    if PIXEL_DEBUG_COUNT == 1 {
                        println!(
                            "🎨 CGRAM: Non-zero colors in palette 1-15: {}/15",
                            non_zero_colors
                        );
                        println!("🎨 PPU STATE: bg_mode={}, main_screen_designation=0x{:02X}, sub_screen_designation=0x{:02X}", 
                                self.bg_mode, self.main_screen_designation, self.sub_screen_designation);
                        println!("🎨 PPU STATE: screen_display=0x{:02X} (forced_blank={}), brightness={}", 
                                self.screen_display, (self.screen_display & 0x80) != 0, self.brightness);
                    }
                }
            }
        }

        // プライオリティベースの合成
        let (final_color, _lid) = self.composite_pixel_with_layer(
            bg_color,
            bg_priority,
            bg_id,
            sprite_color,
            sprite_priority,
        );

        if crate::debug_flags::boot_verbose() {
            if x < 2 && y < 2 {
                println!(
                    "🎨 COMPOSITE[{},{}]: final_color=0x{:08X}",
                    x, y, final_color
                );
            }
        }

        if final_color != 0 {
            let result = self.apply_brightness(final_color);
            if crate::debug_flags::boot_verbose() {
                if x < 2 && y < 2 {
                    println!(
                        "🎨 BRIGHT[{},{}]: final_color=0x{:08X} -> brightness_applied=0x{:08X}",
                        x, y, final_color, result
                    );
                }
            }
            return result;
        }

        // No emergency forcing. If nothing composites, use backdrop color (palette index 0)
        // バックドロップカラー（CGRAMの0番）を使用（代替色は使わない）
        let backdrop = self.cgram_to_rgb(0);
        let result = self.apply_brightness(backdrop);
        if crate::debug_flags::boot_verbose() {
            if x < 2 && y < 2 {
                println!(
                    "🎨 BACKDROP[{},{}]: backdrop=0x{:08X} -> brightness_applied=0x{:08X}",
                    x, y, backdrop, result
                );
            }
        }
        result
    }

    // プライオリティベースのピクセル合成
    #[allow(dead_code)]
    fn composite_pixel(
        &self,
        bg_color: u32,
        bg_priority: u8,
        sprite_color: u32,
        sprite_priority: u8,
    ) -> u32 {
        let (final_color, _layer_id) = self.composite_pixel_with_layer(
            bg_color,
            bg_priority,
            0,
            sprite_color,
            sprite_priority,
        );
        // 画面間のカラー演算は apply_color_math_screens() で一括適用する。
        // ここではレイヤー合成のみ行う。
        final_color
    }

    fn composite_pixel_with_layer(
        &self,
        bg_color: u32,
        bg_priority: u8,
        bg_layer_id: u8,
        sprite_color: u32,
        sprite_priority: u8,
    ) -> (u32, u8) {
        // 透明なピクセルをチェック
        let sprite_transparent = self.is_transparent_pixel(sprite_color);
        let bg_transparent = self.is_transparent_pixel(bg_color);
        if sprite_transparent && bg_transparent {
            // Fully transparent: caller will fall back to backdrop color.
            return (0, 5);
        }
        if sprite_transparent {
            return (bg_color, bg_layer_id);
        }
        if bg_transparent {
            return (sprite_color, 4);
        }

        // Unified priority model via z-rank table (default). Fallback to legacy if requested.
        if crate::debug_flags::priority_model_variant() == 1 {
            let z_obj = self.z_rank_for_obj(sprite_priority);
            let z_bg = self.z_rank_for_bg(bg_layer_id, bg_priority);
            if z_obj > z_bg || (z_obj == z_bg) {
                (sprite_color, 4)
            } else {
                (bg_color, bg_layer_id)
            }
        } else {
            // Legacy path: approximate compare
            if sprite_priority > bg_priority {
                (sprite_color, 4)
            } else {
                (bg_color, bg_layer_id)
            }
        }
    }

    #[inline]
    fn z_rank_for_obj(&self, pr: u8) -> i16 {
        match self.bg_mode {
            7 => match pr {
                3 => crate::debug_flags::m7_z_obj3(),
                2 => crate::debug_flags::m7_z_obj2(),
                1 => crate::debug_flags::m7_z_obj1(),
                _ => crate::debug_flags::m7_z_obj0(),
            },
            _ => match pr {
                3 => 90,
                2 => 70,
                1 => 50,
                _ => 40,
            },
        }
    }

    #[inline]
    fn z_rank_for_bg(&self, layer: u8, pr: u8) -> i16 {
        match self.bg_mode {
            0 => {
                // Any BG: hi over OBJ2; lo between OBJ2 and OBJ1
                if pr >= 1 {
                    80
                } else {
                    60
                }
            }
            1 => {
                // Distinguish BG1(0), BG2(1), BG3(2)
                let bg3_slot_high = self.mode1_bg3_priority; // $2105 bit3
                if layer == 2 {
                    if bg3_slot_high {
                        55
                    } else {
                        20
                    }
                } else if layer == 1 {
                    // BG2
                    if pr >= 1 {
                        80
                    } else {
                        60
                    }
                } else {
                    // BG1
                    if pr >= 1 {
                        70
                    } else {
                        50
                    }
                }
            }
            2 => {
                // BG1 over BG2 at each priority
                match (layer, pr) {
                    (0, 1) => 80,
                    (1, 1) => 70,
                    (0, _) => 50,
                    _ => 40,
                }
            }
            3 | 4 => {
                // BG1 over BG2
                match (layer, pr) {
                    (0, 1) => 80,
                    (1, 1) => 70,
                    (0, _) => 50,
                    _ => 40,
                }
            }
            5 | 6 => {
                // Mode 5/6: BG1 and BG2 have distinct priority slots.
                // Order (front->back) from SNESdev: OBJ3, BG1H, OBJ2, BG2H, OBJ1, BG1L, OBJ0, BG2L.
                // We map into the generic OBJ z-ranks (90/70/50/40) by placing BG ranks between them.
                match (layer, pr) {
                    (0, 1) => 80, // BG1 high
                    (1, 1) => 60, // BG2 high
                    (0, _) => 45, // BG1 low
                    _ => 35,      // BG2 low (and any other BG)
                }
            }
            7 => {
                // EXTBG z-ranks are tunable via env for precise ordering experiments
                if layer == 1 {
                    crate::debug_flags::m7_z_bg2()
                } else {
                    crate::debug_flags::m7_z_bg1()
                }
            }
            _ => {
                if pr >= 1 {
                    60
                } else {
                    40
                }
            }
        }
    }

    // 共通のスプライトピクセル取得（画面有効フラグを引数で渡す）
    fn get_sprite_pixel_common(&self, x: u16, y: u16, enabled: bool, is_main: bool) -> (u32, u8) {
        if !enabled {
            return (0, 0);
        }
        if self.should_mask_sprite(x, is_main) {
            return (0, 0);
        }
        let x_i16 = x as i16;
        let y_u8 = y as u8;
        let sprites = &self.line_sprites;

        // 優先度順に描画（高優先度から低優先度へ）
        for priority in (0..4).rev() {
            for sprite in sprites {
                if sprite.priority != priority {
                    continue;
                }
                let (sprite_width, sprite_height) = self.get_sprite_pixel_size(&sprite.size);
                let sx = Self::sprite_x_signed(sprite.x);
                let sy = sprite.y;
                // Y is 8-bit and wraps; test overlap via wrapped subtraction.
                let dy = y_u8.wrapping_sub(sy);
                if (dy as u16) >= sprite_height as u16 {
                    continue;
                }
                if x_i16 < sx || x_i16 >= sx.saturating_add(sprite_width as i16) {
                    continue;
                }

                // スプライト内相対座標→タイル/ピクセル座標
                let rel_x = (x_i16 - sx) as u8;
                let rel_y = dy;
                let tile_x = rel_x / 8;
                let tile_y = rel_y / 8;
                let pixel_x = rel_x % 8;
                let pixel_y = rel_y % 8;
                // Time-over gating: allow only tiles whose 8px block started before stop_x
                if self.sprite_timeover_stop_x < 256 {
                    let stop_x = self.sprite_timeover_stop_x as i16;
                    let tile_start_x = sx.saturating_add((tile_x as i16) * 8);
                    if tile_start_x >= stop_x {
                        continue;
                    }
                }
                let color = self.render_sprite_tile(sprite, tile_x, tile_y, pixel_x, pixel_y);
                if color != 0 {
                    return (color, sprite.priority);
                }
            }
        }
        (0, 0)
    }

    // Helper: Get effective main screen designation for rendering
    #[inline]
    fn effective_main_screen_designation(&self) -> u8 {
        if self.main_screen_designation == 0 {
            self.main_screen_designation_last_nonzero
        } else {
            self.main_screen_designation
        }
    }

    // メインスクリーン用スプライト
    fn get_sprite_pixel(&self, x: u16, y: u16) -> (u32, u8) {
        let enabled = (self.effective_main_screen_designation() & 0x10) != 0;
        self.get_sprite_pixel_common(x, y, enabled, true)
    }

    fn get_main_bg_layers(&mut self, x: u16, y: u16) -> Vec<(u32, u8, u8)> {
        // Return all background layers with their colors, priorities, and layer IDs
        let mut bg_results = Vec::new();

        // Debug: Sample a few pixels to see what's being rendered
        static mut BG_PIXEL_DEBUG: u32 = 0;
        unsafe {
            if crate::debug_flags::debug_bg_pixel() && BG_PIXEL_DEBUG < 5 && x == 100 && y == 100 {
                BG_PIXEL_DEBUG += 1;
                println!(
                    "🎨 BG_PIXEL[{}] at ({},{}) mode={} effective=0x{:02X}",
                    BG_PIXEL_DEBUG,
                    x,
                    y,
                    self.bg_mode,
                    self.effective_main_screen_designation()
                );
            }
        }

        match self.bg_mode {
            0 => {
                // Mode 0: BG1-4 全て2bpp
                if self.effective_main_screen_designation() & 0x01 != 0
                    && !self.should_mask_bg(x, 0, true)
                {
                    let (color, priority) = self.render_bg_mode0_with_priority(x, y, 0);
                    unsafe {
                        if crate::debug_flags::debug_bg_pixel()
                            && BG_PIXEL_DEBUG <= 5
                            && x == 100
                            && y == 100
                        {
                            println!("  BG1: color=0x{:08X} priority={}", color, priority);
                        }
                    }
                    if color != 0 {
                        bg_results.push((color, priority, 0));
                    }
                }
                if self.effective_main_screen_designation() & 0x02 != 0
                    && !self.should_mask_bg(x, 1, true)
                {
                    let (color, priority) = self.render_bg_mode0_with_priority(x, y, 1);
                    unsafe {
                        if crate::debug_flags::debug_bg_pixel()
                            && BG_PIXEL_DEBUG <= 5
                            && x == 100
                            && y == 100
                        {
                            println!("  BG2: color=0x{:08X} priority={}", color, priority);
                        }
                    }
                    if color != 0 {
                        bg_results.push((color, priority, 1));
                    }
                }
                if self.effective_main_screen_designation() & 0x04 != 0
                    && !self.should_mask_bg(x, 2, true)
                {
                    let (color, priority) = self.render_bg_mode0_with_priority(x, y, 2);
                    if color != 0 {
                        bg_results.push((color, priority, 2));
                    }
                }
                if self.effective_main_screen_designation() & 0x08 != 0
                    && !self.should_mask_bg(x, 3, true)
                {
                    let (color, priority) = self.render_bg_mode0_with_priority(x, y, 3);
                    if color != 0 {
                        bg_results.push((color, priority, 3));
                    }
                }
            }
            1 => {
                // Mode 1: BG1/BG2は4bpp、BG3は2bpp
                if self.effective_main_screen_designation() & 0x01 != 0
                    && !self.should_mask_bg(x, 0, true)
                {
                    let (color, priority) = self.render_bg_4bpp_with_priority(x, y, 0);
                    if color != 0 {
                        bg_results.push((color, priority, 0));
                    }
                }
                if self.effective_main_screen_designation() & 0x02 != 0
                    && !self.should_mask_bg(x, 1, true)
                {
                    let (color, priority) = self.render_bg_4bpp_with_priority(x, y, 1);
                    if color != 0 {
                        bg_results.push((color, priority, 1));
                    }
                }
                if self.effective_main_screen_designation() & 0x04 != 0 {
                    let (color, priority) = self.render_bg_mode0_with_priority(x, y, 2);
                    if color != 0 {
                        bg_results.push((color, priority, 2));
                    }
                }
            }
            4 => {
                // Mode 4: BG1は8bpp、BG2は2bpp
                if self.effective_main_screen_designation() & 0x01 != 0
                    && !self.should_mask_bg(x, 0, true)
                {
                    let (color, priority) = self.render_bg_8bpp_with_priority(x, y, 0);
                    if color != 0 {
                        bg_results.push((color, priority, 0));
                    }
                }
                if self.effective_main_screen_designation() & 0x02 != 0
                    && !self.should_mask_bg(x, 1, true)
                {
                    let (color, priority) = self.render_bg_mode0_with_priority(x, y, 1);
                    if color != 0 {
                        bg_results.push((color, priority, 1));
                    }
                }
            }
            2 => {
                // Mode 2: BG1/BG2は4bpp（オフセットパータイル機能付き）
                if self.effective_main_screen_designation() & 0x01 != 0
                    && !self.should_mask_bg(x, 0, true)
                {
                    let (color, priority) = self.render_bg_mode2_with_priority(x, y, 0);
                    if color != 0 {
                        bg_results.push((color, priority, 0));
                    }
                }
                if self.effective_main_screen_designation() & 0x02 != 0
                    && !self.should_mask_bg(x, 1, true)
                {
                    let (color, priority) = self.render_bg_mode2_with_priority(x, y, 1);
                    if color != 0 {
                        bg_results.push((color, priority, 1));
                    }
                }
            }
            3 => {
                // Mode 3: BG1は8bpp、BG2は4bpp
                if self.effective_main_screen_designation() & 0x01 != 0
                    && !self.should_mask_bg(x, 0, true)
                {
                    let (color, priority) = self.render_bg_8bpp_with_priority(x, y, 0);
                    if color != 0 {
                        bg_results.push((color, priority, 0));
                    }
                }
                if self.effective_main_screen_designation() & 0x02 != 0
                    && !self.should_mask_bg(x, 1, true)
                {
                    let (color, priority) = self.render_bg_4bpp_with_priority(x, y, 1);
                    if color != 0 {
                        bg_results.push((color, priority, 1));
                    }
                }
            }
            5 => {
                // Mode 5: BG1は4bpp、BG2は2bpp（高解像度）
                // Note: Some games (e.g., DQ3) also use BG3 in Mode 5
                if self.effective_main_screen_designation() & 0x01 != 0
                    && !self.should_mask_bg(x, 0, true)
                {
                    let (color, priority) = self.render_bg_mode5_with_priority(x, y, 0);
                    if color != 0 {
                        bg_results.push((color, priority, 0));
                    }
                }
                if self.effective_main_screen_designation() & 0x02 != 0
                    && !self.should_mask_bg(x, 1, true)
                {
                    let (color, priority) = self.render_bg_mode5_with_priority(x, y, 1);
                    if color != 0 {
                        bg_results.push((color, priority, 1));
                    }
                }
                if self.effective_main_screen_designation() & 0x04 != 0
                    && !self.should_mask_bg(x, 2, true)
                {
                    let (color, priority) = self.render_bg_mode5_with_priority(x, y, 2);
                    if color != 0 {
                        bg_results.push((color, priority, 2));
                    }
                }
            }
            6 => {
                // Mode 6: BG1は4bpp（高解像度）
                if self.effective_main_screen_designation() & 0x01 != 0
                    && !self.should_mask_bg(x, 0, true)
                {
                    let (color, priority) = self.render_bg_mode6_with_priority(x, y, 0);
                    if color != 0 {
                        bg_results.push((color, priority, 0));
                    }
                }
            }
            7 => {
                let (c, p, lid) = self.render_mode7_with_layer(x, y);
                if c != 0 {
                    let id = if self.extbg { lid } else { 0 };
                    let en_bit = 1u8 << id;
                    if (self.effective_main_screen_designation() & en_bit) != 0
                        && !self.should_mask_bg(x, id, true)
                    {
                        bg_results.push((c, p, id));
                    }
                }
            }
            _ => {
                // Unknown mode, return empty
            }
        }

        bg_results
    }

    #[allow(dead_code)]
    fn get_bg_pixel(&self, x: u16, y: u16) -> (u32, u8) {
        // Debug background layer enable status
        static mut BG_PIXEL_DEBUG: bool = false;
        unsafe {
            if !BG_PIXEL_DEBUG && x == 0 && y == 1 {
                println!(
                    "🎮 GET_BG_PIXEL: bg_mode={}, main_screen=0x{:02X}, bg_enables=[{},{},{},{}]",
                    self.bg_mode,
                    self.main_screen_designation,
                    self.effective_main_screen_designation() & 0x01 != 0,
                    self.effective_main_screen_designation() & 0x02 != 0,
                    self.effective_main_screen_designation() & 0x04 != 0,
                    self.effective_main_screen_designation() & 0x08 != 0
                );
                BG_PIXEL_DEBUG = true;
            }
        }

        // 全BGレイヤーの描画結果とプライオリティを取得
        let mut bg_results = Vec::new();

        match self.bg_mode {
            0 => {
                // Mode 0: BG1-4 全て2bpp
                if self.effective_main_screen_designation() & 0x01 != 0
                    && !self.should_mask_bg(x, 0, true)
                {
                    let (color, priority) = self.render_bg_mode0_with_priority(x, y, 0);
                    if color != 0 {
                        bg_results.push((color, priority, 0));
                    }
                }
                if self.effective_main_screen_designation() & 0x02 != 0
                    && !self.should_mask_bg(x, 1, true)
                {
                    let (color, priority) = self.render_bg_mode0_with_priority(x, y, 1);
                    if color != 0 {
                        bg_results.push((color, priority, 1));
                    }
                }
                if self.effective_main_screen_designation() & 0x04 != 0
                    && !self.should_mask_bg(x, 2, true)
                {
                    let (color, priority) = self.render_bg_mode0_with_priority(x, y, 2);
                    if color != 0 {
                        bg_results.push((color, priority, 2));
                    }
                }
                if self.effective_main_screen_designation() & 0x08 != 0
                    && !self.should_mask_bg(x, 3, true)
                {
                    let (color, priority) = self.render_bg_mode0_with_priority(x, y, 3);
                    if color != 0 {
                        bg_results.push((color, priority, 3));
                    }
                }
            }
            1 => {
                // Mode 1: BG1/BG2は4bpp、BG3は2bpp
                if self.effective_main_screen_designation() & 0x01 != 0
                    && !self.should_mask_bg(x, 0, true)
                {
                    let (color, priority) = self.render_bg_4bpp_with_priority(x, y, 0);
                    if color != 0 {
                        bg_results.push((color, priority, 0));
                    }
                }
                if self.effective_main_screen_designation() & 0x02 != 0
                    && !self.should_mask_bg(x, 1, true)
                {
                    let (color, priority) = self.render_bg_4bpp_with_priority(x, y, 1);
                    if color != 0 {
                        bg_results.push((color, priority, 1));
                    }
                }
                if self.effective_main_screen_designation() & 0x04 != 0 {
                    let (color, priority) = self.render_bg_mode0_with_priority(x, y, 2);
                    if color != 0 {
                        bg_results.push((color, priority, 2));
                    }
                }
            }
            4 => {
                // Mode 4: BG1は8bpp、BG2は2bpp
                if self.effective_main_screen_designation() & 0x01 != 0
                    && !self.should_mask_bg(x, 0, true)
                {
                    let (color, priority) = self.render_bg_8bpp_with_priority(x, y, 0);
                    if color != 0 {
                        bg_results.push((color, priority, 0));
                    }
                }
                if self.effective_main_screen_designation() & 0x02 != 0
                    && !self.should_mask_bg(x, 1, true)
                {
                    let (color, priority) = self.render_bg_mode0_with_priority(x, y, 1);
                    if color != 0 {
                        bg_results.push((color, priority, 1));
                    }
                }
            }
            2 => {
                // Mode 2: BG1/BG2は4bpp（オフセットパータイル機能付き）
                if self.effective_main_screen_designation() & 0x01 != 0
                    && !self.should_mask_bg(x, 0, true)
                {
                    let (color, priority) = self.render_bg_mode2_with_priority(x, y, 0);
                    if color != 0 {
                        bg_results.push((color, priority, 0));
                    }
                }
                if self.effective_main_screen_designation() & 0x02 != 0
                    && !self.should_mask_bg(x, 1, true)
                {
                    let (color, priority) = self.render_bg_mode2_with_priority(x, y, 1);
                    if color != 0 {
                        bg_results.push((color, priority, 1));
                    }
                }
            }
            3 => {
                // Mode 3: BG1は8bpp、BG2は4bpp
                if self.effective_main_screen_designation() & 0x01 != 0
                    && !self.should_mask_bg(x, 0, true)
                {
                    let (color, priority) = self.render_bg_8bpp_with_priority(x, y, 0);
                    if color != 0 {
                        bg_results.push((color, priority, 0));
                    }
                }
                if self.effective_main_screen_designation() & 0x02 != 0
                    && !self.should_mask_bg(x, 1, true)
                {
                    let (color, priority) = self.render_bg_4bpp_with_priority(x, y, 1);
                    if color != 0 {
                        bg_results.push((color, priority, 1));
                    }
                }
            }
            5 => {
                // Mode 5: BG1は4bpp、BG2は2bpp（高解像度）
                // Note: Some games (e.g., DQ3) also use BG3 in Mode 5
                if self.effective_main_screen_designation() & 0x01 != 0
                    && !self.should_mask_bg(x, 0, true)
                {
                    let (color, priority) = self.render_bg_mode5_with_priority(x, y, 0);
                    if color != 0 {
                        bg_results.push((color, priority, 0));
                    }
                }
                if self.effective_main_screen_designation() & 0x02 != 0
                    && !self.should_mask_bg(x, 1, true)
                {
                    let (color, priority) = self.render_bg_mode5_with_priority(x, y, 1);
                    if color != 0 {
                        bg_results.push((color, priority, 1));
                    }
                }
                if self.effective_main_screen_designation() & 0x04 != 0
                    && !self.should_mask_bg(x, 2, true)
                {
                    let (color, priority) = self.render_bg_mode5_with_priority(x, y, 2);
                    if color != 0 {
                        bg_results.push((color, priority, 2));
                    }
                }
            }
            6 => {
                // Mode 6: BG1は4bpp（高解像度）
                if self.effective_main_screen_designation() & 0x01 != 0
                    && !self.should_mask_bg(x, 0, true)
                {
                    let (color, priority) = self.render_bg_mode6_with_priority(x, y, 0);
                    if color != 0 {
                        bg_results.push((color, priority, 0));
                    }
                }
            }
            7 => {
                // Mode 7: BG1（EXTBG時はBG2相当もあり）
                if self.effective_main_screen_designation() & 0x01 != 0
                    && !self.should_mask_bg(x, 0, true)
                {
                    // Use with-layer sampler to decide BG1/BG2 based on color index bit7 when EXTBG
                    // Apply flips and outside handling same as render_bg_mode7
                    let sx = if (self.m7sel & 0x01) != 0 {
                        255 - (x as i32)
                    } else {
                        x as i32
                    };
                    let sy = if (self.m7sel & 0x02) != 0 {
                        255 - (y as i32)
                    } else {
                        y as i32
                    };
                    let (wx, wy) = self.mode7_world_xy_int(sx, sy);
                    let repeat_off = (self.m7sel & 0x80) != 0;
                    let fill_char0 = (self.m7sel & 0x40) != 0;
                    let inside = (0..1024).contains(&wx) && (0..1024).contains(&wy);
                    let (ix, iy, outside) = if inside {
                        (wx, wy, false)
                    } else if !repeat_off {
                        (
                            ((wx % 1024) + 1024) % 1024,
                            ((wy % 1024) + 1024) % 1024,
                            false,
                        )
                    } else {
                        (wx, wy, true)
                    };
                    if outside {
                        if fill_char0 {
                            let (col, pr, lid) =
                                self.sample_mode7_with_layer(0, (ix & 7) as u8, (iy & 7) as u8);
                            if col != 0 {
                                bg_results.push((col, pr, lid));
                            }
                        }
                    } else {
                        let tile_x = (ix >> 3) & 0x7F;
                        let tile_y = (iy >> 3) & 0x7F;
                        let px = (ix & 7) as u8;
                        let py = (iy & 7) as u8;
                        let map_base = 0x2000usize;
                        let map_index = map_base + (tile_y as usize) * 128 + (tile_x as usize);
                        if map_index < self.vram.len() {
                            let tile_id = self.vram[map_index] as u16;
                            let (col, pr, lid) = self.sample_mode7_with_layer(tile_id, px, py);
                            if col != 0 {
                                bg_results.push((col, pr, lid));
                            }
                        }
                    }
                }
            }
            _ => return (0, 0),
        }

        // プライオリティ順にソート（高い順）
        bg_results.sort_by(|a, b| {
            b.1.cmp(&a.1).then(b.2.cmp(&a.2)) // プライオリティ、BG番号の順
        });

        // 最も高いプライオリティのBGを返す
        bg_results
            .first()
            .map(|(color, priority, _)| (*color, *priority))
            .unwrap_or((0, 0))
    }

    fn render_bg_mode0_with_priority(&self, x: u16, y: u16, bg_num: u8) -> (u32, u8) {
        self.render_bg_mode0(x, y, bg_num)
    }

    fn render_bg_4bpp_with_priority(&self, x: u16, y: u16, bg_num: u8) -> (u32, u8) {
        self.render_bg_4bpp(x, y, bg_num)
    }

    fn render_bg_8bpp_with_priority(&self, x: u16, y: u16, bg_num: u8) -> (u32, u8) {
        self.render_bg_8bpp(x, y, bg_num)
    }

    fn render_bg_mode2_with_priority(&self, x: u16, y: u16, bg_num: u8) -> (u32, u8) {
        self.render_bg_mode2(x, y, bg_num)
    }

    fn render_bg_mode5_with_priority(&self, x: u16, y: u16, bg_num: u8) -> (u32, u8) {
        self.render_bg_mode5(x, y, bg_num, true)
    }

    fn render_bg_mode6_with_priority(&self, x: u16, y: u16, bg_num: u8) -> (u32, u8) {
        self.render_bg_mode6(x, y, bg_num, true)
    }

    #[inline]
    fn sample_tile_2bpp(&self, tile_base: u16, tile_id: u16, px: u8, py: u8) -> u8 {
        // 2bpp tile = 8 words (16 bytes)
        let tile_addr = tile_base.wrapping_add(tile_id.wrapping_mul(8)) & 0x7FFF;
        let row_word = tile_addr.wrapping_add(py as u16) & 0x7FFF;
        let plane0_addr = (row_word as usize) * 2;
        let plane1_addr = plane0_addr + 1;
        if plane1_addr >= self.vram.len() {
            return 0;
        }
        let plane0 = self.vram[plane0_addr];
        let plane1 = self.vram[plane1_addr];
        let bit = 7 - px;
        (((plane1 >> bit) & 1) << 1) | ((plane0 >> bit) & 1)
    }

    #[inline]
    fn sample_tile_4bpp(&self, tile_base: u16, tile_id: u16, px: u8, py: u8) -> u8 {
        // 4bpp tile = 16 words (32 bytes)
        let tile_addr = (tile_base.wrapping_add(tile_id.wrapping_mul(16))) & 0x7FFF;
        let row01_word = (tile_addr.wrapping_add(py as u16)) & 0x7FFF;
        let row23_word = (tile_addr.wrapping_add(8).wrapping_add(py as u16)) & 0x7FFF;
        let plane0_addr = (row01_word as usize) * 2;
        let plane1_addr = plane0_addr + 1;
        let plane2_addr = (row23_word as usize) * 2;
        let plane3_addr = plane2_addr + 1;
        if plane3_addr >= self.vram.len() {
            return 0;
        }
        let plane0 = self.vram[plane0_addr];
        let plane1 = self.vram[plane1_addr];
        let plane2 = self.vram[plane2_addr];
        let plane3 = self.vram[plane3_addr];
        let bit = 7 - px;
        (((plane3 >> bit) & 1) << 3)
            | (((plane2 >> bit) & 1) << 2)
            | (((plane1 >> bit) & 1) << 1)
            | ((plane0 >> bit) & 1)
    }

    fn render_bg_mode0(&self, x: u16, y: u16, bg_num: u8) -> (u32, u8) {
        // Debug: Check if tilemap base addresses are set
        static mut BG_DEBUG_COUNT: u32 = 0;
        unsafe {
            if BG_DEBUG_COUNT < 5 && x == 0 && y == 1 && crate::debug_flags::boot_verbose() {
                let tilemap_base = match bg_num {
                    0 => self.bg1_tilemap_base,
                    1 => self.bg2_tilemap_base,
                    2 => self.bg3_tilemap_base,
                    3 => self.bg4_tilemap_base,
                    _ => 0,
                };
                let tile_base = match bg_num {
                    0 => self.bg1_tile_base,
                    1 => self.bg2_tile_base,
                    2 => self.bg3_tile_base,
                    3 => self.bg4_tile_base,
                    _ => 0,
                };
                if tilemap_base != 0 || tile_base != 0 {
                    BG_DEBUG_COUNT += 1;
                    println!(
                        "🎮 BG{} RENDER[{}]: tilemap_base=0x{:04X}, tile_base=0x{:04X}",
                        bg_num, BG_DEBUG_COUNT, tilemap_base, tile_base
                    );
                }
            }
        }

        let tile_16 = self.bg_tile_16[bg_num as usize];
        let tile_px = if tile_16 { 16 } else { 8 } as u16;
        let ss = self.bg_screen_size[bg_num as usize];
        let width_tiles = if ss == 1 || ss == 3 { 64 } else { 32 } as u16;
        let height_tiles = if ss == 2 || ss == 3 { 64 } else { 32 } as u16;
        let wrap_x = width_tiles * tile_px;
        let wrap_y = height_tiles * tile_px;

        let (mosaic_x, mosaic_y) = self.apply_mosaic(x, y, bg_num);
        let (scroll_x, scroll_y) = match bg_num {
            0 => (self.bg1_hscroll, self.bg1_vscroll),
            1 => (self.bg2_hscroll, self.bg2_vscroll),
            2 => (self.bg3_hscroll, self.bg3_vscroll),
            3 => (self.bg4_hscroll, self.bg4_vscroll),
            _ => (0, 0),
        };
        let bg_x = (mosaic_x + scroll_x) % wrap_x;
        let bg_y = (mosaic_y + scroll_y) % wrap_y;

        let tile_x = bg_x / tile_px;
        let tile_y = bg_y / tile_px;

        // Debug output disabled for performance

        let tilemap_base = match bg_num {
            0 => self.bg1_tilemap_base,
            1 => self.bg2_tilemap_base,
            2 => self.bg3_tilemap_base,
            3 => self.bg4_tilemap_base,
            _ => 0,
        } as u32;
        // Tilemap base is in words
        let tilemap_base_word = tilemap_base;
        let (map_tx, map_ty) = (tile_x % 32, tile_y % 32);
        let scx = (tile_x / 32) as u32;
        let scy = (tile_y / 32) as u32;
        let width_screens = if ss == 1 || ss == 3 { 2 } else { 1 } as u32;
        let quadrant = scx + scy * width_screens;
        // Calculate word address first, then convert to byte index
        let map_entry_word_addr = tilemap_base_word
            .saturating_add(quadrant * 0x400) // 32x32 tilemap = 1024 words
            .saturating_add((map_ty as u32) * 32 + map_tx as u32)
            & 0x7FFF; // VRAM mirrors at 0x8000 words
        let map_entry_addr = (map_entry_word_addr * 2) as usize; // Convert to byte index for VRAM access

        // Debug address calculation and VRAM content
        static mut ADDR_DEBUG_COUNT: u32 = 0;
        unsafe {
            if ADDR_DEBUG_COUNT < 5
                && map_entry_addr >= 0xE000
                && crate::debug_flags::boot_verbose()
            {
                ADDR_DEBUG_COUNT += 1;
                println!("📍 VRAM ACCESS[{}]: BG{} tilemap_base=0x{:04X}, final_addr=0x{:04X}, VRAM_len=0x{:04X}",
                        ADDR_DEBUG_COUNT, bg_num, tilemap_base, map_entry_addr, self.vram.len());

                // Show first few bytes of VRAM at this address (for debugging)
                let start_addr = if self.vram.len() >= 0x10000 {
                    map_entry_addr as usize
                } else {
                    (map_entry_addr & 0x7FFF) as usize
                };

                if start_addr + 15 < self.vram.len() {
                    print!("   VRAM[0x{:04X}]: ", start_addr);
                    for i in 0..16 {
                        print!("{:02X} ", self.vram[start_addr + i]);
                    }
                    println!();
                }
            }
        }

        if map_entry_addr + 1 >= self.vram.len() {
            return (0, 0);
        }
        let map_entry_lo = self.vram[map_entry_addr];
        let map_entry_hi = self.vram[map_entry_addr + 1];
        let map_entry = ((map_entry_hi as u16) << 8) | (map_entry_lo as u16);

        // Debug and validate tilemap entries
        static mut TILEMAP_FOUND_COUNT: u32 = 0;
        static mut INVALID_TILEMAP_COUNT: u32 = 0;
        unsafe {
            if map_entry != 0 {
                let tile_id_raw = map_entry & 0x03FF;
                let _palette_raw = (map_entry >> 10) & 0x07;

                // Do not replace tilemap entries; render exactly what is in VRAM.

                if TILEMAP_FOUND_COUNT < 20 && crate::debug_flags::boot_verbose() {
                    TILEMAP_FOUND_COUNT += 1;
                    println!("🗺️  TILEMAP[{}]: BG{} screen({},{}) bg({},{}) tile({},{}) map({},{}) quad={} base=0x{:04X} word_addr=0x{:04X} byte_addr=0x{:04X} entry=0x{:04X} tile_id={}",
                            TILEMAP_FOUND_COUNT, bg_num, x, y, bg_x, bg_y, tile_x, tile_y, map_tx, map_ty, quadrant, tilemap_base, map_entry_word_addr, map_entry_addr, map_entry, tile_id_raw);
                }
            } else if TILEMAP_FOUND_COUNT == 0
                && INVALID_TILEMAP_COUNT < 5
                && crate::debug_flags::boot_verbose()
            {
                INVALID_TILEMAP_COUNT += 1;
                println!("⚠️  EMPTY TILEMAP[{}]: BG{} at ({},{}) addr=0x{:04X} entry=0x{:04X} tilemap_base=0x{:04X}",
                        INVALID_TILEMAP_COUNT, bg_num, x, y, map_entry_addr, map_entry, tilemap_base);
            }
        }

        let mut tile_id = map_entry & 0x03FF;
        let palette = ((map_entry >> 10) & 0x07) as u8;
        let flip_x = (map_entry & 0x4000) != 0;
        let flip_y = (map_entry & 0x8000) != 0;
        let priority = (map_entry & 0x2000) != 0;

        let mut rel_x = (bg_x % tile_px) as u8;
        let mut rel_y = (bg_y % tile_px) as u8;
        if flip_x {
            rel_x = (tile_px as u8 - 1) - rel_x;
        }
        if flip_y {
            rel_y = (tile_px as u8 - 1) - rel_y;
        }
        if tile_16 {
            let sub_x = (rel_x / 8) as u16;
            let sub_y = (rel_y / 8) as u16;
            tile_id = tile_id
                .wrapping_add(sub_x)
                .wrapping_add(sub_y.wrapping_mul(16));
            rel_x %= 8;
            rel_y %= 8;
        }

        let tile_base = match bg_num {
            0 => self.bg1_tile_base,
            1 => self.bg2_tile_base,
            2 => self.bg3_tile_base,
            3 => self.bg4_tile_base,
            _ => 0,
        };

        // tile_base is in VRAM words (from BGxNBA registers)
        // 2bpp tile = 16 bytes = 8 words
        let tile_addr = tile_base.wrapping_add(tile_id.wrapping_mul(8)) & 0x7FFF;

        // Fix VRAM addressing: Handle high address ranges correctly
        // VRAM is 64KB but may be mirrored/banked, don't reject high addresses immediately

        // Debug problematic tile addresses
        static mut BAD_ADDR_COUNT: u32 = 0;
        unsafe {
            if crate::debug_flags::debug_suspicious_tile() && (tile_base == 0 || tile_id > 1023) {
                BAD_ADDR_COUNT += 1;
                if BAD_ADDR_COUNT <= 3 && !crate::debug_flags::quiet() {
                    println!("⚠️ SUSPICIOUS TILE[{}]: BG{} tile_base=0x{:04X}, tile_id={}, addr=0x{:04X}",
                            BAD_ADDR_COUNT, bg_num, tile_base, tile_id, tile_addr);
                }
            }
        }
        // tile_addr is in words, convert to byte index by multiplying by 2
        let plane0_addr = ((tile_addr + rel_y as u16) as usize) * 2;
        // 2bpp は同じ行内で plane0, plane1 が連続する 2 バイト（row*2 + {0,1}）
        let plane1_addr = plane0_addr + 1;
        if plane0_addr >= self.vram.len() || plane1_addr >= self.vram.len() {
            return (0, 0);
        }
        let plane0 = self.vram[plane0_addr];
        let plane1 = self.vram[plane1_addr];

        // Debug output disabled for performance

        // Debug VRAM tile data (quiet by default)
        if crate::debug_flags::boot_verbose() {
            static mut VRAM_DEBUG_COUNT: u32 = 0;
            unsafe {
                if VRAM_DEBUG_COUNT < 5 && (plane0 != 0 || plane1 != 0) {
                    VRAM_DEBUG_COUNT += 1;
                    println!("📊 VRAM TILE[{}]: tile_id={}, addr=0x{:04X}, plane0=0x{:02X}, plane1=0x{:02X}", 
                            VRAM_DEBUG_COUNT, tile_id, tile_addr, plane0, plane1);
                }
            }
        }
        let bit = 7 - rel_x;
        let color_index = ((plane1 >> bit) & 1) << 1 | ((plane0 >> bit) & 1);

        // Debug first few non-zero pixels found
        static mut PIXEL_FOUND_COUNT: u32 = 0;
        if color_index != 0 {
            let palette_idx = self.get_bg_palette_index(palette, color_index, 2);
            let final_color = self.cgram_to_rgb(palette_idx);

            unsafe {
                if crate::debug_flags::debug_pixel_found()
                    && PIXEL_FOUND_COUNT < 5
                    && !crate::debug_flags::quiet()
                {
                    PIXEL_FOUND_COUNT += 1;
                    println!("🎯 PIXEL FOUND[{}]: BG{} at ({},{}) color_index={}, palette={}, palette_index={}",
                            PIXEL_FOUND_COUNT, bg_num, x, y, color_index, palette, palette_idx);
                    println!("   Final color: 0x{:08X}", final_color);
                }
            }
        }

        if color_index == 0 {
            return (0, 0);
        }
        // Mode 0 uses a dedicated CGRAM range per BG:
        // - BG1: palettes 0..7   (CGRAM 0..31)
        // - BG2: palettes 8..15  (CGRAM 32..63)
        // - BG3: palettes 16..23 (CGRAM 64..95)
        // - BG4: palettes 24..31 (CGRAM 96..127)
        //
        // For other modes, BG palettes share the lower CGRAM region (0..127).
        let palette_index = if self.bg_mode == 0 {
            let bg_off = (bg_num as u16).saturating_mul(32);
            let idx = bg_off + (palette as u16) * 4 + (color_index as u16);
            idx.min(127) as u8
        } else {
            self.get_bg_palette_index(palette, color_index, 2)
        };
        let color = self.cgram_to_rgb(palette_index);

        // Use palette result strictly as-is (no heuristic overrides)

        let priority_value = if priority { 1 } else { 0 };
        (color, priority_value)
    }

    #[allow(dead_code)]
    fn render_bg_mode1(&self, x: u16, y: u16, bg_num: u8) -> (u32, u8) {
        // Mode 1: BG1/BG2は4bpp、BG3は2bpp
        if bg_num <= 1 {
            // 4bpp描画
            self.render_bg_4bpp(x, y, bg_num)
        } else {
            // 2bpp描画
            self.render_bg_mode0(x, y, bg_num)
        }
    }

    fn render_bg_4bpp(&self, x: u16, y: u16, bg_num: u8) -> (u32, u8) {
        if crate::debug_flags::boot_verbose() {
            static mut DEBUG_FUNCTION_COUNT: u32 = 0;
            unsafe {
                DEBUG_FUNCTION_COUNT += 1;
                if DEBUG_FUNCTION_COUNT <= 5 && x < 32 && y < 32 {
                    println!(
                        "DBG: render_bg_4bpp BG{} at ({},{}), map_base=0x{:04X}",
                        bg_num,
                        x,
                        y,
                        match bg_num {
                            0 => self.bg1_tilemap_base,
                            1 => self.bg2_tilemap_base,
                            _ => 0,
                        }
                    );
                }
            }
        }

        let tile_16 = self.bg_tile_16[bg_num as usize];
        let tile_px = if tile_16 { 16 } else { 8 } as u16;
        let ss = self.bg_screen_size[bg_num as usize];
        let width_tiles = if ss == 1 || ss == 3 { 64 } else { 32 } as u16;
        let height_tiles = if ss == 2 || ss == 3 { 64 } else { 32 } as u16;
        let wrap_x = width_tiles * tile_px;
        let wrap_y = height_tiles * tile_px;

        let (mosaic_x, mosaic_y) = self.apply_mosaic(x, y, bg_num);
        let (scroll_x, scroll_y) = match bg_num {
            0 => (self.bg1_hscroll, self.bg1_vscroll),
            1 => (self.bg2_hscroll, self.bg2_vscroll),
            2 => (self.bg3_hscroll, self.bg3_vscroll),
            3 => (self.bg4_hscroll, self.bg4_vscroll),
            _ => (0, 0),
        };
        let bg_x = (mosaic_x + scroll_x) % wrap_x;
        let bg_y = (mosaic_y + scroll_y) % wrap_y;

        let tile_x = bg_x / tile_px;
        let tile_y = bg_y / tile_px;

        // Debug output disabled for performance

        let tilemap_base = match bg_num {
            0 => self.bg1_tilemap_base,
            1 => self.bg2_tilemap_base,
            2 => self.bg3_tilemap_base,
            3 => self.bg4_tilemap_base,
            _ => 0,
        } as u32;
        // Tilemap base is in words
        let tilemap_base_word = tilemap_base;
        let (map_tx, map_ty) = (tile_x % 32, tile_y % 32);
        let scx = (tile_x / 32) as u32;
        let scy = (tile_y / 32) as u32;
        let width_screens = if ss == 1 || ss == 3 { 2 } else { 1 } as u32;
        let quadrant = scx + scy * width_screens;
        // Calculate word address first, then convert to byte index
        let map_entry_word_addr = tilemap_base_word
            .saturating_add(quadrant * 0x400) // 32x32 tilemap = 1024 words
            .saturating_add((map_ty as u32) * 32 + map_tx as u32);

        // Apply VRAM mirroring to word address: VRAM is 32K words (0x0000-0x7FFF)
        // Addresses 0x8000-0xFFFF mirror to 0x0000-0x7FFF
        let map_entry_word_addr = map_entry_word_addr & 0x7FFF;
        let map_entry_addr = map_entry_word_addr * 2; // Convert to byte index for VRAM access

        // デバッグ: タイルマップアドレス計算を確認（オプトイン）
        if std::env::var_os("DEBUG_TILEMAP_ADDR").is_some() && !crate::debug_flags::quiet() {
            static mut DEBUG_TILEMAP_COUNT: u32 = 0;
            unsafe {
                DEBUG_TILEMAP_COUNT += 1;
                if DEBUG_TILEMAP_COUNT <= 3 && x < 5 && y < 5 {
                    println!(
                        "  BG{} tilemap_base=0x{:04X}, map_entry_addr=0x{:04X}, VRAM_len=0x{:04X}",
                        bg_num,
                        tilemap_base,
                        map_entry_addr,
                        self.vram.len()
                    );
                }
            }
        }

        if (map_entry_addr + 1) as usize >= self.vram.len() {
            if std::env::var_os("DEBUG_TILEMAP_ADDR").is_some() && !crate::debug_flags::quiet() {
                static mut DEBUG_MAP_COUNT: u32 = 0;
                unsafe {
                    DEBUG_MAP_COUNT += 1;
                    if DEBUG_MAP_COUNT <= 3 {
                        println!(
                            "  BG{} EARLY RETURN: map_entry_addr=0x{:04X} out of VRAM bounds (len=0x{:04X})",
                            bg_num,
                            map_entry_addr,
                            self.vram.len()
                        );
                    }
                }
            }
            return (0, 0);
        }
        let map_entry_lo = self.vram[map_entry_addr as usize];
        let map_entry_hi = self.vram[(map_entry_addr + 1) as usize];
        let map_entry = ((map_entry_hi as u16) << 8) | (map_entry_lo as u16);

        // Optional tilemap sampling (disabled by default)
        if crate::debug_flags::boot_verbose() && x == 0 && y == 0 {
            println!(
                "Tilemap entry @0x{:04X} = 0x{:04X}",
                map_entry_addr, map_entry
            );
        }

        let mut tile_id = map_entry & 0x03FF;
        let palette = ((map_entry >> 10) & 0x07) as u8;
        let flip_x = (map_entry & 0x4000) != 0;
        let flip_y = (map_entry & 0x8000) != 0;
        let priority = (map_entry & 0x2000) != 0;

        let mut rel_x = (bg_x % tile_px) as u8;
        let mut rel_y = (bg_y % tile_px) as u8;
        if flip_x {
            rel_x = (tile_px as u8 - 1) - rel_x;
        }
        if flip_y {
            rel_y = (tile_px as u8 - 1) - rel_y;
        }
        if tile_16 {
            let sub_x = (rel_x / 8) as u16;
            let sub_y = (rel_y / 8) as u16;
            tile_id = tile_id
                .wrapping_add(sub_x)
                .wrapping_add(sub_y.wrapping_mul(16));
            rel_x %= 8;
            rel_y %= 8;
        }

        let tile_base = match bg_num {
            0 => self.bg1_tile_base,
            1 => self.bg2_tile_base,
            2 => self.bg3_tile_base,
            3 => self.bg4_tile_base,
            _ => 0,
        };
        // tile_base is in VRAM words (from BGxNBA registers)
        // 4bpp tile = 32 bytes = 16 words
        let tile_addr = (tile_base.wrapping_add(tile_id.wrapping_mul(16))) & 0x7FFF; // Mask to VRAM range

        if crate::debug_flags::boot_verbose() {
            static mut DEBUG_TILE_ADDR_COUNT: u32 = 0;
            unsafe {
                DEBUG_TILE_ADDR_COUNT += 1;
                if DEBUG_TILE_ADDR_COUNT <= 3 {
                    println!(
                        "DBG: BG{} tile_addr=0x{:04X} (base=0x{:04X}, id=0x{:03X})",
                        bg_num, tile_addr, tile_base, tile_id
                    );
                }
            }
        }

        // 4bpp tile layout in VRAM (word-addressed):
        // - words 0..7:  plane0 (low byte) + plane1 (high byte) for rows 0..7
        // - words 8..15: plane2 (low byte) + plane3 (high byte) for rows 0..7
        let row01_word = (tile_addr.wrapping_add(rel_y as u16)) & 0x7FFF;
        let row23_word = (tile_addr.wrapping_add(8).wrapping_add(rel_y as u16)) & 0x7FFF;
        let plane0_addr = (row01_word as usize) * 2;
        let plane1_addr = plane0_addr + 1;
        let plane2_addr = (row23_word as usize) * 2;
        let plane3_addr = plane2_addr + 1;
        if plane3_addr >= self.vram.len() {
            return (0, 0);
        }

        // Dragon Quest III fix: Use static pattern for stability
        let base_tile_addr = tile_addr as usize;
        let _vram_data_sample = if base_tile_addr < self.vram.len() {
            self.vram[base_tile_addr]
        } else {
            0
        };
        // Dragon Quest III: Disable fallback to see actual VRAM data
        let needs_fallback = false;

        let get_vram_with_fallback = |addr: usize, plane_offset: usize| -> u8 {
            if addr < self.vram.len() {
                let vram_data = self.vram[addr];

                // If VRAM data is all zeros, use fallback for now
                if vram_data == 0 && needs_fallback {
                    match plane_offset {
                        0..=7 => 0xFF,   // Plane 0: solid
                        8..=15 => 0xAA,  // Plane 1: alternating
                        16..=23 => 0x55, // Plane 2: alternating opposite
                        24..=31 => 0x33, // Plane 3: sparse
                        _ => 0,
                    }
                } else {
                    vram_data
                }
            } else {
                0 // Out of bounds
            }
        };

        let plane0 = get_vram_with_fallback(plane0_addr, rel_y as usize);
        let plane1 = get_vram_with_fallback(plane1_addr, rel_y as usize);
        let plane2 = get_vram_with_fallback(plane2_addr, rel_y as usize);
        let plane3 = get_vram_with_fallback(plane3_addr, rel_y as usize);

        // Optional tile debug (disabled by default)
        if crate::debug_flags::boot_verbose() {
            static mut TILE_DEBUG_COUNT: u32 = 0;
            unsafe {
                TILE_DEBUG_COUNT += 1;
                if TILE_DEBUG_COUNT <= 10 && x < 4 && y < 4 {
                    println!(
                        "Tile({},{}) tile=0x{:03X} planes=[{:02X},{:02X},{:02X},{:02X}]",
                        x, y, tile_id, plane0, plane1, plane2, plane3
                    );
                }
            }
        }
        let bit = 7 - rel_x;
        let color_index = ((plane3 >> bit) & 1) << 3
            | ((plane2 >> bit) & 1) << 2
            | ((plane1 >> bit) & 1) << 1
            | ((plane0 >> bit) & 1);

        if color_index == 0 {
            return (0, 0);
        }
        let palette_index = self.get_bg_palette_index(palette, color_index, 4);

        if crate::debug_flags::boot_verbose() {
            static mut CGRAM_DEBUG_COUNT: u32 = 0;
            unsafe {
                CGRAM_DEBUG_COUNT += 1;
                if CGRAM_DEBUG_COUNT <= 10 && (palette_index as usize) < 32 {
                    println!("CGRAM[{}] sample", palette_index);
                }
            }
        }

        // Use CGRAM color as-is (no special fallbacks)
        let color = self.cgram_to_rgb(palette_index);

        let priority_value = if priority { 1 } else { 0 };
        (color, priority_value)
    }

    #[allow(dead_code)]
    fn render_bg_mode4(&self, x: u16, y: u16, bg_num: u8) -> (u32, u8) {
        // Mode 4: BG1は8bpp、BG2は2bpp
        if bg_num == 0 {
            // BG1: 8bpp描画（256色）
            self.render_bg_8bpp(x, y, bg_num)
        } else {
            // BG2: 2bpp描画
            self.render_bg_mode0(x, y, bg_num)
        }
    }

    fn render_bg_8bpp(&self, x: u16, y: u16, bg_num: u8) -> (u32, u8) {
        let tile_16 = self.bg_tile_16[bg_num as usize];
        let tile_px = if tile_16 { 16 } else { 8 } as u16;
        let ss = self.bg_screen_size[bg_num as usize];
        let width_tiles = if ss == 1 || ss == 3 { 64 } else { 32 } as u16;
        let height_tiles = if ss == 2 || ss == 3 { 64 } else { 32 } as u16;
        let wrap_x = width_tiles * tile_px;
        let wrap_y = height_tiles * tile_px;

        let (mosaic_x, mosaic_y) = self.apply_mosaic(x, y, bg_num);
        let (scroll_x, scroll_y) = match bg_num {
            0 => (self.bg1_hscroll, self.bg1_vscroll),
            1 => (self.bg2_hscroll, self.bg2_vscroll),
            2 => (self.bg3_hscroll, self.bg3_vscroll),
            3 => (self.bg4_hscroll, self.bg4_vscroll),
            _ => (0, 0),
        };
        let bg_x = (mosaic_x.wrapping_add(scroll_x)) % wrap_x;
        let bg_y = (mosaic_y.wrapping_add(scroll_y)) % wrap_y;

        let tile_x = bg_x / tile_px;
        let tile_y = bg_y / tile_px;

        let tilemap_base_word = match bg_num {
            0 => self.bg1_tilemap_base,
            1 => self.bg2_tilemap_base,
            2 => self.bg3_tilemap_base,
            3 => self.bg4_tilemap_base,
            _ => 0,
        } as u32;
        let (map_tx, map_ty) = (tile_x % 32, tile_y % 32);
        let scx = (tile_x / 32) as u32;
        let scy = (tile_y / 32) as u32;
        let width_screens = if ss == 1 || ss == 3 { 2 } else { 1 } as u32;
        let quadrant = scx + scy * width_screens;
        let map_entry_word_addr = tilemap_base_word
            .saturating_add(quadrant * 0x400)
            .saturating_add((map_ty as u32) * 32 + map_tx as u32)
            & 0x7FFF;
        let map_entry_addr = map_entry_word_addr * 2;
        if (map_entry_addr as usize + 1) >= self.vram.len() {
            return (0, 0);
        }

        let map_entry_lo = self.vram[map_entry_addr as usize];
        let map_entry_hi = self.vram[(map_entry_addr as usize) + 1];
        let map_entry = ((map_entry_hi as u16) << 8) | (map_entry_lo as u16);

        let mut tile_id = map_entry & 0x03FF;
        let palette = ((map_entry >> 10) & 0x07) as u8;
        let flip_x = (map_entry & 0x4000) != 0;
        let flip_y = (map_entry & 0x8000) != 0;
        let priority = (map_entry & 0x2000) != 0;

        let mut rel_x = (bg_x % tile_px) as u8;
        let mut rel_y = (bg_y % tile_px) as u8;
        if flip_x {
            rel_x = (tile_px as u8 - 1) - rel_x;
        }
        if flip_y {
            rel_y = (tile_px as u8 - 1) - rel_y;
        }
        if tile_16 {
            let sub_x = (rel_x / 8) as u16;
            let sub_y = (rel_y / 8) as u16;
            tile_id = tile_id
                .wrapping_add(sub_x)
                .wrapping_add(sub_y.wrapping_mul(16));
            rel_x %= 8;
            rel_y %= 8;
        }

        let tile_base = match bg_num {
            0 => self.bg1_tile_base,
            1 => self.bg2_tile_base,
            2 => self.bg3_tile_base,
            3 => self.bg4_tile_base,
            _ => 0,
        };
        let tile_addr = tile_base.wrapping_add(tile_id.wrapping_mul(32)) & 0x7FFF;

        let row01_word = (tile_addr.wrapping_add(rel_y as u16)) & 0x7FFF;
        let row23_word = (tile_addr.wrapping_add(8).wrapping_add(rel_y as u16)) & 0x7FFF;
        let row45_word = (tile_addr.wrapping_add(16).wrapping_add(rel_y as u16)) & 0x7FFF;
        let row67_word = (tile_addr.wrapping_add(24).wrapping_add(rel_y as u16)) & 0x7FFF;

        let plane0_addr = (row01_word as usize) * 2;
        let plane1_addr = plane0_addr + 1;
        let plane2_addr = (row23_word as usize) * 2;
        let plane3_addr = plane2_addr + 1;
        let plane4_addr = (row45_word as usize) * 2;
        let plane5_addr = plane4_addr + 1;
        let plane6_addr = (row67_word as usize) * 2;
        let plane7_addr = plane6_addr + 1;
        if plane7_addr >= self.vram.len() {
            return (0, 0);
        }

        let p0 = self.vram[plane0_addr];
        let p1 = self.vram[plane1_addr];
        let p2 = self.vram[plane2_addr];
        let p3 = self.vram[plane3_addr];
        let p4 = self.vram[plane4_addr];
        let p5 = self.vram[plane5_addr];
        let p6 = self.vram[plane6_addr];
        let p7 = self.vram[plane7_addr];

        let bit = 7 - rel_x;
        let mut color_index = 0u8;
        color_index |= ((p0 >> bit) & 1) << 0;
        color_index |= ((p1 >> bit) & 1) << 1;
        color_index |= ((p2 >> bit) & 1) << 2;
        color_index |= ((p3 >> bit) & 1) << 3;
        color_index |= ((p4 >> bit) & 1) << 4;
        color_index |= ((p5 >> bit) & 1) << 5;
        color_index |= ((p6 >> bit) & 1) << 6;
        color_index |= ((p7 >> bit) & 1) << 7;

        if color_index == 0 {
            return (0, 0);
        }

        // Direct color mode (CGWSEL bit0) for 256-color BGs (Modes 3/4/7, BG1 only).
        let use_direct_color =
            bg_num == 0 && (self.cgwsel & 0x01) != 0 && matches!(self.bg_mode, 3 | 4 | 7);
        let color = if use_direct_color {
            self.direct_color_to_rgb(palette, color_index)
        } else {
            let palette_index = self.get_bg_palette_index(0, color_index, 8);
            self.cgram_to_rgb(palette_index)
        };
        let priority_value = if priority { 1 } else { 0 };
        (color, priority_value)
    }

    #[inline]
    fn direct_color_to_rgb(&self, palette: u8, pixel: u8) -> u32 {
        // Direct Color (MMIO $2130 bit0):
        // - Pixel value is interpreted as BBGGGRRR (8bpp character data).
        // - Tilemap palette bits ppp are interpreted as bgr (one extra bit per component).
        // Final RGB555: 0bbbbbgggggrrrrr, where LSB of each component is 0 (RGB443).
        let r5 = (((pixel & 0x07) as u32) << 2) | (((palette & 0x01) as u32) << 1);
        let g5 = ((((pixel >> 3) & 0x07) as u32) << 2) | ((((palette >> 1) & 0x01) as u32) << 1);
        let b5 = ((((pixel >> 6) & 0x03) as u32) << 3) | ((((palette >> 2) & 0x01) as u32) << 2);

        let r = (r5 << 3) | (r5 >> 2);
        let g = (g5 << 3) | (g5 >> 2);
        let b = (b5 << 3) | (b5 >> 2);
        0xFF000000 | (r << 16) | (g << 8) | b
    }

    fn render_mode7_with_layer(&mut self, x: u16, y: u16) -> (u32, u8, u8) {
        // Mode 7: affine transform into 1024x1024 world; tiles: 8x8 8bpp, map: 128x128 bytes.
        // Helper: sample for a desired layer (0:BG1, 1:BG2 when EXTBG). Applies mosaic per layer.
        let sample_for_layer = |desired_layer: u8| -> (u32, u8, u8, bool, bool, bool, bool) {
            // Screen mosaic per layer
            let (mx, my) = self.apply_mosaic(x, y, desired_layer);
            // Apply flips around 255
            let sx = if (self.m7sel & 0x01) != 0 {
                255 - (mx as i32)
            } else {
                mx as i32
            };
            let sy = if (self.m7sel & 0x02) != 0 {
                255 - (my as i32)
            } else {
                my as i32
            };

            let (wx, wy) = self.mode7_world_xy_int(sx, sy);
            let repeat_off = (self.m7sel & 0x80) != 0; // R
            let fill_char0 = (self.m7sel & 0x40) != 0; // F (only when R=1)
            let inside = (0..1024).contains(&wx) && (0..1024).contains(&wy);
            let (ix, iy, outside, wrapped) = if inside {
                (wx, wy, false, false)
            } else if !repeat_off {
                // Wrap to 0..1023 using Euclidean modulo
                (
                    (((wx % 1024) + 1024) % 1024),
                    (((wy % 1024) + 1024) % 1024),
                    false,
                    true,
                )
            } else {
                (wx, wy, true, false)
            };

            if outside {
                if !fill_char0 {
                    return (0, 0, desired_layer, false, true, false, false);
                }
                let px = (((ix % 8) + 8) % 8) as u8;
                let py = (((iy % 8) + 8) % 8) as u8;
                if self.extbg {
                    let (c, pr, lid) = self.sample_mode7_with_layer(0, px, py);
                    if lid == desired_layer {
                        return (c, pr, lid, false, true, true, false);
                    } else {
                        return (0, 0, desired_layer, false, true, true, false);
                    }
                } else {
                    let (c, pr) = self.sample_mode7_color_only(0, px, py);
                    return (c, pr, 0, false, true, true, false);
                }
            }

            // In-bounds or wrapped sampling
            let tile_x = (ix >> 3) & 0x7F; // 0..127
            let tile_y = (iy >> 3) & 0x7F; // 0..127
            let px = (((ix % 8) + 8) % 8) as u8;
            let py = (((iy % 8) + 8) % 8) as u8;
            let map_base = 0x2000usize;
            let map_index = map_base + (tile_y as usize) * 128 + (tile_x as usize);
            if map_index >= self.vram.len() {
                return (0, 0, desired_layer, wrapped, false, false, false);
            }
            let tile_id = self.vram[map_index] as u16;

            let edge = ix == 0 || ix == 1023 || iy == 0 || iy == 1023;
            if self.extbg {
                let (c, pr, lid) = self.sample_mode7_with_layer(tile_id, px, py);
                if lid == desired_layer {
                    (c, pr, lid, wrapped, false, false, edge)
                } else {
                    (0, 0, desired_layer, wrapped, false, false, edge)
                }
            } else {
                let (c, pr) = self.sample_mode7_color_only(tile_id, px, py);
                (c, pr, 0, wrapped, false, false, edge)
            }
        };

        if self.extbg {
            let (c2, p2, lid2, wrap2, clip2, fill2, edge2) = sample_for_layer(1);
            let (c1, p1, lid1, wrap1, clip1, fill1, edge1) = sample_for_layer(0);
            // Metrics
            if crate::debug_flags::render_metrics() {
                if wrap1 || wrap2 {
                    self.dbg_m7_wrap = self.dbg_m7_wrap.saturating_add(1);
                }
                if clip1 || clip2 {
                    self.dbg_m7_clip = self.dbg_m7_clip.saturating_add(1);
                }
                if fill1 || fill2 {
                    self.dbg_m7_fill = self.dbg_m7_fill.saturating_add(1);
                }
                if c1 != 0 {
                    self.dbg_m7_bg1 = self.dbg_m7_bg1.saturating_add(1);
                }
                if c2 != 0 {
                    self.dbg_m7_bg2 = self.dbg_m7_bg2.saturating_add(1);
                }
                if edge1 || edge2 {
                    self.dbg_m7_edge = self.dbg_m7_edge.saturating_add(1);
                }
            }
            // Prefer BG1 over BG2 when both present; actual sort happens in z-rank stage.
            if c1 != 0 {
                return (c1, p1, lid1);
            }
            if c2 != 0 {
                return (c2, p2, lid2);
            }
            (0, 0, 0)
        } else {
            let (c, p, lid, wrapped, clipped, filled, edge) = sample_for_layer(0);
            if crate::debug_flags::render_metrics() {
                if wrapped {
                    self.dbg_m7_wrap = self.dbg_m7_wrap.saturating_add(1);
                }
                if clipped {
                    self.dbg_m7_clip = self.dbg_m7_clip.saturating_add(1);
                }
                if filled {
                    self.dbg_m7_fill = self.dbg_m7_fill.saturating_add(1);
                }
                if c != 0 {
                    self.dbg_m7_bg1 = self.dbg_m7_bg1.saturating_add(1);
                }
                if edge {
                    self.dbg_m7_edge = self.dbg_m7_edge.saturating_add(1);
                }
            }
            (c, p, lid)
        }
    }

    // Color only (legacy callers). Returns (ARGB, priority)
    // SNES Mode 7 tiles are 8x8, 8bpp, linear (64 bytes per tile).
    #[inline]
    fn sample_mode7_color_only(&self, tile_id: u16, px: u8, py: u8) -> (u32, u8) {
        let chr_base = 0x0000usize;
        let tile_addr = chr_base + (tile_id as usize) * 64;
        let row_off = (py as usize) * 8;
        let addr = tile_addr + row_off + (px as usize);
        if addr >= self.vram.len() {
            return (0, 0);
        }
        let color_index = self.vram[addr];
        if color_index == 0 {
            return (0, 0);
        }
        let palette_index = self.get_bg_palette_index(0, color_index, 8);
        let color = self.cgram_to_rgb(palette_index);
        (color, 1)
    }

    // Full sample with layer id discrimination for EXTBG.
    // Returns (ARGB, priority, layer_id: 0=BG1, 1=BG2 when EXTBG and color>=128)
    #[inline]
    fn sample_mode7_with_layer(&self, tile_id: u16, px: u8, py: u8) -> (u32, u8, u8) {
        let chr_base = 0x0000usize;
        let tile_addr = chr_base + (tile_id as usize) * 64;
        let row_off = (py as usize) * 8;
        let addr = tile_addr + row_off + (px as usize);
        if addr >= self.vram.len() {
            return (0, 0, 0);
        }
        let color_index = self.vram[addr];
        if color_index == 0 {
            return (0, 0, 0);
        }
        let layer_id = if self.extbg && (color_index & 0x80) != 0 {
            1
        } else {
            0
        };
        let palette_index = self.get_bg_palette_index(0, color_index, 8);
        let color = self.cgram_to_rgb(palette_index);
        (color, 1, layer_id)
    }

    fn render_bg_mode2(&self, x: u16, y: u16, bg_num: u8) -> (u32, u8) {
        // Mode 2: BG1/BG2は4bpp + オフセットパータイル（簡易実装）
        if bg_num > 1 {
            return (0, 0);
        }

        // オフセットテーブルはBG3のタイルマップを想定（簡易）
        let tilemap_base = self.bg3_tilemap_base;

        // 画面座標を8x8タイル座標へ
        let tile_x = ((x + match bg_num {
            0 => self.bg1_hscroll,
            1 => self.bg2_hscroll,
            _ => 0,
        }) / 8)
            & 0x1F;
        let tile_y = ((y + match bg_num {
            0 => self.bg1_vscroll,
            1 => self.bg2_vscroll,
            _ => 0,
        }) / 8)
            & 0x1F;

        // Tilemap base is a word address, convert to byte address
        let map_addr = ((tilemap_base as u32) * 2)
            .wrapping_add(((tile_y as u32) * 32 + tile_x as u32) * 2)
            & 0xFFFF;

        let (off_x, off_y) = if (map_addr as usize + 1) < self.vram.len() {
            let lo = self.vram[map_addr as usize];
            let hi = self.vram[map_addr as usize + 1];
            // 簡易: 符号付き8bitのX/Yオフセット
            (lo as i8 as i16, hi as i8 as i16)
        } else {
            (0, 0)
        };

        // 画面座標にオフセットを反映（ラップ考慮）
        let sx = x.wrapping_add(off_x as u16);
        let sy = y.wrapping_add(off_y as u16);

        // 既存の4bpp描画を利用（内部でスクロールを加味）
        self.render_bg_4bpp(sx, sy, bg_num)
    }

    fn render_bg_mode5(&self, x: u16, y: u16, bg_num: u8, is_main: bool) -> (u32, u8) {
        // Mode 5 (hi-res): BG tiles are effectively 16px wide by pairing tiles horizontally.
        // Background layers are de-interleaved between main/sub screens (even/odd columns).
        //
        // We keep a 256-wide framebuffer and treat the main screen as the even columns and
        // the sub screen as the odd columns. So BG sampling uses a doubled X coordinate with
        // a phase offset based on which screen we are rendering.
        if bg_num > 2 {
            return (0, 0);
        }

        let tile_base = match bg_num {
            0 => self.bg1_tile_base,
            1 => self.bg2_tile_base,
            2 => self.bg3_tile_base,
            _ => 0,
        };
        let tilemap_base = match bg_num {
            0 => self.bg1_tilemap_base,
            1 => self.bg2_tilemap_base,
            2 => self.bg3_tilemap_base,
            _ => 0,
        } as u32;
        let ss = self.bg_screen_size[bg_num as usize];
        let width_tiles = if ss == 1 || ss == 3 { 64 } else { 32 } as u16;
        let height_tiles = if ss == 2 || ss == 3 { 64 } else { 32 } as u16;

        let tile_w: u16 = 16;
        let tile_h: u16 = if self.bg_tile_16[bg_num as usize] {
            16
        } else {
            8
        };
        let wrap_x = width_tiles * tile_w;
        let wrap_y = height_tiles * tile_h;

        let phase: u16 = if is_main { 0 } else { 1 };
        let x_hires = x.wrapping_mul(2).wrapping_add(phase);

        let (scroll_x, scroll_y) = match bg_num {
            0 => (self.bg1_hscroll, self.bg1_vscroll),
            1 => (self.bg2_hscroll, self.bg2_vscroll),
            2 => (self.bg3_hscroll, self.bg3_vscroll),
            _ => (0, 0),
        };
        let bg_x = (x_hires.wrapping_add(scroll_x)) % wrap_x;
        let bg_y = (y.wrapping_add(scroll_y)) % wrap_y;

        let tile_x = bg_x / tile_w;
        let tile_y = bg_y / tile_h;

        let (map_tx, map_ty) = (tile_x % 32, tile_y % 32);
        let scx = (tile_x / 32) as u32;
        let scy = (tile_y / 32) as u32;
        let width_screens = if ss == 1 || ss == 3 { 2 } else { 1 } as u32;
        let quadrant = scx + scy * width_screens;
        let map_entry_word_addr = tilemap_base
            .saturating_add(quadrant * 0x400)
            .saturating_add((map_ty as u32) * 32 + map_tx as u32)
            & 0x7FFF;
        let map_entry_addr = (map_entry_word_addr * 2) as usize;
        if map_entry_addr + 1 >= self.vram.len() {
            return (0, 0);
        }
        let lo = self.vram[map_entry_addr];
        let hi = self.vram[map_entry_addr + 1];
        let entry = ((hi as u16) << 8) | (lo as u16);

        let mut tile_id = entry & 0x03FF;
        let palette = ((entry >> 10) & 0x07) as u8;
        let flip_x = (entry & 0x4000) != 0;
        let flip_y = (entry & 0x8000) != 0;
        let priority = (entry & 0x2000) != 0;

        let mut rel_x = (bg_x % tile_w) as u8; // 0..15 (even/odd depends on screen phase)
        let mut rel_y = (bg_y % tile_h) as u8; // 0..7 or 0..15
        if flip_x {
            rel_x = (tile_w as u8 - 1) - rel_x;
        }
        if flip_y {
            rel_y = (tile_h as u8 - 1) - rel_y;
        }

        // Select the paired tile horizontally, and optionally vertically (when tile_h=16).
        let sub_x = (rel_x / 8) as u16; // 0 or 1
        let sub_y = if tile_h == 16 { (rel_y / 8) as u16 } else { 0 };
        tile_id = tile_id
            .wrapping_add(sub_x)
            .wrapping_add(sub_y.wrapping_mul(16));
        rel_x %= 8;
        rel_y %= 8;

        let color_index = match bg_num {
            0 | 2 => self.sample_tile_4bpp(tile_base, tile_id, rel_x, rel_y),
            1 => self.sample_tile_2bpp(tile_base, tile_id, rel_x, rel_y),
            _ => 0,
        };
        if color_index == 0 {
            return (0, 0);
        }

        let bpp = if bg_num == 1 { 2 } else { 4 };
        let palette_index = self.get_bg_palette_index(palette, color_index, bpp);
        let color = self.cgram_to_rgb(palette_index);
        let priority_value = if priority { 1 } else { 0 };
        (color, priority_value)
    }

    fn render_bg_mode6(&self, x: u16, y: u16, bg_num: u8, is_main: bool) -> (u32, u8) {
        // Mode 6: BG1は4bpp（高解像度512x448）
        if bg_num != 0 {
            return (0, 0);
        }

        // Use the Mode 5 sampling rules for BG1 (16px wide tiles + main/sub phase),
        // but only BG1 is displayed in Mode 6.
        self.render_bg_mode5(x, y, 0, is_main)
    }

    fn apply_hires_enhancement(&self, color: u32) -> u32 {
        // 高解像度モード用の色調整（鮮明度向上）
        let r = ((color >> 16) & 0xFF) as u8;
        let g = ((color >> 8) & 0xFF) as u8;
        let b = (color & 0xFF) as u8;

        // 軽微な彩度向上
        let enhanced_r = ((r as u16 * 110 / 100).min(255)) as u8;
        let enhanced_g = ((g as u16 * 110 / 100).min(255)) as u8;
        let enhanced_b = ((b as u16 * 110 / 100).min(255)) as u8;

        0xFF000000 | ((enhanced_r as u32) << 16) | ((enhanced_g as u32) << 8) | (enhanced_b as u32)
    }

    fn apply_brightness(&self, color: u32) -> u32 {
        // Forced blank overrides everything (unless FORCE_DISPLAY or FORCE_NO_BLANK)
        if (self.screen_display & 0x80) != 0
            && !self.force_display_active()
            && std::env::var_os("FORCE_NO_BLANK").is_none()
        {
            return 0xFF000000;
        }
        // Apply INIDISP brightness level (0..15). 15 = full.
        let factor = if self.force_display_active() {
            15
        } else {
            (self.brightness as u32).min(15)
        };
        if factor >= 15 {
            return (color & 0x00FFFFFF) | 0xFF000000;
        }
        let r = ((((color >> 16) & 0xFF) * factor / 15) & 0xFF) << 16;
        let g = ((((color >> 8) & 0xFF) * factor / 15) & 0xFF) << 8;
        let b = ((color & 0xFF) * factor / 15) & 0xFF;
        0xFF000000 | r | g | b
    }

    #[inline]
    fn average_rgb(a: u32, b: u32) -> u32 {
        let ar = ((a >> 16) & 0xFF) as u16;
        let ag = ((a >> 8) & 0xFF) as u16;
        let ab = (a & 0xFF) as u16;
        let br = ((b >> 16) & 0xFF) as u16;
        let bg = ((b >> 8) & 0xFF) as u16;
        let bb = (b & 0xFF) as u16;
        let r = ((ar + br) / 2) as u32;
        let g = ((ag + bg) / 2) as u32;
        let bl = ((ab + bb) / 2) as u32;
        0xFF000000 | (r << 16) | (g << 8) | bl
    }

    // スキャンライン開始時のスプライト評価
    pub fn evaluate_sprites_for_scanline(&mut self, scanline: u16) {
        self.sprites_on_line_count = 0;
        self.sprite_overflow = false;
        self.sprite_time_over = false;

        let mut sprite_time = 0u32;
        let mut tile_budget_used: u32 = 0; // ~34 tiles per scanline

        for step in 0..128 {
            let i = ((self.oam_eval_base as usize) + step) & 0x7F;
            let oam_offset = i * 4;
            if oam_offset + 3 >= self.oam.len() {
                break;
            }

            let sprite_y = self.oam[oam_offset + 1];
            if sprite_y >= 240 {
                continue;
            }

            // 高位テーブルからサイズ情報を取得
            let high_table_offset = 0x200 + (i / 4);
            if high_table_offset >= self.oam.len() {
                break;
            }

            let high_table_byte = self.oam[high_table_offset];
            let bit_shift = (i % 4) * 2;
            let high_bits = (high_table_byte >> bit_shift) & 0x03;
            let size_bit = (high_bits & 0x02) != 0;
            let size = if size_bit {
                SpriteSize::Large
            } else {
                SpriteSize::Small
            };

            let (_, sprite_height) = self.get_sprite_pixel_size(&size);

            // このスプライトが現在のスキャンラインに表示されるかチェック
            if scanline >= sprite_y as u16 && scanline < sprite_y as u16 + sprite_height as u16 {
                self.sprites_on_line_count += 1;

                // スプライト制限チェック
                if self.sprites_on_line_count > 32 {
                    self.sprite_overflow = true;
                    self.sprite_overflow_latched = true;
                    self.obj_overflow_lines = self.obj_overflow_lines.saturating_add(1);
                    break;
                }

                // タイル予算の概算消費（1ラインおよそ34タイル）
                let (sprite_w, _) = self.get_sprite_pixel_size(&size);
                let tiles_across = (sprite_w as u32).div_ceil(8); // 8px単位
                tile_budget_used = tile_budget_used.saturating_add(tiles_across);
                if tile_budget_used > 34 {
                    self.sprite_time_over = true;
                    self.sprite_time_over_latched = true;
                    self.obj_time_over_lines = self.obj_time_over_lines.saturating_add(1);
                    break;
                }

                // 処理時間シミュレーション
                sprite_time += match size {
                    SpriteSize::Small => 2,
                    SpriteSize::Large => 4,
                };

                // タイムオーバーチェック（概算）
                if sprite_time > 34 {
                    self.sprite_time_over = true;
                    self.sprite_time_over_latched = true;
                    self.obj_time_over_lines = self.obj_time_over_lines.saturating_add(1);
                    break;
                }
            }
        }
    }

    // スプライトステータス読み取り（デバッグ用）
    pub fn get_sprite_status(&self) -> u8 {
        let mut status = 0u8;
        if self.sprite_overflow {
            status |= 0x40; // Sprite overflow flag
        }
        if self.sprite_time_over {
            status |= 0x80; // Sprite time over flag
        }
        status | (self.sprites_on_line_count & 0x3F)
    }

    // OAMデータからスプライト情報を解析
    // スキャンライン用スプライトキャッシュ（パフォーマンス向上）
    #[allow(dead_code)]
    fn get_cached_sprites_for_scanline(&self, y: u16) -> Vec<SpriteData> {
        let mut sprites = Vec::new();

        // 最大128個のスプライト
        for step in 0..128 {
            let i = ((self.oam_eval_base as usize) + step) & 0x7F;
            let oam_offset = i * 4;
            if oam_offset + 3 >= self.oam.len() {
                break;
            }

            // OAMの基本データ（4バイト/スプライト）
            let x_lo = self.oam[oam_offset] as u16;
            let sprite_y = self.oam[oam_offset + 1];
            let tile_lo = self.oam[oam_offset + 2] as u16;
            let attr = self.oam[oam_offset + 3];

            // Y座標が240以上の場合は非表示
            if sprite_y >= 240 {
                continue;
            }

            // 高位テーブル（1ビット/スプライトを2つずつ）
            let high_table_offset = 0x200 + (i / 4);
            if high_table_offset >= self.oam.len() {
                break;
            }

            let high_table_byte = self.oam[high_table_offset];
            let bit_shift = (i % 4) * 2;
            let high_bits = (high_table_byte >> bit_shift) & 0x03;

            // X座標の最上位ビット
            let x = x_lo | (((high_bits & 0x01) as u16) << 8);

            // サイズビット
            let size_bit = (high_bits & 0x02) != 0;
            let size = if size_bit {
                SpriteSize::Large
            } else {
                SpriteSize::Small
            };

            // スプライトのサイズを取得
            let (_, sprite_height) = self.get_sprite_pixel_size(&size);

            // このスプライトが現在のスキャンラインに表示されるかチェック
            if y < sprite_y as u16 || y >= sprite_y as u16 + sprite_height as u16 {
                continue;
            }

            // タイル番号（9ビット）
            let tile = tile_lo | (((attr & 0x01) as u16) << 8);

            // 属性ビット
            let palette = (attr >> 1) & 0x07;
            let priority = (attr >> 4) & 0x03;
            let flip_x = (attr & 0x40) != 0;
            let flip_y = (attr & 0x80) != 0;

            sprites.push(SpriteData {
                x,
                y: sprite_y,
                tile,
                palette,
                priority,
                flip_x,
                flip_y,
                size,
            });
        }

        sprites
    }

    #[allow(dead_code)]
    fn parse_sprites(&self) -> Vec<SpriteData> {
        let mut sprites = Vec::new();

        // 最大128個のスプライト
        for i in 0..128 {
            let oam_offset = i * 4;
            if oam_offset + 3 >= self.oam.len() {
                break;
            }

            // OAMの基本データ（4バイト/スプライト）
            let x_lo = self.oam[oam_offset] as u16;
            let y = self.oam[oam_offset + 1];
            let tile_lo = self.oam[oam_offset + 2] as u16;
            let attr = self.oam[oam_offset + 3];

            // 高位テーブル（1ビット/スプライトを2つずつ）
            let high_table_offset = 0x200 + (i / 4);
            if high_table_offset >= self.oam.len() {
                break;
            }

            let high_table_byte = self.oam[high_table_offset];
            let bit_shift = (i % 4) * 2;
            let high_bits = (high_table_byte >> bit_shift) & 0x03;

            // X座標の最上位ビット
            let x = x_lo | (((high_bits & 0x01) as u16) << 8);

            // サイズビット
            let size_bit = (high_bits & 0x02) != 0;
            let size = if size_bit {
                SpriteSize::Large
            } else {
                SpriteSize::Small
            };

            // タイル番号（9ビット）
            let tile = tile_lo | (((attr & 0x01) as u16) << 8);

            // 属性ビット
            let palette = (attr >> 1) & 0x07;
            let priority = (attr >> 4) & 0x03;
            let flip_x = (attr & 0x40) != 0;
            let flip_y = (attr & 0x80) != 0;

            // Y座標が240以上の場合は非表示
            if y >= 240 {
                continue;
            }

            sprites.push(SpriteData {
                x,
                y,
                tile,
                palette,
                priority,
                flip_x,
                flip_y,
                size,
            });
        }

        sprites
    }

    // スプライトの実際のピクセルサイズを取得
    fn get_sprite_pixel_size(&self, size: &SpriteSize) -> (u8, u8) {
        match self.sprite_size {
            0 => match size {
                SpriteSize::Small => (8, 8),
                SpriteSize::Large => (16, 16),
            },
            1 => match size {
                SpriteSize::Small => (8, 8),
                SpriteSize::Large => (32, 32),
            },
            2 => match size {
                SpriteSize::Small => (8, 8),
                SpriteSize::Large => (64, 64),
            },
            3 => match size {
                SpriteSize::Small => (16, 16),
                SpriteSize::Large => (32, 32),
            },
            4 => match size {
                SpriteSize::Small => (16, 16),
                SpriteSize::Large => (64, 64),
            },
            _ => match size {
                SpriteSize::Small => (32, 32),
                SpriteSize::Large => (64, 64),
            },
        }
    }

    // スプライトタイル描画
    fn render_sprite_tile(
        &self,
        sprite: &SpriteData,
        tile_x: u8,
        tile_y: u8,
        pixel_x: u8,
        pixel_y: u8,
    ) -> u32 {
        // 8x8タイル内での座標
        let mut local_x = pixel_x;
        let mut local_y = pixel_y;

        // フリップ処理
        if sprite.flip_x {
            local_x = 7 - local_x;
        }
        if sprite.flip_y {
            local_y = 7 - local_y;
        }

        // スプライトサイズに基づいたタイル番号計算（改善版）
        let tile_num = self.calculate_sprite_tile_number(sprite, tile_x, tile_y);

        // スプライトのbpp数を決定（BGモードによる）
        let bpp = self.get_sprite_bpp();

        match bpp {
            2 => self.render_sprite_2bpp(tile_num, local_x, local_y, sprite.palette),
            4 => self.render_sprite_4bpp(tile_num, local_x, local_y, sprite.palette),
            8 => self.render_sprite_8bpp(tile_num, local_x, local_y),
            _ => 0,
        }
    }

    fn calculate_sprite_tile_number(&self, sprite: &SpriteData, tile_x: u8, tile_y: u8) -> u16 {
        // スプライトのタイルレイアウト計算
        let (sprite_width, sprite_height) = self.get_sprite_pixel_size(&sprite.size);
        let tiles_per_row = sprite_width / 8;

        // フリップを考慮したタイル座標
        let actual_tile_x = if sprite.flip_x {
            (tiles_per_row - 1) - tile_x
        } else {
            tile_x
        };
        let actual_tile_y = if sprite.flip_y {
            (sprite_height / 8 - 1) - tile_y
        } else {
            tile_y
        };

        // SNESスプライトタイル番号計算（16タイル幅で配置）
        sprite.tile + (actual_tile_y as u16) * 16 + (actual_tile_x as u16)
    }

    fn get_sprite_bpp(&self) -> u8 {
        // SNES OBJ (sprites) are always 4bpp.
        4
    }

    #[inline]
    fn sprite_tile_base_word_addr(&self, tile_num: u16) -> u16 {
        // VRAM word address (0x0000-0x7FFF) at the start of the given OBJ tile.
        //
        // - OBJ tile numbers are 9-bit (0..0x1FF). Bit8 selects "bank" 0x000-0x0FF vs 0x100-0x1FF.
        // - OBSEL ($2101) selects:
        //   - Base address for tiles 0x000-0x0FF in 8K-word (16KB) steps
        //   - Gap between tiles 0x0FF and 0x100 in 4K-word (8KB) steps
        //
        // Object tiles are 4bpp: 16 words per 8x8 tile.
        let base_word = self.sprite_name_base & 0x7FFF;
        let gap_word = self.sprite_name_select & 0x7FFF;

        let t = tile_num & 0x01FF;
        let bank = (t >> 8) & 1;
        let index = t & 0x00FF;

        let mut word = base_word.wrapping_add(index.wrapping_mul(16));
        if bank != 0 {
            // +0x1000 words = 256 tiles * 16 words/tile
            word = word.wrapping_add(0x1000).wrapping_add(gap_word);
        }
        word & 0x7FFF
    }

    fn render_sprite_2bpp(&self, tile_num: u16, pixel_x: u8, pixel_y: u8, palette: u8) -> u32 {
        // NOTE: SNES OBJ are 4bpp. This path is kept only for experiments/debugging.
        // 2bpp tile = 8 words.
        let tile_addr = (self.sprite_tile_base_word_addr(tile_num) & 0x7FFF) & !0x0007;

        // tile_addr is in words, convert to byte index
        let plane0_addr = ((tile_addr + pixel_y as u16) as usize) * 2;
        let plane1_addr = plane0_addr + 1;

        if plane0_addr >= self.vram.len() || plane1_addr >= self.vram.len() {
            return 0;
        }

        let plane0 = self.vram[plane0_addr];
        let plane1 = self.vram[plane1_addr];

        let bit = 7 - pixel_x;
        let color_index = ((plane1 >> bit) & 1) << 1 | ((plane0 >> bit) & 1);

        if color_index == 0 {
            return 0; // 透明
        }

        // スプライトパレットは128-255（CGRAM上位128バイト）
        let palette_base = 128 + (palette * 4);
        let palette_index = palette_base + color_index;

        self.cgram_to_rgb(palette_index)
    }

    fn render_sprite_4bpp(&self, tile_num: u16, pixel_x: u8, pixel_y: u8, palette: u8) -> u32 {
        // 4bpp sprite tile = 32 bytes = 16 words
        let tile_addr = self.sprite_tile_base_word_addr(tile_num);

        // 4bpp sprite tile layout matches BG 4bpp: (plane0/1) then (plane2/3), 8 words each.
        let row01_word = (tile_addr.wrapping_add(pixel_y as u16)) & 0x7FFF;
        let row23_word = (tile_addr.wrapping_add(8).wrapping_add(pixel_y as u16)) & 0x7FFF;
        let plane0_addr = (row01_word as usize) * 2;
        let plane1_addr = plane0_addr + 1;
        let plane2_addr = (row23_word as usize) * 2;
        let plane3_addr = plane2_addr + 1;

        if plane3_addr >= self.vram.len() {
            return 0;
        }

        let plane0 = self.vram[plane0_addr];
        let plane1 = self.vram[plane1_addr];
        let plane2 = self.vram[plane2_addr];
        let plane3 = self.vram[plane3_addr];

        let bit = 7 - pixel_x;
        let color_index = ((plane3 >> bit) & 1) << 3
            | ((plane2 >> bit) & 1) << 2
            | ((plane1 >> bit) & 1) << 1
            | ((plane0 >> bit) & 1);

        if color_index == 0 {
            return 0; // 透明
        }

        // スプライト4bppパレットは128-255（16色/パレット）
        let palette_base = 128 + (palette * 16);
        let palette_index = palette_base + color_index;

        self.cgram_to_rgb(palette_index)
    }

    fn render_sprite_8bpp(&self, tile_num: u16, pixel_x: u8, pixel_y: u8) -> u32 {
        // NOTE: SNES OBJ are 4bpp. This path is kept only for experiments/debugging.
        // 8bpp sprite tile = 64 bytes = 32 words
        let tile_addr = self.sprite_tile_base_word_addr(tile_num);

        // 8bpp layout is 4 plane-pairs of 8 words each:
        // - +0:  plane0/1 rows 0..7
        // - +8:  plane2/3 rows 0..7
        // - +16: plane4/5 rows 0..7
        // - +24: plane6/7 rows 0..7
        let row0 = (tile_addr.wrapping_add(pixel_y as u16)) & 0x7FFF;
        let row1 = (tile_addr.wrapping_add(8).wrapping_add(pixel_y as u16)) & 0x7FFF;
        let row2 = (tile_addr.wrapping_add(16).wrapping_add(pixel_y as u16)) & 0x7FFF;
        let row3 = (tile_addr.wrapping_add(24).wrapping_add(pixel_y as u16)) & 0x7FFF;
        let a0 = (row0 as usize) * 2;
        let a1 = (row1 as usize) * 2;
        let a2 = (row2 as usize) * 2;
        let a3 = (row3 as usize) * 2;
        if a3 + 1 >= self.vram.len() {
            return 0;
        }
        let p0 = self.vram[a0];
        let p1 = self.vram[a0 + 1];
        let p2 = self.vram[a1];
        let p3 = self.vram[a1 + 1];
        let p4 = self.vram[a2];
        let p5 = self.vram[a2 + 1];
        let p6 = self.vram[a3];
        let p7 = self.vram[a3 + 1];

        let bit = 7 - pixel_x;
        let color_index = ((p7 >> bit) & 1) << 7
            | ((p6 >> bit) & 1) << 6
            | ((p5 >> bit) & 1) << 5
            | ((p4 >> bit) & 1) << 4
            | ((p3 >> bit) & 1) << 3
            | ((p2 >> bit) & 1) << 2
            | ((p1 >> bit) & 1) << 1
            | ((p0 >> bit) & 1);

        if color_index == 0 {
            return 0; // 透明
        }

        let palette_index = self.get_sprite_palette_index(0, color_index, 8);
        self.cgram_to_rgb(palette_index)
    }

    fn cgram_to_rgb(&self, index: u8) -> u32 {
        let addr = (index as usize) * 2;
        if addr + 1 >= self.cgram.len() {
            return 0xFF000000; // Return opaque black instead of transparent
        }

        let lo = self.cgram[addr];
        let hi = self.cgram[addr + 1];
        let color = ((hi as u16) << 8) | (lo as u16);

        // Debug CGRAM data for important colors - expand to monitor more indices
        static mut CGRAM_ACCESS_COUNT: [u32; 256] = [0; 256];
        let quiet = crate::debug_flags::quiet();
        unsafe {
            CGRAM_ACCESS_COUNT[index as usize] += 1;

            // Show first few accesses to key palette indices
            if !quiet
                && crate::debug_flags::debug_cgram_read()
                && CGRAM_ACCESS_COUNT[index as usize] <= 3
                && (index <= 16 || index == 0 || color != 0)
            {
                let r = color & 0x1F;
                let g = (color >> 5) & 0x1F;
                let b = (color >> 10) & 0x1F;
                println!(
                    "🎨 CGRAM[{}]: color=0x{:04X} RGB555=({},{},{}) RGB888=0x{:02X}{:02X}{:02X}",
                    index,
                    color,
                    r,
                    g,
                    b,
                    ((r << 3) | (r >> 2)) as u8,
                    ((g << 3) | (g >> 2)) as u8,
                    ((b << 3) | (b >> 2)) as u8
                );
            }
        }

        // SNES CGRAM is 15-bit RGB555 in little-endian (bit0-4: Red, 5-9: Green, 10-14: Blue).
        let r5 = (color & 0x001F) as u32;
        let g5 = ((color >> 5) & 0x001F) as u32;
        let b5 = ((color >> 10) & 0x001F) as u32;

        // 5ビットから8ビットへ拡張
        let r = (r5 << 3) | (r5 >> 2);
        let g = (g5 << 3) | (g5 >> 2);
        let b = (b5 << 3) | (b5 >> 2);

        // Return ARGB format with full opacity
        0xFF000000 | (r << 16) | (g << 8) | b
    }

    // 透明ピクセルのチョック
    fn is_transparent_pixel(&self, color: u32) -> bool {
        color == 0
    }

    // BGパレットとスプライトパレットの区別
    fn get_bg_palette_index(&self, palette: u8, color_index: u8, bpp: u8) -> u8 {
        match bpp {
            2 => palette * 4 + color_index,  // 2bpp: 4色/パレット
            4 => palette * 16 + color_index, // 4bpp: 16色/パレット
            8 => color_index,                // 8bpp: 直接インデックス
            _ => 0,
        }
    }

    fn get_sprite_palette_index(&self, palette: u8, color_index: u8, bpp: u8) -> u8 {
        match bpp {
            2 => 128 + palette * 4 + color_index, // スプライトは128番以降
            4 => 128 + palette * 16 + color_index,
            8 => 128 + color_index,
            _ => 128,
        }
    }

    // ウィンドウマスク関連関数
    fn is_inside_window1(&self, x: u16) -> bool {
        let x = x as u8;
        self.window1_left <= self.window1_right && x >= self.window1_left && x <= self.window1_right
            || self.window1_left > self.window1_right
                && (x >= self.window1_left || x <= self.window1_right)
    }

    fn is_inside_window2(&self, x: u16) -> bool {
        let x = x as u8;
        self.window2_left <= self.window2_right && x >= self.window2_left && x <= self.window2_right
            || self.window2_left > self.window2_right
                && (x >= self.window2_left || x <= self.window2_right)
    }

    fn evaluate_window_mask(&self, x: u16, mask_setting: u8, logic: u8) -> bool {
        // マスク設定のビット構成:
        // Bit 0: Window 1 Inverted
        // Bit 1: Window 1 Enabled
        // Bit 2: Window 2 Inverted
        // Bit 3: Window 2 Enabled
        // Logic is provided by WBGLOG/WOBJLOG (00=OR, 01=AND, 10=XOR, 11=XNOR)

        if mask_setting == 0 {
            return false; // ウィンドウ無効
        }

        let w1_enabled = (mask_setting & 0x02) != 0;
        let w1_inverted = (mask_setting & 0x01) != 0;
        let w2_enabled = (mask_setting & 0x08) != 0;
        let w2_inverted = (mask_setting & 0x04) != 0;

        let w1_result = if w1_enabled {
            let inside = self.is_inside_window1(x);
            if w1_inverted {
                !inside
            } else {
                inside
            }
        } else {
            false
        };

        let w2_result = if w2_enabled {
            let inside = self.is_inside_window2(x);
            if w2_inverted {
                !inside
            } else {
                inside
            }
        } else {
            false
        };

        // ウィンドウが1つも有効でない場合
        if !w1_enabled && !w2_enabled {
            return false;
        }

        // ウィンドウが1つだけ有効な場合
        if w1_enabled && !w2_enabled {
            return w1_result;
        }
        if !w1_enabled && w2_enabled {
            return w2_result;
        }

        // 両方有効な場合はロジック演算
        let res = match logic & 0x03 {
            0 => w1_result || w2_result,   // OR
            1 => w1_result && w2_result,   // AND
            2 => w1_result ^ w2_result,    // XOR
            3 => !(w1_result ^ w2_result), // XNOR
            _ => false,
        };

        if crate::debug_flags::render_metrics() && res {
            match logic & 0x03 {
                2 => {
                    /* XOR */
                    let v = self.dbg_win_xor_applied.saturating_add(1);
                    let _ = v;
                }
                3 => {
                    /* XNOR */
                    let v = self.dbg_win_xnor_applied.saturating_add(1);
                    let _ = v;
                }
                _ => {}
            }
        }
        res
    }

    fn should_mask_bg(&self, x: u16, bg_num: u8, is_main: bool) -> bool {
        if bg_num >= 4 {
            return false;
        }
        if self.line_window_prepared {
            let idx = x as usize;
            if is_main {
                self.main_bg_window_lut[bg_num as usize][idx] != 0
            } else {
                self.sub_bg_window_lut[bg_num as usize][idx] != 0
            }
        } else {
            // TMW/TSW gating: if disabled for this screen+BG, do not mask
            let enabled = if is_main {
                (self.tmw_mask & (1 << bg_num)) != 0
            } else {
                (self.tsw_mask & (1 << bg_num)) != 0
            };
            if !enabled {
                return false;
            }
            self.evaluate_window_mask(
                x,
                self.window_bg_mask[bg_num as usize],
                self.bg_window_logic[bg_num as usize],
            )
        }
    }

    fn should_mask_sprite(&self, x: u16, is_main: bool) -> bool {
        if self.line_window_prepared {
            let idx = x as usize;
            if is_main {
                self.main_obj_window_lut[idx] != 0
            } else {
                self.sub_obj_window_lut[idx] != 0
            }
        } else {
            let enabled = if is_main {
                (self.tmw_mask & 0x10) != 0
            } else {
                (self.tsw_mask & 0x10) != 0
            };
            if !enabled {
                return false;
            }
            self.evaluate_window_mask(x, self.window_obj_mask, self.obj_window_logic)
        }
    }

    #[allow(dead_code)]
    fn render_bg_with_window_mask(
        &self,
        x: u16,
        y: u16,
        bg_num: u8,
        render_func: fn(&Self, u16, u16, u8) -> (u32, u8),
    ) -> (u32, u8) {
        // ウィンドウマスクでマスクされている場合は透明を返す
        if self.should_mask_bg(x, bg_num, true) {
            return (0, 0);
        }

        // 通常の描画処理
        render_func(self, x, y, bg_num)
    }

    // VRAM address remapping helper (VMAIN bits 3-2 "Full Graphic Mode")
    //
    // SNESdev wiki:
    // - mode 0: none
    // - mode 1: rotate low 8 bits by 3 (2bpp)  : aaaaaaaaYYYccccc -> aaaaaaaacccccYYY
    // - mode 2: rotate low 9 bits by 3 (4bpp)  : aaaaaaaYYYcccccP -> aaaaaaacccccPYYY
    // - mode 3: rotate low 10 bits by 3 (8bpp) : aaaaaaYYYcccccPP -> aaaaaacccccPPYYY
    fn vram_remap_word_addr(&self, addr: u16) -> u16 {
        let mode = (self.vram_mapping >> 2) & 0x03;
        if mode == 0 {
            return addr & 0x7FFF;
        }

        let rotate_bits = match mode {
            1 => 8u8,
            2 => 9u8,
            _ => 10u8,
        };
        let mask: u16 = (1u16 << rotate_bits) - 1;
        let low = addr & mask;
        let y = low >> (rotate_bits - 3);
        let rest = low & ((1u16 << (rotate_bits - 3)) - 1);
        let remapped = (addr & !mask) | (rest << 3) | y;
        remapped & 0x7FFF
    }

    #[inline]
    fn reload_vram_read_latch(&mut self) {
        // SNESdev wiki: VRAM reads via $2139/$213A are only valid during VBlank or forced blank.
        // Outside those periods, the latch is not updated (returns invalid/old data).
        if !self.can_read_vram_now() {
            return;
        }

        let masked = self.vram_remap_word_addr(self.vram_addr) as usize;
        let idx = masked.saturating_mul(2);
        if idx + 1 < self.vram.len() {
            self.vram_read_buf_lo = self.vram[idx];
            self.vram_read_buf_hi = self.vram[idx + 1];
        } else {
            self.vram_read_buf_lo = 0;
            self.vram_read_buf_hi = 0;
        }
    }

    pub fn read(&mut self, addr: u16) -> u8 {
        match addr {
            0x34 => (self.mode7_mul_result & 0xFF) as u8, // product bit7-0
            0x35 => ((self.mode7_mul_result >> 8) & 0xFF) as u8, // product bit15-8
            0x36 => ((self.mode7_mul_result >> 16) & 0xFF) as u8, // product bit23-16
            0x37 => {
                // $2137 (SLHV) - latch H/V counters on read.
                // On read: counter_latch = 1 (always).
                // The returned value is open bus on real hardware.
                // Super Famicom Dev Wiki: latching is gated by $4201 bit7.
                if self.wio_latch_enable {
                    self.latch_hv_counters();
                }
                0
            }
            0x38 => {
                // OAMDATAREAD ($2138)
                // SNESdev wiki:
                // - $2102/$2103 set OAM *word* address, and internal OAM address becomes (word<<1).
                // - $2138 reads from the internal OAM *byte* address and increments it by 1.
                // - High table (0x200..0x21F) repeats for internal addresses 0x200..0x3FF.
                let internal = self.oam_internal_addr & 0x03FF;
                let mapped = if internal < 0x200 {
                    internal
                } else {
                    0x200 | (internal & 0x001F)
                };
                let v = self.oam.get(mapped as usize).copied().unwrap_or(0);
                self.oam_internal_addr = (internal + 1) & 0x03FF;
                v
            }
            0x39 | 0x3A => {
                // VRAM data read ($2139/$213A): one-word latch behavior.
                // - Reading returns the current latch byte.
                // - On the incrementing access (VMAIN bit7 selects low/high), the latch is reloaded
                //   from the current VMADD *before* VMADD is incremented.
                let ret = if addr == 0x39 {
                    self.vram_read_buf_lo
                } else {
                    self.vram_read_buf_hi
                };

                let inc_on_high = (self.vram_mapping & 0x80) != 0;
                let should_inc = (addr == 0x39 && !inc_on_high) || (addr == 0x3A && inc_on_high);
                if should_inc {
                    self.reload_vram_read_latch();
                    self.vram_addr = self.vram_addr.wrapping_add(self.vram_increment);
                }

                ret
            }
            0x3B => {
                // CGRAM Read ($213B): two-step read like write path.
                // Returns low byte first, then high (with bit7 masked), and increments address after high.
                let base = (self.cgram_addr as usize) * 2;
                if !self.cgram_read_second {
                    self.cgram_read_second = true;
                    if base < self.cgram.len() {
                        self.cgram[base]
                    } else {
                        0
                    }
                } else {
                    self.cgram_read_second = false;
                    let hi = if base + 1 < self.cgram.len() {
                        self.cgram[base + 1] & 0x7F
                    } else {
                        0
                    };
                    self.cgram_addr = self.cgram_addr.wrapping_add(1);
                    hi
                }
            }
            0x3C => {
                // OPHCT ($213C) - Latched horizontal counter (2-step read: low then high bit)
                let v = if !self.ophct_second {
                    (self.hv_latched_h & 0x00FF) as u8
                } else {
                    ((self.hv_latched_h >> 8) & 0x01) as u8
                };
                self.ophct_second = !self.ophct_second;
                v
            }
            0x3D => {
                // OPVCT ($213D) - Latched vertical counter (2-step read: low then high bit)
                let v = if !self.opvct_second {
                    (self.hv_latched_v & 0x00FF) as u8
                } else {
                    ((self.hv_latched_v >> 8) & 0x01) as u8
                };
                self.opvct_second = !self.opvct_second;
                v
            }
            0x3E => {
                // STAT77 - PPU Status Flag and Version
                // trm-vvvv
                // t = time over, r = range over, m = master/slave (always 0 here), v = version.
                const STAT77_VER: u8 = 0x01;
                let mut v = 0u8;
                // SNESdev wiki:
                // bit7: Time over flag (sprite tile fetch overflow, >34 tiles on scanline)
                // bit6: Range over flag (sprite overflow, >32 sprites on scanline)
                if self.sprite_overflow_latched {
                    v |= 0x40;
                }
                if self.sprite_time_over_latched {
                    v |= 0x80;
                }
                v | (STAT77_VER & 0x0F)
            }
            0x3F => {
                // STAT78 - PPU Status Flag and Version
                // fl-pvvvv
                // f = interlace field (toggles every VBlank)
                // l = external latch flag (set on HV latch, cleared on read when $4201 bit7=1)
                // p = PAL (0 on NTSC)
                // v = version
                const STAT78_VER: u8 = 0x03;
                let mut v = 0u8;
                if self.interlace_field {
                    v |= 0x80;
                }
                if self.stat78_latch_flag {
                    v |= 0x40;
                }
                // NTSC: bit4 stays 0
                v |= STAT78_VER & 0x0F;

                // Side effect: reset OPHCT/OPVCT high/low selectors.
                self.ophct_second = false;
                self.opvct_second = false;

                // Side effect: counter_latch = 0.
                // Super Famicom Dev Wiki: latch flag clears on read only when $4201 bit7 is set.
                if self.wio_latch_enable {
                    self.stat78_latch_flag = false;
                }
                v
            }
            _ => 0,
        }
    }

    pub fn write(&mut self, addr: u16, mut value: u8) {
        // Debug $2105 writes to detect corruption
        if addr == 0x05 {
            static mut BG_MODE_WRITE_COUNT: u32 = 0;
            unsafe {
                BG_MODE_WRITE_COUNT += 1;
                if BG_MODE_WRITE_COUNT <= 20 && !crate::debug_flags::quiet() {
                    println!(
                        "🔍 BG_MODE_WRITE[{}]: addr=0x{:02X}, value=0x{:02X} (Mode {})",
                        BG_MODE_WRITE_COUNT,
                        addr,
                        value,
                        value & 0x07
                    );
                }
            }
        }

        // デバッグ：全PPUレジスタ書き込み（抑制可能）
        if crate::debug_flags::ppu_write() {
            static mut TOTAL_PPU_WRITES: u32 = 0;
            unsafe {
                TOTAL_PPU_WRITES += 1;
                if TOTAL_PPU_WRITES <= 50 || TOTAL_PPU_WRITES.is_multiple_of(100) {
                    println!(
                        "PPU Write #{}: 0x21{:02X} = 0x{:02X}",
                        TOTAL_PPU_WRITES, addr, value
                    );
                }
            }
        }

        // デバッグ：重要なPPUレジスタ書き込みをログ
        static mut IMPORTANT_WRITES: u32 = 0;
        static mut VRAM_DATA_WRITES: u32 = 0;
        static mut CGRAM_DATA_WRITES: u32 = 0;
        let is_important = matches!(
            addr,
            0x00 | 0x01 | 0x2C | 0x2D | 0x2E | 0x2F | 0x30 | 0x31 | 0x32 | 0x33
        );
        let is_vram_data = matches!(addr, 0x18 | 0x19); // VRAM data registers
        let is_cgram_data = addr == 0x22; // CGRAM data register

        if is_important && (crate::debug_flags::ppu_write() || crate::debug_flags::boot_verbose()) {
            unsafe {
                IMPORTANT_WRITES += 1;
                if IMPORTANT_WRITES <= IMPORTANT_WRITE_LIMIT {
                    println!("PPU Important Write: 0x21{:02X} = 0x{:02X}", addr, value);
                }
            }
        }

        // Monitor VRAM data writes more closely
        if is_vram_data && (crate::debug_flags::ppu_write() || crate::debug_flags::boot_verbose()) {
            unsafe {
                VRAM_DATA_WRITES += 1;
                // Show more VRAM writes to detect patterns
                if VRAM_DATA_WRITES <= 50 || VRAM_DATA_WRITES.is_multiple_of(100) || value != 0x00 {
                    println!(
                        "VRAM DATA[{}]: 0x21{:02X} = 0x{:02X} (addr=0x{:04X}) [{}]",
                        VRAM_DATA_WRITES,
                        addr,
                        value,
                        self.vram_addr,
                        if value == 0x00 { "clear" } else { "data" }
                    );
                }

                // Detect potential graphics loading patterns
                if value != 0x00 && self.vram_addr <= 0x8000 && VRAM_DATA_WRITES.is_multiple_of(500)
                {
                    println!(
                        "GRAPHICS LOADING: {} non-zero VRAM writes detected at 0x{:04X}",
                        VRAM_DATA_WRITES, self.vram_addr
                    );
                }
            }
        }

        // Monitor CGRAM data writes
        if is_cgram_data && (crate::debug_flags::ppu_write() || crate::debug_flags::boot_verbose())
        {
            unsafe {
                CGRAM_DATA_WRITES += 1;
                if CGRAM_DATA_WRITES <= 100 || CGRAM_DATA_WRITES.is_multiple_of(50) || value != 0x00
                {
                    let color_index = self.cgram_addr >> 1;
                    let is_high = (self.cgram_addr & 1) == 1;
                    println!(
                        "CGRAM DATA[{}]: 0x2122 = 0x{:02X} (addr=0x{:02X}) [color {} {}]",
                        CGRAM_DATA_WRITES,
                        value,
                        self.cgram_addr,
                        color_index,
                        if is_high { "HIGH" } else { "LOW" }
                    );
                }

                // Detect complete palette loading
                if CGRAM_DATA_WRITES > 0 && CGRAM_DATA_WRITES.is_multiple_of(32) {
                    println!(
                        "PALETTE PROGRESS: {} colors potentially loaded",
                        CGRAM_DATA_WRITES / 2
                    );
                }
            }
        }

        match addr {
            0x00 => {
                // INIDISP - Forced blank and brightness
                let prev_display = self.screen_display;
                let defer_update =
                    crate::debug_flags::strict_ppu_timing() && self.in_active_display();

                // DQ3: optionally block DMA/HDMAによる強制ブランキング
                if self.dq3_block_inidisp && self.write_ctx != 0 {
                    return;
                }

                // Optional debug override: ignore forced blank/zero brightness when DQ3_FORCE_DISPLAY=1
                if std::env::var("DQ3_FORCE_DISPLAY")
                    .map(|v| v == "1" || v.to_lowercase() == "true")
                    .unwrap_or(false)
                {
                    // Clear forced blank bit and clamp brightness to at least 1
                    let mut forced = value & 0x0F;
                    if forced == 0 {
                        forced = 0x0F;
                    }
                    let mut patched = value & 0xF0; // keep high nibble for logging
                    patched &= !0x80; // clear forced blank
                    patched = (patched & 0xF0) | (forced & 0x0F);
                    if patched != value {
                        value = patched;
                    }
                }
                // Optional: globally lock the display ON (for stubborn titles like SMW when APU upload is stubbed)
                if std::env::var("FORCE_INIDISP_ON")
                    .map(|v| v == "1" || v.to_lowercase() == "true")
                    .unwrap_or(false)
                {
                    let mut patched = value & 0x0F; // brightness only
                    if patched == 0 {
                        patched = 0x0F;
                    }
                    value = patched; // ensure forced blank bit cleared
                }

                // Optional: ignore CPU writes to INIDISP (debug workaround for stubborn blanking)
                if self.write_ctx == 0 && crate::debug_flags::ignore_inidisp_cpu() {
                    return;
                }

                // CPU writes (write_ctx == 0): log first few to catch unintended values (e.g., 0x9X)
                if self.write_ctx == 0 && std::env::var_os("TRACE_INIDISP_CPU").is_some() {
                    use std::sync::atomic::{AtomicU32, Ordering};
                    static COUNT: AtomicU32 = AtomicU32::new(0);
                    let n = COUNT.fetch_add(1, Ordering::Relaxed);
                    if n < 32 {
                        println!(
                            "[INIDISP-CPU][{}] scanline={} value=0x{:02X} (prev=0x{:02X} forced_blank={})",
                            n + 1,
                            self.scanline,
                            value,
                            prev_display,
                            (value & 0x80) != 0
                        );
                    }
                }

                // DMA/HDMA writes to INIDISP ($2100) are valid on real hardware.
                // We only block them when explicitly requested for debugging.
                if self.write_ctx != 0 {
                    if crate::debug_flags::block_inidisp_dma() {
                        return;
                    }
                    if std::env::var_os("DEBUG_INIDISP_DMA").is_some()
                        && !crate::debug_flags::quiet()
                    {
                        let source = match self.write_ctx {
                            1 => "MDMA",
                            2 => "HDMA",
                            _ => "unknown",
                        };
                        let ch = self.debug_dma_channel.unwrap_or(0xFF);
                        println!(
                            "[INIDISP-DMA] {} ch={} scanline={} cyc={} value=0x{:02X} blank={} brightness={}",
                            source,
                            if ch == 0xFF { -1 } else { ch as i32 },
                            self.scanline,
                            self.cycle,
                            value,
                            ((value & 0x80) != 0) as u8,
                            value & 0x0F,
                        );
                    }
                }

                // Optional debug override: force display on with max brightness
                if std::env::var_os("FORCE_MAX_BRIGHTNESS").is_some() {
                    self.screen_display = 0x0F;
                    self.brightness = 0x0F;
                    return;
                }

                let applied_value = value;
                if defer_update {
                    self.latched_inidisp = Some(applied_value);
                } else {
                    self.screen_display = applied_value;
                    self.brightness = applied_value & 0x0F;
                }
                let log_value = if defer_update {
                    applied_value
                } else {
                    self.screen_display
                };
                let forced_blank_prev = (prev_display & 0x80) != 0;
                let forced_blank_new = (log_value & 0x80) != 0;
                if crate::debug_flags::boot_verbose() {
                    println!(
                        "SCREEN CONTROL: brightness={} (forced_blank={}) value=0x{:02X}",
                        log_value & 0x0F,
                        forced_blank_new,
                        log_value
                    );
                }
                static mut SCREEN_CONTROL_COUNT: u32 = 0;
                static mut SCREEN_CONTROL_SUPPRESSED: bool = false;
                const SCREEN_CONTROL_LOG_LIMIT: u32 = 32;
                let quiet = crate::debug_flags::quiet();
                let verbose = (!quiet)
                    && (crate::debug_flags::render_verbose()
                        || crate::debug_flags::trace_ppu_inidisp());
                unsafe {
                    SCREEN_CONTROL_COUNT = SCREEN_CONTROL_COUNT.saturating_add(1);
                    let count = SCREEN_CONTROL_COUNT;
                    let should_log = (!quiet) && (verbose || count <= SCREEN_CONTROL_LOG_LIMIT);
                    if should_log {
                        println!(
                            "PPU[{}]: Screen control {} 0x{:02X} (brightness={}, forced_blank={}, latched={})",
                            count,
                            if defer_update { "latched" } else { "set to" },
                            log_value,
                            log_value & 0x0F,
                            forced_blank_new,
                            defer_update
                        );
                    } else if !quiet && !SCREEN_CONTROL_SUPPRESSED {
                        println!(
                            "[ppu-screen] ログが多いため以降のINIDISP出力を抑制します (DEBUG_RENDER=1 で全件表示)"
                        );
                        SCREEN_CONTROL_SUPPRESSED = true;
                    }
                }
                if !quiet && crate::debug_flags::trace_ppu_inidisp() {
                    println!(
                        "TRACE_PPU_INIDISP: prev=0x{:02X} new=0x{:02X} forced_blank {}→{} brightness {}→{} (latched={})",
                        prev_display,
                        log_value,
                        forced_blank_prev,
                        forced_blank_new,
                        prev_display & 0x0F,
                        log_value & 0x0F,
                        defer_update
                    );
                }
            }
            0x01 => {
                // OBSEL ($2101): Sprite size and name base
                // bits 5-7: sprite size, bits 3-4: name select, bits 0-2: name base high bits
                self.sprite_size = (value >> 5) & 0x07;
                // Gap between tiles 0x0FF and 0x100 (4K-word steps).
                self.sprite_name_select = (((value >> 3) & 0x03) as u16) << 12;
                // Base address for tiles 0x000..0x0FF (8K-word steps).
                self.sprite_name_base = ((value & 0x07) as u16) << 13;
                if crate::debug_flags::ppu_write() || crate::debug_flags::boot_verbose() {
                    println!(
                        "PPU: Sprite size: {}, name base: 0x{:04X}, select: 0x{:04X}",
                        self.sprite_size, self.sprite_name_base, self.sprite_name_select
                    );
                }
            }
            0x02 => {
                // OAMADDL ($2102)
                // Sets OAM *word* address (low 8 bits). Internal address becomes (word<<1).
                self.oam_addr = (self.oam_addr & 0x0100) | (value as u16);
                self.oam_addr &= 0x01FF;
                self.oam_internal_addr = (self.oam_addr & 0x01FF) << 1;
                self.oam_eval_base = if self.oam_priority_rotation_enabled {
                    ((self.oam_addr & 0x00FE) >> 1) as u8
                } else {
                    0
                };
                // Start small OAM data gap (for MDMA/CPU) after address change
                self.oam_data_gap_ticks = crate::debug_flags::oam_gap_after_oamadd();
            }
            0x03 => {
                // OAMADDH ($2103)
                // SNESdev wiki:
                // - bit0: OAM word address bit8
                // - bit7: OBJ priority rotation enable
                self.oam_priority_rotation_enabled = (value & 0x80) != 0;
                self.oam_addr = (self.oam_addr & 0x00FF) | (((value as u16) & 0x01) << 8);
                self.oam_addr &= 0x01FF;
                self.oam_internal_addr = (self.oam_addr & 0x01FF) << 1;
                self.oam_eval_base = if self.oam_priority_rotation_enabled {
                    ((self.oam_addr & 0x00FE) >> 1) as u8
                } else {
                    0
                };
                self.oam_data_gap_ticks = crate::debug_flags::oam_gap_after_oamadd();
            }
            0x04 => {
                // OAMDATA ($2104)
                // SNESdev wiki:
                // - Low table (internal < 0x200): writes are staged; the *odd* byte write commits a word.
                // - High table (internal >= 0x200): direct byte writes; internal increments by 1 each time.
                if !self.can_write_oam_now() {
                    if crate::debug_flags::ppu_write() || crate::debug_flags::boot_verbose() {
                        println!("PPU TIMING: Skip OAMDATA write outside VBlank (strict)");
                    }
                    self.oam_rejects = self.oam_rejects.saturating_add(1);
                    if self.oam_data_gap_ticks > 0 && self.write_ctx != 2 {
                        self.oam_gap_blocks = self.oam_gap_blocks.saturating_add(1);
                    }
                    if crate::debug_flags::timing_rejects()
                        && self.last_reject_frame_oam != self.frame
                    {
                        let who = match self.write_ctx {
                            2 => "HDMA",
                            1 => "MDMA",
                            _ => "CPU",
                        };
                        let reason = if self.oam_data_gap_ticks > 0 && self.write_ctx != 2 {
                            "[gap]"
                        } else {
                            ""
                        };
                        println!(
                            "⛔ OAM REJECT: y={} x={} ctx={} addr=$2104 {}",
                            self.scanline, self.cycle, who, reason
                        );
                        self.last_reject_frame_oam = self.frame;
                    }
                    return;
                }
                let internal = self.oam_internal_addr & 0x03FF;
                if internal < 0x200 {
                    if (internal & 1) == 0 {
                        self.oam_write_latch = value;
                    } else {
                        let even = (internal & !1) as usize;
                        let odd = internal as usize;
                        if even < self.oam.len() {
                            self.oam[even] = self.oam_write_latch;
                        }
                        if odd < self.oam.len() {
                            self.oam[odd] = value;
                        }
                        self.oam_writes_total = self.oam_writes_total.saturating_add(2);
                    }
                } else {
                    let mapped = (0x200 | (internal & 0x001F)) as usize;
                    if mapped < self.oam.len() {
                        self.oam[mapped] = value;
                    }
                    self.oam_writes_total = self.oam_writes_total.saturating_add(1);
                }
                self.oam_internal_addr = (internal + 1) & 0x03FF;
            }
            0x05 => {
                // BGMODE: bit0-2: mode, bit4-7: tile size for BG1..BG4 (1=16x16)
                let requested_mode = value & 0x07;
                self.bg_mode = requested_mode;
                // Mode 1 BG3 priority bit (bit3)
                self.mode1_bg3_priority = (value & 0x08) != 0;
                self.bg_tile_16[0] = (value & 0x10) != 0;
                self.bg_tile_16[1] = (value & 0x20) != 0;
                self.bg_tile_16[2] = (value & 0x40) != 0;
                self.bg_tile_16[3] = (value & 0x80) != 0;

                if (crate::debug_flags::ppu_write() || crate::debug_flags::boot_verbose())
                    && !crate::debug_flags::quiet()
                {
                    println!(
                        "PPU: BG Mode set to {} (reg $2105 = 0x{:02X}), BG3prio={}, tile16: [{},{},{},{}]",
                        self.bg_mode,
                        value,
                        self.mode1_bg3_priority,
                        self.bg_tile_16[0],
                        self.bg_tile_16[1],
                        self.bg_tile_16[2],
                        self.bg_tile_16[3]
                    );
                    if self.bg_mode == 2 {
                        println!(
                            "  Mode 2 activated: BG1 & BG2 use 4bpp tiles with offset-per-tile"
                        );
                    }
                }
            }
            0x06 => {
                self.bg_mosaic = value;
                self.mosaic_size = ((value >> 4) & 0x0F) + 1; // ビット4-7がモザイクサイズ（0-15 → 1-16）
            }
            0x07 => {
                // BG1SC ($2107):
                // - bits 0-1: screen size
                // - bits 2-7: tilemap base in units of 0x400 bytes (1KB)
                // Tilemap base is stored as VRAM *word* address.
                // Common reference formula (SNESdev): base word address = (value & 0xFC) << 8.
                self.bg1_tilemap_base = ((value as u16) & 0xFC) << 8;
                self.bg_screen_size[0] = value & 0x03;
                if !crate::debug_flags::quiet() {
                    println!(
                        "PPU: BG1 tilemap base: 0x{:04X}, size={}",
                        self.bg1_tilemap_base, self.bg_screen_size[0]
                    );
                }
            }
            0x08 => {
                // BG2SC ($2108): store base as VRAM word address (see $2107)
                self.bg2_tilemap_base = ((value as u16) & 0xFC) << 8;
                self.bg_screen_size[1] = value & 0x03;
                if (crate::debug_flags::ppu_write() || crate::debug_flags::boot_verbose())
                    && !crate::debug_flags::quiet()
                {
                    println!(
                        "PPU: BG2 tilemap base: 0x{:04X}, size={}",
                        self.bg2_tilemap_base, self.bg_screen_size[1]
                    );
                }
            }
            0x09 => {
                // BG3SC ($2109): store base as VRAM word address (see $2107)
                self.bg3_tilemap_base = ((value as u16) & 0xFC) << 8;
                self.bg_screen_size[2] = value & 0x03;
                if !crate::debug_flags::quiet() {
                    println!(
                        "PPU: BG3 tilemap base: raw=0x{:02X} -> base=0x{:04X} (byte=0x{:05X}), size={}",
                        value, self.bg3_tilemap_base, (self.bg3_tilemap_base as u32) * 2, self.bg_screen_size[2]
                    );
                }
            }
            0x0A => {
                // BG4SC ($210A): store base as VRAM word address (see $2107)
                self.bg4_tilemap_base = ((value as u16) & 0xFC) << 8;
                self.bg_screen_size[3] = value & 0x03;
            }
            0x0B => {
                // BG12NBA ($210B): Character (tile) data area designation.
                // Bits 0-3: BG1 base, bits 4-7: BG2 base.
                // Unit is 0x2000 bytes (8 KiB); VRAM is word-addressed.
                // => base_word = nibble * 0x1000 words (0x2000 bytes)
                let bg1 = (value & 0x0F) as u16;
                let bg2 = ((value >> 4) & 0x0F) as u16;
                self.bg1_tile_base = bg1 << 12;
                self.bg2_tile_base = bg2 << 12;
                if !crate::debug_flags::quiet() {
                    println!(
                        "PPU: BG1 tile base: 0x{:04X}, BG2 tile base: 0x{:04X}",
                        self.bg1_tile_base, self.bg2_tile_base
                    );
                }
            }
            0x0C => {
                // BG34NBA ($210C): Character (tile) data area designation.
                // Bits 0-3: BG3 base, bits 4-7: BG4 base.
                // Unit is 0x2000 bytes (8 KiB); VRAM is word-addressed.
                let bg3 = (value & 0x0F) as u16;
                let bg4 = ((value >> 4) & 0x0F) as u16;
                self.bg3_tile_base = bg3 << 12;
                self.bg4_tile_base = bg4 << 12;
                if (crate::debug_flags::ppu_write() || crate::debug_flags::boot_verbose())
                    && !crate::debug_flags::quiet()
                {
                    println!(
                        "PPU: BG3 tile base: 0x{:04X}, BG4 tile base: 0x{:04X}",
                        self.bg3_tile_base, self.bg4_tile_base
                    );
                }
            }
            0x0D => {
                self.write_bghofs(0, value);
            }
            0x0E => {
                self.write_bgvofs(0, value);
            }
            0x0F => {
                self.write_bghofs(1, value);
            }
            0x10 => {
                self.write_bgvofs(1, value);
            }
            0x11 => {
                self.write_bghofs(2, value);
            }
            0x12 => {
                self.write_bgvofs(2, value);
            }
            0x13 => {
                self.write_bghofs(3, value);
            }
            0x14 => {
                self.write_bgvofs(3, value);
            }
            0x15 => {
                // $2115: VRAM Address Increment/Mapping
                // NOTE: VMAIN is a normal control register; in the common (non-strict) path
                // it takes effect immediately. Any deferral is debug-only behind STRICT_PPU_TIMING.
                if crate::debug_flags::strict_ppu_timing() {
                    // In STRICT timing, defer changes to a safe sub-window.
                    // Always record last written for summaries.
                    self.vram_last_vmain = value;
                    if self.can_commit_vmain_now() {
                        // Defer the visible effect by a small number of dots (debug-only)
                        self.vmain_effect_pending = Some(value);
                        self.vmain_effect_ticks = crate::debug_flags::vmain_effect_delay_dots();
                    } else {
                        self.latched_vmain = Some(value);
                    }
                } else {
                    // Immediate apply (default)
                    self.vram_mapping = value;
                    self.vram_last_vmain = value;
                    self.vram_increment = match value & 0x03 {
                        0 => 1,
                        1 => 32,
                        _ => 128,
                    };
                    self.vmain_effect_pending = None;
                    self.vmain_effect_ticks = 0;
                    self.latched_vmain = None;
                }
                if crate::debug_flags::ppu_write() || crate::debug_flags::boot_verbose() {
                    static mut VMAIN_LOG_CNT: u32 = 0;
                    unsafe {
                        VMAIN_LOG_CNT += 1;
                        if VMAIN_LOG_CNT <= 8 {
                            let inc = match value & 0x03 {
                                0 => 1,
                                1 => 32,
                                _ => 128,
                            };
                            let fg = (value >> 2) & 0x03;
                            let inc_on_high = (value & 0x80) != 0;
                            println!("VMAIN write: 0x{:02X} (inc={}, FGmode={}, inc_on_{}, pending_commit={})",
                                value, inc, fg, if inc_on_high {"HIGH"} else {"LOW"},
                                crate::debug_flags::strict_ppu_timing() && !self.can_commit_vmain_now());
                        }
                    }
                }
            }
            0x16 => {
                if self.can_commit_vmadd_now() {
                    self.vram_addr = (self.vram_addr & 0xFF00) | (value as u16);
                    // SNESdev wiki: On VMADD write, vram_latch = [VMADD]
                    self.reload_vram_read_latch();
                } else {
                    self.latched_vmadd_lo = Some(value);
                }
                if crate::debug_flags::boot_verbose() {
                    static mut VRAM_ADDR_SET_COUNT: u32 = 0;
                    unsafe {
                        VRAM_ADDR_SET_COUNT += 1;
                        if VRAM_ADDR_SET_COUNT <= 10 {
                            println!(
                                "VRAM address LOW write: 0x{:02X} (pending_commit={})",
                                value,
                                !self.can_commit_vmadd_now()
                            );
                        }
                    }
                }
            }
            0x17 => {
                if self.can_commit_vmadd_now() {
                    self.vram_addr = (self.vram_addr & 0x00FF) | ((value as u16) << 8);
                    // SNESdev wiki: On VMADD write, vram_latch = [VMADD]
                    self.reload_vram_read_latch();
                } else {
                    self.latched_vmadd_hi = Some(value);
                }
                if crate::debug_flags::boot_verbose() {
                    static mut VRAM_ADDR_SET_COUNT_HIGH: u32 = 0;
                    unsafe {
                        VRAM_ADDR_SET_COUNT_HIGH += 1;
                        if VRAM_ADDR_SET_COUNT_HIGH <= 10 {
                            println!(
                                "VRAM address HIGH write: 0x{:02X} (pending_commit={})",
                                value,
                                !self.can_commit_vmadd_now()
                            );
                        }
                    }
                }
            }
            0x18 => {
                // VRAM Data Write (Low byte) - $2118
                // STRICT: 許可はVBlankまたはHBlank中（HDMA先頭含む）の安全ドットのみ
                if !self.is_vram_write_safe_dot() {
                    if crate::debug_flags::ppu_write() || crate::debug_flags::boot_verbose() {
                        println!("PPU TIMING: Skip VMDATAL write outside blank (strict)");
                    }
                    self.vram_rejects = self.vram_rejects.saturating_add(1);
                    if self.vmain_data_gap_ticks > 0 && self.write_ctx != 2 {
                        self.vram_gap_blocks = self.vram_gap_blocks.saturating_add(1);
                    }
                    if crate::debug_flags::timing_rejects()
                        && self.last_reject_frame_vram != self.frame
                    {
                        let who = match self.write_ctx {
                            2 => "HDMA",
                            1 => "MDMA",
                            _ => "CPU",
                        };
                        let reason = if self.vmain_data_gap_ticks > 0 && self.write_ctx != 2 {
                            "[gap]"
                        } else {
                            ""
                        };
                        println!(
                            "⛔ VRAM REJECT: y={} x={} ctx={} addr=$2118 {}",
                            self.scanline, self.cycle, who, reason
                        );
                        self.last_reject_frame_vram = self.frame;
                    }
                    // Even if the VRAM write is ignored, VMADD still increments depending on VMAIN.
                    // (SNESdev wiki: "VMADD will always increment ... even if the VRAM write is ignored.")
                    if (self.vram_mapping & 0x80) == 0 {
                        self.vram_addr = self.vram_addr.wrapping_add(self.vram_increment);
                    }
                    return;
                }
                if crate::debug_flags::boot_verbose() {
                    static mut VRAM_DIRECT_WRITE_LOW_COUNT: u32 = 0;
                    unsafe {
                        VRAM_DIRECT_WRITE_LOW_COUNT += 1;
                        if VRAM_DIRECT_WRITE_LOW_COUNT <= 20 {
                            println!(
                                "VRAM write $2118 = 0x{:02X} (addr=0x{:04X})",
                                value, self.vram_addr
                            );
                        }
                    }
                }
                let masked_addr = self.vram_remap_word_addr(self.vram_addr); // apply FG mapping
                                                                             // VRAM is word-addressed (0x0000-0x7FFF), but stored as bytes (0x0000-0xFFFF)
                let vram_index = ((masked_addr & 0x7FFF) as usize * 2) & 0xFFFF; // Low byte at even address

                // burn-in-test.sfc DMA MEMORY: detect unexpected writes into the test region.
                if self.burnin_vram_trace_armed
                    && std::env::var_os("TRACE_BURNIN_DMA_MEMORY").is_some()
                    && (0x5000..0x5800).contains(&masked_addr)
                {
                    let dma_ch = self.debug_dma_channel.unwrap_or(0xFF);
                    let is_known = self.write_ctx == 1 && dma_ch == 6;
                    if !is_known {
                        let n = self.burnin_vram_trace_cnt_2118;
                        self.burnin_vram_trace_cnt_2118 =
                            self.burnin_vram_trace_cnt_2118.saturating_add(1);
                        if n < 64 {
                            let who = match self.write_ctx {
                                2 => "HDMA",
                                1 => "MDMA",
                                _ => "CPU",
                            };
                            println!(
                                "[BURNIN-VRAM-WRITE] {} ch={} frame={} sl={} cyc={} vblank={} hblank={} fblank={} vis_h={} VMAIN={:02X} inc={} raw={:04X} masked={:04X} $2118={:02X}",
                                who,
                                dma_ch,
                                self.frame,
                                self.scanline,
                                self.cycle,
                                self.v_blank as u8,
                                self.h_blank as u8,
                                ((self.screen_display & 0x80) != 0) as u8,
                                self.get_visible_height(),
                                self.vram_mapping,
                                self.vram_increment,
                                self.vram_addr,
                                masked_addr,
                                value
                            );
                        }
                    }
                }

                // Summary counters (bucketed by masked word address high bits)
                let bucket = ((masked_addr >> 12) & 0x7) as usize; // 0..7
                if bucket < self.vram_write_buckets.len() {
                    self.vram_write_buckets[bucket] =
                        self.vram_write_buckets[bucket].saturating_add(1);
                }
                self.vram_write_low_count = self.vram_write_low_count.saturating_add(1);
                self.vram_writes_total_low = self.vram_writes_total_low.saturating_add(1);

                // Debug output disabled for performance

                if vram_index < self.vram.len() {
                    self.vram[vram_index] = value;
                } else {
                    println!(
                        "WARNING: VRAM write out of bounds! index=0x{:05X} >= len=0x{:05X}",
                        vram_index,
                        self.vram.len()
                    );
                }
                // アドレスインクリメントモード（bit 7）
                // bit7=0 -> LOW($2118) 書き込み後にインクリメント
                // bit7=1 -> HIGH($2119) 書き込み後にインクリメント
                if (self.vram_mapping & 0x80) == 0 {
                    self.vram_addr = self.vram_addr.wrapping_add(self.vram_increment);
                }
            }
            0x19 => {
                // VRAM Data Write (High byte) - $2119
                if !self.is_vram_write_safe_dot() {
                    if crate::debug_flags::ppu_write() || crate::debug_flags::boot_verbose() {
                        println!("PPU TIMING: Skip VMDATAH write outside blank (strict)");
                    }
                    self.vram_rejects = self.vram_rejects.saturating_add(1);
                    if self.vmain_data_gap_ticks > 0 && self.write_ctx != 2 {
                        self.vram_gap_blocks = self.vram_gap_blocks.saturating_add(1);
                    }
                    if crate::debug_flags::timing_rejects()
                        && self.last_reject_frame_vram != self.frame
                    {
                        let who = match self.write_ctx {
                            2 => "HDMA",
                            1 => "MDMA",
                            _ => "CPU",
                        };
                        let reason = if self.vmain_data_gap_ticks > 0 && self.write_ctx != 2 {
                            "[gap]"
                        } else {
                            ""
                        };
                        println!(
                            "⛔ VRAM REJECT: y={} x={} ctx={} addr=$2119 {}",
                            self.scanline, self.cycle, who, reason
                        );
                        self.last_reject_frame_vram = self.frame;
                    }
                    // Even if the VRAM write is ignored, VMADD still increments depending on VMAIN.
                    if (self.vram_mapping & 0x80) != 0 {
                        self.vram_addr = self.vram_addr.wrapping_add(self.vram_increment);
                    }
                    return;
                }
                if crate::debug_flags::boot_verbose() {
                    static mut VRAM_DIRECT_WRITE_HIGH_COUNT: u32 = 0;
                    unsafe {
                        VRAM_DIRECT_WRITE_HIGH_COUNT += 1;
                        if VRAM_DIRECT_WRITE_HIGH_COUNT <= 20 {
                            println!(
                                "VRAM write $2119 = 0x{:02X} (addr=0x{:04X})",
                                value, self.vram_addr
                            );
                        }
                    }
                }
                let masked_addr = self.vram_remap_word_addr(self.vram_addr);
                let vram_index = (((masked_addr & 0x7FFF) as usize) * 2 + 1) & 0xFFFF; // High byte at odd address

                if self.burnin_vram_trace_armed
                    && std::env::var_os("TRACE_BURNIN_DMA_MEMORY").is_some()
                    && (0x5000..0x5800).contains(&masked_addr)
                {
                    let dma_ch = self.debug_dma_channel.unwrap_or(0xFF);
                    let is_known = self.write_ctx == 1 && dma_ch == 6;
                    if !is_known {
                        let n = self.burnin_vram_trace_cnt_2119;
                        self.burnin_vram_trace_cnt_2119 =
                            self.burnin_vram_trace_cnt_2119.saturating_add(1);
                        if n < 64 {
                            let who = match self.write_ctx {
                                2 => "HDMA",
                                1 => "MDMA",
                                _ => "CPU",
                            };
                            println!(
                                "[BURNIN-VRAM-WRITE] {} ch={} frame={} sl={} cyc={} vblank={} hblank={} fblank={} vis_h={} VMAIN={:02X} inc={} raw={:04X} masked={:04X} $2119={:02X}",
                                who,
                                dma_ch,
                                self.frame,
                                self.scanline,
                                self.cycle,
                                self.v_blank as u8,
                                self.h_blank as u8,
                                ((self.screen_display & 0x80) != 0) as u8,
                                self.get_visible_height(),
                                self.vram_mapping,
                                self.vram_increment,
                                self.vram_addr,
                                masked_addr,
                                value
                            );
                        }
                    }
                }

                // Summary counters
                let bucket = ((masked_addr >> 12) & 0x7) as usize; // 0..7
                if bucket < self.vram_write_buckets.len() {
                    self.vram_write_buckets[bucket] =
                        self.vram_write_buckets[bucket].saturating_add(1);
                }
                self.vram_write_high_count = self.vram_write_high_count.saturating_add(1);
                self.vram_writes_total_high = self.vram_writes_total_high.saturating_add(1);

                if vram_index < self.vram.len() {
                    self.vram[vram_index] = value;
                } else {
                    println!(
                        "WARNING: VRAM high write out of bounds! index=0x{:05X} >= len=0x{:05X}",
                        vram_index,
                        self.vram.len()
                    );
                }

                // Increment when bit7 of VMAIN is 1 (increment on HIGH)
                if (self.vram_mapping & 0x80) != 0 {
                    self.vram_addr = self.vram_addr.wrapping_add(self.vram_increment);
                }
            }

            0x21 => {
                // CGADD - set color index (word address). In strict timing, defer to HBlank mid-window.
                if crate::debug_flags::ppu_write() && !crate::debug_flags::quiet() {
                    static mut CGADD_WRITE_COUNT: u32 = 0;
                    unsafe {
                        CGADD_WRITE_COUNT += 1;
                        if CGADD_WRITE_COUNT <= 64 {
                            println!(
                                "[PPU] CGADD write[{}]: value=0x{:02X}",
                                CGADD_WRITE_COUNT, value
                            );
                        }
                    }
                }
                if self.can_commit_cgadd_now() {
                    self.cgram_addr = value;
                    self.cgram_second = false;
                    self.cgram_read_second = false;
                } else {
                    self.latched_cgadd = Some(value);
                }
            }
            0x22 => {
                // CGDATA - staged writes: commit only on HIGH byte
                if !self.can_write_cgram_now() {
                    if crate::debug_flags::ppu_write() || crate::debug_flags::boot_verbose() {
                        println!("PPU TIMING: Skip CGDATA write outside VBlank (strict)");
                    }
                    self.cgram_rejects = self.cgram_rejects.saturating_add(1);
                    if self.cgram_data_gap_ticks > 0 && self.write_ctx != 2 {
                        self.cgram_gap_blocks = self.cgram_gap_blocks.saturating_add(1);
                    }
                    if crate::debug_flags::timing_rejects()
                        && self.last_reject_frame_cgram != self.frame
                    {
                        let who = match self.write_ctx {
                            2 => "HDMA",
                            1 => "MDMA",
                            _ => "CPU",
                        };
                        let reason = if self.cgram_data_gap_ticks > 0 && self.write_ctx != 2 {
                            "[gap]"
                        } else {
                            ""
                        };
                        println!(
                            "⛔ CGRAM REJECT: y={} x={} ctx={} addr=$2122 {}",
                            self.scanline, self.cycle, who, reason
                        );
                        self.last_reject_frame_cgram = self.frame;
                    }
                    return;
                }
                if !self.cgram_second {
                    // LOW byte stage: latch only
                    self.cgram_latch_lo = value;
                    self.cgram_second = true;
                    static mut CGRAM_WRITE_COUNT: u32 = 0;
                    unsafe {
                        CGRAM_WRITE_COUNT += 1;
                        if CGRAM_WRITE_COUNT <= 10 && !crate::debug_flags::quiet() {
                            println!(
                                "CGRAM write[{}]: color=0x{:02X}, LOW byte, value=0x{:02X}",
                                CGRAM_WRITE_COUNT, self.cgram_addr, value
                            );
                        }
                    }
                } else {
                    // HIGH byte stage: commit both bytes
                    let base = (self.cgram_addr as usize) * 2;
                    if base + 1 < self.cgram.len() {
                        let quiet = crate::debug_flags::quiet();
                        // SNES CGRAM is 15-bit BGR; bit7 of the high byte is ignored.
                        let hi = value & 0x7F;
                        self.cgram[base] = self.cgram_latch_lo;
                        self.cgram[base + 1] = hi;
                        self.cgram_writes_total = self.cgram_writes_total.saturating_add(1);

                        // Debug the actual CGRAM storage
                        static mut CGRAM_STORE_DEBUG: u32 = 0;
                        unsafe {
                            CGRAM_STORE_DEBUG += 1;
                            if crate::debug_flags::ppu_write() && CGRAM_STORE_DEBUG <= 5 && !quiet {
                                let stored_color =
                                    ((hi as u16) << 8) | (self.cgram_latch_lo as u16);
                                println!("🎨 CGRAM STORED[{}]: addr={}, base={}, cgram[{}]=0x{:02X}, cgram[{}]=0x{:02X}, color=0x{:04X}", 
                                        CGRAM_STORE_DEBUG, self.cgram_addr, base, base, self.cgram_latch_lo, base+1, hi, stored_color);
                            }
                        }
                        static mut CGRAM_WRITE_COUNT: u32 = 0;
                        unsafe {
                            CGRAM_WRITE_COUNT += 1;
                            if crate::debug_flags::ppu_write() && CGRAM_WRITE_COUNT <= 10 && !quiet
                            {
                                println!(
                                    "CGRAM write[{}]: color=0x{:02X}, HIGH byte, value=0x{:02X} (masked 0x{:02X})",
                                    CGRAM_WRITE_COUNT, self.cgram_addr, value, hi
                                );
                            }
                        }
                    }
                    // increment address after high byte
                    self.cgram_addr = self.cgram_addr.wrapping_add(1);
                    self.cgram_second = false;
                }
            }
            0x2C => {
                if crate::debug_flags::strict_ppu_timing() && self.in_active_display() {
                    self.latched_tm = Some(value);
                } else {
                    self.main_screen_designation = value;
                    // Remember non-zero values for rendering (workaround for timing issues)
                    if value != 0 {
                        self.main_screen_designation_last_nonzero = value;
                    }
                }
                if crate::debug_flags::ppu_write() && !crate::debug_flags::quiet() {
                    static mut TM_DEBUG_COUNT: u32 = 0;
                    unsafe {
                        if TM_DEBUG_COUNT < 30 {
                            TM_DEBUG_COUNT += 1;
                            println!("PPU[$212C][{}]: scanline={} value=0x{:02X} (BG1:{} BG2:{} BG3:{} BG4:{} OBJ:{}) vblank={} using=0x{:02X}",
                                     TM_DEBUG_COUNT, self.scanline, value,
                                     (value & 1) != 0,
                                     (value & 2) != 0,
                                     (value & 4) != 0,
                                     (value & 8) != 0,
                                     (value & 16) != 0,
                                     self.v_blank,
                                     if value != 0 { value } else { self.main_screen_designation_last_nonzero });
                        }
                    }
                }
            }
            0x2D => {
                if crate::debug_flags::strict_ppu_timing() && self.in_active_display() {
                    self.latched_ts = Some(value);
                } else {
                    self.sub_screen_designation = value;
                }
            }
            0x2E => {
                // TMW - window mask enable (main)
                if crate::debug_flags::strict_ppu_timing() && self.in_active_display() {
                    self.latched_tmw = Some(value & 0x1F);
                } else {
                    self.tmw_mask = value & 0x1F; // BG1..4 + OBJ
                }
            }
            0x2F => {
                // TSW - window mask enable (sub)
                if crate::debug_flags::strict_ppu_timing() && self.in_active_display() {
                    self.latched_tsw = Some(value & 0x1F);
                } else {
                    self.tsw_mask = value & 0x1F;
                }
            }
            // ウィンドウ座標設定
            0x26 => {
                self.window1_left = value;
            }
            0x27 => {
                self.window1_right = value;
            }
            0x28 => {
                self.window2_left = value;
            }
            0x29 => {
                self.window2_right = value;
            }
            // BGウィンドウマスク設定
            0x23 => {
                self.window_bg_mask[0] = value & 0x0F; // BG1
                self.window_bg_mask[1] = (value >> 4) & 0x0F; // BG2
            }
            0x24 => {
                self.window_bg_mask[2] = value & 0x0F; // BG3
                self.window_bg_mask[3] = (value >> 4) & 0x0F; // BG4
            }
            0x25 => {
                self.window_obj_mask = value & 0x0F; // スプライト
                self.window_color_mask = (value >> 4) & 0x0F; // カラー
            }
            0x2A => {
                // WBGLOG: BG1..BG4 window logic (00=OR,01=AND,10=XOR,11=XNOR)
                if crate::debug_flags::strict_ppu_timing() && self.in_active_display() {
                    self.latched_wbglog = Some(value);
                } else {
                    self.bg_window_logic[0] = value & 0x03;
                    self.bg_window_logic[1] = (value >> 2) & 0x03;
                    self.bg_window_logic[2] = (value >> 4) & 0x03;
                    self.bg_window_logic[3] = (value >> 6) & 0x03;
                }
            }
            0x2B => {
                // WOBJLOG: OBJ/COL window logic (00=OR,01=AND,10=XOR,11=XNOR)
                if crate::debug_flags::strict_ppu_timing() && self.in_active_display() {
                    self.latched_wobjlog = Some(value);
                } else {
                    self.obj_window_logic = value & 0x03;
                    self.color_window_logic = (value >> 2) & 0x03;
                }
            }

            // カラー演算制御
            0x30 => {
                // CGWSEL: Color math gating + subscreen/fixed select
                if crate::debug_flags::strict_ppu_timing() && self.in_active_display() {
                    self.latched_cgwsel = Some(value);
                } else {
                    self.cgwsel = value;
                    self.color_math_control = value; // legacy
                }
            }
            0x31 => {
                // CGADSUB: Add/Sub + halve + layer enables
                if crate::debug_flags::strict_ppu_timing() && self.in_active_display() {
                    self.latched_cgadsub = Some(value);
                } else {
                    self.cgadsub = value;
                    self.color_math_designation = value; // legacy: lower 6 bits as layer mask
                }
            }
            0x32 => {
                // 固定色データ設定
                let intensity = value & 0x1F; // 強度（0-31）
                let mut next = self.fixed_color;

                // COLDATA ($2132): bits7/6/5 are enable flags for B/G/R components.
                // When set, the corresponding component of the fixed color is updated to INTENSITY.
                // Multiple components may be updated in a single write.
                //
                // The fixed color uses the same RGB555 layout as CGRAM:
                // bit0-4: Red, bit5-9: Green, bit10-14: Blue.
                if (value & 0x20) != 0 {
                    // Red: bits0-4
                    next = (next & !0x001F) | (intensity as u16);
                }
                if (value & 0x40) != 0 {
                    // Green: bits5-9
                    next = (next & !0x03E0) | ((intensity as u16) << 5);
                }
                if (value & 0x80) != 0 {
                    // Blue: bits10-14
                    next = (next & !0x7C00) | ((intensity as u16) << 10);
                }
                if crate::debug_flags::strict_ppu_timing() && self.in_active_display() {
                    self.latched_fixed_color = Some(next);
                } else {
                    self.fixed_color = next;
                }
            }
            0x33 => {
                // SETINI (pseudo hires, EXTBG, interlace)
                let vblank_start = self.get_visible_height().saturating_add(1);
                if crate::debug_flags::strict_ppu_timing() && self.scanline < vblank_start {
                    // Defer any change during visible region (including HBlank) to line start
                    self.latched_setini = Some(value);
                } else {
                    self.setini = value;
                    self.pseudo_hires = (value & 0x08) != 0;
                    self.extbg = (value & 0x40) != 0;
                    self.overscan = (value & 0x04) != 0;
                    self.obj_interlace = (value & 0x02) != 0;
                    self.interlace = (value & 0x01) != 0;
                }
            }

            // Mode 7 設定
            0x1A => {
                // M7SEL: bit7=R (0:wrap 1:fill), bit6=F (0:transparent 1:char0), bit1=Y flip, bit0=X flip
                self.m7sel = value;
            }
            // Mode 7レジスタ（2回書き込みで16ビット値を構成）
            0x1B..=0x20 => {
                // Mode 7 registers: two writes (low then high), signed 16-bit (8.8 fixed)
                let idx = (addr - 0x1B) as usize; // 0..5 (A,B,C,D,CenterX,CenterY)
                if !self.m7_latch_second[idx] {
                    self.m7_latch_low[idx] = value;
                    self.m7_latch_second[idx] = true;
                } else {
                    let low = self.m7_latch_low[idx] as u16;
                    let high = value as u16;
                    let combined = ((high << 8) | low) as i16;
                    match idx {
                        0 => {
                            self.mode7_matrix_a = combined;
                            self.update_mode7_mul_result();
                        }
                        1 => {
                            self.mode7_matrix_b = combined;
                            self.update_mode7_mul_result();
                            if !crate::debug_flags::quiet() {
                                static M7B_LOG: std::sync::atomic::AtomicU32 =
                                    std::sync::atomic::AtomicU32::new(0);
                                let n = M7B_LOG.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                                if n < 4 {
                                    println!("PPU: Mode 7 matrix B set to {}", self.mode7_matrix_b);
                                }
                            }
                        }
                        2 => {
                            self.mode7_matrix_c = combined;
                        }
                        3 => {
                            self.mode7_matrix_d = combined;
                        }
                        4 => {
                            self.mode7_center_x = combined;
                        }
                        5 => {
                            self.mode7_center_y = combined;
                        }
                        _ => {}
                    }
                    self.m7_latch_second[idx] = false;
                    if idx == 0 && !crate::debug_flags::quiet() {
                        println!("PPU: Mode 7 matrix A set to {}", self.mode7_matrix_a);
                    }
                }
            }

            _ => {}
        }
    }

    pub fn get_framebuffer(&self) -> &[u32] {
        &self.framebuffer
    }

    // Mutable framebuffer accessor (debug use only)
    #[allow(dead_code)]
    pub fn get_framebuffer_mut(&mut self) -> &mut [u32] {
        &mut self.framebuffer
    }

    #[inline]
    pub fn frame(&self) -> u64 {
        self.frame
    }

    /// Mode 7 乗算結果を更新（$2134-$2136）
    fn update_mode7_mul_result(&mut self) {
        // 実機では基本 16x8 符号付き積。デバッグで 16x16 や固定値にも切り替え可能。
        let prod = if let Some(forced) = crate::debug_flags::force_m7_product() {
            forced as i32
        } else {
            let a = self.mode7_matrix_a as i32;
            if crate::debug_flags::m7_mul_full16() {
                let b = self.mode7_matrix_b as i32;
                a * b
            } else {
                let b = (self.mode7_matrix_b as i8) as i32; // low byte, sign-extend 8bit
                a * b
            }
        };
        self.mode7_mul_result = (prod as u32) & 0x00FF_FFFF;
    }

    /// 現在のフレームバッファが全て黒（0x00FFFFFF=0）かどうか簡易判定
    pub fn framebuffer_is_all_black(&self) -> bool {
        self.framebuffer.iter().all(|&p| (p & 0x00FF_FFFF) == 0)
    }

    /// フレームバッファを指定色で塗りつぶす（強制フォールバック用）
    pub fn force_framebuffer_color(&mut self, color: u32) {
        for px in self.framebuffer.iter_mut() {
            *px = color;
        }
    }

    /// DQ3デバッグ用: INIDISP への DMA/HDMA 書き込みを無視するかを設定
    pub fn set_block_inidisp(&mut self, on: bool) {
        self.dq3_block_inidisp = on;
    }

    /// 強制ブランク無視フラグ（DQ3用）を設定
    pub fn set_force_display_override(&mut self, on: bool) {
        self.force_display_override = on;
    }

    /// デバッグ用: BG1 のタイルマップ／タイルベースアドレスを取得
    pub fn dbg_bg1_bases(&self) -> (u16, u16) {
        (self.bg1_tilemap_base, self.bg1_tile_base)
    }

    /// デバッグ用: VRAM 関連レジスタを取得
    pub fn dbg_vram_regs(&self) -> (u16, u16, u8) {
        (self.vram_addr, self.vram_increment, self.vram_mapping)
    }

    // Raw memory accessors (headless debug dump)
    #[allow(dead_code)]
    pub fn get_vram(&self) -> &[u8] {
        &self.vram
    }

    #[allow(dead_code)]
    pub fn get_cgram(&self) -> &[u8] {
        &self.cgram
    }

    #[allow(dead_code)]
    pub fn get_oam(&self) -> &[u8] {
        &self.oam
    }

    // Convenience dumps (head portion) for debugging
    pub fn dump_vram_head(&self, n: usize) -> Vec<u8> {
        let cnt = n.min(self.vram.len());
        self.vram[..cnt].to_vec()
    }

    pub fn dump_cgram_head(&self, n: usize) -> Vec<u16> {
        let mut out = Vec::new();
        let cnt = n.min(16).min(self.cgram.len() / 2);
        for i in 0..cnt {
            let lo = self.cgram[i * 2] as u16;
            let hi = self.cgram[i * 2 + 1] as u16;
            out.push((hi << 8) | lo);
        }
        out
    }

    pub fn dump_oam_head(&self, n: usize) -> Vec<u8> {
        let cnt = n.min(self.oam.len());
        self.oam[..cnt].to_vec()
    }

    #[allow(dead_code)]
    pub fn get_subscreen_buffer(&self) -> &[u32] {
        &self.subscreen_buffer
    }

    pub fn is_forced_blank(&self) -> bool {
        (self.screen_display & 0x80) != 0
    }

    pub fn current_brightness(&self) -> u8 {
        self.brightness & 0x0F
    }

    pub fn get_main_screen_designation(&self) -> u8 {
        self.main_screen_designation
    }

    pub fn get_bg_mode(&self) -> u8 {
        self.bg_mode
    }

    // Dump first n colors from CGRAM as 15-bit BGR
    // --- Save state serialization ---
    pub fn to_save_state(&self) -> crate::savestate::PpuSaveState {
        use crate::savestate::PpuSaveState;
        let mut st = PpuSaveState {
            scanline: self.scanline,
            dot: self.cycle,
            frame_count: self.frame,
            vblank: self.v_blank,
            hblank: self.h_blank,
            brightness: self.brightness,
            forced_blank: (self.screen_display & 0x80) != 0,
            nmi_enabled: self.nmi_enabled,
            nmi_pending: self.nmi_flag,
            rdnmi_read_in_vblank: self.rdnmi_read_in_vblank,
            bg_mode: self.bg_mode,
            mosaic_size: self.mosaic_size,
            ..Default::default()
        };
        // BG enable is approximated from TM register mirror (main_screen_designation)
        for i in 0..4 {
            st.bg_enabled[i] = (self.effective_main_screen_designation() & (1 << i)) != 0;
            st.bg_priority[i] = 0; // priority not explicitly tracked; leave 0
            st.bg_scroll_x[i] = match i {
                0 => self.bg1_hscroll,
                1 => self.bg2_hscroll,
                2 => self.bg3_hscroll,
                _ => self.bg4_hscroll,
            };
            st.bg_scroll_y[i] = match i {
                0 => self.bg1_vscroll,
                1 => self.bg2_vscroll,
                2 => self.bg3_vscroll,
                _ => self.bg4_vscroll,
            };
            st.bg_tilemap_address[i] = match i {
                0 => self.bg1_tilemap_base,
                1 => self.bg2_tilemap_base,
                2 => self.bg3_tilemap_base,
                _ => self.bg4_tilemap_base,
            };
            st.bg_character_address[i] = match i {
                0 => self.bg1_tile_base,
                1 => self.bg2_tile_base,
                2 => self.bg3_tile_base,
                _ => self.bg4_tile_base,
            };
        }
        st.vram = self.vram.clone();
        st.cgram = self.cgram.clone();
        st.oam = self.oam.clone();
        st.vram_address = self.vram_addr;
        st.vram_increment = self.vram_increment;
        st.cgram_address = self.cgram_addr;
        st.oam_address = self.oam_addr;
        st
    }

    pub fn load_from_save_state(&mut self, st: &crate::savestate::PpuSaveState) {
        self.scanline = st.scanline;
        self.cycle = st.dot;
        self.frame = st.frame_count;
        self.v_blank = st.vblank;
        self.h_blank = st.hblank;
        self.brightness = st.brightness;
        if st.forced_blank {
            self.screen_display |= 0x80;
        } else {
            self.screen_display &= !0x80;
        }
        self.nmi_enabled = st.nmi_enabled;
        self.nmi_flag = st.nmi_pending;
        self.rdnmi_read_in_vblank = st.rdnmi_read_in_vblank;
        self.bg_mode = st.bg_mode;
        self.mosaic_size = st.mosaic_size;
        for i in 0..4 {
            if st.bg_enabled[i] {
                self.main_screen_designation |= 1 << i;
            } else {
                self.main_screen_designation &= !(1 << i);
            }
            match i {
                0 => {
                    self.bg1_hscroll = st.bg_scroll_x[0];
                    self.bg1_vscroll = st.bg_scroll_y[0];
                    self.bg1_tilemap_base = st.bg_tilemap_address[0];
                    self.bg1_tile_base = st.bg_character_address[0];
                }
                1 => {
                    self.bg2_hscroll = st.bg_scroll_x[1];
                    self.bg2_vscroll = st.bg_scroll_y[1];
                    self.bg2_tilemap_base = st.bg_tilemap_address[1];
                    self.bg2_tile_base = st.bg_character_address[1];
                }
                2 => {
                    self.bg3_hscroll = st.bg_scroll_x[2];
                    self.bg3_vscroll = st.bg_scroll_y[2];
                    self.bg3_tilemap_base = st.bg_tilemap_address[2];
                    self.bg3_tile_base = st.bg_character_address[2];
                }
                _ => {
                    self.bg4_hscroll = st.bg_scroll_x[3];
                    self.bg4_vscroll = st.bg_scroll_y[3];
                    self.bg4_tilemap_base = st.bg_tilemap_address[3];
                    self.bg4_tile_base = st.bg_character_address[3];
                }
            }
        }
        if self.vram.len() == st.vram.len() {
            self.vram.copy_from_slice(&st.vram);
        }
        if self.cgram.len() == st.cgram.len() {
            self.cgram.copy_from_slice(&st.cgram);
        }
        if self.oam.len() == st.oam.len() {
            self.oam.copy_from_slice(&st.oam);
        }
        self.vram_addr = st.vram_address;
        self.vram_increment = st.vram_increment;
        self.cgram_addr = st.cgram_address;
        self.oam_addr = st.oam_address;
    }

    // Headless init counters summary
    pub fn get_init_counters(&self) -> (u32, u64, u64, u64, u64) {
        (
            self.important_writes_count,
            self.vram_writes_total_low,
            self.vram_writes_total_high,
            self.cgram_writes_total,
            self.oam_writes_total,
        )
    }

    // Summarize and reset OBJ timing metrics (for headless logs)
    pub fn take_obj_summary(&mut self) -> String {
        let ov = self.obj_overflow_lines;
        let to = self.obj_time_over_lines;
        self.obj_overflow_lines = 0;
        self.obj_time_over_lines = 0;
        format!("OBJ: overflow_lines={} time_over_lines={}", ov, to)
    }

    // Build per-line OBJ pipeline: pick first 32 overlapping sprites and precompute tile starts
    fn prepare_line_obj_pipeline(&mut self, scanline: u16) {
        self.line_sprites.clear();
        self.sprite_overflow = false;
        self.sprite_time_over = false;
        // We currently do not emulate the partial-tile fetch artifact precisely; keep rendering ungated.
        self.sprite_timeover_stop_x = 256;

        // Gather sprites in rotated OAM order (starting at oam_eval_base), cap at 32 like hardware.
        //
        // SNESdev/Super Famicom wiki behavior:
        // - Range over (bit6): set if there are >32 sprites on a scanline, but *off-screen* sprites do not count.
        //   Only sprites with -size < X < 256 are considered in the range test.
        // - Time over (bit7): set if there are >34 8x8 tiles on a scanline, evaluated from the 32-range list.
        //   Only tiles with -8 < X < 256 are counted.
        // - Special case: if X == -256 (raw 256), it is treated as X == 0 for range/time evaluation.
        //
        // We approximate this by applying the horizontal inclusion rules and counting tiles across.
        let mut count_seen = 0u8;
        let mut in_range_total = 0u16;
        for n in 0..128u16 {
            let i = ((self.oam_eval_base as u16 + n) & 0x7F) as usize;
            let oam_offset = i * 4;
            if oam_offset + 3 >= self.oam.len() {
                break;
            }
            let y = self.oam[oam_offset + 1];
            // Y>=240 is hidden.
            if y >= 240 {
                continue;
            }
            // Determine size
            let high_table_offset = 0x200 + (i / 4);
            if high_table_offset >= self.oam.len() {
                break;
            }
            let high_table_byte = self.oam[high_table_offset];
            let bit_shift = (i % 4) * 2;
            let high_bits = (high_table_byte >> bit_shift) & 0x03;
            let size_bit = (high_bits & 0x02) != 0;
            let size = if size_bit {
                SpriteSize::Large
            } else {
                SpriteSize::Small
            };
            let (sprite_w, sprite_h) = self.get_sprite_pixel_size(&size);
            // Y is 8-bit and wraps; test overlap via wrapped subtraction.
            let dy = (scanline as u8).wrapping_sub(y);
            if (dy as u16) >= sprite_h as u16 {
                continue;
            }

            // Range stage horizontal inclusion: -size < X < 256 (X is signed 9-bit).
            let x_lo = self.oam[oam_offset] as u16;
            let x_raw = x_lo | (((high_bits & 0x01) as u16) << 8);
            let mut x = Self::sprite_x_signed(x_raw);
            // Bug: treat X == -256 as X == 0 for range/time evaluation.
            if x == -256 {
                x = 0;
            }
            // Only partially-on-screen sprites count towards range overflow.
            if x <= -(sprite_w as i16) {
                continue;
            }
            // x is signed (-256..255), so x < 256 always holds; keep the check for clarity.
            if x >= 256 {
                continue;
            }

            in_range_total = in_range_total.saturating_add(1);

            // Pull rest of fields
            let tile_lo = self.oam[oam_offset + 2] as u16;
            let attr = self.oam[oam_offset + 3];
            let x = x_raw;
            let tile = tile_lo | (((attr & 0x01) as u16) << 8);
            let palette = (attr >> 1) & 0x07;
            let priority = (attr >> 4) & 0x03;
            let flip_x = (attr & 0x40) != 0;
            let flip_y = (attr & 0x80) != 0;
            if in_range_total == 33 && crate::debug_flags::trace_burnin_obj() {
                println!(
                    "[BURNIN-OBJ][OVERFLOW33] frame={} line={} idx={} x_raw={} y={} dy={} size={:?}",
                    self.frame,
                    scanline,
                    i,
                    x,
                    y,
                    dy,
                    size
                );
            }

            if count_seen < 32 {
                self.line_sprites.push(SpriteData {
                    x,
                    y,
                    tile,
                    palette,
                    priority,
                    flip_x,
                    flip_y,
                    size,
                });
                count_seen = count_seen.saturating_add(1);
            }
            // Do not break early; keep scanning to detect overflow realistcally
        }

        // Sprite overflow flag/metric
        self.sprite_overflow = in_range_total > 32;
        if self.sprite_overflow {
            if !self.sprite_overflow_latched && crate::debug_flags::trace_burnin_obj() {
                println!(
                    "[BURNIN-OBJ][LATCH] range_over set: frame={} line={} overlapped_total={} oam_base={}",
                    self.frame, scanline, in_range_total, self.oam_eval_base
                );
            }
            self.sprite_overflow_latched = true;
            self.obj_overflow_lines = self.obj_overflow_lines.saturating_add(1);
        }

        // Time-over (tile overflow) evaluation for this scanline.
        // Count up to 34 tiles across the 32-sprite range list; if there are more, latch time over.
        let mut tiles_seen: u16 = 0;
        'time_eval: for s in self.line_sprites.iter().rev() {
            let mut sx = Self::sprite_x_signed(s.x);
            if sx == -256 {
                sx = 0;
            }
            let (w, _h) = self.get_sprite_pixel_size(&s.size);
            let tiles_across = (w as i16) / 8;
            for k in 0..tiles_across {
                let tx = sx + k * 8;
                // Only tiles with -8 < X < 256 are counted.
                if tx <= -8 || tx >= 256 {
                    continue;
                }
                tiles_seen = tiles_seen.saturating_add(1);
                if tiles_seen > 34 {
                    self.sprite_time_over = true;
                    if !self.sprite_time_over_latched && crate::debug_flags::trace_burnin_obj() {
                        println!(
                            "[BURNIN-OBJ][LATCH] time_over set: frame={} line={} tiles_seen={}",
                            self.frame, scanline, tiles_seen
                        );
                    }
                    self.sprite_time_over_latched = true;
                    self.obj_time_over_lines = self.obj_time_over_lines.saturating_add(1);
                    break 'time_eval;
                }
            }
        }
    }

    // Consume time budget on first pixel of each 8px tile; disable OBJ for rest of line when exhausted
    fn update_obj_time_over_at_x(&mut self, x: u16) {
        // Time-over is evaluated per scanline in `prepare_line_obj_pipeline` above.
        // Keep this as a no-op for now (pixel-level gating is not implemented yet).
        let _ = x;
    }

    // Precompute per-x window masks for BG/OBJ and color window (dot gating)
    fn prepare_line_window_luts(&mut self) {
        self.line_window_prepared = true;
        for x in 0..256u16 {
            // Color window
            let wcol = if self.window_color_mask == 0 {
                true
            } else {
                self.evaluate_window_mask(x, self.window_color_mask, self.color_window_logic)
            };
            self.color_window_lut[x as usize] = if wcol { 1 } else { 0 };

            // BG1..BG4 (main/sub)
            for bg in 0..4u8 {
                // main
                let m_enabled = (self.tmw_mask & (1 << bg)) != 0;
                let m_mask = if m_enabled
                    && self.evaluate_window_mask(
                        x,
                        self.window_bg_mask[bg as usize],
                        self.bg_window_logic[bg as usize],
                    ) {
                    1
                } else {
                    0
                };
                self.main_bg_window_lut[bg as usize][x as usize] = m_mask;
                // sub
                let s_enabled = (self.tsw_mask & (1 << bg)) != 0;
                let s_mask = if s_enabled
                    && self.evaluate_window_mask(
                        x,
                        self.window_bg_mask[bg as usize],
                        self.bg_window_logic[bg as usize],
                    ) {
                    1
                } else {
                    0
                };
                self.sub_bg_window_lut[bg as usize][x as usize] = s_mask;
            }

            // OBJ main/sub
            let obj_m_en = (self.tmw_mask & 0x10) != 0;
            let obj_s_en = (self.tsw_mask & 0x10) != 0;
            self.main_obj_window_lut[x as usize] = if obj_m_en
                && self.evaluate_window_mask(x, self.window_obj_mask, self.obj_window_logic)
            {
                1
            } else {
                0
            };
            self.sub_obj_window_lut[x as usize] = if obj_s_en
                && self.evaluate_window_mask(x, self.window_obj_mask, self.obj_window_logic)
            {
                1
            } else {
                0
            };
        }
    }

    // Summarize VRAM writes since last call, including FG mode info. Resets counters.
    pub fn take_vram_write_summary(&mut self) -> String {
        let mut parts: Vec<String> = Vec::new();
        let fg_mode = (self.vram_last_vmain >> 2) & 0x03;
        let inc = match self.vram_last_vmain & 0x03 {
            0 => 1,
            1 => 32,
            _ => 128,
        };
        let inc_on = if (self.vram_last_vmain & 0x80) != 0 {
            "HIGH"
        } else {
            "LOW"
        };
        parts.push(format!(
            "VMAIN fg={} inc={} inc_on_{}",
            fg_mode, inc, inc_on
        ));
        parts.push(format!(
            "writes: low={} high={}",
            self.vram_write_low_count, self.vram_write_high_count
        ));
        // Buckets 0..7 => 0x0000..0x7000 (word address)
        let mut bucket_strs: Vec<String> = Vec::new();
        for i in 0..8 {
            let base = i * 0x1000;
            let cnt = self.vram_write_buckets[i];
            if cnt > 0 {
                bucket_strs.push(format!("{:04X}-{:04X}:{}", base, base + 0x0FFF, cnt));
            }
        }
        if bucket_strs.is_empty() {
            parts.push("buckets: none".to_string());
        } else {
            parts.push(format!("buckets: {}", bucket_strs.join(", ")));
        }

        // Reject counters and concise gap blocks (timing tune)
        parts.push(format!(
            "rejects: vram={} cgram={} oam={}",
            self.vram_rejects, self.cgram_rejects, self.oam_rejects
        ));
        parts.push(format!(
            "gaps: vram={} cgram={} oam={}",
            self.vram_gap_blocks, self.cgram_gap_blocks, self.oam_gap_blocks
        ));

        // Reset counters
        self.vram_write_buckets = [0; 8];
        self.vram_write_low_count = 0;
        self.vram_write_high_count = 0;
        self.vram_rejects = 0;
        self.cgram_rejects = 0;
        self.oam_rejects = 0;
        self.vram_gap_blocks = 0;
        self.cgram_gap_blocks = 0;
        self.oam_gap_blocks = 0;

        parts.join(" | ")
    }

    // Summarize per-frame render metrics and reset counters
    pub fn take_render_metrics_summary(&mut self) -> String {
        if !crate::debug_flags::render_metrics() {
            return "RENDER_METRICS: off".to_string();
        }
        let s = format!(
            "RENDER_METRICS: clip_in={} clip_out={} add={} add/2={} sub={} sub/2={} masked_bg={} masked_obj={} obj_add={} obj_add/2={} obj_sub={} obj_sub/2={} obj_clip_in={} obj_clip_out={} win_xor={} win_xnor={} math_blocked={} math_blocked_obj={} math_blocked_bd={} m7_wrap={} m7_clip={} m7_fill={} m7_bg1={} m7_bg2={} m7_edge={}",
            self.dbg_clip_inside,
            self.dbg_clip_outside,
            self.dbg_math_add,
            self.dbg_math_add_half,
            self.dbg_math_sub,
            self.dbg_math_sub_half,
            self.dbg_masked_bg,
            self.dbg_masked_obj,
            self.dbg_math_obj_add,
            self.dbg_math_obj_add_half,
            self.dbg_math_obj_sub,
            self.dbg_math_obj_sub_half,
            self.dbg_clip_obj_inside,
            self.dbg_clip_obj_outside,
            self.dbg_win_xor_applied,
            self.dbg_win_xnor_applied,
            self.dbg_math_blocked,
            self.dbg_math_blocked_obj,
            self.dbg_math_blocked_backdrop,
            self.dbg_m7_wrap,
            self.dbg_m7_clip,
            self.dbg_m7_fill,
            self.dbg_m7_bg1,
            self.dbg_m7_bg2,
            self.dbg_m7_edge
        );
        self.dbg_clip_inside = 0;
        self.dbg_clip_outside = 0;
        self.dbg_math_add = 0;
        self.dbg_math_add_half = 0;
        self.dbg_math_sub = 0;
        self.dbg_math_sub_half = 0;
        self.dbg_masked_bg = 0;
        self.dbg_masked_obj = 0;
        self.dbg_math_obj_add = 0;
        self.dbg_math_obj_add_half = 0;
        self.dbg_math_obj_sub = 0;
        self.dbg_math_obj_sub_half = 0;
        self.dbg_clip_obj_inside = 0;
        self.dbg_clip_obj_outside = 0;
        self.dbg_win_xor_applied = 0;
        self.dbg_win_xnor_applied = 0;
        self.dbg_math_blocked = 0;
        self.dbg_math_blocked_obj = 0;
        self.dbg_math_blocked_backdrop = 0;
        self.dbg_m7_wrap = 0;
        self.dbg_m7_clip = 0;
        self.dbg_m7_fill = 0;
        self.dbg_m7_bg1 = 0;
        self.dbg_m7_bg2 = 0;
        self.dbg_m7_edge = 0;
        s
    }

    // カラー演算機能
    #[allow(dead_code)]
    fn apply_color_math(&self, main_color: u32, layer_id: u8) -> u32 {
        if !self.is_color_math_enabled(layer_id) {
            return main_color;
        }
        // Select subsource: CGWSEL bit1 (1=subscreen, 0=fixed)
        let sub_color = self.fixed_color_to_rgb();
        // Use CGADSUB for add/sub + halve
        let is_addition = (self.cgadsub & 0x80) == 0;
        let halve = (self.cgadsub & 0x40) != 0;
        self.blend_colors(main_color, sub_color, is_addition, halve)
    }

    fn is_color_math_enabled(&self, layer_id: u8) -> bool {
        // レイヤーIDに対応するビットをチェック
        let bit_mask = match layer_id {
            0 => 0x01, // BG1
            1 => 0x02, // BG2
            2 => 0x04, // BG3
            3 => 0x08, // BG4
            4 => 0x10, // Sprite
            5 => 0x20, // Backdrop
            _ => return false,
        };
        // Mode 1 EXTBG: BG3 のカラー演算を強制有効（簡易）
        if self.bg_mode == 1 && self.extbg && layer_id == 2 {
            return true;
        }
        (self.cgadsub & bit_mask) != 0
    }

    fn fixed_color_to_rgb(&self) -> u32 {
        let r = (self.fixed_color & 0x1F) as u8;
        let g = ((self.fixed_color >> 5) & 0x1F) as u8;
        let b = ((self.fixed_color >> 10) & 0x1F) as u8;

        // 5bitから8bitに拡張
        let r = (r << 3) | (r >> 2);
        let g = (g << 3) | (g >> 2);
        let b = (b << 3) | (b >> 2);

        ((r as u32) << 16) | ((g as u32) << 8) | (b as u32) | 0xFF000000
    }

    fn blend_colors(&self, color1: u32, color2: u32, is_addition: bool, halve: bool) -> u32 {
        // Work in 5-bit space to better match SNES BGR555 math, then expand to 8-bit.
        let r1 = ((color1 >> 16) & 0xFF) as u16 >> 3;
        let g1 = ((color1 >> 8) & 0xFF) as u16 >> 3;
        let b1 = (color1 & 0xFF) as u16 >> 3;

        let r2 = ((color2 >> 16) & 0xFF) as u16 >> 3;
        let g2 = ((color2 >> 8) & 0xFF) as u16 >> 3;
        let b2 = (color2 & 0xFF) as u16 >> 3;

        let (mut r, mut g, mut b) = if is_addition {
            // 5-bit saturating add
            let r = (r1 + r2).min(31);
            let g = (g1 + g2).min(31);
            let b = (b1 + b2).min(31);
            (r, g, b)
        } else {
            // 5-bit saturating sub
            let r = r1.saturating_sub(r2);
            let g = g1.saturating_sub(g2);
            let b = b1.saturating_sub(b2);
            (r, g, b)
        };

        if halve {
            r >>= 1;
            g >>= 1;
            b >>= 1;
        }

        // Expand back to 8-bit (x<<3 | x>>2 format)
        let r8 = ((r as u32) << 3) | ((r as u32) >> 2);
        let g8 = ((g as u32) << 3) | ((g as u32) >> 2);
        let b8 = ((b as u32) << 3) | ((b as u32) >> 2);
        0xFF000000 | (r8 << 16) | (g8 << 8) | b8
    }

    // モザイク効果
    fn apply_mosaic(&self, x: u16, y: u16, bg_num: u8) -> (u16, u16) {
        // 該当BGでモザイクが有効かチェック
        if !self.is_mosaic_enabled(bg_num) {
            return (x, y);
        }

        // モザイクブロックの左上の座標を計算
        let mosaic_x = (x / self.mosaic_size as u16) * self.mosaic_size as u16;
        let mosaic_y = (y / self.mosaic_size as u16) * self.mosaic_size as u16;

        (mosaic_x, mosaic_y)
    }

    fn is_mosaic_enabled(&self, bg_num: u8) -> bool {
        // BG別のモザイク有効フラグをチェック
        match bg_num {
            0 => self.bg_mosaic & 0x01 != 0, // BG1
            1 => self.bg_mosaic & 0x02 != 0, // BG2
            2 => self.bg_mosaic & 0x04 != 0, // BG3
            3 => self.bg_mosaic & 0x08 != 0, // BG4
            _ => false,
        }
    }

    // Mode 7変換
    #[allow(dead_code)]
    fn mode7_transform(&self, screen_x: u16, screen_y: u16) -> (i32, i32) {
        // 画面座標を中心基準に変換
        let sx = screen_x as i32 - 128;
        let sy = screen_y as i32 - 128;

        // 回転中心からの相対座標
        let rel_x = sx - (self.mode7_center_x >> 8) as i32;
        let rel_y = sy - (self.mode7_center_y >> 8) as i32;

        // 変換行列適用 (固定小数点演算)
        let a = self.mode7_matrix_a as i32;
        let b = self.mode7_matrix_b as i32;
        let c = self.mode7_matrix_c as i32;
        let d = self.mode7_matrix_d as i32;

        let transformed_x = ((a * rel_x + b * rel_y) >> 8) + (self.mode7_center_x >> 8) as i32;
        let transformed_y = ((c * rel_x + d * rel_y) >> 8) + (self.mode7_center_y >> 8) as i32;

        (transformed_x, transformed_y)
    }

    // 8.8 fixed math producing integer world pixels (includes proper center/origin compensation)
    fn mode7_world_xy_int(&self, sx: i32, sy: i32) -> (i32, i32) {
        // Promote to i64 to avoid overflow in worst-case affine products
        let a = self.mode7_matrix_a as i64;
        let b = self.mode7_matrix_b as i64;
        let c = self.mode7_matrix_c as i64;
        let d = self.mode7_matrix_d as i64;
        let cx = self.mode7_center_x as i64; // 8.8
        let cy = self.mode7_center_y as i64; // 8.8
        let xs = (sx as i64) << 8; // 8.8
        let ys = (sy as i64) << 8; // 8.8

        // Proper SNES Mode 7 compensation:
        // Xw = A*(Xs - Cx) + B*(Ys - Cy) + H*256 + A*Cx + B*Cy
        // Yw = C*(Xs - Cx) + D*(Ys - Cy) + V*256 + C*Cx + D*Cy
        let hx = (self.bg1_hscroll as i64) << 8;
        let vy = (self.bg1_vscroll as i64) << 8;

        let dx = xs - cx;
        let dy = ys - cy;
        // First multiply still 8.8; sum of terms stays 16.8 then >>8 to integer pixels
        let xw = a * dx + b * dy + hx + a * cx + b * cy;
        let yw = c * dx + d * dy + vy + c * cx + d * cy;
        (Self::fixed8_floor(xw), Self::fixed8_floor(yw))
    }

    // メインスクリーン描画（レイヤID付き）
    fn render_main_screen_pixel_with_layer(&mut self, x: u16, y: u16) -> (u32, u8) {
        // BGとスプライトの情報を取得
        let (bg_color, bg_priority, bg_id) = self.get_main_bg_pixel(x, y);
        let (sprite_color, sprite_priority) = self.get_sprite_pixel(x, y);

        // プライオリティベースの合成（レイヤIDも取得）
        let (final_color, layer_id) = self.composite_pixel_with_layer(
            bg_color,
            bg_priority,
            bg_id,
            sprite_color,
            sprite_priority,
        );

        // NOTE: This function is called per pixel, so keep debug work behind flags.
        if crate::debug_flags::render_verbose() || crate::debug_flags::debug_graphics_detected() {
            // Debug rendering for title screen detection / sanity checks
            static mut RENDER_DEBUG_COUNT: u32 = 0;
            static mut NON_BLACK_PIXELS: u32 = 0;
            static mut GRAPHICS_DETECTED_PRINTS: u32 = 0;

            unsafe {
                RENDER_DEBUG_COUNT = RENDER_DEBUG_COUNT.saturating_add(1);
                if final_color != 0xFF000000 {
                    NON_BLACK_PIXELS = NON_BLACK_PIXELS.saturating_add(1);
                }

                // Debug: Show coordinate info for specific pixels
                if crate::debug_flags::render_verbose() && x == 10 && y == 10 {
                    static mut PIXEL_10_10_SHOWN: bool = false;
                    if !PIXEL_10_10_SHOWN && !crate::debug_flags::quiet() {
                        println!(
                            "🎯 PIXEL (10,10): bg_color=0x{:06X}, final_color=0x{:06X}, layer={}",
                            bg_color, final_color, layer_id
                        );
                        PIXEL_10_10_SHOWN = true;
                    }
                }

                // Report rendering activity periodically
                if crate::debug_flags::render_verbose()
                    && !crate::debug_flags::quiet()
                    && RENDER_DEBUG_COUNT.is_multiple_of(100000)
                {
                    println!(
                        "🖼️  RENDER STATS: {} pixels rendered, {} non-black ({:.1}%)",
                        RENDER_DEBUG_COUNT,
                        NON_BLACK_PIXELS,
                        (NON_BLACK_PIXELS as f32 / RENDER_DEBUG_COUNT as f32) * 100.0
                    );
                }

                // Detect first non-black pixel (debug-only; intentionally off by default)
                if crate::debug_flags::debug_graphics_detected()
                    && !crate::debug_flags::quiet()
                    && final_color != 0xFF000000
                    && GRAPHICS_DETECTED_PRINTS == 0
                {
                    GRAPHICS_DETECTED_PRINTS = 1;
                    println!(
                        "🎨 GRAPHICS DETECTED: first non-black pixel 0x{:08X} at ({}, {}) layer={}",
                        final_color, x, y, layer_id
                    );
                }
            }
        }

        (final_color, layer_id)
    }

    // メインスクリーン用BGの最前面色とその優先度を取得
    fn get_main_bg_pixel(&mut self, x: u16, y: u16) -> (u32, u8, u8) {
        let bg_layers = self.get_main_bg_layers(x, y);
        bg_layers
            .into_iter()
            .filter(|(c, _, _)| *c != 0)
            .max_by_key(|(_, p, n)| (self.z_rank_for_bg(*n, *p), *n))
            .unwrap_or((0, 0, 0))
    }

    // サブスクリーン描画
    #[allow(dead_code)]
    fn render_sub_screen_pixel(&mut self, x: u16, y: u16) -> u32 {
        let (bg_color, bg_priority, bg_id) = self.get_sub_bg_pixel(x, y);
        let (sprite_color, sprite_priority) = self.get_sub_sprite_pixel(x, y);

        let (final_color, _lid) = self.composite_pixel_with_layer(
            bg_color,
            bg_priority,
            bg_id,
            sprite_color,
            sprite_priority,
        );

        if final_color != 0 {
            final_color
        } else {
            // When the entire sub-screen is transparent, real hardware uses the fixed
            // color ($2132) instead of CGRAM color 0. This matters for color math
            // tricks used by the official burn-in test suite (OBJTEST expects a white
            // background via fixed color).
            self.fixed_color_to_rgb()
        }
    }

    // サブスクリーン描画（レイヤID付き）
    fn render_sub_screen_pixel_with_layer(&mut self, x: u16, y: u16) -> (u32, u8) {
        let (bg_color, bg_priority, bg_id) = self.get_sub_bg_pixel(x, y);
        let (sprite_color, sprite_priority) = self.get_sub_sprite_pixel(x, y);
        let (final_color, layer_id) = self.composite_pixel_with_layer(
            bg_color,
            bg_priority,
            bg_id,
            sprite_color,
            sprite_priority,
        );
        if final_color != 0 {
            (final_color, layer_id)
        } else {
            // Sub-screen backdrop is the fixed color ($2132), not CGRAM[0].
            (self.fixed_color_to_rgb(), 5)
        }
    }

    // サブスクリーン用BG描画（メインと同等のモード対応）
    fn get_sub_bg_pixel(&mut self, x: u16, y: u16) -> (u32, u8, u8) {
        let mut bg_results = Vec::new();

        match self.bg_mode {
            0 => {
                if self.sub_screen_designation & 0x01 != 0 && !self.should_mask_bg(x, 0, false) {
                    let (color, priority) = self.render_bg_mode0(x, y, 0);
                    if color != 0 {
                        bg_results.push((color, priority, 0));
                    }
                }
                if self.sub_screen_designation & 0x02 != 0 && !self.should_mask_bg(x, 1, false) {
                    let (color, priority) = self.render_bg_mode0(x, y, 1);
                    if color != 0 {
                        bg_results.push((color, priority, 1));
                    }
                }
                if self.sub_screen_designation & 0x04 != 0 && !self.should_mask_bg(x, 2, false) {
                    let (color, priority) = self.render_bg_mode0(x, y, 2);
                    if color != 0 {
                        bg_results.push((color, priority, 2));
                    }
                }
                if self.sub_screen_designation & 0x08 != 0 && !self.should_mask_bg(x, 3, false) {
                    let (color, priority) = self.render_bg_mode0(x, y, 3);
                    if color != 0 {
                        bg_results.push((color, priority, 3));
                    }
                }
            }
            1 => {
                if self.sub_screen_designation & 0x01 != 0 && !self.should_mask_bg(x, 0, false) {
                    let (color, priority) = self.render_bg_4bpp(x, y, 0);
                    if color != 0 {
                        bg_results.push((color, priority, 0));
                    }
                }
                if self.sub_screen_designation & 0x02 != 0 && !self.should_mask_bg(x, 1, false) {
                    let (color, priority) = self.render_bg_4bpp(x, y, 1);
                    if color != 0 {
                        bg_results.push((color, priority, 1));
                    }
                }
                if self.sub_screen_designation & 0x04 != 0 && !self.should_mask_bg(x, 2, false) {
                    let (color, priority) = self.render_bg_mode0(x, y, 2);
                    if color != 0 {
                        bg_results.push((color, priority, 2));
                    }
                }
            }
            2 => {
                if self.sub_screen_designation & 0x01 != 0 && !self.should_mask_bg(x, 0, false) {
                    let (color, priority) = self.render_bg_mode2(x, y, 0);
                    if color != 0 {
                        bg_results.push((color, priority, 0));
                    }
                }
                if self.sub_screen_designation & 0x02 != 0 && !self.should_mask_bg(x, 1, false) {
                    let (color, priority) = self.render_bg_mode2(x, y, 1);
                    if color != 0 {
                        bg_results.push((color, priority, 1));
                    }
                }
            }
            3 => {
                if self.sub_screen_designation & 0x01 != 0 && !self.should_mask_bg(x, 0, false) {
                    let (color, priority) = self.render_bg_8bpp(x, y, 0);
                    if color != 0 {
                        bg_results.push((color, priority, 0));
                    }
                }
                if self.sub_screen_designation & 0x02 != 0 && !self.should_mask_bg(x, 1, false) {
                    let (color, priority) = self.render_bg_4bpp(x, y, 1);
                    if color != 0 {
                        bg_results.push((color, priority, 1));
                    }
                }
            }
            4 => {
                if self.sub_screen_designation & 0x01 != 0 && !self.should_mask_bg(x, 0, false) {
                    let (color, priority) = self.render_bg_8bpp(x, y, 0);
                    if color != 0 {
                        bg_results.push((color, priority, 0));
                    }
                }
                if self.sub_screen_designation & 0x02 != 0 && !self.should_mask_bg(x, 1, false) {
                    let (color, priority) = self.render_bg_mode0(x, y, 1);
                    if color != 0 {
                        bg_results.push((color, priority, 1));
                    }
                }
            }
            5 => {
                if self.sub_screen_designation & 0x01 != 0 && !self.should_mask_bg(x, 0, false) {
                    let (color, priority) = self.render_bg_mode5(x, y, 0, false);
                    if color != 0 {
                        bg_results.push((color, priority, 0));
                    }
                }
                if self.sub_screen_designation & 0x02 != 0 && !self.should_mask_bg(x, 1, false) {
                    let (color, priority) = self.render_bg_mode5(x, y, 1, false);
                    if color != 0 {
                        bg_results.push((color, priority, 1));
                    }
                }
            }
            6 => {
                if self.sub_screen_designation & 0x01 != 0 && !self.should_mask_bg(x, 0, false) {
                    let (color, priority) = self.render_bg_mode6(x, y, 0, false);
                    if color != 0 {
                        bg_results.push((color, priority, 0));
                    }
                }
            }
            7 => {
                let (c, p, lid) = self.render_mode7_with_layer(x, y);
                if c != 0 {
                    let id = if self.extbg { lid } else { 0 };
                    let en_bit = 1u8 << id;
                    if (self.sub_screen_designation & en_bit) != 0
                        && !self.should_mask_bg(x, id, false)
                    {
                        bg_results.push((c, p, id));
                    }
                }
            }
            _ => {}
        }

        // 最も高いプライオリティのBGを返す
        bg_results
            .into_iter()
            .max_by_key(|(_, p, n)| (self.z_rank_for_bg(*n, *p), *n))
            .unwrap_or((0, 0, 0))
    }

    // サブスクリーン用スプライト描画（簡易版）
    fn get_sub_sprite_pixel(&self, x: u16, y: u16) -> (u32, u8) {
        let enabled = (self.sub_screen_designation & 0x10) != 0;
        self.get_sprite_pixel_common(x, y, enabled, false)
    }

    // メイン・サブスクリーン間のカラー演算（対象レイヤ限定の簡易版）
    fn apply_color_math_screens(
        &mut self,
        main_color_in: u32,
        sub_color_in: u32,
        main_layer_id: u8,
        x: u16,
    ) -> u32 {
        // Forced blank produces black regardless of color math (unless FORCE_DISPLAY)
        if (self.screen_display & 0x80) != 0 && !self.force_display_active() {
            return 0;
        }

        // Determine color window W(x) for color math/force black gating
        let win = if self.line_window_prepared {
            self.color_window_lut.get(x as usize).copied().unwrap_or(0) != 0
        } else if self.window_color_mask == 0 {
            true
        } else {
            self.evaluate_window_mask(x, self.window_color_mask, self.color_window_logic)
        };
        let math_mode = (self.cgwsel >> 4) & 0x03; // CGWSEL bits 5-4
        let black_mode = (self.cgwsel >> 6) & 0x03; // CGWSEL bits 7-6
                                                    // Map per SNES: math_mode 0=always,1=inside,2=outside,3=never
        let gate_math = |mode: u8, w: bool| -> bool {
            match mode {
                0 => true,
                1 => w,
                2 => !w,
                _ => false,
            }
        };
        // Black clip: 0=never,1=inside,2=outside,3=always
        let gate_black = |mode: u8, w: bool| -> bool {
            match mode {
                0 => false,
                1 => w,
                2 => !w,
                _ => true,
            }
        };

        // Force black clip is a hard override: return black and skip math entirely
        // But FORCE_DISPLAY debug flag should bypass this (like it bypasses forced blank)
        if gate_black(black_mode, win) && !self.force_display_active() {
            if crate::debug_flags::render_metrics() {
                if (self.cgwsel >> 6) & 0x03 == 1 {
                    self.dbg_clip_inside = self.dbg_clip_inside.saturating_add(1);
                } else if (self.cgwsel >> 6) & 0x03 == 2 {
                    self.dbg_clip_outside = self.dbg_clip_outside.saturating_add(1);
                }
                if main_layer_id == 4 {
                    if (self.cgwsel >> 6) & 0x03 == 1 {
                        self.dbg_clip_obj_inside = self.dbg_clip_obj_inside.saturating_add(1);
                    } else if (self.cgwsel >> 6) & 0x03 == 2 {
                        self.dbg_clip_obj_outside = self.dbg_clip_obj_outside.saturating_add(1);
                    }
                }
            }
            return 0;
        }

        let main_color = main_color_in;

        // Subsource is only used for color math; transparent main becomes backdrop earlier
        let use_sub_src = (self.cgwsel & 0x02) != 0; // 1=subscreen, 0=fixed
        let sub_src = if use_sub_src {
            sub_color_in
        } else {
            self.fixed_color_to_rgb()
        };

        // Color math enable gating via window
        if !gate_math(math_mode, win) {
            return main_color;
        }
        // このメインレイヤにカラー演算が許可されているか
        if !self.is_color_math_enabled(main_layer_id) {
            if crate::debug_flags::render_metrics() {
                self.dbg_math_blocked = self.dbg_math_blocked.saturating_add(1);
                if main_layer_id == 4 {
                    self.dbg_math_blocked_obj = self.dbg_math_blocked_obj.saturating_add(1);
                }
                if main_layer_id == 5 {
                    self.dbg_math_blocked_backdrop =
                        self.dbg_math_blocked_backdrop.saturating_add(1);
                }
            }
            return main_color;
        }

        let src = sub_src;
        let src_is_fixed = !use_sub_src;
        // Use CGADSUB for add/sub + halve
        let is_addition = (self.cgadsub & 0x80) == 0;
        let halve_flag = (self.cgadsub & 0x40) != 0;
        // In pseudo-hires, when actually blending main and sub (not fixed color),
        // halve the result to approximate 512px brightness. Skip when src is fixed color.
        let hires_halve =
            self.pseudo_hires && use_sub_src && !src_is_fixed && main_color != 0 && src != 0;
        let effective_halve = halve_flag || hires_halve;
        let out = self.blend_colors(main_color, src, is_addition, effective_halve);
        if crate::debug_flags::render_metrics() {
            if is_addition {
                if effective_halve {
                    self.dbg_math_add_half += 1;
                    if main_layer_id == 4 {
                        self.dbg_math_obj_add_half += 1;
                    }
                } else {
                    self.dbg_math_add += 1;
                    if main_layer_id == 4 {
                        self.dbg_math_obj_add += 1;
                    }
                }
            } else if effective_halve {
                self.dbg_math_sub_half += 1;
                if main_layer_id == 4 {
                    self.dbg_math_obj_sub_half += 1;
                }
            } else {
                self.dbg_math_sub += 1;
                if main_layer_id == 4 {
                    self.dbg_math_obj_sub += 1;
                }
            }
        }
        out
    }

    pub fn nmi_pending(&self) -> bool {
        // CPU側へ通知するNMIリクエストは「ラッチ」(edge)で管理する。
        // nmi_flag は $4210(RDNMI) のbit7用で、読み出しでクリアされる。
        self.nmi_enabled && self.nmi_latched
    }

    // Expose minimal NMI latch control for $4200 edge cases
    pub fn is_nmi_latched(&self) -> bool {
        self.nmi_latched
    }
    pub fn latch_nmi_now(&mut self) {
        self.nmi_flag = true;
        self.nmi_latched = true;
    }

    pub fn get_scanline(&self) -> u16 {
        self.scanline
    }

    pub fn get_frame(&self) -> u64 {
        self.frame
    }

    // Accessors for HVB flags
    pub fn is_vblank(&self) -> bool {
        self.v_blank
    }

    pub fn is_hblank(&self) -> bool {
        self.h_blank
    }

    pub fn get_cycle(&self) -> u16 {
        self.cycle
    }

    // --- Write context control (called by Bus before/after DMA bursts) ---
    #[inline]
    pub fn begin_mdma_context(&mut self) {
        self.write_ctx = 1;
    }
    #[inline]
    pub fn end_mdma_context(&mut self) {
        self.write_ctx = 0;
        self.debug_dma_channel = None;
    }
    #[inline]
    pub fn begin_hdma_context(&mut self) {
        self.write_ctx = 2;
    }
    #[inline]
    pub fn end_hdma_context(&mut self) {
        self.write_ctx = 0;
        self.debug_dma_channel = None;
    }

    // Debug helper: mark which DMA channel is currently active
    #[inline]
    pub fn set_debug_dma_channel(&mut self, ch: Option<u8>) {
        self.debug_dma_channel = ch;
    }

    #[inline]
    pub fn arm_burnin_vram_trace(&mut self) {
        self.burnin_vram_trace_armed = true;
        self.burnin_vram_trace_cnt_2118 = 0;
        self.burnin_vram_trace_cnt_2119 = 0;
    }

    // Mark HBlank head guard window for HDMA operations
    pub fn on_hblank_start_guard(&mut self) {
        let hb = self.first_hblank_dot();
        const HDMA_HEAD_GUARD: u16 = 6;
        self.hdma_head_busy_until = hb.saturating_add(HDMA_HEAD_GUARD);
    }

    #[allow(dead_code)]
    pub fn clear_nmi(&mut self) {
        // NMIラッチだけを解除し、RDNMIフラグ（nmi_flag）は保持する。
        // 実機では $4210 読み出しでクリアされるため、CPU側のポーリングに委ねる。
        self.nmi_latched = false;
    }

    // Lightweight usage stats (counts non-zero bytes)
    pub fn vram_usage(&self) -> usize {
        self.vram.iter().filter(|&&b| b != 0).count()
    }

    /// Analyze VRAM content distribution
    pub fn analyze_vram_content(&self) -> (usize, usize, Vec<(usize, u8)>) {
        let mut nonzero_count = 0;
        let mut unique_values = std::collections::HashSet::new();
        let mut samples = Vec::new();

        for (i, &byte) in self.vram.iter().enumerate() {
            if byte != 0 {
                nonzero_count += 1;
                unique_values.insert(byte);
                if samples.len() < 20 {
                    samples.push((i, byte));
                }
            }
        }

        (nonzero_count, unique_values.len(), samples)
    }

    /// Analyze specific VRAM region (word address)
    pub fn analyze_vram_region(&self, word_addr: u16, word_count: usize) -> (usize, Vec<u8>) {
        // Apply VRAM mirroring: addresses 0x8000-0xFFFF mirror to 0x0000-0x7FFF
        let mirrored_addr = word_addr & 0x7FFF;
        let byte_start = (mirrored_addr as usize) * 2;
        let byte_end = (byte_start + word_count * 2).min(self.vram.len());
        let mut nonzero = 0;
        let mut samples = Vec::new();

        for i in byte_start..byte_end {
            if self.vram[i] != 0 {
                nonzero += 1;
                if samples.len() < 16 {
                    samples.push(self.vram[i]);
                }
            }
        }

        (nonzero, samples)
    }

    /// Get VRAM distribution by 4KB blocks
    pub fn get_vram_distribution(&self) -> Vec<(usize, usize)> {
        let block_size = 4096; // 4KB blocks
        let mut distribution = Vec::new();

        for block in 0..(self.vram.len() / block_size) {
            let start = block * block_size;
            let end = (start + block_size).min(self.vram.len());
            let nonzero = self.vram[start..end].iter().filter(|&&b| b != 0).count();
            if nonzero > 0 {
                distribution.push((block * block_size / 2, nonzero)); // word address
            }
        }

        distribution
    }

    pub fn cgram_usage(&self) -> usize {
        self.cgram.iter().filter(|&&b| b != 0).count()
    }

    /// Count non-zero color entries in CGRAM (each color is 2 bytes)
    pub fn count_nonzero_colors(&self) -> usize {
        self.cgram
            .chunks_exact(2)
            .filter(|chunk| chunk[0] != 0 || chunk[1] != 0)
            .count()
    }

    /// Get BG configuration for debugging
    pub fn get_bg_config(&self, bg_num: u8) -> (u16, u16, bool, u8) {
        let index = (bg_num.saturating_sub(1)) as usize;
        if index >= 4 {
            return (0, 0, false, 0);
        }
        let tile_base = match bg_num {
            1 => self.bg1_tile_base,
            2 => self.bg2_tile_base,
            3 => self.bg3_tile_base,
            4 => self.bg4_tile_base,
            _ => 0,
        };
        let tilemap_base = match bg_num {
            1 => self.bg1_tilemap_base,
            2 => self.bg2_tilemap_base,
            3 => self.bg3_tilemap_base,
            4 => self.bg4_tilemap_base,
            _ => 0,
        };
        (
            tile_base,
            tilemap_base,
            self.bg_tile_16[index],
            self.bg_screen_size[index],
        )
    }

    /// Write a 15-bit RGB color to CGRAM at the given color index
    pub fn write_cgram_color(&mut self, color_index: u8, rgb15: u16) {
        let offset = (color_index as usize) * 2;
        if offset + 1 < self.cgram.len() {
            self.cgram[offset] = (rgb15 & 0xFF) as u8;
            self.cgram[offset + 1] = ((rgb15 >> 8) & 0xFF) as u8;
        }
    }

    /// Write tilemap entry directly to VRAM (bypassing timing checks)
    pub fn write_vram_word(&mut self, word_addr: u16, low_byte: u8, high_byte: u8) {
        // VRAM is 32KB words; wrap addresses the way hardware mirrors the 15-bit address.
        let addr = (word_addr as usize) & 0x7FFF; // 15-bit
        let byte_addr = addr * 2;
        if byte_addr + 1 < self.vram.len() {
            self.vram[byte_addr] = low_byte;
            self.vram[byte_addr + 1] = high_byte;
        }
    }

    pub fn oam_usage(&self) -> usize {
        self.oam.iter().filter(|&&b| b != 0).count()
    }

    // デバッグ用：PPU状態を表示
    pub fn debug_ppu_state(&self) {
        println!("\n=== PPU Debug State ===");
        println!(
            "Scanline: {}, Cycle: {}, Frame: {}",
            self.scanline, self.cycle, self.frame
        );
        println!(
            "Mode: {} (BG3prio={}), SETINI=0x{:02X} (pseudo_hires={}, interlace={}, obj_interlace={}, overscan={}, extbg={})",
            self.bg_mode,
            self.mode1_bg3_priority,
            self.setini,
            self.pseudo_hires,
            self.interlace,
            self.obj_interlace,
            self.overscan,
            self.extbg
        );
        println!(
            "Main Screen: 0x{:02X}, Sub Screen: 0x{:02X}",
            self.main_screen_designation, self.sub_screen_designation
        );
        println!(
            "Color Math: CGWSEL=0x{:02X} CGADSUB=0x{:02X} fixed=0x{:04X}",
            self.cgwsel, self.cgadsub, self.fixed_color
        );
        println!("Screen Display: 0x{:02X}", self.screen_display);
        println!("NMI: enabled={}, flag={}", self.nmi_enabled, self.nmi_flag);

        // BGレイヤー設定
        println!(
            "BG1: tilemap=0x{:04X}, tile=0x{:04X}, scroll=({},{})",
            self.bg1_tilemap_base, self.bg1_tile_base, self.bg1_hscroll, self.bg1_vscroll
        );
        println!(
            "BG2: tilemap=0x{:04X}, tile=0x{:04X}, scroll=({},{})",
            self.bg2_tilemap_base, self.bg2_tile_base, self.bg2_hscroll, self.bg2_vscroll
        );
        println!(
            "BG3: tilemap=0x{:04X}, tile=0x{:04X}, scroll=({},{})",
            self.bg3_tilemap_base, self.bg3_tile_base, self.bg3_hscroll, self.bg3_vscroll
        );
        println!(
            "BG4: tilemap=0x{:04X}, tile=0x{:04X}, scroll=({},{})",
            self.bg4_tilemap_base, self.bg4_tile_base, self.bg4_hscroll, self.bg4_vscroll
        );
        println!(
            "BG tile16: [{},{},{},{}] screen_size: [{},{},{},{}]",
            self.bg_tile_16[0],
            self.bg_tile_16[1],
            self.bg_tile_16[2],
            self.bg_tile_16[3],
            self.bg_screen_size[0],
            self.bg_screen_size[1],
            self.bg_screen_size[2],
            self.bg_screen_size[3]
        );

        // スプライト設定
        println!(
            "Sprite: size={}, name_base=0x{:04X}, name_select=0x{:04X}",
            self.sprite_size, self.sprite_name_base, self.sprite_name_select
        );

        // VRAM/CGRAM状態
        let vram_used = self.vram.iter().filter(|&&b| b != 0).count();
        let cgram_used = self.cgram.iter().filter(|&&b| b != 0).count();
        println!(
            "VRAM: {}/{} bytes used, CGRAM: {}/{} bytes used",
            vram_used,
            self.vram.len(),
            cgram_used,
            self.cgram.len()
        );

        // 最初の8個のCGRAMエントリ表示（パレット0）
        print!("Palette 0: ");
        for i in 0..8 {
            let color = self.cgram_to_rgb(i);
            print!("${:06X} ", color & 0xFFFFFF);
        }
        println!();

        println!("=======================");
    }

    // テストパターンを強制表示（デバッグ用）
    pub fn force_test_pattern(&mut self) {
        println!("Forcing test pattern display...");

        // テストパターン表示のため基本的なPPU設定を上書き
        self.brightness = 15;
        self.main_screen_designation = 0x1F; // 全BGレイヤーとスプライトを有効
        self.screen_display = 0; // forced blank off (表示有効)

        // Dragon Quest III fix: Fill VRAM with test data
        for i in 0..0x8000 {
            self.vram[i] = if i < 0x4000 { 0x11 } else { 0x22 };
        }

        // Set up tilemap entries at high addresses (0xE000-0xFFFF range)
        let tilemap_start = 0x6000; // Start from 0xE000 & 0x7FFF = 0x6000
        for i in (tilemap_start..tilemap_start + 0x800).step_by(2) {
            if i + 1 < self.vram.len() {
                self.vram[i] = 0x01; // Tile ID low
                self.vram[i + 1] = 0x00; // Tile ID high + attributes
            }
        }

        // Set up tile data at 0x6000+ range
        let tile_start = 0x4000; // Start from 0xE000 & 0x7FFF = 0x6000
        for i in tile_start..tile_start + 0x100 {
            if i < self.vram.len() {
                self.vram[i] = 0xFF; // White tile data
            }
        }

        // Fill CGRAM with test colors
        // Palette 0: Background colors
        self.cgram[0] = 0x00;
        self.cgram[1] = 0x00; // Color 0: Black (transparent)
        self.cgram[2] = 0xFF;
        self.cgram[3] = 0x7F; // Color 1: White
        self.cgram[4] = 0x1F;
        self.cgram[5] = 0x00; // Color 2: Red
        self.cgram[6] = 0xE0;
        self.cgram[7] = 0x03; // Color 3: Green

        // Palette 1-7: Fill with distinct colors
        for palette in 1..8 {
            let base = palette * 16 * 2;
            for color in 0..16 {
                let addr = base + color * 2;
                if addr + 1 < self.cgram.len() {
                    // Create distinct colors for each palette
                    let r = ((palette * 4) & 0x1F) as u16;
                    let g = ((color * 2) & 0x1F) as u16;
                    let b = ((palette + color) & 0x1F) as u16;
                    let color_val = (b << 10) | (g << 5) | r;
                    self.cgram[addr] = (color_val & 0xFF) as u8;
                    self.cgram[addr + 1] = ((color_val >> 8) & 0x7F) as u8;
                }
            }
        }

        println!(
            "PPU: Test pattern applied (brightness={}, layers=0x{:02X}) with VRAM test data",
            self.brightness, self.main_screen_designation
        );
    }
}

// --------------------------- tests ---------------------------
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cgram_rgb555_to_rgb888_mapping() {
        let mut ppu = Ppu::new();
        // RGB555 (SNES): bit0-4=R, 5-9=G, 10-14=B.
        ppu.write_cgram_color(0, 0x001F); // red
        ppu.write_cgram_color(1, 0x03E0); // green
        ppu.write_cgram_color(2, 0x7C00); // blue
        ppu.write_cgram_color(3, 0x7FFF); // white

        assert_eq!(ppu.cgram_to_rgb(0), 0xFFFF0000);
        assert_eq!(ppu.cgram_to_rgb(1), 0xFF00FF00);
        assert_eq!(ppu.cgram_to_rgb(2), 0xFF0000FF);
        assert_eq!(ppu.cgram_to_rgb(3), 0xFFFFFFFF);
    }

    #[test]
    fn coldata_updates_fixed_color_components() {
        let mut ppu = Ppu::new();
        // Set R=31, G=0, B=0
        ppu.write(0x32, 0x20 | 0x1F); // R enable + intensity
        ppu.write(0x32, 0x40 | 0x00); // G enable + intensity
        ppu.write(0x32, 0x80 | 0x00); // B enable + intensity
        assert_eq!(ppu.fixed_color_to_rgb(), 0xFFFF0000);

        // Set R=0, G=31, B=0
        ppu.write(0x32, 0x20 | 0x00);
        ppu.write(0x32, 0x40 | 0x1F);
        ppu.write(0x32, 0x80 | 0x00);
        assert_eq!(ppu.fixed_color_to_rgb(), 0xFF00FF00);

        // Set R=0, G=0, B=31
        ppu.write(0x32, 0x20 | 0x00);
        ppu.write(0x32, 0x40 | 0x00);
        ppu.write(0x32, 0x80 | 0x1F);
        assert_eq!(ppu.fixed_color_to_rgb(), 0xFF0000FF);
    }
}
