use std::time::{Duration, Instant};

use cosmic::app::Core;
use cosmic::iced::keyboard::{self, Key, key::Named};
use cosmic::iced::Length;
use cosmic::widget::{self, button, container, settings, text};
use cosmic::{Apply, Application, Element, Task};

use diaframe::{AudioPlayer, SessionDuration};
use crate::config::{BreathingConfig, BreathingPreset, PersistentSettings};
use crate::widgets::BreathingCircle;


/// Breathing phases
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Inhale,
    Exhale,
}

impl Phase {
    /// Get the next phase in the breathing cycle
    fn next(self) -> Phase {
        match self {
            Phase::Inhale => Phase::Exhale,
            Phase::Exhale => Phase::Inhale,
        }
    }

    /// Get the duration of this phase based on config
    fn duration(self, config: &BreathingConfig) -> f32 {
        match self {
            Phase::Inhale => config.inhale_duration,
            Phase::Exhale => config.exhale_duration,
        }
    }
}

/// Views in the application
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    Breathing,
    Settings,
}

/// Application messages
#[derive(Debug, Clone)]
pub enum Message {
    Tick(Instant),
    ToggleRunning,
    OpenSettings,
    CloseSettings,
    SetInhaleDuration(f32),
    SetExhaleDuration(f32),
    ToggleBrownNoise(bool),
    ToggleBinaural(bool),
    ToggleTones(bool),
    SetVolume(f32),
    SetPreset(BreathingPreset),
    SetSessionDuration(SessionDuration),
    // Window controls
    Minimize,
    Maximize,
    CloseWindow,
    // Keyboard
    KeyPressed(Key),
    ToggleFullscreen,
}

/// Main application state
pub struct App {
    core: Core,
    config: BreathingConfig,
    is_running: bool,
    current_phase: Phase,
    phase_progress: f32,
    phase_start: Instant,
    current_view: View,
    audio: Option<AudioPlayer>,
    brown_enabled: bool,
    binaural_enabled: bool,
    tones_enabled: bool,
    volume: f32,
    // UI state
    fullscreen: bool,
    // Phase transition flash (0.0 to 1.0, decays over time)
    transition_flash: f32,
    // Startup fade-in (0.0 to 1.0, increases when starting)
    startup_fade: f32,
    // Ambient particles
    particles: Vec<Particle>,
    // Breath count (for fading labels)
    breath_count: u32,
    // Session timer
    session_duration: SessionDuration,
    session_elapsed: f32,
    // Statistics
    stats: diaframe::PracticeStats,
}

/// A floating ambient particle
#[derive(Clone)]
pub struct Particle {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub size: f32,
    pub opacity: f32,
    pub hue: f32,
}

impl Application for App {
    type Executor = cosmic::executor::Default;
    type Flags = ();
    type Message = Message;

    const APP_ID: &'static str = "io.github.daytimeforninja.Slowly";

    fn core(&self) -> &Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    fn init(mut core: Core, _flags: Self::Flags) -> (Self, Task<cosmic::Action<Self::Message>>) {
        core.window.show_close = false;
        core.window.show_maximize = false;
        core.window.show_minimize = false;
        core.window.show_headerbar = true;

        let saved = PersistentSettings::load();

        let audio = AudioPlayer::new();
        if let Some(ref a) = audio {
            a.set_volume(saved.audio.volume);
            a.set_brown_enabled(saved.audio.brown_noise);
            a.set_binaural_enabled(saved.audio.binaural);
            a.set_tones_enabled(saved.audio.tones);
        }

        let mut stats = saved.stats.clone();
        stats.check_today();

        let app = Self {
            core,
            config: BreathingConfig {
                inhale_duration: saved.inhale_duration,
                exhale_duration: saved.exhale_duration,
                preset: saved.preset,
            },
            is_running: false,
            current_phase: Phase::Inhale,
            phase_progress: 0.0,
            phase_start: Instant::now(),
            current_view: View::Breathing,
            audio,
            brown_enabled: saved.audio.brown_noise,
            binaural_enabled: saved.audio.binaural,
            tones_enabled: saved.audio.tones,
            volume: saved.audio.volume,
            fullscreen: false,
            transition_flash: 0.0,
            startup_fade: 1.0,
            particles: Self::create_particles(30),
            breath_count: 0,
            session_duration: saved.session_duration,
            session_elapsed: 0.0,
            stats,
        };
        (app, Task::none())
    }

    fn header_start(&self) -> Vec<Element<'_, Self::Message>> {
        vec![text::heading("Slowly").into()]
    }

    fn header_end(&self) -> Vec<Element<'_, Self::Message>> {
        let nav_button = if self.current_view == View::Settings {
            button::text("← Back")
                .on_press(Message::CloseSettings)
        } else {
            button::text("⚙ Settings")
                .on_press(Message::OpenSettings)
        };

        let minimize = button::text("−")
            .on_press(Message::Minimize);
        let maximize = button::text("□")
            .on_press(Message::Maximize);
        let close = button::text("×")
            .on_press(Message::CloseWindow)
            .class(cosmic::theme::Button::Destructive);

        vec![
            nav_button.into(),
            widget::horizontal_space().width(Length::Fixed(16.0)).into(),
            minimize.into(),
            maximize.into(),
            close.into(),
        ]
    }

    fn view(&self) -> Element<'_, Self::Message> {
        match self.current_view {
            View::Breathing => self.view_breathing(),
            View::Settings => self.view_settings(),
        }
    }

    fn update(&mut self, message: Self::Message) -> Task<cosmic::Action<Self::Message>> {
        match message {
            Message::Tick(now) => {
                // Decay transition flash
                if self.transition_flash > 0.0 {
                    self.transition_flash = (self.transition_flash - 0.05).max(0.0);
                }

                // Fade in on startup (~500ms at 60fps)
                if self.startup_fade < 1.0 {
                    self.startup_fade = (self.startup_fade + 0.033).min(1.0);
                }

                // Update ambient particles
                let breathing_scale = match self.current_phase {
                    Phase::Inhale => self.phase_progress,
                    Phase::Exhale => 1.0 - self.phase_progress,
                };
                self.update_particles(breathing_scale);

                if self.is_running {
                    // Track session time (~16ms per tick)
                    self.session_elapsed += 0.016;

                    // Check if session is complete
                    if let Some(duration) = self.session_duration.seconds() {
                        if self.session_elapsed >= duration {
                            self.is_running = false;
                            if let Some(ref audio) = self.audio {
                                audio.pause();
                            }
                            self.stats.record_session(self.session_elapsed);
                            self.save_settings();
                            return Task::none();
                        }
                    }

                    let phase_duration = self.current_phase.duration(&self.config);
                    let elapsed = now.duration_since(self.phase_start).as_secs_f32();

                    self.phase_progress = (elapsed / phase_duration).min(1.0);

                    // Update audio with current state
                    if let Some(ref audio) = self.audio {
                        audio.set_state(self.current_phase == Phase::Inhale, self.phase_progress);
                    }

                    if self.phase_progress >= 1.0 {
                        if self.current_phase == Phase::Exhale {
                            self.breath_count += 1;
                        }
                        self.current_phase = self.current_phase.next();
                        self.phase_start = now;
                        self.phase_progress = 0.0;
                        self.transition_flash = 1.0;
                    }
                }
            }
            Message::ToggleRunning => {
                self.is_running = !self.is_running;
                if self.is_running {
                    self.phase_start = Instant::now();
                    self.phase_progress = 0.0;
                    self.startup_fade = 0.0;
                    self.session_elapsed = 0.0;
                    self.current_phase = Phase::Inhale;
                    self.breath_count = 0;
                    if let Some(ref audio) = self.audio {
                        audio.set_state(true, 0.0);
                        audio.play();
                    }
                } else {
                    if let Some(ref audio) = self.audio {
                        audio.pause();
                    }
                    if self.session_elapsed >= 30.0 {
                        self.stats.record_session(self.session_elapsed);
                        self.save_settings();
                    }
                }
            }
            Message::OpenSettings => {
                self.current_view = View::Settings;
                self.is_running = false;
                if let Some(ref audio) = self.audio {
                    audio.pause();
                }
            }
            Message::CloseSettings => {
                self.current_view = View::Breathing;
            }
            Message::SetInhaleDuration(value) => {
                self.config.inhale_duration = value;
                self.config.preset = BreathingPreset::Custom;
                self.save_settings();
            }
            Message::SetExhaleDuration(value) => {
                self.config.exhale_duration = value;
                self.config.preset = BreathingPreset::Custom;
                self.save_settings();
            }
            Message::SetPreset(preset) => {
                self.config.preset = preset;
                self.config.inhale_duration = preset.inhale_duration();
                self.config.exhale_duration = preset.exhale_duration();
                self.save_settings();
            }
            Message::SetSessionDuration(duration) => {
                self.session_duration = duration;
                self.save_settings();
            }
            Message::ToggleBrownNoise(enabled) => {
                self.brown_enabled = enabled;
                if let Some(ref audio) = self.audio {
                    audio.set_brown_enabled(enabled);
                }
                self.save_settings();
            }
            Message::ToggleBinaural(enabled) => {
                self.binaural_enabled = enabled;
                if let Some(ref audio) = self.audio {
                    audio.set_binaural_enabled(enabled);
                }
                self.save_settings();
            }
            Message::ToggleTones(enabled) => {
                self.tones_enabled = enabled;
                if let Some(ref audio) = self.audio {
                    audio.set_tones_enabled(enabled);
                }
                self.save_settings();
            }
            Message::SetVolume(value) => {
                self.volume = value;
                if let Some(ref audio) = self.audio {
                    audio.set_volume(value);
                }
                self.save_settings();
            }
            Message::Minimize => {
                if let Some(id) = self.core.main_window_id() {
                    return cosmic::command::minimize(id);
                }
            }
            Message::Maximize => {
                if let Some(id) = self.core.main_window_id() {
                    return cosmic::command::toggle_maximize(id);
                }
            }
            Message::CloseWindow => {
                std::process::exit(0);
            }
            Message::KeyPressed(key) => {
                match key {
                    Key::Named(Named::Space) => {
                        if self.current_view == View::Breathing {
                            return self.update(Message::ToggleRunning);
                        }
                    }
                    Key::Named(Named::Escape) => {
                        if self.current_view == View::Settings {
                            return self.update(Message::CloseSettings);
                        }
                    }
                    Key::Named(Named::F11) => {
                        return self.update(Message::ToggleFullscreen);
                    }
                    _ => {}
                }
            }
            Message::ToggleFullscreen => {
                self.fullscreen = !self.fullscreen;
                self.core.window.show_headerbar = !self.fullscreen;
            }
        }
        Task::none()
    }

    fn subscription(&self) -> cosmic::iced::Subscription<Self::Message> {
        let tick = if self.is_running {
            cosmic::iced::time::every(Duration::from_millis(16)).map(Message::Tick)
        } else {
            cosmic::iced::Subscription::none()
        };

        let keys = keyboard::on_key_press(|key, _modifiers| {
            Some(Message::KeyPressed(key))
        });

        cosmic::iced::Subscription::batch([tick, keys])
    }
}

impl App {
    fn view_breathing(&self) -> Element<'_, Message> {
        let session_progress = self.session_duration.seconds()
            .map(|total| (self.session_elapsed / total).min(1.0));

        // Smooth fade over 6 breaths: interpolate within the current breath
        let text_opacity = if !self.is_running {
            1.0
        } else {
            let fractional = match self.current_phase {
                Phase::Inhale => self.phase_progress * 0.5,
                Phase::Exhale => 0.5 + self.phase_progress * 0.5,
            };
            (1.0 - (self.breath_count as f32 + fractional) / 6.0).clamp(0.0, 1.0)
        };

        let visualization = BreathingCircle::new(self.current_phase, self.phase_progress, self.is_running)
            .with_flash(self.transition_flash)
            .with_opacity(self.startup_fade)
            .with_particles(self.particles.clone())
            .with_session_progress(session_progress)
            .with_text_opacity(text_opacity)
            .view();

        container(visualization)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn view_settings(&self) -> Element<'_, Message> {
        let cycle_duration = self.config.cycle_duration();
        let breaths_per_min = 60.0 / cycle_duration;

        // Session duration buttons
        let duration_row = widget::row::with_capacity(6)
            .push(
                button::standard("5m")
                    .on_press(Message::SetSessionDuration(SessionDuration::Minutes5))
                    .class(if self.session_duration == SessionDuration::Minutes5 {
                        cosmic::theme::Button::Suggested
                    } else {
                        cosmic::theme::Button::Standard
                    })
            )
            .push(
                button::standard("10m")
                    .on_press(Message::SetSessionDuration(SessionDuration::Minutes10))
                    .class(if self.session_duration == SessionDuration::Minutes10 {
                        cosmic::theme::Button::Suggested
                    } else {
                        cosmic::theme::Button::Standard
                    })
            )
            .push(
                button::standard("15m")
                    .on_press(Message::SetSessionDuration(SessionDuration::Minutes15))
                    .class(if self.session_duration == SessionDuration::Minutes15 {
                        cosmic::theme::Button::Suggested
                    } else {
                        cosmic::theme::Button::Standard
                    })
            )
            .push(
                button::standard("20m")
                    .on_press(Message::SetSessionDuration(SessionDuration::Minutes20))
                    .class(if self.session_duration == SessionDuration::Minutes20 {
                        cosmic::theme::Button::Suggested
                    } else {
                        cosmic::theme::Button::Standard
                    })
            )
            .push(
                button::standard("30m")
                    .on_press(Message::SetSessionDuration(SessionDuration::Minutes30))
                    .class(if self.session_duration == SessionDuration::Minutes30 {
                        cosmic::theme::Button::Suggested
                    } else {
                        cosmic::theme::Button::Standard
                    })
            )
            .push(
                button::standard("∞")
                    .on_press(Message::SetSessionDuration(SessionDuration::Infinite))
                    .class(if self.session_duration == SessionDuration::Infinite {
                        cosmic::theme::Button::Suggested
                    } else {
                        cosmic::theme::Button::Standard
                    })
            )
            .spacing(6);

        // Preset buttons
        let presets_row = widget::row::with_capacity(4)
            .push(
                button::standard("Coherence")
                    .on_press(Message::SetPreset(BreathingPreset::Coherence))
                    .class(if self.config.preset == BreathingPreset::Coherence {
                        cosmic::theme::Button::Suggested
                    } else {
                        cosmic::theme::Button::Standard
                    })
            )
            .push(
                button::standard("Relax")
                    .on_press(Message::SetPreset(BreathingPreset::Relaxation))
                    .class(if self.config.preset == BreathingPreset::Relaxation {
                        cosmic::theme::Button::Suggested
                    } else {
                        cosmic::theme::Button::Standard
                    })
            )
            .push(
                button::standard("Energy")
                    .on_press(Message::SetPreset(BreathingPreset::Energizing))
                    .class(if self.config.preset == BreathingPreset::Energizing {
                        cosmic::theme::Button::Suggested
                    } else {
                        cosmic::theme::Button::Standard
                    })
            )
            .push(
                button::standard("Square")
                    .on_press(Message::SetPreset(BreathingPreset::Square))
                    .class(if self.config.preset == BreathingPreset::Square {
                        cosmic::theme::Button::Suggested
                    } else {
                        cosmic::theme::Button::Standard
                    })
            )
            .spacing(8);

        // Practice section
        let practice_section = settings::section()
            .title("Practice")
            .add(
                settings::item::builder("Session Length")
                    .description("Auto-stop after duration")
                    .control(duration_row)
            );

        // Timing section
        let timing_section = settings::section()
            .title("Timing")
            .add(
                settings::item::builder("Preset")
                    .description(self.config.preset.description())
                    .control(presets_row)
            )
            .add(
                settings::item::builder("Inhale")
                    .description(format!("{:.1} seconds", self.config.inhale_duration))
                    .control(
                        widget::slider(1.0..=15.0, self.config.inhale_duration, Message::SetInhaleDuration)
                            .step(0.5)
                            .width(Length::Fixed(200.0))
                    )
            )
            .add(
                settings::item::builder("Exhale")
                    .description(format!("{:.1} seconds", self.config.exhale_duration))
                    .control(
                        widget::slider(1.0..=15.0, self.config.exhale_duration, Message::SetExhaleDuration)
                            .step(0.5)
                            .width(Length::Fixed(200.0))
                    )
            )
            .add(
                settings::item::builder("Breath Rate")
                    .control(text::body(format!("{:.1} breaths/min", breaths_per_min)))
            );

        // Sound section
        let sound_section = settings::section()
            .title("Sound")
            .add(
                settings::item::builder("Brown Noise")
                    .description("Gentle background noise for focus")
                    .toggler(self.brown_enabled, Message::ToggleBrownNoise)
            )
            .add(
                settings::item::builder("Binaural Beats")
                    .description("Theta waves (6 Hz) for deep relaxation")
                    .toggler(self.binaural_enabled, Message::ToggleBinaural)
            )
            .add(
                settings::item::builder("Breathing Tones")
                    .description("Sub-bass drone following breath")
                    .toggler(self.tones_enabled, Message::ToggleTones)
            )
            .add(
                settings::item::builder("Volume")
                    .control(
                        widget::row::with_capacity(2)
                            .push(
                                widget::slider(0.0..=1.0, self.volume, Message::SetVolume)
                                    .step(0.05)
                                    .width(Length::Fixed(160.0))
                            )
                            .push(text::body(format!("{:.0}%", self.volume * 100.0)))
                            .spacing(12)
                            .align_y(cosmic::iced::Alignment::Center)
                    )
            );

        // Statistics section
        let stats_section = settings::section()
            .title("Statistics")
            .add(
                settings::item::builder("Today")
                    .control(text::body(diaframe::format_duration(self.stats.today_practice_seconds)))
            )
            .add(
                settings::item::builder("Total Practice")
                    .control(text::body(diaframe::format_duration(self.stats.total_practice_seconds)))
            )
            .add(
                settings::item::builder("Sessions")
                    .control(text::body(format!("{}", self.stats.sessions_completed)))
            );

        let content = settings::view_column(vec![
            practice_section.into(),
            timing_section.into(),
            sound_section.into(),
            stats_section.into(),
        ]);

        widget::scrollable(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .apply(container)
            .padding([0, 24])
            .into()
    }

    fn create_particles(count: usize) -> Vec<Particle> {
        use std::f32::consts::PI;
        (0..count)
            .map(|i| {
                let angle = (i as f32 / count as f32) * 2.0 * PI;
                let distance = 180.0 + (i as f32 * 7.0) % 120.0;
                Particle {
                    x: angle.cos() * distance,
                    y: angle.sin() * distance,
                    vx: ((i * 17) % 100) as f32 / 100.0 - 0.5,
                    vy: ((i * 31) % 100) as f32 / 100.0 - 0.5,
                    size: 2.0 + (i % 4) as f32,
                    opacity: 0.2 + (i % 5) as f32 * 0.1,
                    hue: (i as f32 / count as f32),
                }
            })
            .collect()
    }

    fn update_particles(&mut self, breathing_scale: f32) {
        for p in &mut self.particles {
            p.x += p.vx * 0.5;
            p.y += p.vy * 0.5;

            let dist = (p.x * p.x + p.y * p.y).sqrt();
            if dist > 0.0 {
                let breath_force = (breathing_scale - 0.5) * 0.3;
                p.x += (p.x / dist) * breath_force;
                p.y += (p.y / dist) * breath_force;
            }

            let max_dist = 350.0;
            let current_dist = (p.x * p.x + p.y * p.y).sqrt();
            if current_dist > max_dist {
                let angle = p.y.atan2(p.x);
                let new_dist = 150.0 + (current_dist % 50.0);
                p.x = angle.cos() * new_dist;
                p.y = angle.sin() * new_dist;
            }

            p.hue = (p.hue + 0.001) % 1.0;
        }
    }

    fn save_settings(&self) {
        let settings = PersistentSettings {
            preset: self.config.preset,
            inhale_duration: self.config.inhale_duration,
            exhale_duration: self.config.exhale_duration,
            session_duration: self.session_duration,
            audio: diaframe::AudioSettings {
                brown_noise: self.brown_enabled,
                binaural: self.binaural_enabled,
                tones: self.tones_enabled,
                volume: self.volume,
            },
            stats: self.stats.clone(),
        };
        settings.save();
    }
}
