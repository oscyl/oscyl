//! Compares our unit header builder with the `waves` unit read back from
//! osc slot 9 into a local backup (git-ignored), if present.

use std::path::Path;

use oscyl_minilogue_xd::library::Library;
use oscyl_minilogue_xd::unit::{HEADER_LEN, build_header};

#[test]
fn header_matches_unit_stored_on_synth() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../oem/backups/before.mnlgxdlib");
    let Ok(f) = std::fs::File::open(path) else {
        return;
    };
    let lib = Library::read(f).unwrap();
    let Some(slot) = lib.files.get("oscyl/units/osc_08.bin") else {
        return;
    };
    let stored = &slot[8..8 + HEADER_LEN];
    // The stored unit was built with SDK api 1.0-0 and version 1.0-0, and a
    // 0xC00-byte load size; everything else comes from the same manifest.
    let manifest = serde_json::json!({"header": {
        "platform": "minilogue-xd", "module": "osc", "api": "1.0-0",
        "dev_id": 0, "prg_id": 0, "version": "1.0-0", "name": "waves",
        "params": [["Wave A", 0, 45, ""], ["Wave B", 0, 43, ""], ["Sub Wave", 0, 15, ""],
                   ["Sub Mix", 0, 100, "%"], ["Ring Mix", 0, 100, "%"], ["Bit Crush", 0, 100, "%"]]
    }});
    let (_, _, ours) = build_header(&manifest, 0xC00 - 0x84).unwrap();
    let diffs: Vec<usize> = (0..HEADER_LEN).filter(|&i| ours[i] != stored[i]).collect();
    assert!(diffs.is_empty(), "header differs at {diffs:x?}");
}
