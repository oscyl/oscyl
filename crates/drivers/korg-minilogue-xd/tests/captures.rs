//! Decodes every local hardware capture, if any exist.
//! Captures are git-ignored (they hold the owner's programs), so these tests
//! are no-ops on CI and in fresh clones.

use std::path::{Path, PathBuf};

use oscyl_korg_minilogue_xd::msg::{Dump, Reply, decode};
use oscyl_korg_minilogue_xd::{Field, Program};

fn captures() -> Vec<PathBuf> {
    let runs = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../private/research/probes/runs");
    let Ok(dirs) = std::fs::read_dir(runs) else {
        return vec![];
    };
    let mut out = vec![];
    for dir in dirs.flatten() {
        for f in std::fs::read_dir(dir.path()).unwrap().flatten() {
            if f.path().extension().is_some_and(|e| e == "syx") {
                out.push(f.path());
            }
        }
    }
    out
}

#[test]
fn decodes_all_local_captures() {
    for f in captures() {
        let bytes = std::fs::read(&f).unwrap();
        decode(&bytes).unwrap_or_else(|e| panic!("{}: {e}", f.display()));
    }
}

#[test]
fn captured_programs_parse_and_are_in_range() {
    let mut n = 0;
    for f in captures() {
        let Ok(Reply::Dump(Dump::Program(_, data) | Dump::CurrentProgram(data))) =
            decode(&std::fs::read(&f).unwrap())
        else {
            continue;
        };
        let p = Program::from_bytes(&data).unwrap_or_else(|e| panic!("{}: {e}", f.display()));
        assert_eq!(p.as_bytes()[..], data[..]);
        for &field in Field::ALL {
            let s = field.spec();
            let v = p.get(field);
            assert!(
                (s.min..=s.max).contains(&v),
                "{}: {} = {v}",
                f.display(),
                s.name
            );
        }
        n += 1;
    }
    eprintln!("checked {n} captured programs");
}
