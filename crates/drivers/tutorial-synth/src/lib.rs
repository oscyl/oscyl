//! The Tutorial Mono, a small invented synth, and its Oscyl driver.
//!
//! The tutorial "Your first driver" (docs/tutorials/first-driver.md) builds
//! this crate one file at a time. No app lists the Tutorial Mono, because
//! the crate is not in `oscyl_midi::DRIVERS`.

pub mod driver;
pub mod files;
pub mod msg;
pub mod program;
pub mod session;
pub mod sim;
