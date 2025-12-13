#![cfg_attr(not(feature = "dev"), allow(dead_code))]
use rodio::{OutputStream, Sink, Source};
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub struct SnesAudioSource {
    apu: Arc<Mutex<crate::apu::Apu>>,
    sample_rate: u32,
    channels: u16,
    current_frame: Vec<(i16, i16)>,
    // Interleaved sample cursor (L,R,L,R...) within current_frame.
    // current_frame holds stereo frames; cursor counts i16 samples.
    sample_cursor: usize,
    // Fractional remainder accumulator for distributing non-integer samples per 60Hz frame.
    frame_sample_rem_acc: u32,
}

impl SnesAudioSource {
    pub fn new(apu: Arc<Mutex<crate::apu::Apu>>) -> Self {
        let sample_rate = 32000; // SNES APU native sample rate
        let channels = 2; // Stereo

        Self {
            apu,
            sample_rate,
            channels,
            current_frame: Vec::new(),
            sample_cursor: 0,
            frame_sample_rem_acc: 0,
        }
    }

    fn generate_audio_frame(&mut self) {
        // Produce ~60Hz worth of samples. 32000/60 = 533.333..., so distribute the remainder.
        const FPS: u32 = 60;
        let base = (self.sample_rate / FPS) as usize;
        let rem = self.sample_rate % FPS;
        self.frame_sample_rem_acc = self.frame_sample_rem_acc.saturating_add(rem);
        let extra = if self.frame_sample_rem_acc >= FPS {
            self.frame_sample_rem_acc -= FPS;
            1
        } else {
            0
        };
        let frame_size = base + extra;

        // Generate a frame of audio samples
        let mut samples = vec![(0i16, 0i16); frame_size];

        if let Ok(mut apu) = self.apu.lock() {
            apu.generate_audio_samples(&mut samples);
        }

        self.current_frame = samples;
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
    enabled: bool,
    volume: f32,
}

impl AudioSystem {
    pub fn new() -> Result<Self, String> {
        let (output_stream, stream_handle) = OutputStream::try_default()
            .map_err(|e| format!("Failed to create audio output stream: {}", e))?;

        let sink = Sink::try_new(&stream_handle)
            .map_err(|e| format!("Failed to create audio sink: {}", e))?;

        // Create a dummy APU for now - will be replaced when emulator starts
        let apu = Arc::new(Mutex::new(crate::apu::Apu::new()));

        Ok(Self {
            _output_stream: Some(output_stream),
            sink: Some(sink),
            apu_handle: apu,
            enabled: true,
            volume: 0.7,
        })
    }

    // Construct a silent audio system that does not touch the host audio device.
    // Used for HEADLESS runs and environments without audio.
    pub fn new_silent() -> Self {
        let apu = Arc::new(Mutex::new(crate::apu::Apu::new()));
        Self {
            _output_stream: None,
            sink: None,
            apu_handle: apu,
            enabled: false,
            volume: 0.0,
        }
    }

    pub fn set_apu(&mut self, apu: Arc<Mutex<crate::apu::Apu>>) {
        self.apu_handle = apu.clone();

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
            let audio_source = SnesAudioSource::new(self.apu_handle.clone());
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
