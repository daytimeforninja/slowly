use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use rodio::source::Source;
use rodio::{OutputStream, OutputStreamHandle, Sink};
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// Combined audio source with brown noise, binaural beats, and breathing drone
pub struct BreathingAudio {
    sample_rate: u32,
    sample_count: u64,
    brown_value: f32,
    rng: SmallRng,
    // Phase accumulators for smooth tone generation
    binaural_phase_l: f32,
    binaural_phase_r: f32,
    drone_phase_1: f32,
    drone_phase_2: f32,
    drone_phase_3: f32,  // Third oscillator for richness
    vibrato_phase: f32,
    channel: u16,
    // Simple reverb delay buffer
    reverb_buffer: Vec<f32>,
    reverb_index: usize,
    // Shared state
    phase: Arc<AtomicU8>,
    progress: Arc<AtomicU8>,
    brown_enabled: Arc<AtomicBool>,
    binaural_enabled: Arc<AtomicBool>,
    tones_enabled: Arc<AtomicBool>,
}

impl BreathingAudio {
    pub fn new(
        sample_rate: u32,
        phase: Arc<AtomicU8>,
        progress: Arc<AtomicU8>,
        brown_enabled: Arc<AtomicBool>,
        binaural_enabled: Arc<AtomicBool>,
        tones_enabled: Arc<AtomicBool>,
    ) -> Self {
        // Reverb delay ~100ms
        let reverb_size = (sample_rate as f32 * 0.1) as usize;
        Self {
            sample_rate,
            sample_count: 0,
            brown_value: 0.0,
            rng: SmallRng::from_entropy(),
            binaural_phase_l: 0.0,
            binaural_phase_r: 0.0,
            drone_phase_1: 0.0,
            drone_phase_2: 0.0,
            drone_phase_3: 0.0,
            vibrato_phase: 0.0,
            channel: 0,
            reverb_buffer: vec![0.0; reverb_size],
            reverb_index: 0,
            phase,
            progress,
            brown_enabled,
            binaural_enabled,
            tones_enabled,
        }
    }

    fn generate_brown_noise(&mut self) -> f32 {
        let white = self.rng.gen_range(-1.0..1.0);
        self.brown_value += white * 0.02;
        self.brown_value = self.brown_value.clamp(-1.0, 1.0);
        self.brown_value *= 0.999;
        // Reduced volume for brown noise
        self.brown_value * 0.12
    }

    fn generate_binaural(&mut self, is_left: bool) -> f32 {
        // Very low base frequency for deep relaxation
        // With 6 Hz difference for theta wave entrainment
        let base_freq = 65.0;  // Dropped 40%
        let beat_freq = 6.0;   // Theta waves (deep relaxation)

        let freq = if is_left { base_freq } else { base_freq + beat_freq };
        let phase_inc = freq / self.sample_rate as f32;

        let sample = if is_left {
            let s = (self.binaural_phase_l * 2.0 * std::f32::consts::PI).sin();
            self.binaural_phase_l = (self.binaural_phase_l + phase_inc) % 1.0;
            s
        } else {
            let s = (self.binaural_phase_r * 2.0 * std::f32::consts::PI).sin();
            self.binaural_phase_r = (self.binaural_phase_r + phase_inc) % 1.0;
            s
        };

        sample * 0.10
    }

    fn generate_breathing_drone(&mut self) -> f32 {
        let progress = self.progress.load(Ordering::Relaxed) as f32 / 100.0;
        let is_inhale = self.phase.load(Ordering::Relaxed) == 0;

        // Vibrato LFO - gentle pitch wobble
        let vibrato_rate = 4.5; // Hz
        let vibrato_inc = vibrato_rate / self.sample_rate as f32;
        self.vibrato_phase = (self.vibrato_phase + vibrato_inc) % 1.0;
        let vibrato = (self.vibrato_phase * 2.0 * std::f32::consts::PI).sin() * 0.015;

        // Deep sub-bass frequencies
        let (base_1, base_2, base_3) = if is_inhale {
            (35.0 + progress * 15.0, 28.0 + progress * 12.0, 42.0 + progress * 10.0)
        } else {
            (50.0 - progress * 15.0, 40.0 - progress * 12.0, 52.0 - progress * 10.0)
        };

        // Apply vibrato to frequencies
        let freq_1 = base_1 * (1.0 + vibrato);
        let freq_2 = base_2 * (1.0 + vibrato * 0.7);
        let freq_3 = base_3 * (1.0 - vibrato * 0.5);

        // Phase accumulators
        self.drone_phase_1 = (self.drone_phase_1 + freq_1 / self.sample_rate as f32) % 1.0;
        self.drone_phase_2 = (self.drone_phase_2 + freq_2 / self.sample_rate as f32) % 1.0;
        self.drone_phase_3 = (self.drone_phase_3 + freq_3 / self.sample_rate as f32) % 1.0;

        // Sub-bass sines with added harmonics for presence
        let sub_1 = (self.drone_phase_1 * 2.0 * std::f32::consts::PI).sin();
        let sub_2 = (self.drone_phase_2 * 2.0 * std::f32::consts::PI).sin();
        let sub_3 = (self.drone_phase_3 * 2.0 * std::f32::consts::PI).sin();

        // Add octave harmonics for audibility on smaller speakers
        let harm_1 = (self.drone_phase_1 * 2.0 * 2.0 * std::f32::consts::PI).sin() * 0.3;
        let harm_2 = (self.drone_phase_2 * 2.0 * 2.0 * std::f32::consts::PI).sin() * 0.2;

        // Blend - heavy on the sub
        let dry = (sub_1 * 1.0 + sub_2 * 0.8 + sub_3 * 0.5 + harm_1 + harm_2) / 2.5;

        // Simple reverb - mix in delayed signal
        let reverb_out = self.reverb_buffer[self.reverb_index];
        self.reverb_buffer[self.reverb_index] = dry * 0.5;
        self.reverb_index = (self.reverb_index + 1) % self.reverb_buffer.len();
        let wet = dry + reverb_out * 0.4;

        // Slow, gentle wub
        let wub_phase = progress * 0.5 * 2.0 * std::f32::consts::PI;
        let wub = 0.6 + 0.4 * (wub_phase.sin() * 0.5 + 0.5);

        // Breath envelope
        let envelope = (progress * std::f32::consts::PI).sin();

        wet * 0.35 * envelope * wub
    }
}

impl Iterator for BreathingAudio {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        let mut sample = 0.0;
        let is_left = self.channel == 0;

        if self.brown_enabled.load(Ordering::Relaxed) {
            sample += self.generate_brown_noise();
        }

        if self.binaural_enabled.load(Ordering::Relaxed) {
            sample += self.generate_binaural(is_left);
        }

        if self.tones_enabled.load(Ordering::Relaxed) {
            sample += self.generate_breathing_drone();
        }

        // Alternate channels for stereo
        self.channel = (self.channel + 1) % 2;

        // Soft clip
        Some(sample.clamp(-0.9, 0.9))
    }
}

impl Source for BreathingAudio {
    fn current_frame_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> u16 {
        2 // Stereo for binaural
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

/// Audio player with multiple sound options
pub struct AudioPlayer {
    _stream: OutputStream,
    _stream_handle: OutputStreamHandle,
    sink: Sink,
    phase: Arc<AtomicU8>,
    progress: Arc<AtomicU8>,
    pub brown_enabled: Arc<AtomicBool>,
    pub binaural_enabled: Arc<AtomicBool>,
    pub tones_enabled: Arc<AtomicBool>,
}

impl AudioPlayer {
    pub fn new() -> Option<Self> {
        let (stream, stream_handle) = OutputStream::try_default().ok()?;
        let sink = Sink::try_new(&stream_handle).ok()?;

        let phase = Arc::new(AtomicU8::new(0));
        let progress = Arc::new(AtomicU8::new(0));
        let brown_enabled = Arc::new(AtomicBool::new(true));
        let binaural_enabled = Arc::new(AtomicBool::new(false));
        let tones_enabled = Arc::new(AtomicBool::new(true));

        let audio = BreathingAudio::new(
            44100,
            phase.clone(),
            progress.clone(),
            brown_enabled.clone(),
            binaural_enabled.clone(),
            tones_enabled.clone(),
        );

        sink.append(audio);
        sink.pause();

        Some(Self {
            _stream: stream,
            _stream_handle: stream_handle,
            sink,
            phase,
            progress,
            brown_enabled,
            binaural_enabled,
            tones_enabled,
        })
    }

    pub fn play(&self) {
        self.sink.play();
    }

    pub fn pause(&self) {
        self.sink.pause();
    }

    pub fn set_volume(&self, volume: f32) {
        self.sink.set_volume(volume);
    }

    pub fn set_phase(&self, inhale: bool) {
        self.phase.store(if inhale { 0 } else { 1 }, Ordering::Relaxed);
    }

    pub fn set_progress(&self, progress: f32) {
        let p = (progress * 100.0).clamp(0.0, 100.0) as u8;
        self.progress.store(p, Ordering::Relaxed);
    }

    pub fn set_brown_enabled(&self, enabled: bool) {
        self.brown_enabled.store(enabled, Ordering::Relaxed);
    }

    pub fn set_binaural_enabled(&self, enabled: bool) {
        self.binaural_enabled.store(enabled, Ordering::Relaxed);
    }

    pub fn set_tones_enabled(&self, enabled: bool) {
        self.tones_enabled.store(enabled, Ordering::Relaxed);
    }
}
