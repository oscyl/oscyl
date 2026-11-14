//! KORG prologue driver: a 336-byte program with main and sub timbres, its
//! editor curation, CC live edits, generator and graphs, on the
//! logue-family code it shares with the minilogue xd (`oscyl-logue`):
//! messages, the flash commit planner (groups of 12), sessions with the
//! read-lag check, Librarian files and the simulator.

pub mod curate;
pub mod driver;
pub mod fields;
pub mod generate;
pub mod live;
pub mod model;
pub mod program;
pub mod viz;

pub use driver::DRIVER;
pub use program::Program;
