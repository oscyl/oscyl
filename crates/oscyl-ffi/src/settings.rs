//! Preferences (`oscyl_app::settings`) and the language of core strings,
//! for the mobile apps. The app supplies the file; nothing else is kept.

use std::path::PathBuf;
use std::sync::Mutex;

use oscyl_app::settings::{
    PlayLayout as AppPlayLayout, Role as AppRole, Settings as AppSettings, Theme as AppTheme,
};

use crate::synth::FfiError;

/// How the app looks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum Theme {
    System,
    Light,
    Dark,
    /// True black, for OLED screens and dark studios.
    Amoled,
}

/// What the person mainly does (the first-run question).
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum Role {
    Organise,
    SoundDesign,
    Perform,
}

impl From<AppTheme> for Theme {
    fn from(t: AppTheme) -> Theme {
        match t {
            AppTheme::System => Theme::System,
            AppTheme::Light => Theme::Light,
            AppTheme::Dark => Theme::Dark,
            AppTheme::Amoled => Theme::Amoled,
        }
    }
}

impl From<Theme> for AppTheme {
    fn from(t: Theme) -> AppTheme {
        match t {
            Theme::System => AppTheme::System,
            Theme::Light => AppTheme::Light,
            Theme::Dark => AppTheme::Dark,
            Theme::Amoled => AppTheme::Amoled,
        }
    }
}

impl From<AppRole> for Role {
    fn from(r: AppRole) -> Role {
        match r {
            AppRole::Organise => Role::Organise,
            AppRole::SoundDesign => Role::SoundDesign,
            AppRole::Perform => Role::Perform,
        }
    }
}

impl From<Role> for AppRole {
    fn from(r: Role) -> AppRole {
        match r {
            Role::Organise => AppRole::Organise,
            Role::SoundDesign => AppRole::SoundDesign,
            Role::Perform => AppRole::Perform,
        }
    }
}

/// The play page's controls: the XY pad's two fields and four macro knobs,
/// each a field or unassigned (`macros` is always four long).
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct PlayLayout {
    pub x: Option<u32>,
    pub y: Option<u32>,
    pub macros: Vec<Option<u32>>,
}

fn field(f: usize) -> u32 {
    u32::try_from(f).unwrap_or(u32::MAX)
}

impl From<AppPlayLayout> for PlayLayout {
    fn from(l: AppPlayLayout) -> PlayLayout {
        let mut macros: Vec<Option<u32>> = l.macros.into_iter().map(|m| m.map(field)).collect();
        macros.resize(4, None);
        PlayLayout {
            x: l.x.map(field),
            y: l.y.map(field),
            macros,
        }
    }
}

impl From<PlayLayout> for AppPlayLayout {
    fn from(l: PlayLayout) -> AppPlayLayout {
        AppPlayLayout {
            x: l.x.map(|f| f as usize),
            y: l.y.map(|f| f as usize),
            macros: l
                .macros
                .into_iter()
                .map(|m| m.map(|f| f as usize))
                .collect(),
        }
    }
}

/// The person's preferences, read from `path` (defaults when it is missing
/// or unreadable). Setters change them here; [`Settings::save`] writes
/// them.
#[derive(uniffi::Object)]
pub struct Settings {
    path: PathBuf,
    inner: Mutex<AppSettings>,
}

impl Settings {
    fn get<R>(&self, f: impl FnOnce(&AppSettings) -> R) -> R {
        f(&self.inner.lock().unwrap_or_else(|e| e.into_inner()))
    }

    fn set(&self, f: impl FnOnce(&mut AppSettings)) {
        f(&mut self.inner.lock().unwrap_or_else(|e| e.into_inner()));
    }
}

#[uniffi::export]
impl Settings {
    /// Blocks (reads a small file).
    #[uniffi::constructor]
    pub fn load(path: String) -> std::sync::Arc<Settings> {
        let path = PathBuf::from(path);
        std::sync::Arc::new(Settings {
            inner: Mutex::new(AppSettings::load(&path)),
            path,
        })
    }

    /// Write them. Blocks.
    pub fn save(&self) -> Result<(), FfiError> {
        self.get(|s| s.save(&self.path))
            .map_err(|e| FfiError::failed(e.to_string()))
    }

    pub fn theme(&self) -> Theme {
        self.get(|s| s.theme().into())
    }

    pub fn set_theme(&self, theme: Theme) {
        self.set(|s| s.set_theme(theme.into()));
    }

    /// Animations on (the app's own switch, on top of the system's).
    pub fn motion(&self) -> bool {
        self.get(|s| !s.reduce_motion)
    }

    pub fn set_motion(&self, on: bool) {
        self.set(|s| s.reduce_motion = !on);
    }

    pub fn haptics(&self) -> bool {
        self.get(|s| s.haptics)
    }

    pub fn set_haptics(&self, on: bool) {
        self.set(|s| s.haptics = on);
    }

    /// Demo mode: the driver whose simulated synth connects at start.
    pub fn demo(&self) -> Option<String> {
        self.get(|s| s.demo.clone())
    }

    pub fn set_demo(&self, driver_id: Option<String>) {
        self.set(|s| s.demo = driver_id);
    }

    /// The first-run question has been answered or dismissed.
    pub fn onboarded(&self) -> bool {
        self.get(|s| s.onboarded)
    }

    pub fn set_onboarded(&self, done: bool) {
        self.set(|s| s.onboarded = done);
    }

    pub fn role(&self) -> Option<Role> {
        self.get(|s| s.role.map(Role::from))
    }

    pub fn set_role(&self, role: Option<Role>) {
        self.set(|s| s.role = role.map(AppRole::from));
    }

    /// The play page's layout for model `driver_id`, once one was chosen.
    pub fn play_layout(&self, driver_id: String) -> Option<PlayLayout> {
        self.get(|s| s.play.get(&driver_id).cloned().map(PlayLayout::from))
    }

    pub fn set_play_layout(&self, driver_id: String, layout: PlayLayout) {
        self.set(|s| {
            s.play.insert(driver_id, layout.into());
        });
    }

    /// The one-time hints already shown, by name.
    pub fn taught(&self) -> Vec<String> {
        self.get(|s| s.taught.iter().cloned().collect())
    }

    /// Hint `name` has been shown: it is not shown again.
    pub fn set_taught(&self, name: String) {
        self.set(|s| {
            s.taught.insert(name);
        });
    }
}

/// Show core and driver strings in the language of BCP 47 `tag` ("it",
/// "it-IT"); English when Oscyl has no catalog for it. Call it at start
/// and whenever the app's language changes.
#[uniffi::export]
pub fn set_locale(tag: String) {
    oscyl_app::catalog::set_locale(&tag);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_round_trips_through_settings() {
        let dir = std::env::temp_dir().join(format!("oscyl-ffi-play-{}", std::process::id()));
        let path = dir.join("settings.json").to_string_lossy().into_owned();
        let s = Settings::load(path.clone());
        let layout = PlayLayout {
            x: Some(10),
            y: None,
            macros: vec![Some(3), None, Some(7), None],
        };
        s.set_play_layout("korg-minilogue-xd".into(), layout.clone());
        s.save().unwrap();
        assert_eq!(
            Settings::load(path).play_layout("korg-minilogue-xd".into()),
            Some(layout)
        );
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn a_hint_is_taught_once_and_kept() {
        let dir = std::env::temp_dir().join(format!("oscyl-ffi-taught-{}", std::process::id()));
        let path = dir.join("settings.json").to_string_lossy().into_owned();
        let s = Settings::load(path.clone());
        assert!(s.taught().is_empty());
        s.set_taught("stage-rail".into());
        s.set_taught("stage-rail".into());
        s.save().unwrap();
        assert_eq!(
            Settings::load(path).taught(),
            vec!["stage-rail".to_string()]
        );
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn unknown_driver_has_no_layout() {
        let s = Settings::load("/nonexistent/oscyl/settings.json".into());
        assert_eq!(s.play_layout("no-such-synth".into()), None);
    }

    #[test]
    fn preferences_persist() {
        let dir = std::env::temp_dir().join(format!("oscyl-ffi-settings-{}", std::process::id()));
        let path = dir.join("settings.json").to_string_lossy().into_owned();
        let s = Settings::load(path.clone());
        assert_eq!(
            (s.theme(), s.motion(), s.haptics(), s.demo()),
            (Theme::System, true, true, None)
        );
        s.set_theme(Theme::Amoled);
        s.set_motion(false);
        s.set_haptics(false);
        s.set_demo(Some("korg-minilogue-xd".into()));
        s.set_role(Some(Role::Perform));
        s.set_onboarded(true);
        s.save().unwrap();
        let again = Settings::load(path);
        assert_eq!(again.theme(), Theme::Amoled);
        assert!(!again.motion() && !again.haptics() && again.onboarded());
        assert_eq!(again.demo().as_deref(), Some("korg-minilogue-xd"));
        assert_eq!(again.role(), Some(Role::Perform));
        let _ = std::fs::remove_dir_all(dir);
    }
}
