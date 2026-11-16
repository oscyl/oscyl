//! Whole oscillators from a seed, by family; variations and children of
//! oscillators. Every one made here validates ([`Patch::validate`]): within
//! the CPU budget, at a set level, never silent.

use super::{BUDGET, Block, Kind, MAX_MODS, Mod, Patch, Source, Table, waves};

/// What an oscillator is built around.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Family {
    /// Tables that morph into each other, bent and synced.
    Wavetable = 1,
    /// Two-operator FM with feedback.
    Fm,
    /// A simple wave through a wavefolder.
    Fold,
    /// Vowels: rich waves through formant filters.
    Formant,
    /// Noise, sample and hold, crushed and coloured.
    Noise,
    /// Anything goes: two sources and whatever shapes them.
    Hybrid,
}

impl Family {
    pub const ALL: [Family; 6] = [
        Family::Wavetable,
        Family::Fm,
        Family::Fold,
        Family::Formant,
        Family::Noise,
        Family::Hybrid,
    ];

    /// Its id, as the command line and the apps spell it.
    pub fn id(self) -> &'static str {
        match self {
            Family::Wavetable => "wavetable",
            Family::Fm => "fm",
            Family::Fold => "fold",
            Family::Formant => "formant",
            Family::Noise => "noise",
            Family::Hybrid => "hybrid",
        }
    }

    pub fn from_id(id: &str) -> Option<Family> {
        Family::ALL.into_iter().find(|f| f.id() == id)
    }
}

/// SplitMix64, as the drivers' generators.
pub(crate) struct Rng(u64);

impl Rng {
    pub(crate) fn new(seed: u64) -> Rng {
        Rng(seed)
    }

    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// 0 to 1.
    pub(crate) fn f(&mut self) -> f32 {
        (self.next() >> 40) as f32 / (1u64 << 24) as f32
    }

    pub(crate) fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }

    fn chance(&mut self, p: f32) -> bool {
        self.f() < p
    }

    /// A constant between `lo` and `hi` (0 to 1).
    fn k(&mut self, lo: f32, hi: f32) -> u8 {
        ((lo + (hi - lo) * self.f()) * 255.0).round() as u8
    }

    fn pick<T: Copy>(&mut self, of: &[T]) -> T {
        of[self.below(of.len())]
    }
}

const ONSETS: &[&str] = &[
    "b", "br", "c", "d", "dr", "f", "g", "gl", "k", "kr", "l", "m", "n", "p", "pr", "q", "r", "s",
    "sh", "st", "t", "tr", "v", "vr", "z", "zh", "th", "x",
];
const VOWELS: &[&str] = &["a", "e", "i", "o", "u", "y", "ae", "io", "ou", "ei"];
const CODAS: &[&str] = &["", "", "n", "r", "x", "l", "m", "s", "th", "k", "sh", "nd"];

/// A made-up name of two or three syllables, at most 13 letters.
pub fn name(seed: u64) -> String {
    let mut r = Rng::new(seed ^ 0x6E61_6D65);
    let mut s = String::new();
    for _ in 0..2 + r.below(2) {
        s.push_str(r.pick(ONSETS));
        s.push_str(r.pick(VOWELS));
    }
    s.push_str(r.pick(CODAS));
    s.truncate(super::NAME_LEN);
    let mut c = s.chars();
    c.next()
        .map(|f| f.to_ascii_uppercase().to_string() + c.as_str())
        .unwrap_or_default()
}

/// A patch under construction: blocks, mods, the six parameters' names.
struct Draft {
    blocks: Vec<Block>,
    mods: Vec<Mod>,
    params: Vec<String>,
    tables: Vec<Table>,
}

impl Draft {
    fn new() -> Draft {
        Draft {
            blocks: vec![],
            mods: vec![],
            params: vec![],
            tables: vec![],
        }
    }

    /// Add a block; its index.
    fn block(&mut self, kind: Kind, k: [u8; 7]) -> u8 {
        self.blocks.push(Block { kind, k });
        (self.blocks.len() - 1) as u8
    }

    fn cost(&self) -> u32 {
        self.blocks.iter().map(|b| b.kind.cost()).sum()
    }

    fn fits(&self, kind: Kind) -> bool {
        self.blocks.len() < super::MAX_BLOCKS && self.cost() + kind.cost() <= BUDGET
    }

    fn m(&mut self, source: Source, block: u8, param: u8, amount: i8) {
        if self.mods.len() < MAX_MODS && amount != 0 {
            self.mods.push(Mod {
                source,
                block,
                param,
                amount,
            });
        }
    }

    /// A parameter called `name` moving constant `param` of `block` by
    /// `amount` (64ths) from where it is.
    fn knob(&mut self, name: &str, block: u8, param: u8, amount: i8) {
        if self.params.len() < 6 && !self.params.iter().any(|p| p == name) {
            let i = self.params.len() as u8;
            self.params.push(name.into());
            self.m(Source::Param(i), block, param, amount);
        }
    }
}

/// An amount of `lo` to `hi` 64ths, either way round when `either`.
fn amount(r: &mut Rng, lo: i8, hi: i8, either: bool) -> i8 {
    let a = lo + r.below((hi - lo) as usize + 1) as i8;
    if either && r.chance(0.5) { -a } else { a }
}

/// Some movement of its own: the LFO or the envelope on a block's constant.
fn motion(r: &mut Rng, d: &mut Draft, block: u8, param: u8) {
    if r.chance(0.5) {
        let a = amount(r, 3, 14, true);
        d.m(Source::Lfo, block, param, a);
    }
    if r.chance(0.4) {
        let a = amount(r, 6, 32, true);
        d.m(Source::Env, block, param, a);
    }
}

/// Room left: add a sub, a low-pass or a crush when they fit.
fn extras(r: &mut Rng, d: &mut Draft) {
    if r.chance(0.35) && d.fits(Kind::Sub) {
        let b = d.block(Kind::Sub, [0, r.k(0.0, 0.6), 0, 0, 0, 0, 0]);
        d.knob("Sub", b, 0, 40);
    }
    if r.chance(0.45) && d.fits(Kind::Lowpass) {
        let cutoff = r.k(0.45, 1.0);
        let b = d.block(Kind::Lowpass, [cutoff, 0, 0, 0, 0, 0, 0]);
        // Fully down, it still lets eight harmonics through.
        let room = (f32::from(cutoff) / 255.0 - 0.43) * 64.0;
        d.knob("Tone", b, 0, -(room as i8).max(4));
        if r.chance(0.3) {
            d.m(Source::Env, b, 0, amount(r, 8, 24, false));
        }
    }
    if r.chance(0.15) && d.fits(Kind::Crush) {
        let b = d.block(Kind::Crush, [r.k(0.2, 0.6), r.k(0.0, 0.3), 0, 0, 0, 0, 0]);
        d.knob("Crush", b, 2, 64);
    }
}

fn wavetable(r: &mut Rng, seed: u64, d: &mut Draft) {
    d.tables = waves::family(seed, 4 + r.below(5));
    if r.chance(0.35) {
        let b = d.block(Kind::PhaseDist, [r.k(0.0, 0.4), 0, 0, 0, 0, 0, 0]);
        d.m(Source::ShiftShape, b, 0, amount(r, 24, 48, false));
        d.knob("Bend", b, 0, 32);
    } else if r.chance(0.35) {
        let b = d.block(Kind::Sync, [r.k(0.0, 0.2), 0, 0, 0, 0, 0, 0]);
        d.m(Source::ShiftShape, b, 0, amount(r, 16, 48, false));
        d.knob("Sync", b, 0, 40);
    }
    let w = d.block(Kind::Wavetable, [r.k(0.0, 0.3), r.k(0.7, 1.0), 0, 0, 0, 0, 0]);
    d.m(Source::Shape, w, 0, 64);
    motion(r, d, w, 0);
    if r.chance(0.3) && d.fits(Kind::Fold) {
        let b = d.block(Kind::Fold, [r.k(0.0, 0.3), r.k(0.0, 1.0), 0, 0, 0, 0, 0]);
        d.knob("Fold", b, 2, 64);
    }
}

fn fm(r: &mut Rng, d: &mut Draft) {
    let b = d.block(
        Kind::Fm,
        [r.k(0.0, 1.0), r.k(0.0, 0.4), r.k(0.0, 0.5), r.k(0.7, 1.0), 0, 0, 0],
    );
    d.m(Source::Shape, b, 1, amount(r, 32, 56, false));
    d.m(Source::ShiftShape, b, 2, amount(r, 24, 56, false));
    d.knob("Ratio", b, 0, 48);
    d.knob("Feedback", b, 2, 40);
    motion(r, d, b, 1);
    if r.chance(0.3) {
        let s = d.block(Kind::Saw, [r.k(0.1, 0.4), 0, 0, 0, 0, 0, 0]);
        d.knob("Saw", s, 0, 32);
    }
}

fn fold(r: &mut Rng, d: &mut Draft) {
    let src = r.below(3);
    match src {
        0 => {
            d.block(Kind::Wavetable, [0, r.k(0.6, 0.9), 0, 0, 0, 0, 0]);
        }
        1 => {
            d.block(Kind::Saw, [r.k(0.5, 0.8), 0, 0, 0, 0, 0, 0]);
        }
        _ => {
            let b = d.block(
                Kind::Fm,
                [r.k(0.0, 0.5), r.k(0.0, 0.2), 0, r.k(0.6, 0.9), 0, 0, 0],
            );
            d.knob("Index", b, 1, 40);
        }
    }
    let f = d.block(
        Kind::Fold,
        [r.k(0.0, 0.3), r.k(0.0, 0.6), r.k(0.7, 1.0), 0, 0, 0, 0],
    );
    d.m(Source::Shape, f, 0, amount(r, 40, 64, false));
    d.m(Source::ShiftShape, f, 1, amount(r, 24, 64, false));
    d.knob("Bias", f, 1, 48);
    d.knob("Mix", f, 2, -48);
    motion(r, d, f, 0);
}

fn formant(r: &mut Rng, d: &mut Draft) {
    if r.chance(0.35) {
        d.block(Kind::Supersaw, [r.k(0.1, 0.6), r.k(0.6, 0.9), 0, 0, 0, 0, 0]);
    } else {
        d.block(Kind::Saw, [r.k(0.6, 0.9), 0, 0, 0, 0, 0, 0]);
        if r.chance(0.4) {
            let n = d.block(Kind::Noise, [r.k(0.05, 0.25), 0, 0, 0, 0, 0, 0]);
            d.knob("Breath", n, 0, 32);
        }
    }
    let f = d.block(
        Kind::Formant,
        [
            r.k(0.0, 0.5),
            r.k(0.2, 0.8),
            r.k(0.7, 1.0),
            r.k(0.3, 0.7),
            0,
            0,
            0,
        ],
    );
    d.m(Source::Shape, f, 0, 64);
    d.m(Source::ShiftShape, f, 3, amount(r, 16, 40, true));
    d.knob("Reso", f, 1, 40);
    d.knob("Mix", f, 2, -48);
    motion(r, d, f, 0);
}

fn noise(r: &mut Rng, d: &mut Draft) {
    let n = d.block(Kind::Noise, [r.k(0.6, 0.9), r.k(0.0, 0.5), 0, 0, 0, 0, 0]);
    d.m(Source::Shape, n, 1, amount(r, 24, 64, false));
    d.knob("Hold", n, 1, 40);
    // A pitched part, so it plays in tune.
    let tone = d.block(Kind::Wavetable, [0, r.k(0.2, 0.6), 0, 0, 0, 0, 0]);
    d.knob("Tone mix", tone, 1, 32);
    if r.chance(0.5) {
        let f = d.block(
            Kind::Formant,
            [r.k(0.0, 1.0), r.k(0.3, 0.9), r.k(0.5, 1.0), r.k(0.3, 0.7), 0, 0, 0],
        );
        d.m(Source::ShiftShape, f, 0, 64);
        motion(r, d, f, 0);
    } else {
        let l = d.block(Kind::Lowpass, [r.k(0.3, 0.9), 0, 0, 0, 0, 0, 0]);
        d.m(Source::ShiftShape, l, 0, amount(r, 24, 48, true));
        motion(r, d, l, 0);
    }
    if r.chance(0.4) {
        let c = d.block(Kind::Crush, [r.k(0.3, 0.8), r.k(0.0, 0.5), 0, 0, 0, 0, 0]);
        d.knob("Crush", c, 2, 64);
    }
}

fn hybrid(r: &mut Rng, seed: u64, d: &mut Draft) {
    const SHAPERS: [Kind; 4] = [Kind::PhaseDist, Kind::Sync, Kind::Sync, Kind::PhaseDist];
    const SOURCES: [Kind; 5] = [
        Kind::Saw,
        Kind::Wavetable,
        Kind::Fm,
        Kind::Supersaw,
        Kind::Wavetable,
    ];
    const AFTER: [Kind; 4] = [Kind::Fold, Kind::Crush, Kind::Lowpass, Kind::Formant];
    if r.chance(0.6) {
        d.tables = waves::family(seed, 2 + r.below(5));
    }
    let names = ["Warp", "Edge", "Grit", "Color", "Drift", "Spin"];
    let mut knob = 0;
    if r.chance(0.6) {
        let b = d.block(r.pick(&SHAPERS), [r.k(0.0, 0.3), 0, 0, 0, 0, 0, 0]);
        d.m(Source::ShiftShape, b, 0, amount(r, 16, 48, false));
    }
    for i in 0..2 {
        let kind = r.pick(&SOURCES);
        if !d.fits(kind) {
            continue;
        }
        let level = if i == 0 { (0.6, 0.9) } else { (0.15, 0.45) };
        let lv = r.k(level.0, level.1);
        let b = match kind {
            Kind::Saw => d.block(kind, [lv, 0, 0, 0, 0, 0, 0]),
            Kind::Wavetable => d.block(kind, [r.k(0.0, 1.0), lv, 0, 0, 0, 0, 0]),
            Kind::Fm => d.block(
                kind,
                [r.k(0.0, 1.0), r.k(0.0, 0.5), r.k(0.0, 0.5), lv, 0, 0, 0],
            ),
            _ => d.block(kind, [r.k(0.0, 0.7), lv, 0, 0, 0, 0, 0]),
        };
        let (p, lvl) = match kind {
            Kind::Saw => (0, 0),
            Kind::Fm => (1, 3),
            _ => (0, 1),
        };
        if i == 0 {
            d.m(Source::Shape, b, p, amount(r, 24, 64, false));
        } else {
            d.knob(names[knob], b, lvl, 40);
            knob += 1;
        }
        motion(r, d, b, p);
    }
    for _ in 0..1 + r.below(2) {
        let kind = r.pick(&AFTER);
        if !d.fits(kind) {
            continue;
        }
        let b = d.block(
            kind,
            [
                r.k(0.0, 0.5),
                r.k(0.0, 0.7),
                r.k(0.4, 1.0),
                r.k(0.3, 0.7),
                0,
                0,
                0,
            ],
        );
        let p = if kind == Kind::Lowpass { 0 } else { 2 };
        d.knob(names[knob], b, p, if kind == Kind::Lowpass { -40 } else { 48 });
        knob += 1;
        if r.chance(0.5) {
            d.m(Source::Shape, b, 0, amount(r, 16, 48, true));
        }
        if r.chance(0.3) {
            d.m(Source::Key, b, 0, amount(r, 8, 24, true));
        }
    }
}

/// The level every made oscillator's loudest probe is set to.
const LEVEL: f32 = 0.6;

/// Finish a draft: globals, a name, the level set; `None` when it does
/// not validate.
fn finish(r: &mut Rng, seed: u64, family: Family, d: Draft) -> Option<Patch> {
    let mut p = Patch {
        name: name(seed),
        family,
        params: d.params,
        gain: 16,
        lfo_rate: r.k(0.1, 0.8),
        lfo_shape: r.below(3) as u8,
        attack: r.k(0.0, 0.3),
        decay: r.k(0.3, 0.8),
        sustain: r.k(0.0, 1.0),
        blocks: d.blocks,
        mods: d.mods,
        tables: d.tables,
    };
    p.normalize(LEVEL);
    p.validate().ok().map(|()| p)
}

/// An oscillator of `family` from `seed`: the same seed makes the same one.
pub fn generate(family: Family, seed: u64) -> Patch {
    for attempt in 0u64.. {
        let s = seed.wrapping_add(attempt.wrapping_mul(0x9E37_79B9));
        let mut r = Rng::new(s ^ ((family as u64) << 56));
        let mut d = Draft::new();
        match family {
            Family::Wavetable => wavetable(&mut r, s, &mut d),
            Family::Fm => fm(&mut r, &mut d),
            Family::Fold => fold(&mut r, &mut d),
            Family::Formant => formant(&mut r, &mut d),
            Family::Noise => noise(&mut r, &mut d),
            Family::Hybrid => hybrid(&mut r, s, &mut d),
        }
        if family != Family::Noise {
            extras(&mut r, &mut d);
        }
        if let Some(p) = finish(&mut r, seed, family, d) {
            return p;
        }
    }
    unreachable!("an endless range")
}

fn nudge(r: &mut Rng, v: u8, amount: f32) -> u8 {
    let d = (r.f() * 2.0 - 1.0) * amount * 255.0;
    (f32::from(v) + d).round().clamp(0.0, 255.0) as u8
}

/// A variation of `p` from `seed`: its constants, mods and LFO moved by up
/// to `amount` (0 to 1) of their range, its tables blended toward fresh
/// ones by as much, under a new name. `p` itself when nothing moved far
/// enough to stay valid.
pub fn vary(p: &Patch, seed: u64, amount: f32) -> Patch {
    let amount = amount.clamp(0.0, 1.0);
    for attempt in 0u64..8 {
        let mut r = Rng::new(seed ^ (attempt << 48) ^ 0x7661_7279);
        let mut v = p.clone();
        for b in &mut v.blocks {
            for k in &mut b.k {
                *k = nudge(&mut r, *k, amount);
            }
        }
        for m in &mut v.mods {
            let d = (r.f() * 2.0 - 1.0) * amount * 32.0;
            m.amount = (f32::from(m.amount) + d).round().clamp(-64.0, 64.0) as i8;
        }
        v.lfo_rate = nudge(&mut r, v.lfo_rate, amount);
        v.decay = nudge(&mut r, v.decay, amount);
        v.sustain = nudge(&mut r, v.sustain, amount);
        let fresh = waves::family(seed, v.tables.len());
        for (t, f) in v.tables.iter_mut().zip(fresh) {
            for (s, x) in t.iter_mut().zip(f) {
                *s = (f32::from(*s) + (f32::from(x) - f32::from(*s)) * amount * 0.5) as i16;
            }
        }
        v.name = name(seed ^ attempt);
        v.normalize(LEVEL);
        if v.validate().is_ok() {
            return v;
        }
    }
    p.clone()
}

/// A child of `a` and `b` from `seed`: `a`'s wiring and parameters, each
/// block's constants from either parent where `b` has that block too,
/// tables mixed, an extra shaper of `b`'s when it fits, a name from both.
/// `a` when no child validates.
pub fn breed(a: &Patch, b: &Patch, seed: u64) -> Patch {
    for attempt in 0u64..8 {
        let mut r = Rng::new(seed ^ (attempt << 48) ^ 0x6272_6564);
        let mut c = a.clone();
        for blk in &mut c.blocks {
            if let Some(o) = b.blocks.iter().find(|o| o.kind == blk.kind) {
                for (k, ok) in blk.k.iter_mut().zip(o.k) {
                    if r.chance(0.5) {
                        *k = ok;
                    }
                }
            }
        }
        for (i, t) in c.tables.iter_mut().enumerate() {
            if let Some(o) = b.tables.get(i)
                && r.chance(0.5)
            {
                *t = *o;
            }
        }
        if c.tables.len() < b.tables.len() && r.chance(0.5) {
            let room = super::MAX_TABLES - c.tables.len();
            c.tables
                .extend(b.tables.iter().skip(c.tables.len()).take(room).copied());
        }
        let extra = b
            .blocks
            .iter()
            .filter(|o| !o.kind.is_source() && !c.blocks.iter().any(|x| x.kind == o.kind))
            .copied()
            .find(|o| c.cost() + o.kind.cost() <= BUDGET && c.blocks.len() < super::MAX_BLOCKS);
        if let Some(o) = extra
            && r.chance(0.6)
        {
            c.blocks.push(o);
        }
        if r.chance(0.5) {
            (c.lfo_rate, c.lfo_shape) = (b.lfo_rate, b.lfo_shape);
        }
        if r.chance(0.5) {
            (c.attack, c.decay, c.sustain) = (b.attack, b.decay, b.sustain);
        }
        if a.family != b.family {
            c.family = Family::Hybrid;
        }
        // The start of `a`'s name and the end of `b`'s, cut where the seed
        // says: each child of a pair is named apart.
        let ha = (2 + r.below(a.name.len().saturating_sub(2).max(1))).min(a.name.len());
        let hb = (1 + r.below(b.name.len().saturating_sub(1).max(1))).min(b.name.len());
        c.name = format!("{}{}", &a.name[..ha], &b.name[hb..]);
        c.name.truncate(super::NAME_LEN);
        if c.name.len() < 3 || c.name == a.name || c.name == b.name {
            c.name = name(seed ^ attempt);
        }
        c.normalize(LEVEL);
        if c.validate().is_ok() {
            return c;
        }
    }
    a.clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_seed_makes_one_oscillator_and_families_differ() {
        for f in Family::ALL {
            let p = generate(f, 42);
            assert_eq!(p, generate(f, 42));
            assert_ne!(p, generate(f, 43));
            assert_eq!(p.family, f);
            assert_eq!(Family::from_id(f.id()), Some(f));
            assert_eq!(p.validate(), Ok(()));
            assert!(!p.params.is_empty(), "{f:?} has knobs");
        }
        let names: std::collections::BTreeSet<String> = (0..50).map(name).collect();
        assert!(names.len() >= 48, "names rarely repeat");
        assert!(
            names
                .iter()
                .all(|n| n.len() <= 13 && n.is_ascii() && !n.is_empty())
        );
    }

    #[test]
    fn variations_and_children_stay_valid_and_new() {
        let a = generate(Family::Wavetable, 1);
        let b = generate(Family::Fm, 2);
        let mut names = std::collections::BTreeSet::new();
        for seed in 0..20 {
            let v = vary(&a, seed, 0.35);
            assert_eq!(v.validate(), Ok(()));
            assert_ne!(v, a);
            let c = breed(&a, &b, seed);
            assert_eq!(c.validate(), Ok(()));
            assert_ne!(c.name, a.name);
            names.insert(c.name);
        }
        assert!(names.len() >= 10, "children named apart: {names:?}");
        assert_eq!(breed(&a, &b, 5).family, Family::Hybrid);
    }
}
