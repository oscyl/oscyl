//! Code the KORG logue-family drivers share (minilogue xd, prologue): SysEx
//! messages, the flash commit planner, Librarian and unit files, and, per
//! model, the constants that differ ([`Model`]).

mod error;
pub mod engine;
pub mod kinds;
pub mod lag;
pub mod library;
mod model;
pub mod msg;
pub mod session;
pub mod sim;
pub mod store;
pub mod unit;

#[cfg(test)]
mod testing;

pub use error::Error;
pub use model::Model;
