//! The patch maker for Oscyl's oscillator unit: oscillators made of data for
//! one prebuilt logue SDK unit (`units/oscyl-osc`, C), so every app can
//! make new ones without a compiler.
//!
//! The unit holds fixed DSP blocks and a marked *patch region*; a [`Patch`]
//! names which blocks run, in what order, with what constants, what Shape,
//! Shift-Shape, an internal LFO and envelope, the key and the unit's six
//! parameters move, and up to eight single-cycle tables. [`build`] writes
//! one into the unit's payload with a header naming it and its parameters.
//! [`make`] deals whole oscillators by [`Family`], varies and breeds them;
//! [`waves`] makes tables; [`render()`] is the unit's DSP in Rust, for
//! pictures and level checks (a test holds it to the C).
//!
//! The region's bytes ([`Patch::to_bytes`]): a 20-byte head (`OSCYLPAT`,
//! version 1, block, mod and table counts, body length u16 LE, two zero
//! bytes, CRC-32 of the body u32 LE), then the body: globals (8 bytes:
//! gain, LFO rate, LFO shape, envelope attack, decay, sustain, two zero),
//! names (96 bytes: family, parameter count, the name in 14 bytes, six
//! parameter names in 12 bytes each, eight zero; the unit does not read
//! them), blocks (8 bytes: type, seven constants), mods (4 bytes: source,
//! block, constant, signed amount in 64ths) and tables (256 i16 LE each).

use std::io::Write;

use crate::Error;
use crate::unit::{Platform, Unit};

pub mod make;
pub mod render;
pub mod waves;

pub use make::{Family, breed, generate, vary};
pub use render::{Play, render};

/// The region's size in the unit.
pub const PATCH_SIZE: usize = 4608;
pub const MAX_BLOCKS: usize = 8;
pub const MAX_MODS: usize = 16;
pub const MAX_TABLES: usize = 8;
pub const TABLE_LEN: usize = 256;
/// A unit parameter's name, at most.
pub const PARAM_NAME_LEN: usize = 12;
/// An oscillator's name, at most.
pub const NAME_LEN: usize = 13;
/// What the blocks of one patch may cost together ([`Kind::cost`]).
// ponytail: a paper budget (a supersaw and two cheap blocks); measure on an
// xd and tune it with real cycle counts.
pub const BUDGET: u32 = 16;
/// The loudest a patch may play at any probe setting ([`Patch::peak`]).
pub const MAX_PEAK: f32 = 0.9;

const MARK: &[u8; 8] = b"OSCYLPAT";
const HEAD: usize = 20;
const GLOBALS: usize = 8;
const NAMES: usize = 96;
/// Oscyl's developer id in unit headers: "OSCY".
pub const DEV_ID: u32 = 0x4F53_4359;

/// A single-cycle table.
pub type Table = [i16; TABLE_LEN];

/// A DSP block of the unit (`engine.h`); its constants are 0-255 and mean,
/// in order:
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Kind {
    /// level.
    Saw = 1,
    /// position across the tables, level (a sine without tables).
    Wavetable,
    /// ratio (1-8), index, feedback, level.
    Fm,
    /// detune, level.
    Supersaw,
    /// level, sine to square (an octave down).
    Sub,
    /// level, hold (sample and hold).
    Noise,
    /// amount: bends the phase the next sources read.
    PhaseDist,
    /// ratio (1-8): a hard-synced phase for the next sources.
    Sync,
    /// drive, bias, mix.
    Fold,
    /// bits, rate, mix.
    Crush,
    /// cutoff (1 to 128 times the note's pitch).
    Lowpass,
    /// vowel, resonance, mix, shift.
    Formant,
}

impl Kind {
    pub const ALL: [Kind; 12] = [
        Kind::Saw,
        Kind::Wavetable,
        Kind::Fm,
        Kind::Supersaw,
        Kind::Sub,
        Kind::Noise,
        Kind::PhaseDist,
        Kind::Sync,
        Kind::Fold,
        Kind::Crush,
        Kind::Lowpass,
        Kind::Formant,
    ];

    fn from_u8(b: u8) -> Option<Kind> {
        Kind::ALL.into_iter().find(|k| *k as u8 == b)
    }

    /// Whether it makes sound (rather than shaping it).
    pub fn is_source(self) -> bool {
        (self as u8) <= Kind::Noise as u8
    }

    /// Its share of the unit's time, in [`BUDGET`]'s units.
    pub fn cost(self) -> u32 {
        match self {
            Kind::Noise | Kind::PhaseDist | Kind::Sync | Kind::Crush | Kind::Lowpass => 1,
            Kind::Saw | Kind::Sub => 2,
            Kind::Wavetable | Kind::Fold => 3,
            Kind::Fm => 4,
            Kind::Formant => 5,
            Kind::Supersaw => 8,
        }
    }
}

/// A block and its seven constants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Block {
    pub kind: Kind,
    pub k: [u8; 7],
}

/// What a mod reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Source {
    /// The Shape knob (with the synth's LFO on it), 0 to 1.
    Shape,
    /// Shift-Shape, 0 to 1.
    ShiftShape,
    /// The patch's LFO, -1 to 1.
    Lfo,
    /// The patch's envelope, 0 to 1, from each note.
    Env,
    /// One of the unit's six parameters (0-5), 0 to 1.
    Param(u8),
    /// The note, 0 at middle C, 1 five octaves up.
    Key,
}

impl Source {
    fn to_u8(self) -> u8 {
        match self {
            Source::Shape => 0,
            Source::ShiftShape => 1,
            Source::Lfo => 2,
            Source::Env => 3,
            Source::Param(i) => 4 + i.min(5),
            Source::Key => 10,
        }
    }

    fn from_u8(b: u8) -> Option<Source> {
        Some(match b {
            0 => Source::Shape,
            1 => Source::ShiftShape,
            2 => Source::Lfo,
            3 => Source::Env,
            4..=9 => Source::Param(b - 4),
            10 => Source::Key,
            _ => return None,
        })
    }
}

/// A mod: `amount` 64ths of `source` added to constant `param` of block
/// `block` (as 0 to 1), each render call.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Mod {
    pub source: Source,
    pub block: u8,
    pub param: u8,
    pub amount: i8,
}

/// An oscillator: what the unit plays, and its name and parameters' names.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Patch {
    /// At most [`NAME_LEN`] ASCII characters.
    pub name: String,
    pub family: Family,
    /// The unit's parameters, in order: at most six names of at most
    /// [`PARAM_NAME_LEN`] ASCII characters. Parameter `i` is
    /// [`Source::Param`]`(i)`, 0 to 100 %.
    pub params: Vec<String>,
    /// Output gain in 64ths.
    pub gain: u8,
    /// 0.05 Hz to 25.6 Hz.
    pub lfo_rate: u8,
    /// 0 sine, 1 triangle, 2 random steps.
    pub lfo_shape: u8,
    /// 1 ms to 2 s.
    pub attack: u8,
    /// 1 ms to 2 s.
    pub decay: u8,
    pub sustain: u8,
    pub blocks: Vec<Block>,
    pub mods: Vec<Mod>,
    pub tables: Vec<Table>,
}

/// CRC-32 (IEEE), as the unit checks it.
pub(crate) fn crc32(bytes: &[u8]) -> u32 {
    !bytes.iter().fold(0xFFFF_FFFFu32, |c, b| {
        (0..8).fold(c ^ u32::from(*b), |c, _| {
            (c >> 1) ^ (0xEDB8_8320 & 0u32.wrapping_sub(c & 1))
        })
    })
}

fn put_name(out: &mut Vec<u8>, s: &str, len: usize) {
    let mut b = s.as_bytes().to_vec();
    b.resize(len, 0);
    out.extend_from_slice(&b[..len]);
}

fn text(raw: &[u8]) -> String {
    let end = raw.iter().position(|b| *b == 0).unwrap_or(raw.len());
    String::from_utf8_lossy(&raw[..end]).into_owned()
}

fn malformed(why: &'static str) -> Error {
    Error::Malformed(why)
}

impl Patch {
    /// Its blocks' cost together ([`Kind::cost`]).
    pub fn cost(&self) -> u32 {
        self.blocks.iter().map(|b| b.kind.cost()).sum()
    }

    fn body(&self) -> Vec<u8> {
        let mut b = vec![
            self.gain,
            self.lfo_rate,
            self.lfo_shape,
            self.attack,
            self.decay,
            self.sustain,
            0,
            0,
        ];
        b.push(self.family as u8);
        b.push(self.params.len() as u8);
        put_name(&mut b, &self.name, 14);
        for i in 0..6 {
            put_name(
                &mut b,
                self.params.get(i).map_or("", String::as_str),
                PARAM_NAME_LEN,
            );
        }
        b.extend_from_slice(&[0; 8]);
        for k in &self.blocks {
            b.push(k.kind as u8);
            b.extend_from_slice(&k.k);
        }
        for m in &self.mods {
            b.extend_from_slice(&[m.source.to_u8(), m.block, m.param, m.amount as u8]);
        }
        for t in &self.tables {
            for s in t {
                b.extend_from_slice(&s.to_le_bytes());
            }
        }
        b
    }

    /// The region's bytes (see the module's documentation).
    pub fn to_bytes(&self) -> Vec<u8> {
        let body = self.body();
        let mut out = MARK.to_vec();
        out.extend_from_slice(&[
            1,
            self.blocks.len() as u8,
            self.mods.len() as u8,
            self.tables.len() as u8,
        ]);
        out.extend_from_slice(&(body.len() as u16).to_le_bytes());
        out.extend_from_slice(&[0, 0]);
        out.extend_from_slice(&crc32(&body).to_le_bytes());
        out.extend_from_slice(&body);
        out
    }

    /// A patch from its region's bytes ([`Patch::to_bytes`]); refused when
    /// they do not check out. Not [`Patch::validate`]d.
    pub fn from_bytes(p: &[u8]) -> Result<Patch, Error> {
        if p.len() < HEAD || &p[..8] != MARK || p[8] != 1 {
            return Err(malformed("not an Oscyl oscillator"));
        }
        let (nb, nm, nt) = (p[9] as usize, p[10] as usize, p[11] as usize);
        let len = usize::from(u16::from_le_bytes([p[12], p[13]]));
        if nb > MAX_BLOCKS
            || nm > MAX_MODS
            || nt > MAX_TABLES
            || len != GLOBALS + NAMES + 8 * nb + 4 * nm + 2 * TABLE_LEN * nt
            || p.len() < HEAD + len
        {
            return Err(malformed("an oscillator's counts do not add up"));
        }
        let body = &p[HEAD..HEAD + len];
        if crc32(body) != u32::from_le_bytes([p[16], p[17], p[18], p[19]]) {
            return Err(malformed("an oscillator's check sum does not match"));
        }
        let g = &body[..GLOBALS];
        let n = &body[GLOBALS..GLOBALS + NAMES];
        let family = Family::ALL
            .into_iter()
            .find(|f| *f as u8 == n[0])
            .ok_or_else(|| malformed("an oscillator of no known family"))?;
        let params = (0..usize::from(n[1]).min(6))
            .map(|i| text(&n[16 + i * PARAM_NAME_LEN..16 + (i + 1) * PARAM_NAME_LEN]))
            .collect();
        let mut at = GLOBALS + NAMES;
        let mut blocks = vec![];
        for _ in 0..nb {
            let b = &body[at..at + 8];
            blocks.push(Block {
                kind: Kind::from_u8(b[0]).ok_or_else(|| malformed("an unknown block"))?,
                k: b[1..8].try_into().expect("seven"),
            });
            at += 8;
        }
        let mut mods = vec![];
        for _ in 0..nm {
            let m = &body[at..at + 4];
            mods.push(Mod {
                source: Source::from_u8(m[0]).ok_or_else(|| malformed("an unknown source"))?,
                block: m[1],
                param: m[2],
                amount: m[3] as i8,
            });
            at += 4;
        }
        let tables = (0..nt)
            .map(|i| {
                let t = &body[at + i * 2 * TABLE_LEN..];
                std::array::from_fn(|j| i16::from_le_bytes([t[2 * j], t[2 * j + 1]]))
            })
            .collect();
        Ok(Patch {
            name: text(&n[2..16]),
            family,
            params,
            gain: g[0],
            lfo_rate: g[1],
            lfo_shape: g[2],
            attack: g[3],
            decay: g[4],
            sustain: g[5],
            blocks,
            mods,
            tables,
        })
    }

    /// The loudest it plays across the probe settings (every knob low,
    /// middle, high and mixed, three notes apart), after the unit's soft
    /// knee at 0.75.
    pub fn peak(&self) -> f32 {
        render::probes()
            .iter()
            .map(|p| peak(&render(self, p, 4096)))
            .fold(0.0, f32::max)
    }

    /// Why the unit could not play it as made, if it could not: names too
    /// long or not ASCII, too many blocks, mods, tables or parameters,
    /// a mod into nothing, no sound source, over [`BUDGET`], or louder
    /// than [`MAX_PEAK`] or near silent (RMS under 0.03 once a note has
    /// sounded for 21 ms) at a probe setting.
    pub fn validate(&self) -> Result<(), String> {
        let ascii = |s: &str, n: usize| s.is_ascii() && !s.contains('\0') && s.len() <= n;
        if self.name.is_empty() || !ascii(&self.name, NAME_LEN) {
            return Err(format!("name {:?}", self.name));
        }
        if self.params.len() > 6 || !self.params.iter().all(|p| ascii(p, PARAM_NAME_LEN)) {
            return Err(format!("parameters {:?}", self.params));
        }
        if self.blocks.len() > MAX_BLOCKS
            || self.mods.len() > MAX_MODS
            || self.tables.len() > MAX_TABLES
        {
            return Err("too many blocks, mods or tables".into());
        }
        if !self.blocks.iter().any(|b| b.kind.is_source()) {
            return Err("no sound source".into());
        }
        for m in &self.mods {
            let to_param = match m.source {
                Source::Param(i) => usize::from(i) < self.params.len(),
                _ => true,
            };
            if usize::from(m.block) >= self.blocks.len() || m.param >= 7 || !to_param {
                return Err(format!("mod {m:?} leads nowhere"));
            }
        }
        if self.cost() > BUDGET {
            return Err(format!("costs {} of {BUDGET}", self.cost()));
        }
        let (mut loud, mut quiet) = (0.0f32, 1.0f32);
        for p in render::probes() {
            let s = render(self, &p, 4096);
            let body = &s[1024..];
            let rms = (body.iter().map(|x| x * x).sum::<f32>() / body.len() as f32).sqrt();
            loud = loud.max(peak(&s));
            quiet = quiet.min(rms);
        }
        if loud > MAX_PEAK {
            return Err(format!("peaks at {loud:.2}"));
        }
        if quiet < 0.03 {
            return Err(format!("near silent (RMS {quiet:.3}) at a probe"));
        }
        Ok(())
    }

    /// Set the gain so its loudest probe peaks at `target` (below the
    /// knee, where the level is linear in the gain).
    pub fn normalize(&mut self, target: f32) {
        for trial in [16u8, 2] {
            self.gain = trial;
            let p = self.peak();
            if p <= 0.0 {
                return;
            }
            if p < 0.7 {
                let g = f32::from(trial) * target / p;
                self.gain = g.round().clamp(1.0, 255.0) as u8;
                return;
            }
        }
    }
}

fn peak(s: &[f32]) -> f32 {
    s.iter().fold(0.0, |m, x| m.max(x.abs()))
}

/// The unit as built, per platform (`scripts/build-units.sh`).
fn payload(platform: &Platform) -> Option<&'static [u8]> {
    match platform.name {
        "minilogue-xd" => Some(include_bytes!(
            "../../../../units/oscyl-osc/payload/minilogue-xd.bin"
        )),
        "prologue" => Some(include_bytes!(
            "../../../../units/oscyl-osc/payload/prologue.bin"
        )),
        _ => None,
    }
}

/// Whether `platform` runs the oscillator unit.
pub fn supports(platform: &Platform) -> bool {
    payload(platform).is_some()
}

/// `patch` as a unit file for `platform` (`.mnlgxdunit`, `.prlgunit`): the
/// unit's payload with the patch in its region, and a manifest naming
/// it, its parameters (0 to 100 %) and an id of its own (its CRC), so a
/// synth tells two oscillators apart.
pub fn file(platform: &Platform, patch: &Patch) -> Result<Vec<u8>, Error> {
    patch.validate().map_err(Error::NotALibrary)?;
    let mut payload = payload(platform)
        .ok_or_else(|| Error::NotALibrary(format!("no oscillator unit for the {}", platform.name)))?
        .to_vec();
    let at = payload
        .windows(MARK.len())
        .position(|w| w == MARK)
        .expect("the unit's payload holds its region");
    let bytes = patch.to_bytes();
    payload[at..at + bytes.len()].copy_from_slice(&bytes);
    let params: Vec<_> = patch
        .params
        .iter()
        .map(|p| serde_json::json!([p, 0, 100, "%"]))
        .collect();
    let manifest = serde_json::json!({"header": {
        "platform": platform.name, "module": "osc", "api": "1.1-0",
        "dev_id": DEV_ID, "prg_id": crc32(&bytes), "version": "1.0-0",
        "name": patch.name, "num_param": params.len(), "params": params,
    }});
    let arch = |e: zip::result::ZipError| Error::Archive(e.to_string());
    let io = |e: std::io::Error| Error::Archive(e.to_string());
    let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default();
    zip.start_file(format!("{}/manifest.json", patch.name), options)
        .map_err(arch)?;
    zip.write_all(manifest.to_string().as_bytes()).map_err(io)?;
    zip.start_file(format!("{}/payload.bin", patch.name), options)
        .map_err(arch)?;
    zip.write_all(&payload).map_err(io)?;
    Ok(zip.finish().map_err(arch)?.into_inner())
}

/// `patch` as a unit for `platform`, read back from its [`file()`].
pub fn build(platform: &Platform, patch: &Patch) -> Result<Unit, Error> {
    Unit::read(platform, std::io::Cursor::new(file(platform, patch)?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::msg::Module;
    use crate::unit::ParamKind;

    const XD: Platform = Platform {
        name: "minilogue-xd",
        id: 2,
    };

    #[test]
    fn a_patch_survives_its_bytes_and_bad_bytes_are_refused() {
        let p = generate(Family::Wavetable, 7);
        let b = p.to_bytes();
        assert!(b.len() <= PATCH_SIZE);
        assert_eq!(Patch::from_bytes(&b).unwrap(), p);
        let mut bad = b.clone();
        *bad.last_mut().unwrap() ^= 1;
        assert!(Patch::from_bytes(&bad).is_err(), "CRC");
        assert!(Patch::from_bytes(&b[..30]).is_err());
        assert!(Patch::from_bytes(b"OSCYLPAT").is_err());
    }

    #[test]
    fn a_unit_names_the_oscillator_and_its_parameters() {
        let p = generate(Family::Fm, 3);
        let unit = build(&XD, &p).unwrap();
        let info = unit.info();
        assert_eq!(
            (info.module, info.platform, &*info.name),
            (Module::Osc, 2, &*p.name)
        );
        assert_eq!(info.dev_id, DEV_ID);
        let params = info.params.unwrap();
        assert_eq!(
            params.iter().map(|q| q.name.clone()).collect::<Vec<_>>(),
            p.params
        );
        assert!(
            params
                .iter()
                .all(|q| (q.min, q.max, q.kind) == (0, 100, ParamKind::Percent))
        );
        // The region holds the patch; another patch is another unit.
        let at = unit.payload.windows(8).position(|w| w == MARK).unwrap();
        let back = Patch::from_bytes(&unit.payload[at..]).unwrap();
        assert_eq!(back, p);
        let other = build(&XD, &generate(Family::Fm, 4)).unwrap().info();
        assert_ne!(other.prg_id, info.prg_id);
        assert!(
            build(
                &Platform {
                    name: "nts-1",
                    id: 3
                },
                &p
            )
            .is_err()
        );
    }

    #[test]
    fn validation_bounds_cost_wiring_and_level() {
        let good = generate(Family::Hybrid, 11);
        assert_eq!(good.validate(), Ok(()));
        let mut p = good.clone();
        p.blocks = vec![
            Block {
                kind: Kind::Supersaw,
                k: [0, 255, 0, 0, 0, 0, 0],
            };
            3
        ];
        p.mods.clear();
        assert!(p.validate().unwrap_err().starts_with("costs"));
        let mut p = good.clone();
        p.blocks.retain(|b| !b.kind.is_source());
        p.mods.clear();
        assert_eq!(p.validate(), Err("no sound source".into()));
        let mut p = good.clone();
        p.mods.push(Mod {
            source: Source::Shape,
            block: 9,
            param: 0,
            amount: 1,
        });
        assert!(p.validate().unwrap_err().contains("leads nowhere"));
        let mut p = good.clone();
        p.gain = 255;
        assert!(p.validate().unwrap_err().starts_with("peaks"));
        let mut p = good;
        p.name = "a name far too long".into();
        assert!(p.validate().is_err());
    }
}
