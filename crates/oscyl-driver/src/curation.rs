//! Curation and generation data a driver gives the apps: editor pages,
//! controls, help, generation recipes. Platform-neutral; apps render it.

use crate::contract::{DriverError, FieldInfo};

/// A titled group of fields on a [`Page`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Section {
    pub title: &'static str,
    /// Field indices, in display order.
    pub fields: Vec<usize>,
}

/// A top-level editor page (a stage of the signal path, a layer, …).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Page {
    pub title: &'static str,
    /// Symbolic icon name the apps ship, if any. For now these are the GTK
    /// app's symbolic icon ids (`oscyl-filter-symbolic`, …); other apps map
    /// them to their own icons.
    pub icon: Option<&'static str>,
    /// An introduction shown under the page heading, or empty.
    pub help: &'static str,
    pub sections: Vec<Section>,
}

/// How the editor lays out a driver's fields. Every field is in exactly
/// one section or in `excluded` (fields the editor does not show, such as
/// sequencer data that needs its own view).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Curation {
    pub pages: Vec<Page>,
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
        names: &'static [&'static str],
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

/// A kind of sound a driver can generate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Character {
    /// Stable, lowercase, dash-separated.
    pub id: &'static str,
    pub title: &'static str,
    /// One line on what it sounds like.
    pub hint: &'static str,
    /// The page whose graphs say most about this character, as an index
    /// into [`Curation::pages`].
    pub signature_page: Option<usize>,
}

/// How a driver names and explains one of the four [`Macros`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MacroInfo {
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
    pub voice: u8,
    pub start: f32,
    pub end: f32,
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
        points: Vec<(f32, f32)>,
        cutoff: f32,
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
        taps: Vec<(f32, f32)>,
        tail: Option<(f32, f32)>,
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

/// What to generate.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Recipe {
    /// Index into [`Driver::characters`](crate::Driver::characters).
    pub character: usize,
    pub macros: Macros,
    pub seed: u64,
    /// Indices into [`Curation::pages`] whose fields are copied from the
    /// base program as they are.
    pub keep: Vec<usize>,
}

pub(crate) fn no_generation() -> DriverError {
    DriverError("this driver does not generate programs".into())
}
