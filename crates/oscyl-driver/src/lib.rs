//! The contract every Oscyl synth driver implements, plus what drivers
//! share: 7-in-8 packing, SysEx framing and the one-in-flight request
//! engine. Sans-IO: time enters only as `Instant` arguments.

pub mod contract;
pub mod curation;
pub mod engine;
pub mod pack;
pub mod sysex;

pub use contract::{
    Bank, Capabilities, Driver, DriverError, DriverSession, FieldInfo, FileFormat, Info,
    LibProgram, LibraryData, NameRules, SessionEvent, SessionOp, Simulator, UnitKind,
};
pub use curation::*;
