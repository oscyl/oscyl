//! Oscyl's own library file, `.oscyllib`: a ZIP holding `oscyl.json`
//! (format version, driver id, per-program metadata), the program bytes in
//! the driver's native layout, and the driver's extras verbatim. See
//! `docs/protocol/native-library-format.md`.

use std::collections::BTreeMap;
use std::io::{Read, Seek, Write};

use oscyl_driver::{LibProgram, LibraryData};
use serde::{Deserialize, Serialize};
use zip::write::SimpleFileOptions;
use zip::{ZipArchive, ZipWriter};

/// File extension of a native library.
pub const EXT: &str = "oscyllib";
/// The format this version writes and the newest it reads.
pub const FORMAT: u32 = 1;

const MANIFEST: &str = "oscyl.json";
const PROGRAMS: &str = "programs/";
const EXTRAS: &str = "extras/";

#[derive(Debug, thiserror::Error)]
pub enum NativeError {
    #[error("library archive error: {0}")]
    Archive(String),
    #[error("not an Oscyl library: {0}")]
    NotNative(String),
    #[error("this library was made by a newer version of Oscyl (format {0})")]
    Newer(u32),
}

#[derive(Serialize, Deserialize)]
struct Manifest {
    format: u32,
    driver: String,
    programs: Vec<Meta>,
}

#[derive(Serialize, Deserialize)]
struct Meta {
    #[serde(default)]
    author: String,
    #[serde(default)]
    comment: String,
}

fn archive(e: impl std::fmt::Display) -> NativeError {
    NativeError::Archive(e.to_string())
}

fn program_name(i: usize) -> String {
    format!("{PROGRAMS}{i:04}.bin")
}

/// Write `lib`, made with driver `driver`.
pub fn write(w: impl Write + Seek, driver: &str, lib: &LibraryData) -> Result<(), NativeError> {
    let manifest = Manifest {
        format: FORMAT,
        driver: driver.to_string(),
        programs: lib
            .programs
            .iter()
            .map(|p| Meta {
                author: p.author.clone(),
                comment: p.comment.clone(),
            })
            .collect(),
    };
    let json = serde_json::to_vec_pretty(&manifest).map_err(archive)?;
    let mut zip = ZipWriter::new(w);
    let opts = SimpleFileOptions::default();
    let mut member = |name: &str, data: &[u8]| -> Result<(), NativeError> {
        zip.start_file(name, opts).map_err(archive)?;
        zip.write_all(data).map_err(archive)
    };
    member(MANIFEST, &json)?;
    for (i, p) in lib.programs.iter().enumerate() {
        member(&program_name(i), &p.data)?;
    }
    for (name, data) in &lib.extras {
        member(&format!("{EXTRAS}{name}"), data)?;
    }
    zip.finish().map_err(archive)?;
    Ok(())
}

/// Read a native library: its driver id and contents.
pub fn read(r: impl Read + Seek) -> Result<(String, LibraryData), NativeError> {
    let mut zip = ZipArchive::new(r).map_err(|e| NativeError::NotNative(e.to_string()))?;
    let mut files = BTreeMap::new();
    for i in 0..zip.len() {
        let mut f = zip.by_index(i).map_err(archive)?;
        if f.is_dir() {
            continue;
        }
        let mut data = Vec::new();
        f.read_to_end(&mut data).map_err(archive)?;
        files.insert(f.name().map_err(archive)?.into_owned(), data);
    }
    let json = files
        .remove(MANIFEST)
        .ok_or_else(|| NativeError::NotNative(format!("missing {MANIFEST}")))?;
    let version: serde_json::Value =
        serde_json::from_slice(&json).map_err(|e| NativeError::NotNative(e.to_string()))?;
    match version.get("format").and_then(serde_json::Value::as_u64) {
        Some(f) if f > FORMAT as u64 => return Err(NativeError::Newer(f as u32)),
        Some(_) => {}
        None => return Err(NativeError::NotNative("no format version".into())),
    }
    let m: Manifest =
        serde_json::from_value(version).map_err(|e| NativeError::NotNative(e.to_string()))?;
    let programs = m
        .programs
        .into_iter()
        .enumerate()
        .map(|(i, meta)| {
            let data = files
                .remove(&program_name(i))
                .ok_or_else(|| NativeError::NotNative(format!("missing {}", program_name(i))))?;
            Ok(LibProgram {
                data,
                author: meta.author,
                comment: meta.comment,
            })
        })
        .collect::<Result<Vec<_>, NativeError>>()?;
    let extras = files
        .into_iter()
        .filter_map(|(n, d)| Some((n.strip_prefix(EXTRAS)?.to_string(), d)))
        .collect();
    Ok((m.driver, LibraryData { programs, extras }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use oscyl_driver::{LibProgram, LibraryData};
    use std::io::{Cursor, Write};
    use zip::write::SimpleFileOptions;

    fn sample() -> LibraryData {
        let mut lib = LibraryData::default();
        for (i, a) in ["Ann", "Bob"].iter().enumerate() {
            lib.programs.push(LibProgram {
                data: vec![i as u8; 4],
                author: a.to_string(),
                comment: format!("c{i} <&>"),
            });
        }
        lib.extras
            .insert("FileInformation.xml".into(), b"<x/>".to_vec());
        lib.extras
            .insert("deep/nested/member.bin".into(), vec![0, 255]);
        lib
    }

    fn zip_with(members: &[(&str, &[u8])]) -> Vec<u8> {
        let mut z = zip::ZipWriter::new(Cursor::new(Vec::new()));
        for (name, data) in members {
            z.start_file(*name, SimpleFileOptions::default()).unwrap();
            z.write_all(data).unwrap();
        }
        z.finish().unwrap().into_inner()
    }

    #[test]
    fn round_trip() {
        let mut out = Cursor::new(Vec::new());
        write(&mut out, "korg-minilogue-xd", &sample()).unwrap();
        out.set_position(0);
        let (driver, lib) = read(out).unwrap();
        assert_eq!(driver, "korg-minilogue-xd");
        assert_eq!(lib, sample());
    }

    #[test]
    fn newer_format_is_refused() {
        let z = zip_with(&[(MANIFEST, br#"{"format":2,"driver":"x","programs":[]}"#)]);
        assert!(matches!(read(Cursor::new(z)), Err(NativeError::Newer(2))));
    }

    #[test]
    fn missing_manifest_is_not_native() {
        let z = zip_with(&[("other.txt", b"hi")]);
        assert!(matches!(
            read(Cursor::new(z)),
            Err(NativeError::NotNative(_))
        ));
    }

    #[test]
    fn program_count_mismatch_is_rejected() {
        let z = zip_with(&[
            (
                MANIFEST,
                br#"{"format":1,"driver":"x","programs":[{"author":"","comment":""},{"author":"","comment":""}]}"#,
            ),
            ("programs/0000.bin", &[1]),
        ]);
        assert!(matches!(
            read(Cursor::new(z)),
            Err(NativeError::NotNative(_))
        ));
    }
}
