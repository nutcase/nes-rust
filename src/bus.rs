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

    pub fn step_apu(&mut self) {
        self.apu.step();
    }

    pub fn set_controller(&mut self, controller: u8) {
        self.controller = controller;
    }

    fn read_controller(&mut self) -> u8 {
        // Controller read handling
        
        // Standard controller read behavior
        // Bit 0: button state, Bit 6: always 1 for standard controllers
        let value = if self.controller_state & 0x01 != 0 { 0x41 } else { 0x40 };
        self.controller_state >>= 1;
        // After 8 reads, return 0x41 (bit 0 = 1) to indicate no more buttons
        if self.controller_state == 0 {
            self.controller_state = 0x100; // Set bit 8 as a marker
        }
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
                    // DMA completed
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
}

impl CpuBus for Bus {
    fn read(&mut self, addr: u16) -> u8 {
        let data = match addr {
            0x0000..=0x1FFF => self.memory.read(addr),
            0x2000..=0x2007 => {
                let data = self.ppu.read_register(addr, self.cartridge.as_ref());
                // PPU status read
                data
            }
            0x4000..=0x4013 | 0x4015 => {
                let data = self.apu.read_register(addr);
                data
            },
            0x4016 => {
                let data = self.read_controller();
                // Controller read
                data
            }
            0x4017 => 0,
            0x8000..=0xFFFF => {
                if let Some(ref cartridge) = self.cartridge {
                    let data = cartridge.read_prg(addr);
                    // Reading from cartridge
                    data
                } else {
                    0
                }
            },
            _ => 0,
        };
        
        // Return data
        
        data
    }

    fn write(&mut self, addr: u16, data: u8) {
        
        match addr {
            0x0000..=0x1FFF => self.memory.write(addr, data),
            0x2000..=0x2007 => {
                if let Some((chr_addr, chr_data)) = self.ppu.write_register(addr, data, self.cartridge.as_ref()) {
                    // Handle CHR write to cartridge
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
                // DMA started
            },
            0x4000..=0x4013 | 0x4015 | 0x4017 => {
                self.apu.write_register(addr, data);
            },
            0x4016 => {
                if data & 0x01 != 0 {
                    // Strobe high - load controller state from the actual controller
                    self.controller_state = self.controller as u16;
                    // Controller state loaded
                } else {
                    // Strobe low - prepare for reading sequence
                    // Controller strobe low
                }
            },
            0x6000..=0x7FFF => {
                // Mapper bank switching (e.g., Mapper 87)
                if let Some(ref mut cartridge) = self.cartridge {
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
}

