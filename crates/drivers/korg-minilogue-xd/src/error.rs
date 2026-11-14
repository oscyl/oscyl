use thiserror::Error;

/// Errors produced by `oscyl-minilogue-xd`.
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
    /// A program name longer than 12 characters or using a character the
    /// synth cannot display.
    #[error("invalid program name: {0:?}")]
    InvalidName(String),
    /// A file that is not a minilogue xd library, program or preset.
    #[error("not a minilogue xd library: {0}")]
    NotALibrary(String),
    /// An I/O or ZIP error while reading or writing a library file.
    #[error("library archive error: {0}")]
    Archive(String),
}

/// The shared crate's errors, under the xd's names and messages.
impl From<oscyl_logue::Error> for Error {
    fn from(e: oscyl_logue::Error) -> Error {
        use oscyl_logue::Error as E;
        match e {
            E::NotSevenBit(b) => Error::NotSevenBit(b),
            E::Malformed(m) => Error::Malformed(m),
            E::Length { expected, got } => Error::Length { expected, got },
            E::OutOfRange {
                field,
                value,
                min,
                max,
            } => Error::OutOfRange {
                field,
                value,
                min,
                max,
            },
            E::InvalidName(n) => Error::InvalidName(n),
            E::NotALibrary(m) => Error::NotALibrary(m),
            E::Archive(m) => Error::Archive(m),
        }
    }
}

impl From<oscyl_driver::pack::NotSevenBit> for Error {
    fn from(e: oscyl_driver::pack::NotSevenBit) -> Error {
        Error::NotSevenBit(e.0)
    }
}
