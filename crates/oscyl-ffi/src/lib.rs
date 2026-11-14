//! Oscyl's shared core for the mobile apps, through UniFFI: the registered
//! drivers, port recognition, the synth worker as a [`Synth`] object,
//! Oscyl's [`Libraries`], [`Document`]s with undo, library files as bytes,
//! and a program open in the [`ProgramEditor`] with its review.
//! The platform moves MIDI bytes: Rust sends through a foreign
//! [`MidiOut`], the platform pushes what it receives into a
//! [`MidiInputSink`].

mod document;
mod editor;
mod generator;
mod library;
mod settings;
mod synth;
mod units;

pub use document::{
    Clip, Document, FileFormatInfo, HistoryStep, ImportedLibrary, LibraryTarget, MenuEntry,
    ProgramAction, ProgramMenu, ProgramRow, ReorderHow, export_file, export_formats, import_file,
    program_menu,
};
pub use editor::{
    Audition, AxisWord, ControlKind, EditorFacts, FieldChange, FieldState, GraphLevel, GraphMark,
    GraphShape, GraphTick, HandleInfo, MarkStyle, PageInfo, PanelUpdate, ProgramEditor,
    ReviewChange, ReviewProgram, SectionInfo, StageGraph, StageState, SyncPlan, SynthRef,
    audition_target, write_backup,
};
pub use generator::{CharacterInfo, Generator, HandState, MacroInfo, SoundTile};
pub use library::{Libraries, NodeKind, OutlineLine, Trashed};
pub use settings::{PlayLayout, Role, Settings, Theme, set_locale};
pub use synth::{
    FfiError, LinkError, MidiInputSink, MidiOut, SlotInfo, SlotProgram, Synth, SynthEvent,
    SynthListener,
};
pub use units::{
    BackupFile, UnitFile, backups, forget_unsynced, keep_unsynced, new_library_driver, read_file,
    read_unit, unit_backup_path, unit_ext, unit_section, unsynced,
};

uniffi::setup_scaffolding!();

/// A registered driver.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct DriverInfo {
    pub id: String,
    pub vendor: String,
    pub model: String,
    /// Polyphony in words ("4 voices", "8 or 16 voices"), or empty.
    #[uniffi(default = "")]
    pub voices: String,
    /// Support tier (GOVERNANCE.md): 1 hardware-validated, 2
    /// conformance-proven, 3 community.
    #[uniffi(default = 3)]
    pub tier: u8,
    /// The nameplate drawing for the synth picker.
    #[uniffi(default)]
    pub panel: PanelDrawing,
}

/// A knob on a nameplate drawing, in unit coordinates.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct KnobMark {
    pub x: f32,
    pub y: f32,
    pub r: f32,
}

/// A rectangle in unit coordinates.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct UnitRect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

/// A synth's abstract panel, laid out from its driver's pages and fields
/// (`oscyl_app::panel`): never a vendor's panel.
#[derive(Debug, Clone, Default, PartialEq, uniffi::Record)]
pub struct PanelDrawing {
    #[uniffi(default)]
    pub knobs: Vec<KnobMark>,
    #[uniffi(default)]
    pub display: Option<UnitRect>,
}

/// A port whose name matches a driver's hint (the device inquiry confirms
/// it when opened).
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct CandidateInfo {
    pub driver_id: String,
    pub port: String,
}

/// Every driver Oscyl has.
#[uniffi::export]
pub fn drivers() -> Vec<DriverInfo> {
    oscyl_midi::DRIVERS
        .iter()
        .map(|d| {
            let i = d.info();
            let p = oscyl_app::panel::panel(*d);
            DriverInfo {
                id: i.id.into(),
                vendor: i.vendor.into(),
                model: i.model.into(),
                voices: oscyl_app::panel::voices_text(d.voices()),
                tier: d.tier(),
                panel: PanelDrawing {
                    knobs: p
                        .knobs
                        .iter()
                        .map(|k| KnobMark {
                            x: k.x,
                            y: k.y,
                            r: k.r,
                        })
                        .collect(),
                    display: p.display.map(|s| UnitRect {
                        x: s.x,
                        y: s.y,
                        w: s.w,
                        h: s.h,
                    }),
                },
            }
        })
        .collect()
}

/// The (driver, port) pairs among `ports` worth opening, in port order.
#[uniffi::export]
pub fn candidates(ports: Vec<String>) -> Vec<CandidateInfo> {
    oscyl_midi::candidates(&ports)
        .into_iter()
        .map(|c| CandidateInfo {
            driver_id: c.driver.info().id.into(),
            port: c.port,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drivers_and_candidates() {
        assert!(drivers().iter().any(|d| d.id == "korg-minilogue-xd"));
        let hint = oscyl_midi::driver("korg-minilogue-xd").unwrap().port_hint()[0];
        let port = format!("{hint} port");
        assert_eq!(
            candidates(vec!["nothing".into(), port.clone()]),
            vec![CandidateInfo {
                driver_id: "korg-minilogue-xd".into(),
                port
            }]
        );
    }
}
