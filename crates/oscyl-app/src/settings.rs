//! User preferences, persisted as JSON in a file the app chooses.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::i18n::tr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Theme {
    #[default]
    System,
    Light,
    Dark,
    /// True black, for OLED screens and dark studios.
    Amoled,
}

impl Theme {
    pub const ALL: [Theme; 4] = [Theme::System, Theme::Light, Theme::Dark, Theme::Amoled];

    pub fn label(self) -> String {
        match self {
            Theme::System => tr("Follow system"),
            Theme::Light => tr("Light"),
            Theme::Dark => tr("Dark"),
            Theme::Amoled => tr("AMOLED black"),
        }
    }
}

/// What the person mainly does; picks the starting layout (first-run guide).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Role {
    Organise,
    SoundDesign,
    Perform,
}

impl Role {
    /// The icon of the editor page it opens at (a driver may not have
    /// that page; the editor then stays where it is).
    pub fn start_page_icon(self) -> &'static str {
        match self {
            Role::SoundDesign => "oscyl-oscillator-symbolic",
            Role::Perform | Role::Organise => "oscyl-voice-symbolic",
        }
    }
}

/// Developer command-line options: they apply to this run only and are
/// never saved.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Overrides {
    pub theme: Option<Theme>,
    pub zoom: Option<f64>,
    pub simulator: bool,
}

/// The play page's controls (mobile): the XY pad's two fields and four
/// macro knobs, each a field or unassigned.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct PlayLayout {
    pub x: Option<usize>,
    pub y: Option<usize>,
    /// Always four.
    pub macros: Vec<Option<usize>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub theme: Theme,
    /// Interface zoom, 1.0 = 100 %.
    pub zoom: f64,
    /// Turn off animations even when the system allows them.
    pub reduce_motion: bool,
    /// Use the built-in simulated synth instead of USB-MIDI.
    pub use_simulator: bool,
    /// The first-run guide has been completed or dismissed.
    pub onboarded: bool,
    pub role: Option<Role>,
    /// Touch feedback (mobile); the system's own switch applies too.
    pub haptics: bool,
    /// Demo mode on mobile: the driver whose simulated synth connects at
    /// start, or none.
    pub demo: Option<String>,
    /// The play page's layout per synth model (driver id), once changed.
    pub play: std::collections::BTreeMap<String, PlayLayout>,
    /// One-time hints already shown (mobile), by name: each teaches once per install.
    pub taught: std::collections::BTreeSet<String>,
    #[serde(skip)]
    pub overrides: Overrides,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            theme: Theme::System,
            zoom: 1.0,
            reduce_motion: false,
            use_simulator: false,
            onboarded: false,
            role: None,
            haptics: true,
            demo: None,
            play: std::collections::BTreeMap::new(),
            taught: std::collections::BTreeSet::new(),
            overrides: Overrides::default(),
        }
    }
}

pub const ZOOM_MIN: f64 = 0.8;
pub const ZOOM_MAX: f64 = 2.0;
pub const ZOOM_STEP: f64 = 0.1;

impl Settings {
    /// Load saved settings; missing or unreadable files give the defaults.
    pub fn load(path: &Path) -> Settings {
        std::fs::read(path)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let json = serde_json::to_vec_pretty(self).map_err(std::io::Error::other)?;
        // Atomic: a kill mid-write leaves the old file, never a cut one that loads as defaults.
        crate::libraries::write_atomic(path, |w| {
            std::io::Write::write_all(w, &json).map_err(|e| e.to_string())
        })
        .map_err(std::io::Error::other)
    }

    pub fn theme(&self) -> Theme {
        self.overrides.theme.unwrap_or(self.theme)
    }

    pub fn zoom(&self) -> f64 {
        self.overrides.zoom.unwrap_or(self.zoom)
    }

    pub fn simulator(&self) -> bool {
        self.overrides.simulator || self.use_simulator
    }

    /// The person chose a theme, zoom or synth: drop the run's overrides
    /// so what they see is what is saved.
    pub fn set_theme(&mut self, t: Theme) {
        self.theme = t;
        self.overrides.theme = None;
    }

    pub fn set_zoom(&mut self, z: f64) {
        self.zoom = Settings::clamp_zoom(z);
        self.overrides.zoom = None;
    }

    pub fn set_simulator(&mut self, on: bool) {
        self.use_simulator = on;
        self.overrides.simulator = false;
    }

    /// Zoom clamped to the supported range and rounded to whole steps.
    pub fn clamp_zoom(z: f64) -> f64 {
        ((z / ZOOM_STEP).round() * ZOOM_STEP).clamp(ZOOM_MIN, ZOOM_MAX)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zoom_is_clamped_and_stepped() {
        assert_eq!(Settings::clamp_zoom(5.0), ZOOM_MAX);
        assert_eq!(Settings::clamp_zoom(0.1), ZOOM_MIN);
        assert!((Settings::clamp_zoom(1.23) - 1.2).abs() < 1e-9);
    }

    #[test]
    fn overrides_apply_but_are_never_saved() {
        let mut s = Settings {
            overrides: Overrides {
                theme: Some(Theme::Dark),
                zoom: Some(1.5),
                simulator: true,
            },
            ..Settings::default()
        };
        assert_eq!(
            (s.theme(), s.zoom(), s.simulator()),
            (Theme::Dark, 1.5, true)
        );
        let saved: Settings = serde_json::from_slice(&serde_json::to_vec(&s).unwrap()).unwrap();
        assert_eq!(saved, Settings::default());
        s.set_zoom(1.2);
        assert!((s.zoom() - 1.2).abs() < 1e-9);
    }

    #[test]
    fn unknown_fields_and_missing_fields_are_tolerated() {
        let s: Settings = serde_json::from_str(r#"{"zoom":1.5,"future":true}"#).unwrap();
        assert_eq!(s.zoom, 1.5);
        assert_eq!(s.theme, Theme::System);
    }

    #[test]
    fn missing_file_gives_defaults() {
        let s = Settings::load(Path::new("/nonexistent/oscyl/settings.json"));
        assert_eq!(s, Settings::default());
    }

    #[test]
    fn save_then_load_round_trips() {
        let dir = std::env::temp_dir().join(format!("oscyl-app-settings-{}", std::process::id()));
        let path = dir.join("nested").join("settings.json");
        let mut s = Settings::default();
        s.set_zoom(1.5);
        s.theme = Theme::Amoled;
        s.save(&path).unwrap();
        assert_eq!(Settings::load(&path), s);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
