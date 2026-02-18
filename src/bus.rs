use crate::cpu::CpuBus;
use crate::memory::Memory;
use crate::ppu::Ppu;
use crate::apu::Apu;
use crate::cartridge::Cartridge;

pub struct Bus {
    memory: Memory,
    ppu: Ppu,
    apu: Apu,
    cartridge: Option<Cartridge>,
    pub controller: u8,
    controller_state: u16,
    strobe: bool, // Controller strobe mode
    dma_cycles: u32, // Cycles to add due to DMA operations
    dma_in_progress: bool, // Flag to indicate DMA is in progress
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
        }
    }

    pub fn load_cartridge(&mut self, cartridge: Cartridge) {
        self.cartridge = Some(cartridge);
    }

    #[inline]
    pub fn step_ppu(&mut self) -> bool {
        self.ppu.step(self.cartridge.as_ref())
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

        // Step APU at CPU rate
        for _ in 0..cycles {
            self.apu.step();
        }

        nmi_triggered
    }

    pub fn step_apu(&mut self) {
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
        let value = if self.controller_state & 0x01 != 0 { 0x01 } else { 0x00 };
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

    pub fn get_audio_buffer(&mut self) -> Vec<f32> {
        self.apu.get_audio_buffer()
    }

    // Check if APU frame IRQ is pending
    pub fn apu_irq_pending(&self) -> bool {
        self.apu.frame_irq_pending()
    }

    // Clear APU frame IRQ
    pub fn clear_apu_irq(&mut self) {
        self.apu.clear_frame_irq();
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

    pub fn read_chr(&self, addr: u16) -> u8 {
        if let Some(ref cartridge) = self.cartridge {
            cartridge.read_chr(addr)
        } else {
            0
        }
    }

    pub fn is_goonies(&self) -> bool {
        if let Some(ref cartridge) = self.cartridge {
            cartridge.is_goonies()
        } else {
            false
        }
    }
}

impl CpuBus for Bus {
    fn check_game_specific_cpu_protection(&self, pc: u16, sp: u8, cycles: u64) -> Option<(u16, u8)> {
        if let Some(ref cartridge) = self.cartridge {
            if let Some(_result) = cartridge.goonies_check_ce7x_loop(pc, sp, cycles) {
            }
        }
        None
    }

    fn check_game_specific_brk_protection(&self, pc: u16, sp: u8, cycles: u64) -> Option<(u16, u8)> {
        if let Some(ref cartridge) = self.cartridge {
            cartridge.goonies_check_abnormal_brk(pc, sp, cycles)
        } else {
            None
        }
    }

    fn is_goonies(&self) -> bool {
        if let Some(ref cartridge) = self.cartridge {
            cartridge.is_goonies()
        } else {
            false
        }
    }

    fn read(&mut self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x1FFF => {
                self.memory.read(addr)
            },
            0x2000..=0x3FFF => {
                let mirrored = 0x2000 + (addr & 0x07);
                self.ppu.read_register(mirrored, self.cartridge.as_ref())
            }
            0x4000..=0x4013 | 0x4015 => {
                self.apu.read_register(addr)
            },
            0x4016 => {
                self.read_controller()
            }
            0x4017 => 0,
            0x6000..=0x7FFF => {
                if let Some(ref cartridge) = self.cartridge {
                    cartridge.read_prg_ram(addr)
                } else {
                    0
                }
            },
            0x8000..=0xFFFF => {
                if let Some(ref cartridge) = self.cartridge {
                    cartridge.read_prg(addr)
                } else {
                    0
                }
            },
            _ => 0,
        }
    }

    fn write(&mut self, addr: u16, data: u8) {
        match addr {
            0x0000..=0x1FFF => {
                self.memory.write(addr, data);
            },
            0x2000..=0x3FFF => {
                let mirrored = 0x2000 + (addr & 0x07);
                if let Some((chr_addr, chr_data)) = self.ppu.write_register(mirrored, data, self.cartridge.as_ref()) {
                    if let Some(ref mut cartridge) = self.cartridge {
                        cartridge.write_chr(chr_addr, chr_data);
                    }
                }
            },
            0x4000..=0x4013 | 0x4015 | 0x4017 => {
                self.apu.write_register(addr, data);
            },
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
                        },
                        0x8000..=0xFFFF => {
                            if let Some(ref cartridge) = self.cartridge {
                                cartridge.read_prg(src)
                            } else {
                                0
                            }
                        },
                        _ => 0,
                    };
                    let oam_dst = oam_addr.wrapping_add(i as u8);
                    self.ppu.write_oam_data(oam_dst, byte);
                }
                self.dma_in_progress = true;
                self.dma_cycles = 513;
            },
            0x4016 => {
                // Controller strobe
                let new_strobe = (data & 0x01) != 0;
                if self.strobe && !new_strobe {
                    // Falling edge: latch controller state
                    self.controller_state = self.controller as u16;
                }
                self.strobe = new_strobe;
            },
            0x4020..=0xFFFF => {
                if let Some(ref mut cartridge) = self.cartridge {
                    match addr {
                        0x6000..=0x7FFF => {
                            cartridge.write_prg_ram(addr, data);
                        },
                        0x8000..=0xFFFF => {
                            cartridge.write_prg(addr, data);
                        },
                        _ => {}
                    }
                }
            },
            _ => {},
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

    pub fn restore_state_flat(
        &mut self,
        palette: impl AsRef<[u8]>,
        nametables: impl AsRef<[u8]>,
        oam: impl AsRef<[u8]>,
        ram: impl AsRef<[u8]>,
        prg_bank: u8,
        chr_bank: u8,
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
