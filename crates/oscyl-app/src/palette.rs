//! The user units each synth holds, as Generate's palette: learned from a
//! synth's slot listings, kept with the app's data so Generate deals with
//! the last known units while the synth is away, and narrowed by the
//! units the player switches off for the session. Shared by every app
//! through [`shared`].

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

use oscyl_driver::{Driver, Palette, PaletteUnit, UnitKind, UnitParam, UnitParamKind};
use oscyl_logue::unit::{ParamKind, UnitInfo, palette_unit};
use serde::{Deserialize, Serialize};

use crate::i18n::tr;
use crate::synth::SlotInfo;
use crate::units::module_title;

/// A unit a synth held when its slots were last listed, as it is kept on
/// disk.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Held {
    /// The unit kind's name ([`UnitKind::name`]).
    kind: String,
    slot: u8,
    name: String,
    platform: u8,
    dev_id: u32,
    prg_id: u32,
    version: u32,
    /// `(name, min, max, kind)`, the kind 0 percent, 1 bipolar, 2 a choice;
    /// `None` while they are not known.
    params: Option<Vec<(String, i8, i8, u8)>>,
}

impl Held {
    fn of(kind: &str, slot: u8, info: &UnitInfo) -> Held {
        Held {
            kind: kind.into(),
            slot,
            name: info.name.clone(),
            platform: info.platform,
            dev_id: info.dev_id,
            prg_id: info.prg_id,
            version: info.version,
            params: info.params.as_ref().map(|ps| {
                ps.iter()
                    .map(|p| {
                        let kind = match p.kind {
                            ParamKind::Percent => 0,
                            ParamKind::Bipolar => 1,
                            ParamKind::Select => 2,
                        };
                        (p.name.clone(), p.min, p.max, kind)
                    })
                    .collect()
            }),
        }
    }

    fn info(&self) -> Option<UnitInfo> {
        let module = oscyl_midi::logue::Module::ALL
            .into_iter()
            .find(|m| m.name() == self.kind)?;
        Some(UnitInfo {
            module,
            platform: self.platform,
            name: self.name.clone(),
            dev_id: self.dev_id,
            prg_id: self.prg_id,
            version: self.version,
            params: self.params.as_ref().map(|ps| {
                ps.iter()
                    .map(|(name, min, max, kind)| oscyl_logue::unit::UnitParam {
                        name: name.clone(),
                        min: *min,
                        max: *max,
                        kind: match kind {
                            0 => ParamKind::Percent,
                            1 => ParamKind::Bipolar,
                            _ => ParamKind::Select,
                        },
                    })
                    .collect()
            }),
        })
    }
}

/// One of a synth's units, for a page to show: what it is and whether
/// Generate may use it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub unit: PaletteUnit,
    /// Its section, for people ("Oscillators").
    pub section: String,
    pub on: bool,
}

/// The units every synth model held when last listed, and the ones
/// switched off for now.
#[derive(Debug, Default)]
pub struct Store {
    /// Where it is kept; `None` keeps it in memory only.
    path: Option<PathBuf>,
    /// By driver id.
    held: BTreeMap<String, Vec<Held>>,
    /// `(driver id, kind, slot)` the player excluded: for the session.
    off: BTreeSet<(String, String, u8)>,
    /// The warning about keeping a sound away from its unit was given.
    warned: bool,
}

impl Store {
    /// The store kept at `path` (empty when there is none yet, or it
    /// cannot be read).
    pub fn load(path: &Path) -> Store {
        let held = std::fs::read(path)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        Store {
            path: Some(path.to_path_buf()),
            held,
            ..Store::default()
        }
    }

    fn save(&self) {
        let Some(path) = &self.path else { return };
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        // Losing the cache costs one listing; nothing to tell the player.
        if let Ok(json) = serde_json::to_vec_pretty(&self.held) {
            let _ = std::fs::write(path, json);
        }
    }

    /// Every unit whose parameters are known: the synth worker does not
    /// read those from a synth again.
    pub fn known(&self) -> Vec<UnitInfo> {
        self.held
            .values()
            .flatten()
            .filter(|h| h.params.is_some())
            .filter_map(Held::info)
            .collect()
    }

    /// `d`'s synth listed its slots: they are its units from now on. A
    /// slot that is empty now leaves; a unit seen before keeps the
    /// parameters learned for it.
    pub fn listed(&mut self, d: &dyn Driver, slots: &[SlotInfo]) {
        let id = d.info().id;
        let before = self.held.remove(id).unwrap_or_default();
        let now: Vec<Held> = slots
            .iter()
            .filter_map(|s| {
                let mut h = Held::of(s.module.name(), s.slot, s.info.as_ref()?);
                if h.params.is_none() {
                    let same = |b: &&Held| {
                        (&b.kind, &b.name, b.platform) == (&h.kind, &h.name, h.platform)
                            && (b.dev_id, b.prg_id, b.version) == (h.dev_id, h.prg_id, h.version)
                    };
                    h.params = before.iter().find(same).and_then(|b| b.params.clone());
                }
                Some(h)
            })
            .collect();
        // What left its slot is no longer switched off there.
        self.off.retain(|(driver, kind, slot)| {
            driver != id || now.iter().any(|h| (&h.kind, h.slot) == (kind, *slot))
        });
        if before != now {
            self.held.insert(id.into(), now);
            self.save();
        } else {
            self.held.insert(id.into(), before);
        }
    }

    fn units_of<'a>(&'a self, d: &'a dyn Driver) -> impl Iterator<Item = (&'a Held, UnitKind)> {
        self.held
            .get(d.info().id)
            .into_iter()
            .flatten()
            .filter_map(move |h| {
                let kind = d.capabilities().units.iter().find(|k| k.name == h.kind)?;
                (h.slot < kind.slots).then_some((h, *kind))
            })
    }

    fn is_on(&self, d: &dyn Driver, h: &Held) -> bool {
        !self
            .off
            .contains(&(d.info().id.to_string(), h.kind.clone(), h.slot))
    }

    /// `d`'s units as last listed, each on unless the player excluded it.
    pub fn units(&self, d: &dyn Driver) -> Vec<Entry> {
        self.units_of(d)
            .filter_map(|(h, kind)| {
                let info = h.info()?;
                Some(Entry {
                    unit: palette_unit(kind, h.slot, &info),
                    section: module_title(info.module),
                    on: self.is_on(d, h),
                })
            })
            .collect()
    }

    /// Let Generate use the unit in `slot` of kind `kind` (its name, as
    /// [`UnitKind::name`]), or not, for the session.
    pub fn set_on(&mut self, d: &dyn Driver, kind: &str, slot: u8, on: bool) {
        let key = (d.info().id.to_string(), kind.to_string(), slot);
        if on {
            self.off.remove(&key);
        } else {
            self.off.insert(key);
        }
    }

    /// What Generate may use on `d`'s synth: its units that are on.
    pub fn palette(&self, d: &dyn Driver) -> Palette {
        Palette {
            units: self
                .units(d)
                .into_iter()
                .filter(|e| e.on)
                .map(|e| e.unit)
                .collect(),
        }
    }

    /// Once a session, what to tell the player who keeps `program`, which
    /// relies on user units, somewhere that may not hold them: a synth
    /// (`on_synth`) whose slots hold something else, or a library, which
    /// any synth may play. `None` when the program needs no unit, the
    /// synth holds them all, or it was said already.
    pub fn keep_warning(
        &mut self,
        d: &dyn Driver,
        program: &[u8],
        on_synth: bool,
    ) -> Option<String> {
        let held: Vec<(UnitKind, u8)> = self.units_of(d).map(|(h, k)| (k, h.slot)).collect();
        let lacking: Vec<(UnitKind, u8)> = d
            .needs(program)
            .into_iter()
            .filter(|n| !on_synth || !held.contains(n))
            .collect();
        if lacking.is_empty() || std::mem::replace(&mut self.warned, true) {
            return None;
        }
        let names = unit_names(d, program, &self.all(d));
        let what = if names.is_empty() {
            tr("a user unit")
        } else {
            names.join(", ")
        };
        Some(
            if on_synth {
                tr("This sound uses {units}, which the synth does not hold in that slot. It will sound different until the unit is loaded there.")
            } else {
                tr("This sound uses {units}. A synth plays it as it sounds here only with that unit in the same slot.")
            }
            .replace("{units}", &what),
        )
    }

    /// Every unit of `d`'s, on or off: for naming what a program uses.
    pub fn all(&self, d: &dyn Driver) -> Palette {
        Palette {
            units: self.units(d).into_iter().map(|e| e.unit).collect(),
        }
    }
}

/// The names of the units `program` relies on, as `palette` names them
/// ("waves"); a slot the palette does not know reads as its section and
/// number ("Oscillators 3").
pub fn unit_names(d: &dyn Driver, program: &[u8], palette: &Palette) -> Vec<String> {
    d.needs(program)
        .into_iter()
        .map(|(kind, slot)| {
            let named = palette
                .units
                .iter()
                .find(|u| u.kind == kind && u.slot == slot);
            match named {
                Some(u) => u.name.clone(),
                None => {
                    let section = oscyl_logue::unit::module_of(&kind)
                        .map_or_else(|| kind.name.to_string(), module_title);
                    format!("{section} {}", slot + 1)
                }
            }
        })
        .collect()
}

/// A unit parameter's range for people: "0 to 45", "0 % to 100 %",
/// "-100 % to +100 %". A choice reads one higher, as the synth shows it.
pub fn param_range(p: &UnitParam) -> String {
    let (lo, hi) = match p.kind {
        UnitParamKind::Percent => (format!("{} %", p.min), format!("{} %", p.max)),
        UnitParamKind::Bipolar => (format!("{:+} %", p.min), format!("{:+} %", p.max)),
        UnitParamKind::Choice => ((p.min + 1).to_string(), (p.max + 1).to_string()),
    };
    tr("{from} to {to}")
        .replace("{from}", &lo)
        .replace("{to}", &hi)
}

static SHARED: OnceLock<Mutex<Store>> = OnceLock::new();

/// Keep the app's palettes at `path` from now on (once, at start; later
/// calls change nothing).
pub fn init(path: &Path) {
    let _ = SHARED.set(Mutex::new(Store::load(path)));
}

/// The app's palettes: in memory only until [`init`] says where they are
/// kept.
pub fn shared() -> MutexGuard<'static, Store> {
    SHARED
        .get_or_init(|| Mutex::new(Store::default()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;
    use oscyl_midi::logue::Module;

    fn xd() -> &'static dyn Driver {
        oscyl_midi::driver("korg-minilogue-xd").unwrap()
    }

    fn info(module: Module, name: &str, params: bool) -> UnitInfo {
        UnitInfo {
            module,
            platform: 2,
            name: name.into(),
            dev_id: 1,
            prg_id: 2,
            version: 0x010000,
            params: params.then(|| {
                vec![oscyl_logue::unit::UnitParam {
                    name: "Wave".into(),
                    min: 0,
                    max: 45,
                    kind: ParamKind::Select,
                }]
            }),
        }
    }

    fn slot(module: Module, slot: u8, unit: Option<UnitInfo>) -> SlotInfo {
        SlotInfo {
            module,
            slot,
            name: unit.as_ref().map(|u| u.name.clone()),
            info: unit,
        }
    }

    #[test]
    fn a_listing_is_the_palette_and_survives_the_app() {
        let path = std::env::temp_dir().join(format!("oscyl-palette-{}.json", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let d = xd();
        let mut s = Store::load(&path);
        assert!(s.palette(d).units.is_empty());
        s.listed(
            d,
            &[
                slot(Module::Osc, 3, Some(info(Module::Osc, "waves", true))),
                slot(Module::Osc, 4, None),
                slot(
                    Module::RevFx,
                    7,
                    Some(info(Module::RevFx, "shimmer", false)),
                ),
            ],
        );
        let units = s.units(d);
        assert_eq!(units.len(), 2);
        assert_eq!((units[0].unit.kind.name, units[0].unit.slot), ("osc", 3));
        assert_eq!(units[0].section, "Oscillators");
        let p = &units[0].unit.params.as_ref().unwrap()[0];
        assert_eq!(param_range(p), "1 to 46");
        // Kept: the next run deals with them while the synth is away, and
        // tells the worker what it need not read again.
        let mut again = Store::load(&path);
        assert_eq!(again.units(d), units);
        assert_eq!(again.known(), [info(Module::Osc, "waves", true)]);
        // Excluded for the session, not on disk.
        again.set_on(d, "osc", 3, false);
        assert_eq!(again.palette(d).units.len(), 1);
        assert!(Store::load(&path).units(d).iter().all(|e| e.on));
        // A listing that names no parameters keeps the ones learned; a
        // cleared slot leaves, and its exclusion with it.
        again.listed(
            d,
            &[slot(
                Module::Osc,
                3,
                Some(info(Module::Osc, "waves", false)),
            )],
        );
        assert!(again.units(d)[0].unit.params.is_some());
        again.listed(d, &[slot(Module::Osc, 3, None)]);
        assert!(again.units(d).is_empty());
        again.listed(
            d,
            &[slot(Module::Osc, 3, Some(info(Module::Osc, "waves", true)))],
        );
        assert!(again.units(d)[0].on);
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn keeping_a_sound_away_from_its_unit_warns_once() {
        let d = xd();
        let mut s = Store::default();
        s.listed(
            d,
            &[slot(Module::Osc, 3, Some(info(Module::Osc, "waves", true)))],
        );
        let pad = d.characters().iter().position(|c| c.id == "pad").unwrap();
        let recipe = |palette| oscyl_driver::Recipe {
            character: pad,
            seed: 4,
            palette,
            ..Default::default()
        };
        let plain = d.generate(&recipe(Palette::default()), None).unwrap();
        let with = d.generate(&recipe(s.palette(d)), None).unwrap();
        assert_eq!(unit_names(d, &with, &s.all(d)), ["waves"]);
        assert!(unit_names(d, &plain, &s.all(d)).is_empty());
        // The synth holds it; a sound without units needs nothing.
        assert_eq!(s.keep_warning(d, &with, true), None);
        assert_eq!(s.keep_warning(d, &plain, false), None);
        // A library may be played by any synth: said once.
        let said = s.keep_warning(d, &with, false).unwrap();
        assert!(said.contains("waves"), "{said}");
        assert_eq!(s.keep_warning(d, &with, false), None);
        // Another session, the slot cleared since: the synth lacks it.
        let mut s2 = Store::default();
        s2.listed(d, &[slot(Module::Osc, 3, None)]);
        let said = s2.keep_warning(d, &with, true).unwrap();
        assert!(said.contains("Oscillators 4"), "{said}");
    }
}
