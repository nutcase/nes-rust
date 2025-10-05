//! SA-1 co-processor implementation.
//!
//! This module mirrors the main S-CPU implementation but wraps it in a
//! lightweight adapter so both processors can share the same 65C816 core. Most
//! behavioural details are still approximations; the priority is to expose the
//! register interface and scheduling hooks needed by the rest of the emulator.

use crate::cpu::{Cpu, StatusFlags};
use crate::cpu_bus::CpuBus;

/// SA-1 control/status register file ($2200-$23FF window as seen by the S-CPU).
#[derive(Debug, Clone)]
pub struct Registers {
    pub control: u8,       // $2200
    pub sie: u8,           // $2201 S-CPU interrupt enable mirror
    pub sic: u8,           // $2202 S-CPU interrupt clear
    pub reset_vector: u16, // $2203/$2204
    pub nmi_vector: u16,   // $2205/$2206
    pub irq_vector: u16,   // $2207/$2208
    pub scnt: u8,          // $2209 S-CPU control
    pub cie: u8,           // $220A SA-1 interrupt enable (to S-CPU)
    pub cic: u8,           // $220B SA-1 interrupt clear (to S-CPU)
    pub snv: u16,          // $220C/$220D S-CPU NMI vector
    pub siv: u16,          // $220E/$220F S-CPU IRQ vector
    pub tmcnt: u8,         // $2210 timer control (aka CFR in some docs)
    pub ctr: u8,           // $2211 timer counter
    pub h_timer: u16,      // $2212/$2213 H-timer compare
    pub v_timer: u16,      // $2214/$2215 V-timer compare

    pub dma_control: u8,   // $2230 DMA control (DCNT)
    pub ccdma_control: u8, // $2231 char-conversion DMA control (CDMA)
    pub dma_source: u32,   // $2232-$2234 source address
    pub dma_dest: u32,     // $2235-$2237 destination address
    pub dma_length: u16,   // $2238/$2239 transfer length (normal DMA)

    pub interrupt_enable: u8,  // combined CIE|SIE mask
    pub interrupt_pending: u8, // pending bits delivered to S-CPU
    pub timer_pending: u8,     // pending timer IRQ bits

    pub bwram_select_snes: u8, // $2224 SNES-side BW-RAM mapping
    pub bwram_select_sa1: u8,  // $2225 SA-1 BW-RAM mapping
    pub sbwe: u8,              // $2226 SNES BW-RAM write enable (bit7)
    pub cbwe: u8,              // $2227 SA-1 BW-RAM write enable (bit7)
    pub bwram_protect: u8,     // $2228 BW-RAM write-protected area (low nibble)
    pub iram_wp_snes: u8,      // $2229 SNES I-RAM write protection mask
    pub iram_wp_sa1: u8,       // $222A SA-1 I-RAM write protection mask

    pub sfr: u8, // $2300 SA-1 status flags
    #[allow(dead_code)]
    pub status: u8, // read-only mirror for $2300 reads

    pub dma_pending: bool,
    pub ccdma_pending: bool,
    pub ccdma_buffer_ready: bool,
    pub handshake_state: u8,
}

impl Default for Registers {
    fn default() -> Self {
        Self {
            control: 0,
            sie: 0,
            sic: 0,
            reset_vector: 0,
            nmi_vector: 0,
            irq_vector: 0,
            scnt: 0,
            cie: 0,
            cic: 0,
            snv: 0,
            siv: 0,
            tmcnt: 0,
            ctr: 0,
            h_timer: 0,
            v_timer: 0,
            dma_control: 0,
            ccdma_control: 0,
            dma_source: 0,
            dma_dest: 0,
            dma_length: 0,
            interrupt_enable: 0,
            interrupt_pending: 0,
            timer_pending: 0,
            bwram_select_snes: 0,
            bwram_select_sa1: 0,
            sbwe: 0,
            cbwe: 0,
            bwram_protect: 0,
            iram_wp_snes: 0xFF,
            iram_wp_sa1: 0xFF,
            sfr: 0,
            status: 0,
            dma_pending: false,
            ccdma_pending: false,
            ccdma_buffer_ready: false,
            handshake_state: 0,
        }
    }
}

/// Shared bus adapter used to feed the SA-1 core without borrowing the [`Bus`]
/// mutably for the entire duration of the step.
struct Sa1BusAdapter<'a> {
    bus_ptr: *mut crate::bus::Bus,
    _marker: core::marker::PhantomData<&'a ()>,
}

impl<'a> Sa1BusAdapter<'a> {
    fn new(bus: &'a mut crate::bus::Bus) -> Self {
        Self {
            bus_ptr: bus as *mut crate::bus::Bus,
            _marker: core::marker::PhantomData,
        }
    }

    #[inline]
    unsafe fn bus(&mut self) -> &mut crate::bus::Bus {
        &mut *self.bus_ptr
    }
}

impl<'a> CpuBus for Sa1BusAdapter<'a> {
    fn read_u8(&mut self, addr: u32) -> u8 {
        unsafe { self.bus().sa1_read_u8(addr) }
    }

    fn write_u8(&mut self, addr: u32, value: u8) {
        unsafe { self.bus().sa1_write_u8(addr, value) }
    }

    fn poll_irq(&mut self) -> bool {
        false // SA-1 external IRQ routing not yet modelled
    }

    fn poll_nmi(&mut self) -> bool {
        false
    }

    fn opcode_memory_penalty(&mut self, addr: u32) -> u8 {
        unsafe { self.bus().opcode_memory_penalty(addr) }
    }
}

/// SA-1 co-processor state wrapper.
pub struct Sa1 {
    pub cpu: Cpu,
    pub registers: Registers,
    pub(crate) boot_vector_applied: bool,
    h_timer_accum: u32,
    v_timer_accum: u32,
}

impl Sa1 {
    pub(crate) const IRQ_DMA_BIT: u8 = 0x20;
    pub(crate) const IRQ_CCDMA_BIT: u8 = 0x20;

    pub fn new() -> Self {
        let mut cpu = Cpu::new();
        cpu.emulation_mode = false;
        cpu.p = StatusFlags::from_bits_truncate(0x34);
        cpu.core.reset(StatusFlags::from_bits_truncate(0x34), false);
        Self {
            cpu,
            registers: Registers::default(),
            boot_vector_applied: false,
            h_timer_accum: 0,
            v_timer_accum: 0,
        }
    }

    #[allow(dead_code)]
    pub fn reset(&mut self, vector: u16) {
        self.cpu.reset(vector);
        self.cpu.emulation_mode = false;
        self.cpu.p = StatusFlags::from_bits_truncate(0x34);
        self.cpu
            .core
            .reset(StatusFlags::from_bits_truncate(0x34), false);
        self.registers = Registers::default();
        self.boot_vector_applied = false;
        self.h_timer_accum = 0;
        self.v_timer_accum = 0;
    }

    pub fn step(&mut self, bus: &mut crate::bus::Bus) -> u8 {
        if !self.boot_vector_applied && self.registers.reset_vector != 0 {
            self.cpu.pc = self.registers.reset_vector;
            self.cpu.pb = 0xC0; // SA-1 boot ROM bank heuristic
            self.boot_vector_applied = true;
        }

        let mut adapter = Sa1BusAdapter::new(bus);
        let cycles = self.cpu.step_with_bus(&mut adapter);
        self.cpu.sync_cpu_from_core();
        cycles
    }

    #[inline]
    fn update_interrupt_mask(&mut self) {
        self.registers.interrupt_enable = self.registers.sie | self.registers.cie;
    }

    #[inline]
    fn ccdma_enabled(&self) -> bool {
        (self.registers.dma_control & 0x20) != 0
    }

    #[inline]
    pub(crate) fn ccdma_type(&self) -> Option<u8> {
        if !self.ccdma_enabled() {
            None
        } else if (self.registers.dma_control & 0x10) != 0 {
            Some(1)
        } else {
            Some(0)
        }
    }

    #[inline]
    #[allow(dead_code)]
    pub(crate) fn dma_source_device(&self) -> u8 {
        self.registers.dma_control & 0x03
    }

    #[inline]
    #[allow(dead_code)]
    pub(crate) fn dma_dest_device(&self) -> u8 {
        (self.registers.dma_control >> 2) & 0x01
    }

    #[inline]
    pub(crate) fn ccdma_color_code(&self) -> Option<u8> {
        match self.registers.ccdma_control & 0x03 {
            0 => Some(0),
            1 => Some(1),
            2 => Some(2),
            3 => Some(1),
            _ => None,
        }
    }

    #[inline]
    pub(crate) fn ccdma_color_depth_bits(&self) -> Option<u8> {
        self.ccdma_color_code().map(|code| match code {
            0 => 8,
            1 => 4,
            2 => 2,
            _ => unreachable!(),
        })
    }

    #[inline]
    #[allow(dead_code)]
    pub(crate) fn ccdma_dmacb(&self) -> Option<u8> {
        self.ccdma_color_code().map(|code| match code {
            0 => 0,
            1 => 1,
            2 => 2,
            _ => unreachable!(),
        })
    }

    #[inline]
    pub(crate) fn ccdma_virtual_width_shift(&self) -> u8 {
        (self.registers.ccdma_control >> 2) & 0x07
    }

    #[inline]
    #[allow(dead_code)]
    pub(crate) fn ccdma_chars_per_line(&self) -> usize {
        1usize << (self.ccdma_virtual_width_shift().min(5) as usize)
    }

    #[inline]
    fn dma_is_normal(&self) -> bool {
        (self.registers.dma_control & 0x20) == 0
    }

    #[inline]
    fn is_ccdma_terminated(&self) -> bool {
        // Bit 7 of CCDMA control ($2231) is the DMA enable bit, not terminate
        // When bit 7 is set, CC-DMA should be enabled, not terminated
        // The bit is cleared by hardware when CC-DMA completes
        false // Never consider CC-DMA as "terminated" based on bit 7
    }

    pub(crate) fn reset_ccdma_state(&mut self) {
        self.registers.ccdma_pending = false;
        self.registers.ccdma_buffer_ready = false;
        self.registers.handshake_state = 0;
    }

    fn maybe_queue_ccdma(&mut self, reason: &str) {
        if !self.ccdma_enabled() {
            return;
        }
        if self.is_ccdma_terminated() {
            return;
        }
        if self.registers.ccdma_pending {
            return;
        }
        if self.registers.dma_length == 0 {
            return;
        }
        self.registers.ccdma_pending = true;
        self.registers.ccdma_buffer_ready = false;
        self.registers.handshake_state = 1;
        if crate::debug_flags::trace_sa1_ccdma() {
            self.log_ccdma_state(&format!("queue:{}", reason));
        }
    }

    pub fn is_dma_pending(&self) -> bool {
        self.registers.dma_pending
    }

    pub fn is_ccdma_pending(&self) -> bool {
        self.registers.ccdma_pending
    }

    pub(crate) fn log_ccdma_state(&self, reason: &str) {
        if !crate::debug_flags::trace_sa1_ccdma() {
            return;
        }
        use std::sync::atomic::{AtomicU32, Ordering};
        static TRACE_IDX: AtomicU32 = AtomicU32::new(0);
        let idx = TRACE_IDX.fetch_add(1, Ordering::Relaxed) + 1;
        println!(
            "TRACE_SA1_CCDMA[{}] {} ctrl=0x{:02X} cctrl=0x{:02X} src=0x{:06X} dest=0x{:06X} len=0x{:04X} buf_ready={} pending={} handshake={}",
            idx,
            reason,
            self.registers.dma_control,
            self.registers.ccdma_control,
            self.registers.dma_source,
            self.registers.dma_dest,
            self.registers.dma_length,
            self.registers.ccdma_buffer_ready as u8,
            self.registers.ccdma_pending as u8,
            self.registers.handshake_state
        );
    }

    #[allow(dead_code)]
    pub fn pending_scpu_irq_mask(&self) -> u8 {
        self.registers.interrupt_pending & self.registers.interrupt_enable
    }

    #[allow(dead_code)]
    pub fn tick_timers(&mut self, sa1_cycles: u32) {
        if sa1_cycles == 0 {
            return;
        }

        let mut h_triggers = 0u32;
        if (self.registers.tmcnt & 0x01) != 0 && self.registers.h_timer != 0 {
            let period = self.registers.h_timer as u32;
            self.h_timer_accum = self.h_timer_accum.saturating_add(sa1_cycles);
            while self.h_timer_accum >= period {
                self.h_timer_accum -= period;
                h_triggers = h_triggers.saturating_add(1);
                self.registers.timer_pending |= 0x01;
                self.registers.interrupt_pending |= 0x01;
                self.registers.ctr = self.registers.ctr.wrapping_add(1);
            }
        }

        if (self.registers.tmcnt & 0x02) != 0 && self.registers.v_timer != 0 && h_triggers > 0 {
            let period = self.registers.v_timer as u32;
            self.v_timer_accum = self.v_timer_accum.saturating_add(h_triggers);
            while self.v_timer_accum >= period {
                self.v_timer_accum -= period;
                self.registers.timer_pending |= 0x02;
                self.registers.interrupt_pending |= 0x02;
            }
        }
    }

    pub fn complete_dma(&mut self) -> bool {
        self.registers.dma_pending = false;
        self.registers.dma_control &= !0x80;
        let irq_enabled = (self.registers.interrupt_enable & Self::IRQ_DMA_BIT) != 0;
        if crate::debug_flags::trace_sa1_dma() {
            println!(
                "TRACE_SA1_DMA: complete irq_enabled={} ctrl=0x{:02X} enable=0x{:02X}",
                irq_enabled, self.registers.dma_control, self.registers.interrupt_enable
            );
        }
        if irq_enabled {
            self.registers.interrupt_pending |= Self::IRQ_DMA_BIT;
            true
        } else {
            false
        }
    }

    pub fn complete_ccdma(&mut self) -> bool {
        self.registers.ccdma_pending = false;
        self.registers.sfr |= 0x20;
        self.registers.ccdma_control &= !0x80;
        let irq_enabled = (self.registers.interrupt_enable & Self::IRQ_CCDMA_BIT) != 0;
        if crate::debug_flags::trace_sa1_dma() {
            println!(
                "TRACE_SA1_DMA: CC-DMA complete irq_enabled={} cctrl=0x{:02X} enable=0x{:02X}",
                irq_enabled, self.registers.ccdma_control, self.registers.interrupt_enable
            );
        }
        if irq_enabled {
            self.registers.interrupt_pending |= Self::IRQ_CCDMA_BIT;
            true
        } else {
            false
        }
    }

    fn clear_scpu_irq_pending(&mut self, mask: u8) {
        let before = self.registers.interrupt_pending;
        self.registers.interrupt_pending &= !mask;
        if before != self.registers.interrupt_pending
            && mask & (Self::IRQ_DMA_BIT | Self::IRQ_CCDMA_BIT) != 0
        {
            self.registers.handshake_state = 0;
        }
    }

    fn write_sie(&mut self, value: u8) {
        self.registers.sie = value;
        self.update_interrupt_mask();
    }

    fn write_cie(&mut self, value: u8) {
        self.registers.cie = value;
        self.update_interrupt_mask();
    }

    fn write_cfr(&mut self, value: u8) {
        self.registers.tmcnt = value;
        if (value & 0x80) != 0 {
            self.h_timer_accum = 0;
            self.v_timer_accum = 0;
            self.registers.timer_pending = 0;
        }
    }

    pub fn read_register(&mut self, offset: u16) -> u8 {
        match offset {
            0x00 => self.registers.control,
            0x01 => self.registers.sie,
            0x02 => self.registers.interrupt_pending,
            0x03 => (self.registers.reset_vector & 0xFF) as u8,
            0x04 => (self.registers.reset_vector >> 8) as u8,
            0x05 => (self.registers.nmi_vector & 0xFF) as u8,
            0x06 => (self.registers.nmi_vector >> 8) as u8,
            0x07 => (self.registers.irq_vector & 0xFF) as u8,
            0x08 => (self.registers.irq_vector >> 8) as u8,
            0x09 => self.registers.scnt,
            0x0A => self.registers.cie,
            0x0B => self.registers.cic,
            0x0C => (self.registers.snv & 0xFF) as u8,
            0x0D => (self.registers.snv >> 8) as u8,
            0x0E => (self.registers.siv & 0xFF) as u8,
            0x0F => (self.registers.siv >> 8) as u8,
            0x10 => self.registers.tmcnt,
            0x11 => self.registers.ctr,
            0x12 => (self.registers.h_timer & 0xFF) as u8,
            0x13 => (self.registers.h_timer >> 8) as u8,
            0x14 => (self.registers.v_timer & 0xFF) as u8,
            0x15 => (self.registers.v_timer >> 8) as u8,
            0x24 => self.registers.bwram_select_snes,
            0x25 => self.registers.bwram_select_sa1,
            0x26 => self.registers.sbwe,
            0x27 => self.registers.cbwe,
            0x28 => self.registers.bwram_protect,
            0x29 => self.registers.iram_wp_snes,
            0x2A => self.registers.iram_wp_sa1,
            0x30 => self.registers.dma_control,
            0x31 => self.registers.ccdma_control,
            0x32 => (self.registers.dma_source & 0xFF) as u8,
            0x33 => ((self.registers.dma_source >> 8) & 0xFF) as u8,
            0x34 => ((self.registers.dma_source >> 16) & 0xFF) as u8,
            0x35 => (self.registers.dma_dest & 0xFF) as u8,
            0x36 => ((self.registers.dma_dest >> 8) & 0xFF) as u8,
            0x37 => ((self.registers.dma_dest >> 16) & 0xFF) as u8,
            0x38 => (self.registers.dma_length & 0xFF) as u8,
            0x39 => (self.registers.dma_length >> 8) as u8,
            0x100 => self.registers.sfr,
            0x10E => self.registers.timer_pending,
            _ => 0,
        }
    }

    pub fn write_register(&mut self, offset: u16, value: u8) {
        match offset {
            0x00 => {
                self.registers.control = value;
                if std::env::var_os("TRACE_SA1_BOOT").is_some()
                    || std::env::var_os("DEBUG_SA1_SCHEDULER").is_some()
                {
                    println!(
                        "SA-1 $2200 write: control=0x{:02X} (SA1_EN={} IRQ_EN={})",
                        value,
                        value & 0x80,
                        value & 0x01
                    );
                }
            }
            0x01 => self.write_sie(value),
            0x02 => {
                self.registers.sic = value;
                self.clear_scpu_irq_pending(value);
            }
            0x03 => {
                self.registers.reset_vector =
                    (self.registers.reset_vector & 0xFF00) | (value as u16);
                self.boot_vector_applied = false;
                if std::env::var_os("TRACE_SA1_BOOT").is_some()
                    || std::env::var_os("DEBUG_SA1_SCHEDULER").is_some()
                {
                    println!(
                        "SA-1 $2203 write: reset_vector low=0x{:02X}, full=0x{:04X}",
                        value, self.registers.reset_vector
                    );
                }
            }
            0x04 => {
                self.registers.reset_vector =
                    (self.registers.reset_vector & 0x00FF) | ((value as u16) << 8);
                self.boot_vector_applied = false;
                if std::env::var_os("TRACE_SA1_BOOT").is_some()
                    || std::env::var_os("DEBUG_SA1_SCHEDULER").is_some()
                {
                    println!(
                        "SA-1 $2204 write: reset_vector high=0x{:02X}, full=0x{:04X}",
                        value, self.registers.reset_vector
                    );
                }
            }
            0x05 => {
                self.registers.nmi_vector = (self.registers.nmi_vector & 0xFF00) | (value as u16);
            }
            0x06 => {
                self.registers.nmi_vector =
                    (self.registers.nmi_vector & 0x00FF) | ((value as u16) << 8);
            }
            0x07 => {
                self.registers.irq_vector = (self.registers.irq_vector & 0xFF00) | (value as u16);
            }
            0x08 => {
                self.registers.irq_vector =
                    (self.registers.irq_vector & 0x00FF) | ((value as u16) << 8);
            }
            0x09 => self.registers.scnt = value,
            0x0A => self.write_cie(value),
            0x0B => {
                self.registers.cic = value;
                self.clear_scpu_irq_pending(value);
            }
            0x0C => self.registers.snv = (self.registers.snv & 0xFF00) | (value as u16),
            0x0D => self.registers.snv = (self.registers.snv & 0x00FF) | ((value as u16) << 8),
            0x0E => self.registers.siv = (self.registers.siv & 0xFF00) | (value as u16),
            0x0F => self.registers.siv = (self.registers.siv & 0x00FF) | ((value as u16) << 8),
            0x10 => self.write_cfr(value),
            0x11 => self.registers.ctr = value,
            0x12 => self.registers.h_timer = (self.registers.h_timer & 0xFF00) | (value as u16),
            0x13 => {
                self.registers.h_timer = (self.registers.h_timer & 0x00FF) | ((value as u16) << 8)
            }
            0x14 => self.registers.v_timer = (self.registers.v_timer & 0xFF00) | (value as u16),
            0x15 => {
                self.registers.v_timer = (self.registers.v_timer & 0x00FF) | ((value as u16) << 8)
            }
            0x24 => self.registers.bwram_select_snes = value & 0x1F,
            0x25 => {
                let masked = if (value & 0x80) != 0 {
                    // Bit 7 selects the virtual bitmap window; keep bits 0-6.
                    0x80 | (value & 0x7F)
                } else {
                    value & 0x1F
                };
                self.registers.bwram_select_sa1 = masked;
                if std::env::var_os("TRACE_SA1_BWRAM_GUARD").is_some() {
                    println!(
                        "📝 SA-1 $2225 write: value=0x{:02X} (masked=0x{:02X})",
                        value, masked
                    );
                }
            }
            0x26 => {
                self.registers.sbwe = value & 0x80;
                if std::env::var_os("TRACE_SA1_BWRAM_GUARD").is_some() {
                    println!("📝 SA-1 $2226 write: SBWE=0x{:02X}", self.registers.sbwe);
                }
            }
            0x27 => {
                self.registers.cbwe = value & 0x80;
                if std::env::var_os("TRACE_SA1_BWRAM_GUARD").is_some() {
                    println!("📝 SA-1 $2227 write: CBWE=0x{:02X}", self.registers.cbwe);
                }
            }
            0x28 => {
                self.registers.bwram_protect = value & 0x0F;
                if std::env::var_os("TRACE_SA1_BWRAM_GUARD").is_some() {
                    println!(
                        "📝 SA-1 $2228 write: BWPA=0x{:02X}",
                        self.registers.bwram_protect
                    );
                }
            }
            0x29 => {
                self.registers.iram_wp_snes = value;
                if std::env::var_os("TRACE_SA1_IRAM_GUARD").is_some() {
                    println!("📝 SA-1 $2229 write: SIWP=0x{:02X}", value);
                }
            }
            0x2A => {
                self.registers.iram_wp_sa1 = value;
                if std::env::var_os("TRACE_SA1_IRAM_GUARD").is_some() {
                    println!("📝 SA-1 $222A write: CIWP=0x{:02X}", value);
                }
            }
            0x30 => {
                let previous = self.registers.dma_control;
                self.registers.dma_control = value;
                if self.dma_is_normal() {
                    self.registers.dma_pending = (value & 0x80) != 0;
                    self.reset_ccdma_state();
                } else {
                    self.registers.dma_pending = false;
                    if self.ccdma_type() == Some(0) {
                        println!("⚠️ SA-1 CC-DMA type 2 not fully modelled");
                    }
                    if self.is_ccdma_terminated() {
                        self.reset_ccdma_state();
                    } else if (value & 0x80) != 0 || (previous & 0x20) == 0 {
                        self.maybe_queue_ccdma("dcnt");
                    }
                }
                if crate::debug_flags::trace_sa1_dma() {
                    println!(
                        "TRACE_SA1_DMA: $2230 write ctrl=0x{:02X}→0x{:02X} pending={} type={} src=0x{:06X} dest=0x{:06X} len=0x{:04X}",
                        previous,
                        self.registers.dma_control,
                        self.registers.dma_pending,
                        if self.dma_is_normal() { "normal" } else { "cc" },
                        self.registers.dma_source,
                        self.registers.dma_dest,
                        self.registers.dma_length
                    );
                }
            }
            0x31 => {
                let previous = self.registers.ccdma_control;
                self.registers.ccdma_control = value;
                let terminate = (value & 0x80) != 0;
                if self.ccdma_enabled() {
                    if terminate {
                        if previous & 0x80 == 0 && !crate::debug_flags::quiet() {
                            println!("🛑 SA-1 CC-DMA terminate request");
                        }
                        self.reset_ccdma_state();
                        self.registers.sfr &= !0x20;
                        self.registers.interrupt_pending &= !Self::IRQ_CCDMA_BIT;
                    } else {
                        self.maybe_queue_ccdma("ccdma_ctrl");
                    }
                } else if terminate {
                    self.reset_ccdma_state();
                }
                if crate::debug_flags::trace_sa1_dma() {
                    println!(
                        "TRACE_SA1_DMA: $2231 write ctrl=0x{:02X}→0x{:02X} pending={} terminate={} buf_ready={} len=0x{:04X}",
                        previous,
                        self.registers.ccdma_control,
                        self.registers.ccdma_pending,
                        terminate,
                        self.registers.ccdma_buffer_ready,
                        self.registers.dma_length
                    );
                }
            }
            0x32 => {
                self.registers.dma_source = (self.registers.dma_source & 0xFFFF00) | (value as u32);
            }
            0x33 => {
                self.registers.dma_source =
                    (self.registers.dma_source & 0xFF00FF) | ((value as u32) << 8);
            }
            0x34 => {
                self.registers.dma_source =
                    (self.registers.dma_source & 0x00FFFF) | ((value as u32) << 16);
            }
            0x35 => {
                self.registers.dma_dest = (self.registers.dma_dest & 0xFFFF00) | (value as u32);
            }
            0x36 => {
                self.registers.dma_dest =
                    (self.registers.dma_dest & 0xFF00FF) | ((value as u32) << 8);
            }
            0x37 => {
                self.registers.dma_dest =
                    (self.registers.dma_dest & 0x00FFFF) | ((value as u32) << 16);
            }
            0x38 => {
                self.registers.dma_length = (self.registers.dma_length & 0xFF00) | (value as u16);
            }
            0x39 => {
                self.registers.dma_length =
                    (self.registers.dma_length & 0x00FF) | ((value as u16) << 8);
            }
            _ => {}
        }
    }
}
