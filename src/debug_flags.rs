use std::sync::OnceLock;

fn env_flag(key: &str, default: bool) -> bool {
    std::env::var(key)
        .map(|v| matches!(v.as_str(), "1" | "true" | "TRUE" | "on" | "ON"))
        .unwrap_or(default)
}

fn env_u16(key: &str, default: u16) -> u16 {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse::<u16>().ok())
        .unwrap_or(default)
}

pub fn dma() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| env_flag("DEBUG_DMA", false))
}

pub fn dma_reg() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| env_flag("DEBUG_DMA_REG", false))
}

// Trace all APU port reads/writes (very verbose)
pub fn trace_apu_port_all() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| env_flag("TRACE_APU_PORT_ALL", false))
}

// Trace only port0 writes/reads (lightweight handshake debug)
pub fn trace_apu_port0() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| env_flag("TRACE_APU_PORT0", false))
}

// Force APU port0/1 to fixed values (HLE debug: APU_PORT0_VAL/APU_PORT1_VAL)
pub fn apu_force_port0() -> Option<u8> {
    static VAL: OnceLock<Option<u8>> = OnceLock::new();
    VAL.get_or_init(|| {
        std::env::var("APU_FORCE_PORT0").ok().and_then(|v| {
            u8::from_str_radix(v.trim_start_matches("0x"), 16)
                .ok()
                .or_else(|| v.parse().ok())
        })
    })
    .clone()
}

pub fn apu_force_port1() -> Option<u8> {
    static VAL: OnceLock<Option<u8>> = OnceLock::new();
    VAL.get_or_init(|| {
        std::env::var("APU_FORCE_PORT1").ok().and_then(|v| {
            u8::from_str_radix(v.trim_start_matches("0x"), 16)
                .ok()
                .or_else(|| v.parse().ok())
        })
    })
    .clone()
}

// CPU テスト専用の簡易HLE（VBlank/JOY/4210 を強制値で返す）
pub fn cpu_test_hle() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| env_flag("CPUTEST_HLE", false))
}

// CPUテストHLE: VBlank中に必ず $4210=0x82 を返し、VBlank外では 0x02 を返す
pub fn cpu_test_hle_strict_vblank() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| env_flag("CPUTEST_HLE_STRICT_VBL", false))
}

// CPUテストHLE: さらに強制的に常時 $4210=0x82 / $4212=0x80 / JOY1L=0x7F を返す
pub fn cpu_test_hle_force() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| env_flag("CPUTEST_HLE_FORCE", false))
}

pub fn mapper() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| env_flag("DEBUG_MAPPER", false))
}

pub fn ppu_write() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| env_flag("DEBUG_PPU_WRITE", false))
}

pub fn boot_verbose() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| env_flag("DEBUG_BOOT", false))
}

// Extra-chatter for rendering/first frames. Alias of DEBUG_BOOT for now,
// but kept separate in case we want finer control later.
pub fn render_verbose() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| env_flag("DEBUG_RENDER", false) || env_flag("DEBUG_BOOT", false))
}

// Trace CGRAM-targeted DMA setup/bytes concisely
pub fn cgram_dma() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| env_flag("DEBUG_CGRAM_DMA", false))
}

#[allow(dead_code)]
pub fn graphics_dma_verbose() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| env_flag("DEBUG_GRAPHICS_DMA", false))
}

pub fn quiet() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| env_flag("QUIET", false))
}

pub fn headless() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| env_flag("HEADLESS", false))
}

// Trace long jumps/returns (JSL/RTL) for DQ3 investigation
pub fn trace_jsl() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| env_flag("TRACE_JSL", false))
}

pub fn trace_rtl() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| env_flag("TRACE_RTL", false))
}

// Watch S-CPU PC for specific addresses (comma-separated hex, bank:addr or addr)
pub fn watch_pc_list() -> Option<&'static [u32]> {
    static LIST: OnceLock<Option<Vec<u32>>> = OnceLock::new();
    LIST.get_or_init(|| {
        if let Ok(val) = std::env::var("WATCH_PC") {
            let mut pcs = Vec::new();
            for part in val.split(',') {
                let p = part.trim();
                if p.is_empty() {
                    continue;
                }
                if let Some((b, a)) = p.split_once(':') {
                    if let (Ok(bank), Ok(addr)) =
                        (u8::from_str_radix(b, 16), u16::from_str_radix(a, 16))
                    {
                        pcs.push(((bank as u32) << 16) | addr as u32);
                    }
                } else if let Ok(addr) = u32::from_str_radix(p, 16) {
                    pcs.push(addr);
                }
            }
            Some(pcs)
        } else {
            None
        }
    })
    .as_deref()
}

// Dump CPU ring buffer when S-CPU PC hits specific addresses (env: DUMP_ON_PC)
// Accepts comma-separated hex, bank:addr or addr (same format as WATCH_PC).
pub fn dump_on_pc_list() -> Option<&'static [u32]> {
    static LIST: OnceLock<Option<Vec<u32>>> = OnceLock::new();
    LIST.get_or_init(|| {
        if let Ok(val) = std::env::var("DUMP_ON_PC") {
            let mut pcs = Vec::new();
            for part in val.split(',') {
                let p = part.trim();
                if p.is_empty() {
                    continue;
                }
                if let Some((b, a)) = p.split_once(':') {
                    if let (Ok(bank), Ok(addr)) =
                        (u8::from_str_radix(b, 16), u16::from_str_radix(a, 16))
                    {
                        pcs.push(((bank as u32) << 16) | addr as u32);
                    }
                } else if let Ok(addr) = u32::from_str_radix(p, 16) {
                    pcs.push(addr);
                }
            }
            Some(pcs)
        } else {
            None
        }
    })
    .as_deref()
}

// Dump CPU ring buffer when the fetched opcode matches (env: DUMP_ON_OPCODE=DB)
pub fn dump_on_opcode() -> Option<u8> {
    static VAL: OnceLock<Option<u8>> = OnceLock::new();
    VAL.get_or_init(|| {
        std::env::var("DUMP_ON_OPCODE").ok().and_then(|v| {
            u8::from_str_radix(v.trim_start_matches("0x"), 16)
                .ok()
                .or_else(|| v.parse().ok())
        })
    })
    .clone()
}

// Filter for ring buffer dumps (env: DUMP_ON_TEST_IDX=000C or 0x000C).
pub fn dump_on_test_idx() -> Option<u16> {
    static VAL: OnceLock<Option<u16>> = OnceLock::new();
    VAL.get_or_init(|| {
        std::env::var("DUMP_ON_TEST_IDX").ok().and_then(|v| {
            u16::from_str_radix(v.trim_start_matches("0x"), 16)
                .ok()
                .or_else(|| v.parse().ok())
        })
    })
    .clone()
}

// Watch SA-1 PC (env: WATCH_SA1_PC, comma separated list, bank:addr or addr)
pub fn watch_sa1_pc_list() -> Option<Vec<u32>> {
    static LIST: OnceLock<Option<Vec<u32>>> = OnceLock::new();
    LIST.get_or_init(|| {
        if let Ok(val) = std::env::var("WATCH_SA1_PC") {
            let mut pcs = Vec::new();
            for part in val.split(',') {
                let p = part.trim();
                if p.is_empty() {
                    continue;
                }
                if let Some((b, a)) = p.split_once(':') {
                    if let (Ok(bank), Ok(addr)) =
                        (u8::from_str_radix(b, 16), u16::from_str_radix(a, 16))
                    {
                        pcs.push(((bank as u32) << 16) | addr as u32);
                    }
                } else if let Ok(addr) = u32::from_str_radix(p, 16) {
                    pcs.push(addr);
                }
            }
            Some(pcs)
        } else {
            None
        }
    })
    .clone()
}

// Trace the first N SA-1 steps (env TRACE_SA1_STEPS=N)
pub fn trace_sa1_steps() -> Option<usize> {
    static VAL: OnceLock<Option<usize>> = OnceLock::new();
    VAL.get_or_init(|| {
        std::env::var("TRACE_SA1_STEPS")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
    })
    .clone()
}

// Trace first N S-CPU instructions (env TRACE_PC_STEPS=N)
pub fn trace_pc_steps() -> Option<usize> {
    static VAL: OnceLock<Option<usize>> = OnceLock::new();
    VAL.get_or_init(|| {
        std::env::var("TRACE_PC_STEPS")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
    })
    .clone()
}

// TRACE_PC_FILE: PCトレースを指定ファイルへ出力（標準出力がノイジーな場合の代替）
pub fn trace_pc_file() -> Option<String> {
    static VAL: OnceLock<Option<String>> = OnceLock::new();
    VAL.get_or_init(|| std::env::var("TRACE_PC_FILE").ok())
        .clone()
}

// Trace SA-1 PC for first N instructions after a forced IRQ (env TRACE_SA1_WAKE_STEPS=N)
pub fn trace_sa1_wake_steps() -> Option<usize> {
    static VAL: OnceLock<Option<usize>> = OnceLock::new();
    VAL.get_or_init(|| {
        std::env::var("TRACE_SA1_WAKE_STEPS")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
    })
    .clone()
}

// Watch a single WRAM/BWRAM address on S-CPU side (env WATCH_ADDR like "7F:7DC0")
pub fn watch_addr() -> Option<u32> {
    static VAL: OnceLock<Option<u32>> = OnceLock::new();
    VAL.get_or_init(|| {
        std::env::var("WATCH_ADDR").ok().and_then(|s| {
            if let Some((b, a)) = s.split_once(':') {
                let bank = u8::from_str_radix(b, 16).ok()?;
                let addr = u16::from_str_radix(a, 16).ok()?;
                Some(((bank as u32) << 16) | addr as u32)
            } else {
                u32::from_str_radix(&s, 16).ok()
            }
        })
    })
    .clone()
}

// Watch a single S-CPU write address (env WATCH_ADDR_W)
pub fn watch_addr_write() -> Option<u32> {
    static VAL: OnceLock<Option<u32>> = OnceLock::new();
    VAL.get_or_init(|| {
        std::env::var("WATCH_ADDR_W").ok().and_then(|s| {
            if let Some((b, a)) = s.split_once(':') {
                let bank = u8::from_str_radix(b, 16).ok()?;
                let addr = u16::from_str_radix(a, 16).ok()?;
                Some(((bank as u32) << 16) | addr as u32)
            } else {
                u32::from_str_radix(&s, 16).ok()
            }
        })
    })
    .clone()
}

// WATCH_WRAM_W: watch WRAM writes (7E/7F banks) with simple logging
pub fn watch_wram_write() -> Option<u32> {
    static VAL: OnceLock<Option<u32>> = OnceLock::new();
    VAL.get_or_init(|| {
        std::env::var("WATCH_WRAM_W").ok().and_then(|s| {
            if let Some((b, a)) = s.split_once(':') {
                let bank = u8::from_str_radix(b, 16).ok()?;
                let addr = u16::from_str_radix(a, 16).ok()?;
                Some(((bank as u32) << 16) | addr as u32)
            } else {
                u32::from_str_radix(&s, 16).ok()
            }
        })
    })
    .clone()
}

// WATCH_WRAM_W_FORCE: 指定アドレスへの書き込みを強制値に置き換える（デバッグ用）
// 形式: WATCH_WRAM_W_FORCE=7E:E95C:01
pub fn watch_wram_write_force() -> Option<(u32, u8)> {
    static VAL: OnceLock<Option<(u32, u8)>> = OnceLock::new();
    VAL.get_or_init(|| {
        std::env::var("WATCH_WRAM_W_FORCE").ok().and_then(|s| {
            let mut parts = s.split(':');
            let b = parts.next()?;
            let a = parts.next()?;
            let v = parts.next()?;
            let bank = u8::from_str_radix(b, 16).ok()?;
            let addr = u16::from_str_radix(a, 16).ok()?;
            let val = u8::from_str_radix(v, 16).ok()?;
            Some((((bank as u32) << 16) | addr as u32, val))
        })
    })
    .clone()
}

// Force S-CPU IRQ for first N frames (env FORCE_SCPU_IRQ_FRAMES=N)
pub fn force_scpu_irq_frames() -> Option<u32> {
    static VAL: OnceLock<Option<u32>> = OnceLock::new();
    VAL.get_or_init(|| {
        std::env::var("FORCE_SCPU_IRQ_FRAMES")
            .ok()
            .and_then(|v| v.parse::<u32>().ok())
    })
    .clone()
}

// Force a repeated SA-1 IRQ every frame (debug; uses IRQ_DMA_BIT)
pub fn sa1_force_irq_each_frame() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| env_flag("SA1_FORCE_IRQ_EACH_FRAME", false))
}

// Force a one-shot SA-1 IRQ to S-CPU early in boot (debug)
pub fn sa1_force_irq_once() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| env_flag("SA1_FORCE_IRQ_ONCE", false))
}

// Force SA-1 to start executing (DQ3 workaround)
pub fn dq3_force_sa1_boot() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| env_flag("DQ3_FORCE_SA1_BOOT", false))
}

// DQ3専用: SA-1を任意のエントリに飛ばす簡易ハック
pub fn dq3_sa1_hack() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| env_flag("DQ3_SA1_HACK", false))
}

// DQ3専用: SA-1 内蔵IPLを最小限スタブする
pub fn dq3_sa1_ipl_stub() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| env_flag("DQ3_SA1_IPL_STUB", false))
}

// DQ3デバッグ: SA-1のBRKをNOPに差し替えて実行継続を試す
pub fn dq3_sa1_brk_to_nop() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| env_flag("DQ3_SA1_BRK_TO_NOP", false))
}

// DQ3デバッグ: SA-1→S-CPU ハンドシェイクを強制 (初期IRQ/NMI要求)
pub fn dq3_sa1_handshake_stub() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| env_flag("DQ3_SA1_HANDSHAKE_STUB", false))
}

// SA-1待機ループ観察用: 特定PCでレジスタダンプ
pub fn trace_sa1_wait() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| env_flag("TRACE_SA1_WAIT", false))
}

// Enable enhanced APU handshake shim ($2140-$2143)
pub fn apu_handshake_plus() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| env_flag("APU_HANDSHAKE_PLUS", false))
}

// Trace concise APU handshake (ports $2140-$2143) reads/writes
pub fn trace_apu_handshake() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| env_flag("TRACE_APU_HANDSHAKE", false))
}

// burn-in-test.sfc debug: trace STAT77 ($213E) reads for OBJ L OVER
pub fn trace_burnin_obj() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| env_flag("TRACE_BURNIN_OBJ", false))
}

// burn-in-test.sfc OBJ L OVER debug: trace the ROM-side checks (few lines)
pub fn trace_burnin_obj_checks() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| env_flag("TRACE_BURNIN_OBJ_CHECKS", false))
}

// Enable coarse memory timing (SlowROM penalty unless $420D FastROM)
pub fn mem_timing() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| env_flag("ENABLE_MEM_TIMING", false))
}

// Compatibility tweaks (boot auto-unblank, etc.) logging
pub fn compat() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| env_flag("DEBUG_COMPAT", false))
}

pub fn trace_sa1_ccdma() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| env_flag("TRACE_SA1_CCDMA", false))
}

#[allow(dead_code)]
pub fn trace_sa1_bwram_guard() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| env_flag("TRACE_SA1_BWRAM_GUARD", false))
}

#[allow(dead_code)]
pub fn trace_sa1_iram_guard() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| env_flag("TRACE_SA1_IRAM_GUARD", false))
}

pub fn trace_sa1_dma() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| env_flag("TRACE_SA1_DMA", false))
}

pub fn trace_ppu_inidisp() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| env_flag("TRACE_PPU_INIDISP", false))
}

// Burn-in test (V224/V239) focused trace (RDNMI/SLHV/OPVCT). Very verbose; enable only when needed.
pub fn trace_burnin_v224() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| env_flag("TRACE_BURNIN_V224", false))
}

// CPU trace / verbose instruction logs (very noisy)
pub fn trace() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| env_flag("DEBUG_TRACE", false) || env_flag("DEBUG_BOOT", false))
}

// Force display (ignore forced blank/brightness when rendering output). Debug-only.
pub fn force_display() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| env_flag("FORCE_DISPLAY", false))
}

// Ignore CPU writes to INIDISP (keep HDMA/MDMA). Debug aid for DQ3.
pub fn ignore_inidisp_cpu() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| env_flag("IGNORE_INIDISP_CPU", false))
}

// Block DMA/HDMA writes to INIDISP (debug)
pub fn block_inidisp_dma() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| env_flag("BLOCK_INIDISP_DMA", false))
}

#[allow(dead_code)]
pub fn trace_sa1_regs() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| env_flag("TRACE_SA1_REGS", false) || env_flag("TRACE_SA1_REG", false))
}

// Enforce approximate PPU timing (gates VRAM/CG writes to safe periods). Debug-only.
pub fn strict_ppu_timing() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| env_flag("STRICT_PPU_TIMING", false))
}

// Debug: treat DMA size=0 as zero bytes instead of 65536 (to bypass runaway DMAs)
pub fn dma_zero_is_zero() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| env_flag("DMA_ZERO_IS_ZERO", false))
}

// Mode7: use full 16x16 signed multiply for $2134-2136 (default: 16x8)
pub fn m7_mul_full16() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| env_flag("M7_MUL_FULL16", false))
}

// Mode7: force product value for $2134-2136 (hex up to 6 hex digits)
pub fn force_m7_product() -> Option<u32> {
    static VAL: OnceLock<Option<u32>> = OnceLock::new();
    VAL.get_or_init(|| {
        std::env::var("FORCE_M7_PRODUCT")
            .ok()
            .and_then(|s| u32::from_str_radix(s.trim_start_matches("0x"), 16).ok())
            .map(|v| v & 0x00FF_FFFF)
    })
    .clone()
}
// Aggregate per-frame render metrics (color window/black clip/color math counts)
pub fn render_metrics() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| env_flag("DEBUG_RENDER_METRICS", false))
}

// Priority model variant: 0 = legacy ad-hoc, 1 = unified z-rank
pub fn priority_model_variant() -> u8 {
    use std::sync::OnceLock;
    fn env_u8(key: &str, default: u8) -> u8 {
        std::env::var(key)
            .ok()
            .and_then(|v| v.parse::<u8>().ok())
            .unwrap_or(default)
    }
    static V: OnceLock<u8> = OnceLock::new();
    *V.get_or_init(|| env_u8("PRIORITY_MODEL", 1))
}

// --- Mode 7 z-rank tunables (i16 semantics but parsed as u16 then clamped) ---
fn env_i16(key: &str, default: i16) -> i16 {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse::<i32>().ok())
        .map(|n| n.clamp(i16::MIN as i32, i16::MAX as i32) as i16)
        .unwrap_or(default)
}

pub fn m7_z_obj3() -> i16 {
    use std::sync::OnceLock;
    static V: OnceLock<i16> = OnceLock::new();
    *V.get_or_init(|| env_i16("M7_Z_OBJ3", 90))
}
pub fn m7_z_obj2() -> i16 {
    use std::sync::OnceLock;
    static V: OnceLock<i16> = OnceLock::new();
    *V.get_or_init(|| env_i16("M7_Z_OBJ2", 70))
}
pub fn m7_z_obj1() -> i16 {
    use std::sync::OnceLock;
    static V: OnceLock<i16> = OnceLock::new();
    *V.get_or_init(|| env_i16("M7_Z_OBJ1", 50))
}
pub fn m7_z_obj0() -> i16 {
    use std::sync::OnceLock;
    static V: OnceLock<i16> = OnceLock::new();
    *V.get_or_init(|| env_i16("M7_Z_OBJ0", 30))
}
pub fn m7_z_bg1() -> i16 {
    use std::sync::OnceLock;
    static V: OnceLock<i16> = OnceLock::new();
    *V.get_or_init(|| env_i16("M7_Z_BG1", 65))
}
pub fn m7_z_bg2() -> i16 {
    use std::sync::OnceLock;
    static V: OnceLock<i16> = OnceLock::new();
    *V.get_or_init(|| env_i16("M7_Z_BG2", 45))
}

// Control-port commit margins in HBlank for VMADD/CGADD effects (head/tail)
pub fn vmadd_ctrl_head() -> u16 {
    static V: OnceLock<u16> = OnceLock::new();
    *V.get_or_init(|| env_u16("VMADD_CTRL_HEAD", 2))
}
pub fn vmadd_ctrl_tail() -> u16 {
    static V: OnceLock<u16> = OnceLock::new();
    *V.get_or_init(|| env_u16("VMADD_CTRL_TAIL", 2))
}
pub fn cgadd_ctrl_head() -> u16 {
    static V: OnceLock<u16> = OnceLock::new();
    *V.get_or_init(|| env_u16("CGADD_CTRL_HEAD", 2))
}
pub fn cgadd_ctrl_tail() -> u16 {
    static V: OnceLock<u16> = OnceLock::new();
    *V.get_or_init(|| env_u16("CGADD_CTRL_TAIL", 2))
}
pub fn cgadd_effect_delay_dots() -> u16 {
    static V: OnceLock<u16> = OnceLock::new();
    *V.get_or_init(|| env_u16("CGADD_EFFECT_DELAY", 1))
}
pub fn vram_gap_after_vmain() -> u16 {
    static V: OnceLock<u16> = OnceLock::new();
    *V.get_or_init(|| env_u16("VRAM_DATA_GAP_AFTER_VMAIN", 2))
}
pub fn cgram_gap_after_cgadd() -> u16 {
    static V: OnceLock<u16> = OnceLock::new();
    *V.get_or_init(|| env_u16("CGRAM_DATA_GAP_AFTER_CGADD", 2))
}

// Defer applying VMAIN effects by this many dots after commit (to simulate pipeline latency)
pub fn vmain_effect_delay_dots() -> u16 {
    static V: OnceLock<u16> = OnceLock::new();
    *V.get_or_init(|| env_u16("VMAIN_EFFECT_DELAY", 1))
}

// Log the first strict-timing rejection per frame (port, scanline, dot, context)
pub fn timing_rejects() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| env_flag("DEBUG_TIMING_REJECTS", false))
}

// --- HBlank head guard (HDMA-only phase) ---
pub fn hblank_hdma_guard_dots() -> u16 {
    static V: OnceLock<u16> = OnceLock::new();
    *V.get_or_init(|| env_u16("HBLANK_HDMA_GUARD", 6))
}

// --- Per-port margins (head/tail) for HDMA/MDMA in HBlank ---
pub fn vram_hdma_head() -> u16 {
    static V: OnceLock<u16> = OnceLock::new();
    *V.get_or_init(|| env_u16("VRAM_HDMA_HEAD", 0))
}
pub fn vram_hdma_tail() -> u16 {
    static V: OnceLock<u16> = OnceLock::new();
    *V.get_or_init(|| env_u16("VRAM_HDMA_TAIL", 2))
}
pub fn vram_mdma_head() -> u16 {
    static V: OnceLock<u16> = OnceLock::new();
    *V.get_or_init(|| env_u16("VRAM_MDMA_HEAD", 6))
}
pub fn vram_mdma_tail() -> u16 {
    static V: OnceLock<u16> = OnceLock::new();
    *V.get_or_init(|| env_u16("VRAM_MDMA_TAIL", 9))
}

pub fn cgram_hdma_head() -> u16 {
    static V: OnceLock<u16> = OnceLock::new();
    *V.get_or_init(|| env_u16("CGRAM_HDMA_HEAD", 4))
}
pub fn cgram_hdma_tail() -> u16 {
    static V: OnceLock<u16> = OnceLock::new();
    *V.get_or_init(|| env_u16("CGRAM_HDMA_TAIL", 4))
}

pub fn oam_hdma_head() -> u16 {
    static V: OnceLock<u16> = OnceLock::new();
    *V.get_or_init(|| env_u16("OAM_HDMA_HEAD", 6))
}
pub fn oam_hdma_tail() -> u16 {
    static V: OnceLock<u16> = OnceLock::new();
    *V.get_or_init(|| env_u16("OAM_HDMA_TAIL", 6))
}
pub fn oam_gap_after_oamadd() -> u16 {
    static V: OnceLock<u16> = OnceLock::new();
    *V.get_or_init(|| env_u16("OAM_DATA_GAP_AFTER_OAMADD", 2))
}
pub fn oam_gap_in_vblank() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| env_flag("OAM_GAP_IN_VBLANK", false))
}

// --- VBlank head/tail safety sub-windows for MDMA/CPU writes ---
fn env_u16_local(key: &str, default: u16) -> u16 {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse::<u16>().ok())
        .unwrap_or(default)
}

// VRAM MDMA/CPU writes allowed in VBlank except the first/last N dots of VBlank edges
pub fn vram_vblank_head() -> u16 {
    static V: OnceLock<u16> = OnceLock::new();
    *V.get_or_init(|| env_u16_local("VRAM_VBLANK_HEAD", 0))
}
pub fn vram_vblank_tail() -> u16 {
    static V: OnceLock<u16> = OnceLock::new();
    *V.get_or_init(|| env_u16_local("VRAM_VBLANK_TAIL", 0))
}

// CGRAM MDMA/CPU writes are VBlank-only; optionally reserve head/tail sub-windows
pub fn cgram_vblank_head() -> u16 {
    static V: OnceLock<u16> = OnceLock::new();
    *V.get_or_init(|| env_u16_local("CGRAM_VBLANK_HEAD", 0))
}
pub fn cgram_vblank_tail() -> u16 {
    static V: OnceLock<u16> = OnceLock::new();
    *V.get_or_init(|| env_u16_local("CGRAM_VBLANK_TAIL", 0))
}

// OAM MDMA/CPU writes are VBlank-only; optionally reserve head/tail sub-windows
pub fn oam_vblank_head() -> u16 {
    static V: OnceLock<u16> = OnceLock::new();
    *V.get_or_init(|| env_u16_local("OAM_VBLANK_HEAD", 0))
}
pub fn oam_vblank_tail() -> u16 {
    static V: OnceLock<u16> = OnceLock::new();
    *V.get_or_init(|| env_u16_local("OAM_VBLANK_TAIL", 0))
}

// --- Verbose diagnostic logs (typically off for production) ---

// Log RESET area ($00FFxx) reads during initialization
pub fn debug_reset_area() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| env_flag("DEBUG_RESET_AREA", false))
}

// Log CGRAM color reads during rendering (very noisy)
pub fn debug_cgram_read() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| env_flag("DEBUG_CGRAM_READ", false))
}

// Log per-pixel BG rendering details (very noisy)
pub fn debug_bg_pixel() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| env_flag("DEBUG_BG_PIXEL", false))
}

// Log render dot details per scanline (very noisy)
pub fn debug_render_dot() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| env_flag("DEBUG_RENDER_DOT", false))
}

// Log suspicious tile configurations (empty tile base, etc.)
pub fn debug_suspicious_tile() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| env_flag("DEBUG_SUSPICIOUS_TILE", false))
}

// Log DQ3-specific bank access patterns
pub fn debug_dq3_bank() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| env_flag("DEBUG_DQ3_BANK", false))
}

// Log stack reads during initialization
pub fn debug_stack_read() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| env_flag("DEBUG_STACK_READ", false))
}

// Log pixel found events (when non-zero pixels are rendered)
pub fn debug_pixel_found() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| env_flag("DEBUG_PIXEL_FOUND", false))
}

// Log periodic "graphics detected" messages (can be noisy; off by default).
pub fn debug_graphics_detected() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| env_flag("DEBUG_GRAPHICS_DETECTED", false))
}
