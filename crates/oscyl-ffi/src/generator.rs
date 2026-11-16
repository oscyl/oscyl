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
    /// What it is for, to group by: "keys", "bass", "lead", "pad",
    /// "rhythm" or "effect".
    pub family: String,
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
    /// The user units it relies on, by name ("waves"); empty for most.
    #[uniffi(default = [])]
    pub units: Vec<String>,
}

/// One of the synth's user units, as Generate's "My units" shows it.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct UnitChip {
    /// Its section's short name (`osc`, `modfx`, `delfx`, `revfx`).
    pub module: String,
    pub slot: u8,
    pub name: String,
    /// Its section, for people ("Oscillators").
    pub section: String,
    /// Generate may use it (on until the player excludes it).
    pub on: bool,
}

/// What a surprise or a recipe code dealt, for the page to show.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct Dealt {
    /// Index into [`Generator::characters`].
    pub character: u32,
    /// Bright, movement, grit, space, each 0..1.
    pub macros: Vec<f32>,
}

impl Dealt {
    fn of((character, m): (usize, Macros)) -> Dealt {
        Dealt {
            character: u32::try_from(character).unwrap_or(0),
            macros: vec![m.bright, m.movement, m.grit, m.space],
        }
    }
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
    /// The hands deal with the units that are on.
    fn use_units(&self) {
        let palette = oscyl_app::palette::shared().palette(self.driver);
        self.lock().set_palette(palette);
    }

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
                family: c.family.id().into(),
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

    /// The user units the synth held when last listed (remembered between
    /// runs), each on unless excluded: Generate's "My units". Empty when
    /// it holds none.
    pub fn units(&self) -> Vec<UnitChip> {
        oscyl_app::palette::shared()
            .units(self.driver)
            .into_iter()
            .map(|e| UnitChip {
                module: e.unit.kind.name.into(),
                slot: e.unit.slot,
                name: e.unit.name,
                section: e.section,
                on: e.on,
            })
            .collect()
    }

    /// Let Generate use the unit in `slot` of section `module`, or not,
    /// for the session.
    pub fn set_unit_on(&self, module: String, slot: u8, on: bool) {
        oscyl_app::palette::shared().set_on(self.driver, &module, slot, on);
    }

    /// Once a session, what to tell the player who keeps the selected
    /// sound when it relies on a user unit its target may lack: a slot of
    /// the synth (`on_synth`) that does not hold it, or a library. None
    /// when there is nothing to say.
    pub fn keep_warning(&self, on_synth: bool) -> Option<String> {
        let (p, _) = self.lock().selected()?;
        oscyl_app::palette::shared().keep_warning(self.driver, &p, on_synth)
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
        self.use_units();
        self.lock().deal(
            self.driver,
            character as usize,
            macros,
            &keep,
            base.as_deref(),
        )
    }

    /// Six new sounds of `character` shaped by `macros`, as `deal` makes
    /// them (no pages kept), using only the unit called `name` that the
    /// synth holds: a new oscillator just loaded. False while the synth has
    /// not listed it with its parameters yet. Blocks.
    pub fn deal_with_unit(&self, character: u32, macros: Vec<f32>, name: String) -> bool {
        let unit = oscyl_app::palette::shared()
            .units(self.driver)
            .into_iter()
            .find(|e| e.unit.name == name && e.unit.params.is_some());
        let Some(e) = unit else { return false };
        let m = |i: usize| macros.get(i).copied().unwrap_or(0.5).clamp(0.0, 1.0);
        let macros = Macros {
            bright: m(0),
            movement: m(1),
            grit: m(2),
            space: m(3),
        };
        let mut hands = self.lock();
        hands.set_palette(oscyl_driver::Palette {
            units: vec![e.unit],
        });
        hands.deal(self.driver, character as usize, macros, &[], None)
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

    /// Six steps from the selected sound to sound `other`, neither end
    /// among them. Blocks.
    pub fn morph(&self, other: u32) -> bool {
        self.lock().morph(self.driver, other as usize)
    }

    /// A random character and dials, dealt; what it chose. None when the
    /// synth deals nothing. Blocks.
    pub fn surprise(&self) -> Option<Dealt> {
        self.use_units();
        self.lock().surprise(self.driver).map(Dealt::of)
    }

    /// The selected sound's recipe code, to share ("pad-4f2a…"); none
    /// for a variation, a child, a morph step or a sound that holds pages
    /// of a base program.
    pub fn code(&self) -> Option<String> {
        self.lock().code()
    }

    /// The hand a recipe code names: its sound first, then five more of
    /// the same character and dials; what the code chose. Refused, saying
    /// why, when the code is mistyped or names a character this synth
    /// lacks. Blocks.
    pub fn deal_code(&self, code: String) -> Result<Dealt, FfiError> {
        self.lock()
            .deal_code(self.driver, &code)
            .map(Dealt::of)
            .map_err(|reason| FfiError::Refused { reason })
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
                    units: h.units.get(i).cloned().unwrap_or_default(),
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
        assert!(a.characters().iter().all(|c| matches!(
            &*c.family,
            "keys" | "bass" | "lead" | "pad" | "rhythm" | "effect"
        )));
        let rev2 = Generator::new("sequential-prophet-rev2".into(), 1).unwrap();
        if rev2.characters().is_empty() {
            assert!(
                !rev2.deal(0, vec![], vec![], None),
                "no characters, no hand"
            );
        }
    }

    #[test]
    fn codes_surprises_and_morphs() {
        let a = Generator::new("korg-minilogue-xd".into(), 7).unwrap();
        assert_eq!(a.code(), None, "nothing dealt yet");
        assert!(a.deal(3, vec![0.1, 0.9, 0.5, 0.25], vec![], None));
        a.select(2);
        let (code, sound) = (a.code().unwrap(), a.selected_program().unwrap());
        assert!(code.starts_with("riser-"), "{code}");
        // The code deals the same sound first, on another page too.
        let b = Generator::new("korg-minilogue-xd".into(), 1234).unwrap();
        let dealt = b.deal_code(code.to_uppercase()).unwrap();
        assert_eq!(dealt.character, 3);
        assert_eq!(dealt.macros, vec![0.1, 0.9, 0.5, 0.25]);
        assert_eq!(b.selected_program(), Some(sound));
        assert_eq!(b.code(), Some(code));
        assert!(matches!(
            b.deal_code("riser-oops".into()),
            Err(FfiError::Refused { .. })
        ));
        // A surprise says what it dealt; the same seed surprises alike.
        let c = Generator::new("korg-minilogue-xd".into(), 1234).unwrap();
        let s = b.surprise().unwrap();
        assert!(c.deal_code("riser-oops".into()).is_err());
        assert!((s.character as usize) < b.characters().len());
        assert!(s.macros.len() == 4 && s.macros.iter().all(|m| (0.0..=1.0).contains(m)));
        assert_eq!(b.hand(8).unwrap().count, 2);
        // Morph: six steps, each its own name, none with a code.
        assert!(!b.morph(0), "not with itself");
        assert!(b.morph(4));
        let h = b.hand(8).unwrap();
        assert_eq!((h.sounds.len(), h.count), (6, 3));
        assert!(h.origin.starts_with("From"));
        assert_eq!(b.code(), None);
        // Sounds that hold a base program's pages have no code.
        let base = a.selected_program();
        assert!(a.deal(0, vec![0.5; 4], vec![0], base));
        assert_eq!(a.code(), None);
    }
}
