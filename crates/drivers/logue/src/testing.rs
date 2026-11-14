//! A prologue-shaped test model with 16-byte programs (`PROG`, 8 data
//! bytes, `PRED`, padding), so the shared code is tested with the
//! prologue's parameters without depending on its driver.

use crate::library::Product;
use crate::msg::Family;
use crate::store::Store;
use crate::unit::Platform;
use crate::{Error, Model};

pub struct Test;

pub const LEN: usize = 16;

impl Model for Test {
    const FAMILY: Family = Family {
        id: 0x4B,
        program_len: LEN,
        global_len: 32,
        foreign: "not a test message",
    };
    const STORE: Store = Store {
        group_size: 12,
        last: 499,
    };
    const PLATFORM: Platform = Platform {
        name: "prologue",
        id: 1,
    };
    const PRODUCT: Product = Product {
        name: "prologue",
        info_root: "prologue_ProgramInformation",
        counts: &["LivesetData", "ProgramData", "PresetInformation"],
    };
    const NAME: &'static str = "test synth";
    const RACE_REPLY_VALID: bool = true;

    type Program = Vec<u8>;
    type Error = Error;

    fn parse(b: &[u8]) -> Result<Vec<u8>, Error> {
        if b.len() != LEN {
            return Err(Error::Length {
                expected: LEN,
                got: b.len(),
            });
        }
        if &b[..4] != b"PROG" || &b[12..] != b"PRED" {
            return Err(Error::Malformed("missing program marker"));
        }
        Ok(b.to_vec())
    }

    fn bytes(p: &Vec<u8>) -> &[u8] {
        p
    }

    fn init() -> Vec<u8> {
        program(0)
    }
}

/// A valid program whose data bytes are `tag`.
pub fn program(tag: u8) -> Vec<u8> {
    [&b"PROG"[..], &[tag; 8], b"PRED"].concat()
}
