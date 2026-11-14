use thiserror::Error;

/// Errors produced by `oscyl-logue`. Drivers convert them into their own
/// error type ([`crate::Model::Error`]) so their messages can name the synth.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum Error {
    /// A byte that must be 7-bit had its high bit set.
    #[error("byte {0:#04x} is not 7-bit")]
    NotSevenBit(u8),
    /// A message did not have the expected structure.
    #[error("malformed message: {0}")]
    Malformed(&'static str),
    /// A payload had the wrong size.
    #[error("expected {expected} bytes, got {got}")]
    Length { expected: usize, got: usize },
    /// A parameter value outside its documented range.
    #[error("{field} = {value} is outside {min}..={max}")]
    OutOfRange {
        field: &'static str,
        value: u16,
        min: u16,
        max: u16,
    },
    /// A program name too long or using a character the synth cannot show.
    #[error("invalid program name: {0:?}")]
    InvalidName(String),
    /// A file that is not a library, program or unit for this synth.
    #[error("not a library for this synth: {0}")]
    NotALibrary(String),
    /// An I/O or ZIP error while reading or writing an archive.
    #[error("library archive error: {0}")]
    Archive(String),
}

impl From<oscyl_driver::pack::NotSevenBit> for Error {
    fn from(e: oscyl_driver::pack::NotSevenBit) -> Error {
        Error::NotSevenBit(e.0)
    }
}
