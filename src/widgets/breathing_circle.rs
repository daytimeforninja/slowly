use cosmic::iced::mouse;
use cosmic::iced::widget::canvas::{self, Canvas, Event, Geometry, Path, Text};
use cosmic::iced::{Color, Point, Rectangle};
use cosmic::Element;
use cosmic::Theme;

use crate::app::{Message, Particle, Phase};

/// A custom widget that renders an animated breathing circle with background
pub struct BreathingCircle {
    phase: Phase,
    progress: f32,
    is_running: bool,
    flash: f32,
    opacity: f32,
    particles: Vec<Particle>,
    session_progress: Option<f32>,
}

impl BreathingCircle {
    // Radius is now calculated as a fraction of the window size
    const MIN_SCALE: f32 = 0.25; // 25% of window at smallest
    const MAX_SCALE: f32 = 0.80; // 80% of window at largest

    pub fn new(phase: Phase, progress: f32, is_running: bool) -> Self {
        Self { phase, progress, is_running, flash: 0.0, opacity: 1.0, particles: Vec::new(), session_progress: None }
    }

    pub fn with_flash(mut self, flash: f32) -> Self {
        self.flash = flash;
        self
    }

    pub fn with_opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity;
        self
    }

    pub fn with_particles(mut self, particles: Vec<Particle>) -> Self {
        self.particles = particles;
        self
    }

    pub fn with_session_progress(mut self, progress: Option<f32>) -> Self {
        self.session_progress = progress;
        self
    }

    /// Ease-in-out function for smooth animation
    fn ease_in_out(t: f32) -> f32 {
        if t < 0.5 {
            2.0 * t * t
        } else {
            1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
        }
    }

    /// Calculate the breathing scale (0.0 to 1.0)
    fn breathing_scale(&self) -> f32 {
        match self.phase {
            Phase::Inhale => Self::ease_in_out(self.progress),
            Phase::Exhale => 1.0 - Self::ease_in_out(self.progress),
        }
    }

    /// Get the display text for current phase
    fn phase_text(&self) -> &'static str {
        if !self.is_running {
            "Begin"
        } else {
            match self.phase {
                Phase::Inhale => "In",
                Phase::Exhale => "Out",
            }
        }
    }

    /// Convert to an Element
    pub fn view(self) -> Element<'static, Message> {
        Canvas::new(BreathingCircleProgram {
            breathing_scale: self.breathing_scale(),
            phase_text: self.phase_text(),
            is_running: self.is_running,
            flash: self.flash,
            opacity: self.opacity,
            particles: self.particles,
            session_progress: self.session_progress,
        })
        .width(cosmic::iced::Length::Fill)
        .height(cosmic::iced::Length::Fill)
        .into()
    }
}

struct BreathingCircleProgram {
    breathing_scale: f32, // 0.0 (exhaled) to 1.0 (inhaled)
    phase_text: &'static str,
    is_running: bool,
    flash: f32,
    opacity: f32,
    particles: Vec<Particle>,
    session_progress: Option<f32>,
}

impl BreathingCircleProgram {
    /// Apply opacity to a color
    fn with_opacity(color: Color, opacity: f32) -> Color {
        Color::from_rgba(color.r, color.g, color.b, color.a * opacity)
    }

    /// Interpolate between two colors
    fn lerp_color(from: Color, to: Color, t: f32) -> Color {
        Color::from_rgba(
            from.r + (to.r - from.r) * t,
            from.g + (to.g - from.g) * t,
            from.b + (to.b - from.b) * t,
            from.a + (to.a - from.a) * t,
        )
    }

    /// Convert hue (0-1) to RGB color
    fn hue_to_rgb(hue: f32, alpha: f32) -> Color {
        let h = hue * 6.0;
        let c = 1.0;
        let x = 1.0 - (h % 2.0 - 1.0).abs();

        let (r, g, b) = if h < 1.0 {
            (c, x, 0.0)
        } else if h < 2.0 {
            (x, c, 0.0)
        } else if h < 3.0 {
            (0.0, c, x)
        } else if h < 4.0 {
            (0.0, x, c)
        } else if h < 5.0 {
            (x, 0.0, c)
        } else {
            (c, 0.0, x)
        };

        Color::from_rgba(r, g, b, alpha)
    }
}

impl canvas::Program<Message, Theme> for BreathingCircleProgram {
    type State = ();

    fn update(
        &self,
        _state: &mut Self::State,
        event: Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> (canvas::event::Status, Option<Message>) {
        if let Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) = event {
            if let Some(pos) = cursor.position_in(bounds) {
                let center = Point::new(bounds.width / 2.0, bounds.height / 2.0);
                let dx = pos.x - center.x;
                let dy = pos.y - center.y;
                let distance = (dx * dx + dy * dy).sqrt();

                // Click anywhere in the general circle area
                let half_size = bounds.width.min(bounds.height) / 2.0;
                let max_radius = half_size * BreathingCircle::MAX_SCALE;
                if distance <= max_radius + 30.0 {
                    return (canvas::event::Status::Captured, Some(Message::ToggleRunning));
                }
            }
        }
        (canvas::event::Status::Ignored, None)
    }

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &cosmic::Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry<cosmic::Renderer>> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let center = Point::new(bounds.width / 2.0, bounds.height / 2.0);

        // Compute radius from window size
        let half_size = bounds.width.min(bounds.height) / 2.0;
        let min_radius = half_size * BreathingCircle::MIN_SCALE;
        let max_radius = half_size * BreathingCircle::MAX_SCALE;
        let radius = min_radius + (max_radius - min_radius) * self.breathing_scale;

        // Draw gradient background
        let bg_dark = Color::from_rgb(0.08, 0.06, 0.10);

        // Fill background
        let bg = Path::rectangle(Point::new(0.0, 0.0), bounds.size());
        frame.fill(&bg, bg_dark);

        // Add subtle radial gradient effect with concentric circles
        for i in (1..=8).rev() {
            let gradient_radius = half_size * (i as f32 / 8.0);
            let alpha = 0.03 * (1.0 - (i as f32 / 8.0));
            let gradient_color = Color::from_rgba(0.25, 0.15, 0.30, alpha);
            let gradient_circle = Path::circle(center, gradient_radius);
            frame.fill(&gradient_circle, gradient_color);
        }

        // Draw ambient particles (scale positions to window)
        let particle_scale = half_size / 300.0;
        for p in &self.particles {
            let particle_pos = Point::new(center.x + p.x * particle_scale, center.y + p.y * particle_scale);
            let particle_color = Self::hue_to_rgb(p.hue, p.opacity * self.opacity);
            let particle = Path::circle(particle_pos, p.size * particle_scale.max(1.0));
            frame.fill(&particle, particle_color);
        }

        // Tibetan color palette
        let saffron = Color::from_rgb(0.85, 0.45, 0.15);
        let gold = Color::from_rgb(0.78, 0.60, 0.20);
        let deep_maroon = Color::from_rgb(0.40, 0.08, 0.12);

        // Color based on breathing scale
        let color = if !self.is_running {
            deep_maroon
        } else {
            Self::lerp_color(deep_maroon, saffron, self.breathing_scale)
        };

        // Draw outer glow layers with warm tones
        let glow_step = half_size * 0.04;
        for i in (1..=6).rev() {
            let glow_radius = radius + (i as f32 * glow_step);
            let alpha = (0.15 / (i as f32)) * self.opacity;
            let glow_color = Color::from_rgba(saffron.r, saffron.g, saffron.b, alpha);
            let glow = Path::circle(center, glow_radius);
            frame.fill(&glow, glow_color);
        }

        // Draw main circle
        let circle = Path::circle(center, radius);
        frame.fill(&circle, Self::with_opacity(color, self.opacity));

        // Subtle transition glow (brief brightness boost on the circle itself)
        if self.flash > 0.0 {
            let glow_alpha = self.flash * 0.2 * self.opacity;
            let glow_color = Color::from_rgba(gold.r, gold.g, gold.b, glow_alpha);
            let glow = Path::circle(center, radius + 2.0);
            frame.fill(&glow, glow_color);
        }

        // Draw rainbow rim - multiple overlapping arcs
        let num_segments = 60;
        for i in 0..num_segments {
            let angle_start = (i as f32 / num_segments as f32) * 2.0 * std::f32::consts::PI;
            let angle_end = ((i + 1) as f32 / num_segments as f32) * 2.0 * std::f32::consts::PI;

            // Rainbow hue based on position around circle
            let hue = i as f32 / num_segments as f32;
            let rainbow_color = Self::hue_to_rgb(hue, 0.8 * self.opacity);

            // Draw arc segment
            let start = Point::new(
                center.x + radius * angle_start.cos(),
                center.y + radius * angle_start.sin(),
            );
            let end = Point::new(
                center.x + radius * angle_end.cos(),
                center.y + radius * angle_end.sin(),
            );

            let rim_width = (half_size * 0.015).max(2.0);
            let segment = Path::line(start, end);
            frame.stroke(
                &segment,
                canvas::Stroke::default()
                    .with_color(rainbow_color)
                    .with_width(rim_width),
            );
        }

        // Inner decorative ring
        if radius > 60.0 {
            let inner_ring = Path::circle(center, radius * 0.65);
            let inner_rim_color = Color::from_rgba(gold.r, gold.g, gold.b, 0.3 * self.opacity);
            frame.stroke(
                &inner_ring,
                canvas::Stroke::default()
                    .with_color(inner_rim_color)
                    .with_width(1.5),
            );
        }

        // Draw phase text with glow effect
        let text_size = (half_size * 0.12).clamp(24.0, 64.0);

        // Draw text glow (multiple layers for soft glow)
        for i in (1..=3).rev() {
            let glow_offset = i as f32 * 1.5;
            let glow_alpha = 0.15 * self.opacity / (i as f32);
            let glow_color = Color::from_rgba(1.0, 0.85, 0.6, glow_alpha);

            // Draw glow in multiple directions
            for &(dx, dy) in &[(0.0, -1.0), (0.0, 1.0), (-1.0, 0.0), (1.0, 0.0)] {
                let glow_text = Text {
                    content: self.phase_text.to_string(),
                    position: Point::new(center.x + dx * glow_offset, center.y + dy * glow_offset),
                    color: glow_color,
                    size: cosmic::iced::Pixels(text_size),
                    horizontal_alignment: cosmic::iced::alignment::Horizontal::Center,
                    vertical_alignment: cosmic::iced::alignment::Vertical::Center,
                    ..Text::default()
                };
                frame.fill_text(glow_text);
            }
        }

        // Draw main text
        let text_color = Color::from_rgba(1.0, 0.98, 0.92, self.opacity);
        let text = Text {
            content: self.phase_text.to_string(),
            position: center,
            color: text_color,
            size: cosmic::iced::Pixels(text_size),
            horizontal_alignment: cosmic::iced::alignment::Horizontal::Center,
            vertical_alignment: cosmic::iced::alignment::Vertical::Center,
            ..Text::default()
        };
        frame.fill_text(text);

        // Draw session progress bar at bottom
        if let Some(progress) = self.session_progress {
            if self.is_running && progress > 0.0 {
                let bar_y = bounds.height - 20.0;
                let bar_width = bounds.width * 0.6;
                let bar_x = (bounds.width - bar_width) / 2.0;
                let bar_height = 4.0;

                // Background bar
                let bg_bar = Path::rectangle(
                    Point::new(bar_x, bar_y),
                    cosmic::iced::Size::new(bar_width, bar_height),
                );
                frame.fill(&bg_bar, Color::from_rgba(1.0, 1.0, 1.0, 0.1 * self.opacity));

                // Progress bar
                let progress_bar = Path::rectangle(
                    Point::new(bar_x, bar_y),
                    cosmic::iced::Size::new(bar_width * progress, bar_height),
                );
                let progress_color = Color::from_rgba(0.85, 0.65, 0.35, 0.5 * self.opacity);
                frame.fill(&progress_bar, progress_color);
            }
        }

        vec![frame.into_geometry()]
    }
}
