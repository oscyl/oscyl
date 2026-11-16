//! A deterministic generator for the prologue.
//!
//! It sets the main timbre's sound (oscillators, multi engine as noise or
//! VPM, mixer, filter, both EGs, LFO) and the effects. Everything else
//! keeps the init (or, for vary and breed, the parent) value: the program
//! and timbre settings, the arpeggiator, the controllers, the voice mode
//! and the whole sub timbre (left off, so a generated program is one
//! timbre). The user oscillator's parameter types are fixed: the loaded
//! unit sets them. Each field draws from its own random stream (seed and
//! field index), so a macro changes only arithmetic, never which numbers a
//! field gets.
//!
//! Coherence: a VCO or the multi engine is audible, the filter is open at
//! least a fifth of the way, resonance stays under 800, a note either
//! sustains or decays slowly enough to hear, the multi engine is noise or
//! VPM (a user oscillator may not be loaded), and an effect that is on has
//! a type that is not a user slot.
//!
//! Each character says what it needs of a field ([`special`]); the rest
//! comes from ranges all share ([`common`]).

use oscyl_driver::{
    Character, Control, DriverError, Family, MacroInfo, Macros, Mutability, Recipe,
};

use crate::Program;
use crate::curate::{CURATION, ENVELOPES, FILTER, OSCILLATORS, control};
use crate::fields::{Field, all, field};
use oscyl_logue::kinds::VPM_SAFE;

pub const CHARACTERS: &[Character] = &[
    Character {
        id: "pad",
        title: "Pad",
        hint: "Slow swell, long release",
        family: Family::Pad,
        signature_page: Some(ENVELOPES),
    },
    Character {
        id: "pluck",
        title: "Pluck",
        hint: "Short and percussive",
        family: Family::Keys,
        signature_page: Some(ENVELOPES),
    },
    Character {
        id: "bass",
        title: "Bass",
        hint: "Low and tight",
        family: Family::Bass,
        signature_page: Some(OSCILLATORS),
    },
    Character {
        id: "lead",
        title: "Lead",
        hint: "Bright, sustained solo voice",
        family: Family::Lead,
        signature_page: Some(OSCILLATORS),
    },
    Character {
        id: "keys",
        title: "Keys",
        hint: "Decaying, playable chords",
        family: Family::Keys,
        signature_page: Some(ENVELOPES),
    },
    Character {
        id: "strings",
        title: "Strings",
        hint: "Bowed ensemble",
        family: Family::Pad,
        signature_page: Some(ENVELOPES),
    },
    Character {
        id: "brass",
        title: "Brass",
        hint: "Bold swell with a filter blat",
        family: Family::Lead,
        signature_page: Some(FILTER),
    },
    Character {
        id: "organ",
        title: "Organ",
        hint: "Drawbars, straight on and off",
        family: Family::Keys,
        signature_page: Some(OSCILLATORS),
    },
    Character {
        id: "sub-bass",
        title: "Sub Bass",
        hint: "Clean weight below",
        family: Family::Bass,
        signature_page: Some(OSCILLATORS),
    },
    Character {
        id: "acid",
        title: "Acid",
        hint: "Squelching line",
        family: Family::Bass,
        signature_page: Some(FILTER),
    },
    Character {
        id: "sync-lead",
        title: "Sync Lead",
        hint: "Hard sync sweep",
        family: Family::Lead,
        signature_page: Some(OSCILLATORS),
    },
    Character {
        id: "fm-keys",
        title: "FM Keys",
        hint: "Glassy digital keys",
        family: Family::Keys,
        signature_page: Some(OSCILLATORS),
    },
    Character {
        id: "bell",
        title: "Bell",
        hint: "Metallic ring, long decay",
        family: Family::Keys,
        signature_page: Some(ENVELOPES),
    },
    Character {
        id: "kick",
        title: "Kick",
        hint: "Bass drum: a pitch drop",
        family: Family::Rhythm,
        signature_page: Some(ENVELOPES),
    },
    Character {
        id: "snare",
        title: "Snare",
        hint: "Noise crack with a body",
        family: Family::Rhythm,
        signature_page: Some(ENVELOPES),
    },
    Character {
        id: "laser",
        title: "Laser",
        hint: "A fast zap",
        family: Family::Effect,
        signature_page: Some(ENVELOPES),
    },
    Character {
        id: "wind",
        title: "Wind",
        hint: "Whistling air the keys play",
        family: Family::Effect,
        signature_page: Some(FILTER),
    },
    Character {
        id: "siren",
        title: "Siren",
        hint: "Rising and falling wail",
        family: Family::Effect,
        signature_page: Some(OSCILLATORS),
    },
];

pub const MACRO_INFO: [MacroInfo; 4] = [
    MacroInfo {
        title: "Bright",
        help: "Cutoff and resonance",
    },
    MacroInfo {
        title: "Movement",
        help: "LFO, filter EG amount and pitch EG",
    },
    MacroInfo {
        title: "Grit",
        help: "Drive, cross mod, ring and sync, multi engine level",
    },
    MacroInfo {
        title: "Ambience",
        help: "Modulation effect, delay and reverb",
    },
];

/// A generated program's name: the character (its first word when the
/// title is long) and four hex digits of the seed, within 12 characters.
pub fn name(c: &Character, seed: u64) -> String {
    let title = if c.title.len() > 7 {
        c.title.split(' ').next().unwrap_or(c.title)
    } else {
        c.title
    };
    format!("{title} {:04X}", seed & 0xFFFF)
}

pub fn mutability(f: &Field) -> Mutability {
    let param_type =
        f.section == "Multi Engine" && f.label.ends_with(" Type") && f.label.starts_with("Param");
    if param_type {
        return Mutability::Fixed;
    }
    let free = match (f.timbre, f.section, f.label) {
        // Routing is the player's; dry/wet is a 2.00 field (keeps "not
        // set"); the subtype bytes 46-49 are unverified (Ruling 6).
        (None, "Mod FX" | "Delay/Reverb", l) => !matches!(
            l,
            "Routing" | "User" | "Dry/Wet" | "Chorus" | "Ensemble" | "Phaser" | "Flanger"
        ),
        (
            Some(0),
            "VCO 1" | "VCO 2" | "Pitch EG" | "Mixer" | "Filter" | "Amp EG" | "EG" | "LFO",
            _,
        ) => true,
        (Some(0), "Multi Engine", l) => !(l.starts_with("User") || l.starts_with("Param")),
        _ => false,
    };
    if free {
        Mutability::Free
    } else {
        Mutability::Restricted
    }
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

fn roll_field(seed: u64, i: usize, k: u64) -> f32 {
    roll(seed, ((i as u64) << 8) | k)
}

/// `lo..=hi` scaled by `t` from `lo`.
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

/// An envelope's `(attack, decay, sustain, release)` ranges, by stage label.
fn adsr(label: &str, a: [(u16, u16); 4]) -> Option<Draw> {
    let (lo, hi) = match label {
        "Attack" => a[0],
        "Decay" => a[1],
        "Sustain" => a[2],
        "Release" => a[3],
        _ => return None,
    };
    r(lo, hi)
}

/// `lo` rising with `t`, and `width` above it.
fn band(lo: (u16, u16), width: u16, t: f32) -> Option<Draw> {
    let lo = by(lo.0, lo.1, t);
    r(lo, (lo + width).min(1023))
}

// Raw values (KORG's MIDI implementation).
const SQR: u16 = 0;
const TRI: u16 = 1;
const SAW: u16 = 2;
const FT16: u16 = 0;
const FT8: u16 = 1;
const FT4: u16 = 2;
const FT2: u16 = 3;
const RING: u16 = 0;
const OFF: u16 = 1;
const SYNC: u16 = 2;
const NOISE: u16 = 0;
const VPM: u16 = 1;
const N_HIGH: u16 = 0;
const N_LOW: u16 = 1;
const N_PEAK: u16 = 2;
const N_DECIM: u16 = 3;
const TO_CUTOFF: u16 = 0;
const TO_SHAPE: u16 = 1;
const TO_PITCH: u16 = 2;
const LFO_SLOW: u16 = 1;
const LFO_FAST: u16 = 2;
const CHORUS: u16 = 1;
const ENSEMBLE: u16 = 2;
const PHASER: u16 = 3;
const DELAY: u16 = 1;
const REVERB: u16 = 2;
/// The pitch EG on both analogue oscillators.
const PITCH_BOTH: u16 = 1;
const PITCH_VCO2: u16 = 2;

/// What a character's draw can read: the macros, and the fields drawn
/// before this one (the table's order: a type before its shape).
struct Ctx<'a> {
    m: &'a Macros,
    p: &'a Program,
}

/// What character `id` wants of field (`s`, `l`: section and label), or
/// `None` for [`common`]'s range. Envelope times, tunings and levels are
/// reasoned from the panel's ranges, not measured on hardware.
fn special(id: &str, c: &Ctx, s: &str, l: &str) -> Option<Draw> {
    let Macros {
        bright: b,
        movement: mv,
        grit: g,
        space: sp,
    } = *c.m;
    let ringing = || c.p.get(field("Main VCO 2 Ring/Sync")) == RING;
    match id {
        "pad" => match (s, l) {
            ("Amp EG", _) => adsr(l, [(300, 700), (300, 800), (700, 1023), (500, 900)]),
            // The filter swells behind the amp.
            ("EG", _) => adsr(l, [(300, 800), (400, 900), (300, 900), (400, 900)]),
            ("LFO", "Mode") => r(LFO_SLOW, LFO_SLOW),
            ("LFO", "Rate") => r(100, by(250, 600, mv)),
            ("Mod FX", "On") => r(1, 1),
            ("Mod FX", "Type") => k(&[CHORUS, ENSEMBLE]),
            ("Delay/Reverb", "Type") => r(REVERB, REVERB),
            _ => None,
        },
        "pluck" => match (s, l) {
            ("Amp EG", _) => adsr(l, [(0, 30), (200, 500), (0, 100), (100, 400)]),
            // The EG snaps the filter open and lets it fall.
            ("EG", _) => adsr(l, [(0, 0), (150, 400), (0, 100), (100, 400)]),
            ("Filter", "EG Int") => r(by(560, 700, mv), by(650, 950, mv)),
            ("Filter", "Cutoff") => band((200, 560), 160, b),
            ("LFO", "Int") => r(0, by(0, 150, mv)),
            _ => None,
        },
        "bass" => match (s, l) {
            ("Amp EG", _) => adsr(l, [(0, 40), (200, 600), (400, 900), (50, 250)]),
            ("VCO 1" | "VCO 2" | "Multi Engine", "Octave") => r(FT16, FT8),
            ("VCO 1" | "VCO 2", "Wave") => k(&[SAW, SQR]),
            ("Mixer", "VCO 2") => r(400, 900),
            ("LFO", "Int") => r(0, by(0, 200, mv)),
            ("Mod FX", "On") => r(0, 0),
            ("Delay/Reverb", "Depth") => r(0, by(0, 250, sp)),
            _ => None,
        },
        "lead" => match (s, l) {
            ("Amp EG", _) => adsr(l, [(0, 100), (300, 700), (700, 1023), (100, 400)]),
            ("VCO 1" | "VCO 2", "Wave") => k(&[SAW, SQR]),
            ("Filter", "Cutoff") => band((450, 850), 170, b),
            // Vibrato, never a siren.
            ("LFO", "Target") => r(TO_PITCH, TO_PITCH),
            ("LFO", "Wave") => r(TRI, TRI),
            ("LFO", "Int") => r(by(0, 20, mv), by(20, 140, mv)),
            ("LFO", "Rate") => r(500, 800),
            ("Delay/Reverb", "Type") => r(DELAY, DELAY),
            _ => None,
        },
        "keys" => match (s, l) {
            ("Amp EG", _) => adsr(l, [(0, 60), (400, 800), (200, 600), (200, 500)]),
            ("EG", _) => adsr(l, [(0, 0), (300, 600), (0, 300), (200, 500)]),
            ("Filter", "Velocity") => r(40, 110),
            ("Filter", "Key Track") => r(1, 2),
            ("LFO", "Int") => r(0, by(0, 150, mv)),
            ("Mod FX", "Type") => k(&[CHORUS, PHASER, ENSEMBLE]),
            _ => None,
        },
        "strings" => match (s, l) {
            ("VCO 1", "Wave") => r(SAW, SAW),
            ("VCO 2", "Wave") => k(&[SAW, SAW, SQR]),
            // Two sections a little apart.
            ("VCO 2", "Pitch") => r(536, 566),
            ("VCO 2", "Ring/Sync") => r(OFF, OFF),
            ("Mixer", "VCO 2") => r(600, 950),
            ("Amp EG", _) => adsr(l, [(280, 560), (500, 800), (800, 1000), (380, 760)]),
            ("EG", _) => adsr(l, [(300, 700), (500, 900), (300, 800), (380, 760)]),
            ("Filter", "Cutoff") => band((380, 820), 160, b),
            ("Filter", "Resonance") => r(0, by(0, 200, b)),
            // Bowing: the shapes drift slowly, and the ensemble is always on.
            ("LFO", "Target") => r(TO_SHAPE, TO_SHAPE),
            ("LFO", "Wave") => r(TRI, TRI),
            ("LFO", "Mode") => r(LFO_SLOW, LFO_SLOW),
            ("LFO", "Rate") => r(80, by(200, 380, mv)),
            ("LFO", "Int") => r(by(120, 250, mv), by(250, 520, mv)),
            ("Mod FX", "On") => r(1, 1),
            ("Mod FX", "Type") => k(&[ENSEMBLE, ENSEMBLE, CHORUS]),
            ("Delay/Reverb", "Type") => r(REVERB, REVERB),
            ("Delay/Reverb", "Reverb Type") => k(&[0, 1, 2]),
            _ => None,
        },
        "brass" => match (s, l) {
            ("VCO 1" | "VCO 2", "Wave") => r(SAW, SAW),
            ("VCO 2", "Pitch") => r(532, 548),
            ("VCO 2", "Ring/Sync") => r(OFF, OFF),
            ("Mixer", "VCO 2") => r(600, 950),
            ("Amp EG", _) => adsr(l, [(20, 110), (450, 750), (700, 900), (120, 320)]),
            // The blat: the filter opens just after the note starts.
            ("EG", _) => adsr(l, [(60, 220), (350, 650), (300, 600), (150, 350)]),
            ("Filter", "EG Int") => r(by(620, 700, mv), by(720, 900, mv)),
            ("Filter", "Cutoff") => band((200, 560), 120, b),
            ("Filter", "Velocity") => r(50, 110),
            ("LFO", "Target") => r(TO_PITCH, TO_PITCH),
            ("LFO", "Wave") => r(TRI, TRI),
            ("LFO", "Int") => r(0, by(0, 120, mv)),
            ("Mod FX", "Type") => k(&[ENSEMBLE, CHORUS]),
            ("Delay/Reverb", "Type") => r(REVERB, REVERB),
            _ => None,
        },
        "organ" => match (s, l) {
            // Drawbars: a fundamental, a stop one or two octaves above it
            // in tune, and a sine from the VPM engine below or between.
            ("VCO 1", "Wave") => k(&[SQR, TRI]),
            ("VCO 1", "Octave") => r(FT8, FT8),
            ("VCO 2", "Wave") => k(&[TRI, SQR]),
            ("VCO 2", "Octave") => k(&[FT4, FT4, FT2]),
            ("VCO 2", "Pitch") => r(506, 518),
            ("VCO 1" | "VCO 2", "Shape") => r(0, 120),
            ("VCO 2", "Ring/Sync") => r(OFF, OFF),
            ("VCO 2", "Cross Mod") => r(0, 0),
            ("Pitch EG", "Int") => r(512, 512),
            ("Mixer", "VCO 2") => r(350, 800),
            ("Multi Engine", "Type") => r(VPM, VPM),
            ("Multi Engine", "VPM Type") => r(0, 0),
            ("Multi Engine", "VPM Shape") => r(0, 100),
            ("Multi Engine", "Octave") => k(&[FT16, FT8, FT4]),
            ("Multi Engine", "Routing") => r(0, 0),
            ("Mixer", "Multi") => r(250, 600),
            // No touch response, and gone when the key is.
            ("Amp EG", _) => adsr(l, [(0, 6), (600, 600), (1023, 1023), (15, 90)]),
            ("Filter", "Cutoff") => band((480, 900), 120, b),
            ("Filter", "Resonance") => r(0, by(0, 120, b)),
            ("Filter", "Velocity") => r(0, 15),
            ("Filter", "Key Track") => r(2, 2),
            // Key click.
            ("EG", _) => adsr(l, [(0, 0), (60, 200), (0, 0), (60, 200)]),
            ("Filter", "EG Int") => r(512, by(512, 600, mv)),
            // A rotary speaker's shimmer.
            ("LFO", "Target") => r(TO_PITCH, TO_PITCH),
            ("LFO", "Wave") => r(TRI, TRI),
            ("LFO", "Rate") => r(600, 820),
            ("LFO", "Int") => r(0, by(0, 90, mv)),
            ("Mod FX", "Type") => k(&[CHORUS, PHASER, ENSEMBLE]),
            _ => None,
        },
        "sub-bass" => match (s, l) {
            // Close to a sine; a quiet triangle an octave up gives it a
            // pitch on small speakers.
            ("VCO 1", "Wave") => k(&[TRI, TRI, SQR]),
            ("VCO 1", "Octave") => r(FT16, FT16),
            ("VCO 1", "Shape") => r(0, 80),
            ("VCO 2", "Wave") => r(TRI, TRI),
            ("VCO 2", "Octave") => r(FT8, FT8),
            ("VCO 2", "Pitch") => r(506, 518),
            ("VCO 2", "Ring/Sync") => r(OFF, OFF),
            ("VCO 2", "Cross Mod") => r(0, 0),
            ("Pitch EG", "Int") => r(512, 512),
            ("Mixer", "VCO 1") => r(950, 1023),
            ("Mixer", "VCO 2") => r(0, 320),
            ("Mixer", "Multi") => r(0, by(0, 150, g)),
            ("Filter", "Cutoff") => band((200, 420), 80, b),
            ("Filter", "Resonance") => r(0, by(0, 160, b)),
            ("Amp EG", _) => adsr(l, [(0, 12), (400, 700), (800, 1023), (40, 160)]),
            ("LFO", "Target") => r(TO_CUTOFF, TO_CUTOFF),
            ("LFO", "Int") => r(0, by(0, 100, mv)),
            ("Mod FX", "On") => r(0, 0),
            ("Delay/Reverb", "Depth") => r(0, by(0, 200, sp)),
            _ => None,
        },
        "acid" => match (s, l) {
            // One oscillator.
            ("VCO 1", "Wave") => k(&[SAW, SQR]),
            ("VCO 1", "Octave") => r(FT16, FT8),
            ("VCO 1", "Shape") => r(0, 300),
            ("VCO 2", "Ring/Sync") => r(OFF, OFF),
            ("VCO 2", "Cross Mod") => r(0, 0),
            ("Pitch EG", "Int") => r(512, 512),
            ("Mixer", "VCO 1") => r(900, 1023),
            ("Mixer", "VCO 2" | "Multi") => r(0, 0),
            // The squelch: resonance near the edge, and the EG snaps the
            // filter open on every note, harder on a hard one.
            ("Filter", "Resonance") => r(by(450, 560, b), by(620, 800, b)),
            ("Filter", "Cutoff") => band((200, 460), 90, b),
            ("Filter", "EG Int") => r(by(680, 760, mv), by(780, 960, mv)),
            ("Filter", "Velocity") => r(70, 127),
            ("Filter", "Key Track") => r(0, 1),
            ("EG", _) => adsr(l, [(0, 0), (110, 380), (0, 150), (80, 300)]),
            ("Amp EG", _) => adsr(l, [(0, 4), (300, 520), (650, 1000), (25, 100)]),
            ("LFO", "Int") => r(0, by(0, 140, mv)),
            ("Mod FX", "On") => r(0, 0),
            // A tempo delay.
            ("Delay/Reverb", "Type") => r(DELAY, DELAY),
            ("Delay/Reverb", "Delay Type") => k(&[6, 7, 8]),
            _ => None,
        },
        "sync-lead" => match (s, l) {
            // Hard sync, the second oscillator tuned well above the first,
            // and the EG pulling it through its harmonics.
            ("VCO 1" | "VCO 2", "Wave") => k(&[SAW, SAW, SQR]),
            ("VCO 2", "Ring/Sync") => r(SYNC, SYNC),
            ("VCO 2", "Pitch") => r(620, 960),
            ("VCO 2", "Octave") => r(FT8, FT4),
            ("Mixer", "VCO 2") => r(700, 1000),
            ("Pitch EG", "Target") => r(PITCH_VCO2, PITCH_VCO2),
            ("Pitch EG", "Int") => r(by(640, 700, mv), by(720, 900, mv)),
            ("EG", _) => adsr(l, [(0, 160), (350, 820), (0, 400), (200, 500)]),
            ("Amp EG", _) => adsr(l, [(0, 25), (500, 800), (800, 1000), (150, 380)]),
            ("Filter", "Cutoff") => band((480, 860), 160, b),
            ("LFO", "Target") => r(TO_PITCH, TO_PITCH),
            ("LFO", "Wave") => r(TRI, TRI),
            ("LFO", "Int") => r(by(0, 20, mv), by(20, 140, mv)),
            ("Delay/Reverb", "Type") => r(DELAY, DELAY),
            _ => None,
        },
        "fm-keys" => match (s, l) {
            // The VPM engine leads; a soft triangle may sit under it.
            ("Multi Engine", "Type") => r(VPM, VPM),
            ("Multi Engine", "VPM Type") => k(&[0, 1, 2, 3, 12, 13]),
            ("Multi Engine", "Octave") => k(&[FT8, FT8, FT4]),
            ("Mixer", "Multi") => r(820, 1000),
            ("VCO 1", "Wave") => r(TRI, TRI),
            ("Mixer", "VCO 1") => r(0, 360),
            ("Mixer", "VCO 2") => r(0, 0),
            ("VCO 2", "Ring/Sync") => r(OFF, OFF),
            ("VCO 2", "Cross Mod") => r(0, 0),
            ("Pitch EG", "Int") => r(512, 512),
            // Struck, then ringing down.
            ("Amp EG", _) => adsr(l, [(0, 0), (520, 750), (0, 200), (350, 700)]),
            ("EG", _) => adsr(l, [(0, 0), (300, 600), (0, 200), (300, 600)]),
            ("Filter", "Cutoff") => band((520, 860), 160, b),
            ("Filter", "Resonance") => r(0, by(0, 200, b)),
            ("LFO", "Target") => r(TO_SHAPE, TO_SHAPE),
            ("LFO", "Int") => r(0, by(0, 260, mv)),
            ("Mod FX", "Type") => k(&[CHORUS, ENSEMBLE, PHASER]),
            _ => None,
        },
        "bell" => match (s, l) {
            ("VCO 1", "Wave") => r(TRI, TRI),
            ("VCO 1", "Octave") => r(FT8, FT4),
            ("VCO 2", "Wave") => k(&[TRI, SQR]),
            // Metal: the second oscillator far out of tune, ringing the
            // first or bending it.
            ("VCO 2", "Pitch") => r(700, 980),
            ("VCO 2", "Ring/Sync") => k(&[RING, RING, OFF]),
            ("VCO 2", "Cross Mod") if ringing() => r(0, 100),
            ("VCO 2", "Cross Mod") => r(380, 750),
            ("Pitch EG", "Int") => r(512, 512),
            ("Multi Engine", "Type") => r(VPM, VPM),
            ("Multi Engine", "VPM Type") => k(&[0, 1, 2, 3, 12, 13]),
            ("Multi Engine", "Octave") => k(&[FT8, FT4, FT2]),
            ("Mixer", "Multi") => r(300, 600),
            ("Amp EG", _) => adsr(l, [(0, 0), (780, 1000), (0, 180), (500, 850)]),
            ("EG", _) => adsr(l, [(0, 10), (600, 900), (0, 200), (500, 850)]),
            ("Filter", "Cutoff") => band((450, 800), 150, b),
            ("Filter", "Resonance") => r(0, by(0, 300, b)),
            ("LFO", "Target") => r(TO_SHAPE, TO_SHAPE),
            ("LFO", "Int") => r(0, by(0, 300, mv)),
            ("Mod FX", "Type") => k(&[CHORUS, ENSEMBLE]),
            ("Delay/Reverb", "Type") => r(REVERB, REVERB),
            _ => None,
        },
        "kick" => match (s, l) {
            // One round oscillator, low; the keys do not move the filter.
            ("VCO 1", "Wave") => k(&[TRI, TRI, SQR]),
            ("VCO 1", "Octave") => r(FT16, FT16),
            ("VCO 1", "Shape") => r(0, 60),
            ("VCO 2", "Ring/Sync") => r(OFF, OFF),
            ("VCO 2", "Cross Mod") => r(0, 0),
            ("Mixer", "VCO 1") => r(950, 1023),
            ("Mixer", "VCO 2") => r(0, 0),
            // The beater: a touch of noise, more with grit.
            ("Multi Engine", "Type") => r(NOISE, NOISE),
            ("Multi Engine", "Noise Type") => k(&[N_LOW, N_HIGH, N_DECIM]),
            ("Mixer", "Multi") => r(0, by(0, 260, g)),
            // The thump: the pitch falls fast into the note.
            ("Pitch EG", "Target") => r(PITCH_BOTH, PITCH_BOTH),
            ("Pitch EG", "Int") => r(by(740, 780, mv), by(800, 900, mv)),
            ("EG", _) => adsr(l, [(0, 0), (60, 210), (0, 0), (60, 210)]),
            ("Amp EG", _) => adsr(l, [(0, 0), (200, 470), (0, 0), (120, 400)]),
            ("Filter", "Cutoff") => band((230, 640), 80, b),
            ("Filter", "Resonance") => r(0, by(0, 200, b)),
            ("Filter", "Key Track") => r(0, 0),
            ("Filter", "EG Int") => r(512, by(512, 700, mv)),
            ("LFO", "Int") => r(0, by(0, 150, mv)),
            ("Mod FX", "On") => r(0, 0),
            ("Delay/Reverb", "Type") => r(REVERB, REVERB),
            ("Delay/Reverb", "Reverb Type") => k(&[4, 3, 2]),
            ("Delay/Reverb", "Depth") => r(0, by(0, 350, sp)),
            _ => None,
        },
        "snare" => match (s, l) {
            // The shell: two oscillators out of tune with each other, as
            // a drum's modes are.
            ("VCO 1", "Wave") => k(&[TRI, SQR]),
            ("VCO 1", "Octave") => r(FT8, FT8),
            ("VCO 2", "Wave") => r(TRI, TRI),
            ("VCO 2", "Pitch") => r(700, 900),
            ("VCO 2", "Ring/Sync") => r(OFF, OFF),
            ("Mixer", "VCO 1") => r(400, 700),
            ("Mixer", "VCO 2") => r(0, 450),
            // The wires: noise, louder than the shell.
            ("Multi Engine", "Type") => r(NOISE, NOISE),
            ("Multi Engine", "Noise Type") => k(&[N_HIGH, N_PEAK, N_HIGH, N_DECIM]),
            ("Multi Engine", "Noise Shape") => r(300, 900),
            ("Mixer", "Multi") => r(720, 1023),
            ("Pitch EG", "Target") => r(PITCH_BOTH, PITCH_BOTH),
            ("Pitch EG", "Int") => r(by(680, 700, mv), by(720, 800, mv)),
            ("EG", _) => adsr(l, [(0, 0), (40, 150), (0, 0), (40, 150)]),
            ("Amp EG", _) => adsr(l, [(0, 0), (200, 400), (0, 0), (110, 380)]),
            ("Filter", "Cutoff") => band((380, 800), 120, b),
            ("Filter", "Key Track") => r(0, 0),
            ("LFO", "Int") => r(0, by(0, 150, mv)),
            ("Mod FX", "On") => r(0, 0),
            ("Delay/Reverb", "Type") => r(REVERB, REVERB),
            ("Delay/Reverb", "Reverb Type") => k(&[4, 3, 2]),
            _ => None,
        },
        "laser" => match (s, l) {
            ("VCO 1" | "VCO 2", "Wave") => k(&[SAW, SQR]),
            ("VCO 2", "Cross Mod") => r(0, by(0, 500, g)),
            // The zap: the pitch falls a long way.
            ("Pitch EG", "Target") => r(PITCH_BOTH, PITCH_BOTH),
            ("Pitch EG", "Int") => r(by(820, 860, mv), by(900, 1013, mv)),
            ("EG", _) => adsr(l, [(0, 0), (120, 520), (0, 0), (120, 520)]),
            ("Amp EG", _) => adsr(l, [(0, 0), (220, 600), (0, 0), (200, 600)]),
            ("Filter", "Resonance") => r(by(250, 400, b), by(450, 700, b)),
            // A fast LFO makes the beam shimmer.
            ("LFO", "Target") => r(TO_CUTOFF, TO_CUTOFF),
            ("LFO", "Mode") => r(LFO_FAST, LFO_FAST),
            ("LFO", "Rate") => r(450, 1000),
            ("LFO", "Int") => r(0, by(0, 600, mv)),
            ("Delay/Reverb", "Type") => r(DELAY, DELAY),
            ("Delay/Reverb", "Delay Type") => k(&[2, 0, 8]),
            _ => None,
        },
        "wind" => match (s, l) {
            // Noise alone.
            ("Mixer", "VCO 1" | "VCO 2") => r(0, 0),
            ("Multi Engine", "Type") => r(NOISE, NOISE),
            ("Multi Engine", "Noise Type") => k(&[N_HIGH, N_LOW, N_PEAK, N_PEAK]),
            ("Multi Engine", "Noise Shape") => r(150, 900),
            ("Mixer", "Multi") => r(750, 1023),
            // The whistle: resonance sings where the cutoff sits, and the
            // keys move it.
            ("Filter", "Resonance") => r(by(350, 450, b), by(560, 760, b)),
            ("Filter", "Cutoff") => band((260, 640), 120, b),
            ("Filter", "Key Track") => r(2, 2),
            ("Filter", "EG Int") => r(512, by(512, 640, mv)),
            ("Amp EG", _) => adsr(l, [(280, 700), (700, 700), (850, 1023), (500, 950)]),
            ("EG", _) => adsr(l, [(300, 800), (600, 1000), (300, 900), (500, 950)]),
            // Gusts.
            ("LFO", "Target") => r(TO_CUTOFF, TO_CUTOFF),
            ("LFO", "Wave") => r(TRI, TRI),
            ("LFO", "Mode") => r(LFO_SLOW, LFO_SLOW),
            ("LFO", "Rate") => r(20, by(120, 300, mv)),
            ("LFO", "Int") => r(by(220, 300, mv), by(420, 720, mv)),
            ("LFO", "Voice Sync") => r(0, 0),
            ("Delay/Reverb", "Type") => r(REVERB, REVERB),
            ("Delay/Reverb", "Reverb Type") => k(&[6, 0, 8, 9]),
            _ => None,
        },
        "siren" => match (s, l) {
            ("VCO 1" | "VCO 2", "Wave") => k(&[SAW, SQR]),
            ("VCO 2", "Ring/Sync") => r(OFF, OFF),
            ("Mixer", "VCO 2") => r(0, 500),
            ("Pitch EG", "Int") => r(512, 512),
            ("Amp EG", _) => adsr(l, [(0, 80), (700, 700), (950, 1023), (150, 600)]),
            ("Filter", "Cutoff") => band((450, 850), 150, b),
            ("Filter", "Resonance") => r(by(100, 200, b), by(300, 500, b)),
            // The wail: a slow LFO carries the pitch up and down,
            // restarting with each key.
            ("LFO", "Target") => r(TO_PITCH, TO_PITCH),
            ("LFO", "Wave") => k(&[TRI, TRI, SAW]),
            ("LFO", "Mode") => r(LFO_SLOW, LFO_SLOW),
            ("LFO", "Rate") => r(by(60, 250, mv), by(200, 420, mv)),
            ("LFO", "Int") => r(by(420, 560, mv), by(600, 820, mv)),
            ("LFO", "Key Sync") => r(1, 1),
            _ => None,
        },
        _ => None,
    }
}

/// The range any character draws free field `f` from unless it says
/// otherwise ([`special`]).
fn common(f: &Field, c: &Ctx) -> Draw {
    let m = c.m;
    let vpm = &VPM_SAFE[usize::from(c.p.get(field("Main Multi Engine VPM Type"))).min(15)];
    let (lo, hi) = match (f.section, f.label) {
        ("VCO 1" | "VCO 2", "Octave") => (FT8, FT4),
        ("VCO 1", "Pitch") => (500, 524),
        ("VCO 2", "Pitch") => (470, 560),
        ("VCO 2", "Ring/Sync") if m.grit > 0.7 => (RING, SYNC),
        ("VCO 2", "Ring/Sync") => (OFF, OFF),
        ("VCO 2", "Cross Mod") => (0, by(0, 600, m.grit)),
        ("Pitch EG", "Int") => (512 - by(0, 112, m.movement), by(512, 640, m.movement)),
        ("Multi Engine", "Type") => (NOISE, VPM),
        ("Multi Engine", "Octave") => (FT8, FT4),
        // Each VPM type within what it takes (the logue VPM table).
        ("Multi Engine", "VPM Shape") => (vpm.shape.0, by(vpm.shape.0, vpm.shape.1, m.bright)),
        ("Multi Engine", "VPM Shift") => vpm.shift,
        ("Multi Engine", "Feedback") => (100, by(100, vpm.feedback, m.grit)),
        ("Multi Engine", "Noise Depth") => (100, by(100, vpm.noise, m.grit)),
        ("Multi Engine", "Shape Mod") => vpm.shape_mod,
        ("Multi Engine", "Mod Attack") => vpm.attack,
        ("Multi Engine", "Mod Decay") => vpm.decay,
        ("Multi Engine", "Mod Key Track") => vpm.track,
        ("Mixer", "VCO 1") => (600, 1023),
        ("Mixer", "Multi") => (0, by(0, 700, m.grit)),
        ("Filter", "Cutoff") => {
            let lo = by(200, 800, m.bright);
            (lo, (lo + 250).min(1023))
        }
        ("Filter", "Resonance") => (0, by(0, 700, m.bright)),
        ("Filter", "EG Int") => (512, by(512, 900, m.movement)),
        ("Filter", "Drive") if m.grit > 0.5 => (1, 2),
        ("Filter", "Drive") => (0, 1),
        ("Filter", "Low Cut") => (0, 0),
        ("EG", "Attack") => (0, 400),
        ("EG", "Decay") => (100, 800),
        ("EG", "Release") => (100, 700),
        // The LFO moves the tone; only a character sends it to the pitch.
        ("LFO", "Target") => (TO_CUTOFF, TO_SHAPE),
        ("LFO", "Mode") => (LFO_SLOW, LFO_FAST),
        ("LFO", "Rate") => (100, 800),
        ("LFO", "Int") => (0, by(0, 400, m.movement)),
        ("Mod FX", "On") => {
            let on = u16::from(m.space > 0.3);
            (on, on)
        }
        ("Mod FX", "Type") => (1, 4),
        ("Delay/Reverb", "On") => {
            let on = u16::from(m.space > 0.2);
            (on, on)
        }
        ("Delay/Reverb", "Type") => (DELAY, REVERB),
        ("Delay/Reverb", "Delay Type") => (0, 11),
        ("Delay/Reverb", "Reverb Type") => (0, 9),
        ("Delay/Reverb", "Time") => (200, 900),
        ("Delay/Reverb", "Depth") => (0, by(0, 600, m.space)),
        _ => (f.min, f.max),
    };
    Range(lo, hi)
}

/// Whether something sounds: an oscillator or the multi engine at a level
/// to hear.
fn audible(p: &Program) -> bool {
    ["VCO 1", "VCO 2", "Multi"]
        .iter()
        .any(|s| p.get(field(&format!("Main Mixer {s}"))) >= 400)
}

/// Make the program sound (see the module docs). Touches only free fields;
/// idempotent.
fn cohere(p: &mut Program) {
    let clamp = |p: &mut Program, name: &str, lo: u16, hi: u16| {
        let f = field(name);
        let v = p.get(f).clamp(lo, hi);
        p.set(f, v).expect("in range");
    };
    if !audible(p) {
        clamp(p, "Main Mixer VCO 1", 400, 1023);
    }
    clamp(p, "Main Filter Cutoff", 200, 1023);
    clamp(p, "Main Filter Resonance", 0, 800);
    clamp(p, "Main Multi Engine Type", 0, 1);
    clamp(p, "Mod FX Type", 1, 4);
    clamp(p, "Delay/Reverb Delay Type", 0, 11);
    clamp(p, "Delay/Reverb Reverb Type", 0, 9);
    if p.get(field("Main Amp EG Sustain")) < 100 {
        clamp(p, "Main Amp EG Decay", 200, 1023);
    }
}

/// `bytes` after the coherence pass.
pub fn cohered(bytes: &[u8]) -> Result<Vec<u8>, DriverError> {
    let mut p = program(bytes)?;
    cohere(&mut p);
    Ok(p.as_bytes().to_vec())
}

fn program(bytes: &[u8]) -> Result<Program, DriverError> {
    Program::from_bytes(bytes).map_err(|e| DriverError(e.to_string()))
}

fn free() -> impl Iterator<Item = (usize, &'static Field)> {
    all()
        .iter()
        .enumerate()
        .filter(|(_, f)| mutability(f) == Mutability::Free)
}

fn set(p: &mut Program, f: &Field, v: u16) {
    p.set(f, v.clamp(f.min, f.max)).expect("clamped into range");
}

pub fn generate(recipe: &Recipe, base: Option<&[u8]>) -> Result<Vec<u8>, DriverError> {
    let c = CHARACTERS
        .get(recipe.character)
        .ok_or_else(|| DriverError(format!("no character {}", recipe.character)))?;
    let seed = recipe.seed ^ (recipe.character as u64).wrapping_mul(0xA24B_AED4_963E_E407);
    // Non-free fields come from the base as they are, in range or not.
    let base = base.map(program).transpose()?;
    let mut p = base.clone().unwrap_or_else(Program::init);
    for (i, f) in free() {
        let ctx = Ctx {
            m: &recipe.macros,
            p: &p,
        };
        let draw = special(c.id, &ctx, f.section, f.label).unwrap_or_else(|| common(f, &ctx));
        let roll = roll_field(seed, i, 0);
        let v = match draw {
            Range(lo, hi) => lo + ((roll * f32::from(hi - lo + 1)) as u16).min(hi - lo),
            Pick(values) => values[((roll * values.len() as f32) as usize).min(values.len() - 1)],
        };
        set(&mut p, f, v);
    }
    cohere(&mut p);
    p.set_name(&name(c, recipe.seed))
        .map_err(|e| DriverError(e.to_string()))?;
    // Kept pages come back bit for bit, never through `set`.
    if let Some(base) = &base {
        for &page in &recipe.keep {
            let page = CURATION
                .pages
                .get(page)
                .ok_or_else(|| DriverError(format!("no page {page}")))?;
            for &i in page.sections.iter().flat_map(|s| &s.fields) {
                p.copy_field(base, &all()[i]);
            }
        }
    }
    Ok(p.as_bytes().to_vec())
}

/// Continuous free fields move by up to `amount` of half their range;
/// choices and switches change with probability `amount / 5`.
pub fn vary(bytes: &[u8], amount: f32, seed: u64) -> Result<Vec<u8>, DriverError> {
    let amount = amount.clamp(0.0, 1.0);
    let mut p = program(bytes)?;
    for (i, f) in free() {
        let (span, now) = (f32::from(f.max - f.min), f32::from(p.get(f)));
        let r = roll_field(seed, i, 1);
        let v = if control(&p, f) == Control::Continuous {
            now + (r * 2.0 - 1.0) * amount * span * 0.5
        } else if r < amount / 5.0 {
            f32::from(f.min) + (roll_field(seed, i, 2) * (span + 1.0)).floor()
        } else {
            now
        };
        set(&mut p, f, v.round().max(0.0) as u16);
    }
    cohere(&mut p);
    Ok(p.as_bytes().to_vec())
}

/// Continuous free fields interpolate by `mix`; choices and switches come
/// from `b` with probability `mix`; values are clamped into range (a parent
/// may hold an out-of-range byte). Everything else, and the name, is `a`'s.
pub fn breed(a: &[u8], b: &[u8], mix: f32, seed: u64) -> Result<Vec<u8>, DriverError> {
    let mix = mix.clamp(0.0, 1.0);
    let (mut p, b) = (program(a)?, program(b)?);
    for (i, f) in free() {
        let (x, y) = (f32::from(p.get(f)), f32::from(b.get(f)));
        let v = if control(&p, f) == Control::Continuous {
            x + (y - x) * mix
        } else if roll_field(seed, i, 3) < mix {
            y
        } else {
            x
        };
        set(&mut p, f, v.round() as u16);
    }
    cohere(&mut p);
    Ok(p.as_bytes().to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn recipe(character: usize, seed: u64) -> Recipe {
        Recipe {
            character,
            seed,
            ..Recipe::default()
        }
    }

    #[test]
    fn generation_is_coherent() {
        let get = |p: &Program, n: &str| p.get(field(n));
        for (c, character) in CHARACTERS.iter().enumerate() {
            for seed in 0..32 {
                let b = generate(&recipe(c, seed), None).unwrap();
                assert_eq!(generate(&recipe(c, seed), None).unwrap(), b);
                let p = Program::from_bytes(&b).unwrap();
                assert!(audible(&p));
                assert!(get(&p, "Main Filter Cutoff") >= 200);
                assert!(get(&p, "Main Filter Resonance") <= 800);
                assert!(get(&p, "Main Multi Engine Type") <= 1);
                assert!(
                    get(&p, "Main Amp EG Sustain") >= 100 || get(&p, "Main Amp EG Decay") >= 200
                );
                assert_eq!(get(&p, "Sub On"), 0);
                assert_eq!(p.name(), name(character, seed));
            }
        }
    }

    /// The quality gate. A hand's sounds differ in at least 16 fields
    /// (the closest pair in any hand differs in 30):
    /// two draws of one recipe share its fixed choices but not a dozen of
    /// its dials, and a clone or a near-clone fails.
    #[test]
    fn quality_gate() {
        let program = |b: &[u8]| Program::from_bytes(b).unwrap();
        oscyl_conformance::generated_sounds_are_good(
            &crate::DRIVER,
            &oscyl_conformance::Quality {
                audible: &|b| {
                    let p = program(b);
                    audible(&p)
                        && (p.get(field("Main Amp EG Sustain")) >= 100
                            || p.get(field("Main Amp EG Decay")) >= 200)
                },
                level_capped: &|b| program(b).get(field("Program Level")) <= 102,
                min_spread: 16,
            },
        );
    }

    #[test]
    fn restricted_and_fixed_fields_stay() {
        assert_eq!(mutability(field("Main Filter Cutoff")), Mutability::Free);
        assert_eq!(
            mutability(field("Sub Filter Cutoff")),
            Mutability::Restricted
        );
        assert_eq!(mutability(field("Micro Tuning")), Mutability::Restricted);
        assert_eq!(
            mutability(field("Main Multi Engine Param 1 Type")),
            Mutability::Fixed
        );
        let init = Program::init();
        let p = Program::from_bytes(&generate(&recipe(0, 5), None).unwrap()).unwrap();
        for f in all().iter().filter(|f| mutability(f) != Mutability::Free) {
            assert_eq!(p.get(f), init.get(f), "{}", f.name);
        }
    }

    #[test]
    fn pre_2_00_zeros_survive_generation() {
        // A 1.x base (init is one): every 2.00 field stays "not set"
        // through generate, vary and breed.
        let base = Program::init();
        let other = Program::from_bytes(&generate(&recipe(0, 9), None).unwrap()).unwrap();
        let made = [
            generate(&recipe(1, 3), Some(base.as_bytes())).unwrap(),
            vary(base.as_bytes(), 1.0, 2).unwrap(),
            breed(base.as_bytes(), other.as_bytes(), 1.0, 4).unwrap(),
        ];
        for b in made {
            let p = Program::from_bytes(&b).unwrap();
            assert_eq!(&p.as_bytes()[51..59], &[0; 8], "bytes 51-58");
            for t in [80, 206] {
                assert_eq!(p.as_bytes()[t + 120], 0xFF, "aftertouch");
            }
        }
        for name in [
            "Micro Tuning",
            "Scale Key",
            "Program Tuning",
            "Program Transpose",
            "Arpeggiator Gate Time",
            "Arpeggiator Rate",
            "Delay/Reverb Dry/Wet",
            "Main Controllers Aftertouch",
            "Sub Controllers Aftertouch",
        ] {
            assert_eq!(mutability(field(name)), Mutability::Restricted, "{name}");
        }
    }

    #[test]
    fn unverified_mod_fx_subtypes_are_restricted() {
        for name in ["Chorus", "Ensemble", "Phaser", "Flanger"] {
            let f = field(&format!("Mod FX {name}"));
            assert_eq!(mutability(f), Mutability::Restricted, "{name}");
        }
    }
}
