//! A small deterministic generator for the Prophet Rev2.
//!
//! It sets only layer A's sound fields ([`Mutability::Free`]): oscillators,
//! filter, filter and amp envelopes, LFO rate/shape/amount and the effect.
//! Everything else keeps the init (or, for vary and breed, the parent)
//! value: layer B, volume, destinations and the mod matrix (the docs do
//! not say what their values mean), voice, arp and sequencer. Each field
//! draws from its own random stream (seed and offset), so a macro changes
//! only arithmetic, never which numbers a field gets.
//!
//! Coherence: both oscillators sound, the filter is open at least 40 of
//! 164, resonance stays at or under 110, the effect mix at or under 100,
//! the amp envelope amount is at least 90, and a note either sustains or
//! decays slowly enough to hear.
//!
//! Limits: ranges are chosen by ear on paper, not on hardware; LFOs move
//! whatever their (init) destination is; the effect parameters are drawn
//! over their whole range because the docs give no meaning for them.

use oscyl_driver::{Character, Control, DriverError, MacroInfo, Macros, Mutability, Recipe};

use crate::curate::{CURATION, control, index};
use crate::{Field, Program, fields};

pub(crate) const CHARACTERS: &[Character] = &[
    Character {
        id: "pad",
        title: "Pad",
        hint: "Slow swell, long release",
        signature_page: Some(2), // Envelopes
    },
    Character {
        id: "pluck",
        title: "Pluck",
        hint: "Short and percussive",
        signature_page: Some(2), // Envelopes
    },
    Character {
        id: "bass",
        title: "Bass",
        hint: "Low and tight",
        signature_page: Some(2), // Envelopes
    },
    Character {
        id: "lead",
        title: "Lead",
        hint: "Bright, sustained solo voice",
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
        help: "LFO amount and filter envelope amount",
    },
    MacroInfo {
        title: "Grit",
        help: "Noise, oscillator slop, sync and filter audio mod",
    },
    MacroInfo {
        title: "Ambience",
        help: "Effect on and its mix",
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
    let env = b.starts_with("Filter env ") || b.starts_with("Amp env ");
    let lfo = b.starts_with("LFO ") && ["rate", "shape", "amount"].iter().any(|w| b.ends_with(w));
    if osc || env || lfo || FREE.contains(&b) {
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

/// Field `f`'s `k`-th random number.
fn roll_field(seed: u64, f: &Field, k: u64) -> f32 {
    roll(seed, (u64::from(f.offset) << 8) | k)
}

/// `lo..=hi` scaled by `t` from `lo`: `lo + t * (hi - lo)`.
fn by(lo: u16, hi: u16, t: f32) -> u16 {
    lo + (f32::from(hi - lo) * t.clamp(0.0, 1.0)).round() as u16
}

/// The range a fresh program draws field `b` (a layer A name) from.
fn range(b: &str, f: &Field, character: usize, m: &Macros) -> (u16, u16) {
    let stage = b.rsplit(' ').next().unwrap_or("");
    // (attack, decay, sustain, release) per character.
    let amp = [
        [(60, 110), (40, 100), (90, 127), (60, 110)],
        [(0, 5), (20, 60), (0, 10), (10, 40)],
        [(0, 10), (20, 80), (40, 110), (5, 30)],
        [(0, 20), (30, 90), (80, 127), (10, 50)],
    ][character];
    let adsr = |a: [(u16, u16); 4]| match stage {
        "attack" => a[0],
        "decay" => a[1],
        "sustain" => a[2],
        "release" => a[3],
        _ => (0, 0),
    };
    match b {
        _ if b.ends_with(" frequency") && character == 2 => (1, 2),
        _ if b.ends_with(" frequency") => (2, 4),
        _ if b.ends_with(" fine tune") => (44, 56),
        _ if b.starts_with("Osc ") && b.ends_with(" shape") => (1, 4),
        "Sub osc level" if character == 2 => (60, 127),
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
        _ if b.starts_with("Amp env ") && stage != "velocity" => adsr(amp),
        _ if b.starts_with("Filter env ") && stage != "velocity" => {
            adsr([(0, 60), (20, 100), (0, 127), (10, 90)])
        }
        _ if b.starts_with("LFO ") && b.ends_with(" amount") => (0, by(0, 40, m.movement)),
        "FX on" => {
            let on = u16::from(m.space > 0.2);
            (on, on)
        }
        "FX mix" => (0, by(0, 100, m.space)),
        _ => (f.min, f.max),
    }
}

/// Make the program sound (see the module docs). Touches only free
/// fields; idempotent.
fn cohere(p: &mut Program) {
    let f = |name: &str| &fields::all()[index(name)];
    let clamp = |p: &mut Program, name: &str, lo: u16, hi: u16| {
        let v = p.get(f(name)).clamp(lo, hi);
        p.set(f(name), v).expect("in range");
    };
    clamp(p, "A Osc 1 shape", 1, 4);
    clamp(p, "A Osc 2 shape", 1, 4);
    clamp(p, "A Filter cutoff", 40, 164);
    clamp(p, "A Amp env amount", 90, 127);
    clamp(p, "A Resonance", 0, 110);
    clamp(p, "A FX mix", 0, 100);
    if p.get(f("A Amp env sustain")) < 20 {
        clamp(p, "A Amp env decay", 40, 127);
    }
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
        let (lo, hi) = range(&f.name[2..], f, recipe.character, &recipe.macros);
        let v = lo + ((roll_field(seed, f, 0) * f32::from(hi - lo + 1)) as u16).min(hi - lo);
        // Oscillator frequencies are drawn in octaves.
        let v = if f.name.ends_with(" frequency") {
            v * 12
        } else {
            v
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

/// Continuous free fields move by up to `amount` of half their range;
/// choices and switches change with probability `amount / 5`.
pub(crate) fn vary(bytes: &[u8], amount: f32, seed: u64) -> Result<Vec<u8>, DriverError> {
    let amount = amount.clamp(0.0, 1.0);
    let mut p = program(bytes)?;
    for (i, f) in free() {
        let (span, now) = (f32::from(f.max - f.min), f32::from(p.get(f)));
        let r = roll_field(seed, f, 1);
        let v = if control(i) == Control::Continuous {
            now + (r * 2.0 - 1.0) * amount * span * 0.5
        } else if r < amount / 5.0 {
            f32::from(f.min) + (roll_field(seed, f, 2) * (span + 1.0)).floor()
        } else {
            now
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
        let v = if control(i) == Control::Continuous {
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
        assert_eq!(DRIVER.characters().len(), 4);
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
        assert_eq!(m[3].help, "Effect on and its mix");
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
