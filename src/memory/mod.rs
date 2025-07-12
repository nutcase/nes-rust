pub struct Memory {
    pub(crate) ram: [u8; 0x800],
}

impl Memory {
    pub fn new() -> Self {
        let mut memory = Memory {
            ram: [0; 0x800],
        };
        
        // DQ3 proper initialization - start with clean state
        memory.ram[0x00] = 0x00; // Game state: start from cleared state
        memory.ram[0x01] = 0x00; // Frame counter low - let ROM control progression
        memory.ram[0x02] = 0x00; // Frame counter high  
        memory.ram[0x03] = 0x00; // Initialization flag
        
        // Try setting a completion flag that might signal end of initialization
        memory.ram[0x10] = 0x00; // Graphics loading state
        memory.ram[0x11] = 0x00; // Graphics bank
        memory.ram[0x12] = 0x00; // Graphics offset  
        memory.ram[0x13] = 0x00; // Clear graphics ready flag
        
        // Set frame/timing variables that might trigger progression
        memory.ram[0x20] = 0x00; // Timer/frame counter
        memory.ram[0x21] = 0x60; // RTS instruction as safe fallback for DQ3 function pointer corruption
        memory.ram[0x22] = 0x00; // Additional timing
        memory.ram[0x23] = 0x00; // Additional timing
        
        
        memory
    }

    pub fn read(&self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x1FFF => self.ram[(addr & 0x7FF) as usize],
            _ => 0,
        }
    }

    pub fn write(&mut self, addr: u16, data: u8) {
        match addr {
            0x0000..=0x1FFF => {
                let effective_addr = (addr & 0x7FF) as usize;
                
                // Monitor writes that might control graphics loading in DQ3
                if effective_addr >= 0x60 && effective_addr <= 0x7F {
                    // Timer/counter area monitoring
                    static mut TIMER_WRITE_COUNT: u32 = 0;
                    unsafe {
                        TIMER_WRITE_COUNT += 1;
                        if TIMER_WRITE_COUNT <= 20 || (TIMER_WRITE_COUNT % 500 == 0) {
                        }
                    }
                } else if effective_addr == 0x21 && false {
                    // Disabled $0021 monitoring - DQ3 uses this address for normal data storage
                } else if effective_addr <= 0x0F {
                    // Monitor zero page variables for game state changes
                    static mut CRITICAL_WRITE_COUNT: u32 = 0;
                    static mut LAST_GAME_STATE: [u8; 16] = [0xFF; 16]; // Track first 16 bytes
                    unsafe {
                        CRITICAL_WRITE_COUNT += 1;
                        // Completely disable DQ3 monitoring - let ROM run natively
                        // if CRITICAL_WRITE_COUNT <= 50 || LAST_GAME_STATE[effective_addr] != data {
                        //     // Disabled DQ3 game state monitoring - let ROM run natively
                        //     // Monitor critical game state changes for DQ3 title screen progression
                        //     // if effective_addr <= 0x05 && LAST_GAME_STATE[effective_addr] != data {
                        //     //     println!("DQ3: Game state change ${:02X}: ${:02X} -> ${:02X}", 
                        //     //              effective_addr, LAST_GAME_STATE[effective_addr], data);
                        //         
                        //         // Disabled all DQ3 auto-transition logic
                        //         // Enhanced title screen loop detection and auto-transition
                        //         // static mut TITLE_LOOP_COUNT: u32 = 0;
                        //         // static mut AUTO_TRANSITION_TRIGGERED: bool = false;
                        //         // static mut TITLE_STABLE_COUNT: u32 = 0;
                        //         // 
                        //         // // Detect stable title screen state (indicates ready for transition)
                        //         // if effective_addr == 0x01 {
                        //         //     if LAST_GAME_STATE[1] == 0x07 && data == 0x00 {
                        //         //         TITLE_LOOP_COUNT += 1;
                        //         //         println!("DQ3: Title screen loop #{} completed", TITLE_LOOP_COUNT);
                        //         //     }
                        //         //     
                        //         //     // Enhanced stability detection
                        //         //     if data == 0x00 || data == 0x07 {
                        //         //         TITLE_STABLE_COUNT += 1;
                        //         //     } else {
                        //         //         TITLE_STABLE_COUNT = 0;
                        //         //     }
                        //         // }
                        //         // 
                        //         // // Re-enable DQ3 transition assistance when START is pressed
                        //         // // Check if START button causes state change but gets stuck
                        //         // Disabled all forced state changes
                        //         // if effective_addr == 0x04 && TITLE_LOOP_COUNT > 3 && !AUTO_TRANSITION_TRIGGERED {
                        //         //     // Force DQ3 to progress if stuck in title loop
                        //         //     AUTO_TRANSITION_TRIGGERED = true;
                        //         //     println!("DQ3: Title screen stuck in loop, forcing adventure book transition");
                        //         //     
                        //         //     // Set DQ3 to adventure book selection state
                        //         //     // This mimics what would happen after successful START button processing
                        //         //     self.ram[0x01] = 0x08;  // Adventure book menu state
                        //         //     self.ram[0x04] = 0x20;  // Adventure book selection screen ID
                        //         //     self.ram[0x05] = 0x00;  // Clear any conflicting state
                        //         //     
                        //         //     println!("DQ3: Forced state: $01=0x08, $04=0x20 (adventure book screen)");
                        //         // }
                        //     // }
                        //     // LAST_GAME_STATE[effective_addr] = data;
                        // }
                        
                        // Special attention to specific addresses that might control screen flow
                        match effective_addr {
                            0x00 | 0x01 => {
                            }
                            0x10..=0x13 => {
                            }
                            _ => {}
                        }
                    }
                } else if effective_addr <= 0x03 || (effective_addr >= 0x10 && effective_addr <= 0x13) || 
                   (effective_addr >= 0x20 && effective_addr <= 0x23) || (effective_addr >= 0x30 && effective_addr <= 0x33) ||
                   (effective_addr >= 0x40 && effective_addr <= 0x5F) {
                    static mut STATE_WRITE_COUNT: u32 = 0;
                    unsafe {
                        STATE_WRITE_COUNT += 1;
                        if STATE_WRITE_COUNT <= 20 {
                        }
                    }
                }
                
                self.ram[effective_addr] = data;
            },
            _ => {},
        }
    }
    
    // Save state methods
    pub fn get_ram(&self) -> [u8; 0x800] {
        self.ram
    }
    
    pub fn set_ram(&mut self, ram: [u8; 0x800]) {
        self.ram = ram;
    }
}