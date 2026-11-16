//! The minilogue xd behind the Oscyl driver contract.

use std::borrow::Cow;
use std::sync::LazyLock;

use oscyl_driver::{
    Bank, Capabilities, Character, Control, Curation, Driver, DriverError, DriverSession,
    FieldInfo, FileFormat, Graph, Handle, Help, Info, LibProgram, LibraryData, MacroInfo,
    Mutability, NameRules, Note, Page, Recipe, Section, Simulator, UnitKind,
};

use crate::fields::Group;
use crate::library::{Library, from_data, to_data};
use crate::msg::Module;
use crate::present::{self, Stage};
use crate::program::name_char;
use crate::{Field, Program, generate, help, live, viz};

pub struct MinilogueXd;

pub static DRIVER: MinilogueXd = MinilogueXd;

static INFO: Info = Info {
    id: "korg-minilogue-xd",
    vendor: "KORG",
    model: "minilogue xd",
};

static CAPABILITIES: Capabilities = Capabilities {
    banks: &[Bank {
        name: "Programs",
        first: 0,
        len: crate::msg::PROGRAM_COUNT,
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

fn group_name(g: Group) -> &'static str {
    match g {
        Group::Voice => "Voice",
        Group::Vco1 => "VCO 1",
        Group::Vco2 => "VCO 2",
        Group::Multi => "Multi Engine",
        Group::Mixer => "Mixer",
        Group::Filter => "Filter",
        Group::AmpEg => "Amp EG",
        Group::Eg => "EG",
        Group::Lfo => "LFO",
        Group::ModFx => "Mod FX",
        Group::Delay => "Delay",
        Group::Reverb => "Reverb",
        Group::Arp => "Arp",
        Group::Sequencer => "Sequencer",
        Group::Misc => "Misc",
    }
}

static FIELDS: LazyLock<Vec<FieldInfo>> = LazyLock::new(|| {
    Field::ALL
        .iter()
        .map(|f| {
            let s = f.spec();
            FieldInfo {
                name: s.name,
                group: group_name(s.group),
                min: s.min,
                max: s.max,
            }
        })
        .collect()
});

/// The Librarian's three files share one ZIP layout.
const FORMATS: &[FileFormat] = &[
    FileFormat {
        ext: "mnlgxdlib",
        name: "minilogue xd library",
        keeps_metadata: true,
    },
    FileFormat {
        ext: "mnlgxdpreset",
        name: "minilogue xd preset pack",
        keeps_metadata: true,
    },
    FileFormat {
        ext: "mnlgxdprog",
        name: "minilogue xd program",
        keeps_metadata: true,
    },
];

fn library_ext(ext: &str) -> Result<(), DriverError> {
    if FORMATS.iter().any(|f| f.ext == ext) {
        Ok(())
    } else {
        Err(DriverError(format!(
            "the minilogue xd driver has no .{ext} format"
        )))
    }
}

fn err(e: crate::Error) -> DriverError {
    DriverError(e.to_string())
}

fn program(bytes: &[u8]) -> Result<Program, DriverError> {
    Program::from_bytes(bytes).map_err(err)
}

fn field(i: usize) -> Result<Field, DriverError> {
    Field::ALL
        .get(i)
        .copied()
        .ok_or_else(|| DriverError(format!("no field {i}")))
}

/// `f`'s index in [`Field::ALL`], which is the contract's field index.
fn index(f: Field) -> usize {
    Field::ALL
        .iter()
        .position(|g| *g == f)
        .expect("every field is in ALL")
}

static CURATION: LazyLock<Curation> = LazyLock::new(|| Curation {
    pages: Stage::ALL
        .iter()
        .map(|&s| Page {
            title: s.title(),
            icon: Some(s.icon_name()),
            help: help::stage_help(s),
            sections: s
                .sections()
                .iter()
                .map(|sec| Section {
                    title: sec.title,
                    fields: sec.fields.iter().map(|&f| index(f)).collect(),
                })
                .collect(),
        })
        .collect(),
    excluded: present::EXCLUDED.iter().map(|&f| index(f)).collect(),
});

static CHARACTERS: LazyLock<Vec<Character>> = LazyLock::new(|| {
    generate::Character::ALL
        .iter()
        .map(|c| Character {
            id: c.title().to_lowercase().replace(' ', "-").leak(),
            title: c.title(),
            hint: c.hint(),
            signature_page: Stage::ALL.iter().position(|s| *s == signature(*c)),
        })
        .collect()
});

/// The stage whose picture says most about a character (as the app's
/// generator shows it).
fn signature(c: generate::Character) -> Stage {
    use generate::Character::*;
    match c {
        PowerChord | Keys | ArpPulse => Stage::Voice,
        FuzzBass | ScreamLead | Drone => Stage::Oscillators,
        Wobble => Stage::Filter,
        SpaceFx => Stage::Effects,
        Riser | Pad | Pluck | Bell => Stage::Envelopes,
    }
}

/// Category names, in [`present::Category`] order. "FX" is the
/// canonical spelling, as the app shows it.
const CATEGORIES: &[&str] = &[
    "Bass", "Lead", "Pad", "Keys", "Pluck", "Bell", "Arp", "FX", "Drone",
];

fn category_name(c: present::Category) -> &'static str {
    use present::Category::*;
    CATEGORIES[match c {
        Bass => 0,
        Lead => 1,
        Pad => 2,
        Keys => 3,
        Pluck => 4,
        Bell => 5,
        Arp => 6,
        Fx => 7,
        Drone => 8,
    }]
}

/// The macro titles and help the app's generator shows.
const MACRO_INFO: [MacroInfo; 4] = [
    MacroInfo {
        title: "Bright",
        help: "Cutoff, resonance and the second oscillator's octave",
    },
    MacroInfo {
        title: "Movement",
        help: "LFO, envelope amount and modulation effect",
    },
    MacroInfo {
        title: "Grit",
        help: "Drive, cross mod, ring, sync and noise",
    },
    MacroInfo {
        title: "Ambience",
        help: "Delay and reverb",
    },
];

fn handle_name(h: viz::Handle) -> &'static str {
    match h {
        viz::Handle::FilterPeak => "Cutoff and resonance",
        viz::Handle::Attack => "Attack",
        viz::Handle::Decay => "Decay and sustain",
        viz::Handle::Sustain => "Sustain",
        viz::Handle::Release => "Release",
        viz::Handle::Level(_, f) => present::short_label(f),
    }
}

/// A program and a page's stage, or `None`.
fn program_stage(bytes: &[u8], page: usize) -> Option<(Program, Stage)> {
    Some((Program::from_bytes(bytes).ok()?, *Stage::ALL.get(page)?))
}

/// Both a program and a field index, or `None`.
fn program_field(bytes: &[u8], i: usize) -> Option<(Program, Field)> {
    Some((Program::from_bytes(bytes).ok()?, field(i).ok()?))
}

fn bytes(p: Program) -> Vec<u8> {
    p.as_bytes().to_vec()
}

/// Copy field `f`'s bits from `src` to `dst` as they are, in range or not.
fn copy_field(dst: &mut [u8], src: &[u8], f: Field) {
    let s = f.spec();
    let mask = ((1u64 << s.bits) - 1) << s.shift;
    for i in 0..usize::from(s.width) {
        let m = (mask >> (8 * i)) as u8;
        let at = s.offset + i;
        dst[at] = dst[at] & !m | src[at] & m;
    }
}

/// `bytes` as a program with every out-of-range field clamped into
/// range, so the `generate` module (which expects legal values) can work
/// on it. The contract methods copy non-free fields back raw afterwards.
fn in_range(bytes: &[u8]) -> Result<Program, DriverError> {
    let mut p = program(bytes)?;
    for &f in Field::ALL {
        let s = f.spec();
        p.set(f, p.get(f).clamp(s.min, s.max))
            .expect("clamped into range");
    }
    Ok(p)
}

impl MinilogueXd {
    /// The contract's generation semantics on top of the `generate`
    /// module: every non-free field of `out` is `from`'s, as it is.
    fn keep_non_free(&self, out: &mut [u8], from: &[u8]) {
        for (i, &f) in Field::ALL.iter().enumerate() {
            if self.mutability(i) != Mutability::Free {
                copy_field(out, from, f);
            }
        }
    }
}

impl Driver for MinilogueXd {
    fn tier(&self) -> u8 {
        // GOVERNANCE.md, as the README's supported synths table says.
        2
    }
    fn voices(&self) -> &'static [u8] {
        &[4]
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
        FORMATS
    }

    fn import(&self, ext: &str, bytes: &[u8]) -> Result<LibraryData, DriverError> {
        library_ext(ext)?;
        let lib = Library::read(std::io::Cursor::new(bytes)).map_err(err)?;
        to_data(&lib).map_err(err)
    }

    fn export(&self, ext: &str, data: &LibraryData) -> Result<Vec<u8>, DriverError> {
        library_ext(ext)?;
        let mut out = std::io::Cursor::new(Vec::new());
        from_data(data).map_err(err)?.write(&mut out).map_err(err)?;
        Ok(out.into_inner())
    }

    fn session(&self, channel: u8) -> Box<dyn DriverSession> {
        Box::new(crate::adapter::XdSession::new(channel))
    }

    fn simulator(&self) -> Box<dyn Simulator> {
        Box::new(crate::adapter::XdSimulator::default())
    }

    // Curation, generation and live edits delegate to `present`, `help`,
    // `generate` and `live`, which the app also uses directly.

    fn curation(&self) -> Curation {
        CURATION.clone()
    }

    fn control(&self, bytes: &[u8], i: usize) -> Control {
        let Some((p, f)) = program_field(bytes, i) else {
            return Control::Continuous;
        };
        if f == Field::VoiceModeDepth
            && let Some((names, width)) = present::depth_choices(p.get(Field::VoiceModeType))
        {
            return Control::Bands { names, width };
        }
        match present::control(f) {
            present::Control::Dial => Control::Continuous,
            present::Control::Toggle => Control::Toggle,
            present::Control::Choice(names) => Control::Choice(names),
        }
    }

    fn short_label(&self, i: usize) -> &'static str {
        field(i).map_or("", present::short_label)
    }

    fn label_in(&self, bytes: &[u8], i: usize) -> Cow<'static, str> {
        match program_field(bytes, i) {
            Some((p, f)) => Cow::Borrowed(present::label_in(&p, f)),
            None => Cow::Borrowed(self.short_label(i)),
        }
    }

    fn emphasized(&self, i: usize) -> bool {
        field(i).is_ok_and(|f| present::PRIMARY.contains(&f))
    }

    fn format_value(&self, bytes: &[u8], i: usize) -> String {
        program_field(bytes, i).map_or_else(String::new, |(p, f)| present::format_in(&p, f))
    }

    fn format_raw(&self, i: usize, raw: u16) -> String {
        field(i).map_or_else(|_| String::new(), |f| present::format_value(f, raw))
    }

    fn page_summary(&self, bytes: &[u8], page: usize) -> String {
        program_stage(bytes, page).map_or_else(String::new, |(p, s)| present::summary(s, &p))
    }

    fn page_summary_in(
        &self,
        bytes: &[u8],
        page: usize,
        compact: bool,
        tr: &dyn Fn(&str) -> String,
    ) -> String {
        program_stage(bytes, page).map_or_else(String::new, |(p, s)| {
            present::summary_in(s, &p, compact, tr)
        })
    }

    fn graphs(&self, bytes: &[u8], page: usize, points: usize) -> Vec<Graph> {
        let Some((p, stage)) = program_stage(bytes, page) else {
            return vec![];
        };
        vec![match stage {
            Stage::Voice => Graph::Voices(
                viz::voices(&p)
                    .into_iter()
                    .map(|n| Note {
                        voice: n.voice,
                        start: n.start,
                        end: n.end,
                        pitch: n.pitch,
                    })
                    .collect(),
            ),
            Stage::Oscillators => Graph::Wave(viz::osc_cycle(&p, points)),
            Stage::Mixer => Graph::Levels(viz::mixer_levels(&p).to_vec()),
            Stage::Filter => Graph::Response {
                points: viz::filter_response(&p, points),
                cutoff: f32::from(p.get(Field::Cutoff)) / 1023.0,
                unity: viz::FILTER_UNITY,
            },
            Stage::Envelopes => Graph::Envelope(viz::amp_envelope(&p)),
            Stage::Effects => {
                let e = viz::effects(&p);
                Graph::Effects {
                    taps: e.taps,
                    tail: e.tail,
                    sweep: e.sweep,
                }
            }
        }]
    }

    fn handles(&self, bytes: &[u8], page: usize) -> Vec<Handle> {
        let Some((p, stage)) = program_stage(bytes, page) else {
            return vec![];
        };
        viz::handles(stage)
            .iter()
            .map(|&h| {
                let (x, y) = h.fields();
                Handle {
                    name: handle_name(h),
                    at: h.position(&p),
                    x: x.map(index),
                    y: y.map(index),
                }
            })
            .collect()
    }

    fn drag(&self, bytes: &[u8], page: usize, handle: usize, at: (f32, f32)) -> Vec<(usize, u16)> {
        let Some((p, stage)) = program_stage(bytes, page) else {
            return vec![];
        };
        viz::handles(stage).get(handle).map_or_else(Vec::new, |h| {
            h.values(&p, at)
                .into_iter()
                .map(|(f, v)| (index(f), v))
                .collect()
        })
    }

    fn relevant(&self, bytes: &[u8], i: usize) -> bool {
        program_field(bytes, i).is_some_and(|(p, f)| present::relevant(&p, f))
    }

    fn help(&self, i: usize) -> Help {
        field(i).map_or_else(
            |_| Help::default(),
            |f| {
                let h = help::help(f);
                Help {
                    summary: h.summary,
                    detail: h.detail,
                }
            },
        )
    }

    fn category(&self, bytes: &[u8]) -> Option<&'static str> {
        Some(category_name(present::category(
            &Program::from_bytes(bytes).ok()?,
        )))
    }

    fn categories(&self) -> &'static [&'static str] {
        CATEGORIES
    }

    fn macro_info(&self) -> [MacroInfo; 4] {
        MACRO_INFO
    }

    fn generated_name(&self, character: usize, seed: u64) -> String {
        match generate::Character::ALL.get(character) {
            Some(&c) => generate::name(seed, c),
            None => format!("Sound {}", seed % 1000),
        }
    }

    /// The user oscillator's parameter types are set by the loaded unit;
    /// tuning, controller and CV routing (except the level) and the
    /// sequencer are the player's, not the sound's.
    fn mutability(&self, i: usize) -> Mutability {
        use Field::*;
        match field(i) {
            Ok(
                UserParam1Type | UserParam2Type | UserParam3Type | UserParam4Type | UserParam5Type
                | UserParam6Type,
            ) => Mutability::Fixed,
            Ok(ProgramLevel) => Mutability::Free,
            Ok(f) if matches!(f.spec().group, Group::Misc | Group::Sequencer) => {
                Mutability::Restricted
            }
            Ok(_) => Mutability::Free,
            Err(_) => Mutability::Restricted,
        }
    }

    fn characters(&self) -> &'static [Character] {
        &CHARACTERS
    }

    fn generate(&self, recipe: &Recipe, base: Option<&[u8]>) -> Result<Vec<u8>, DriverError> {
        let character = *generate::Character::ALL
            .get(recipe.character)
            .ok_or_else(|| DriverError(format!("no character {}", recipe.character)))?;
        let keep = recipe
            .keep
            .iter()
            .map(|&i| {
                Stage::ALL
                    .get(i)
                    .copied()
                    .ok_or_else(|| DriverError(format!("no page {i}")))
            })
            .collect::<Result<_, _>>()?;
        let m = recipe.macros;
        let r = generate::Recipe {
            character,
            macros: generate::Macros {
                bright: m.bright,
                movement: m.movement,
                grit: m.grit,
                space: m.space,
            },
            seed: recipe.seed,
            keep,
        };
        let mut out = bytes(generate::generate(&r));
        if let Some(base) = base {
            program(base)?;
            self.keep_non_free(&mut out, base);
            for &stage in &r.keep {
                for &f in stage.sections().iter().flat_map(|s| s.fields) {
                    copy_field(&mut out, base, f);
                }
            }
        }
        Ok(out)
    }

    fn vary(&self, p: &[u8], amount: f32, seed: u64) -> Result<Vec<u8>, DriverError> {
        let mut out = bytes(generate::vary(&in_range(p)?, amount, seed));
        self.keep_non_free(&mut out, p);
        Ok(out)
    }

    fn breed(&self, a: &[u8], b: &[u8], mix: f32, seed: u64) -> Result<Vec<u8>, DriverError> {
        let mut out = bytes(generate::breed(&in_range(a)?, &in_range(b)?, mix, seed));
        self.keep_non_free(&mut out, a);
        Ok(out)
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

    /// The synth engine's USB port; "MIDI OUT"/"MIDI IN" are the DIN jacks.
    fn port_hint(&self) -> &'static [&'static str] {
        &["minilogue xd _ SOUND"]
    }

    fn recognize(&self, reply: &[u8]) -> bool {
        matches!(
            crate::msg::decode(reply),
            Ok(crate::msg::Reply::Identity { .. })
        )
    }

    fn demo_library(&self) -> LibraryData {
        LibraryData {
            programs: crate::demo::demo_library()
                .into_iter()
                .map(|(p, info)| LibProgram {
                    data: bytes(p),
                    author: info.programmer,
                    comment: info.comment,
                })
                .collect(),
            ..LibraryData::default()
        }
    }

    /// Demo programs and units, as the app's demo has always had.
    fn demo_simulator(&self) -> Box<dyn Simulator> {
        Box::new(crate::adapter::XdSimulator(crate::sim::SimDevice::demo()))
    }

    fn list_facet(&self) -> Option<usize> {
        Some(index(Field::VoiceModeType))
    }

    fn firmware(&self, reply: &[u8]) -> Option<String> {
        match crate::msg::decode(reply) {
            Ok(crate::msg::Reply::Identity { version, .. }) => {
                Some(format!("{}.{:02}", version.major, version.minor))
            }
            _ => None,
        }
    }

    /// Bank Select MSB 0 and LSB 0-4 pick the hundred, the Program Change
    /// 0-99 the program in it (MIDI Implementation, notes *1-1).
    fn program_slot(&self, bank_msb: u8, bank_lsb: u8, program: u8) -> Option<u16> {
        (bank_msb == 0 && bank_lsb <= 4 && program <= 99)
            .then(|| u16::from(bank_lsb) * 100 + u16::from(program))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The panel's bank (LSB 0-4) and program change (0-99) name a slot;
    /// anything outside the 500 does not.
    #[test]
    fn program_slot_from_the_panel() {
        let d = &DRIVER;
        assert_eq!(d.program_slot(0, 2, 5), Some(205));
        assert_eq!(d.program_slot(0, 4, 99), Some(499));
        assert_eq!(d.program_slot(0, 5, 0), None);
        assert_eq!(d.program_slot(1, 0, 0), None);
        assert_eq!(d.program_slot(0, 0, 100), None);
    }
    use crate::{Field, Program};
    use oscyl_driver::Driver;

    #[test]
    fn identity_hooks() {
        assert_eq!(DRIVER.port_hint(), ["minilogue xd _ SOUND"]);
        let replies = DRIVER
            .simulator()
            .handle(&oscyl_driver::sysex::DEVICE_INQUIRY);
        assert!(replies.iter().any(|r| DRIVER.recognize(r)));
        assert!(!DRIVER.recognize(&oscyl_driver::sysex::DEVICE_INQUIRY));
        // A Prophet Rev2's identity reply.
        let rev2 = [
            0xF0, 0x7E, 0x7F, 0x06, 0x02, 0x01, 0x2F, 0x01, 0x00, 0x00, 0x01, 0xF7,
        ];
        assert!(!DRIVER.recognize(&rev2));
    }

    #[test]
    fn capabilities_match_hardware() {
        let c = DRIVER.capabilities();
        assert_eq!(c.banks.iter().map(|b| b.len as usize).sum::<usize>(), 500);
        assert!(c.banks.iter().all(|b| b.writable));
        assert!(c.edit_buffer && c.live_changes);
        assert_eq!(c.units.iter().map(|u| u.slots as usize).sum::<usize>(), 48);
        assert_eq!(DRIVER.info().id, "korg-minilogue-xd");
    }

    #[test]
    fn fields_match_program_model() {
        let f = DRIVER.fields();
        assert_eq!(f.len(), Field::ALL.len());
        for (info, field) in f.iter().zip(Field::ALL) {
            let s = field.spec();
            assert_eq!((info.name, info.min, info.max), (s.name, s.min, s.max));
        }
        assert_eq!(f[0].group, "Voice");
    }

    #[test]
    fn get_set_through_contract_equals_program_api() {
        let mut bytes = DRIVER.init_program();
        assert_eq!(bytes, Program::init().as_bytes().to_vec());
        let i = Field::ALL.iter().position(|f| *f == Field::Cutoff).unwrap();
        DRIVER.set(&mut bytes, i, 300).unwrap();
        assert_eq!(DRIVER.get(&bytes, i), Ok(300));
        assert_eq!(Program::from_bytes(&bytes).unwrap().get(Field::Cutoff), 300);
        assert!(DRIVER.set(&mut bytes, i, 1024).is_err());
        assert!(DRIVER.get(&bytes, Field::ALL.len()).is_err());
    }

    #[test]
    fn wrong_length_program_is_rejected() {
        let mut short = vec![0u8; 1023];
        assert!(DRIVER.check_program(&short).is_err());
        assert!(DRIVER.get(&short, 0).is_err());
        assert!(DRIVER.set(&mut short, 0, 0).is_err());
        assert!(DRIVER.name(&short).is_err());
        assert!(DRIVER.set_name(&mut short, "A").is_err());
    }

    #[test]
    fn import_export_round_trip() {
        let mut data = oscyl_driver::LibraryData::default();
        data.programs.push(oscyl_driver::LibProgram {
            data: DRIVER.init_program(),
            author: "A".into(),
            comment: "C".into(),
        });
        let bytes = DRIVER.export("mnlgxdlib", &data).unwrap();
        let mut back = DRIVER.import("mnlgxdlib", &bytes).unwrap();
        back.extras.remove("FileInformation.xml");
        assert_eq!(back, data);
    }

    #[test]
    fn unknown_extension_is_refused() {
        let data = oscyl_driver::LibraryData::default();
        assert!(DRIVER.export("syx", &data).is_err());
        assert!(DRIVER.import("syx", &[]).is_err());
    }

    fn idx(f: Field) -> usize {
        Field::ALL.iter().position(|g| *g == f).unwrap()
    }

    #[test]
    fn curation_mirrors_present() {
        use crate::present::{EXCLUDED, Stage};
        let c = DRIVER.curation();
        let titles: Vec<_> = c.pages.iter().map(|p| p.title).collect();
        let stages: Vec<_> = Stage::ALL.iter().map(|s| s.title()).collect();
        assert_eq!(titles, stages);
        assert_eq!(c.pages[0].icon, Some("oscyl-voice-symbolic"));
        assert_eq!(c.pages[0].help, crate::help::stage_help(Stage::Voice));
        assert_eq!(c.excluded.len(), EXCLUDED.len());
        assert_eq!(c.pages[0].sections[0].fields[0], idx(Field::VoiceModeType));
        assert_eq!(DRIVER.short_label(idx(Field::ArpRate)), "Rate");
        let h = DRIVER.help(idx(Field::Cutoff));
        assert_eq!(h.summary, crate::help::help(Field::Cutoff).summary);
    }

    #[test]
    fn controls_and_formats_delegate() {
        use oscyl_driver::Control;
        let mut p = DRIVER.init_program();
        assert_eq!(
            DRIVER.control(&p, idx(Field::Vco1Wave)),
            Control::Choice(&["Square", "Triangle", "Saw"])
        );
        assert_eq!(DRIVER.control(&p, idx(Field::Sync)), Control::Toggle);
        assert_eq!(DRIVER.control(&p, idx(Field::Cutoff)), Control::Continuous);
        DRIVER.set(&mut p, idx(Field::VoiceModeType), 2).unwrap();
        assert!(matches!(
            DRIVER.control(&p, idx(Field::VoiceModeDepth)),
            Control::Bands { width: 73, .. }
        ));
        DRIVER.set(&mut p, idx(Field::ProgramLevel), 102).unwrap();
        assert_eq!(DRIVER.format_value(&p, idx(Field::ProgramLevel)), "0.0 dB");
        DRIVER.set(&mut p, idx(Field::MultiType), 1).unwrap();
        assert!(!DRIVER.relevant(&p, idx(Field::ShapeNoise)));
        assert!(DRIVER.relevant(&p, idx(Field::ShapeVpm)));
        let cat = crate::present::category(&Program::from_bytes(&p).unwrap());
        assert_eq!(DRIVER.category(&p), Some(category_name(cat)));
    }

    #[test]
    fn live_messages_are_cc_bytes() {
        let mut p = DRIVER.init_program();
        DRIVER.set(&mut p, idx(Field::AmpEgAttack), 1023).unwrap();
        assert_eq!(
            DRIVER.live_messages(&p, idx(Field::AmpEgAttack), 2),
            Some(vec![vec![0xB2, 63, 7], vec![0xB2, 16, 127]])
        );
    }

    #[test]
    fn generation_matches_generate_module() {
        use crate::generate::{self, Character, Recipe};
        use oscyl_driver::{Mutability, Recipe as R};
        let r = R {
            character: 3,
            seed: 7,
            ..R::default()
        };
        let want = generate::generate(&Recipe {
            character: Character::ALL[3],
            macros: generate::Macros::default(),
            seed: 7,
            keep: vec![],
        });
        assert_eq!(DRIVER.generate(&r, None).unwrap(), want.as_bytes().to_vec());
        assert_eq!(DRIVER.characters()[3].title, Character::ALL[3].title());
        let a = DRIVER.generate(&r, None).unwrap();
        let b = DRIVER.init_program();
        let pa = Program::from_bytes(&a).unwrap();
        let pb = Program::from_bytes(&b).unwrap();
        assert_eq!(
            DRIVER.vary(&a, 0.6, 9).unwrap(),
            generate::vary(&pa, 0.6, 9).as_bytes().to_vec()
        );
        assert_eq!(
            DRIVER.breed(&a, &b, 0.4, 9).unwrap(),
            generate::breed(&pa, &pb, 0.4, 9).as_bytes().to_vec()
        );
        assert_eq!(DRIVER.mutability(idx(Field::Cutoff)), Mutability::Free);
        assert_eq!(
            DRIVER.mutability(idx(Field::Microtuning)),
            Mutability::Restricted
        );
        assert_eq!(
            DRIVER.mutability(idx(Field::UserParam1Type)),
            Mutability::Fixed
        );
        assert!(
            DRIVER
                .generate(
                    &R {
                        character: 99,
                        ..R::default()
                    },
                    None
                )
                .is_err()
        );
    }

    #[test]
    fn non_free_fields_come_from_the_base() {
        use oscyl_driver::Recipe as R;
        let mut base = DRIVER.init_program();
        DRIVER
            .set(&mut base, idx(Field::UserParam1Type), 2)
            .unwrap();
        DRIVER.set(&mut base, idx(Field::Microtuning), 5).unwrap();
        DRIVER
            .set(&mut base, idx(Field::BendRangePlus), 12)
            .unwrap();
        let r = R {
            seed: 3,
            ..R::default()
        };
        let p = DRIVER.generate(&r, Some(&base)).unwrap();
        assert_eq!(DRIVER.get(&p, idx(Field::UserParam1Type)), Ok(2));
        assert_eq!(DRIVER.get(&p, idx(Field::Microtuning)), Ok(5));
        let other = DRIVER.init_program();
        let child = DRIVER.breed(&base, &other, 0.5, 1).unwrap();
        assert_eq!(DRIVER.get(&child, idx(Field::BendRangePlus)), Ok(12));
        assert_eq!(DRIVER.get(&child, idx(Field::UserParam1Type)), Ok(2));
        let child = DRIVER.breed(&other, &base, 0.5, 1).unwrap();
        assert_eq!(DRIVER.get(&child, idx(Field::UserParam1Type)), Ok(0));
        let v = DRIVER.vary(&base, 1.0, 1).unwrap();
        assert_eq!(DRIVER.get(&v, idx(Field::Microtuning)), Ok(5));
    }

    #[test]
    fn name_rules_match_set_name() {
        let r = DRIVER.name_rules();
        assert_eq!(r.max_len, 12);
        let mut bytes = DRIVER.init_program();
        DRIVER.set_name(&mut bytes, "Bass 1").unwrap();
        assert_eq!(DRIVER.name(&bytes).unwrap(), "Bass 1");
        assert!((r.allowed)('A') && (r.allowed)(' '));
        assert!(!(r.allowed)('é') && !(r.allowed)('~'));
        assert!(DRIVER.set_name(&mut bytes, "~").is_err());
    }

    #[test]
    fn app_hooks() {
        let demo = DRIVER.demo_library();
        let want = crate::demo::demo_library();
        assert_eq!(demo.programs.len(), want.len());
        for (p, (q, info)) in demo.programs.iter().zip(&want) {
            assert_eq!(p.data, q.as_bytes().to_vec());
            assert_eq!((&p.author, &p.comment), (&info.programmer, &info.comment));
        }
        let sim = DRIVER.demo_simulator();
        assert_eq!(sim.program(0), Some(demo.programs[0].data.clone()));
        assert_eq!(DRIVER.list_facet(), Some(idx(Field::VoiceModeType)));
        let reply = DRIVER
            .simulator()
            .handle(&oscyl_driver::sysex::DEVICE_INQUIRY)
            .into_iter()
            .find(|r| DRIVER.recognize(r))
            .unwrap();
        assert_eq!(DRIVER.firmware(&reply).as_deref(), Some("2.10"));
        assert_eq!(DRIVER.firmware(&[0xF0, 0xF7]), None);
        let exts: Vec<_> = DRIVER.file_formats().iter().map(|f| f.ext).collect();
        assert_eq!(exts, ["mnlgxdlib", "mnlgxdpreset", "mnlgxdprog"]);
        for ext in exts {
            let bytes = DRIVER.export(ext, &demo).unwrap();
            let back = DRIVER.import(ext, &bytes).unwrap();
            assert_eq!(back.programs, demo.programs);
        }
        assert_eq!(DRIVER.setup_hint(), "");
    }
    /// The stage summary as the Linux app drew it before the driver took
    /// it over (`apps/linux/src/editor.rs` on main, 2026-11-12): long and
    /// short forms, every phrase and value through `tr`.
    fn app_summary(stage: Stage, p: &Program, tr: &dyn Fn(&str) -> String) -> (String, String) {
        let f = |field| tr(&present::format_in(p, field));
        let same = |s: String| (s.clone(), s);
        match stage {
            Stage::Voice => {
                let mode = f(Field::VoiceModeType);
                if p.get(Field::Octave) == Field::Octave.spec().max / 2 {
                    same(mode)
                } else {
                    let oct = f(Field::Octave);
                    (
                        tr("{mode} · Octave {octave}")
                            .replace("{mode}", &mode)
                            .replace("{octave}", &oct),
                        format!("{mode} {oct}"),
                    )
                }
            }
            Stage::Oscillators => same(format!("{} + {}", f(Field::Vco1Wave), f(Field::Vco2Wave))),
            Stage::Mixer => {
                let on: Vec<(String, u32)> = [
                    (Field::Vco1Level, tr("VCO 1")),
                    (Field::Vco2Level, tr("VCO 2")),
                    (Field::MultiLevel, tr("Multi")),
                ]
                .into_iter()
                .filter(|(fl, _)| p.get(*fl) > 0)
                .map(|(fl, n)| (n, (u32::from(p.get(fl)) * 100 + 511) / 1023))
                .collect();
                if on.is_empty() {
                    return same(tr("Silent"));
                }
                let long = on.iter().map(|(n, v)| format!("{n} {v}%"));
                let short = on.iter().map(|(_, v)| format!("{v}%"));
                (
                    long.collect::<Vec<_>>().join(" · "),
                    short.collect::<Vec<_>>().join(" · "),
                )
            }
            Stage::Filter => (
                tr("Cutoff {cutoff} · Res {resonance}")
                    .replace("{cutoff}", &f(Field::Cutoff))
                    .replace("{resonance}", &f(Field::Resonance)),
                tr("Cut {cutoff} · Res {resonance}")
                    .replace("{cutoff}", &f(Field::Cutoff))
                    .replace("{resonance}", &f(Field::Resonance)),
            ),
            Stage::Envelopes => same(
                tr("A {attack} · R {release}")
                    .replace("{attack}", &f(Field::AmpEgAttack))
                    .replace("{release}", &f(Field::AmpEgRelease)),
            ),
            Stage::Effects => {
                let names: Vec<String> = [
                    (Field::ModFxOn, tr("Mod")),
                    (Field::DelayOn, tr("Delay")),
                    (Field::ReverbOn, tr("Reverb")),
                ]
                .into_iter()
                .filter(|(fl, _)| p.get(*fl) != 0)
                .map(|(_, n)| n)
                .collect();
                if names.is_empty() {
                    same(tr("All off"))
                } else {
                    same(names.join(" · "))
                }
            }
        }
    }

    #[test]
    fn summaries_are_the_apps_long_and_compact_forms_translated() {
        let plain = |s: &str| s.to_string();
        let marked = |s: &str| format!("<{s}>");
        let mut programs = vec![Program::init()];
        for (i, (p, _)) in crate::demo::demo_library().into_iter().enumerate() {
            programs.push(crate::generate::vary(&p, 0.9, i as u64));
            programs.push(p);
        }
        for p in &programs {
            for (page, stage) in Stage::ALL.into_iter().enumerate() {
                for tr in [&plain as &dyn Fn(&str) -> String, &marked] {
                    let (long, short) = app_summary(stage, p, tr);
                    assert_eq!(DRIVER.page_summary_in(p.as_bytes(), page, false, tr), long);
                    assert_eq!(DRIVER.page_summary_in(p.as_bytes(), page, true, tr), short);
                }
                assert_eq!(
                    DRIVER.page_summary(p.as_bytes(), page),
                    app_summary(stage, p, &plain).0
                );
            }
        }
    }
}
