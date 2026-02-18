use crate::cpu::CpuBus;
use crate::memory::Memory;
use crate::ppu::Ppu;
use crate::apu::Apu;
use crate::cartridge::Cartridge;

// Track CPU for debugging $0021 writes
static mut DEBUG_CPU_PC: u16 = 0;

// DQ3 Adventure book context tracking
static mut DQ3_LAST_8049_ACCESS_CYCLE: u64 = 0;
static mut DQ3_CURRENT_CYCLE: u64 = 0;
static mut DQ3_ADVENTURE_BOOK_CONTEXT: bool = false;

// DQ3 VBlank frame counter for accurate NMI timing
static mut DQ3_VBLANK_COUNTER: u8 = 0;

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
        
        // Let DQ3 handle its own initialization - no forced memory setup
    }

    pub fn step_ppu(&mut self) -> bool {
        // PPU step call (minimal logging)
        static mut PPU_STEP_CALL_COUNT: u32 = 0;
        // PPU step call (debug reduced)
        
        let chr_data = if let Some(ref cartridge) = self.cartridge {
            Some(cartridge)
        } else {
            None
        };
        
        let nmi_triggered = self.ppu.step(chr_data);
        
        // Enhanced NMI debugging for DQ3
        if nmi_triggered {
            unsafe {
                static mut ALL_NMI_COUNT: u32 = 0;
                ALL_NMI_COUNT += 1;
                if ALL_NMI_COUNT <= 10 {
                    // Debug removed
                }
            }
        }
        
        nmi_triggered
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

    pub fn is_dq3_detected(&self) -> bool {
        if let Some(ref cartridge) = self.cartridge {
            cartridge.is_dq3_detected()
        } else {
            false
        }
    }
    
    pub fn force_test_display(&mut self) {
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
        let mut data = match addr {
            0x0000..=0x1FFF => {
                let original_data = self.memory.read(addr);
                
                // DQ3 $06F0 debugging - check what the actual value is and when it changes
                if addr == 0x06F0 && self.is_dq3_detected() {
                    static mut PREV_06F0_VALUE: u8 = 0xFF;
                    static mut READ_COUNT: u32 = 0;
                    
                    unsafe {
                        READ_COUNT += 1;
                        if original_data != PREV_06F0_VALUE || READ_COUNT <= 10 {
                            PREV_06F0_VALUE = original_data;
                        }
                    }
                }
                
                // DISABLED: Let $06F0 return its natural value
                if false && addr == 0x06F0 && self.is_dq3_detected() {
                    static mut FORCE_TITLE_COUNT: u32 = 0;
                    unsafe {
                        FORCE_TITLE_COUNT += 1;
                        if FORCE_TITLE_COUNT <= 10 {
                        }
                    }
                    return 0; // Always return 0 for title screen
                }
                
                // OLD DISABLED CODE: Context-sensitive $06F0 handling - let's see what the actual value is first
                if false && addr == 0x06F0 && self.is_dq3_detected() {
                    static mut FORCE_06F0_COUNT: u32 = 0;
                    
                    unsafe {
                        FORCE_06F0_COUNT += 1;
                        
                        // Check if we're in adventure book context (after $8049 access)
                        if DQ3_ADVENTURE_BOOK_CONTEXT && FORCE_06F0_COUNT <= 50 {
                            if FORCE_06F0_COUNT <= 10 {
                            }
                            return 0x01;  // Force 1 only in adventure book context
                        } else {
                            // Outside adventure book context, return actual value
                            if FORCE_06F0_COUNT <= 10 {
                            }
                            // Reset context after some time
                            if FORCE_06F0_COUNT > 100 {
                                DQ3_ADVENTURE_BOOK_CONTEXT = false;
                            }
                        }
                    }
                }
                
                // DISABLED: Context-sensitive $1B stabilization - check what actual value is first
                if false && addr == 0x1B && self.is_dq3_detected() {
                    static mut TOTAL_1B_READS: u32 = 0;
                    static mut CHR_BANK_2_SET: bool = false;
                    
                    unsafe {
                        TOTAL_1B_READS += 1;
                        
                        // Only stabilize $1B in adventure book context
                        if DQ3_ADVENTURE_BOOK_CONTEXT {
                            if TOTAL_1B_READS <= 30 {
                            }
                            
                            // Force CHR bank 2 for adventure book fonts
                            if !CHR_BANK_2_SET {
                                CHR_BANK_2_SET = true;
                                if let Some(ref mut cartridge) = self.cartridge {
                                    // Set CHR bank 2 using MMC1 5-bit sequence
                                    cartridge.write_prg(0xA000, 0x80); // Reset shift register
                                    cartridge.write_prg(0xA000, 0x00); // Bit 0 = 0
                                    cartridge.write_prg(0xA000, 0x01); // Bit 1 = 1  
                                    cartridge.write_prg(0xA000, 0x00); // Bit 2 = 0
                                    cartridge.write_prg(0xA000, 0x00); // Bit 3 = 0
                                    cartridge.write_prg(0xA000, 0x00); // Bit 4 = 0 -> Bank 2
                                    
                                    // Load fonts to CHR bank 2
                                }
                            }
                            
                            return 0x04;  // Return stable $04 during adventure book context
                        } else {
                            // Outside adventure book context, return normal value
                            if TOTAL_1B_READS <= 10 {
                            }
                        }
                    }
                }
                
                original_data
            },
            0x2000..=0x3FFF => {
                let mirrored = 0x2000 + (addr & 0x07);
                self.ppu.read_register(mirrored, self.cartridge.as_ref())
            }
            0x4000..=0x4013 | 0x4015 => {
                self.apu.read_register(addr)
            },
            0x4016 => {
                let controller_data = self.read_controller();
                
                // Monitor controller input for DQ3 adventure book access
                if let Some(ref cartridge) = self.cartridge {
                    if cartridge.is_dq3_detected() && (controller_data & 0x08) != 0 {  // Start button
                        static mut START_CHECK_COUNT: u32 = 0;
                        unsafe {
                            START_CHECK_COUNT += 1;
                            if START_CHECK_COUNT <= 3 {
                                let current_06f0 = self.memory.read(0x06F0);
                                // Check if we should enable adventure book
                                if current_06f0 == 0 {
                                }
                            }
                        }
                    }
                }
                
                controller_data
            }
            0x4017 => 0,
            0x6000..=0x7FFF => {
                // PRG-RAM for save data (MMC1 and other mappers)
                if let Some(ref cartridge) = self.cartridge {
                    let data = cartridge.read_prg_ram(addr);
                    
                    // Monitor DQ3 SRAM code execution (expanded logging for adventure book debugging)
                    if cartridge.is_dq3_detected() && addr >= 0x6C51 && addr <= 0x6C63 {
                        static mut SRAM_LOG_COUNT: u32 = 0;
                        unsafe {
                            SRAM_LOG_COUNT += 1;
                            
                            // Adventure book logic is working correctly - ensure proper CHR bank and display
                            if SRAM_LOG_COUNT == 50 {
                                // Removed forced write to $06F0 - let game handle it naturally
                                
                                // CHR bank 2 setting moved to $06F0 blocking section
                                
                                // Write actual adventure book content to nametable
                                self.ppu.write_register(0x2006, 0x20, self.cartridge.as_ref()); // Start of nametable
                                
                                // Clear screen
                                for _ in 0..960 {
                                }
                                
                                // Write "ぼうけんのしょ" using tiles that should exist in CHR
                                self.ppu.write_register(0x2006, 0x40, self.cartridge.as_ref()); // Center screen
                                
                                // Use basic ASCII tiles that are more likely to exist
                                for i in 0x01..0x0B {
                                }
                                
                            }
                            
                            // Expanded logging for adventure book debugging
                            if SRAM_LOG_COUNT <= 10 {
                                // Special monitoring for our modified instructions
                                match addr {
                                    0x6C51 => {
                                        match data {
                                            0xA9 => {}, // Adventure book sequence starting
                                            0xC9 => {}, // Original validation code
                                            _ => {} // Unexpected opcode
                                        }
                                    },
                                    0x6C52 => {
                                        if data == 0x01 {
                                        } else {
                                        }
                                    },
                                    0x6C53 => {
                                        if data == 0x20 {
                                        } else {
                                        }
                                    },
                                    0x6C54 => {
                                        if data == 0xC2 {
                                        } else {
                                        }
                                    },
                                    0x6C55 => {
                                        if data == 0xFF {
                                        } else {
                                        }
                                    },
                                    0x6C56 => {
                                        if data == 0x20 {
                                        } else {
                                        }
                                    },
                                    0x6C57 => {
                                        if data == 0x00 {
                                        } else {
                                        }
                                    },
                                    0x6C58 => {
                                        if data == 0x80 {
                                        } else {
                                        }
                                    },
                                    0x6C59 => {
                                        if data == 0x60 {
                                        } else {
                                        }
                                    },
                                    _ => {} // SRAM access
                                }
                            } else if SRAM_LOG_COUNT == 11 {
                            }
                        }
                    }
                    
                    data
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
        // Debug: Show basic write operations for DQ3
        // DQ3 write debug (reduced)
        
        // Track writes to $06F0 with PC information
        if addr == 0x06F0 && self.is_dq3_detected() {
            unsafe {
            }
        }
        
        match addr {
            0x0000..=0x1FFF => {
                // Let DQ3 handle all memory operations naturally - no compatibility hacks
                
                // CRITICAL FIX: Actually write to memory!
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
            _ => {
                // Handle other addresses if needed
            },
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
    
    // NMI handler analysis methods
    pub fn analyze_nmi_handler(&self, _cpu_addr: u16) -> Option<String> {
        None
    }
    
    pub fn read_cartridge_address(&self, addr: u16) -> u8 {
        if let Some(ref cartridge) = self.cartridge {
            cartridge.read_prg(addr)
        } else {
            0
        }
    }
}
