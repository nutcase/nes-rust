#![cfg_attr(not(feature = "dev"), allow(dead_code))]
pub struct Apu {
    ram: Vec<u8>,
    ports: [u8; 4],
    dsp_registers: Vec<u8>,
    timers: [Timer; 3],
    cycle_counter: u64,

    // サウンド生成関連
    channels: [SoundChannel; 8],
    master_volume_left: u8,
    master_volume_right: u8,
    echo_volume_left: u8,
    echo_volume_right: u8,
    sample_rate: u32,
    // Simple CPU<->APU handshake shim to unblock games that poll $2140-$2143.
    handshake_enabled: bool,
    handshake_state: HandshakeState,
    handshake_reads: u32,
    last_host_write: Option<(u8, u8)>,
}

#[derive(Debug, Clone)]
struct SoundChannel {
    // チャンネル制御
    volume_left: u8,
    volume_right: u8,
    pitch: u16,        // 周波数
    sample_start: u16, // サンプル開始アドレス
    sample_loop: u16,  // ループポイント
    envelope: Envelope,

    // 現在の状態
    enabled: bool,
    current_sample: u16,
    phase: u32,
    amplitude: i16,
}

#[derive(Debug, Clone)]
struct Envelope {
    attack_rate: u8,
    decay_rate: u8,
    sustain_level: u8,
    release_rate: u8,
    current_level: u16,
    state: EnvelopeState,
}

#[derive(Debug, Clone, PartialEq)]
enum EnvelopeState {
    Attack,
    Decay,
    Sustain,
    Release,
}

impl SoundChannel {
    fn new() -> Self {
        Self {
            volume_left: 0,
            volume_right: 0,
            pitch: 0,
            sample_start: 0,
            sample_loop: 0,
            envelope: Envelope::new(),
            enabled: false,
            current_sample: 0,
            phase: 0,
            amplitude: 0,
        }
    }

    fn key_on(&mut self) {
        self.enabled = true;
        self.envelope.state = EnvelopeState::Attack;
        self.envelope.current_level = 0;
        self.phase = 0;
    }

    fn key_off(&mut self) {
        self.envelope.state = EnvelopeState::Release;
    }
}

impl Envelope {
    fn new() -> Self {
        Self {
            attack_rate: 0,
            decay_rate: 0,
            sustain_level: 0,
            release_rate: 0,
            current_level: 0,
            state: EnvelopeState::Release,
        }
    }

    fn step(&mut self) {
        match self.state {
            EnvelopeState::Attack => {
                if self.current_level < 0x7FF {
                    self.current_level += (self.attack_rate as u16 + 1) * 8;
                    if self.current_level >= 0x7FF {
                        self.current_level = 0x7FF;
                        self.state = EnvelopeState::Decay;
                    }
                }
            }
            EnvelopeState::Decay => {
                let sustain_target = (self.sustain_level as u16) << 8;
                if self.current_level > sustain_target {
                    let decay_amount = (self.decay_rate as u16 + 1) * 4;
                    self.current_level = self.current_level.saturating_sub(decay_amount);
                    if self.current_level <= sustain_target {
                        self.current_level = sustain_target;
                        self.state = EnvelopeState::Sustain;
                    }
                }
            }
            EnvelopeState::Sustain => {
                // サステインレベル維持
            }
            EnvelopeState::Release => {
                if self.current_level > 0 {
                    let release_amount = (self.release_rate as u16 + 1) * 4;
                    self.current_level = self.current_level.saturating_sub(release_amount);
                }
            }
        }
    }
}

struct Timer {
    enabled: bool,
    target: u8,
    counter: u8,
    divider: u16,
    divider_target: u16,
}

impl Timer {
    fn new(divider_target: u16) -> Self {
        Self {
            enabled: false,
            target: 0,
            counter: 0,
            divider: 0,
            divider_target,
        }
    }

    fn step(&mut self, cycles: u16) {
        if !self.enabled {
            return;
        }

        self.divider += cycles;
        while self.divider >= self.divider_target {
            self.divider -= self.divider_target;
            self.counter = self.counter.wrapping_add(1);
            if self.counter == self.target {
                self.counter = 0;
            }
        }
    }
}

impl Apu {
    pub fn new() -> Self {
        Self {
            ram: vec![0; 0x10000],
            ports: [0; 4],
            dsp_registers: vec![0; 128],
            timers: [Timer::new(128), Timer::new(128), Timer::new(16)],
            cycle_counter: 0,
            channels: [
                SoundChannel::new(),
                SoundChannel::new(),
                SoundChannel::new(),
                SoundChannel::new(),
                SoundChannel::new(),
                SoundChannel::new(),
                SoundChannel::new(),
                SoundChannel::new(),
            ],
            master_volume_left: 127,
            master_volume_right: 127,
            echo_volume_left: 0,
            echo_volume_right: 0,
            sample_rate: 32000, // 32kHz
            handshake_enabled: crate::debug_flags::apu_handshake_plus(),
            handshake_state: HandshakeState::Booting,
            handshake_reads: 0,
            last_host_write: None,
        }
    }

    pub fn step(&mut self, cycles: u8) {
        self.cycle_counter += cycles as u64;

        for timer in &mut self.timers {
            timer.step(cycles as u16);
        }

        // サウンドチャンネルの更新
        self.update_sound_channels(cycles);
    }

    fn update_sound_channels(&mut self, cycles: u8) {
        for channel in &mut self.channels {
            if channel.enabled {
                // エンベロープ更新
                channel.envelope.step();

                // 位相更新（簡易実装）
                channel.phase = channel
                    .phase
                    .wrapping_add(channel.pitch as u32 * cycles as u32);

                // 波形生成（簡易な矩形波）
                let wave_position = (channel.phase >> 16) & 0xFFFF;
                channel.amplitude = if wave_position < 0x8000 {
                    (channel.envelope.current_level as i16) >> 3
                } else {
                    -((channel.envelope.current_level as i16) >> 3)
                };
            } else {
                channel.amplitude = 0;
            }
        }
    }

    pub fn read(&mut self, addr: u8) -> u8 {
        if self.handshake_enabled {
            match self.handshake_state {
                HandshakeState::Booting => {
                    // Return a simple toggling pattern per port to indicate life
                    let base = if addr & 1 == 0 { 0xAA } else { 0x55 };
                    let resp = if (self.handshake_reads & 1) == 0 {
                        base
                    } else {
                        base ^ 0xFF
                    };
                    self.handshake_reads = self.handshake_reads.saturating_add(1);
                    // Expedite READY transition — many titles poll a handful of times only
                    if self.handshake_reads >= 16 {
                        self.handshake_state = HandshakeState::Ready;
                    }
                    return resp;
                }
                HandshakeState::Ready => {
                    return 0x00;
                }
            }
        }
        match addr {
            0x00..=0x03 => self.ports[addr as usize],
            _ => 0,
        }
    }

    pub fn write(&mut self, addr: u8, value: u8) {
        if let 0x00..=0x03 = addr {
            self.ports[addr as usize] = value;
            if self.handshake_enabled {
                // Simple pattern-based unlocks commonly used by boot code
                // Any of these values indicate host has poked APU and is ready to proceed.
                match value {
                    0x00 | 0xCC | 0xAA | 0x55 => {
                        self.handshake_state = HandshakeState::Ready;
                    }
                    _ => {}
                }
                // Any direct host write to APUIO signifies liveness — remember last write
                self.last_host_write = Some((addr, value));
            }
        }
    }

    pub fn read_ram(&self, addr: u16) -> u8 {
        self.ram[addr as usize]
    }

    pub fn write_ram(&mut self, addr: u16, value: u8) {
        self.ram[addr as usize] = value;
    }

    pub fn read_dsp(&self, addr: u8) -> u8 {
        if (addr as usize) < self.dsp_registers.len() {
            self.dsp_registers[addr as usize]
        } else {
            0
        }
    }

    pub fn write_dsp(&mut self, addr: u8, value: u8) {
        if (addr as usize) < self.dsp_registers.len() {
            self.dsp_registers[addr as usize] = value;

            // DSPレジスタに基づいてサウンドチャンネルを更新
            self.update_channel_from_dsp(addr, value);
        }
    }

    fn update_channel_from_dsp(&mut self, addr: u8, value: u8) {
        let channel_num = (addr & 0x0F) as usize;

        if channel_num >= 8 {
            return;
        }

        match addr & 0xF0 {
            0x00 => {
                // 各チャンネルのボリューム (左)
                match addr & 0x0F {
                    0x00 => self.channels[0].volume_left = value,
                    0x01 => self.channels[0].volume_right = value,
                    0x02 => {
                        self.channels[0].pitch = (self.channels[0].pitch & 0xFF00) | value as u16
                    }
                    0x03 => {
                        self.channels[0].pitch =
                            (self.channels[0].pitch & 0x00FF) | ((value as u16) << 8)
                    }
                    0x04 => self.channels[0].sample_start = value as u16,
                    0x05 => {
                        // ADSR設定
                        self.channels[0].envelope.attack_rate = value & 0x0F;
                        self.channels[0].envelope.decay_rate = (value >> 4) & 0x07;
                    }
                    0x06 => {
                        self.channels[0].envelope.sustain_level = value >> 5;
                        self.channels[0].envelope.release_rate = value & 0x1F;
                    }
                    _ => {}
                }
            }
            0x10 => {
                // チャンネル 1
                self.apply_channel_register(1, addr & 0x0F, value);
            }
            0x20 => {
                // チャンネル 2
                self.apply_channel_register(2, addr & 0x0F, value);
            }
            0x30 => {
                // チャンネル 3
                self.apply_channel_register(3, addr & 0x0F, value);
            }
            0x40 => {
                // チャンネル 4
                self.apply_channel_register(4, addr & 0x0F, value);
            }
            0x50 => {
                // チャンネル 5
                self.apply_channel_register(5, addr & 0x0F, value);
            }
            0x60 => {
                // チャンネル 6
                self.apply_channel_register(6, addr & 0x0F, value);
            }
            0x70 => {
                // チャンネル 7
                self.apply_channel_register(7, addr & 0x0F, value);
            }
            _ => {}
        }

        // グローバル制御
        match addr {
            0x0C => self.master_volume_left = value,
            0x1C => self.master_volume_right = value,
            0x2C => self.echo_volume_left = value,
            0x3C => self.echo_volume_right = value,
            0x4C => {
                // Key On
                for i in 0..8 {
                    if value & (1 << i) != 0 {
                        self.channels[i].key_on();
                    }
                }
            }
            0x5C => {
                // Key Off
                for i in 0..8 {
                    if value & (1 << i) != 0 {
                        self.channels[i].key_off();
                    }
                }
            }
            _ => {}
        }
    }

    fn apply_channel_register(&mut self, channel: usize, reg: u8, value: u8) {
        if channel >= 8 {
            return;
        }

        match reg {
            0x00 => self.channels[channel].volume_left = value,
            0x01 => self.channels[channel].volume_right = value,
            0x02 => {
                self.channels[channel].pitch =
                    (self.channels[channel].pitch & 0xFF00) | value as u16
            }
            0x03 => {
                self.channels[channel].pitch =
                    (self.channels[channel].pitch & 0x00FF) | ((value as u16) << 8)
            }
            0x04 => self.channels[channel].sample_start = value as u16,
            0x05 => {
                self.channels[channel].envelope.attack_rate = value & 0x0F;
                self.channels[channel].envelope.decay_rate = (value >> 4) & 0x07;
            }
            0x06 => {
                self.channels[channel].envelope.sustain_level = value >> 5;
                self.channels[channel].envelope.release_rate = value & 0x1F;
            }
            _ => {}
        }
    }

    pub fn reset(&mut self) {
        self.ram.fill(0);
        self.ports = [0; 4];
        self.dsp_registers.fill(0);
        self.cycle_counter = 0;

        for timer in &mut self.timers {
            timer.enabled = false;
            timer.counter = 0;
            timer.divider = 0;
        }

        for channel in &mut self.channels {
            *channel = SoundChannel::new();
        }

        self.master_volume_left = 127;
        self.master_volume_right = 127;
        self.echo_volume_left = 0;
        self.echo_volume_right = 0;
    }

    // オーディオサンプル生成（ステレオ）
    pub fn generate_audio_samples(&self, samples: &mut [(i16, i16)]) {
        for sample in samples {
            let mut left_sum: i32 = 0;
            let mut right_sum: i32 = 0;

            // 全チャンネルをミックス
            for channel in &self.channels {
                if channel.enabled {
                    let amplitude = channel.amplitude as i32;
                    left_sum += (amplitude * channel.volume_left as i32) >> 7;
                    right_sum += (amplitude * channel.volume_right as i32) >> 7;
                }
            }

            // マスターボリューム適用
            left_sum = (left_sum * self.master_volume_left as i32) >> 7;
            right_sum = (right_sum * self.master_volume_right as i32) >> 7;

            // クリッピング
            left_sum = left_sum.clamp(-32768, 32767);
            right_sum = right_sum.clamp(-32768, 32767);

            *sample = (left_sum as i16, right_sum as i16);
        }
    }

    pub fn get_sample_rate(&self) -> u32 {
        self.sample_rate
    }

    // --- Save state serialization ---
    pub fn to_save_state(&self) -> crate::savestate::ApuSaveState {
        use crate::savestate::{
            ApuSaveState, EnvelopeSaveState, SoundChannelSaveState, TimerSaveState,
        };
        let timers = self
            .timers
            .iter()
            .map(|t| TimerSaveState {
                enabled: t.enabled,
                target: t.target,
                counter: t.counter,
                divider: t.divider,
                divider_target: t.divider_target,
            })
            .collect();
        let channels = self
            .channels
            .iter()
            .map(|c| SoundChannelSaveState {
                volume_left: c.volume_left,
                volume_right: c.volume_right,
                pitch: c.pitch,
                sample_start: c.sample_start,
                sample_loop: c.sample_loop,
                envelope: EnvelopeSaveState {
                    attack_rate: c.envelope.attack_rate,
                    decay_rate: c.envelope.decay_rate,
                    sustain_level: c.envelope.sustain_level,
                    release_rate: c.envelope.release_rate,
                    current_level: c.envelope.current_level,
                    state: match c.envelope.state {
                        EnvelopeState::Attack => 0,
                        EnvelopeState::Decay => 1,
                        EnvelopeState::Sustain => 2,
                        EnvelopeState::Release => 3,
                    },
                },
                enabled: c.enabled,
                current_sample: c.current_sample,
                phase: c.phase,
                amplitude: c.amplitude,
            })
            .collect();

        ApuSaveState {
            ram: self.ram.clone(),
            ports: self.ports,
            dsp_registers: self.dsp_registers.clone(),
            cycle_counter: self.cycle_counter,
            timers,
            channels,
            master_volume_left: self.master_volume_left,
            master_volume_right: self.master_volume_right,
            echo_volume_left: self.echo_volume_left,
            echo_volume_right: self.echo_volume_right,
        }
    }

    pub fn load_from_save_state(&mut self, st: &crate::savestate::ApuSaveState) {
        if self.ram.len() == st.ram.len() {
            self.ram.copy_from_slice(&st.ram);
        }
        self.ports = st.ports;
        if self.dsp_registers.len() == st.dsp_registers.len() {
            self.dsp_registers.copy_from_slice(&st.dsp_registers);
        }
        self.cycle_counter = st.cycle_counter;
        for (i, t) in st.timers.iter().enumerate() {
            if i < self.timers.len() {
                self.timers[i].enabled = t.enabled;
                self.timers[i].target = t.target;
                self.timers[i].counter = t.counter;
                self.timers[i].divider = t.divider;
                self.timers[i].divider_target = t.divider_target;
            }
        }
        for (i, c) in st.channels.iter().enumerate() {
            if i < self.channels.len() {
                let ch = &mut self.channels[i];
                ch.volume_left = c.volume_left;
                ch.volume_right = c.volume_right;
                ch.pitch = c.pitch;
                ch.sample_start = c.sample_start;
                ch.sample_loop = c.sample_loop;
                ch.envelope.attack_rate = c.envelope.attack_rate;
                ch.envelope.decay_rate = c.envelope.decay_rate;
                ch.envelope.sustain_level = c.envelope.sustain_level;
                ch.envelope.release_rate = c.envelope.release_rate;
                ch.envelope.current_level = c.envelope.current_level;
                ch.envelope.state = match c.envelope.state {
                    0 => EnvelopeState::Attack,
                    1 => EnvelopeState::Decay,
                    2 => EnvelopeState::Sustain,
                    _ => EnvelopeState::Release,
                };
                ch.enabled = c.enabled;
                ch.current_sample = c.current_sample;
                ch.phase = c.phase;
                ch.amplitude = c.amplitude;
            }
        }
        self.master_volume_left = st.master_volume_left;
        self.master_volume_right = st.master_volume_right;
        self.echo_volume_left = st.echo_volume_left;
        self.echo_volume_right = st.echo_volume_right;
    }

    // Enable/disable the lightweight handshake shim at runtime
    pub fn set_handshake_enabled(&mut self, enabled: bool) {
        self.handshake_enabled = enabled;
        if enabled {
            self.handshake_state = HandshakeState::Booting;
            self.handshake_reads = 0;
            self.last_host_write = None;
        }
    }

    pub fn handshake_state_str(&self) -> &'static str {
        if !self.handshake_enabled {
            return "off";
        }
        match self.handshake_state {
            HandshakeState::Booting => "booting",
            HandshakeState::Ready => "ready",
        }
    }
}
//
// Many APU fields/methods are currently stubs for future work.
// Suppress dead_code warnings while iterating.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HandshakeState {
    Booting,
    Ready,
}
