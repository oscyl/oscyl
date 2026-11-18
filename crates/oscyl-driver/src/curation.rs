//! Curation and generation data a driver gives the apps: editor pages,
//! controls, help, generation recipes. Platform-neutral; apps render it.

use crate::contract::{DriverError, FieldInfo, UnitKind};

/// A titled group of fields on a [`Page`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Section {
    /// The section's title ("VCO 1").
    pub title: &'static str,
    /// Field indices, in display order.
    pub fields: Vec<usize>,
}

/// A top-level editor page (a stage of the signal path, a layer, …).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Page {
    /// The page's title ("Filter").
    pub title: &'static str,
    /// Symbolic icon name the apps ship, if any. For now these are the GTK
    /// app's symbolic icon ids (`oscyl-filter-symbolic`, …); other apps map
    /// them to their own icons.
    pub icon: Option<&'static str>,
    /// An introduction shown under the page heading, or empty.
    pub help: &'static str,
    /// The page's sections, in display order.
    pub sections: Vec<Section>,
}

/// How the editor lays out a driver's fields. Every field is in exactly
/// one section or in `excluded` (fields the editor does not show, such as
/// sequencer data that needs its own view).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Curation {
    /// The editor's pages, in display order.
    pub pages: Vec<Page>,
    /// The indices of the fields that the editor does not show.
    pub excluded: Vec<usize>,
}

/// `raw` for `f` with no curation: "Off"/"On" for a two-value field (the
/// default control is then a toggle), else the number.
pub fn format_raw_default(f: &FieldInfo, raw: u16) -> String {
    match (f.max - f.min == 1, raw == f.min) {
        (true, true) => "Off".into(),
        (true, false) => "On".into(),
        _ => raw.to_string(),
    }
}

/// One page per schema group, in the order the groups first appear, each
/// with one section of the same name. A starting point for a simple
/// driver's [`Driver::curation`](crate::Driver::curation).
pub fn curation_by_group(fields: &[FieldInfo]) -> Curation {
    let mut pages: Vec<Page> = vec![];
    for (i, f) in fields.iter().enumerate() {
        match pages.iter_mut().find(|p| p.title == f.group) {
            Some(p) => p.sections[0].fields.push(i),
            None => pages.push(Page {
                title: f.group,
                icon: None,
                help: "",
                sections: vec![Section {
                    title: f.group,
                    fields: vec![i],
                }],
            }),
        }
    }
    Curation {
        pages,
        excluded: vec![],
    }
}

/// The widget a field wants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Control {
    /// A numeric range: a dial or slider.
    Continuous,
    /// Off / On, for a two-value field.
    Toggle,
    /// Named values: `names[raw - min]`. Raw values past the end show as
    /// [`Driver::format_value`](crate::Driver::format_value) formats them.
    Choice(&'static [&'static str]),
    /// Named bands of raw values: name `i` covers raw `i * width` up to
    /// the next band; the last band extends to the field's max. To pick
    /// band `i`, set `i * width + width / 2`.
    Bands {
        /// The bands' names, from the lowest raw values.
        names: &'static [&'static str],
        /// How many raw values each band covers.
        width: u16,
    },
}

/// Help text for one field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Help {
    /// One line: what the control does to the sound, or empty.
    pub summary: &'static str,
    /// A non-obvious tip, or empty.
    pub detail: &'static str,
}

/// What generation may do to a field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mutability {
    /// Generation, vary and breed may set it.
    Free,
    /// Generation keeps it: the base program's value (init's without a
    /// base); vary leaves it; breed takes parent `a`'s. Tuning, MIDI and
    /// performance settings.
    Restricted,
    /// Never changed by generate (with a base), vary or breed: metadata
    /// set by the synth or the user, such as a loaded user unit's
    /// parameter types.
    Fixed,
}

/// What a [`Character`] is for; apps group characters by it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Family {
    /// Keys, organs, bells and plucks.
    Keys,
    /// Basses.
    Bass,
    /// Leads.
    Lead,
    /// Pads, strings and drones.
    Pad,
    /// Drums and rhythmic pulses.
    Rhythm,
    /// Sound effects: risers, sirens, impacts.
    Effect,
}

impl Family {
    /// Every family, in the order apps show them.
    pub const ALL: [Family; 6] = [
        Family::Keys,
        Family::Bass,
        Family::Lead,
        Family::Pad,
        Family::Rhythm,
        Family::Effect,
    ];

    /// Stable and lowercase, for bindings and command lines.
    pub fn id(self) -> &'static str {
        match self {
            Family::Keys => "keys",
            Family::Bass => "bass",
            Family::Lead => "lead",
            Family::Pad => "pad",
            Family::Rhythm => "rhythm",
            Family::Effect => "effect",
        }
    }

    /// For people.
    pub fn title(self) -> &'static str {
        match self {
            Family::Keys => "Keys",
            Family::Bass => "Bass",
            Family::Lead => "Lead",
            Family::Pad => "Pad",
            Family::Rhythm => "Rhythm",
            Family::Effect => "Effect",
        }
    }
}

/// A kind of sound a driver can generate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Character {
    /// Stable, lowercase, dash-separated.
    pub id: &'static str,
    /// The character's name, for people ("Fuzz bass").
    pub title: &'static str,
    /// One line on what it sounds like.
    pub hint: &'static str,
    /// The family that apps group it in.
    pub family: Family,
    /// The page whose graphs say most about this character, as an index
    /// into [`Curation::pages`].
    pub signature_page: Option<usize>,
}

/// How a driver names and explains one of the four [`Macros`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MacroInfo {
    /// The macro's name on this synth ("Bright").
    pub title: &'static str,
    /// What the macro changes on this synth.
    pub help: &'static str,
}

/// Generic macro titles and help, for a driver that says nothing better.
/// In [`Macros`] order: bright, movement, grit, space.
pub const MACROS: [MacroInfo; 4] = [
    MacroInfo {
        title: "Bright",
        help: "Filter openness and resonance",
    },
    MacroInfo {
        title: "Movement",
        help: "Modulation amount and speed",
    },
    MacroInfo {
        title: "Grit",
        help: "Drive, noise and other dirt",
    },
    MacroInfo {
        title: "Ambience",
        help: "Delay, reverb and other effects",
    },
];

/// A note on a piano roll, for [`Graph::Voices`]. Times and pitch 0..1.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Note {
    /// The voice that plays the note, from 0.
    pub voice: u8,
    /// When the note starts.
    pub start: f32,
    /// When the note ends.
    pub end: f32,
    /// How high the note is.
    pub pitch: f32,
}

/// An illustrative picture of part of a program, as plain numbers the apps
/// draw. Coordinates are 0..1 unless noted; none of it claims to match the
/// synth's audio.
#[derive(Debug, Clone, PartialEq)]
pub enum Graph {
    /// Samples of a waveform, −1..1.
    Wave(Vec<f32>),
    /// A filter response: `(x, y)` points, the cutoff's x, and the y of
    /// unity gain.
    Response {
        /// The curve, as `(frequency, gain)` points.
        points: Vec<(f32, f32)>,
        /// The cutoff's x.
        cutoff: f32,
        /// The y of unity gain.
        unity: f32,
    },
    /// Envelope corners as `(time, level)`: start, attack peak, decay end,
    /// sustain end, release end.
    Envelope([(f32, f32); 5]),
    /// Source levels.
    Levels(Vec<f32>),
    /// Echo taps `(time, level)`, a reverb tail `(length, level)` and a
    /// modulation sweep `(cycles, depth)`, each when on.
    Effects {
        /// The echo taps, as `(time, level)`.
        taps: Vec<(f32, f32)>,
        /// The reverb tail, as `(length, level)`, when the reverb is on.
        tail: Option<(f32, f32)>,
        /// The modulation sweep, as `(cycles, depth)`, when it is on.
        sweep: Option<(f32, f32)>,
    },
    /// What each voice plays.
    Voices(Vec<Note>),
}

/// A draggable point on a page's first graph; dragging it sets the fields
/// on its axes through [`Driver::drag`](crate::Driver::drag).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Handle {
    /// What it moves, for people ("Cutoff and resonance").
    pub name: &'static str,
    /// Where it is, in graph coordinates.
    pub at: (f32, f32),
    /// The field the horizontal axis sets, if any.
    pub x: Option<usize>,
    /// The field the vertical axis sets, if any.
    pub y: Option<usize>,
}

/// Four generic ways to shape a generated sound, each `0.0..=1.0`. Each
/// driver maps them onto its own fields.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Macros {
    /// Filter openness and resonance.
    pub bright: f32,
    /// Modulation amount and speed.
    pub movement: f32,
    /// Drive, noise and other dirt.
    pub grit: f32,
    /// Delay, reverb and other ambience.
    pub space: f32,
}

impl Default for Macros {
    fn default() -> Self {
        Macros {
            bright: 0.5,
            movement: 0.5,
            grit: 0.5,
            space: 0.5,
        }
    }
}

/// How a user unit's parameter reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnitParamKind {
    /// A percentage.
    Percent,
    /// A percentage either side of zero.
    Bipolar,
    /// A plain number: one of a list of choices.
    Choice,
}

/// A parameter a user unit declares, in its own values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnitParam {
    /// The parameter's name, as the unit declares it.
    pub name: String,
    /// The smallest value.
    pub min: i16,
    /// The largest value.
    pub max: i16,
    /// How the value reads.
    pub kind: UnitParamKind,
}

/// A user unit (a loaded oscillator or effect) a synth holds, as a source
/// generation may use.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaletteUnit {
    /// Which of [`Capabilities::units`](crate::Capabilities::units) it is.
    pub kind: UnitKind,
    /// Its slot, from 0.
    pub slot: u8,
    /// The unit's name, as it declares it.
    pub name: String,
    /// Its parameters, in order; `None` when they are not known (then
    /// generation leaves them alone).
    pub params: Option<Vec<UnitParam>>,
}

/// The extra sources a synth holds right now, beyond its built-in engines.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Palette {
    /// The units, in slot order.
    pub units: Vec<PaletteUnit>,
}

/// What to generate.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Recipe {
    /// Index into [`Driver::characters`](crate::Driver::characters).
    pub character: usize,
    /// The four dials.
    pub macros: Macros,
    /// The seed: the same recipe and seed make the same bytes.
    pub seed: u64,
    /// Indices into [`Curation::pages`] whose fields are copied from the
    /// base program as they are.
    pub keep: Vec<usize>,
    /// User units the program may select. Empty: none is used. Not empty:
    /// the driver uses one (or an oscillator and an effect) wherever the
    /// character can take it, with each parameter inside its declared
    /// range, so a caller decides which sounds of a hand get units by
    /// which recipes carry a palette. A program never selects a unit the
    /// palette lacks.
    pub palette: Palette,
}

/// SplitMix64's output step.
fn mix64(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Step `t` (`0.0..=1.0`) of the way from `a` to `b`, over the free fields
/// ([`Driver::morph`](crate::Driver::morph)'s default): a dial ([`Control::Continuous`] in `a`)
/// interpolates; every other free field is `a`'s until `t` reaches its
/// group's threshold (from `seed`, within 0.15..0.85), then `b`'s. So 0 is
/// `a`, 1 is `b`'s sound, and a field never turns back on the way.
/// Non-free fields and the name are `a`'s. Parent values out of range are
/// clamped; a value the program refuses stays `a`'s.
pub fn morph_fields<D: crate::Driver + ?Sized>(
    d: &D,
    a: &[u8],
    b: &[u8],
    t: f32,
    seed: u64,
) -> Result<Vec<u8>, DriverError> {
    morph_fields_by(d, a, b, t, seed, |i| d.control(a, i) == Control::Continuous)
}

/// [`morph_fields`] for a driver that knows better than its controls
/// which fields are dials (`dial`, by field index): a number that names
/// something (a destination) must switch, not slide.
pub fn morph_fields_by<D: crate::Driver + ?Sized>(
    d: &D,
    a: &[u8],
    b: &[u8],
    t: f32,
    seed: u64,
    dial: impl Fn(usize) -> bool,
) -> Result<Vec<u8>, DriverError> {
    let t = t.clamp(0.0, 1.0);
    let mut out = a.to_vec();
    for (i, f) in d.fields().iter().enumerate() {
        if d.mutability(i) != Mutability::Free {
            continue;
        }
        let (x, y) = (d.get(a, i)?, d.get(b, i)?);
        let v = if dial(i) {
            (f32::from(x) + (f32::from(y) - f32::from(x)) * t).round() as u16
        } else {
            // FNV-1a of the group's name: one threshold per group.
            let h = f.group.bytes().fold(0xCBF2_9CE4_8422_2325u64, |h, c| {
                (h ^ u64::from(c)).wrapping_mul(0x0100_0000_01B3)
            });
            let u = (mix64(seed ^ h) >> 40) as f32 / (1u64 << 24) as f32;
            if t >= 0.15 + 0.7 * u { y } else { x }
        };
        let _ = d.set(&mut out, i, v.clamp(f.min, f.max));
    }
    Ok(out)
}

pub(crate) fn no_generation() -> DriverError {
    DriverError("this driver does not generate programs".into())
}
