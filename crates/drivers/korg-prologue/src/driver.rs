//! The prologue behind the Oscyl driver contract.

use std::borrow::Cow;
use std::io::Cursor;

use oscyl_driver::{
    Bank, Capabilities, Character, Control, Curation, Driver, DriverError, DriverSession,
    FieldInfo, FileFormat, Graph, Help, Info, LibraryData, MacroInfo, Mutability, NameRules,
    Recipe, Simulator, UnitKind,
};
use oscyl_logue::kinds::name_char;
use oscyl_logue::lag::LogueSession;
use oscyl_logue::library::{Library, from_data, to_data};
use oscyl_logue::msg::{Module, PROGRAM_COUNT, Reply, decode};
use oscyl_logue::sim::LogueSimulator;

use crate::fields::{Field, all};
use crate::model::PrologueModel;
use crate::{Program, curate, generate, live, viz};

pub struct Prologue;

pub static DRIVER: Prologue = Prologue;

static INFO: Info = Info {
    id: "korg-prologue",
    vendor: "KORG",
    model: "prologue",
};

const fn unit(m: Module) -> UnitKind {
    UnitKind {
        name: match m {
            Module::Osc => "osc",
            Module::ModFx => "modfx",
            Module::DelFx => "delfx",
            Module::RevFx => "revfx",
        },
        slots: match m {
            Module::ModFx | Module::Osc => 16,
            Module::DelFx | Module::RevFx => 8,
        },
    }
}

static CAPABILITIES: Capabilities = Capabilities {
    banks: &[Bank {
        name: "Programs",
        first: 0,
        len: PROGRAM_COUNT,
        writable: true,
    }],
    edit_buffer: true,
    live_changes: true,
    units: &[
        unit(Module::Osc),
        unit(Module::ModFx),
        unit(Module::DelFx),
        unit(Module::RevFx),
    ],
};

static FIELDS: std::sync::LazyLock<Vec<FieldInfo>> = std::sync::LazyLock::new(|| {
    all()
        .iter()
        .map(|f| FieldInfo {
            name: f.name,
            group: f.group,
            min: f.min,
            max: f.max,
        })
        .collect()
});

const LIBRARY_EXT: &str = "prlglib";

fn library_ext(ext: &str) -> Result<(), DriverError> {
    if ext == LIBRARY_EXT {
        Ok(())
    } else {
        Err(DriverError(format!(
            "the prologue driver has no .{ext} format"
        )))
    }
}

fn err(e: oscyl_logue::Error) -> DriverError {
    DriverError(e.to_string())
}

fn program(bytes: &[u8]) -> Result<Program, DriverError> {
    Program::from_bytes(bytes).map_err(err)
}

fn field(i: usize) -> Result<&'static Field, DriverError> {
    all()
        .get(i)
        .ok_or_else(|| DriverError(format!("no field {i}")))
}

/// Both a program and a field, or `None`.
fn program_field(bytes: &[u8], i: usize) -> Option<(Program, &'static Field)> {
    Some((Program::from_bytes(bytes).ok()?, all().get(i)?))
}

impl Driver for Prologue {
    fn tier(&self) -> u8 {
        // GOVERNANCE.md, as the README's supported synths table says.
        2
    }
    fn voices(&self) -> &'static [u8] {
        &[8, 16]
    }
    fn info(&self) -> &'static Info {
        &INFO
    }

    fn capabilities(&self) -> &'static Capabilities {
        &CAPABILITIES
    }

    fn fields(&self) -> &'static [FieldInfo] {
        &FIELDS
    }

    /// 12 characters; the allowed set is the xd's until a probe shows the
    /// prologue's.
    fn name_rules(&self) -> NameRules {
        NameRules {
            max_len: 12,
            allowed: |c| c.is_ascii() && name_char(c as u8),
        }
    }

    fn init_program(&self) -> Vec<u8> {
        Program::init().as_bytes().to_vec()
    }

    fn check_program(&self, bytes: &[u8]) -> Result<(), DriverError> {
        program(bytes).map(|_| ())
    }

    fn get(&self, bytes: &[u8], i: usize) -> Result<u16, DriverError> {
        Ok(program(bytes)?.get(field(i)?))
    }

    fn set(&self, bytes: &mut [u8], i: usize, value: u16) -> Result<(), DriverError> {
        let mut p = program(bytes)?;
        p.set(field(i)?, value).map_err(err)?;
        bytes.copy_from_slice(p.as_bytes());
        Ok(())
    }

    fn name(&self, bytes: &[u8]) -> Result<String, DriverError> {
        Ok(program(bytes)?.name())
    }

    fn set_name(&self, bytes: &mut [u8], name: &str) -> Result<(), DriverError> {
        let mut p = program(bytes)?;
        p.set_name(name).map_err(err)?;
        bytes.copy_from_slice(p.as_bytes());
        Ok(())
    }

    fn file_formats(&self) -> &'static [FileFormat] {
        &[FileFormat {
            ext: LIBRARY_EXT,
            name: "prologue library",
            keeps_metadata: true,
        }]
    }

    fn import(&self, ext: &str, bytes: &[u8]) -> Result<LibraryData, DriverError> {
        library_ext(ext)?;
        let lib = Library::<PrologueModel>::read(Cursor::new(bytes)).map_err(err)?;
        to_data(&lib).map_err(err)
    }

    fn export(&self, ext: &str, data: &LibraryData) -> Result<Vec<u8>, DriverError> {
        library_ext(ext)?;
        let mut out = Cursor::new(Vec::new());
        from_data::<PrologueModel>(data)
            .map_err(err)?
            .write(&mut out)
            .map_err(err)?;
        Ok(out.into_inner())
    }

    fn curation(&self) -> Curation {
        curate::CURATION.clone()
    }

    fn control(&self, bytes: &[u8], i: usize) -> Control {
        match program_field(bytes, i) {
            Some((p, f)) => curate::control(&p, f),
            None => Control::Continuous,
        }
    }

    fn short_label(&self, i: usize) -> &'static str {
        field(i).map_or("", curate::short_label)
    }

    fn label_in(&self, bytes: &[u8], i: usize) -> Cow<'static, str> {
        match program_field(bytes, i) {
            Some((p, f)) => Cow::Borrowed(curate::label_in(&p, f)),
            None => Cow::Borrowed(self.short_label(i)),
        }
    }

    fn emphasized(&self, i: usize) -> bool {
        field(i).is_ok_and(curate::emphasized)
    }

    fn format_value(&self, bytes: &[u8], i: usize) -> String {
        program_field(bytes, i).map_or_else(String::new, |(p, f)| curate::format_in(&p, f))
    }

    fn format_raw(&self, i: usize, raw: u16) -> String {
        field(i).map_or_else(|_| String::new(), |f| curate::format_raw(f, raw))
    }

    fn page_summary(&self, bytes: &[u8], page: usize) -> String {
        Program::from_bytes(bytes).map_or_else(|_| String::new(), |p| curate::summary(&p, page))
    }

    fn graphs(&self, bytes: &[u8], page: usize, points: usize) -> Vec<Graph> {
        Program::from_bytes(bytes).map_or_else(|_| vec![], |p| viz::graphs(&p, page, points))
    }

    fn relevant(&self, bytes: &[u8], i: usize) -> bool {
        program_field(bytes, i).is_some_and(|(p, f)| curate::relevant(&p, f))
    }

    fn help(&self, i: usize) -> Help {
        field(i).map_or_else(|_| Help::default(), curate::help)
    }

    fn category(&self, bytes: &[u8]) -> Option<&'static str> {
        curate::category(&Program::from_bytes(bytes).ok()?)
    }

    fn categories(&self) -> &'static [&'static str] {
        curate::CATEGORIES
    }

    fn mutability(&self, i: usize) -> Mutability {
        field(i).map_or(Mutability::Restricted, generate::mutability)
    }

    fn characters(&self) -> &'static [Character] {
        generate::CHARACTERS
    }

    fn macro_info(&self) -> [MacroInfo; 4] {
        generate::MACRO_INFO
    }

    fn generated_name(&self, character: usize, seed: u64) -> String {
        match generate::CHARACTERS.get(character) {
            Some(c) => generate::name(c, seed),
            None => format!("Sound {}", seed % 1000),
        }
    }

    fn generate(&self, recipe: &Recipe, base: Option<&[u8]>) -> Result<Vec<u8>, DriverError> {
        generate::generate(recipe, base)
    }

    fn vary(&self, p: &[u8], amount: f32, seed: u64) -> Result<Vec<u8>, DriverError> {
        generate::vary(p, amount, seed)
    }

    fn breed(&self, a: &[u8], b: &[u8], mix: f32, seed: u64) -> Result<Vec<u8>, DriverError> {
        generate::breed(a, b, mix, seed)
    }

    fn live_messages(&self, bytes: &[u8], i: usize, channel: u8) -> Option<Vec<Vec<u8>>> {
        let (p, f) = program_field(bytes, i)?;
        let status = 0xB0 | (channel & 0x0F);
        Some(
            live::messages(f, &p)?
                .into_iter()
                .map(|(cc, v)| vec![status, cc, v])
                .collect(),
        )
    }

    fn session(&self, channel: u8) -> Box<dyn DriverSession> {
        Box::new(LogueSession::<PrologueModel>::new(channel))
    }

    fn simulator(&self) -> Box<dyn Simulator> {
        Box::new(LogueSimulator::<PrologueModel>::default())
    }

    /// The synth engine's USB port, named like the xd's "minilogue xd _
    /// SOUND" on Linux (the Librarian manual names the ports KBD/KNOB and
    /// SOUND; unverified on a prologue, probe P1).
    fn port_hint(&self) -> &'static [&'static str] {
        &["prologue _ SOUND"]
    }

    fn recognize(&self, reply: &[u8]) -> bool {
        use oscyl_logue::Model;
        matches!(
            decode(&PrologueModel::FAMILY, reply),
            Ok(Reply::Identity { .. })
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fields::index;
    use oscyl_driver::{SessionEvent, SessionOp};

    #[test]
    fn capabilities_match_the_prologue() {
        let c = DRIVER.capabilities();
        assert_eq!(c.banks[0].len, 500);
        assert!(c.edit_buffer && c.live_changes);
        let units: Vec<_> = c.units.iter().map(|u| (u.name, u.slots)).collect();
        assert_eq!(
            units,
            [("osc", 16), ("modfx", 16), ("delfx", 8), ("revfx", 8)]
        );
        assert_eq!(DRIVER.file_formats()[0].ext, "prlglib");
        assert!(DRIVER.file_formats()[0].keeps_metadata);
    }

    #[test]
    fn live_messages_use_the_given_channel_for_main_only() {
        let mut p = DRIVER.init_program();
        let cutoff = index("Main Filter Cutoff");
        DRIVER.set(&mut p, cutoff, 1023).unwrap();
        assert_eq!(
            DRIVER.live_messages(&p, cutoff, 5),
            Some(vec![vec![0xB5, 43, 127]])
        );
        assert_eq!(
            DRIVER.live_messages(&p, index("Sub Filter Cutoff"), 5),
            None
        );
        assert_eq!(DRIVER.live_messages(&p, index("Micro Tuning"), 5), None);
    }

    fn pump(s: &mut dyn DriverSession, sim: &mut dyn Simulator) -> Vec<SessionEvent> {
        let mut now = std::time::Instant::now();
        let mut out = vec![];
        loop {
            if let Some(m) = s.poll_transmit(now) {
                for r in sim.handle(&m) {
                    s.handle_input(&r);
                }
            } else if let Some(d) = s.next_deadline() {
                now = d;
                s.handle_timeout(now);
            } else if s.is_idle() {
                break;
            }
            out.extend(std::iter::from_fn(|| s.poll_event()));
        }
        out.extend(std::iter::from_fn(|| s.poll_event()));
        out
    }

    fn named(name: &str) -> Vec<u8> {
        let mut p = DRIVER.init_program();
        DRIVER.set_name(&mut p, name).unwrap();
        p
    }

    #[test]
    fn single_edit_sends_13_writes_and_leaves_store_clean() {
        let (mut s, mut sim) = (DRIVER.session(0), DRIVER.simulator());
        s.submit(SessionOp::WritePrograms(vec![(300, named("Edit"))]))
            .unwrap();
        let ev = pump(&mut *s, &mut *sim);
        let total = ev.iter().find_map(|e| match e {
            SessionEvent::Progress { total, .. } => Some(*total),
            _ => None,
        });
        assert_eq!(total, Some(13));
        assert_eq!(ev.last(), Some(&SessionEvent::Done));
        assert_eq!(sim.program(300), Some(named("Edit")));
        assert!(sim.is_settled());
    }

    #[test]
    fn write_499_alone_uses_a_group_41_neighbour() {
        // Programs 0 and 1 differ, so the lag check vouches for reads.
        let lib = [
            Program::init(),
            Program::from_bytes(&named("Other")).unwrap(),
        ];
        let mut s = DRIVER.session(0);
        let mut sim: Box<dyn Simulator> = Box::new(LogueSimulator(oscyl_logue::sim::Sim::<
            PrologueModel,
        >::with_library(&lib)));
        assert!(
            s.submit(SessionOp::WritePrograms(vec![(499, named("Last"))]))
                .is_err(),
            "no neighbour known yet"
        );
        s.submit(SessionOp::ReadPrograms(vec![495])).unwrap();
        pump(&mut *s, &mut *sim);
        s.submit(SessionOp::WritePrograms(vec![(499, named("Last"))]))
            .unwrap();
        assert_eq!(pump(&mut *s, &mut *sim).last(), Some(&SessionEvent::Done));
        assert_eq!(sim.program(499), Some(named("Last")));
        assert!(sim.is_settled());
    }

    #[test]
    fn invalid_program_is_refused_before_sending() {
        let mut s = DRIVER.session(0);
        let mut bad = DRIVER.init_program();
        bad[332] = 0;
        assert!(s.submit(SessionOp::WritePrograms(vec![(3, bad)])).is_err());
        assert!(s.poll_transmit(std::time::Instant::now()).is_none());
    }

    #[test]
    fn library_names_the_prologue() {
        let data = LibraryData {
            programs: vec![oscyl_driver::LibProgram {
                data: named("One"),
                author: "A".into(),
                comment: "C".into(),
            }],
            extras: Default::default(),
        };
        let bytes = DRIVER.export("prlglib", &data).unwrap();
        let back = DRIVER.import("prlglib", &bytes).unwrap();
        assert_eq!(back.programs, data.programs);
        let index = String::from_utf8(back.extras["FileInformation.xml"].clone()).unwrap();
        assert!(index.contains("<Product>prologue</Product>"));
        assert!(DRIVER.import("mnlgxdlib", &bytes).is_err());
    }
}
