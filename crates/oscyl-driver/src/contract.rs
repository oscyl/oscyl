//! What a driver tells Oscyl about one synth model.
//!
//! Programs are opaque bytes in the driver's own native layout; generic
//! code reads and writes them only through the driver. Field indices are
//! positions in [`Driver::fields`].

use std::borrow::Cow;
use std::collections::BTreeMap;
use std::time::Instant;

use crate::curation::*;

/// A driver-specific failure, as text the apps can show.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{0}")]
pub struct DriverError(pub String);

/// Which synth a driver serves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Info {
    /// Stable id: `<vendor>-<model>`, lowercase, dash-separated.
    pub id: &'static str,
    pub vendor: &'static str,
    pub model: &'static str,
}

/// A run of program slots, numbered from `first`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bank {
    pub name: &'static str,
    pub first: u16,
    pub len: u16,
    /// False for factory/ROM banks Oscyl must never write.
    pub writable: bool,
}

/// One kind of user unit (custom oscillator, effect, …) and its slot count.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnitKind {
    pub name: &'static str,
    pub slots: u8,
}

/// What the synth can do. Apps show only what is listed here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Capabilities {
    pub banks: &'static [Bank],
    /// Programs can be loaded into an edit buffer without storing them.
    pub edit_buffer: bool,
    /// Parameters can change while notes sound (CC/NRPN).
    pub live_changes: bool,
    pub units: &'static [UnitKind],
}

/// One program parameter: raw stored values `min..=max`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FieldInfo {
    pub name: &'static str,
    pub group: &'static str,
    pub min: u16,
    pub max: u16,
}

/// Program-name limits.
#[derive(Debug, Clone, Copy)]
pub struct NameRules {
    pub max_len: usize,
    pub allowed: fn(char) -> bool,
}

/// One program in a library, with its author metadata.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LibProgram {
    /// The driver's native program bytes.
    pub data: Vec<u8>,
    pub author: String,
    pub comment: String,
}

/// A library's contents, independent of any file format.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LibraryData {
    pub programs: Vec<LibProgram>,
    /// Driver-owned members (vendor entries Oscyl does not edit), kept
    /// verbatim so an export loses nothing. Names are the driver's.
    pub extras: BTreeMap<String, Vec<u8>>,
}

/// A vendor file format a driver reads and writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileFormat {
    /// File extension, without the dot.
    pub ext: &'static str,
    pub name: &'static str,
    /// Whether the format stores each program's author and comment. When
    /// false, import gives them empty.
    pub keeps_metadata: bool,
}

/// One synth model. Object-safe, so apps can hold several at once.
pub trait Driver: Send + Sync {
    fn info(&self) -> &'static Info;
    fn capabilities(&self) -> &'static Capabilities;
    fn fields(&self) -> &'static [FieldInfo];
    fn name_rules(&self) -> NameRules;
    fn init_program(&self) -> Vec<u8>;
    fn check_program(&self, program: &[u8]) -> Result<(), DriverError>;
    fn get(&self, program: &[u8], field: usize) -> Result<u16, DriverError>;
    fn set(&self, program: &mut [u8], field: usize, value: u16) -> Result<(), DriverError>;
    fn name(&self, program: &[u8]) -> Result<String, DriverError>;
    fn set_name(&self, program: &mut [u8], name: &str) -> Result<(), DriverError>;
    fn file_formats(&self) -> &'static [FileFormat];
    /// Read a vendor file of format `ext`.
    fn import(&self, ext: &str, bytes: &[u8]) -> Result<LibraryData, DriverError>;
    /// Write a vendor file of format `ext`; `import` of the result gives
    /// back `lib`.
    fn export(&self, ext: &str, lib: &LibraryData) -> Result<Vec<u8>, DriverError>;

    // Curation: data the apps render. Everything but `curation` has a
    // default that suits a driver with no curation of its own.

    /// Pages and sections of the editor. Required: a simple driver may
    /// return [`curation_by_group`]`(self.fields())`, but says so on
    /// purpose.
    fn curation(&self) -> Curation;
    /// The widget for `field`. It may depend on other fields of `program`
    /// (a depth that names chords in one mode and is a dial in another), so
    /// apps query it again after an edit and rebuild the control when it
    /// changes.
    fn control(&self, _program: &[u8], field: usize) -> Control {
        match self.fields().get(field) {
            Some(f) if f.max - f.min == 1 => Control::Toggle,
            _ => Control::Continuous,
        }
    }
    /// A compact label for use inside a titled section.
    fn short_label(&self, field: usize) -> &'static str {
        self.fields().get(field).map_or("", |f| f.name)
    }
    /// The label for `field` under `program`'s settings, when its job
    /// changes with another field (a depth that is a chord in one mode and
    /// a detune in another). Defaults to [`Driver::short_label`].
    fn label_in(&self, _program: &[u8], field: usize) -> Cow<'static, str> {
        Cow::Borrowed(self.short_label(field))
    }
    /// Whether `field` is one of its section's main controls, which apps
    /// draw larger.
    fn emphasized(&self, _field: usize) -> bool {
        false
    }
    /// `field`'s value in `program`, for people, read in the context of
    /// the program's other fields. Empty when unreadable.
    fn format_value(&self, program: &[u8], field: usize) -> String {
        self.get(program, field)
            .map(|v| self.format_raw(field, v))
            .unwrap_or_default()
    }
    /// `raw` for `field` without a program (range tooltips, before/after
    /// text). Values whose meaning depends on other fields show plainly.
    fn format_raw(&self, field: usize, raw: u16) -> String {
        self.fields()
            .get(field)
            .map(|f| format_raw_default(f, raw))
            .unwrap_or_default()
    }
    /// A one-line description of `page`'s state in `program`, or empty.
    fn page_summary(&self, _program: &[u8], _page: usize) -> String {
        String::new()
    }
    /// Pictures of `page` in `program`; curves use about `points` points.
    fn graphs(&self, _program: &[u8], _page: usize, _points: usize) -> Vec<Graph> {
        vec![]
    }
    /// Draggable points on `page`'s first graph.
    fn handles(&self, _program: &[u8], _page: usize) -> Vec<Handle> {
        vec![]
    }
    /// The field values that put handle `handle` of `page` at `at` (graph
    /// coordinates), only those that change.
    fn drag(
        &self,
        _program: &[u8],
        _page: usize,
        _handle: usize,
        _at: (f32, f32),
    ) -> Vec<(usize, u16)> {
        vec![]
    }
    /// Whether `field` does anything under `program`'s other settings;
    /// editors hide fields that do not.
    fn relevant(&self, _program: &[u8], _field: usize) -> bool {
        true
    }
    fn help(&self, _field: usize) -> Help {
        Help::default()
    }
    /// What kind of sound `program` is (for example "Bass"), if the driver
    /// can tell.
    fn category(&self, _program: &[u8]) -> Option<&'static str> {
        None
    }
    /// Every value [`Driver::category`] can return, in display order, for
    /// filters.
    fn categories(&self) -> &'static [&'static str] {
        &[]
    }

    // Generation. A driver with no characters does not generate.

    fn mutability(&self, _field: usize) -> Mutability {
        Mutability::Restricted
    }
    fn characters(&self) -> &'static [Character] {
        &[]
    }
    /// Titles and help for the four [`Macros`], in their order.
    fn macro_info(&self) -> [MacroInfo; 4] {
        MACROS
    }
    /// A name for a generated program of `character`, from `seed`.
    fn generated_name(&self, character: usize, seed: u64) -> String {
        let title = self
            .characters()
            .get(character)
            .map_or("Sound", |c| c.title);
        format!("{title} {}", seed % 1000)
    }
    /// A new program from `recipe`. Same recipe and base, same bytes.
    ///
    /// Without a base, non-free fields are the init program's. With
    /// `Some(base)`, every non-free ([`Mutability::Restricted`] or
    /// [`Mutability::Fixed`]) field is the base's, and every field on the
    /// pages in `recipe.keep` comes back byte for byte from the base, even
    /// a value out of range; only free fields on other pages are
    /// generated.
    fn generate(&self, _recipe: &Recipe, _base: Option<&[u8]>) -> Result<Vec<u8>, DriverError> {
        Err(no_generation())
    }
    /// A mutation of `program` by `amount` (`0.0..=1.0`). Only free fields
    /// change; restricted and fixed fields stay as they are.
    fn vary(&self, _program: &[u8], _amount: f32, _seed: u64) -> Result<Vec<u8>, DriverError> {
        Err(no_generation())
    }
    /// A child of `a` and `b`; `mix` 0 is `a`, 1 is `b`. Free fields mix;
    /// restricted and fixed fields are `a`'s, as they are. Out-of-range
    /// parent values are clamped where they mix.
    fn breed(&self, _a: &[u8], _b: &[u8], _mix: f32, _seed: u64) -> Result<Vec<u8>, DriverError> {
        Err(no_generation())
    }

    /// The MIDI messages that set `field` to its value in `program` on the
    /// sounding voices, in order, one complete message per inner `Vec`: a
    /// transport sends each one separately, since some backends send only
    /// the first event of a buffer. Callers pass `channel` 0-15. `None`
    /// when only a whole-program audition can make the change. Only with
    /// [`Capabilities::live_changes`].
    fn live_messages(&self, _program: &[u8], _field: usize, _channel: u8) -> Option<Vec<Vec<u8>>> {
        None
    }

    // Sessions.

    fn session(&self, channel: u8) -> Box<dyn DriverSession>;
    fn simulator(&self) -> Box<dyn Simulator>;
    /// Substrings of the MIDI port names this synth shows up as.
    fn port_hint(&self) -> &'static [&'static str];
    /// Whether `identity_reply`, an answer to
    /// [`crate::sysex::DEVICE_INQUIRY`], comes from this synth.
    fn recognize(&self, identity_reply: &[u8]) -> bool;

    // What apps show around the editor. All optional.

    /// A library to explore without a synth. By default one generated
    /// program per character (none without characters).
    fn demo_library(&self) -> LibraryData {
        let programs = (0..self.characters().len())
            .filter_map(|character| {
                let recipe = Recipe {
                    character,
                    seed: character as u64,
                    ..Recipe::default()
                };
                self.generate(&recipe, None).ok()
            })
            .map(|data| LibProgram {
                data,
                ..LibProgram::default()
            })
            .collect();
        LibraryData {
            programs,
            extras: BTreeMap::new(),
        }
    }
    /// The simulator demo mode connects to, holding whatever makes a demo
    /// worth exploring. By default [`Driver::simulator`].
    fn demo_simulator(&self) -> Box<dyn Simulator> {
        self.simulator()
    }
    /// A [`Control::Choice`] field program lists can filter by (a voice
    /// mode), if any.
    fn list_facet(&self) -> Option<usize> {
        None
    }
    /// The firmware version an identity reply names, for people.
    fn firmware(&self, _identity_reply: &[u8]) -> Option<String> {
        None
    }
    /// What to check on the synth when it does not answer the device
    /// inquiry (a setting that routes SysEx elsewhere), or empty.
    fn setup_hint(&self) -> &'static str {
        ""
    }
    /// The driver's support tier (GOVERNANCE.md): 1 hardware-validated,
    /// 2 conformance-proven, 3 community. By default 3.
    fn tier(&self) -> u8 {
        3
    }
    /// The polyphony of the models it covers, fewest first (`[8, 16]` for a
    /// synth sold in 8- and 16-voice versions), or empty when unknown.
    fn voices(&self) -> &'static [u8] {
        &[]
    }
    /// [`Driver::page_summary`] for an app: `compact` asks for the shortest
    /// form that still reads (narrow columns), and `tr` translates the
    /// driver's own phrases (templates with `{name}` placeholders, filled
    /// after translating) and value names. By default, `page_summary` as
    /// it is.
    fn page_summary_in(
        &self,
        program: &[u8],
        page: usize,
        _compact: bool,
        _tr: &dyn Fn(&str) -> String,
    ) -> String {
        self.page_summary(program, page)
    }
}

/// A generic operation on a connected synth.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionOp {
    ReadCurrent,
    ReadPrograms(Vec<u16>),
    /// Store programs; the driver plans the exact sequence that leaves the
    /// synth committed and safe to read.
    WritePrograms(Vec<(u16, Vec<u8>)>),
    /// Load a program into the edit buffer without storing it.
    Audition(Vec<u8>),
}

/// Results of [`SessionOp`]s, in order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionEvent {
    Current(Vec<u8>),
    Program(u16, Vec<u8>),
    Progress { done: usize, total: usize },
    Done,
    Failed(crate::engine::OpError),
}

/// A sans-IO session with one connected synth.
pub trait DriverSession: Send {
    /// Plans the op. Errors before anything is queued.
    fn submit(&mut self, op: SessionOp) -> Result<(), DriverError>;
    fn is_idle(&self) -> bool;
    fn poll_transmit(&mut self, now: Instant) -> Option<Vec<u8>>;
    fn next_deadline(&self) -> Option<Instant>;
    fn handle_timeout(&mut self, now: Instant);
    fn handle_input(&mut self, bytes: &[u8]);
    fn poll_event(&mut self) -> Option<SessionEvent>;
}

/// A sans-IO simulated synth, for tests and demo mode.
pub trait Simulator: Send {
    /// One complete SysEx message in, the replies out.
    fn handle(&mut self, msg: &[u8]) -> Vec<Vec<u8>>;
    /// One firmware main-loop pass with no MIDI traffic.
    fn idle_tick(&mut self);
    fn program(&self, slot: u16) -> Option<Vec<u8>>;
    fn edit_buffer(&self) -> Vec<u8>;
    /// No pending save and no corrupted reply state.
    fn is_settled(&self) -> bool;
    /// Failures seen on hardware that [`Simulator::inject`] can switch on.
    fn faults(&self) -> &'static [&'static str];
    fn inject(&mut self, fault: &str);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn curation_by_group_keeps_schema_order() {
        let f = |name, group| FieldInfo {
            name,
            group,
            min: 0,
            max: 1,
        };
        let c = curation_by_group(&[f("a", "X"), f("b", "Y"), f("c", "X")]);
        let titles: Vec<_> = c.pages.iter().map(|p| p.title).collect();
        assert_eq!(titles, ["X", "Y"]);
        assert_eq!(c.pages[0].sections.len(), 1);
        assert_eq!(c.pages[0].sections[0].title, "X");
        assert_eq!(c.pages[0].sections[0].fields, [0, 2]);
        assert_eq!(c.pages[1].sections[0].fields, [1]);
        assert!(c.excluded.is_empty());
    }

    #[test]
    fn default_format_matches_the_default_control() {
        let f = |min, max| FieldInfo {
            name: "f",
            group: "G",
            min,
            max,
        };
        assert_eq!(format_raw_default(&f(0, 1), 0), "Off");
        assert_eq!(format_raw_default(&f(0, 1), 1), "On");
        assert_eq!(format_raw_default(&f(3, 4), 4), "On");
        assert_eq!(format_raw_default(&f(0, 9), 1), "1");
    }
}
