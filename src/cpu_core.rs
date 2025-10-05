#![allow(dead_code)]
//! Shared 65C816 CPU core implementation.
//!
//! This module provides the complete 65C816 instruction set execution
//! that can be used by both S-CPU and SA-1 through bus abstraction.

use crate::{cpu::StatusFlags, cpu_bus::CpuBus, debug_flags};

#[derive(Debug, Clone)]
pub struct FetchResult {
    pub opcode: u8,
    pub memspeed_penalty: u8,
    pub pc_before: u16,
    pub full_addr: u32,
}

#[derive(Debug, Clone)]
pub struct StepResult {
    pub cycles: u8,
    pub fetch: FetchResult,
}

#[derive(Debug, Clone)]
pub struct Core {
    pub state: CoreState,
}

#[derive(Debug, Clone)]
pub struct CoreState {
    pub a: u16,
    pub x: u16,
    pub y: u16,
    pub sp: u16,
    pub dp: u16,
    pub db: u8,
    pub pb: u8,
    pub pc: u16,
    pub p: StatusFlags,
    pub emulation_mode: bool,
    pub cycles: u64,
    pub waiting_for_irq: bool,
    pub stopped: bool,
}

impl CoreState {
    pub fn new(default_flags: StatusFlags, emulation_mode: bool) -> Self {
        Self {
            a: 0,
            x: 0,
            y: 0,
            sp: 0x01FF,
            dp: 0,
            db: 0,
            pb: 0,
            pc: 0,
            p: default_flags,
            emulation_mode,
            cycles: 0,
            waiting_for_irq: false,
            stopped: false,
        }
    }
}

impl Core {
    pub fn new(default_flags: StatusFlags, emulation_mode: bool) -> Self {
        Self {
            state: CoreState::new(default_flags, emulation_mode),
        }
    }

    pub fn reset(&mut self, default_flags: StatusFlags, emulation_mode: bool) {
        self.state = CoreState::new(default_flags, emulation_mode);
    }

    pub fn step<B: CpuBus>(&mut self, bus: &mut B) -> StepResult {
        let fetch = fetch_opcode_generic(&mut self.state, bus);
        let opcode = fetch.opcode;
        let mut cycles = execute_instruction_generic(&mut self.state, opcode, bus);
        if fetch.memspeed_penalty != 0 {
            self.state.cycles = self
                .state
                .cycles
                .wrapping_add(fetch.memspeed_penalty as u64);
        }
        cycles += fetch.memspeed_penalty;
        StepResult { cycles, fetch }
    }

    pub fn state(&self) -> &CoreState {
        &self.state
    }

    pub fn state_mut(&mut self) -> &mut CoreState {
        &mut self.state
    }
}

#[inline]
pub fn full_address(state: &CoreState, offset: u16) -> u32 {
    ((state.pb as u32) << 16) | (offset as u32)
}

pub fn fetch_opcode(state: &mut CoreState, bus: &mut crate::bus::Bus) -> FetchResult {
    let pc_before = state.pc;
    let full_addr = full_address(state, pc_before);
    let opcode = bus.read_u8(full_addr);
    let mut memspeed_penalty = 0;
    if debug_flags::mem_timing() && bus.is_rom_address(full_addr) && !bus.is_fastrom() {
        memspeed_penalty = 2;
    }
    state.pc = state.pc.wrapping_add(1);
    FetchResult {
        opcode,
        memspeed_penalty,
        pc_before,
        full_addr,
    }
}

// Generic version for SA-1 using CpuBus trait
pub fn fetch_opcode_generic<T: crate::cpu_bus::CpuBus>(
    state: &mut CoreState,
    bus: &mut T,
) -> FetchResult {
    let pc_before = state.pc;
    let full_addr = full_address(state, pc_before);
    let opcode = bus.read_u8(full_addr);
    let memspeed_penalty = bus.opcode_memory_penalty(full_addr);
    state.pc = state.pc.wrapping_add(1);
    FetchResult {
        opcode,
        memspeed_penalty,
        pc_before,
        full_addr,
    }
}

// Generic helper functions for instruction execution

#[inline]
fn add_cycles(state: &mut CoreState, cycles: u8) {
    state.cycles = state.cycles.wrapping_add(cycles as u64);
}

fn read_u8_generic<T: CpuBus>(state: &mut CoreState, bus: &mut T) -> u8 {
    let addr = full_address(state, state.pc);
    let value = bus.read_u8(addr);
    state.pc = state.pc.wrapping_add(1);
    add_cycles(state, 1);
    value
}

fn write_u8_generic<T: CpuBus>(bus: &mut T, addr: u32, value: u8) {
    bus.write_u8(addr, value);
}

fn set_flags_nz_8(state: &mut CoreState, value: u8) {
    state.p.set(StatusFlags::NEGATIVE, value & 0x80 != 0);
    state.p.set(StatusFlags::ZERO, value == 0);
}

fn set_flags_nz_16(state: &mut CoreState, value: u16) {
    state.p.set(StatusFlags::NEGATIVE, value & 0x8000 != 0);
    state.p.set(StatusFlags::ZERO, value == 0);
}

fn read_u16_generic<T: CpuBus>(state: &mut CoreState, bus: &mut T) -> u16 {
    let addr = full_address(state, state.pc);
    let value = bus.read_u16(addr);
    state.pc = state.pc.wrapping_add(2);
    add_cycles(state, 2);
    value
}

fn read_u24_generic<T: CpuBus>(state: &mut CoreState, bus: &mut T) -> u32 {
    let addr = full_address(state, state.pc);
    let lo = bus.read_u8(addr) as u32;
    let mid = bus.read_u8(addr + 1) as u32;
    let hi = bus.read_u8(addr + 2) as u32;
    state.pc = state.pc.wrapping_add(3);
    add_cycles(state, 3);
    lo | (mid << 8) | (hi << 16)
}

fn read_absolute_address_generic<T: CpuBus>(state: &mut CoreState, bus: &mut T) -> u32 {
    let addr = read_u16_generic(state, bus);
    ((state.db as u32) << 16) | (addr as u32)
}

fn push_u8_generic<T: CpuBus>(state: &mut CoreState, bus: &mut T, value: u8) {
    let addr = if state.emulation_mode {
        0x0100 | (state.sp as u32)
    } else {
        state.sp as u32
    };
    bus.write_u8(addr, value);
    state.sp = if state.emulation_mode {
        0x0100 | ((state.sp.wrapping_sub(1)) & 0xFF)
    } else {
        state.sp.wrapping_sub(1)
    };
    add_cycles(state, 1);
}

fn push_u16_generic<T: CpuBus>(state: &mut CoreState, bus: &mut T, value: u16) {
    push_u8_generic(state, bus, (value >> 8) as u8);
    push_u8_generic(state, bus, (value & 0xFF) as u8);
}

fn pop_u8_generic<T: CpuBus>(state: &mut CoreState, bus: &mut T) -> u8 {
    state.sp = if state.emulation_mode {
        0x0100 | ((state.sp.wrapping_add(1)) & 0xFF)
    } else {
        state.sp.wrapping_add(1)
    };
    let addr = if state.emulation_mode {
        0x0100 | (state.sp as u32)
    } else {
        state.sp as u32
    };
    add_cycles(state, 1);
    bus.read_u8(addr)
}

fn pop_u16_generic<T: CpuBus>(state: &mut CoreState, bus: &mut T) -> u16 {
    let lo = pop_u8_generic(state, bus) as u16;
    let hi = pop_u8_generic(state, bus) as u16;
    (hi << 8) | lo
}

fn read_direct_address_generic<T: CpuBus>(state: &mut CoreState, bus: &mut T) -> (u32, u8) {
    let offset = read_u8_generic(state, bus) as u16;
    let penalty = if state.dp & 0x00FF != 0 { 1 } else { 0 };
    let addr = state.dp.wrapping_add(offset) as u32;
    (addr, penalty)
}

fn read_direct_x_address_generic<T: CpuBus>(state: &mut CoreState, bus: &mut T) -> (u32, u8) {
    let offset = read_u8_generic(state, bus) as u16;
    let penalty = if state.dp & 0x00FF != 0 { 1 } else { 0 };
    let addr = state.dp.wrapping_add(offset).wrapping_add(state.x) as u32;
    (addr, penalty)
}

fn read_absolute_x_address_generic<T: CpuBus>(state: &mut CoreState, bus: &mut T) -> (u32, u8) {
    let base = read_u16_generic(state, bus);
    let low_sum = (base & 0x00FF) as u32 + (state.x & 0x00FF) as u32;
    let penalty = if low_sum >= 0x100 { 1 } else { 0 };
    let addr = ((state.db as u32) << 16) | (base.wrapping_add(state.x) as u32);
    (addr, penalty)
}

fn read_absolute_y_address_generic<T: CpuBus>(state: &mut CoreState, bus: &mut T) -> (u32, u8) {
    let base = read_u16_generic(state, bus);
    let low_sum = (base & 0x00FF) as u32 + (state.y & 0x00FF) as u32;
    let penalty = if low_sum >= 0x100 { 1 } else { 0 };
    let addr = ((state.db as u32) << 16) | (base.wrapping_add(state.y) as u32);
    (addr, penalty)
}

fn read_absolute_long_address_generic<T: CpuBus>(state: &mut CoreState, bus: &mut T) -> u32 {
    read_u24_generic(state, bus)
}

fn read_absolute_long_x_address_generic<T: CpuBus>(state: &mut CoreState, bus: &mut T) -> u32 {
    let base = read_u24_generic(state, bus);
    base.wrapping_add(state.x as u32)
}

fn read_indirect_address_generic<T: CpuBus>(state: &mut CoreState, bus: &mut T) -> u32 {
    let pointer = read_u16_generic(state, bus);
    let lo = bus.read_u8(pointer as u32) as u16;
    let hi = bus.read_u8(pointer.wrapping_add(1) as u32) as u16;
    ((state.db as u32) << 16) | ((hi << 8) | lo) as u32
}

fn read_indirect_x_address_generic<T: CpuBus>(state: &mut CoreState, bus: &mut T) -> (u32, u8) {
    let base = read_u8_generic(state, bus) as u16;
    let penalty = if state.dp & 0x00FF != 0 { 1 } else { 0 };
    let addr = state.dp.wrapping_add(base).wrapping_add(state.x) as u16;
    let lo = bus.read_u8(addr as u32) as u16;
    let hi = bus.read_u8(addr.wrapping_add(1) as u32) as u16;
    let full = ((state.db as u32) << 16) | ((hi << 8) | lo) as u32;
    (full, penalty)
}

fn read_indirect_y_address_generic<T: CpuBus>(state: &mut CoreState, bus: &mut T) -> (u32, u8) {
    let base = read_u8_generic(state, bus) as u16;
    let mut penalty = 0u8;
    if state.dp & 0x00FF != 0 {
        penalty = penalty.saturating_add(1);
    }
    let addr = state.dp.wrapping_add(base);
    let lo = bus.read_u8(addr as u32) as u16;
    let hi = bus.read_u8(addr.wrapping_add(1) as u32) as u16;
    let base16 = (hi << 8) | lo;
    if ((base16 & 0x00FF) as u32) + (state.y & 0x00FF) as u32 >= 0x100 {
        penalty = penalty.saturating_add(1);
    }
    let full = ((state.db as u32) << 16) | (base16.wrapping_add(state.y) as u32);
    (full, penalty)
}

fn read_indirect_long_address_generic<T: CpuBus>(state: &mut CoreState, bus: &mut T) -> (u32, u8) {
    let pointer = read_u8_generic(state, bus) as u16;
    let mut penalty = 0u8;
    if state.dp & 0x00FF != 0 {
        penalty = penalty.saturating_add(1);
    }
    let addr = state.dp.wrapping_add(pointer);
    let lo = bus.read_u8(addr as u32) as u32;
    let mid = bus.read_u8(addr.wrapping_add(1) as u32) as u32;
    let hi = bus.read_u8(addr.wrapping_add(2) as u32) as u32;
    ((hi << 16) | (mid << 8) | lo, penalty)
}

fn read_indirect_long_y_address_generic<T: CpuBus>(
    state: &mut CoreState,
    bus: &mut T,
) -> (u32, u8) {
    let pointer = read_u8_generic(state, bus) as u16;
    let mut penalty = 0u8;
    if state.dp & 0x00FF != 0 {
        penalty = penalty.saturating_add(1);
    }
    let addr = state.dp.wrapping_add(pointer);
    let lo = bus.read_u8(addr as u32) as u32;
    let mid = bus.read_u8(addr.wrapping_add(1) as u32) as u32;
    let hi = bus.read_u8(addr.wrapping_add(2) as u32) as u32;
    let full = (hi << 16) | (mid << 8) | lo;
    (full.wrapping_add(state.y as u32), penalty)
}

fn read_stack_relative_address_generic<T: CpuBus>(state: &mut CoreState, bus: &mut T) -> u32 {
    let offset = read_u8_generic(state, bus) as u16;
    state.sp.wrapping_add(offset) as u32
}

fn read_stack_relative_indirect_y_generic<T: CpuBus>(
    state: &mut CoreState,
    bus: &mut T,
) -> (u32, u8) {
    let offset = read_u8_generic(state, bus) as u16;
    let addr = state.sp.wrapping_add(offset);
    let lo = bus.read_u8(addr as u32) as u16;
    let hi = bus.read_u8(addr.wrapping_add(1) as u32) as u16;
    let base16 = (hi << 8) | lo;
    let mut penalty = 0u8;
    if ((base16 & 0x00FF) as u32) + (state.y & 0x00FF) as u32 >= 0x100 {
        penalty = penalty.saturating_add(1);
    }
    let full = ((state.db as u32) << 16) | (base16.wrapping_add(state.y) as u32);
    (full, penalty)
}

#[inline]
fn memory_is_8bit(state: &CoreState) -> bool {
    state.emulation_mode || state.p.contains(StatusFlags::MEMORY_8BIT)
}

#[inline]
fn index_is_8bit(state: &CoreState) -> bool {
    state.emulation_mode || state.p.contains(StatusFlags::INDEX_8BIT)
}

fn write_a_generic<T: CpuBus>(state: &CoreState, bus: &mut T, addr: u32) {
    if memory_is_8bit(state) {
        bus.write_u8(addr, (state.a & 0xFF) as u8);
    } else {
        bus.write_u16(addr, state.a);
    }
}

fn write_x_generic<T: CpuBus>(state: &CoreState, bus: &mut T, addr: u32) {
    if index_is_8bit(state) {
        bus.write_u8(addr, (state.x & 0xFF) as u8);
    } else {
        bus.write_u16(addr, state.x);
    }
}

fn write_y_generic<T: CpuBus>(state: &CoreState, bus: &mut T, addr: u32) {
    if index_is_8bit(state) {
        bus.write_u8(addr, (state.y & 0xFF) as u8);
    } else {
        bus.write_u16(addr, state.y);
    }
}

fn set_flags_index(state: &mut CoreState, value: u16) {
    if index_is_8bit(state) {
        set_flags_nz_8(state, (value & 0xFF) as u8);
    } else {
        set_flags_nz_16(state, value);
    }
}

fn cmp_operand(state: &mut CoreState, operand: u16) {
    if memory_is_8bit(state) {
        let a = (state.a & 0xFF) as u8;
        let value = (operand & 0xFF) as u8;
        let result = a.wrapping_sub(value);
        state.p.set(StatusFlags::CARRY, a >= value);
        state.p.set(StatusFlags::ZERO, result == 0);
        state.p.set(StatusFlags::NEGATIVE, (result & 0x80) != 0);
    } else {
        let result = state.a.wrapping_sub(operand);
        state.p.set(StatusFlags::CARRY, state.a >= operand);
        state.p.set(StatusFlags::ZERO, result == 0);
        state.p.set(StatusFlags::NEGATIVE, (result & 0x8000) != 0);
    }
}

fn read_operand_m<T: CpuBus>(_state: &CoreState, bus: &mut T, addr: u32, memory_8bit: bool) -> u16 {
    if memory_8bit {
        bus.read_u8(addr) as u16
    } else {
        bus.read_u16(addr)
    }
}

fn ora_operand(state: &mut CoreState, operand: u16) {
    if memory_is_8bit(state) {
        let result = ((state.a & 0xFF) | (operand & 0xFF)) as u8;
        state.a = (state.a & 0xFF00) | (result as u16);
        set_flags_nz_8(state, result);
    } else {
        state.a |= operand;
        set_flags_nz_16(state, state.a);
    }
}

fn and_operand(state: &mut CoreState, operand: u16) {
    if memory_is_8bit(state) {
        let result = ((state.a & 0xFF) & (operand & 0xFF)) as u8;
        state.a = (state.a & 0xFF00) | (result as u16);
        set_flags_nz_8(state, result);
    } else {
        state.a &= operand;
        set_flags_nz_16(state, state.a);
    }
}

fn eor_operand(state: &mut CoreState, operand: u16) {
    if memory_is_8bit(state) {
        let result = ((state.a & 0xFF) ^ (operand & 0xFF)) as u8;
        state.a = (state.a & 0xFF00) | (result as u16);
        set_flags_nz_8(state, result);
    } else {
        state.a ^= operand;
        set_flags_nz_16(state, state.a);
    }
}

fn modify_memory<T: CpuBus, F8, F16>(
    state: &mut CoreState,
    bus: &mut T,
    addr: u32,
    memory_8bit: bool,
    mut modify8: F8,
    mut modify16: F16,
) where
    F8: FnMut(&mut CoreState, u8) -> u8,
    F16: FnMut(&mut CoreState, u16) -> u16,
{
    if memory_8bit {
        let value = bus.read_u8(addr);
        let result = modify8(state, value);
        bus.write_u8(addr, result);
    } else {
        let value = bus.read_u16(addr);
        let result = modify16(state, value);
        bus.write_u16(addr, result);
    }
}

fn asl8(state: &mut CoreState, value: u8) -> u8 {
    state.p.set(StatusFlags::CARRY, value & 0x80 != 0);
    let result = value << 1;
    set_flags_nz_8(state, result);
    result
}

fn asl16(state: &mut CoreState, value: u16) -> u16 {
    state.p.set(StatusFlags::CARRY, value & 0x8000 != 0);
    let result = value << 1;
    set_flags_nz_16(state, result);
    result
}

fn lsr8(state: &mut CoreState, value: u8) -> u8 {
    state.p.set(StatusFlags::CARRY, value & 0x01 != 0);
    let result = value >> 1;
    set_flags_nz_8(state, result);
    result
}

fn lsr16(state: &mut CoreState, value: u16) -> u16 {
    state.p.set(StatusFlags::CARRY, value & 0x0001 != 0);
    let result = value >> 1;
    set_flags_nz_16(state, result);
    result
}

fn rol8(state: &mut CoreState, value: u8) -> u8 {
    let carry_in = if state.p.contains(StatusFlags::CARRY) {
        1
    } else {
        0
    };
    state.p.set(StatusFlags::CARRY, value & 0x80 != 0);
    let result = (value << 1) | carry_in;
    set_flags_nz_8(state, result);
    result
}

fn rol16(state: &mut CoreState, value: u16) -> u16 {
    let carry_in = if state.p.contains(StatusFlags::CARRY) {
        1
    } else {
        0
    };
    state.p.set(StatusFlags::CARRY, value & 0x8000 != 0);
    let result = (value << 1) | carry_in;
    set_flags_nz_16(state, result);
    result
}

fn ror8(state: &mut CoreState, value: u8) -> u8 {
    let carry_in = if state.p.contains(StatusFlags::CARRY) {
        0x80
    } else {
        0
    };
    state.p.set(StatusFlags::CARRY, value & 0x01 != 0);
    let result = (value >> 1) | carry_in;
    set_flags_nz_8(state, result);
    result
}

fn ror16(state: &mut CoreState, value: u16) -> u16 {
    let carry_in = if state.p.contains(StatusFlags::CARRY) {
        0x8000
    } else {
        0
    };
    state.p.set(StatusFlags::CARRY, value & 0x0001 != 0);
    let result = (value >> 1) | carry_in;
    set_flags_nz_16(state, result);
    result
}

fn bit_operand(state: &mut CoreState, operand: u16) {
    let memory_8bit = memory_is_8bit(state);
    let zero = (state.a & operand) == 0;
    state.p.set(StatusFlags::ZERO, zero);
    if memory_8bit {
        state.p.set(StatusFlags::NEGATIVE, (operand & 0x80) != 0);
        state.p.set(StatusFlags::OVERFLOW, (operand & 0x40) != 0);
    } else {
        state.p.set(StatusFlags::NEGATIVE, (operand & 0x8000) != 0);
        state.p.set(StatusFlags::OVERFLOW, (operand & 0x4000) != 0);
    }
}

fn branch_if_generic<T: CpuBus>(state: &mut CoreState, bus: &mut T, condition: bool) -> u8 {
    let offset = read_u8_generic(state, bus) as i8;
    if condition {
        let old_pc = state.pc;
        let new_pc = state.pc.wrapping_add(offset as u16);
        state.pc = new_pc;
        let mut total_cycles = 3u8;
        if (old_pc & 0xFF00) != (new_pc & 0xFF00) {
            total_cycles = total_cycles.saturating_add(1);
        }
        // read_u8_generic already accounted for one cycle
        add_cycles(state, total_cycles.saturating_sub(1));
        total_cycles
    } else {
        // Not taken branch is 2 cycles total
        add_cycles(state, 1); // one more cycle beyond operand fetch
        2
    }
}

fn brl_generic<T: CpuBus>(state: &mut CoreState, bus: &mut T) -> u8 {
    let offset = read_u16_generic(state, bus) as i16;
    let old_pc = state.pc;
    let new_pc = state.pc.wrapping_add(offset as u16);
    state.pc = new_pc;
    let mut total_cycles = 4u8;
    if (old_pc & 0xFF00) != (new_pc & 0xFF00) {
        total_cycles = total_cycles.saturating_add(1);
    }
    // read_u16_generic already accounted for 3 cycles (2 for read + 1 for add below)
    add_cycles(state, total_cycles.saturating_sub(2));
    total_cycles
}

fn per_generic<T: CpuBus>(state: &mut CoreState, bus: &mut T) -> u8 {
    let offset = read_u16_generic(state, bus) as i16;
    let value = state.pc.wrapping_add(offset as u16);
    push_u16_generic(state, bus, value);
    let total_cycles: u8 = 6;
    // read_u16_generic accounted for 2 cycles, push_u16 added 2 cycles
    add_cycles(state, total_cycles.saturating_sub(4));
    total_cycles
}

#[inline]
fn bcd_adc8(a: u8, b: u8, carry_in: u8) -> (u8, bool) {
    let mut sum = a as u16 + b as u16 + carry_in as u16;
    if (sum & 0x0F) > 0x09 {
        sum += 0x06;
    }
    if (sum & 0xF0) > 0x90 {
        sum += 0x60;
    }
    ((sum & 0xFF) as u8, sum > 0x99)
}

#[inline]
fn bcd_sbc8(a: u8, b: u8, borrow_in: u8) -> (u8, bool) {
    let mut low = (a & 0x0F) as i16 - (b & 0x0F) as i16 - borrow_in as i16;
    let mut borrow = 0i16;
    if low < 0 {
        low += 10;
        borrow = 1;
    }
    let mut high = (a >> 4) as i16 - (b >> 4) as i16 - borrow;
    let mut borrow_high = 0i16;
    if high < 0 {
        high += 10;
        borrow_high = 1;
    }
    let result = ((high as u8) << 4) | (low as u8 & 0x0F);
    (result, borrow_high == 0)
}

fn adc_generic(state: &mut CoreState, operand: u16) {
    let carry_in = if state.p.contains(StatusFlags::CARRY) {
        1
    } else {
        0
    };
    let memory_8bit = state.emulation_mode || state.p.contains(StatusFlags::MEMORY_8BIT);
    let original_a = state.a;

    if state.p.contains(StatusFlags::DECIMAL) {
        if memory_8bit {
            let a8 = (original_a & 0x00FF) as u8;
            let b8 = (operand & 0x00FF) as u8;
            let binary_sum = (a8 as u16) + (b8 as u16) + (carry_in as u16);
            let (res, carry_out) = bcd_adc8(a8, b8, carry_in as u8);
            state.p.set(StatusFlags::CARRY, carry_out);
            let overflow = ((!(a8 ^ b8)) & ((a8 ^ (binary_sum as u8)) as u8) & 0x80) != 0;
            state.p.set(StatusFlags::OVERFLOW, overflow);
            state.a = (original_a & 0xFF00) | (res as u16);
        } else {
            let a = original_a;
            let b = operand;
            let binary_sum = (a as u32) + (b as u32) + (carry_in as u32);
            let (lo, carry1) = bcd_adc8((a & 0x00FF) as u8, (b & 0x00FF) as u8, carry_in as u8);
            let (hi, carry2) = bcd_adc8((a >> 8) as u8, (b >> 8) as u8, carry1 as u8);
            state.p.set(StatusFlags::CARRY, carry2);
            let overflow = (((!(a ^ b)) & (a ^ (binary_sum as u16))) & 0x8000) != 0;
            state.p.set(StatusFlags::OVERFLOW, overflow);
            state.a = ((hi as u16) << 8) | (lo as u16);
        }
    } else {
        let result = (original_a as u32) + (operand as u32) + (carry_in as u32);
        if memory_8bit {
            state.p.set(StatusFlags::CARRY, result > 0xFF);
            let overflow =
                ((original_a ^ operand) & 0x80) == 0 && ((original_a ^ result as u16) & 0x80) != 0;
            state.p.set(StatusFlags::OVERFLOW, overflow);
            state.a = (original_a & 0xFF00) | ((result & 0xFF) as u16);
        } else {
            state.p.set(StatusFlags::CARRY, result > 0xFFFF);
            let overflow = ((original_a ^ operand) & 0x8000) == 0
                && ((original_a ^ result as u16) & 0x8000) != 0;
            state.p.set(StatusFlags::OVERFLOW, overflow);
            state.a = (result & 0xFFFF) as u16;
        }
    }

    if memory_8bit {
        set_flags_nz_8(state, (state.a & 0x00FF) as u8);
    } else {
        set_flags_nz_16(state, state.a);
    }
}

fn sbc_generic(state: &mut CoreState, operand: u16) {
    let carry_clear = if state.p.contains(StatusFlags::CARRY) {
        0
    } else {
        1
    };
    let memory_8bit = state.emulation_mode || state.p.contains(StatusFlags::MEMORY_8BIT);
    let original_a = state.a;

    if state.p.contains(StatusFlags::DECIMAL) {
        if memory_8bit {
            let a8 = (original_a & 0xFF) as u8;
            let b8 = (operand & 0xFF) as u8;
            let binary = (a8 as i16) - (b8 as i16) - (carry_clear as i16);
            let (res, borrow) = bcd_sbc8(a8, b8, carry_clear as u8);
            state.p.set(StatusFlags::CARRY, borrow);
            let result8 = binary as i16 as u8;
            let overflow = ((a8 ^ b8) & (a8 ^ result8) & 0x80) != 0;
            state.p.set(StatusFlags::OVERFLOW, overflow);
            state.a = (original_a & 0xFF00) | (res as u16);
        } else {
            let a = original_a;
            let b = operand;
            let binary = (a as i32) - (b as i32) - (carry_clear as i32);
            let (lo, borrow_lo) =
                bcd_sbc8((a & 0x00FF) as u8, (b & 0x00FF) as u8, carry_clear as u8);
            let (hi, borrow_hi) = bcd_sbc8((a >> 8) as u8, (b >> 8) as u8, (!borrow_lo) as u8);
            state.p.set(StatusFlags::CARRY, borrow_hi);
            let result16 = binary as i32 as u16;
            let overflow = ((a ^ b) & (a ^ result16) & 0x8000) != 0;
            state.p.set(StatusFlags::OVERFLOW, overflow);
            state.a = ((hi as u16) << 8) | (lo as u16);
        }
    } else {
        let result = (original_a as i32) - (operand as i32) - (carry_clear as i32);
        if memory_8bit {
            state.p.set(StatusFlags::CARRY, result >= 0);
            let overflow =
                ((original_a ^ operand) & 0x80) != 0 && ((original_a ^ result as u16) & 0x80) != 0;
            state.p.set(StatusFlags::OVERFLOW, overflow);
            state.a = (original_a & 0xFF00) | ((result as u16) & 0x00FF);
        } else {
            state.p.set(StatusFlags::CARRY, result >= 0);
            let overflow = ((original_a ^ operand) & 0x8000) != 0
                && ((original_a ^ result as u16) & 0x8000) != 0;
            state.p.set(StatusFlags::OVERFLOW, overflow);
            state.a = (result as u16) & 0xFFFF;
        }
    }

    if memory_8bit {
        set_flags_nz_8(state, (state.a & 0xFF) as u8);
    } else {
        set_flags_nz_16(state, state.a);
    }
}

// Generic instruction implementations

fn jsr_generic<T: CpuBus>(state: &mut CoreState, bus: &mut T) -> u8 {
    let addr = read_absolute_address_generic(state, bus);
    push_u16_generic(state, bus, state.pc.wrapping_sub(1));
    state.pc = (addr & 0xFFFF) as u16;
    6
}

fn rts_generic<T: CpuBus>(state: &mut CoreState, bus: &mut T) -> u8 {
    let addr = pop_u16_generic(state, bus);
    state.pc = addr.wrapping_add(1);
    6
}

fn jsl_generic<T: CpuBus>(state: &mut CoreState, bus: &mut T) -> u8 {
    let addr_lo = bus.read_u8(full_address(state, state.pc)) as u32;
    let addr_hi = bus.read_u8(full_address(state, state.pc + 1)) as u32;
    let addr_bank = bus.read_u8(full_address(state, state.pc + 2)) as u32;
    let target = addr_lo | (addr_hi << 8) | (addr_bank << 16);
    state.pc = state.pc.wrapping_add(3);

    push_u8_generic(state, bus, state.pb);
    push_u16_generic(state, bus, state.pc.wrapping_sub(1));

    state.pb = (target >> 16) as u8;
    state.pc = (target & 0xFFFF) as u16;
    8
}

fn rtl_generic<T: CpuBus>(state: &mut CoreState, bus: &mut T) -> u8 {
    let addr = pop_u16_generic(state, bus);
    state.pb = pop_u8_generic(state, bus);
    state.pc = addr.wrapping_add(1);
    6
}

fn rep_generic<T: CpuBus>(state: &mut CoreState, bus: &mut T) -> u8 {
    let mask = bus.read_u8(full_address(state, state.pc));
    state.pc = state.pc.wrapping_add(1);
    let new_flags = StatusFlags::from_bits_truncate(state.p.bits() & !mask);
    state.p = new_flags;
    add_cycles(state, 3);
    3
}

fn sep_generic<T: CpuBus>(state: &mut CoreState, bus: &mut T) -> u8 {
    let mask = read_u8_generic(state, bus);
    let prev_flags = state.p;
    let mut new_flags = StatusFlags::from_bits_truncate(prev_flags.bits() | mask);
    if state.emulation_mode {
        new_flags.insert(StatusFlags::MEMORY_8BIT | StatusFlags::INDEX_8BIT);
    }
    let prev_x_16 = !prev_flags.contains(StatusFlags::INDEX_8BIT) && !state.emulation_mode;
    let new_x_16 = !new_flags.contains(StatusFlags::INDEX_8BIT) && !state.emulation_mode;
    state.p = new_flags;
    if prev_x_16 && !new_x_16 {
        state.x &= 0x00FF;
        state.y &= 0x00FF;
    }
    add_cycles(state, 2);
    3
}

// Main generic instruction execution function
pub fn execute_instruction_generic<T: CpuBus>(
    state: &mut CoreState,
    opcode: u8,
    bus: &mut T,
) -> u8 {
    match opcode {
        // Interrupt instructions - Essential for proper CPU operation
        0x00 => {
            // BRK - Software Interrupt
            // BRK pushes PC+2 and status register, then jumps to BRK vector
            let next_pc = state.pc.wrapping_add(1); // BRK has a dummy operand byte
            state.pc = next_pc;

            // Push program bank (only in native mode)
            if !state.emulation_mode {
                push_u8_generic(state, bus, state.pb);
            }

            // Push return address (PC after BRK + 1)
            push_u16_generic(state, bus, next_pc);

            // Push status register with B flag set
            let mut status_to_push = state.p.bits();
            status_to_push |= 0x10; // Set B flag
            if state.emulation_mode {
                status_to_push |= 0x20; // Set unused bit in emulation mode
            }
            push_u8_generic(state, bus, status_to_push);

            // Set interrupt disable flag
            state.p.insert(StatusFlags::IRQ_DISABLE);

            // Jump to BRK vector
            let vector_addr = if state.emulation_mode { 0xFFFE } else { 0xFFE6 };
            let vector = bus.read_u16(vector_addr);
            state.pc = vector;

            // Clear program bank in emulation mode
            if state.emulation_mode {
                state.pb = 0;
            }

            add_cycles(state, if state.emulation_mode { 7 } else { 8 });
            if state.emulation_mode {
                7
            } else {
                8
            }
        }

        0x02 => {
            // COP - Co-Processor Enable (software interrupt)
            let _signature = read_u8_generic(state, bus);
            let return_pc = state.pc;
            let mut pushed_status = state.p.bits() | 0x20; // bit 5 always set
            pushed_status &= !0x10; // COP pushes B=0

            if state.emulation_mode {
                push_u16_generic(state, bus, return_pc);
                push_u8_generic(state, bus, pushed_status);
                let accounted = 1 + 3; // operand fetch + pushes (3 cycles)
                add_cycles(state, 7 - accounted);
            } else {
                push_u8_generic(state, bus, state.pb);
                push_u16_generic(state, bus, return_pc);
                push_u8_generic(state, bus, pushed_status);
                let accounted = 1 + 4; // operand fetch + pushes (4 cycles)
                add_cycles(state, 7 - accounted);
            }

            state.p.insert(StatusFlags::IRQ_DISABLE);
            state.pb = 0;
            let vector_addr = if state.emulation_mode { 0xFFF4 } else { 0xFFE4 };
            let vector = bus.read_u16(vector_addr as u32);
            state.pc = vector;
            7
        }

        // Critical instructions used by DQ3 SA-1 code
        0x20 => jsr_generic(state, bus), // JSR absolute
        0x22 => jsl_generic(state, bus), // JSL long
        0x60 => rts_generic(state, bus), // RTS
        0x62 => per_generic(state, bus), // PER push effective relative address
        0x6B => rtl_generic(state, bus), // RTL
        0xC2 => rep_generic(state, bus), // REP
        0xE2 => sep_generic(state, bus), // SEP

        // Simple instructions that don't need bus access
        0xEA => {
            // NOP
            add_cycles(state, 2);
            2
        }
        0x18 => {
            // CLC
            state.p.remove(StatusFlags::CARRY);
            add_cycles(state, 2);
            2
        }

        0x1A => {
            // INC A
            if memory_is_8bit(state) {
                let value = ((state.a & 0xFF).wrapping_add(1)) as u8;
                state.a = (state.a & 0xFF00) | (value as u16);
                set_flags_nz_8(state, value);
            } else {
                state.a = state.a.wrapping_add(1);
                set_flags_nz_16(state, state.a);
            }
            add_cycles(state, 2);
            2
        }

        0x38 => {
            // SEC
            state.p.insert(StatusFlags::CARRY);
            add_cycles(state, 2);
            2
        }

        0x44 => {
            // MVP (Block Move Positive)
            let dest_bank = read_u8_generic(state, bus);
            let src_bank = read_u8_generic(state, bus);
            let src_addr = ((src_bank as u32) << 16) | (state.x as u32);
            let dest_addr = ((dest_bank as u32) << 16) | (state.y as u32);
            let value = bus.read_u8(src_addr);
            bus.write_u8(dest_addr, value);
            state.x = state.x.wrapping_sub(1);
            state.y = state.y.wrapping_sub(1);
            state.a = state.a.wrapping_sub(1);
            if state.a != 0xFFFF {
                state.pc = state.pc.wrapping_sub(3);
            }
            let base_cycles: u8 = 7;
            let already_accounted: u8 = 2; // two immediate bytes already consumed
            add_cycles(state, base_cycles.saturating_sub(already_accounted));
            base_cycles
        }

        0x54 => {
            // MVN (Block Move Negative)
            let dest_bank = read_u8_generic(state, bus);
            let src_bank = read_u8_generic(state, bus);
            let src_addr = ((src_bank as u32) << 16) | (state.x as u32);
            let dest_addr = ((dest_bank as u32) << 16) | (state.y as u32);
            let value = bus.read_u8(src_addr);
            bus.write_u8(dest_addr, value);
            state.x = state.x.wrapping_add(1);
            state.y = state.y.wrapping_add(1);
            state.a = state.a.wrapping_sub(1);
            if state.a != 0xFFFF {
                state.pc = state.pc.wrapping_sub(3);
            }
            let base_cycles: u8 = 7;
            let already_accounted: u8 = 2;
            add_cycles(state, base_cycles.saturating_sub(already_accounted));
            base_cycles
        }
        0x78 => {
            // SEI
            state.p.insert(StatusFlags::IRQ_DISABLE);
            add_cycles(state, 2);
            2
        }
        0xFB => {
            // XCE
            let old_carry = state.p.contains(StatusFlags::CARRY);
            if state.emulation_mode {
                state.p.insert(StatusFlags::CARRY);
            } else {
                state.p.remove(StatusFlags::CARRY);
            }
            state.emulation_mode = old_carry;
            add_cycles(state, 2);
            2
        }

        // Jump instructions
        0x4C => {
            // JMP absolute
            let addr = read_u16_generic(state, bus);
            state.pc = addr;
            3
        }
        0x5C => {
            // JML long
            let addr_lo = bus.read_u8(full_address(state, state.pc)) as u32;
            let addr_hi = bus.read_u8(full_address(state, state.pc + 1)) as u32;
            let addr_bank = bus.read_u8(full_address(state, state.pc + 2)) as u32;
            let target = addr_lo | (addr_hi << 8) | (addr_bank << 16);
            state.pb = (target >> 16) as u8;
            state.pc = (target & 0xFFFF) as u16;
            add_cycles(state, 4);
            4
        }
        0x6C => {
            // JMP (addr)
            let ptr = read_u16_generic(state, bus);
            let target = bus.read_u16(ptr as u32);
            state.pc = target;
            add_cycles(state, 5 - 2);
            5
        }
        0x7C => {
            // JMP (addr,X)
            let base = read_u16_generic(state, bus);
            let ptr = base.wrapping_add(state.x);
            let target = bus.read_u16(ptr as u32);
            state.pc = target;
            add_cycles(state, 6 - 2);
            6
        }
        0xDC => {
            // JMP [addr]
            let ptr = read_u16_generic(state, bus);
            let lo = bus.read_u8(ptr as u32) as u32;
            let mid = bus.read_u8(ptr.wrapping_add(1) as u32) as u32;
            let hi = bus.read_u8(ptr.wrapping_add(2) as u32) as u32;
            let target = (hi << 16) | (mid << 8) | lo;
            state.pb = ((target >> 16) & 0xFF) as u8;
            state.pc = (target & 0xFFFF) as u16;
            add_cycles(state, 6 - 2);
            6
        }

        // ORA logical OR operations
        0x04 => {
            // TSB direct page
            let (addr, penalty) = read_direct_address_generic(state, bus);
            if penalty != 0 {
                add_cycles(state, penalty);
            }
            let value = bus.read_u8(addr);
            let a_low = (state.a & 0xFF) as u8;
            state.p.set(StatusFlags::ZERO, (value & a_low) == 0);
            bus.write_u8(addr, value | a_low);
            let base_cycles: u8 = 5;
            let total_cycles = base_cycles.saturating_add(penalty);
            let already_accounted: u8 = 1 + penalty;
            add_cycles(state, total_cycles.saturating_sub(already_accounted));
            total_cycles
        }

        0x05 => {
            // ORA direct page
            let (addr, penalty) = read_direct_address_generic(state, bus);
            if penalty != 0 {
                add_cycles(state, penalty);
            }
            let memory_8bit = memory_is_8bit(state);
            let operand = read_operand_m(state, bus, addr, memory_8bit);
            ora_operand(state, operand);
            let base_cycles: u8 = 3;
            let total_cycles = base_cycles.saturating_add(penalty);
            let already_accounted: u8 = 1 + penalty;
            add_cycles(state, total_cycles.saturating_sub(already_accounted));
            total_cycles
        }

        0x15 => {
            // ORA direct page,X
            let (addr, penalty) = read_direct_x_address_generic(state, bus);
            if penalty != 0 {
                add_cycles(state, penalty);
            }
            let memory_8bit = memory_is_8bit(state);
            let operand = read_operand_m(state, bus, addr, memory_8bit);
            ora_operand(state, operand);
            let base_cycles: u8 = 4;
            let total_cycles = base_cycles.saturating_add(penalty);
            let already_accounted: u8 = 1 + penalty;
            add_cycles(state, total_cycles.saturating_sub(already_accounted));
            total_cycles
        }

        0x0D => {
            // ORA absolute
            let addr = read_absolute_address_generic(state, bus);
            let memory_8bit = memory_is_8bit(state);
            let operand = read_operand_m(state, bus, addr, memory_8bit);
            ora_operand(state, operand);
            let base_cycles: u8 = 4;
            let already_accounted: u8 = 2;
            add_cycles(state, base_cycles.saturating_sub(already_accounted));
            base_cycles
        }

        0x1D => {
            // ORA absolute,X
            let (addr, penalty) = read_absolute_x_address_generic(state, bus);
            if penalty != 0 {
                add_cycles(state, penalty);
            }
            let memory_8bit = memory_is_8bit(state);
            let operand = read_operand_m(state, bus, addr, memory_8bit);
            ora_operand(state, operand);
            let base_cycles: u8 = 4;
            let total_cycles = base_cycles.saturating_add(penalty);
            let already_accounted: u8 = 2 + penalty;
            add_cycles(state, total_cycles.saturating_sub(already_accounted));
            total_cycles
        }

        0x19 => {
            // ORA absolute,Y
            let (addr, penalty) = read_absolute_y_address_generic(state, bus);
            if penalty != 0 {
                add_cycles(state, penalty);
            }
            let memory_8bit = memory_is_8bit(state);
            let operand = read_operand_m(state, bus, addr, memory_8bit);
            ora_operand(state, operand);
            let base_cycles: u8 = if memory_8bit { 4 } else { 5 };
            let total_cycles = base_cycles.saturating_add(penalty);
            let already_accounted: u8 = 2 + penalty;
            add_cycles(state, total_cycles.saturating_sub(already_accounted));
            total_cycles
        }

        0x0F => {
            // ORA absolute long
            let addr = read_absolute_long_address_generic(state, bus);
            let memory_8bit = memory_is_8bit(state);
            let operand = read_operand_m(state, bus, addr, memory_8bit);
            ora_operand(state, operand);
            let base_cycles: u8 = 5;
            let already_accounted: u8 = 3;
            add_cycles(state, base_cycles.saturating_sub(already_accounted));
            base_cycles
        }

        0x1F => {
            // ORA absolute long,X
            let addr = read_absolute_long_x_address_generic(state, bus);
            let memory_8bit = memory_is_8bit(state);
            let operand = read_operand_m(state, bus, addr, memory_8bit);
            ora_operand(state, operand);
            let base_cycles: u8 = 5;
            let already_accounted: u8 = 3;
            add_cycles(state, base_cycles.saturating_sub(already_accounted));
            base_cycles
        }

        0x01 => {
            // ORA (dp,X)
            let (addr, penalty) = read_indirect_x_address_generic(state, bus);
            if penalty != 0 {
                add_cycles(state, penalty);
            }
            let memory_8bit = memory_is_8bit(state);
            let operand = read_operand_m(state, bus, addr, memory_8bit);
            ora_operand(state, operand);
            let base_cycles: u8 = 6;
            let total_cycles = base_cycles.saturating_add(penalty);
            let already_accounted: u8 = 1 + penalty;
            add_cycles(state, total_cycles.saturating_sub(already_accounted));
            total_cycles
        }

        0x11 => {
            // ORA (dp),Y
            let (addr, penalty) = read_indirect_y_address_generic(state, bus);
            if penalty != 0 {
                add_cycles(state, penalty);
            }
            let memory_8bit = memory_is_8bit(state);
            let operand = read_operand_m(state, bus, addr, memory_8bit);
            ora_operand(state, operand);
            let base_cycles: u8 = 5;
            let total_cycles = base_cycles.saturating_add(penalty);
            let already_accounted: u8 = 1 + penalty;
            add_cycles(state, total_cycles.saturating_sub(already_accounted));
            total_cycles
        }

        0x12 => {
            // ORA (dp)
            let addr = read_indirect_address_generic(state, bus);
            let memory_8bit = memory_is_8bit(state);
            let operand = read_operand_m(state, bus, addr, memory_8bit);
            ora_operand(state, operand);
            let base_cycles: u8 = 5;
            let already_accounted: u8 = 2;
            add_cycles(state, base_cycles.saturating_sub(already_accounted));
            base_cycles
        }

        0x13 => {
            // ORA (sr,S),Y
            let (addr, penalty) = read_stack_relative_indirect_y_generic(state, bus);
            if penalty != 0 {
                add_cycles(state, penalty);
            }
            let memory_8bit = memory_is_8bit(state);
            let operand = read_operand_m(state, bus, addr, memory_8bit);
            ora_operand(state, operand);
            let base_cycles: u8 = 7;
            let total_cycles = base_cycles.saturating_add(penalty);
            let already_accounted: u8 = 1 + penalty;
            add_cycles(state, total_cycles.saturating_sub(already_accounted));
            total_cycles
        }

        0x03 => {
            // ORA stack relative
            let addr = read_stack_relative_address_generic(state, bus);
            let memory_8bit = memory_is_8bit(state);
            let operand = read_operand_m(state, bus, addr, memory_8bit);
            ora_operand(state, operand);
            let base_cycles: u8 = 4;
            let already_accounted: u8 = 1;
            add_cycles(state, base_cycles.saturating_sub(already_accounted));
            base_cycles
        }

        0x07 => {
            // ORA [dp]
            let (addr, penalty) = read_indirect_long_address_generic(state, bus);
            if penalty != 0 {
                add_cycles(state, penalty);
            }
            let memory_8bit = memory_is_8bit(state);
            let operand = read_operand_m(state, bus, addr, memory_8bit);
            ora_operand(state, operand);
            let base_cycles: u8 = 6;
            let total_cycles = base_cycles.saturating_add(penalty);
            let already_accounted: u8 = 1 + penalty;
            add_cycles(state, total_cycles.saturating_sub(already_accounted));
            total_cycles
        }

        0x17 => {
            // ORA [dp],Y
            let (addr, penalty) = read_indirect_long_y_address_generic(state, bus);
            if penalty != 0 {
                add_cycles(state, penalty);
            }
            let memory_8bit = memory_is_8bit(state);
            let operand = read_operand_m(state, bus, addr, memory_8bit);
            ora_operand(state, operand);
            let base_cycles: u8 = 6;
            let total_cycles = base_cycles.saturating_add(penalty);
            let already_accounted: u8 = 1 + penalty;
            add_cycles(state, total_cycles.saturating_sub(already_accounted));
            total_cycles
        }

        // Load/Store Instructions - Essential for DQ3 SA-1
        0x25 => {
            // AND direct page
            let (addr, penalty) = read_direct_address_generic(state, bus);
            if penalty != 0 {
                add_cycles(state, penalty);
            }
            let memory_8bit = memory_is_8bit(state);
            let operand = read_operand_m(state, bus, addr, memory_8bit);
            and_operand(state, operand);
            let base_cycles: u8 = if memory_8bit { 3 } else { 4 };
            let total_cycles = base_cycles.saturating_add(penalty);
            let already_accounted: u8 = 1 + penalty;
            add_cycles(state, total_cycles.saturating_sub(already_accounted));
            total_cycles
        }

        0x35 => {
            // AND direct page,X
            let (addr, penalty) = read_direct_x_address_generic(state, bus);
            if penalty != 0 {
                add_cycles(state, penalty);
            }
            let memory_8bit = memory_is_8bit(state);
            let operand = read_operand_m(state, bus, addr, memory_8bit);
            and_operand(state, operand);
            let base_cycles: u8 = if memory_8bit { 4 } else { 5 };
            let total_cycles = base_cycles.saturating_add(penalty);
            let already_accounted: u8 = 1 + penalty;
            add_cycles(state, total_cycles.saturating_sub(already_accounted));
            total_cycles
        }

        0x2D => {
            // AND absolute
            let addr = read_absolute_address_generic(state, bus);
            let memory_8bit = memory_is_8bit(state);
            let operand = read_operand_m(state, bus, addr, memory_8bit);
            and_operand(state, operand);
            let base_cycles: u8 = if memory_8bit { 4 } else { 5 };
            let already_accounted: u8 = 2;
            add_cycles(state, base_cycles.saturating_sub(already_accounted));
            base_cycles
        }

        0x3D => {
            // AND absolute,X
            let (addr, penalty) = read_absolute_x_address_generic(state, bus);
            if penalty != 0 {
                add_cycles(state, penalty);
            }
            let memory_8bit = memory_is_8bit(state);
            let operand = read_operand_m(state, bus, addr, memory_8bit);
            and_operand(state, operand);
            let base_cycles: u8 = if memory_8bit { 4 } else { 5 };
            let total_cycles = base_cycles.saturating_add(penalty);
            let already_accounted: u8 = 2 + penalty;
            add_cycles(state, total_cycles.saturating_sub(already_accounted));
            total_cycles
        }

        0x39 => {
            // AND absolute,Y
            let (addr, penalty) = read_absolute_y_address_generic(state, bus);
            if penalty != 0 {
                add_cycles(state, penalty);
            }
            let memory_8bit = memory_is_8bit(state);
            let operand = read_operand_m(state, bus, addr, memory_8bit);
            and_operand(state, operand);
            let base_cycles: u8 = if memory_8bit { 4 } else { 5 };
            let total_cycles = base_cycles.saturating_add(penalty);
            let already_accounted: u8 = 2 + penalty;
            add_cycles(state, total_cycles.saturating_sub(already_accounted));
            total_cycles
        }

        0x2F => {
            // AND absolute long
            let addr = read_absolute_long_address_generic(state, bus);
            let memory_8bit = memory_is_8bit(state);
            let operand = read_operand_m(state, bus, addr, memory_8bit);
            and_operand(state, operand);
            let base_cycles: u8 = if memory_8bit { 5 } else { 6 };
            let already_accounted: u8 = 3;
            add_cycles(state, base_cycles.saturating_sub(already_accounted));
            base_cycles
        }

        0x3F => {
            // AND absolute long,X
            let addr = read_absolute_long_x_address_generic(state, bus);
            let memory_8bit = memory_is_8bit(state);
            let operand = read_operand_m(state, bus, addr, memory_8bit);
            and_operand(state, operand);
            let base_cycles: u8 = if memory_8bit { 5 } else { 6 };
            let already_accounted: u8 = 3;
            add_cycles(state, base_cycles.saturating_sub(already_accounted));
            base_cycles
        }

        0x21 => {
            // AND (dp,X)
            let (addr, penalty) = read_indirect_x_address_generic(state, bus);
            if penalty != 0 {
                add_cycles(state, penalty);
            }
            let memory_8bit = memory_is_8bit(state);
            let operand = read_operand_m(state, bus, addr, memory_8bit);
            and_operand(state, operand);
            let base_cycles: u8 = if memory_8bit { 6 } else { 7 };
            let total_cycles = base_cycles.saturating_add(penalty);
            let already_accounted: u8 = 1 + penalty;
            add_cycles(state, total_cycles.saturating_sub(already_accounted));
            total_cycles
        }

        0x31 => {
            // AND (dp),Y
            let (addr, penalty) = read_indirect_y_address_generic(state, bus);
            if penalty != 0 {
                add_cycles(state, penalty);
            }
            let memory_8bit = memory_is_8bit(state);
            let operand = read_operand_m(state, bus, addr, memory_8bit);
            and_operand(state, operand);
            let base_cycles: u8 = if memory_8bit { 5 } else { 6 };
            let total_cycles = base_cycles.saturating_add(penalty);
            let already_accounted: u8 = 1 + penalty;
            add_cycles(state, total_cycles.saturating_sub(already_accounted));
            total_cycles
        }

        0x32 => {
            // AND (dp)
            let addr = read_indirect_address_generic(state, bus);
            let memory_8bit = memory_is_8bit(state);
            let operand = read_operand_m(state, bus, addr, memory_8bit);
            and_operand(state, operand);
            let base_cycles: u8 = if memory_8bit { 5 } else { 6 };
            let already_accounted: u8 = 2;
            add_cycles(state, base_cycles.saturating_sub(already_accounted));
            base_cycles
        }

        0x33 => {
            // AND (sr,S),Y
            let (addr, penalty) = read_stack_relative_indirect_y_generic(state, bus);
            if penalty != 0 {
                add_cycles(state, penalty);
            }
            let memory_8bit = memory_is_8bit(state);
            let operand = read_operand_m(state, bus, addr, memory_8bit);
            and_operand(state, operand);
            let base_cycles: u8 = if memory_8bit { 7 } else { 8 };
            let total_cycles = base_cycles.saturating_add(penalty);
            let already_accounted: u8 = 1 + penalty;
            add_cycles(state, total_cycles.saturating_sub(already_accounted));
            total_cycles
        }

        0x23 => {
            // AND stack relative
            let addr = read_stack_relative_address_generic(state, bus);
            let memory_8bit = memory_is_8bit(state);
            let operand = read_operand_m(state, bus, addr, memory_8bit);
            and_operand(state, operand);
            let base_cycles: u8 = if memory_8bit { 4 } else { 5 };
            let already_accounted: u8 = 1;
            add_cycles(state, base_cycles.saturating_sub(already_accounted));
            base_cycles
        }

        0x27 => {
            // AND [dp]
            let (addr, penalty) = read_indirect_long_address_generic(state, bus);
            if penalty != 0 {
                add_cycles(state, penalty);
            }
            let memory_8bit = memory_is_8bit(state);
            let operand = read_operand_m(state, bus, addr, memory_8bit);
            and_operand(state, operand);
            let base_cycles: u8 = if memory_8bit { 6 } else { 7 };
            let total_cycles = base_cycles.saturating_add(penalty);
            let already_accounted: u8 = 1 + penalty;
            add_cycles(state, total_cycles.saturating_sub(already_accounted));
            total_cycles
        }

        0x37 => {
            // AND [dp],Y
            let (addr, penalty) = read_indirect_long_y_address_generic(state, bus);
            if penalty != 0 {
                add_cycles(state, penalty);
            }
            let memory_8bit = memory_is_8bit(state);
            let operand = read_operand_m(state, bus, addr, memory_8bit);
            and_operand(state, operand);
            let base_cycles: u8 = if memory_8bit { 6 } else { 7 };
            let total_cycles = base_cycles.saturating_add(penalty);
            let already_accounted: u8 = 1 + penalty;
            add_cycles(state, total_cycles.saturating_sub(already_accounted));
            total_cycles
        }

        // EOR logical exclusive OR operations
        0x45 => {
            // EOR direct page
            let (addr, penalty) = read_direct_address_generic(state, bus);
            if penalty != 0 {
                add_cycles(state, penalty);
            }
            let memory_8bit = memory_is_8bit(state);
            let operand = read_operand_m(state, bus, addr, memory_8bit);
            eor_operand(state, operand);
            let base_cycles: u8 = if memory_8bit { 3 } else { 4 };
            let total_cycles = base_cycles.saturating_add(penalty);
            let already_accounted: u8 = 1 + penalty;
            add_cycles(state, total_cycles.saturating_sub(already_accounted));
            total_cycles
        }

        0x55 => {
            // EOR direct page,X
            let (addr, penalty) = read_direct_x_address_generic(state, bus);
            if penalty != 0 {
                add_cycles(state, penalty);
            }
            let memory_8bit = memory_is_8bit(state);
            let operand = read_operand_m(state, bus, addr, memory_8bit);
            eor_operand(state, operand);
            let base_cycles: u8 = if memory_8bit { 4 } else { 5 };
            let total_cycles = base_cycles.saturating_add(penalty);
            let already_accounted: u8 = 1 + penalty;
            add_cycles(state, total_cycles.saturating_sub(already_accounted));
            total_cycles
        }

        0x4D => {
            // EOR absolute
            let addr = read_absolute_address_generic(state, bus);
            let memory_8bit = memory_is_8bit(state);
            let operand = read_operand_m(state, bus, addr, memory_8bit);
            eor_operand(state, operand);
            let base_cycles: u8 = if memory_8bit { 4 } else { 5 };
            let already_accounted: u8 = 2;
            add_cycles(state, base_cycles.saturating_sub(already_accounted));
            base_cycles
        }

        0x5D => {
            // EOR absolute,X
            let (addr, penalty) = read_absolute_x_address_generic(state, bus);
            if penalty != 0 {
                add_cycles(state, penalty);
            }
            let memory_8bit = memory_is_8bit(state);
            let operand = read_operand_m(state, bus, addr, memory_8bit);
            eor_operand(state, operand);
            let base_cycles: u8 = if memory_8bit { 4 } else { 5 };
            let total_cycles = base_cycles.saturating_add(penalty);
            let already_accounted: u8 = 2 + penalty;
            add_cycles(state, total_cycles.saturating_sub(already_accounted));
            total_cycles
        }

        0x59 => {
            // EOR absolute,Y
            let (addr, penalty) = read_absolute_y_address_generic(state, bus);
            if penalty != 0 {
                add_cycles(state, penalty);
            }
            let memory_8bit = memory_is_8bit(state);
            let operand = read_operand_m(state, bus, addr, memory_8bit);
            eor_operand(state, operand);
            let base_cycles: u8 = if memory_8bit { 4 } else { 5 };
            let total_cycles = base_cycles.saturating_add(penalty);
            let already_accounted: u8 = 2 + penalty;
            add_cycles(state, total_cycles.saturating_sub(already_accounted));
            total_cycles
        }

        0x4F => {
            // EOR absolute long
            let addr = read_absolute_long_address_generic(state, bus);
            let memory_8bit = memory_is_8bit(state);
            let operand = read_operand_m(state, bus, addr, memory_8bit);
            eor_operand(state, operand);
            let base_cycles: u8 = if memory_8bit { 5 } else { 6 };
            let already_accounted: u8 = 3;
            add_cycles(state, base_cycles.saturating_sub(already_accounted));
            base_cycles
        }

        0x5F => {
            // EOR absolute long,X
            let addr = read_absolute_long_x_address_generic(state, bus);
            let memory_8bit = memory_is_8bit(state);
            let operand = read_operand_m(state, bus, addr, memory_8bit);
            eor_operand(state, operand);
            let base_cycles: u8 = if memory_8bit { 5 } else { 6 };
            let already_accounted: u8 = 3;
            add_cycles(state, base_cycles.saturating_sub(already_accounted));
            base_cycles
        }

        0x41 => {
            // EOR (dp,X)
            let (addr, penalty) = read_indirect_x_address_generic(state, bus);
            if penalty != 0 {
                add_cycles(state, penalty);
            }
            let memory_8bit = memory_is_8bit(state);
            let operand = read_operand_m(state, bus, addr, memory_8bit);
            eor_operand(state, operand);
            let base_cycles: u8 = if memory_8bit { 6 } else { 7 };
            let total_cycles = base_cycles.saturating_add(penalty);
            let already_accounted: u8 = 1 + penalty;
            add_cycles(state, total_cycles.saturating_sub(already_accounted));
            total_cycles
        }

        0x51 => {
            // EOR (dp),Y
            let (addr, penalty) = read_indirect_y_address_generic(state, bus);
            if penalty != 0 {
                add_cycles(state, penalty);
            }
            let memory_8bit = memory_is_8bit(state);
            let operand = read_operand_m(state, bus, addr, memory_8bit);
            eor_operand(state, operand);
            let base_cycles: u8 = if memory_8bit { 5 } else { 6 };
            let total_cycles = base_cycles.saturating_add(penalty);
            let already_accounted: u8 = 1 + penalty;
            add_cycles(state, total_cycles.saturating_sub(already_accounted));
            total_cycles
        }

        0x52 => {
            // EOR (dp)
            let addr = read_indirect_address_generic(state, bus);
            let memory_8bit = memory_is_8bit(state);
            let operand = read_operand_m(state, bus, addr, memory_8bit);
            eor_operand(state, operand);
            let base_cycles: u8 = if memory_8bit { 5 } else { 6 };
            let already_accounted: u8 = 2;
            add_cycles(state, base_cycles.saturating_sub(already_accounted));
            base_cycles
        }

        0x53 => {
            // EOR (sr,S),Y
            let (addr, penalty) = read_stack_relative_indirect_y_generic(state, bus);
            if penalty != 0 {
                add_cycles(state, penalty);
            }
            let memory_8bit = memory_is_8bit(state);
            let operand = read_operand_m(state, bus, addr, memory_8bit);
            eor_operand(state, operand);
            let base_cycles: u8 = if memory_8bit { 7 } else { 8 };
            let total_cycles = base_cycles.saturating_add(penalty);
            let already_accounted: u8 = 1 + penalty;
            add_cycles(state, total_cycles.saturating_sub(already_accounted));
            total_cycles
        }

        0x43 => {
            // EOR stack relative
            let addr = read_stack_relative_address_generic(state, bus);
            let memory_8bit = memory_is_8bit(state);
            let operand = read_operand_m(state, bus, addr, memory_8bit);
            eor_operand(state, operand);
            let base_cycles: u8 = if memory_8bit { 4 } else { 5 };
            let already_accounted: u8 = 1;
            add_cycles(state, base_cycles.saturating_sub(already_accounted));
            base_cycles
        }

        0x47 => {
            // EOR [dp]
            let (addr, penalty) = read_indirect_long_address_generic(state, bus);
            if penalty != 0 {
                add_cycles(state, penalty);
            }
            let memory_8bit = memory_is_8bit(state);
            let operand = read_operand_m(state, bus, addr, memory_8bit);
            eor_operand(state, operand);
            let base_cycles: u8 = if memory_8bit { 6 } else { 7 };
            let total_cycles = base_cycles.saturating_add(penalty);
            let already_accounted: u8 = 1 + penalty;
            add_cycles(state, total_cycles.saturating_sub(already_accounted));
            total_cycles
        }

        0x57 => {
            // EOR [dp],Y
            let (addr, penalty) = read_indirect_long_y_address_generic(state, bus);
            if penalty != 0 {
                add_cycles(state, penalty);
            }
            let memory_8bit = memory_is_8bit(state);
            let operand = read_operand_m(state, bus, addr, memory_8bit);
            eor_operand(state, operand);
            let base_cycles: u8 = if memory_8bit { 6 } else { 7 };
            let total_cycles = base_cycles.saturating_add(penalty);
            let already_accounted: u8 = 1 + penalty;
            add_cycles(state, total_cycles.saturating_sub(already_accounted));
            total_cycles
        }

        0x0A => {
            // ASL accumulator
            if memory_is_8bit(state) {
                let result = asl8(state, (state.a & 0xFF) as u8);
                state.a = (state.a & 0xFF00) | (result as u16);
            } else {
                state.a = asl16(state, state.a);
            }
            add_cycles(state, 2);
            2
        }

        0x2A => {
            // ROL accumulator
            if memory_is_8bit(state) {
                let result = rol8(state, (state.a & 0xFF) as u8);
                state.a = (state.a & 0xFF00) | (result as u16);
            } else {
                state.a = rol16(state, state.a);
            }
            add_cycles(state, 2);
            2
        }

        0x4A => {
            // LSR accumulator
            if memory_is_8bit(state) {
                let result = lsr8(state, (state.a & 0xFF) as u8);
                state.a = (state.a & 0xFF00) | (result as u16);
            } else {
                state.a = lsr16(state, state.a);
            }
            add_cycles(state, 2);
            2
        }

        0x6A => {
            // ROR accumulator
            if memory_is_8bit(state) {
                let result = ror8(state, (state.a & 0xFF) as u8);
                state.a = (state.a & 0xFF00) | (result as u16);
            } else {
                state.a = ror16(state, state.a);
            }
            add_cycles(state, 2);
            2
        }

        0x06 => {
            // ASL direct page
            let (addr, penalty) = read_direct_address_generic(state, bus);
            if penalty != 0 {
                add_cycles(state, penalty);
            }
            let memory_8bit = memory_is_8bit(state);
            modify_memory(state, bus, addr, memory_8bit, asl8, asl16);
            let base_cycles: u8 = 5;
            let total_cycles = base_cycles.saturating_add(penalty);
            let already_accounted: u8 = 1 + penalty;
            add_cycles(state, total_cycles.saturating_sub(already_accounted));
            total_cycles
        }

        0x16 => {
            // ASL direct page,X
            let (addr, penalty) = read_direct_x_address_generic(state, bus);
            if penalty != 0 {
                add_cycles(state, penalty);
            }
            let memory_8bit = memory_is_8bit(state);
            modify_memory(state, bus, addr, memory_8bit, asl8, asl16);
            let base_cycles: u8 = 6;
            let total_cycles = base_cycles.saturating_add(penalty);
            let already_accounted: u8 = 1 + penalty;
            add_cycles(state, total_cycles.saturating_sub(already_accounted));
            total_cycles
        }

        0x0E => {
            // ASL absolute
            let addr = read_absolute_address_generic(state, bus);
            let memory_8bit = memory_is_8bit(state);
            modify_memory(state, bus, addr, memory_8bit, asl8, asl16);
            let base_cycles: u8 = 6;
            let already_accounted: u8 = 3;
            add_cycles(state, base_cycles.saturating_sub(already_accounted));
            base_cycles
        }

        0x1E => {
            // ASL absolute,X
            let (addr, penalty) = read_absolute_x_address_generic(state, bus);
            if penalty != 0 {
                add_cycles(state, penalty);
            }
            let memory_8bit = memory_is_8bit(state);
            modify_memory(state, bus, addr, memory_8bit, asl8, asl16);
            let base_cycles: u8 = 7;
            let total_cycles = base_cycles.saturating_add(penalty);
            let already_accounted: u8 = 3 + penalty;
            add_cycles(state, total_cycles.saturating_sub(already_accounted));
            total_cycles
        }

        0x26 => {
            // ROL direct page
            let (addr, penalty) = read_direct_address_generic(state, bus);
            if penalty != 0 {
                add_cycles(state, penalty);
            }
            let memory_8bit = memory_is_8bit(state);
            modify_memory(state, bus, addr, memory_8bit, rol8, rol16);
            let base_cycles: u8 = 5;
            let total_cycles = base_cycles.saturating_add(penalty);
            let already_accounted: u8 = 1 + penalty;
            add_cycles(state, total_cycles.saturating_sub(already_accounted));
            total_cycles
        }

        0x36 => {
            // ROL direct page,X
            let (addr, penalty) = read_direct_x_address_generic(state, bus);
            if penalty != 0 {
                add_cycles(state, penalty);
            }
            let memory_8bit = memory_is_8bit(state);
            modify_memory(state, bus, addr, memory_8bit, rol8, rol16);
            let base_cycles: u8 = 6;
            let total_cycles = base_cycles.saturating_add(penalty);
            let already_accounted: u8 = 1 + penalty;
            add_cycles(state, total_cycles.saturating_sub(already_accounted));
            total_cycles
        }

        0x2E => {
            // ROL absolute
            let addr = read_absolute_address_generic(state, bus);
            let memory_8bit = memory_is_8bit(state);
            modify_memory(state, bus, addr, memory_8bit, rol8, rol16);
            let base_cycles: u8 = 6;
            let already_accounted: u8 = 3;
            add_cycles(state, base_cycles.saturating_sub(already_accounted));
            base_cycles
        }

        0x3E => {
            // ROL absolute,X
            let (addr, penalty) = read_absolute_x_address_generic(state, bus);
            if penalty != 0 {
                add_cycles(state, penalty);
            }
            let memory_8bit = memory_is_8bit(state);
            modify_memory(state, bus, addr, memory_8bit, rol8, rol16);
            let base_cycles: u8 = 7;
            let total_cycles = base_cycles.saturating_add(penalty);
            let already_accounted: u8 = 3 + penalty;
            add_cycles(state, total_cycles.saturating_sub(already_accounted));
            total_cycles
        }

        0x46 => {
            // LSR direct page
            let (addr, penalty) = read_direct_address_generic(state, bus);
            if penalty != 0 {
                add_cycles(state, penalty);
            }
            let memory_8bit = memory_is_8bit(state);
            modify_memory(state, bus, addr, memory_8bit, lsr8, lsr16);
            let base_cycles: u8 = 5;
            let total_cycles = base_cycles.saturating_add(penalty);
            let already_accounted: u8 = 1 + penalty;
            add_cycles(state, total_cycles.saturating_sub(already_accounted));
            total_cycles
        }

        0x56 => {
            // LSR direct page,X
            let (addr, penalty) = read_direct_x_address_generic(state, bus);
            if penalty != 0 {
                add_cycles(state, penalty);
            }
            let memory_8bit = memory_is_8bit(state);
            modify_memory(state, bus, addr, memory_8bit, lsr8, lsr16);
            let base_cycles: u8 = 6;
            let total_cycles = base_cycles.saturating_add(penalty);
            let already_accounted: u8 = 1 + penalty;
            add_cycles(state, total_cycles.saturating_sub(already_accounted));
            total_cycles
        }

        0x4E => {
            // LSR absolute
            let addr = read_absolute_address_generic(state, bus);
            let memory_8bit = memory_is_8bit(state);
            modify_memory(state, bus, addr, memory_8bit, lsr8, lsr16);
            let base_cycles: u8 = 6;
            let already_accounted: u8 = 3;
            add_cycles(state, base_cycles.saturating_sub(already_accounted));
            base_cycles
        }

        0x5E => {
            // LSR absolute,X
            let (addr, penalty) = read_absolute_x_address_generic(state, bus);
            if penalty != 0 {
                add_cycles(state, penalty);
            }
            let memory_8bit = memory_is_8bit(state);
            modify_memory(state, bus, addr, memory_8bit, lsr8, lsr16);
            let base_cycles: u8 = 7;
            let total_cycles = base_cycles.saturating_add(penalty);
            let already_accounted: u8 = 3 + penalty;
            add_cycles(state, total_cycles.saturating_sub(already_accounted));
            total_cycles
        }

        0x66 => {
            // ROR direct page
            let (addr, penalty) = read_direct_address_generic(state, bus);
            if penalty != 0 {
                add_cycles(state, penalty);
            }
            let memory_8bit = memory_is_8bit(state);
            modify_memory(state, bus, addr, memory_8bit, ror8, ror16);
            let base_cycles: u8 = 5;
            let total_cycles = base_cycles.saturating_add(penalty);
            let already_accounted: u8 = 1 + penalty;
            add_cycles(state, total_cycles.saturating_sub(already_accounted));
            total_cycles
        }

        0x76 => {
            // ROR direct page,X
            let (addr, penalty) = read_direct_x_address_generic(state, bus);
            if penalty != 0 {
                add_cycles(state, penalty);
            }
            let memory_8bit = memory_is_8bit(state);
            modify_memory(state, bus, addr, memory_8bit, ror8, ror16);
            let base_cycles: u8 = 6;
            let total_cycles = base_cycles.saturating_add(penalty);
            let already_accounted: u8 = 1 + penalty;
            add_cycles(state, total_cycles.saturating_sub(already_accounted));
            total_cycles
        }

        0x6E => {
            // ROR absolute
            let addr = read_absolute_address_generic(state, bus);
            let memory_8bit = memory_is_8bit(state);
            modify_memory(state, bus, addr, memory_8bit, ror8, ror16);
            let base_cycles: u8 = 6;
            let already_accounted: u8 = 3;
            add_cycles(state, base_cycles.saturating_sub(already_accounted));
            base_cycles
        }

        0x7E => {
            // ROR absolute,X
            let (addr, penalty) = read_absolute_x_address_generic(state, bus);
            if penalty != 0 {
                add_cycles(state, penalty);
            }
            let memory_8bit = memory_is_8bit(state);
            modify_memory(state, bus, addr, memory_8bit, ror8, ror16);
            let base_cycles: u8 = 7;
            let total_cycles = base_cycles.saturating_add(penalty);
            let already_accounted: u8 = 3 + penalty;
            add_cycles(state, total_cycles.saturating_sub(already_accounted));
            total_cycles
        }

        0x89 => {
            // BIT immediate
            let memory_8bit = memory_is_8bit(state);
            let operand = if memory_8bit {
                read_u8_generic(state, bus) as u16
            } else {
                read_u16_generic(state, bus)
            };
            bit_operand(state, operand);
            let total_cycles: u8 = if memory_8bit { 2 } else { 3 };
            let already_accounted: u8 = if memory_8bit { 1 } else { 2 };
            add_cycles(state, total_cycles.saturating_sub(already_accounted));
            total_cycles
        }

        0x24 => {
            // BIT direct page
            let (addr, penalty) = read_direct_address_generic(state, bus);
            if penalty != 0 {
                add_cycles(state, penalty);
            }
            let memory_8bit = memory_is_8bit(state);
            let operand = read_operand_m(state, bus, addr, memory_8bit);
            bit_operand(state, operand);
            let base_cycles: u8 = 3;
            let total_cycles = base_cycles.saturating_add(penalty);
            let already_accounted: u8 = 1 + penalty;
            add_cycles(state, total_cycles.saturating_sub(already_accounted));
            total_cycles
        }

        0x34 => {
            // BIT direct page,X
            let (addr, penalty) = read_direct_x_address_generic(state, bus);
            if penalty != 0 {
                add_cycles(state, penalty);
            }
            let memory_8bit = memory_is_8bit(state);
            let operand = read_operand_m(state, bus, addr, memory_8bit);
            bit_operand(state, operand);
            let base_cycles: u8 = 4;
            let total_cycles = base_cycles.saturating_add(penalty);
            let already_accounted: u8 = 1 + penalty;
            add_cycles(state, total_cycles.saturating_sub(already_accounted));
            total_cycles
        }

        0x2C => {
            // BIT absolute
            let addr = read_absolute_address_generic(state, bus);
            let memory_8bit = memory_is_8bit(state);
            let operand = read_operand_m(state, bus, addr, memory_8bit);
            bit_operand(state, operand);
            let base_cycles: u8 = 4;
            let already_accounted: u8 = 3;
            add_cycles(state, base_cycles.saturating_sub(already_accounted));
            base_cycles
        }

        0x3C => {
            // BIT absolute,X
            let (addr, penalty) = read_absolute_x_address_generic(state, bus);
            if penalty != 0 {
                add_cycles(state, penalty);
            }
            let memory_8bit = memory_is_8bit(state);
            let operand = read_operand_m(state, bus, addr, memory_8bit);
            bit_operand(state, operand);
            let base_cycles: u8 = 4;
            let total_cycles = base_cycles.saturating_add(penalty);
            let already_accounted: u8 = 3 + penalty;
            add_cycles(state, total_cycles.saturating_sub(already_accounted));
            total_cycles
        }

        0xA1 => {
            // LDA (dp,X)
            let (addr, penalty) = read_indirect_x_address_generic(state, bus);
            if penalty != 0 {
                add_cycles(state, penalty);
            }
            let memory_8bit = memory_is_8bit(state);
            let value = read_operand_m(state, bus, addr, memory_8bit);
            if memory_8bit {
                state.a = (state.a & 0xFF00) | (value & 0xFF);
                set_flags_nz_8(state, value as u8);
            } else {
                state.a = value;
                set_flags_nz_16(state, value);
            }
            let base_cycles: u8 = 6;
            let total_cycles = base_cycles.saturating_add(penalty);
            let already_accounted: u8 = 1 + penalty;
            add_cycles(state, total_cycles.saturating_sub(already_accounted));
            total_cycles
        }

        0xA3 => {
            // LDA stack relative
            let addr = read_stack_relative_address_generic(state, bus);
            let memory_8bit = memory_is_8bit(state);
            let value = read_operand_m(state, bus, addr, memory_8bit);
            if memory_8bit {
                state.a = (state.a & 0xFF00) | (value & 0xFF);
                set_flags_nz_8(state, value as u8);
            } else {
                state.a = value;
                set_flags_nz_16(state, value);
            }
            let base_cycles: u8 = 4;
            let already_accounted: u8 = 1;
            add_cycles(state, base_cycles.saturating_sub(already_accounted));
            base_cycles
        }

        0xA4 => {
            // LDY direct page
            let (addr, penalty) = read_direct_address_generic(state, bus);
            if penalty != 0 {
                add_cycles(state, penalty);
            }
            if index_is_8bit(state) {
                let value = bus.read_u8(addr) as u16;
                state.y = (state.y & 0xFF00) | value;
                set_flags_nz_8(state, value as u8);
            } else {
                let value = bus.read_u16(addr);
                state.y = value;
                set_flags_nz_16(state, value);
            }
            let base_cycles: u8 = if index_is_8bit(state) { 3 } else { 4 };
            let total_cycles = base_cycles.saturating_add(penalty);
            let already_accounted: u8 = 1 + penalty;
            add_cycles(state, total_cycles.saturating_sub(already_accounted));
            total_cycles
        }

        0xA5 => {
            // LDA direct page
            let (addr, penalty) = read_direct_address_generic(state, bus);
            if penalty != 0 {
                add_cycles(state, penalty);
            }
            let memory_8bit = state.emulation_mode || state.p.contains(StatusFlags::MEMORY_8BIT);
            if memory_8bit {
                let value = bus.read_u8(addr) as u16;
                state.a = (state.a & 0xFF00) | value;
                set_flags_nz_8(state, value as u8);
            } else {
                let value = bus.read_u16(addr);
                state.a = value;
                set_flags_nz_16(state, value);
            }
            let base_cycles: u8 = if memory_8bit { 3 } else { 4 };
            let total_cycles = base_cycles.saturating_add(penalty);
            let already_accounted: u8 = 1 + penalty; // operand fetch + dp penalty
            add_cycles(state, total_cycles.saturating_sub(already_accounted));
            total_cycles
        }

        0xA6 => {
            // LDX direct page
            let (addr, penalty) = read_direct_address_generic(state, bus);
            if penalty != 0 {
                add_cycles(state, penalty);
            }
            if index_is_8bit(state) {
                let value = bus.read_u8(addr) as u16;
                state.x = (state.x & 0xFF00) | value;
                set_flags_nz_8(state, value as u8);
            } else {
                let value = bus.read_u16(addr);
                state.x = value;
                set_flags_nz_16(state, value);
            }
            let base_cycles: u8 = if index_is_8bit(state) { 3 } else { 4 };
            let total_cycles = base_cycles.saturating_add(penalty);
            let already_accounted: u8 = 1 + penalty;
            add_cycles(state, total_cycles.saturating_sub(already_accounted));
            total_cycles
        }

        0xA7 => {
            // LDA [dp]
            let (addr, penalty) = read_indirect_long_address_generic(state, bus);
            if penalty != 0 {
                add_cycles(state, penalty);
            }
            let memory_8bit = memory_is_8bit(state);
            let value = read_operand_m(state, bus, addr, memory_8bit);
            if memory_8bit {
                state.a = (state.a & 0xFF00) | (value & 0xFF);
                set_flags_nz_8(state, value as u8);
            } else {
                state.a = value;
                set_flags_nz_16(state, value);
            }
            let base_cycles: u8 = 6;
            let total_cycles = base_cycles.saturating_add(penalty);
            let already_accounted: u8 = 1 + penalty;
            add_cycles(state, total_cycles.saturating_sub(already_accounted));
            total_cycles
        }

        0xA9 => {
            // LDA immediate
            let value = if state.p.contains(StatusFlags::MEMORY_8BIT) {
                read_u8_generic(state, bus) as u16
            } else {
                read_u16_generic(state, bus)
            };
            state.a = value;
            if state.p.contains(StatusFlags::MEMORY_8BIT) {
                set_flags_nz_8(state, value as u8);
            } else {
                set_flags_nz_16(state, value);
            }
            add_cycles(
                state,
                if state.p.contains(StatusFlags::MEMORY_8BIT) {
                    2
                } else {
                    3
                },
            );
            if state.p.contains(StatusFlags::MEMORY_8BIT) {
                2
            } else {
                3
            }
        }
        0xAC => {
            // LDY absolute
            let addr = read_absolute_address_generic(state, bus);
            if index_is_8bit(state) {
                let value = bus.read_u8(addr) as u16;
                state.y = (state.y & 0xFF00) | value;
                set_flags_nz_8(state, value as u8);
            } else {
                let value = bus.read_u16(addr);
                state.y = value;
                set_flags_nz_16(state, value);
            }
            add_cycles(state, 4);
            4
        }
        0xAE => {
            // LDX absolute
            let addr = read_absolute_address_generic(state, bus);
            if index_is_8bit(state) {
                let value = bus.read_u8(addr) as u16;
                state.x = (state.x & 0xFF00) | value;
                set_flags_nz_8(state, value as u8);
            } else {
                let value = bus.read_u16(addr);
                state.x = value;
                set_flags_nz_16(state, value);
            }
            add_cycles(state, 4);
            4
        }
        0xA2 => {
            // LDX immediate
            let value = if state.p.contains(StatusFlags::INDEX_8BIT) {
                read_u8_generic(state, bus) as u16
            } else {
                read_u16_generic(state, bus)
            };
            state.x = value;
            if state.p.contains(StatusFlags::INDEX_8BIT) {
                set_flags_nz_8(state, value as u8);
            } else {
                set_flags_nz_16(state, value);
            }
            add_cycles(
                state,
                if state.p.contains(StatusFlags::INDEX_8BIT) {
                    2
                } else {
                    3
                },
            );
            if state.p.contains(StatusFlags::INDEX_8BIT) {
                2
            } else {
                3
            }
        }
        0xA8 => {
            // TAY (Transfer Accumulator to Y)
            if index_is_8bit(state) {
                state.y = (state.y & 0xFF00) | ((state.a & 0xFF) as u16);
                set_flags_nz_8(state, (state.y & 0xFF) as u8);
            } else {
                state.y = state.a;
                set_flags_nz_16(state, state.y);
            }
            add_cycles(state, 2);
            2
        }
        0xAA => {
            // TAX (Transfer Accumulator to X)
            if index_is_8bit(state) {
                state.x = (state.x & 0xFF00) | ((state.a & 0xFF) as u16);
                set_flags_nz_8(state, (state.x & 0xFF) as u8);
            } else {
                state.x = state.a;
                set_flags_nz_16(state, state.x);
            }
            add_cycles(state, 2);
            2
        }
        0xA0 => {
            // LDY immediate
            let value = if state.p.contains(StatusFlags::INDEX_8BIT) {
                read_u8_generic(state, bus) as u16
            } else {
                read_u16_generic(state, bus)
            };
            state.y = value;
            if state.p.contains(StatusFlags::INDEX_8BIT) {
                set_flags_nz_8(state, value as u8);
            } else {
                set_flags_nz_16(state, value);
            }
            add_cycles(
                state,
                if state.p.contains(StatusFlags::INDEX_8BIT) {
                    2
                } else {
                    3
                },
            );
            if state.p.contains(StatusFlags::INDEX_8BIT) {
                2
            } else {
                3
            }
        }

        // Store instructions
        0x8D => {
            // STA absolute
            let addr = read_absolute_address_generic(state, bus);
            if state.p.contains(StatusFlags::MEMORY_8BIT) {
                write_u8_generic(bus, addr, state.a as u8);
                add_cycles(state, 4);
                4
            } else {
                bus.write_u16(addr, state.a);
                add_cycles(state, 5);
                5
            }
        }
        0x8E => {
            // STX absolute
            let addr = read_absolute_address_generic(state, bus);
            if state.p.contains(StatusFlags::INDEX_8BIT) {
                write_u8_generic(bus, addr, state.x as u8);
                add_cycles(state, 4);
                4
            } else {
                bus.write_u16(addr, state.x);
                add_cycles(state, 5);
                5
            }
        }
        0x8C => {
            // STY absolute
            let addr = read_absolute_address_generic(state, bus);
            if state.p.contains(StatusFlags::INDEX_8BIT) {
                write_u8_generic(bus, addr, state.y as u8);
                add_cycles(state, 4);
                4
            } else {
                bus.write_u16(addr, state.y);
                add_cycles(state, 5);
                5
            }
        }

        // Stack operations - Critical for SA-1 function calls
        0x0B => {
            // PHD - Push Direct Page register
            push_u16_generic(state, bus, state.dp);
            add_cycles(state, 4);
            4
        }

        0x48 => {
            // PHA
            if state.p.contains(StatusFlags::MEMORY_8BIT) {
                push_u8_generic(state, bus, state.a as u8);
                add_cycles(state, 3);
                3
            } else {
                push_u16_generic(state, bus, state.a);
                add_cycles(state, 4);
                4
            }
        }
        0x4B => {
            // PHK - Push Program Bank
            push_u8_generic(state, bus, state.pb);
            add_cycles(state, 3);
            3
        }
        0x5A => {
            // PHY - Push Y register
            if index_is_8bit(state) {
                push_u8_generic(state, bus, (state.y & 0xFF) as u8);
                add_cycles(state, 3);
                3
            } else {
                push_u16_generic(state, bus, state.y);
                add_cycles(state, 4);
                4
            }
        }
        0x68 => {
            // PLA
            if state.p.contains(StatusFlags::MEMORY_8BIT) {
                state.a = (state.a & 0xFF00) | (pop_u8_generic(state, bus) as u16);
                set_flags_nz_8(state, state.a as u8);
                add_cycles(state, 4);
                4
            } else {
                state.a = pop_u16_generic(state, bus);
                set_flags_nz_16(state, state.a);
                add_cycles(state, 5);
                5
            }
        }
        0x8B => {
            // PHB - Push Data Bank register
            push_u8_generic(state, bus, state.db);
            add_cycles(state, 3);
            3
        }
        0xAB => {
            // PLB - Pull Data Bank register
            state.db = pop_u8_generic(state, bus);
            set_flags_nz_8(state, state.db);
            add_cycles(state, 4);
            4
        }
        0xDA => {
            // PHX - Push X register
            if index_is_8bit(state) {
                push_u8_generic(state, bus, (state.x & 0xFF) as u8);
                add_cycles(state, 3);
                3
            } else {
                push_u16_generic(state, bus, state.x);
                add_cycles(state, 4);
                4
            }
        }
        0xFA => {
            // PLX - Pull X register
            if index_is_8bit(state) {
                let value = pop_u8_generic(state, bus);
                state.x = (state.x & 0xFF00) | (value as u16);
                set_flags_nz_8(state, value);
                add_cycles(state, 4);
                4
            } else {
                state.x = pop_u16_generic(state, bus);
                set_flags_nz_16(state, state.x);
                add_cycles(state, 5);
                5
            }
        }
        0xF4 => {
            // PEA
            let value = read_u16_generic(state, bus);
            push_u16_generic(state, bus, value);
            add_cycles(state, 5);
            5
        }
        0x1B => {
            // TCS - Transfer Accumulator to Stack Pointer
            state.sp = state.a;
            add_cycles(state, 2);
            2
        }

        // Arithmetic operations
        0x69 => {
            // ADC immediate (supports decimal mode)
            let memory_8bit = state.emulation_mode || state.p.contains(StatusFlags::MEMORY_8BIT);
            let operand = if memory_8bit {
                read_u8_generic(state, bus) as u16
            } else {
                read_u16_generic(state, bus)
            };
            adc_generic(state, operand);
            let total_cycles: u8 = if memory_8bit { 2 } else { 3 };
            let already_accounted: u8 = if memory_8bit { 1 } else { 2 };
            add_cycles(state, total_cycles.saturating_sub(already_accounted));
            total_cycles
        }

        0x65 => {
            // ADC direct page
            let (addr, penalty) = read_direct_address_generic(state, bus);
            if penalty != 0 {
                add_cycles(state, penalty);
            }
            let memory_8bit = state.emulation_mode || state.p.contains(StatusFlags::MEMORY_8BIT);
            let operand = if memory_8bit {
                bus.read_u8(addr) as u16
            } else {
                bus.read_u16(addr)
            };
            adc_generic(state, operand);
            let base_cycles: u8 = if memory_8bit { 3 } else { 4 };
            let total_cycles = base_cycles.saturating_add(penalty);
            let already_accounted: u8 = 1 + penalty;
            add_cycles(state, total_cycles.saturating_sub(already_accounted));
            total_cycles
        }

        0x75 => {
            // ADC direct page,X
            let (addr, penalty) = read_direct_x_address_generic(state, bus);
            if penalty != 0 {
                add_cycles(state, penalty);
            }
            let memory_8bit = state.emulation_mode || state.p.contains(StatusFlags::MEMORY_8BIT);
            let operand = if memory_8bit {
                bus.read_u8(addr) as u16
            } else {
                bus.read_u16(addr)
            };
            adc_generic(state, operand);
            let base_cycles: u8 = if memory_8bit { 4 } else { 5 };
            let total_cycles = base_cycles.saturating_add(penalty);
            let already_accounted: u8 = 1 + penalty;
            add_cycles(state, total_cycles.saturating_sub(already_accounted));
            total_cycles
        }

        0x6D => {
            // ADC absolute
            let addr = read_absolute_address_generic(state, bus);
            let memory_8bit = state.emulation_mode || state.p.contains(StatusFlags::MEMORY_8BIT);
            let operand = if memory_8bit {
                bus.read_u8(addr) as u16
            } else {
                bus.read_u16(addr)
            };
            adc_generic(state, operand);
            let base_cycles: u8 = if memory_8bit { 4 } else { 5 };
            let already_accounted: u8 = 2;
            add_cycles(state, base_cycles.saturating_sub(already_accounted));
            base_cycles
        }

        0x7D => {
            // ADC absolute,X
            let (addr, penalty) = read_absolute_x_address_generic(state, bus);
            if penalty != 0 {
                add_cycles(state, penalty);
            }
            let memory_8bit = state.emulation_mode || state.p.contains(StatusFlags::MEMORY_8BIT);
            let operand = if memory_8bit {
                bus.read_u8(addr) as u16
            } else {
                bus.read_u16(addr)
            };
            adc_generic(state, operand);
            let base_cycles: u8 = if memory_8bit { 4 } else { 5 };
            let total_cycles = base_cycles.saturating_add(penalty);
            let already_accounted: u8 = 2 + penalty;
            add_cycles(state, total_cycles.saturating_sub(already_accounted));
            total_cycles
        }

        0x79 => {
            // ADC absolute,Y
            let (addr, penalty) = read_absolute_y_address_generic(state, bus);
            if penalty != 0 {
                add_cycles(state, penalty);
            }
            let memory_8bit = state.emulation_mode || state.p.contains(StatusFlags::MEMORY_8BIT);
            let operand = if memory_8bit {
                bus.read_u8(addr) as u16
            } else {
                bus.read_u16(addr)
            };
            adc_generic(state, operand);
            let base_cycles: u8 = if memory_8bit { 4 } else { 5 };
            let total_cycles = base_cycles.saturating_add(penalty);
            let already_accounted: u8 = 2 + penalty;
            add_cycles(state, total_cycles.saturating_sub(already_accounted));
            total_cycles
        }

        0x6F => {
            // ADC absolute long
            let addr = read_absolute_long_address_generic(state, bus);
            let memory_8bit = state.emulation_mode || state.p.contains(StatusFlags::MEMORY_8BIT);
            let operand = if memory_8bit {
                bus.read_u8(addr) as u16
            } else {
                bus.read_u16(addr)
            };
            adc_generic(state, operand);
            let base_cycles: u8 = if memory_8bit { 5 } else { 6 };
            let already_accounted: u8 = 3;
            add_cycles(state, base_cycles.saturating_sub(already_accounted));
            base_cycles
        }

        0x7F => {
            // ADC absolute long,X
            let addr = read_absolute_long_x_address_generic(state, bus);
            let memory_8bit = state.emulation_mode || state.p.contains(StatusFlags::MEMORY_8BIT);
            let operand = if memory_8bit {
                bus.read_u8(addr) as u16
            } else {
                bus.read_u16(addr)
            };
            adc_generic(state, operand);
            let base_cycles: u8 = if memory_8bit { 5 } else { 6 };
            let already_accounted: u8 = 3;
            add_cycles(state, base_cycles.saturating_sub(already_accounted));
            base_cycles
        }

        0x61 => {
            // ADC (dp,X)
            let (addr, penalty) = read_indirect_x_address_generic(state, bus);
            if penalty != 0 {
                add_cycles(state, penalty);
            }
            let memory_8bit = state.emulation_mode || state.p.contains(StatusFlags::MEMORY_8BIT);
            let operand = if memory_8bit {
                bus.read_u8(addr) as u16
            } else {
                bus.read_u16(addr)
            };
            adc_generic(state, operand);
            let base_cycles: u8 = if memory_8bit { 6 } else { 7 };
            let total_cycles = base_cycles.saturating_add(penalty);
            let already_accounted: u8 = 1 + penalty;
            add_cycles(state, total_cycles.saturating_sub(already_accounted));
            total_cycles
        }

        0x71 => {
            // ADC (dp),Y
            let (addr, penalty) = read_indirect_y_address_generic(state, bus);
            if penalty != 0 {
                add_cycles(state, penalty);
            }
            let memory_8bit = state.emulation_mode || state.p.contains(StatusFlags::MEMORY_8BIT);
            let operand = if memory_8bit {
                bus.read_u8(addr) as u16
            } else {
                bus.read_u16(addr)
            };
            adc_generic(state, operand);
            let base_cycles: u8 = if memory_8bit { 5 } else { 6 };
            let total_cycles = base_cycles.saturating_add(penalty);
            let already_accounted: u8 = 1 + penalty;
            add_cycles(state, total_cycles.saturating_sub(already_accounted));
            total_cycles
        }

        0x72 => {
            // ADC (dp)
            let addr = read_indirect_address_generic(state, bus);
            let memory_8bit = state.emulation_mode || state.p.contains(StatusFlags::MEMORY_8BIT);
            let operand = if memory_8bit {
                bus.read_u8(addr) as u16
            } else {
                bus.read_u16(addr)
            };
            adc_generic(state, operand);
            let base_cycles: u8 = if memory_8bit { 5 } else { 6 };
            let already_accounted: u8 = 2;
            add_cycles(state, base_cycles.saturating_sub(already_accounted));
            base_cycles
        }

        0x67 => {
            // ADC [dp]
            let (addr, penalty) = read_indirect_long_address_generic(state, bus);
            if penalty != 0 {
                add_cycles(state, penalty);
            }
            let memory_8bit = state.emulation_mode || state.p.contains(StatusFlags::MEMORY_8BIT);
            let operand = if memory_8bit {
                bus.read_u8(addr) as u16
            } else {
                bus.read_u16(addr)
            };
            adc_generic(state, operand);
            let base_cycles: u8 = if memory_8bit { 6 } else { 7 };
            let total_cycles = base_cycles.saturating_add(penalty);
            let already_accounted: u8 = 1 + penalty;
            add_cycles(state, total_cycles.saturating_sub(already_accounted));
            total_cycles
        }

        0x77 => {
            // ADC [dp],Y
            let (addr, penalty) = read_indirect_long_y_address_generic(state, bus);
            if penalty != 0 {
                add_cycles(state, penalty);
            }
            let memory_8bit = state.emulation_mode || state.p.contains(StatusFlags::MEMORY_8BIT);
            let operand = if memory_8bit {
                bus.read_u8(addr) as u16
            } else {
                bus.read_u16(addr)
            };
            adc_generic(state, operand);
            let base_cycles: u8 = if memory_8bit { 6 } else { 7 };
            let total_cycles = base_cycles.saturating_add(penalty);
            let already_accounted: u8 = 1 + penalty;
            add_cycles(state, total_cycles.saturating_sub(already_accounted));
            total_cycles
        }

        0x63 => {
            // ADC stack relative
            let addr = read_stack_relative_address_generic(state, bus);
            let memory_8bit = state.emulation_mode || state.p.contains(StatusFlags::MEMORY_8BIT);
            let operand = if memory_8bit {
                bus.read_u8(addr) as u16
            } else {
                bus.read_u16(addr)
            };
            adc_generic(state, operand);
            let base_cycles: u8 = if memory_8bit { 4 } else { 5 };
            let already_accounted: u8 = 1;
            add_cycles(state, base_cycles.saturating_sub(already_accounted));
            base_cycles
        }

        0x73 => {
            // ADC (sr,S),Y
            let (addr, penalty) = read_stack_relative_indirect_y_generic(state, bus);
            if penalty != 0 {
                add_cycles(state, penalty);
            }
            let memory_8bit = state.emulation_mode || state.p.contains(StatusFlags::MEMORY_8BIT);
            let operand = if memory_8bit {
                bus.read_u8(addr) as u16
            } else {
                bus.read_u16(addr)
            };
            adc_generic(state, operand);
            let base_cycles: u8 = if memory_8bit { 7 } else { 8 };
            let total_cycles = base_cycles.saturating_add(penalty);
            let already_accounted: u8 = 1 + penalty;
            add_cycles(state, total_cycles.saturating_sub(already_accounted));
            total_cycles
        }

        0xE9 => {
            // SBC immediate (supports decimal mode)
            let memory_8bit = state.emulation_mode || state.p.contains(StatusFlags::MEMORY_8BIT);
            let operand = if memory_8bit {
                read_u8_generic(state, bus) as u16
            } else {
                read_u16_generic(state, bus)
            };
            sbc_generic(state, operand);
            let total_cycles: u8 = if memory_8bit { 2 } else { 3 };
            let already_accounted: u8 = if memory_8bit { 1 } else { 2 };
            add_cycles(state, total_cycles.saturating_sub(already_accounted));
            total_cycles
        }

        0xE5 => {
            // SBC direct page
            let (addr, penalty) = read_direct_address_generic(state, bus);
            if penalty != 0 {
                add_cycles(state, penalty);
            }
            let memory_8bit = state.emulation_mode || state.p.contains(StatusFlags::MEMORY_8BIT);
            let operand = if memory_8bit {
                bus.read_u8(addr) as u16
            } else {
                bus.read_u16(addr)
            };
            sbc_generic(state, operand);
            let base_cycles: u8 = if memory_8bit { 3 } else { 4 };
            let total_cycles = base_cycles.saturating_add(penalty);
            let already_accounted: u8 = 1 + penalty;
            add_cycles(state, total_cycles.saturating_sub(already_accounted));
            total_cycles
        }

        0xF5 => {
            // SBC direct page,X
            let (addr, penalty) = read_direct_x_address_generic(state, bus);
            if penalty != 0 {
                add_cycles(state, penalty);
            }
            let memory_8bit = state.emulation_mode || state.p.contains(StatusFlags::MEMORY_8BIT);
            let operand = if memory_8bit {
                bus.read_u8(addr) as u16
            } else {
                bus.read_u16(addr)
            };
            sbc_generic(state, operand);
            let base_cycles: u8 = if memory_8bit { 4 } else { 5 };
            let total_cycles = base_cycles.saturating_add(penalty);
            let already_accounted: u8 = 1 + penalty;
            add_cycles(state, total_cycles.saturating_sub(already_accounted));
            total_cycles
        }

        0xED => {
            // SBC absolute
            let addr = read_absolute_address_generic(state, bus);
            let memory_8bit = state.emulation_mode || state.p.contains(StatusFlags::MEMORY_8BIT);
            let operand = if memory_8bit {
                bus.read_u8(addr) as u16
            } else {
                bus.read_u16(addr)
            };
            sbc_generic(state, operand);
            let base_cycles: u8 = if memory_8bit { 4 } else { 5 };
            let already_accounted: u8 = 2;
            add_cycles(state, base_cycles.saturating_sub(already_accounted));
            base_cycles
        }

        0xFD => {
            // SBC absolute,X
            let (addr, penalty) = read_absolute_x_address_generic(state, bus);
            if penalty != 0 {
                add_cycles(state, penalty);
            }
            let memory_8bit = state.emulation_mode || state.p.contains(StatusFlags::MEMORY_8BIT);
            let operand = if memory_8bit {
                bus.read_u8(addr) as u16
            } else {
                bus.read_u16(addr)
            };
            sbc_generic(state, operand);
            let base_cycles: u8 = if memory_8bit { 4 } else { 5 };
            let total_cycles = base_cycles.saturating_add(penalty);
            let already_accounted: u8 = 2 + penalty;
            add_cycles(state, total_cycles.saturating_sub(already_accounted));
            total_cycles
        }

        0xF9 => {
            // SBC absolute,Y
            let (addr, penalty) = read_absolute_y_address_generic(state, bus);
            if penalty != 0 {
                add_cycles(state, penalty);
            }
            let memory_8bit = state.emulation_mode || state.p.contains(StatusFlags::MEMORY_8BIT);
            let operand = if memory_8bit {
                bus.read_u8(addr) as u16
            } else {
                bus.read_u16(addr)
            };
            sbc_generic(state, operand);
            let base_cycles: u8 = if memory_8bit { 4 } else { 5 };
            let total_cycles = base_cycles.saturating_add(penalty);
            let already_accounted: u8 = 2 + penalty;
            add_cycles(state, total_cycles.saturating_sub(already_accounted));
            total_cycles
        }

        0xEF => {
            // SBC absolute long
            let addr = read_absolute_long_address_generic(state, bus);
            let memory_8bit = state.emulation_mode || state.p.contains(StatusFlags::MEMORY_8BIT);
            let operand = if memory_8bit {
                bus.read_u8(addr) as u16
            } else {
                bus.read_u16(addr)
            };
            sbc_generic(state, operand);
            let base_cycles: u8 = if memory_8bit { 5 } else { 6 };
            let already_accounted: u8 = 3;
            add_cycles(state, base_cycles.saturating_sub(already_accounted));
            base_cycles
        }

        0xFF => {
            // SBC absolute long,X
            let addr = read_absolute_long_x_address_generic(state, bus);
            let memory_8bit = state.emulation_mode || state.p.contains(StatusFlags::MEMORY_8BIT);
            let operand = if memory_8bit {
                bus.read_u8(addr) as u16
            } else {
                bus.read_u16(addr)
            };
            sbc_generic(state, operand);
            let base_cycles: u8 = if memory_8bit { 5 } else { 6 };
            let already_accounted: u8 = 3;
            add_cycles(state, base_cycles.saturating_sub(already_accounted));
            base_cycles
        }

        0xE1 => {
            // SBC (dp,X)
            let (addr, penalty) = read_indirect_x_address_generic(state, bus);
            if penalty != 0 {
                add_cycles(state, penalty);
            }
            let memory_8bit = state.emulation_mode || state.p.contains(StatusFlags::MEMORY_8BIT);
            let operand = if memory_8bit {
                bus.read_u8(addr) as u16
            } else {
                bus.read_u16(addr)
            };
            sbc_generic(state, operand);
            let base_cycles: u8 = if memory_8bit { 6 } else { 7 };
            let total_cycles = base_cycles.saturating_add(penalty);
            let already_accounted: u8 = 1 + penalty;
            add_cycles(state, total_cycles.saturating_sub(already_accounted));
            total_cycles
        }

        0xF1 => {
            // SBC (dp),Y
            let (addr, penalty) = read_indirect_y_address_generic(state, bus);
            if penalty != 0 {
                add_cycles(state, penalty);
            }
            let memory_8bit = state.emulation_mode || state.p.contains(StatusFlags::MEMORY_8BIT);
            let operand = if memory_8bit {
                bus.read_u8(addr) as u16
            } else {
                bus.read_u16(addr)
            };
            sbc_generic(state, operand);
            let base_cycles: u8 = if memory_8bit { 5 } else { 6 };
            let total_cycles = base_cycles.saturating_add(penalty);
            let already_accounted: u8 = 1 + penalty;
            add_cycles(state, total_cycles.saturating_sub(already_accounted));
            total_cycles
        }

        0xE7 => {
            // SBC [dp]
            let (addr, penalty) = read_indirect_long_address_generic(state, bus);
            if penalty != 0 {
                add_cycles(state, penalty);
            }
            let memory_8bit = state.emulation_mode || state.p.contains(StatusFlags::MEMORY_8BIT);
            let operand = if memory_8bit {
                bus.read_u8(addr) as u16
            } else {
                bus.read_u16(addr)
            };
            sbc_generic(state, operand);
            let base_cycles: u8 = if memory_8bit { 6 } else { 7 };
            let total_cycles = base_cycles.saturating_add(penalty);
            let already_accounted: u8 = 1 + penalty;
            add_cycles(state, total_cycles.saturating_sub(already_accounted));
            total_cycles
        }

        0xF7 => {
            // SBC [dp],Y
            let (addr, penalty) = read_indirect_long_y_address_generic(state, bus);
            if penalty != 0 {
                add_cycles(state, penalty);
            }
            let memory_8bit = state.emulation_mode || state.p.contains(StatusFlags::MEMORY_8BIT);
            let operand = if memory_8bit {
                bus.read_u8(addr) as u16
            } else {
                bus.read_u16(addr)
            };
            sbc_generic(state, operand);
            let base_cycles: u8 = if memory_8bit { 6 } else { 7 };
            let total_cycles = base_cycles.saturating_add(penalty);
            let already_accounted: u8 = 1 + penalty;
            add_cycles(state, total_cycles.saturating_sub(already_accounted));
            total_cycles
        }

        0xE3 => {
            // SBC stack relative
            let addr = read_stack_relative_address_generic(state, bus);
            let memory_8bit = state.emulation_mode || state.p.contains(StatusFlags::MEMORY_8BIT);
            let operand = if memory_8bit {
                bus.read_u8(addr) as u16
            } else {
                bus.read_u16(addr)
            };
            sbc_generic(state, operand);
            let base_cycles: u8 = if memory_8bit { 4 } else { 5 };
            let already_accounted: u8 = 1;
            add_cycles(state, base_cycles.saturating_sub(already_accounted));
            base_cycles
        }

        0xF3 => {
            // SBC (sr,S),Y
            let (addr, penalty) = read_stack_relative_indirect_y_generic(state, bus);
            if penalty != 0 {
                add_cycles(state, penalty);
            }
            let memory_8bit = state.emulation_mode || state.p.contains(StatusFlags::MEMORY_8BIT);
            let operand = if memory_8bit {
                bus.read_u8(addr) as u16
            } else {
                bus.read_u16(addr)
            };
            sbc_generic(state, operand);
            let base_cycles: u8 = if memory_8bit { 7 } else { 8 };
            let total_cycles = base_cycles.saturating_add(penalty);
            let already_accounted: u8 = 1 + penalty;
            add_cycles(state, total_cycles.saturating_sub(already_accounted));
            total_cycles
        }

        // Logical operations
        0x29 => {
            // AND immediate
            let memory_8bit = memory_is_8bit(state);
            let operand = if memory_8bit {
                read_u8_generic(state, bus) as u16
            } else {
                read_u16_generic(state, bus)
            };
            and_operand(state, operand);
            let total_cycles: u8 = if memory_8bit { 2 } else { 3 };
            let already_accounted: u8 = if memory_8bit { 1 } else { 2 };
            add_cycles(state, total_cycles.saturating_sub(already_accounted));
            total_cycles
        }
        0x49 => {
            // EOR immediate
            let memory_8bit = memory_is_8bit(state);
            let operand = if memory_8bit {
                read_u8_generic(state, bus) as u16
            } else {
                read_u16_generic(state, bus)
            };
            eor_operand(state, operand);
            let total_cycles: u8 = if memory_8bit { 2 } else { 3 };
            let already_accounted: u8 = if memory_8bit { 1 } else { 2 };
            add_cycles(state, total_cycles.saturating_sub(already_accounted));
            total_cycles
        }
        0x09 => {
            // ORA immediate
            let memory_8bit = memory_is_8bit(state);
            let operand = if memory_8bit {
                read_u8_generic(state, bus) as u16
            } else {
                read_u16_generic(state, bus)
            };
            ora_operand(state, operand);
            let total_cycles: u8 = if memory_8bit { 2 } else { 3 };
            let already_accounted: u8 = if memory_8bit { 1 } else { 2 };
            add_cycles(state, total_cycles.saturating_sub(already_accounted));
            total_cycles
        }

        // Additional critical instructions used by DQ3 SA-1
        0xD8 => {
            // CLD - Clear Decimal Mode Flag
            state.p.remove(StatusFlags::DECIMAL);
            add_cycles(state, 2);
            2
        }
        0x7A => {
            // PLY - Pull Y from Stack
            if state.p.contains(StatusFlags::INDEX_8BIT) {
                state.y = (state.y & 0xFF00) | (pop_u8_generic(state, bus) as u16);
                set_flags_nz_8(state, state.y as u8);
                add_cycles(state, 4);
                4
            } else {
                state.y = pop_u16_generic(state, bus);
                set_flags_nz_16(state, state.y);
                add_cycles(state, 5);
                5
            }
        }

        0x7B => {
            // TDC - Transfer Direct Page register to Accumulator
            state.a = state.dp;
            if memory_is_8bit(state) {
                set_flags_nz_8(state, (state.a & 0xFF) as u8);
            } else {
                set_flags_nz_16(state, state.a);
            }
            add_cycles(state, 2);
            2
        }
        0xCE => {
            // DEC absolute - Decrement Absolute Memory
            let addr = read_absolute_address_generic(state, bus);
            if state.p.contains(StatusFlags::MEMORY_8BIT) {
                let value = bus.read_u8(addr).wrapping_sub(1);
                write_u8_generic(bus, addr, value);
                set_flags_nz_8(state, value);
                add_cycles(state, 6);
                6
            } else {
                let value = bus.read_u16(addr).wrapping_sub(1);
                bus.write_u16(addr, value);
                set_flags_nz_16(state, value);
                add_cycles(state, 7);
                7
            }
        }
        0xCB => {
            // WAI - Wait for Interrupt
            // SA-1 should halt until next interrupt
            if !crate::debug_flags::quiet() {
                println!(
                    "🛑 SA-1 WAI: Waiting for interrupt at ${:02X}:{:04X}",
                    state.pb, state.pc
                );
            }
            add_cycles(state, 3);
            3
        }

        0xDB => {
            // STP - Stop the processor until reset
            state.stopped = true;
            add_cycles(state, 3);
            3
        }
        0xCC => {
            // CPY absolute - Compare Y with Absolute Memory
            let addr = read_absolute_address_generic(state, bus);
            if state.p.contains(StatusFlags::INDEX_8BIT) {
                let value = bus.read_u8(addr);
                let result = (state.y as u8).wrapping_sub(value);
                state.p.set(StatusFlags::CARRY, (state.y as u8) >= value);
                state.p.set(StatusFlags::ZERO, result == 0);
                state.p.set(StatusFlags::NEGATIVE, (result & 0x80) != 0);
                add_cycles(state, 4);
                4
            } else {
                let value = bus.read_u16(addr);
                let result = state.y.wrapping_sub(value);
                state.p.set(StatusFlags::CARRY, state.y >= value);
                state.p.set(StatusFlags::ZERO, result == 0);
                state.p.set(StatusFlags::NEGATIVE, (result & 0x8000) != 0);
                add_cycles(state, 5);
                5
            }
        }
        0xC3 => {
            // CMP stack relative indirect indexed
            let stack_offset = read_u8_generic(state, bus);
            let base_addr = state.sp.wrapping_add(stack_offset as u16);
            let indirect_addr = bus.read_u16(base_addr as u32);
            let final_addr = indirect_addr.wrapping_add(state.y);

            if state.p.contains(StatusFlags::MEMORY_8BIT) {
                let value = bus.read_u8(final_addr as u32);
                let result = (state.a as u8).wrapping_sub(value);
                state.p.set(StatusFlags::CARRY, (state.a as u8) >= value);
                state.p.set(StatusFlags::ZERO, result == 0);
                state.p.set(StatusFlags::NEGATIVE, (result & 0x80) != 0);
                add_cycles(state, 7);
                7
            } else {
                let value = bus.read_u16(final_addr as u32);
                let result = state.a.wrapping_sub(value);
                state.p.set(StatusFlags::CARRY, state.a >= value);
                state.p.set(StatusFlags::ZERO, result == 0);
                state.p.set(StatusFlags::NEGATIVE, (result & 0x8000) != 0);
                add_cycles(state, 8);
                8
            }
        }

        // Second batch of critical instructions used by DQ3 SA-1
        0xB3 => {
            // LDA stack relative indirect indexed (sr,S),Y
            let stack_offset = read_u8_generic(state, bus);
            let base_addr = state.sp.wrapping_add(stack_offset as u16);
            let indirect_addr = bus.read_u16(base_addr as u32);
            let final_addr = indirect_addr.wrapping_add(state.y);

            if state.p.contains(StatusFlags::MEMORY_8BIT) {
                let value = bus.read_u8(final_addr as u32);
                state.a = (state.a & 0xFF00) | (value as u16);
                set_flags_nz_8(state, value);
                add_cycles(state, 7);
                7
            } else {
                let value = bus.read_u16(final_addr as u32);
                state.a = value;
                set_flags_nz_16(state, value);
                add_cycles(state, 8);
                8
            }
        }
        0xC4 => {
            // CPY direct page
            let addr = read_u8_generic(state, bus) as u32 + state.dp as u32;
            if state.p.contains(StatusFlags::INDEX_8BIT) {
                let value = bus.read_u8(addr);
                let result = (state.y as u8).wrapping_sub(value);
                state.p.set(StatusFlags::CARRY, (state.y as u8) >= value);
                state.p.set(StatusFlags::ZERO, result == 0);
                state.p.set(StatusFlags::NEGATIVE, (result & 0x80) != 0);
                add_cycles(state, 3);
                3
            } else {
                let value = bus.read_u16(addr);
                let result = state.y.wrapping_sub(value);
                state.p.set(StatusFlags::CARRY, state.y >= value);
                state.p.set(StatusFlags::ZERO, result == 0);
                state.p.set(StatusFlags::NEGATIVE, (result & 0x8000) != 0);
                add_cycles(state, 4);
                4
            }
        }
        0xDF => {
            // CMP long,X - Compare with Long Indexed X
            let addr_lo = bus.read_u8(full_address(state, state.pc)) as u32;
            let addr_hi = bus.read_u8(full_address(state, state.pc + 1)) as u32;
            let addr_bank = bus.read_u8(full_address(state, state.pc + 2)) as u32;
            state.pc = state.pc.wrapping_add(3);
            let addr = (addr_lo | (addr_hi << 8) | (addr_bank << 16)).wrapping_add(state.x as u32);

            if state.p.contains(StatusFlags::MEMORY_8BIT) {
                let value = bus.read_u8(addr);
                let result = (state.a as u8).wrapping_sub(value);
                state.p.set(StatusFlags::CARRY, (state.a as u8) >= value);
                state.p.set(StatusFlags::ZERO, result == 0);
                state.p.set(StatusFlags::NEGATIVE, (result & 0x80) != 0);
                add_cycles(state, 5);
                5
            } else {
                let value = bus.read_u16(addr);
                let result = state.a.wrapping_sub(value);
                state.p.set(StatusFlags::CARRY, state.a >= value);
                state.p.set(StatusFlags::ZERO, result == 0);
                state.p.set(StatusFlags::NEGATIVE, (result & 0x8000) != 0);
                add_cycles(state, 6);
                6
            }
        }
        0xF8 => {
            // SED - Set Decimal Mode Flag
            state.p.insert(StatusFlags::DECIMAL);
            add_cycles(state, 2);
            2
        }
        // Third batch of critical instructions used by DQ3 SA-1
        // Fourth batch: Critical instructions found in DQ3 SA-1 execution
        0xB6 => {
            // LDX direct page,Y
            let addr =
                (read_u8_generic(state, bus) as u32 + state.dp as u32 + state.y as u32) & 0xFFFFFF;
            if state.p.contains(StatusFlags::INDEX_8BIT) {
                let value = bus.read_u8(addr);
                state.x = (state.x & 0xFF00) | (value as u16);
                set_flags_nz_8(state, value);
                add_cycles(state, 4);
                4
            } else {
                let value = bus.read_u16(addr);
                state.x = value;
                set_flags_nz_16(state, value);
                add_cycles(state, 5);
                5
            }
        }

        0xBE => {
            // LDX absolute,Y
            let (addr, penalty) = read_absolute_y_address_generic(state, bus);
            if penalty != 0 {
                add_cycles(state, penalty);
            }
            if index_is_8bit(state) {
                let value = bus.read_u8(addr);
                state.x = (state.x & 0xFF00) | (value as u16);
                set_flags_nz_8(state, value);
            } else {
                let value = bus.read_u16(addr);
                state.x = value;
                set_flags_nz_16(state, value);
            }
            let base_cycles: u8 = if index_is_8bit(state) { 4 } else { 5 };
            let total_cycles = base_cycles.saturating_add(penalty);
            let already_accounted: u8 = 2 + penalty;
            add_cycles(state, total_cycles.saturating_sub(already_accounted));
            total_cycles
        }

        0x58 => {
            // CLI - Clear Interrupt Disable
            state.p.remove(StatusFlags::IRQ_DISABLE);
            add_cycles(state, 2);
            2
        }

        0x08 => {
            // PHP - Push Processor Status
            let value = state.p.bits();
            push_u8_generic(state, bus, value);
            add_cycles(state, 3);
            3
        }

        0xB4 => {
            // LDY direct page,X
            let addr =
                (read_u8_generic(state, bus) as u32 + state.dp as u32 + state.x as u32) & 0xFFFFFF;
            if state.p.contains(StatusFlags::INDEX_8BIT) {
                let value = bus.read_u8(addr);
                state.y = (state.y & 0xFF00) | (value as u16);
                set_flags_nz_8(state, value);
                add_cycles(state, 4);
                4
            } else {
                let value = bus.read_u16(addr);
                state.y = value;
                set_flags_nz_16(state, value);
                add_cycles(state, 5);
                5
            }
        }

        0x10 => branch_if_generic(state, bus, !state.p.contains(StatusFlags::NEGATIVE)),

        0xD5 => {
            // CMP direct page,X
            let addr =
                (read_u8_generic(state, bus) as u32 + state.dp as u32 + state.x as u32) & 0xFFFFFF;
            if state.p.contains(StatusFlags::MEMORY_8BIT) {
                let value = bus.read_u8(addr);
                let a_low = state.a as u8;
                let result = a_low.wrapping_sub(value);
                state.p.set(StatusFlags::CARRY, a_low >= value);
                state.p.set(StatusFlags::ZERO, result == 0);
                state.p.set(StatusFlags::NEGATIVE, (result & 0x80) != 0);
                add_cycles(state, 4);
                4
            } else {
                let value = bus.read_u16(addr);
                let result = state.a.wrapping_sub(value);
                state.p.set(StatusFlags::CARRY, state.a >= value);
                state.p.set(StatusFlags::ZERO, result == 0);
                state.p.set(StatusFlags::NEGATIVE, (result & 0x8000) != 0);
                add_cycles(state, 5);
                5
            }
        }

        0xD6 => {
            // DEC direct page,X
            let (addr, penalty) = read_direct_x_address_generic(state, bus);
            if penalty != 0 {
                add_cycles(state, penalty);
            }
            if memory_is_8bit(state) {
                let value = bus.read_u8(addr).wrapping_sub(1);
                bus.write_u8(addr, value);
                set_flags_nz_8(state, value);
            } else {
                let value = bus.read_u16(addr).wrapping_sub(1);
                bus.write_u16(addr, value);
                set_flags_nz_16(state, value);
            }
            let base_cycles: u8 = 6;
            let total_cycles = base_cycles.saturating_add(penalty);
            let already_accounted: u8 = 1 + penalty;
            add_cycles(state, total_cycles.saturating_sub(already_accounted));
            total_cycles
        }

        0xB9 => {
            // LDA absolute,Y
            let addr_base = read_u16_generic(state, bus) as u32;
            let addr = (addr_base + state.y as u32) & 0xFFFFFF;
            if state.p.contains(StatusFlags::MEMORY_8BIT) {
                let value = bus.read_u8(addr);
                state.a = (state.a & 0xFF00) | (value as u16);
                set_flags_nz_8(state, value);
                add_cycles(state, 4);
                4
            } else {
                let value = bus.read_u16(addr);
                state.a = value;
                set_flags_nz_16(state, value);
                add_cycles(state, 5);
                5
            }
        }

        0xBA => {
            // TSX - Transfer Stack Pointer to X
            if state.p.contains(StatusFlags::INDEX_8BIT) {
                let sp_low = state.sp as u8;
                state.x = (state.x & 0xFF00) | (sp_low as u16);
                set_flags_nz_8(state, sp_low);
            } else {
                state.x = state.sp;
                set_flags_nz_16(state, state.sp);
            }
            add_cycles(state, 2);
            2
        }

        0xBB => {
            // TYX - Transfer Y to X
            state.x = state.y;
            set_flags_index(state, state.x);
            add_cycles(state, 2);
            2
        }

        0xD4 => {
            // PEI - Push Effective Indirect Address
            let dp_offset = read_u8_generic(state, bus) as u32;
            let indirect_addr = (state.dp as u32 + dp_offset) & 0xFFFFFF;
            let effective_addr = bus.read_u16(indirect_addr);
            push_u16_generic(state, bus, effective_addr);
            add_cycles(state, 6);
            6
        }

        // Fifth batch: More critical instructions found in DQ3 SA-1 execution
        0x0C => {
            // TSB absolute - Test and Set Bits
            let addr = read_u16_generic(state, bus) as u32;
            if state.p.contains(StatusFlags::MEMORY_8BIT) {
                let value = bus.read_u8(addr);
                let a_low = state.a as u8;
                state.p.set(StatusFlags::ZERO, (a_low & value) == 0);
                bus.write_u8(addr, value | a_low);
                add_cycles(state, 6);
                6
            } else {
                let value = bus.read_u16(addr);
                state.p.set(StatusFlags::ZERO, (state.a & value) == 0);
                bus.write_u16(addr, value | state.a);
                add_cycles(state, 8);
                8
            }
        }

        0xC1 => {
            // CMP indirect,X
            let dp_base = read_u8_generic(state, bus) as u32;
            let indirect_addr = ((state.dp as u32 + dp_base + state.x as u32) & 0xFFFF) as u32;
            let target_addr = bus.read_u16(indirect_addr);
            if state.p.contains(StatusFlags::MEMORY_8BIT) {
                let value = bus.read_u8(target_addr as u32);
                let a_low = state.a as u8;
                let result = a_low.wrapping_sub(value);
                state.p.set(StatusFlags::CARRY, a_low >= value);
                state.p.set(StatusFlags::ZERO, result == 0);
                state.p.set(StatusFlags::NEGATIVE, (result & 0x80) != 0);
                add_cycles(state, 6);
                6
            } else {
                let value = bus.read_u16(target_addr as u32);
                let result = state.a.wrapping_sub(value);
                state.p.set(StatusFlags::CARRY, state.a >= value);
                state.p.set(StatusFlags::ZERO, result == 0);
                state.p.set(StatusFlags::NEGATIVE, (result & 0x8000) != 0);
                add_cycles(state, 7);
                7
            }
        }

        0xC5 => {
            // CMP direct page
            let (addr, penalty) = read_direct_address_generic(state, bus);
            if penalty != 0 {
                add_cycles(state, penalty);
            }
            if memory_is_8bit(state) {
                let value = bus.read_u8(addr);
                let a_low = (state.a & 0xFF) as u8;
                let result = a_low.wrapping_sub(value);
                state.p.set(StatusFlags::CARRY, a_low >= value);
                state.p.set(StatusFlags::ZERO, result == 0);
                state.p.set(StatusFlags::NEGATIVE, (result & 0x80) != 0);
                let base_cycles: u8 = 3;
                let total_cycles = base_cycles.saturating_add(penalty);
                let already_accounted: u8 = 1 + penalty;
                add_cycles(state, total_cycles.saturating_sub(already_accounted));
                total_cycles
            } else {
                let value = bus.read_u16(addr);
                let result = state.a.wrapping_sub(value);
                state.p.set(StatusFlags::CARRY, state.a >= value);
                state.p.set(StatusFlags::ZERO, result == 0);
                state.p.set(StatusFlags::NEGATIVE, (result & 0x8000) != 0);
                let base_cycles: u8 = 4;
                let total_cycles = base_cycles.saturating_add(penalty);
                let already_accounted: u8 = 1 + penalty;
                add_cycles(state, total_cycles.saturating_sub(already_accounted));
                total_cycles
            }
        }

        0xC6 => {
            // DEC direct page
            let (addr, penalty) = read_direct_address_generic(state, bus);
            if penalty != 0 {
                add_cycles(state, penalty);
            }
            if memory_is_8bit(state) {
                let value = bus.read_u8(addr).wrapping_sub(1);
                bus.write_u8(addr, value);
                set_flags_nz_8(state, value);
            } else {
                let value = bus.read_u16(addr).wrapping_sub(1);
                bus.write_u16(addr, value);
                set_flags_nz_16(state, value);
            }
            let base_cycles: u8 = 5;
            let total_cycles = base_cycles.saturating_add(penalty);
            let already_accounted: u8 = 1 + penalty;
            add_cycles(state, total_cycles.saturating_sub(already_accounted));
            total_cycles
        }

        0xC7 => {
            // CMP [dp]
            let (addr, penalty) = read_indirect_long_address_generic(state, bus);
            if penalty != 0 {
                add_cycles(state, penalty);
            }
            if memory_is_8bit(state) {
                let value = bus.read_u8(addr);
                let a_low = (state.a & 0xFF) as u8;
                let result = a_low.wrapping_sub(value);
                state.p.set(StatusFlags::CARRY, a_low >= value);
                state.p.set(StatusFlags::ZERO, result == 0);
                state.p.set(StatusFlags::NEGATIVE, (result & 0x80) != 0);
            } else {
                let value = bus.read_u16(addr);
                let result = state.a.wrapping_sub(value);
                state.p.set(StatusFlags::CARRY, state.a >= value);
                state.p.set(StatusFlags::ZERO, result == 0);
                state.p.set(StatusFlags::NEGATIVE, (result & 0x8000) != 0);
            }
            let base_cycles: u8 = 6;
            let total_cycles = base_cycles.saturating_add(penalty);
            let already_accounted: u8 = 1 + penalty;
            add_cycles(state, total_cycles.saturating_sub(already_accounted));
            total_cycles
        }

        0xC8 => {
            // INY
            if index_is_8bit(state) {
                let value = ((state.y & 0xFF).wrapping_add(1)) as u8;
                state.y = (state.y & 0xFF00) | (value as u16);
                set_flags_nz_8(state, value);
            } else {
                state.y = state.y.wrapping_add(1);
                set_flags_nz_16(state, state.y);
            }
            add_cycles(state, 2);
            2
        }

        0xCA => {
            // DEX
            if index_is_8bit(state) {
                let value = ((state.x & 0xFF).wrapping_sub(1)) as u8;
                state.x = (state.x & 0xFF00) | (value as u16);
                set_flags_nz_8(state, value);
            } else {
                state.x = state.x.wrapping_sub(1);
                set_flags_nz_16(state, state.x);
            }
            add_cycles(state, 2);
            2
        }

        0xCD => {
            // CMP absolute
            let addr = read_absolute_address_generic(state, bus);
            let operand = if memory_is_8bit(state) {
                bus.read_u8(addr) as u16
            } else {
                bus.read_u16(addr)
            };
            cmp_operand(state, operand);
            let base_cycles: u8 = 4;
            let already_accounted: u8 = 3;
            add_cycles(state, base_cycles.saturating_sub(already_accounted));
            base_cycles
        }

        0xCF => {
            // CMP absolute long
            let addr = read_absolute_long_address_generic(state, bus);
            let operand = if memory_is_8bit(state) {
                bus.read_u8(addr) as u16
            } else {
                bus.read_u16(addr)
            };
            cmp_operand(state, operand);
            let base_cycles: u8 = 5;
            let already_accounted: u8 = 3;
            add_cycles(state, base_cycles.saturating_sub(already_accounted));
            base_cycles
        }

        0xD1 => {
            // CMP (dp),Y
            let (addr, penalty) = read_indirect_y_address_generic(state, bus);
            if penalty != 0 {
                add_cycles(state, penalty);
            }
            let operand = if memory_is_8bit(state) {
                bus.read_u8(addr) as u16
            } else {
                bus.read_u16(addr)
            };
            cmp_operand(state, operand);
            let base_cycles: u8 = 5;
            let total_cycles = base_cycles.saturating_add(penalty);
            let already_accounted: u8 = 1 + penalty;
            add_cycles(state, total_cycles.saturating_sub(already_accounted));
            total_cycles
        }

        0xD2 => {
            // CMP (dp)
            let addr = read_indirect_address_generic(state, bus);
            let operand = if memory_is_8bit(state) {
                bus.read_u8(addr) as u16
            } else {
                bus.read_u16(addr)
            };
            cmp_operand(state, operand);
            let base_cycles: u8 = 5;
            let already_accounted: u8 = 2;
            add_cycles(state, base_cycles.saturating_sub(already_accounted));
            base_cycles
        }

        0xD3 => {
            // CMP (sr,S),Y
            let (addr, penalty) = read_stack_relative_indirect_y_generic(state, bus);
            if penalty != 0 {
                add_cycles(state, penalty);
            }
            let operand = if memory_is_8bit(state) {
                bus.read_u8(addr) as u16
            } else {
                bus.read_u16(addr)
            };
            cmp_operand(state, operand);
            let base_cycles: u8 = 7;
            let total_cycles = base_cycles.saturating_add(penalty);
            let already_accounted: u8 = 1 + penalty;
            add_cycles(state, total_cycles.saturating_sub(already_accounted));
            total_cycles
        }

        0xD7 => {
            // CMP [dp],Y
            let (addr, penalty) = read_indirect_long_y_address_generic(state, bus);
            if penalty != 0 {
                add_cycles(state, penalty);
            }
            let operand = if memory_is_8bit(state) {
                bus.read_u8(addr) as u16
            } else {
                bus.read_u16(addr)
            };
            cmp_operand(state, operand);
            let base_cycles: u8 = 6;
            let total_cycles = base_cycles.saturating_add(penalty);
            let already_accounted: u8 = 1 + penalty;
            add_cycles(state, total_cycles.saturating_sub(already_accounted));
            total_cycles
        }

        0xD9 => {
            // CMP absolute,Y
            let (addr, penalty) = read_absolute_y_address_generic(state, bus);
            if penalty != 0 {
                add_cycles(state, penalty);
            }
            let operand = if memory_is_8bit(state) {
                bus.read_u8(addr) as u16
            } else {
                bus.read_u16(addr)
            };
            cmp_operand(state, operand);
            let base_cycles: u8 = 4;
            let total_cycles = base_cycles.saturating_add(penalty);
            let already_accounted: u8 = 2 + penalty;
            add_cycles(state, total_cycles.saturating_sub(already_accounted));
            total_cycles
        }

        0xDD => {
            // CMP absolute,X
            let (addr, penalty) = read_absolute_x_address_generic(state, bus);
            if penalty != 0 {
                add_cycles(state, penalty);
            }
            let operand = if memory_is_8bit(state) {
                bus.read_u8(addr) as u16
            } else {
                bus.read_u16(addr)
            };
            cmp_operand(state, operand);
            let base_cycles: u8 = 4;
            let total_cycles = base_cycles.saturating_add(penalty);
            let already_accounted: u8 = 2 + penalty;
            add_cycles(state, total_cycles.saturating_sub(already_accounted));
            total_cycles
        }

        0xDE => {
            // DEC absolute,X
            let (addr, penalty) = read_absolute_x_address_generic(state, bus);
            if penalty != 0 {
                add_cycles(state, penalty);
            }
            if memory_is_8bit(state) {
                let value = bus.read_u8(addr).wrapping_sub(1);
                bus.write_u8(addr, value);
                set_flags_nz_8(state, value);
            } else {
                let value = bus.read_u16(addr).wrapping_sub(1);
                bus.write_u16(addr, value);
                set_flags_nz_16(state, value);
            }
            let base_cycles: u8 = 7;
            let total_cycles = base_cycles.saturating_add(penalty);
            let already_accounted: u8 = 2 + penalty;
            add_cycles(state, total_cycles.saturating_sub(already_accounted));
            total_cycles
        }

        0xE4 => {
            // CPX direct page
            let addr = (read_u8_generic(state, bus) as u32 + state.dp as u32) & 0xFFFFFF;
            if state.p.contains(StatusFlags::INDEX_8BIT) {
                let value = bus.read_u8(addr);
                let x_low = state.x as u8;
                let result = x_low.wrapping_sub(value);
                state.p.set(StatusFlags::CARRY, x_low >= value);
                state.p.set(StatusFlags::ZERO, result == 0);
                state.p.set(StatusFlags::NEGATIVE, (result & 0x80) != 0);
                add_cycles(state, 3);
                3
            } else {
                let value = bus.read_u16(addr);
                let result = state.x.wrapping_sub(value);
                state.p.set(StatusFlags::CARRY, state.x >= value);
                state.p.set(StatusFlags::ZERO, result == 0);
                state.p.set(StatusFlags::NEGATIVE, (result & 0x8000) != 0);
                add_cycles(state, 4);
                4
            }
        }

        0x2B => {
            // PLD - Pull Direct Page Register
            let lo = pop_u8_generic(state, bus) as u16;
            let hi = pop_u8_generic(state, bus) as u16;
            state.dp = (hi << 8) | lo;
            set_flags_nz_16(state, state.dp);
            add_cycles(state, 5);
            5
        }

        0x40 => {
            // RTI - Return from Interrupt
            if state.emulation_mode {
                let status = pop_u8_generic(state, bus);
                state.p = StatusFlags::from_bits_truncate(status);
                let lo = pop_u8_generic(state, bus) as u16;
                let hi = pop_u8_generic(state, bus) as u16;
                state.pc = (hi << 8) | lo;
                add_cycles(state, 6);
                6
            } else {
                let status = pop_u8_generic(state, bus);
                state.p = StatusFlags::from_bits_truncate(status);
                let lo = pop_u8_generic(state, bus) as u16;
                let hi = pop_u8_generic(state, bus) as u16;
                state.pc = (hi << 8) | lo;
                state.pb = pop_u8_generic(state, bus);
                add_cycles(state, 7);
                7
            }
        }

        0x30 => branch_if_generic(state, bus, state.p.contains(StatusFlags::NEGATIVE)),
        0x50 => branch_if_generic(state, bus, !state.p.contains(StatusFlags::OVERFLOW)),
        0x70 => branch_if_generic(state, bus, state.p.contains(StatusFlags::OVERFLOW)),
        0x80 => branch_if_generic(state, bus, true),
        0x82 => brl_generic(state, bus),
        0x90 => branch_if_generic(state, bus, !state.p.contains(StatusFlags::CARRY)),
        0xB0 => branch_if_generic(state, bus, state.p.contains(StatusFlags::CARRY)),
        0xD0 => branch_if_generic(state, bus, !state.p.contains(StatusFlags::ZERO)),
        0xF0 => branch_if_generic(state, bus, state.p.contains(StatusFlags::ZERO)),

        0xFE => {
            // INC absolute,X
            let addr_base = read_u16_generic(state, bus) as u32;
            let addr = (addr_base + state.x as u32) & 0xFFFFFF;
            if state.p.contains(StatusFlags::MEMORY_8BIT) {
                let value = bus.read_u8(addr).wrapping_add(1);
                bus.write_u8(addr, value);
                set_flags_nz_8(state, value);
                add_cycles(state, 7);
                7
            } else {
                let value = bus.read_u16(addr).wrapping_add(1);
                bus.write_u16(addr, value);
                set_flags_nz_16(state, value);
                add_cycles(state, 9);
                9
            }
        }

        // Missing DQ3 SA-1 critical instructions
        0x8F => {
            // STA long absolute
            let addr_lo = read_u8_generic(state, bus) as u32;
            let addr_hi = read_u8_generic(state, bus) as u32;
            let addr_bank = read_u8_generic(state, bus) as u32;
            let full_addr = addr_lo | (addr_hi << 8) | (addr_bank << 16);

            if state.p.contains(StatusFlags::MEMORY_8BIT) {
                bus.write_u8(full_addr, (state.a & 0xFF) as u8);
            } else {
                bus.write_u16(full_addr, state.a);
            }
            add_cycles(state, 5);
            5
        }

        0x42 => {
            // WDM (No operation on SA-1, but consume signature byte)
            read_u8_generic(state, bus); // Read and ignore signature byte
            add_cycles(state, 2);
            2
        }

        0x3A => {
            // DEC A (Decrement Accumulator)
            if state.p.contains(StatusFlags::MEMORY_8BIT) {
                state.a = ((state.a & 0xFF).wrapping_sub(1) & 0xFF) | (state.a & 0xFF00);
                set_flags_nz_8(state, (state.a & 0xFF) as u8);
            } else {
                state.a = state.a.wrapping_sub(1);
                set_flags_nz_16(state, state.a);
            }
            add_cycles(state, 2);
            2
        }

        0x3B => {
            // TSC - Transfer Stack Pointer to Accumulator
            state.a = state.sp;
            if memory_is_8bit(state) {
                set_flags_nz_8(state, (state.a & 0xFF) as u8);
            } else {
                set_flags_nz_16(state, state.a);
            }
            add_cycles(state, 2);
            2
        }

        0x9A => {
            // TXS (Transfer X to Stack Pointer)
            if state.emulation_mode {
                state.sp = 0x0100 | (state.x & 0xFF);
            } else {
                state.sp = state.x;
            }
            add_cycles(state, 2);
            2
        }

        0x9B => {
            // TXY - Transfer X to Y
            state.y = state.x;
            set_flags_index(state, state.y);
            add_cycles(state, 2);
            2
        }

        // Missing opcodes frequently encountered in DQ3 SA-1 code
        0x99 => {
            // STA absolute,Y
            let (addr, penalty) = read_absolute_y_address_generic(state, bus);
            if penalty != 0 {
                add_cycles(state, penalty);
            }
            write_a_generic(state, bus, addr);
            let base_cycles: u8 = 5;
            let total_cycles = base_cycles.saturating_add(penalty);
            let already_accounted: u8 = 2 + penalty;
            add_cycles(state, total_cycles.saturating_sub(already_accounted));
            total_cycles
        }

        0x9C => {
            // STZ absolute
            let addr = read_absolute_address_generic(state, bus);
            if memory_is_8bit(state) {
                bus.write_u8(addr, 0);
            } else {
                bus.write_u16(addr, 0);
            }
            let base_cycles: u8 = 4;
            let already_accounted: u8 = 3;
            add_cycles(state, base_cycles.saturating_sub(already_accounted));
            base_cycles
        }

        0x9D => {
            // STA absolute,X
            let (addr, penalty) = read_absolute_x_address_generic(state, bus);
            if penalty != 0 {
                add_cycles(state, penalty);
            }
            write_a_generic(state, bus, addr);
            let base_cycles: u8 = 5;
            let total_cycles = base_cycles.saturating_add(penalty);
            let already_accounted: u8 = 2 + penalty;
            add_cycles(state, total_cycles.saturating_sub(already_accounted));
            total_cycles
        }

        0x9E => {
            // STZ absolute,X
            let (addr, penalty) = read_absolute_x_address_generic(state, bus);
            if penalty != 0 {
                add_cycles(state, penalty);
            }
            if memory_is_8bit(state) {
                bus.write_u8(addr, 0);
            } else {
                bus.write_u16(addr, 0);
            }
            let base_cycles: u8 = 5;
            let total_cycles = base_cycles.saturating_add(penalty);
            let already_accounted: u8 = 2 + penalty;
            add_cycles(state, total_cycles.saturating_sub(already_accounted));
            total_cycles
        }

        0x9F => {
            // STA long absolute,X
            let addr_lo = read_u8_generic(state, bus) as u32;
            let addr_hi = read_u8_generic(state, bus) as u32;
            let addr_bank = read_u8_generic(state, bus) as u32;
            let full_addr =
                (addr_lo | (addr_hi << 8) | (addr_bank << 16)).wrapping_add(state.x as u32);

            if state.p.contains(StatusFlags::MEMORY_8BIT) {
                bus.write_u8(full_addr, (state.a & 0xFF) as u8);
            } else {
                bus.write_u16(full_addr, state.a);
            }
            add_cycles(state, 5);
            5
        }

        0xB1 => {
            // LDA (dp),Y
            let (addr, penalty) = read_indirect_y_address_generic(state, bus);
            if penalty != 0 {
                add_cycles(state, penalty);
            }
            let memory_8bit = memory_is_8bit(state);
            let value = read_operand_m(state, bus, addr, memory_8bit);
            if memory_8bit {
                state.a = (state.a & 0xFF00) | (value & 0xFF);
                set_flags_nz_8(state, value as u8);
            } else {
                state.a = value;
                set_flags_nz_16(state, value);
            }
            let base_cycles: u8 = 5;
            let total_cycles = base_cycles.saturating_add(penalty);
            let already_accounted: u8 = 1 + penalty;
            add_cycles(state, total_cycles.saturating_sub(already_accounted));
            total_cycles
        }

        0xB2 => {
            // LDA (dp)
            let addr = read_indirect_address_generic(state, bus);
            let memory_8bit = memory_is_8bit(state);
            let value = read_operand_m(state, bus, addr, memory_8bit);
            if memory_8bit {
                state.a = (state.a & 0xFF00) | (value & 0xFF);
                set_flags_nz_8(state, value as u8);
            } else {
                state.a = value;
                set_flags_nz_16(state, value);
            }
            let base_cycles: u8 = 5;
            let already_accounted: u8 = 2;
            add_cycles(state, base_cycles.saturating_sub(already_accounted));
            base_cycles
        }

        0xB7 => {
            // LDA [dp],Y
            let (addr, penalty) = read_indirect_long_y_address_generic(state, bus);
            if penalty != 0 {
                add_cycles(state, penalty);
            }
            let memory_8bit = memory_is_8bit(state);
            let value = read_operand_m(state, bus, addr, memory_8bit);
            if memory_8bit {
                state.a = (state.a & 0xFF00) | (value & 0xFF);
                set_flags_nz_8(state, value as u8);
            } else {
                state.a = value;
                set_flags_nz_16(state, value);
            }
            let base_cycles: u8 = 6;
            let total_cycles = base_cycles.saturating_add(penalty);
            let already_accounted: u8 = 1 + penalty;
            add_cycles(state, total_cycles.saturating_sub(already_accounted));
            total_cycles
        }

        0xB5 => {
            // LDA zero page,X
            let addr = (read_u8_generic(state, bus).wrapping_add((state.x & 0xFF) as u8)) as u32;
            if state.p.contains(StatusFlags::MEMORY_8BIT) {
                state.a = (state.a & 0xFF00) | (bus.read_u8(addr) as u16);
                set_flags_nz_8(state, (state.a & 0xFF) as u8);
            } else {
                state.a = bus.read_u16(addr);
                set_flags_nz_16(state, state.a);
            }
            add_cycles(state, 4);
            4
        }

        0x14 => {
            // TRB zero page
            let addr = read_u8_generic(state, bus) as u32;
            if state.p.contains(StatusFlags::MEMORY_8BIT) {
                let value = bus.read_u8(addr);
                let result = value & !(state.a as u8);
                bus.write_u8(addr, result);
                state
                    .p
                    .set(StatusFlags::ZERO, (value & (state.a as u8)) == 0);
            } else {
                let value = bus.read_u16(addr);
                let result = value & !state.a;
                bus.write_u16(addr, result);
                state.p.set(StatusFlags::ZERO, (value & state.a) == 0);
            }
            add_cycles(state, 5);
            5
        }

        0x1C => {
            // TRB absolute
            let addr = read_absolute_address_generic(state, bus);
            let value = bus.read_u8(addr);
            let a_low = (state.a & 0xFF) as u8;
            state.p.set(StatusFlags::ZERO, (value & a_low) == 0);
            bus.write_u8(addr, value & !a_low);
            let base_cycles: u8 = 6;
            let already_accounted: u8 = 3;
            add_cycles(state, base_cycles.saturating_sub(already_accounted));
            base_cycles
        }

        0x88 => {
            // DEY
            if index_is_8bit(state) {
                let value = ((state.y & 0xFF).wrapping_sub(1)) as u8;
                state.y = (state.y & 0xFF00) | (value as u16);
                set_flags_nz_8(state, value);
            } else {
                state.y = state.y.wrapping_sub(1);
                set_flags_nz_16(state, state.y);
            }
            add_cycles(state, 2);
            2
        }

        0x8A => {
            // TXA (Transfer X to Accumulator)
            if state.p.contains(StatusFlags::MEMORY_8BIT) {
                state.a = (state.a & 0xFF00) | ((state.x & 0xFF) as u16);
                set_flags_nz_8(state, (state.a & 0xFF) as u8);
            } else {
                state.a = state.x;
                set_flags_nz_16(state, state.a);
            }
            add_cycles(state, 2);
            2
        }

        0x98 => {
            // TYA (Transfer Y to Accumulator)
            if memory_is_8bit(state) {
                state.a = (state.a & 0xFF00) | ((state.y & 0xFF) as u16);
                set_flags_nz_8(state, (state.a & 0xFF) as u8);
            } else {
                state.a = state.y;
                set_flags_nz_16(state, state.a);
            }
            add_cycles(state, 2);
            2
        }

        0xAD => {
            // LDA absolute
            let addr_lo = read_u8_generic(state, bus) as u32;
            let addr_hi = read_u8_generic(state, bus) as u32;
            let addr = addr_lo | (addr_hi << 8);
            if state.p.contains(StatusFlags::MEMORY_8BIT) {
                state.a = (state.a & 0xFF00) | (bus.read_u8(addr) as u16);
                set_flags_nz_8(state, (state.a & 0xFF) as u8);
            } else {
                state.a = bus.read_u16(addr);
                set_flags_nz_16(state, state.a);
            }
            add_cycles(state, 4);
            4
        }

        0xAF => {
            // LDA absolute long
            let addr = read_absolute_long_address_generic(state, bus);
            let memory_8bit = memory_is_8bit(state);
            let value = read_operand_m(state, bus, addr, memory_8bit);
            if memory_8bit {
                state.a = (state.a & 0xFF00) | (value & 0xFF);
                set_flags_nz_8(state, value as u8);
            } else {
                state.a = value;
                set_flags_nz_16(state, value);
            }
            let base_cycles: u8 = 5;
            let already_accounted: u8 = 3;
            add_cycles(state, base_cycles.saturating_sub(already_accounted));
            base_cycles
        }

        0xBD => {
            // LDA absolute,X with page-cross penalty
            let (addr, penalty) = read_absolute_x_address_generic(state, bus);
            if penalty != 0 {
                add_cycles(state, penalty);
            }
            let memory_8bit = state.emulation_mode || state.p.contains(StatusFlags::MEMORY_8BIT);
            if memory_8bit {
                let value = bus.read_u8(addr) as u16;
                state.a = (state.a & 0xFF00) | value;
                set_flags_nz_8(state, value as u8);
            } else {
                let value = bus.read_u16(addr);
                state.a = value;
                set_flags_nz_16(state, value);
            }
            let base_cycles: u8 = 4;
            let total_cycles = base_cycles.saturating_add(penalty);
            let already_accounted: u8 = 2 + penalty; // operand fetch (2) + penalty already applied
            add_cycles(state, total_cycles.saturating_sub(already_accounted));
            total_cycles
        }

        0xBF => {
            // LDA absolute long,X
            let addr = read_absolute_long_x_address_generic(state, bus);
            let memory_8bit = memory_is_8bit(state);
            let value = read_operand_m(state, bus, addr, memory_8bit);
            if memory_8bit {
                state.a = (state.a & 0xFF00) | (value & 0xFF);
                set_flags_nz_8(state, value as u8);
            } else {
                state.a = value;
                set_flags_nz_16(state, value);
            }
            let base_cycles: u8 = 5;
            let already_accounted: u8 = 3;
            add_cycles(state, base_cycles.saturating_sub(already_accounted));
            base_cycles
        }

        0x28 => {
            // PLP (Pull Processor Status)
            let value = pop_u8_generic(state, bus);
            state.p = StatusFlags::from_bits_truncate(value);
            add_cycles(state, 4);
            4
        }

        0x5B => {
            // TCD (Transfer Accumulator to Direct Page)
            state.dp = state.a;
            set_flags_nz_16(state, state.dp);
            add_cycles(state, 2);
            2
        }

        0xBC => {
            // LDY absolute,X
            let addr_lo = read_u8_generic(state, bus) as u32;
            let addr_hi = read_u8_generic(state, bus) as u32;
            let addr = (addr_lo | (addr_hi << 8)).wrapping_add(state.x as u32);
            if state.p.contains(StatusFlags::INDEX_8BIT) {
                state.y = (state.y & 0xFF00) | (bus.read_u8(addr) as u16);
                set_flags_nz_8(state, (state.y & 0xFF) as u8);
            } else {
                state.y = bus.read_u16(addr);
                set_flags_nz_16(state, state.y);
            }
            add_cycles(state, 4);
            4
        }

        0x83 => {
            // STA stack relative,S
            let offset = read_u8_generic(state, bus) as u16;
            let addr = state.sp.wrapping_add(offset);
            if state.p.contains(StatusFlags::MEMORY_8BIT) {
                bus.write_u8(addr as u32, (state.a & 0xFF) as u8);
            } else {
                bus.write_u16(addr as u32, state.a);
            }
            add_cycles(state, 4);
            4
        }

        0x94 => {
            // STY zero page,X
            let (addr, penalty) = read_direct_x_address_generic(state, bus);
            if penalty != 0 {
                add_cycles(state, penalty);
            }
            write_y_generic(state, bus, addr);
            let base_cycles: u8 = 4;
            let total_cycles = base_cycles.saturating_add(penalty);
            let already_accounted: u8 = 1 + penalty;
            add_cycles(state, total_cycles.saturating_sub(already_accounted));
            total_cycles
        }

        0x96 => {
            // STX zero page,Y
            let addr = (read_u8_generic(state, bus).wrapping_add((state.y & 0xFF) as u8)) as u32;
            if state.p.contains(StatusFlags::INDEX_8BIT) {
                bus.write_u8(addr, (state.x & 0xFF) as u8);
            } else {
                bus.write_u16(addr, state.x);
            }
            add_cycles(state, 4);
            4
        }

        // Critical instructions for DQ3 SA-1 BW-RAM communication
        0x64 => {
            // STZ direct page
            let (addr, penalty) = read_direct_address_generic(state, bus);
            if penalty != 0 {
                add_cycles(state, penalty);
            }
            if memory_is_8bit(state) {
                bus.write_u8(addr, 0);
            } else {
                bus.write_u16(addr, 0);
            }
            let base_cycles: u8 = 3;
            let total_cycles = base_cycles.saturating_add(penalty);
            let already_accounted: u8 = 1 + penalty;
            add_cycles(state, total_cycles.saturating_sub(already_accounted));
            total_cycles
        }

        0x74 => {
            // STZ direct page,X
            let (addr, penalty) = read_direct_x_address_generic(state, bus);
            if penalty != 0 {
                add_cycles(state, penalty);
            }
            if memory_is_8bit(state) {
                bus.write_u8(addr, 0);
            } else {
                bus.write_u16(addr, 0);
            }
            let base_cycles: u8 = 4;
            let total_cycles = base_cycles.saturating_add(penalty);
            let already_accounted: u8 = 1 + penalty;
            add_cycles(state, total_cycles.saturating_sub(already_accounted));
            total_cycles
        }

        0x81 => {
            // STA (dp,X)
            let (addr, penalty) = read_indirect_x_address_generic(state, bus);
            if penalty != 0 {
                add_cycles(state, penalty);
            }
            write_a_generic(state, bus, addr);
            let base_cycles: u8 = 6;
            let total_cycles = base_cycles.saturating_add(penalty);
            let already_accounted: u8 = 1 + penalty;
            add_cycles(state, total_cycles.saturating_sub(already_accounted));
            total_cycles
        }

        0x87 => {
            // STA [dp]
            let (addr, penalty) = read_indirect_long_address_generic(state, bus);
            if penalty != 0 {
                add_cycles(state, penalty);
            }
            write_a_generic(state, bus, addr);
            let base_cycles: u8 = 6;
            let total_cycles = base_cycles.saturating_add(penalty);
            let already_accounted: u8 = 1 + penalty;
            add_cycles(state, total_cycles.saturating_sub(already_accounted));
            total_cycles
        }

        0x91 => {
            // STA (dp),Y
            let (addr, penalty) = read_indirect_y_address_generic(state, bus);
            if penalty != 0 {
                add_cycles(state, penalty);
            }
            write_a_generic(state, bus, addr);
            let base_cycles: u8 = 6;
            let total_cycles = base_cycles.saturating_add(penalty);
            let already_accounted: u8 = 1 + penalty;
            add_cycles(state, total_cycles.saturating_sub(already_accounted));
            total_cycles
        }

        0x92 => {
            // STA (dp)
            let addr = read_indirect_address_generic(state, bus);
            write_a_generic(state, bus, addr);
            let base_cycles: u8 = 5;
            let already_accounted: u8 = 2;
            add_cycles(state, base_cycles.saturating_sub(already_accounted));
            base_cycles
        }

        0x93 => {
            // STA (sr,S),Y
            let (addr, penalty) = read_stack_relative_indirect_y_generic(state, bus);
            if penalty != 0 {
                add_cycles(state, penalty);
            }
            write_a_generic(state, bus, addr);
            let base_cycles: u8 = 7;
            let total_cycles = base_cycles.saturating_add(penalty);
            let already_accounted: u8 = 1 + penalty;
            add_cycles(state, total_cycles.saturating_sub(already_accounted));
            total_cycles
        }

        0x95 => {
            // STA dp,X
            let (addr, penalty) = read_direct_x_address_generic(state, bus);
            if penalty != 0 {
                add_cycles(state, penalty);
            }
            write_a_generic(state, bus, addr);
            let base_cycles: u8 = 4;
            let total_cycles = base_cycles.saturating_add(penalty);
            let already_accounted: u8 = 1 + penalty;
            add_cycles(state, total_cycles.saturating_sub(already_accounted));
            total_cycles
        }

        0x97 => {
            // STA [dp],Y
            let (addr, penalty) = read_indirect_long_y_address_generic(state, bus);
            if penalty != 0 {
                add_cycles(state, penalty);
            }
            write_a_generic(state, bus, addr);
            let base_cycles: u8 = 6;
            let total_cycles = base_cycles.saturating_add(penalty);
            let already_accounted: u8 = 1 + penalty;
            add_cycles(state, total_cycles.saturating_sub(already_accounted));
            total_cycles
        }

        0x85 => {
            // STA zero page
            let addr = read_u8_generic(state, bus) as u32;
            if state.p.contains(StatusFlags::MEMORY_8BIT) {
                bus.write_u8(addr, (state.a & 0xFF) as u8);
            } else {
                bus.write_u16(addr, state.a);
            }
            add_cycles(state, 3);
            3
        }

        0x86 => {
            // STX zero page
            let addr = read_u8_generic(state, bus) as u32;
            if state.p.contains(StatusFlags::INDEX_8BIT) {
                bus.write_u8(addr, (state.x & 0xFF) as u8);
            } else {
                bus.write_u16(addr, state.x);
            }
            add_cycles(state, 3);
            3
        }

        0x84 => {
            // STY zero page
            let addr = read_u8_generic(state, bus) as u32;
            if state.p.contains(StatusFlags::INDEX_8BIT) {
                bus.write_u8(addr, (state.y & 0xFF) as u8);
            } else {
                bus.write_u16(addr, state.y);
            }
            add_cycles(state, 3);
            3
        }

        0xC9 => {
            // CMP immediate
            if state.p.contains(StatusFlags::MEMORY_8BIT) {
                let value = read_u8_generic(state, bus);
                let result = (state.a as u8).wrapping_sub(value);
                state
                    .p
                    .set(StatusFlags::CARRY, (state.a & 0xFF) >= value as u16);
                state.p.set(StatusFlags::ZERO, result == 0);
                state.p.set(StatusFlags::NEGATIVE, (result & 0x80) != 0);
            } else {
                let value_lo = read_u8_generic(state, bus) as u16;
                let value_hi = read_u8_generic(state, bus) as u16;
                let value = value_lo | (value_hi << 8);
                let result = state.a.wrapping_sub(value);
                state.p.set(StatusFlags::CARRY, state.a >= value);
                state.p.set(StatusFlags::ZERO, result == 0);
                state.p.set(StatusFlags::NEGATIVE, (result & 0x8000) != 0);
            }
            add_cycles(state, 2);
            2
        }

        0xE0 => {
            // CPX immediate
            if state.p.contains(StatusFlags::INDEX_8BIT) {
                let value = read_u8_generic(state, bus);
                let result = (state.x as u8).wrapping_sub(value);
                state
                    .p
                    .set(StatusFlags::CARRY, (state.x & 0xFF) >= value as u16);
                state.p.set(StatusFlags::ZERO, result == 0);
                state.p.set(StatusFlags::NEGATIVE, (result & 0x80) != 0);
            } else {
                let value_lo = read_u8_generic(state, bus) as u16;
                let value_hi = read_u8_generic(state, bus) as u16;
                let value = value_lo | (value_hi << 8);
                let result = state.x.wrapping_sub(value);
                state.p.set(StatusFlags::CARRY, state.x >= value);
                state.p.set(StatusFlags::ZERO, result == 0);
                state.p.set(StatusFlags::NEGATIVE, (result & 0x8000) != 0);
            }
            add_cycles(state, 2);
            2
        }

        0xE6 => {
            // INC direct page
            let (addr, penalty) = read_direct_address_generic(state, bus);
            if penalty != 0 {
                add_cycles(state, penalty);
            }
            if memory_is_8bit(state) {
                let value = bus.read_u8(addr).wrapping_add(1);
                bus.write_u8(addr, value);
                set_flags_nz_8(state, value);
            } else {
                let value = bus.read_u16(addr).wrapping_add(1);
                bus.write_u16(addr, value);
                set_flags_nz_16(state, value);
            }
            let base_cycles: u8 = 5;
            let total_cycles = base_cycles.saturating_add(penalty);
            let already_accounted: u8 = 1 + penalty;
            add_cycles(state, total_cycles.saturating_sub(already_accounted));
            total_cycles
        }

        0xE8 => {
            // INX
            if index_is_8bit(state) {
                let value = ((state.x & 0xFF).wrapping_add(1)) as u8;
                state.x = (state.x & 0xFF00) | (value as u16);
                set_flags_nz_8(state, value);
            } else {
                state.x = state.x.wrapping_add(1);
                set_flags_nz_16(state, state.x);
            }
            add_cycles(state, 2);
            2
        }

        0xEB => {
            // XBA - Exchange B and A
            let low = (state.a & 0xFF) as u8;
            let high = (state.a >> 8) as u8;
            state.a = ((low as u16) << 8) | (high as u16);
            let new_low = (state.a & 0xFF) as u8;
            state.p.set(StatusFlags::ZERO, new_low == 0);
            state.p.set(StatusFlags::NEGATIVE, (new_low & 0x80) != 0);
            add_cycles(state, 3);
            3
        }

        0xEC => {
            // CPX absolute
            let addr = read_absolute_address_generic(state, bus);
            if index_is_8bit(state) {
                let value = bus.read_u8(addr);
                let result = (state.x as u8).wrapping_sub(value);
                state
                    .p
                    .set(StatusFlags::CARRY, (state.x & 0xFF) >= value as u16);
                state.p.set(StatusFlags::ZERO, result == 0);
                state.p.set(StatusFlags::NEGATIVE, (result & 0x80) != 0);
            } else {
                let value = bus.read_u16(addr);
                let result = state.x.wrapping_sub(value);
                state.p.set(StatusFlags::CARRY, state.x >= value);
                state.p.set(StatusFlags::ZERO, result == 0);
                state.p.set(StatusFlags::NEGATIVE, (result & 0x8000) != 0);
            }
            add_cycles(state, 4);
            4
        }

        0xEE => {
            // INC absolute
            let addr = read_absolute_address_generic(state, bus);
            if memory_is_8bit(state) {
                let value = bus.read_u8(addr).wrapping_add(1);
                bus.write_u8(addr, value);
                set_flags_nz_8(state, value);
            } else {
                let value = bus.read_u16(addr).wrapping_add(1);
                bus.write_u16(addr, value);
                set_flags_nz_16(state, value);
            }
            add_cycles(state, 6);
            6
        }

        0xF2 => {
            // SBC (dp)
            let addr = read_indirect_address_generic(state, bus);
            let memory_8bit = memory_is_8bit(state);
            let operand = if memory_8bit {
                bus.read_u8(addr) as u16
            } else {
                bus.read_u16(addr)
            };
            sbc_generic(state, operand);
            let base_cycles: u8 = if memory_8bit { 5 } else { 6 };
            let already_accounted: u8 = 2;
            add_cycles(state, base_cycles.saturating_sub(already_accounted));
            base_cycles
        }

        0xF6 => {
            // INC direct page,X
            let (addr, penalty) = read_direct_x_address_generic(state, bus);
            if penalty != 0 {
                add_cycles(state, penalty);
            }
            if memory_is_8bit(state) {
                let value = bus.read_u8(addr).wrapping_add(1);
                bus.write_u8(addr, value);
                set_flags_nz_8(state, value);
            } else {
                let value = bus.read_u16(addr).wrapping_add(1);
                bus.write_u16(addr, value);
                set_flags_nz_16(state, value);
            }
            let base_cycles: u8 = 6;
            let total_cycles = base_cycles.saturating_add(penalty);
            let already_accounted: u8 = 1 + penalty;
            add_cycles(state, total_cycles.saturating_sub(already_accounted));
            total_cycles
        }

        0xFC => {
            // JSR (addr,X)
            let base = read_u16_generic(state, bus);
            let addr = base.wrapping_add(state.x);
            let target = bus.read_u16(addr as u32);
            let return_addr = state.pc.wrapping_sub(1);
            push_u16_generic(state, bus, return_addr);
            state.pc = target;
            let base_cycles: u8 = 8;
            let accounted: u8 = 2 + 2; // operand read + push
            add_cycles(state, base_cycles.saturating_sub(accounted));
            base_cycles
        }

        0xC0 => {
            // CPY immediate
            if state.p.contains(StatusFlags::INDEX_8BIT) {
                let value = read_u8_generic(state, bus);
                let result = (state.y as u8).wrapping_sub(value);
                state
                    .p
                    .set(StatusFlags::CARRY, (state.y & 0xFF) >= value as u16);
                state.p.set(StatusFlags::ZERO, result == 0);
                state.p.set(StatusFlags::NEGATIVE, (result & 0x80) != 0);
            } else {
                let value_lo = read_u8_generic(state, bus) as u16;
                let value_hi = read_u8_generic(state, bus) as u16;
                let value = value_lo | (value_hi << 8);
                let result = state.y.wrapping_sub(value);
                state.p.set(StatusFlags::CARRY, state.y >= value);
                state.p.set(StatusFlags::ZERO, result == 0);
                state.p.set(StatusFlags::NEGATIVE, (result & 0x8000) != 0);
            }
            add_cycles(state, 2);
            2
        }

        0xB8 => {
            // CLV (Clear Overflow)
            state.p.remove(StatusFlags::OVERFLOW);
            add_cycles(state, 2);
            2
        }
    }
}

pub fn service_nmi<T: CpuBus>(state: &mut CoreState, bus: &mut T) -> u8 {
    let before = state.cycles;
    if state.emulation_mode {
        // Emulation mode pushes PC high, PC low, then status (bit5 forced 1, B cleared)
        push_u8_generic(state, bus, (state.pc >> 8) as u8);
        push_u8_generic(state, bus, (state.pc & 0xFF) as u8);
        push_u8_generic(state, bus, (state.p.bits() | 0x20) & !0x10);
        let vector = bus.read_u16(0x00FFFA);
        state.pc = vector;
        state.pb = 0;
    } else {
        // Native mode pushes PB, PCH, PCL, then status with B=0
        push_u8_generic(state, bus, state.pb);
        push_u8_generic(state, bus, (state.pc >> 8) as u8);
        push_u8_generic(state, bus, (state.pc & 0xFF) as u8);
        push_u8_generic(state, bus, (state.p.bits() | 0x20) & !0x10);
        let vector = bus.read_u16(0x00FFEA);
        state.pc = vector;
        state.pb = 0;
    }

    state.p.insert(StatusFlags::IRQ_DISABLE);
    state.waiting_for_irq = false;
    bus.acknowledge_nmi();

    let consumed = state.cycles.wrapping_sub(before) as u8;
    let target = 7u8;
    if consumed < target {
        add_cycles(state, target - consumed);
        target
    } else {
        consumed
    }
}

pub fn service_irq<T: CpuBus>(state: &mut CoreState, bus: &mut T) -> u8 {
    let before = state.cycles;
    if state.emulation_mode {
        push_u8_generic(state, bus, (state.pc >> 8) as u8);
        push_u8_generic(state, bus, (state.pc & 0xFF) as u8);
        push_u8_generic(state, bus, state.p.bits());
        let vector = bus.read_u16(0x00FFFE);
        state.pc = vector;
        state.pb = 0;
    } else {
        push_u8_generic(state, bus, state.pb);
        push_u8_generic(state, bus, (state.pc >> 8) as u8);
        push_u8_generic(state, bus, (state.pc & 0xFF) as u8);
        push_u8_generic(state, bus, (state.p.bits() | 0x20) & !0x10);
        let vector = bus.read_u16(0x00FFEE);
        state.pc = vector;
        state.pb = 0;
    }

    state.p.insert(StatusFlags::IRQ_DISABLE);
    state.waiting_for_irq = false;

    if std::env::var_os("TRACE_IRQ").is_some() {
        use std::sync::atomic::{AtomicU32, Ordering};
        static PRINT_COUNT: AtomicU32 = AtomicU32::new(0);
        if PRINT_COUNT.fetch_add(1, Ordering::Relaxed) < 16 {
            println!(
                "IRQ serviced → next PC {:02X}:{:04X} (emulation={})",
                state.pb, state.pc, state.emulation_mode
            );
        }
    }

    let consumed = state.cycles.wrapping_sub(before) as u8;
    let target = 7u8;
    if consumed < target {
        add_cycles(state, target - consumed);
        target
    } else {
        consumed
    }
}
