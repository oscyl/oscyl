//! New oscillators for Generate, shared by every app: hands of six made by
//! Oscyl's oscillator unit (`oscyl_logue::osc`), varied and bred like
//! sounds. Keeping one puts it on the shelf ([`crate::shelf`]) and gives
//! what to load into the synth's first free oscillator slot
//! ([`crate::synth::Cmd::LoadUnit`]); once loaded, the synth lists it and
//! it joins Generate's palette ([`crate::palette`]) with its parameters.

use oscyl_driver::Driver;
use oscyl_logue::osc::{self, Family, Patch};
use oscyl_midi::logue::platform;

use crate::generator::HAND;
use crate::i18n::tr;
use crate::shelf::Shelf;
use crate::units::UnitFile;

/// A family's name, for people.
pub fn family_title(f: Family) -> String {
    match f {
        Family::Wavetable => tr("Wavetable"),
        Family::Fm => tr("FM"),
        Family::Fold => tr("Fold"),
        Family::Formant => tr("Formant"),
        Family::Noise => tr("Noise"),
        Family::Hybrid => tr("Hybrid"),
    }
}

/// SplitMix64: the next seed.
fn next(seed: &mut u64) -> u64 {
    *seed = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *seed;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// The hand of oscillators shown, and the one selected.
#[derive(Debug, Clone)]
pub struct Oscillators {
    hand: Vec<Patch>,
    selected: usize,
    seed: u64,
}

impl Oscillators {
    /// No hand yet; the hands that follow come from `seed`.
    pub fn new(seed: u64) -> Oscillators {
        Oscillators {
            hand: vec![],
            selected: 0,
            seed,
        }
    }

    pub fn hand(&self) -> &[Patch] {
        &self.hand
    }

    pub fn selected(&self) -> Option<&Patch> {
        self.hand.get(self.selected)
    }

    pub fn selected_index(&self) -> usize {
        self.selected
    }

    pub fn select(&mut self, i: usize) {
        if i < self.hand.len() {
            self.selected = i;
        }
    }

    /// Six oscillators, each with a name of its own, made by `make` from
    /// fresh seeds; the first selected.
    fn fill(&mut self, mut make: impl FnMut(u64) -> Patch) {
        let mut hand: Vec<Patch> = vec![];
        while hand.len() < HAND {
            let p = make(next(&mut self.seed));
            if !hand.iter().any(|h| h.name == p.name || *h == p) {
                hand.push(p);
            }
        }
        self.hand = hand;
        self.selected = 0;
    }

    /// Six new oscillators of `family`, or of every family in turn.
    pub fn deal(&mut self, family: Option<Family>) {
        let mut turn = 0;
        self.fill(|seed| {
            let f = family.unwrap_or(Family::ALL[turn % Family::ALL.len()]);
            turn += 1;
            osc::generate(f, seed)
        });
    }

    /// Six variations of the selected oscillator, moved by `amount` (0 to
    /// 1; see [`crate::generator::AMOUNTS`]). False without a hand.
    pub fn vary(&mut self, amount: f32) -> bool {
        let Some(p) = self.selected().cloned() else {
            return false;
        };
        self.fill(|seed| osc::vary(&p, seed, amount));
        true
    }

    /// Six children of the selected oscillator and oscillator `other`.
    /// False without both.
    pub fn breed(&mut self, other: usize) -> bool {
        let (Some(a), Some(b)) = (self.selected().cloned(), self.hand.get(other).cloned()) else {
            return false;
        };
        if other == self.selected {
            return false;
        }
        self.fill(|seed| osc::breed(&a, &b, seed));
        true
    }
}

/// Keep `patch` for `d`'s synth: put it on the shelf as a unit file, and
/// give what to load into the first free oscillator slot. Why not, in
/// words, when the synth does not run Oscyl's oscillators.
pub fn keep(shelf: &Shelf, d: &dyn Driver, patch: &Patch) -> Result<UnitFile, String> {
    let p = platform(d)
        .filter(osc::supports)
        .ok_or_else(|| tr("This synth cannot play Oscyl's oscillators."))?;
    let bytes = osc::file(&p, patch).map_err(|e| e.to_string())?;
    let added = shelf.add(d, &bytes)?;
    shelf.unit(d, &added.file)
}

#[cfg(test)]
mod tests {
    use super::*;
    use oscyl_midi::logue::Module;

    #[test]
    fn hands_deal_vary_breed_and_keep() {
        let mut o = Oscillators::new(9);
        assert!(!o.vary(0.35) && o.selected().is_none());
        o.deal(None);
        let families: Vec<Family> = o.hand().iter().map(|p| p.family).collect();
        assert_eq!(families, Family::ALL, "every family in turn");
        o.deal(Some(Family::Fm));
        assert!(o.hand().iter().all(|p| p.family == Family::Fm));
        let first = o.hand().to_vec();
        o.select(2);
        assert!(o.vary(0.35));
        assert_eq!(o.hand().len(), HAND);
        assert!(o.hand().iter().all(|p| !first.contains(p)));
        assert!(!o.breed(0), "itself");
        assert!(o.breed(3));
        assert_eq!(o.selected_index(), 0);
        // Again from the same seed: the same hands.
        let mut again = Oscillators::new(9);
        again.deal(None);
        again.deal(Some(Family::Fm));
        assert_eq!(again.hand(), first);

        let root = std::env::temp_dir().join(format!("oscyl-oscs-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let shelf = Shelf::at(&root);
        let xd = oscyl_midi::driver("korg-minilogue-xd").unwrap();
        let rev2 = oscyl_midi::driver("sequential-prophet-rev2").unwrap();
        let p = o.selected().unwrap().clone();
        let unit = keep(&shelf, xd, &p).unwrap();
        assert_eq!((unit.module, &*unit.name), (Module::Osc, &*p.name));
        let shelved = shelf.list(xd);
        assert_eq!(shelved.len(), 1);
        let params: Vec<String> = shelved[0]
            .info
            .params
            .iter()
            .flatten()
            .map(|q| q.name.clone())
            .collect();
        assert_eq!(params, p.params);
        assert!(keep(&shelf, rev2, &p).is_err());
        std::fs::remove_dir_all(&root).unwrap();
    }
}
