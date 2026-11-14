//! logue SDK unit files (`.mnlgxdunit`) and the slot data the synth stores.
//!
//! A unit file is a ZIP holding `<name>/manifest.json` and
//! `<name>/payload.bin`. The synth stores a blob of
//! `header (0x400) + payload + signature area (0x84 zero bytes)`, where the
//! header is the SDK's `user_prg_header_t` built from the manifest. Layout
//! confirmed against a unit read back from hardware
//! (a user slot's data, read back). The
//! logic is the logue family's ([`oscyl_logue::unit`]) for the xd's
//! [`PLATFORM`].

use std::io::{Read, Seek};

use oscyl_logue::unit::Platform;
use serde_json::Value;

use crate::Error;
use crate::msg::Module;

pub use oscyl_logue::unit::{HEADER_LEN, SIG_LEN, unit_name};

/// The minilogue xd's logue SDK platform.
pub const PLATFORM: Platform = Platform {
    name: "minilogue-xd",
    id: 2,
};

/// A parsed unit file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unit {
    pub module: Module,
    pub name: String,
    pub header: Vec<u8>,
    pub payload: Vec<u8>,
}

/// Build `user_prg_header_t` (little-endian, packed) from a manifest.
pub fn build_header(
    manifest: &Value,
    payload_len: usize,
) -> Result<(Module, String, Vec<u8>), Error> {
    Ok(oscyl_logue::unit::build_header(
        &PLATFORM,
        manifest,
        payload_len,
    )?)
}

impl Unit {
    pub fn read(r: impl Read + Seek) -> Result<Unit, Error> {
        let u = oscyl_logue::unit::Unit::read(&PLATFORM, r)?;
        Ok(Unit {
            module: u.module,
            name: u.name,
            header: u.header,
            payload: u.payload,
        })
    }

    /// The blob the synth stores: header, payload, zeroed signature area.
    pub fn blob(&self) -> Vec<u8> {
        oscyl_logue::unit::blob(&self.header, &self.payload)
    }

    /// TABLE 7 slot data for upload: `u32 size, u32 checksum, blob`. The
    /// checksum is sent as 0, as the official Librarian does; the algorithm
    /// the synth reports on read-back is not known yet.
    pub fn slot_data(&self) -> Vec<u8> {
        oscyl_logue::unit::slot_data(&self.blob())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_name_reads_the_padded_record_field() {
        let mut header = vec![0u8; 32];
        header[18..22].copy_from_slice(b"Wave");
        assert_eq!(unit_name(&header), "Wave");
        assert_eq!(unit_name(&[1, 2]), "");
    }
    use serde_json::json;

    fn waves() -> Value {
        json!({"header": {
            "platform": "minilogue-xd", "module": "osc", "api": "1.1-0",
            "dev_id": 0, "prg_id": 0, "version": "1.0-1", "name": "waves", "num_param": 6,
            "params": [["Wave A", 0, 45, ""], ["Wave B", 0, 43, ""], ["Sub Wave", 0, 15, ""],
                       ["Sub Mix", 0, 100, "%"], ["Ring Mix", 0, 100, "%"], ["Bit Crush", 0, 100, "%"]]
        }})
    }

    #[test]
    fn header_matches_hardware_layout() {
        let (module, name, h) = build_header(&waves(), 2016).unwrap();
        assert_eq!((module, name.as_str()), (Module::Osc, "waves"));
        assert_eq!(h.len(), HEADER_LEN);
        // Byte layout as read back from osc slot 9 on the synth.
        assert_eq!(&h[0..2], &[0x04, 0x02]);
        assert_eq!(&h[2..6], &[0x00, 0x01, 0x01, 0x00]); // api 1.1-0
        assert_eq!(&h[14..18], &[0x01, 0x00, 0x01, 0x00]); // version 1.0-1
        assert_eq!(&h[18..24], b"waves\0");
        assert_eq!(&h[32..36], &[6, 0, 0, 0]);
        assert_eq!(
            &h[36..45],
            &[0x00, 0x2D, 0x02, b'W', b'a', b'v', b'e', b' ', b'A']
        );
        assert_eq!(&h[84..90], &[0x00, 0x64, 0x00, b'S', b'u', b'b']);
        assert_eq!(&h[0x3FC..], &((2016 + SIG_LEN) as u32).to_le_bytes());
    }

    #[test]
    fn rejects_other_platforms_and_long_names() {
        let mut m = waves();
        m["header"]["platform"] = json!("prologue");
        assert!(build_header(&m, 0).is_err());
        let mut m = waves();
        m["header"]["name"] = json!("fourteen chars");
        assert!(build_header(&m, 0).is_err());
    }

    #[test]
    fn slot_data_layout() {
        let (module, name, header) = build_header(&waves(), 8).unwrap();
        let u = Unit {
            module,
            name,
            header,
            payload: b"UOSCxxxx".to_vec(),
        };
        let d = u.slot_data();
        assert_eq!(&d[0..4], &((HEADER_LEN + 8 + SIG_LEN) as u32).to_le_bytes());
        assert_eq!(&d[4..8], &[0; 4]);
        assert_eq!(&d[8 + HEADER_LEN..8 + HEADER_LEN + 4], b"UOSC");
        assert_eq!(d.len(), 8 + HEADER_LEN + 8 + SIG_LEN);
    }
}
