//! Trait representing the minimal bus interface required by the 65C816 core.

pub trait CpuBus {
    fn read_u8(&mut self, addr: u32) -> u8;
    fn write_u8(&mut self, addr: u32, value: u8);
    fn opcode_memory_penalty(&mut self, _addr: u32) -> u8 {
        0
    }
    fn poll_nmi(&mut self) -> bool {
        false
    }
    fn read_u16(&mut self, addr: u32) -> u16 {
        let lo = self.read_u8(addr) as u16;
        let hi = self.read_u8(addr.wrapping_add(1)) as u16;
        (hi << 8) | lo
    }
    fn write_u16(&mut self, addr: u32, value: u16) {
        self.write_u8(addr, (value & 0xFF) as u8);
        self.write_u8(addr.wrapping_add(1), (value >> 8) as u8);
    }
    fn acknowledge_nmi(&mut self) {}
    fn poll_irq(&mut self) -> bool;
}
