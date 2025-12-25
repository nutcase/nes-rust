use super::smp::Smp;
use super::dsp::dsp::Dsp;
use super::timer::Timer;
use super::spc::spc::{Spc, RAM_LEN, IPL_ROM_LEN};

static DEFAULT_IPL_ROM: [u8; IPL_ROM_LEN] = [
    0xcd, 0xef, 0xbd, 0xe8, 0x00, 0xc6, 0x1d, 0xd0,
    0xfc, 0x8f, 0xaa, 0xf4, 0x8f, 0xbb, 0xf5, 0x78,
    0xcc, 0xf4, 0xd0, 0xfb, 0x2f, 0x19, 0xeb, 0xf4,
    0xd0, 0xfc, 0x7e, 0xf4, 0xd0, 0x0b, 0xe4, 0xf5,
    0xcb, 0xf4, 0xd7, 0x00, 0xfc, 0xd0, 0xf3, 0xab,
    0x01, 0x10, 0xef, 0x7e, 0xf4, 0x10, 0xeb, 0xba,
    0xf6, 0xda, 0x00, 0xba, 0xf4, 0xc4, 0xf4, 0xdd,
    0x5d, 0xd0, 0xdb, 0x1f, 0x00, 0x00, 0xc0, 0xff];

pub struct Apu {
    ram: Box<[u8; RAM_LEN]>,
    ipl_rom: Box<[u8; IPL_ROM_LEN]>,

    pub smp: Option<Box<Smp>>,
    pub dsp: Option<Box<Dsp>>,

    timers: [Timer; 3],

    is_ipl_rom_enabled: bool,
    dsp_reg_address: u8,

    // CPU<->APU I/O ports ($2140-$2143 <-> $F4-$F7)
    //
    // 実機では S-CPU 側と S-SMP 側で「読み出し対象」が異なる（2組のラッチ）。
    // - S-SMP が $F4-$F7 を読むと、S-CPU が APUIO に最後に書いた値を読む（CPU->APU）。
    // - S-SMP が $F4-$F7 に書くと、S-CPU が APUIO を読んだときに返る値が更新される（APU->CPU）。
    //
    // 参考: S-SMP CONTROL($F1) の Clear data ports ビットは「Data-from-CPU read registers」をクリアする。
    cpu_to_apu_ports: [u8; 4],
    apu_to_cpu_ports: [u8; 4],
}

impl Apu {
    pub fn new() -> Box<Apu> {
        let mut ret = Box::new(Apu {
            ram: Box::new([0; RAM_LEN]),
            ipl_rom: Box::new([0; IPL_ROM_LEN]),

            smp: None,
            dsp: None,

            timers: [Timer::new(256), Timer::new(256), Timer::new(32)],

            is_ipl_rom_enabled: true,
            dsp_reg_address: 0,

            cpu_to_apu_ports: [0; 4],
            apu_to_cpu_ports: [0; 4],
        });
        let ret_ptr = &mut *ret as *mut _;
        ret.smp = Some(Box::new(Smp::new(ret_ptr)));
        ret.dsp = Some(Dsp::new(ret_ptr));
        ret.reset();
        ret
    }

    pub fn reset(&mut self) {
        for i in 0..RAM_LEN {
            self.ram[i] = 0;
        }

        for i in 0..IPL_ROM_LEN {
            self.ipl_rom[i] = DEFAULT_IPL_ROM[i];
        }

        self.smp.as_mut().unwrap().reset();
        self.dsp.as_mut().unwrap().reset();
        for timer in self.timers.iter_mut() {
            timer.reset();
        }

        self.is_ipl_rom_enabled = true;
        self.dsp_reg_address = 0;
        self.cpu_to_apu_ports = [0; 4];
        self.apu_to_cpu_ports = [0; 4];
        // Power-on/reset defaults (S-SMP):
        // - TEST($F0) = $0A
        // - CONTROL($F1) behaves as if IPL is enabled and ports are cleared.
        // Note: $F0/$F1 are write-only and read back as $00; we still store the
        // written values internally for side effects.
        self.ram[0x00f0] = 0x0A;
        self.set_control_reg(0xB0);
        self.ram[0x00f1] = 0xB0;
    }

    /// S-CPU からの APUIO 書き込み（$2140-$2143）。S-SMP からは $F4-$F7 の読み出しで観測される。
    pub fn cpu_write_port(&mut self, port: u8, value: u8) {
        let p = (port & 0x03) as usize;
        self.cpu_to_apu_ports[p] = value;
    }

    /// S-CPU からの APUIO 読み出し（$2140-$2143）。S-SMP が $F4-$F7 に書いた値が返る。
    pub fn cpu_read_port(&self, port: u8) -> u8 {
        let p = (port & 0x03) as usize;
        self.apu_to_cpu_ports[p]
    }

    pub fn render(&mut self, left_buffer: &mut [i16], right_buffer: &mut [i16], num_samples: i32) {
        let smp = self.smp.as_mut().unwrap();
        let dsp = self.dsp.as_mut().unwrap();
        while dsp.output_buffer.get_sample_count() < num_samples {
            smp.run(num_samples * 64);
            dsp.flush();
        }

        dsp.output_buffer.read(left_buffer, right_buffer, num_samples);
    }

    pub fn cpu_cycles_callback(&mut self, num_cycles: i32) {
        self.dsp.as_mut().unwrap().cycles_callback(num_cycles);
        // TEST($F0) can disable timers (Enable timers bit must be set, and Halt timers must be clear).
        let test = self.ram[0x00f0];
        let timers_enabled = (test & 0x08) != 0 && (test & 0x01) == 0;
        if timers_enabled {
            for timer in self.timers.iter_mut() {
                timer.cpu_cycles_callback(num_cycles);
            }
        }
    }

    pub fn debug_timer_state(&self, idx: usize) -> Option<(i32, bool, u8, u8, u8, i32)> {
        self.timers.get(idx).map(|t| t.debug_state())
    }

    pub fn read_u8(&mut self, address: u32) -> u8 {
        let address = address & 0xffff;
        if address >= 0xf0 && address < 0x0100 {
            match address {
                // Write-only registers read back as $00.
                0xf0 | 0xf1 => 0x00,

                0xf2 => self.dsp_reg_address,
                0xf3 => self.dsp.as_mut().unwrap().get_register(self.dsp_reg_address),

                // Write-only timer targets read back as $00.
                0xfa ..= 0xfc => 0x00,

                0xfd => {
                    let v = self.timers[0].read_counter();
                    if std::env::var_os("TRACE_BURNIN_SMP_TIMER").is_some() {
                        use std::sync::atomic::{AtomicU32, Ordering};
                        static CNT: AtomicU32 = AtomicU32::new(0);
                        let n = CNT.fetch_add(1, Ordering::Relaxed);
                        if n < 64 {
                            let pc = self.smp.as_ref().map(|s| s.reg_pc).unwrap_or(0);
                            let ctrl = self.ram[0x00f1];
                            let target = self.ram[0x00fa];
                            let (ticks, running, t_target, low, high, res) = self.timers[0].debug_state();
                            println!(
                                "[SMP-T0] pc={:04X} $FD -> {:02X} (ctrl={:02X} target={:02X} timer:run={} res={} ticks={} low={:02X} high={:02X} tgt={:02X})",
                                pc, v, ctrl, target, running, res, ticks, low, high, t_target
                            );
                        }
                    }
                    v
                }
                0xfe => self.timers[1].read_counter(),
                0xff => self.timers[2].read_counter(),

                // CPU->APU ports: reads return values written by the S-CPU.
                0xf4 ..= 0xf7 => {
                    let idx = (address - 0xf4) as usize;
                    let v = self.cpu_to_apu_ports[idx];
                    if std::env::var_os("TRACE_SFS_APU_F4_READ").is_some() {
                        use std::sync::atomic::{AtomicU32, Ordering};
                        static CNT: AtomicU32 = AtomicU32::new(0);
                        let (pc, a, x, y, psw) = self
                            .smp
                            .as_ref()
                            .map(|s| (s.reg_pc, s.reg_a, s.reg_x, s.reg_y, s.get_psw()))
                            .unwrap_or((0, 0, 0, 0, 0));
                        // Skip IPL-only noise so we can see post-upload port reads.
                        if pc < 0xFFC0 {
                            let n = CNT.fetch_add(1, Ordering::Relaxed);
                            if n < 512 {
                                println!(
                                    "[SFS][SMP][F{:X}R] pc={:04X} A={:02X} X={:02X} Y={:02X} psw={:02X} -> {:02X} in=[{:02X} {:02X} {:02X} {:02X}]",
                                    4 + idx,
                                    pc,
                                    a,
                                    x,
                                    y,
                                    psw,
                                    v,
                                    self.cpu_to_apu_ports[0],
                                    self.cpu_to_apu_ports[1],
                                    self.cpu_to_apu_ports[2],
                                    self.cpu_to_apu_ports[3]
                                );
                            }
                        }
                    }
                    if std::env::var_os("TRACE_BURNIN_APU_F4F7_READS").is_some() {
                        use std::sync::atomic::{AtomicU32, Ordering};
                        static CNT: AtomicU32 = AtomicU32::new(0);
                        static LAST: std::sync::OnceLock<std::sync::Mutex<[u8; 4]>> =
                            std::sync::OnceLock::new();
                        let mut last = LAST.get_or_init(|| std::sync::Mutex::new([0; 4]))
                            .lock()
                            .unwrap();
                        if last[idx] != v {
                            let n = CNT.fetch_add(1, Ordering::Relaxed);
                            if n < 512 {
                                let pc = self.smp.as_ref().map(|s| s.reg_pc).unwrap_or(0);
                                println!("[SMP][F{:X}R] pc={:04X} {:02X}->{:02X}", 4 + idx, pc, last[idx], v);
                            }
                            last[idx] = v;
                        }
                    }
                    v
                },

                _ => self.ram[address as usize]
            }
        } else if address >= 0xffc0 && self.is_ipl_rom_enabled {
            self.ipl_rom[(address - 0xffc0) as usize]
        } else {
            self.ram[address as usize]
        }
    }

    /// Debug-only peek: read RAM/IPL without side effects (no timers/ports).
    pub(crate) fn peek_u8(&self, address: u16) -> u8 {
        let address = address as usize & 0xFFFF;
        if address >= 0xFFC0 && self.is_ipl_rom_enabled {
            self.ipl_rom[address - 0xFFC0]
        } else {
            self.ram[address]
        }
    }

    pub fn write_u8(&mut self, address: u32, value: u8) {
        let address = address & 0xffff;
        if address >= 0x00f0 && address < 0x0100 {
            match address {
                0xf0 => {
                    // TEST ($F0) writes only take effect when the P flag is clear.
                    let psw = self.smp.as_ref().map(|s| s.get_psw()).unwrap_or(0);
                    if (psw & 0x20) == 0 {
                        if std::env::var_os("TRACE_BURNIN_APU_F0_WRITES").is_some() {
                            use std::sync::atomic::{AtomicU32, Ordering};
                            static CNT: AtomicU32 = AtomicU32::new(0);
                            let n = CNT.fetch_add(1, Ordering::Relaxed);
                            if n < 256 {
                                let pc = self.smp.as_ref().map(|s| s.reg_pc).unwrap_or(0);
                                let prev = self.ram[address as usize];
                                println!(
                                    "[SMP][F0W] pc={:04X} psw={:02X} {:02X}->{:02X}",
                                    pc, psw, prev, value
                                );
                            }
                        }
                        self.set_test_reg(value);
                        self.ram[address as usize] = value;
                    }
                },
                0xf1 => {
                    if std::env::var_os("TRACE_BURNIN_APU_F1_WRITES").is_some() {
                        use std::sync::atomic::{AtomicU32, Ordering};
                        static CNT: AtomicU32 = AtomicU32::new(0);
                        let n = CNT.fetch_add(1, Ordering::Relaxed);
                        if n < 1024 {
                            let pc = self.smp.as_ref().map(|s| s.reg_pc).unwrap_or(0);
                            let prev = self.ram[address as usize];
                            println!("[SMP][F1W] pc={:04X} {:02X}->{:02X}", pc, prev, value);
                        }
                    }
                    self.set_control_reg(value);
                    self.ram[address as usize] = value;
                    if std::env::var_os("TRACE_BURNIN_SMP_TIMER").is_some() {
                        use std::sync::atomic::{AtomicU32, Ordering};
                        static CNT: AtomicU32 = AtomicU32::new(0);
                        let n = CNT.fetch_add(1, Ordering::Relaxed);
                        if n < 64 {
                            let pc = self.smp.as_ref().map(|s| s.reg_pc).unwrap_or(0);
                            println!("[SMP-CTRL] pc={:04X} $F1 <- {:02X}", pc, value);
                        }
                    }
                },
                0xf2 => { self.dsp_reg_address = value; self.ram[address as usize] = value; },
                0xf3 => { self.dsp.as_mut().unwrap().set_register(self.dsp_reg_address, value); },

                // APU->CPU ports: writes update values readable by the S-CPU.
                0xf4 ..= 0xf7 => {
                    let idx = (address - 0xf4) as usize;
                    let prev = self.apu_to_cpu_ports[idx];
                    self.apu_to_cpu_ports[idx] = value;
                    if std::env::var_os("TRACE_BURNIN_APU_F4_WRITES").is_some()
                        && address == 0x00f4
                        && prev != value
                    {
                        use std::sync::atomic::{AtomicU32, Ordering};
                        static CNT: AtomicU32 = AtomicU32::new(0);
                        let n = CNT.fetch_add(1, Ordering::Relaxed);
                        if n < 512 {
                            let pc = self.smp.as_ref().map(|s| s.reg_pc).unwrap_or(0);
                            let psw = self.smp.as_ref().map(|s| s.get_psw()).unwrap_or(0);
                            println!(
                                "[SMP][F4W] pc={:04X} psw={:02X} {:02X}->{:02X}",
                                pc, psw, prev, value
                            );
                        }
                    }
                    if std::env::var_os("TRACE_BURNIN_APU_F5_WRITES").is_some()
                        && address == 0x00f5
                        && prev != value
                    {
                        use std::sync::atomic::{AtomicU32, Ordering};
                        static CNT: AtomicU32 = AtomicU32::new(0);
                        let n = CNT.fetch_add(1, Ordering::Relaxed);
                        if n < 256 {
                            let pc = self.smp.as_ref().map(|s| s.reg_pc).unwrap_or(0);
                            let psw = self.smp.as_ref().map(|s| s.get_psw()).unwrap_or(0);
                            let ctx_start = pc.wrapping_sub(0x10);
                            let mut code = [0u8; 32];
                            for (i, b) in code.iter_mut().enumerate() {
                                *b = self.read_u8(ctx_start.wrapping_add(i as u16) as u32);
                            }
                            println!(
                                "[SMP][F5W] pc={:04X} psw={:02X} {:02X}->{:02X} in=[{:02X} {:02X} {:02X} {:02X}] code@{:04X}={:02X?}",
                                pc,
                                psw,
                                prev,
                                value,
                                self.cpu_to_apu_ports[0],
                                self.cpu_to_apu_ports[1],
                                self.cpu_to_apu_ports[2],
                                self.cpu_to_apu_ports[3],
                                ctx_start,
                                code
                            );
                            // Optional: dump more of the APU routine around $09E7 once.
                            if std::env::var_os("TRACE_BURNIN_APU_DUMP_09E7").is_some()
                                && pc == 0x09E6
                            {
                                static ONCE: std::sync::OnceLock<()> = std::sync::OnceLock::new();
                                if ONCE.set(()).is_ok() {
                                    let start = 0x09E7u16;
                                    let mut blob = [0u8; 128];
                                    for (i, b) in blob.iter_mut().enumerate() {
                                        *b = self.read_u8(start.wrapping_add(i as u16) as u32);
                                    }
                                    println!("[SMP][DUMP09E7] @09E7={:02X?}", blob);
                                }
                            }
                        }
                    }
                },

                // $F8-$F9 are general-purpose RAM locations (not CPU ports).
                0xf8 ..= 0xf9 => { self.ram[address as usize] = value; },

                0xfa => {
                    self.timers[0].set_target(value);
                    self.ram[address as usize] = value;
                    if std::env::var_os("TRACE_BURNIN_SMP_TIMER").is_some() {
                        use std::sync::atomic::{AtomicU32, Ordering};
                        static CNT: AtomicU32 = AtomicU32::new(0);
                        let n = CNT.fetch_add(1, Ordering::Relaxed);
                        if n < 64 {
                            let pc = self.smp.as_ref().map(|s| s.reg_pc).unwrap_or(0);
                            println!("[SMP-T0TGT] pc={:04X} $FA <- {:02X}", pc, value);
                        }
                    }
                },
                0xfb => { self.timers[1].set_target(value); self.ram[address as usize] = value; },
                0xfc => { self.timers[2].set_target(value); self.ram[address as usize] = value; },

                _ => () // Do nothing
            }
        } else {
            if std::env::var_os("TRACE_SFS_APU_VAR81").is_some()
                && (address == 0x0081 || address == 0x0181)
            {
                use std::sync::atomic::{AtomicU32, Ordering};
                static CNT: AtomicU32 = AtomicU32::new(0);
                let n = CNT.fetch_add(1, Ordering::Relaxed);
                if n < 256 {
                    let pc = self.smp.as_ref().map(|s| s.reg_pc).unwrap_or(0);
                    let psw = self.smp.as_ref().map(|s| s.get_psw()).unwrap_or(0);
                    let prev = self.ram[address as usize];
                    println!(
                        "[SFS][SMP][VAR81] pc={:04X} psw={:02X} ${:04X} {:02X}->{:02X}",
                        pc, psw, address, prev, value
                    );
                }
            }
            if std::env::var_os("TRACE_SFS_APU_VAR14").is_some()
                && (address == 0x0014 || address == 0x0114)
            {
                use std::sync::atomic::{AtomicU32, Ordering};
                static CNT: AtomicU32 = AtomicU32::new(0);
                let n = CNT.fetch_add(1, Ordering::Relaxed);
                if n < 256 {
                    let pc = self.smp.as_ref().map(|s| s.reg_pc).unwrap_or(0);
                    let psw = self.smp.as_ref().map(|s| s.get_psw()).unwrap_or(0);
                    let prev = self.ram[address as usize];
                    println!(
                        "[SFS][SMP][VAR14] pc={:04X} psw={:02X} ${:04X} {:02X}->{:02X}",
                        pc, psw, address, prev, value
                    );
                }
            }
            if std::env::var_os("TRACE_BURNIN_APU_VAR2A").is_some()
                && (address == 0x002A || address == 0x012A)
            {
                use std::sync::atomic::{AtomicU32, Ordering};
                static CNT: AtomicU32 = AtomicU32::new(0);
                let n = CNT.fetch_add(1, Ordering::Relaxed);
                if n < 256 {
                    let pc = self.smp.as_ref().map(|s| s.reg_pc).unwrap_or(0);
                    let psw = self.smp.as_ref().map(|s| s.get_psw()).unwrap_or(0);
                    let prev = self.ram[address as usize];
                    println!(
                        "[SMP][VAR2A] pc={:04X} psw={:02X} ${:04X} {:02X}->{:02X}",
                        pc, psw, address, prev, value
                    );
                }
            }
            self.ram[address as usize] = value;
        }
    }

    pub fn set_state(&mut self, spc: &Spc) {
        self.reset();

        for i in 0..RAM_LEN {
            self.ram[i] = spc.ram[i];
        }
        for i in 0..IPL_ROM_LEN {
            self.ipl_rom[i] = spc.ipl_rom[i];
        }

        {
            let smp = self.smp.as_mut().unwrap();
            smp.reg_pc = spc.pc;
            smp.reg_a = spc.a;
            smp.reg_x = spc.x;
            smp.reg_y = spc.y;
            smp.set_psw(spc.psw);
            smp.reg_sp = spc.sp;
        }

        self.dsp.as_mut().unwrap().set_state(spc);

        for i in 0..3 {
            self.timers[i].set_target(self.ram[0xfa + i]);
        }
        let control_reg = self.ram[0xf1];
        self.set_control_reg(control_reg);

        self.dsp_reg_address = self.ram[0xf2];
    }

    pub fn clear_echo_buffer(&mut self) {
        let dsp = self.dsp.as_mut().unwrap();
        let length = dsp.calculate_echo_length();
        let mut end_addr = dsp.get_echo_start_address() as i32 + length;
        if end_addr > RAM_LEN as i32 {
            end_addr = RAM_LEN as i32;
        }
        for i in dsp.get_echo_start_address() as i32..end_addr {
            self.ram[i as usize] = 0xff;
        }
    }

    fn set_test_reg(&self, value: u8) {
        let _ = value;
        // TEST ($F0) is rarely used by commercial software. It controls a number of
        // low-level functions (timer speed selection, etc). For now, we treat it as
        // a no-op to avoid panics on test ROMs.
    }

    fn set_control_reg(&mut self, value: u8) {
        self.is_ipl_rom_enabled = (value & 0x80) != 0;
        if (value & 0x20) != 0 {
            // Clear data-from-CPU read registers (ports 2/3).
            self.cpu_to_apu_ports[2] = 0x00;
            self.cpu_to_apu_ports[3] = 0x00;
        }
        if (value & 0x10) != 0 {
            // Clear data-from-CPU read registers (ports 0/1).
            self.cpu_to_apu_ports[0] = 0x00;
            self.cpu_to_apu_ports[1] = 0x00;
        }
        self.timers[0].set_start_stop_bit((value & 0x01) != 0);
        self.timers[1].set_start_stop_bit((value & 0x02) != 0);
        self.timers[2].set_start_stop_bit((value & 0x04) != 0);
    }
}
