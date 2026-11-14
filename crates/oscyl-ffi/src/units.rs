//! User units (logue family) and other synth-side helpers for the mobile
//! apps: reading a unit file, the sections' names, and which synth a new
//! library is for. The logic is `oscyl-app`'s.

use crate::document::Document;
use crate::synth::{FfiError, driver, module_of};
use std::sync::Arc;

/// A unit file, read for a slot: its name and the data to load.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct UnitFile {
    pub name: String,
    pub data: Vec<u8>,
}

/// Unit file `bytes` for a slot of section `module` on `driver_id`'s synth;
/// refused, in words, when it is not a unit for this synth or belongs in
/// another section. Blocks (unzips).
#[uniffi::export]
pub fn read_unit(driver_id: String, module: String, bytes: Vec<u8>) -> Result<UnitFile, FfiError> {
    let d = driver(&driver_id)?;
    oscyl_app::units::read_unit(d, &bytes, module_of(&module)?)
        .map(|u| UnitFile {
            name: u.name,
            data: u.data,
        })
        .map_err(|reason| FfiError::Refused { reason })
}

/// The section (`osc`, `modfx`, …) a slot backup (`.slot.bin`) of
/// `driver_id`'s synth goes back into; none when `bytes` are not one.
#[uniffi::export]
pub fn unit_backup_section(driver_id: String, bytes: Vec<u8>) -> Option<String> {
    let d = driver(&driver_id).ok()?;
    oscyl_app::units::backup_section(d, &bytes).map(|m| m.name().to_string())
}

/// The extension of `driver_id`'s unit files (`mnlgxdunit`), or none
/// when the synth has no user units.
#[uniffi::export]
pub fn unit_ext(driver_id: String) -> Option<String> {
    driver(&driver_id)
        .ok()
        .and_then(oscyl_midi::logue::unit_ext)
        .map(str::to_string)
}

/// A unit section's name for people ("Oscillators").
#[uniffi::export]
pub fn unit_section(module: String) -> String {
    module_of(&module)
        .map(oscyl_app::units::module_title)
        .unwrap_or(module)
}

/// A backup file in the backups folder.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct BackupFile {
    pub path: String,
    /// Its file name ("Synth backup 2026-11-13 10.04.11.mnlgxdlib").
    pub name: String,
    /// Seconds since 1970, for the newest first.
    pub modified: u64,
}

/// The backups in folder `dir`, newest first (none when it does not
/// exist yet). Blocks.
#[uniffi::export]
pub fn backups(dir: String) -> Vec<BackupFile> {
    let mut all: Vec<BackupFile> = std::fs::read_dir(&dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.file_type().is_ok_and(|t| t.is_file()))
        .map(|e| BackupFile {
            path: e.path().to_string_lossy().into_owned(),
            name: e.file_name().to_string_lossy().into_owned(),
            modified: e
                .metadata()
                .and_then(|m| m.modified())
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map_or(0, |d| d.as_secs()),
        })
        .collect();
    all.sort_by(|a, b| {
        b.modified
            .cmp(&a.modified)
            .then_with(|| b.name.cmp(&a.name))
    });
    all
}

/// The bytes of file `path` (a backup to open or share). Blocks.
#[uniffi::export]
pub fn read_file(path: String) -> Result<Vec<u8>, FfiError> {
    std::fs::read(&path).map_err(|e| FfiError::failed(e.to_string()))
}

/// A new file name in the backups folder for slot `name`'s unit, before a
/// clear: `<name> <stamp>.slot.bin`.
#[uniffi::export]
pub fn unit_backup_path(dir: String, name: String) -> String {
    let base = std::path::Path::new(&dir);
    let stamp = oscyl_app::review::utc_stamp();
    let safe: String = name
        .chars()
        .map(|c| {
            if c == '/' || c == '\\' || c.is_control() {
                '_'
            } else {
                c
            }
        })
        .collect();
    (1..)
        .map(|n| match n {
            1 => base.join(format!("{safe} {stamp}.slot.bin")),
            n => base.join(format!("{safe} {stamp} ({n}).slot.bin")),
        })
        .find(|p| !p.exists())
        .expect("a free name")
        .to_string_lossy()
        .into_owned()
}

/// Keep a connected synth's unsynced edits in folder `dir` as `key`, so
/// they outlive the app's process: `kept` is what [`Document::offline`]
/// gave for it (taken where the document is current, so a later close
/// can't lose it), written atomically; `None` (nothing edited) removes the
/// file. Blocks.
#[uniffi::export]
pub fn keep_unsynced(
    dir: String,
    key: String,
    kept: Option<Arc<Document>>,
) -> Result<(), FfiError> {
    let path = std::path::Path::new(&dir).join(format!("{key}.{}", oscyl_core::native::EXT));
    match kept {
        Some(kept) => {
            let bytes = crate::export_file(&kept, oscyl_core::native::EXT.into())?;
            std::fs::create_dir_all(&dir).map_err(|e| FfiError::failed(e.to_string()))?;
            oscyl_app::libraries::write_atomic(&path, |w| {
                std::io::Write::write_all(w, &bytes).map_err(|e| e.to_string())
            })
            .map_err(FfiError::failed)
        }
        None => match std::fs::remove_file(&path) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => {
                Err(FfiError::failed(e.to_string()))
            }
            _ => Ok(()),
        },
    }
}

/// Forget the unsynced edits kept as `key` in `dir` (they are a library now,
/// or synced). Blocks.
#[uniffi::export]
pub fn forget_unsynced(dir: String, key: String) {
    let _ = std::fs::remove_file(
        std::path::Path::new(&dir).join(format!("{key}.{}", oscyl_core::native::EXT)),
    );
}

/// The unsynced edits kept in `dir` (a previous run's), as files to import,
/// then forget once kept. Blocks.
#[uniffi::export]
pub fn unsynced(dir: String) -> Vec<BackupFile> {
    backups(dir)
        .into_iter()
        .filter(|f| f.name.ends_with(&format!(".{}", oscyl_core::native::EXT)))
        .collect()
}

/// The synth a new library is for, as on the desktop: the first connected
/// synth's, else the library shown's, else the first synth Oscyl knows.
#[uniffi::export]
pub fn new_library_driver(connected: Vec<String>, shown: Option<String>) -> String {
    connected
        .into_iter()
        .chain(shown)
        .find(|id| driver(id).is_ok())
        .unwrap_or_else(|| oscyl_midi::DRIVERS[0].info().id.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn units_and_new_libraries() {
        assert_eq!(
            unit_ext("korg-minilogue-xd".into()).as_deref(),
            Some("mnlgxdunit")
        );
        assert_eq!(unit_ext("sequential-prophet-rev2".into()), None);
        assert!(read_unit("korg-minilogue-xd".into(), "osc".into(), vec![1, 2]).is_err());
        assert_eq!(unit_section("osc".into()), "Oscillators");
        let xd = "korg-minilogue-xd".to_string();
        let rev2 = "sequential-prophet-rev2".to_string();
        assert_eq!(
            new_library_driver(vec![rev2.clone()], Some(xd.clone())),
            rev2
        );
        assert_eq!(new_library_driver(vec![], Some(xd.clone())), xd);
        assert_eq!(
            new_library_driver(vec![], None),
            oscyl_midi::DRIVERS[0].info().id
        );
    }

    #[test]
    fn backups_are_listed_and_unit_backups_named() {
        let dir = std::env::temp_dir().join(format!("oscyl-ffi-backups-{}", std::process::id()));
        let d = dir.to_string_lossy().into_owned();
        assert!(backups(d.clone()).is_empty(), "no folder yet");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a.mnlgxdlib"), b"x").unwrap();
        let list = backups(d.clone());
        assert_eq!(list.len(), 1);
        assert_eq!(read_file(list[0].path.clone()).unwrap(), b"x");
        assert!(unit_backup_path(d, "a/b".into()).contains("a_b "));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn unsynced_edits_outlive_the_process() {
        let dir = std::env::temp_dir().join(format!("oscyl-ffi-unsynced-{}", std::process::id()));
        let d = dir.to_string_lossy().into_owned();
        let doc = Document::synth("korg-minilogue-xd".into(), "xd".into()).unwrap();
        keep_unsynced(d.clone(), "xd-1".into(), doc.offline(String::new())).unwrap();
        assert!(
            unsynced(d.clone()).is_empty(),
            "nothing edited, nothing kept"
        );
        doc.set_name(2, "Mine".into()).unwrap();
        keep_unsynced(d.clone(), "xd-1".into(), doc.offline(String::new())).unwrap();
        let kept = unsynced(d.clone());
        assert_eq!(kept.len(), 1);
        let bytes = read_file(kept[0].path.clone()).unwrap();
        let back = crate::import_file(kept[0].name.clone(), bytes).unwrap();
        assert_eq!(back.document.name(2), "Mine");
        forget_unsynced(d.clone(), "xd-1".into());
        assert!(unsynced(d).is_empty());
        let _ = std::fs::remove_dir_all(dir);
    }
}
