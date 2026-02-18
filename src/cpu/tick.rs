use super::*;

impl Cpu {
    // New fine-grained execution method inspired by reference emulator
    pub fn step_with_tick(&mut self, bus: &mut dyn CpuBusWithTick) -> bool {
        // Safety check: prevent execution in invalid areas
        if self.pc < 0x0010 {
            self.reset_basic(bus);
            return false;
        }

        // Read opcode and execute instruction
        let opcode = bus.read(self.pc);
        let old_pc = self.pc;
        self.pc = self.pc.wrapping_add(1);

        let cycles = match opcode {
            0xEA => 2, // NOP
            0xA9 => self.lda_immediate_tick(bus), // LDA immediate
            0xAD => self.lda_absolute_tick(bus),  // LDA absolute
            0x2C => self.bit_absolute_tick(bus),  // BIT absolute
            0x10 => self.bpl_tick(bus),           // BPL
            0x30 => self.bmi_tick(bus),           // BMI
            0x4C => self.jmp_absolute_tick(bus),  // JMP absolute
            0x20 => self.jsr_tick(bus),           // JSR
            0x60 => self.rts_tick(bus),           // RTS
            _ => {
                // Fallback to original step method for unhandled opcodes
                self.pc = old_pc;
                return false;
            }
        };

        // Tick the bus with instruction cycles
        let nmi_triggered = bus.tick(cycles);

        // Handle NMI if triggered during instruction
        if nmi_triggered {
            self.nmi_basic(bus);
        }

        true
    }

    // Simplified instruction implementations for tick-based execution
    fn lda_immediate_tick(&mut self, bus: &mut dyn CpuBusWithTick) -> u8 {
        self.a = self.read_byte_tick(bus);
        self.set_zero_negative_flags(self.a);
        2
    }

    fn lda_absolute_tick(&mut self, bus: &mut dyn CpuBusWithTick) -> u8 {
        let addr = self.read_word_tick(bus);
        self.a = bus.read(addr);
        self.set_zero_negative_flags(self.a);
        4
    }

    fn bit_absolute_tick(&mut self, bus: &mut dyn CpuBusWithTick) -> u8 {
        let addr = self.read_word_tick(bus);
        let value = bus.read(addr);
        self.status.set(StatusFlags::ZERO, (self.a & value) == 0);
        self.status.set(StatusFlags::OVERFLOW, value & 0x40 != 0);
        self.status.set(StatusFlags::NEGATIVE, value & 0x80 != 0);
        4
    }

    fn bpl_tick(&mut self, bus: &mut dyn CpuBusWithTick) -> u8 {
        self.branch_tick(bus, !self.status.contains(StatusFlags::NEGATIVE))
    }

    fn bmi_tick(&mut self, bus: &mut dyn CpuBusWithTick) -> u8 {
        self.branch_tick(bus, self.status.contains(StatusFlags::NEGATIVE))
    }

    fn jmp_absolute_tick(&mut self, bus: &mut dyn CpuBusWithTick) -> u8 {
        self.pc = self.read_word_tick(bus);
        3
    }

    fn jsr_tick(&mut self, bus: &mut dyn CpuBusWithTick) -> u8 {
        let addr = self.read_word_tick(bus);
        let return_addr = self.pc.wrapping_sub(1);
        self.push_tick(bus, (return_addr >> 8) as u8);
        self.push_tick(bus, return_addr as u8);
        self.pc = addr;
        6
    }

    fn rts_tick(&mut self, bus: &mut dyn CpuBusWithTick) -> u8 {
        let low = self.pull_tick(bus) as u16;
        let high = self.pull_tick(bus) as u16;
        self.pc = ((high << 8) | low).wrapping_add(1);
        6
    }

    fn branch_tick(&mut self, bus: &mut dyn CpuBusWithTick, condition: bool) -> u8 {
        let offset = self.read_byte_tick(bus) as i8;
        if condition {
            let old_pc = self.pc;
            let new_pc = self.pc.wrapping_add(offset as u16);
            let cycles = if (old_pc & 0xFF00) != (new_pc & 0xFF00) { 4 } else { 3 };
            self.pc = new_pc;
            cycles
        } else {
            2
        }
    }

    // Helper methods for tick-based execution
    fn read_byte_tick(&mut self, bus: &mut dyn CpuBusWithTick) -> u8 {
        let byte = bus.read(self.pc);
        self.pc = self.pc.wrapping_add(1);
        byte
    }

    fn read_word_tick(&mut self, bus: &mut dyn CpuBusWithTick) -> u16 {
        let low = self.read_byte_tick(bus) as u16;
        let high = self.read_byte_tick(bus) as u16;
        (high << 8) | low
    }

    fn push_tick(&mut self, bus: &mut dyn CpuBusWithTick, value: u8) {
        let addr = 0x0100 | self.sp as u16;
        bus.write(addr, value);
        self.sp = self.sp.wrapping_sub(1);
    }

    fn pull_tick(&mut self, bus: &mut dyn CpuBusWithTick) -> u8 {
        self.sp = self.sp.wrapping_add(1);
        let addr = 0x0100 | self.sp as u16;
        bus.read(addr)
    }

    // Basic reset without complex bus operations
    fn reset_basic(&mut self, bus: &mut dyn CpuBusWithTick) {
        self.a = 0;
        self.x = 0;
        self.y = 0;
        self.sp = 0xFD;
        self.status = StatusFlags::from_bits_truncate(0x24);

        let low = bus.read(0xFFFC) as u16;
        let high = bus.read(0xFFFD) as u16;
        self.pc = (high << 8) | low;
        self.cycles = 8;
    }

    // Basic NMI handler
    fn nmi_basic(&mut self, bus: &mut dyn CpuBusWithTick) {
        self.push_tick(bus, (self.pc >> 8) as u8);
        self.push_tick(bus, self.pc as u8);
        let status = self.status.bits() & !StatusFlags::BREAK.bits();
        self.push_tick(bus, status);
        self.status.insert(StatusFlags::INTERRUPT_DISABLE);

        let low = bus.read(0xFFFA) as u16;
        let high = bus.read(0xFFFB) as u16;
        let nmi_vector = (high << 8) | low;
        self.pc = nmi_vector;

        // Tick for NMI processing cycles
        bus.tick(7);
    }
}
