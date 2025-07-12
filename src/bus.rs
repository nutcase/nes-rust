use crate::cpu::{CpuBus, CpuBusWithTick};
use crate::memory::Memory;
use crate::ppu::Ppu;
use crate::apu::Apu;
use crate::cartridge::Cartridge;

// Track CPU for debugging $0021 writes
static mut DEBUG_CPU_PC: u16 = 0;

pub struct Bus {
    memory: Memory,
    ppu: Ppu,
    apu: Apu,
    cartridge: Option<Cartridge>,
    pub controller: u8,
    controller_state: u16,
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
            dma_cycles: 0,
            dma_in_progress: false,
        }
    }

    pub fn load_cartridge(&mut self, cartridge: Cartridge) {
        self.cartridge = Some(cartridge);
    }

    pub fn step_ppu(&mut self) -> bool {
        let chr_data = if let Some(ref cartridge) = self.cartridge {
            Some(cartridge)
        } else {
            None
        };
        self.ppu.step(chr_data)
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
        let value = if self.controller_state & 0x01 != 0 { 0x01 } else { 0x00 };
        self.controller_state >>= 1;
        value
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

impl Bus {
    pub fn set_debug_pc(&mut self, pc: u16) {
        unsafe {
            DEBUG_CPU_PC = pc;
        }
    }
}

impl CpuBus for Bus {
    fn check_game_specific_cpu_protection(&self, pc: u16, sp: u8, cycles: u64) -> Option<(u16, u8)> {
        if let Some(ref cartridge) = self.cartridge {
            // Check Goonies-specific protections
            if let Some(result) = cartridge.goonies_check_ce7x_loop(pc, sp, cycles) {
                return Some(result);
            }
            // Future: Add other game-specific protections here
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
        let data = match addr {
            0x0000..=0x1FFF => self.memory.read(addr),
            0x2000..=0x2007 => {
                self.ppu.read_register(addr, self.cartridge.as_ref())
            }
            0x4000..=0x4013 | 0x4015 => {
                self.apu.read_register(addr)
            },
            0x4016 => {
                self.read_controller()
            }
            0x4017 => 0,
            0x6000..=0x7FFF => {
                // PRG-RAM for save data (MMC1 and other mappers)
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
        };
        
        data
    }

    fn write(&mut self, addr: u16, data: u8) {
        match addr {
            0x0000..=0x1FFF => self.memory.write(addr, data),
            0x2000..=0x2007 => {
                if let Some((chr_addr, chr_data)) = self.ppu.write_register(addr, data, self.cartridge.as_ref()) {
                    if let Some(ref mut cartridge) = self.cartridge {
                        cartridge.write_chr(chr_addr, chr_data);
                    }
                }
            },
            0x4014 => {
                let start = (data as u16) << 8;
                // OAM DMA transfer
                
                // Perform DMA transfer immediately
                for i in 0..256 {
                    let byte = self.read(start + i);
                    if let Some((chr_addr, chr_data)) = self.ppu.write_register(0x2004, byte, self.cartridge.as_ref()) {
                        // Handle CHR write during DMA (unlikely but possible)
                        if let Some(ref mut cartridge) = self.cartridge {
                            cartridge.write_chr(chr_addr, chr_data);
                        }
                    }
                }
                
                // Set DMA in progress with cycle count
                self.dma_cycles = 513; // OAM DMA takes 513 cycles
                self.dma_in_progress = true;
            },
            0x4000..=0x4013 | 0x4015 | 0x4017 => {
                self.apu.write_register(addr, data);
            },
            0x4016 => {
                if data & 0x01 != 0 {
                    // Strobe high - load controller state from the actual controller
                    self.controller_state = self.controller as u16;
                } else {
                    // Strobe low - prepare for reading sequence
                }
            },
            0x6000..=0x7FFF => {
                // PRG-RAM for save data and mapper bank switching
                if let Some(ref mut cartridge) = self.cartridge {
                    // First try PRG-RAM write (for MMC1 save data)
                    cartridge.write_prg_ram(addr, data);
                    // Also try mapper bank switching (for Mapper 87)
                    cartridge.write_prg(addr, data);
                }
            },
            0x8000..=0xFFFF => {
                if let Some(ref mut cartridge) = self.cartridge {
                    cartridge.write_prg(addr, data);
                }
            },
            _ => {},
        }
    }
}

impl Bus {
    // Save state methods
    pub fn get_ppu_state(&self) -> (u8, u8, u8, u8) {
        (
            self.ppu.get_control(),
            self.ppu.get_mask(), 
            self.ppu.get_status(),
            self.ppu.get_oam_addr()
        )
    }
    
    pub fn get_ppu_palette(&self) -> [u8; 32] {
        self.ppu.get_palette()
    }
    
    pub fn get_ppu_nametables(&self) -> [[u8; 1024]; 2] {
        self.ppu.get_nametable()
    }
    
    pub fn get_ppu_oam(&self) -> [u8; 256] {
        self.ppu.get_oam()
    }
    
    pub fn get_ram(&self) -> [u8; 0x800] {
        self.memory.get_ram()
    }
    
    // Flattened versions for serialization
    pub fn get_ppu_nametables_flat(&self) -> Vec<u8> {
        let nametables = self.ppu.get_nametable();
        let mut flat = Vec::with_capacity(2048);
        flat.extend_from_slice(&nametables[0]);
        flat.extend_from_slice(&nametables[1]);
        flat
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
    
    pub fn restore_state(
        &mut self,
        ppu_palette: [u8; 32],
        ppu_nametable: [[u8; 1024]; 2], 
        ppu_oam: [u8; 256],
        ram: [u8; 0x800],
        prg_bank: u8,
        chr_bank: u8,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Restore PPU state
        self.ppu.set_palette(ppu_palette);
        self.ppu.set_nametable(ppu_nametable);
        self.ppu.set_oam(ppu_oam);
        
        // Restore RAM
        self.memory.set_ram(ram);
        
        // Restore cartridge state
        if let Some(ref mut cartridge) = self.cartridge {
            cartridge.set_prg_bank(prg_bank);
            cartridge.set_chr_bank(chr_bank);
        }
        
        Ok(())
    }
    
    pub fn restore_state_flat(
        &mut self,
        ppu_palette: [u8; 32],
        ppu_nametable_flat: Vec<u8>, 
        ppu_oam_flat: Vec<u8>,
        ram_flat: Vec<u8>,
        prg_bank: u8,
        chr_bank: u8,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Restore PPU state
        self.ppu.set_palette(ppu_palette);
        
        // Restore nametables from flat data
        if ppu_nametable_flat.len() == 2048 {
            let mut nametables = [[0u8; 1024]; 2];
            nametables[0].copy_from_slice(&ppu_nametable_flat[0..1024]);
            nametables[1].copy_from_slice(&ppu_nametable_flat[1024..2048]);
            self.ppu.set_nametable(nametables);
        }
        
        // Restore OAM from flat data
        if ppu_oam_flat.len() == 256 {
            let mut oam = [0u8; 256];
            oam.copy_from_slice(&ppu_oam_flat);
            self.ppu.set_oam(oam);
        }
        
        // Restore RAM from flat data
        if ram_flat.len() == 0x800 {
            let mut ram = [0u8; 0x800];
            ram.copy_from_slice(&ram_flat);
            self.memory.set_ram(ram);
        }
        
        // Restore cartridge state
        if let Some(ref mut cartridge) = self.cartridge {
            cartridge.set_prg_bank(prg_bank);
            cartridge.set_chr_bank(chr_bank);
        }
        
        Ok(())
    }
    
    pub fn schedule_dq3_graphics_loading(&mut self) {
        // Disabled - no special graphics loading
    }
    
    pub fn check_dq3_graphics_loading(&mut self) {
        // Disabled - no special processing
    }
    
    // DQ3-specific methods (disabled)
    pub fn is_dq3_mode(&self) -> bool {
        if let Some(ref cartridge) = self.cartridge {
            cartridge.mapper_number() == 1 && cartridge.prg_rom_size() == 256 * 1024
        } else {
            false
        }
    }
    
    pub fn get_sram_data(&self) -> Option<Vec<u8>> {
        if let Some(ref cartridge) = self.cartridge {
            cartridge.get_sram_data().map(|data| data.to_vec())
        } else {
            None
        }
    }
    
    pub fn process_dq3_title_screen_logic(&mut self) {
        // DISABLED: Don't load adventure book fonts that overwrite DRAGON tiles
        // Original DRAGON text uses tile IDs 0x0E, 0x1C, 0x0B, 0x11, 0x19, 0x18
        // which are the same as adventure book tiles, causing corruption
        if false && self.ppu.dq3_font_load_needed {
            if let Some(ref mut cartridge) = self.cartridge {
                cartridge.load_dq3_adventure_book_tiles();
                self.ppu.dq3_font_load_needed = false;
            }
        }
        
        // DISABLED: Don't force reload DRAGON fonts - use natural ROM state
        // Let ROM data remain in its original state for proper title screen display
    }
    
    pub fn on_nmi_title_screen_check(&mut self) {
        // Disabled
    }
    
    pub fn pre_nmi_dq3_processing(&mut self) {
        // Disabled
    }
    
    pub fn post_nmi_dq3_processing(&mut self) {
        // Disabled
    }
}

// Implement the new tick-enabled trait
impl CpuBusWithTick for Bus {
    fn tick(&mut self, cycles: u8) -> bool {
        self.tick(cycles)
    }
}