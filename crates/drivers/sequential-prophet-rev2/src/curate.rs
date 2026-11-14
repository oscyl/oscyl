//! Editor curation for the Prophet Rev2: pages, controls, labels, value
//! formats, relevance and help. Where the docs
//! give no meaning or unit (destinations, mod sources, clock divide, …)
//! the raw stored value is shown.

use std::collections::HashMap;
use std::sync::LazyLock;

use oscyl_driver::{Control, Curation, Graph, Help, Page, Section};

use crate::fields;

/// One editor page: title, icon, help, and its per-layer section titles.
struct PageDef {
    title: &'static str,
    icon: Option<&'static str>,
    help: &'static str,
    sections: &'static [&'static str],
}

const PAGES: &[PageDef] = &[
    PageDef {
        title: "Oscillators",
        icon: Some("oscyl-oscillator-symbolic"),
        help: "Two oscillators per layer, plus sub oscillator and noise, mixed into the filter.",
        sections: &["Osc 1", "Osc 2", "Mix"],
    },
    PageDef {
        title: "Filter and amp",
        icon: Some("oscyl-filter-symbolic"),
        help: "The filter shapes the tone; the amp sets level and stereo spread.",
        sections: &["Filter", "Amp"],
    },
    PageDef {
        title: "Envelopes",
        icon: Some("oscyl-envelope-symbolic"),
        help: "Filter and amp envelopes, and a third envelope with a selectable destination.",
        sections: &["Filter env", "Amp env", "Env 3"],
    },
    PageDef {
        title: "LFOs",
        icon: None,
        help: "Four LFOs per layer, each with its own rate, shape, amount and destination.",
        sections: &["LFO 1", "LFO 2", "LFO 3", "LFO 4"],
    },
    PageDef {
        title: "Modulation",
        icon: None,
        help: "Eight modulation slots per layer, and the performance controllers' routing.",
        sections: &[
            "Mod 1",
            "Mod 2",
            "Mod 3",
            "Mod 4",
            "Mod 5",
            "Mod 6",
            "Mod 7",
            "Mod 8",
            "Controllers",
        ],
    },
    PageDef {
        title: "Effects",
        icon: Some("oscyl-effects-symbolic"),
        help: "One effect per layer: pick it, switch it on and set its mix and two parameters.",
        sections: &["Effects"],
    },
    PageDef {
        title: "Voice and arp",
        icon: Some("oscyl-voice-symbolic"),
        help: "How the layers combine, key and unison modes, clock, arpeggiator and sequencer.",
        sections: &["Voice", "Clock and arp", "Sequencer"],
    },
];

/// The page and section of layer field `base` (its name without the
/// `"A "`/`"B "` prefix), or `None` for sequencer step data, which needs
/// a sequencer view of its own.
fn place(base: &str) -> Option<(usize, &'static str)> {
    let starts = |p: &str| base.starts_with(p);
    // Numbered sections: "Osc 1 …", "LFO 3 …", "Mod 8 …".
    for (page, prefix) in [(0, "Osc "), (3, "LFO "), (4, "Mod ")] {
        if let Some(rest) = base.strip_prefix(prefix)
            && let Some(n) = rest.split(' ').next().and_then(|n| n.parse::<usize>().ok())
        {
            let section = PAGES[page]
                .sections
                .iter()
                .find(|s| s.strip_prefix(prefix).is_some_and(|m| m.parse() == Ok(n)))?;
            return Some((page, section));
        }
    }
    Some(match base {
        _ if starts("Gated seq track") && base.contains(" step ") => return None,
        _ if starts("Poly seq") => return None,
        _ if starts("Filter env ") => (2, "Filter env"),
        _ if starts("Amp env ") => (2, "Amp env"),
        _ if starts("Env 3 ") => (2, "Env 3"),
        "Osc mix" | "Sub osc level" | "Noise level" | "Sync" | "Glide mode" | "Glide"
        | "Pitch bend range" | "Osc slop" => (0, "Mix"),
        _ if starts("Filter ") || base == "Resonance" => (1, "Filter"),
        "VCA level" | "Program volume" | "Pan spread" | "Pan mode" => (1, "Amp"),
        _ if ["Mod wheel", "Pressure", "Breath", "Velocity", "Foot"]
            .iter()
            .any(|c| starts(c)) =>
        {
            (4, "Controllers")
        }
        _ if starts("FX ") => (5, "Effects"),
        "Key mode" | "Unison" | "Unison mode" | "Unison detune" => (6, "Voice"),
        _ if starts("Arp ") || base == "BPM" || base == "Clock divide" => (6, "Clock and arp"),
        _ if starts("Sequencer ") || starts("Gated seq track") => (6, "Sequencer"),
        _ => panic!("no page for {base}"),
    })
}

/// What curation needs to know about one field.
struct Meta {
    /// 0 = layer A, 1 = layer B, `None` = program-wide.
    layer: Option<usize>,
    /// The name without the layer prefix.
    base: &'static str,
    place: Option<(usize, &'static str)>,
    label: &'static str,
}

fn short_label(base: &'static str, section: Option<&str>) -> &'static str {
    let rest = section
        .and_then(|s| base.strip_prefix(s))
        .and_then(|r| r.strip_prefix(' '))
        .or_else(|| base.strip_prefix("FX "))
        .or_else(|| base.strip_prefix("Gated seq "))
        .unwrap_or(base);
    if rest == base {
        return base;
    }
    let mut c = rest.chars();
    let first = c.next().map(|f| f.to_ascii_uppercase());
    first.into_iter().chain(c).collect::<String>().leak()
}

static META: LazyLock<Vec<Meta>> = LazyLock::new(|| {
    fields::all()
        .iter()
        .map(|f| {
            let (layer, base) = match f.name.split_at_checked(2) {
                Some(("A ", rest)) => (Some(0), rest),
                Some(("B ", rest)) => (Some(1), rest),
                _ => (None, f.name),
            };
            let place = layer.and_then(|_| place(base));
            Meta {
                layer,
                base,
                place,
                label: short_label(base, place.map(|(_, s)| s)),
            }
        })
        .collect()
});

static INDEX: LazyLock<HashMap<&'static str, usize>> = LazyLock::new(|| {
    fields::all()
        .iter()
        .enumerate()
        .map(|(i, f)| (f.name, i))
        .collect()
});

/// The index of field `name`.
pub(crate) fn index(name: &str) -> usize {
    INDEX[name]
}

/// The index of `base` in layer `layer` (0 = A).
fn in_layer(layer: usize, base: &str) -> usize {
    index(&format!("{} {base}", ["A", "B"][layer]))
}

pub(crate) static CURATION: LazyLock<Curation> = LazyLock::new(|| {
    let meta = &*META;
    let mut pages: Vec<Page> = PAGES
        .iter()
        .enumerate()
        .map(|(p, def)| Page {
            title: def.title,
            icon: def.icon,
            help: def.help,
            sections: (0..2)
                .flat_map(|layer| {
                    def.sections.iter().map(move |&title| Section {
                        title: format!("{} {title}", ["A", "B"][layer]).leak(),
                        fields: meta
                            .iter()
                            .enumerate()
                            .filter(|(_, m)| m.layer == Some(layer) && m.place == Some((p, title)))
                            .map(|(i, _)| i)
                            .collect(),
                    })
                })
                .collect(),
        })
        .collect();
    pages[6].sections.insert(
        0,
        Section {
            title: "Layers",
            fields: vec![index("A/B mode"), index("Split point")],
        },
    );
    Curation {
        pages,
        excluded: meta
            .iter()
            .enumerate()
            .filter(|(_, m)| m.layer.is_some() && m.place.is_none())
            .map(|(i, _)| i)
            .collect(),
    }
});

const OSC_SHAPES: &[&str] = &["Off", "Saw", "Saw-Tri", "Tri", "Pulse"];
const LFO_SHAPES: &[&str] = &["Off", "Tri", "Saw", "Rev Saw", "Square", "Random"];
/// Edisyn's abbreviations.
const FX: &[&str] = &[
    "Off", "DM", "DDS", "BBD", "Ch", "PH", "PL", "PM", "F1", "F2", "Rvb", "RM", "Dst", "HPF",
];

pub(crate) fn control(i: usize) -> Control {
    let Some(m) = META.get(i) else {
        return Control::Continuous;
    };
    let shape = |prefix: &str| {
        m.base
            .strip_prefix(prefix)
            .is_some_and(|r| r.ends_with(" shape"))
    };
    match m.base {
        _ if shape("Osc ") => Control::Choice(OSC_SHAPES),
        _ if shape("LFO ") => Control::Choice(LFO_SHAPES),
        "Glide mode" => {
            Control::Choice(&["Fixed Rate", "Fixed Rate A", "Fixed Time", "Fixed Time A"])
        }
        "Filter poles" => Control::Choice(&["2 pole", "4 pole"]),
        "FX select" => Control::Choice(FX),
        "Key mode" => Control::Choice(&["Low", "High", "Last", "Low R", "High R", "Last R"]),
        "Arp mode" => Control::Choice(&["Up", "Down", "Up+Down", "Random", "Assign"]),
        "Arp range" => Control::Choice(&["1 oct", "2 oct", "3 oct"]),
        "Sequencer mode" => {
            Control::Choice(&["Normal", "No Reset", "No Gate", "No Gate/Reset", "Key Step"])
        }
        "Sequencer type" => Control::Choice(&["Gated", "Poly"]),
        "Pan mode" => Control::Choice(&["Alternate", "Fixed"]),
        _ => {
            let f = &fields::all()[i];
            if f.max - f.min == 1 {
                Control::Toggle
            } else {
                Control::Continuous
            }
        }
    }
}

pub(crate) fn short_label_of(i: usize) -> &'static str {
    META.get(i).map_or("", |m| m.label)
}

fn signed(v: i32) -> String {
    if v > 0 {
        format!("+{v}")
    } else {
        v.to_string()
    }
}

/// `v`, stored in field `i`, for people.
pub(crate) fn format(i: usize, v: u16) -> String {
    let m = &META[i];
    let f = &fields::all()[i];
    let b = m.base;
    let has = |s: &str| b.contains(s);
    match b {
        _ if has("velocity ") && b.starts_with("Poly seq") => match v {
            0 => "Reset".into(),
            128 => "Rest".into(),
            129.. => (v - 128).to_string(),
            _ => v.to_string(),
        },
        _ if has(" note ") && b.starts_with("Poly seq") => {
            let tie = if v & 0x80 != 0 { " tie" } else { "" };
            format!("{}{tie}", v & 0x7F)
        }
        _ if b.starts_with("Gated seq") && has(" step ") => match v {
            126 => "Reset".into(),
            127 => "Rest".into(),
            _ => v.to_string(),
        },
        _ if b.starts_with("Gated seq") && v == 53 => "Slew".into(),
        _ if b.starts_with("Osc ") && b.ends_with(" frequency") => format!("{v} st"),
        _ if b.ends_with(" fine tune") => signed(i32::from(v) - 50),
        // Bipolar amounts: offset binary, 127 = zero.
        _ if b.ends_with(" amount") && f.max == 254 => signed(i32::from(v) - 127),
        "Unison mode" if v == 16 => "Chord".into(),
        "A/B mode" if v == 0 => "Single".into(),
        _ => match control(i) {
            Control::Toggle => if v == 0 { "Off" } else { "On" }.into(),
            Control::Choice(names) => names
                .get(usize::from(v.wrapping_sub(f.min)))
                .map_or_else(|| v.to_string(), |s| (*s).into()),
            _ => v.to_string(),
        },
    }
}

/// Whether field `i` matters under `get`'s other values.
pub(crate) fn relevant(i: usize, get: impl Fn(usize) -> u16) -> bool {
    let m = &META[i];
    let layered = get(index("A/B mode")) != 0;
    let Some(layer) = m.layer else {
        return m.base != "Split point" || layered;
    };
    let on = |base: &str| get(in_layer(layer, base)) != 0;
    if layer == 1 && !layered {
        return false;
    }
    match m.base {
        "Unison mode" | "Unison detune" => on("Unison"),
        "FX mix" | "FX param 1" | "FX param 2" | "FX clock sync" => on("FX on") && on("FX select"),
        "Glide mode" => on("Glide"),
        b if b.starts_with("Osc ") && b.ends_with(" glide") => on("Glide"),
        _ => true,
    }
}

/// The filter's cutoff and resonance are each section's main controls.
pub(crate) fn emphasized(i: usize) -> bool {
    META.get(i)
        .is_some_and(|m| m.layer.is_some() && matches!(m.base, "Filter cutoff" | "Resonance"))
}

/// A one-line description of `page` for layer A, from field values `get`.
pub(crate) fn summary(page: usize, get: impl Fn(usize) -> u16) -> String {
    let a = |base: &str| in_layer(0, base);
    let f = |i: usize| format(i, get(i));
    let fa = |base: &str| f(a(base));
    let joined = |parts: Vec<String>, none: &str| {
        if parts.is_empty() {
            none.to_string()
        } else {
            parts.join(" · ")
        }
    };
    match page {
        0 => format!("{} + {}", fa("Osc 1 shape"), fa("Osc 2 shape")),
        1 => format!("Cutoff {} · Res {}", fa("Filter cutoff"), fa("Resonance")),
        2 => format!("A {} · R {}", fa("Amp env attack"), fa("Amp env release")),
        3 => joined(
            (1..=4)
                .filter(|l| {
                    get(a(&format!("LFO {l} amount"))) > 0 && get(a(&format!("LFO {l} shape"))) > 0
                })
                .map(|l| format!("LFO {l} {}", fa(&format!("LFO {l} shape"))))
                .collect(),
            "All off",
        ),
        4 => {
            let n = (1..=8)
                .filter(|m| get(a(&format!("Mod {m} amount"))) != 127)
                .count();
            if n == 0 {
                "None".into()
            } else {
                format!("{n} of 8 slots")
            }
        }
        5 if get(a("FX on")) != 0 && get(a("FX select")) != 0 => fa("FX select"),
        5 => "Off".into(),
        6 => {
            let mut parts = vec![f(index("A/B mode")), fa("Key mode")];
            if get(a("Arp on")) != 0 {
                parts.push("Arp".into());
            }
            parts.join(" · ")
        }
        _ => String::new(),
    }
}

/// Pictures for layer A: the three envelopes on the Envelopes page, each
/// LFO's waveform on the LFOs page (off and random LFOs have none). The
/// envelope delay is not drawn; each stage gets up to a quarter of the
/// width, as on the minilogue xd.
pub(crate) fn graphs(page: usize, get: impl Fn(usize) -> u16, points: usize) -> Vec<Graph> {
    let a = |base: &str| get(in_layer(0, base));
    match page {
        2 => ["Filter env", "Amp env", "Env 3"]
            .iter()
            .map(|env| {
                let v = |stage: &str| f32::from(a(&format!("{env} {stage}")).min(127)) / 127.0;
                let (at, de, s) = (v("attack") / 4.0, v("decay") / 4.0, v("sustain"));
                let d = at + de;
                let h = d + 0.25;
                Graph::Envelope([
                    (0.0, 0.0),
                    (at, 1.0),
                    (d, s),
                    (h, s),
                    (h + v("release") / 4.0, 0.0),
                ])
            })
            .collect(),
        3 => (1..=4)
            .filter_map(|l| {
                let shape = a(&format!("LFO {l} shape"));
                let wave = |x: f32| match shape {
                    1 if x < 0.5 => 4.0 * x - 1.0,
                    1 => 3.0 - 4.0 * x,
                    2 => 2.0 * x - 1.0,
                    3 => 1.0 - 2.0 * x,
                    4 if x < 0.5 => 1.0,
                    4 => -1.0,
                    _ => f32::NAN,
                };
                (1..=4).contains(&shape).then(|| {
                    Graph::Wave(
                        (0..points)
                            .map(|k| wave(k as f32 / points as f32))
                            .collect(),
                    )
                })
            })
            .collect(),
        _ => vec![],
    }
}

const fn h(summary: &'static str, detail: &'static str) -> Help {
    Help { summary, detail }
}

pub(crate) fn help(i: usize) -> Help {
    let Some(m) = META.get(i) else {
        return Help::default();
    };
    let b = m.base;
    let ends = |s: &str| b.ends_with(s);
    match b {
        "A/B mode" => h(
            "How layers A and B play: one layer, split across the keyboard, or stacked.",
            "0 plays layer A alone. Which of 1 and 2 is split and which is stack is not verified.",
        ),
        "Split point" => h(
            "The key where layer A gives way to layer B in split mode.",
            "",
        ),
        _ if b.starts_with("Osc ") && ends(" frequency") => {
            h("Oscillator pitch, in semitones.", "")
        }
        _ if ends(" fine tune") => h("Fine pitch offset; the centre is no offset.", ""),
        _ if b.starts_with("Osc ") && ends(" shape") => h(
            "Waveform: off, sawtooth, saw-triangle, triangle or pulse.",
            "",
        ),
        _ if ends(" amount") && fields::all()[i].max == 254 => h(
            "Modulation amount, positive or negative.",
            "The middle value is no modulation.",
        ),
        "Filter poles" => h("Filter slope: 2 or 4 poles.", ""),
        "Unison mode" => h("How unison stacks voices.", "The last value plays a chord."),
        "FX select" => h("Which effect this layer uses.", ""),
        "Program volume" => h("The program's overall volume.", ""),
        "Sequencer type" => h(
            "Gated (four tracks of values) or poly (six tracks of notes).",
            "",
        ),
        _ if b.starts_with("Gated seq") && b.contains(" step ") => h(
            "One sequencer step's value.",
            "The two highest values are reset and rest.",
        ),
        _ => Help::default(),
    }
}

#[cfg(test)]
mod tests {
    use crate::DRIVER;
    use oscyl_driver::{Control, Driver};

    fn idx(name: &str) -> usize {
        DRIVER.fields().iter().position(|f| f.name == name).unwrap()
    }

    fn fmt(name: &str, v: u16) -> String {
        let mut p = DRIVER.init_program();
        DRIVER.set(&mut p, idx(name), v).unwrap();
        DRIVER.format_value(&p, idx(name))
    }

    #[test]
    fn pages_cover_both_layers() {
        let c = DRIVER.curation();
        let pages: Vec<_> = c.pages.iter().map(|p| p.title).collect();
        assert_eq!(
            pages,
            [
                "Oscillators",
                "Filter and amp",
                "Envelopes",
                "LFOs",
                "Modulation",
                "Effects",
                "Voice and arp"
            ]
        );
        let osc: Vec<_> = c.pages[0].sections.iter().map(|s| s.title).collect();
        assert_eq!(
            osc,
            ["A Osc 1", "A Osc 2", "A Mix", "B Osc 1", "B Osc 2", "B Mix"]
        );
        assert_eq!(c.pages[6].sections[0].title, "Layers");
        assert_eq!(
            c.pages[6].sections[0].fields,
            [idx("A/B mode"), idx("Split point")]
        );
        assert!(c.excluded.contains(&idx("A Poly seq track 1 note 1")));
        assert!(c.excluded.contains(&idx("B Gated seq track 4 step 16")));
        assert_eq!(DRIVER.short_label(idx("A Osc 1 frequency")), "Frequency");
        assert_eq!(DRIVER.short_label(idx("B Filter cutoff")), "Cutoff");
        assert_eq!(DRIVER.short_label(idx("A FX select")), "Select");
        let init = DRIVER.init_program();
        assert_eq!(
            DRIVER.control(&init, idx("A Filter poles")),
            Control::Choice(&["2 pole", "4 pole"])
        );
        assert_eq!(DRIVER.control(&init, idx("A Sync")), Control::Toggle);
    }

    #[test]
    fn formats_follow_program_format() {
        assert_eq!(fmt("A Osc 1 shape", 1), "Saw");
        assert_eq!(fmt("A Osc 1 frequency", 24), "24 st");
        assert_eq!(fmt("A Osc 2 fine tune", 50), "0");
        assert_eq!(fmt("A Osc 2 fine tune", 40), "-10");
        assert_eq!(fmt("A Mod 1 amount", 127), "0");
        assert_eq!(fmt("A Mod 1 amount", 254), "+127");
        assert_eq!(fmt("B Filter env amount", 0), "-127");
        assert_eq!(fmt("A Poly seq track 1 velocity 1", 0), "Reset");
        assert_eq!(fmt("A Poly seq track 1 velocity 1", 128), "Rest");
        assert_eq!(fmt("A Poly seq track 1 velocity 1", 200), "72");
        assert_eq!(fmt("A Poly seq track 1 note 1", 188), "60 tie");
        assert_eq!(fmt("A Poly seq track 1 note 1", 60), "60");
        assert_eq!(fmt("A Gated seq track 1 step 1", 127), "Rest");
        assert_eq!(fmt("A Gated seq track 1 step 1", 126), "Reset");
        assert_eq!(fmt("A Gated seq track 2 destination", 53), "Slew");
        assert_eq!(fmt("A Unison mode", 16), "Chord");
        assert_eq!(fmt("A/B mode", 0), "Single");
        assert_eq!(fmt("A/B mode", 2), "2");
        assert_eq!(fmt("A FX on", 1), "On");
        assert_eq!(fmt("A LFO 2 destination", 7), "7");
    }

    #[test]
    fn layer_b_hidden_when_single() {
        let mut p = DRIVER.init_program();
        assert!(!DRIVER.relevant(&p, idx("B Filter cutoff")));
        assert!(!DRIVER.relevant(&p, idx("Split point")));
        assert!(DRIVER.relevant(&p, idx("A Filter cutoff")));
        assert!(!DRIVER.relevant(&p, idx("A Unison mode")));
        DRIVER.set(&mut p, idx("A/B mode"), 1).unwrap();
        DRIVER.set(&mut p, idx("A Unison"), 1).unwrap();
        assert!(DRIVER.relevant(&p, idx("B Filter cutoff")));
        assert!(DRIVER.relevant(&p, idx("A Unison mode")));
        assert!(!DRIVER.help(idx("A/B mode")).summary.is_empty());
    }

    #[test]
    fn format_raw_needs_no_program() {
        assert_eq!(DRIVER.format_raw(idx("A Osc 1 shape"), 4), "Pulse");
        assert_eq!(DRIVER.format_raw(idx("B Mod 3 amount"), 0), "-127");
        // Out of range, as a raw byte may be: the number, no panic.
        assert_eq!(DRIVER.format_raw(idx("A BPM"), 10), "10");
        assert_eq!(DRIVER.format_raw(idx("A Osc 1 shape"), 9), "9");
    }

    #[test]
    fn cutoff_and_resonance_are_emphasized() {
        let on: Vec<&str> = DRIVER
            .fields()
            .iter()
            .enumerate()
            .filter(|(i, _)| DRIVER.emphasized(*i))
            .map(|(_, f)| f.name)
            .collect();
        assert_eq!(
            on,
            [
                "A Filter cutoff",
                "A Resonance",
                "B Filter cutoff",
                "B Resonance"
            ]
        );
    }

    #[test]
    fn page_summaries_describe_layer_a() {
        let mut p = DRIVER.init_program();
        let s = |p: &[u8], page| DRIVER.page_summary(p, page);
        assert_eq!(s(&p, 0), "Saw + Saw");
        assert_eq!(s(&p, 1), "Cutoff 164 · Res 0");
        assert_eq!(s(&p, 2), "A 0 · R 0");
        assert_eq!(s(&p, 3), "All off");
        assert_eq!(s(&p, 4), "None");
        assert_eq!(s(&p, 5), "Off");
        assert_eq!(s(&p, 6), "Single · Low");
        DRIVER.set(&mut p, idx("A LFO 2 shape"), 1).unwrap();
        DRIVER.set(&mut p, idx("A LFO 2 amount"), 30).unwrap();
        DRIVER.set(&mut p, idx("A Mod 1 amount"), 200).unwrap();
        DRIVER.set(&mut p, idx("A FX on"), 1).unwrap();
        DRIVER.set(&mut p, idx("A FX select"), 3).unwrap();
        assert_eq!(s(&p, 3), "LFO 2 Tri");
        assert_eq!(s(&p, 4), "1 of 8 slots");
        assert_eq!(s(&p, 5), "BBD");
        assert_eq!(s(&p, 99), "");
    }

    #[test]
    fn envelopes_and_lfos_have_graphs() {
        use oscyl_driver::Graph;
        let mut p = DRIVER.init_program();
        let g = DRIVER.graphs(&p, 2, 64);
        assert_eq!(g.len(), 3);
        // Init amp env: attack 0, decay 0, sustain 127, release 0.
        assert_eq!(
            g[1],
            Graph::Envelope([(0.0, 0.0), (0.0, 1.0), (0.0, 1.0), (0.25, 1.0), (0.25, 0.0)])
        );
        assert!(DRIVER.graphs(&p, 3, 64).is_empty());
        DRIVER.set(&mut p, idx("A LFO 1 shape"), 2).unwrap();
        DRIVER.set(&mut p, idx("A LFO 4 shape"), 5).unwrap(); // random: no picture
        let g = DRIVER.graphs(&p, 3, 64);
        let [Graph::Wave(w)] = &g[..] else {
            panic!("{g:?}")
        };
        assert_eq!(w.len(), 64);
        assert_eq!((w[0], w[32]), (-1.0, 0.0));
        assert!(DRIVER.graphs(&p, 0, 64).is_empty());
        assert!(DRIVER.handles(&p, 2).is_empty());
    }
}
