//! `.syx` files: program dumps (`02`) back to back, as the panel's Dump
//! Bank and Sequential's own files write them.

use oscyl_driver::sysex::Framer;
use oscyl_driver::{LibProgram, LibraryData};

use crate::msg::{Reply, SLOTS, decode, encode_program_dump};
use crate::{Error, Program};

/// Every program dump in `bytes`, in file order; slot numbers are ignored.
/// Author and comment stay empty: the format has no place for them.
pub fn import(bytes: &[u8]) -> Result<LibraryData, Error> {
    let programs = Framer::new()
        .push(bytes)
        .iter()
        .map(|m| match decode(m)? {
            Reply::Program(_, p) => Ok(LibProgram {
                data: p.as_bytes().to_vec(),
                ..Default::default()
            }),
            _ => Err(Error(
                "the file holds a message that is not a program dump".into(),
            )),
        })
        .collect::<Result<Vec<_>, _>>()?;
    if programs.is_empty() {
        return Err(Error("no Prophet Rev2 program dumps in the file".into()));
    }
    Ok(LibraryData {
        programs,
        extras: Default::default(),
    })
}

/// The programs as dumps numbered from slot 0 (U1-1). Author, comment
/// and extras are not written.
pub fn export(lib: &LibraryData) -> Result<Vec<u8>, Error> {
    if lib.programs.len() > usize::from(SLOTS) {
        return Err(Error(format!(
            "a .syx file holds at most {SLOTS} programs, got {}",
            lib.programs.len()
        )));
    }
    let mut out = vec![];
    for (n, p) in (0..).zip(&lib.programs) {
        out.extend(encode_program_dump(n, &Program::from_bytes(&p.data)?));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Program;
    use crate::msg::{encode_edit_request, encode_program_dump};

    fn named(name: &str) -> Vec<u8> {
        let mut p = Program::init();
        p.set_name(name).unwrap();
        p.as_bytes().to_vec()
    }

    fn lib(names: &[&str]) -> LibraryData {
        LibraryData {
            programs: names
                .iter()
                .map(|n| LibProgram {
                    data: named(n),
                    ..Default::default()
                })
                .collect(),
            extras: Default::default(),
        }
    }

    #[test]
    fn syx_round_trip() {
        let l = lib(&["One", "Two", "Three"]);
        let bytes = export(&l).unwrap();
        assert_eq!(bytes.len(), 3 * 2346);
        let p = Program::from_bytes(&named("Two")).unwrap();
        assert_eq!(bytes[2346..2 * 2346], encode_program_dump(1, &p)[..]);
        assert_eq!(import(&bytes).unwrap(), l);
    }

    #[test]
    fn import_keeps_file_order_and_ignores_slots() {
        let a = Program::from_bytes(&named("A")).unwrap();
        let b = Program::from_bytes(&named("B")).unwrap();
        let file = [encode_program_dump(900, &a), encode_program_dump(3, &b)].concat();
        assert_eq!(import(&file).unwrap(), lib(&["A", "B"]));
    }

    #[test]
    fn import_rejects_other_files() {
        assert!(import(&[]).is_err());
        assert!(import(b"not sysex").is_err());
        assert!(import(&encode_edit_request()).is_err());
        let mut bad = encode_program_dump(0, &Program::init());
        bad.remove(100);
        assert!(import(&bad).is_err());
    }

    #[test]
    fn export_refuses_more_than_1024_programs() {
        let names = vec!["X"; 1025];
        assert!(export(&lib(&names)).is_err());
        assert!(
            export(&LibraryData {
                programs: vec![LibProgram::default()],
                extras: Default::default(),
            })
            .is_err()
        );
    }
}
