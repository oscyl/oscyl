//! Library files as bytes: Oscyl's own `.oscyllib`, or any registered
//! synth's own format, told apart by the file's extension (and, for a name
//! without a known one, by its contents). Each app owns paths and pickers:
//! GTK reads and writes files, the mobile apps hand bytes across.

use oscyl_core::native;
use oscyl_driver::{Driver, LibraryData};
use oscyl_midi::DRIVERS;

use crate::i18n::tr;

fn text(e: impl std::fmt::Display) -> String {
    e.to_string()
}

/// The lower-case extension of file name `name`, without the dot.
pub fn ext_of(name: &str) -> String {
    std::path::Path::new(name)
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default()
}

/// File name `name` without its folder or extension: a library's title.
pub fn title_of(name: &str) -> String {
    std::path::Path::new(name)
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// An Oscyl library's synth and contents.
fn read_native(bytes: &[u8]) -> Result<(&'static dyn Driver, LibraryData), String> {
    let (id, lib) = native::read(std::io::Cursor::new(bytes)).map_err(text)?;
    let d = oscyl_midi::driver(&id)
        .ok_or_else(|| format!("this library is for a synth Oscyl does not know ({id})"))?;
    Ok((d, lib))
}

/// The contents of file `name`: an Oscyl library, or a synth's own format
/// chosen by the extension. A name with no extension Oscyl knows is read
/// as an Oscyl library if it is one (a picker may drop the extension).
pub fn read(name: &str, bytes: &[u8]) -> Result<(&'static dyn Driver, LibraryData), String> {
    let ext = ext_of(name);
    if ext == native::EXT {
        return read_native(bytes);
    }
    match DRIVERS
        .iter()
        .copied()
        .find(|d| d.file_formats().iter().any(|f| f.ext == ext))
    {
        Some(d) => Ok((d, d.import(&ext, bytes).map_err(text)?)),
        None => read_native(bytes)
            .map_err(|_| tr("Oscyl can't read .{ext} files.").replace("{ext}", &ext)),
    }
}

/// Whether a library of `d` can hold what file `lib` holds; if not, why.
pub fn fits(d: &dyn Driver, lib: &LibraryData) -> Result<(), String> {
    let max = crate::slots::library_max(d);
    if lib.programs.len() > max {
        return Err(tr(
            "This file holds {count} programs; a library for this synth holds up to {max}.",
        )
        .replace("{count}", &lib.programs.len().to_string())
        .replace("{max}", &max.to_string()));
    }
    Ok(())
}

/// `lib` of synth `d` in the format extension `ext` names.
pub fn write(ext: &str, d: &dyn Driver, lib: &LibraryData) -> Result<Vec<u8>, String> {
    let ext = ext.to_lowercase();
    if ext == native::EXT {
        let mut out = std::io::Cursor::new(Vec::new());
        native::write(&mut out, d.info().id, lib).map_err(text)?;
        Ok(out.into_inner())
    } else {
        d.export(&ext, lib).map_err(text)
    }
}

/// The extension a library of `d` exports to: the synth's first format
/// (an unreadable library's stored file: Oscyl's own).
pub fn export_ext(d: &dyn Driver, broken: bool) -> &'static str {
    match d.file_formats().first() {
        Some(f) if !broken => f.ext,
        _ => native::EXT,
    }
}

/// A format a library can be written in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Format {
    pub ext: &'static str,
    pub name: String,
}

/// Every format a library of `d` can be exported to: the synth's own,
/// first the one [`export_ext`] picks, then Oscyl's.
pub fn formats(d: &dyn Driver) -> Vec<Format> {
    d.file_formats()
        .iter()
        .map(|f| Format {
            ext: f.ext,
            name: f.name.to_string(),
        })
        .chain(std::iter::once(Format {
            ext: native::EXT,
            name: tr("Oscyl library"),
        }))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data(l: &LibraryData) -> Vec<Vec<u8>> {
        l.programs.iter().map(|p| p.data.clone()).collect()
    }

    #[test]
    fn every_format_of_every_synth_round_trips() {
        for d in DRIVERS {
            let lib = d.demo_library();
            for f in formats(*d) {
                let bytes = write(f.ext, *d, &lib).unwrap();
                let (back_d, back) = read(&format!("x.{}", f.ext.to_uppercase()), &bytes).unwrap();
                assert_eq!(back_d.info().id, d.info().id);
                assert_eq!(data(&back), data(&lib), "{}", f.ext);
            }
            assert_eq!(formats(*d)[0].ext, export_ext(*d, false));
            assert_eq!(export_ext(*d, true), native::EXT);
        }
    }

    #[test]
    fn a_name_without_extension_reads_by_contents() {
        let d = DRIVERS[0];
        let bytes = write(native::EXT, d, &d.demo_library()).unwrap();
        assert_eq!(read("download", &bytes).unwrap().0.info().id, d.info().id);
        let e = read("notes.txt", b"hello").err();
        assert_eq!(e.as_deref(), Some("Oscyl can't read .txt files."));
    }

    #[test]
    fn a_library_holds_up_to_the_writable_slots() {
        let d = DRIVERS[0];
        let max = crate::slots::library_max(d);
        let mut lib = LibraryData {
            programs: vec![d.demo_library().programs[0].clone(); max],
            ..LibraryData::default()
        };
        assert!(fits(d, &lib).is_ok());
        lib.programs.push(lib.programs[0].clone());
        assert!(
            fits(d, &lib)
                .unwrap_err()
                .starts_with(&format!("This file holds {}", max + 1))
        );
    }

    #[test]
    fn names() {
        assert_eq!(ext_of("Gig.MNLGXDLIB"), "mnlgxdlib");
        assert_eq!(ext_of("nothing"), "");
        assert_eq!(title_of("Gig set.oscyllib"), "Gig set");
    }
}
