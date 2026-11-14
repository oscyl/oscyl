//! The prologue's constants for the shared logue-family code.

use oscyl_logue::library::Product;
use oscyl_logue::msg::Family;
use oscyl_logue::store::Store;
use oscyl_logue::unit::Platform;
use oscyl_logue::{Error, Model};

use crate::Program;
use crate::program::PROGRAM_LEN;

/// The prologue (both the -8 and the -16; nothing in SysEx tells them
/// apart) as a logue [`Model`].
pub struct PrologueModel;

impl Model for PrologueModel {
    const FAMILY: Family = Family {
        id: 0x4B,
        program_len: PROGRAM_LEN,
        global_len: 32,
        foreign: "not a prologue message",
    };
    /// 12 programs per 4 KB sector, 499's special case
    ///.
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
    const NAME: &'static str = "prologue";
    /// The idle flush re-reads its group into the 0x1C reply buffer (§3.2).
    const RACE_REPLY_VALID: bool = true;

    type Program = Program;
    type Error = Error;

    fn parse(bytes: &[u8]) -> Result<Program, Error> {
        Program::from_bytes(bytes)
    }

    fn bytes(p: &Program) -> &[u8] {
        p.as_bytes()
    }

    fn init() -> Program {
        Program::init()
    }
}
