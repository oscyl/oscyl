//! The Generate page's model, shared by every app: hands of six sounds
//! dealt from a driver's `generate`, `vary` and `breed`
//! (`oscyl_core::generate::hand`), the last few kept for Back and
//! Forward, the sound selected in each, and the one kept from it. Apps
//! draw the hands and play the selected sound.
//!
//! A dealt sound carries a recipe code ([`code`]): its character, the four
//! dials and its seed, short enough to type. The same code makes the same
//! sound again anywhere ([`parse_code`], [`Hands::deal_code`]).

use oscyl_core::generate::{hand, hand_seeded};
use oscyl_driver::{Driver, DriverError, Macros, Recipe};

use crate::i18n::tr;
use crate::state::Program;

/// Sounds in a hand.
pub const HAND: usize = 6;

/// Hands kept for Back: the current one and three before it.
pub const HISTORY: usize = 4;

/// How far Vary moves: Subtle, Clear, Wild.
pub const AMOUNTS: [f32; 3] = [0.15, 0.35, 0.7];

/// One hand of sounds.
#[derive(Debug, Clone, PartialEq)]
pub struct Hand {
    pub programs: Vec<Program>,
    /// Index into the driver's characters.
    pub character: usize,
    /// Where it came from, for its legend (translated).
    pub origin: String,
    pub selected: usize,
    /// The sound kept from this hand, and where it went (a slot, a library).
    pub kept: Option<(usize, String)>,
    /// Each sound's recipe code, for the ones a recipe alone makes again:
    /// dealt sounds. Variations, children, morphs and sounds dealt onto a
    /// base program have none.
    pub codes: Vec<Option<String>>,
}

/// Dealt seeds are 32 bits, so a code is 13 characters after the
/// character's name.
const SEED_MASK: u64 = 0xFFFF_FFFF;

/// Crockford's base 32: no I, L, O or U to misread.
const DIGITS: &[u8; 32] = b"0123456789abcdefghjkmnpqrstvwxyz";

/// A dial as a code holds it: 0 to 100.
fn step(dial: f32) -> u8 {
    (dial.clamp(0.0, 1.0) * 100.0).round() as u8
}

/// `macros` as a code holds them: each dial at one of its 101 steps.
pub fn stepped(m: Macros) -> Macros {
    let s = |v: f32| f32::from(step(v)) / 100.0;
    Macros {
        bright: s(m.bright),
        movement: s(m.movement),
        grit: s(m.grit),
        space: s(m.space),
    }
}

/// The check digit of a code's character id and digits.
// ponytail: a 5-bit hash, so one typo in 32 slips through (and then makes
// another valid sound, nothing worse); a real check code if codes ever
// carry more than a sound.
fn check(id: &str, digits: &[u8]) -> u8 {
    let h = id
        .bytes()
        .chain(digits.iter().copied())
        .fold(0xCBF2_9CE4_8422_2325u64, |h, b| {
            (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01B3)
        });
    ((h >> 32) % 32) as u8
}

/// The recipe code of `recipe` for `d`: the character's id, a dash, then
/// base 32 digits (most significant first, twelve or more) of the seed
/// above the four dials (7 bits each: bright, movement, grit, space, at 0
/// to 100), and a check digit. Lowercase, no I, L, O or U. It names no synth: the same
/// code deals that character on any synth that has it. `None` when the
/// recipe keeps pages of a base program (the code could not say which
/// program) or names no character.
pub fn code(d: &dyn Driver, recipe: &Recipe) -> Option<String> {
    let id = d.characters().get(recipe.character)?.id;
    if !recipe.keep.is_empty() {
        return None;
    }
    let m = recipe.macros;
    let dials = [m.bright, m.movement, m.grit, m.space]
        .iter()
        .fold(0u128, |n, &v| (n << 7) | u128::from(step(v)));
    let mut n = (u128::from(recipe.seed) << 28) | dials;
    let mut digits = vec![];
    // Twelve digits hold the dials and a 32-bit seed: a dealt sound's.
    while n > 0 || digits.len() < 12 {
        digits.push((n % 32) as u8);
        n /= 32;
    }
    digits.reverse();
    digits.push(check(id, &digits));
    let text: String = digits
        .iter()
        .map(|&v| DIGITS[usize::from(v)] as char)
        .collect();
    Some(format!("{id}-{text}"))
}

/// The recipe a code names, for `d`. Reads either case, spaces around it,
/// and the letters people type for digits (I and L for 1, O for 0). An
/// error says what is wrong, translated.
pub fn parse_code(d: &dyn Driver, code: &str) -> Result<Recipe, String> {
    let bad = || tr("That is not a recipe code");
    let code = code.trim().to_lowercase();
    let (id, text) = code.rsplit_once('-').ok_or_else(bad)?;
    let character = d
        .characters()
        .iter()
        .position(|c| c.id == id)
        .ok_or_else(|| tr("This synth has no “{name}” sounds").replace("{name}", id))?;
    let mut digits = text
        .bytes()
        .map(|b| {
            let b = match b {
                b'i' | b'l' => b'1',
                b'o' => b'0',
                b => b,
            };
            DIGITS.iter().position(|&x| x == b).map(|v| v as u8)
        })
        .collect::<Option<Vec<u8>>>()
        .ok_or_else(bad)?;
    // Twelve digits at least, the check digit, and no more than a 64-bit
    // seed above the dials takes.
    let sum = digits.pop().ok_or_else(bad)?;
    if !(12..=19).contains(&digits.len()) {
        return Err(bad());
    }
    if sum != check(id, &digits) {
        return Err(tr("That code has a typing mistake"));
    }
    let n = digits.iter().fold(0u128, |n, &v| (n << 5) | u128::from(v));
    let dial = |k: u32| ((n >> (7 * k)) & 0x7F) as u8;
    let seed = u64::try_from(n >> 28).map_err(|_| bad())?;
    if (0..4).any(|k| dial(k) > 100) {
        return Err(bad());
    }
    let m = |k: u32| f32::from(dial(k)) / 100.0;
    Ok(Recipe {
        character,
        macros: Macros {
            bright: m(3),
            movement: m(2),
            grit: m(1),
            space: m(0),
        },
        seed,
        keep: vec![],
    })
}

/// The hands dealt so far for one synth, and where the page is among them.
#[derive(Debug, Clone, PartialEq)]
pub struct Hands {
    hands: Vec<Hand>,
    at: usize,
    seed: u64,
}

/// SplitMix64, for what a surprise picks.
struct Pick(u64);

impl Pick {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
}

impl Hands {
    /// Dealing from `seed`: the same seed and choices deal the same hands.
    pub fn new(seed: u64) -> Hands {
        Hands {
            hands: Vec::new(),
            at: 0,
            seed,
        }
    }

    /// Start dealing from `seed` again (screenshots use a fixed one).
    pub fn set_seed(&mut self, seed: u64) {
        self.seed = seed;
    }

    /// The next hand's seed: a step of SplitMix64 on the hands' own. (The
    /// state itself would not do: a hand steps its seed the same way, so
    /// two hands in a row would share five of their six sounds.)
    fn next_seed(&mut self) -> u64 {
        let mut pick = Pick(self.seed);
        let s = pick.next();
        self.seed = pick.0;
        s
    }

    /// The hand shown.
    pub fn current(&self) -> Option<&Hand> {
        self.hands.get(self.at)
    }

    /// The hand shown's position (from 0) and how many hands there are.
    pub fn position(&self) -> (usize, usize) {
        (self.at, self.hands.len())
    }

    /// The sound selected in the hand shown, with its character.
    pub fn selected(&self) -> Option<(Program, usize)> {
        let h = self.current()?;
        Some((h.programs.get(h.selected)?.clone(), h.character))
    }

    /// Forget every hand (another synth).
    pub fn clear(&mut self) {
        self.hands.clear();
        self.at = 0;
    }

    /// Six new sounds of `character`, shaped by `macros` (each dial at the
    /// nearest of its 101 steps, as a code holds it); the fields of the
    /// pages in `keep` come from `base` as they are. False when the driver
    /// deals nothing.
    pub fn deal(
        &mut self,
        d: &dyn Driver,
        character: usize,
        macros: Macros,
        keep: &[usize],
        base: Option<&[u8]>,
    ) -> bool {
        let seed = self.next_seed();
        self.deal_from(d, character, macros, keep, base, seed, None)
    }

    /// [`Hands::deal`] from `seed`, the first sound from seed `first` when
    /// given.
    #[allow(clippy::too_many_arguments)]
    fn deal_from(
        &mut self,
        d: &dyn Driver,
        character: usize,
        macros: Macros,
        keep: &[usize],
        base: Option<&[u8]>,
        seed: u64,
        first: Option<u64>,
    ) -> bool {
        let Some(title) = d.characters().get(character).map(|c| c.title) else {
            return false;
        };
        let recipe = |seed| Recipe {
            character,
            macros: stepped(macros),
            seed,
            keep: keep.to_vec(),
        };
        let dealt = hand_seeded(d, character, seed, SEED_MASK, first, HAND, |seed| {
            d.generate(&recipe(seed), base)
        });
        let Ok(dealt) = dealt else {
            return false;
        };
        let codes = dealt
            .iter()
            .map(|(_, seed)| {
                let recipe = Recipe {
                    keep: vec![],
                    ..recipe((*seed)?)
                };
                code(d, &recipe).filter(|_| base.is_none())
            })
            .collect();
        let programs = dealt.into_iter().map(|(p, _)| p).collect();
        self.push_coded(programs, codes, character, tr(title))
    }

    /// A random character and dials (at steps of five), dealt; what it
    /// chose, for the page to show. `None` when the driver deals nothing.
    pub fn surprise(&mut self, d: &dyn Driver) -> Option<(usize, Macros)> {
        let n = d.characters().len() as u64;
        if n == 0 {
            return None;
        }
        let mut pick = Pick(self.next_seed());
        let character = (pick.next() % n) as usize;
        let mut dial = || (pick.next() % 21) as f32 * 0.05;
        let macros = stepped(Macros {
            bright: dial(),
            movement: dial(),
            grit: dial(),
            space: dial(),
        });
        self.deal(d, character, macros, &[], None)
            .then_some((character, macros))
    }

    /// The hand a code names: its sound first, then five more of the same
    /// character and dials. What the code chose, for the page to show, or
    /// what is wrong with it.
    pub fn deal_code(&mut self, d: &dyn Driver, code: &str) -> Result<(usize, Macros), String> {
        let r = parse_code(d, code)?;
        let seed = self.next_seed();
        if self.deal_from(d, r.character, r.macros, &[], None, seed, Some(r.seed)) {
            Ok((r.character, r.macros))
        } else {
            Err(tr("This synth deals no sounds"))
        }
    }

    /// The selected sound's recipe code, when a recipe alone makes it
    /// again (see [`Hand::codes`]).
    pub fn code(&self) -> Option<String> {
        let h = self.current()?;
        h.codes.get(h.selected)?.clone()
    }

    /// Six variations of the selected sound, by amount `amount` (an index
    /// into [`AMOUNTS`]).
    pub fn vary(&mut self, d: &dyn Driver, amount: usize) -> bool {
        let Some((p, c)) = self.selected() else {
            return false;
        };
        let amount = AMOUNTS[amount.min(AMOUNTS.len() - 1)];
        let seed = self.next_seed();
        let programs = hand(d, c, seed, HAND, |s| d.vary(&p, amount, s));
        let name = d.name(&p).unwrap_or_default();
        self.push(
            programs,
            c,
            tr("Variations of “{name}”").replace("{name}", &name),
        )
    }

    /// The other sounds of the hand shown, the ones the selected one can be
    /// bred with, in order.
    pub fn others(&self) -> Vec<usize> {
        self.current().map_or_else(Vec::new, |h| {
            (0..h.programs.len()).filter(|&i| i != h.selected).collect()
        })
    }

    /// Six children of the selected sound and sound `other` of the hand.
    pub fn breed(&mut self, d: &dyn Driver, other: usize) -> bool {
        let Some((a, c)) = self.selected() else {
            return false;
        };
        let Some(b) = self
            .current()
            .filter(|h| other != h.selected)
            .and_then(|h| h.programs.get(other).cloned())
        else {
            return false;
        };
        let seed = self.next_seed();
        // Each child takes its own share of each parent.
        let programs = hand(d, c, seed, HAND, |s| {
            let mix = 0.2 + 0.6 * (s % 1000) as f32 / 1000.0;
            d.breed(&a, &b, mix, s)
        });
        let name = |p: &[u8]| d.name(p).unwrap_or_default();
        self.push(
            programs,
            c,
            tr("Children of “{a}” and “{b}”")
                .replace("{a}", &name(&a))
                .replace("{b}", &name(&b)),
        )
    }

    /// Six steps from the selected sound to sound `other` of the hand, in
    /// order: neither end is among them (both stay a step Back), so the
    /// first is a seventh of the way and the last six sevenths.
    pub fn morph(&mut self, d: &dyn Driver, other: usize) -> bool {
        let Some((a, c)) = self.selected() else {
            return false;
        };
        let Some(b) = self
            .current()
            .filter(|h| other != h.selected)
            .and_then(|h| h.programs.get(other).cloned())
        else {
            return false;
        };
        let seed = self.next_seed();
        let name = |p: &[u8]| d.name(p).unwrap_or_default();
        let mut names: Vec<String> = vec![];
        let programs = (1..=HAND)
            .map(|k| {
                let mut p = d.morph(&a, &b, k as f32 / (HAND + 1) as f32, seed)?;
                // Every step would carry the first sound's name.
                let fresh = (0..64)
                    .map(|i| d.generated_name(c, seed.wrapping_add(i * 64 + k as u64)))
                    .find(|n| !names.contains(n));
                if let Some(fresh) = fresh {
                    d.set_name(&mut p, &fresh)?;
                    names.push(fresh);
                }
                Ok(p)
            })
            .collect();
        self.push(
            programs,
            c,
            tr("From “{a}” to “{b}”")
                .replace("{a}", &name(&a))
                .replace("{b}", &name(&b)),
        )
    }

    /// Show a new hand of sounds with no codes (see [`Hands::push_coded`]).
    fn push(
        &mut self,
        programs: Result<Vec<Program>, DriverError>,
        character: usize,
        origin: String,
    ) -> bool {
        let Ok(programs) = programs else {
            return false;
        };
        let codes = vec![None; programs.len()];
        self.push_coded(programs, codes, character, origin)
    }

    /// Show a new hand after the current one, dropping any hands ahead of
    /// it and the oldest beyond [`HISTORY`].
    fn push_coded(
        &mut self,
        programs: Vec<Program>,
        codes: Vec<Option<String>>,
        character: usize,
        origin: String,
    ) -> bool {
        self.hands.truncate(self.at + 1);
        self.hands.push(Hand {
            programs,
            character,
            origin,
            selected: 0,
            kept: None,
            codes,
        });
        if self.hands.len() > HISTORY {
            self.hands.remove(0);
        }
        self.at = self.hands.len() - 1;
        true
    }

    /// Back (-1) or Forward (+1); false at either end.
    pub fn step(&mut self, delta: isize) -> bool {
        let at = self.at as isize + delta;
        if self.hands.is_empty() || at < 0 || at >= self.hands.len() as isize {
            return false;
        }
        self.at = at as usize;
        true
    }

    /// Select sound `i` of the hand shown.
    pub fn select(&mut self, i: usize) {
        if let Some(h) = self.hands.get_mut(self.at)
            && i < h.programs.len()
        {
            h.selected = i;
        }
    }

    /// The selected sound was kept at `place` (a slot, a library).
    pub fn mark_kept(&mut self, place: String) {
        if let Some(h) = self.hands.get_mut(self.at) {
            h.kept = Some((h.selected, place));
        }
    }
}

/// The history's words for a roll of the stage titled `stage` (translated).
pub fn rolled_label(stage: &str) -> String {
    tr("Rolled {stage}").replace("{stage}", stage)
}

/// Program `p` with page `page` rolled: the driver varies the whole
/// program fully from `seed` (free fields only), and only the fields of
/// that page's sections are taken into `p`. A value the program refuses in
/// its own context stays as it was. Same seed, same roll.
pub fn roll_page(d: &dyn Driver, p: &[u8], page: usize, seed: u64) -> Result<Vec<u8>, DriverError> {
    let curation = d.curation();
    let fields: Vec<usize> = curation
        .pages
        .get(page)
        .ok_or_else(|| DriverError("no such page".into()))?
        .sections
        .iter()
        .flat_map(|s| s.fields.iter().copied())
        .collect();
    // A full variation can leave a page as it was (it picks which fields
    // move): further seeds from this one are tried until the page changes.
    // ponytail: 16 tries; a page with no free fields comes back unchanged.
    let mut out = p.to_vec();
    for k in 0..16u64 {
        let varied = d.vary(
            p,
            1.0,
            seed.wrapping_add(k.wrapping_mul(0x9E37_79B9_7F4A_7C15)),
        )?;
        out = p.to_vec();
        for &f in &fields {
            if let Ok(v) = d.get(&varied, f) {
                let _ = d.set(&mut out, f, v);
            }
        }
        if out != p {
            break;
        }
    }
    d.check_program(&out)?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn xd() -> &'static dyn Driver {
        oscyl_midi::driver("korg-minilogue-xd").unwrap()
    }

    /// Up to 20 demo programs of driver `id`.
    fn demos(id: &str) -> (&'static dyn Driver, Vec<Vec<u8>>) {
        let d = oscyl_midi::driver(id).unwrap();
        let doc = crate::state::Doc::from_data(d, d.demo_library(), None, "Demo").unwrap();
        let programs = (0..doc.len().min(20))
            .filter_map(|i| doc.work(i).map(<[u8]>::to_vec))
            .collect();
        (d, programs)
    }

    /// The spike for dice: every page of 20 demo programs of each driver,
    /// rolled, is a valid program with every field in range.
    #[test]
    fn rolled_pages_are_valid_programs() {
        for id in [
            "korg-minilogue-xd",
            "korg-prologue",
            "sequential-prophet-rev2",
        ] {
            let (d, programs) = demos(id);
            let pages = d.curation().pages.len();
            for (n, p) in programs.iter().enumerate() {
                for page in 0..pages {
                    let rolled = roll_page(d, p, page, 1000 + n as u64)
                        .unwrap_or_else(|e| panic!("{id} {n} page {page}: {e}"));
                    assert!(
                        d.check_program(&rolled).is_ok(),
                        "{id} {n} page {page} invalid"
                    );
                    for (f, info) in d.fields().iter().enumerate() {
                        let v = d.get(&rolled, f).unwrap();
                        assert!(
                            (info.min..=info.max).contains(&v),
                            "{id} {n} page {page}: {} = {v}",
                            info.name
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn roll_changes_only_that_page() {
        let (d, programs) = demos("korg-minilogue-xd");
        let c = d.curation();
        for (page, info) in c.pages.iter().enumerate() {
            let on_page: Vec<usize> = info
                .sections
                .iter()
                .flat_map(|s| s.fields.iter().copied())
                .collect();
            let p = &programs[0];
            let rolled = roll_page(d, p, page, 42).unwrap();
            for f in (0..d.fields().len()).filter(|f| !on_page.contains(f)) {
                assert_eq!(
                    d.get(&rolled, f),
                    d.get(p, f),
                    "page {page} left {} alone",
                    d.fields()[f].name
                );
            }
            assert_eq!(d.name(&rolled).ok(), d.name(p).ok());
        }
        // A page with free fields changes on every seed: the dice are never a dud.
        for page in 0..c.pages.len() {
            let moves = (0..40u64)
                .filter(|&seed| roll_page(d, &programs[0], page, seed).unwrap() != programs[0])
                .count();
            assert!(
                moves == 0 || moves == 40,
                "page {page} moved on {moves} of 40 seeds"
            );
        }
    }

    #[test]
    fn codes_make_the_same_sound_again() {
        for id in [
            "korg-minilogue-xd",
            "korg-prologue",
            "sequential-prophet-rev2",
        ] {
            let d = oscyl_midi::driver(id).unwrap();
            let mut h = Hands::new(11);
            let macros = Macros {
                bright: 0.2,
                movement: 0.73,
                grit: 1.0,
                space: 0.0,
            };
            for character in 0..d.characters().len() {
                assert!(h.deal(d, character, macros, &[], None));
                for i in 0..HAND {
                    h.select(i);
                    let code = h.code().expect("a dealt sound has a code");
                    let prefix = format!("{}-", d.characters()[character].id);
                    assert!(code.starts_with(&prefix), "{code}");
                    assert_eq!(code.len(), prefix.len() + 13, "{code}");
                    let r = parse_code(d, &code).unwrap();
                    assert_eq!((r.character, r.macros), (character, macros));
                    assert_eq!(super::code(d, &r).as_deref(), Some(&*code));
                    // The same sound, name and all.
                    let (p, _) = h.selected().unwrap();
                    assert_eq!(d.generate(&r, None).unwrap(), p, "{id} {code}");
                }
            }
        }
    }

    #[test]
    fn codes_are_typed_by_people() {
        let d = xd();
        let r = Recipe {
            character: 6,
            macros: Macros {
                bright: 0.0,
                movement: 0.5,
                grit: 1.0,
                space: 0.25,
            },
            seed: u64::MAX,
            keep: vec![],
        };
        let code = code(d, &r).unwrap();
        assert_eq!(parse_code(d, &code).unwrap(), r, "a 64-bit seed");
        assert_eq!(
            parse_code(d, &format!("  {} ", code.to_uppercase())).unwrap(),
            r
        );
        let small = Recipe {
            seed: 0,
            ..r.clone()
        };
        let short = super::code(d, &small).unwrap();
        assert_eq!(short.len(), "pluck-".len() + 13, "{short}");
        assert_eq!(parse_code(d, &short).unwrap(), small);
        // Letters typed for digits.
        if short.contains('1') || short.contains('0') {
            let typed = short.replace('1', "l").replace('0', "O");
            assert_eq!(parse_code(d, &typed).unwrap(), small);
        }
        // A wrong digit is caught (31 times in 32; this one is).
        let (head, last) = short.split_at(short.len() - 1);
        let other = if last == "7" { "8" } else { "7" };
        assert!(parse_code(d, &format!("{head}{other}")).is_err());
        for bad in [
            "",
            "pluck",
            "pluck-",
            "nope-0000000000000",
            "pluck-zzzzzzzzzzzzzzzzzzzzzz",
            "pluck-u000000000000",
        ] {
            assert!(parse_code(d, bad).is_err(), "{bad:?}");
        }
        // Kept pages cannot be named by a code.
        let kept = Recipe { keep: vec![0], ..r };
        assert_eq!(super::code(d, &kept), None);
    }

    #[test]
    fn a_code_deals_its_sound_first() {
        let d = xd();
        let mut h = Hands::new(3);
        assert!(h.deal(d, 4, Macros::default(), &[], None));
        h.select(4);
        let (code, (sound, _)) = (h.code().unwrap(), h.selected().unwrap());
        let mut other = Hands::new(99);
        assert_eq!(other.deal_code(d, &code), Ok((4, Macros::default())));
        assert_eq!(other.selected().unwrap().0, sound);
        assert_eq!(other.code().as_deref(), Some(&*code));
        assert_eq!(other.current().unwrap().programs.len(), HAND);
        assert!(other.deal_code(d, "wobble-nonsense").is_err());
        assert_eq!(other.position(), (0, 1), "a bad code deals nothing");
    }

    #[test]
    fn a_morph_walks_from_one_sound_to_the_other() {
        let d = xd();
        let mut h = Hands::new(5);
        assert!(h.deal(d, 5, Macros::default(), &[], None));
        let hand = h.current().unwrap().clone();
        let (a, b) = (&hand.programs[0], &hand.programs[3]);
        assert!(!h.morph(d, 0), "not with itself");
        assert!(h.morph(d, 3));
        let m = h.current().unwrap();
        assert_eq!(m.programs.len(), HAND);
        assert!(m.origin.starts_with("From"));
        assert!(m.codes.iter().all(Option::is_none));
        assert!(h.code().is_none());
        let cutoff = d.fields().iter().position(|f| f.name == "Cutoff").unwrap();
        let line: Vec<u16> = std::iter::once(a)
            .chain(&m.programs)
            .chain(std::iter::once(b))
            .map(|p| d.get(p, cutoff).unwrap())
            .collect();
        assert!(
            line.windows(2).all(|w| w[0] <= w[1]) || line.windows(2).all(|w| w[0] >= w[1]),
            "{line:?}"
        );
        let mut names: Vec<String> = m.programs.iter().map(|p| d.name(p).unwrap()).collect();
        names.dedup();
        assert_eq!(names.len(), HAND, "each step has its own name");
        // Variations and children have no code either.
        assert!(h.vary(d, 1) && h.code().is_none());
    }

    #[test]
    fn a_surprise_says_what_it_dealt() {
        let d = xd();
        let (mut a, mut b) = (Hands::new(21), Hands::new(21));
        let (character, macros) = a.surprise(d).unwrap();
        assert_eq!(b.surprise(d), Some((character, macros)), "same seed");
        assert_eq!(a.current(), b.current());
        assert_eq!(a.current().unwrap().character, character);
        let r = parse_code(d, &a.code().unwrap()).unwrap();
        assert_eq!((r.character, r.macros), (character, macros));
        // Over a few, more than one character comes up.
        let seen: std::collections::BTreeSet<usize> = (0..12)
            .filter_map(|_| a.surprise(d))
            .map(|(c, _)| c)
            .collect();
        assert!(seen.len() > 3, "{seen:?}");
    }

    #[test]
    fn hands_deal_the_same_for_a_seed_and_walk_back() {
        let d = xd();
        let (mut a, mut b) = (Hands::new(7), Hands::new(7));
        assert!(a.deal(d, 0, Macros::default(), &[], None));
        assert!(b.deal(d, 0, Macros::default(), &[], None));
        assert_eq!(a.current(), b.current(), "same seed, same sounds");
        // Dealing again deals six other sounds.
        let first = a.current().unwrap().programs.clone();
        assert!(b.deal(d, 0, Macros::default(), &[], None));
        let again = &b.current().unwrap().programs;
        assert!(first.iter().all(|p| !again.contains(p)));
        assert_eq!(a.current().unwrap().programs.len(), HAND);
        a.select(2);
        assert!(a.vary(d, 1));
        assert!(a.current().unwrap().origin.starts_with("Variations of"));
        let other = a.others()[0];
        assert!(a.breed(d, other));
        assert_eq!(a.position(), (2, 3));
        assert!(a.step(-1) && a.step(-1) && !a.step(-1));
        a.mark_kept("001".into());
        assert_eq!(
            a.current().unwrap().kept,
            Some((2, "001".into())),
            "the first hand still has sound 3 selected"
        );
        // A new hand from an earlier one drops those ahead.
        assert!(a.deal(d, 1, Macros::default(), &[], None));
        assert_eq!(a.position(), (1, 2));
        for _ in 0..HISTORY {
            a.deal(d, 1, Macros::default(), &[], None);
        }
        assert_eq!(a.position().1, HISTORY);
    }
}
