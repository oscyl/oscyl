//! New oscillators for Generate on the mobile apps: hands of six made by
//! Oscyl's oscillator engine (`oscyl_app::oscillators`). Dealing blocks for
//! a moment: call it off the main thread. Keep one with
//! `Synth.keep_oscillator`.

use std::sync::{Arc, Mutex};

use oscyl_logue::engine::{Family, Patch, render::picture};

use crate::synth::FfiError;

/// A family of oscillators.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct OscillatorFamily {
    /// As `deal` takes it: `wavetable`, `fm`, `fold`, `formant`, `noise`,
    /// `hybrid`.
    pub id: String,
    pub title: String,
}

/// An oscillator of the hand.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct OscillatorTile {
    pub name: String,
    /// Its family, for people ("Wavetable").
    pub family: String,
    /// Its parameters' names, in order.
    pub params: Vec<String>,
    /// One cycle of it, each point -1 to 1.
    pub wave: Vec<f32>,
    /// The oscillator itself, for `Synth.keep_oscillator`.
    pub patch: Vec<u8>,
}

/// The hand of oscillators shown.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct OscillatorHand {
    pub oscillators: Vec<OscillatorTile>,
    pub selected: u32,
}

/// Every family, in order.
#[uniffi::export]
pub fn oscillator_families() -> Vec<OscillatorFamily> {
    Family::ALL
        .into_iter()
        .map(|f| OscillatorFamily {
            id: f.id().into(),
            title: oscyl_app::oscillators::family_title(f),
        })
        .collect()
}

/// Whether `driver_id`'s synth plays Oscyl's oscillators (the minilogue
/// xd and the prologue): Generate shows New oscillators only then.
#[uniffi::export]
pub fn oscillators_supported(driver_id: String) -> bool {
    crate::synth::driver(&driver_id)
        .ok()
        .and_then(oscyl_midi::logue::platform)
        .is_some_and(|p| oscyl_logue::engine::supports(&p))
}

/// Keep oscillator `patch` on `driver_id`'s shelf without a synth to load
/// it into (`Synth.keep_oscillator` does both). Its name. Blocks.
#[uniffi::export]
pub fn shelve_oscillator(
    shelf: Arc<crate::units::Shelf>,
    driver_id: String,
    patch: Vec<u8>,
) -> Result<String, FfiError> {
    let d = crate::synth::driver(&driver_id)?;
    oscyl_app::oscillators::keep(&shelf.0, d, &self::patch(&patch)?)
        .map(|u| u.name)
        .map_err(|reason| FfiError::Refused { reason })
}

/// An oscillator from its bytes (`OscillatorTile.patch`).
pub(crate) fn patch(bytes: &[u8]) -> Result<Patch, FfiError> {
    Patch::from_bytes(bytes).map_err(|e| FfiError::failed(e.to_string()))
}

/// The page's hands of oscillators.
#[derive(uniffi::Object)]
pub struct Oscillators(Mutex<oscyl_app::oscillators::Oscillators>);

impl Oscillators {
    fn lock(&self) -> std::sync::MutexGuard<'_, oscyl_app::oscillators::Oscillators> {
        self.0.lock().unwrap_or_else(|e| e.into_inner())
    }
}

#[uniffi::export]
impl Oscillators {
    /// No hand yet; the hands that follow come from `seed`.
    #[uniffi::constructor]
    pub fn new(seed: u64) -> Arc<Oscillators> {
        Arc::new(Oscillators(Mutex::new(
            oscyl_app::oscillators::Oscillators::new(seed),
        )))
    }

    /// Six new oscillators of family `family` (an id), or of every family
    /// in turn. Blocks.
    pub fn deal(&self, family: Option<String>) -> Result<(), FfiError> {
        let family = match family {
            Some(id) => Some(
                Family::from_id(&id)
                    .ok_or_else(|| FfiError::failed(format!("no oscillator family {id:?}")))?,
            ),
            None => None,
        };
        self.lock().deal(family);
        Ok(())
    }

    /// Six variations of the selected oscillator; `amount` indexes
    /// Subtle, Clear, Wild. False without a hand. Blocks.
    pub fn vary(&self, amount: u32) -> bool {
        let amounts = oscyl_app::generator::AMOUNTS;
        let a = amounts[(amount as usize).min(amounts.len() - 1)];
        self.lock().vary(a)
    }

    /// Six children of the selected oscillator and `other`. Blocks.
    pub fn breed(&self, other: u32) -> bool {
        self.lock().breed(other as usize)
    }

    pub fn select(&self, index: u32) {
        self.lock().select(index as usize);
    }

    /// The hand, each oscillator's wave `points` long; none before the
    /// first deal.
    pub fn hand(&self, points: u32) -> Option<OscillatorHand> {
        let o = self.lock();
        if o.hand().is_empty() {
            return None;
        }
        Some(OscillatorHand {
            oscillators: o
                .hand()
                .iter()
                .map(|p| OscillatorTile {
                    name: p.name.clone(),
                    family: oscyl_app::oscillators::family_title(p.family),
                    params: p.params.clone(),
                    wave: picture(p, points as usize),
                    patch: p.to_bytes(),
                })
                .collect(),
            selected: u32::try_from(o.selected_index()).unwrap_or(0),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hands_of_oscillators() {
        assert_eq!(oscillator_families().len(), 6);
        let o = Oscillators::new(1);
        assert!(o.hand(32).is_none() && !o.vary(1));
        assert!(o.deal(Some("nope".into())).is_err());
        o.deal(Some("fold".into())).unwrap();
        let hand = o.hand(32).unwrap();
        assert_eq!(hand.oscillators.len(), 6);
        let t = &hand.oscillators[0];
        assert_eq!((t.family.as_str(), t.wave.len()), ("Fold", 32));
        assert_eq!(patch(&t.patch).unwrap().name, t.name);
        o.select(4);
        assert!(o.breed(1));
        assert_eq!(o.hand(8).unwrap().selected, 0);
        assert!(oscillators_supported("korg-prologue".into()));
        assert!(!oscillators_supported("sequential-prophet-rev2".into()));
        let dir = std::env::temp_dir().join(format!("oscyl-ffi-oscs-{}", std::process::id()));
        let shelf = crate::units::Shelf::new(dir.to_string_lossy().into_owned());
        let p = o.hand(8).unwrap().oscillators[0].clone();
        let name = shelve_oscillator(shelf.clone(), "korg-minilogue-xd".into(), p.patch).unwrap();
        assert_eq!(name, p.name);
        assert_eq!(shelf.list("korg-minilogue-xd".into()).unwrap().len(), 1);
        let _ = std::fs::remove_dir_all(dir);
        // Sounds with a unit the synth does not hold yet: none dealt.
        let g = crate::generator::Generator::new("korg-minilogue-xd".into(), 1).unwrap();
        assert_eq!(
            g.deal_with_unit(0, vec![0.5; 4], "nowhere".into()).unwrap(),
            None
        );
    }
}
