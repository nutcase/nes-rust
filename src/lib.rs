pub mod cpu;
pub mod ppu;
pub mod apu;
pub mod memory;
pub mod cartridge;
pub mod bus;
pub mod save_state;
pub mod sram;
pub mod cheat;

pub use cpu::Cpu;
pub use cpu::StatusFlags;
pub use bus::Bus;
pub use cartridge::Cartridge;

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
        let total_cycles: u32;

        // If DMA is in progress, don't execute CPU instruction
        if self.bus.is_dma_in_progress() {
            let dma_completed = self.bus.step_dma();
            total_cycles = 1;
            if dma_completed {
                // DMA completed
            }
        } else {
            // Normal CPU execution
            let cpu_cycles = self.cpu.step(&mut self.bus);

            // Safety check for zero cycles
            if cpu_cycles == 0 {
                return false;
            }

            total_cycles = cpu_cycles as u32;
        }

        let mut nmi_triggered = false;
        let mut _nmi_count = 0;
        let ppu_cycles = total_cycles * 3;

        // Process all PPU cycles
        for _cycle in 0..ppu_cycles {
            let nmi = self.bus.step_ppu();
            if nmi {
                nmi_triggered = true;
                _nmi_count += 1;
            }
        }

        // Only process one NMI per CPU instruction (prevent double NMI)
        if nmi_triggered {
            self.cpu.nmi(&mut self.bus);
        }

        // Check for APU Frame IRQ
        // Don't clear frame_irq here - let the game acknowledge it via $4015 read.
        // On real hardware, the IRQ line stays asserted until acknowledged.
        // cpu.irq() will be silently ignored if the I flag is set (normal behavior).
        if self.bus.apu_irq_pending() {
            self.cpu.irq(&mut self.bus);
        }


        // APU runs at CPU clock rate
        for _ in 0..total_cycles {
            self.bus.step_apu();
        }

        // Use PPU frame completion as the authoritative frame boundary
        self.bus.ppu_frame_complete()
    }

    pub fn get_frame_buffer(&self) -> &[u8] {
        self.bus.get_ppu_buffer()
    }

    pub fn get_audio_buffer(&mut self) -> Vec<f32> {
        self.bus.get_audio_buffer()
    }

    pub fn set_controller(&mut self, controller: u8) {
        self.bus.set_controller(controller);
    }

    pub fn save_state(&self, slot: u8, rom_filename: &str) -> Result<(), Box<dyn std::error::Error>> {
        let save_state = save_state::SaveState {
            cpu_a: self.cpu.a,
            cpu_x: self.cpu.x,
            cpu_y: self.cpu.y,
            cpu_pc: self.cpu.pc,
            cpu_sp: self.cpu.sp,
            cpu_status: self.cpu.status.bits(),
            cpu_cycles: 0,
            ppu_control: self.bus.get_ppu_state().0,
            ppu_mask: self.bus.get_ppu_state().1,
            ppu_status: self.bus.get_ppu_state().2,
            ppu_oam_addr: self.bus.get_ppu_state().3,
            ppu_scroll_x: 0,
            ppu_scroll_y: 0,
            ppu_addr: 0,
            ppu_data_buffer: 0,
            ppu_w: false,
            ppu_t: 0,
            ppu_v: 0,
            ppu_x: 0,
            ppu_scanline: 0,
            ppu_cycle: 0,
            ppu_frame: 0,
            ppu_palette: self.bus.get_ppu_palette(),
            ppu_nametable: self.bus.get_ppu_nametables_flat(),
            ppu_oam: self.bus.get_ppu_oam_flat(),
            ram: self.bus.get_ram_flat(),
            cartridge_prg_bank: self.bus.get_cartridge_prg_bank(),
            cartridge_chr_bank: self.bus.get_cartridge_chr_bank(),
            apu_frame_counter: 0,
            apu_frame_interrupt: false,
            rom_filename: rom_filename.to_string(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs(),
        };

        let filename = format!("save_state_{}.sav", slot);
        save_state.save_to_file(&filename)?;
        Ok(())
    }

    pub fn load_state(&mut self, slot: u8) -> Result<(), Box<dyn std::error::Error>> {
        let filename = format!("save_state_{}.sav", slot);
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
        )?;

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
