pub mod apu;
pub mod audio_ring;
pub mod bus;
pub mod cartridge;
pub mod cheat;
pub mod cpu;
pub mod hud_toast;
pub mod memory;
pub mod ppu;
pub mod save_state;
pub mod sram;

pub use bus::Bus;
pub use cartridge::Cartridge;
pub use cpu::Cpu;
pub use cpu::StatusFlags;

pub const CPU_CYCLES_PER_FRAME: u32 = 29830;

pub struct Nes {
    cpu: Cpu,
    bus: Bus,
    current_rom_path: Option<String>,
}

impl Nes {
    pub fn new() -> Self {
        Nes {
            cpu: Cpu::new(),
            bus: Bus::new(),
            current_rom_path: None,
        }
    }

    pub fn load_rom(&mut self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut cartridge = Cartridge::load(path)?;

        // Load SRAM data if exists
        if cartridge.has_battery_save() {
            if let Ok(Some(sram_data)) = sram::load_sram(path) {
                cartridge.set_sram_data(sram_data);
            }
        }

        self.bus.load_cartridge(cartridge);
        self.cpu.reset(&mut self.bus);
        self.current_rom_path = Some(path.to_string());
        Ok(())
    }

    pub fn save_sram(&self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(ref rom_path) = self.current_rom_path {
            if let Some(sram_data) = self.bus.get_sram_data() {
                sram::save_sram(rom_path, &sram_data)?;
                println!("SRAM saved successfully");
            }
        }
        Ok(())
    }

    pub fn step(&mut self) -> bool {
        let cpu_cycles: u32;

        // If DMA is in progress, don't execute CPU instruction
        if self.bus.is_dma_in_progress() {
            let dma_completed = self.bus.step_dma();
            cpu_cycles = 1;
            if dma_completed {
                // DMA completed
            }
        } else {
            // Normal CPU execution
            let cycles = self.cpu.step(&mut self.bus);

            // Safety check for zero cycles
            if cycles == 0 {
                return false;
            }

            cpu_cycles = cycles as u32;
        }

        // --- Run all components for CPU instruction cycles ---
        let mut nmi_triggered = false;
        for _cycle in 0..(cpu_cycles * 3) {
            if self.bus.step_ppu() {
                nmi_triggered = true;
            }
        }
        self.bus.clock_mapper_irq_cycles(cpu_cycles);
        for _ in 0..cpu_cycles {
            self.bus.step_apu();
        }

        // --- Handle NMI (7-cycle entry must advance all components) ---
        if nmi_triggered {
            let nmi_cycles = self.cpu.nmi(&mut self.bus) as u32;
            for _ in 0..(nmi_cycles * 3) {
                self.bus.step_ppu();
            }
            self.bus.clock_mapper_irq_cycles(nmi_cycles);
            for _ in 0..nmi_cycles {
                self.bus.step_apu();
            }
        }

        // --- Handle APU Frame IRQ ---
        // cpu.irq() is silently ignored if I flag is set (returns 0 cycles).
        if self.bus.apu_irq_pending() {
            let irq_cycles = self.cpu.irq(&mut self.bus) as u32;
            if irq_cycles > 0 {
                for _ in 0..(irq_cycles * 3) {
                    self.bus.step_ppu();
                }
                self.bus.clock_mapper_irq_cycles(irq_cycles);
                for _ in 0..irq_cycles {
                    self.bus.step_apu();
                }
            }
        }

        // --- Handle mapper IRQ (MMC3 scanline counter, FME-7 cycle counter) ---
        if self.bus.mapper_irq_pending() {
            let irq_cycles = self.cpu.irq(&mut self.bus) as u32;
            if irq_cycles > 0 {
                for _ in 0..(irq_cycles * 3) {
                    self.bus.step_ppu();
                }
                self.bus.clock_mapper_irq_cycles(irq_cycles);
                for _ in 0..irq_cycles {
                    self.bus.step_apu();
                }
            }
        }

        // Use PPU frame completion as the authoritative frame boundary
        self.bus.ppu_frame_complete()
    }

    pub fn get_frame_buffer(&self) -> &[u8] {
        self.bus.get_ppu_buffer()
    }

    /// Attach a ring buffer so the APU pushes samples directly as they
    /// are generated (no batching, no intermediate Vec).
    pub fn set_audio_ring(&mut self, ring: std::sync::Arc<audio_ring::SpscRingBuffer>) {
        self.bus.set_audio_ring(ring);
    }

    pub fn get_audio_buffer(&mut self) -> Vec<f32> {
        self.bus.get_audio_buffer()
    }

    /// Push accumulated audio samples directly into the ring buffer,
    /// avoiding intermediate Vec allocation.
    pub fn drain_audio_to_ring(&mut self, ring: &audio_ring::SpscRingBuffer) {
        self.bus.drain_audio_to_ring(ring);
    }

    pub fn set_controller(&mut self, controller: u8) {
        self.bus.set_controller(controller);
    }

    /// Derive a filesystem-safe ROM stem from the loaded ROM path.
    fn rom_stem(&self) -> String {
        self.current_rom_path
            .as_deref()
            .and_then(|p| std::path::Path::new(p).file_stem())
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string()
    }

    pub fn save_state(
        &self,
        slot: u8,
        _rom_filename: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let (ppu_control, ppu_mask, ppu_status, ppu_oam_addr) = self.bus.get_ppu_state();
        let (ppu_v, ppu_t, ppu_x, ppu_w, ppu_scanline, ppu_cycle, ppu_frame, ppu_data_buffer) =
            self.bus.get_ppu_registers();

        let rom_stem = self.rom_stem();

        let save_state = save_state::SaveState {
            cpu_a: self.cpu.a,
            cpu_x: self.cpu.x,
            cpu_y: self.cpu.y,
            cpu_pc: self.cpu.pc,
            cpu_sp: self.cpu.sp,
            cpu_status: self.cpu.status.bits(),
            cpu_cycles: 0,
            ppu_control,
            ppu_mask,
            ppu_status,
            ppu_oam_addr,
            ppu_scroll_x: 0,
            ppu_scroll_y: 0,
            ppu_addr: ppu_v,
            ppu_data_buffer,
            ppu_w,
            ppu_t,
            ppu_v,
            ppu_x,
            ppu_scanline,
            ppu_cycle,
            ppu_frame,
            ppu_palette: self.bus.get_ppu_palette(),
            ppu_nametable: self.bus.get_ppu_nametables_flat(),
            ppu_oam: self.bus.get_ppu_oam_flat(),
            ram: self.bus.get_ram_flat(),
            cartridge_prg_bank: self.bus.get_cartridge_prg_bank(),
            cartridge_chr_bank: self.bus.get_cartridge_chr_bank(),
            cartridge_state: self.bus.get_cartridge_state(),
            apu_frame_counter: 0,
            apu_frame_interrupt: false,
            rom_filename: rom_stem.clone(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs(),
        };

        let dir = std::path::Path::new("states");
        if !dir.exists() {
            std::fs::create_dir_all(dir)?;
        }
        let filename = format!("states/{}.slot{}.sav", rom_stem, slot);
        save_state.save_to_file(&filename)?;
        Ok(())
    }

    pub fn load_state(&mut self, slot: u8) -> Result<(), Box<dyn std::error::Error>> {
        let rom_stem = self.rom_stem();
        let filename = format!("states/{}.slot{}.sav", rom_stem, slot);
        let save_state = save_state::SaveState::load_from_file(&filename)?;

        self.cpu.a = save_state.cpu_a;
        self.cpu.x = save_state.cpu_x;
        self.cpu.y = save_state.cpu_y;
        self.cpu.pc = save_state.cpu_pc;
        self.cpu.sp = save_state.cpu_sp;
        self.cpu.status = StatusFlags::from_bits_truncate(save_state.cpu_status);

        self.bus.restore_state_flat(
            save_state.ppu_palette,
            save_state.ppu_nametable,
            save_state.ppu_oam,
            save_state.ram,
            save_state.cartridge_prg_bank,
            save_state.cartridge_chr_bank,
            Some((
                save_state.ppu_control,
                save_state.ppu_mask,
                save_state.ppu_status,
                save_state.ppu_oam_addr,
                save_state.ppu_v,
                save_state.ppu_t,
                save_state.ppu_x,
                save_state.ppu_w,
                save_state.ppu_scanline,
                save_state.ppu_cycle,
                save_state.ppu_frame,
                save_state.ppu_data_buffer,
            )),
        )?;
        if let Some(ref state) = save_state.cartridge_state {
            self.bus.restore_cartridge_state(state);
        }

        Ok(())
    }

    pub fn get_controller(&self) -> u8 {
        self.bus.controller
    }

    /// Direct reference to CPU RAM (2KB).
    pub fn ram(&self) -> &[u8] {
        self.bus.ram_ref()
    }

    /// Mutable reference to CPU RAM (2KB).
    pub fn ram_mut(&mut self) -> &mut [u8] {
        self.bus.ram_mut()
    }

    /// Direct reference to PRG-RAM / SRAM (mapper-dependent, may be None).
    pub fn prg_ram(&self) -> Option<&[u8]> {
        self.bus.prg_ram_ref()
    }

    /// Mutable reference to PRG-RAM / SRAM.
    pub fn prg_ram_mut(&mut self) -> Option<&mut [u8]> {
        self.bus.prg_ram_mut()
    }
}
