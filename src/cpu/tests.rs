use super::*;

#[path = "addressing_tests.rs"]
mod addressing_mode_tests;

struct TestBus {
    memory: [u8; 0x10000],
}

impl TestBus {
    fn new() -> Self {
        Self {
            memory: [0; 0x10000],
        }
    }
    
    fn load_program(&mut self, program: &[u8], start_addr: u16) {
        for (i, &byte) in program.iter().enumerate() {
            self.memory[start_addr as usize + i] = byte;
        }
    }
}

impl CpuBus for TestBus {
    fn read(&mut self, addr: u16) -> u8 {
        self.memory[addr as usize]
    }
    
    fn write(&mut self, addr: u16, data: u8) {
        self.memory[addr as usize] = data;
    }
}

fn setup_cpu() -> (Cpu, TestBus) {
        let cpu = Cpu::new();
        let mut bus = TestBus::new();
        // Set reset vector
        bus.write(0xFFFC, 0x00);
        bus.write(0xFFFD, 0x80);
        (cpu, bus)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_lda_immediate() {
        let (mut cpu, mut bus) = setup_cpu();
        cpu.reset(&mut bus);
        
        // LDA #$42
        bus.load_program(&[0xA9, 0x42], 0x8000);
        cpu.pc = 0x8000;
        
        let cycles = cpu.step(&mut bus);
        
        assert_eq!(cpu.a, 0x42);
        assert_eq!(cpu.pc, 0x8002);
        assert_eq!(cycles, 2);
        assert!(!cpu.status.contains(StatusFlags::ZERO));
        assert!(!cpu.status.contains(StatusFlags::NEGATIVE));
    }
    
    #[test]
    fn test_lda_zero_flag() {
        let (mut cpu, mut bus) = setup_cpu();
        cpu.reset(&mut bus);
        
        // LDA #$00
        bus.load_program(&[0xA9, 0x00], 0x8000);
        cpu.pc = 0x8000;
        
        cpu.step(&mut bus);
        
        assert_eq!(cpu.a, 0x00);
        assert!(cpu.status.contains(StatusFlags::ZERO));
        assert!(!cpu.status.contains(StatusFlags::NEGATIVE));
    }
    
    #[test]
    fn test_lda_negative_flag() {
        let (mut cpu, mut bus) = setup_cpu();
        cpu.reset(&mut bus);
        
        // LDA #$80
        bus.load_program(&[0xA9, 0x80], 0x8000);
        cpu.pc = 0x8000;
        
        cpu.step(&mut bus);
        
        assert_eq!(cpu.a, 0x80);
        assert!(!cpu.status.contains(StatusFlags::ZERO));
        assert!(cpu.status.contains(StatusFlags::NEGATIVE));
    }
    
    #[test]
    fn test_sta_zero_page() {
        let (mut cpu, mut bus) = setup_cpu();
        cpu.reset(&mut bus);
        
        cpu.a = 0x42;
        // STA $10
        bus.load_program(&[0x85, 0x10], 0x8000);
        cpu.pc = 0x8000;
        
        let cycles = cpu.step(&mut bus);
        
        assert_eq!(bus.read(0x0010), 0x42);
        assert_eq!(cpu.pc, 0x8002);
        assert_eq!(cycles, 3);
    }
    
    #[test]
    fn test_ldx_ldy() {
        let (mut cpu, mut bus) = setup_cpu();
        cpu.reset(&mut bus);
        
        // LDX #$10, LDY #$20
        bus.load_program(&[0xA2, 0x10, 0xA0, 0x20], 0x8000);
        cpu.pc = 0x8000;
        
        cpu.step(&mut bus);
        assert_eq!(cpu.x, 0x10);
        
        cpu.step(&mut bus);
        assert_eq!(cpu.y, 0x20);
    }
    
    #[test]
    fn test_inx_iny() {
        let (mut cpu, mut bus) = setup_cpu();
        cpu.reset(&mut bus);
        
        cpu.x = 0x10;
        cpu.y = 0x20;
        
        // INX, INY
        bus.load_program(&[0xE8, 0xC8], 0x8000);
        cpu.pc = 0x8000;
        
        cpu.step(&mut bus);
        assert_eq!(cpu.x, 0x11);
        
        cpu.step(&mut bus);
        assert_eq!(cpu.y, 0x21);
    }
    
    #[test]
    fn test_inx_wraparound() {
        let (mut cpu, mut bus) = setup_cpu();
        cpu.reset(&mut bus);
        
        cpu.x = 0xFF;
        
        // INX
        bus.load_program(&[0xE8], 0x8000);
        cpu.pc = 0x8000;
        
        cpu.step(&mut bus);
        assert_eq!(cpu.x, 0x00);
        assert!(cpu.status.contains(StatusFlags::ZERO));
    }
    
    #[test]
    fn test_adc_no_carry() {
        let (mut cpu, mut bus) = setup_cpu();
        cpu.reset(&mut bus);
        
        cpu.a = 0x10;
        cpu.status.remove(StatusFlags::CARRY);
        
        // ADC #$20
        bus.load_program(&[0x69, 0x20], 0x8000);
        cpu.pc = 0x8000;
        
        cpu.step(&mut bus);
        assert_eq!(cpu.a, 0x30);
        assert!(!cpu.status.contains(StatusFlags::CARRY));
        assert!(!cpu.status.contains(StatusFlags::OVERFLOW));
    }
    
    #[test]
    fn test_adc_with_carry() {
        let (mut cpu, mut bus) = setup_cpu();
        cpu.reset(&mut bus);
        
        cpu.a = 0xFF;
        cpu.status.remove(StatusFlags::CARRY);
        
        // ADC #$01
        bus.load_program(&[0x69, 0x01], 0x8000);
        cpu.pc = 0x8000;
        
        cpu.step(&mut bus);
        assert_eq!(cpu.a, 0x00);
        assert!(cpu.status.contains(StatusFlags::CARRY));
        assert!(cpu.status.contains(StatusFlags::ZERO));
    }
    
    #[test]
    fn test_sbc() {
        let (mut cpu, mut bus) = setup_cpu();
        cpu.reset(&mut bus);
        
        cpu.a = 0x50;
        cpu.status.insert(StatusFlags::CARRY); // No borrow
        
        // SBC #$20
        bus.load_program(&[0xE9, 0x20], 0x8000);
        cpu.pc = 0x8000;
        
        cpu.step(&mut bus);
        assert_eq!(cpu.a, 0x30);
        assert!(cpu.status.contains(StatusFlags::CARRY)); // No borrow occurred
    }
    
    #[test]
    fn test_jmp_absolute() {
        let (mut cpu, mut bus) = setup_cpu();
        cpu.reset(&mut bus);
        
        // JMP $1234
        bus.load_program(&[0x4C, 0x34, 0x12], 0x8000);
        cpu.pc = 0x8000;
        
        let cycles = cpu.step(&mut bus);
        
        assert_eq!(cpu.pc, 0x1234);
        assert_eq!(cycles, 3);
    }
    
    #[test]
    fn test_jsr_rts() {
        let (mut cpu, mut bus) = setup_cpu();
        cpu.reset(&mut bus);
        
        // JSR $9000
        bus.load_program(&[0x20, 0x00, 0x90], 0x8000);
        // RTS at $9000
        bus.load_program(&[0x60], 0x9000);
        cpu.pc = 0x8000;
        cpu.sp = 0xFF; // Reset stack pointer
        
        // Execute JSR
        cpu.step(&mut bus);
        assert_eq!(cpu.pc, 0x9000);
        assert_eq!(cpu.sp, 0xFD); // Stack pointer decremented by 2
        
        // Check return address on stack (PC+2)
        assert_eq!(bus.read(0x01FF), 0x80); // High byte
        assert_eq!(bus.read(0x01FE), 0x02); // Low byte
        
        // Execute RTS
        cpu.step(&mut bus);
        assert_eq!(cpu.pc, 0x8003);
        assert_eq!(cpu.sp, 0xFF);
    }
    
    #[test]
    fn test_beq_taken() {
        let (mut cpu, mut bus) = setup_cpu();
        cpu.reset(&mut bus);
        
        cpu.status.insert(StatusFlags::ZERO);
        
        // BEQ $10 (relative +16)
        bus.load_program(&[0xF0, 0x10], 0x8000);
        cpu.pc = 0x8000;
        
        let cycles = cpu.step(&mut bus);
        
        assert_eq!(cpu.pc, 0x8012); // 0x8002 + 0x10
        assert_eq!(cycles, 3); // Branch taken
    }
    
    #[test]
    fn test_beq_not_taken() {
        let (mut cpu, mut bus) = setup_cpu();
        cpu.reset(&mut bus);
        
        cpu.status.remove(StatusFlags::ZERO);
        
        // BEQ $10
        bus.load_program(&[0xF0, 0x10], 0x8000);
        cpu.pc = 0x8000;
        
        let cycles = cpu.step(&mut bus);
        
        assert_eq!(cpu.pc, 0x8002);
        assert_eq!(cycles, 2); // Branch not taken
    }
    
    #[test]
    fn test_cmp() {
        let (mut cpu, mut bus) = setup_cpu();
        cpu.reset(&mut bus);
        
        cpu.a = 0x30;
        
        // CMP #$30
        bus.load_program(&[0xC9, 0x30], 0x8000);
        cpu.pc = 0x8000;
        
        cpu.step(&mut bus);
        
        assert!(cpu.status.contains(StatusFlags::CARRY)); // A >= M
        assert!(cpu.status.contains(StatusFlags::ZERO));  // A == M
        assert!(!cpu.status.contains(StatusFlags::NEGATIVE));
    }
    
    #[test]
    fn test_stack_operations() {
        let (mut cpu, mut bus) = setup_cpu();
        cpu.reset(&mut bus);
        
        cpu.a = 0x42;
        cpu.sp = 0xFF;
        
        // PHA, PLA
        bus.load_program(&[0x48, 0x68], 0x8000);
        cpu.pc = 0x8000;
        
        // Push A
        cpu.step(&mut bus);
        assert_eq!(cpu.sp, 0xFE);
        assert_eq!(bus.read(0x01FF), 0x42);
        
        cpu.a = 0x00; // Clear A
        
        // Pull A
        cpu.step(&mut bus);
        assert_eq!(cpu.a, 0x42);
        assert_eq!(cpu.sp, 0xFF);
    }
    
    #[test]
    fn test_nmi_interrupt() {
        let (mut cpu, mut bus) = setup_cpu();
        
        // Set NMI vector
        bus.write(0xFFFA, 0x00);
        bus.write(0xFFFB, 0x90);
        
        cpu.reset(&mut bus);
        cpu.pc = 0x8000;
        cpu.status = StatusFlags::from_bits_truncate(0x24); // Some flags set
        cpu.sp = 0xFF;
        
        cpu.nmi(&mut bus);
        
        // Check PC jumped to NMI vector
        assert_eq!(cpu.pc, 0x9000);
        
        // Check stack has old PC and status
        assert_eq!(bus.read(0x01FF), 0x80); // PC high
        assert_eq!(bus.read(0x01FE), 0x00); // PC low
        assert_eq!(bus.read(0x01FD), 0x24); // Status
        assert_eq!(cpu.sp, 0xFC);
        
        // Check interrupt disable flag set
        assert!(cpu.status.contains(StatusFlags::INTERRUPT_DISABLE));
    }
    
    #[test]
    fn test_bit_instruction() {
        let (mut cpu, mut bus) = setup_cpu();
        cpu.reset(&mut bus);
        
        cpu.a = 0x0F;
        bus.write(0x10, 0xF0);
        
        // BIT $10
        bus.load_program(&[0x24, 0x10], 0x8000);
        cpu.pc = 0x8000;
        
        cpu.step(&mut bus);
        
        // A & M = 0x0F & 0xF0 = 0x00
        assert!(cpu.status.contains(StatusFlags::ZERO));
        
        // Bit 7 of M
        assert!(cpu.status.contains(StatusFlags::NEGATIVE));
        
        // Bit 6 of M
        assert!(cpu.status.contains(StatusFlags::OVERFLOW));
    }
    
    #[test]
    fn test_and_or_eor() {
        let (mut cpu, mut bus) = setup_cpu();
        cpu.reset(&mut bus);
        
        // Test AND
        cpu.a = 0xFF;
        bus.load_program(&[0x29, 0x0F], 0x8000); // AND #$0F
        cpu.pc = 0x8000;
        cpu.step(&mut bus);
        assert_eq!(cpu.a, 0x0F);
        
        // Test ORA
        bus.load_program(&[0x09, 0xF0], 0x8002); // ORA #$F0
        cpu.step(&mut bus);
        assert_eq!(cpu.a, 0xFF);
        
        // Test EOR
        bus.load_program(&[0x49, 0xFF], 0x8004); // EOR #$FF
        cpu.step(&mut bus);
        assert_eq!(cpu.a, 0x00);
        assert!(cpu.status.contains(StatusFlags::ZERO));
    }
    
    #[test]
    fn test_shift_operations() {
        let (mut cpu, mut bus) = setup_cpu();
        cpu.reset(&mut bus);
        
        // Test ASL accumulator
        cpu.a = 0x81;
        bus.load_program(&[0x0A], 0x8000); // ASL A
        cpu.pc = 0x8000;
        cpu.step(&mut bus);
        assert_eq!(cpu.a, 0x02);
        assert!(cpu.status.contains(StatusFlags::CARRY));
        
        // Test LSR accumulator
        cpu.a = 0x81;
        bus.load_program(&[0x4A], 0x8001); // LSR A
        cpu.step(&mut bus);
        assert_eq!(cpu.a, 0x40);
        assert!(cpu.status.contains(StatusFlags::CARRY));
        
        // Test ROL with carry
        cpu.a = 0x80;
        cpu.status.insert(StatusFlags::CARRY);
        bus.load_program(&[0x2A], 0x8002); // ROL A
        cpu.step(&mut bus);
        assert_eq!(cpu.a, 0x01);
        assert!(cpu.status.contains(StatusFlags::CARRY));
        
        // Test ROR with carry
        cpu.a = 0x01;
        cpu.status.insert(StatusFlags::CARRY);
        bus.load_program(&[0x6A], 0x8003); // ROR A
        cpu.step(&mut bus);
        assert_eq!(cpu.a, 0x80);
        assert!(cpu.status.contains(StatusFlags::CARRY));
    }
}