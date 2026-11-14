//! What one logue model tells the shared code.

use std::fmt::{Debug, Display};

use crate::Error;
use crate::library::Product;
use crate::msg::Family;
use crate::store::Store;
use crate::unit::Platform;

/// One logue-family synth model: its constants and its program type. The
/// shared messages, store, files, sessions and simulator are generic over
/// it.
pub trait Model: Send + Sync + 'static {
    /// SysEx family id and dump sizes.
    const FAMILY: Family;
    /// Flash group size and special-case program.
    const STORE: Store;
    /// logue SDK platform, for unit files and uploads.
    const PLATFORM: Platform;
    /// Librarian file names.
    const PRODUCT: Product;
    /// The model's name for people, as in "power-cycle the minilogue xd".
    const NAME: &'static str;
    /// Whether the reply that races a deferred save can look like a valid
    /// program (prologue: the flush re-reads its group into the reply
    /// buffer) instead of an empty one (xd).
    const RACE_REPLY_VALID: bool;

    type Program: Clone + PartialEq + Eq + Debug + Send + 'static;
    type Error: From<Error> + Display + Debug;

    /// A program from its unpacked bytes, checked.
    fn parse(bytes: &[u8]) -> Result<Self::Program, Self::Error>;
    fn bytes(p: &Self::Program) -> &[u8];
    fn init() -> Self::Program;
}
