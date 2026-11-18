//! `.syx` files: program dumps one after the other, as the synth sends
//! them. The format has no place for a program's author or comment.

use oscyl_driver::sysex::Framer;
use oscyl_driver::{DriverError, LibProgram, LibraryData};

use crate::msg::{Msg, SLOTS};
use crate::program;

/// Every program dump in `bytes`, in file order.
pub fn import(bytes: &[u8]) -> Result<LibraryData, DriverError> {
    let mut programs = vec![];
    for m in Framer::new().push(bytes) {
        let Some(Msg::ProgramDump(_, data)) = Msg::decode(&m) else {
            return Err(DriverError(
                "the file holds a message that is not a program dump".into(),
            ));
        };
        program::check(&data)?;
        programs.push(LibProgram {
            data,
            ..LibProgram::default()
        });
    }
    if programs.is_empty() {
        return Err(DriverError(
            "the file holds no Tutorial Mono programs".into(),
        ));
    }
    Ok(LibraryData {
        programs,
        extras: Default::default(),
    })
}

/// The programs of `lib` as dumps for slots 0, 1, 2 and on.
pub fn export(lib: &LibraryData) -> Result<Vec<u8>, DriverError> {
    if lib.programs.len() > usize::from(SLOTS) {
        return Err(DriverError(format!(
            "a file holds at most {SLOTS} programs"
        )));
    }
    let mut out = vec![];
    for (n, p) in (0..).zip(&lib.programs) {
        program::check(&p.data)?;
        out.extend(Msg::ProgramDump(n, p.data.clone()).encode());
    }
    Ok(out)
}
