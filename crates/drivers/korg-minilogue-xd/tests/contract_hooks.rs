//! The xd's contract hooks give exactly what its own modules give.

use oscyl_driver::{Control, Driver, Graph, Handle, Recipe};
use oscyl_minilogue_xd::generate::{self, Character};
use oscyl_minilogue_xd::present::{self, Stage};
use oscyl_minilogue_xd::{DRIVER, Field, Program, help, viz};

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
