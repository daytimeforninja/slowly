use serde::{Deserialize, Serialize};

/// Breathing pattern presets
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum BreathingPreset {
    #[default]
    Coherence,  // 5s in, 5s out - HRV optimal
    Relaxation, // 4s in, 8s out - calming
    Energizing, // 4s in, 2s out - stimulating
    Square,     // 4s in, 4s out - balanced (simplified box)
    Custom,     // User-defined
}

impl BreathingPreset {
    /// Get the inhale duration for this preset
    pub fn inhale_duration(self) -> f32 {
        match self {
            Self::Coherence => 5.0,
            Self::Relaxation => 4.0,
            Self::Energizing => 4.0,
            Self::Square => 4.0,
            Self::Custom => 5.0,
        }
    }

    /// Get the exhale duration for this preset
    pub fn exhale_duration(self) -> f32 {
        match self {
            Self::Coherence => 5.0,
            Self::Relaxation => 8.0,
            Self::Energizing => 2.0,
            Self::Square => 4.0,
            Self::Custom => 5.0,
        }
    }

    /// Get a description of this preset
    pub fn description(self) -> &'static str {
        match self {
            Self::Coherence => "5s in, 5s out • HRV optimal",
            Self::Relaxation => "4s in, 8s out • Calming",
            Self::Energizing => "4s in, 2s out • Stimulating",
            Self::Square => "4s in, 4s out • Balanced",
            Self::Custom => "Custom timing",
        }
    }
}

/// Configuration for breathing pattern timing
#[derive(Debug, Clone, Copy)]
pub struct BreathingConfig {
    /// Duration of inhale phase in seconds
    pub inhale_duration: f32,
    /// Duration of exhale phase in seconds
    pub exhale_duration: f32,
    /// Current preset (Custom if manually adjusted)
    pub preset: BreathingPreset,
}

impl Default for BreathingConfig {
    fn default() -> Self {
        Self {
            inhale_duration: 5.0,
            exhale_duration: 5.0,
            preset: BreathingPreset::Coherence,
        }
    }
}

impl BreathingConfig {
    /// Get the total duration of one complete breath cycle
    pub fn cycle_duration(&self) -> f32 {
        self.inhale_duration + self.exhale_duration
    }
}

/// Persistent settings saved to disk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistentSettings {
    pub preset: BreathingPreset,
    pub inhale_duration: f32,
    pub exhale_duration: f32,
    pub session_duration: diaframe::SessionDuration,
    #[serde(flatten)]
    pub audio: diaframe::AudioSettings,
    #[serde(flatten)]
    pub stats: diaframe::PracticeStats,
}

impl Default for PersistentSettings {
    fn default() -> Self {
        Self {
            preset: BreathingPreset::default(),
            inhale_duration: 5.0,
            exhale_duration: 5.0,
            session_duration: diaframe::SessionDuration::default(),
            audio: diaframe::AudioSettings::default(),
            stats: diaframe::PracticeStats::default(),
        }
    }
}

impl PersistentSettings {
    /// Load settings from disk, or return defaults
    pub fn load() -> Self {
        diaframe::config_path("com", "slowly", "slowly")
            .map(|path| diaframe::load_json::<Self>(&path))
            .unwrap_or_default()
    }

    /// Save settings to disk
    pub fn save(&self) {
        if let Some(path) = diaframe::config_path("com", "slowly", "slowly") {
            diaframe::save_json(&path, self);
        }
    }
}
