use super::*;

#[cfg(test)]
mod addressing_mode_tests {
    use super::*;
    
    #[test]
    fn test_zero_page_addressing() {
        let (mut cpu, mut bus) = setup_cpu();
        cpu.reset(&mut bus);
        
        // Set value at zero page address
        bus.write(0x42, 0xAB);
        
        // LDA $42 (zero page)
        bus.load_program(&[0xA5, 0x42], 0x8000);
        cpu.pc = 0x8000;
        
        let cycles = cpu.step(&mut bus);
        
        assert_eq!(cpu.a, 0xAB);
        assert_eq!(cycles, 3);
    }
    
    #[test]
    fn test_zero_page_x_addressing() {
        let (mut cpu, mut bus) = setup_cpu();
        cpu.reset(&mut bus);
        
        cpu.x = 0x10;
        bus.write(0x52, 0xCD); // 0x42 + 0x10
        
        // LDA $42,X
        bus.load_program(&[0xB5, 0x42], 0x8000);
        cpu.pc = 0x8000;
        
        let cycles = cpu.step(&mut bus);
        
        assert_eq!(cpu.a, 0xCD);
        assert_eq!(cycles, 4);
    }
    
    #[test]
    fn test_zero_page_x_wraparound() {
        let (mut cpu, mut bus) = setup_cpu();
        cpu.reset(&mut bus);
        
        cpu.x = 0xFF;
        bus.write(0x41, 0xEF); // (0x42 + 0xFF) & 0xFF = 0x41
        
        // LDA $42,X
        bus.load_program(&[0xB5, 0x42], 0x8000);
        cpu.pc = 0x8000;
        
        cpu.step(&mut bus);
        
        assert_eq!(cpu.a, 0xEF);
    }
    
    #[test]
    fn test_absolute_addressing() {
        let (mut cpu, mut bus) = setup_cpu();
        cpu.reset(&mut bus);
        
        bus.write(0x1234, 0x56);
        
        // LDA $1234
        bus.load_program(&[0xAD, 0x34, 0x12], 0x8000);
        cpu.pc = 0x8000;
        
        let cycles = cpu.step(&mut bus);
        
        assert_eq!(cpu.a, 0x56);
        assert_eq!(cycles, 4);
    }
    
    #[test]
    fn test_absolute_x_addressing() {
        let (mut cpu, mut bus) = setup_cpu();
        cpu.reset(&mut bus);
        
        cpu.x = 0x10;
        bus.write(0x1244, 0x78); // 0x1234 + 0x10
        
        // LDA $1234,X
        bus.load_program(&[0xBD, 0x34, 0x12], 0x8000);
        cpu.pc = 0x8000;
        
        let cycles = cpu.step(&mut bus);
        
        assert_eq!(cpu.a, 0x78);
        assert_eq!(cycles, 4); // No page cross
    }
    
    #[test]
    fn test_absolute_x_page_cross() {
        let (mut cpu, mut bus) = setup_cpu();
        cpu.reset(&mut bus);
        
        cpu.x = 0xFF;
        bus.write(0x1333, 0x9A); // 0x1234 + 0xFF = 0x1333 (page cross)
        
        // LDA $1234,X
        bus.load_program(&[0xBD, 0x34, 0x12], 0x8000);
        cpu.pc = 0x8000;
        
        let cycles = cpu.step(&mut bus);
        
        assert_eq!(cpu.a, 0x9A);
        assert_eq!(cycles, 5); // Page cross penalty
    }
    
    #[test]
    fn test_absolute_y_addressing() {
        let (mut cpu, mut bus) = setup_cpu();
        cpu.reset(&mut bus);
        
        cpu.y = 0x20;
        bus.write(0x1254, 0xBC); // 0x1234 + 0x20
        
        // LDA $1234,Y
        bus.load_program(&[0xB9, 0x34, 0x12], 0x8000);
        cpu.pc = 0x8000;
        
        let cycles = cpu.step(&mut bus);
        
        assert_eq!(cpu.a, 0xBC);
        assert_eq!(cycles, 4);
    }
    
    #[test]
    fn test_indexed_indirect_x() {
        let (mut cpu, mut bus) = setup_cpu();
        cpu.reset(&mut bus);
        
        cpu.x = 0x04;
        // Set pointer at ($40,X) = $44
        bus.write(0x44, 0x00);
        bus.write(0x45, 0x20);
        // Set value at $2000
        bus.write(0x2000, 0xDE);
        
        // LDA ($40,X)
        bus.load_program(&[0xA1, 0x40], 0x8000);
        cpu.pc = 0x8000;
        
        let cycles = cpu.step(&mut bus);
        
        assert_eq!(cpu.a, 0xDE);
        assert_eq!(cycles, 6);
    }
    
    #[test]
    fn test_indexed_indirect_x_wraparound() {
        let (mut cpu, mut bus) = setup_cpu();
        cpu.reset(&mut bus);
        
        cpu.x = 0xFF;
        // ($FF,X) = ($FF + $FF) & $FF = $FE
        bus.write(0xFE, 0x00);
        bus.write(0xFF, 0x30);
        bus.write(0x3000, 0xAA);
        
        // LDA ($FF,X)
        bus.load_program(&[0xA1, 0xFF], 0x8000);
        cpu.pc = 0x8000;
        
        cpu.step(&mut bus);
        
        assert_eq!(cpu.a, 0xAA);
    }
    
    #[test]
    fn test_indirect_indexed_y() {
        let (mut cpu, mut bus) = setup_cpu();
        cpu.reset(&mut bus);
        
        cpu.y = 0x10;
        // Set pointer at $40
        bus.write(0x40, 0x00);
        bus.write(0x41, 0x20);
        // Value at $2000 + Y = $2010
        bus.write(0x2010, 0xF0);
        
        // LDA ($40),Y
        bus.load_program(&[0xB1, 0x40], 0x8000);
        cpu.pc = 0x8000;
        
        let cycles = cpu.step(&mut bus);
        
        assert_eq!(cpu.a, 0xF0);
        assert_eq!(cycles, 5); // No page cross
    }
    
    #[test]
    fn test_indirect_indexed_y_page_cross() {
        let (mut cpu, mut bus) = setup_cpu();
        cpu.reset(&mut bus);
        
        cpu.y = 0xFF;
        // Set pointer at $40
        bus.write(0x40, 0x01);
        bus.write(0x41, 0x20);
        // Value at $2001 + Y = $2100 (page cross)
        bus.write(0x2100, 0x12);
        
        // LDA ($40),Y
        bus.load_program(&[0xB1, 0x40], 0x8000);
        cpu.pc = 0x8000;
        
        let cycles = cpu.step(&mut bus);
        
        assert_eq!(cpu.a, 0x12);
        assert_eq!(cycles, 6); // Page cross penalty
    }
    
    #[test]
    fn test_jmp_indirect() {
        let (mut cpu, mut bus) = setup_cpu();
        cpu.reset(&mut bus);
        
        // Set indirect address
        bus.write(0x2000, 0x34);
        bus.write(0x2001, 0x12);
        
        // JMP ($2000)
        bus.load_program(&[0x6C, 0x00, 0x20], 0x8000);
        cpu.pc = 0x8000;
        
        let cycles = cpu.step(&mut bus);
        
        assert_eq!(cpu.pc, 0x1234);
        assert_eq!(cycles, 5);
    }
    
    #[test]
    fn test_jmp_indirect_bug() {
        let (mut cpu, mut bus) = setup_cpu();
        cpu.reset(&mut bus);
        
        // Test the 6502 JMP indirect bug at page boundary
        bus.write(0x20FF, 0x34);
        bus.write(0x2000, 0x12); // Bug: should read from 0x2100, but reads from 0x2000
        
        // JMP ($20FF)
        bus.load_program(&[0x6C, 0xFF, 0x20], 0x8000);
        cpu.pc = 0x8000;
        
        cpu.step(&mut bus);
        
        assert_eq!(cpu.pc, 0x1234);
    }
    
    #[test]
    fn test_relative_addressing_forward() {
        let (mut cpu, mut bus) = setup_cpu();
        cpu.reset(&mut bus);
        
        // Test forward branch
        bus.load_program(&[0x10, 0x0A], 0x8000); // BPL +10
        cpu.pc = 0x8000;
        cpu.status.remove(StatusFlags::NEGATIVE);
        
        cpu.step(&mut bus);
        
        assert_eq!(cpu.pc, 0x800C); // 0x8002 + 0x0A
    }
    
    #[test]
    fn test_relative_addressing_backward() {
        let (mut cpu, mut bus) = setup_cpu();
        cpu.reset(&mut bus);
        
        // Test backward branch (two's complement)
        bus.load_program(&[0x10, 0xFC], 0x8000); // BPL -4
        cpu.pc = 0x8000;
        cpu.status.remove(StatusFlags::NEGATIVE);
        
        cpu.step(&mut bus);
        
        assert_eq!(cpu.pc, 0x7FFE); // 0x8002 - 4
    }
    
    #[test]
    fn test_implied_addressing() {
        let (mut cpu, mut bus) = setup_cpu();
        cpu.reset(&mut bus);
        
        // Test implied addressing (no operands)
        bus.load_program(&[0xEA], 0x8000); // NOP
        cpu.pc = 0x8000;
        
        let cycles = cpu.step(&mut bus);
        
        assert_eq!(cpu.pc, 0x8001);
        assert_eq!(cycles, 2);
    }
    
    #[test]
    fn test_accumulator_addressing() {
        let (mut cpu, mut bus) = setup_cpu();
        cpu.reset(&mut bus);
        
        cpu.a = 0x40;
        
        // Test accumulator addressing
        bus.load_program(&[0x0A], 0x8000); // ASL A
        cpu.pc = 0x8000;
        
        cpu.step(&mut bus);
        
        assert_eq!(cpu.a, 0x80);
        assert_eq!(cpu.pc, 0x8001);
    }
}