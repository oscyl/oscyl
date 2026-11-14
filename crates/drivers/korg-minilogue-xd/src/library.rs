//! The official Librarian's file formats: `.mnlgxdlib` (library),
//! `.mnlgxdpreset` (preset pack) and `.mnlgxdprog` (single program).
//!
//! All three are ZIP archives with a `FileInformation.xml` index that lists
//! each entry. Programs are typed; every other entry (favorites, tuning
//! tables, preset information, unknown members) is kept verbatim so a read
//! followed by a write loses nothing. The format is the logue family's
//! ([`oscyl_logue::library`]) with the xd's names
//! ([`crate::model::XdModel`]).

use oscyl_driver::LibraryData;

use crate::Error;
use crate::model::XdModel;

#[cfg(test)]
use crate::Program;
#[cfg(test)]
use oscyl_logue::library::PROGRAM_DATA;
#[cfg(test)]
use zip::{ZipWriter, write::SimpleFileOptions};

pub use oscyl_logue::library::{Entry, ProgInfo};

/// A Librarian file held in memory.
pub type Library = oscyl_logue::library::Library<XdModel>;

/// A Librarian file as neutral library data: programs with their author
/// metadata; every other entry and member kept in the extras, the
/// non-program index entries as `FileInformation.xml`.
pub fn to_data(lib: &Library) -> Result<LibraryData, Error> {
    oscyl_logue::library::to_data(lib)
}

/// The Librarian file for neutral library data: programs first, then the
/// other entries and members from the extras.
pub fn from_data(data: &LibraryData) -> Result<Library, Error> {
    oscyl_logue::library::from_data(data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Field;
    use std::io::Cursor;

    fn named(name: &str) -> Program {
        let mut p = Program::init();
        p.set_name(name).unwrap();
        p
    }

    fn with_extras() -> Library {
        let mut lib = Library::default();
        let info = |p: &str, c: &str| ProgInfo {
            programmer: p.into(),
            comment: c.into(),
        };
        lib.push_program(&named("One"), &info("Ann", "first"));
        lib.push_program(&named("Two"), &info("Bob", "second"));
        lib.entries.push(Entry {
            kind: "FavoriteData".into(),
            fields: vec![("FavoriteBinary".into(), "Fav_000.fav_data".into())],
        });
        lib.files.insert("Fav_000.fav_data".into(), vec![1, 2, 3]);
        lib.files.insert("Unknown/member.bin".into(), vec![9]);
        lib
    }

    fn non_program(lib: &Library) -> (Vec<Entry>, Vec<(String, Vec<u8>)>) {
        (
            lib.entries
                .iter()
                .filter(|e| e.kind != PROGRAM_DATA)
                .cloned()
                .collect(),
            lib.files
                .iter()
                .filter(|(n, _)| !n.starts_with("Prog_"))
                .map(|(n, d)| (n.clone(), d.clone()))
                .collect(),
        )
    }

    #[test]
    fn extras_survive_native_round_trip() {
        let lib = with_extras();
        let back = from_data(&to_data(&lib).unwrap()).unwrap();
        assert_eq!(back.programs().unwrap(), lib.programs().unwrap());
        assert_eq!(non_program(&back), non_program(&lib));
    }

    #[test]
    fn data_programs_carry_author_and_comment() {
        let d = to_data(&with_extras()).unwrap();
        assert_eq!(d.programs.len(), 2);
        assert_eq!(d.programs[1].data, named("Two").as_bytes().to_vec());
        assert_eq!(
            (
                d.programs[1].author.as_str(),
                d.programs[1].comment.as_str()
            ),
            ("Bob", "second")
        );
        assert!(d.extras.contains_key("FileInformation.xml"));
        assert!(!d.extras.keys().any(|k| k.starts_with("Prog_")));
    }

    fn round_trip(lib: &Library) -> Library {
        let mut out = Cursor::new(Vec::new());
        lib.write(&mut out).unwrap();
        Library::read(Cursor::new(out.into_inner())).unwrap()
    }

    #[test]
    fn three_programs_round_trip() {
        let mut lib = Library::default();
        for (i, n) in ["Alpha", "Beta", "Gamma"].iter().enumerate() {
            let info = ProgInfo {
                programmer: "me & you".into(),
                comment: format!("<{i}>"),
            };
            lib.push_program(&named(n), &info);
        }
        let back = round_trip(&lib);
        assert_eq!(back.programs().unwrap(), lib.programs().unwrap());
        assert_eq!(back.programs().unwrap()[2].1.comment, "<2>");
    }

    #[test]
    fn index_counts_programs() {
        let mut lib = Library::default();
        for n in ["A", "B", "C"] {
            lib.push_program(&named(n), &ProgInfo::default());
        }
        assert!(lib.index_xml().contains("NumProgramData=\"3\""));
    }

    #[test]
    fn keeps_unknown_members_and_entries() {
        let mut lib = Library::default();
        lib.push_program(&Program::init(), &ProgInfo::default());
        lib.files.insert("Notes.txt".into(), b"hello".to_vec());
        lib.files
            .insert("FavoriteData.fav_data".into(), vec![1, 2, 3]);
        lib.entries.push(Entry {
            kind: "FavoriteData".into(),
            fields: vec![("File".into(), "FavoriteData.fav_data".into())],
        });
        let back = round_trip(&lib);
        assert_eq!(back, lib);
        assert!(back.index_xml().contains("NumFavoriteData=\"1\""));
    }

    #[test]
    fn edited_field_survives() {
        let mut p = Program::init();
        p.set(Field::ALL[3], Field::ALL[3].spec().max).unwrap();
        let mut lib = Library::default();
        lib.push_program(&p, &ProgInfo::default());
        assert_eq!(round_trip(&lib).programs().unwrap()[0].0, p);
    }

    #[test]
    fn zip_without_index_is_not_a_library() {
        let mut out = Cursor::new(Vec::new());
        let mut z = ZipWriter::new(&mut out);
        z.start_file("x.txt", SimpleFileOptions::default()).unwrap();
        z.finish().unwrap();
        assert!(matches!(
            Library::read(Cursor::new(out.into_inner())),
            Err(Error::NotALibrary(_))
        ));
    }
}
