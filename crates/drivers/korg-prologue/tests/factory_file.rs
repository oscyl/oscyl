//! Checks the driver against KORG's factory library. The files are vendor
//! content and never committed: point `OSCYL_PROLOGUE_FACTORY_DIR` at the
//! extracted members of the factory `.prlglib` (`FileInformation.xml`,
//! `Prog_NNN.prog_bin`, …), or keep them at the default path under the
//! gitignored `oem/`.

use std::io::Write;
use std::path::PathBuf;

use oscyl_driver::Driver;
use oscyl_korg_prologue::{DRIVER, Program, fields};

#[test]
#[ignore = "needs the vendor's factory library, local only"]
fn factory_programs_parse_in_range_and_round_trip() {
    let dir = std::env::var_os("OSCYL_PROLOGUE_FACTORY_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../../oem/candidates/korg-prologue/extracted-factory/x")
        });
    let Ok(entries) = std::fs::read_dir(&dir) else {
        println!("skipped: no factory library at {}", dir.display());
        return;
    };
    // Zip the members back into a .prlglib, as the Librarian ships it.
    let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let mut names: Vec<_> = entries.map(|e| e.unwrap().path()).collect();
    names.sort();
    for path in &names {
        let name = path.file_name().unwrap().to_str().unwrap();
        zip.start_file(name, zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(&std::fs::read(path).unwrap()).unwrap();
    }
    let bytes = zip.finish().unwrap().into_inner();

    let lib = DRIVER
        .import("prlglib", &bytes)
        .expect("factory library imports");
    assert_eq!(lib.programs.len(), 500);
    let mut out_of_range = vec![];
    for (i, p) in lib.programs.iter().enumerate() {
        DRIVER.check_program(&p.data).expect("valid");
        let prog = Program::from_bytes(&p.data).unwrap();
        for (k, f) in fields::all().iter().enumerate() {
            let v = prog.get(f);
            if !(f.min..=f.max).contains(&v) {
                out_of_range.push(format!("program {i}: {} = {v}", f.name));
            }
            assert!(!DRIVER.format_value(&p.data, k).is_empty());
        }
    }
    assert!(out_of_range.is_empty(), "{out_of_range:#?}");
    let again = DRIVER
        .import("prlglib", &DRIVER.export("prlglib", &lib).unwrap())
        .unwrap();
    assert_eq!(again, lib, "export then import changes nothing");
    let not_set = lib
        .programs
        .iter()
        .filter(|p| p.data[51..59].iter().all(|b| *b == 0))
        .count();
    println!(
        "{} factory programs: valid, every field in range, round trip identical; \
         {not_set} hold 1.x \"not set\" 2.00 fields",
        lib.programs.len()
    );
}
