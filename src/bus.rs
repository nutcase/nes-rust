#![allow(static_mut_refs)]
#![allow(unreachable_patterns)]
use std::sync::{
    atomic::{AtomicU32, Ordering},
    OnceLock,
};
use std::sync::{Arc, Mutex};

// Logging controls
use crate::cpu_bus::CpuBus;
use crate::debug_flags;
use crate::fake_apu::FakeApuUploadState;
use crate::sa1::Sa1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CpuTestResult {
    Pass { test_idx: u16 },
    Fail { test_idx: u16 },
    InvalidOrder { test_idx: u16 },
}

pub struct Bus {
    wram: Vec<u8>,
    wram_64k_mirror: bool,
    trace_nmi_wram: bool,
    sram: Vec<u8>,
    rom: Vec<u8>,
    ppu: crate::ppu::Ppu,
    apu: Arc<Mutex<crate::apu::Apu>>,
    dma_controller: crate::dma::DmaController,
    input_system: crate::input::InputSystem,
    mapper_type: crate::cartridge::MapperType,
    rom_size: usize,
    sram_size: usize,
    // Mark when battery-backed RAM was modified
    sram_dirty: bool,
    // Memory mapping registers
    nmitimen: u8,      // $4200 - Interrupt Enable
    wram_address: u32, // $2181-2183 - WRAM Address
    mdr: u8,           // Memory Data Register (open bus)
    // Hardware math registers (CPU I/O $4202-$4206; results at $4214-$4217)
    mul_a: u8,
    mul_b: u8,
    mul_result: u16,
    div_a: u16,
    div_b: u8,
    div_quot: u16,
    div_rem: u16,
    // Hardware math in-flight timing (coarse per S-CPU cycle slice)
    mul_busy: bool,
    mul_just_started: bool,
    mul_cycles_left: u8,
    mul_work_a: u16,
    mul_work_b: u8,
    mul_partial: u16,
    div_busy: bool,
    div_just_started: bool,
    div_cycles_left: u8,
    div_work_dividend: u16,
    div_work_divisor: u8,
    div_work_quot: u16,
    div_work_rem: u16,
    div_work_bit: i8,
    // CPU命令内のバスアクセス数（サイクル近似）を数えるためのフック。
    // - CpuBusトレイト経由の read_u8/write_u8 を 1回=1サイクル相当として扱い、
    //   $4202-$4206 等の時間依存I/Oをより正確に進める。
    cpu_instr_active: bool,
    cpu_instr_bus_cycles: u8,
    // CPUアクセスのウェイト状態（Fast/Slow/JOYSER）を master cycles で積む。
    // ベースは 6 master cycles/CPU cycle としているため、差分（+2/+6）だけをここに蓄積する。
    cpu_instr_extra_master_cycles: u64,

    // IRQ/Timer
    irq_h_enabled: bool,             // $4200 bit4
    irq_v_enabled: bool,             // $4200 bit5
    irq_pending: bool,               // TIMEUP ($4211)
    irq_v_matched_line: Option<u16>, // remember V-match scanline when both H&V are enabled
    h_timer: u16,                    // $4207/$4208 (not fully used yet)
    v_timer: u16,                    // $4209/$420A
    h_timer_set: bool,
    v_timer_set: bool,

    // Auto-joypad (NMITIMEN bit0) + JOYBUSY/JOY registers
    joy_busy_counter: u8,   // >0 while auto-joy is in progress
    joy_data: [u8; 8],      // $4218..$421F (JOY1L,JOY1H,JOY2L,JOY2H,JOY3L,JOY3H,JOY4L,JOY4H)
    joy_busy_scanlines: u8, // configurable duration of JOYBUSY after VBlank start
    // CPUテストROM用の自動入力（RIGHT を短時間だけ押下）
    cpu_test_mode: bool,
    cpu_test_auto_frames: u32,
    cpu_test_auto_joy_phase: u8,
    cpu_test_result: Option<CpuTestResult>,

    // Run-wide counters for headless init summary
    nmitimen_writes_count: u32,
    mdmaen_nonzero_count: u32,
    hdmaen_nonzero_count: u32,

    // DMA config observation (how many writes to $43x0-$43x6 etc.)
    dma_reg_writes: u32,
    // DMA destination histogram (B-bus low 7 bits)
    dma_dest_hist: [u32; 256],
    // Pending graphics DMA mask (strict timing: defer VRAM/CGRAM/OAM MDMA to VBlank)
    pending_gdma_mask: u8,
    // Pending general DMA mask (MDMAEN): starts after the *next opcode fetch*.
    pending_mdma_mask: u8,
    // One-shot: set when an opcode fetch triggered MDMA start.
    // Used by the CPU core to defer executing that instruction until after the DMA stall.
    mdma_started_after_opcode_fetch: bool,
    last_cpu_pc: u32,       // debug: last S-CPU PC that touched the bus
    last_cpu_bus_addr: u32, // debug: last S-CPU bus address (for timing heuristics)
    // HDMA aggregate stats (visible for headless summaries)
    hdma_lines_executed: u32,
    hdma_bytes_vram: u32,
    hdma_bytes_cgram: u32,
    hdma_bytes_oam: u32,
    rdnmi_consumed: bool,
    rdnmi_high_byte_for_test: u8,

    // Extra master cycles consumed by DMA stalls (CPU is halted while PPU/APU continue).
    pending_stall_master_cycles: u64,

    // Optional APU handshake + SPC upload HLE (protocol-faithful)
    fake_apu: bool,
    fake_apu_ports: [u8; 4],
    fake_apu_booted: bool,
    fake_apu_upload: bool,
    fake_apu_upload_state: FakeApuUploadState,
    fake_apu_upload_bytes: u32,
    fake_apu_upload_data_bytes: u32,
    fake_apu_upload_buf: Vec<u8>,
    fake_apu_upload_echo: [u8; 4],
    fake_apu_spc_ram: Vec<u8>, // 64KB SPC RAM mirror for HLE upload
    fake_apu_spc_addr: u16,    // current SPC upload destination
    fake_apu_last_port0: u8,   // last written port0 (index/kick)
    fake_apu_next_index: u8,
    fake_apu_run_cooldown: u8,
    fake_apu_size: u16,
    fake_apu_size_set: bool,
    fake_apu_idle_reads: u32,
    fake_apu_fast_done: bool,

    // SMW用デバッグHLE: WRAM DMAからSPCコードを抜き取り即ロードする
    smw_apu_hle: bool,
    smw_apu_hle_done: bool,
    smw_apu_hle_buf: Vec<u8>,
    smw_apu_hle_echo_idx: u32,

    // Programmable I/O and memory speed
    wio: u8,       // $4201 write; read back via $4213
    fastrom: bool, // $420D bit0
    // Test ROM integration: capture APU $2140 prints
    test_apu_print: bool,
    test_apu_buf: String,
    sa1: Sa1,
    sa1_bwram: Vec<u8>,
    #[allow(dead_code)]
    sa1_iram: [u8; 0x800],
    sa1_cycle_deficit: i64,
    // SA-1 initialization support: delay NMI during boot
    pub(crate) sa1_nmi_delay_active: bool,
}

impl Bus {
    #[inline]
    fn dma_a_bus_is_mmio_blocked(addr: u32) -> bool {
        let bank = ((addr >> 16) & 0xFF) as u8;
        let off = (addr & 0xFFFF) as u16;
        // SNESdev wiki: DMA cannot access A-bus addresses that overlap MMIO registers:
        // $2100-$21FF, $4000-$41FF, $4200-$421F, $4300-$437F (in system banks).
        //
        // These MMIO ranges are only mapped in banks $00-$3F and $80-$BF; in other banks
        // the same low addresses typically map to ROM/RAM and are accessible.
        if !((0x00..=0x3F).contains(&bank) || (0x80..=0xBF).contains(&bank)) {
            return false;
        }
        matches!(
            off,
            0x2100..=0x21FF | 0x4000..=0x41FF | 0x4200..=0x421F | 0x4300..=0x437F
        )
    }

    #[inline]
    fn dma_read_a_bus(&mut self, addr: u32) -> u8 {
        if Self::dma_a_bus_is_mmio_blocked(addr) {
            // Open bus (MDR) – do not trigger side-effects.
            self.mdr
        } else {
            self.read_u8(addr)
        }
    }

    #[inline]
    fn dma_write_a_bus(&mut self, addr: u32, value: u8) {
        if Self::dma_a_bus_is_mmio_blocked(addr) {
            // Ignore writes to MMIO addresses on the A-bus (hardware blocks DMA access).
            return;
        }
        self.write_u8(addr, value);
    }

    #[inline]
    fn on_cpu_bus_cycle(&mut self) {
        if !self.cpu_instr_active {
            return;
        }
        self.cpu_instr_bus_cycles = self.cpu_instr_bus_cycles.saturating_add(1);
        self.cpu_instr_extra_master_cycles = self
            .cpu_instr_extra_master_cycles
            .saturating_add(self.cpu_access_extra_master_cycles(self.last_cpu_bus_addr));
        self.tick_cpu_cycles(1);
    }

    #[inline]
    fn is_wram_address(&self, addr: u32) -> bool {
        let bank = ((addr >> 16) & 0xFF) as u8;
        let off = (addr & 0xFFFF) as u16;
        // WRAM direct: $7E:0000-$7F:FFFF
        if (0x7E..=0x7F).contains(&bank) {
            return true;
        }
        // WRAM mirror: $00-$3F/$80-$BF:0000-1FFF
        ((0x00..=0x3F).contains(&bank) || (0x80..=0xBF).contains(&bank)) && off < 0x2000
    }

    #[inline]
    fn cpu_access_master_cycles(&self, addr: u32) -> u8 {
        // Reference: https://snes.nesdev.org/wiki/Timing
        let bank = ((addr >> 16) & 0xFF) as u8;
        let off = (addr & 0xFFFF) as u16;

        // JOYSER0/1: always 12 master clocks
        if ((0x00..=0x3F).contains(&bank) || (0x80..=0xBF).contains(&bank))
            && matches!(off, 0x4016 | 0x4017)
        {
            return 12;
        }

        // Most MMIO: 6 master clocks
        if ((0x00..=0x3F).contains(&bank) || (0x80..=0xBF).contains(&bank))
            && matches!(
                off,
                0x2100..=0x21FF | 0x4000..=0x41FF | 0x4200..=0x421F | 0x4300..=0x437F
            )
        {
            return 6;
        }

        // Internal WRAM: 8 master clocks
        if self.is_wram_address(addr) {
            return 8;
        }

        // ROM: 6 master clocks for FastROM ($80:0000+ with MEMSEL=1), otherwise 8.
        if self.is_rom_address(addr) {
            let fast = self.fastrom && (addr & 0x80_0000) != 0;
            return if fast { 6 } else { 8 };
        }

        // Default to 8 (safe/slow) for SRAM/unknown regions.
        8
    }

    #[inline]
    fn cpu_access_extra_master_cycles(&self, addr: u32) -> u64 {
        let mc = self.cpu_access_master_cycles(addr);
        mc.saturating_sub(6) as u64
    }

    #[inline]
    pub fn wram(&self) -> &[u8] {
        &self.wram
    }

    #[inline]
    fn add16_in_bank(addr: u32, delta: u32) -> u32 {
        let bank = addr & 0x00FF_0000;
        let lo = (addr & 0x0000_FFFF).wrapping_add(delta) & 0x0000_FFFF; // allow wrapping within 16-bit
        bank | lo
    }
    #[allow(dead_code)]
    pub fn new(rom: Vec<u8>) -> Self {
        let rom_size = rom.len();
        // APU handshake stub flag (accept both APU_FAKE and legacy APU_FAKE_BBAA)
        let fake_apu = std::env::var("APU_FAKE")
            .map(|v| v == "1" || v.to_lowercase() == "true")
            .unwrap_or_else(|_| {
                std::env::var("APU_FAKE_BBAA")
                    .map(|v| v == "1" || v.to_lowercase() == "true")
                    .unwrap_or(false)
            });
        // Optional: also fake SPC upload protocol (short-circuit APU boot)
        let fake_apu_upload = std::env::var("APU_FAKE_UPLOAD")
            .map(|v| v == "1" || v.to_lowercase() == "true")
            .unwrap_or(false);
        let fake_apu_fast_done = std::env::var("APU_FAKE_FASTDONE")
            .map(|v| v != "0" && v.to_lowercase() != "false")
            .unwrap_or(true);
        let mut bus = Self {
            wram: vec![0; 0x20000],
            wram_64k_mirror: std::env::var_os("WRAM_64K_MIRROR").is_some(),
            trace_nmi_wram: std::env::var_os("TRACE_NMI_WRAM").is_some(),
            sram: vec![0; 0x8000],
            rom,
            ppu: crate::ppu::Ppu::new(),
            apu: Arc::new(Mutex::new(crate::apu::Apu::new())),
            dma_controller: crate::dma::DmaController::new(),
            input_system: crate::input::InputSystem::new(),
            mapper_type: crate::cartridge::MapperType::LoRom, // Default to LoROM
            rom_size,
            sram_size: 0x8000,
            sram_dirty: false,
            nmitimen: 0,
            wram_address: 0,
            mdr: 0,
            mul_a: 0,
            mul_b: 0,
            mul_result: 0,
            div_a: 0,
            div_b: 0,
            div_quot: 0,
            div_rem: 0,
            mul_busy: false,
            mul_just_started: false,
            mul_cycles_left: 0,
            mul_work_a: 0,
            mul_work_b: 0,
            mul_partial: 0,
            div_busy: false,
            div_just_started: false,
            div_cycles_left: 0,
            div_work_dividend: 0,
            div_work_divisor: 0,
            div_work_quot: 0,
            div_work_rem: 0,
            div_work_bit: 0,
            cpu_instr_active: false,
            cpu_instr_bus_cycles: 0,
            cpu_instr_extra_master_cycles: 0,

            irq_h_enabled: false,
            irq_v_enabled: false,
            irq_pending: false,
            irq_v_matched_line: None,
            h_timer: 0,
            v_timer: 0,
            h_timer_set: false,
            v_timer_set: false,

            joy_busy_counter: 0,
            // $4218-$421F (JOY1..4): power-on should read as "no buttons pressed".
            // SNES joypad bits are "1=Low=Pressed", so default is 0x00.
            joy_data: [0x00; 8],
            // JOYBUSY はオートジョイパッド読み取り中だけ立つ。
            // 実機では約 3 本分のスキャンライン相当 (4224 master cycles) 継続する。
            // CPU テスト ROM では VBlank 突入から数ライン後に $4212 を覗くため、
            // 少し長めのデフォルト (8 ライン) にして読み損ねを防ぐ。
            joy_busy_scanlines: std::env::var("JOYBUSY_SCANLINES")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(8),
            cpu_test_mode: false,
            cpu_test_auto_frames: 120,
            cpu_test_auto_joy_phase: 0,
            cpu_test_result: None,

            nmitimen_writes_count: 0,
            mdmaen_nonzero_count: 0,
            hdmaen_nonzero_count: 0,

            // WRIO ($4201) behaves as if initialized to all-1s at power-on.
            wio: 0xFF,
            fastrom: false,
            dma_reg_writes: 0,
            dma_dest_hist: [0; 256],
            pending_gdma_mask: 0,
            pending_mdma_mask: 0,
            mdma_started_after_opcode_fetch: false,
            last_cpu_pc: 0,
            last_cpu_bus_addr: 0,
            hdma_lines_executed: 0,
            hdma_bytes_vram: 0,
            hdma_bytes_cgram: 0,
            hdma_bytes_oam: 0,
            rdnmi_consumed: false,
            rdnmi_high_byte_for_test: 0,
            pending_stall_master_cycles: 0,
            fake_apu,
            fake_apu_ports: if fake_apu {
                [0xAA, 0xBB, 0x00, 0x00]
            } else {
                [0; 4]
            },
            fake_apu_booted: false,
            fake_apu_upload,
            fake_apu_upload_state: crate::fake_apu::FakeApuUploadState::default(),
            fake_apu_upload_bytes: 0,
            fake_apu_upload_data_bytes: 0,
            fake_apu_upload_buf: Vec::new(),
            fake_apu_upload_echo: [0; 4],
            fake_apu_spc_ram: vec![0; 0x10000],
            fake_apu_spc_addr: 0,
            fake_apu_last_port0: 0,
            fake_apu_next_index: 0,
            fake_apu_run_cooldown: 0,
            fake_apu_size: 0,
            fake_apu_size_set: false,
            fake_apu_idle_reads: 0,
            fake_apu_fast_done,
            // SMW専用のWRAM→APU自動ロード（HLE）はデフォルト無効。
            smw_apu_hle: std::env::var("SMW_APU_HLE")
                .map(|v| v != "0" && v.to_lowercase() != "false")
                .unwrap_or(false),
            smw_apu_hle_done: false,
            smw_apu_hle_buf: Vec::new(),
            smw_apu_hle_echo_idx: 0,
            test_apu_print: std::env::var("TESTROM_APU_PRINT")
                .map(|v| v == "1" || v.to_lowercase() == "true")
                .unwrap_or(false),
            test_apu_buf: String::new(),
            sa1: Sa1::new(),
            sa1_bwram: vec![0; 0x40000],
            sa1_iram: [0; 0x800],
            sa1_cycle_deficit: 0,
            sa1_nmi_delay_active: false,
        };

        // Mirror WRIO bit7 to PPU latch enable.
        bus.ppu.set_wio_latch_enable(true);

        // DQ3専用ブートハック: BW-RAM に SA-1 ROM 0C:6000-FFFF を展開して待機ループを回避
        if bus.mapper_type == crate::cartridge::MapperType::DragonQuest3 {
            // Fill BW-RAM with 0xFF to mimic power-on state
            bus.sa1_bwram.fill(0xFF);
            bus.copy_sa1_bwram_from_rom(0x0C, 0x6000, 0xA000);
            // Set ready flag at 00:7DE2 (BWRAM) to 01 00 00 00
            bus.dq3_bwram_set_ready();
        }

        bus
    }

    // Helper: copy a chunk from SA-1 ROM bank into BW-RAM (for DQ3 bootstrap)
    fn copy_sa1_bwram_from_rom(&mut self, bank: u8, offset: u16, len: usize) {
        if self.sa1_bwram.is_empty() {
            return;
        }
        let mut remaining = len.min(self.sa1_bwram.len());
        let mut off = offset as usize;
        let mut written = 0usize;
        while remaining > 0 {
            let phys = self.sa1_phys_addr(bank as u32, (off & 0xFFFF) as u16);
            let byte = self.rom.get(phys % self.rom_size).copied().unwrap_or(0x00);
            self.sa1_bwram[written] = byte;
            written += 1;
            off = off.wrapping_add(1);
            remaining -= 1;
        }
        if std::env::var_os("TRACE_SA1_BOOT").is_some() {
            println!(
                "[SA1] BW-RAM filled from ROM bank {:02X} offset 0x{:04X} len=0x{:04X}",
                bank, offset, len
            );
        }
    }

    pub fn dq3_bwram_set_ready(&mut self) {
        if self.mapper_type != crate::cartridge::MapperType::DragonQuest3 {
            return;
        }
        // BWRAM index for 00:7DE2 = 0x7DE2 -> block0 offset
        let idx = 0x7DE2usize;
        if idx + 4 <= self.sa1_bwram.len() {
            self.sa1_bwram[idx] = 0x01;
            self.sa1_bwram[idx + 1] = 0x00;
            self.sa1_bwram[idx + 2] = 0x00;
            self.sa1_bwram[idx + 3] = 0x00;
            if std::env::var_os("TRACE_BWRAM_SYS").is_some() {
                println!("BWRAM SYS W (hack) idx=0x{:05X} val=01 00 00 00", idx);
            }
        }
    }

    pub fn new_with_mapper(
        rom: Vec<u8>,
        mapper: crate::cartridge::MapperType,
        sram_size: usize,
    ) -> Self {
        let rom_size = rom.len();
        // APU handshake stub flag (accept both APU_FAKE and legacy APU_FAKE_BBAA)
        let fake_apu = std::env::var("APU_FAKE")
            .map(|v| v == "1" || v.to_lowercase() == "true")
            .unwrap_or_else(|_| {
                std::env::var("APU_FAKE_BBAA")
                    .map(|v| v == "1" || v.to_lowercase() == "true")
                    .unwrap_or(false)
            });
        let fake_apu_upload = std::env::var("APU_FAKE_UPLOAD")
            .map(|v| v == "1" || v.to_lowercase() == "true")
            .unwrap_or(false);
        let fake_apu_fast_done = std::env::var("APU_FAKE_FASTDONE")
            .map(|v| v != "0" && v.to_lowercase() != "false")
            .unwrap_or(true);
        let mut bus = Self {
            wram: vec![0; 0x20000],
            wram_64k_mirror: std::env::var_os("WRAM_64K_MIRROR").is_some(),
            trace_nmi_wram: std::env::var_os("TRACE_NMI_WRAM").is_some(),
            sram: vec![0; sram_size.max(0x2000)], // Minimum 8KB SRAM
            rom,
            ppu: crate::ppu::Ppu::new(),
            apu: Arc::new(Mutex::new(crate::apu::Apu::new())),
            dma_controller: crate::dma::DmaController::new(),
            input_system: crate::input::InputSystem::new(),
            mapper_type: mapper,
            rom_size,
            sram_size,
            sram_dirty: false,
            nmitimen: 0,
            wram_address: 0,
            mdr: 0,
            mul_a: 0,
            mul_b: 0,
            mul_result: 0,
            div_a: 0,
            div_b: 0,
            div_quot: 0,
            div_rem: 0,
            mul_busy: false,
            mul_just_started: false,
            mul_cycles_left: 0,
            mul_work_a: 0,
            mul_work_b: 0,
            mul_partial: 0,
            div_busy: false,
            div_just_started: false,
            div_cycles_left: 0,
            div_work_dividend: 0,
            div_work_divisor: 0,
            div_work_quot: 0,
            div_work_rem: 0,
            div_work_bit: 0,
            cpu_instr_active: false,
            cpu_instr_bus_cycles: 0,
            cpu_instr_extra_master_cycles: 0,

            irq_h_enabled: false,
            irq_v_enabled: false,
            irq_pending: false,
            irq_v_matched_line: None,
            h_timer: 0,
            v_timer: 0,
            h_timer_set: false,
            v_timer_set: false,

            joy_busy_counter: 0,
            // $4218-$421F (JOY1..4): power-on should read as "no buttons pressed".
            // SNES joypad bits are "1=Low=Pressed", so default is 0x00.
            joy_data: [0x00; 8],
            joy_busy_scanlines: std::env::var("JOYBUSY_SCANLINES")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(8),
            cpu_test_mode: false,
            cpu_test_auto_frames: 120,
            cpu_test_auto_joy_phase: 0,
            cpu_test_result: None,

            nmitimen_writes_count: 0,
            mdmaen_nonzero_count: 0,
            hdmaen_nonzero_count: 0,

            // WRIO ($4201) behaves as if initialized to all-1s at power-on.
            wio: 0xFF,
            fastrom: false,
            dma_reg_writes: 0,
            dma_dest_hist: [0; 256],
            pending_gdma_mask: 0,
            pending_mdma_mask: 0,
            mdma_started_after_opcode_fetch: false,
            last_cpu_pc: 0,
            last_cpu_bus_addr: 0,
            hdma_lines_executed: 0,
            hdma_bytes_vram: 0,
            hdma_bytes_cgram: 0,
            hdma_bytes_oam: 0,
            rdnmi_consumed: false,
            rdnmi_high_byte_for_test: 0,
            pending_stall_master_cycles: 0,
            fake_apu,
            fake_apu_ports: if fake_apu {
                [0xAA, 0xBB, 0x00, 0x00]
            } else {
                [0; 4]
            },
            fake_apu_booted: false,
            fake_apu_upload,
            fake_apu_upload_state: crate::fake_apu::FakeApuUploadState::default(),
            fake_apu_upload_bytes: 0,
            fake_apu_upload_data_bytes: 0,
            fake_apu_upload_buf: Vec::new(),
            fake_apu_upload_echo: [0; 4],
            fake_apu_spc_ram: vec![0; 0x10000],
            fake_apu_spc_addr: 0,
            fake_apu_last_port0: 0,
            fake_apu_next_index: 0,
            fake_apu_run_cooldown: 0,
            fake_apu_size: 0,
            fake_apu_size_set: false,
            fake_apu_idle_reads: 0,
            fake_apu_fast_done,
            smw_apu_hle: std::env::var("SMW_APU_HLE")
                .map(|v| v != "0" && v.to_lowercase() != "false")
                .unwrap_or(false),
            smw_apu_hle_done: false,
            smw_apu_hle_buf: Vec::new(),
            smw_apu_hle_echo_idx: 0,
            test_apu_print: std::env::var("TESTROM_APU_PRINT")
                .map(|v| v == "1" || v.to_lowercase() == "true")
                .unwrap_or(false),
            test_apu_buf: String::new(),
            sa1: Sa1::new(),
            sa1_bwram: vec![0xFF; sram_size.max(0x2000)], // fill with 0xFF for SA-1
            sa1_iram: [0; 0x800],
            sa1_cycle_deficit: 0,
            sa1_nmi_delay_active: false,
        };

        // Mirror WRIO bit7 to PPU latch enable.
        bus.ppu.set_wio_latch_enable(true);

        // DQ3 boot hack: seed WRAM 7F:002C with a sane stack pointer until SA-1 initializes it.
        if mapper == crate::cartridge::MapperType::DragonQuest3 {
            let idx = 0x1002C;
            if idx + 1 < bus.wram.len() {
                bus.wram[idx] = 0x32;
                bus.wram[idx + 1] = 0x01;
            }

            // DQ3専用: SA-1内蔵IPLの初期化相当をエミュレート。
            //  - BWRAMを0xFFでクリア（上で初期化済み）
            //  - ROMバンク0Cの 0x6000-0xFFFF をBWRAMに展開
            //  - ゲームが参照する「準備完了」フラグ(00:7DE2)をセット
            bus.copy_sa1_bwram_from_rom(0x0C, 0x6000, 0xA000);
            bus.dq3_bwram_set_ready();
        }

        bus
    }

    #[inline]
    pub fn is_sa1_active(&self) -> bool {
        matches!(
            self.mapper_type,
            crate::cartridge::MapperType::Sa1 | crate::cartridge::MapperType::DragonQuest3
        )
    }

    /// Force disable all IRQs (for SA-1 initialization delay)
    pub(crate) fn force_disable_irq(&mut self) {
        self.irq_h_enabled = false;
        self.irq_v_enabled = false;
        self.irq_pending = false;
    }

    #[allow(dead_code)]
    pub fn sa1(&self) -> &Sa1 {
        &self.sa1
    }

    #[allow(dead_code)]
    pub fn sa1_mut(&mut self) -> &mut Sa1 {
        &mut self.sa1
    }

    fn init_sa1_vectors_from_rom(&mut self) {
        if !self.is_sa1_active() {
            return;
        }
        let debug = std::env::var_os("TRACE_SA1_BOOT").is_some();
        let fetch_vec = |addr: u16, this: &mut Self| -> u16 {
            let phys = this.sa1_phys_addr(0x00, addr);
            let lo = this.rom.get(phys % this.rom_size).copied().unwrap_or(0x00);
            let hi = this
                .rom
                .get((phys + 1) % this.rom_size)
                .copied()
                .unwrap_or(0x00);
            (hi as u16) << 8 | lo as u16
        };
        let reset_vec = fetch_vec(0xFFFC, self);
        let nmi_vec = fetch_vec(0xFFEA, self);
        let irq_vec = fetch_vec(0xFFEE, self);
        self.sa1.registers.reset_vector = reset_vec;
        self.sa1.registers.nmi_vector = nmi_vec;
        self.sa1.registers.irq_vector = irq_vec;
        // Default SA-1: use ROM header vectors, program bank chunk = 0 (C block).
        self.sa1.boot_pb = 0x00;

        // If a real SA-1 IPL dump is present, load it into IRAM and boot from 0x0000.
        let mut ipl_loaded = false;
        if self.is_sa1_active() {
            let candidates = [
                std::path::Path::new("sa1.rom"),
                std::path::Path::new("roms/sa1.rom"),
                std::path::Path::new("roms/ipl.rom"),
            ];
            for path in candidates.iter() {
                if let Ok(data) = std::fs::read(path) {
                    // Real SA-1 IPL is exactly 0x800 bytes. Reject other sizes to avoid
                    // accidentally treating a full game ROM or placeholder as the IPL.
                    if data.len() == 0x800 {
                        self.sa1_iram
                            .iter_mut()
                            .zip(data.iter())
                            .for_each(|(dst, src)| *dst = *src);
                        self.sa1.registers.reset_vector = 0x0000;
                        self.sa1.registers.control = 0x20;
                        ipl_loaded = true;
                        if debug {
                            println!(
                                "[SA1] Loaded external IPL from {:?} ({} bytes)",
                                path,
                                data.len()
                            );
                        }
                        break;
                    } else if debug {
                        println!(
                            "[SA1] Ignoring IPL candidate {:?} ({} bytes, expected 2048)",
                            path,
                            data.len()
                        );
                    }
                }
            }
        }

        // HLE fallback IPL: when no external IPL is present, seed IRAM with a tiny stub
        // that jumps to the ROM reset vector and also sets the handshake flags the real
        // IPL would assert (BW-RAM ready / DMA done).
        if self.is_sa1_active() && !ipl_loaded {
            // Build a 65c816 long jump to the ROM reset vector we just fetched.
            // 0000: JML $00:FC48
            self.sa1_iram[0] = 0x5C;
            self.sa1_iram[1] = (reset_vec & 0xFF) as u8;
            self.sa1_iram[2] = (reset_vec >> 8) as u8;
            self.sa1_iram[3] = 0x00;
            self.sa1.registers.reset_vector = 0x0000;
            self.sa1.registers.control = 0x20;
            // Signal “DMA complete / BW-RAM ready” and raise SA-1→S-CPU IRQ once,
            // which matches the observable post-IPL state many games (DQ3含む) rely on.
            self.sa1.registers.sie |= Sa1::IRQ_LINE_BIT;
            self.sa1.registers.interrupt_enable = self.sa1.registers.sie;
            self.sa1.registers.interrupt_pending |= Sa1::IRQ_DMA_FLAG | Sa1::IRQ_LINE_BIT;
            if debug {
                println!("[SA1] HLE IPL injected (stub jump to {:04X})", reset_vec);
            }
        }

        // Note: do NOT auto-assert SA-1 IRQ/SIE here. Dragon Quest III expects the
        // internal IPL to finish copying BW-RAM stubs before the first IRQ is raised.
        // The old code forced an early IRQ, which jumped into uninitialized BW-RAM
        // (vector 00:7222) and halted progress. If you need the legacy behavior for
        // debugging, set DQ3_FAKE_IPL=1.
        if self.mapper_type == crate::cartridge::MapperType::DragonQuest3
            && std::env::var("DQ3_FAKE_IPL")
                .map(|v| v == "1" || v.to_lowercase() == "true")
                .unwrap_or(false)
        {
            self.sa1.registers.sie = Sa1::IRQ_LINE_BIT;
            self.sa1.registers.interrupt_enable = self.sa1.registers.sie;
            self.sa1.registers.interrupt_pending |= Sa1::IRQ_DMA_FLAG | Sa1::IRQ_LINE_BIT;
        }
        // Immediately position SA-1 core at reset vector (avoid pending_reset wiping flags)
        self.sa1.cpu.emulation_mode = false;
        self.sa1.cpu.p = crate::cpu::StatusFlags::from_bits_truncate(0x34);
        self.sa1
            .cpu
            .core
            .reset(crate::cpu::StatusFlags::from_bits_truncate(0x34), false);
        self.sa1.cpu.pb = self.sa1.boot_pb;
        self.sa1.cpu.pc = self.sa1.registers.reset_vector;
        self.sa1.cpu.sync_core_from_cpu();
        self.sa1.boot_vector_applied = true;
        self.sa1.pending_reset = false;
        self.sa1.ipl_ran = true;
        if debug {
            println!(
                "[SA1] init vectors from ROM: reset={:04X} nmi={:04X} irq={:04X}",
                reset_vec, nmi_vec, irq_vec
            );
        }
    }

    /// Force-start SA-1 execution (DQ3-only debug helper).
    pub fn force_sa1_boot(&mut self) {
        if !self.is_sa1_active() {
            return;
        }
        self.init_sa1_vectors_from_rom();
        // Enable SA-1 IRQ handling and assert S-CPU->SA-1 IRQ request (SCNT bit5).
        self.sa1.registers.control |= 0x01;
        self.sa1.registers.scnt |= 0x20;
        self.sa1.cpu.pb = 0x00;
        self.sa1.cpu.pc = self.sa1.registers.reset_vector;
        self.sa1.boot_vector_applied = true;
        self.sa1.cpu.sync_core_from_cpu();
        // Optional: force an SA-1 IRQ to S-CPU once, for debugging DQ3 stalls
        if crate::debug_flags::sa1_force_irq_once() {
            self.sa1.registers.interrupt_pending |= Sa1::IRQ_LINE_BIT;
            println!("[debug] SA-1 forced IRQ once (IRQ_LINE_BIT)");
        }
        if std::env::var_os("TRACE_SA1_BOOT").is_some() {
            println!(
                "DQ3 force SA-1 boot: PB={:02X} PC={:04X} ctrl=0x{:02X} scnt=0x{:02X}",
                self.sa1.cpu.pb,
                self.sa1.cpu.pc,
                self.sa1.registers.control,
                self.sa1.registers.scnt
            );
        }
    }

    /// Run the SA-1 core for a slice of time proportional to the S-CPU cycles just executed.
    /// We use a coarse 3:1 frequency ratio (SA-1 ~10.74MHz vs S-CPU 3.58MHz).
    pub fn run_sa1_scheduler(&mut self, cpu_cycles: u8) {
        if !self.is_sa1_active() {
            return;
        }

        // Optional: dump SA-1 IRAM/BWRAM head once for debugging (DQ3調査)
        if std::env::var_os("TRACE_SA1_MEM").is_some() {
            use std::sync::atomic::{AtomicBool, Ordering};
            use std::sync::OnceLock;
            static DUMPED: OnceLock<AtomicBool> = OnceLock::new();
            let flag = DUMPED.get_or_init(|| AtomicBool::new(false));
            if !flag.swap(true, Ordering::SeqCst) {
                let iram_head: Vec<u8> = self.sa1_iram.iter().take(64).copied().collect();
                let bwram_head: Vec<u8> = self.sa1_bwram.iter().take(64).copied().collect();
                // Also dump area around 00:7DE0 (DQ3 polls there)
                let bw_idx = 0x07DE0usize;
                let bw_slice: Vec<u8> = self
                    .sa1_bwram
                    .iter()
                    .skip(bw_idx)
                    .take(32)
                    .copied()
                    .collect();
                println!(
                    "[SA1-MEM] IRAM[0..64]={:02X?}\n[SA1-MEM] BWRAM[0..64]={:02X?}\n[SA1-MEM] BWRAM[0x07DE0..]={:02X?}",
                    iram_head, bwram_head, bw_slice
                );
            }
        }

        // DQ3: 毎スライスでSA-1マスク/ラインを強制ON（CIE/SIE/CONTROL）
        if self.mapper_type == crate::cartridge::MapperType::DragonQuest3 {
            let force_ctrl = std::env::var("DQ3_FORCE_SA1_CTRL")
                .map(|v| v == "1" || v.to_lowercase() == "true")
                .unwrap_or(false);
            if force_ctrl {
                self.sa1.registers.control |= 0xF0; // IRQ/NMI select bits ON
                self.sa1.registers.cie = 0xE0; // enable IRQ/NMI/TMR toward SA-1 core
                self.sa1.registers.sie = 0xA0 | Sa1::IRQ_LINE_BIT; // enable IRQ + DMA flag toward S-CPU
                self.sa1.registers.interrupt_enable = self.sa1.registers.sie;
                // Wake SA-1 core if WAI/STP latched previously
                self.sa1.cpu.core.state.waiting_for_irq = false;
                self.sa1.cpu.core.state.stopped = false;
                if std::env::var_os("TRACE_SA1_CTRL").is_some() {
                    use std::sync::atomic::{AtomicU32, Ordering};
                    static COUNT: AtomicU32 = AtomicU32::new(0);
                    let n = COUNT.fetch_add(1, Ordering::Relaxed);
                    if n < 16 {
                        println!(
                            "[SA1-CTRL] ctrl=0x{:02X} cie=0x{:02X} sie=0x{:02X} pending=0x{:02X} pb={:02X} pc={:04X} wai={} stp={} sfr=0x{:02X}",
                            self.sa1.registers.control,
                            self.sa1.registers.cie,
                            self.sa1.registers.sie,
                            self.sa1.registers.interrupt_pending,
                            self.sa1.cpu.pb,
                            self.sa1.cpu.pc,
                            self.sa1.cpu.core.state.waiting_for_irq,
                            self.sa1.cpu.core.state.stopped,
                            self.sa1.registers.sfr,
                        );
                    }
                }
            }
        }

        // DQ3用: 簡易IPLスタブ。SA-1を強制的に起こし、S-CPUへIRQラインを立てる。
        // 環境変数 DQ3_SA1_IPL_STUB=1 で一度だけ実行。
        if std::env::var_os("DQ3_SA1_IPL_STUB").is_some() {
            use std::sync::Once;
            static STUB_ONCE: Once = Once::new();
            STUB_ONCE.call_once(|| {
                // Enable S-CPU IRQ mask for SA-1 line
                self.sa1.registers.sie |= Sa1::IRQ_LINE_BIT;
                self.sa1.registers.interrupt_enable = self.sa1.registers.sie;
                // Assert IRQ pending toward S-CPU immediately
                self.sa1.registers.interrupt_pending |= Sa1::IRQ_LINE_BIT;
                // Clear WAI state so SA-1 core can run
                self.sa1.cpu.core.state.waiting_for_irq = false;
                self.sa1.cpu.core.state.stopped = false;
                // Also raise main CPU IRQ line once
                self.irq_pending = true;
                println!("⚡ DQ3_SA1_IPL_STUB: forced SA-1 IRQ line to S-CPU (one-shot)");
            });
        }

        // Optional debug hack: force SA-1 IRQ line each slice to escape WAI loops (DQ3)
        if std::env::var_os("DQ3_SA1_IRQ_HACK").is_some() {
            // Enable IRQ line in control/CIE/SIE
            self.sa1.registers.control |= 0x80;
            self.sa1.registers.cie |= 0x80;
            self.sa1.registers.sie |= 0x80;
            self.sa1.registers.interrupt_enable = self.sa1.registers.sie;
            // Assert IRQ pending toward S-CPU and SA-1 CPU
            self.sa1.registers.interrupt_pending |= Sa1::IRQ_LINE_BIT;
            self.sa1.registers.interrupt_pending |= Sa1::IRQ_DMA_FLAG;
            // Wake SA-1 core out of WAI/STP if set
            self.sa1.cpu.core.state.waiting_for_irq = false;
            self.sa1.cpu.core.state.stopped = false;
            // 併せてS-CPU側IRQラインも立ててIRQを起こす（Iフラグが下りていれば即応）
            self.irq_pending = true;
            static DQ3_IRQ_HACK_LOG: OnceLock<AtomicU32> = OnceLock::new();
            let n = DQ3_IRQ_HACK_LOG
                .get_or_init(|| AtomicU32::new(0))
                .fetch_add(1, Ordering::Relaxed);
            if n < 8 {
                println!("⚡ DQ3_SA1_IRQ_HACK: forcing SA-1 IRQ (count={})", n + 1);
            }
        }

        // DQ3 IPL強化: SFR bit7（SA-1→S-CPU IRQライン）が立つまで SCNT をパルスし続ける
        //   - bit7 を立てる（IRQ要求）
        //   - 下位ニブルをインクリメントしてメッセージ変化を知らせる
        //   - bit6/bit5 を常時1にして SNV/SIV 有効を強制
        // 環境変数 DQ3_SA1_SCNT_PULSE=0 で無効化。デフォルト有効。
        if self.mapper_type == crate::cartridge::MapperType::DragonQuest3 {
            let pulse_enabled = std::env::var("DQ3_SA1_SCNT_PULSE")
                .map(|v| v == "1" || v.to_lowercase() == "true")
                .unwrap_or(false);
            if pulse_enabled && !self.sa1.scpu_irq_asserted() {
                // 低ニブルを回転させてメッセージ変化を伝える
                let msg = (self.sa1.registers.scnt.wrapping_add(1)) & 0x0F;
                // bit5=SIV enable, bit6=SNV enable を常時ON、bit7=IRQ要求
                self.sa1.registers.scnt = 0xE0 | msg | 0x80;
                // Set SCNT bit7 (message bit to S-CPU) and assert IRQ line pending
                self.sa1.registers.sie |= Sa1::IRQ_LINE_BIT;
                self.sa1.registers.interrupt_enable = self.sa1.registers.sie;
                self.sa1.registers.interrupt_pending |= Sa1::IRQ_LINE_BIT;
                // Also raise the S-CPU TIMEUP flag to make sure IRQ is noticed
                self.irq_pending = true;
                // Wake SA-1 core from WAI/STP
                self.sa1.cpu.core.state.waiting_for_irq = false;
                self.sa1.cpu.core.state.stopped = false;
                use std::sync::atomic::{AtomicU32, Ordering};
                static PULSE_LOG: AtomicU32 = AtomicU32::new(0);
                let n = PULSE_LOG.fetch_add(1, Ordering::Relaxed);
                if n < 6 {
                    println!(
                        "⚡ DQ3_SA1_SCNT_PULSE: SCNT=0x{:02X} (msg={}, SNV/SIV toggle) count={}",
                        self.sa1.registers.scnt,
                        msg,
                        n + 1
                    );
                }
            }
        }

        // Ensure vectors are seeded from ROM header at first use
        if !self.sa1.boot_vector_applied && self.sa1.registers.reset_vector == 0 {
            self.init_sa1_vectors_from_rom();
        }

        const SA1_RATIO_NUM: i64 = 3;
        const SA1_RATIO_DEN: i64 = 1;
        const SA1_MAX_STEPS: usize = 256; // Increased from 128 to allow more SA-1 execution

        self.sa1_cycle_deficit += (cpu_cycles as i64) * SA1_RATIO_NUM;

        // Log SA-1 reset vector on first run
        static FIRST_RUN: std::sync::Once = std::sync::Once::new();
        FIRST_RUN.call_once(|| {
            if std::env::var_os("DEBUG_SA1_SCHEDULER").is_some()
                || std::env::var_os("TRACE_SA1_BOOT").is_some()
            {
                println!(
                    "SA-1 first run: reset_vector=0x{:04X} PC=${:02X}:{:04X} boot_applied={}",
                    self.sa1.registers.reset_vector,
                    self.sa1.cpu.pb,
                    self.sa1.cpu.pc,
                    self.sa1.boot_vector_applied
                );
            }
        });

        let mut steps = 0usize;
        let mut total_sa1_cycles = 0u32;
        let mut wake_trace_left = crate::debug_flags::trace_sa1_wake_steps().unwrap_or(0);
        while self.sa1_cycle_deficit >= SA1_RATIO_DEN && steps < SA1_MAX_STEPS {
            let sa1_cycles = unsafe {
                let bus_ptr = self as *mut Bus;
                let sa1_ptr = &mut self.sa1 as *mut Sa1;
                (*sa1_ptr).step(&mut *bus_ptr)
            } as i64;

            if sa1_cycles <= 0 {
                if std::env::var_os("DEBUG_SA1_SCHEDULER").is_some() && steps == 0 {
                    println!(
                        "SA-1 scheduler: step returned 0 cycles at PC=${:02X}:{:04X}",
                        self.sa1.cpu.pb, self.sa1.cpu.pc
                    );
                }
                break;
            }

            total_sa1_cycles = total_sa1_cycles.saturating_add(sa1_cycles as u32);

            // Optional wake trace: print first N instructions after forced IRQ poke
            if wake_trace_left > 0 {
                println!(
                    "[SA1-wake] PB={:02X} PC={:04X} cycles={} ctrl=0x{:02X} scnt=0x{:02X}",
                    self.sa1.cpu.pb,
                    self.sa1.cpu.pc,
                    sa1_cycles,
                    self.sa1.registers.control,
                    self.sa1.registers.scnt
                );
                wake_trace_left -= 1;
            }

            // Check if SA-1 is in WAI or STP state - if so, break early to avoid spinning
            if self.sa1.cpu.core.state.waiting_for_irq || self.sa1.cpu.core.state.stopped {
                // DQ3 sometimes leaves SA-1 in WAI without the S-CPU asserting SCNT IRQ.
                // For this mapper, gently poke the SA-1 IRQ line (CONTROL bit7) to wake it.
                if self.mapper_type == crate::cartridge::MapperType::DragonQuest3 {
                    // Option: ignore WAI and brute-force step a few instructions
                    if std::env::var("DQ3_SA1_IGNORE_WAI")
                        .map(|v| v == "1" || v.to_lowercase() == "true")
                        .unwrap_or(false)
                    {
                        // If stuck exactly on WAI opcode, skip it by advancing PC
                        let pc_full = ((self.sa1.cpu.pb as u32) << 16) | (self.sa1.cpu.pc as u32);
                        let opcode = self.sa1_read_u8(pc_full);
                        if opcode == 0xCB {
                            self.sa1.cpu.pc = self.sa1.cpu.pc.wrapping_add(1);
                            self.sa1.cpu.core.state.waiting_for_irq = false;
                            self.sa1.cpu.core.state.stopped = false;
                        }
                        let mut skip_steps = 8u32;
                        while skip_steps > 0 {
                            let extra = unsafe {
                                let bus_ptr = self as *mut Bus;
                                let sa1_ptr = &mut self.sa1 as *mut Sa1;
                                (*sa1_ptr).step(&mut *bus_ptr)
                            } as i64;
                            if extra <= 0 {
                                break;
                            }
                            total_sa1_cycles = total_sa1_cycles.saturating_add(extra as u32);
                            self.sa1_cycle_deficit -= extra * SA1_RATIO_DEN;
                            steps += 1;
                            skip_steps -= 1;
                        }
                        self.sa1.cpu.core.state.waiting_for_irq = false;
                        self.sa1.cpu.core.state.stopped = false;
                        continue;
                    }
                    self.sa1.registers.control |= 0x80;
                    self.sa1.registers.cie = 0xE0; // force IRQ/NMI/TMR enable
                    self.sa1.registers.sie = 0xA0 | Sa1::IRQ_LINE_BIT; // ensure S-CPU mask too
                    self.sa1.registers.interrupt_enable = self.sa1.registers.sie;
                    // Also assert an internal DMA-flag IRQ so poll_irq returns true.
                    self.sa1.registers.interrupt_pending |= Sa1::IRQ_DMA_FLAG;
                    // One-shot wake: force IRQ line if not already set
                    if (self.sa1.registers.interrupt_pending & Sa1::IRQ_LINE_BIT) == 0 {
                        self.sa1.registers.interrupt_pending |= Sa1::IRQ_LINE_BIT;
                        if std::env::var_os("DEBUG_SA1_SCHEDULER").is_some() {
                            println!(
                                "[SA1] forced IRQ line while WAI at ${:02X}:{:04X}",
                                self.sa1.cpu.pb, self.sa1.cpu.pc
                            );
                        }
                    }
                    // Optionally inject IRQ directly into SA-1 CPU core to break WAI
                    let core_irq_enabled = std::env::var("DQ3_SA1_CORE_IRQ")
                        .map(|v| v == "1" || v.to_lowercase() == "true")
                        .unwrap_or(true);
                    if core_irq_enabled {
                        self.sa1.cpu.p.remove(crate::cpu::StatusFlags::IRQ_DISABLE);
                        self.sa1
                            .cpu
                            .core
                            .state_mut()
                            .p
                            .remove(crate::cpu::StatusFlags::IRQ_DISABLE);
                        self.sa1.cpu.core.state.waiting_for_irq = false;
                        self.sa1.cpu.core.state.stopped = false;
                        // Force the adapter's poll_irq() to see a pending IRQ + SNV/SIV enable
                        self.sa1.registers.control |= 0x90; // IRQ line + SNV select
                        self.sa1.registers.cie = 0xE0; // force enable IRQ/NMI/TMR toward SA-1 core
                        self.sa1.registers.sie = 0xA0 | Sa1::IRQ_LINE_BIT; // S-CPU mask fully on
                        self.sa1.registers.interrupt_enable = self.sa1.registers.sie;
                        self.sa1.registers.interrupt_pending |= Sa1::IRQ_LINE_BIT;
                    }
                    // After poking, try one extra step immediately to give SA-1 a chance to run.
                    let extra = unsafe {
                        let bus_ptr = self as *mut Bus;
                        let sa1_ptr = &mut self.sa1 as *mut Sa1;
                        (*sa1_ptr).step(&mut *bus_ptr)
                    } as i64;
                    if extra > 0 {
                        total_sa1_cycles = total_sa1_cycles.saturating_add(extra as u32);
                        self.sa1_cycle_deficit -= extra * SA1_RATIO_DEN;
                        steps += 1;
                        if wake_trace_left > 0 {
                            println!(
                                "[SA1-wake] PB={:02X} PC={:04X} cycles={} ctrl=0x{:02X} scnt=0x{:02X}",
                                self.sa1.cpu.pb, self.sa1.cpu.pc, extra, self.sa1.registers.control,
                                self.sa1.registers.scnt
                            );
                            wake_trace_left = wake_trace_left.saturating_sub(1);
                        }
                        continue;
                    }
                }
                if std::env::var_os("DEBUG_SA1_SCHEDULER").is_some() {
                    println!(
                        "SA-1 scheduler: breaking at step {} (WAI={} STP={} PC=${:02X}:{:04X})",
                        steps,
                        self.sa1.cpu.core.state.waiting_for_irq,
                        self.sa1.cpu.core.state.stopped,
                        self.sa1.cpu.pb,
                        self.sa1.cpu.pc
                    );
                }
                break;
            }

            if std::env::var_os("TRACE_SA1_STEP").is_some() && steps < 64 {
                println!(
                    "SA1 STEP {} PB={:02X} PC={:04X} cycles={} ctrl=0x{:02X} scnt=0x{:02X} WAI={} STP={}",
                    steps + 1,
                    self.sa1.cpu.pb,
                    self.sa1.cpu.pc,
                    sa1_cycles,
                    self.sa1.registers.control,
                    self.sa1.registers.scnt,
                    self.sa1.cpu.core.state.waiting_for_irq,
                    self.sa1.cpu.core.state.stopped,
                );
            }

            self.sa1_cycle_deficit -= sa1_cycles * SA1_RATIO_DEN;
            steps += 1;
        }

        // Tick SA-1 timers with accumulated cycles
        if total_sa1_cycles > 0 {
            self.sa1.tick_timers(total_sa1_cycles);
        }

        // Log statistics every 1000 steps
        if std::env::var_os("DEBUG_SA1_SCHEDULER").is_some() {
            static mut STEP_COUNT: usize = 0;
            unsafe {
                STEP_COUNT += steps;
                if STEP_COUNT >= 1000 {
                    println!(
                        "SA-1 scheduler: {} total steps executed, PC=${:02X}:{:04X}",
                        STEP_COUNT, self.sa1.cpu.pb, self.sa1.cpu.pc
                    );
                    STEP_COUNT = 0;
                }
            }
        }
    }

    /// Process pending SA-1 DMA/CC-DMA transfers and notify S-CPU via IRQ
    pub fn process_sa1_dma(&mut self) {
        if !self.is_sa1_active() {
            return;
        }

        // Check for pending normal DMA
        if self.sa1.is_dma_pending() {
            if crate::debug_flags::trace_sa1_dma() {
                println!(
                    "SA1_DMA: Normal DMA pending src=0x{:06X} dest=0x{:06X} len=0x{:04X}",
                    self.sa1.registers.dma_source,
                    self.sa1.registers.dma_dest,
                    self.sa1.registers.dma_length
                );
            }
            // Execute a simple byte copy for now.
            // Avoid borrow conflicts by splitting the call: first copy using a helper that
            // doesn't borrow sa1 mutably after taking &mut self.
            {
                // SAFETY: run_sa1_normal_dma is a small helper that only uses &mut Bus.
                // We'll define it below to perform the copy while reading sa1 registers.
                self.run_sa1_normal_dma_copy();
            }
            let irq_fired = {
                let sa1 = &mut self.sa1;
                sa1.complete_dma()
            };
            if irq_fired && crate::debug_flags::trace_sa1_dma() {
                println!("SA1_DMA: Normal DMA complete, IRQ fired to S-CPU");
            }
        }

        // Check for pending CC-DMA
        if self.sa1.is_ccdma_pending() || self.sa1.registers.ccdma_buffer_ready {
            if crate::debug_flags::trace_sa1_ccdma() {
                self.sa1.log_ccdma_state("process_begin");
                self.sa1.trace_ccdma_transfer("begin");
                println!(
                    "SA1_CCDMA: Processing CC-DMA src=0x{:06X} dest=0x{:06X} len=0x{:04X}",
                    self.sa1.registers.dma_source,
                    self.sa1.registers.dma_dest,
                    self.sa1.registers.dma_length
                );
            }

            // Perform CC-DMA conversion
            self.perform_sa1_ccdma();

            // Complete CC-DMA and possibly fire IRQ to S-CPU
            let irq_fired = self.sa1.complete_ccdma();
            if crate::debug_flags::trace_sa1_ccdma() {
                self.sa1.log_ccdma_state("process_complete");
                self.sa1.trace_ccdma_transfer("complete");
                if irq_fired {
                    println!("SA1_CCDMA: CC-DMA complete, IRQ fired to S-CPU");
                } else {
                    println!("SA1_CCDMA: CC-DMA complete, no IRQ");
                }
            }
        }
    }

    /// Perform SA-1 character conversion DMA (type 0/1): linear pixels -> SNES bitplane tiles.
    fn perform_sa1_ccdma(&mut self) {
        let src = self.sa1.registers.dma_source;
        let dest = self.sa1.registers.dma_dest;
        let len = self.sa1.registers.dma_length as usize;

        let typ = self.sa1.ccdma_type().unwrap_or(0);
        if len == 0 {
            // For CC-DMA type2 allow zero-length as a single tile write
            if typ != 2 {
                return;
            }
        }
        // Resolve source/dest devices from DCNT (CPMT-DSS: SS bits0-1, D bit2)
        let dcnt = self.sa1.registers.dma_control;
        let src_dev = dcnt & 0x03; // 0:ROM,1:BWRAM,2:IRAM
        let dst_dev = (dcnt >> 2) & 0x01; // 0:IRAM,1:BWRAM

        // Type0/1: bitmap -> planar tile conversion
        let depth_bits = self.sa1.ccdma_color_depth_bits().unwrap_or(4);

        let bytes_per_tile_src = 64; // 8x8 pixels, 1 byte per pixel in source bitmap
        let bytes_per_tile_dst = (depth_bits / 2) * 16; // 2 bytes per plane per row * 8 rows

        if typ == 2 {
            // Type2: BRF (two 8-byte rows) -> planar tile written to SA-1 dest
            let tile_dst = dest.wrapping_add(self.sa1.registers.brf_tile_offset);
            let bytes_per_row_out = depth_bits as u32 * 2; // per SNES tile row
            for row in 0..2 {
                let start = row * 8;
                let mut line = [0u8; 8];
                line.copy_from_slice(&self.sa1.registers.brf[start..start + 8]);
                let row_off = (row as u32) * bytes_per_row_out;
                let mask = (1u8 << depth_bits) - 1;
                let mut out_idx = row_off;
                for plane in (0..depth_bits).step_by(2) {
                    let mut byte_lo = 0u8;
                    let mut byte_hi = 0u8;
                    for x in 0..8 {
                        let val = line[x] & mask;
                        byte_lo |= ((val >> plane) & 1) << (7 - x);
                        byte_hi |= ((val >> (plane + 1)) & 1) << (7 - x);
                    }
                    self.sa1_write_u8(tile_dst.wrapping_add(out_idx), byte_lo);
                    self.sa1_write_u8(tile_dst.wrapping_add(out_idx + 1), byte_hi);
                    out_idx += 2;
                }
            }
            let adv = bytes_per_tile_dst as u32;
            self.sa1.registers.brf_tile_offset =
                self.sa1.registers.brf_tile_offset.wrapping_add(adv);

            // conversion consumed the BRF tile; clear buffer/flag
            self.sa1.registers.brf.fill(0);
            self.sa1.registers.brf_pos = 0;
            self.sa1.registers.ccdma_pending = false;
            self.sa1.registers.ccdma_buffer_ready = false;
            if crate::debug_flags::trace_sa1_ccdma() {
                println!(
                    "SA1_CCDMA(type2 convert depth={}bpp) dest=0x{:06X} brf_off=0x{:06X}",
                    depth_bits, tile_dst, self.sa1.registers.brf_tile_offset
                );
            }
        } else {
            // Type0/1: normal bitmap -> tile conversion
            let tiles = len / bytes_per_tile_src;
            for t in 0..tiles {
                let tile_src = src.wrapping_add((t * bytes_per_tile_src) as u32);
                let tile_dst = dest.wrapping_add((t * bytes_per_tile_dst as usize) as u32);

                // load 8x8 pixels
                let mut pix = [0u8; 64];
                for i in 0..64 {
                    pix[i] = self.sa1_dma_read_device(src_dev, tile_src.wrapping_add(i as u32));
                }

                // write bitplanes
                let mut out_idx = 0usize;
                for y in 0..8 {
                    for plane in 0..depth_bits {
                        // SNES packs two planes per byte pair; plane 0/1,2/3,4/5,6/7
                        if plane % 2 == 0 {
                            let p0 = plane;
                            let p1 = plane + 1;
                            let mut byte_lo = 0u8;
                            let mut byte_hi = 0u8;
                            for x in 0..8 {
                                let val = pix[y * 8 + x];
                                byte_lo |= ((val >> p0) & 1) << (7 - x);
                                byte_hi |= ((val >> p1) & 1) << (7 - x);
                            }
                            self.sa1_dma_write_device(
                                dst_dev,
                                tile_dst.wrapping_add(out_idx as u32),
                                byte_lo,
                            );
                            self.sa1_dma_write_device(
                                dst_dev,
                                tile_dst.wrapping_add(out_idx as u32 + 1),
                                byte_hi,
                            );
                            out_idx += 2;
                        }
                    }
                }
            }

            if crate::debug_flags::trace_sa1_ccdma() {
                println!(
                    "SA1_CCDMA(type{} convert depth={}bpp) tiles={} src=0x{:06X} dest=0x{:06X}",
                    typ, depth_bits, tiles, src, dest
                );
            }
        }
    }

    /// Helper to run simple SA-1 normal DMA copy (ROM->BWRAM) without double-borrowing sa1.
    fn run_sa1_normal_dma_copy(&mut self) {
        if !self.sa1.dma_is_normal_public() || !self.sa1.registers.dma_pending {
            return;
        }
        let len = self.sa1.registers.dma_length as usize;
        if len == 0 {
            self.sa1.registers.dma_pending = false;
            return;
        }
        let dcnt = self.sa1.registers.dma_control;
        // DCNT CPMT-DSS: SS=src, D=dest
        let src_dev = dcnt & 0x03; // 0:ROM,1:BWRAM,2:IRAM
        let dst_dev = (dcnt >> 2) & 0x01; // 0:IRAM,1:BWRAM

        let src_addr = self.sa1.registers.dma_source as usize;
        let dst_addr = self.sa1.registers.dma_dest as usize;

        let read_src = |bus: &mut Bus, idx: usize| -> u8 {
            match src_dev {
                0 => {
                    // ROM via SA-1 mapping
                    let addr = src_addr + idx;
                    let bank = (addr >> 16) as u32 & 0xFF;
                    let off = addr as u16;
                    let phys = bus.sa1_phys_addr(bank, off);
                    bus.rom.get(phys % bus.rom_size).copied().unwrap_or(0xFF)
                }
                1 => {
                    if bus.sa1_bwram.is_empty() {
                        0xFF
                    } else {
                        let bank = (src_addr >> 16) & 0xFF;
                        let off = src_addr & 0xFFFF;
                        let base = (((bank & 0x1F) << 16) | off) as usize;
                        let di = (base + idx) % bus.sa1_bwram.len();
                        bus.sa1_bwram[di]
                    }
                }
                2 => {
                    let base = src_addr & 0x7FF;
                    bus.sa1_iram[(base + idx) % bus.sa1_iram.len()]
                }
                _ => 0xFF,
            }
        };

        let write_dst = |bus: &mut Bus, idx: usize, val: u8| match dst_dev {
            0 => {
                let base = dst_addr & 0x7FF;
                let di = (base + idx) % bus.sa1_iram.len();
                bus.sa1_iram[di] = val;
            }
            1 => {
                if !bus.sa1_bwram.is_empty() {
                    let bank = (dst_addr >> 16) & 0xFF;
                    let off = dst_addr & 0xFFFF;
                    let base = (((bank & 0x1F) << 16) | off) as usize;
                    let di = (base + idx) % bus.sa1_bwram.len();
                    bus.sa1_bwram[di] = val;
                }
            }
            _ => {}
        };

        for i in 0..len {
            let v = read_src(self, i);
            write_dst(self, i, v);
        }
        self.sa1.registers.dma_pending = false;
        self.sa1.registers.dma_control &= !0x80;
        // Flag completion for both CPUs
        self.sa1.registers.interrupt_pending |= Sa1::IRQ_DMA_FLAG;
        if (self.sa1.registers.interrupt_enable & Sa1::IRQ_LINE_BIT) != 0 {
            self.sa1.registers.interrupt_pending |= Sa1::IRQ_LINE_BIT;
        }
    }

    #[inline]
    #[allow(dead_code)]
    pub fn sa1_bwram_slice(&self) -> &[u8] {
        &self.sa1_bwram
    }

    #[allow(dead_code)]
    pub fn sa1_bwram_slice_mut(&mut self) -> &mut [u8] {
        &mut self.sa1_bwram
    }

    #[inline]
    #[allow(dead_code)]
    pub fn sa1_iram_slice(&self) -> &[u8] {
        &self.sa1_iram
    }

    #[inline]
    #[allow(dead_code)]
    pub fn sa1_iram_slice_mut(&mut self) -> &mut [u8] {
        &mut self.sa1_iram
    }

    /// SA-1 DMA helper: read from ROM/BWRAM/IRAM based on device selector.
    #[inline]
    fn sa1_dma_read_device(&self, dev: u8, addr: u32) -> u8 {
        match dev {
            0 => {
                let phys = self.sa1_phys_addr((addr >> 16) & 0xFF, addr as u16);
                self.rom.get(phys % self.rom_size).copied().unwrap_or(0xFF)
            }
            1 => {
                if self.sa1_bwram.is_empty() {
                    0xFF
                } else {
                    let idx = addr as usize % self.sa1_bwram.len();
                    self.sa1_bwram[idx]
                }
            }
            2 => {
                let off = (addr & 0x1FFF) as usize;
                self.sa1_iram
                    .get(off % self.sa1_iram.len())
                    .copied()
                    .unwrap_or(0xFF)
            }
            _ => 0xFF,
        }
    }

    /// SA-1 DMA helper: write to BWRAM/IRAM ignoring bank mapping when targeting BW-RAM.
    #[inline]
    fn sa1_dma_write_device(&mut self, dev: u8, addr: u32, val: u8) {
        match dev {
            0 => {
                let off = (addr & 0x1FFF) as usize;
                if off < self.sa1_iram.len() {
                    self.sa1_iram[off] = val;
                }
            }
            1 => {
                if !self.sa1_bwram.is_empty() {
                    let idx = addr as usize % self.sa1_bwram.len();
                    self.sa1_bwram[idx] = val;
                }
            }
            _ => {}
        }
    }

    /// Copy a slice from SA-1 ROM into SA-1 IRAM (used to emulate the missing SA-1 IPL).
    fn copy_sa1_iram_from_rom(&mut self, bank: u8, offset: u16, len: usize) {
        let dst = &mut self.sa1_iram;
        let mut remaining = len.min(dst.len());
        let mut off = offset as usize;
        let mut written = 0usize;
        while remaining > 0 {
            let phys = {
                let b = bank as u32;
                let o = (off & 0xFFFF) as u16;
                // Compute without borrowing dst
                let base = if (0x00..=0x1F).contains(&b)
                    || (0x20..=0x3F).contains(&b)
                    || (0x80..=0x9F).contains(&b)
                    || (0xA0..=0xBF).contains(&b)
                {
                    let chunk = match b {
                        0x00..=0x1F => self.sa1.registers.mmc_bank_c,
                        0x20..=0x3F => self.sa1.registers.mmc_bank_d,
                        0x80..=0x9F => self.sa1.registers.mmc_bank_e,
                        _ => self.sa1.registers.mmc_bank_f,
                    } as usize;
                    let off = (o | 0x8000) as usize;
                    let bank_lo = (b & 0x1F) as usize;
                    chunk * 0x100000 + bank_lo * 0x8000 + (off - 0x8000)
                } else {
                    let chunk = match b {
                        0xC0..=0xCF => self.sa1.registers.mmc_bank_c,
                        0xD0..=0xDF => self.sa1.registers.mmc_bank_d,
                        0xE0..=0xEF => self.sa1.registers.mmc_bank_e,
                        _ => self.sa1.registers.mmc_bank_f,
                    } as usize;
                    chunk * 0x100000 + o as usize
                };
                base
            };
            let byte = self.rom.get(phys % self.rom_size).copied().unwrap_or(0x00);
            dst[written] = byte;
            written += 1;
            off = off.wrapping_add(1);
            remaining -= 1;
        }
        if std::env::var_os("TRACE_SA1_BOOT").is_some() {
            println!(
                "[SA1] IRAM filled from ROM bank {:02X} offset 0x{:04X} len=0x{:04X}",
                bank, offset, len
            );
        }
    }

    /// DQ3専用: SA-1 WAIループ地点(0C:CCB7)を NOP/BRA -2 に置き換え、永続WAIを回避
    fn patch_sa1_wai_loop(&mut self) {
        if self.mapper_type != crate::cartridge::MapperType::DragonQuest3 {
            return;
        }
        // 対象アドレス
        let bank = 0x0C;
        let addr: u16 = 0xCCB7;

        // まずIRAMに入っている場合だけ直接パッチ
        let iram_idx = (addr as usize) & 0x7FF; // IRAMは0x0000-07FF
        if iram_idx + 3 <= self.sa1_iram.len() {
            let patch = [0xEA, 0x80, 0xFE]; // NOP; BRA -2
            self.sa1_iram[iram_idx..iram_idx + 3].copy_from_slice(&patch);
            if std::env::var_os("TRACE_SA1_BOOT").is_some() {
                println!(
                    "[SA1] Patched WAI loop in IRAM at 0C:{:04X} -> NOP/BRA",
                    addr
                );
            }
            return;
        }

        // IRAMに無い場合はBWRAMミラーへ（bank & 0x1Fでページ決定）
        let idx = ((bank & 0x1F) as usize) * 0x10000 + (addr as usize);
        if idx + 3 <= self.sa1_bwram.len() {
            let patch = [0xEA, 0x80, 0xFE];
            self.sa1_bwram[idx..idx + 3].copy_from_slice(&patch);
            if std::env::var_os("TRACE_SA1_BOOT").is_some() {
                println!(
                    "[SA1] Patched WAI loop in BWRAM mirror at 0C:{:04X} (idx=0x{:05X})",
                    addr, idx
                );
            }
        }
    }

    /// Minimal SA-1 IPL stub: JML to given bank:address. Fills IRAM with 0xFF first.
    fn write_sa1_ipl_stub(&mut self, target_addr: u16, target_bank: u8) {
        self.sa1_iram.fill(0xFF);
        // JML absolute long: opcode 0x5C
        self.sa1_iram[0] = 0x5C;
        self.sa1_iram[1] = (target_addr & 0xFF) as u8;
        self.sa1_iram[2] = (target_addr >> 8) as u8;
        self.sa1_iram[3] = target_bank;
        // After jump, unused
        if std::env::var_os("TRACE_SA1_BOOT").is_some() {
            println!(
                "[SA1] IPL stub -> JML ${:02X}:{:04X} (IRAM filled with 0xFF)",
                target_bank, target_addr
            );
        }
    }

    #[inline]
    fn sa1_bwram_addr(&self, offset: u16) -> Option<usize> {
        if self.sa1_bwram.is_empty() || offset < 0x6000 {
            return None;
        }
        let window_offset = (offset - 0x6000) as usize;
        let block = (self.sa1.registers.bwram_select_snes & 0x1F) as usize;
        let base = block << 13; // 8 KB blocks
        let idx = base.wrapping_add(window_offset) % self.sa1_bwram.len();
        Some(idx)
    }

    /// SA-1 CPU側のBWRAMアドレス計算（bwram_select_sa1を使用）
    fn sa1_cpu_bwram_addr(&self, offset: u16) -> Option<usize> {
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
    fn sa1_phys_addr(&self, bank: u32, offset: u16) -> usize {
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

    /// SA-1 CPU側のROM/BWRAMリード
    pub fn sa1_read_u8(&mut self, addr: u32) -> u8 {
        let bank = (addr >> 16) & 0xFF;
        let offset = (addr & 0xFFFF) as u16;
        // DQ3: banks C0-DF should be treated as ROM mirrors, not BWRAM
        if self.mapper_type == crate::cartridge::MapperType::DragonQuest3
            && (0xC0..=0xDF).contains(&bank)
        {
            let phys = self.sa1_phys_addr(bank, offset);
            return self.rom.get(phys % self.rom_size).copied().unwrap_or(0xFF);
        }
        match bank {
            0x00..=0x3F | 0x80..=0xBF => {
                // SA-1 I-RAM (2KB) mapped at 00:0000-07FF for SA-1 CPU
                if (0x0000..=0x07FF).contains(&offset) {
                    return self.sa1_iram[(offset as usize) % self.sa1_iram.len()];
                }
                // Mirror at 00:3000-37FF
                if (0x3000..=0x37FF).contains(&offset) {
                    let idx = (offset - 0x3000) as usize;
                    return self.sa1_iram[idx % self.sa1_iram.len()];
                }
                // DQ3 workaround: map 00:FC00-FFFF vectors to the SA-1 program bank (0C)
                if self.mapper_type == crate::cartridge::MapperType::DragonQuest3
                    && offset >= 0xFC00
                {
                    let phys = self.sa1_phys_addr(self.sa1.boot_pb as u32, offset);
                    return self.rom.get(phys % self.rom_size).copied().unwrap_or(0xFF);
                }
                if (0x6000..=0x7FFF).contains(&offset) {
                    if let Some(idx) = self.sa1_cpu_bwram_addr(offset) {
                        return self.sa1_bwram[idx];
                    }
                }
                // SA-1 CPU can access its control registers in this window
                if (0x2200..=0x23FF).contains(&offset) {
                    if std::env::var_os("TRACE_SA1_REG").is_some() {
                        use std::sync::atomic::{AtomicU32, Ordering};
                        static COUNT: AtomicU32 = AtomicU32::new(0);
                        let n = COUNT.fetch_add(1, Ordering::Relaxed);
                        if n < 64 {
                            println!("SA1 REG R (SA1) {:02X}:{:04X} = (deferred)", bank, offset);
                        }
                    }
                    return self.sa1.read_register(offset - 0x2200);
                }
                let phys = self.sa1_phys_addr(bank, offset);
                return self.rom.get(phys % self.rom_size).copied().unwrap_or(0xFF);
            }
            0x40..=0x5F | 0xC0..=0xDF => {
                // Direct BWRAM access for SA-1
                let idx = ((bank & 0x1F) as usize) << 16 | (offset as usize);
                return self
                    .sa1_bwram
                    .get(idx % self.sa1_bwram.len())
                    .copied()
                    .unwrap_or(0);
            }
            0xC0..=0xFF => {
                let phys = self.sa1_phys_addr(bank, offset);
                return self.rom.get(phys % self.rom_size).copied().unwrap_or(0xFF);
            }
            _ => 0xFF,
        }
    }

    pub fn sa1_write_u8(&mut self, addr: u32, value: u8) {
        let bank = (addr >> 16) & 0xFF;
        let offset = (addr & 0xFFFF) as u16;
        match bank {
            0x00..=0x3F | 0x80..=0xBF => {
                // SA-1 I-RAM (2KB) mapped at 00:0000-07FF for SA-1 CPU
                if (0x0000..=0x07FF).contains(&offset) {
                    let idx = (offset as usize) % self.sa1_iram.len();
                    self.sa1_iram[idx] = value;
                    return;
                }
                // Mirror at 00:3000-37FF
                if (0x3000..=0x37FF).contains(&offset) {
                    let idx = ((offset - 0x3000) as usize) % self.sa1_iram.len();
                    self.sa1_iram[idx] = value;
                    return;
                }
                if (0x6000..=0x7FFF).contains(&offset) {
                    // Use SA-1 CPU's own BWRAM mapping register
                    if let Some(idx) = self.sa1_cpu_bwram_addr(offset) {
                        self.sa1_bwram[idx] = value;
                    }
                }
                // SA-1 CPU access to its registers
                if (0x2200..=0x23FF).contains(&offset) {
                    if std::env::var_os("TRACE_SA1_REG").is_some() {
                        use std::sync::atomic::{AtomicU32, Ordering};
                        static COUNT: AtomicU32 = AtomicU32::new(0);
                        let n = COUNT.fetch_add(1, Ordering::Relaxed);
                        if n < 64 {
                            println!(
                                "SA1 REG W (SA1) {:02X}:{:04X} = {:02X}",
                                bank, offset, value
                            );
                        }
                    }
                    self.sa1.write_register(offset - 0x2200, value);
                    return;
                }
            }
            0x40..=0x5F | 0xC0..=0xDF => {
                // Direct BWRAM access for SA-1
                let idx = ((bank & 0x1F) as usize) << 16 | (offset as usize);
                if !self.sa1_bwram.is_empty() {
                    let actual = idx % self.sa1_bwram.len();
                    self.sa1_bwram[actual] = value;
                }
            }
            _ => {}
        }
    }

    pub fn read_u8(&mut self, addr: u32) -> u8 {
        let bank = (addr >> 16) & 0xFF;
        let offset = (addr & 0xFFFF) as u16;

        // SA-1 vector override for S-CPU when SCNT selects SA-1 provided vectors.
        // SCNT bit6 -> use SIV (IRQ vector) instead of ROM $FFEE
        // SCNT bit5 -> use SNV (NMI vector) instead of ROM $FFEA
        if self.is_sa1_active() && bank == 0x00 {
            match offset {
                0xFFEA | 0xFFEB if (self.sa1.registers.scnt & 0x20) != 0 => {
                    let v = self.sa1.registers.snv;
                    return if offset & 1 == 0 {
                        (v & 0xFF) as u8
                    } else {
                        (v >> 8) as u8
                    };
                }
                0xFFEE | 0xFFEF if (self.sa1.registers.scnt & 0x40) != 0 => {
                    let v = self.sa1.registers.siv;
                    return if offset & 1 == 0 {
                        (v & 0xFF) as u8
                    } else {
                        (v >> 8) as u8
                    };
                }
                _ => {}
            }
        }

        // Debug: trace BRK/IRQ/NMI vector reads to verify mapping (SMW freeze investigation)
        if bank == 0x00
            && (0xFFE0..=0xFFFF).contains(&offset)
            && crate::debug_flags::trace_vectors()
        {
            use std::sync::atomic::{AtomicU32, Ordering};
            static COUNT_VEC: AtomicU32 = AtomicU32::new(0);
            let n = COUNT_VEC.fetch_add(1, Ordering::Relaxed);
            if n < 32 {
                // Read raw without recursion (peek ROM mapping directly)
                let raw = self.read_rom_lohi(bank as u32, offset);
                println!(
                    "[VEC] read {:02X}:{:04X} -> {:02X} mdr={:02X}",
                    bank, offset, raw, self.mdr
                );
            }
        }

        // Debug: trace HVBJOY reads to confirm address decoding (opt-in)
        if offset == 0x4212 && crate::debug_flags::trace_4212() {
            use std::sync::atomic::{AtomicU32, Ordering};
            static READ_COUNT_4212: AtomicU32 = AtomicU32::new(0);
            let idx = READ_COUNT_4212.fetch_add(1, Ordering::Relaxed);
            if idx < 32 {
                println!(
                    "[TRACE4212] addr={:06X} bank={:02X} offset={:04X} MDR=0x{:02X}",
                    addr, bank, offset, self.mdr
                );
            }
        }
        // Debug: trace S-CPU reads of SA-1 status regs ($2300/$2301) (opt-in)
        if offset == 0x2300 || offset == 0x2301 {
            let trace_sfr = crate::debug_flags::trace_sfr();
            let trace_sfr_values = crate::debug_flags::trace_sfr_values();
            if trace_sfr || trace_sfr_values {
                use std::sync::atomic::{AtomicU32, Ordering};
                static READ_COUNT_SFR: AtomicU32 = AtomicU32::new(0);
                let idx = READ_COUNT_SFR.fetch_add(1, Ordering::Relaxed);
                if idx < 16 {
                    let val = if trace_sfr_values {
                        // Safe to double-read; SA-1 register reads are side-effect free.
                        if offset == 0x2300 {
                            Some(self.sa1.read_register(0))
                        } else {
                            Some(self.sa1.read_register(1))
                        }
                    } else {
                        None
                    };
                    if let Some(v) = val {
                        println!(
                            "[TRACE_SFR] addr={:06X} bank={:02X} offset={:04X} val=0x{:02X}",
                            addr, bank, offset, v
                        );
                    } else {
                        println!(
                            "[TRACE_SFR] addr={:06X} bank={:02X} offset={:04X}",
                            addr, bank, offset
                        );
                    }
                }
            }
        }

        // SA-1 BW-RAM mapping for S-CPU in banks $40-$4F and high-speed mirror $60-$6F (full 64KB each)
        if self.is_sa1_active() && ((0x40..=0x4F).contains(&bank) || (0x60..=0x6F).contains(&bank))
        {
            if !self.sa1_bwram.is_empty() {
                let base = if (0x60..=0x6F).contains(&bank) {
                    (bank - 0x60) as usize
                } else {
                    (bank - 0x40) as usize
                };
                let idx = (base << 16) | offset as usize;
                return self.sa1_bwram[idx % self.sa1_bwram.len()];
            }
            return 0xFF;
        }

        // Update MDR for open bus behavior
        let value = match bank {
            // Dragon Quest 3 special banks - highest priority
            0x03 | 0x24 if self.mapper_type == crate::cartridge::MapperType::DragonQuest3 => {
                return self.read_dq3_rom(bank, offset);
            }
            // System area banks (mirror in 80-BF)
            0x00..=0x3F | 0x80..=0xBF => {
                match offset {
                    // SA-1 I-RAM window for S-CPU (00:3000-37FF)
                    0x3000..=0x37FF if self.is_sa1_active() => {
                        let idx = (offset - 0x3000) as usize;
                        if idx < self.sa1_iram.len() {
                            return self.sa1_iram[idx];
                        }
                        return 0xFF;
                    }
                    // SA-1 registers window (banks 00-3F/80-BF)
                    0x2200..=0x23FF if self.is_sa1_active() => {
                        return self.sa1.read_register(offset - 0x2200);
                    }
                    // 0x0000-0x1FFF: WRAM (標準挙動に統一)
                    // Stack area (0x0100-0x01FF)
                    0x0100..=0x01FF => {
                        let value = self.wram[offset as usize];
                        // Debug stack reads returning 0xFF
                        if crate::debug_flags::debug_stack_read() {
                            static mut STACK_READ_COUNT: u32 = 0;
                            unsafe {
                                if value == 0xFF {
                                    STACK_READ_COUNT += 1;
                                    if STACK_READ_COUNT <= 20 {
                                        println!("STACK READ #{}: Reading 0xFF from stack 0x{:04X}, bank=0x{:02X}",
                                                 STACK_READ_COUNT, offset, bank);
                                    }
                                }
                            }
                        }
                        value
                    }
                    // Mirror WRAM in first 8KB (excluding stack area already handled above)
                    0x0000..=0x00FF | 0x0200..=0x1FFF => self.wram[offset as usize],
                    // Mirror of first page (0x000-0x0FF) in 0x2000-0x20FF
                    0x2000..=0x20FF => self.wram[(offset & 0xFF) as usize],
                    0x6000..=0x7FFF if self.is_sa1_active() => {
                        if let Some(idx) = self.sa1_bwram_addr(offset) {
                            let v = self.sa1_bwram[idx];
                            if crate::debug_flags::trace_bwram_sys() {
                                use std::sync::atomic::{AtomicU32, Ordering};
                                static COUNT_R: AtomicU32 = AtomicU32::new(0);
                                let n = COUNT_R.fetch_add(1, Ordering::Relaxed);
                                if n < 32 {
                                    println!(
                                "BWRAM SYS R bank={:02X} off={:04X} idx=0x{:05X} val={:02X}",
                                bank, offset, idx, v
                            );
                                }
                            }
                            return v;
                        }
                        if crate::debug_flags::trace_bwram_sys() {
                            use std::sync::atomic::{AtomicU32, Ordering};
                            static COUNT: AtomicU32 = AtomicU32::new(0);
                            let n = COUNT.fetch_add(1, Ordering::Relaxed);
                            if n < 32 {
                                println!(
                                    "BWRAM SYS R bank={:02X} off={:04X} (no-map) val=FF",
                                    bank, offset
                                );
                            }
                        }
                        0xFF
                    }
                    // SA-1 register window
                    0x2200..=0x23FF if self.is_sa1_active() => {
                        let reg = offset - 0x2200;
                        let v = self.sa1.read_register(reg);
                        if crate::debug_flags::trace_sa1_reg() {
                            println!("SA1 REG R {:02X}:{:04X} -> {:02X}", bank, offset, v);
                        }
                        if reg <= 1 && crate::debug_flags::trace_sfr_val() {
                            use std::sync::atomic::{AtomicU32, Ordering};
                            static COUNT_SFR: AtomicU32 = AtomicU32::new(0);
                            let idx = COUNT_SFR.fetch_add(1, Ordering::Relaxed);
                            if idx < 32 {
                                println!(
                                     "[SFR READ] reg=0x{:04X} val=0x{:02X} enable=0x{:02X} pending=0x{:02X} CIE=0x{:02X} SIE=0x{:02X}",
                                     0x2200 + reg,
                                     v,
                                     self.sa1.registers.interrupt_enable,
                                     self.sa1.registers.interrupt_pending,
                                     self.sa1.registers.cie,
                                     self.sa1.registers.sie
                                 );
                            }
                        }
                        return v;
                    }
                    // PPU registers
                    0x2100..=0x213F => {
                        let ppu_reg = offset & 0xFF;
                        if matches!(ppu_reg, 0x39 | 0x3A)
                            && crate::debug_flags::trace_burnin_dma_memory()
                        {
                            let pc16 = (self.last_cpu_pc & 0xFFFF) as u16;
                            if (0xAE80..=0xAEEF).contains(&pc16) {
                                use std::sync::atomic::{AtomicU32, Ordering};
                                static CNT: AtomicU32 = AtomicU32::new(0);
                                let n = CNT.fetch_add(1, Ordering::Relaxed);
                                if n < 64 {
                                    let (vmadd, inc, vmain) = self.ppu.dbg_vram_regs();
                                    println!(
                                        "[BURNIN-DMAMEM][PPU-R] PC={:06X} ${:04X} VMADD={:04X} VMAIN={:02X} inc={} (pre)",
                                        self.last_cpu_pc, offset, vmadd, vmain, inc
                                    );
                                }
                            }
                        }
                        let v = self.ppu.read(ppu_reg);
                        if matches!(ppu_reg, 0x39 | 0x3A)
                            && crate::debug_flags::trace_burnin_dma_memory()
                        {
                            let pc16 = (self.last_cpu_pc & 0xFFFF) as u16;
                            if (0xAE80..=0xAEEF).contains(&pc16) {
                                use std::sync::atomic::{AtomicU32, Ordering};
                                static CNT: AtomicU32 = AtomicU32::new(0);
                                let n = CNT.fetch_add(1, Ordering::Relaxed);
                                if n < 64 {
                                    let (vmadd, inc, vmain) = self.ppu.dbg_vram_regs();
                                    println!(
                                        "[BURNIN-DMAMEM][PPU-R] PC={:06X} ${:04X} -> {:02X} VMADD={:04X} VMAIN={:02X} inc={} (post)",
                                        self.last_cpu_pc, offset, v, vmadd, vmain, inc
                                    );
                                }
                            }
                        }
                        if crate::debug_flags::trace_burnin_v224() {
                            let pc16 = (self.last_cpu_pc & 0xFFFF) as u16;
                            if (0x97D0..=0x98FF).contains(&pc16) {
                                match offset {
                                    0x2137 | 0x213D | 0x213F => {
                                        println!(
                                            "[BURNIN-V224][PPU-R] PC={:06X} ${:04X} -> {:02X} sl={} cyc={} vblank={} vis_h={}",
                                            self.last_cpu_pc,
                                            offset,
                                            v,
                                            self.ppu.scanline,
                                            self.ppu.get_cycle(),
                                            self.ppu.is_vblank() as u8,
                                            self.ppu.get_visible_height()
                                        );
                                    }
                                    _ => {}
                                }
                            }
                        }
                        if crate::debug_flags::trace_burnin_ext_latch() {
                            use std::sync::atomic::{AtomicU32, Ordering};
                            static CNT: AtomicU32 = AtomicU32::new(0);
                            let n = CNT.fetch_add(1, Ordering::Relaxed);
                            if n < 2048 {
                                match offset {
                                    0x2137 | 0x213C | 0x213D | 0x213F => {
                                        println!(
                                            "[BURNIN-EXT][PPU-R] PC={:06X} ${:04X} -> {:02X} sl={} cyc={} vblank={} wio=0x{:02X}",
                                            self.last_cpu_pc,
                                            offset,
                                            v,
                                            self.ppu.scanline,
                                            self.ppu.get_cycle(),
                                            self.ppu.is_vblank() as u8,
                                            self.wio
                                        );
                                    }
                                    _ => {}
                                }
                            }
                        }
                        if crate::debug_flags::trace_burnin_obj() && offset == 0x213E {
                            use std::sync::atomic::{AtomicU32, Ordering};
                            static CNT: AtomicU32 = AtomicU32::new(0);
                            let n = CNT.fetch_add(1, Ordering::Relaxed);
                            if n < 256 {
                                println!(
                                    "[BURNIN-OBJ][STAT77] PC={:06X} -> {:02X} frame={} sl={} cyc={} vblank={}",
                                    self.last_cpu_pc,
                                    v,
                                    self.ppu.get_frame(),
                                    self.ppu.scanline,
                                    self.ppu.get_cycle(),
                                    self.ppu.is_vblank() as u8
                                );
                            }
                        }
                        v
                    }
                    // APU registers
                    0x2140..=0x217F => {
                        // Optional: fake APU boot handshake (SMW/DQ3 early init)
                        let val = if self.fake_apu {
                            // Before the first $CC, expose the IPL signature AA/BB.
                            let sig = match offset & 0x03 {
                                0 => 0xAA,
                                1 => 0xBB,
                                _ => 0x00,
                            };
                            if !self.fake_apu_booted {
                                if crate::debug_flags::trace_fake_apu() {
                                    use std::sync::atomic::{AtomicU32, Ordering};
                                    static COUNT: AtomicU32 = AtomicU32::new(0);
                                    let n = COUNT.fetch_add(1, Ordering::Relaxed);
                                    if n < 128 {
                                        println!(
                                            "[FAKE-APU] R ${:04X} (sig) -> {:02X}",
                                            offset, sig
                                        );
                                    }
                                }
                                sig
                            } else {
                                let port_idx = (offset & 0x03) as usize;
                                // Echo last written value (simple HLE).
                                let v = self.fake_apu_ports[port_idx];
                                if crate::debug_flags::trace_fake_apu() {
                                    use std::sync::atomic::{AtomicU32, Ordering};
                                    static COUNT: AtomicU32 = AtomicU32::new(0);
                                    let n = COUNT.fetch_add(1, Ordering::Relaxed);
                                    if n < 256 {
                                        println!(
                                            "[FAKE-APU] R ${:04X} -> {:02X} state={:?}",
                                            offset, v, self.fake_apu_upload_state
                                        );
                                    }
                                }
                                v
                            }
                        } else {
                            self.apu
                                .lock()
                                .map(|mut apu| {
                                    let p = (offset & 0x03) as u8;
                                    let v = apu.read_port(p);
                                    // burn-in-test.sfc APU FAIL調査: CPU側が最終判定で $2141 を読む瞬間に
                                    // APU(S-SMP) の実行位置をログに出す（opt-in, 少量）。
                                    if crate::debug_flags::trace_burnin_apu_prog()
                                        && offset == 0x2141
                                        && self.last_cpu_pc == 0x00863F
                                    {
                                        use std::sync::atomic::{AtomicU32, Ordering};
                                        static CNT: AtomicU32 = AtomicU32::new(0);
                                        let n = CNT.fetch_add(1, Ordering::Relaxed);
                                        if n < 4 {
                                            if let Some(smp) = apu.inner.smp.as_ref() {
                                                let smp_pc = smp.reg_pc;
                                                let smp_a = smp.reg_a;
                                                let smp_x = smp.reg_x;
                                                let smp_y = smp.reg_y;
                                                let smp_sp = smp.reg_sp;
                                                let smp_psw = smp.get_psw();
                                                let ctx_start = smp_pc.wrapping_sub(0x10);
                                                let mut code = [0u8; 32];
                                                for (i, b) in code.iter_mut().enumerate() {
                                                    *b = apu
                                                        .inner
                                                        .read_u8(ctx_start.wrapping_add(i as u16) as u32);
                                                }
                                                let t0 = apu.inner.debug_timer_state(0);
                                                println!(
                                                    "[BURNIN-APU-PROG] cpu_pc=00:{:04X} apui1={:02X} sl={} cyc={} frame={} vblank={} vis_h={} apu_cycles={} smp_pc={:04X} A={:02X} X={:02X} Y={:02X} SP={:02X} PSW={:02X} t0={:?} code@{:04X}={:02X?}",
                                                    (self.last_cpu_pc & 0xFFFF) as u16,
                                                    v,
                                                    self.ppu.scanline,
                                                    self.ppu.get_cycle(),
                                                    self.ppu.get_frame(),
                                                    self.ppu.is_vblank() as u8,
                                                    self.ppu.get_visible_height(),
                                                    apu.total_smp_cycles,
                                                    smp_pc,
                                                    smp_a,
                                                    smp_x,
                                                    smp_y,
                                                    smp_sp,
                                                    smp_psw,
                                                    t0,
                                                    ctx_start,
                                                    code
                                                );
                                            } else {
                                                println!(
                                                    "[BURNIN-APU-PROG] cpu_pc=00:{:04X} apui1={:02X} smp=<none>",
                                                    (self.last_cpu_pc & 0xFFFF) as u16,
                                                    v
                                                );
                                            }
                                        }
                                    }
                                    if crate::debug_flags::trace_apu_port() {
                                        use std::sync::atomic::{AtomicU32, Ordering};
                                        static COUNT: AtomicU32 = AtomicU32::new(0);
                                        let n = COUNT.fetch_add(1, Ordering::Relaxed);
                                        if n < 256 {
                                            println!(
                                                "[APU] R ${:04X} (port{}) -> {:02X}",
                                                offset, p, v
                                            );
                                        }
                                    }
                                    v
                                })
                                .unwrap_or(0)
                        };
                        // Test ROM support: SPC->CPU 2140 streamをコンソールへ転送
                        if (self.test_apu_print || crate::debug_flags::cpu_test_hle())
                            && offset == 0x2140
                        {
                            let ch = val as char;
                            if ch.is_ascii_graphic() || ch == ' ' || ch == '\n' || ch == '\r' {
                                self.test_apu_buf.push(ch);
                                if ch == '\n' || self.test_apu_buf.len() > 512 {
                                    let line = self.test_apu_buf.replace('\r', "");
                                    println!("[TESTROM] APU: {}", line.trim_end());
                                    let lower = line.to_ascii_lowercase();
                                    if lower.contains("passed") || lower.contains("pass") {
                                        println!("[TESTROM] PASS");
                                        crate::shutdown::request_quit();
                                    } else if lower.contains("fail") {
                                        println!("[TESTROM] FAIL");
                                        crate::shutdown::request_quit();
                                    }
                                    self.test_apu_buf.clear();
                                }
                            }
                        }
                        // Concise APU handshake trace (read side)
                        if crate::debug_flags::trace_apu_handshake() && offset <= 0x2143 {
                            let state = if self.fake_apu {
                                "fake-apu"
                            } else {
                                self.apu
                                    .lock()
                                    .map(|apu| apu.handshake_state_str())
                                    .unwrap_or("apu-lock")
                            };
                            use std::sync::atomic::{AtomicU32, Ordering};
                            static CNT: AtomicU32 = AtomicU32::new(0);
                            let n = CNT.fetch_add(1, Ordering::Relaxed);
                            let limit = crate::debug_flags::trace_apu_handshake_limit();
                            if n < limit {
                                println!(
                                    "[APU-HS][R] ${:04X} -> {:02X} state={} pc={:06X} frame={} sl={} cyc={}",
                                    offset,
                                    val,
                                    state,
                                    self.last_cpu_pc,
                                    self.ppu.get_frame(),
                                    self.ppu.scanline,
                                    self.ppu.get_cycle()
                                );
                            }
                        }
                        val
                    }
                    // WRAM access port
                    0x2180 => {
                        let addr = self.wram_address as usize;
                        if addr < self.wram.len() {
                            let value = self.wram[addr];
                            // WMADD ($2181-2183) is a 17-bit address; auto-increment carries across bit16.
                            self.wram_address = (self.wram_address + 1) & 0x1FFFF;
                            value
                        } else {
                            0xFF
                        }
                    }
                    0x2181..=0x2183 => 0x00, // WRAM Address registers (write-only)
                    // Expansion area
                    0x2184..=0x21FF => 0xFF, // Open bus
                    0x2200..=0x3FFF => 0xFF, // Open bus
                    // Controller/IO registers
                    0x4000..=0x42FF => self.read_io_register(offset),
                    // DMA registers
                    0x4300..=0x43FF => self.dma_controller.read(offset),
                    // More IO registers
                    0x4400..=0x5FFF => self.read_io_register(offset),
                    // Cartridge expansion
                    0x6000..=0x7FFF => {
                        match self.mapper_type {
                            crate::cartridge::MapperType::LoRom => {
                                // SRAM window in system banks ($00-$3F/$80-$BF): $6000-$7FFF (8KB)
                                if self.sram_size == 0 {
                                    0xFF
                                } else {
                                    let bank_index = (bank & 0x3F) as usize;
                                    let window = bank_index * 0x2000 + ((offset - 0x6000) as usize);
                                    let idx = window % self.sram_size;
                                    self.sram[idx]
                                }
                            }
                            crate::cartridge::MapperType::HiRom => {
                                // SRAM in HiROM
                                let sram_addr =
                                    ((bank as usize) << 13) + ((offset - 0x6000) as usize);
                                if sram_addr < self.sram.len() {
                                    self.sram[sram_addr]
                                } else {
                                    0xFF
                                }
                            }
                            crate::cartridge::MapperType::DragonQuest3 => {
                                // DQ3: treat 6000-7FFF as BW-RAM window via SA-1 mapping
                                if let Some(idx) = self.sa1_bwram_addr(offset) {
                                    self.sa1_bwram[idx]
                                } else {
                                    0xFF
                                }
                            }
                            _ => 0xFF, // Other mappers not implemented yet
                        }
                    }
                    // ROM area
                    0x8000..=0xFFFF => self.read_rom_lohi(bank, offset),
                }
            }
            // ROM banks 40-7D (HiROM/ExHiROM lower half)
            0x40..=0x7D => {
                match self.mapper_type {
                    crate::cartridge::MapperType::DragonQuest3 => {
                        // ドラクエ3専用マッピング（HiROMベース + エンハンスメント対応）
                        self.read_dq3_rom(bank, offset)
                    }
                    crate::cartridge::MapperType::ExHiRom => {
                        // ExHiROM: banks 40-7D map to lower half of ROM (0x000000..)
                        let rom_addr = (bank as usize) * 0x10000 + (offset as usize);
                        if rom_addr < self.rom_size {
                            self.rom[rom_addr]
                        } else {
                            0xFF
                        }
                    }
                    crate::cartridge::MapperType::LoRom => {
                        // LoROM:
                        // - banks 0x70-0x7D: $0000-$7FFF maps to cartridge SRAM (mirrored to SRAM size)
                        // - other banks: $8000-$FFFF maps to ROM
                        if self.sram_size > 0 && offset < 0x8000 && (0x70..=0x7D).contains(&bank) {
                            let window = ((bank - 0x70) as usize) * 0x8000 + (offset as usize);
                            let idx = window % self.sram_size;
                            self.sram[idx]
                        } else {
                            let rom_addr =
                                ((bank - 0x40) as usize) * 0x8000 + ((offset - 0x8000) as usize);
                            if rom_addr < self.rom_size {
                                self.rom[rom_addr]
                            } else {
                                0xFF
                            }
                        }
                    }
                    crate::cartridge::MapperType::HiRom => {
                        // HiROM/ExHiROM: Full 64KB banks; if >2MB, this region is lower half
                        let base = ((bank - 0x40) as usize) * 0x10000 + (offset as usize);
                        let phys = if self.rom_size > 0x200000 {
                            base % 0x200000
                        } else {
                            base % self.rom_size
                        };
                        self.rom[phys]
                    }
                    _ => 0xFF, // Other mappers
                }
            }
            // Extended WRAM banks
            0x7E..=0x7F => {
                // Optionally mirror 7E/7F to the same 64KB (useful for some test ROMs)
                let wram_addr = if self.wram_64k_mirror {
                    (offset as usize) & 0xFFFF
                } else {
                    ((bank - 0x7E) as usize) * 0x10000 + (offset as usize)
                };
                // Debug: trace key handshake variables in WRAM (DQ3 NMI paths)
                if self.trace_nmi_wram {
                    use std::sync::atomic::{AtomicU32, Ordering};
                    static READ_COUNT: AtomicU32 = AtomicU32::new(0);
                    if let Some(label) = match wram_addr {
                        0x07DE => Some("00:07DE"),
                        0x07E0 => Some("00:07E0"),
                        0x07E4 => Some("00:07E4"),
                        0x07F6 => Some("00:07F6"),
                        0x0FDE => Some("7E:0FDE"),
                        0x0FE0 => Some("7E:0FE0"),
                        0x0FE4 => Some("7E:0FE4"),
                        0x0FF6 => Some("7E:0FF6"),
                        0x1FDE => Some("7F:0FDE"),
                        0x1FE0 => Some("7F:0FE0"),
                        0x1FE4 => Some("7F:0FE4"),
                        0x1FF6 => Some("7F:0FF6"),
                        _ => None,
                    } {
                        let idx = READ_COUNT.fetch_add(1, Ordering::Relaxed);
                        if idx < 64 {
                            let v = if wram_addr < self.wram.len() {
                                self.wram[wram_addr]
                            } else {
                                0xFF
                            };
                            println!(
                                "[WRAM TRACE READ {}] val=0x{:02X} bank={:02X} off={:04X}",
                                label, v, bank, offset
                            );
                        }
                    }
                }
                if wram_addr < self.wram.len() {
                    self.wram[wram_addr]
                } else {
                    0xFF
                }
            }
            // ROM mirror banks (HiROM/ExHiROM upper half)
            0xC0..=0xFF => {
                match self.mapper_type {
                    crate::cartridge::MapperType::DragonQuest3 => self.read_dq3_rom(bank, offset),
                    crate::cartridge::MapperType::ExHiRom => {
                        // ExHiROM: banks C0-FF mirror to 00-3F area
                        let mirror_bank = bank - 0xC0; // C0->00 .. FF->3F
                        let rom_addr = (mirror_bank as usize) * 0x10000 + (offset as usize);
                        if rom_addr < self.rom_size {
                            self.rom[rom_addr]
                        } else {
                            0xFF
                        }
                    }
                    crate::cartridge::MapperType::LoRom => {
                        // LoROM: Mirror of 40-7F region
                        let mirror_bank = bank - 0x80; // C0->40 .. FF->7F
                        if offset < 0x8000 {
                            if self.sram_size == 0 || !(0x70..=0x7D).contains(&mirror_bank) {
                                0xFF
                            } else {
                                let window =
                                    ((mirror_bank - 0x70) as usize) * 0x8000 + (offset as usize);
                                let idx = window % self.sram_size;
                                self.sram[idx]
                            }
                        } else {
                            self.read_rom_lohi(mirror_bank, offset)
                        }
                    }
                    crate::cartridge::MapperType::HiRom => {
                        // HiROM: Many titles mirror C0-FF to 40-7F region. Prefer that simple mirror.
                        let mirror_bank = (bank - 0x80) & 0xFF; // C0->40, FF->7F
                        let phys = (mirror_bank as usize) * 0x10000 + (offset as usize);
                        let phys = phys % self.rom_size;
                        self.rom[phys]
                    }
                    _ => 0xFF,
                }
            }
            // Other banks - open bus
            _ => 0xFF,
        };

        self.mdr = value;
        value
    }

    // Helper method for ROM reading in system banks
    // Dragon Quest 3専用ROM読み取り処理
    fn read_dq3_rom(&mut self, bank: u32, offset: u16) -> u8 {
        // SA-1 LoROM: 32KB banks across 4MB; 0x80-0xFF mirror 0x00-0x7F.
        let rom_addr = self.dq3_phys_addr(bank as u8, offset);

        if (crate::debug_flags::mapper() || crate::debug_flags::boot_verbose())
            && bank == 0xC0
            && offset < 0x0100
        {
            static mut C0_ACCESS_COUNT: u32 = 0;
            unsafe {
                C0_ACCESS_COUNT += 1;
                if C0_ACCESS_COUNT <= 5 {
                    println!(
                        "DQ3 ROM read {:02X}:{:04X} -> phys=0x{:06X} (size=0x{:06X})",
                        bank, offset, rom_addr, self.rom_size
                    );
                }
            }
        }

        let value = self.rom[rom_addr % self.rom_size];
        self.mdr = value;
        value
    }

    // DQ3エンハンスメント領域の判定
    #[allow(dead_code)]
    fn is_dq3_enhancement_area(&self, bank: u32, _offset: u16) -> bool {
        // エンハンスメントチップ0x30の専用領域
        match bank {
            0x03 | 0x24 | 0x30..=0x37 => true, // エンハンスメント専用バンク
            _ => false,
        }
    }

    // DQ3エンハンスメント処理
    fn handle_dq3_enhancement(&self, bank: u32, offset: u16) -> u8 {
        // エンハンスメント機能の実装
        match bank {
            // 標準システムバンク 00-3F の低位アドレス処理
            0x00..=0x3F => {
                // Dragon Quest 3はHiROMベース：全アドレス範囲にROMデータ
                let rom_addr = (bank as usize) * 0x10000 + (offset as usize);

                // Bank 00の低位アドレスをデバッグ
                if bank == 0x00 && offset <= 0x1000 && crate::debug_flags::debug_dq3_bank() {
                    static mut BANK00_DEBUG_COUNT: u32 = 0;
                    unsafe {
                        BANK00_DEBUG_COUNT += 1;
                        if BANK00_DEBUG_COUNT <= 5 {
                            println!(
                                "DQ3 Bank 00:{:04X} -> rom_addr=0x{:06X}, rom_size=0x{:06X}",
                                offset, rom_addr, self.rom_size
                            );
                        }
                    }
                }

                if rom_addr < self.rom_size {
                    let value = self.rom[rom_addr];

                    // Bank 00の値をログ
                    if bank == 0x00 && offset <= 0x1000 {
                        static mut BANK00_VALUE_COUNT: u32 = 0;
                        unsafe {
                            BANK00_VALUE_COUNT += 1;
                            if BANK00_VALUE_COUNT <= 5 {
                                println!(
                                    "  DQ3 ROM[0x{:06X}] = 0x{:02X} (Bank {:02X}:{:04X})",
                                    rom_addr, value, bank, offset
                                );
                            }
                        }
                    }

                    value
                } else {
                    // ROM範囲外の場合はミラー
                    let mirror_addr = rom_addr % self.rom_size;
                    self.rom[mirror_addr]
                }
            }
            0x03 | 0x24 => {
                // Bank 03/24を適切なROM領域にマップ
                // DQ3の4MB ROMでの特殊バンク処理
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

    fn read_rom_lohi(&self, bank: u32, offset: u16) -> u8 {
        match self.mapper_type {
            crate::cartridge::MapperType::LoRom => {
                // LoROM: 32KB banks in upper half
                let rom_addr = ((bank & 0x3F) as usize) * 0x8000 + ((offset - 0x8000) as usize);
                if rom_addr < self.rom_size {
                    self.rom[rom_addr]
                } else {
                    0xFF
                }
            }
            crate::cartridge::MapperType::HiRom => {
                // HiROM: Full 64KB banks
                let rom_addr = (bank as usize) * 0x10000 + (offset as usize);
                if rom_addr < self.rom_size {
                    self.rom[rom_addr]
                } else {
                    0xFF
                }
            }
            crate::cartridge::MapperType::ExHiRom => {
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
            crate::cartridge::MapperType::DragonQuest3 => {
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
    fn dq3_phys_addr(&self, bank: u8, offset: u16) -> usize {
        let bank_idx = bank as usize;
        let addr = bank_idx * 0x10000 + offset as usize;
        addr % self.rom_size
    }

    pub fn write_u8(&mut self, addr: u32, value: u8) {
        let bank = (addr >> 16) & 0xFF;
        let offset = (addr & 0xFFFF) as u16;

        // Debug: watch a specific address write (S-CPU side)
        if let Some(watch) = crate::debug_flags::watch_addr_write() {
            if watch == addr {
                println!(
                    "[watchW] {:02X}:{:04X} <= {:02X} PC={:06X}",
                    bank, offset, value, self.last_cpu_pc
                );
            }
        }
        // Debug: watch/force WRAM writes (banks 7E/7F)
        if bank == 0x7E || bank == 0x7F {
            if let Some(watch) = crate::debug_flags::watch_wram_write() {
                if watch == addr {
                    println!(
                        "[WRAM-WATCH] PC={:06X} {:02X}:{:04X} <= {:02X}",
                        self.last_cpu_pc, bank, offset, value
                    );
                }
            }
            if let Some((watch, forced)) = crate::debug_flags::watch_wram_write_force() {
                if watch == addr {
                    println!(
                        "[WRAM-FORCE] PC={:06X} {:02X}:{:04X} {:02X} -> {:02X}",
                        self.last_cpu_pc, bank, offset, value, forced
                    );
                    // 監視アドレス以外でも、強制書き込みモードでは値を差し替える
                    self.wram[offset as usize] = forced;
                    return;
                }
            }
        }

        if ((offset >= 0x0100 && offset <= 0x01FF) || offset == 0xFFFF)
            && crate::debug_flags::trace_stack_write()
        {
            println!(
                "[STACK-WRITE] PC={:06X} wrote {:02X} to {:02X}:{:04X}",
                self.last_cpu_pc, value, bank, offset
            );
        }

        // SA-1 BW-RAM mapping for S-CPU in banks $40-$4F and $60-$6F
        if self.is_sa1_active() && ((0x40..=0x4F).contains(&bank) || (0x60..=0x6F).contains(&bank))
        {
            if !self.sa1_bwram.is_empty() {
                let base = if (0x60..=0x6F).contains(&bank) {
                    (bank - 0x60) as usize
                } else {
                    (bank - 0x40) as usize
                };
                let idx = (base << 16) | offset as usize;
                let actual = idx % self.sa1_bwram.len();
                self.sa1_bwram[actual] = value;
            }
            return;
        }

        match bank {
            // System area banks (mirror in 80-BF)
            0x00..=0x3F | 0x80..=0xBF => {
                match offset {
                    // Stack area (0x0100-0x01FF)
                    0x0100..=0x01FF => {
                        // Debug stack corruption - trace suspicious writes
                        if crate::debug_flags::debug_stack_trace() {
                            static mut STACK_TRACE_COUNT: u32 = 0;
                            unsafe {
                                STACK_TRACE_COUNT += 1;
                                if STACK_TRACE_COUNT <= 50 || value == 0xFF {
                                    println!(
                                        "🔍 STACK WRITE #{}: addr=0x{:04X} value=0x{:02X} (suspect={})",
                                        STACK_TRACE_COUNT,
                                        offset,
                                        value,
                                        if value == 0xFF { "YES" } else { "no" }
                                    );
                                }
                            }
                        }
                        self.wram[offset as usize] = value;
                    }
                    // Mirror WRAM in first 8KB (excluding stack area already handled above)
                    0x0000..=0x00FF | 0x0200..=0x1FFF => {
                        if let Some(watch) = crate::debug_flags::watch_wram_write() {
                            let full = ((bank as u32) << 16) | offset as u32;
                            if full == watch {
                                println!(
                                    "[WRAM-WATCH] PC={:06X} {:02X}:{:04X} <= {:02X}",
                                    self.last_cpu_pc, bank, offset, value
                                );
                            }
                        }
                        if crate::debug_flags::trace_burnin_zp16()
                            && matches!(offset, 0x0016 | 0x0017 | 0x001F)
                        {
                            println!(
                                "[BURNIN-ZP] PC={:06X} ${:04X} <- {:02X} frame={} sl={} cyc={}",
                                self.last_cpu_pc,
                                offset,
                                value,
                                self.ppu.get_frame(),
                                self.ppu.scanline,
                                self.ppu.get_cycle()
                            );
                        }
                        if offset < 0x0010 && crate::debug_flags::trace_zp() {
                            use std::sync::atomic::{AtomicU32, Ordering};
                            static COUNT: AtomicU32 = AtomicU32::new(0);
                            let n = COUNT.fetch_add(1, Ordering::Relaxed);
                            if n < 64 {
                                println!(
                                    "[ZP-W] PC={:06X} addr=0x{:04X} <= {:02X}",
                                    self.last_cpu_pc, offset, value
                                );
                            }
                        }
                        self.wram[offset as usize] = value;
                    }
                    // Mirror of first page (0x000-0x0FF) in 0x2000-0x20FF
                    0x2000..=0x20FF => self.wram[(offset & 0xFF) as usize] = value,
                    0x6000..=0x7FFF if self.is_sa1_active() => {
                        if let Some(idx) = self.sa1_bwram_addr(offset) {
                            self.sa1_bwram[idx] = value;
                            if crate::debug_flags::trace_bwram_sys() {
                                use std::sync::atomic::{AtomicU32, Ordering};
                                static COUNT: AtomicU32 = AtomicU32::new(0);
                                let n = COUNT.fetch_add(1, Ordering::Relaxed);
                                if n < 32 {
                                    println!(
                                        "BWRAM SYS W bank={:02X} off={:04X} idx=0x{:05X} val={:02X}",
                                        bank, offset, idx, value
                                    );
                                }
                            }
                        }
                    }
                    // PPU registers (no DQ3-specific overrides)
                    0x2100..=0x213F => {
                        if crate::debug_flags::trace_burnin_v224() {
                            let pc16 = (self.last_cpu_pc & 0xFFFF) as u16;
                            if (0x97D0..=0x98FF).contains(&pc16) && offset == 0x2133 {
                                println!(
                                    "[BURNIN-V224][PPU-W] PC={:06X} ${:04X} <- {:02X} frame={} sl={} cyc={} vblank={} vis_h={}",
                                    self.last_cpu_pc,
                                    offset,
                                    value,
                                    self.ppu.get_frame(),
                                    self.ppu.scanline,
                                    self.ppu.get_cycle(),
                                    self.ppu.is_vblank() as u8,
                                    self.ppu.get_visible_height()
                                );
                            }
                        }
                        let ppu_reg = offset & 0xFF;
                        // burn-in-test.sfc diagnostics: include S-CPU PC for VRAM data port writes
                        // that touch the DMA MEMORY test region (VMADD 0x5000..0x57FF).
                        if matches!(ppu_reg, 0x18 | 0x19) {
                            let trace_dmamem = crate::debug_flags::trace_burnin_dma_memory();
                            let trace_status = crate::debug_flags::trace_burnin_status();
                            let trace_apu_status = crate::debug_flags::trace_burnin_apu_status();
                            if trace_dmamem || trace_status || trace_apu_status {
                                use std::sync::atomic::{AtomicU32, Ordering};
                                let (vmadd, _inc, vmain) = self.ppu.dbg_vram_regs();

                                // burn-in-test.sfc diagnostics: include S-CPU PC for VRAM data port writes
                                // that touch the DMA MEMORY test region (VMADD 0x5000..0x57FF).
                                // Only count/log writes that actually land in the interesting range;
                                // otherwise early VRAM traffic (font/tiles) exhausts the counter.
                                if trace_dmamem && (0x5000..0x5800).contains(&vmadd) {
                                    static CNT: AtomicU32 = AtomicU32::new(0);
                                    let n = CNT.fetch_add(1, Ordering::Relaxed);
                                    if n < 256 {
                                        println!(
	                                            "[BURNIN-VRAM-PC] PC={:06X} ${:04X} <- {:02X} VMADD={:04X} VMAIN={:02X}",
	                                            self.last_cpu_pc,
	                                            offset,
	                                            value,
	                                            vmadd,
	                                            vmain
	                                        );
                                    }
                                }

                                // Focused logging for PASS/FAIL column updates (opt-in).
                                if trace_status && (0x50F0..0x5200).contains(&vmadd) {
                                    let ch = value as char;
                                    let printable = ch.is_ascii_graphic() || ch == ' ';
                                    println!(
	                                        "[BURNIN-STATUS] PC={:06X} ${:04X} <- {:02X}{} VMADD={:04X} VMAIN={:02X}",
	                                        self.last_cpu_pc,
	                                        offset,
	                                        value,
	                                        if printable {
	                                            format!(" ('{}')", ch)
	                                        } else {
	                                            String::new()
	                                        },
	                                        vmadd,
	                                        vmain
	                                    );
                                }

                                // Focused logging for the APU status row (menu 5 results).
                                // The PASS/FAIL column for the bottom rows lives around VMADD ~= $52D0.
                                if trace_apu_status && (0x52C0..=0x52FF).contains(&vmadd) {
                                    println!(
	                                        "[BURNIN-APU-STATUS] PC={:06X} ${:04X} <- {:02X} VMADD={:04X} VMAIN={:02X}",
	                                        self.last_cpu_pc, offset, value, vmadd, vmain
	                                    );
                                }
                            }
                        }
                        self.ppu.write(ppu_reg, value);
                        if matches!(ppu_reg, 0x00 | 0x15 | 0x16 | 0x17)
                            && crate::debug_flags::trace_burnin_dma_memory()
                        {
                            let pc16 = (self.last_cpu_pc & 0xFFFF) as u16;
                            if (0xAE80..=0xAEEF).contains(&pc16) {
                                use std::sync::atomic::{AtomicU32, Ordering};
                                static CNT: AtomicU32 = AtomicU32::new(0);
                                let n = CNT.fetch_add(1, Ordering::Relaxed);
                                if n < 128 {
                                    let (vmadd, inc, vmain) = self.ppu.dbg_vram_regs();
                                    println!(
                                        "[BURNIN-DMAMEM][PPU-W] PC={:06X} ${:04X} <- {:02X} VMADD={:04X} VMAIN={:02X} inc={}",
                                        self.last_cpu_pc, offset, value, vmadd, vmain, inc
                                    );
                                }
                            }
                        }
                    }
                    0x2200..=0x23FF if self.is_sa1_active() => {
                        if crate::debug_flags::trace_sa1_reg() {
                            println!(
                                "SA1 REG W (S-CPU) {:02X}:{:04X} = {:02X}",
                                bank, offset, value
                            );
                        }
                        self.sa1.write_register(offset - 0x2200, value);
                    }
                    // APU registers
                    0x2140..=0x217F => {
                        // burn-in-test.sfc APU test: trace the CPU command sequence (opt-in, low volume).
                        if crate::debug_flags::trace_burnin_apu_cpu()
                            && offset <= 0x2143
                            && (0x008600..=0x008700).contains(&self.last_cpu_pc)
                        {
                            let apu_cycles =
                                self.apu.lock().map(|apu| apu.total_smp_cycles).unwrap_or(0);
                            println!(
                                "[BURNIN-APU-CPU] PC={:06X} ${:04X} <- {:02X} frame={} sl={} cyc={} apu_cycles={}",
                                self.last_cpu_pc,
                                offset,
                                value,
                                self.ppu.get_frame(),
                                self.ppu.scanline,
                                self.ppu.get_cycle(),
                                apu_cycles
                            );
                        }
                        // burn-in-test.sfc: broader APU port write trace with frame correlation (opt-in).
                        if crate::debug_flags::trace_burnin_apu_writes()
                            && offset <= 0x2143
                            && (150..=420).contains(&self.ppu.get_frame())
                        {
                            use std::sync::atomic::{AtomicU32, Ordering};
                            static CNT: AtomicU32 = AtomicU32::new(0);
                            let n = CNT.fetch_add(1, Ordering::Relaxed);
                            if n < 2048 {
                                println!(
                                    "[BURNIN-APU-W] PC={:06X} ${:04X} <- {:02X} frame={} sl={} cyc={}",
                                    self.last_cpu_pc,
                                    offset,
                                    value,
                                    self.ppu.get_frame(),
                                    self.ppu.scanline,
                                    self.ppu.get_cycle()
                                );
                            }
                        }
                        if crate::debug_flags::trace_apu_port_all()
                            || (offset == 0x2140 && crate::debug_flags::trace_apu_port0())
                        {
                            use std::sync::atomic::{AtomicU32, Ordering};
                            static CNT: AtomicU32 = AtomicU32::new(0);
                            let n = CNT.fetch_add(1, Ordering::Relaxed);
                            if n < 512 {
                                println!(
                                    "[APU-W{}] ${:04X} <- {:02X} fake={} state={:?}",
                                    if self.fake_apu { "-FAKE" } else { "" },
                                    offset,
                                    value,
                                    self.fake_apu,
                                    self.fake_apu_upload_state
                                );
                            }
                        }
                        // Concise handshake trace (write side)
                        if crate::debug_flags::trace_apu_handshake() && offset <= 0x2143 {
                            use std::sync::atomic::{AtomicU32, Ordering};
                            static CNT: AtomicU32 = AtomicU32::new(0);
                            let n = CNT.fetch_add(1, Ordering::Relaxed);
                            let limit = crate::debug_flags::trace_apu_handshake_limit();
                            if n < limit {
                                if self.fake_apu {
                                    println!(
                                        "[APU-HS][W] ${:04X} <- {:02X} state=fake-{:?} pc={:06X} frame={} sl={} cyc={}",
                                        offset,
                                        value,
                                        self.fake_apu_upload_state,
                                        self.last_cpu_pc,
                                        self.ppu.get_frame(),
                                        self.ppu.scanline,
                                        self.ppu.get_cycle()
                                    );
                                } else if let Ok(mut apu) = self.apu.lock() {
                                    println!(
                                        "[APU-HS][W] ${:04X} <- {:02X} state={} pc={:06X} frame={} sl={} cyc={}",
                                        offset,
                                        value,
                                        apu.handshake_state_str(),
                                        self.last_cpu_pc,
                                        self.ppu.get_frame(),
                                        self.ppu.scanline,
                                        self.ppu.get_cycle()
                                    );
                                }
                            }
                        }
                        // 強制応答デバッグ: APU_FORCE_PORT0/1 が指定されていれば、その値を常にエコーする
                        if let Some(force0) = crate::debug_flags::apu_force_port0() {
                            self.fake_apu_ports[0] = force0;
                        }
                        if let Some(force1) = crate::debug_flags::apu_force_port1() {
                            self.fake_apu_ports[1] = force1;
                        }

                        if self.fake_apu {
                            // CPUテストHLEならポートは常に即応答 0x00 にする（文字列出力のみ利用）
                            if crate::debug_flags::cpu_test_hle() {
                                let port = (offset & 0x03) as usize;
                                self.fake_apu_ports[port] = value;
                                return;
                            }
                            let port = (offset & 0x03) as usize;

                            // Phase 1: waiting for IPL kick ($CC on port0)
                            if !self.fake_apu_booted {
                                // Remember address bytes even before kick (some games pre-load)
                                if port == 2 || port == 3 {
                                    self.fake_apu_upload_echo[port] = value;
                                }
                                if port == 0 {
                                    // Only a $CC kick should transition the fake APU out of the signature phase.
                                    // Other pre-kick writes are ignored so the CPU can still see AA/BB.
                                    if value == 0xCC {
                                        self.fake_apu_booted = true;
                                        self.fake_apu_upload_state = if self.fake_apu_fast_done {
                                            crate::fake_apu::FakeApuUploadState::Done
                                        } else {
                                            crate::fake_apu::FakeApuUploadState::Uploading
                                        };
                                        self.fake_apu_spc_addr = 0;
                                        self.fake_apu_spc_ram.fill(0);
                                        if crate::debug_flags::trace_fake_apu() {
                                            println!(
                                                "[FAKE-APU] STATE -> {:?} (on CC kick)",
                                                self.fake_apu_upload_state
                                            );
                                        }
                                        self.fake_apu_next_index = 0;
                                        self.fake_apu_run_cooldown =
                                            if self.fake_apu_fast_done { 0 } else { 0 };
                                        self.fake_apu_upload_bytes = 0;
                                        self.fake_apu_size = 0;
                                        self.fake_apu_size_set = false;
                                        self.fake_apu_upload_bytes = 0;
                                        self.fake_apu_upload_data_bytes = 0;
                                        self.fake_apu_idle_reads = 0;
                                        // Port0 immediately echoes the kick byte so host can verify.
                                        // In fast-done mode we still echo the kick value instead of clearing to 0.
                                        self.fake_apu_ports = if self.fake_apu_fast_done {
                                            [value, 0x00, 0x00, 0x00]
                                        } else {
                                            [value, 0xBB, 0x00, 0x00]
                                        };
                                    } else {
                                        // Preserve signature; remember the write for optional inspection.
                                        self.fake_apu_upload_echo[port] = value;
                                    }
                                }
                                if crate::debug_flags::trace_fake_apu() {
                                    println!(
                                        "[FAKE-APU] W ${:04X} <- {:02X} (preboot state={:?})",
                                        offset, value, self.fake_apu_upload_state
                                    );
                                }
                                return;
                            }

                            // Phase 2+: after kick, behave like the real IPL handshake:
                            // - CPU writes data to port1, index/kick to port0.
                            // - IPL echoes port0; port1=0 then port0=0xCC signals completion.
                            match self.fake_apu_upload_state {
                                crate::fake_apu::FakeApuUploadState::Uploading => {
                                    // Timeout: if port0 is read too many times without progress, force completion.
                                    if self.fake_apu_idle_reads > 2048 {
                                        self.fake_apu_upload_state =
                                            crate::fake_apu::FakeApuUploadState::Done;
                                        self.fake_apu_ports[0] = 0;
                                        self.fake_apu_ports[1] = 0;
                                        if crate::debug_flags::trace_fake_apu() {
                                            println!(
                                                "[FAKE-APU] Timeout -> Done (idle_reads={})",
                                                self.fake_apu_idle_reads
                                            );
                                        }
                                    }
                                    // Capture destination address (port2/3)
                                    if port == 2 {
                                        self.fake_apu_ports[2] = value;
                                        self.fake_apu_spc_addr =
                                            (self.fake_apu_spc_addr & 0xFF00) | value as u16;
                                    } else if port == 3 {
                                        self.fake_apu_ports[3] = value;
                                        self.fake_apu_spc_addr = (self.fake_apu_spc_addr & 0x00FF)
                                            | ((value as u16) << 8);
                                    }
                                    // Data or index
                                    match port {
                                        0 => {
                                            self.fake_apu_ports[0] = value; // echo counter/kick
                                            self.fake_apu_upload_bytes =
                                                self.fake_apu_upload_bytes.saturating_add(1);
                                            self.fake_apu_last_port0 = value;
                                            self.fake_apu_idle_reads = 0;
                                        }
                                        1 => {
                                            // Write into fake SPC RAM at current address, then advance
                                            let addr = self.fake_apu_spc_addr as usize & 0xFFFF;
                                            self.fake_apu_spc_ram[addr] = value;
                                            self.fake_apu_spc_addr =
                                                self.fake_apu_spc_addr.wrapping_add(1);
                                            self.fake_apu_ports[1] = value;
                                            self.fake_apu_upload_buf.push(value);
                                            self.fake_apu_upload_bytes =
                                                self.fake_apu_upload_bytes.saturating_add(1);
                                            self.fake_apu_upload_data_bytes =
                                                self.fake_apu_upload_data_bytes.saturating_add(1);
                                            self.fake_apu_idle_reads = 0;
                                        }
                                        2 | 3 => {
                                            // already handled
                                        }
                                        _ => {}
                                    }

                                    // Completion: port1=0 then port0 advances (>=last+2), or >=32KB uploaded (failsafe)
                                    let host_signaled_end = self.fake_apu_ports[1] == 0
                                        && port == 0
                                        && value.wrapping_sub(self.fake_apu_last_port0) >= 2
                                        && self.fake_apu_upload_data_bytes > 0;
                                    if host_signaled_end
                                        || self.fake_apu_upload_data_bytes >= 0x8000
                                    {
                                        self.fake_apu_upload_state =
                                            crate::fake_apu::FakeApuUploadState::Done;
                                        self.fake_apu_run_cooldown = 2;
                                        // 実機では完了後もエコー維持とされるためポートはクリアしない
                                        if crate::debug_flags::trace_fake_apu() {
                                            println!(
                                                "[FAKE-APU] Upload complete (bytes={})",
                                                self.fake_apu_upload_data_bytes
                                            );
                                        }
                                    }
                                }
                                crate::fake_apu::FakeApuUploadState::Done => {
                                    // 完了後もエコー維持（ポート0/1も最後の値を返す）
                                    self.fake_apu_ports[port] = value;
                                }
                                crate::fake_apu::FakeApuUploadState::WaitingCc => {
                                    // Shouldn't happen once booted, but echo defensively.
                                    self.fake_apu_ports[port] = value;
                                }
                            }

                            if crate::debug_flags::trace_fake_apu() {
                                use std::sync::atomic::{AtomicU32, Ordering};
                                static COUNT_W: AtomicU32 = AtomicU32::new(0);
                                let n = COUNT_W.fetch_add(1, Ordering::Relaxed);
                                if n < 128 {
                                    println!(
                                        "[FAKE-APU] W ${:04X} <- {:02X} (echo={:02X}) state={:?}",
                                        offset,
                                        value,
                                        self.fake_apu_ports[port],
                                        self.fake_apu_upload_state
                                    );
                                }
                            }
                        } else if let Ok(mut apu) = self.apu.lock() {
                            let p = (offset & 0x03) as u8;
                            if crate::debug_flags::trace_apu_port() {
                                use std::sync::atomic::{AtomicU32, Ordering};
                                static COUNT_W: AtomicU32 = AtomicU32::new(0);
                                let n = COUNT_W.fetch_add(1, Ordering::Relaxed);
                                if n < 256 {
                                    println!("[APU] W ${:04X} port{} <- {:02X}", offset, p, value);
                                }
                            }
                            apu.write_port(p, value);
                        }
                        // Optional: treat writes to $2140 as ASCII stream for test ROMs
                        if self.test_apu_print && offset == 0x2140 {
                            let ch = value as char;
                            if ch.is_ascii_graphic() || ch == ' ' || ch == '\n' || ch == '\r' {
                                self.test_apu_buf.push(ch);
                                if ch == '\n' || self.test_apu_buf.len() > 512 {
                                    let line = self.test_apu_buf.replace('\r', "");
                                    println!("[TESTROM] APU: {}", line.trim_end());
                                    let lower = line.to_ascii_lowercase();
                                    if lower.contains("passed") {
                                        println!("[TESTROM] PASS");
                                        crate::shutdown::request_quit();
                                    } else if lower.contains("fail") || lower.contains("failed") {
                                        println!("[TESTROM] FAIL");
                                        crate::shutdown::request_quit();
                                    }
                                    self.test_apu_buf.clear();
                                }
                            }
                        }
                    }
                    // WRAM access port
                    0x2180 => {
                        let addr = self.wram_address as usize;
                        if addr < self.wram.len() {
                            if (0x0100..=0x01FF).contains(&(addr as u32))
                                && crate::debug_flags::trace_wram_stack_dma()
                            {
                                println!(
                                    "[WRAM-STACK] PC={:06X} addr=0x{:05X} val=0x{:02X}",
                                    self.last_cpu_pc, addr, value
                                );
                            }
                            self.wram[addr] = value;
                            // WMADD ($2181-2183) is a 17-bit address; auto-increment carries across bit16.
                            self.wram_address = (self.wram_address + 1) & 0x1FFFF;
                            if crate::debug_flags::trace_wram_addr() {
                                static TRACE_WRAM_CNT: OnceLock<std::sync::atomic::AtomicU32> =
                                    OnceLock::new();
                                let n = TRACE_WRAM_CNT
                                    .get_or_init(|| std::sync::atomic::AtomicU32::new(0))
                                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                                if n < 32 {
                                    println!(
                                        "[WRAM PORT] W addr=0x{:05X} val=0x{:02X}",
                                        addr, value
                                    );
                                }
                            }
                        }
                    }
                    // WRAM Address registers
                    0x2181 => {
                        self.wram_address = (self.wram_address & 0xFFFF00) | (value as u32);
                        if crate::debug_flags::trace_wram_addr() {
                            println!(
                                "[WRAM ADR] write 2181 = {:02X} -> addr=0x{:05X}",
                                value, self.wram_address
                            );
                        }
                    }
                    0x2182 => {
                        self.wram_address = (self.wram_address & 0xFF00FF) | ((value as u32) << 8);
                        if crate::debug_flags::trace_wram_addr() {
                            println!(
                                "[WRAM ADR] write 2182 = {:02X} -> addr=0x{:05X}",
                                value, self.wram_address
                            );
                        }
                    }
                    0x2183 => {
                        self.wram_address =
                            (self.wram_address & 0x00FFFF) | (((value & 0x01) as u32) << 16);
                        if crate::debug_flags::trace_wram_addr() {
                            println!(
                                "[WRAM ADR] write 2183 = {:02X} -> addr=0x{:05X}",
                                value, self.wram_address
                            );
                        }
                    }
                    // Expansion area - ignore writes
                    0x2184..=0x21FF => {}
                    // SA-1 I-RAM window for S-CPU
                    0x3000..=0x37FF if self.is_sa1_active() => {
                        let idx = (offset - 0x3000) as usize;
                        if idx < self.sa1_iram.len() {
                            self.sa1_iram[idx] = value;
                        }
                    }
                    0x2200..=0x3FFF => {}
                    // Controller/IO registers
                    0x4000..=0x42FF => self.write_io_register(offset, value),
                    // DMA registers
                    0x4300..=0x43FF => {
                        if crate::debug_flags::trace_dma_reg_pc() {
                            let pc = self.last_cpu_pc;
                            println!(
                                "[DMA-REG-PC] PC={:06X} W ${:04X} val={:02X}",
                                pc, offset, value
                            );
                        }
                        if crate::debug_flags::trace_dma_addr() {
                            println!(
                                "[DMA-REG-W] bank={:02X} addr={:04X} value=0x{:02X}",
                                bank, offset, value
                            );
                        }
                        self.dma_controller.write(offset, value);
                        self.dma_reg_writes = self.dma_reg_writes.saturating_add(1);
                    }
                    // More IO registers
                    0x4400..=0x5FFF => self.write_io_register(offset, value),
                    // Expansion area/unused
                    0x6000..=0x7FFF => {
                        match self.mapper_type {
                            crate::cartridge::MapperType::LoRom => {
                                // SRAM window in system banks ($00-$3F/$80-$BF): $6000-$7FFF (8KB)
                                if self.sram_size > 0 {
                                    let bank_index = (bank & 0x3F) as usize;
                                    let window = bank_index * 0x2000 + ((offset - 0x6000) as usize);
                                    let idx = window % self.sram_size;
                                    self.sram[idx] = value;
                                    self.sram_dirty = true;
                                }
                            }
                            crate::cartridge::MapperType::HiRom
                            | crate::cartridge::MapperType::ExHiRom => {
                                // SRAM in HiROM
                                let sram_addr =
                                    ((bank as usize) << 13) + ((offset - 0x6000) as usize);
                                if sram_addr < self.sram.len() {
                                    self.sram[sram_addr] = value;
                                    self.sram_dirty = true;
                                }
                            }
                            crate::cartridge::MapperType::DragonQuest3 => {
                                // DQ3: treat 6000-7FFF as BW-RAM window via SA-1 mapping
                                if let Some(idx) = self.sa1_bwram_addr(offset) {
                                    self.sa1_bwram[idx] = value;
                                    self.sram_dirty = true;
                                }
                            }
                            _ => {} // Other mappers
                        }
                    }
                    // ROM area - writes ignored
                    0x8000..=0xFFFF => {}
                }
            }
            // ROM banks 40-7D - writes to SRAM only
            0x40..=0x7D => {
                match self.mapper_type {
                    crate::cartridge::MapperType::LoRom => {
                        // Banks 0x70-0x7D, offsets 0x0000-0x7FFF map to SRAM
                        if self.sram_size > 0 && offset < 0x8000 && (0x70..=0x7D).contains(&bank) {
                            let window = ((bank - 0x70) as usize) * 0x8000 + (offset as usize);
                            let idx = window % self.sram_size;
                            self.sram[idx] = value;
                            self.sram_dirty = true;
                        }
                        // Upper half is ROM, ignore writes
                    }
                    crate::cartridge::MapperType::HiRom | crate::cartridge::MapperType::ExHiRom => {
                        // SRAM area varies by bank in HiROM
                        if (0x6000..0x8000).contains(&offset) {
                            let sram_addr =
                                ((bank - 0x40) as usize) * 0x2000 + ((offset - 0x6000) as usize);
                            if sram_addr < self.sram.len() {
                                self.sram[sram_addr] = value;
                                self.sram_dirty = true;
                            }
                        }
                        // Other areas are ROM, ignore writes
                    }
                    crate::cartridge::MapperType::DragonQuest3 => {
                        // DQ3 SRAM (HiROMベース)
                        if (0x6000..0x8000).contains(&offset) {
                            let sram_addr =
                                ((bank - 0x40) as usize) * 0x2000 + ((offset - 0x6000) as usize);
                            if sram_addr < self.sram.len() {
                                self.sram[sram_addr] = value;
                                self.sram_dirty = true;
                            }
                        }
                        // その他はエンハンスメントチップ処理または無視
                    }
                    _ => {}
                }
            }
            // Extended WRAM banks
            0x7E..=0x7F => {
                let wram_addr = if self.wram_64k_mirror {
                    (offset as usize) & 0xFFFF
                } else {
                    ((bank - 0x7E) as usize) * 0x10000 + (offset as usize)
                };
                if (0x1002C..=0x1002D).contains(&wram_addr)
                    && crate::debug_flags::trace_dq3_sp_mem()
                {
                    println!(
                        "DQ3_SP_MEM write {:02X}:{:04X} -> WRAM[0x{:05X}] = {:02X}",
                        bank, offset, wram_addr, value
                    );
                }
                // Watch suspected handshake flag 7F:7DC0 (opt-in)
                if wram_addr == 0x1FDC0
                    && crate::debug_flags::trace_handshake()
                    && !crate::debug_flags::quiet()
                {
                    println!(
                        "[WRAM 7F:7DC0 WRITE] val=0x{:02X} bank={:02X} off={:04X}",
                        value, bank, offset
                    );
                }
                if self.trace_nmi_wram {
                    use std::sync::atomic::{AtomicU32, Ordering};
                    static WRITE_COUNT: AtomicU32 = AtomicU32::new(0);
                    if let Some(label) = match wram_addr {
                        0x07DE => Some("00:07DE"),
                        0x07E0 => Some("00:07E0"),
                        0x07E4 => Some("00:07E4"),
                        0x07F6 => Some("00:07F6"),
                        0x0FDE => Some("7E:0FDE"),
                        0x0FE0 => Some("7E:0FE0"),
                        0x0FE4 => Some("7E:0FE4"),
                        0x0FF6 => Some("7E:0FF6"),
                        0x1FDE => Some("7F:0FDE"),
                        0x1FE0 => Some("7F:0FE0"),
                        0x1FE4 => Some("7F:0FE4"),
                        0x1FF6 => Some("7F:0FF6"),
                        _ => None,
                    } {
                        let idx = WRITE_COUNT.fetch_add(1, Ordering::Relaxed);
                        if idx < 64 {
                            println!(
                                "[WRAM TRACE WRITE {}] val=0x{:02X} bank={:02X} off={:04X}",
                                label, value, bank, offset
                            );
                        }
                    }
                }
                if wram_addr < self.wram.len() {
                    self.wram[wram_addr] = value;
                }
            }
            // ROM mirror banks - writes ignored (except SRAM areas)
            0xC0..=0xFF => {
                // Some SRAM might be accessible here depending on mapper
                match self.mapper_type {
                    crate::cartridge::MapperType::LoRom => {
                        // LoROM: banks $F0-$FD mirror SRAM banks $70-$7D in $0000-$7FFF
                        if self.sram_size > 0 && offset < 0x8000 {
                            let mirror_bank = bank.wrapping_sub(0x80);
                            if (0x70..=0x7D).contains(&mirror_bank) {
                                let window =
                                    ((mirror_bank - 0x70) as usize) * 0x8000 + (offset as usize);
                                let idx = window % self.sram_size;
                                self.sram[idx] = value;
                                self.sram_dirty = true;
                            }
                        }
                    }
                    crate::cartridge::MapperType::HiRom | crate::cartridge::MapperType::ExHiRom => {
                        if (0x6000..0x8000).contains(&offset) {
                            let sram_addr =
                                ((bank - 0xC0) as usize) * 0x2000 + ((offset - 0x6000) as usize);
                            if sram_addr < self.sram.len() {
                                self.sram[sram_addr] = value;
                            }
                        }
                    }
                    crate::cartridge::MapperType::DragonQuest3 => {
                        // DQ3 SRAM (ミラー領域)
                        if (0x6000..0x8000).contains(&offset) {
                            let sram_addr =
                                ((bank - 0xC0) as usize) * 0x2000 + ((offset - 0x6000) as usize);
                            if sram_addr < self.sram.len() {
                                self.sram[sram_addr] = value;
                            }
                        }
                    }
                    _ => {} // Other mappers don't have SRAM here
                }
            }
            // Other banks - ignore writes
            _ => {}
        }
    }

    pub fn read_u16(&mut self, addr: u32) -> u16 {
        // CPUテストROMで $4210 を16bit読みした場合、上位バイトにも bit7 を複製して
        // BIT (16bit) でも VBlank フラグを検出できるようにする。
        if self.cpu_test_mode && addr == 0x004210 {
            let lo = self.read_u8(addr) as u16;
            let hi = if (lo & 0x80) != 0 { 0x80 } else { 0x00 };
            return (hi << 8) | lo;
        }
        // Special case for interrupt vectors in Dragon Quest III
        if (0xFFE0..=0xFFFF).contains(&addr)
            && self.mapper_type == crate::cartridge::MapperType::DragonQuest3
        {
            // Read from ROM directly for interrupt vectors
            let rom_offset = addr as usize;
            if rom_offset + 1 < self.rom.len() {
                let lo = self.rom[rom_offset] as u16;
                let hi = self.rom[rom_offset + 1] as u16;
                let result = lo | (hi << 8);

                // Debug for BRK vector
                if addr == 0xFFFE {
                    unsafe {
                        static mut VECTOR_DEBUG: u32 = 0;
                        VECTOR_DEBUG += 1;
                        if VECTOR_DEBUG <= 5 {
                            println!("Reading BRK/IRQ vector at 0x{:04X}: 0x{:04X} (ROM[{:06X}]={:02X},{:02X})", 
                                     addr, result, rom_offset, lo as u8, hi as u8);
                        }
                    }
                }
                return result;
            }
        }

        let lo = self.read_u8(addr) as u16;
        let hi = self.read_u8(addr.wrapping_add(1)) as u16;
        (hi << 8) | lo
    }

    pub fn write_u16(&mut self, addr: u32, value: u16) {
        self.write_u8(addr, (value & 0xFF) as u8);
        self.write_u8(addr.wrapping_add(1), (value >> 8) as u8);
    }

    fn read_io_register(&mut self, addr: u16) -> u8 {
        match addr {
            0x4016 => {
                // JOYSER0 ($4016): returns two bits (D0/D1) per read.
                // Standard controllers use only D0 (bit0). D1 (bit1) is used by multitap/etc.
                let d0 = self.input_system.read_controller1() & 1;
                let d1 = if self.input_system.is_multitap_enabled() {
                    self.input_system.read_controller3() & 1
                } else {
                    0
                };
                let v = d0 | (d1 << 1);
                if std::env::var_os("TRACE_4016").is_some() {
                    use std::sync::atomic::{AtomicU32, Ordering};
                    static COUNT: AtomicU32 = AtomicU32::new(0);
                    let n = COUNT.fetch_add(1, Ordering::Relaxed);
                    if n < 256 {
                        println!(
                            "[TRACE4016] read#{} $4016 -> 0b{:02b} PC={:06X}",
                            n + 1,
                            v & 0x03,
                            self.last_cpu_pc,
                        );
                    }
                }
                v
            }
            0x4017 => {
                // JOYSER1 ($4017): returns two bits (D0/D1) per read plus fixed 1s in bits2-4.
                let d0 = self.input_system.read_controller2() & 1;
                let d1 = if self.input_system.is_multitap_enabled() {
                    self.input_system.read_controller4() & 1
                } else {
                    0
                };
                let v = 0x1C | d0 | (d1 << 1);
                if std::env::var_os("TRACE_4016").is_some() {
                    use std::sync::atomic::{AtomicU32, Ordering};
                    static COUNT: AtomicU32 = AtomicU32::new(0);
                    let n = COUNT.fetch_add(1, Ordering::Relaxed);
                    if n < 256 {
                        println!(
                            "[TRACE4016] read#{} $4017 -> 0b{:02b} PC={:06X}",
                            n + 1,
                            v & 0x03,
                            self.last_cpu_pc,
                        );
                    }
                }
                v
            }
            // 0x4210 - RDNMI: NMI flag and version
            0x4210 => {
                // 強制デバッグ: 常に 0x82 を返す（ループ脱出用）
                if std::env::var_os("RDNMI_ALWAYS_82").is_some() {
                    if std::env::var_os("TRACE_4210").is_some() {
                        println!(
                            "[TRACE4210] read(force 0x82) PC={:06X} vblank={} nmi_en={}",
                            self.last_cpu_pc,
                            self.ppu.is_vblank(),
                            self.ppu.nmi_enabled
                        );
                    }
                    return 0x82;
                }
                // BIT $4210 ループ専用ハック: PC が 0x825B/0x8260/0x8263 のときは 0x82 を返す（VBlank判定なし）
                // 環境変数 RDNMI_FORCE_BITLOOP=1 で有効化
                if std::env::var_os("RDNMI_FORCE_BITLOOP").is_some()
                    && (self.last_cpu_pc == 0x00825B
                        || self.last_cpu_pc == 0x008260
                        || self.last_cpu_pc == 0x008263)
                {
                    // ラッチは一度クリアしておく
                    self.ppu.nmi_flag = false;
                    self.ppu.nmi_latched = false;
                    self.rdnmi_consumed = true;
                    return 0x82;
                }
                // BITループ専用ハック/ワンショットは現状なし（実機準拠）
                // デバッグ: 強制 0x82 を一度だけ返す (FORCE_4210_ONCE=1)
                use std::sync::atomic::{AtomicBool, Ordering};
                static FORCE_4210_ONCE_DONE: AtomicBool = AtomicBool::new(false);
                let force_once = std::env::var_os("FORCE_4210_ONCE").is_some();
                if force_once && !FORCE_4210_ONCE_DONE.load(Ordering::Relaxed) {
                    FORCE_4210_ONCE_DONE.store(true, Ordering::Relaxed);
                    return 0x82;
                }
                // CPUテスト専用の強制 0x82 は環境変数 CPUTEST_FORCE_82 がある場合のみ
                if self.cpu_test_mode && std::env::var_os("CPUTEST_FORCE_82").is_some() {
                    if std::env::var_os("TRACE_4210").is_some() {
                        println!(
                            "[TRACE4210] read(cpu_test_mode force) PC={:06X} vblank={} nmi_en={}",
                            self.last_cpu_pc,
                            self.ppu.is_vblank(),
                            self.ppu.nmi_enabled
                        );
                    }
                    return 0x82;
                }

                // CPUテストHLE
                if crate::debug_flags::cpu_test_hle() {
                    let vblank = self.ppu.is_vblank();
                    let force = crate::debug_flags::cpu_test_hle_force();
                    let val = if force {
                        0x82 // 常時強制
                    } else if crate::debug_flags::cpu_test_hle_strict_vblank() {
                        if vblank {
                            0x82
                        } else {
                            0x02
                        }
                    } else {
                        0x82
                    };
                    if std::env::var_os("TRACE_4210").is_some() {
                        println!(
                            "[TRACE4210] read(cpu_test_hle) PC={:06X} vblank={} nmi_en={} -> {:02X}",
                            self.last_cpu_pc,
                            vblank,
                            self.ppu.nmi_enabled,
                            val
                        );
                    }
                    return val;
                }

                // デフォルトはバージョン 0x02。bit7(NMIフラグ)は PPU 側で立てた nmi_flag を返すだけ。
                let mut value = 0x02;
                if std::env::var_os("FORCE_NMI_FLAG").is_some() {
                    self.ppu.nmi_flag = true;
                }
                static FORCE_RDNMI_ONCE_DONE: AtomicBool = AtomicBool::new(false);
                // 起動直後1回だけ強制で bit7 を立てる（環境変数がなくても CPU テスト時は実行）
                let force_once_env = std::env::var_os("FORCE_RDNMI_ONCE").is_some();
                let force_once_auto =
                    self.cpu_test_mode && !FORCE_RDNMI_ONCE_DONE.load(Ordering::Relaxed);
                if (force_once_env || force_once_auto)
                    && !FORCE_RDNMI_ONCE_DONE.load(Ordering::Relaxed)
                {
                    FORCE_RDNMI_ONCE_DONE.store(true, Ordering::Relaxed);
                    self.ppu.nmi_flag = true;
                }

                let in_vblank = self.ppu.is_vblank();
                // 電源投入直後の特別扱いはしない（実機準拠に戻す）
                let sticky_power_on = false;
                if self.ppu.nmi_flag {
                    value |= 0x80;
                }
                if sticky_power_on {
                    value |= 0x80;
                    self.ppu.nmi_flag = true;
                }
                if std::env::var_os("RDNMI_FORCE_ON").is_some() {
                    value |= 0x80;
                }
                if std::env::var_os("RDNMI_FORCE_VBL").is_some() && in_vblank {
                    value |= 0x80;
                }
                if std::env::var_os("RDNMI_ALWAYS_82").is_some() {
                    value = 0x82;
                }

                // CPUテスト時は16bit BIT対策で上位バイトにもbit7を複製
                if self.cpu_test_mode {
                    self.rdnmi_high_byte_for_test = if (value & 0x80) != 0 { 0x80 } else { 0x00 };
                }

                // 実機同様、読み出しでラッチをクリア。sticky指定時のみ保持。
                let sticky_rdnmi = std::env::var_os("RDNMI_STICKY").is_some();
                if !sticky_rdnmi && !sticky_power_on {
                    self.ppu.nmi_flag = false;
                    self.rdnmi_consumed = true;
                }

                if crate::debug_flags::trace_burnin_v224() {
                    let pc16 = (self.last_cpu_pc & 0xFFFF) as u16;
                    if (0x97D0..=0x98FF).contains(&pc16) {
                        use std::sync::atomic::{AtomicU8, Ordering};
                        static LAST: AtomicU8 = AtomicU8::new(0xFF);
                        let prev = LAST.swap(value, Ordering::Relaxed);
                        // Log only on NMI-flag (bit7) edges to avoid spamming tight loops.
                        if (prev ^ value) & 0x80 != 0 {
                            println!(
                                "[BURNIN-V224][RDNMI] PC={:06X} sl={} cyc={} vblank={} nmi_en={} {:02X}->{:02X}",
                                self.last_cpu_pc,
                                self.ppu.scanline,
                                self.ppu.get_cycle(),
                                self.ppu.is_vblank() as u8,
                                self.ppu.nmi_enabled as u8,
                                prev,
                                value
                            );
                        }
                    }
                }

                if std::env::var_os("TRACE_4210").is_some() {
                    use std::sync::atomic::{AtomicU32, Ordering};
                    static COUNT: AtomicU32 = AtomicU32::new(0);
                    let n = COUNT.fetch_add(1, Ordering::Relaxed);
                    let interesting = self.ppu.is_vblank() || (value & 0x80) != 0 || n < 64;
                    if interesting {
                        println!(
                            "[TRACE4210] read#{} value=0x{:02X} (nmi_flag_after_clear={} vblank={} nmi_en={}) PC={:06X} scanline={} cycle={}",
                            n + 1,
                            value,
                            self.ppu.nmi_flag,
                            self.ppu.is_vblank(),
                            self.ppu.nmi_enabled,
                            self.last_cpu_pc,
                            self.ppu.scanline,
                            self.ppu.get_cycle()
                        );
                    }
                }
                value
            }
            // 0x4211 - TIMEUP: IRQ time-up (read/clear)
            0x4211 => {
                let v = if self.cpu_test_mode {
                    // 高バイトにもbit7を残し、BIT (16bit) でもVBlankを検出できるようにする。
                    self.rdnmi_high_byte_for_test
                } else if self.irq_pending {
                    0x80
                } else {
                    0x00
                };
                self.irq_pending = false; // reading clears
                v
            }
            // 0x4212 - HVBJOY: H/V-Blank and Joypad busy flags
            0x4212 => {
                // デバッグ: 強制値を返す（例: 0x80 なら VBlank=1, HBlank=0, JOYBUSY=0）
                if let Some(force) = std::env::var("FORCE_4212")
                    .ok()
                    .and_then(|v| u8::from_str_radix(v.trim_start_matches("0x"), 16).ok())
                {
                    return force;
                }
                let mut value = 0u8;
                if crate::debug_flags::cpu_test_hle_force() {
                    value = 0x80; // VBlank=1, HBlank=0, JOYBUSY=0
                } else {
                    if self.ppu.is_vblank() {
                        value |= 0x80;
                    }
                    if self.ppu.is_hblank() {
                        value |= 0x40;
                    }
                    // bit0 (JOYBUSY): set while auto-joypad is running
                    if self.joy_busy_counter > 0 {
                        value |= 0x01;
                    }
                }
                // Debug: log transitions of $4212 to confirm VBlank/HBlank visibility (opt-in)
                if std::env::var_os("TRACE_4212_VALUES").is_some() && !crate::debug_flags::quiet() {
                    use std::sync::atomic::{AtomicU32, AtomicU8, Ordering};
                    static LAST: AtomicU8 = AtomicU8::new(0xFF);
                    static COUNT: AtomicU32 = AtomicU32::new(0);
                    let prev = LAST.swap(value, Ordering::Relaxed);
                    // Log only when VBlank bit (bit7) toggles to avoid flooding with HBlank edges.
                    if (prev ^ value) & 0x80 != 0 {
                        let n = COUNT.fetch_add(1, Ordering::Relaxed);
                        if n < 64 {
                            println!(
                                "[4212] change#{:02} {:02X}->{:02X} vblank={} hblank={} joybusy={} scanline={} cycle={} PC={:06X}",
                                n + 1,
                                prev,
                                value,
                                self.ppu.is_vblank() as u8,
                                self.ppu.is_hblank() as u8,
                                (self.joy_busy_counter > 0) as u8,
                                self.ppu.scanline,
                                self.ppu.get_cycle(),
                                self.last_cpu_pc
                            );
                        }
                    }
                }
                // Debug: dump reads to see JOYBUSY behavior (opt-in)
                if std::env::var_os("DEBUG_JOYBUSY").is_some() && !crate::debug_flags::quiet() {
                    use std::sync::atomic::{AtomicU32, Ordering};
                    static LOG_COUNT: AtomicU32 = AtomicU32::new(0);
                    let idx = LOG_COUNT.fetch_add(1, Ordering::Relaxed);
                    if idx < 128 {
                        println!(
                            "[JOYBUSY] read#{:03} value=0x{:02X} counter={} vblank={} hblank={} scanline={} cycle={}",
                            idx + 1,
                            value,
                            self.joy_busy_counter,
                            self.ppu.is_vblank() as u8,
                            self.ppu.is_hblank() as u8,
                            self.ppu.scanline,
                            self.ppu.get_cycle()
                        );
                    }
                }
                value
            }
            // 0x4213 - RDIO: Programmable I/O port readback
            0x4213 => {
                // Minimal behavior: return last value written to $4201.
                // Some hw ties bits to controller/expansion; we keep it simple for now.
                self.wio
            }
            // JOY1/2/3/4 data
            0x4218 => {
                // cputest-full 初期操作補助（ヘッドレス向け）:
                // 最初の読みで未押下、次の読みでAボタン押下(bit7=1)を返し、
                // BPL/BMI の二段階チェックを通過させる。
                // - CPU_TEST_MODE により cpu_test_mode が有効になるが、ウィンドウ表示では入力を尊重するため無効
                // - 明示的に有効化したい場合は CPUTEST_AUTORIGHT=1
                if (self.cpu_test_mode && crate::debug_flags::headless())
                    || std::env::var_os("CPUTEST_AUTORIGHT").is_some()
                {
                    let always = std::env::var_os("CPUTEST_AUTORIGHT").is_some();
                    let auto_active = always
                        || (self.cpu_test_mode
                            && self.ppu.frame() < (self.cpu_test_auto_frames as u64));
                    if auto_active {
                        let mut val = self.joy_data[0];
                        if self.cpu_test_auto_joy_phase == 0 {
                            self.cpu_test_auto_joy_phase = 1;
                            // まず未押下を返す（A=0, 1=Low=Pressed）
                            val &= !0x80;
                            if std::env::var_os("TRACE_CPU_TEST_AUTO").is_some() {
                                println!(
                                    "[CPU-TEST-AUTO] JOY1L read (unpressed) val=0x{:02X}",
                                    val
                                );
                            }
                            return val;
                        }
                        self.cpu_test_auto_joy_phase = 0;
                        // 次はA押下を返す（bit7=1）
                        val |= 0x80;
                        if std::env::var_os("TRACE_CPU_TEST_AUTO").is_some() {
                            println!("[CPU-TEST-AUTO] JOY1L read (A) val=0x{:02X}", val);
                        }
                        return val;
                    }
                }
                let force = std::env::var("JOY1L_FORCE")
                    .ok()
                    .and_then(|v| u8::from_str_radix(v.trim_start_matches("0x"), 16).ok());
                let val = force.unwrap_or(self.joy_data[0]);
                if std::env::var_os("TRACE_4218").is_some() {
                    use std::sync::atomic::{AtomicU32, Ordering};
                    static COUNT: AtomicU32 = AtomicU32::new(0);
                    let n = COUNT.fetch_add(1, Ordering::Relaxed);
                    if n < 32 {
                        println!(
                            "[TRACE4218] read#{:02} value=0x{:02X} joy_data[0]=0x{:02X} joy_data[1]=0x{:02X} vblank={} busy={} PC={:06X} force={:?}",
                            n + 1,
                            val,
                            self.joy_data[0],
                            self.joy_data[1],
                            self.ppu.is_vblank(),
                            self.joy_busy_counter,
                            self.last_cpu_pc,
                            force
                        );
                    }
                }
                val
            } // JOY1L
            0x4219 => self.joy_data[1], // JOY1H
            0x421A => self.joy_data[2], // JOY2L
            0x421B => self.joy_data[3], // JOY2H
            0x421C => self.joy_data[4], // JOY3L
            0x421D => self.joy_data[5], // JOY3H
            0x421E => self.joy_data[6], // JOY4L
            0x421F => self.joy_data[7], // JOY4H
            // Hardware multiplication/division results
            // 0x4214/0x4215: Quotient (low/high)
            0x4214 => (self.div_quot & 0xFF) as u8,
            0x4215 => (self.div_quot >> 8) as u8,
            // 0x4216/0x4217: Multiplication result (if last op was MUL) or Division remainder
            0x4216 => (self.mul_result & 0xFF) as u8, // or div_rem low after DIV
            0x4217 => (self.mul_result >> 8) as u8,   // or div_rem high after DIV
            // $420B/$420C are write-only (W8). Reads return open bus.
            0x420B => self.mdr,
            0x420C => self.mdr,
            // APU registers readback
            0x2140..=0x217F => {
                let port = (addr & 0x3F) as u8;
                // デバッグ: APU_FORCE_PORT{0,1} で固定値を返す
                if port == 0x00 {
                    if let Some(v) = crate::debug_flags::apu_force_port0() {
                        return v;
                    }
                } else if port == 0x01 {
                    if let Some(v) = crate::debug_flags::apu_force_port1() {
                        return v;
                    }
                }
                // SMW APU HLE: 2140 reads echo連動で WRAM DMAバッファの内容を返す
                if self.smw_apu_hle && !self.smw_apu_hle_buf.is_empty() && !self.smw_apu_hle_done {
                    let idx = (self.smw_apu_hle_echo_idx as usize) % self.smw_apu_hle_buf.len();
                    let v = self.smw_apu_hle_buf[idx];
                    self.smw_apu_hle_echo_idx = self.smw_apu_hle_echo_idx.wrapping_add(1);
                    return v;
                }
                if let Ok(mut apu) = self.apu.lock() {
                    let v = apu.read_port(port & 0x03);
                    if crate::debug_flags::boot_verbose() {
                        static mut APU_RD_LOG: u32 = 0;
                        unsafe {
                            APU_RD_LOG += 1;
                            if APU_RD_LOG <= 16 {
                                println!("APU READ  port=0x{:02X} -> 0x{:02X}", port, v);
                            }
                        }
                    }
                    v
                } else {
                    0x00
                }
            }
            _ => 0xFF,
        }
    }

    fn write_io_register(&mut self, addr: u16, value: u8) {
        match addr {
            // Controller ports
            0x4016 => {
                if std::env::var_os("TRACE_4016").is_some() {
                    use std::sync::atomic::{AtomicU32, Ordering};
                    static COUNT: AtomicU32 = AtomicU32::new(0);
                    let n = COUNT.fetch_add(1, Ordering::Relaxed);
                    if n < 256 {
                        println!(
                            "[TRACE4016] write#{} $4016 <- 0x{:02X} PC={:06X}",
                            n + 1,
                            value,
                            self.last_cpu_pc
                        );
                    }
                }
                self.input_system.write_strobe(value);
            }
            // PPU/CPU communication
            0x4200 => {
                let pc = self.last_cpu_pc;
                // NMITIMEN - Interrupt Enable Register
                let mut actual_value = value;

                // SA-1 NMI delay: prevent NMI enable during SA-1 initialization
                if self.sa1_nmi_delay_active && (value & 0x80) != 0 {
                    actual_value = value & 0x7F; // Clear NMI enable bit
                    static mut NMI_DELAY_LOG_COUNT: u32 = 0;
                    unsafe {
                        NMI_DELAY_LOG_COUNT += 1;
                        if NMI_DELAY_LOG_COUNT <= 10
                            && std::env::var_os("DEBUG_SA1_SCHEDULER").is_some()
                        {
                            println!("SA-1 NMI delay: blocked $4200 NMI enable (value=0x{:02X} -> 0x{:02X})",
                                value, actual_value);
                        }
                    }
                }

                self.nmitimen = actual_value;
                self.nmitimen_writes_count = self.nmitimen_writes_count.saturating_add(1);
                let prev_nmi_en = self.ppu.nmi_enabled;
                let nmi_en = (actual_value & 0x80) != 0;
                self.ppu.nmi_enabled = nmi_en;
                self.irq_h_enabled = (value & 0x10) != 0;
                self.irq_v_enabled = (value & 0x20) != 0;
                // Reset HV shadow when enables change
                self.irq_v_matched_line = None;
                // If NMI is enabled mid-VBlank, hardware may latch an NMI immediately *only if*
                // the NMI flag ($4210 bit7) is still set (i.e., the VBlank-edge has occurred and
                // has not yet been acknowledged via $4210 read).
                if nmi_en
                    && !prev_nmi_en
                    && self.ppu.is_vblank()
                    && self.ppu.nmi_flag
                    && !self.ppu.is_nmi_latched()
                {
                    self.ppu.latch_nmi_now();
                }
                // bit0: auto-joypad enable (ignored here)
                if crate::debug_flags::boot_verbose() && !crate::debug_flags::quiet() {
                    println!(
                        "$4200 NMITIMEN write: 0x{:02X} (NMI:{}, IRQ:{}, Auto-joypad:{}) PC={:06X}",
                        self.nmitimen,
                        (self.nmitimen & 0x80) != 0,
                        (self.nmitimen & 0x20) != 0,
                        (self.nmitimen & 0x01) != 0,
                        pc
                    );
                }
            }
            // WRIO - Joypad Programmable I/O Port; read back via $4213
            0x4201 => {
                // Bit7 ("a") is connected to the PPU latch line.
                // HV counter latch via WRIO: latching occurs on the 1->0 transition (writing 0),
                // and it latches 1 dot later than a $2137 read (see Super Famicom Dev Wiki "Timing").
                let prev = self.wio;
                self.wio = value;
                let prev_a = (prev & 0x80) != 0;
                let new_a = (value & 0x80) != 0;
                self.ppu.set_wio_latch_enable(new_a);
                if crate::debug_flags::trace_burnin_ext_latch() {
                    use std::sync::atomic::{AtomicU32, Ordering};
                    static CNT: AtomicU32 = AtomicU32::new(0);
                    let n = CNT.fetch_add(1, Ordering::Relaxed);
                    if n < 1024 {
                        println!(
                            "[BURNIN-EXT][WRIO] PC={:06X} $4201 <- {:02X} (prev={:02X}) sl={} cyc={}",
                            self.last_cpu_pc,
                            value,
                            prev,
                            self.ppu.scanline,
                            self.ppu.get_cycle()
                        );
                    }
                }
                if prev_a && !new_a {
                    self.ppu.request_wrio_hv_latch();
                }
            }
            0x4202 => {
                // WRMPYA - Multiplicand A (8-bit)
                self.mul_a = value;
            }
            0x4203 => {
                // WRMPYB - Multiplicand B (start 8x8 multiply)
                self.mul_b = value;
                // Any in-flight divide is aborted (single shared math unit behavior).
                self.div_busy = false;
                self.div_just_started = false;

                if self.mul_busy {
                    // Real hardware quirk: writing to WRMPYB again before the 8-cycle
                    // multiply has completed does *not* correctly restart the unit; the
                    // remaining cycles continue and the result becomes "corrupted".
                    // Model this by updating the internal multiplier shift register only.
                    self.mul_work_b = self.mul_b;
                } else {
                    // Start 8-cycle multiply; results ($4216/$4217) update while in-flight.
                    self.mul_busy = true;
                    self.mul_just_started = true;
                    self.mul_cycles_left = 8;
                    self.mul_work_a = self.mul_a as u16;
                    self.mul_work_b = self.mul_b;
                    self.mul_partial = 0;
                    self.mul_result = 0;
                }
            }
            0x4204 => {
                // WRDIVL - Dividend Low
                self.div_a = (self.div_a & 0xFF00) | (value as u16);
            }
            0x4205 => {
                // WRDIVH - Dividend High
                self.div_a = (self.div_a & 0x00FF) | ((value as u16) << 8);
            }
            0x4206 => {
                // WRDIVB - Divisor (start 16/8 divide)
                self.div_b = value;
                // Abort in-flight multiply (single shared math unit behavior).
                self.mul_busy = false;
                self.mul_just_started = false;

                if self.div_b == 0 {
                    // Division-by-zero special case.
                    self.div_quot = 0xFFFF;
                    self.div_rem = self.div_a;
                    self.mul_result = self.div_rem;
                    self.div_busy = false;
                    self.div_just_started = false;
                    self.div_cycles_left = 0;
                    self.div_work_dividend = 0;
                    self.div_work_divisor = 0;
                    self.div_work_quot = 0;
                    self.div_work_rem = 0;
                    self.div_work_bit = 0;
                } else {
                    // 16-cycle restoring division; results ($4214-$4217) update while in-flight.
                    self.div_busy = true;
                    self.div_just_started = true;
                    self.div_cycles_left = 16;
                    self.div_work_dividend = self.div_a;
                    self.div_work_divisor = self.div_b;
                    self.div_work_quot = 0;
                    self.div_work_rem = 0;
                    self.div_work_bit = 15;
                    self.div_quot = 0;
                    self.div_rem = 0;
                    self.mul_result = 0;
                }
            }
            0x4207 => {
                // HTIMEL - Horizontal Timer Low
                self.h_timer = (self.h_timer & 0xFF00) | (value as u16);
                self.h_timer_set = true;
            }
            0x4208 => {
                // HTIMEH - Horizontal Timer High
                self.h_timer = (self.h_timer & 0x00FF) | ((value as u16) << 8);
                self.h_timer_set = true;
            }
            0x4209 => {
                // VTIMEL - Vertical Timer Low
                self.v_timer = (self.v_timer & 0xFF00) | (value as u16);
                self.v_timer_set = true;
            }
            0x420A => {
                // VTIMEH - Vertical Timer High
                self.v_timer = (self.v_timer & 0x00FF) | ((value as u16) << 8);
                self.v_timer_set = true;
            }
            0x420B => {
                // MDMAEN - General DMA Enable
                if std::env::var_os("TRACE_DMA_REG_PC").is_some() {
                    println!(
                        "[DMA-EN-PC] PC={:06X} W $420B val={:02X}",
                        self.last_cpu_pc, value
                    );
                }
                self.dma_controller.write(addr, value);
                if value != 0 {
                    self.mdmaen_nonzero_count = self.mdmaen_nonzero_count.saturating_add(1);
                }

                // Debug/test mode: 強制的に即時MDMAを実行（タイミングゲート無視）
                // STRICT_PPU_TIMING などで defer されて実行されない疑いがある場合に使う。
                if std::env::var_os("FORCE_MDMA_NOW").is_some() && value != 0 {
                    println!("[FORCE_MDMA_NOW] value=0x{:02X}", value);
                    for i in 0..8 {
                        if value & (1 << i) != 0 {
                            self.perform_dma_transfer(i as usize);
                        }
                    }
                    return;
                }

                let strict = crate::debug_flags::strict_ppu_timing();
                let in_vblank = self.ppu.is_vblank();
                let mut now_mask = value;
                let mut defer_mask = 0u8;
                if strict && !in_vblank {
                    // Defer graphics-related channels to next VBlank
                    for i in 0..8u8 {
                        if value & (1 << i) == 0 {
                            continue;
                        }
                        let ch = &self.dma_controller.channels[i as usize];
                        let dest = ch.dest_address & 0x7F;
                        let is_vram = dest == 0x18 || dest == 0x19;
                        let is_cgram = dest == 0x22;
                        let is_oam = dest == 0x04;
                        // Allow VRAM MDMA in HBlank; CGRAM/OAMはVBlankのみ
                        if is_vram {
                            // Only allow during HBlank safe sub-window (query PPU for exact gate)
                            let safe = self.ppu.is_hblank_safe_for_vram_mdma();
                            if !safe {
                                defer_mask |= 1 << i;
                                now_mask &= !(1 << i);
                            }
                        } else if is_cgram || is_oam {
                            defer_mask |= 1 << i;
                            now_mask &= !(1 << i);
                        }
                    }
                    if defer_mask != 0 {
                        self.pending_gdma_mask |= defer_mask;
                    }
                }
                // Enhanced DMA monitoring for graphics transfers (throttled unless verbose)
                if value != 0 {
                    static mut GRAPHICS_DMA_COUNT: u32 = 0;
                    unsafe {
                        GRAPHICS_DMA_COUNT = GRAPHICS_DMA_COUNT.saturating_add(1);
                    }
                    let verbose = crate::debug_flags::graphics_dma_verbose();
                    let quiet = crate::debug_flags::quiet();
                    let under_cap = unsafe { GRAPHICS_DMA_COUNT } <= 64;

                    if (verbose && !quiet) || (!quiet && under_cap) {
                        println!(
                            "🎮 GRAPHICS DMA[{}]: MDMAEN set: 0x{:02X}",
                            unsafe { GRAPHICS_DMA_COUNT },
                            value
                        );
                        for i in 0..8 {
                            if value & (1 << i) != 0 {
                                let ch = &self.dma_controller.channels[i];
                                let dest_masked = ch.dest_address; // keep high bit (0x80..) to distinguish WRAM port
                                let dir_cpu_to_ppu = (ch.control & 0x80) == 0;
                                let control = ch.control;
                                let src_addr = ch.src_address;
                                let size_reg = ch.size;
                                let unit = ch.get_transfer_unit();
                                let incmode = ch.get_address_mode();

                                // Detect graphics-related transfers
                                let is_vram = dest_masked == 0x18 || dest_masked == 0x19;
                                let is_cgram = dest_masked == 0x22;
                                let is_oam = dest_masked == 0x04;
                                let is_wram = (0x80..=0x83).contains(&dest_masked);

                                let transfer_type = if is_vram {
                                    "VRAM"
                                } else if is_cgram {
                                    "CGRAM"
                                } else if is_oam {
                                    "OAM"
                                } else if is_wram {
                                    "WRAM"
                                } else {
                                    "OTHER"
                                };

                                // Snapshot first bytes of source for early graphics DMAs (helps detect zero-filled buffers)
                                if verbose && (is_vram || is_cgram || is_oam) && dir_cpu_to_ppu {
                                    use std::sync::atomic::{AtomicU32, Ordering};
                                    static SNAP_COUNT: AtomicU32 = AtomicU32::new(0);
                                    if SNAP_COUNT.fetch_add(1, Ordering::Relaxed) < 8 {
                                        let mut buf = [0u8; 16];
                                        for j in 0..16 {
                                            buf[j] = self.read_u8(src_addr.wrapping_add(j as u32));
                                        }
                                        let nonzero = buf.iter().any(|&b| b != 0);
                                        println!(
                                            "    SRC[0..16] = {:02X?} (nonzero={})",
                                            &buf[..],
                                            nonzero
                                        );
                                    }
                                }
                                println!(
                                    "  📊 CH{} [{}] ctrl=0x{:02X} {} dest=$21{:02X} src=0x{:06X} size={} unit={} incmode={}",
                                    i,
                                    transfer_type,
                                    control,
                                    if dir_cpu_to_ppu { "CPU->PPU" } else { "PPU->CPU" },
                                    dest_masked,
                                    src_addr,
                                    size_reg,
                                    unit,
                                    incmode
                                );
                                if verbose && crate::debug_flags::cgram_dma() && is_cgram {
                                    println!(
                                        "➡️  CGRAM DMA start: ch{} size={} src=0x{:06X} (unit={} addr_mode={})",
                                        i, size_reg, src_addr, unit, incmode
                                    );
                                }
                            }
                        }
                    }
                }
                // MDMAEN starts after the *next opcode fetch* (SNESdev timing note).
                // So here we only queue the channels; the actual transfer happens in
                // `CpuBus::opcode_memory_penalty()` for the S-CPU bus.
                for i in 0..8 {
                    if (now_mask & (1 << i)) != 0 && !self.dma_controller.channels[i].configured {
                        now_mask &= !(1 << i);
                    }
                }
                self.pending_mdma_mask |= now_mask;
            }
            0x420C => {
                // HDMAEN - H-blank DMA Enable
                self.dma_controller.write(addr, value);
                if value != 0 {
                    self.hdmaen_nonzero_count = self.hdmaen_nonzero_count.saturating_add(1);
                }
            }
            0x420D => {
                // MEMSEL - Memory Speed Control
                // bit0: 1=FastROM, 0=SlowROM. We store the bit for future timing use.
                self.fastrom = (value & 0x01) != 0;
            }
            _ => {
                // Other IO registers - stub for now
            }
        }
    }

    // --- Save-state helpers (WRAM/SRAM and simple IO) ---
    pub fn snapshot_memory(&self) -> (Vec<u8>, Vec<u8>) {
        (self.wram.clone(), self.sram.clone())
    }

    pub fn restore_memory(&mut self, wram: &[u8], sram: &[u8]) {
        if self.wram.len() == wram.len() {
            self.wram.copy_from_slice(wram);
        }
        if self.sram.len() == sram.len() {
            self.sram.copy_from_slice(sram);
            self.sram_dirty = false;
        }
    }

    // --- SRAM access/persistence helpers ---
    pub fn sram(&self) -> &[u8] {
        &self.sram
    }
    pub fn sram_mut(&mut self) -> &mut [u8] {
        &mut self.sram
    }
    pub fn sram_size(&self) -> usize {
        self.sram_size
    }
    pub fn is_sram_dirty(&self) -> bool {
        self.sram_dirty
    }
    pub fn clear_sram_dirty(&mut self) {
        self.sram_dirty = false;
    }

    pub fn get_input_system(&self) -> &crate::input::InputSystem {
        &self.input_system
    }

    #[allow(dead_code)]
    fn read_expansion(&mut self, _addr: u32) -> u8 {
        0xFF
    }

    #[allow(dead_code)]
    fn write_expansion(&mut self, _addr: u32, _value: u8) {}

    pub fn get_ppu(&self) -> &crate::ppu::Ppu {
        &self.ppu
    }

    pub fn get_ppu_mut(&mut self) -> &mut crate::ppu::Ppu {
        &mut self.ppu
    }

    /// 現在のNMITIMEN値（$4200）を取得（デバッグ/フォールバック用）
    #[inline]
    pub fn nmitimen(&self) -> u8 {
        self.nmitimen
    }

    // Debug accessor for JOYBUSY counter (auto-joypad in progress)
    pub fn joy_busy_counter(&self) -> u8 {
        self.joy_busy_counter
    }

    /// CPUテストROM用の自動入力を有効化する
    pub fn enable_cpu_test_mode(&mut self) {
        self.cpu_test_mode = true;
        self.cpu_test_auto_frames = std::env::var("CPU_TEST_AUTO_FRAMES")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(120);
        self.cpu_test_auto_joy_phase = 0;
        self.cpu_test_result = None;
    }

    #[inline]
    pub fn is_cpu_test_mode(&self) -> bool {
        self.cpu_test_mode
    }

    pub fn take_cpu_test_result(&mut self) -> Option<CpuTestResult> {
        self.cpu_test_result.take()
    }

    #[inline]
    pub fn is_fake_apu(&self) -> bool {
        self.fake_apu
    }

    // --- ROM mapping helpers (approximate) ---
    pub fn is_rom_address(&self, addr: u32) -> bool {
        let bank = ((addr >> 16) & 0xFF) as u8;
        let off = (addr & 0xFFFF) as u16;
        match self.mapper_type {
            crate::cartridge::MapperType::LoRom | crate::cartridge::MapperType::DragonQuest3 => {
                match bank {
                    0x00..=0x3F | 0x80..=0xBF => off >= 0x8000,
                    0x40..=0x7D | 0xC0..=0xFF => off >= 0x8000, // LoROM mirrors
                    _ => false,
                }
            }
            crate::cartridge::MapperType::HiRom | crate::cartridge::MapperType::ExHiRom => {
                match bank {
                    0x40..=0x7D | 0xC0..=0xFF => true,
                    0x00..=0x3F | 0x80..=0xBF => off >= 0x8000,
                    _ => false,
                }
            }
            _ => false,
        }
    }

    #[inline]
    pub fn is_fastrom(&self) -> bool {
        self.fastrom
    }

    pub fn get_apu_shared(&self) -> Arc<Mutex<crate::apu::Apu>> {
        self.apu.clone()
    }

    pub fn set_mapper_type(&mut self, mapper: crate::cartridge::MapperType) {
        self.mapper_type = mapper;
    }

    pub fn get_mapper_type(&self) -> crate::cartridge::MapperType {
        self.mapper_type
    }

    pub fn get_input_system_mut(&mut self) -> &mut crate::input::InputSystem {
        &mut self.input_system
    }

    // Headless init counters (for concise summary)
    pub fn get_init_counters(&self) -> (u32, u32, u32, u32) {
        (
            self.nmitimen_writes_count,
            self.mdmaen_nonzero_count,
            self.hdmaen_nonzero_count,
            self.dma_reg_writes,
        )
    }

    // Short DMA config summary for INIT logs
    pub fn get_dma_config_summary(&self) -> String {
        let mut parts = Vec::new();
        for (i, ch) in self.dma_controller.channels.iter().enumerate() {
            let mut flags = String::new();
            if ch.cfg_ctrl {
                flags.push('C');
            }
            if ch.cfg_dest {
                flags.push('D');
            }
            if ch.cfg_src {
                flags.push('S');
            }
            if ch.cfg_size {
                flags.push('Z');
            }
            if !flags.is_empty() {
                parts.push(format!("ch{}:{}", i, flags));
            }
        }
        if parts.is_empty() {
            "DMAcfg:none".to_string()
        } else {
            format!("DMAcfg:{}", parts.join(","))
        }
    }

    pub fn irq_is_pending(&mut self) -> bool {
        if self.irq_pending {
            return true;
        }
        // SA-1 -> S-CPU IRQ (via CIE mask)
        if self.is_sa1_active() {
            // S-CPU can only see SA-1 IRQ when SIE permits it.
            if self.sa1.scpu_irq_asserted() {
                return true;
            }
        }
        false
    }

    pub fn clear_irq_pending(&mut self) {
        self.irq_pending = false;
    }

    /// Tick CPU-cycle based peripherals (currently: hardware math).
    /// Call once per executed S-CPU instruction slice with the number of cycles consumed.
    pub fn tick_cpu_cycles(&mut self, cpu_cycles: u8) {
        if cpu_cycles == 0 {
            return;
        }

        // Fast path: nothing in flight.
        if !self.mul_busy && !self.div_busy {
            return;
        }

        for _ in 0..cpu_cycles {
            if self.mul_busy {
                // Defer by 1 CPU cycle so we don't advance within the same cycle as the
                // start write (WRMPYB). This matches common documentation and is enough
                // to satisfy in-flight test ROMs.
                if self.mul_just_started {
                    self.mul_just_started = false;
                    continue;
                }
                if self.mul_cycles_left == 0 {
                    self.mul_busy = false;
                    continue;
                }
                if (self.mul_work_b & 1) != 0 {
                    self.mul_partial = self.mul_partial.wrapping_add(self.mul_work_a);
                }
                self.mul_work_b >>= 1;
                self.mul_work_a = self.mul_work_a.wrapping_shl(1);
                self.mul_cycles_left = self.mul_cycles_left.saturating_sub(1);
                self.mul_result = self.mul_partial;
                if self.mul_cycles_left == 0 {
                    self.mul_busy = false;
                }
                continue;
            }

            if self.div_busy {
                // Defer by 1 CPU cycle so we don't advance within the same cycle as the
                // start write (WRDIVB).
                if self.div_just_started {
                    self.div_just_started = false;
                    continue;
                }
                if self.div_cycles_left == 0 {
                    self.div_busy = false;
                    continue;
                }
                let divisor = self.div_work_divisor as u16;
                if divisor == 0 {
                    // Shouldn't happen (handled on start), but keep behavior safe.
                    self.div_quot = 0xFFFF;
                    self.div_rem = self.div_work_dividend;
                    self.mul_result = self.div_rem;
                    self.div_busy = false;
                    continue;
                }

                let bit = self.div_work_bit;
                if bit < 0 {
                    // Completed.
                    self.div_quot = self.div_work_quot;
                    self.div_rem = self.div_work_rem;
                    self.mul_result = self.div_rem;
                    self.div_busy = false;
                    continue;
                }

                let next = (self.div_work_dividend >> (bit as u16)) & 1;
                self.div_work_rem = (self.div_work_rem << 1) | next;
                if self.div_work_rem >= divisor {
                    self.div_work_rem = self.div_work_rem.wrapping_sub(divisor);
                    self.div_work_quot |= 1u16 << (bit as u16);
                }
                self.div_work_bit = self.div_work_bit.saturating_sub(1);
                self.div_cycles_left = self.div_cycles_left.saturating_sub(1);

                // Expose intermediate state through result registers.
                self.div_quot = self.div_work_quot;
                self.div_rem = self.div_work_rem;
                self.mul_result = self.div_work_rem;

                if self.div_cycles_left == 0 {
                    self.div_busy = false;
                }
            }
        }
    }

    // Called by emulator each time scanline advances; minimal V-timer IRQ
    pub fn tick_timers(&mut self) {
        // Called at scanline boundary (good moment to check V compare)
        if !(self.irq_h_enabled || self.irq_v_enabled) {
            return;
        }
        let line = self.ppu.get_scanline();
        let v_match = self.v_timer_set && (line == self.v_timer);
        if self.irq_v_enabled && !self.irq_h_enabled {
            if v_match {
                self.irq_pending = true;
            }
        } else if self.irq_h_enabled && self.irq_v_enabled {
            // When both enabled, remember V matched line; H will be checked in tick_timers_hv
            self.irq_v_matched_line = if v_match { Some(line) } else { None };
        } else {
            // Only H enabled: do nothing here; handled in tick_timers_hv
        }
    }

    // Called after PPU step with old/new cycle to approximate H/V timer match
    pub fn tick_timers_hv(&mut self, old_cycle: u16, new_cycle: u16, scanline: u16) {
        if !(self.irq_h_enabled || self.irq_v_enabled) {
            return;
        }

        let v_match = self.v_timer_set && (scanline == self.v_timer);
        let mut h_match = false;
        if self.h_timer_set {
            // Hardware note (coarse): H-IRQ triggers slightly after the programmed dot.
            // Many docs describe ~HTIME+3.5 dots; approximate with +4 here.
            let h = self.h_timer.wrapping_add(4);
            // Detect crossing of the H timer threshold within this PPU step
            if old_cycle <= new_cycle {
                // same scanline
                if old_cycle <= h && h < new_cycle {
                    h_match = true;
                }
            } else {
                // scanline advanced and dot counter wrapped
                if old_cycle <= h || h < new_cycle {
                    h_match = true;
                }
            }
        }

        match (self.irq_h_enabled, self.irq_v_enabled) {
            (true, true) => {
                // Require both V matched on this line and H crossing
                if h_match {
                    if let Some(vline) = self.irq_v_matched_line {
                        if vline == scanline {
                            self.irq_pending = true;
                        }
                    }
                }
            }
            (true, false) => {
                if h_match {
                    self.irq_pending = true;
                }
            }
            (false, true) => {
                if v_match {
                    self.irq_pending = true;
                }
            }
            _ => {}
        }
    }

    // Called when VBlank starts (scanline 224). Handles auto-joy if enabled.
    pub fn on_vblank_start(&mut self) {
        if (self.nmitimen & 0x01) != 0 {
            // Auto-joypad read begins with a latch pulse (equivalent to writing 1->0 to $4016).
            // This also prepares the manual serial read registers ($4016/$4017) for ROMs that
            // read them after enabling auto-joypad without explicitly strobbing.
            self.input_system.write_strobe(1);
            self.input_system.write_strobe(0);

            // Auto-joypad: emulate the hardware serial read (16 bits per pad).
            // SNES stores bits as "1=Low=Pressed" with first serial bit (B) as MSB.
            let mt = self.input_system.is_multitap_enabled();
            let mut b1: u16 = 0;
            let mut b2: u16 = 0;
            let mut b3: u16 = 0;
            let mut b4: u16 = 0;
            for _ in 0..16 {
                b1 = (b1 << 1) | ((self.input_system.read_controller1() & 1) as u16);
                b2 = (b2 << 1) | ((self.input_system.read_controller2() & 1) as u16);
                if mt {
                    b3 = (b3 << 1) | ((self.input_system.read_controller3() & 1) as u16);
                    b4 = (b4 << 1) | ((self.input_system.read_controller4() & 1) as u16);
                }
            }

            self.joy_data[0] = (b1 & 0x00FF) as u8;
            self.joy_data[1] = ((b1 >> 8) & 0x00FF) as u8;
            self.joy_data[2] = (b2 & 0x00FF) as u8;
            self.joy_data[3] = ((b2 >> 8) & 0x00FF) as u8;
            self.joy_data[4] = (b3 & 0x00FF) as u8;
            self.joy_data[5] = ((b3 >> 8) & 0x00FF) as u8;
            self.joy_data[6] = (b4 & 0x00FF) as u8;
            self.joy_data[7] = ((b4 >> 8) & 0x00FF) as u8;
            // CPUテストROM専用（ヘッドレスのみ）:
            // ラッチ値を「未押下」に固定し、$4218 の2回目読みでA押下を返す。
            // ウィンドウ表示時はユーザー入力を優先する。
            if self.cpu_test_mode && crate::debug_flags::headless() {
                self.joy_data[0] = 0x00;
                self.joy_data[1] = 0x00;
            }
            // Set JOYBUSY for a short duration (approximation)
            // CPUテストHLE_FORCE中は BUSY=0（常に完了扱い）、そうでなければ長めに保持
            self.joy_busy_counter = if crate::debug_flags::cpu_test_hle_force() {
                0
            } else if crate::debug_flags::cpu_test_hle() {
                32
            } else {
                self.joy_busy_scanlines
            };
            if std::env::var_os("TRACE_AUTOJOY").is_some() {
                println!(
                    "[AUTOJOY] latched b1=0x{:04X} b2=0x{:04X} busy={} scanline={} cycle={}",
                    b1,
                    b2,
                    self.joy_busy_counter,
                    self.ppu.scanline,
                    self.ppu.get_cycle()
                );
            }
        }
        // Strict timing: run deferred graphics DMA now
        if self.pending_gdma_mask != 0 {
            let mask = self.pending_gdma_mask;
            self.pending_gdma_mask = 0;
            for i in 0..8 {
                if mask & (1 << i) != 0 {
                    if !self.dma_controller.channels[i].configured {
                        continue;
                    }
                    self.perform_dma_transfer(i);
                }
            }
        }
    }

    // Called once per scanline to update JOYBUSY timing
    pub fn on_scanline_advance(&mut self) {
        if self.joy_busy_counter > 0 {
            self.joy_busy_counter -= 1;
        }
    }

    pub fn hdma_scanline(&mut self) {
        // HDMAチャンネルのスキャンライン処理を実行
        for i in 0..8 {
            if !self.dma_controller.channels[i].hdma_enabled
                || self.dma_controller.channels[i].hdma_terminated
            {
                continue;
            }

            // 行カウンタが0なら新しいエントリをロード
            if self.dma_controller.channels[i].hdma_line_counter == 0 && !self.load_hdma_entry(i) {
                self.dma_controller.channels[i].hdma_terminated = true;
                continue;
            }

            // HDMA転送実行
            self.perform_hdma_transfer(i);

            // 行カウンタをデクリメント
            let new_count = self.dma_controller.channels[i]
                .hdma_line_counter
                .saturating_sub(1);
            self.dma_controller.channels[i].hdma_line_counter = new_count;
        }
    }

    // H-Blank開始タイミングで呼ばれる想定のHDMA処理
    pub fn hdma_hblank(&mut self) {
        // 実機はH-Blankの頭でHDMAを行う。ここではhdma_scanlineと同等処理を呼ぶ。
        self.hdma_scanline();
        self.hdma_lines_executed = self.hdma_lines_executed.saturating_add(1);
    }

    fn load_hdma_entry(&mut self, channel: usize) -> bool {
        // 参照の衝突を避けるため、必要値を先に取り出す
        let table_addr = { self.dma_controller.channels[channel].hdma_table_addr };
        let control = { self.dma_controller.channels[channel].control };

        let line_info = self.read_u8(table_addr);
        if line_info == 0 {
            return false;
        }

        let repeat_flag = (line_info & 0x80) != 0;
        let line_count = line_info & 0x7F;
        let unit = control & 0x07;
        let data_len = Self::hdma_transfer_len(unit) as u32;
        let indirect = (control & 0x40) != 0; // bit6: indirect addressing

        // まず基本フィールドを更新
        {
            let ch = &mut self.dma_controller.channels[channel];
            ch.hdma_line_counter = line_count;
            ch.hdma_repeat_flag = repeat_flag;
            ch.hdma_table_addr = Bus::add16_in_bank(table_addr, 1); // ヘッダ分（16bit）進める
            ch.hdma_latched = [0; 4];
            ch.hdma_latched_len = 0;
            ch.hdma_indirect = indirect;
        }

        // 間接アドレッシング：2バイトのアドレスを読み込み（バンクは初期のsrc.bank）
        if indirect {
            let lo = self.read_u8(self.dma_controller.channels[channel].hdma_table_addr) as u32;
            let hi = self.read_u8(Bus::add16_in_bank(
                self.dma_controller.channels[channel].hdma_table_addr,
                1,
            )) as u32;
            let bank = (self.dma_controller.channels[channel].src_address >> 16) & 0xFF;
            {
                let ch = &mut self.dma_controller.channels[channel];
                ch.hdma_indirect_addr = (bank << 16) | (hi << 8) | lo;
                ch.hdma_table_addr = Bus::add16_in_bank(ch.hdma_table_addr, 2);
            }
        } else if repeat_flag {
            // 直接モード＋リピート: データを一度だけ読み込んでラッチ
            let start = Bus::add16_in_bank(table_addr, 1);
            let mut buf = [0u8; 4];
            for i in 0..data_len {
                buf[i as usize] = self.read_u8(Bus::add16_in_bank(start, i));
            }
            {
                let ch = &mut self.dma_controller.channels[channel];
                ch.hdma_latched[..data_len as usize].copy_from_slice(&buf[..data_len as usize]);
                ch.hdma_latched_len = data_len as u8;
                ch.hdma_table_addr = Bus::add16_in_bank(start, data_len);
            }
        } else {
            // 直接モード＋ノンリピート: テーブルポインタをデータ長分進めるだけ
            let start = Bus::add16_in_bank(table_addr, 1);
            let ch = &mut self.dma_controller.channels[channel];
            ch.hdma_table_addr = Bus::add16_in_bank(start, data_len);
        }

        true
    }

    fn perform_hdma_transfer(&mut self, channel: usize) {
        // Mark write context so PPU can allow HDMA during HBlank appropriately
        self.ppu.begin_hdma_context();
        // 必要な情報を事前に取得して、借用を短く保つ
        let dest_base = { self.dma_controller.channels[channel].dest_address };
        let control = { self.dma_controller.channels[channel].control };
        let repeat_flag = { self.dma_controller.channels[channel].hdma_repeat_flag };
        let latched_len = { self.dma_controller.channels[channel].hdma_latched_len } as usize;
        let unit = control & 0x07;
        let len = Self::hdma_transfer_len(unit) as usize;
        let indirect = (control & 0x40) != 0;

        let use_latched = repeat_flag && latched_len == len;
        let table_addr_snapshot = { self.dma_controller.channels[channel].hdma_table_addr };

        // 1ライン分のデータを用意
        let mut bytes: [u8; 4] = [0; 4];
        if use_latched {
            let latched = { self.dma_controller.channels[channel].hdma_latched };
            bytes[..len].copy_from_slice(&latched[..len]);
        } else if indirect {
            // 間接アドレッシング：間接アドレスから読み出す
            let start = { self.dma_controller.channels[channel].hdma_indirect_addr };
            for (i, slot) in bytes.iter_mut().enumerate().take(len) {
                *slot = self.read_u8(Bus::add16_in_bank(start, i as u32));
            }
            // HDMAでは間接アドレスは毎ライン len 分前進（リピート有無に関わらず）
            let ch = &mut self.dma_controller.channels[channel];
            ch.hdma_indirect_addr = Bus::add16_in_bank(start, len as u32);
        } else {
            for (i, slot) in bytes.iter_mut().enumerate().take(len) {
                *slot = self.read_u8(Bus::add16_in_bank(table_addr_snapshot, i as u32));
            }
            // テーブルアドレスを進める
            {
                let ch = &mut self.dma_controller.channels[channel];
                ch.hdma_table_addr = Bus::add16_in_bank(table_addr_snapshot, len as u32);
            }
        }

        // 書き込み（PPU writable or APU I/O）
        for (i, data) in bytes.iter().enumerate().take(len) {
            let dest_off = Self::hdma_dest_offset(unit, dest_base, i as u8);
            let dest_addr = 0x2100u32 + dest_off as u32;
            if dest_off <= 0x33 || (0x40..=0x43).contains(&dest_off) {
                self.write_u8(dest_addr, *data);
                // Aggregate per-port stats for concise logs
                match dest_off {
                    0x15..=0x19 => {
                        // VRAM path (incl. VMAIN/VMADD*)
                        self.hdma_bytes_vram = self.hdma_bytes_vram.saturating_add(1);
                    }
                    0x21 | 0x22 => {
                        // CGRAM path
                        self.hdma_bytes_cgram = self.hdma_bytes_cgram.saturating_add(1);
                    }
                    0x04 => {
                        // OAMDATA
                        self.hdma_bytes_oam = self.hdma_bytes_oam.saturating_add(1);
                    }
                    _ => {}
                }
            }
        }
        self.ppu.end_hdma_context();
    }

    #[inline]
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
    fn hdma_dest_offset(unit: u8, base: u8, index: u8) -> u8 {
        let i = index;
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

    // 通常のDMA転送処理
    fn perform_dma_transfer(&mut self, channel: usize) {
        // General DMA: mark MDMA during this burst
        self.ppu.set_debug_dma_channel(Some(channel as u8));
        self.ppu.begin_mdma_context();
        if std::env::var_os("DMA_PROBE").is_some() {
            let chp = &self.dma_controller.channels[channel];
            println!(
                "[DMA_PROBE] ch{} ctrl=0x{:02X} dest=$21{:02X} size=0x{:04X} src=0x{:06X}",
                channel, chp.control, chp.dest_address, chp.size, chp.src_address
            );
        }
        let ch = &self.dma_controller.channels[channel];
        // Skip obviously unconfigured junk (only skip if completely unconfigured)
        if !ch.configured {
            static mut DMA_SKIP_CFG_LOGGED: [bool; 8] = [false; 8];
            unsafe {
                if debug_flags::dma() && !DMA_SKIP_CFG_LOGGED[channel] {
                    println!(
                        "DMA skipped: CH{} not configured (ctrl=0x{:02X}, size={})",
                        channel, ch.control, ch.size
                    );
                    DMA_SKIP_CFG_LOGGED[channel] = true;
                }
            }
            return;
        }
        // 転送方向を取得
        let cpu_to_ppu = (ch.control & 0x80) == 0;

        let mut transfer_size = ch.size as u32;
        if transfer_size == 0 {
            // size未設定（0）をどう扱うか: デフォルトは実機同様65536、フラグで0扱いにできる
            if debug_flags::dma_zero_is_zero() {
                if debug_flags::dma() {
                    println!(
                        "DMA size=0 treated as zero (env DMA_ZERO_IS_ZERO=1) ch{} ctrl=0x{:02X} dest=$21{:02X}",
                        channel, ch.control, ch.dest_address
                    );
                }
                self.ppu.end_mdma_context();
                self.ppu.set_debug_dma_channel(None);
                return;
            }

            if !ch.cfg_size {
                // 未設定サイズの誤爆を防ぐ（デフォルト0=65536で暴走しがち）
                if debug_flags::dma() {
                    println!(
                        "DMA skipped: CH{} size not configured (size=0, ctrl=0x{:02X}, dest=$21{:02X})",
                        channel, ch.control, ch.dest_address
                    );
                }
                self.ppu.end_mdma_context();
                self.ppu.set_debug_dma_channel(None);
                return;
            }
            // 実機仕様: size=0 は 65536バイト
            transfer_size = 0x10000;
        }
        let src_addr = ch.src_address;

        // --- burn-in-test.sfc DMA MEMORY diagnostics (opt-in) ---
        //
        // The official burn-in ROM uses DMA ch6/ch7 to roundtrip 0x1000 bytes between
        // WRAM $7E:4000 and VRAM (write via $2118/$2119, read via $2139/$213A).
        // If the DMA MEMORY test FAILs, enable TRACE_BURNIN_DMA_MEMORY=1 to print
        // a small fingerprint and detect common off-by-one/latch issues.
        let trace_burnin_dma_mem = std::env::var_os("TRACE_BURNIN_DMA_MEMORY").is_some();
        #[derive(Clone, Copy)]
        struct BurninDmaSnap {
            pc: u32,
            frame: u64,
            scanline: u16,
            cycle: u16,
            vblank: bool,
            hblank: bool,
            forced_blank: bool,
            vram_addr: u16,
            vram_inc: u16,
            vmain: u8,
            hash: u64,
            sample: [u8; 32],
        }
        static BURNIN_DMA_SNAP: OnceLock<Mutex<Option<BurninDmaSnap>>> = OnceLock::new();
        static BURNIN_DMA_DUMPED: OnceLock<AtomicU32> = OnceLock::new();
        let fnv1a64 = |data: &[u8]| -> u64 {
            let mut h: u64 = 0xcbf29ce484222325;
            for &b in data {
                h ^= b as u64;
                h = h.wrapping_mul(0x100000001b3);
            }
            h
        };

        // 特定ROM用のアドレス補正ハックは廃止（正規マッピング/CPU実装で解決する）

        // B-bus destination uses low 7 bits (0x2100-0x217F)
        let transfer_unit = ch.get_transfer_unit();
        let dest_base_full = ch.dest_address;

        // burn-in-test.sfc: track unexpected VRAM DMAs that might clobber the DMA MEMORY test region.
        // (Covers both $2118/$2119 bases and all transfer modes; we only special-case the known
        // DMA MEMORY write via ch6.)
        if trace_burnin_dma_mem && cpu_to_ppu && (dest_base_full == 0x18 || dest_base_full == 0x19)
        {
            let (vmadd_start, vram_inc, vmain) = self.ppu.dbg_vram_regs();
            if vram_inc == 1 {
                let words = (transfer_size / 2) as u16;
                let vmadd_end = vmadd_start.wrapping_add(words);
                let overlaps = vmadd_start < 0x5800 && vmadd_end > 0x5000;
                let is_known_dmamem_write =
                    channel == 6 && src_addr == 0x7E4000 && transfer_size == 0x1000;
                if overlaps && !is_known_dmamem_write {
                    println!(
                        "[BURNIN-DMAMEM] UNEXPECTED VRAM DMA: pc={:06X} ch{} src=0x{:06X} size=0x{:04X} base=$21{:02X} unit={} addr_mode={} VMADD={}..{} VMAIN={:02X}",
                        self.last_cpu_pc,
                        channel,
                        src_addr,
                        transfer_size,
                        dest_base_full,
                        transfer_unit,
                        ch.get_address_mode(),
                        vmadd_start,
                        vmadd_end,
                        vmain
                    );
                }
            }
        }

        // Snapshot the source buffer before it gets overwritten by the VRAM->WRAM read-back DMA.
        if trace_burnin_dma_mem
            && cpu_to_ppu
            && channel == 6
            && transfer_unit == 1
            && dest_base_full == 0x18
            && src_addr == 0x7E4000
            && transfer_size == 0x1000
            && self.wram.len() >= 0x5000
        {
            let slice = &self.wram[0x4000..0x5000];
            let mut sample = [0u8; 32];
            for (seg, off) in [0x000usize, 0x100, 0x200, 0x300].into_iter().enumerate() {
                let start = seg * 8;
                sample[start..start + 8].copy_from_slice(&slice[off..off + 8]);
            }
            let hash = fnv1a64(slice);
            let (vram_addr, vram_inc, vmain) = self.ppu.dbg_vram_regs();
            let pc = self.last_cpu_pc;
            // Arm fine-grained VRAM clobber tracing (PPU-side) after the DMA MEMORY routine starts.
            self.ppu.arm_burnin_vram_trace();
            let frame = self.ppu.get_frame();
            let scanline = self.ppu.get_scanline();
            let cycle = self.ppu.get_cycle();
            let vblank = self.ppu.is_vblank();
            let hblank = self.ppu.is_hblank();
            let forced_blank = self.ppu.is_forced_blank();
            *BURNIN_DMA_SNAP
                .get_or_init(|| Mutex::new(None))
                .lock()
                .unwrap() = Some(BurninDmaSnap {
                pc,
                frame,
                scanline,
                cycle,
                vblank,
                hblank,
                forced_blank,
                vram_addr,
                vram_inc,
                vmain,
                hash,
                sample,
            });
            println!(
                "[BURNIN-DMAMEM] SNAP pc={:06X} frame={} sl={} cyc={} vblank={} hblank={} fblank={} VMADD={:04X} VMAIN={:02X} inc={} hash={:016X} sample@0/100/200/300={:02X?}",
                pc,
                frame,
                scanline,
                cycle,
                vblank as u8,
                hblank as u8,
                forced_blank as u8,
                vram_addr,
                vmain,
                vram_inc,
                hash,
                sample
            );
        }

        if dest_base_full == 0 {
            static INIDISP_DMA_ALERT: OnceLock<AtomicU32> = OnceLock::new();
            let n = INIDISP_DMA_ALERT
                .get_or_init(|| AtomicU32::new(0))
                .fetch_add(1, Ordering::Relaxed);
            if n < 4 {
                println!(
                    "[DEBUG-INIDISP-DMA] ch{} ctrl=0x{:02X} src=0x{:06X} size={} unit={} addr_mode={} (dest_base=0) mdmaen=0x{:02X}",
                    channel,
                    ch.control,
                    src_addr,
                    transfer_size,
                    transfer_unit,
                    ch.get_address_mode(),
                    self.dma_controller.dma_enable
                );
            }
        }
        if std::env::var_os("TRACE_DMA_DEST").is_some() {
            println!(
                "[DMA-DEST] ch{} ctrl=0x{:02X} dest_base=$21{:02X} size={} unit={} addr_mode={}",
                channel,
                ch.control,
                dest_base_full,
                transfer_size,
                transfer_unit,
                ch.get_address_mode()
            );
        }

        // DMA転送のデバッグ（許可時のみ）

        // Early sanity check: skip obviously invalid B-bus target ranges to reduce noise
        // CPU->PPU: allow $2100-$2133 and $2140-$2143 only
        // PPU->CPU: allow $2134-$213F and $2140-$2143 only
        let allowed = if cpu_to_ppu {
            (dest_base_full <= 0x33)
                || (0x40..=0x43).contains(&dest_base_full)
                || (0x80..=0x83).contains(&dest_base_full) // WRAM port
        } else {
            (0x34..=0x3F).contains(&dest_base_full)
                || (0x40..=0x43).contains(&dest_base_full)
                || (0x80..=0x83).contains(&dest_base_full)
        };
        if !allowed {
            static DMA_BBUS_WARN: OnceLock<AtomicU32> = OnceLock::new();
            {
                let ctr = DMA_BBUS_WARN.get_or_init(|| AtomicU32::new(0));
                if debug_flags::dma() && ctr.load(Ordering::Relaxed) < 8 {
                    ctr.fetch_add(1, Ordering::Relaxed);
                    println!(
                        "DMA skipped: CH{} {} to invalid B-bus $21{:02X} (size={})",
                        channel,
                        if cpu_to_ppu { "CPU->PPU" } else { "PPU->CPU" },
                        dest_base_full,
                        transfer_size
                    );
                }
            }
            return;
        }
        // Guard against obviously bogus INIDISP floods (e.g., uninitialized channels)
        // Note: This early-return used to drop large MDMA transfers targeting $2100.
        // However, some titles briefly program DMA with size 0 (=> 65536) before
        // immediately updating the registers. Skipping here could eat real transfers
        // when dest decoding goes wrong, leaving the screen black.  Allow them to run;
        // the regular PPU register handling will clamp brightness safely.
        // if cpu_to_ppu && dest_base_full == 0x00 && transfer_size > 0x0100 {
        //     static SKIP_INIDISP_DMA: OnceLock<AtomicU32> = OnceLock::new();
        //     let n = SKIP_INIDISP_DMA
        //         .get_or_init(|| AtomicU32::new(0))
        //         .fetch_add(1, Ordering::Relaxed);
        //     if n < 4 {
        //         println!(
        //             "⚠️  Skipping suspicious INIDISP DMA: ch{} size={} src=0x{:06X} mdmaen=0x{:02X}",
        //             channel, transfer_size, src_addr, self.dma_controller.dma_enable
        //         );
        //     }
        //     self.ppu.end_mdma_context();
        //     self.ppu.set_debug_dma_channel(None);
        //     return;
        // }

        // ここまで到達したものだけを転送ログ対象にする
        if debug_flags::dma() {
            static DMA_COUNT: OnceLock<AtomicU32> = OnceLock::new();
            let n = DMA_COUNT
                .get_or_init(|| AtomicU32::new(0))
                .fetch_add(1, Ordering::Relaxed)
                + 1;
            if n <= 10 || transfer_size > 100 {
                println!(
                    "DMA Transfer #{}: CH{} {} size={} src=0x{:06X} dest=$21{:02X}",
                    n,
                    channel,
                    if cpu_to_ppu { "CPU->PPU" } else { "PPU->CPU" },
                    transfer_size,
                    src_addr,
                    dest_base_full
                );
            }
        }

        // Special log for CGRAM transfers (debug-only)
        if debug_flags::cgram_dma() && !debug_flags::quiet() && dest_base_full == 0x22 && cpu_to_ppu
        {
            static CGRAM_DMA_COUNT: OnceLock<AtomicU32> = OnceLock::new();
            let n = CGRAM_DMA_COUNT
                .get_or_init(|| AtomicU32::new(0))
                .fetch_add(1, Ordering::Relaxed)
                + 1;
            if n <= 20 {
                println!(
                    "🎨 CGRAM DMA #{}: CH{} size={} src=0x{:06X} -> $2122 (CGDATA)",
                    n, channel, transfer_size, src_addr
                );
            }
        }

        if transfer_size == 0 {
            return; // 転送サイズが0なら何もしない
        }

        // NOTE: PPU->CPU DMA from $2134 (Mode7 product) is commonly used as a fast
        // "memset" trick (fill WRAM with a constant). Do NOT clamp its size here.

        // 実際の転送を実行
        if std::env::var_os("TRACE_WRAM_STACK_DMA").is_some()
            && cpu_to_ppu
            && (dest_base_full >= 0x80 && dest_base_full <= 0x83)
        {
            println!(
                "[WRAM-DMA-START] ch{} start_wram_addr=0x{:05X} size=0x{:04X} src=0x{:06X}",
                channel, self.wram_address, transfer_size, src_addr
            );
        }

        // burn-in-test.sfc DMA MEMORY: capture the destination WRAM buffer before VRAM->WRAM DMA overwrites it.
        let mut burnin_pre_wram_hash: Option<u64> = None;
        if trace_burnin_dma_mem
            && !cpu_to_ppu
            && channel == 7
            && transfer_unit == 1
            && dest_base_full == 0x39
            && src_addr == 0x7E4000
            && transfer_size == 0x1000
            && self.wram.len() >= 0x5000
        {
            let pre = &self.wram[0x4000..0x5000];
            burnin_pre_wram_hash = Some(fnv1a64(pre));
            println!(
                "[BURNIN-DMAMEM] PREREAD-WRAM pc={:06X} hash={:016X}",
                self.last_cpu_pc,
                burnin_pre_wram_hash.unwrap()
            );
        }

        let mut cur_src = src_addr;
        let addr_mode = ch.get_address_mode(); // 0:inc, 1:fix, 2:dec, 3:inc(approx)
        let mut i = 0;

        // Debug: capture first few DMA setups to see what games configure (helps stuck WRAM fills)
        if std::env::var_os("TRACE_DMA_SETUP_ONCE").is_some() {
            use std::sync::atomic::{AtomicU32, Ordering};
            static ONCE: AtomicU32 = AtomicU32::new(0);
            let count = ONCE.fetch_add(1, Ordering::Relaxed);
            if count < 16 {
                println!(
                    "[DMA-SETUP] ch{} ctrl=0x{:02X} dest_base=$21{:02X} size={} src=0x{:06X} unit={} addr_mode={} cfgSz={} cfgDst={} cfgSrc={} cfgCtrl={}",
                    channel,
                    ch.control,
                    dest_base_full,
                    transfer_size,
                    src_addr,
                    transfer_unit,
                    addr_mode,
                    ch.cfg_size,
                    ch.cfg_dest,
                    ch.cfg_src,
                    ch.cfg_ctrl,
                );
            }
        }
        // CGRAM DMA burst summary (debug): capture first few bytes and total count
        let capture_cgram =
            crate::debug_flags::cgram_dma() && (dest_base_full == 0x22) && cpu_to_ppu;
        let mut cgram_first: [u8; 16] = [0; 16];
        let mut cgram_captured: usize = 0;
        let mut cgram_total: u32 = 0;
        // 実機準拠: 転送サイズ全体を処理（サイズ=0は65536バイト）
        while (i as u32) < transfer_size {
            if cpu_to_ppu {
                // CPU -> PPU転送（最も一般的）
                // Bバス宛先アドレスを転送モードに応じて決定
                let dest_offset = self.mdma_dest_offset(transfer_unit, dest_base_full, i as u8);

                if std::env::var_os("TRACE_DMA_DEST").is_some() && channel == 0 && i < 32 {
                    println!(
                        "[DMA-DEST-TRACE] ch{} i={} base=$21{:02X} unit={} dest_offset=$21{:02X}",
                        channel, i, dest_base_full, transfer_unit, dest_offset
                    );
                }

                let dest_full = 0x2100 + dest_offset as u32;
                self.dma_hist_note(dest_offset);

                let data = self.dma_read_a_bus(cur_src);

                // One-shot trace of early DMA bytes to understand real dests (opt-in)
                if crate::debug_flags::dma() && !crate::debug_flags::quiet() {
                    use std::sync::atomic::{AtomicU32, Ordering};
                    static BYTE_TRACE_COUNT: AtomicU32 = AtomicU32::new(0);
                    let n = BYTE_TRACE_COUNT.fetch_add(1, Ordering::Relaxed);
                    if n < 64 {
                        println!(
                            "[DMA-BYTE] ch{} i={} base=$21{:02X} offset=$21{:02X} full=$21{:04X} src=0x{:06X} data=0x{:02X}",
                            channel,
                            i,
                            dest_base_full,
                            dest_offset,
                            dest_full,
                            cur_src,
                            data
                        );
                    }
                }

                // SMW APU HLE: 2180-2183 (WRAM port) に向かうDMAを捕まえてSPC転送バッファを構築
                if self.smw_apu_hle && !self.smw_apu_hle_done && dest_base_full >= 0x80 {
                    self.smw_apu_hle_buf.push(data);
                }

                // Log INIDISP ($2100) writes during DMA to diagnose forced blank issues (opt-in)
                if dest_offset == 0x00
                    && crate::debug_flags::trace_ppu_inidisp()
                    && !crate::debug_flags::quiet()
                {
                    static INIDISP_DMA_COUNT: OnceLock<AtomicU32> = OnceLock::new();
                    let n = INIDISP_DMA_COUNT
                        .get_or_init(|| AtomicU32::new(0))
                        .fetch_add(1, Ordering::Relaxed)
                        + 1;
                    if n <= 128 {
                        println!(
                            "[INIDISP-DMA] #{}: CH{} src=0x{:06X} value=0x{:02X} (blank={} brightness={})",
                            n,
                            channel,
                            cur_src,
                            data,
                            if (data & 0x80) != 0 { "ON" } else { "OFF" },
                            data & 0x0F
                        );
                    }
                }

                if crate::debug_flags::cgram_dma() && dest_offset == 0x22 {
                    static CGDMA_BYTES: OnceLock<AtomicU32> = OnceLock::new();
                    let n = CGDMA_BYTES
                        .get_or_init(|| AtomicU32::new(0))
                        .fetch_add(1, Ordering::Relaxed)
                        + 1;
                    if n <= 16 {
                        println!(
                            "CGRAM DMA byte #{}: src=0x{:06X} data=0x{:02X}",
                            n, cur_src, data
                        );
                    }
                }
                // Debug capture for CGRAM bursts
                if capture_cgram && dest_offset == 0x22 {
                    if cgram_captured < cgram_first.len() {
                        cgram_first[cgram_captured] = data;
                        cgram_captured += 1;
                    }
                    cgram_total = cgram_total.saturating_add(1);
                }

                // PPU writable ($2100-$2133)
                if dest_offset <= 0x33 {
                    // Optional debug guard: block DMA writes to INIDISP
                    if dest_offset == 0x00 && crate::debug_flags::block_inidisp_dma() {
                        static mut INIDISP_DMA_BLOCK_LOG: u32 = 0;
                        unsafe {
                            if INIDISP_DMA_BLOCK_LOG < 8 {
                                INIDISP_DMA_BLOCK_LOG += 1;
                                println!(
                                    "⛔ BLOCK_INIDISP_DMA: ch{} data=0x{:02X} src=0x{:06X} i={} transfer_size={}",
                                    channel, data, cur_src, i, transfer_size
                                );
                            }
                        }
                        // advance addresses but skip write
                        i += 1;
                        // DMAP bit3=1 => fixed; bit4 is ignored in that case.
                        cur_src = match addr_mode {
                            0 => cur_src.wrapping_add(1), // inc
                            2 => cur_src.wrapping_sub(1), // dec
                            _ => cur_src,                 // fixed (1 or 3)
                        };
                        continue;
                    }
                    self.write_u8(dest_full, data);
                } else if (0x80..=0x83).contains(&dest_offset) {
                    // WRAM port ($2180-$2183)
                    self.write_u8(dest_full, data);
                } else if (0x40..=0x43).contains(&dest_offset) {
                    // APU I/O ($2140-$2143)
                    self.write_u8(dest_full, data);
                } else {
                    // $2134-$213F read-only or $2144-$217F undefined: ignore
                    static DMA_SKIP_DEST_LOGGED: OnceLock<Mutex<[bool; 256]>> = OnceLock::new();
                    let mut logged = DMA_SKIP_DEST_LOGGED
                        .get_or_init(|| Mutex::new([false; 256]))
                        .lock()
                        .unwrap();
                    let idx = dest_offset as usize;
                    if idx < logged.len() && debug_flags::dma() && !logged[idx] {
                        println!(
                            "DMA skipped invalid dest: CH{} base=$21{:02X} (read-only/unimplemented)",
                            channel,
                            dest_offset
                        );
                        logged[idx] = true;
                    }
                }

                // VRAMへの転送の場合は、デバッグ出力
                if debug_flags::dma() && (dest_full == 0x2118 || dest_full == 0x2119) {
                    static mut DMA_VRAM_COUNT: u32 = 0;
                    unsafe {
                        DMA_VRAM_COUNT += 1;
                        if DMA_VRAM_COUNT <= 10 {
                            println!("DMA to VRAM: src=0x{:06X}, data=0x{:02X}", cur_src, data);
                        }
                    }
                }
            } else {
                // PPU -> CPU転送（稀）
                let dest_offset = self.mdma_dest_offset(transfer_unit, dest_base_full, i as u8);
                let dest_reg = 0x2100 + dest_offset as u32;
                let data = self.read_u8(dest_reg);
                self.dma_write_a_bus(cur_src, data);
            }

            // A-busアドレスの更新（バンク固定、16bitアドレスのみ増減）
            let bank = cur_src & 0x00FF_0000;
            let lo16 = (cur_src & 0x0000_FFFF) as u16;
            let next_lo16 = match addr_mode {
                0 => lo16.wrapping_add(1), // inc
                2 => lo16.wrapping_sub(1), // dec
                _ => lo16,                 // fixed (1 or 3)
            } as u32;
            cur_src = bank | next_lo16;
            i += 1;
        }

        // --- DMA register side effects (hardware behavior) ---
        //
        // SNESdev wiki:
        // - After DMA completes, DASn becomes 0.
        // - A1Tn (low 16 bits) advances by the number of bytes transferred for increment/decrement
        //   modes; the bank (A1Bn) is fixed and wraps at the bank boundary.
        //
        // We model this by updating the channel's A-bus address (src_address) to the final cur_src
        // and clearing the transfer size register.
        {
            let ch = &mut self.dma_controller.channels[channel];
            ch.src_address = cur_src;
            ch.size = 0;
        }

        // --- Timing: S-CPU stalls during MDMA ---
        //
        // On real hardware, general DMA blocks the S-CPU while the PPU/APU continue to run.
        // We approximate the duration as:
        //   8 master cycles per transferred byte + 8 master cycles overhead.
        //
        // (This is intentionally tracked in master cycles so it can be applied without rounding.)
        let bytes_transferred = i.max(0) as u64;
        if bytes_transferred > 0 {
            let stall_master_cycles = 8u64.saturating_mul(bytes_transferred.saturating_add(1));
            self.add_pending_stall_master_cycles(stall_master_cycles);
        }

        // After WRAM->VRAM DMA completes, verify the target VRAM range matches the source buffer.
        // This helps distinguish "VRAM write blocked/corrupted" vs "VRAM read-back wrong".
        if trace_burnin_dma_mem
            && cpu_to_ppu
            && channel == 6
            && transfer_unit == 1
            && dest_base_full == 0x18
            && src_addr == 0x7E4000
            && transfer_size == 0x1000
            && self.wram.len() >= 0x5000
        {
            let src = &self.wram[0x4000..0x5000];
            let src_hash = fnv1a64(src);
            let vram = self.ppu.get_vram();
            let start = 0x5000usize.saturating_mul(2);
            let end = start.saturating_add(0x1000).min(vram.len());
            let vram_slice = &vram[start..end];
            let vram_hash = fnv1a64(vram_slice);
            println!(
                "[BURNIN-DMAMEM] POSTWRITE pc={:06X} VMADD_end={:04X} src_hash={:016X} vram_hash={:016X} match={}",
                self.last_cpu_pc,
                self.ppu.dbg_vram_regs().0,
                src_hash,
                vram_hash,
                (src_hash == vram_hash) as u8
            );
        }

        // Before VRAM->WRAM DMA begins, fingerprint the VRAM range that should be read back.
        if trace_burnin_dma_mem
            && !cpu_to_ppu
            && channel == 7
            && transfer_unit == 1
            && dest_base_full == 0x39
            && src_addr == 0x7E4000
            && transfer_size == 0x1000
        {
            let vram = self.ppu.get_vram();
            let start = 0x5000usize.saturating_mul(2);
            let end = start.saturating_add(0x1000).min(vram.len());
            let vram_slice = &vram[start..end];
            let vram_hash = fnv1a64(vram_slice);
            println!(
                "[BURNIN-DMAMEM] PREREAD pc={:06X} VMADD_start={:04X} vram_hash={:016X}",
                self.last_cpu_pc,
                self.ppu.dbg_vram_regs().0,
                vram_hash
            );
        }

        // Compare read-back buffer after VRAM->WRAM DMA completes.
        if trace_burnin_dma_mem
            && !cpu_to_ppu
            && channel == 7
            && transfer_unit == 1
            && dest_base_full == 0x39
            && src_addr == 0x7E4000
            && transfer_size == 0x1000
            && self.wram.len() >= 0x5000
        {
            let slice = &self.wram[0x4000..0x5000];
            let mut sample = [0u8; 32];
            for (seg, off) in [0x000usize, 0x100, 0x200, 0x300].into_iter().enumerate() {
                let start = seg * 8;
                sample[start..start + 8].copy_from_slice(&slice[off..off + 8]);
            }
            let hash = fnv1a64(slice);
            let (vram_addr, vram_inc, vmain) = self.ppu.dbg_vram_regs();
            let pc = self.last_cpu_pc;
            let snap = *BURNIN_DMA_SNAP
                .get_or_init(|| Mutex::new(None))
                .lock()
                .unwrap();
            if let Some(s) = snap {
                let ok = s.hash == hash;
                println!(
                    "[BURNIN-DMAMEM] READBACK pc={:06X} frame={} sl={} cyc={} vblank={} hblank={} fblank={} VMADD={:04X} VMAIN={:02X} inc={} hash={:016X} match={}",
                    pc,
                    self.ppu.get_frame(),
                    self.ppu.get_scanline(),
                    self.ppu.get_cycle(),
                    self.ppu.is_vblank() as u8,
                    self.ppu.is_hblank() as u8,
                    self.ppu.is_forced_blank() as u8,
                    vram_addr,
                    vmain,
                    vram_inc,
                    hash,
                    ok as u8
                );
                if !ok {
                    // Count and summarize differences (byte-wise) to spot shifts vs corruption.
                    let mut diff_count: u32 = 0;
                    let mut first_diff: Option<usize> = None;
                    for (i, (&a, &b)) in s.sample.iter().zip(sample.iter()).enumerate() {
                        if a != b {
                            diff_count = diff_count.saturating_add(1);
                            if first_diff.is_none() {
                                first_diff = Some(i);
                            }
                        }
                    }
                    println!(
                        "[BURNIN-DMAMEM] mismatch: src(pc={:06X} VMADD={:04X} VMAIN={:02X} inc={} hash={:016X} sample={:02X?}) rb(sample={:02X?})",
                        s.pc,
                        s.vram_addr,
                        s.vmain,
                        s.vram_inc,
                        s.hash,
                        s.sample,
                        sample
                    );
                    println!(
                        "[BURNIN-DMAMEM] mismatch detail: sample_diff_bytes={} first_diff_idx={}",
                        diff_count,
                        first_diff.map(|v| v as i32).unwrap_or(-1)
                    );
                    // One-shot dump for offline diffing.
                    let dumped = BURNIN_DMA_DUMPED
                        .get_or_init(|| AtomicU32::new(0))
                        .fetch_add(1, Ordering::Relaxed);
                    if dumped == 0 {
                        let src_wram = &self.wram[0x4000..0x5000];
                        let vram = self.ppu.get_vram();
                        let start = 0x5000usize.saturating_mul(2);
                        let end = start.saturating_add(0x1000).min(vram.len());
                        let vram_slice = &vram[start..end];
                        let _ = std::fs::create_dir_all("logs");
                        let _ = std::fs::write("logs/burnin_dmamem_src_wram.bin", src_wram);
                        let _ = std::fs::write("logs/burnin_dmamem_rb_wram.bin", slice);
                        let _ = std::fs::write("logs/burnin_dmamem_vram.bin", vram_slice);
                        println!(
                            "[BURNIN-DMAMEM] dumped logs/burnin_dmamem_src_wram.bin, logs/burnin_dmamem_rb_wram.bin, logs/burnin_dmamem_vram.bin"
                        );
                    }
                }
            } else {
                println!(
                    "[BURNIN-DMAMEM] READBACK pc={:06X} VMADD={:04X} VMAIN={:02X} inc={} hash={:016X} sample={:02X?} (no source snap)",
                    pc, vram_addr, vmain, vram_inc, hash, sample
                );
            }
        }

        if capture_cgram && cgram_total > 0 {
            let shown = cgram_captured.min(8);
            let bytes: Vec<String> = cgram_first
                .iter()
                .take(shown)
                .map(|b| format!("{:02X}", b))
                .collect();
            println!(
                "CGRAM DMA summary: ch{} total_bytes={} first[{}]=[{}]",
                channel,
                cgram_total,
                shown,
                bytes.join(", ")
            );
        }

        // SMW APU HLE: 十分なWRAM DMAデータが溜まったら一度だけSPCへロード
        if self.smw_apu_hle && !self.smw_apu_hle_done && self.smw_apu_hle_buf.len() >= 0x8400 {
            if let Ok(mut apu) = self.apu.lock() {
                apu.load_and_start(&self.smw_apu_hle_buf, 0x0400, 0x0400);
                self.smw_apu_hle_done = true;
                if std::env::var_os("TRACE_SMW_APU_HLE").is_some() {
                    println!(
                        "[SMW-APU-HLE] Loaded {} bytes from WRAM DMA into SPC, start_pc=$0400",
                        self.smw_apu_hle_buf.len()
                    );
                }
            }
        }

        self.ppu.end_mdma_context();
    }

    #[inline]
    fn mdma_dest_offset(&self, unit: u8, base: u8, index: u8) -> u8 {
        // SNESdev wiki: B-bus address is an 8-bit selector in $2100-$21FF; additions wrap at 0xFF.
        // Transfer pattern (DMAPn bits 0-2) selects the B-bus address sequence.
        let i = index as usize;
        const P0: &[u8] = &[0];
        const P1: &[u8] = &[0, 1];
        const P2: &[u8] = &[0, 0];
        const P3: &[u8] = &[0, 0, 1, 1];
        const P4: &[u8] = &[0, 1, 2, 3];
        const P5: &[u8] = &[0, 1, 0, 1]; // undocumented
        const P6: &[u8] = &[0, 0]; // undocumented (same as 2)
        const P7: &[u8] = &[0, 0, 1, 1]; // undocumented (same as 3)
        let pat = match unit & 0x07 {
            0 => P0,
            1 => P1,
            2 => P2,
            3 => P3,
            4 => P4,
            5 => P5,
            6 => P6,
            _ => P7,
        };
        base.wrapping_add(pat[i % pat.len()])
    }

    fn dma_hist_note(&mut self, dest_off: u8) {
        let idx = dest_off as usize;
        if idx < self.dma_dest_hist.len() {
            self.dma_dest_hist[idx] = self.dma_dest_hist[idx].saturating_add(1);
        }
    }

    pub fn take_dma_dest_summary(&mut self) -> String {
        let mut parts = Vec::new();
        let mut push = |name: &str, off: u8| {
            let n = self.dma_dest_hist[off as usize];
            if n > 0 {
                parts.push(format!("{}:{}", name, n));
            }
        };
        // Key PPU ports
        push("OAM", 0x04); // $2104
        push("INIDISP", 0x00); // $2100
        push("VMAIN", 0x15); // $2115
        push("VMADDL", 0x16); // $2116
        push("VMADDH", 0x17); // $2117
        push("VMDATAL", 0x18); // $2118
        push("VMDATAH", 0x19); // $2119
        push("CGADD", 0x21); // $2121
        push("CGDATA", 0x22); // $2122
        push("TM", 0x2C); // $212C
                          // WRAM port
        push("WRAM", 0x80); // $2180
                            // Any others with counts
        for (i, &n) in self.dma_dest_hist.iter().enumerate() {
            let i_u8 = i as u8;
            if n > 0
                && !matches!(
                    i_u8,
                    0x04 | 0x00 | 0x15 | 0x16 | 0x17 | 0x18 | 0x19 | 0x21 | 0x22 | 0x2C | 0x80
                )
            {
                parts.push(format!("$21{:02X}:{}", i_u8, n));
            }
        }
        // reset
        self.dma_dest_hist.fill(0);
        if parts.is_empty() {
            "DMA dests: none".to_string()
        } else {
            format!("DMA dests: {}", parts.join(", "))
        }
    }

    // Summarize HDMA activity since last call; resets counters.
    pub fn take_hdma_summary(&mut self) -> String {
        let lines = self.hdma_lines_executed;
        let vram = self.hdma_bytes_vram;
        let cgram = self.hdma_bytes_cgram;
        let oam = self.hdma_bytes_oam;
        self.hdma_lines_executed = 0;
        self.hdma_bytes_vram = 0;
        self.hdma_bytes_cgram = 0;
        self.hdma_bytes_oam = 0;
        if lines == 0 && vram == 0 && cgram == 0 && oam == 0 {
            "HDMA: none".to_string()
        } else {
            format!(
                "HDMA: lines={} VRAM={} CGRAM={} OAM={}",
                lines, vram, cgram, oam
            )
        }
    }

    /// Extra master cycles consumed by DMA/other stalls since last call.
    /// This is used by the main emulator loop to advance PPU/APU while the S-CPU is halted.
    #[inline]
    pub fn take_pending_stall_master_cycles(&mut self) -> u64 {
        let v = self.pending_stall_master_cycles;
        self.pending_stall_master_cycles = 0;
        v
    }

    #[inline]
    fn add_pending_stall_master_cycles(&mut self, cycles: u64) {
        self.pending_stall_master_cycles = self.pending_stall_master_cycles.saturating_add(cycles);
    }
}

impl CpuBus for Bus {
    fn read_u8(&mut self, addr: u32) -> u8 {
        let v = Bus::read_u8(self, addr);
        self.last_cpu_bus_addr = addr;
        self.on_cpu_bus_cycle();
        v
    }

    fn write_u8(&mut self, addr: u32, value: u8) {
        Bus::write_u8(self, addr, value);
        self.last_cpu_bus_addr = addr;
        self.on_cpu_bus_cycle();
    }

    fn begin_cpu_instruction(&mut self) {
        self.cpu_instr_active = true;
        self.cpu_instr_bus_cycles = 0;
        self.cpu_instr_extra_master_cycles = 0;
    }

    fn end_cpu_instruction(&mut self, cycles: u8) {
        // 命令内で発生したバスアクセス分は read_u8/write_u8 側で tick 済み。
        // 残り（内部サイクル/ウェイト相当）だけ進める。
        let bus_cycles = self.cpu_instr_bus_cycles;
        let extra_master = self.cpu_instr_extra_master_cycles;
        self.cpu_instr_active = false;
        self.cpu_instr_bus_cycles = 0;
        self.cpu_instr_extra_master_cycles = 0;
        let remaining = cycles.saturating_sub(bus_cycles);
        if remaining != 0 {
            self.tick_cpu_cycles(remaining);
        }
        if extra_master != 0 {
            // Slow/joypad access stretches CPU cycles in master clocks; model as time that
            // elapses with no further S-CPU execution.
            self.add_pending_stall_master_cycles(extra_master);
        }
    }

    fn opcode_memory_penalty(&mut self, addr: u32) -> u8 {
        // General DMA (MDMAEN) begins after the *next opcode fetch* following the write to $420B.
        // We model that by consuming the queued mask here, right after the opcode byte has been
        // read by the core (see cpu_core::fetch_opcode_generic).
        if self.pending_mdma_mask != 0 {
            let mask = self.pending_mdma_mask;
            self.pending_mdma_mask = 0;
            let mut any = false;
            for i in 0..8 {
                if (mask & (1 << i)) == 0 {
                    continue;
                }
                if !self.dma_controller.channels[i].configured {
                    continue;
                }
                any = true;
                self.perform_dma_transfer(i);
            }
            if any {
                self.mdma_started_after_opcode_fetch = true;
            }
        }

        if debug_flags::mem_timing() && self.is_rom_address(addr) && !self.is_fastrom() {
            2
        } else {
            0
        }
    }

    fn take_dma_start_event(&mut self) -> bool {
        let v = self.mdma_started_after_opcode_fetch;
        self.mdma_started_after_opcode_fetch = false;
        v
    }

    fn poll_nmi(&mut self) -> bool {
        self.ppu.nmi_pending()
    }

    fn poll_irq(&mut self) -> bool {
        self.irq_is_pending()
    }

    fn acknowledge_nmi(&mut self) {
        // Clear the latched NMI flag so we don't immediately retrigger.
        self.ppu.clear_nmi();
    }

    fn set_last_cpu_pc(&mut self, pc24: u32) {
        self.last_cpu_pc = pc24;

        // burn-in-test.sfc EXT LATCH: trace tight PC flow with PPU timing (opt-in).
        // Useful to understand whether the latch pulse is occurring at the expected H/V position.
        if std::env::var_os("TRACE_BURNIN_EXT_FLOW").is_some() {
            let bank = (pc24 >> 16) as u8;
            let pc = (pc24 & 0xFFFF) as u16;
            if bank == 0x00 && (0x94C0..=0x9610).contains(&pc) {
                println!(
                    "[BURNIN-EXT][FLOW] PC={:06X} sl={} cyc={} frame={} vblank={} hblank={} wio=0x{:02X}",
                    pc24,
                    self.ppu.scanline,
                    self.ppu.get_cycle(),
                    self.ppu.get_frame(),
                    self.ppu.is_vblank() as u8,
                    self.ppu.is_hblank() as u8,
                    self.wio
                );
            }
        }

        // cputest-full.sfc: detect PASS/FAIL/Invalid by watching known PC points where it prints
        // the result string. This is used by headless runners to exit with an appropriate code.
        //
        // Guarded by:
        // - cpu_test_mode (title-based) AND
        // - HEADLESS=1 AND
        // - CPU_TEST_MODE env var set (explicit opt-in for auto-exit)
        if !self.cpu_test_mode || self.cpu_test_result.is_some() || !crate::debug_flags::headless()
        {
            return;
        }
        static ENABLED: OnceLock<bool> = OnceLock::new();
        let enabled = *ENABLED.get_or_init(|| std::env::var_os("CPU_TEST_MODE").is_some());
        if !enabled {
            return;
        }

        let test_idx = ((self.wram.get(0x0011).copied().unwrap_or(0) as u16) << 8)
            | (self.wram.get(0x0010).copied().unwrap_or(0) as u16);

        // These addresses are stable for the bundled roms/tests/cputest-full.sfc.
        // 00:8199 -> prints "Success"
        // 00:8148/00:81B8 -> prints "Failed"
        // 00:8150 -> prints "Invalid test order"
        self.cpu_test_result = match pc24 {
            0x008199 => Some(CpuTestResult::Pass { test_idx }),
            0x008148 | 0x0081B8 => Some(CpuTestResult::Fail { test_idx }),
            0x008150 => Some(CpuTestResult::InvalidOrder { test_idx }),
            _ => None,
        };
    }
}
