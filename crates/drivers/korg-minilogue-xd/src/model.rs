//! The minilogue xd's constants for the shared logue-family code.

use oscyl_logue::Model;
use oscyl_logue::library::Product;
use oscyl_logue::msg::Family;
use oscyl_logue::store::Store;
use oscyl_logue::unit::Platform;

use crate::{Error, Program};

/// The minilogue xd as a logue [`Model`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct XdModel;

impl Model for XdModel {
    const FAMILY: Family = crate::msg::FAMILY;
    const STORE: Store = crate::store::STORE;
    const PLATFORM: Platform = crate::unit::PLATFORM;
    const PRODUCT: Product = Product {
        name: "minilogue xd",
        info_root: "xd_ProgramInformation",
        counts: &[
            "FavoriteData",
            "ProgramData",
            "PresetInformation",
            "TuneScaleData",
            "TuneOctData",
        ],
    };
    const NAME: &'static str = "minilogue xd";
    /// Hardware showed a marker-less reply.
    const RACE_REPLY_VALID: bool = false;

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
