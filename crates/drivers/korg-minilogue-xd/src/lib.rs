//! Sans-IO protocol, data model and file formats for the KORG minilogue xd.

pub mod adapter;
pub mod demo;
pub mod driver;
mod error;
pub mod fields;
pub mod generate;
pub mod help;
pub mod library;
pub mod live;
pub mod model;
pub mod msg;
pub mod present;
pub mod program;
pub mod session;
pub mod sim;
pub mod store;
pub mod sync;
pub mod unit;
pub mod viz;

pub use driver::DRIVER;
pub use error::Error;
pub use fields::{Field, FieldSpec, Group};
pub use oscyl_driver::{pack, sysex};
pub use program::Program;
