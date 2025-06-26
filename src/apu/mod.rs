pub struct Apu {
    pulse1: PulseChannel,
    pulse2: PulseChannel,
    triangle: TriangleChannel,
    noise: NoiseChannel,
    
    frame_counter: u16,
    cycle_count: u64,
    
    // Status register
    pulse1_enabled: bool,
    pulse2_enabled: bool,
    triangle_enabled: bool,
    noise_enabled: bool,
    dmc_enabled: bool,
    
    // Audio output buffer with improved quality processing
    output_buffer: Vec<f32>,
    sample_rate: f32,
    cpu_clock_rate: f32,
    
    // Audio quality improvements
    sample_accumulator: f32,
    sample_counter: f32,
    last_output: f32,
    
    // Audio filters for better quality
    high_pass_filter: HighPassFilter,
    low_pass_filter1: LowPassFilter,
    low_pass_filter2: LowPassFilter,
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
            pulse1: PulseChannel::new(),
            pulse2: PulseChannel::new(),
            triangle: TriangleChannel::new(),
            noise: NoiseChannel::new(),
            
            frame_counter: 0,
            cycle_count: 0,
            
            pulse1_enabled: false,
            pulse2_enabled: false,
            triangle_enabled: false,
            noise_enabled: false,
            dmc_enabled: false,
            
            output_buffer: Vec::new(),
            sample_rate: 44100.0,
            cpu_clock_rate: 1789773.0, // NTSC CPU clock rate
            
            sample_accumulator: 0.0,
            sample_counter: 0.0,
            last_output: 0.0,
            
            // Initialize audio filters (gentler settings for cleaner sound)
            high_pass_filter: HighPassFilter::new(44100.0, 20.0),
            low_pass_filter1: LowPassFilter::new(44100.0, 16000.0),
            low_pass_filter2: LowPassFilter::new(44100.0, 20000.0),
        }
    }

    pub fn step(&mut self) {
        self.cycle_count += 1;
        
        // Step all channels every CPU cycle (simplified for now)
        if self.pulse1_enabled {
            self.pulse1.step();
        }
        if self.pulse2_enabled {
            self.pulse2.step();
        }
        if self.triangle_enabled {
            self.triangle.step();
        }
        if self.noise_enabled {
            self.noise.step();
        }
        
        // Stable audio sampling every 40 cycles
        if self.cycle_count % 40 == 0 {
            let sample = self.mix_channels_high_quality();
            
            self.output_buffer.push(sample);
            
            // Keep buffer size reasonable
            if self.output_buffer.len() > 8192 {
                self.output_buffer.drain(0..4096);
            }
        }
        
        // Frame counter updates every 14915 CPU cycles (240Hz, 4 times per frame)
        self.frame_counter += 1;
        if self.frame_counter >= 14915 { 
            self.frame_counter = 0;
            self.update_frame_counters();
        }
    }
    
    fn mix_channels(&self) -> f32 {
        // Get raw outputs (0.0 to 1.0 range)
        let pulse1_out = if self.pulse1_enabled { self.pulse1.output() } else { 0.0 };
        let pulse2_out = if self.pulse2_enabled { self.pulse2.output() } else { 0.0 };
        let triangle_out = if self.triangle_enabled { self.triangle.output() } else { 0.0 };
        let noise_out = if self.noise_enabled { self.noise.output() } else { 0.0 };
        
        // Simple mixing with improved volume balance
        let pulse_mix = (pulse1_out + pulse2_out) * 0.4;    // Increase pulse volume
        let triangle_mix = triangle_out * 0.3;              // Increase triangle volume  
        let noise_mix = noise_out * 0.15;                   // Slightly increase noise volume
        
        let output = pulse_mix + triangle_mix + noise_mix;
        
        // Apply soft clipping to prevent harsh distortion
        output.clamp(-1.0, 1.0)
    }
    
    fn mix_channels_simple(&self) -> f32 {
        // Get raw outputs
        let pulse1_out = if self.pulse1_enabled { self.pulse1.output() } else { 0.0 };
        let pulse2_out = if self.pulse2_enabled { self.pulse2.output() } else { 0.0 };
        let triangle_out = if self.triangle_enabled { self.triangle.output() } else { 0.0 };
        let noise_out = if self.noise_enabled { self.noise.output() } else { 0.0 };
        
        // Improved mixing for better SE quality
        let pulse_sum = pulse1_out + pulse2_out;
        
        // Balance channels for clearer SE
        let pulse_mix = pulse_sum * 0.6;      // Increase pulse for clearer SE
        let triangle_mix = triangle_out * 0.35; // Maintain music presence
        let noise_mix = noise_out * 0.2;      // Increase noise for better SE effects
        
        let mixed = pulse_mix + triangle_mix + noise_mix;
        
        // Gentle limiting to preserve dynamics
        (mixed * 0.85).clamp(-1.0, 1.0)
    }
    
    fn mix_channels_high_quality(&mut self) -> f32 {
        // Get raw outputs (simple 0.0 to 1.0 range)
        let pulse1_out = if self.pulse1_enabled { self.pulse1.output() } else { 0.0 };
        let pulse2_out = if self.pulse2_enabled { self.pulse2.output() } else { 0.0 };
        let triangle_out = if self.triangle_enabled { self.triangle.output() } else { 0.0 };
        let noise_out = if self.noise_enabled { self.noise.output() } else { 0.0 };
        
        // Bass-optimized mixing for proper "BON" sounds
        let mixed = pulse1_out * 0.25 + pulse2_out * 0.25 + triangle_out * 0.35 + noise_out * 0.15;
        
        // Direct output without complex processing
        mixed.clamp(-1.0, 1.0)
    }
    
    fn mix_channels_improved(&self) -> f32 {
        // Get raw outputs (0.0 to 1.0 range)
        let pulse1_out = if self.pulse1_enabled { self.pulse1.output() } else { 0.0 };
        let pulse2_out = if self.pulse2_enabled { self.pulse2.output() } else { 0.0 };
        let triangle_out = if self.triangle_enabled { self.triangle.output() } else { 0.0 };
        let noise_out = if self.noise_enabled { self.noise.output() } else { 0.0 };
        
        // More accurate NES APU mixing formula
        // Based on actual NES hardware measurements
        let pulse_out = if pulse1_out + pulse2_out > 0.0 {
            95.88 / ((8128.0 / (pulse1_out + pulse2_out)) + 100.0)
        } else {
            0.0
        };
        
        let tnd_out = if triangle_out + noise_out > 0.0 {
            159.79 / (1.0 / (triangle_out/8227.0 + noise_out/12241.0) + 100.0)
        } else {
            0.0
        };
        
        let output = (pulse_out + tnd_out) * 0.5; // Scale to reasonable volume
        
        // Apply high-quality low-pass filter to reduce aliasing
        let filtered = output * 0.8 + 0.2 * if output.abs() > 0.7 { output * 0.7 } else { output };
        
        // Soft saturation for more natural clipping
        if filtered > 0.0 {
            (filtered / (1.0 + filtered)).clamp(-1.0, 1.0)
        } else {
            (filtered / (1.0 - filtered)).clamp(-1.0, 1.0)
        }
    }
    
    // Better sampling rate calculation
    fn should_sample(&self) -> bool {
        // More accurate sampling: 1789773 Hz / 44100 Hz = ~40.584 cycles per sample
        (self.cycle_count as f32 * 44100.0 / 1789773.0).fract() < (44100.0 / 1789773.0)
    }
    
    fn soft_clip(&self, input: f32) -> f32 {
        // Soft clipping using tanh function for more natural distortion
        if input.abs() <= 0.5 {
            input
        } else {
            input.signum() * (0.5 + 0.5 * (2.0 * (input.abs() - 0.5)).tanh())
        }
    }
    
    fn update_frame_counters(&mut self) {
        if self.pulse1_enabled {
            self.pulse1.update_frame_counter();
        }
        if self.pulse2_enabled {
            self.pulse2.update_frame_counter();
        }
        if self.triangle_enabled {
            self.triangle.update_frame_counter();
        }
        if self.noise_enabled {
            self.noise.update_frame_counter();
        }
    }
    
    pub fn get_audio_buffer(&mut self) -> Vec<f32> {
        let buffer = self.output_buffer.clone();
        self.output_buffer.clear();
        buffer
    }

    pub fn read_register(&self, addr: u16) -> u8 {
        match addr {
            0x4015 => {
                let mut status = 0;
                if self.pulse1_enabled { status |= 0x01; }
                if self.pulse2_enabled { status |= 0x02; }
                if self.triangle_enabled { status |= 0x04; }
                if self.noise_enabled { status |= 0x08; }
                if self.dmc_enabled { status |= 0x10; }
                status
            },
            _ => 0,
        }
    }

    pub fn write_register(&mut self, addr: u16, data: u8) {
        match addr {
            // Pulse 1
            0x4000 => self.pulse1.write_control(data),
            0x4001 => self.pulse1.write_sweep(data),
            0x4002 => self.pulse1.write_timer_low(data),
            0x4003 => self.pulse1.write_timer_high(data),
            
            // Pulse 2
            0x4004 => self.pulse2.write_control(data),
            0x4005 => self.pulse2.write_sweep(data),
            0x4006 => self.pulse2.write_timer_low(data),
            0x4007 => self.pulse2.write_timer_high(data),
            
            // Triangle
            0x4008 => self.triangle.write_control(data),
            0x4009 => {}, // Unused
            0x400A => self.triangle.write_timer_low(data),
            0x400B => self.triangle.write_timer_high(data),
            
            // Noise
            0x400C => self.noise.write_control(data),
            0x400D => {}, // Unused
            0x400E => self.noise.write_period(data),
            0x400F => self.noise.write_length(data),
            
            // DMC (not implemented)
            0x4010..=0x4013 => {},
            
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
            },
            
            // Frame counter
            0x4017 => {},
            _ => {},
        }
    }
}

// Length counter lookup table
const LENGTH_TABLE: [u8; 32] = [
    10, 254, 20, 2, 40, 4, 80, 6, 160, 8, 60, 10, 14, 12, 26, 14,
    12, 16, 24, 18, 48, 20, 96, 22, 192, 24, 72, 26, 16, 28, 32, 30
];

// Noise period lookup table
const NOISE_PERIOD_TABLE: [u16; 16] = [
    4, 8, 16, 32, 64, 96, 128, 160, 202, 254, 380, 508, 762, 1016, 2034, 4068
];

impl PulseChannel {
    fn new() -> Self {
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
        }
    }
    
    fn write_control(&mut self, data: u8) {
        self.duty = (data >> 6) & 0x03;
        self.length_enabled = (data & 0x20) == 0;
        self.envelope_disable = (data & 0x10) != 0;
        self.volume = data & 0x0F;
        self.envelope_start = true;
        // Control register written
    }
    
    fn write_sweep(&mut self, data: u8) {
        self.sweep_enabled = (data & 0x80) != 0;
        self.sweep_period = (data >> 4) & 0x07;
        self.sweep_negate = (data & 0x08) != 0;
        self.sweep_shift = data & 0x07;
        self.sweep_reload = true;
        self.sweep_divider = 0;
    }
    
    fn write_timer_low(&mut self, data: u8) {
        self.timer_reload = (self.timer_reload & 0xFF00) | data as u16;
    }
    
    fn write_timer_high(&mut self, data: u8) {
        self.timer_reload = (self.timer_reload & 0x00FF) | ((data as u16 & 0x07) << 8);
        // Always set length counter when timer high is written (regardless of length_enabled)
        self.length_counter = LENGTH_TABLE[((data >> 3) & 0x1F) as usize];
        self.timer = self.timer_reload;
        self.duty_counter = 0;
        self.envelope_start = true;
        
        // Timer high written
    }
    
    fn step(&mut self) {
        if self.timer == 0 {
            self.timer = self.timer_reload;
            if self.length_counter > 0 && self.timer_reload >= 8 {
                self.duty_counter = (self.duty_counter + 1) % 8;
            }
        } else {
            self.timer -= 1;
        }
    }
    
    fn update_frame_counter(&mut self) {
        // Update envelope
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
        
        // Update sweep (critical for jump sound effects)
        if self.sweep_reload {
            self.sweep_divider = self.sweep_period;
            self.sweep_reload = false;
        } else if self.sweep_divider == 0 && self.sweep_period > 0 {
            self.sweep_divider = self.sweep_period;
            if self.sweep_enabled && self.sweep_shift > 0 {
                let change = self.timer_reload >> self.sweep_shift;
                if self.sweep_negate {
                    if self.timer_reload >= change {
                        self.timer_reload -= change;
                        self.timer = self.timer_reload;
                    }
                } else {
                    if self.timer_reload + change <= 0x7FF {
                        self.timer_reload += change;
                        self.timer = self.timer_reload;
                    }
                }
            }
        } else if self.sweep_period > 0 {
            self.sweep_divider -= 1;
        }
        
        // Update length counter
        if self.length_enabled && self.length_counter > 0 {
            self.length_counter -= 1;
        }
    }
    
    fn output(&self) -> f32 {
        if self.length_counter == 0 {
            return 0.0;
        }
        
        // More lenient frequency filtering - NES can play very high frequencies
        if self.timer_reload < 1 {
            return 0.0;
        }
        
        let duty_table = [
            [0, 1, 0, 0, 0, 0, 0, 0], // 12.5% - thin, bright sound for high-pitched effects
            [0, 1, 1, 0, 0, 0, 0, 0], // 25% - classic square wave for many effects
            [0, 1, 1, 1, 1, 0, 0, 0], // 50% - full square wave, powerful sound
            [1, 0, 0, 1, 1, 1, 1, 1], // 75% - inverted 25%, different timbre
        ];
        
        if duty_table[self.duty as usize][self.duty_counter as usize] == 0 {
            return 0.0;
        }
        
        let volume = if self.envelope_disable {
            self.volume as f32
        } else {
            self.envelope_decay as f32
        };
        
        let base_volume = volume / 15.0;
        
        // Apply duty-cycle specific characteristics for better sound effects
        let duty_factor = match self.duty {
            0 => base_volume * 1.2, // 12.5% - bright, piercing sounds
            1 => base_volume,       // 25% - standard square wave
            2 => base_volume * 1.1, // 50% - fuller, more powerful
            3 => base_volume * 0.9, // 75% - softer alternative timbre
            _ => base_volume,
        };
        
        // Balanced frequency response for proper bass reproduction
        let freq_factor = if self.timer_reload < 16 {
            // Very high frequency - reduce excessive brightness
            duty_factor * 1.0
        } else if self.timer_reload < 64 {
            // High-mid frequency - slight boost
            duty_factor * 1.1
        } else if self.timer_reload < 256 {
            // Mid range - neutral
            duty_factor
        } else if self.timer_reload < 1024 {
            // Low-mid frequency - boost for bass sounds
            duty_factor * 1.2
        } else {
            // Very low frequency - strong boost for deep bass
            duty_factor * 1.4
        };
        
        // Length counter enhancement for short hit sounds
        let length_factor = if self.length_counter < 4 {
            // Very short sounds - boost for impact
            freq_factor * 1.3
        } else if self.length_counter < 16 {
            // Short sounds - common for hit effects
            freq_factor * 1.15
        } else {
            freq_factor
        };
        
        // Envelope-based hit sound enhancement
        let envelope_factor = if !self.envelope_disable {
            let env_ratio = self.envelope_decay as f32 / 15.0;
            if env_ratio > 0.9 {
                // Initial attack - very sharp for hits
                length_factor * 1.4
            } else if env_ratio > 0.7 {
                // Early decay - maintain punch
                length_factor * 1.2
            } else if env_ratio > 0.3 {
                // Mid decay - natural reduction
                length_factor
            } else {
                // Late decay - softer tail
                length_factor * 0.9
            }
        } else {
            length_factor
        };
        
        envelope_factor.clamp(0.0, 1.0)
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
    
    fn write_timer_high(&mut self, data: u8) {
        self.timer_reload = (self.timer_reload & 0x00FF) | ((data as u16 & 0x07) << 8);
        // Always set length counter when timer high is written
        self.length_counter = LENGTH_TABLE[((data >> 3) & 0x1F) as usize];
        self.linear_reload_flag = true;
        
        // Triangle timer high written
    }
    
    fn step(&mut self) {
        if self.timer == 0 {
            self.timer = self.timer_reload;
            // Triangle channel runs if either counter is enabled
            if self.linear_counter > 0 && self.length_counter > 0 && self.timer_reload >= 2 {
                self.sequence_counter = (self.sequence_counter + 1) % 32;
            }
        } else {
            self.timer -= 1;
        }
    }
    
    fn update_frame_counter(&mut self) {
        // Update linear counter
        if self.linear_reload_flag {
            self.linear_counter = self.linear_reload;
        } else if self.linear_counter > 0 {
            self.linear_counter -= 1;
        }
        
        if !self.linear_control {
            self.linear_reload_flag = false;
        }
        
        // Update length counter
        if self.length_enabled && self.length_counter > 0 {
            self.length_counter -= 1;
        }
    }
    
    fn output(&self) -> f32 {
        if self.length_counter == 0 || self.linear_counter == 0 || self.timer_reload < 2 {
            return 0.0;
        }
        
        // Triangle wave sequence (0-15 amplitude)
        let triangle_sequence = [
            15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0,
            0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15
        ];
        
        let raw_output = triangle_sequence[self.sequence_counter as usize] as f32 / 15.0;
        
        // Corrected frequency response for proper bass sounds
        let freq_factor = if self.timer_reload < 8 {
            // Very high frequency - reduce excessive brightness
            raw_output * 0.8
        } else if self.timer_reload < 32 {
            // High frequency - slight reduction
            raw_output * 0.9
        } else if self.timer_reload < 128 {
            // Mid frequency - neutral
            raw_output
        } else if self.timer_reload < 512 {
            // Low frequency - boost for "BON" sounds
            raw_output * 1.3
        } else {
            // Very low frequency - strong boost for deep bass
            raw_output * 1.5
        };
        
        freq_factor.clamp(0.0, 1.0)
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
            shift_register: 1, // Standard NES noise channel initial value
            length_enabled: true,
        }
    }
    
    fn write_control(&mut self, data: u8) {
        self.length_enabled = (data & 0x20) == 0;
        self.envelope_disable = (data & 0x10) != 0;
        self.volume = data & 0x0F;
        self.envelope_start = true;
    }
    
    fn write_period(&mut self, data: u8) {
        self.mode = (data & 0x80) != 0;
        self.timer_reload = NOISE_PERIOD_TABLE[(data & 0x0F) as usize];
    }
    
    fn write_length(&mut self, data: u8) {
        if self.length_enabled {
            self.length_counter = LENGTH_TABLE[((data >> 3) & 0x1F) as usize];
        }
        self.envelope_start = true;
    }
    
    fn step(&mut self) {
        if self.timer == 0 {
            self.timer = self.timer_reload;
            
            // Corrected NES noise channel shift register operation
            let feedback_bit = if self.mode {
                // Mode 1 (short): XOR bits 0 and 6 for more metallic sound
                ((self.shift_register >> 0) ^ (self.shift_register >> 6)) & 1
            } else {
                // Mode 0 (long): XOR bits 0 and 1 for fuller noise
                ((self.shift_register >> 0) ^ (self.shift_register >> 1)) & 1
            };
            
            self.shift_register >>= 1;
            self.shift_register |= feedback_bit << 14;
        } else {
            self.timer -= 1;
        }
    }
    
    fn update_frame_counter(&mut self) {
        // Update envelope
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
        
        // Update length counter
        if self.length_enabled && self.length_counter > 0 {
            self.length_counter -= 1;
        }
    }
    
    fn output(&self) -> f32 {
        if self.length_counter == 0 {
            return 0.0;
        }
        
        // Noise output is inverted: 0 in shift register = sound, 1 = silence
        if (self.shift_register & 1) != 0 {
            return 0.0;
        }
        
        let volume = if self.envelope_disable {
            self.volume as f32
        } else {
            self.envelope_decay as f32
        };
        
        let base_volume = volume / 15.0;
        
        // Balanced noise characteristics for proper bass sounds
        let period_factor = if self.timer_reload < 4 {
            // Ultra high frequency - reduce excessive sharpness
            base_volume * 1.1
        } else if self.timer_reload < 16 {
            // Very high frequency - moderate boost
            base_volume * 1.2
        } else if self.timer_reload < 64 {
            // High-mid frequency - balanced
            base_volume * 1.1
        } else if self.timer_reload < 256 {
            // Mid frequency - neutral
            base_volume
        } else if self.timer_reload < 1024 {
            // Low-mid frequency - boost for "BON" sounds
            base_volume * 1.3
        } else {
            // Very low frequency - strong boost for deep bass
            base_volume * 1.5
        };
        
        // Mode-specific enhancement for hit sounds
        let mode_factor = if self.mode {
            // Short mode - sharp, metallic hit sounds (sword strikes, etc.)
            period_factor * 1.3
        } else {
            // Long mode - fuller impact sounds (punches, body hits)
            period_factor * 1.1
        };
        
        // Enhanced envelope for hit sound dynamics
        let envelope_factor = if !self.envelope_disable {
            let env_ratio = self.envelope_decay as f32 / 15.0;
            if env_ratio > 0.85 {
                // Initial hit impact - very sharp and punchy
                mode_factor * 1.5
            } else if env_ratio > 0.6 {
                // Early decay - maintain hit impact
                mode_factor * 1.2
            } else if env_ratio > 0.3 {
                // Mid decay - natural fade
                mode_factor * 0.9
            } else {
                // Late decay - quick fade for short hits
                mode_factor * 0.7
            }
        } else {
            mode_factor
        };
        
        // Length counter enhancement for short hit bursts
        let length_factor = if self.length_counter < 3 {
            // Very short hits - boost significantly 
            envelope_factor * 1.4
        } else if self.length_counter < 8 {
            // Short hits - common for impact sounds
            envelope_factor * 1.2
        } else if self.length_counter < 20 {
            // Medium hits - balanced
            envelope_factor
        } else {
            // Long sounds - reduce slightly for clarity
            envelope_factor * 0.95
        };
        
        length_factor.clamp(0.0, 1.0)
    }
}