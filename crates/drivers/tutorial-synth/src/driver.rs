// ANCHOR: statics
//! The Tutorial Mono behind the driver contract: what the apps see.

use oscyl_driver::{
    Bank, Capabilities, Control, Curation, Driver, DriverError, DriverSession, FieldInfo,
    FileFormat, Info, LibraryData, NameRules, Simulator, curation_by_group, format_raw_default,
};

use crate::msg::{SLOTS, is_identity};
use crate::program::{self, FIELDS, WAVES};
use crate::{files, session, sim};

/// The Tutorial Mono's driver. Apps see it as `&dyn Driver`.
pub struct TutorialMono;

pub static DRIVER: TutorialMono = TutorialMono;

static INFO: Info = Info {
    id: "oscyl-tutorial-mono",
    vendor: "Oscyl",
    model: "Tutorial Mono",
};

/// One bank of 16 slots, all of them writable, and an edit buffer.
static CAPABILITIES: Capabilities = Capabilities {
    banks: &[Bank {
        name: "User",
        first: 0,
        len: SLOTS,
        writable: true,
    }],
    edit_buffer: true,
    live_changes: true,
    units: &[],
};

const SYX: FileFormat = FileFormat {
    ext: "syx",
    name: "Tutorial Mono program dumps",
    keeps_metadata: false,
};
// ANCHOR_END: statics

// ANCHOR: program
impl Driver for TutorialMono {
    fn info(&self) -> &'static Info {
        &INFO
    }

    fn capabilities(&self) -> &'static Capabilities {
        &CAPABILITIES
    }

    fn fields(&self) -> &'static [FieldInfo] {
        &FIELDS
    }

    fn name_rules(&self) -> NameRules {
        NameRules {
            max_len: program::NAME_LEN,
            allowed: program::allowed,
        }
    }

    fn init_program(&self) -> Vec<u8> {
        program::init()
    }

    fn check_program(&self, bytes: &[u8]) -> Result<(), DriverError> {
        program::check(bytes)
    }

    fn get(&self, bytes: &[u8], field: usize) -> Result<u16, DriverError> {
        program::get(bytes, field)
    }

    fn set(&self, bytes: &mut [u8], field: usize, value: u16) -> Result<(), DriverError> {
        program::set(bytes, field, value)
    }

    fn name(&self, bytes: &[u8]) -> Result<String, DriverError> {
        program::name(bytes)
    }

    fn set_name(&self, bytes: &mut [u8], name: &str) -> Result<(), DriverError> {
        program::set_name(bytes, name)
    }
    // ANCHOR_END: program

    // ANCHOR: files
    fn file_formats(&self) -> &'static [FileFormat] {
        &[SYX]
    }

    fn import(&self, ext: &str, bytes: &[u8]) -> Result<LibraryData, DriverError> {
        syx(ext)?;
        files::import(bytes)
    }

    fn export(&self, ext: &str, lib: &LibraryData) -> Result<Vec<u8>, DriverError> {
        syx(ext)?;
        files::export(lib)
    }
    // ANCHOR_END: files

    // ANCHOR: editor
    /// One editor page for each group of fields.
    fn curation(&self) -> Curation {
        curation_by_group(self.fields())
    }

    /// Wave picks one of four names. Glide is a switch.
    fn control(&self, _bytes: &[u8], field: usize) -> Control {
        match field {
            0 => Control::Choice(&WAVES),
            1 => Control::Toggle,
            _ => Control::Continuous,
        }
    }

    fn format_raw(&self, field: usize, raw: u16) -> String {
        match (field, FIELDS.get(field)) {
            (0, _) => WAVES.get(usize::from(raw)).copied().unwrap_or("?").into(),
            (_, Some(f)) => format_raw_default(f, raw),
            (_, None) => String::new(),
        }
    }
    // ANCHOR_END: editor

    // ANCHOR: live
    /// One Control Change for each field, so a change sounds at once on
    /// the notes that play.
    fn live_messages(&self, bytes: &[u8], field: usize, channel: u8) -> Option<Vec<Vec<u8>>> {
        const CC: [u8; 6] = [70, 65, 74, 71, 73, 72];
        let value = program::get(bytes, field).ok()?;
        // Glide is a switch: MIDI reads 64 or more as on.
        let value = if field == 1 { value * 127 } else { value };
        Some(vec![vec![0xB0 | (channel & 0x0F), CC[field], value as u8]])
    }
    // ANCHOR_END: live

    // ANCHOR: link
    /// The Tutorial Mono's SysEx has no channel byte, so `channel` is unused.
    fn session(&self, _channel: u8) -> Box<dyn DriverSession> {
        Box::new(session::Session::default())
    }

    fn simulator(&self) -> Box<dyn Simulator> {
        Box::new(sim::Sim::default())
    }

    fn port_hint(&self) -> &'static [&'static str] {
        &["Tutorial Mono"]
    }

    fn recognize(&self, identity_reply: &[u8]) -> bool {
        is_identity(identity_reply)
    }
}

/// Refuses every file extension but `syx`.
fn syx(ext: &str) -> Result<(), DriverError> {
    if ext == SYX.ext {
        Ok(())
    } else {
        Err(DriverError(format!(
            "the Tutorial Mono has no .{ext} files"
        )))
    }
}
// ANCHOR_END: link
