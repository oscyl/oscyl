//! The Generate page for the mobile apps: a synth's characters and
//! macros, and its hands of sounds (`oscyl_app::generator`), each sound
//! with its name, kind and picture. Dealing blocks for a moment (six
//! programs): call it off the main thread.

use std::sync::{Arc, Mutex};

use oscyl_app::generator::Hands;
use oscyl_app::i18n::tr;
use oscyl_driver::{Driver, Macros};

use crate::editor::GraphShape;
use crate::synth::{FfiError, driver};

/// A kind of sound the synth can generate.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct CharacterInfo {
    pub title: String,
    /// One line on what it sounds like.
    pub hint: String,
}

/// One of the four macro dials: its name and what it changes.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct MacroInfo {
    pub title: String,
    pub help: String,
}

/// A sound of a hand.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct SoundTile {
    pub name: String,
    /// Its kind ("Bass"), if the synth can tell.
    pub category: Option<String>,
    /// The character's signature page's picture of it.
    pub graph: Option<GraphShape>,
    /// Where it was kept (a slot, a library), when it was.
    pub kept: Option<String>,
}

/// The hand shown.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct HandState {
    pub sounds: Vec<SoundTile>,
    pub selected: u32,
    /// Where it came from ("Variations of “Glass”"), for its legend.
    pub origin: String,
    /// Its position (from 0) among the hands kept, and how many there are.
    pub at: u32,
    pub count: u32,
}

/// One synth's Generate page.
#[derive(uniffi::Object)]
pub struct Generator {
    driver: &'static dyn Driver,
    hands: Mutex<Hands>,
}

impl Generator {
    fn lock(&self) -> std::sync::MutexGuard<'_, Hands> {
        self.hands.lock().unwrap_or_else(|e| e.into_inner())
    }
}

#[uniffi::export]
impl Generator {
    /// `driver_id`'s page, dealing from `seed` (the same seed and choices
    /// deal the same sounds).
    #[uniffi::constructor]
    pub fn new(driver_id: String, seed: u64) -> Result<Arc<Generator>, FfiError> {
        Ok(Arc::new(Generator {
            driver: driver(&driver_id)?,
            hands: Mutex::new(Hands::new(seed)),
        }))
    }

    pub fn characters(&self) -> Vec<CharacterInfo> {
        self.driver
            .characters()
            .iter()
            .map(|c| CharacterInfo {
                title: tr(c.title),
                hint: tr(c.hint),
            })
            .collect()
    }

    pub fn macros(&self) -> Vec<MacroInfo> {
        self.driver
            .macro_info()
            .iter()
            .map(|m| MacroInfo {
                title: tr(m.title),
                help: tr(m.help),
            })
            .collect()
    }

    /// The editor's pages, whose fields can be kept from a base program.
    pub fn pages(&self) -> Vec<String> {
        self.driver
            .curation()
            .pages
            .iter()
            .map(|p| tr(p.title))
            .collect()
    }

    /// Six new sounds of `character` shaped by `macros` (bright, movement,
    /// grit, space, each 0..1); the fields of pages `keep` come from
    /// `base` as they are. False when the synth deals nothing. Blocks.
    pub fn deal(
        &self,
        character: u32,
        macros: Vec<f32>,
        keep: Vec<u32>,
        base: Option<Vec<u8>>,
    ) -> bool {
        let m = |i: usize| macros.get(i).copied().unwrap_or(0.5).clamp(0.0, 1.0);
        let macros = Macros {
            bright: m(0),
            movement: m(1),
            grit: m(2),
            space: m(3),
        };
        let keep: Vec<usize> = if base.is_some() {
            keep.into_iter().map(|k| k as usize).collect()
        } else {
            vec![]
        };
        self.lock().deal(
            self.driver,
            character as usize,
            macros,
            &keep,
            base.as_deref(),
        )
    }

    /// Six variations of the selected sound: `amount` 0 subtle, 1 clear,
    /// 2 wild. Blocks.
    pub fn vary(&self, amount: u32) -> bool {
        self.lock().vary(self.driver, amount as usize)
    }

    /// Six children of the selected sound and sound `other`. Blocks.
    pub fn breed(&self, other: u32) -> bool {
        self.lock().breed(self.driver, other as usize)
    }

    /// Back (-1) or Forward (+1); false at either end.
    pub fn step(&self, delta: i32) -> bool {
        self.lock().step(delta as isize)
    }

    pub fn select(&self, index: u32) {
        self.lock().select(index as usize);
    }

    /// The selected sound was kept at `place` (a slot, a library).
    pub fn mark_kept(&self, place: String) {
        self.lock().mark_kept(place);
    }

    /// The selected sound's program, to play or keep.
    pub fn selected_program(&self) -> Option<Vec<u8>> {
        self.lock().selected().map(|(p, _)| p)
    }

    /// The hand shown, each sound with its picture of the one page whose
    /// pictures differ most across the hand (so the tiles tell the sounds
    /// apart), the character's own page when none differs more; none before
    /// the first deal.
    pub fn hand(&self, points: u32) -> Option<HandState> {
        let hands = self.lock();
        let h = hands.current()?;
        let signature = self
            .driver
            .characters()
            .get(h.character)
            .and_then(|c| c.signature_page)
            .unwrap_or(0);
        let pictures = |page: usize| -> Vec<Option<GraphShape>> {
            h.programs
                .iter()
                .map(|p| {
                    oscyl_app::editor::graph(self.driver, p, page, points as usize)
                        .as_ref()
                        .map(crate::editor::shape)
                })
                .collect()
        };
        let own = pictures(signature);
        let mut best = (spread(&own), own);
        for page in (0..self.driver.curation().pages.len()).filter(|p| *p != signature) {
            let candidate = pictures(page);
            let score = spread(&candidate);
            if score > best.0 {
                best = (score, candidate);
            }
        }
        let mut graphs = best.1.into_iter();
        let (at, count) = hands.position();
        Some(HandState {
            sounds: h
                .programs
                .iter()
                .enumerate()
                .map(|(i, p)| SoundTile {
                    name: self.driver.name(p).unwrap_or_default(),
                    category: self.driver.category(p).map(tr),
                    graph: graphs.next().flatten(),
                    kept: h
                        .kept
                        .as_ref()
                        .filter(|(k, _)| *k == i)
                        .map(|(_, s)| s.clone()),
                })
                .collect(),
            selected: u32::try_from(h.selected).unwrap_or(0),
            origin: h.origin.clone(),
            at: u32::try_from(at).unwrap_or(0),
            count: u32::try_from(count).unwrap_or(0),
        })
    }
}

/// How much the pictures differ: the summed squared distance of their points
/// from the mean picture. Zero when one is missing or they are not drawn
/// alike (marks of other lengths), so such a page never wins.
fn spread(pictures: &[Option<GraphShape>]) -> f64 {
    let flat: Option<Vec<Vec<f32>>> = pictures
        .iter()
        .map(|g| {
            g.as_ref().map(|g| {
                g.marks
                    .iter()
                    .flat_map(|m| m.points.iter().copied())
                    .collect()
            })
        })
        .collect();
    let Some(flat) = flat else { return 0.0 };
    let len = flat.first().map_or(0, Vec::len);
    if flat.len() < 2 || len == 0 || flat.iter().any(|f| f.len() != len) {
        return 0.0;
    }
    let n = flat.len() as f64;
    (0..len)
        .map(|i| {
            let mean = flat.iter().map(|f| f64::from(f[i])).sum::<f64>() / n;
            flat.iter()
                .map(|f| (f64::from(f[i]) - mean).powi(2))
                .sum::<f64>()
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_seed_deals_the_same_bytes_and_hands_walk() {
        let a = Generator::new("korg-minilogue-xd".into(), 42).unwrap();
        let b = Generator::new("korg-minilogue-xd".into(), 42).unwrap();
        assert!(!a.characters().is_empty() && a.macros().len() == 4);
        assert!(a.deal(0, vec![0.5; 4], vec![], None));
        assert!(b.deal(0, vec![0.5; 4], vec![], None));
        assert_eq!(a.selected_program(), b.selected_program());
        let h = a.hand(64).unwrap();
        assert_eq!(h.sounds.len(), 6);
        assert!(h.sounds[0].graph.is_some());
        // The tiles' pictures tell the sounds apart.
        let pictures: Vec<_> = h.sounds.iter().map(|s| s.graph.clone()).collect();
        assert!(spread(&pictures) > 0.0, "the pictures differ");
        a.select(3);
        assert!(a.vary(1));
        assert!(a.breed(1));
        assert_eq!(a.hand(64).unwrap().count, 3);
        assert!(a.step(-1));
        a.mark_kept("001".into());
        let kept = a.hand(8).unwrap();
        assert_eq!(
            kept.sounds[kept.selected as usize].kept.as_deref(),
            Some("001")
        );
        // Kept at the end of a library, then over its first program.
        let doc =
            crate::Document::of_programs("korg-minilogue-xd".into(), "L".into(), vec![]).unwrap();
        let p = a.selected_program().unwrap();
        assert_eq!(doc.put(None, p.clone()).unwrap(), 0);
        assert_eq!(doc.put(Some(0), p).unwrap(), 0);
        let rev2 = Generator::new("sequential-prophet-rev2".into(), 1).unwrap();
        if rev2.characters().is_empty() {
            assert!(
                !rev2.deal(0, vec![], vec![], None),
                "no characters, no hand"
            );
        }
    }
}
