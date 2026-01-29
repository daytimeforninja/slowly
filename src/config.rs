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

/// Session duration options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum SessionDuration {
    Minutes5,
    Minutes10,
    #[default]
    Minutes15,
    Minutes20,
    Minutes30,
    Infinite,
}

impl SessionDuration {
    /// Get duration in seconds (None for infinite)
    pub fn seconds(self) -> Option<f32> {
        match self {
            Self::Minutes5 => Some(5.0 * 60.0),
            Self::Minutes10 => Some(10.0 * 60.0),
            Self::Minutes15 => Some(15.0 * 60.0),
            Self::Minutes20 => Some(20.0 * 60.0),
            Self::Minutes30 => Some(30.0 * 60.0),
            Self::Infinite => None,
        }
    }

    /// Display label
    pub fn label(self) -> &'static str {
        match self {
            Self::Minutes5 => "5 min",
            Self::Minutes10 => "10 min",
            Self::Minutes15 => "15 min",
            Self::Minutes20 => "20 min",
            Self::Minutes30 => "30 min",
            Self::Infinite => "∞",
        }
    }
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
    pub session_duration: SessionDuration,
    pub brown_noise: bool,
    pub binaural: bool,
    pub tones: bool,
    pub volume: f32,
    // Statistics
    #[serde(default)]
    pub total_practice_seconds: f32,
    #[serde(default)]
    pub sessions_completed: u32,
    #[serde(default)]
    pub last_session_date: Option<String>,
    #[serde(default)]
    pub today_practice_seconds: f32,
}

impl Default for PersistentSettings {
    fn default() -> Self {
        Self {
            preset: BreathingPreset::default(),
            inhale_duration: 5.0,
            exhale_duration: 5.0,
            session_duration: SessionDuration::default(),
            brown_noise: true,
            binaural: false,
            tones: true,
            volume: 0.5,
            total_practice_seconds: 0.0,
            sessions_completed: 0,
            last_session_date: None,
            today_practice_seconds: 0.0,
        }
    }
}

impl PersistentSettings {
    /// Get the config file path
    fn config_path() -> Option<std::path::PathBuf> {
        directories::ProjectDirs::from("com", "slowly", "slowly")
            .map(|dirs| dirs.config_dir().join("settings.json"))
    }

    /// Load settings from disk, or return defaults
    pub fn load() -> Self {
        Self::config_path()
            .and_then(|path| std::fs::read_to_string(&path).ok())
            .and_then(|content| serde_json::from_str(&content).ok())
            .unwrap_or_default()
    }

    /// Save settings to disk
    pub fn save(&self) {
        if let Some(path) = Self::config_path() {
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            if let Ok(content) = serde_json::to_string_pretty(self) {
                let _ = std::fs::write(&path, content);
            }
        }
    }
}
