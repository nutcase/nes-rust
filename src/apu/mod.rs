use std::sync::Arc;

#[derive(Debug, Clone, Copy, Default)]
pub struct AudioDiagFull {
    pub pulse1_enabled: bool,
    pub pulse1_length: u8,
    pub pulse2_enabled: bool,
    pub pulse2_length: u8,
    pub triangle_enabled: bool,
    pub triangle_length: u8,
    pub noise_enabled: bool,
    pub noise_length: u8,
    pub noise_vol: u8,
    pub noise_period: u16,
    pub noise_envelope_disable: bool,
    pub expansion: f32,
}

pub struct Apu {
    pulse1: PulseChannel,
    pulse2: PulseChannel,
    triangle: TriangleChannel,
    noise: NoiseChannel,

    frame_counter: u16,
    cycle_count: u64,

    // Frame counter control
    frame_mode: bool,  // false = 4-step, true = 5-step
    irq_disable: bool, // IRQ inhibit flag
    frame_irq: bool,   // Frame IRQ flag

    // Status register
    pulse1_enabled: bool,
    pulse2_enabled: bool,
    triangle_enabled: bool,
    noise_enabled: bool,
    dmc_enabled: bool,

    // Audio output — samples are pushed directly to ring buffer when available,
    // or fall back to the Vec buffer.
    audio_ring: Option<Arc<crate::audio_ring::SpscRingBuffer>>,
    output_buffer: Vec<f32>,
    sample_rate: f32,
    cpu_clock_rate: f32,

    // Fractional sample accumulator
    sample_counter: f32,

    // Oversampling anti-aliasing: accumulate raw mixer output every CPU cycle,
    // then average when producing an output sample (~40x oversampling).
    sample_accumulator: f32,
    sample_accumulator_count: u32,

    // Anti-aliasing pre-filters running at CPU rate (~1.79 MHz).
    // Two cascaded first-order IIR low-pass at 18 kHz.
    aa_filter1: LowPassFilter,
    aa_filter2: LowPassFilter,

    // NES hardware audio filters (nesdev wiki)
    high_pass_90hz: HighPassFilter,  // AC coupling capacitor (~90 Hz)
    high_pass_440hz: HighPassFilter, // Amplifier feedback (~440 Hz)
    low_pass_14khz: LowPassFilter,   // Amplifier bandwidth (~14 kHz)

    // Expansion audio (e.g. Sunsoft 5B) — set by bus each CPU cycle
    expansion_audio: f32,
}

struct PulseChannel {
    duty: u8,
    length_counter: u8,
    envelope_divider: u8,
    envelope_decay: u8,
    envelope_disable: bool,
    envelope_start: bool,
    volume: u8,
    sweep_enabled: bool,
    sweep_period: u8,
    sweep_negate: bool,
    sweep_shift: u8,
    sweep_reload: bool,
    sweep_divider: u8,
    timer: u16,
    timer_reload: u16,
    duty_counter: u8,
    length_enabled: bool,
    is_pulse1: bool, // true=Pulse1 (one's complement negate), false=Pulse2 (two's complement)
}

struct TriangleChannel {
    linear_counter: u8,
    linear_reload: u8,
    linear_control: bool,
    linear_reload_flag: bool,
    length_counter: u8,
    timer: u16,
    timer_reload: u16,
    sequence_counter: u8,
    length_enabled: bool,
}

struct NoiseChannel {
    length_counter: u8,
    envelope_divider: u8,
    envelope_decay: u8,
    envelope_disable: bool,
    envelope_start: bool,
    volume: u8,
    mode: bool,
    timer: u16,
    timer_reload: u16,
    shift_register: u16,
    length_enabled: bool,
}

// High-quality audio filters
struct HighPassFilter {
    prev_input: f32,
    prev_output: f32,
    alpha: f32,
}

struct LowPassFilter {
    prev_output: f32,
    alpha: f32,
}

impl HighPassFilter {
    fn new(sample_rate: f32, cutoff: f32) -> Self {
        let rc = 1.0 / (2.0 * std::f32::consts::PI * cutoff);
        let dt = 1.0 / sample_rate;
        let alpha = rc / (rc + dt);

        HighPassFilter {
            prev_input: 0.0,
            prev_output: 0.0,
            alpha,
        }
    }

    fn process(&mut self, input: f32) -> f32 {
        let output = self.alpha * (self.prev_output + input - self.prev_input);
        self.prev_input = input;
        self.prev_output = output;
        output
    }
}

impl LowPassFilter {
    fn new(sample_rate: f32, cutoff: f32) -> Self {
        let rc = 1.0 / (2.0 * std::f32::consts::PI * cutoff);
        let dt = 1.0 / sample_rate;
        let alpha = dt / (rc + dt);

        LowPassFilter {
            prev_output: 0.0,
            alpha,
        }
    }

    fn process(&mut self, input: f32) -> f32 {
        let output = self.prev_output + self.alpha * (input - self.prev_output);
        self.prev_output = output;
        output
    }
}

impl Apu {
    pub fn new() -> Self {
        Apu {
            pulse1: PulseChannel::new(true),
            pulse2: PulseChannel::new(false),
            triangle: TriangleChannel::new(),
            noise: NoiseChannel::new(),

            frame_counter: 0,
            cycle_count: 0,

            frame_mode: false,
            irq_disable: true,
            frame_irq: false,

            pulse1_enabled: false,
            pulse2_enabled: false,
            triangle_enabled: false,
            noise_enabled: false,
            dmc_enabled: false,

            audio_ring: None,
            output_buffer: Vec::new(),
            sample_rate: 44100.0,
            cpu_clock_rate: 1789773.0,

            sample_counter: 0.0,

            sample_accumulator: 0.0,
            sample_accumulator_count: 0,

            aa_filter1: LowPassFilter::new(1789773.0, 18000.0),
            aa_filter2: LowPassFilter::new(1789773.0, 18000.0),

            high_pass_90hz: HighPassFilter::new(44100.0, 90.0),
            high_pass_440hz: HighPassFilter::new(44100.0, 440.0),
            low_pass_14khz: LowPassFilter::new(44100.0, 14000.0),

            expansion_audio: 0.0,
        }
    }

    pub fn set_expansion_audio(&mut self, value: f32) {
        self.expansion_audio = value;
    }

    pub fn audio_diag_full(&self) -> AudioDiagFull {
        AudioDiagFull {
            pulse1_enabled: self.pulse1_enabled,
            pulse1_length: self.pulse1.length_counter,
            pulse2_enabled: self.pulse2_enabled,
            pulse2_length: self.pulse2.length_counter,
            triangle_enabled: self.triangle_enabled,
            triangle_length: self.triangle.length_counter,
            noise_enabled: self.noise_enabled,
            noise_length: self.noise.length_counter,
            noise_vol: if self.noise.envelope_disable {
                self.noise.volume
            } else {
                self.noise.envelope_decay
            },
            noise_period: self.noise.timer_reload,
            noise_envelope_disable: self.noise.envelope_disable,
            expansion: self.expansion_audio,
        }
    }

    pub fn step(&mut self) {
        self.cycle_count += 1;
        self.frame_counter += 1;

        // Triangle: clocked every CPU cycle (timer always runs)
        self.triangle.step();

        // Pulse and Noise: clocked every 2 CPU cycles (timers always run)
        if self.cycle_count & 1 == 0 {
            self.pulse1.step();
            self.pulse2.step();
            self.noise.step();
        }

        // Frame sequencer with proper 4-step/5-step timing
        // Values are in CPU cycles (APU cycle * 2, since step() is called per CPU cycle)
        // APU 3728.5 = CPU 7457, APU 7456.5 = CPU 14913, etc.
        if !self.frame_mode {
            // 4-step mode
            match self.frame_counter {
                7457 => self.clock_quarter_frame(),
                14913 => self.clock_half_frame(),
                22371 => self.clock_quarter_frame(),
                29829 => {
                    self.clock_half_frame();
                    if !self.irq_disable {
                        self.frame_irq = true;
                    }
                    self.frame_counter = 0;
                }
                _ => {}
            }
        } else {
            // 5-step mode (no IRQ)
            match self.frame_counter {
                7457 => self.clock_quarter_frame(),
                14913 => self.clock_half_frame(),
                22371 => self.clock_quarter_frame(),
                29829 => {} // nothing
                37281 => {
                    self.clock_half_frame();
                    self.frame_counter = 0;
                }
                _ => {}
            }
        }

        // Anti-aliasing: filter raw mixer output at CPU rate, then accumulate.
        let raw = self.raw_mix() + self.expansion_audio;
        let aa = self.aa_filter1.process(raw);
        let aa = self.aa_filter2.process(aa);
        self.sample_accumulator += aa;
        self.sample_accumulator_count += 1;

        // Fractional sample accumulator for accurate 44100 Hz sampling
        self.sample_counter += self.sample_rate;
        if self.sample_counter >= self.cpu_clock_rate {
            self.sample_counter -= self.cpu_clock_rate;
            let sample = self.produce_sample();
            // Push directly to ring buffer for jitter-free delivery,
            // fall back to Vec when no ring buffer is attached.
            if let Some(ref ring) = self.audio_ring {
                ring.push_one(sample);
            } else {
                self.output_buffer.push(sample);
            }
        }
    }

    /// Quarter frame: envelopes + triangle linear counter
    fn clock_quarter_frame(&mut self) {
        self.pulse1.clock_envelope();
        self.pulse2.clock_envelope();
        self.triangle.clock_linear_counter();
        self.noise.clock_envelope();
    }

    /// Half frame: quarter frame + length counters + sweeps
    fn clock_half_frame(&mut self) {
        self.clock_quarter_frame();
        self.pulse1.clock_length_counter();
        self.pulse1.clock_sweep();
        self.pulse2.clock_length_counter();
        self.pulse2.clock_sweep();
        self.triangle.clock_length_counter();
        self.noise.clock_length_counter();
    }

    /// Non-linear mixer (nesdev wiki) without filters. Called every CPU cycle
    /// for oversampling accumulation.
    #[inline]
    fn raw_mix(&self) -> f32 {
        let pulse1_out = if self.pulse1_enabled && self.pulse1.length_counter > 0 {
            self.pulse1.output()
        } else {
            0.0
        };
        let pulse2_out = if self.pulse2_enabled && self.pulse2.length_counter > 0 {
            self.pulse2.output()
        } else {
            0.0
        };
        let triangle_out = if self.triangle_enabled && self.triangle.length_counter > 0 {
            self.triangle.output()
        } else {
            0.0
        };
        let noise_out = if self.noise_enabled && self.noise.length_counter > 0 {
            self.noise.output()
        } else {
            0.0
        };

        // Non-linear mixer (nesdev wiki) - models the NES resistor DAC
        // Channel outputs are 0.0-15.0. Mixer naturally outputs 0.0-~1.0.
        let pulse_sum = pulse1_out + pulse2_out;
        let pulse_out = if pulse_sum > 0.0 {
            95.88 / (8128.0 / pulse_sum + 100.0)
        } else {
            0.0
        };

        let tnd_sum = triangle_out / 8227.0 + noise_out / 12241.0;
        let tnd_out = if tnd_sum > 0.0 {
            159.79 / (1.0 / tnd_sum + 100.0)
        } else {
            0.0
        };

        pulse_out + tnd_out
    }

    /// Average accumulated raw mix, apply hardware filters, produce final sample.
    fn produce_sample(&mut self) -> f32 {
        let averaged = if self.sample_accumulator_count > 0 {
            self.sample_accumulator / self.sample_accumulator_count as f32
        } else {
            0.0
        };
        self.sample_accumulator = 0.0;
        self.sample_accumulator_count = 0;

        // Apply NES hardware filter chain (nesdev wiki)
        let filtered = self.high_pass_90hz.process(averaged);
        let filtered = self.high_pass_440hz.process(filtered);
        let filtered = self.low_pass_14khz.process(filtered);

        // Scale to fill audio output range (HP filters center the signal around 0)
        (filtered * 1.8).clamp(-1.0, 1.0)
    }

    /// Attach a ring buffer for direct sample delivery (bypasses output_buffer).
    pub fn set_audio_ring(&mut self, ring: Arc<crate::audio_ring::SpscRingBuffer>) {
        self.audio_ring = Some(ring);
    }

    pub fn get_audio_buffer(&mut self) -> Vec<f32> {
        self.output_buffer.drain(..).collect()
    }

    /// Push accumulated samples directly into the ring buffer, avoiding
    /// an intermediate Vec allocation.
    pub fn drain_to_ring(&mut self, ring: &crate::audio_ring::SpscRingBuffer) {
        if !self.output_buffer.is_empty() {
            ring.push_slice(&self.output_buffer);
            self.output_buffer.clear();
        }
    }

    pub fn frame_irq_pending(&self) -> bool {
        self.frame_irq && !self.irq_disable
    }

    pub fn clear_frame_irq(&mut self) {
        self.frame_irq = false;
    }

    pub fn read_register(&mut self, addr: u16) -> u8 {
        match addr {
            0x4015 => {
                let mut status = 0;
                if self.pulse1_enabled && self.pulse1.length_counter > 0 {
                    status |= 0x01;
                }
                if self.pulse2_enabled && self.pulse2.length_counter > 0 {
                    status |= 0x02;
                }
                if self.triangle_enabled && self.triangle.length_counter > 0 {
                    status |= 0x04;
                }
                if self.noise_enabled && self.noise.length_counter > 0 {
                    status |= 0x08;
                }
                if self.dmc_enabled {
                    status |= 0x10;
                }

                if self.frame_irq {
                    status |= 0x40;
                }

                // Reading $4015 clears the frame IRQ flag
                self.frame_irq = false;

                status
            }
            _ => 0,
        }
    }

    pub fn write_register(&mut self, addr: u16, data: u8) {
        match addr {
            // Pulse 1
            0x4000 => self.pulse1.write_control(data),
            0x4001 => self.pulse1.write_sweep(data),
            0x4002 => self.pulse1.write_timer_low(data),
            0x4003 => self.pulse1.write_timer_high(data, self.pulse1_enabled),

            // Pulse 2
            0x4004 => self.pulse2.write_control(data),
            0x4005 => self.pulse2.write_sweep(data),
            0x4006 => self.pulse2.write_timer_low(data),
            0x4007 => self.pulse2.write_timer_high(data, self.pulse2_enabled),

            // Triangle
            0x4008 => self.triangle.write_control(data),
            0x4009 => {}
            0x400A => self.triangle.write_timer_low(data),
            0x400B => self.triangle.write_timer_high(data, self.triangle_enabled),

            // Noise
            0x400C => self.noise.write_control(data),
            0x400D => {}
            0x400E => self.noise.write_period(data),
            0x400F => self.noise.write_length(data, self.noise_enabled),

            // DMC (not implemented)
            0x4010..=0x4013 => {}

            // Status
            0x4015 => {
                self.pulse1_enabled = data & 0x01 != 0;
                self.pulse2_enabled = data & 0x02 != 0;
                self.triangle_enabled = data & 0x04 != 0;
                self.noise_enabled = data & 0x08 != 0;
                self.dmc_enabled = data & 0x10 != 0;

                if !self.pulse1_enabled {
                    self.pulse1.length_counter = 0;
                }
                if !self.pulse2_enabled {
                    self.pulse2.length_counter = 0;
                }
                if !self.triangle_enabled {
                    self.triangle.length_counter = 0;
                }
                if !self.noise_enabled {
                    self.noise.length_counter = 0;
                }
            }

            // Frame counter
            0x4017 => {
                self.frame_mode = (data & 0x80) != 0;
                self.irq_disable = (data & 0x40) != 0;

                self.frame_irq = false;
                self.frame_counter = 0;

                // 5-step mode immediately clocks quarter + half frame
                if self.frame_mode {
                    self.clock_half_frame();
                }
            }
            _ => {}
        }
    }
}

// Length counter lookup table
const LENGTH_TABLE: [u8; 32] = [
    10, 254, 20, 2, 40, 4, 80, 6, 160, 8, 60, 10, 14, 12, 26, 14, 12, 16, 24, 18, 48, 20, 96, 22,
    192, 24, 72, 26, 16, 28, 32, 30,
];

// Noise period lookup table
const NOISE_PERIOD_TABLE: [u16; 16] = [
    4, 8, 16, 32, 64, 96, 128, 160, 202, 254, 380, 508, 762, 1016, 2034, 4068,
];

impl PulseChannel {
    fn new(is_pulse1: bool) -> Self {
        PulseChannel {
            duty: 0,
            length_counter: 0,
            envelope_divider: 0,
            envelope_decay: 15,
            envelope_disable: false,
            envelope_start: false,
            volume: 0,
            sweep_enabled: false,
            sweep_period: 0,
            sweep_negate: false,
            sweep_shift: 0,
            sweep_reload: false,
            sweep_divider: 0,
            timer: 0,
            timer_reload: 0,
            duty_counter: 0,
            length_enabled: true,
            is_pulse1,
        }
    }

    fn write_control(&mut self, data: u8) {
        self.duty = (data >> 6) & 0x03;
        self.length_enabled = (data & 0x20) == 0;
        self.envelope_disable = (data & 0x10) != 0;
        self.volume = data & 0x0F;
        // Note: writing to $4000/$4004 does NOT restart the envelope.
        // Only writing to $4003/$4007 (4th register) sets envelope_start.
    }

    fn write_sweep(&mut self, data: u8) {
        self.sweep_enabled = (data & 0x80) != 0;
        self.sweep_period = (data >> 4) & 0x07;
        self.sweep_negate = (data & 0x08) != 0;
        self.sweep_shift = data & 0x07;
        self.sweep_reload = true;
    }

    fn write_timer_low(&mut self, data: u8) {
        self.timer_reload = (self.timer_reload & 0xFF00) | data as u16;
    }

    fn write_timer_high(&mut self, data: u8, enabled: bool) {
        self.timer_reload = (self.timer_reload & 0x00FF) | ((data as u16 & 0x07) << 8);
        if enabled {
            self.length_counter = LENGTH_TABLE[((data >> 3) & 0x1F) as usize];
        }
        self.timer = self.timer_reload;
        self.duty_counter = 0;
        self.envelope_start = true;
    }

    fn step(&mut self) {
        if self.timer == 0 {
            self.timer = self.timer_reload;
            self.duty_counter = (self.duty_counter + 1) % 8;
        } else {
            self.timer -= 1;
        }
    }

    fn clock_envelope(&mut self) {
        if self.envelope_start {
            self.envelope_start = false;
            self.envelope_decay = 15;
            self.envelope_divider = self.volume;
        } else if self.envelope_divider == 0 {
            self.envelope_divider = self.volume;
            if self.envelope_decay > 0 {
                self.envelope_decay -= 1;
            } else if !self.length_enabled {
                // Loop mode (length counter halt = loop envelope)
                self.envelope_decay = 15;
            }
        } else {
            self.envelope_divider -= 1;
        }
    }

    /// Compute the sweep target period. Used for both period updates and muting.
    fn sweep_target_period(&self) -> i32 {
        let current = self.timer_reload as i32;
        let change = current >> self.sweep_shift;
        if self.sweep_negate {
            if self.is_pulse1 {
                current - change - 1 // Pulse 1: one's complement (extra -1)
            } else {
                current - change // Pulse 2: two's complement
            }
        } else {
            current + change
        }
    }

    /// Returns true if the channel should be muted due to sweep conditions.
    /// Muting is evaluated continuously regardless of sweep_enabled.
    fn is_sweep_muting(&self) -> bool {
        self.timer_reload < 8 || self.sweep_target_period() > 0x7FF
    }

    fn clock_sweep(&mut self) {
        // When reload flag is set: if divider was also 0, fire period update first
        if self.sweep_reload {
            let old_divider = self.sweep_divider;
            self.sweep_divider = self.sweep_period;
            self.sweep_reload = false;
            if old_divider == 0
                && self.sweep_enabled
                && self.sweep_shift > 0
                && !self.is_sweep_muting()
            {
                let target = self.sweep_target_period();
                if target >= 0 {
                    self.timer_reload = target as u16;
                }
            }
        } else if self.sweep_divider == 0 {
            self.sweep_divider = self.sweep_period;
            if self.sweep_enabled && self.sweep_shift > 0 && !self.is_sweep_muting() {
                let target = self.sweep_target_period();
                if target >= 0 {
                    self.timer_reload = target as u16;
                }
            }
        } else {
            self.sweep_divider -= 1;
        }
    }

    fn clock_length_counter(&mut self) {
        if self.length_enabled && self.length_counter > 0 {
            self.length_counter -= 1;
        }
    }

    fn output(&self) -> f32 {
        // Sweep muting: timer_reload < 8 OR target period > $7FF (continuous check)
        if self.length_counter == 0 || self.is_sweep_muting() {
            return 0.0;
        }

        const DUTY_TABLE: [[u8; 8]; 4] = [
            [0, 1, 0, 0, 0, 0, 0, 0], // 12.5%
            [0, 1, 1, 0, 0, 0, 0, 0], // 25%
            [0, 1, 1, 1, 1, 0, 0, 0], // 50%
            [1, 0, 0, 1, 1, 1, 1, 1], // 75%
        ];

        if DUTY_TABLE[self.duty as usize][self.duty_counter as usize] == 0 {
            return 0.0;
        }

        let volume = if self.envelope_disable {
            self.volume as f32
        } else {
            self.envelope_decay as f32
        };

        volume
    }
}

impl TriangleChannel {
    fn new() -> Self {
        TriangleChannel {
            linear_counter: 0,
            linear_reload: 0,
            linear_control: false,
            linear_reload_flag: false,
            length_counter: 0,
            timer: 0,
            timer_reload: 0,
            sequence_counter: 0,
            length_enabled: true,
        }
    }

    fn write_control(&mut self, data: u8) {
        self.linear_control = (data & 0x80) != 0;
        self.linear_reload = data & 0x7F;
        self.length_enabled = !self.linear_control;
    }

    fn write_timer_low(&mut self, data: u8) {
        self.timer_reload = (self.timer_reload & 0xFF00) | data as u16;
    }

    fn write_timer_high(&mut self, data: u8, enabled: bool) {
        self.timer_reload = (self.timer_reload & 0x00FF) | ((data as u16 & 0x07) << 8);
        if enabled {
            self.length_counter = LENGTH_TABLE[((data >> 3) & 0x1F) as usize];
        }
        self.linear_reload_flag = true;
    }

    fn step(&mut self) {
        if self.timer == 0 {
            self.timer = self.timer_reload;
            if self.linear_counter > 0 && self.length_counter > 0 && self.timer_reload >= 2 {
                self.sequence_counter = (self.sequence_counter + 1) % 32;
            }
        } else {
            self.timer -= 1;
        }
    }

    fn clock_linear_counter(&mut self) {
        if self.linear_reload_flag {
            self.linear_counter = self.linear_reload;
        } else if self.linear_counter > 0 {
            self.linear_counter -= 1;
        }

        if !self.linear_control {
            self.linear_reload_flag = false;
        }
    }

    fn clock_length_counter(&mut self) {
        if self.length_enabled && self.length_counter > 0 {
            self.length_counter -= 1;
        }
    }

    fn output(&self) -> f32 {
        if self.length_counter == 0 || self.linear_counter == 0 || self.timer_reload < 2 {
            return 0.0;
        }

        const TRIANGLE_SEQUENCE: [u8; 32] = [
            15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10,
            11, 12, 13, 14, 15,
        ];

        TRIANGLE_SEQUENCE[self.sequence_counter as usize] as f32
    }
}

impl NoiseChannel {
    fn new() -> Self {
        NoiseChannel {
            length_counter: 0,
            envelope_divider: 0,
            envelope_decay: 0,
            envelope_disable: false,
            envelope_start: false,
            volume: 0,
            mode: false,
            timer: 0,
            timer_reload: 0,
            shift_register: 1,
            length_enabled: true,
        }
    }

    fn write_control(&mut self, data: u8) {
        self.length_enabled = (data & 0x20) == 0;
        self.envelope_disable = (data & 0x10) != 0;
        self.volume = data & 0x0F;
        // Note: writing to $400C does NOT restart the envelope.
        // Only writing to $400F (4th register) sets envelope_start.
    }

    fn write_period(&mut self, data: u8) {
        self.mode = (data & 0x80) != 0;
        self.timer_reload = NOISE_PERIOD_TABLE[(data & 0x0F) as usize];
    }

    fn write_length(&mut self, data: u8, enabled: bool) {
        if enabled {
            self.length_counter = LENGTH_TABLE[((data >> 3) & 0x1F) as usize];
        }
        self.envelope_start = true;
    }

    fn step(&mut self) {
        if self.timer == 0 {
            self.timer = self.timer_reload;

            let feedback_bit = if self.mode {
                ((self.shift_register >> 0) ^ (self.shift_register >> 6)) & 1
            } else {
                ((self.shift_register >> 0) ^ (self.shift_register >> 1)) & 1
            };

            self.shift_register >>= 1;
            self.shift_register |= feedback_bit << 14;
        } else {
            self.timer -= 1;
        }
    }

    fn clock_envelope(&mut self) {
        if self.envelope_start {
            self.envelope_start = false;
            self.envelope_decay = 15;
            self.envelope_divider = self.volume;
        } else if self.envelope_divider == 0 {
            self.envelope_divider = self.volume;
            if self.envelope_decay > 0 {
                self.envelope_decay -= 1;
            } else if !self.length_enabled {
                self.envelope_decay = 15;
            }
        } else {
            self.envelope_divider -= 1;
        }
    }

    fn clock_length_counter(&mut self) {
        if self.length_enabled && self.length_counter > 0 {
            self.length_counter -= 1;
        }
    }

    fn output(&self) -> f32 {
        if self.length_counter == 0 {
            return 0.0;
        }

        // Noise output: bit 0 of shift register inverted
        if (self.shift_register & 1) != 0 {
            return 0.0;
        }

        let volume = if self.envelope_disable {
            self.volume as f32
        } else {
            self.envelope_decay as f32
        };

        volume
    }
}
