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
use crate::sa1::Sa1;

pub struct Bus {
    wram: Vec<u8>,
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

    // Run-wide counters for headless init summary
    nmitimen_writes_count: u32,
    mdmaen_nonzero_count: u32,
    hdmaen_nonzero_count: u32,

    // DMA config observation (how many writes to $43x0-$43x6 etc.)
    dma_reg_writes: u32,
    // DMA destination histogram (B-bus low 7 bits)
    dma_dest_hist: [u32; 128],
    // Pending graphics DMA mask (strict timing: defer VRAM/CGRAM/OAM MDMA to VBlank)
    pending_gdma_mask: u8,
    // HDMA aggregate stats (visible for headless summaries)
    hdma_lines_executed: u32,
    hdma_bytes_vram: u32,
    hdma_bytes_cgram: u32,
    hdma_bytes_oam: u32,

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
    fn add16_in_bank(addr: u32, delta: u32) -> u32 {
        let bank = addr & 0x00FF_0000;
        let lo = (addr & 0x0000_FFFF).wrapping_add(delta) & 0x0000_FFFF; // allow wrapping within 16-bit
        bank | lo
    }
    #[allow(dead_code)]
    pub fn new(rom: Vec<u8>) -> Self {
        let rom_size = rom.len();
        Self {
            wram: vec![0; 0x20000],
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

            irq_h_enabled: false,
            irq_v_enabled: false,
            irq_pending: false,
            irq_v_matched_line: None,
            h_timer: 0,
            v_timer: 0,
            h_timer_set: false,
            v_timer_set: false,

            joy_busy_counter: 0,
            joy_data: [0; 8],
            joy_busy_scanlines: std::env::var("JOYBUSY_SCANLINES")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(2),

            nmitimen_writes_count: 0,
            mdmaen_nonzero_count: 0,
            hdmaen_nonzero_count: 0,

            wio: 0,
            fastrom: false,
            dma_reg_writes: 0,
            dma_dest_hist: [0; 128],
            pending_gdma_mask: 0,
            hdma_lines_executed: 0,
            hdma_bytes_vram: 0,
            hdma_bytes_cgram: 0,
            hdma_bytes_oam: 0,
            test_apu_print: std::env::var("TESTROM_APU_PRINT")
                .map(|v| v == "1" || v.to_lowercase() == "true")
                .unwrap_or(false),
            test_apu_buf: String::new(),
            sa1: Sa1::new(),
            sa1_bwram: vec![0; 0x40000],
            sa1_iram: [0; 0x800],
            sa1_cycle_deficit: 0,
            sa1_nmi_delay_active: false,
        }
    }

    pub fn new_with_mapper(
        rom: Vec<u8>,
        mapper: crate::cartridge::MapperType,
        sram_size: usize,
    ) -> Self {
        let rom_size = rom.len();
        Self {
            wram: vec![0; 0x20000],
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

            irq_h_enabled: false,
            irq_v_enabled: false,
            irq_pending: false,
            irq_v_matched_line: None,
            h_timer: 0,
            v_timer: 0,
            h_timer_set: false,
            v_timer_set: false,

            joy_busy_counter: 0,
            joy_data: [0; 8],
            joy_busy_scanlines: std::env::var("JOYBUSY_SCANLINES")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(2),

            nmitimen_writes_count: 0,
            mdmaen_nonzero_count: 0,
            hdmaen_nonzero_count: 0,

            wio: 0,
            fastrom: false,
            dma_reg_writes: 0,
            dma_dest_hist: [0; 128],
            pending_gdma_mask: 0,
            hdma_lines_executed: 0,
            hdma_bytes_vram: 0,
            hdma_bytes_cgram: 0,
            hdma_bytes_oam: 0,
            test_apu_print: std::env::var("TESTROM_APU_PRINT")
                .map(|v| v == "1" || v.to_lowercase() == "true")
                .unwrap_or(false),
            test_apu_buf: String::new(),
            sa1: Sa1::new(),
            sa1_bwram: vec![0; sram_size.max(0x2000)],
            sa1_iram: [0; 0x800],
            sa1_cycle_deficit: 0,
            sa1_nmi_delay_active: false,
        }
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

    /// Run the SA-1 core for a slice of time proportional to the S-CPU cycles just executed.
    /// We use a coarse 3:1 frequency ratio (SA-1 ~10.74MHz vs S-CPU 3.58MHz).
    pub fn run_sa1_scheduler(&mut self, cpu_cycles: u8) {
        if !self.is_sa1_active() {
            return;
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

            // Check if SA-1 is in WAI or STP state - if so, break early to avoid spinning
            if self.sa1.cpu.core.state.waiting_for_irq || self.sa1.cpu.core.state.stopped {
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

            self.sa1_cycle_deficit -= sa1_cycles * SA1_RATIO_DEN;
            steps += 1;
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
            // TODO: Implement normal DMA transfer
            let irq_fired = self.sa1.complete_dma();
            if irq_fired && crate::debug_flags::trace_sa1_dma() {
                println!("SA1_DMA: Normal DMA complete, IRQ fired to S-CPU");
            }
        }

        // Check for pending CC-DMA
        if self.sa1.is_ccdma_pending() {
            if crate::debug_flags::trace_sa1_ccdma() {
                self.sa1.log_ccdma_state("process_begin");
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
                if irq_fired {
                    println!("SA1_CCDMA: CC-DMA complete, IRQ fired to S-CPU");
                } else {
                    println!("SA1_CCDMA: CC-DMA complete, no IRQ");
                }
            }
        }
    }

    /// Perform SA-1 character conversion DMA
    fn perform_sa1_ccdma(&mut self) {
        let src = self.sa1.registers.dma_source;
        let dest = self.sa1.registers.dma_dest;
        let len = self.sa1.registers.dma_length as usize;

        if len == 0 {
            return;
        }

        // Get color depth and virtual width from CC-DMA control register
        let color_code = self.sa1.ccdma_color_code();
        let virtual_width_shift = self.sa1.ccdma_virtual_width_shift();

        if crate::debug_flags::trace_sa1_ccdma() {
            println!(
                "SA1_CCDMA: Converting {:?}bpp tiles, vwidth_shift={}, src=0x{:06X}, dest=0x{:06X}, len=0x{:04X}",
                color_code.and_then(|_c| self.sa1.ccdma_color_depth_bits()),
                virtual_width_shift,
                src,
                dest,
                len
            );
        }

        // Simple byte-by-byte copy for now (TODO: implement actual character conversion)
        // In a full implementation, this would:
        // 1. Read compressed/indexed tile data from source
        // 2. Convert to SNES bitplane format
        // 3. Write to BW-RAM destination
        for i in 0..len {
            let src_addr = src.wrapping_add(i as u32);
            let dest_addr = dest.wrapping_add(i as u32);

            let value = self.sa1_read_u8(src_addr);
            self.sa1_write_u8(dest_addr, value);
        }

        if crate::debug_flags::trace_sa1_ccdma() {
            println!("SA1_CCDMA: Transfer complete ({} bytes copied)", len);
        }
    }

    #[inline]
    #[allow(dead_code)]
    pub fn sa1_bwram_slice(&self) -> &[u8] {
        &self.sa1_bwram
    }

    #[inline]
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

    pub fn sa1_read_u8(&mut self, addr: u32) -> u8 {
        let bank = (addr >> 16) & 0xFF;
        let offset = (addr & 0xFFFF) as u16;
        match bank {
            0x00..=0x3F | 0x80..=0xBF => {
                if (0x6000..=0x7FFF).contains(&offset) {
                    // Use SA-1 CPU's own BWRAM mapping register
                    if let Some(idx) = self.sa1_cpu_bwram_addr(offset) {
                        return self.sa1_bwram[idx];
                    }
                }
                self.read_rom_lohi(bank, offset)
            }
            0x40..=0x5F | 0xC0..=0xDF => {
                // Direct BWRAM access for SA-1
                let idx = ((bank & 0x1F) as usize) << 16 | (offset as usize);
                self.sa1_bwram
                    .get(idx % self.sa1_bwram.len())
                    .copied()
                    .unwrap_or(0)
            }
            _ => self.read_rom_lohi(bank, offset),
        }
    }

    pub fn sa1_write_u8(&mut self, addr: u32, value: u8) {
        let bank = (addr >> 16) & 0xFF;
        let offset = (addr & 0xFFFF) as u16;
        match bank {
            0x00..=0x3F | 0x80..=0xBF => {
                if (0x6000..=0x7FFF).contains(&offset) {
                    // Use SA-1 CPU's own BWRAM mapping register
                    if let Some(idx) = self.sa1_cpu_bwram_addr(offset) {
                        self.sa1_bwram[idx] = value;
                    }
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

        // Update MDR for open bus behavior
        let value = match bank {
            // Dragon Quest 3 special banks - highest priority
            0x03 | 0x24 if self.mapper_type == crate::cartridge::MapperType::DragonQuest3 => {
                return self.read_dq3_rom(bank, offset);
            }
            // System area banks (mirror in 80-BF)
            0x00..=0x3F | 0x80..=0xBF => {
                match offset {
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
                            return self.sa1_bwram[idx];
                        }
                        0xFF
                    }
                    // SA-1 register window
                    0x2200..=0x23FF if self.is_sa1_active() => {
                        return self.sa1.read_register(offset - 0x2200);
                    }
                    // DQ3 special: system bank low area maps to ROM for specific banks (and bank 00 for BRK/COP vectors)
                    0x2200..=0x7FFF
                        if self.mapper_type == crate::cartridge::MapperType::DragonQuest3 =>
                    {
                        // Dragon Quest III uses a special mapping where bank 00 low area
                        // (0x2200-0x7FFF) maps to ROM, not WRAM
                        // This is essential for proper execution as the game expects code here
                        let effective_bank = bank & 0x3F;
                        let rom_addr = (effective_bank as usize) * 0x10000 + (offset as usize);

                        // Debug logging for problematic address range
                        if (crate::debug_flags::mapper() || crate::debug_flags::boot_verbose())
                            && bank == 0x00
                            && (0x7220..=0x7230).contains(&offset)
                        {
                            static mut DQ3_BANK00_DEBUG: u32 = 0;
                            unsafe {
                                DQ3_BANK00_DEBUG += 1;
                                if DQ3_BANK00_DEBUG <= 20 {
                                    let value = if rom_addr < self.rom_size {
                                        self.rom[rom_addr]
                                    } else {
                                        0xFF
                                    };
                                    println!(
                                        "DQ3 Bank00:{:04X} -> ROM[0x{:06X}] = 0x{:02X}",
                                        offset, rom_addr, value
                                    );
                                }
                            }
                        }

                        return if rom_addr < self.rom_size {
                            self.rom[rom_addr]
                        } else {
                            0xFF
                        };
                    }
                    // PPU registers
                    0x2100..=0x213F => self.ppu.read(offset & 0xFF),
                    // APU registers
                    0x2140..=0x217F => {
                        if let Ok(mut apu) = self.apu.lock() {
                            apu.read((offset & 0x3F) as u8)
                        } else {
                            0
                        }
                    }
                    // WRAM access port
                    0x2180 => {
                        let addr = self.wram_address as usize;
                        if addr < self.wram.len() {
                            let value = self.wram[addr];
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
                                // SRAM in LoROM
                                if (0x70..=0x7D).contains(&bank) {
                                    let sram_addr = offset as usize;
                                    if sram_addr < self.sram.len() {
                                        self.sram[sram_addr]
                                    } else {
                                        0xFF
                                    }
                                } else {
                                    0xFF // Open bus
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
                                // DQ3 SRAM (HiROMベース)
                                let sram_addr =
                                    ((bank as usize) << 13) + ((offset - 0x6000) as usize);
                                if sram_addr < self.sram.len() {
                                    self.sram[sram_addr]
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
                        // - banks 0x40-0x7D: upper half (0x8000-0xFFFF) = ROM
                        // - banks 0x70-0x7D: lower half (0x0000-0x7FFF) = SRAM
                        if offset < 0x8000 {
                            if (0x70..=0x7D).contains(&bank) {
                                let sram_addr =
                                    ((bank - 0x40) as usize) * 0x8000 + (offset as usize);
                                if sram_addr < self.sram.len() {
                                    self.sram[sram_addr]
                                } else {
                                    0xFF
                                }
                            } else {
                                0xFF
                            }
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
                let wram_addr = ((bank - 0x7E) as usize) * 0x10000 + (offset as usize);
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
                            if (0x70..=0x7D).contains(&mirror_bank) {
                                let sram_addr =
                                    ((mirror_bank - 0x40) as usize) * 0x8000 + (offset as usize);
                                if sram_addr < self.sram.len() {
                                    self.sram[sram_addr]
                                } else {
                                    0xFF
                                }
                            } else {
                                0xFF
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
    fn read_dq3_rom(&self, bank: u32, offset: u16) -> u8 {
        // Dragon Quest 3 (Type 31) 専用メモリマッピング
        // 4MB ROM + エンハンスメントチップ対応（HiROMベース）

        let rom_addr = match bank {
            // 特殊バンク：Bank 03/24をROM低領域にマップ（エンハンスメントチップ用）
            0x03 => {
                // Bank 03 -> ROM先頭から3番目のバンク相当
                0x30000 + (offset as usize)
            }
            0x24 => {
                // Bank 24 -> ROM中間部のバンクにマップ
                0x240000 + (offset as usize)
            }
            // 標準ROMバンク 40-7F（HiROMベース）
            0x40..=0x7F => {
                // Dragon Quest 3はHiROMベースの4MB ROM
                // Banks 40-7F map directly to ROM
                let base_addr = (bank - 0x40) as usize * 0x10000;
                base_addr + (offset as usize)
            }
            // ミラー領域 C0-FF（HiROMミラー: 40-7F に対応）
            0xC0..=0xFF => {
                // In HiROM, C0-FF mirrors 40-7F (full 64KB banks)
                let mapped_bank = bank - 0x80; // C0->40 .. FF->7F
                let rom_addr = (mapped_bank as usize) * 0x10000 + (offset as usize);

                // Debug C0 bank access during boot
                if std::env::var_os("DEBUG_DQ3_C0_ACCESS").is_some() {
                    static mut C0_ACCESS_COUNT: u32 = 0;
                    unsafe {
                        C0_ACCESS_COUNT += 1;
                        if C0_ACCESS_COUNT <= 20 {
                            println!("DQ3 C0 access: bank=${:02X} offset=${:04X} rom_addr=0x{:06X} mapped_bank=${:02X}",
                                bank, offset, rom_addr, mapped_bank);
                        }
                    }
                }

                rom_addr % self.rom_size
            }
            // システム領域バンク 00-3F, 80-BF
            0x00..=0x3F | 0x80..=0xBF => {
                let effective_bank = bank & 0x3F;
                if offset >= 0x8000 {
                    // 上位半分(0x8000-0xFFFF)：HiROMマッピング
                    // Map to ROM banks 00-3F in HiROM style
                    (effective_bank as usize) * 0x10000 + (offset as usize)
                } else {
                    // 下位半分(0x0000-0x7FFF)：Dragon Quest 3専用マッピング
                    // For DQ3, the low area also maps to ROM
                    (effective_bank as usize) * 0x10000 + (offset as usize)
                }
            }
            _ => return 0xFF,
        };

        // Bank 08 ROM読み取りデバッグ
        if (crate::debug_flags::mapper() || crate::debug_flags::boot_verbose())
            && bank == 0x08
            && (0x7220..=0x7240).contains(&offset)
        {
            static mut BANK08_DQ3_DEBUG_COUNT: u32 = 0;
            unsafe {
                BANK08_DQ3_DEBUG_COUNT += 1;
                if BANK08_DQ3_DEBUG_COUNT <= 10 {
                    println!(
                        "DQ3 ROM read Bank 08:{:04X} -> rom_addr=0x{:06X}, rom_size=0x{:06X}",
                        offset, rom_addr, self.rom_size
                    );

                    if rom_addr < self.rom_size {
                        let value = self.rom[rom_addr];
                        println!(
                            "  DQ3 ROM[0x{:06X}] = 0x{:02X} (Bank {:02X}:{:04X})",
                            rom_addr, value, bank, offset
                        );
                    } else {
                        println!(
                            "  DQ3 ROM address 0x{:06X} is out of range (size=0x{:06X})",
                            rom_addr, self.rom_size
                        );
                    }
                }
            }
        }

        // ROMサイズ内なら直接読み取り、範囲外なら適切なミラー処理
        if rom_addr < self.rom_size {
            let value = self.rom[rom_addr];

            // Debug C0:0FE3 BRK issue
            if (crate::debug_flags::mapper() || crate::debug_flags::boot_verbose())
                && bank == 0xC0
                && (0x0FE0..=0x0FF0).contains(&offset)
            {
                static mut C0_LOW_DEBUG: u32 = 0;
                unsafe {
                    C0_LOW_DEBUG += 1;
                    if C0_LOW_DEBUG <= 10 {
                        println!(
                            "DEBUG C0:{:04X} -> ROM[0x{:06X}] = 0x{:02X}",
                            offset, rom_addr, value
                        );
                    }
                }
            }

            // Bank 42の問題のあるFFxx領域の値をログ
            if (crate::debug_flags::mapper() || crate::debug_flags::boot_verbose())
                && bank == 0x42
                && offset >= 0xFF00
            {
                static mut BANK42_VALUE_COUNT: u32 = 0;
                unsafe {
                    BANK42_VALUE_COUNT += 1;
                    if BANK42_VALUE_COUNT <= 5 {
                        println!(
                            "  DQ3 ROM[0x{:06X}] = 0x{:02X} (Bank {:02X}:{:04X})",
                            rom_addr, value, bank, offset
                        );
                    }
                }
            }

            value
        } else {
            // Dragon Quest 3 4MB ROM のミラー処理
            let mirror_addr = rom_addr % self.rom_size;
            self.rom[mirror_addr]
        }
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
                // ドラクエ3専用マッピング（HiROMベースに統一）
                if offset < 0x8000 {
                    // エンハンスメントチップ/特例
                    return self.handle_dq3_enhancement(bank, offset);
                }
                // 上位半分はHiROMスタイルでフル64KBをマップ
                let mut rom_addr = (bank as usize) * 0x10000 + (offset as usize);
                if rom_addr >= self.rom_size {
                    rom_addr %= self.rom_size;
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

    pub fn write_u8(&mut self, addr: u32, value: u8) {
        let bank = (addr >> 16) & 0xFF;
        let offset = (addr & 0xFFFF) as u16;

        match bank {
            // System area banks (mirror in 80-BF)
            0x00..=0x3F | 0x80..=0xBF => {
                match offset {
                    // Stack area (0x0100-0x01FF)
                    0x0100..=0x01FF => {
                        // Debug stack corruption - trace suspicious writes
                        if std::env::var_os("DEBUG_STACK_TRACE").is_some() {
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
                    0x0000..=0x00FF | 0x0200..=0x1FFF => self.wram[offset as usize] = value,
                    // Mirror of first page (0x000-0x0FF) in 0x2000-0x20FF
                    0x2000..=0x20FF => self.wram[(offset & 0xFF) as usize] = value,
                    0x6000..=0x7FFF if self.is_sa1_active() => {
                        if let Some(idx) = self.sa1_bwram_addr(offset) {
                            self.sa1_bwram[idx] = value;
                        }
                    }
                    // PPU registers (no DQ3-specific overrides)
                    0x2100..=0x213F => {
                        self.ppu.write(offset & 0xFF, value);
                    }
                    0x2200..=0x23FF if self.is_sa1_active() => {
                        self.sa1.write_register(offset - 0x2200, value);
                    }
                    // APU registers
                    0x2140..=0x217F => {
                        if let Ok(mut apu) = self.apu.lock() {
                            let port = (offset & 0x3F) as u8;
                            static mut APU_WR_LOG: u32 = 0;
                            unsafe {
                                APU_WR_LOG += 1;
                                if APU_WR_LOG <= 16 && crate::debug_flags::boot_verbose() {
                                    println!("APU WRITE port=0x{:02X} value=0x{:02X}", port, value);
                                }
                            }
                            apu.write(port, value);
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
                            self.wram[addr] = value;
                            self.wram_address = (self.wram_address + 1) & 0x1FFFF;
                        }
                    }
                    // WRAM Address registers
                    0x2181 => {
                        self.wram_address = (self.wram_address & 0xFFFF00) | (value as u32);
                    }
                    0x2182 => {
                        self.wram_address = (self.wram_address & 0xFF00FF) | ((value as u32) << 8);
                    }
                    0x2183 => {
                        self.wram_address =
                            (self.wram_address & 0x00FFFF) | (((value & 0x01) as u32) << 16);
                    }
                    // Expansion area - ignore writes
                    0x2184..=0x21FF => {}
                    0x2200..=0x3FFF => {}
                    // Controller/IO registers
                    0x4000..=0x42FF => self.write_io_register(offset, value),
                    // DMA registers
                    0x4300..=0x43FF => {
                        self.dma_controller.write(offset, value);
                        self.dma_reg_writes = self.dma_reg_writes.saturating_add(1);
                    }
                    // More IO registers
                    0x4400..=0x5FFF => self.write_io_register(offset, value),
                    // Expansion area/unused
                    0x6000..=0x7FFF => {
                        match self.mapper_type {
                            crate::cartridge::MapperType::LoRom => {
                                // SRAM in LoROM
                                if (0x70..=0x7D).contains(&bank) {
                                    let sram_addr = offset as usize;
                                    if sram_addr < self.sram.len() {
                                        self.sram[sram_addr] = value;
                                        self.sram_dirty = true;
                                    }
                                }
                                // ROM writes are ignored
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
                                // DQ3 SRAM (HiROMベース)
                                let sram_addr =
                                    ((bank as usize) << 13) + ((offset - 0x6000) as usize);
                                if sram_addr < self.sram.len() {
                                    self.sram[sram_addr] = value;
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
                        // SRAM in lower half of these banks
                        if offset < 0x8000 {
                            let sram_addr = ((bank - 0x40) as usize) * 0x8000 + (offset as usize);
                            if sram_addr < self.sram.len() {
                                self.sram[sram_addr] = value;
                                self.sram_dirty = true;
                            }
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
                let wram_addr = ((bank - 0x7E) as usize) * 0x10000 + (offset as usize);
                if wram_addr < self.wram.len() {
                    self.wram[wram_addr] = value;
                }
            }
            // ROM mirror banks - writes ignored (except SRAM areas)
            0xC0..=0xFF => {
                // Some SRAM might be accessible here depending on mapper
                match self.mapper_type {
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
            0x4016 => self.input_system.read_controller1(),
            0x4017 => self.input_system.read_controller2(),
            // 0x4210 - RDNMI: NMI flag and version
            0x4210 => {
                // bit7: NMI occurred since last read (cleared on read)
                // bit4-0: version (we return 0x02 like many emus)
                let mut value = 0x02;
                if self.ppu.nmi_flag {
                    value |= 0x80;
                    self.ppu.nmi_flag = false; // reading clears the flag
                }
                value
            }
            // 0x4211 - TIMEUP: IRQ time-up (read/clear)
            0x4211 => {
                let v = if self.irq_pending { 0x80 } else { 0x00 };
                self.irq_pending = false; // reading clears
                v
            }
            // 0x4212 - HVBJOY: H/V-Blank and Joypad busy flags
            0x4212 => {
                let mut value = 0u8;
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
                value
            }
            // 0x4213 - RDIO: Programmable I/O port readback
            0x4213 => {
                // Minimal behavior: return last value written to $4201.
                // Some hw ties bits to controller/expansion; we keep it simple for now.
                self.wio
            }
            // JOY1/2/3/4 data
            0x4218 => self.joy_data[0], // JOY1L
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
            0x420B => self.dma_controller.read(addr),
            0x420C => self.dma_controller.read(addr),
            // APU registers readback
            0x2140..=0x217F => {
                let port = (addr & 0x3F) as u8;
                if let Ok(mut apu) = self.apu.lock() {
                    let v = apu.read(port);
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
                self.input_system.write_strobe(value);
            }
            // PPU/CPU communication
            0x4200 => {
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
                // If NMI is enabled mid-VBlank, hardware latches an NMI immediately.
                if nmi_en && !prev_nmi_en && self.ppu.is_vblank() && !self.ppu.is_nmi_latched() {
                    self.ppu.latch_nmi_now();
                }
                // If NMI is disabled, drop any pending flag to avoid spurious triggers.
                if !nmi_en {
                    self.ppu.nmi_flag = false;
                }
                // bit0: auto-joypad enable (ignored here)
                println!(
                    "$4200 NMITIMEN write: 0x{:02X} (NMI:{}, IRQ:{}, Auto-joypad:{})",
                    self.nmitimen,
                    (self.nmitimen & 0x80) != 0,
                    (self.nmitimen & 0x20) != 0,
                    (self.nmitimen & 0x01) != 0
                );
            }
            // WRIO - Joypad Programmable I/O Port; read back via $4213
            0x4201 => {
                self.wio = value;
            }
            0x4202 => {
                // WRMPYA - Multiplicand A (8-bit)
                self.mul_a = value;
            }
            0x4203 => {
                // WRMPYB - Multiplicand B (start 8x8 multiply)
                self.mul_b = value;
                self.mul_result = (self.mul_a as u16) * (self.mul_b as u16);
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
                if self.div_b == 0 {
                    self.div_quot = 0xFFFF;
                    self.div_rem = self.div_a;
                } else {
                    let divisor = u16::from(self.div_b);
                    self.div_quot = self.div_a / divisor;
                    self.div_rem = self.div_a % divisor;
                }
                // Also reflect remainder into mul_result registers per common emu behavior
                self.mul_result = self.div_rem;
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
                self.dma_controller.write(addr, value);
                if value != 0 {
                    self.mdmaen_nonzero_count = self.mdmaen_nonzero_count.saturating_add(1);
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
                // Execute non-deferred channels immediately
                if now_mask != 0 {
                    self.execute_pending_dma(now_mask);
                }

                // Enhanced DMA monitoring for graphics transfers
                if value != 0 {
                    static mut GRAPHICS_DMA_COUNT: u32 = 0;
                    unsafe {
                        GRAPHICS_DMA_COUNT += 1;
                    }

                    println!(
                        "🎮 GRAPHICS DMA[{}]: MDMAEN set: 0x{:02X}",
                        unsafe { GRAPHICS_DMA_COUNT },
                        value
                    );
                    for i in 0..8 {
                        if value & (1 << i) != 0 {
                            let ch = &self.dma_controller.channels[i];
                            let dest_masked = ch.dest_address & 0x7F;
                            let dir_cpu_to_ppu = (ch.control & 0x80) == 0;

                            // Detect graphics-related transfers
                            let is_vram = dest_masked == 0x18 || dest_masked == 0x19;
                            let is_cgram = dest_masked == 0x22;
                            let is_oam = dest_masked == 0x04;

                            let transfer_type = if is_vram {
                                "VRAM"
                            } else if is_cgram {
                                "CGRAM"
                            } else if is_oam {
                                "OAM"
                            } else {
                                "OTHER"
                            };

                            println!(
                                "  📊 CH{} [{}] ctrl=0x{:02X} {} dest=$21{:02X} src=0x{:06X} size={} unit={} incmode={}",
                                i,
                                transfer_type,
                                ch.control,
                                if dir_cpu_to_ppu {"CPU->PPU"} else {"PPU->CPU"},
                                dest_masked,
                                ch.src_address,
                                ch.size,
                                ch.get_transfer_unit(),
                                ch.get_address_mode()
                            );
                            if crate::debug_flags::cgram_dma() && is_cgram {
                                println!(
                                    "➡️  CGRAM DMA start: ch{} size={} src=0x{:06X} (unit={} addr_mode={})",
                                    i, ch.size, ch.src_address, ch.get_transfer_unit(), ch.get_address_mode()
                                );
                            }
                        }
                    }
                }
                // DMA転送を実行（非遅延分）
                for i in 0..8 {
                    if now_mask & (1 << i) != 0 {
                        // Skip unconfigured channels to avoid bogus huge transfers
                        if !self.dma_controller.channels[i].configured {
                            continue;
                        }
                        self.perform_dma_transfer(i);
                    }
                }
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

    pub fn irq_is_pending(&self) -> bool {
        self.irq_pending
    }

    pub fn clear_irq_pending(&mut self) {
        self.irq_pending = false;
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
            let h = self.h_timer;
            // Detect crossing of the H timer threshold within this PPU step
            if old_cycle <= h && h < new_cycle {
                h_match = true;
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
            // Latch controller states into JOY registers
            let b1 = self.input_system.controller1.get_buttons();
            let b2 = self.input_system.controller2.get_buttons();
            let mt = self.input_system.is_multitap_enabled();
            let b3 = if mt {
                self.input_system.controller3_buttons()
            } else {
                0
            };
            let b4 = if mt {
                self.input_system.controller4_buttons()
            } else {
                0
            };
            self.joy_data[0] = (b1 & 0x00FF) as u8; // JOY1L: B,Y,Sel,Start,Up,Down,Left,Right
            self.joy_data[1] = ((b1 >> 8) & 0x00FF) as u8; // JOY1H: A,X,L,R
            self.joy_data[2] = (b2 & 0x00FF) as u8; // JOY2L
            self.joy_data[3] = ((b2 >> 8) & 0x00FF) as u8; // JOY2H
                                                           // JOY3/4 for multitap
            self.joy_data[4] = (b3 & 0x00FF) as u8;
            self.joy_data[5] = ((b3 >> 8) & 0x00FF) as u8;
            self.joy_data[6] = (b4 & 0x00FF) as u8;
            self.joy_data[7] = ((b4 >> 8) & 0x00FF) as u8;
            // Set JOYBUSY for a short duration (approximation)
            self.joy_busy_counter = self.joy_busy_scanlines; // configurable number of scanlines
        }
        // Strict timing: run deferred graphics DMA now
        if self.pending_gdma_mask != 0 {
            let mask = self.pending_gdma_mask;
            self.pending_gdma_mask = 0;
            // Execute snapshots
            self.execute_pending_dma(mask);
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
        let dest_base = { self.dma_controller.channels[channel].dest_address & 0x3F };
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
            let dest_off = if dest_base == 0x22 {
                0x22
            } else if dest_base == 0x04 {
                0x04
            } else {
                Self::hdma_dest_offset(unit, dest_base, i as u8)
            };
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
            7 => 1,
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
            5 => base.wrapping_add((i >> 1) & 3), // A,A,B,B,C,C,D,D
            6 => base.wrapping_add((i >> 1) & 1), // A,A,B,B
            7 => base,                            // A,A
            _ => base,
        }
    }

    // 通常のDMA転送処理
    fn perform_dma_transfer(&mut self, channel: usize) {
        // General DMA: mark MDMA during this burst
        self.ppu.begin_mdma_context();
        let ch = &self.dma_controller.channels[channel];
        // Skip obviously unconfigured junk (only skip if completely unconfigured)
        if !ch.configured || ch.control == 0 {
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
        let mut transfer_size = ch.size as u32;
        if transfer_size == 0 {
            transfer_size = 0x10000;
        } // size=0 means 65536 bytes on SNES
        let src_addr = ch.src_address;

        // 特定ROM用のアドレス補正ハックは廃止（正規マッピング/CPU実装で解決する）

        // B-bus destination uses low 7 bits (0x2100-0x217F)
        let dest_base_full = ch.dest_address & 0x7F;
        let transfer_unit = ch.get_transfer_unit();

        // DMA転送のデバッグ（許可時のみ）

        // 転送方向を取得
        let cpu_to_ppu = (ch.control & 0x80) == 0;

        // Early sanity check: skip obviously invalid B-bus target ranges to reduce noise
        // CPU->PPU: allow $2100-$2133 and $2140-$2143 only
        // PPU->CPU: allow $2134-$213F and $2140-$2143 only
        let allowed = if cpu_to_ppu {
            (dest_base_full <= 0x33) || (0x40..=0x43).contains(&dest_base_full)
        } else {
            (0x34..=0x3F).contains(&dest_base_full) || (0x40..=0x43).contains(&dest_base_full)
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
                    dest_base_full & 0x7F
                );
            }
        }

        // Special log for CGRAM transfers
        if dest_base_full == 0x22 && cpu_to_ppu {
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

        // 実際の転送を実行
        let mut cur_src = src_addr;
        let addr_mode = ch.get_address_mode(); // 0:inc, 1:fix, 2:dec, 3:inc(approx)
        let mut i = 0;
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
                let data = self.read_u8(cur_src);
                // Bバス宛先アドレスを転送モードに応じて決定
                // Special-cases for B-bus ports
                let dest_offset = self.mdma_dest_offset(transfer_unit, dest_base_full, i as u8);

                let dest_full = 0x2100 + dest_offset as u32;
                self.dma_hist_note(dest_offset);

                // Log INIDISP ($2100) writes during DMA to diagnose forced blank issues
                if dest_offset == 0x00 {
                    static INIDISP_DMA_COUNT: OnceLock<AtomicU32> = OnceLock::new();
                    let n = INIDISP_DMA_COUNT
                        .get_or_init(|| AtomicU32::new(0))
                        .fetch_add(1, Ordering::Relaxed)
                        + 1;
                    if std::env::var_os("DEBUG_DMA").is_some() || n <= 20 {
                        println!(
                            "⚠️  DMA write to INIDISP #{}: CH{} src=0x{:06X} value=0x{:02X} (blank={} brightness={})",
                            n, channel, cur_src, data,
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
                    self.write_u8(dest_full, data);
                } else if (0x40..=0x43).contains(&dest_offset) {
                    // APU I/O ($2140-$2143)
                    self.write_u8(dest_full, data);
                } else {
                    // $2134-$213F read-only or $2144-$217F undefined: ignore
                    static DMA_SKIP_DEST_LOGGED: OnceLock<Mutex<[bool; 128]>> = OnceLock::new();
                    let mut logged = DMA_SKIP_DEST_LOGGED
                        .get_or_init(|| Mutex::new([false; 128]))
                        .lock()
                        .unwrap();
                    let idx = dest_offset as usize;
                    if debug_flags::dma() && !logged[idx] {
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
                let dest_reg = 0x2100 + dest_base_full as u32; // simple read from base
                let data = self.read_u8(dest_reg);
                self.write_u8(cur_src, data);
            }

            // A-busアドレスの更新（バンク固定、16bitアドレスのみ増減）
            let bank = cur_src & 0x00FF_0000;
            let lo16 = (cur_src & 0x0000_FFFF) as u16;
            let next_lo16 = match addr_mode {
                0 => lo16.wrapping_add(1), // inc
                1 => lo16,                 // fixed
                2 => lo16.wrapping_sub(1), // dec
                _ => lo16.wrapping_add(1), // treat 3 as inc
            } as u32;
            cur_src = bank | next_lo16;
            i += 1;
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

        // DMAカウンタをリセット
        self.dma_controller.channels[channel].size = 0;
        self.ppu.end_mdma_context();
    }

    #[inline]
    fn mdma_dest_offset(&self, unit: u8, base: u8, index: u8) -> u8 {
        // Fixed ports
        if base == 0x22 {
            return 0x22;
        } // CGDATA
        if base == 0x04 {
            return 0x04;
        } // OAMDATA

        let i = index as usize;
        let b = base as usize;
        // Table-driven patterns for modes 0..7
        const P0: &[u8] = &[0];
        const P1: &[u8] = &[0, 1];
        const P2: &[u8] = &[0, 0];
        const P3: &[u8] = &[0, 0, 1, 1];
        const P4: &[u8] = &[0, 1, 2, 3];
        const P5: &[u8] = &[0, 0, 1, 1, 2, 2, 3, 3];
        const P6: &[u8] = &[0, 0, 1, 1];
        const P7: &[u8] = &[0, 0];
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
        let rel = pat[i % pat.len()] as usize;
        ((b + rel) & 0x7F) as u8
    }

    fn dma_hist_note(&mut self, dest_off: u8) {
        let idx = (dest_off & 0x7F) as usize;
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
                          // Any others with counts
        for (i, &n) in self.dma_dest_hist.iter().enumerate() {
            if n > 0
                && !matches!(
                    i as u8,
                    0x04 | 0x00 | 0x15 | 0x16 | 0x17 | 0x18 | 0x19 | 0x21 | 0x22 | 0x2C
                )
            {
                parts.push(format!("$21{:02X}:{}", i, n));
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

    // DMA実行処理（借用問題を避けるため分離）
    fn execute_pending_dma(&mut self, channels_mask: u8) {
        // Grouped MDMA execution (e.g., deferred to VBlank)
        self.ppu.begin_mdma_context();
        for channel in 0..8 {
            if channels_mask & (1 << channel) == 0 {
                continue;
            }

            let size = self.dma_controller.channels[channel].size;
            // SNES DMA: size=0 means transfer 65536 bytes
            let transfer_size = if size == 0 { 0x10000 } else { size as u32 };

            let src_addr = self.dma_controller.channels[channel].src_address;
            let dest_reg = 0x2100 + self.dma_controller.channels[channel].dest_address as u32;

            if crate::debug_flags::dma() {
                println!(
                    "DMA CH{}: Executing {} bytes from {:06X} to reg {:04X}",
                    channel, transfer_size, src_addr, dest_reg
                );
            }

            // 実際の転送実行
            for i in 0..transfer_size {
                let byte = self.read_u8(src_addr + i);

                // Debug first few bytes of VRAM transfers
                if dest_reg == 0x2118 && i < 16 && crate::debug_flags::dma() {
                    println!(
                        "  DMA CH{}: byte {}: 0x{:02X} from {:06X} -> VRAM",
                        channel,
                        i,
                        byte,
                        src_addr + i
                    );
                }

                self.write_u8(dest_reg, byte);
            }

            // 転送後にサイズをクリア
            self.dma_controller.channels[channel].size = 0;
        }
        self.ppu.end_mdma_context();
    }
}

impl CpuBus for Bus {
    fn read_u8(&mut self, addr: u32) -> u8 {
        Bus::read_u8(self, addr)
    }

    fn write_u8(&mut self, addr: u32, value: u8) {
        Bus::write_u8(self, addr, value)
    }

    fn poll_irq(&mut self) -> bool {
        self.irq_is_pending()
    }
}
