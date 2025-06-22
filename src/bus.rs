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
                let data = self.ppu.read_register(addr);
                // PPU status read
                data
            }
            0x4000..=0x4013 | 0x4015 => self.apu.read_register(addr),
            0x4016 => {
                let data = self.read_controller();
                // Controller read
                data
            }
            0x4017 => 0,
            0x8000..=0xFFFF => {
                if let Some(ref cartridge) = self.cartridge {
                    let data = cartridge.read_prg(addr - 0x8000);
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
            0x2000..=0x2007 => self.ppu.write_register(addr, data),
            0x4000..=0x4013 | 0x4015 | 0x4017 => self.apu.write_register(addr, data),
            0x4014 => {
                let start = (data as u16) << 8;
                // OAM DMA transfer
                
                // Perform DMA transfer immediately
                for i in 0..256 {
                    let byte = self.read(start + i);
                    self.ppu.write_register(0x2004, byte);
                }
                
                // Set DMA in progress with cycle count
                self.dma_cycles = 513; // OAM DMA takes 513 cycles
                self.dma_in_progress = true;
                // DMA started
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
                    cartridge.write_prg(addr - 0x8000, data);
                }
            },
            _ => {},
        }
    }
}

