use bitflags::bitflags;

#[cfg(test)]
mod tests;
mod instructions;
mod tick;

bitflags! {
    #[derive(Debug, Clone, Copy)]
    pub struct StatusFlags: u8 {
        const CARRY = 0b00000001;
        const ZERO = 0b00000010;
        const INTERRUPT_DISABLE = 0b00000100;
        const DECIMAL = 0b00001000;
        const BREAK = 0b00010000;
        const UNUSED = 0b00100000;
        const OVERFLOW = 0b01000000;
        const NEGATIVE = 0b10000000;
    }
}

pub struct Cpu {
    pub a: u8,      // Accumulator
    pub x: u8,      // X register
    pub y: u8,      // Y register
    pub sp: u8,     // Stack pointer
    pub pc: u16,    // Program counter
    pub status: StatusFlags,
    cycles: u64,
    rts_count: u32, // Counter for consecutive RTS calls at same PC
    last_rts_pc: u16, // Last PC where RTS was executed
}

impl Cpu {
    pub fn new() -> Self {
        Cpu {
            a: 0,
            x: 0,
            y: 0,
            sp: 0xFD,
            pc: 0,
            status: StatusFlags::from_bits_truncate(0x24),
            cycles: 0,
            rts_count: 0,
            last_rts_pc: 0,
        }
    }

    pub fn reset(&mut self, bus: &mut dyn CpuBus) {
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

    pub fn step(&mut self, bus: &mut dyn CpuBus) -> u8 {
        let opcode = bus.read(self.pc);

        // Increment PC for most instructions - special ones handle it themselves
        self.pc = self.pc.wrapping_add(1);
        
        let cycles = self.execute_instruction(opcode, bus);
        
        // Safety check: ensure we're making progress
        if cycles == 0 {
            return 2; // Return minimum cycles to prevent infinite loop
        }
        
        self.cycles += cycles as u64;
        cycles
    }

    pub fn nmi(&mut self, bus: &mut dyn CpuBus) {
        self.push(bus, (self.pc >> 8) as u8);
        self.push(bus, self.pc as u8);
        self.push(bus, self.status.bits() & !StatusFlags::BREAK.bits());
        
        self.status.insert(StatusFlags::INTERRUPT_DISABLE);
        
        let low = bus.read(0xFFFA) as u16;
        let high = bus.read(0xFFFB) as u16;
        let nmi_vector = (high << 8) | low;
        self.pc = nmi_vector;

        self.cycles += 7;
    }
    
    pub fn irq(&mut self, bus: &mut dyn CpuBus) {
        // IRQ is maskable - check interrupt disable flag
        if self.status.contains(StatusFlags::INTERRUPT_DISABLE) {
            return;
        }
        
        self.push(bus, (self.pc >> 8) as u8);
        self.push(bus, self.pc as u8);
        self.push(bus, self.status.bits() & !StatusFlags::BREAK.bits());
        
        self.status.insert(StatusFlags::INTERRUPT_DISABLE);
        
        // IRQ vector at $FFFE-$FFFF
        let low = bus.read(0xFFFE) as u16;
        let high = bus.read(0xFFFF) as u16;
        let irq_vector = (high << 8) | low;
        
        self.pc = irq_vector;

        self.cycles += 7;
    }

    fn execute_instruction(&mut self, opcode: u8, bus: &mut dyn CpuBus) -> u8 {
        match opcode {
            0x00 => self.brk(bus),
            0x01 => self.ora_indexed_indirect(bus),
            0x05 => self.ora_zero_page(bus),
            0x06 => self.asl_zero_page(bus),
            0x08 => self.php(bus),
            0x09 => self.ora_immediate(bus),
            0x0A => self.asl_accumulator(),
            0x0D => self.ora_absolute(bus),
            0x0E => self.asl_absolute(bus),
            
            0x10 => self.bpl(bus),
            0x11 => self.ora_indirect_indexed(bus),
            0x15 => self.ora_zero_page_x(bus),
            0x16 => self.asl_zero_page_x(bus),
            0x18 => self.clc(),
            0x19 => self.ora_absolute_y(bus),
            0x1D => self.ora_absolute_x(bus),
            0x1E => self.asl_absolute_x(bus),
            
            0x20 => self.jsr(bus),
            0x21 => self.and_indexed_indirect(bus),
            0x24 => self.bit_zero_page(bus),
            0x25 => self.and_zero_page(bus),
            0x26 => self.rol_zero_page(bus),
            0x28 => self.plp(bus),
            0x29 => self.and_immediate(bus),
            0x2A => self.rol_accumulator(),
            0x2C => self.bit_absolute(bus),
            0x2D => self.and_absolute(bus),
            0x2E => self.rol_absolute(bus),
            
            0x30 => self.bmi(bus),
            0x31 => self.and_indirect_indexed(bus),
            0x35 => self.and_zero_page_x(bus),
            0x36 => self.rol_zero_page_x(bus),
            0x38 => self.sec(),
            0x39 => self.and_absolute_y(bus),
            0x3D => self.and_absolute_x(bus),
            0x3E => self.rol_absolute_x(bus),
            
            0x40 => self.rti(bus),
            0x41 => self.eor_indexed_indirect(bus),
            0x45 => self.eor_zero_page(bus),
            0x46 => self.lsr_zero_page(bus),
            0x47 => {
                // SRE zeropage - Shift Right and Exclusive OR
                let addr = self.read_byte(bus) as u16;
                let value = bus.read(addr);
                let shifted = value >> 1;
                bus.write(addr, shifted);
                self.status.set(StatusFlags::CARRY, value & 0x01 != 0);
                self.a ^= shifted;
                self.set_zero_negative_flags(self.a);
                5
            }
            0x48 => self.pha(bus),
            0x49 => self.eor_immediate(bus),
            0x4A => self.lsr_accumulator(),
            0x4C => self.jmp_absolute(bus),
            0x4D => self.eor_absolute(bus),
            0x4E => self.lsr_absolute(bus),
            
            0x50 => self.bvc(bus),
            0x51 => self.eor_indirect_indexed(bus),
            0x55 => self.eor_zero_page_x(bus),
            0x56 => self.lsr_zero_page_x(bus),
            0x58 => self.cli(),
            0x59 => self.eor_absolute_y(bus),
            0x5D => self.eor_absolute_x(bus),
            0x5E => self.lsr_absolute_x(bus),
            
            0x60 => self.rts(bus),
            0x61 => self.adc_indexed_indirect(bus),
            0x65 => self.adc_zero_page(bus),
            0x66 => self.ror_zero_page(bus),
            0x68 => self.pla(bus),
            0x69 => self.adc_immediate(bus),
            0x6A => self.ror_accumulator(),
            0x6C => self.jmp_indirect(bus),
            0x6D => self.adc_absolute(bus),
            0x6E => self.ror_absolute(bus),
            
            0x70 => self.bvs(bus),
            0x71 => self.adc_indirect_indexed(bus),
            0x75 => self.adc_zero_page_x(bus),
            0x76 => self.ror_zero_page_x(bus),
            0x78 => self.sei(),
            0x79 => self.adc_absolute_y(bus),
            0x7D => self.adc_absolute_x(bus),
            0x7E => self.ror_absolute_x(bus),
            
            0x81 => self.sta_indexed_indirect(bus),
            0x83 => self.sax_indexed_indirect(bus),
            0x84 => self.sty_zero_page(bus),
            0x85 => self.sta_zero_page(bus),
            0x86 => self.stx_zero_page(bus),
            0x88 => self.dey(),
            0x8A => self.txa(),
            0x8C => self.sty_absolute(bus),
            0x8D => self.sta_absolute(bus),
            0x8E => self.stx_absolute(bus),
            
            0x90 => self.bcc(bus),
            0x91 => self.sta_indirect_indexed(bus),
            0x94 => self.sty_zero_page_x(bus),
            0x95 => self.sta_zero_page_x(bus),
            0x96 => self.stx_zero_page_y(bus),
            0x98 => self.tya(),
            0x99 => self.sta_absolute_y(bus),
            0x9A => self.txs(),
            0x9D => self.sta_absolute_x(bus),
            
            0xA0 => self.ldy_immediate(bus),
            0xA1 => self.lda_indexed_indirect(bus),
            0xA2 => self.ldx_immediate(bus),
            0xA4 => self.ldy_zero_page(bus),
            0xA5 => self.lda_zero_page(bus),
            0xA6 => self.ldx_zero_page(bus),
            0xA8 => self.tay(),
            0xA9 => self.lda_immediate(bus),
            0xAA => self.tax(),
            0xAC => self.ldy_absolute(bus),
            0xAD => self.lda_absolute(bus),
            0xAE => self.ldx_absolute(bus),
            
            0xB0 => self.bcs(bus),
            0xB1 => self.lda_indirect_indexed(bus),
            0xB4 => self.ldy_zero_page_x(bus),
            0xB5 => self.lda_zero_page_x(bus),
            0xB6 => self.ldx_zero_page_y(bus),
            0xB8 => self.clv(),
            0xB9 => self.lda_absolute_y(bus),
            0xBA => self.tsx(),
            0xBC => self.ldy_absolute_x(bus),
            0xBD => self.lda_absolute_x(bus),
            0xBE => self.ldx_absolute_y(bus),
            
            0xC0 => self.cpy_immediate(bus),
            0xC1 => self.cmp_indexed_indirect(bus),
            0xC4 => self.cpy_zero_page(bus),
            0xC5 => self.cmp_zero_page(bus),
            0xC6 => self.dec_zero_page(bus),
            0xC8 => self.iny(),
            0xC9 => self.cmp_immediate(bus),
            0xCA => self.dex(),
            0xCC => self.cpy_absolute(bus),
            0xCD => self.cmp_absolute(bus),
            0xCE => self.dec_absolute(bus),
            
            0xD0 => self.bne(bus),
            0xD1 => self.cmp_indirect_indexed(bus),
            0xD5 => self.cmp_zero_page_x(bus),
            0xD6 => self.dec_zero_page_x(bus),
            0xD8 => self.cld(),
            0xD9 => self.cmp_absolute_y(bus),
            0xDD => self.cmp_absolute_x(bus),
            0xDE => self.dec_absolute_x(bus),
            
            0xE0 => self.cpx_immediate(bus),
            0xE1 => self.sbc_indexed_indirect(bus),
            0xE4 => self.cpx_zero_page(bus),
            0xE5 => self.sbc_zero_page(bus),
            0xE6 => self.inc_zero_page(bus),
            0xE8 => self.inx(),
            0xE9 => self.sbc_immediate(bus),
            0xEA => self.nop(),
            0xEC => self.cpx_absolute(bus),
            0xED => self.sbc_absolute(bus),
            0xEE => self.inc_absolute(bus),
            
            0xF0 => self.beq(bus),
            0xF1 => self.sbc_indirect_indexed(bus),
            0xF5 => self.sbc_zero_page_x(bus),
            0xF6 => self.inc_zero_page_x(bus),
            0xF8 => self.sed(),
            0xF9 => self.sbc_absolute_y(bus),
            0xFD => self.sbc_absolute_x(bus),
            0xFE => self.inc_absolute_x(bus),
            
            _ => {
                // Some games use unofficial opcodes, try to continue
                match opcode {
                    // Common unofficial NOPs
                    0x1A | 0x3A | 0x5A | 0x7A | 0xDA | 0xFA => 2,
                    0x80 | 0x82 | 0x89 | 0xC2 | 0xE2 => {
                        self.read_byte(bus); // Consume immediate byte
                        2
                    }
                    0x04 | 0x44 | 0x64 => {
                        self.read_byte(bus); // Consume zero page byte
                        3
                    }
                    0x14 | 0x34 | 0x54 | 0x74 | 0xD4 | 0xF4 => {
                        self.read_byte(bus); // Consume zero page,X byte
                        4
                    }
                    0x0C => {
                        self.read_word(bus); // Consume absolute address - NOP absolute
                        4
                    }
                    0x1C | 0x3C | 0x5C | 0x7C | 0xDC | 0xFC => {
                        self.read_word(bus); // Consume absolute,X address - NOP absolute,X
                        4 // Could be 5 with page crossing, but we'll use 4
                    }
                    // Unofficial opcodes
                    0x07 => {
                        // SLO zero page - Shift Left, OR
                        let addr = self.read_byte(bus) as u16;
                        let value = bus.read(addr);
                        self.status.set(StatusFlags::CARRY, value & 0x80 != 0);
                        let shifted = value << 1;
                        bus.write(addr, shifted);
                        self.a |= shifted;
                        self.set_zero_negative_flags(self.a);
                        5
                    }
                    0x0B => {
                        // ANC immediate - AND, Copy N to C
                        let value = self.read_byte(bus);
                        self.a &= value;
                        self.set_zero_negative_flags(self.a);
                        self.status.set(StatusFlags::CARRY, self.status.contains(StatusFlags::NEGATIVE));
                        2
                    }
                    0x03 => {
                        // SLO (indirect,X) - Shift Left, OR
                        let addr = self.get_indexed_indirect_addr(bus);
                        let value = bus.read(addr);
                        self.status.set(StatusFlags::CARRY, value & 0x80 != 0);
                        let shifted = value << 1;
                        bus.write(addr, shifted);
                        self.a |= shifted;
                        self.set_zero_negative_flags(self.a);
                        8
                    }
                    0x0F => {
                        // SLO absolute - Shift Left, OR
                        let addr = self.read_word(bus);
                        let value = bus.read(addr);
                        self.status.set(StatusFlags::CARRY, value & 0x80 != 0);
                        let shifted = value << 1;
                        bus.write(addr, shifted);
                        self.a |= shifted;
                        self.set_zero_negative_flags(self.a);
                        6
                    }
                    0x7F => {
                        // RRA absolute,X - Rotate Right and Add
                        let addr = self.read_word(bus);
                        let effective_addr = addr.wrapping_add(self.x as u16);
                        let value = bus.read(effective_addr);
                        let carry = if self.status.contains(StatusFlags::CARRY) { 1 } else { 0 };
                        let new_carry = value & 0x01;
                        let rotated = (value >> 1) | (carry << 7);
                        bus.write(effective_addr, rotated);
                        self.status.set(StatusFlags::CARRY, new_carry != 0);
                        self.adc(rotated);
                        7
                    }
                    0xFB => {
                        // ISC absolute,Y - Increment and Subtract with Carry
                        let addr = self.read_word(bus);
                        let effective_addr = addr.wrapping_add(self.y as u16);
                        let value = bus.read(effective_addr);
                        let incremented = value.wrapping_add(1);
                        bus.write(effective_addr, incremented);
                        self.sbc(incremented);
                        7
                    }
                    0x43 => {
                        // SRE (indirect,X) - Shift Right and Exclusive OR
                        let addr = self.get_indexed_indirect_addr(bus);
                        let value = bus.read(addr);
                        let shifted = value >> 1;
                        bus.write(addr, shifted);
                        self.status.set(StatusFlags::CARRY, value & 0x01 != 0);
                        self.a ^= shifted;
                        self.set_zero_negative_flags(self.a);
                        8
                    }
                    0x4F => {
                        // SRE absolute - Shift Right and Exclusive OR
                        let addr = self.read_word(bus);
                        let value = bus.read(addr);
                        let shifted = value >> 1;
                        bus.write(addr, shifted);
                        self.status.set(StatusFlags::CARRY, value & 0x01 != 0);
                        self.a ^= shifted;
                        self.set_zero_negative_flags(self.a);
                        6
                    }
                    0x53 => {
                        // SRE (indirect),Y - Shift Right and Exclusive OR
                        let (addr, _page_crossed) = self.get_indirect_indexed_addr(bus);
                        let value = bus.read(addr);
                        let shifted = value >> 1;
                        bus.write(addr, shifted);
                        self.status.set(StatusFlags::CARRY, value & 0x01 != 0);
                        self.a ^= shifted;
                        self.set_zero_negative_flags(self.a);
                        8
                    }
                    0x57 => {
                        // SRE zero page,X - Shift Right and Exclusive OR
                        let addr = self.get_zero_page_x_addr(bus);
                        let value = bus.read(addr);
                        let shifted = value >> 1;
                        bus.write(addr, shifted);
                        self.status.set(StatusFlags::CARRY, value & 0x01 != 0);
                        self.a ^= shifted;
                        self.set_zero_negative_flags(self.a);
                        6
                    }
                    0x5B => {
                        // SRE absolute,Y - Shift Right and Exclusive OR
                        let addr = self.read_word(bus);
                        let effective_addr = addr.wrapping_add(self.y as u16);
                        let value = bus.read(effective_addr);
                        let shifted = value >> 1;
                        bus.write(effective_addr, shifted);
                        self.status.set(StatusFlags::CARRY, value & 0x01 != 0);
                        self.a ^= shifted;
                        self.set_zero_negative_flags(self.a);
                        7
                    }
                    0x87 => {
                        // SAX zero page - Store A AND X
                        let addr = self.read_byte(bus) as u16;
                        let value = self.a & self.x;
                        bus.write(addr, value);
                        3
                    }
                    0x8B => {
                        // XAA immediate - (A OR CONST) AND X AND immediate [EXTREMELY UNSTABLE]
                        // WARNING: Extremely unstable - behavior varies by temperature, chip, etc.
                        // Using magic constant 0xFF for basic compatibility
                        let value = self.read_byte(bus);
                        let magic_const = 0xFF; // Common fallback value
                        self.a = ((self.a | magic_const) & self.x) & value;
                        self.set_zero_negative_flags(self.a);
                        2
                    }
                    0x8F => {
                        // SAX absolute - Store A AND X
                        let addr = self.read_word(bus);
                        let value = self.a & self.x;
                        bus.write(addr, value);
                        4
                    }
                    // LAX unofficial opcodes (LDA + TAX combined)
                    0xA7 | 0xB7 | 0xAF | 0xBF | 0xA3 | 0xB3 | 0xAB => {
                        match opcode {
                            0xAB => {
                                // LAX immediate - Load A and X with memory
                                let value = self.read_byte(bus);
                                self.a = value;
                                self.x = value;
                                self.set_zero_negative_flags(value);
                                2
                            }
                            0xA7 => {
                                // LAX zero page
                                let addr = self.read_byte(bus) as u16;
                                let value = bus.read(addr);
                                self.a = value;
                                self.x = value;
                                self.set_zero_negative_flags(value);
                                3
                            }
                            0xB7 => {
                                // LAX zero page,Y
                                let addr = self.get_zero_page_y_addr(bus);
                                let value = bus.read(addr);
                                self.a = value;
                                self.x = value;
                                self.set_zero_negative_flags(value);
                                4
                            }
                            0xAF => {
                                // LAX absolute
                                let addr = self.read_word(bus);
                                let value = bus.read(addr);
                                self.a = value;
                                self.x = value;
                                self.set_zero_negative_flags(value);
                                4
                            }
                            0xBF => {
                                // LAX absolute,Y
                                let addr = self.read_word(bus);
                                let effective_addr = addr.wrapping_add(self.y as u16);
                                let value = bus.read(effective_addr);
                                self.a = value;
                                self.x = value;
                                self.set_zero_negative_flags(value);
                                4
                            }
                            0xA3 => {
                                // LAX (indirect,X)
                                let addr = self.get_indexed_indirect_addr(bus);
                                let value = bus.read(addr);
                                self.a = value;
                                self.x = value;
                                self.set_zero_negative_flags(value);
                                6
                            }
                            0xB3 => {
                                // LAX (indirect),Y
                                let (addr, _) = self.get_indirect_indexed_addr(bus);
                                let value = bus.read(addr);
                                self.a = value;
                                self.x = value;
                                self.set_zero_negative_flags(value);
                                5
                            }
                            _ => 2
                        }
                    }
                    // SHY unofficial opcode
                    // JAM/KIL opcodes - These should halt the CPU but we treat as NOP for compatibility
                    0x02 | 0x12 | 0x22 | 0x32 | 0x42 | 0x52 | 0x62 | 0x72 | 0x92 | 0xB2 | 0xD2 | 0xF2 => {
                        // JAM/KIL - Officially halt CPU, but treat as NOP for game compatibility
                        1 // 1 cycle minimal operation
                    }
                    0x9C => {
                        // SHY absolute,X - Store Y AND (high byte of original addr + 1) [UNSTABLE]
                        let addr = self.read_word(bus);
                        let effective_addr = addr.wrapping_add(self.x as u16);
                        let high_byte = (addr >> 8) as u8; // Use original addr, not effective_addr
                        let value = self.y & high_byte.wrapping_add(1);
                        bus.write(effective_addr, value);
                        5
                    }
                    0x13 => {
                        // SLO (indirect),Y - Shift Left, OR
                        let (addr, _page_crossed) = self.get_indirect_indexed_addr(bus);
                        let value = bus.read(addr);
                        self.status.set(StatusFlags::CARRY, value & 0x80 != 0);
                        let shifted = value << 1;
                        bus.write(addr, shifted);
                        self.a |= shifted;
                        self.set_zero_negative_flags(self.a);
                        8
                    }
                    0x17 => {
                        // SLO zero page,X - Shift Left, OR
                        let addr = self.get_zero_page_x_addr(bus);
                        let value = bus.read(addr);
                        self.status.set(StatusFlags::CARRY, value & 0x80 != 0);
                        let shifted = value << 1;
                        bus.write(addr, shifted);
                        self.a |= shifted;
                        self.set_zero_negative_flags(self.a);
                        6
                    }
                    0x1F => {
                        // SLO absolute,X - Shift Left, OR
                        let addr = self.read_word(bus);
                        let effective_addr = addr.wrapping_add(self.x as u16);
                        let value = bus.read(effective_addr);
                        self.status.set(StatusFlags::CARRY, value & 0x80 != 0);
                        let shifted = value << 1;
                        bus.write(effective_addr, shifted);
                        self.a |= shifted;
                        self.set_zero_negative_flags(self.a);
                        7
                    }
                    0x23 => {
                        // RLA (indirect,X) - Rotate Left, AND
                        let addr = self.get_indexed_indirect_addr(bus);
                        let value = bus.read(addr);
                        let carry = if self.status.contains(StatusFlags::CARRY) { 1 } else { 0 };
                        let rotated = (value << 1) | carry;
                        self.status.set(StatusFlags::CARRY, value & 0x80 != 0);
                        bus.write(addr, rotated);
                        self.a &= rotated;
                        self.set_zero_negative_flags(self.a);
                        8
                    }
                    0x27 => {
                        // RLA zero page - Rotate Left, AND
                        let addr = self.read_byte(bus) as u16;
                        let value = bus.read(addr);
                        let carry = if self.status.contains(StatusFlags::CARRY) { 1 } else { 0 };
                        let rotated = (value << 1) | carry;
                        self.status.set(StatusFlags::CARRY, value & 0x80 != 0);
                        bus.write(addr, rotated);
                        self.a &= rotated;
                        self.set_zero_negative_flags(self.a);
                        5
                    }
                    0x2B => {
                        // ANC immediate - AND, Copy N to C (duplicate of 0x0B)
                        let value = self.read_byte(bus);
                        self.a &= value;
                        self.set_zero_negative_flags(self.a);
                        self.status.set(StatusFlags::CARRY, self.status.contains(StatusFlags::NEGATIVE));
                        2
                    }
                    0x2F => {
                        // RLA absolute - Rotate Left, AND
                        let addr = self.read_word(bus);
                        let value = bus.read(addr);
                        let carry = if self.status.contains(StatusFlags::CARRY) { 1 } else { 0 };
                        let rotated = (value << 1) | carry;
                        self.status.set(StatusFlags::CARRY, value & 0x80 != 0);
                        bus.write(addr, rotated);
                        self.a &= rotated;
                        self.set_zero_negative_flags(self.a);
                        6
                    }
                    0x33 => {
                        // RLA (indirect),Y - Rotate Left, AND
                        let (addr, _page_crossed) = self.get_indirect_indexed_addr(bus);
                        let value = bus.read(addr);
                        let carry = if self.status.contains(StatusFlags::CARRY) { 1 } else { 0 };
                        let rotated = (value << 1) | carry;
                        self.status.set(StatusFlags::CARRY, value & 0x80 != 0);
                        bus.write(addr, rotated);
                        self.a &= rotated;
                        self.set_zero_negative_flags(self.a);
                        8
                    }
                    0x37 => {
                        // RLA zero page,X - Rotate Left, AND
                        let addr = self.get_zero_page_x_addr(bus);
                        let value = bus.read(addr);
                        let carry = if self.status.contains(StatusFlags::CARRY) { 1 } else { 0 };
                        let rotated = (value << 1) | carry;
                        self.status.set(StatusFlags::CARRY, value & 0x80 != 0);
                        bus.write(addr, rotated);
                        self.a &= rotated;
                        self.set_zero_negative_flags(self.a);
                        6
                    }
                    0x3B => {
                        // RLA absolute,Y - Rotate Left, AND
                        let addr = self.read_word(bus);
                        let effective_addr = addr.wrapping_add(self.y as u16);
                        let value = bus.read(effective_addr);
                        let carry = if self.status.contains(StatusFlags::CARRY) { 1 } else { 0 };
                        let rotated = (value << 1) | carry;
                        self.status.set(StatusFlags::CARRY, value & 0x80 != 0);
                        bus.write(effective_addr, rotated);
                        self.a &= rotated;
                        self.set_zero_negative_flags(self.a);
                        7
                    }
                    0x6B => {
                        // ARR immediate - AND with accumulator, then rotate right
                        let value = self.read_byte(bus);
                        self.a &= value;
                        let carry = if self.status.contains(StatusFlags::CARRY) { 0x80 } else { 0 };
                        let result = (self.a >> 1) | carry;
                        self.status.set(StatusFlags::CARRY, self.a & 0x01 != 0);
                        self.status.set(StatusFlags::OVERFLOW, ((result ^ (result << 1)) & 0x40) != 0);
                        self.a = result;
                        self.set_zero_negative_flags(self.a);
                        2
                    }
                    0x73 => {
                        // RRA (indirect),Y - Rotate Right, Add
                        let (addr, _page_crossed) = self.get_indirect_indexed_addr(bus);
                        let value = bus.read(addr);
                        let carry = if self.status.contains(StatusFlags::CARRY) { 0x80 } else { 0 };
                        let rotated = (value >> 1) | carry;
                        self.status.set(StatusFlags::CARRY, value & 0x01 != 0);
                        bus.write(addr, rotated);
                        self.adc(rotated);
                        8
                    }
                    0x7B => {
                        // RRA absolute,Y - Rotate Right, Add
                        let addr = self.read_word(bus);
                        let effective_addr = addr.wrapping_add(self.y as u16);
                        let value = bus.read(effective_addr);
                        let carry = if self.status.contains(StatusFlags::CARRY) { 0x80 } else { 0 };
                        let rotated = (value >> 1) | carry;
                        self.status.set(StatusFlags::CARRY, value & 0x01 != 0);
                        bus.write(effective_addr, rotated);
                        self.adc(rotated);
                        7
                    }
                    0xE3 => {
                        // ISC (indirect,X) - Increment, Subtract with Carry
                        let addr = self.get_indexed_indirect_addr(bus);
                        let value = bus.read(addr);
                        let incremented = value.wrapping_add(1);
                        bus.write(addr, incremented);
                        self.sbc(incremented);
                        8
                    }
                    0xF7 => {
                        // ISC zero page,X - Increment, Subtract with Carry
                        let addr = self.get_zero_page_x_addr(bus);
                        let value = bus.read(addr);
                        let incremented = value.wrapping_add(1);
                        bus.write(addr, incremented);
                        self.sbc(incremented);
                        6
                    }
                    0xEF => {
                        // ISC absolute - Increment, Subtract with Carry
                        let addr = self.read_word(bus);
                        let value = bus.read(addr);
                        let incremented = value.wrapping_add(1);
                        bus.write(addr, incremented);
                        self.sbc(incremented);
                        6
                    }
                    0x9F => {
                        // SHY absolute,X - Store Y AND (high byte of address + 1)
                        // Store Y AND (high byte of address + 1)
                        let addr = self.read_word(bus); // Read absolute address
                        let effective_addr = addr.wrapping_add(self.x as u16);
                        let high_byte = (effective_addr >> 8) as u8;
                        let store_value = self.y & high_byte.wrapping_add(1);
                        bus.write(effective_addr, store_value);
                        5 // 5 cycles for absolute,X with write
                    }
                    0xFF => {
                        // ISC absolute,X - unofficial opcode (duplicate implementation)
                        let addr = self.read_word(bus);
                        let effective_addr = addr.wrapping_add(self.x as u16);
                        let value = bus.read(effective_addr);
                        let incremented = value.wrapping_add(1);
                        bus.write(effective_addr, incremented);
                        self.sbc(incremented);
                        7
                    }
                    // Additional unofficial opcodes
                    0x3F => {
                        // RLA absolute,X - Rotate Left, AND
                        let addr = self.read_word(bus);
                        let effective_addr = addr.wrapping_add(self.x as u16);
                        let value = bus.read(effective_addr);
                        let carry = if self.status.contains(StatusFlags::CARRY) { 1 } else { 0 };
                        let rotated = (value << 1) | carry;
                        self.status.set(StatusFlags::CARRY, value & 0x80 != 0);
                        bus.write(effective_addr, rotated);
                        self.a &= rotated;
                        self.set_zero_negative_flags(self.a);
                        7
                    }
                    0x4B => {
                        // ALR immediate - AND + LSR
                        let value = self.read_byte(bus);
                        self.a &= value;
                        self.status.set(StatusFlags::CARRY, self.a & 0x01 != 0);
                        self.a >>= 1;
                        self.set_zero_negative_flags(self.a);
                        2
                    }
                    0x77 => {
                        // RRA zero page,X - Rotate Right, Add
                        let addr = self.get_zero_page_x_addr(bus);
                        let value = bus.read(addr);
                        let carry = if self.status.contains(StatusFlags::CARRY) { 0x80 } else { 0 };
                        let rotated = (value >> 1) | carry;
                        self.status.set(StatusFlags::CARRY, value & 0x01 != 0);
                        bus.write(addr, rotated);
                        self.adc(rotated);
                        6
                    }
                    0x93 => {
                        // SHA/AHX (indirect),Y - Store A AND X AND (H+1) [UNSTABLE]
                        // WARNING: This is an unstable instruction - behavior varies between 6502 chips
                        // Official spec: A & X & (high byte of target address + 1) → memory
                        let (addr, _) = self.get_indirect_indexed_addr(bus);
                        let high_byte = (addr >> 8) as u8;
                        let value = self.a & self.x & high_byte.wrapping_add(1);
                        bus.write(addr, value);
                        6
                    }
                    0x97 => {
                        // SAX zero page,Y - Store A AND X
                        let addr = self.get_zero_page_y_addr(bus);
                        let value = self.a & self.x;
                        bus.write(addr, value);
                        4
                    }
                    0x9B => {
                        // TAS/XAS absolute,Y - Transfer A AND X to SP, Store A AND X AND (H+1) [UNSTABLE]
                        // WARNING: This is an unstable instruction - behavior varies between 6502 chips
                        // WARNING: This can corrupt the stack pointer!
                        let addr = self.read_word(bus);
                        let effective_addr = addr.wrapping_add(self.y as u16);
                        // Only update SP if result is reasonable (>= 0x80)
                        let new_sp = self.a & self.x;
                        if new_sp >= 0x80 {
                            self.sp = new_sp;
                        }
                        let high_byte = (addr >> 8) as u8;
                        let value = self.a & self.x & high_byte.wrapping_add(1);
                        bus.write(effective_addr, value);
                        5
                    }
                    0x9E => {
                        // SHX absolute,Y - Store X AND (high byte + 1) [UNSTABLE]
                        let addr = self.read_word(bus);
                        let effective_addr = addr.wrapping_add(self.y as u16);
                        let high_byte = (addr >> 8) as u8;
                        let value = self.x & high_byte.wrapping_add(1);
                        bus.write(effective_addr, value);
                        5
                    }
                    0xD3 => {
                        // DCP (indirect),Y - Decrement, Compare
                        let (addr, _) = self.get_indirect_indexed_addr(bus);
                        let value = bus.read(addr);
                        let decremented = value.wrapping_sub(1);
                        bus.write(addr, decremented);
                        self.compare(self.a, decremented);
                        8
                    }
                    0xD7 => {
                        // DCP zero page,X - Decrement, Compare
                        let addr = self.get_zero_page_x_addr(bus);
                        let value = bus.read(addr);
                        let decremented = value.wrapping_sub(1);
                        bus.write(addr, decremented);
                        self.compare(self.a, decremented);
                        6
                    }
                    0xDB => {
                        // DCP absolute,Y - Decrement, Compare
                        let addr = self.read_word(bus);
                        let effective_addr = addr.wrapping_add(self.y as u16);
                        let value = bus.read(effective_addr);
                        let decremented = value.wrapping_sub(1);
                        bus.write(effective_addr, decremented);
                        self.compare(self.a, decremented);
                        7
                    }
                    0xDF => {
                        // DCP absolute,X - Decrement, Compare
                        let addr = self.read_word(bus);
                        let effective_addr = addr.wrapping_add(self.x as u16);
                        let value = bus.read(effective_addr);
                        let decremented = value.wrapping_sub(1);
                        bus.write(effective_addr, decremented);
                        self.compare(self.a, decremented);
                        7
                    }
                    0xE7 => {
                        // ISC zero page - Increment, Subtract with Carry
                        let addr = self.read_byte(bus) as u16;
                        let value = bus.read(addr);
                        let incremented = value.wrapping_add(1);
                        bus.write(addr, incremented);
                        self.sbc(incremented);
                        5
                    }
                    0xEB => {
                        // SBC immediate (unofficial duplicate of 0xE9)
                        let value = self.read_byte(bus);
                        self.sbc(value);
                        2
                    }
                    // More unofficial opcodes
                    0x1B => {
                        // SLO absolute,Y - Shift Left, OR
                        let addr = self.read_word(bus);
                        let effective_addr = addr.wrapping_add(self.y as u16);
                        let value = bus.read(effective_addr);
                        self.status.set(StatusFlags::CARRY, value & 0x80 != 0);
                        let shifted = value << 1;
                        bus.write(effective_addr, shifted);
                        self.a |= shifted;
                        self.set_zero_negative_flags(self.a);
                        7
                    }
                    0x5F => {
                        // SRE absolute,X - Shift Right, EOR
                        let addr = self.read_word(bus);
                        let effective_addr = addr.wrapping_add(self.x as u16);
                        let value = bus.read(effective_addr);
                        let shifted = value >> 1;
                        bus.write(effective_addr, shifted);
                        self.status.set(StatusFlags::CARRY, value & 0x01 != 0);
                        self.a ^= shifted;
                        self.set_zero_negative_flags(self.a);
                        7
                    }
                    0x63 => {
                        // RRA (indirect,X) - Rotate Right, Add
                        let base = self.read_byte(bus) as u16;
                        let addr_low = (base.wrapping_add(self.x as u16)) & 0xFF;
                        let addr_high = (addr_low + 1) & 0xFF;
                        let effective_addr = bus.read(addr_low) as u16 | ((bus.read(addr_high) as u16) << 8);
                        let value = bus.read(effective_addr);
                        let carry = if self.status.contains(StatusFlags::CARRY) { 0x80 } else { 0 };
                        let rotated = (value >> 1) | carry;
                        self.status.set(StatusFlags::CARRY, value & 0x01 != 0);
                        bus.write(effective_addr, rotated);
                        self.adc(rotated);
                        8
                    }
                    0x67 => {
                        // RRA zero page - Rotate Right, Add
                        let addr = self.read_byte(bus) as u16;
                        let value = bus.read(addr);
                        let carry = if self.status.contains(StatusFlags::CARRY) { 0x80 } else { 0 };
                        let rotated = (value >> 1) | carry;
                        self.status.set(StatusFlags::CARRY, value & 0x01 != 0);
                        bus.write(addr, rotated);
                        self.adc(rotated);
                        5
                    }
                    0x6F => {
                        // RRA absolute - Rotate Right, Add
                        let addr = self.read_word(bus);
                        let value = bus.read(addr);
                        let carry = if self.status.contains(StatusFlags::CARRY) { 0x80 } else { 0 };
                        let rotated = (value >> 1) | carry;
                        self.status.set(StatusFlags::CARRY, value & 0x01 != 0);
                        bus.write(addr, rotated);
                        self.adc(rotated);
                        6
                    }
                    0xBB => {
                        // LAS absolute,Y - Load A, X, S with memory AND S
                        let addr = self.read_word(bus);
                        let effective_addr = addr.wrapping_add(self.y as u16);
                        let value = bus.read(effective_addr) & self.sp;
                        self.a = value;
                        self.x = value;
                        // Only update SP if result is reasonable (>= 0x80)
                        if value >= 0x80 {
                            self.sp = value;
                        }
                        self.set_zero_negative_flags(value);
                        4
                    }
                    0xC3 => {
                        // DCP (indirect,X) - Decrement, Compare
                        let addr = self.get_indexed_indirect_addr(bus);
                        let value = bus.read(addr);
                        let decremented = value.wrapping_sub(1);
                        bus.write(addr, decremented);
                        self.compare(self.a, decremented);
                        8
                    }
                    0xC7 => {
                        // DCP zero page - Decrement, Compare
                        let addr = self.read_byte(bus) as u16;
                        let value = bus.read(addr);
                        let decremented = value.wrapping_sub(1);
                        bus.write(addr, decremented);
                        self.compare(self.a, decremented);
                        5
                    }
                    0xCF => {
                        // DCP absolute - Decrement, Compare
                        let addr = self.read_word(bus);
                        let value = bus.read(addr);
                        let decremented = value.wrapping_sub(1);
                        bus.write(addr, decremented);
                        self.compare(self.a, decremented);
                        6
                    }
                    0xCB => {
                        // AXS immediate - AND X register with accumulator, subtract immediate
                        let value = self.read_byte(bus);
                        let and_result = self.a & self.x;
                        let result = and_result.wrapping_sub(value);
                        self.x = result;
                        self.status.set(StatusFlags::CARRY, and_result >= value);
                        self.set_zero_negative_flags(result);
                        2
                    }
                    0xF3 => {
                        // ISC (indirect),Y - Increment, Subtract with Carry
                        let (addr, _) = self.get_indirect_indexed_addr(bus);
                        let value = bus.read(addr);
                        let incremented = value.wrapping_add(1);
                        bus.write(addr, incremented);
                        self.sbc(incremented);
                        8
                    }
                    _ => {
                        log::error!("Halting on truly unknown opcode: 0x{:02X} at PC: 0x{:04X}", opcode, self.pc.wrapping_sub(1));
                        1 // Minimal cycles to avoid complete freeze
                    }
                }
            }
        }
    }

    fn read_byte(&mut self, bus: &mut dyn CpuBus) -> u8 {
        let byte = bus.read(self.pc);
        self.pc = self.pc.wrapping_add(1);
        byte
    }

    fn read_word(&mut self, bus: &mut dyn CpuBus) -> u16 {
        let low = self.read_byte(bus) as u16;
        let high = self.read_byte(bus) as u16;
        (high << 8) | low
    }

    fn push(&mut self, bus: &mut dyn CpuBus, value: u8) {
        // Push to stack
        let addr = 0x0100 | self.sp as u16;
        bus.write(addr, value);
        self.sp = self.sp.wrapping_sub(1);
    }

    fn pull(&mut self, bus: &mut dyn CpuBus) -> u8 {
        // Pull from stack
        self.sp = self.sp.wrapping_add(1);
        let addr = 0x0100 | self.sp as u16;
        let value = bus.read(addr);
        value
    }

    fn set_zero_negative_flags(&mut self, value: u8) {
        self.status.set(StatusFlags::ZERO, value == 0);
        self.status.set(StatusFlags::NEGATIVE, value & 0x80 != 0);
    }

    fn branch(&mut self, bus: &mut dyn CpuBus, condition: bool) -> u8 {
        // Branch instructions: read offset byte and conditionally branch
        let branch_pc = self.pc.wrapping_sub(1); // PC where branch instruction was located
        let offset = self.read_byte(bus) as i8;
        if condition {
            let new_pc = self.pc.wrapping_add(offset as u16);
            
            // Page crossing check should use the branch instruction PC vs destination PC
            let cycles = if (branch_pc & 0xFF00) != (new_pc & 0xFF00) { 4 } else { 3 };
            self.pc = new_pc;
            cycles
        } else {
            2
        }
    }

}

pub trait CpuBus {
    fn read(&mut self, addr: u16) -> u8;
    fn write(&mut self, addr: u16, data: u8);
    
    // Game-specific protection methods (with default implementations)
    fn check_game_specific_cpu_protection(&self, _pc: u16, _sp: u8, _cycles: u64) -> Option<(u16, u8)> {
        None
    }
    
    fn check_game_specific_brk_protection(&self, _pc: u16, _sp: u8, _cycles: u64) -> Option<(u16, u8)> {
        None
    }
    
    fn is_goonies(&self) -> bool {
        false
    }
}

// New trait for tick-enabled bus operations
pub trait CpuBusWithTick: CpuBus {
    fn tick(&mut self, cycles: u8) -> bool; // Returns true if NMI triggered
}