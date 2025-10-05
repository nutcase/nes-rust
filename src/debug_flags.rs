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

// Enable enhanced APU handshake shim ($2140-$2143)
pub fn apu_handshake_plus() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| env_flag("APU_HANDSHAKE_PLUS", false))
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
