//! The Generate page's model, shared by every app: hands of six sounds
//! dealt from a driver's `generate`, `vary` and `breed`
//! (`oscyl_core::generate::hand`), the last few kept for Back and
//! Forward, the sound selected in each, and the one kept from it. Apps
//! draw the hands and play the selected sound.

use oscyl_core::generate::hand;
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
}

/// The hands dealt so far for one synth, and where the page is among them.
#[derive(Debug, Clone, PartialEq)]
pub struct Hands {
    hands: Vec<Hand>,
    at: usize,
    seed: u64,
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

    fn next_seed(&mut self) -> u64 {
        let s = self.seed;
        self.seed = s.wrapping_add(0x9E37_79B9_7F4A_7C15);
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

    /// Six new sounds of `character`, shaped by `macros`; the fields of the
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
        let Some(title) = d.characters().get(character).map(|c| c.title) else {
            return false;
        };
        let seed = self.next_seed();
        let programs = hand(d, character, seed, HAND, |seed| {
            let recipe = Recipe {
                character,
                macros,
                seed,
                keep: keep.to_vec(),
            };
            d.generate(&recipe, base)
        });
        self.push(programs, character, tr(title))
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

    /// Show a new hand after the current one, dropping any hands ahead of
    /// it and the oldest beyond [`HISTORY`]. A dealer failing (a driver
    /// error) deals nothing.
    fn push(
        &mut self,
        programs: Result<Vec<Program>, DriverError>,
        character: usize,
        origin: String,
    ) -> bool {
        let Ok(programs) = programs else {
            return false;
        };
        self.hands.truncate(self.at + 1);
        self.hands.push(Hand {
            programs,
            character,
            origin,
            selected: 0,
            kept: None,
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
    fn hands_deal_the_same_for_a_seed_and_walk_back() {
        let d = xd();
        let (mut a, mut b) = (Hands::new(7), Hands::new(7));
        assert!(a.deal(d, 0, Macros::default(), &[], None));
        assert!(b.deal(d, 0, Macros::default(), &[], None));
        assert_eq!(a.current(), b.current(), "same seed, same sounds");
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
