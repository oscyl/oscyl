//! The xd's contract hooks give exactly what its own modules give.

use oscyl_driver::{Control, Driver, Graph, Handle, Recipe};
use oscyl_korg_minilogue_xd::generate::{self, Character};
use oscyl_korg_minilogue_xd::present::{self, Stage};
use oscyl_korg_minilogue_xd::{DRIVER, Field, Program, help, viz};

fn idx(f: Field) -> usize {
    Field::ALL.iter().position(|g| *g == f).unwrap()
}

fn page(s: Stage) -> usize {
    Stage::ALL.iter().position(|t| *t == s).unwrap()
}

fn sample(c: usize, seed: u64) -> Vec<u8> {
    let r = Recipe {
        character: c,
        seed,
        ..Recipe::default()
    };
    DRIVER.generate(&r, None).unwrap()
}

/// Init plus one generated program per character.
fn programs() -> Vec<Vec<u8>> {
    let mut out = vec![DRIVER.init_program()];
    out.extend((0..Character::ALL.len()).map(|c| sample(c, 5)));
    out
}

#[test]
fn every_field_matches_the_modules() {
    for bytes in programs() {
        let p = Program::from_bytes(&bytes).unwrap();
        for (i, &f) in Field::ALL.iter().enumerate() {
            assert_eq!(DRIVER.format_value(&bytes, i), present::format_in(&p, f));
            assert_eq!(DRIVER.relevant(&bytes, i), present::relevant(&p, f));
            assert_eq!(DRIVER.short_label(i), present::short_label(f));
            assert_eq!(DRIVER.help(i).summary, help::help(f).summary);
            let want = match present::control(f) {
                present::Control::Dial => Control::Continuous,
                present::Control::Toggle => Control::Toggle,
                present::Control::Choice(n) => Control::Choice(n),
            };
            if f != Field::VoiceModeDepth {
                assert_eq!(DRIVER.control(&bytes, i), want, "{f:?}");
            }
        }
    }
}

#[test]
fn format_raw_matches_present() {
    for (i, &f) in Field::ALL.iter().enumerate() {
        let s = f.spec();
        for v in [s.min, s.max] {
            assert_eq!(DRIVER.format_raw(i, v), present::format_value(f, v));
        }
    }
}

#[test]
fn primary_fields_are_emphasized() {
    let on: Vec<Field> = Field::ALL
        .iter()
        .copied()
        .filter(|&f| DRIVER.emphasized(idx(f)))
        .collect();
    assert_eq!(on, present::PRIMARY);
    assert_eq!(
        present::PRIMARY,
        [
            Field::Vco1Shape,
            Field::Vco2Shape,
            Field::Cutoff,
            Field::Resonance
        ]
    );
}

#[test]
fn page_summary_reads_like_the_editor() {
    let bytes = DRIVER.init_program();
    let p = Program::from_bytes(&bytes).unwrap();
    let f = |field| present::format_in(&p, field);
    assert_eq!(
        DRIVER.page_summary(&bytes, page(Stage::Filter)),
        format!("Cutoff {} · Res {}", f(Field::Cutoff), f(Field::Resonance))
    );
    assert_eq!(
        DRIVER.page_summary(&bytes, page(Stage::Oscillators)),
        format!("{} + {}", f(Field::Vco1Wave), f(Field::Vco2Wave))
    );
    for s in Stage::ALL {
        assert_eq!(
            DRIVER.page_summary(&bytes, page(s)),
            present::summary(s, &p)
        );
    }
    let mut q = p.clone();
    for l in [Field::Vco1Level, Field::Vco2Level, Field::MultiLevel] {
        q.set(l, 0).unwrap();
    }
    assert_eq!(present::summary(Stage::Mixer, &q), "Silent");
}

#[test]
fn depth_label_follows_voice_mode() {
    let mut p = DRIVER.init_program();
    let (mode, depth) = (idx(Field::VoiceModeType), idx(Field::VoiceModeDepth));
    for (m, label) in [(1, "Pattern"), (2, "Chord"), (3, "Detune"), (4, "Duo")] {
        DRIVER.set(&mut p, mode, m).unwrap();
        assert_eq!(DRIVER.label_in(&p, depth), label);
    }
    assert_eq!(DRIVER.label_in(&p, idx(Field::ArpRate)), "Rate");
}

#[test]
fn categories_list_what_category_returns() {
    let cats = DRIVER.categories();
    assert_eq!(
        cats,
        [
            "Bass", "Lead", "Pad", "Keys", "Pluck", "Bell", "Arp", "FX", "Drone"
        ]
    );
    for bytes in programs() {
        assert!(cats.contains(&DRIVER.category(&bytes).unwrap()));
    }
    let space = Character::ALL
        .iter()
        .position(|c| *c == Character::SpaceFx)
        .unwrap();
    assert_eq!(DRIVER.category(&sample(space, 1)), Some("FX"));
}

#[test]
fn graphs_and_handles_delegate_to_viz() {
    let bytes = sample(5, 3);
    let p = Program::from_bytes(&bytes).unwrap();
    assert_eq!(
        DRIVER.graphs(&bytes, page(Stage::Filter), 96),
        [Graph::Response {
            points: viz::filter_response(&p, 96),
            cutoff: f32::from(p.get(Field::Cutoff)) / 1023.0,
            unity: viz::FILTER_UNITY,
        }]
    );
    assert_eq!(
        DRIVER.graphs(&bytes, page(Stage::Oscillators), 512),
        [Graph::Wave(viz::osc_cycle(&p, 512))]
    );
    assert_eq!(
        DRIVER.graphs(&bytes, page(Stage::Envelopes), 0),
        [Graph::Envelope(viz::amp_envelope(&p))]
    );
    assert_eq!(
        DRIVER.graphs(&bytes, page(Stage::Mixer), 0),
        [Graph::Levels(viz::mixer_levels(&p).to_vec())]
    );
    let voices = DRIVER.graphs(&bytes, page(Stage::Voice), 0);
    assert!(matches!(&voices[..], [Graph::Voices(v)] if v.len() == viz::voices(&p).len()));
    assert!(matches!(
        &DRIVER.graphs(&bytes, page(Stage::Effects), 0)[..],
        [Graph::Effects { .. }]
    ));
    let handles = DRIVER.handles(&bytes, page(Stage::Envelopes));
    let want = viz::handles(Stage::Envelopes);
    assert_eq!(handles.len(), want.len());
    let h: &Handle = &handles[1];
    assert_eq!((h.name, h.at), ("Attack", want[1].position(&p)));
    assert_eq!((h.x, h.y), (Some(idx(Field::AmpEgAttack)), None));
    let dragged = DRIVER.drag(&bytes, page(Stage::Filter), 0, (0.3, 0.6));
    let want: Vec<(usize, u16)> = viz::Handle::FilterPeak
        .values(&p, (0.3, 0.6))
        .into_iter()
        .map(|(f, v)| (idx(f), v))
        .collect();
    assert_eq!(dragged, want);
    assert!(DRIVER.handles(&bytes, page(Stage::Voice)).is_empty());
}

#[test]
fn characters_have_signature_pages_and_macros() {
    let sig = |c: Character| {
        let i = Character::ALL.iter().position(|d| *d == c).unwrap();
        DRIVER.characters()[i].signature_page
    };
    assert_eq!(sig(Character::PowerChord), Some(page(Stage::Voice)));
    assert_eq!(sig(Character::FuzzBass), Some(page(Stage::Oscillators)));
    assert_eq!(sig(Character::Wobble), Some(page(Stage::Filter)));
    assert_eq!(sig(Character::SpaceFx), Some(page(Stage::Effects)));
    assert_eq!(sig(Character::Bell), Some(page(Stage::Envelopes)));
    let m = DRIVER.macro_info();
    assert_eq!(
        m[0].help,
        "Cutoff, resonance and the second oscillator's octave"
    );
    assert_eq!(m[3].title, "Ambience");
    assert_eq!(
        DRIVER.generated_name(2, 77),
        generate::name(77, Character::ALL[2])
    );
}

#[test]
fn hand_matches_the_generate_module() {
    let c = 2;
    let ours = oscyl_core::generate::hand(&DRIVER, c, 99, generate::HAND, |s| {
        DRIVER.generate(
            &Recipe {
                character: c,
                seed: s,
                ..Recipe::default()
            },
            None,
        )
    })
    .unwrap();
    let theirs = generate::hand(Character::ALL[c], 99, |s| {
        generate::generate(&generate::Recipe {
            character: Character::ALL[c],
            macros: generate::Macros::default(),
            seed: s,
            keep: vec![],
            units: vec![],
        })
    });
    let theirs: Vec<Vec<u8>> = theirs.iter().map(|p| p.as_bytes().to_vec()).collect();
    assert_eq!(ours, theirs);
    // A maker that repeats itself still deals distinct, distinctly named
    // programs.
    let same = sample(c, 1);
    let dealt = oscyl_core::generate::hand(&DRIVER, c, 5, 4, |_| Ok(same.clone())).unwrap();
    for (i, a) in dealt.iter().enumerate() {
        for b in &dealt[i + 1..] {
            assert_ne!(a, b);
            assert_ne!(DRIVER.name(a), DRIVER.name(b));
        }
    }
}

/// With a palette, a sound that takes the user oscillator stores each
/// declared parameter inside its range, as its type stores it, and notes
/// the types; a variation keeps the selection, and a child of it and a
/// plain sound never selects another slot.
#[test]
fn a_palette_oscillator_is_used_within_its_parameters() {
    use oscyl_driver::{Palette, PaletteUnit, UnitParam, UnitParamKind as K};
    let get = |p: &[u8], name: &str| {
        let i = DRIVER.fields().iter().position(|f| f.name == name).unwrap();
        DRIVER.get(p, i).unwrap()
    };
    let osc = DRIVER.capabilities().units[0];
    assert_eq!(osc.name, "osc");
    let param = |min, max, kind| UnitParam {
        name: "P".into(),
        min,
        max,
        kind,
    };
    let palette = Palette {
        units: vec![PaletteUnit {
            kind: osc,
            slot: 5,
            name: "waves".into(),
            params: Some(vec![
                param(0, 45, K::Choice),
                param(10, 90, K::Percent),
                param(-50, 20, K::Bipolar),
            ]),
        }],
    };
    let pad = DRIVER
        .characters()
        .iter()
        .position(|c| c.id == "pad")
        .unwrap();
    let wind = DRIVER
        .characters()
        .iter()
        .position(|c| c.id == "wind")
        .unwrap();
    let recipe = |character, seed, palette: &Palette| Recipe {
        character,
        seed,
        palette: palette.clone(),
        ..Recipe::default()
    };
    for seed in 0..50 {
        let p = DRIVER.generate(&recipe(pad, seed, &palette), None).unwrap();
        assert_eq!(DRIVER.needs(&p), [(osc, 5)], "seed {seed}");
        assert_eq!(get(&p, "Multi Type"), 2);
        assert!(get(&p, "Multi Level") >= 450);
        assert!(get(&p, "User Param 1") <= 45);
        assert!((10..=90).contains(&get(&p, "User Param 2")));
        assert!(
            (50..=120).contains(&get(&p, "User Param 3")),
            "bipolar is stored plus 100"
        );
        let types = [
            "User Param 1 Type",
            "User Param 2 Type",
            "User Param 3 Type",
        ];
        assert_eq!(types.map(|t| get(&p, t)), [2, 0, 1]);
        // Over a base that lacks the unit, with a page kept, the types are
        // still the unit's.
        let base = DRIVER
            .generate(&recipe(pad, seed, &Palette::default()), None)
            .unwrap();
        let filter = DRIVER
            .curation()
            .pages
            .iter()
            .position(|pg| pg.title == "Filter")
            .unwrap();
        let kept = DRIVER
            .generate(
                &Recipe {
                    keep: vec![filter],
                    ..recipe(pad, seed, &palette)
                },
                Some(&base),
            )
            .unwrap();
        if get(&kept, "Multi Type") == 2 {
            assert_eq!(types.map(|t| get(&kept, t)), [2, 0, 1], "seed {seed}");
        }
        // Wind is noise alone: it leaves an oscillator be.
        let w = DRIVER
            .generate(&recipe(wind, seed, &palette), None)
            .unwrap();
        assert!(DRIVER.needs(&w).is_empty());
        // No palette, no unit; the same recipe otherwise.
        let plain = DRIVER
            .generate(&recipe(pad, seed, &Palette::default()), None)
            .unwrap();
        assert!(DRIVER.needs(&plain).is_empty());
        // A variation keeps the oscillator; children select slot 5 or none.
        let v = DRIVER.vary(&p, 0.9, seed).unwrap();
        assert_eq!(get(&v, "Multi Type"), 2);
        assert_eq!(get(&v, "Select User"), 5);
        for (a, b) in [(&p, &plain), (&plain, &p)] {
            for child in [DRIVER.breed(a, b, 0.5, seed), DRIVER.morph(a, b, 0.5, seed)] {
                let child = child.unwrap();
                assert!(DRIVER.needs(&child).iter().all(|n| *n == (osc, 5)));
                if get(&child, "Multi Type") == 2 {
                    assert_eq!(
                        types.map(|t| get(&child, t)),
                        [2, 0, 1],
                        "types travel with it"
                    );
                }
            }
        }
    }
}
