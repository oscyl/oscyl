//! A deterministic generator for the Prophet Rev2.
//!
//! It sets only layer A's sound fields ([`Mutability::Free`]): oscillators,
//! filter, the three envelopes (the third with its destination), the four
//! LFOs with their destinations, the first two modulation slots and the
//! effect. Everything else keeps the init (or, for vary and breed, the
//! parent) value: layer B, volume, the other modulation slots and the
//! controllers, voice, arp and sequencer. Each field draws from its own
//! random stream (seed and offset), so a macro changes only arithmetic,
//! never which numbers a field gets.
//!
//! Each character says what it needs of a field ([`special`]); the rest
//! comes from ranges all share ([`common`]).
//!
//! Coherence: something sounds (both oscillators on, or one with the mix
//! near the middle, or noise), the filter is open at least 40 of 164,
//! resonance stays at or under 110, the effect mix at or under 100, the
//! amp envelope amount is at least 90, and a note either sustains or
//! decays slowly enough to hear.
//!
//! Limits: ranges are chosen by ear on paper, not on hardware. The
//! modulation destinations and sources ([`dest`], [`source`]) are numbered
//! in the order the User's Guide lists them, which no capture here has
//! confirmed; only the first fifteen destinations are used. The effect
//! parameters are drawn over their whole range because the docs give no
//! meaning for them.

use oscyl_driver::{
    Character, Control, DriverError, Family, MacroInfo, Macros, Mutability, Recipe,
};

use crate::curate::{CURATION, control, index};
use crate::{Field, Program, fields};

pub(crate) const CHARACTERS: &[Character] = &[
    Character {
        id: "pad",
        title: "Pad",
        hint: "Slow swell, long release",
        family: Family::Pad,
        signature_page: Some(2), // Envelopes
    },
    Character {
        id: "pluck",
        title: "Pluck",
        hint: "Short and percussive",
        family: Family::Keys,
        signature_page: Some(2), // Envelopes
    },
    Character {
        id: "bass",
        title: "Bass",
        hint: "Low and tight",
        family: Family::Bass,
        signature_page: Some(2), // Envelopes
    },
    Character {
        id: "lead",
        title: "Lead",
        hint: "Bright, sustained solo voice",
        family: Family::Lead,
        signature_page: Some(2), // Envelopes
    },
    Character {
        id: "keys",
        title: "Keys",
        hint: "Decaying, playable chords",
        family: Family::Keys,
        signature_page: Some(2), // Envelopes
    },
    Character {
        id: "strings",
        title: "Strings",
        hint: "Bowed ensemble",
        family: Family::Pad,
        signature_page: Some(2), // Envelopes
    },
    Character {
        id: "brass",
        title: "Brass",
        hint: "Bold swell with a filter blat",
        family: Family::Lead,
        signature_page: Some(2), // Envelopes
    },
    Character {
        id: "organ",
        title: "Organ",
        hint: "Drawbars, straight on and off",
        family: Family::Keys,
        signature_page: Some(2), // Envelopes
    },
    Character {
        id: "sub-bass",
        title: "Sub Bass",
        hint: "Clean weight below",
        family: Family::Bass,
        signature_page: Some(2), // Envelopes
    },
    Character {
        id: "sync-lead",
        title: "Sync Lead",
        hint: "Hard sync sweep",
        family: Family::Lead,
        signature_page: Some(2), // Envelopes
    },
    Character {
        id: "bell",
        title: "Bell",
        hint: "Metallic ring, long decay",
        family: Family::Keys,
        signature_page: Some(2), // Envelopes
    },
    Character {
        id: "kick",
        title: "Kick",
        hint: "Bass drum: a pitch drop",
        family: Family::Rhythm,
        signature_page: Some(2), // Envelopes
    },
    Character {
        id: "snare",
        title: "Snare",
        hint: "Noise crack with a body",
        family: Family::Rhythm,
        signature_page: Some(2), // Envelopes
    },
    Character {
        id: "laser",
        title: "Laser",
        hint: "A fast zap",
        family: Family::Effect,
        signature_page: Some(2), // Envelopes
    },
    Character {
        id: "wind",
        title: "Wind",
        hint: "Whistling air the keys play",
        family: Family::Effect,
        signature_page: Some(2), // Envelopes
    },
    Character {
        id: "siren",
        title: "Siren",
        hint: "Rising and falling wail",
        family: Family::Effect,
        signature_page: Some(2), // Envelopes
    },
];

/// What each macro changes here, in [`Macros`] order.
pub(crate) const MACRO_INFO: [MacroInfo; 4] = [
    MacroInfo {
        title: "Bright",
        help: "Filter cutoff and resonance",
    },
    MacroInfo {
        title: "Movement",
        help: "LFO amounts, envelope amounts and pulse width sweep",
    },
    MacroInfo {
        title: "Grit",
        help: "Noise, oscillator slop, sync and filter audio mod",
    },
    MacroInfo {
        title: "Ambience",
        help: "Effect, its mix and the stereo spread",
    },
];

/// A generated program's name: the character and four hex digits of the
/// seed.
pub(crate) fn name(c: &Character, seed: u64) -> String {
    format!("{} {:04X}", c.title, seed & 0xFFFF)
}

/// Layer A names the generator may set.
const FREE: &[&str] = &[
    "Osc mix",
    "Sub osc level",
    "Noise level",
    "Sync",
    "Osc slop",
    "Filter cutoff",
    "Resonance",
    "Filter key amount",
    "Filter audio mod",
    "Filter poles",
    "Pan spread",
    "FX select",
    "FX on",
    "FX mix",
    "FX param 1",
    "FX param 2",
];

pub(crate) fn mutability(f: &Field) -> Mutability {
    let Some(b) = f.name.strip_prefix("A ") else {
        return Mutability::Restricted;
    };
    let osc = ["frequency", "fine tune", "shape", "shape mod"]
        .iter()
        .any(|w| (b.starts_with("Osc 1 ") || b.starts_with("Osc 2 ")) && b.ends_with(w));
    let env = b.starts_with("Filter env ")
        || b.starts_with("Amp env ")
        || (b.starts_with("Env 3 ") && b != "Env 3 repeat");
    let lfo = b.starts_with("LFO ")
        && ["rate", "shape", "amount", "destination", "key sync"]
            .iter()
            .any(|w| b.ends_with(w));
    let slot = b.starts_with("Mod 1 ") || b.starts_with("Mod 2 ");
    if osc || env || lfo || slot || FREE.contains(&b) {
        Mutability::Free
    } else {
        Mutability::Restricted
    }
}

/// Modulation destinations, in the User's Guide's order (see the module
/// docs: unconfirmed).
mod dest {
    pub const OFF: u16 = 0;
    pub const OSC_1: u16 = 1;
    pub const FREQ: u16 = 3;
    pub const SHAPE: u16 = 9;
    pub const CUTOFF: u16 = 10;
    pub const VCA: u16 = 13;
    pub const PAN: u16 = 14;
    /// The ones vary may switch to.
    pub const KNOWN: &[u16] = &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14];
}

/// Modulation sources, likewise.
mod source {
    pub const FILTER_ENV: u16 = 9;
    pub const NOISE: u16 = 20;
    /// The ones vary may switch to: off, the LFOs, the envelopes,
    /// velocity, note number and noise.
    pub const KNOWN: &[u16] = &[0, 5, 6, 7, 8, 9, 10, 11, 18, 19, 20];
}

/// The values a field that names something may take, or `None` for a
/// field that is an amount or a plain choice.
fn named(b: &str) -> Option<&'static [u16]> {
    if b.ends_with(" destination") {
        Some(dest::KNOWN)
    } else if b.ends_with(" source") {
        Some(source::KNOWN)
    } else {
        None
    }
}

/// Whether field `i` is an amount to slide, not a choice to switch.
pub(crate) fn dial(i: usize) -> bool {
    control(i) == Control::Continuous && named(&fields::all()[i].name[2..]).is_none()
}

/// SplitMix64 on `seed` mixed with `key`, as `0.0..1.0`.
fn roll(seed: u64, key: u64) -> f32 {
    let mut z =
        (seed ^ key.wrapping_mul(0xD1B5_4A32_D192_ED03)).wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^= z >> 31;
    (z >> 40) as f32 / (1u64 << 24) as f32
}

/// Field `f`'s `k`-th random number.
fn roll_field(seed: u64, f: &Field, k: u64) -> f32 {
    roll(seed, (u64::from(f.offset) << 8) | k)
}

/// `lo..=hi` scaled by `t` from `lo`: `lo + t * (hi - lo)`.
fn by(lo: u16, hi: u16, t: f32) -> u16 {
    lo + (f32::from(hi - lo) * t.clamp(0.0, 1.0)).round() as u16
}

/// How a fresh program draws a field: anywhere in a range, or one of a
/// few values.
enum Draw {
    Range(u16, u16),
    Pick(&'static [u16]),
}
use Draw::*;

fn r(lo: u16, hi: u16) -> Option<Draw> {
    Some(Range(lo, hi.max(lo)))
}

fn k(values: &'static [u16]) -> Option<Draw> {
    Some(Pick(values))
}

/// An envelope's `(attack, decay, sustain, release)` ranges for the stage
/// field `b` ends with.
fn adsr(b: &str, a: [(u16, u16); 4]) -> Option<Draw> {
    let (lo, hi) = match b.rsplit(' ').next() {
        Some("attack") => a[0],
        Some("decay") => a[1],
        Some("sustain") => a[2],
        Some("release") => a[3],
        _ => return None,
    };
    r(lo, hi)
}

/// `lo` rising with `t`, and `width` above it, within the cutoff's range.
fn band(lo: (u16, u16), width: u16, t: f32) -> Option<Draw> {
    let lo = by(lo.0, lo.1, t);
    r(lo, (lo + width).min(164))
}

// Oscillator shapes.
const OSC_OFF: u16 = 0;
const SAW: u16 = 1;
const SAW_TRI: u16 = 2;
const TRI: u16 = 3;
const PULSE: u16 = 4;
// LFO shapes.
const L_TRI: u16 = 1;
const L_SAW: u16 = 2;
const L_SQUARE: u16 = 4;
const L_RANDOM: u16 = 5;
// Effects (the Guide's order): delays, chorus, phasers, flangers, reverb,
// ring mod, distortion.
const FX_MONO_DELAY: u16 = 1;
const FX_STEREO_DELAY: u16 = 2;
const FX_BBD: u16 = 3;
const FX_CHORUS: u16 = 4;
const FX_PHASER: u16 = 5;
const FX_PHASER_LOW: u16 = 6;
const FX_PHASE_MOD: u16 = 7;
const FX_FLANGER: u16 = 8;
const FX_FLANGER_2: u16 = 9;
const FX_REVERB: u16 = 10;
const FX_RING_MOD: u16 = 11;
const FX_DISTORTION: u16 = 12;

/// What character `id` wants of layer A field `b`, or `None` for
/// [`common`]'s. Oscillator frequencies are semitones (24 is the init
/// program's); bipolar amounts are zero at 127.
fn special(id: &str, b: &str, m: &Macros) -> Option<Draw> {
    let Macros {
        bright: br,
        movement: mv,
        grit: g,
        space: sp,
    } = *m;
    let amp = |a| b.strip_prefix("Amp env ").and_then(|_| adsr(b, a));
    let filter = |a| b.strip_prefix("Filter env ").and_then(|_| adsr(b, a));
    let env3 = |a| b.strip_prefix("Env 3 ").and_then(|_| adsr(b, a));
    match id {
        "pad" => match b {
            "Osc 1 shape" | "Osc 2 shape" => k(&[SAW, SAW_TRI, PULSE]),
            // Pulse width drifts slowly.
            "LFO 1 destination" => r(dest::SHAPE, dest::SHAPE),
            "LFO 1 shape" => r(L_TRI, L_TRI),
            "LFO 1 rate" => r(10, by(30, 60, mv)),
            "LFO 1 amount" => r(by(0, 20, mv), by(20, 70, mv)),
            "Pan spread" => r(by(30, 60, sp), by(60, 127, sp)),
            "FX select" => k(&[FX_REVERB, FX_CHORUS, FX_BBD]),
            _ => amp([(60, 110), (40, 100), (90, 127), (60, 110)])
                .or_else(|| filter([(40, 100), (50, 110), (40, 110), (50, 100)])),
        },
        "pluck" => match b {
            // The envelope snaps the filter open and lets it fall.
            "Filter env amount" => r(by(150, 180, mv), by(190, 254, mv)),
            "Filter cutoff" => band((40, 80), 30, br),
            "FX select" => k(&[FX_STEREO_DELAY, FX_BBD, FX_REVERB]),
            _ => amp([(0, 5), (20, 60), (0, 10), (10, 40)])
                .or_else(|| filter([(0, 0), (15, 55), (0, 15), (10, 40)])),
        },
        "bass" => match b {
            "Osc 1 frequency" | "Osc 2 frequency" => k(&[12, 24]),
            "Osc 1 shape" | "Osc 2 shape" => k(&[SAW, PULSE]),
            "Sub osc level" => r(60, 127),
            "LFO 1 amount" | "LFO 2 amount" => r(0, by(0, 15, mv)),
            // Noise shakes the filter: dirt.
            "Mod 2 amount" => r(127, by(127, 200, g)),
            "FX select" => k(&[FX_DISTORTION, FX_CHORUS, FX_BBD]),
            "FX mix" => r(0, by(0, 50, sp)),
            _ => amp([(0, 10), (20, 80), (40, 110), (5, 30)]),
        },
        "lead" => match b {
            "Filter cutoff" => band((70, 130), 34, br),
            // Vibrato, never a siren.
            "LFO 1 destination" => r(dest::FREQ, dest::FREQ),
            "LFO 1 shape" => r(L_TRI, L_TRI),
            "LFO 1 rate" => r(55, 75),
            "LFO 1 amount" => r(0, by(2, 14, mv)),
            "FX select" => k(&[FX_STEREO_DELAY, FX_BBD, FX_MONO_DELAY]),
            _ => amp([(0, 20), (30, 90), (80, 127), (10, 50)]),
        },
        "keys" => match b {
            "Osc 1 shape" | "Osc 2 shape" => k(&[PULSE, SAW_TRI, TRI]),
            "Filter env amount" => r(by(140, 170, mv), by(170, 220, mv)),
            "Filter env velocity" | "Amp env velocity" => r(40, 110),
            "FX select" => k(&[FX_CHORUS, FX_PHASER, FX_BBD]),
            _ => amp([(0, 6), (55, 95), (20, 70), (25, 55)])
                .or_else(|| filter([(0, 0), (40, 80), (0, 40), (25, 55)])),
        },
        "strings" => match b {
            "Osc 1 shape" => r(SAW, SAW),
            "Osc 2 shape" => k(&[SAW, SAW, PULSE]),
            // Two sections a little apart, and never quite steady.
            "Osc 1 fine tune" => r(44, 48),
            "Osc 2 fine tune" => r(52, 57),
            "Osc slop" => r(5, by(10, 40, g)),
            "Sync" => r(0, 0),
            "Filter cutoff" => band((70, 125), 34, br),
            "Resonance" => r(0, by(0, 30, br)),
            // Bowing: the pulse width drifts, the pitch wavers a hair.
            "LFO 1 destination" => r(dest::SHAPE, dest::SHAPE),
            "LFO 1 shape" => r(L_TRI, L_TRI),
            "LFO 1 rate" => r(10, by(25, 50, mv)),
            "LFO 1 amount" => r(by(5, 20, mv), by(20, 60, mv)),
            "LFO 3 destination" => r(dest::FREQ, dest::FREQ),
            "LFO 3 amount" => r(0, 4),
            "LFO 3 rate" => r(40, 60),
            "Pan spread" => r(by(30, 60, sp), by(60, 127, sp)),
            "FX select" => k(&[FX_CHORUS, FX_CHORUS, FX_REVERB]),
            "FX on" => r(1, 1),
            _ => amp([(45, 80), (60, 100), (100, 127), (50, 90)])
                .or_else(|| filter([(40, 90), (60, 110), (60, 110), (50, 90)])),
        },
        "brass" => match b {
            "Osc 1 shape" | "Osc 2 shape" => r(SAW, SAW),
            "Sync" => r(0, 0),
            // The blat: a four-pole filter the envelope opens just after
            // the note starts, harder on a hard note.
            "Filter poles" => r(1, 1),
            "Filter cutoff" => band((40, 70), 25, br),
            "Filter env amount" => r(by(160, 185, mv), by(190, 240, mv)),
            "Filter env velocity" => r(50, 110),
            "LFO 1 destination" => r(dest::FREQ, dest::FREQ),
            "LFO 1 shape" => r(L_TRI, L_TRI),
            "LFO 1 rate" => r(55, 75),
            "LFO 1 amount" => r(0, by(0, 10, mv)),
            "FX select" => k(&[FX_REVERB, FX_BBD, FX_CHORUS]),
            _ => amp([(8, 30), (50, 90), (85, 115), (15, 40)])
                .or_else(|| filter([(12, 40), (40, 80), (40, 80), (20, 45)])),
        },
        "organ" => match b {
            // Drawbars: a square, a stop one or two octaves above it in
            // tune, and the sub oscillator below.
            "Osc 1 shape" => r(PULSE, PULSE),
            "Osc 1 shape mod" => r(45, 55),
            "Osc 2 shape" => k(&[TRI, PULSE]),
            "Osc 1 frequency" => k(&[24, 36]),
            "Osc 2 frequency" => k(&[36, 48, 48]),
            "Osc 1 fine tune" | "Osc 2 fine tune" => r(49, 51),
            "Sub osc level" => r(40, 110),
            "Noise level" | "Sync" | "Filter audio mod" => r(0, 0),
            "Osc slop" => r(0, 3),
            "Filter cutoff" => band((90, 130), 34, br),
            "Resonance" => r(0, by(0, 25, br)),
            "Filter env amount" => r(127, by(127, 150, mv)),
            // No touch response, and gone when the key is.
            "Amp env velocity" | "Filter env velocity" => r(0, 0),
            "Mod 1 amount" | "Mod 2 amount" => r(127, 127),
            // A rotary speaker: fast vibrato and tremolo.
            "LFO 1 destination" => r(dest::FREQ, dest::FREQ),
            "LFO 1 shape" | "LFO 2 shape" => r(L_TRI, L_TRI),
            "LFO 1 rate" | "LFO 2 rate" => r(65, 80),
            "LFO 1 amount" => r(0, by(0, 8, mv)),
            "LFO 2 destination" => r(dest::VCA, dest::VCA),
            "LFO 2 amount" => r(0, by(0, 30, mv)),
            "FX select" => k(&[FX_CHORUS, FX_PHASER, FX_PHASER_LOW, FX_DISTORTION]),
            _ => amp([(0, 2), (60, 60), (127, 127), (3, 12)]),
        },
        "sub-bass" => match b {
            // Triangles an octave or two down, the sub oscillator under
            // them, and nothing that moves.
            "Osc 1 shape" => r(TRI, TRI),
            "Osc 2 shape" => k(&[TRI, PULSE]),
            "Osc 1 frequency" => r(12, 12),
            "Osc 2 frequency" => k(&[12, 24]),
            "Osc 1 fine tune" | "Osc 2 fine tune" => r(49, 51),
            "Osc mix" => r(40, 88),
            "Sub osc level" => r(90, 127),
            "Noise level" | "Sync" | "Osc slop" | "Filter audio mod" => r(0, 0),
            "Filter cutoff" => band((40, 75), 20, br),
            "Resonance" => r(0, by(0, 30, br)),
            "LFO 1 amount" | "LFO 2 amount" => r(0, 0),
            "Mod 1 amount" | "Mod 2 amount" => r(127, 127),
            "Pan spread" => r(0, 0),
            "FX on" => r(0, 0),
            _ => amp([(0, 6), (40, 80), (100, 127), (8, 30)]),
        },
        "sync-lead" => match b {
            // Oscillator 1 is the one synced: tuned above oscillator 2,
            // with the third envelope pulling it through its harmonics.
            "Sync" => r(1, 1),
            "Osc 1 frequency" => k(&[31, 36, 43, 48]),
            "Osc 2 frequency" => r(24, 24),
            "Osc 1 shape" | "Osc 2 shape" => k(&[SAW, SAW, PULSE]),
            "Osc mix" => r(40, 88),
            "Env 3 destination" => r(dest::OSC_1, dest::OSC_1),
            "Env 3 amount" => r(by(150, 180, mv), by(190, 254, mv)),
            "Filter cutoff" => band((75, 130), 34, br),
            "LFO 1 destination" => r(dest::FREQ, dest::FREQ),
            "LFO 1 shape" => r(L_TRI, L_TRI),
            "LFO 1 rate" => r(55, 75),
            "LFO 1 amount" => r(0, by(2, 14, mv)),
            "FX select" => k(&[FX_STEREO_DELAY, FX_BBD, FX_FLANGER]),
            _ => amp([(0, 10), (50, 90), (100, 127), (15, 45)])
                .or_else(|| env3([(0, 20), (45, 100), (0, 60), (30, 60)])),
        },
        "bell" => match b {
            // Metal: triangles a fifth, a seventh or an eleventh apart,
            // oscillator 1 shaking the filter, or the ring modulator.
            "Osc 1 shape" | "Osc 2 shape" => r(TRI, TRI),
            "Osc 1 frequency" => k(&[36, 48]),
            "Osc 2 frequency" => k(&[43, 53, 55, 58]),
            "Sync" => r(0, 0),
            "Filter audio mod" => r(by(40, 60, g), by(70, 120, g)),
            "Resonance" => r(by(30, 50, br), by(60, 100, br)),
            "Filter poles" => r(0, 0),
            "Filter key amount" => r(100, 127),
            "Filter cutoff" => band((70, 120), 34, br),
            "FX select" => k(&[FX_RING_MOD, FX_REVERB, FX_CHORUS]),
            _ => amp([(0, 0), (80, 115), (0, 20), (70, 105)])
                .or_else(|| filter([(0, 3), (70, 110), (0, 30), (70, 105)])),
        },
        "kick" => match b {
            // Two triangles in tune and the sub oscillator, low; the keys
            // do not move the filter.
            "Osc 1 shape" | "Osc 2 shape" => r(TRI, TRI),
            "Osc 1 frequency" | "Osc 2 frequency" => r(12, 12),
            "Osc 1 fine tune" | "Osc 2 fine tune" => r(50, 50),
            "Sub osc level" => r(60, 127),
            "Noise level" => r(0, by(0, 30, g)),
            "Sync" => r(0, 0),
            "Filter key amount" => r(0, 0),
            "Filter cutoff" => band((40, 90), 25, br),
            "Resonance" => r(0, by(0, 30, br)),
            "Filter env amount" => r(127, by(127, 180, mv)),
            // The thump: the third envelope drops the pitch into the note.
            "Env 3 destination" => r(dest::FREQ, dest::FREQ),
            "Env 3 amount" => r(by(170, 190, mv), by(200, 240, mv)),
            "LFO 1 amount" | "LFO 2 amount" => r(0, 0),
            "Mod 1 amount" => r(127, 127),
            "FX select" => k(&[FX_DISTORTION, FX_REVERB]),
            "FX mix" => r(0, by(0, 50, sp)),
            _ => amp([(0, 0), (40, 70), (0, 0), (25, 60)])
                .or_else(|| filter([(0, 0), (20, 50), (0, 0), (20, 50)]))
                .or_else(|| env3([(0, 0), (15, 40), (0, 0), (15, 40)])),
        },
        "snare" => match b {
            // The shell: two oscillators out of tune with each other, as
            // a drum's modes are. The wires: noise.
            "Osc 1 shape" => k(&[TRI, PULSE]),
            "Osc 2 shape" => r(TRI, TRI),
            "Osc 1 frequency" => r(36, 36),
            "Osc 2 frequency" => k(&[41, 43, 46]),
            "Noise level" => r(90, 127),
            "Sub osc level" | "Sync" => r(0, 0),
            "Filter key amount" => r(0, 0),
            "Filter poles" => r(0, 0),
            "Filter cutoff" => band((70, 120), 30, br),
            "Env 3 destination" => r(dest::FREQ, dest::FREQ),
            "Env 3 amount" => r(by(150, 160, mv), by(165, 190, mv)),
            "LFO 1 amount" | "LFO 2 amount" => r(0, 0),
            "FX select" => k(&[FX_REVERB, FX_DISTORTION]),
            _ => amp([(0, 0), (40, 65), (0, 0), (20, 50)])
                .or_else(|| env3([(0, 0), (10, 30), (0, 0), (10, 30)])),
        },
        "laser" => match b {
            "Osc 1 shape" | "Osc 2 shape" => k(&[SAW, PULSE]),
            "Resonance" => r(by(30, 50, br), by(60, 100, br)),
            // The zap: the third envelope drops the pitch a long way.
            "Env 3 destination" => r(dest::FREQ, dest::FREQ),
            "Env 3 amount" => r(by(200, 225, mv), by(230, 254, mv)),
            // A fast LFO makes the beam shimmer.
            "LFO 1 destination" => r(dest::CUTOFF, dest::CUTOFF),
            "LFO 1 shape" => k(&[L_SQUARE, L_SAW, L_RANDOM]),
            "LFO 1 rate" => r(70, 89),
            "LFO 1 amount" => r(0, by(0, 60, mv)),
            "FX select" => k(&[FX_STEREO_DELAY, FX_BBD, FX_PHASE_MOD, FX_FLANGER_2]),
            _ => amp([(0, 0), (45, 90), (0, 0), (40, 85)])
                .or_else(|| env3([(0, 0), (30, 80), (0, 0), (30, 80)])),
        },
        "wind" => match b {
            // Noise alone.
            "Osc 1 shape" | "Osc 2 shape" => r(OSC_OFF, OSC_OFF),
            "Noise level" => r(110, 127),
            "Sub osc level" => r(0, 0),
            // The whistle: resonance sings where the cutoff sits, and the
            // keys move it.
            "Resonance" => r(by(50, 70, br), by(80, 110, br)),
            "Filter cutoff" => band((45, 100), 25, br),
            "Filter key amount" => r(127, 127),
            "Filter poles" => r(1, 1),
            // Gusts.
            "LFO 1 destination" => r(dest::CUTOFF, dest::CUTOFF),
            "LFO 1 shape" => k(&[L_TRI, L_TRI, L_RANDOM]),
            "LFO 1 rate" => r(5, by(20, 45, mv)),
            "LFO 1 amount" => r(by(15, 30, mv), by(40, 90, mv)),
            "Mod 1 amount" | "Mod 2 amount" => r(127, 127),
            "FX select" => k(&[FX_REVERB, FX_BBD]),
            _ => amp([(50, 100), (60, 60), (100, 127), (70, 115)])
                .or_else(|| filter([(50, 100), (60, 110), (40, 110), (70, 115)])),
        },
        "siren" => match b {
            "Osc 1 shape" | "Osc 2 shape" => k(&[SAW, PULSE]),
            "Sync" => r(0, 0),
            "Filter cutoff" => band((80, 130), 34, br),
            // The wail: a slow LFO carries the pitch up and down,
            // restarting with each key.
            "LFO 1 destination" => r(dest::FREQ, dest::FREQ),
            "LFO 1 shape" => k(&[L_TRI, L_TRI, L_SAW]),
            "LFO 1 rate" => r(by(8, 25, mv), by(25, 50, mv)),
            "LFO 1 amount" => r(by(30, 50, mv), by(60, 110, mv)),
            "LFO 1 key sync" => r(1, 1),
            "FX select" => k(&[FX_STEREO_DELAY, FX_REVERB, FX_PHASER]),
            _ => amp([(0, 10), (60, 60), (115, 127), (15, 60)]),
        },
        _ => None,
    }
}

/// What any character draws layer A field `b` from unless it says
/// otherwise ([`special`]).
fn common(b: &str, f: &Field, m: &Macros) -> Draw {
    let stage = b.rsplit(' ').next().unwrap_or("");
    let adsr = |a: [(u16, u16); 4]| match stage {
        "attack" => a[0],
        "decay" => a[1],
        "sustain" => a[2],
        "release" => a[3],
        _ => (f.min, f.max),
    };
    let (lo, hi) = match b {
        _ if b.ends_with(" frequency") => return Pick(&[24, 36, 48]),
        _ if b.ends_with(" fine tune") => (44, 56),
        _ if b.starts_with("Osc ") && b.ends_with(" shape") => (SAW, PULSE),
        // A pulse neither too thin nor too wide to hear.
        _ if b.ends_with(" shape mod") => (15, 85),
        "Sub osc level" => (0, 60),
        "Noise level" => (0, by(0, 40, m.grit)),
        "Sync" => (0, u16::from(m.grit > 0.7)),
        "Osc slop" => (0, by(0, 40, m.grit)),
        "Filter cutoff" => {
            let lo = by(40, 120, m.bright);
            (lo, (lo + 44).min(164))
        }
        "Resonance" => (0, by(0, 100, m.bright)),
        "Filter audio mod" => (0, by(0, 60, m.grit)),
        "Filter env amount" => (127, by(127, 254, m.movement)),
        "Amp env amount" => (90, 127),
        _ if b.ends_with(" delay") => (0, 0),
        _ if b.starts_with("Amp env ") && stage != "velocity" => {
            adsr([(0, 20), (30, 90), (60, 127), (10, 60)])
        }
        _ if b.starts_with("Filter env ") && stage != "velocity" => {
            adsr([(0, 60), (20, 100), (0, 127), (10, 90)])
        }
        // The third envelope rests unless a character sends it somewhere.
        "Env 3 amount" => (127, 127),
        "Env 3 destination" => (dest::OFF, dest::OFF),
        "Env 3 velocity" => (0, 0),
        _ if b.starts_with("Env 3 ") => adsr([(0, 20), (20, 90), (0, 0), (20, 90)]),
        // LFO 1 moves the tone, LFO 2 the stereo picture; 3 and 4 rest.
        "LFO 1 destination" => return Pick(&[dest::CUTOFF, dest::SHAPE]),
        "LFO 1 shape" => return Pick(&[L_TRI, L_TRI, L_SAW, 3, L_SQUARE]),
        "LFO 1 rate" => (15, by(50, 89, m.movement)),
        "LFO 1 amount" => (0, by(0, 40, m.movement)),
        "LFO 2 destination" => (dest::PAN, dest::PAN),
        "LFO 2 rate" => (10, 60),
        "LFO 2 amount" => (0, by(0, 60, m.movement)),
        _ if b.starts_with("LFO ") && b.ends_with(" destination") => (dest::OFF, dest::OFF),
        _ if b.starts_with("LFO ") && b.ends_with(" amount") => (0, 0),
        _ if b.starts_with("LFO ") && b.ends_with(" shape") => (L_TRI, L_TRI),
        _ if b.starts_with("LFO ") && b.ends_with(" rate") => (20, 60),
        _ if b.starts_with("LFO ") && b.ends_with(" key sync") => (0, 0),
        // Slot 1: the filter envelope sweeps the pulse width with each
        // note. Slot 2: noise shakes the filter.
        "Mod 1 source" => (source::FILTER_ENV, source::FILTER_ENV),
        "Mod 1 destination" => (dest::SHAPE, dest::SHAPE),
        "Mod 1 amount" => (127, by(127, 190, m.movement)),
        "Mod 2 source" => (source::NOISE, source::NOISE),
        "Mod 2 destination" => (dest::CUTOFF, dest::CUTOFF),
        "Mod 2 amount" => (127, by(127, 170, m.grit)),
        "Pan spread" => (0, by(20, 100, m.space)),
        "FX select" => {
            return Pick(&[FX_STEREO_DELAY, FX_BBD, FX_CHORUS, FX_PHASER, FX_REVERB]);
        }
        "FX on" => {
            let on = u16::from(m.space > 0.2);
            (on, on)
        }
        "FX mix" => (0, by(0, 100, m.space)),
        _ => (f.min, f.max),
    };
    Range(lo, hi)
}

/// Whether something sounds: both oscillators on, one on with the mix
/// near the middle (which way the mix runs is not documented), or noise.
fn audible(p: &Program) -> bool {
    let get = |name: &str| p.get(&fields::all()[index(name)]);
    let (on1, on2) = (get("A Osc 1 shape") != 0, get("A Osc 2 shape") != 0);
    (on1 && on2)
        || ((on1 || on2) && (30..=98).contains(&get("A Osc mix")))
        || get("A Noise level") >= 50
}

/// Make the program sound (see the module docs). Touches only free
/// fields; idempotent.
fn cohere(p: &mut Program) {
    let f = |name: &str| &fields::all()[index(name)];
    let clamp = |p: &mut Program, name: &str, lo: u16, hi: u16| {
        let v = p.get(f(name)).clamp(lo, hi);
        p.set(f(name), v).expect("in range");
    };
    if !audible(p) {
        clamp(p, "A Osc 1 shape", SAW, PULSE);
        clamp(p, "A Osc 2 shape", SAW, PULSE);
    }
    clamp(p, "A Filter cutoff", 40, 164);
    clamp(p, "A Amp env amount", 90, 127);
    clamp(p, "A Resonance", 0, 110);
    clamp(p, "A FX mix", 0, 100);
    if p.get(f("A Amp env sustain")) < 20 {
        clamp(p, "A Amp env decay", 40, 127);
    }
}

/// `bytes` after the coherence pass.
pub(crate) fn cohered(bytes: &[u8]) -> Result<Vec<u8>, DriverError> {
    let mut p = program(bytes)?;
    cohere(&mut p);
    Ok(p.as_bytes().to_vec())
}

fn program(bytes: &[u8]) -> Result<Program, DriverError> {
    Program::from_bytes(bytes)
}

/// The free fields, with their indices.
fn free() -> impl Iterator<Item = (usize, &'static Field)> {
    fields::all()
        .iter()
        .enumerate()
        .filter(|(_, f)| mutability(f) == Mutability::Free)
}

pub(crate) fn generate(recipe: &Recipe, base: Option<&[u8]>) -> Result<Vec<u8>, DriverError> {
    let c = CHARACTERS
        .get(recipe.character)
        .ok_or_else(|| DriverError(format!("no character {}", recipe.character)))?;
    let seed = recipe.seed ^ (recipe.character as u64).wrapping_mul(0xA24B_AED4_963E_E407);
    // Non-free fields come from the base as they are, in range or not.
    let mut p = base.map_or_else(|| Ok(Program::init()), program)?;
    for (_, f) in free() {
        let b = &f.name[2..];
        let draw = special(c.id, b, &recipe.macros).unwrap_or_else(|| common(b, f, &recipe.macros));
        let roll = roll_field(seed, f, 0);
        let v = match draw {
            Range(lo, hi) => lo + ((roll * f32::from(hi - lo + 1)) as u16).min(hi - lo),
            Pick(values) => values[((roll * values.len() as f32) as usize).min(values.len() - 1)],
        };
        p.set(f, v)?;
    }
    cohere(&mut p);
    p.set_name(&name(c, recipe.seed))?;
    let mut out = p.as_bytes().to_vec();
    // Kept pages come back byte for byte, never through `set`.
    if let Some(base) = base {
        for &page in &recipe.keep {
            let page = CURATION
                .pages
                .get(page)
                .ok_or_else(|| DriverError(format!("no page {page}")))?;
            for &i in page.sections.iter().flat_map(|s| &s.fields) {
                let at = usize::from(fields::all()[i].offset);
                out[at] = base[at];
            }
        }
    }
    Ok(out)
}

/// `v` clamped into `f`'s range.
fn clamp(f: &Field, v: f32) -> u16 {
    v.round().clamp(f32::from(f.min), f32::from(f.max)) as u16
}

/// Free amounts move by up to `amount` of half their range; choices,
/// switches, destinations and sources change with probability
/// `amount / 5` (a destination or source to one of the known ones).
pub(crate) fn vary(bytes: &[u8], amount: f32, seed: u64) -> Result<Vec<u8>, DriverError> {
    let amount = amount.clamp(0.0, 1.0);
    let mut p = program(bytes)?;
    for (i, f) in free() {
        let (span, now) = (f32::from(f.max - f.min), f32::from(p.get(f)));
        let r = roll_field(seed, f, 1);
        let v = if dial(i) {
            now + (r * 2.0 - 1.0) * amount * span * 0.5
        } else if r >= amount / 5.0 {
            now
        } else if let Some(known) = named(&f.name[2..]) {
            let k = (roll_field(seed, f, 2) * known.len() as f32) as usize;
            f32::from(known[k.min(known.len() - 1)])
        } else {
            f32::from(f.min) + (roll_field(seed, f, 2) * (span + 1.0)).floor()
        };
        p.set(f, clamp(f, v))?;
    }
    cohere(&mut p);
    Ok(p.as_bytes().to_vec())
}

/// Continuous free fields interpolate by `mix`; choices and switches come
/// from `b` with probability `mix`; either way the value is clamped into
/// range (a parent may hold an out-of-range byte). Everything else, and
/// the name, is `a`'s, as it is.
pub(crate) fn breed(a: &[u8], b: &[u8], mix: f32, seed: u64) -> Result<Vec<u8>, DriverError> {
    let mix = mix.clamp(0.0, 1.0);
    let (mut p, b) = (program(a)?, program(b)?);
    for (i, f) in free() {
        let (x, y) = (f32::from(p.get(f)), f32::from(b.get(f)));
        let v = if dial(i) {
            x + (y - x) * mix
        } else if roll_field(seed, f, 3) < mix {
            y
        } else {
            x
        };
        p.set(f, clamp(f, v))?;
    }
    cohere(&mut p);
    Ok(p.as_bytes().to_vec())
}

#[cfg(test)]
mod tests {
    use crate::DRIVER;
    use oscyl_driver::{Driver, Mutability, Recipe};

    fn idx(name: &str) -> usize {
        DRIVER.fields().iter().position(|f| f.name == name).unwrap()
    }

    fn recipe(character: usize, seed: u64) -> Recipe {
        Recipe {
            character,
            seed,
            ..Recipe::default()
        }
    }

    /// The quality gate. The Rev2 has no program level the generator
    /// sets: its cap is the resonance and the effect mix. A hand's sounds
    /// differ in at least 16 fields (the closest pair in any hand differs
    /// in 22): two draws of one recipe share its fixed choices but not a
    /// dozen of its dials, and a clone or a near-clone fails.
    #[test]
    fn quality_gate() {
        let get = |p: &[u8], n: &str| DRIVER.get(p, idx(n)).unwrap();
        oscyl_conformance::generated_sounds_are_good(
            &DRIVER,
            &oscyl_conformance::Quality {
                audible: &|b| {
                    super::audible(&crate::Program::from_bytes(b).unwrap())
                        && get(b, "A Amp env amount") >= 90
                        && (get(b, "A Amp env sustain") >= 20 || get(b, "A Amp env decay") >= 40)
                },
                level_capped: &|b| get(b, "A Resonance") <= 110 && get(b, "A FX mix") <= 100,
                min_spread: 16,
            },
        );
    }

    #[test]
    fn same_seed_same_bytes() {
        let a = DRIVER.generate(&recipe(0, 42), None).unwrap();
        assert_eq!(DRIVER.generate(&recipe(0, 42), None).unwrap(), a);
        assert_ne!(DRIVER.generate(&recipe(0, 43), None).unwrap(), a);
        assert_eq!(DRIVER.vary(&a, 0.5, 1), DRIVER.vary(&a, 0.5, 1));
        assert_ne!(DRIVER.vary(&a, 0.5, 1).unwrap(), a);
    }

    #[test]
    fn generated_programs_are_audible() {
        let get = |p: &[u8], n: &str| DRIVER.get(p, idx(n)).unwrap();
        assert!(DRIVER.characters().len() >= 10);
        for c in 0..4 {
            for seed in 0..16 {
                let p = DRIVER.generate(&recipe(c, seed), None).unwrap();
                assert_ne!(get(&p, "A Osc 1 shape"), 0);
                assert_ne!(get(&p, "A Osc 2 shape"), 0);
                assert!(get(&p, "A Filter cutoff") >= 40);
                assert!(get(&p, "A Amp env amount") >= 90);
                assert!(get(&p, "A Amp env sustain") >= 20 || get(&p, "A Amp env decay") >= 40);
                assert!(get(&p, "A Resonance") <= 110);
                assert!(!DRIVER.name(&p).unwrap().is_empty());
            }
        }
    }

    #[test]
    fn out_of_range_bytes_survive_keep_and_breed() {
        let off = |name: &str| usize::from(crate::fields::all()[idx(name)].offset);
        let mut base = DRIVER.init_program();
        base[off("A Filter cutoff")] = 200; // past 164; check_program allows it
        base[off("A BPM")] = 10; // below 30, restricted
        assert!(DRIVER.check_program(&base).is_ok());
        let r = Recipe {
            keep: vec![1], // Filter and amp
            ..recipe(0, 9)
        };
        let p = DRIVER.generate(&r, Some(&base)).unwrap();
        assert_eq!(p[off("A Filter cutoff")], 200);
        assert_eq!(p[off("A BPM")], 10);
        let p = DRIVER.generate(&recipe(0, 9), Some(&base)).unwrap();
        assert!(p[off("A Filter cutoff")] <= 164);
        assert_eq!(p[off("A BPM")], 10);
        let child = DRIVER.breed(&base, &DRIVER.init_program(), 0.5, 2).unwrap();
        assert!(child[off("A Filter cutoff")] <= 164);
        assert_eq!(child[off("A BPM")], 10);
        let v = DRIVER.vary(&base, 0.5, 2).unwrap();
        assert_eq!(v[off("A BPM")], 10);
    }

    #[test]
    fn characters_name_their_programs_and_macros() {
        for (c, ch) in DRIVER.characters().iter().enumerate() {
            assert_eq!(ch.signature_page, Some(2));
            let p = DRIVER.generate(&recipe(c, 0x1234), None).unwrap();
            assert_eq!(DRIVER.name(&p).unwrap(), DRIVER.generated_name(c, 0x1234));
        }
        assert_eq!(DRIVER.generated_name(0, 0xABCD), "Pad ABCD");
        let m = DRIVER.macro_info();
        assert_eq!(m[0].help, "Filter cutoff and resonance");
        assert_eq!(m[3].help, "Effect, its mix and the stereo spread");
    }

    #[test]
    fn fx_mix_is_capped_by_coherence() {
        let mut p = DRIVER.init_program();
        DRIVER.set(&mut p, idx("A FX mix"), 127).unwrap();
        let v = DRIVER.vary(&p, 0.0, 1).unwrap();
        assert_eq!(DRIVER.get(&v, idx("A FX mix")), Ok(100));
    }

    #[test]
    fn restricted_fields_stay_at_init() {
        let init = DRIVER.init_program();
        let p = DRIVER.generate(&recipe(2, 5), None).unwrap();
        assert_eq!(DRIVER.mutability(idx("A Filter cutoff")), Mutability::Free);
        for name in ["B Filter cutoff", "A Program volume", "A/B mode", "A BPM"] {
            assert_eq!(DRIVER.mutability(idx(name)), Mutability::Restricted);
            assert_eq!(DRIVER.get(&p, idx(name)), DRIVER.get(&init, idx(name)));
        }
    }
}
