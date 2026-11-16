//! The Prophet Rev2 behind the Oscyl driver contract.

use std::sync::LazyLock;

use oscyl_driver::{
    Bank, Capabilities, Character, Control, Curation, Driver, DriverError, DriverSession,
    FieldInfo, FileFormat, Graph, Help, Info, LibraryData, MacroInfo, Mutability, NameRules,
    Recipe, Simulator,
};

use crate::program::{PROGRAM_LEN, name_char};
use crate::session::Rev2Session;
use crate::sim::SimRev2;
use crate::{Field, Program, curate, fields, generate, syx};

pub struct ProphetRev2;

pub static DRIVER: ProphetRev2 = ProphetRev2;

static INFO: Info = Info {
    id: "sequential-prophet-rev2",
    vendor: "Sequential",
    model: "Prophet Rev2",
};

const fn bank(name: &'static str, i: u16) -> Bank {
    Bank {
        name,
        first: i * 128,
        len: 128,
        // F1-F4 take SysEx writes, but are read-only on the panel.
        writable: i < 4,
    }
}

static CAPABILITIES: Capabilities = Capabilities {
    banks: &[
        bank("U1", 0),
        bank("U2", 1),
        bank("U3", 2),
        bank("U4", 3),
        bank("F1", 4),
        bank("F2", 5),
        bank("F3", 6),
        bank("F4", 7),
    ],
    edit_buffer: true,
    live_changes: true,
    units: &[],
};

static FIELDS: LazyLock<Vec<FieldInfo>> = LazyLock::new(|| {
    fields::all()
        .iter()
        .map(|f| FieldInfo {
            name: f.name,
            group: f.group,
            min: f.min,
            max: f.max,
        })
        .collect()
});

const SYX: FileFormat = FileFormat {
    ext: "syx",
    name: "Prophet Rev2 program dumps",
    keeps_metadata: false,
};

fn syx_ext(ext: &str) -> Result<(), DriverError> {
    if ext == SYX.ext {
        Ok(())
    } else {
        Err(DriverError(format!(
            "the Prophet Rev2 driver has no .{ext} format"
        )))
    }
}

fn program(bytes: &[u8]) -> Result<Program, DriverError> {
    Program::from_bytes(bytes)
}

/// Field `i`, once `bytes` is known to be a whole program. Reads and
/// writes stay on the slice: the schema has about 2000 fields.
fn field(bytes: &[u8], i: usize) -> Result<&'static Field, DriverError> {
    if bytes.len() != PROGRAM_LEN {
        program(bytes)?;
    }
    fields::all()
        .get(i)
        .ok_or_else(|| DriverError(format!("no field {i}")))
}

/// Field `j`'s stored byte in a whole program.
fn byte(bytes: &[u8], j: usize) -> u16 {
    bytes[usize::from(fields::all()[j].offset)].into()
}

impl Driver for ProphetRev2 {
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

    fn name_rules(&self) -> NameRules {
        NameRules {
            max_len: 20,
            allowed: name_char,
        }
    }

    fn init_program(&self) -> Vec<u8> {
        Program::init().as_bytes().to_vec()
    }

    fn check_program(&self, bytes: &[u8]) -> Result<(), DriverError> {
        program(bytes).map(|_| ())
    }

    fn get(&self, bytes: &[u8], i: usize) -> Result<u16, DriverError> {
        let f = field(bytes, i)?;
        Ok(bytes[usize::from(f.offset)].into())
    }

    fn set(&self, bytes: &mut [u8], i: usize, value: u16) -> Result<(), DriverError> {
        let f = field(bytes, i)?;
        if !(f.min..=f.max).contains(&value) {
            return Err(DriverError(format!(
                "{} = {value} is outside {}..={}",
                f.name, f.min, f.max
            )));
        }
        // Every field's max is at most 255.
        bytes[usize::from(f.offset)] = value as u8;
        Ok(())
    }

    fn name(&self, bytes: &[u8]) -> Result<String, DriverError> {
        Ok(program(bytes)?.name())
    }

    fn set_name(&self, bytes: &mut [u8], name: &str) -> Result<(), DriverError> {
        let mut p = program(bytes)?;
        p.set_name(name)?;
        bytes.copy_from_slice(p.as_bytes());
        Ok(())
    }

    fn file_formats(&self) -> &'static [FileFormat] {
        &[SYX]
    }

    fn import(&self, ext: &str, bytes: &[u8]) -> Result<LibraryData, DriverError> {
        syx_ext(ext)?;
        syx::import(bytes)
    }

    fn export(&self, ext: &str, lib: &LibraryData) -> Result<Vec<u8>, DriverError> {
        syx_ext(ext)?;
        syx::export(lib)
    }

    /// The Rev2's SysEx carries no channel, so `channel` is unused.
    fn session(&self, _channel: u8) -> Box<dyn DriverSession> {
        Box::new(Rev2Session::default())
    }

    fn simulator(&self) -> Box<dyn Simulator> {
        Box::new(SimRev2::default())
    }

    fn curation(&self) -> Curation {
        curate::CURATION.clone()
    }

    fn control(&self, _bytes: &[u8], i: usize) -> Control {
        curate::control(i)
    }

    fn short_label(&self, i: usize) -> &'static str {
        curate::short_label_of(i)
    }

    fn emphasized(&self, i: usize) -> bool {
        curate::emphasized(i)
    }

    fn format_value(&self, bytes: &[u8], i: usize) -> String {
        self.get(bytes, i)
            .map(|v| curate::format(i, v))
            .unwrap_or_default()
    }

    fn format_raw(&self, i: usize, raw: u16) -> String {
        if i < fields::all().len() {
            curate::format(i, raw)
        } else {
            String::new()
        }
    }

    fn page_summary(&self, bytes: &[u8], page: usize) -> String {
        if Program::from_bytes(bytes).is_err() {
            return String::new();
        }
        curate::summary(page, |j| byte(bytes, j))
    }

    fn graphs(&self, bytes: &[u8], page: usize, points: usize) -> Vec<Graph> {
        if Program::from_bytes(bytes).is_err() {
            return vec![];
        }
        curate::graphs(page, |j| byte(bytes, j), points)
    }

    fn relevant(&self, bytes: &[u8], i: usize) -> bool {
        field(bytes, i).is_ok() && curate::relevant(i, |j| byte(bytes, j))
    }

    fn help(&self, i: usize) -> Help {
        curate::help(i)
    }

    fn mutability(&self, i: usize) -> Mutability {
        fields::all()
            .get(i)
            .map_or(Mutability::Restricted, generate::mutability)
    }

    fn characters(&self) -> &'static [Character] {
        generate::CHARACTERS
    }

    fn macro_info(&self) -> [MacroInfo; 4] {
        generate::MACRO_INFO
    }

    fn generated_name(&self, character: usize, seed: u64) -> String {
        generate::CHARACTERS.get(character).map_or_else(
            || format!("Sound {:04X}", seed & 0xFFFF),
            |c| generate::name(c, seed),
        )
    }

    fn generate(&self, recipe: &Recipe, base: Option<&[u8]>) -> Result<Vec<u8>, DriverError> {
        generate::generate(recipe, base)
    }

    fn vary(&self, bytes: &[u8], amount: f32, seed: u64) -> Result<Vec<u8>, DriverError> {
        generate::vary(bytes, amount, seed)
    }

    fn breed(&self, a: &[u8], b: &[u8], mix: f32, seed: u64) -> Result<Vec<u8>, DriverError> {
        generate::breed(a, b, mix, seed)
    }

    fn morph(&self, a: &[u8], b: &[u8], t: f32, seed: u64) -> Result<Vec<u8>, DriverError> {
        generate::cohered(&oscyl_driver::morph_fields_by(
            self,
            a,
            b,
            t,
            seed,
            generate::dial,
        )?)
    }

    /// NRPN on `channel`: parameter
    /// MSB/LSB on CC 99/98, value MSB/LSB on CC 6/38. Poly sequencer
    /// velocities below 128 (0 = reset) are outside the Guide's NRPN range
    /// 128-255, so they have no live message.
    fn live_messages(&self, bytes: &[u8], i: usize, channel: u8) -> Option<Vec<Vec<u8>>> {
        let f = field(bytes, i).ok()?;
        let v = u16::from(bytes[usize::from(f.offset)]);
        if f.name.contains("Poly seq") && f.name.contains(" velocity ") && v < 128 {
            return None;
        }
        let s = 0xB0 | (channel & 0x0F);
        let n = f.nrpn;
        Some(
            [(99, n >> 7), (98, n & 0x7F), (6, v >> 7), (38, v & 0x7F)]
                .into_iter()
                .map(|(cc, d)| vec![s, cc, d as u8])
                .collect(),
        )
    }

    /// One USB port.
    fn port_hint(&self) -> &'static [&'static str] {
        &["Prophet Rev2"]
    }

    fn recognize(&self, reply: &[u8]) -> bool {
        matches!(crate::msg::decode(reply), Ok(crate::msg::Reply::Identity))
    }

    /// Guide p. 83 and Appendix D: SysEx goes only to the cable chosen in
    /// the Global menu.
    fn setup_hint(&self) -> &'static str {
        "On the Prophet Rev2, press Global and set MIDI SysEx Cable to USB \
         (or to MIDI Port when connected over MIDI cables), and MIDI Prog Enable to On."
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Program, fields};

    #[test]
    fn identity_and_capabilities_match_the_docs() {
        assert_eq!(
            *DRIVER.info(),
            Info {
                id: "sequential-prophet-rev2",
                vendor: "Sequential",
                model: "Prophet Rev2",
            }
        );
        let c = DRIVER.capabilities();
        let banks: Vec<_> = c
            .banks
            .iter()
            .map(|b| (b.name, b.first, b.len, b.writable))
            .collect();
        assert_eq!(
            banks,
            [
                ("U1", 0, 128, true),
                ("U2", 128, 128, true),
                ("U3", 256, 128, true),
                ("U4", 384, 128, true),
                ("F1", 512, 128, false),
                ("F2", 640, 128, false),
                ("F3", 768, 128, false),
                ("F4", 896, 128, false),
            ]
        );
        assert!(c.edit_buffer && c.live_changes && c.units.is_empty());
        assert_eq!(
            DRIVER.file_formats(),
            [FileFormat {
                ext: "syx",
                name: "Prophet Rev2 program dumps",
                keeps_metadata: false,
            }]
        );
    }

    #[test]
    fn identity_hooks() {
        assert_eq!(DRIVER.port_hint(), ["Prophet Rev2"]);
        let replies = DRIVER
            .simulator()
            .handle(&oscyl_driver::sysex::DEVICE_INQUIRY);
        assert!(replies.iter().any(|r| DRIVER.recognize(r)));
        assert!(!DRIVER.recognize(&oscyl_driver::sysex::DEVICE_INQUIRY));
        // A minilogue xd's identity reply (hardware capture).
        let xd = [
            0xF0, 0x7E, 0x00, 0x06, 0x02, 0x42, 0x51, 0x01, 0x00, 0x00, 0x02, 0x00, 0x0A, 0x00,
            0xF7,
        ];
        assert!(!DRIVER.recognize(&xd));
    }

    #[test]
    fn fields_follow_the_schema() {
        let f = DRIVER.fields();
        assert_eq!(f.len(), fields::all().len());
        for (info, field) in f.iter().zip(fields::all()) {
            assert_eq!(
                (info.name, info.group, info.min, info.max),
                (field.name, field.group, field.min, field.max)
            );
        }
        let mut bytes = DRIVER.init_program();
        let i = f.iter().position(|f| f.name == "B Filter cutoff").unwrap();
        DRIVER.set(&mut bytes, i, 99).unwrap();
        assert_eq!(DRIVER.get(&bytes, i), Ok(99));
        assert_eq!(bytes[22 + 1024], 99);
        assert!(DRIVER.set(&mut bytes, i, 165).is_err());
        assert!(DRIVER.get(&bytes, f.len()).is_err());
        assert_eq!(bytes.len(), Program::init().as_bytes().len());
    }

    #[test]
    fn names_are_printable_ascii_up_to_20() {
        let r = DRIVER.name_rules();
        assert_eq!(r.max_len, 20);
        assert!((r.allowed)('~') && (r.allowed)(' ') && (r.allowed)('a'));
        assert!(!(r.allowed)('\u{e9}') && !(r.allowed)('\t'));
        let mut bytes = DRIVER.init_program();
        DRIVER.set_name(&mut bytes, "Brass ~ 2").unwrap();
        assert_eq!(DRIVER.name(&bytes).unwrap(), "Brass ~ 2");
    }

    fn idx(name: &str) -> usize {
        DRIVER.fields().iter().position(|f| f.name == name).unwrap()
    }

    #[test]
    fn cutoff_is_a_four_message_nrpn() {
        let mut p = DRIVER.init_program();
        DRIVER.set(&mut p, idx("B Filter cutoff"), 164).unwrap();
        assert_eq!(
            DRIVER.live_messages(&p, idx("B Filter cutoff"), 0),
            Some(vec![
                vec![0xB0, 99, 0x10],
                vec![0xB0, 98, 0x0F],
                vec![0xB0, 6, 1],
                vec![0xB0, 38, 36]
            ])
        );
        let m = DRIVER.live_messages(&p, idx("Split point"), 3).unwrap();
        assert_eq!(
            m,
            [[0xB3, 99, 1], [0xB3, 98, 43], [0xB3, 6, 0], [0xB3, 38, 60]]
        );
    }

    #[test]
    fn velocity_reset_has_no_live_message() {
        let mut p = DRIVER.init_program();
        let v = idx("A Poly seq track 1 velocity 1");
        assert_eq!(DRIVER.live_messages(&p, v, 0), None);
        DRIVER.set(&mut p, v, 200).unwrap();
        assert!(DRIVER.live_messages(&p, v, 0).is_some());
    }

    #[test]
    fn only_syx_is_known() {
        assert!(DRIVER.import("mnlgxdlib", &[]).is_err());
        assert!(DRIVER.export("mnlgxdlib", &LibraryData::default()).is_err());
    }

    #[test]
    fn app_hooks() {
        assert!(DRIVER.setup_hint().contains("MIDI SysEx Cable"));
        let demo = DRIVER.demo_library();
        assert_eq!(demo.programs.len(), DRIVER.characters().len());
        assert!(!demo.programs.is_empty());
        for p in &demo.programs {
            DRIVER.check_program(&p.data).unwrap();
        }
        assert_eq!(DRIVER.list_facet(), None);
        assert_eq!(DRIVER.firmware(&[0xF0, 0xF7]), None);
    }
}
