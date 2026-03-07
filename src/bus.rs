use crate::apu::{Apu, ApuState};
use crate::cartridge::{Cartridge, CartridgeState};
use crate::cpu::CpuBus;
use crate::memory::Memory;
use crate::ppu::Ppu;

pub struct Bus {
    memory: Memory,
    ppu: Ppu,
    apu: Apu,
    cartridge: Option<Cartridge>,
    pub controller: u8,
    controller_state: u16,
    strobe: bool,          // Controller strobe mode
    dma_cycles: u32,       // Cycles to add due to DMA operations
    dma_in_progress: bool, // Flag to indicate DMA is in progress
    dmc_stall_cycles: u32,
}

impl Bus {
    pub fn new() -> Self {
        Bus {
            memory: Memory::new(),
            ppu: Ppu::new(),
            apu: Apu::new(),
            cartridge: None,
            controller: 0,
            controller_state: 0,
            strobe: false,
            dma_cycles: 0,
            dma_in_progress: false,
            dmc_stall_cycles: 0,
        }
    }

    pub fn load_cartridge(&mut self, cartridge: Cartridge) {
        self.cartridge = Some(cartridge);
    }

    fn service_dmc_sample(&mut self) {
        if let Some((addr, stall_cycles)) = self.apu.pull_dmc_sample_request() {
            let data = self.read_dmc_sample(addr);
            self.apu.push_dmc_sample(data);
            self.dmc_stall_cycles += stall_cycles as u32;
        }
    }

    #[inline]
    pub fn step_ppu(&mut self) -> bool {
        let nmi = self.ppu.step(self.cartridge.as_ref());
        if self.ppu.mapper_irq_clock {
            self.ppu.mapper_irq_clock = false;
            if let Some(ref mut cartridge) = self.cartridge {
                cartridge.clock_irq_counter();
            }
        }
        nmi
    }

    // New tick method for fine-grained synchronization (similar to reference emulator)
    pub fn tick(&mut self, cycles: u8) -> bool {
        let mut nmi_triggered = false;

        // PPU runs 3x faster than CPU
        let ppu_cycles = cycles as u32 * 3;

        // Step PPU for the calculated cycles
        for _cycle in 0..ppu_cycles {
            let nmi = self.step_ppu();
            if nmi && !nmi_triggered {
                nmi_triggered = true;
            }
        }

        // Step APU at CPU rate (with expansion audio)
        for _ in 0..cycles {
            let exp = if let Some(ref mut cartridge) = self.cartridge {
                cartridge.clock_expansion_audio()
            } else {
                0.0
            };
            self.apu.set_expansion_audio(exp);
            self.service_dmc_sample();
            self.apu.step();
        }

        nmi_triggered
    }

    pub fn step_apu(&mut self) {
        let exp = if let Some(ref mut cartridge) = self.cartridge {
            cartridge.clock_expansion_audio()
        } else {
            0.0
        };
        self.apu.set_expansion_audio(exp);
        self.service_dmc_sample();
        self.apu.step();
    }

    pub fn set_controller(&mut self, controller: u8) {
        self.controller = controller;
    }

    fn read_controller(&mut self) -> u8 {
        if self.strobe {
            // While strobe is high, continuously reload and return bit 0 (A button)
            self.controller_state = self.controller as u16;
            return self.controller as u8 & 0x01;
        }
        let value = if self.controller_state & 0x01 != 0 {
            0x01
        } else {
            0x00
        };
        self.controller_state >>= 1;
        value
    }

    pub fn ppu_frame_complete(&mut self) -> bool {
        let complete = self.ppu.frame_complete;
        if complete {
            self.ppu.frame_complete = false;
        }
        complete
    }

    pub fn get_ppu_buffer(&self) -> &[u8] {
        self.ppu.get_buffer()
    }

    pub fn set_audio_ring(&mut self, ring: std::sync::Arc<crate::audio_ring::SpscRingBuffer>) {
        self.apu.set_audio_ring(ring);
    }

    pub fn get_audio_buffer(&mut self) -> Vec<f32> {
        self.apu.get_audio_buffer()
    }

    pub fn drain_audio_to_ring(&mut self, ring: &crate::audio_ring::SpscRingBuffer) {
        self.apu.drain_to_ring(ring);
    }

    pub fn audio_diag_full(&self) -> crate::apu::AudioDiagFull {
        self.apu.audio_diag_full()
    }

    // Check if APU frame IRQ is pending
    pub fn apu_irq_pending(&self) -> bool {
        self.apu.irq_pending()
    }

    // Clear APU frame IRQ
    pub fn clear_apu_irq(&mut self) {
        self.apu.clear_frame_irq();
    }

    pub fn mapper_irq_pending(&self) -> bool {
        if let Some(ref cartridge) = self.cartridge {
            cartridge.irq_pending()
        } else {
            false
        }
    }

    pub fn clock_mapper_irq(&mut self) {
        if let Some(ref mut cartridge) = self.cartridge {
            cartridge.clock_irq_counter();
        }
    }

    pub fn clock_mapper_irq_cycles(&mut self, cycles: u32) {
        if let Some(ref mut cartridge) = self.cartridge {
            cartridge.clock_irq_counter_cycles(cycles);
        }
    }

    pub fn get_dma_cycles(&mut self) -> u32 {
        let cycles = self.dma_cycles;
        self.dma_cycles = 0; // Reset after reading
        cycles
    }

    pub fn is_dma_in_progress(&self) -> bool {
        self.dma_in_progress
    }

    pub fn step_dma(&mut self) -> bool {
        if self.dma_in_progress {
            if self.dma_cycles > 0 {
                self.dma_cycles -= 1;
                if self.dma_cycles == 0 {
                    self.dma_in_progress = false;
                    return true; // DMA completed this cycle
                }
            }
        }
        false
    }

    pub fn take_dmc_stall_cycles(&mut self) -> u32 {
        std::mem::take(&mut self.dmc_stall_cycles)
    }

    pub fn timing_state(&self) -> (u32, bool, u32, bool) {
        (
            self.dma_cycles,
            self.dma_in_progress,
            self.dmc_stall_cycles,
            self.ppu.frame_complete,
        )
    }

    pub fn restore_timing_state(
        &mut self,
        dma_cycles: u32,
        dma_in_progress: bool,
        dmc_stall_cycles: u32,
        ppu_frame_complete: bool,
    ) {
        self.dma_cycles = dma_cycles;
        self.dma_in_progress = dma_in_progress;
        self.dmc_stall_cycles = dmc_stall_cycles;
        self.ppu.frame_complete = ppu_frame_complete;
    }

    pub fn read_chr(&self, addr: u16) -> u8 {
        if let Some(ref cartridge) = self.cartridge {
            cartridge.read_chr(addr)
        } else {
            0
        }
    }

    fn read_dmc_sample(&self, addr: u16) -> u8 {
        match addr {
            0x8000..=0xFFFF => {
                if let Some(ref cartridge) = self.cartridge {
                    cartridge.read_prg(addr)
                } else {
                    0
                }
            }
            _ => 0,
        }
    }
}

impl CpuBus for Bus {
    fn read(&mut self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x1FFF => self.memory.read(addr),
            0x2000..=0x3FFF => {
                let mirrored = 0x2000 + (addr & 0x07);
                self.ppu.read_register(mirrored, self.cartridge.as_ref())
            }
            0x4000..=0x4013 | 0x4015 => self.apu.read_register(addr),
            0x4016 => self.read_controller(),
            0x4017 => 0,
            0x6000..=0x7FFF => {
                if let Some(ref cartridge) = self.cartridge {
                    cartridge.read_prg_ram(addr)
                } else {
                    0
                }
            }
            0x8000..=0xFFFF => {
                if let Some(ref cartridge) = self.cartridge {
                    cartridge.read_prg(addr)
                } else {
                    0
                }
            }
            _ => 0,
        }
    }

    fn write(&mut self, addr: u16, data: u8) {
        match addr {
            0x0000..=0x1FFF => {
                self.memory.write(addr, data);
            }
            0x2000..=0x3FFF => {
                let mirrored = 0x2000 + (addr & 0x07);
                if let Some((chr_addr, chr_data)) =
                    self.ppu
                        .write_register(mirrored, data, self.cartridge.as_ref())
                {
                    if let Some(ref mut cartridge) = self.cartridge {
                        cartridge.write_chr(chr_addr, chr_data);
                    }
                }
            }
            0x4000..=0x4013 | 0x4015 | 0x4017 => {
                self.apu.write_register(addr, data);
            }
            0x4014 => {
                // OAM DMA: Copy 256 bytes from CPU page to PPU OAM
                let base = (data as u16) << 8;
                let oam_addr = self.ppu.get_oam_addr();
                for i in 0u16..256 {
                    let src = base + i;
                    let byte = match src {
                        0x0000..=0x1FFF => self.memory.read(src),
                        0x6000..=0x7FFF => {
                            if let Some(ref cartridge) = self.cartridge {
                                cartridge.read_prg_ram(src)
                            } else {
                                0
                            }
                        }
                        0x8000..=0xFFFF => {
                            if let Some(ref cartridge) = self.cartridge {
                                cartridge.read_prg(src)
                            } else {
                                0
                            }
                        }
                        _ => 0,
                    };
                    let oam_dst = oam_addr.wrapping_add(i as u8);
                    self.ppu.write_oam_data(oam_dst, byte);
                }
                self.dma_in_progress = true;
                self.dma_cycles = 513;
            }
            0x4016 => {
                // Controller strobe
                let new_strobe = (data & 0x01) != 0;
                if self.strobe && !new_strobe {
                    // Falling edge: latch controller state
                    self.controller_state = self.controller as u16;
                }
                self.strobe = new_strobe;
            }
            0x4020..=0xFFFF => {
                if let Some(ref mut cartridge) = self.cartridge {
                    match addr {
                        0x4020..=0x5FFF => {
                            cartridge.write_prg(addr, data);
                        }
                        0x6000..=0x7FFF => {
                            cartridge.write_prg_ram(addr, data);
                        }
                        0x8000..=0xFFFF => {
                            cartridge.write_prg(addr, data);
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
}

// Additional methods for save/load state
impl Bus {
    pub fn get_sram_data(&self) -> Option<Vec<u8>> {
        if let Some(ref cartridge) = self.cartridge {
            cartridge.get_sram_data().map(|data| data.to_vec())
        } else {
            None
        }
    }

    pub fn get_ppu_state(&self) -> (u8, u8, u8, u8) {
        (
            self.ppu.get_control_bits(),
            self.ppu.get_mask_bits(),
            self.ppu.get_status_bits(),
            self.ppu.get_oam_addr(),
        )
    }

    pub fn get_ppu_registers(&self) -> (u16, u16, u8, bool, i16, u16, u64, u8) {
        (
            self.ppu.get_vram_addr(),
            self.ppu.get_t(),
            self.ppu.get_x_scroll(),
            self.ppu.get_w(),
            self.ppu.get_scanline(),
            self.ppu.get_cycle(),
            self.ppu.get_frame(),
            self.ppu.get_read_buffer(),
        )
    }

    pub fn get_ppu_palette(&self) -> [u8; 32] {
        self.ppu.get_palette()
    }

    pub fn get_ppu_nametables_flat(&self) -> Vec<u8> {
        let nt = self.ppu.get_nametable();
        let mut data = Vec::with_capacity(2048);
        data.extend_from_slice(&nt[0]);
        data.extend_from_slice(&nt[1]);
        data
    }

    pub fn get_ppu_oam_flat(&self) -> Vec<u8> {
        self.ppu.get_oam().to_vec()
    }

    pub fn get_ram_flat(&self) -> Vec<u8> {
        self.memory.get_ram().to_vec()
    }

    pub fn get_cartridge_prg_bank(&self) -> u8 {
        if let Some(ref cartridge) = self.cartridge {
            cartridge.get_prg_bank()
        } else {
            0
        }
    }

    pub fn get_cartridge_chr_bank(&self) -> u8 {
        if let Some(ref cartridge) = self.cartridge {
            cartridge.get_chr_bank()
        } else {
            0
        }
    }

    pub fn get_cartridge_state(&self) -> Option<CartridgeState> {
        self.cartridge.as_ref().map(|c| c.snapshot_state())
    }

    pub fn get_apu_state(&self) -> ApuState {
        self.apu.snapshot_state()
    }

    pub fn restore_cartridge_state(&mut self, state: &CartridgeState) {
        if let Some(ref mut cartridge) = self.cartridge {
            cartridge.restore_state(state);
        }
    }

    pub fn restore_apu_state(&mut self, state: &ApuState) {
        self.apu.restore_state(state);
        self.dmc_stall_cycles = 0;
    }

    pub fn restore_legacy_apu_state(&mut self, frame_counter: u8, frame_irq: bool) {
        self.apu.restore_legacy_state(frame_counter, frame_irq);
        self.dmc_stall_cycles = 0;
    }

    pub fn restore_state_flat(
        &mut self,
        palette: impl AsRef<[u8]>,
        nametables: impl AsRef<[u8]>,
        oam: impl AsRef<[u8]>,
        ram: impl AsRef<[u8]>,
        prg_bank: u8,
        chr_bank: u8,
        ppu_regs: Option<(u8, u8, u8, u8, u16, u16, u8, bool, i16, u16, u64, u8)>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let ram = ram.as_ref();
        let palette = palette.as_ref();
        let nametables = nametables.as_ref();
        let oam = oam.as_ref();

        // Restore RAM
        if ram.len() >= 0x800 {
            let mut ram_array = [0u8; 0x800];
            ram_array.copy_from_slice(&ram[..0x800]);
            self.memory.set_ram(ram_array);
        }

        // Restore PPU palette
        if palette.len() >= 32 {
            let mut pal = [0u8; 32];
            pal.copy_from_slice(&palette[..32]);
            self.ppu.set_palette(pal);
        }

        // Restore PPU nametables
        if nametables.len() >= 2048 {
            let mut nt = [[0u8; 1024]; 2];
            nt[0].copy_from_slice(&nametables[..1024]);
            nt[1].copy_from_slice(&nametables[1024..2048]);
            self.ppu.set_nametable(nt);
        }

        // Restore PPU OAM
        if oam.len() >= 256 {
            let mut oam_array = [0u8; 256];
            oam_array.copy_from_slice(&oam[..256]);
            self.ppu.set_oam(oam_array);
        }

        // Restore PPU registers
        if let Some((
            control,
            mask,
            status,
            oam_addr,
            v,
            t,
            x,
            w,
            scanline,
            cycle,
            frame,
            read_buf,
        )) = ppu_regs
        {
            self.ppu.restore_registers(
                control, mask, status, oam_addr, v, t, x, w, scanline, cycle, frame, read_buf,
            );
        }

        // Restore cartridge bank state
        if let Some(ref mut cartridge) = self.cartridge {
            cartridge.set_prg_bank(prg_bank);
            cartridge.set_chr_bank(chr_bank);
        }

        Ok(())
    }

    pub fn read_cartridge_address(&self, addr: u16) -> u8 {
        if let Some(ref cartridge) = self.cartridge {
            cartridge.read_prg(addr)
        } else {
            0
        }
    }

    /// Direct reference to CPU RAM (2KB).
    pub fn ram_ref(&self) -> &[u8] {
        &self.memory.ram
    }

    /// Mutable reference to CPU RAM (2KB).
    pub fn ram_mut(&mut self) -> &mut [u8] {
        &mut self.memory.ram
    }

    /// Direct reference to PRG-RAM / SRAM (mapper-dependent).
    pub fn prg_ram_ref(&self) -> Option<&[u8]> {
        self.cartridge.as_ref().and_then(|c| c.prg_ram_ref())
    }

    /// Mutable reference to PRG-RAM / SRAM.
    pub fn prg_ram_mut(&mut self) -> Option<&mut [u8]> {
        self.cartridge.as_mut().and_then(|c| c.prg_ram_mut())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dmc_sample_fetch_schedules_cpu_stall_cycles() {
        let mut bus = Bus::new();
        bus.apu.write_register(0x4010, 0x0F);
        bus.apu.write_register(0x4012, 0x00);
        bus.apu.write_register(0x4013, 0x00);
        bus.apu.write_register(0x4015, 0x10);

        bus.step_apu();
        assert_eq!(bus.take_dmc_stall_cycles(), 0);
        bus.step_apu();
        assert_eq!(bus.take_dmc_stall_cycles(), 0);
        bus.step_apu();

        assert_eq!(bus.take_dmc_stall_cycles(), 3);
        assert_eq!(bus.take_dmc_stall_cycles(), 0);
    }

    #[test]
    fn restore_timing_state_restores_dma_and_frame_flags() {
        let mut bus = Bus::new();
        bus.restore_timing_state(7, true, 2, true);

        assert!(bus.is_dma_in_progress());
        assert_eq!(bus.take_dmc_stall_cycles(), 2);
        assert!(bus.ppu_frame_complete());
        assert!(!bus.ppu_frame_complete());

        assert!(!bus.step_dma());
        let (dma_cycles, dma_in_progress, dmc_stall_cycles, ppu_frame_complete) =
            bus.timing_state();
        assert_eq!(dma_cycles, 6);
        assert!(dma_in_progress);
        assert_eq!(dmc_stall_cycles, 0);
        assert!(!ppu_frame_complete);
    }
}
