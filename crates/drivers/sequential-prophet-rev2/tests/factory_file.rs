//! Checks the driver against Sequential's factory programs. The file is
//! vendor content and never committed: point `OSCYL_REV2_FACTORY_SYX` at
//! `Rev2_Programs_v1.0.syx`, or keep it at the default path under the
//! gitignored `oem/`.

use std::path::PathBuf;

use oscyl_driver::sysex::Framer;
use oscyl_sequential_prophet_rev2::fields;
use oscyl_sequential_prophet_rev2::msg::{Reply, decode, encode_program_dump};

#[test]
#[ignore = "needs the vendor's factory .syx file, local only"]
fn factory_programs_parse_in_range_and_re_encode_identically() {
    let path = std::env::var_os("OSCYL_REV2_FACTORY_SYX")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(
                "../../../oem/candidates/sequential-prophet-rev2/factory/unzipped/Rev2_Programs_v1.0.syx",
            )
        });
    let Ok(bytes) = std::fs::read(&path) else {
        println!("skipped: no factory file at {}", path.display());
        return;
    };
    let msgs = Framer::new().push(&bytes);
    assert_eq!(msgs.len(), 512);
    for (i, msg) in (0u16..).zip(&msgs) {
        let Ok(Reply::Program(slot, p)) = decode(msg) else {
            panic!("message {i} is not a program dump");
        };
        assert_eq!(slot, i, "factory programs are U1-U4 in order");
        for f in fields::all() {
            let v = p.get(f);
            assert!(
                (f.min..=f.max).contains(&v),
                "slot {slot}: {} = {v}",
                f.name
            );
        }
        assert_eq!(encode_program_dump(slot, &p), *msg, "slot {slot}");
    }
    println!(
        "{} factory programs: every field in range, re-encode identical",
        msgs.len()
    );
}
