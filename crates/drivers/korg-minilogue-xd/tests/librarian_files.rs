//! Reads a real Librarian file named by `OSCYL_LIBRARIAN_FILE`, if set, and
//! checks that it parses and survives a write/read round trip unchanged.
//! Such files are not committed (KORG or user content).

use std::io::Cursor;

use oscyl_minilogue_xd::Field;
use oscyl_minilogue_xd::library::Library;

#[test]
fn real_librarian_file_round_trips() {
    let Ok(path) = std::env::var("OSCYL_LIBRARIAN_FILE") else {
        return;
    };
    let lib = Library::read(std::fs::File::open(&path).unwrap()).unwrap();
    let programs = lib.programs().unwrap();
    eprintln!(
        "{path}: {} programs, {} members",
        programs.len(),
        lib.files.len()
    );
    let mut bad = std::collections::BTreeMap::<&str, usize>::new();
    for (p, _) in &programs {
        for &f in Field::ALL {
            let (v, s) = (p.get(f), f.spec());
            if v < s.min || v > s.max {
                *bad.entry(s.name).or_default() += 1;
            }
        }
    }
    // Reported, not asserted: KORG's own factory library has one program with
    // Arp Gate Time = 77 (documented range 0-72). Reading must still work.
    if !bad.is_empty() {
        eprintln!("fields out of documented range (field: program count): {bad:?}");
    }
    let mut out = Cursor::new(Vec::new());
    lib.write(&mut out).unwrap();
    let back = Library::read(Cursor::new(out.into_inner())).unwrap();
    assert_eq!(back, lib);
}
