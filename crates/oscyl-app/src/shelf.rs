//! The shelf: unit files kept in the app's data folder, under
//! `units/<platform>/`, so a user oscillator or effect is one tap away
//! from a synth's first free slot ([`crate::synth::Cmd::LoadUnit`]).
//! Shared by every app.

use std::path::{Path, PathBuf};

use oscyl_driver::Driver;
use oscyl_logue::unit::{Unit, UnitInfo};
use oscyl_midi::logue::{platform, unit_ext};

use crate::i18n::tr;
use crate::units::{UnitFile, module_title};

/// A unit on the shelf.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShelfUnit {
    /// Its file's name on the shelf: what [`Shelf::remove`] and
    /// [`Shelf::unit`] take.
    pub file: String,
    /// What it says about itself, parameters included.
    pub info: UnitInfo,
    /// Its section, for people ("Oscillators").
    pub section: String,
}

/// `name` as one plain file name: letters, digits, spaces, `-`, `_` and
/// `.` stay, anything else (a path separator above all) becomes `_`.
fn safe_name(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || " -_.".contains(c) {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// The shelf under one folder (`<app data>/units`).
#[derive(Debug, Clone)]
pub struct Shelf {
    root: PathBuf,
}

impl Shelf {
    pub fn at(root: &Path) -> Shelf {
        Shelf {
            root: root.to_path_buf(),
        }
    }

    /// Where `d`'s units are kept, when it takes units at all.
    fn dir(&self, d: &dyn Driver) -> Result<PathBuf, String> {
        let p = platform(d).ok_or_else(|| tr("This synth has no user units."))?;
        Ok(self.root.join(p.name))
    }

    fn read(d: &dyn Driver, bytes: &[u8]) -> Result<Unit, String> {
        let p = platform(d).ok_or_else(|| tr("This synth has no user units."))?;
        Unit::read(&p, std::io::Cursor::new(bytes)).map_err(|e| e.to_string())
    }

    fn entry(file: String, unit: &Unit) -> ShelfUnit {
        ShelfUnit {
            file,
            info: unit.info(),
            section: module_title(unit.module),
        }
    }

    /// Put unit file `bytes` on `d`'s shelf, under its unit's own name (a
    /// unit of that name already there is replaced). Why not, in words,
    /// when it is not a unit for this synth.
    pub fn add(&self, d: &dyn Driver, bytes: &[u8]) -> Result<ShelfUnit, String> {
        let unit = Self::read(d, bytes)?;
        let dir = self.dir(d)?;
        let ext = unit_ext(d).unwrap_or("unit");
        let file = format!("{}.{ext}", safe_name(&unit.name));
        std::fs::create_dir_all(&dir)
            .and_then(|()| std::fs::write(dir.join(&file), bytes))
            .map_err(|e| e.to_string())?;
        Ok(Self::entry(file, &unit))
    }

    /// `d`'s shelf, by section then name. A file that is not a unit for
    /// this synth (put there by hand) is left out.
    pub fn list(&self, d: &dyn Driver) -> Vec<ShelfUnit> {
        let Ok(dir) = self.dir(d) else { return vec![] };
        let mut out: Vec<ShelfUnit> = std::fs::read_dir(dir)
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|e| {
                let file = e.file_name().into_string().ok()?;
                let unit = Self::read(d, &std::fs::read(e.path()).ok()?).ok()?;
                Some(Self::entry(file, &unit))
            })
            .collect();
        let order = |u: &ShelfUnit| {
            let section = oscyl_midi::logue::Module::ALL
                .iter()
                .position(|m| *m == u.info.module);
            (section, u.info.name.to_lowercase(), u.file.clone())
        };
        out.sort_by_key(order);
        out
    }

    /// The file `file` of `d`'s shelf: only a plain file name is one.
    fn path(&self, d: &dyn Driver, file: &str) -> Result<PathBuf, String> {
        if file.is_empty() || file != safe_name(file) || file.starts_with('.') {
            return Err(tr("That is not a unit on the shelf."));
        }
        Ok(self.dir(d)?.join(file))
    }

    /// Take `file` off `d`'s shelf.
    pub fn remove(&self, d: &dyn Driver, file: &str) -> Result<(), String> {
        std::fs::remove_file(self.path(d, file)?).map_err(|e| e.to_string())
    }

    /// `file` of `d`'s shelf, read: its name, its section and the slot
    /// data to load.
    pub fn unit(&self, d: &dyn Driver, file: &str) -> Result<UnitFile, String> {
        let bytes = std::fs::read(self.path(d, file)?).map_err(|e| e.to_string())?;
        let u = Self::read(d, &bytes)?;
        Ok(UnitFile {
            data: u.slot_data(),
            name: u.name,
            module: u.module,
        })
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use oscyl_midi::logue::Module;
    use std::io::Write;

    /// A unit file for `platform` ("minilogue-xd"): an oscillator called
    /// `name` with two parameters.
    pub(crate) fn unit_file(platform: &str, module: &str, name: &str) -> Vec<u8> {
        let manifest = serde_json::json!({"header": {
            "platform": platform, "module": module, "api": "1.1-0",
            "dev_id": 0, "prg_id": 0, "version": "1.0-0", "name": name,
            "num_param": 2,
            "params": [["Wave", 0, 45, ""], ["Mix", 0, 100, "%"]]
        }});
        let magic: &[u8] = match module {
            "osc" => b"UOSC",
            "modfx" => b"UMOD",
            "delfx" => b"UDEL",
            _ => b"UREV",
        };
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        let options = zip::write::SimpleFileOptions::default();
        zip.start_file(format!("{name}/manifest.json"), options)
            .unwrap();
        zip.write_all(manifest.to_string().as_bytes()).unwrap();
        zip.start_file(format!("{name}/payload.bin"), options)
            .unwrap();
        zip.write_all(&[magic, &[0u8; 60]].concat()).unwrap();
        zip.finish().unwrap().into_inner()
    }

    #[test]
    fn units_are_added_listed_read_and_removed_per_synth() {
        let root = std::env::temp_dir().join(format!("oscyl-shelf-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let shelf = Shelf::at(&root);
        let xd = oscyl_midi::driver("korg-minilogue-xd").unwrap();
        let prologue = oscyl_midi::driver("korg-prologue").unwrap();
        let rev2 = oscyl_midi::driver("sequential-prophet-rev2").unwrap();
        assert!(shelf.list(xd).is_empty());
        let waves = unit_file("minilogue-xd", "osc", "waves");
        let added = shelf.add(xd, &waves).unwrap();
        assert_eq!(
            (&*added.file, &*added.section),
            ("waves.mnlgxdunit", "Oscillators")
        );
        let params = added.info.params.clone().unwrap();
        assert_eq!(params.len(), 2);
        assert_eq!((&*params[0].name, params[0].max), ("Wave", 45));
        shelf
            .add(xd, &unit_file("minilogue-xd", "revfx", "Shimmer"))
            .unwrap();
        shelf
            .add(xd, &unit_file("minilogue-xd", "osc", "anthem"))
            .unwrap();
        // Again: the same unit, not a second one.
        shelf.add(xd, &waves).unwrap();
        let names: Vec<String> = shelf.list(xd).into_iter().map(|u| u.info.name).collect();
        assert_eq!(names, ["anthem", "waves", "Shimmer"], "oscillators first");
        // Each synth has its own shelf, and takes only its own units.
        assert!(shelf.list(prologue).is_empty());
        assert!(shelf.add(prologue, &waves).is_err());
        assert!(shelf.add(xd, b"not a unit").is_err());
        assert!(shelf.add(rev2, &waves).is_err() && shelf.list(rev2).is_empty());
        assert!(root.join("minilogue-xd").join("waves.mnlgxdunit").is_file());
        // Read for loading; only a name on the shelf is taken.
        let unit = shelf.unit(xd, "waves.mnlgxdunit").unwrap();
        assert_eq!((unit.module, &*unit.name), (Module::Osc, "waves"));
        assert!(shelf.unit(xd, "../waves.mnlgxdunit").is_err());
        assert!(shelf.remove(xd, "../../etc/passwd").is_err());
        shelf.remove(xd, "waves.mnlgxdunit").unwrap();
        assert_eq!(shelf.list(xd).len(), 2);
        assert!(shelf.remove(xd, "waves.mnlgxdunit").is_err());
        std::fs::remove_dir_all(&root).unwrap();
    }
}
