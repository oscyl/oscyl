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
//! Coherence: a VCO is audible, the filter is open at least a fifth of the
//! way, resonance stays under 800, a note either sustains or decays slowly
//! enough to hear, the multi engine is noise or VPM (a user oscillator may
//! not be loaded), and an effect that is on has a type that is not a user
//! slot.

use oscyl_driver::{Character, Control, DriverError, MacroInfo, Macros, Mutability, Recipe};

use crate::Program;
use crate::curate::{CURATION, ENVELOPES, OSCILLATORS, control};
use crate::fields::{Field, all, field};

pub const CHARACTERS: &[Character] = &[
    Character {
        id: "pad",
        title: "Pad",
        hint: "Slow swell, long release",
        signature_page: Some(ENVELOPES),
    },
    Character {
        id: "pluck",
        title: "Pluck",
        hint: "Short and percussive",
        signature_page: Some(ENVELOPES),
    },
    Character {
        id: "bass",
        title: "Bass",
        hint: "Low and tight",
        signature_page: Some(OSCILLATORS),
    },
    Character {
        id: "lead",
        title: "Lead",
        hint: "Bright, sustained solo voice",
        signature_page: Some(OSCILLATORS),
    },
    Character {
        id: "keys",
        title: "Keys",
        hint: "Decaying, playable chords",
        signature_page: Some(ENVELOPES),
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

/// A generated program's name: the character and four hex digits of the
/// seed.
pub fn name(c: &Character, seed: u64) -> String {
    format!("{} {:04X}", c.title, seed & 0xFFFF)
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

/// The range a fresh program draws free field `f` from.
fn range(f: &Field, character: usize, m: &Macros) -> (u16, u16) {
    // (attack, decay, sustain, release) per character.
    let amp = [
        [(300, 700), (300, 800), (700, 1023), (500, 900)],
        [(0, 30), (200, 500), (0, 100), (100, 400)],
        [(0, 40), (200, 600), (400, 900), (50, 250)],
        [(0, 100), (300, 700), (700, 1023), (100, 400)],
        [(0, 60), (400, 800), (200, 600), (200, 500)],
    ][character];
    let stage = |a: [(u16, u16); 4]| match f.label {
        "Attack" => a[0],
        "Decay" => a[1],
        "Sustain" => a[2],
        _ => a[3],
    };
    match (f.section, f.label) {
        ("VCO 1" | "VCO 2", "Octave") if character == 2 => (0, 1),
        ("VCO 1" | "VCO 2", "Octave") => (1, 2),
        ("VCO 1", "Pitch") => (500, 524),
        ("VCO 2", "Pitch") => (470, 560),
        ("VCO 2", "Ring/Sync") if m.grit > 0.7 => (0, 2),
        ("VCO 2", "Ring/Sync") => (1, 1),
        ("VCO 2", "Cross Mod") => (0, by(0, 600, m.grit)),
        ("Pitch EG", "Int") => (512 - by(0, 112, m.movement), by(512, 640, m.movement)),
        ("Multi Engine", "Type") => (0, 1),
        ("Multi Engine", "Octave") => (1, 2),
        ("Multi Engine", "Feedback" | "Noise Depth" | "Shape Mod") => (60, 140),
        ("Multi Engine", "Mod Attack" | "Mod Decay" | "Mod Key Track") => (60, 140),
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
        ("Amp EG", _) => stage(amp),
        ("EG", _) => stage([(0, 400), (100, 800), (0, 1023), (100, 700)]),
        ("LFO", "Mode") => (1, 2),
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
        ("Delay/Reverb", "Type") => (1, 2),
        ("Delay/Reverb", "Delay Type") => (0, 11),
        ("Delay/Reverb", "Reverb Type") => (0, 9),
        ("Delay/Reverb", "Time") => (200, 900),
        ("Delay/Reverb", "Depth") => (0, by(0, 600, m.space)),
        _ => (f.min, f.max),
    }
}

/// Make the program sound (see the module docs). Touches only free fields;
/// idempotent.
fn cohere(p: &mut Program) {
    let clamp = |p: &mut Program, name: &str, lo: u16, hi: u16| {
        let f = field(name);
        let v = p.get(f).clamp(lo, hi);
        p.set(f, v).expect("in range");
    };
    if p.get(field("Main Mixer VCO 1"))
        .max(p.get(field("Main Mixer VCO 2")))
        < 400
    {
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
        let (lo, hi) = range(f, recipe.character, &recipe.macros);
        let v = lo + ((roll_field(seed, i, 0) * f32::from(hi - lo + 1)) as u16).min(hi - lo);
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
                assert!(get(&p, "Main Mixer VCO 1").max(get(&p, "Main Mixer VCO 2")) >= 400);
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
