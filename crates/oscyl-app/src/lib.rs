//! Oscyl's app logic, shared by every app (GTK on Linux, Compose on
//! mobile): documents with change tracking and undo, the synth's slots,
//! Oscyl's libraries, settings, the synth worker and the i18n hook. No UI toolkit here; each app draws it.

pub mod catalog;
pub mod editor;
pub mod files;
pub mod generator;
pub mod i18n;
pub mod libraries;
pub mod palette;
pub mod panel;
pub mod program_menu;
pub mod review;
pub mod settings;
pub mod shelf;
pub mod slots;
pub mod state;
pub mod synth;
pub mod units;
