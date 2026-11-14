//! A synth's nameplate drawing for the apps' synth picker: an abstract panel
//! laid out from the driver's own editor pages and fields (one column of
//! knobs per page, as many rows as the page is deep) and a name display as
//! wide as its program names. Deterministic, and never a vendor's panel.

use oscyl_driver::Driver;

use crate::i18n::{tr, trn};

/// A knob on the drawing, in unit coordinates (0..1 across and down).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Knob {
    pub x: f32,
    pub y: f32,
    pub r: f32,
}

/// The name display, in unit coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Display {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Panel {
    pub knobs: Vec<Knob>,
    pub display: Option<Display>,
}

/// At most this many pages make columns; a deeper synth still fits the tile.
const MAX_COLUMNS: usize = 6;

/// `d`'s panel drawing.
pub fn panel(d: &dyn Driver) -> Panel {
    let curation = d.curation();
    let depths: Vec<usize> = curation
        .pages
        .iter()
        .map(|p| p.sections.iter().map(|s| s.fields.len()).sum::<usize>())
        .filter(|n| *n > 0)
        .take(MAX_COLUMNS)
        .collect();
    let names = d.name_rules().max_len;
    let display = (names > 0).then(|| Display {
        x: 0.06,
        y: 0.22,
        // Longer names, a wider display.
        w: (0.10 + names as f32 * 0.008).min(0.30),
        h: 0.20,
    });
    let x0 = display.map_or(0.06, |s| s.x + s.w + 0.06);
    let span = 0.94 - x0;
    let columns = depths.len().max(1);
    let step = span / columns as f32;
    let mut knobs = Vec::new();
    for (c, depth) in depths.iter().enumerate() {
        // One row per eight fields, up to three.
        let rows = depth.div_ceil(8).clamp(1, 3);
        let r = (step * 0.32).min(0.09);
        for row in 0..rows {
            knobs.push(Knob {
                x: x0 + step * (c as f32 + 0.5),
                y: (row as f32 + 0.5) / rows as f32 * 0.7 + 0.15,
                r,
            });
        }
    }
    Panel { knobs, display }
}

/// Polyphony in words, translated.
pub fn voices_text(v: &[u8]) -> String {
    match v {
        [] => String::new(),
        [n] => trn("%d voice", "%d voices", u32::from(*n)),
        [a, .., b] => tr("{a} or {b} voices")
            .replace("{a}", &a.to_string())
            .replace("{b}", &b.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_driver_draws_a_panel_the_same_way_twice() {
        for d in oscyl_midi::DRIVERS {
            let a = panel(*d);
            assert!(!a.knobs.is_empty(), "{} has knobs", d.info().id);
            assert_eq!(a, panel(*d));
            for k in &a.knobs {
                assert!(
                    k.x - k.r >= 0.0 && k.x + k.r <= 1.0 && k.y - k.r >= 0.0 && k.y + k.r <= 1.0,
                    "{k:?} inside"
                );
            }
        }
    }

    #[test]
    fn tiers_and_voices_match_the_governance_table() {
        let tier = |id: &str| oscyl_midi::driver(id).unwrap().tier();
        assert_eq!(tier("korg-minilogue-xd"), 2);
        assert_eq!(tier("sequential-prophet-rev2"), 2);
        assert_eq!(tier("korg-prologue"), 2);
        assert_eq!(
            oscyl_midi::driver("korg-minilogue-xd").unwrap().voices(),
            &[4]
        );
    }
}
