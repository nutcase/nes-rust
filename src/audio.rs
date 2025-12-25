#![cfg_attr(not(feature = "dev"), allow(dead_code))]
use rodio::{OutputStream, Sink, Source};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::Duration;

const DEFAULT_AUDIO_CHUNK_FRAMES: usize = 256;
const DEFAULT_AUDIO_BUFFER_FRAMES: usize = 32768;

struct AudioRing {
    buffer: VecDeque<(i16, i16)>,
    max_frames: usize,
}

impl AudioRing {
    fn new(max_frames: usize) -> Self {
        Self {
            buffer: VecDeque::with_capacity(max_frames),
            max_frames,
        }
    }

    fn len(&self) -> usize {
        self.buffer.len()
    }

    fn clear(&mut self) {
        self.buffer.clear();
    }

    fn push_samples(&mut self, samples: &[(i16, i16)]) {
        if samples.is_empty() {
            return;
        }
        let needed = self.buffer.len().saturating_add(samples.len());
        if needed > self.max_frames {
            let drop = needed - self.max_frames;
            for _ in 0..drop {
                self.buffer.pop_front();
            }
        }
        self.buffer.extend(samples.iter().copied());
    }

    fn pop_into(&mut self, out: &mut Vec<(i16, i16)>, max: usize) -> usize {
        out.clear();
        let count = self.buffer.len().min(max);
        out.reserve(count);
        for _ in 0..count {
            if let Some(v) = self.buffer.pop_front() {
                out.push(v);
            }
        }
        count
    }
}

pub struct SnesAudioSource {
    ring: Arc<Mutex<AudioRing>>,
    sample_rate: u32,
    channels: u16,
    current_frame: Vec<(i16, i16)>,
    // Interleaved sample cursor (L,R,L,R...) within current_frame.
    // current_frame holds stereo frames; cursor counts i16 samples.
    sample_cursor: usize,
    chunk_frames: usize,
}

impl SnesAudioSource {
    pub fn new(ring: Arc<Mutex<AudioRing>>, sample_rate: u32, chunk_frames: usize) -> Self {
        let channels = 2; // Stereo

        Self {
            ring,
            sample_rate,
            channels,
            current_frame: Vec::with_capacity(chunk_frames),
            sample_cursor: 0,
            chunk_frames: chunk_frames.max(1),
        }
    }

    fn generate_audio_frame(&mut self) {
        let mut got = 0usize;
        if let Ok(mut ring) = self.ring.lock() {
            got = ring.pop_into(&mut self.current_frame, self.chunk_frames);
        }

        if got < self.chunk_frames {
            self.current_frame.resize(self.chunk_frames, (0, 0));
        }
        self.sample_cursor = 0;
    }
}

impl Iterator for SnesAudioSource {
    type Item = i16;

    fn next(&mut self) -> Option<Self::Item> {
        // Generate new frame if we've consumed the current one
        if self.sample_cursor >= self.current_frame.len().saturating_mul(2) {
            self.generate_audio_frame();
        }

        if self.current_frame.is_empty() {
            return Some(0);
        }

        let sample_index = self.sample_cursor / 2;
        let is_right_channel = self.sample_cursor % 2 == 1;

        if sample_index >= self.current_frame.len() {
            return Some(0);
        }

        let sample = if is_right_channel {
            self.current_frame[sample_index].1
        } else {
            self.current_frame[sample_index].0
        };

        self.sample_cursor += 1;
        Some(sample)
    }
}

impl Source for SnesAudioSource {
    fn current_frame_len(&self) -> Option<usize> {
        None // Infinite source
    }

    fn channels(&self) -> u16 {
        self.channels
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn total_duration(&self) -> Option<Duration> {
        None // Infinite duration
    }
}

pub struct AudioSystem {
    // In headless/silent mode these are None to avoid device init
    _output_stream: Option<OutputStream>,
    sink: Option<Sink>,
    apu_handle: Arc<Mutex<crate::apu::Apu>>,
    ring: Arc<Mutex<AudioRing>>,
    enabled: bool,
    volume: f32,
    sample_rate: u32,
    frame_sample_rem_acc: u32,
    frame_scratch: Vec<(i16, i16)>,
    buffer_frames: usize,
    chunk_frames: usize,
}

impl AudioSystem {
    pub fn new() -> Result<Self, String> {
        let (output_stream, stream_handle) = OutputStream::try_default()
            .map_err(|e| format!("Failed to create audio output stream: {}", e))?;

        let sink = Sink::try_new(&stream_handle)
            .map_err(|e| format!("Failed to create audio sink: {}", e))?;

        // Create a dummy APU for now - will be replaced when emulator starts
        let apu = Arc::new(Mutex::new(crate::apu::Apu::new()));
        let buffer_frames = std::env::var("AUDIO_BUFFER_FRAMES")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .filter(|&v| v > 0)
            .unwrap_or(DEFAULT_AUDIO_BUFFER_FRAMES);
        let chunk_frames = std::env::var("AUDIO_CHUNK_FRAMES")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .filter(|&v| v > 0)
            .unwrap_or(DEFAULT_AUDIO_CHUNK_FRAMES);
        let ring = Arc::new(Mutex::new(AudioRing::new(buffer_frames)));
        let sample_rate = 32000;

        Ok(Self {
            _output_stream: Some(output_stream),
            sink: Some(sink),
            apu_handle: apu,
            ring,
            enabled: true,
            volume: 0.7,
            sample_rate,
            frame_sample_rem_acc: 0,
            frame_scratch: Vec::new(),
            buffer_frames,
            chunk_frames,
        })
    }

    // Construct a silent audio system that does not touch the host audio device.
    // Used for HEADLESS runs and environments without audio.
    pub fn new_silent() -> Self {
        let apu = Arc::new(Mutex::new(crate::apu::Apu::new()));
        let buffer_frames = std::env::var("AUDIO_BUFFER_FRAMES")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .filter(|&v| v > 0)
            .unwrap_or(DEFAULT_AUDIO_BUFFER_FRAMES);
        let chunk_frames = std::env::var("AUDIO_CHUNK_FRAMES")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .filter(|&v| v > 0)
            .unwrap_or(DEFAULT_AUDIO_CHUNK_FRAMES);
        let ring = Arc::new(Mutex::new(AudioRing::new(buffer_frames)));
        let sample_rate = 32000;
        Self {
            _output_stream: None,
            sink: None,
            apu_handle: apu,
            ring,
            enabled: false,
            volume: 0.0,
            sample_rate,
            frame_sample_rem_acc: 0,
            frame_scratch: Vec::new(),
            buffer_frames,
            chunk_frames,
        }
    }

    pub fn set_apu(&mut self, apu: Arc<Mutex<crate::apu::Apu>>) {
        self.apu_handle = apu.clone();
        if let Ok(mut ring) = self.ring.lock() {
            ring.clear();
        }
        self.frame_sample_rem_acc = 0;

        if self.enabled {
            self.restart_audio();
        }
    }

    pub fn start(&mut self) {
        if self.enabled {
            self.restart_audio();
        }
    }

    #[allow(dead_code)]
    pub fn stop(&mut self) {
        if let Some(s) = &self.sink {
            s.stop();
        }
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if enabled {
            self.restart_audio();
        } else if let Some(s) = &self.sink {
            s.pause();
            if let Ok(mut ring) = self.ring.lock() {
                ring.clear();
            }
            self.frame_sample_rem_acc = 0;
        }
    }

    pub fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0.0, 1.0);
        if let Some(s) = &self.sink {
            s.set_volume(self.volume);
        }
    }

    pub fn get_volume(&self) -> f32 {
        self.volume
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn mix_frame_from_apu(&mut self, apu: &mut crate::apu::Apu) {
        if !self.enabled {
            return;
        }
        let sample_rate = apu.get_sample_rate();
        const FPS: u32 = 60;
        let base = (sample_rate / FPS) as usize;
        let rem = sample_rate % FPS;
        self.frame_sample_rem_acc = self.frame_sample_rem_acc.saturating_add(rem);
        let extra = if self.frame_sample_rem_acc >= FPS {
            self.frame_sample_rem_acc -= FPS;
            1
        } else {
            0
        };
        let frame_size = base + extra;
        if frame_size == 0 {
            return;
        }

        if let Ok(ring) = self.ring.lock() {
            if ring.len() >= self.buffer_frames.saturating_sub(frame_size) {
                return;
            }
        }

        if self.frame_scratch.len() < frame_size {
            self.frame_scratch.resize(frame_size, (0, 0));
        }
        let scratch = &mut self.frame_scratch[..frame_size];
        apu.generate_audio_samples(scratch);
        if let Ok(mut ring) = self.ring.lock() {
            ring.push_samples(scratch);
        }
        self.sample_rate = sample_rate;
    }

    fn restart_audio(&mut self) {
        // Ensure we have a stream/sink; create if missing
        if self._output_stream.is_none() || self.sink.is_none() {
            if let Ok((output_stream, stream_handle)) = OutputStream::try_default() {
                if let Ok(sink) = Sink::try_new(&stream_handle) {
                    self._output_stream = Some(output_stream);
                    self.sink = Some(sink);
                } else {
                    // Could not create sink; keep silent
                    return;
                }
            } else {
                // Could not create output stream; keep silent
                return;
            }
        }

        if let Some(s) = &self.sink {
            s.stop();
        }

        if let Some(s) = &self.sink {
            if let Ok(mut ring) = self.ring.lock() {
                ring.clear();
            }
            let audio_source =
                SnesAudioSource::new(self.ring.clone(), self.sample_rate, self.chunk_frames);
            s.append(audio_source);
            s.set_volume(self.volume);
            s.play();
        }
    }
}

impl Drop for AudioSystem {
    fn drop(&mut self) {
        if let Some(s) = &self.sink {
            s.stop();
        }
    }
}
