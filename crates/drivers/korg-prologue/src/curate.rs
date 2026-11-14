//! Editor curation for the prologue: pages that follow the panel (voice,
//! oscillators, mixer, filter, envelopes and LFO, effects), with the main
//! timbre's section before the sub timbre's; controls, labels, value
//! formats from the MIDI Implementation's notes (P2-P27), relevance and
//! help.

use std::sync::LazyLock;

use oscyl_driver::{Control, Curation, Help, Page, Section};
use oscyl_logue::kinds::{
    ARP_RATES, CHORDS, CHORUS_TYPES, DELAY_TYPES, ENSEMBLE_TYPES, FEET, FLANGER_TYPES,
    LFO_BPM_RATES, LFO_TARGET_OSC, LFO_TARGETS, MOD_FX_TYPES, MULTI_ROUTING, MULTI_TYPES,
    NOISE_TYPES, PHASER_TYPES, REVERB_TYPES, THIRDS, USER_PARAM_TYPES, USER_SLOTS, VPM_TYPES,
    WAVES, eg_int_percent, gate_percent, level_db, microtuning, pitch_cents, signed,
};

use crate::Program;
use crate::fields::{Field, all, field};

struct PageDef {
    title: &'static str,
    icon: &'static str,
    help: &'static str,
    /// Section titles, which are schema groups.
    sections: &'static [&'static str],
}

const PAGES: &[PageDef] = &[
    PageDef {
        title: "Voice",
        icon: "oscyl-voice-symbolic",
        help: "The program, how its two timbres combine, the arpeggiator, and each \
               timbre's voice mode and controllers.",
        sections: &[
            "Program",
            "Timbres",
            "Arpeggiator",
            "Main Voice",
            "Sub Voice",
            "Main Controllers",
            "Sub Controllers",
        ],
    },
    PageDef {
        title: "Oscillators",
        icon: "oscyl-oscillator-symbolic",
        help: "Two analog VCOs and the multi engine (noise, VPM or a user oscillator) \
               per timbre, with the pitch EG.",
        sections: &[
            "Main VCO 1",
            "Main VCO 2",
            "Main Pitch EG",
            "Main Multi Engine",
            "Sub VCO 1",
            "Sub VCO 2",
            "Sub Pitch EG",
            "Sub Multi Engine",
        ],
    },
    PageDef {
        title: "Mixer",
        icon: "oscyl-mixer-symbolic",
        help: "How loud each source is going into the filter.",
        sections: &["Main Mixer", "Sub Mixer"],
    },
    PageDef {
        title: "Filter",
        icon: "oscyl-filter-symbolic",
        help: "The low-pass filter shapes the tone; the EG and the keyboard can move it.",
        sections: &["Main Filter", "Sub Filter"],
    },
    PageDef {
        title: "Envelopes",
        icon: "oscyl-envelope-symbolic",
        help: "The amp EG shapes the loudness of each note; the EG moves cutoff and \
               pitch; the LFO adds repeating movement.",
        sections: &[
            "Main Amp EG",
            "Main EG",
            "Main LFO",
            "Sub Amp EG",
            "Sub EG",
            "Sub LFO",
        ],
    },
    PageDef {
        title: "Effects",
        icon: "oscyl-effects-symbolic",
        help: "One modulation effect and one delay or reverb for the whole program, each \
               routed to the main timbre, the sub timbre or both.",
        sections: &["Mod FX", "Delay/Reverb"],
    },
];

/// Pages, as indices into [`Curation::pages`].
pub const VOICE: usize = 0;
pub const OSCILLATORS: usize = 1;
pub const MIXER: usize = 2;
pub const FILTER: usize = 3;
pub const ENVELOPES: usize = 4;
pub const EFFECTS: usize = 5;

/// Fields the editor does not show: the user oscillator's parameter types,
/// which the loaded unit sets.
fn excluded(f: &Field) -> bool {
    f.section == "Multi Engine" && f.label.starts_with("Param ") && f.label.ends_with(" Type")
}

pub static CURATION: LazyLock<Curation> = LazyLock::new(|| Curation {
    pages: PAGES
        .iter()
        .map(|def| Page {
            title: def.title,
            icon: Some(def.icon),
            help: def.help,
            sections: def
                .sections
                .iter()
                .map(|&title| Section {
                    title,
                    fields: all()
                        .iter()
                        .enumerate()
                        .filter(|(_, f)| f.group == title && !excluded(f))
                        .map(|(i, _)| i)
                        .collect(),
                })
                .collect(),
        })
        .collect(),
    excluded: (0..all().len()).filter(|&i| excluded(&all()[i])).collect(),
});

/// The E.pedal assign list (note P24); mod wheel and aftertouch use it
/// from "Balance" (2) on.
const P24: [&str; 33] = [
    "Off",
    "Volume",
    "Balance",
    "Portamento",
    "V.Spread",
    "V.M Depth",
    "VCO 1 Pitch",
    "VCO 1 Shape",
    "VCO 2 Pitch",
    "VCO 2 Shape",
    "Cross Mod",
    "Pitch EG Int",
    "Multi Shape",
    "VCO 1 Level",
    "VCO 2 Level",
    "Multi Level",
    "Cutoff",
    "Resonance",
    "Cutoff EG Int",
    "A.EG Attack",
    "A.EG Decay",
    "A.EG Sustain",
    "A.EG Release",
    "EG Attack",
    "EG Decay",
    "EG Sustain",
    "EG Release",
    "LFO Rate",
    "LFO Int",
    "Mod FX Speed",
    "Mod FX Depth",
    "DL/RV Time",
    "DL/RV Depth",
];
const MOD_WHEEL: &[&str] = P24.split_at(2).1;
pub const CATEGORIES: &[&str] = &[
    "Poly Synth",
    "Bass",
    "Lead",
    "Pad/Strings",
    "Key/Bell",
    "Chord",
    "Arp",
    "Combination",
    "SFX",
];
const ROUTES: &[&str] = &["Main+Sub", "Main", "Sub"];
const NOT_SET: &str = "Not set";

/// The control for `f` without context.
fn base_control(f: &Field) -> Control {
    use Control::*;
    let c = |names| Choice(names);
    match (f.section, f.label) {
        ("Program", "Octave") => c(&["-2", "-1", "0", "+1", "+2"]),
        ("Program", "Portamento Mode") => c(&["Auto", "On"]),
        ("Program", "Category") => c(CATEGORIES),
        ("Timbres", "Edit Timbre") => c(&["Sub", "Main+Sub", "Main"]),
        ("Timbres", "Timbre Type") => c(&["Layer", "XFade", "Split"]),
        ("Timbres", "Main/Sub Position") => c(&["Sub<>Main", "Main<>Sub"]),
        ("Arpeggiator", "Mode") => c(&["Off", "On", "Latch"]),
        ("Arpeggiator", "Type") => c(&[
            "Manual",
            "Rise",
            "Fall",
            "Rise Fall",
            "Random",
            "Poly Random",
        ]),
        ("Arpeggiator", "Target") | ("Mod FX", "Routing") | ("Delay/Reverb", "Routing") => {
            c(ROUTES)
        }
        ("Mod FX", "Type") => c(MOD_FX_TYPES),
        ("Mod FX", "Chorus") => c(CHORUS_TYPES),
        ("Mod FX", "Ensemble") => c(ENSEMBLE_TYPES),
        ("Mod FX", "Phaser") => c(PHASER_TYPES),
        ("Mod FX", "Flanger") => c(FLANGER_TYPES),
        ("Mod FX", "User") | ("Multi Engine", "User Osc") => c(USER_SLOTS),
        ("Delay/Reverb", "Type") => c(&["Off", "Delay", "Reverb"]),
        ("Delay/Reverb", "Delay Type") => c(DELAY_TYPES),
        ("Delay/Reverb", "Reverb Type") => c(REVERB_TYPES),
        ("Voice", "Mode") => c(&["Poly", "Mono", "Unison", "Chord"]),
        ("VCO 1" | "VCO 2" | "LFO", "Wave") => c(WAVES),
        ("VCO 1" | "VCO 2" | "Multi Engine", "Octave") => c(FEET),
        ("VCO 2", "Ring/Sync") => c(&["Ring", "Off", "Sync"]),
        ("Pitch EG", "Target") => c(&["VCO 1", "VCO 1+2", "VCO 2"]),
        ("Multi Engine", "Type") => c(MULTI_TYPES),
        ("Multi Engine", "Routing") => c(MULTI_ROUTING),
        ("Multi Engine", "Noise Type") => c(NOISE_TYPES),
        ("Multi Engine", "VPM Type") => c(VPM_TYPES),
        ("Multi Engine", l) if l.ends_with(" Type") => c(USER_PARAM_TYPES),
        ("Filter", "Drive" | "Key Track") => c(THIRDS),
        ("LFO", "Mode") => c(&["BPM", "Slow", "Fast"]),
        ("LFO", "Target") => c(LFO_TARGETS),
        ("LFO", "Target Osc") => c(LFO_TARGET_OSC),
        ("Controllers", "Mod Wheel") => c(MOD_WHEEL),
        ("Controllers", "E.Pedal") => c(&P24),
        _ if f.max - f.min == 1 => Toggle,
        _ => Continuous,
    }
}

/// The control for `f` in `p`: the voice mode depth names a chord in
/// chord mode (note P13, 14 bands of 73).
pub fn control(p: &Program, f: &Field) -> Control {
    if (f.section, f.label) != ("Voice", "Depth") {
        return base_control(f);
    }
    match p.get(mode_of(f)) {
        3 => Control::Bands {
            names: CHORDS,
            width: 73,
        },
        _ => Control::Continuous,
    }
}

/// The voice mode field of `f`'s timbre.
fn mode_of(f: &Field) -> &'static Field {
    sibling(f, "Voice Mode")
}

/// `section_label` in `f`'s timbre.
fn sibling(f: &Field, section_label: &str) -> &'static Field {
    let t = ["Main", "Sub"][usize::from(f.timbre.unwrap_or(0))];
    field(&format!("{t} {section_label}"))
}

pub fn short_label(f: &Field) -> &'static str {
    f.label
}

/// The voice mode depth's job under the timbre's mode (note P13).
pub fn label_in(p: &Program, f: &Field) -> &'static str {
    if (f.section, f.label) != ("Voice", "Depth") {
        return f.label;
    }
    match p.get(mode_of(f)) {
        0 => "Duo",
        1 => "Sub",
        2 => "Detune",
        _ => "Chord",
    }
}

const NOTES: [&str; 12] = [
    "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
];

/// `raw` for `f` by itself (no context).
pub fn format_raw(f: &Field, raw: u16) -> String {
    let unset = |show: &dyn Fn(u16) -> String| {
        if raw == 0 {
            NOT_SET.into()
        } else {
            show(raw - 1)
        }
    };
    match (f.section, f.label) {
        ("VCO 1" | "VCO 2", "Pitch") => signed(pitch_cents(raw).round() as i32, " c"),
        // Note P17: the VCO pitch curve at four times the range.
        ("Pitch EG", "Int") => signed((pitch_cents(raw) * 4.0).round() as i32, " c"),
        ("Filter", "EG Int") => signed(eg_int_percent(raw).round() as i32, "%"),
        ("Multi Engine", l) if !l.starts_with("Param") && f.max == 200 => {
            signed(raw as i32 - 100, "%")
        }
        ("Controllers", "Mod Wheel Range") => signed(raw as i32 - 100, "%"),
        ("Controllers", "Bend +") if raw > 0 => format!("+{raw}"),
        ("Controllers", "Bend -") if raw > 0 => format!("-{raw}"),
        ("Controllers", "Bend +" | "Bend -") => "Off".into(),
        ("Controllers", "Aftertouch") if raw == crate::fields::NOT_SET => NOT_SET.into(),
        ("Controllers", "Aftertouch") => P24
            .get(raw as usize)
            .map_or(raw.to_string(), |s| (*s).into()),
        ("Program", "Program Level") => level_db(raw),
        ("Program", "Program Tuning") => unset(&|v| signed(v as i32 - 50, " c")),
        ("Program", "Program Transpose") => unset(&|v| signed(v as i32 - 12, "")),
        ("Program", "Scale Key") => unset(&|v| signed(v as i32 - 12, "")),
        ("Program", "Micro Tuning") => unset(&microtuning),
        ("Arpeggiator", "Gate Time") => unset(&gate_percent),
        ("Arpeggiator", "Rate") => unset(&|v| {
            ARP_RATES
                .get(v as usize)
                .map_or(v.to_string(), |s| (*s).into())
        }),
        ("Arpeggiator", "Tempo") => format!("{}.{} BPM", raw / 10, raw % 10),
        ("Delay/Reverb", "Dry/Wet") => unset(&|v| v.to_string()),
        ("Timbres", "Split Point") => format!(
            "{}{}",
            NOTES[usize::from(raw % 12)],
            i32::from(raw / 12) - 1
        ),
        _ => match base_control(f) {
            Control::Toggle => if raw == f.min { "Off" } else { "On" }.into(),
            Control::Choice(names) => raw
                .checked_sub(f.min)
                .and_then(|i| names.get(usize::from(i)))
                .map_or_else(|| raw.to_string(), |s| (*s).into()),
            _ => raw.to_string(),
        },
    }
}

/// `f`'s value in `p`, read with the fields it depends on: voice mode depth
/// per mode (P13), LFO rate as a note length in BPM mode (P22), user
/// oscillator parameters per their type (P26).
pub fn format_in(p: &Program, f: &Field) -> String {
    let raw = p.get(f);
    if f.section == "Multi Engine" && f.label.starts_with("Param ") && f.label.len() == 7 {
        return match p.get(sibling(f, &format!("Multi Engine {} Type", f.label))) {
            0 => format!("{raw}%"),
            1 => signed(raw as i32 - 100, ""),
            2 => (raw + 1).to_string(),
            _ => raw.to_string(),
        };
    }
    let pick = |names: &[&str], width: u16| {
        names[usize::from(raw / width).min(names.len() - 1)].to_string()
    };
    match (f.section, f.label) {
        ("Voice", "Depth") => match p.get(mode_of(f)) {
            0 if raw < 256 => "Poly".into(),
            0 => format!("Duo {}", raw - 256),
            1 => format!("Sub {raw}"),
            2 => format!("{} c", (u32::from(raw) * 50 + 511) / 1023),
            _ => pick(CHORDS, 73),
        },
        ("LFO", "Rate") if p.get(sibling(f, "LFO Mode")) == 0 => pick(LFO_BPM_RATES, 64),
        _ => format_raw(f, raw),
    }
}

/// Whether `f` does anything under `p`'s other settings.
pub fn relevant(p: &Program, f: &Field) -> bool {
    if f.timbre == Some(1) && p.get(field("Sub On")) == 0 {
        return false;
    }
    let g = |name: &str| p.get(field(name));
    match (f.section, f.label) {
        ("Multi Engine", "Noise Type" | "Noise Shape") => {
            p.get(sibling(f, "Multi Engine Type")) == 0
        }
        (
            "Multi Engine",
            "VPM Type" | "VPM Shape" | "VPM Shift" | "Feedback" | "Noise Depth" | "Shape Mod"
            | "Mod Attack" | "Mod Decay" | "Mod Key Track",
        ) => p.get(sibling(f, "Multi Engine Type")) == 1,
        ("Multi Engine", l) if l.starts_with("User") || l.starts_with("Param") => {
            p.get(sibling(f, "Multi Engine Type")) == 2
        }
        ("Mod FX", "Chorus") => g("Mod FX Type") == 1,
        ("Mod FX", "Ensemble") => g("Mod FX Type") == 2,
        ("Mod FX", "Phaser") => g("Mod FX Type") == 3,
        ("Mod FX", "Flanger") => g("Mod FX Type") == 4,
        ("Mod FX", "User") => g("Mod FX Type") == 5,
        ("Delay/Reverb", "Delay Type") => g("Delay/Reverb Type") == 1,
        ("Delay/Reverb", "Reverb Type") => g("Delay/Reverb Type") == 2,
        ("Timbres", "Split Point") => g("Sub On") == 1 && g("Timbre Type") == 2,
        ("Timbres", "Balance" | "Main/Sub Position" | "Timbre Type" | "Edit Timbre") => {
            g("Sub On") == 1
        }
        ("Arpeggiator", l) if l != "Mode" => g("Arpeggiator Mode") != 0,
        _ => true,
    }
}

/// Fields editors draw larger.
pub fn emphasized(f: &Field) -> bool {
    matches!(
        (f.section, f.label),
        ("VCO 1" | "VCO 2", "Shape") | ("Filter", "Cutoff" | "Resonance")
    )
}

pub fn help(f: &Field) -> Help {
    let summary = match (f.section, f.label) {
        ("Timbres", "Sub On") => "Adds the sub timbre, a second complete voice, to the main one",
        ("Timbres", "Timbre Type") => {
            "Layer plays both timbres, XFade blends them by key, Split puts them either side \
             of the split point"
        }
        ("Timbres", "Balance") => "The volume balance between the main and sub timbres",
        ("Voice", "Mode") => "Poly, mono with a sub oscillator, unison, or chords",
        ("Voice", "Depth") => "What the voice mode adds: duo, sub, detune or which chord",
        ("Voice", "Voice Spread") => "Spreads the voices across the stereo field",
        ("VCO 1" | "VCO 2", "Shape") => "Changes the wave's shape and harmonics",
        ("VCO 1" | "VCO 2", "Pitch") => "Fine and coarse tuning, ±1200 cents",
        ("VCO 2", "Ring/Sync") => "Ring modulates VCO 2 with VCO 1, or syncs it to VCO 1",
        ("VCO 2", "Cross Mod") => "VCO 1 modulates VCO 2's pitch for metallic tones",
        ("Pitch EG", "Int") => "How far the EG bends the pitch, ±4800 cents",
        ("Multi Engine", "Type") => "Noise, the VPM oscillator or a loaded user oscillator",
        ("Filter", "Cutoff") => "Where the filter starts cutting the highs",
        ("Filter", "Resonance") => "Emphasis at the cutoff; high values whistle",
        ("Filter", "EG Int") => "How far the EG moves the cutoff, up or down",
        ("Filter", "Low Cut") => "Removes low end before the filter",
        ("Amp EG", "Attack") => "How fast a note fades in",
        ("Amp EG", "Release") => "How long a note rings after the key is let go",
        ("LFO", "Mode") => "BPM syncs the rate to the tempo; Slow and Fast set its range",
        ("Mod FX", "Routing") | ("Delay/Reverb", "Routing") => {
            "Which timbres go through the effect"
        }
        ("Program", "Micro Tuning" | "Scale Key" | "Program Tuning" | "Program Transpose")
        | ("Arpeggiator", "Gate Time" | "Rate")
        | ("Delay/Reverb", "Dry/Wet") => {
            "Added in firmware 2.00; programs from 1.x hold \"Not set\" and keep it until you \
             change it"
        }
        _ => "",
    };
    Help {
        summary,
        detail: "",
    }
}

/// A one-line description of `page` in `p`.
pub fn summary(p: &Program, page: usize) -> String {
    let main = |name: &str| format_in(p, field(&format!("Main {name}")));
    let sub_on = p.get(field("Sub On")) == 1;
    match page {
        VOICE if sub_on => format!(
            "{} · {} + {}",
            format_in(p, field("Timbre Type")),
            main("Voice Mode"),
            format_in(p, field("Sub Voice Mode"))
        ),
        VOICE => main("Voice Mode"),
        OSCILLATORS => format!("{} + {}", main("VCO 1 Wave"), main("VCO 2 Wave")),
        MIXER => format!(
            "VCO 1 {} · VCO 2 {} · Multi {}",
            main("Mixer VCO 1"),
            main("Mixer VCO 2"),
            main("Mixer Multi")
        ),
        FILTER => format!(
            "Cutoff {} · Res {}",
            main("Filter Cutoff"),
            main("Filter Resonance")
        ),
        ENVELOPES => format!("A {} · R {}", main("Amp EG Attack"), main("Amp EG Release")),
        EFFECTS => {
            let on: Vec<String> = [
                ("Mod FX On", "Mod FX Type"),
                ("Delay/Reverb On", "Delay/Reverb Type"),
            ]
            .into_iter()
            .filter(|(on, _)| p.get(field(on)) == 1)
            .map(|(_, ty)| format_in(p, field(ty)))
            .collect();
            if on.is_empty() {
                "All off".into()
            } else {
                on.join(" · ")
            }
        }
        _ => String::new(),
    }
}

/// The category stored in the program (byte 29), by name.
pub fn category(p: &Program) -> Option<&'static str> {
    CATEGORIES
        .get(usize::from(p.get(field("Category"))))
        .copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fmt(name: &str, raw: u16) -> String {
        format_raw(field(name), raw)
    }

    #[test]
    fn formats_follow_korg_notes() {
        assert_eq!(fmt("Main VCO 1 Pitch", 0), "-1200 c");
        assert_eq!(fmt("Main VCO 1 Pitch", 512), "0 c");
        assert_eq!(fmt("Main Pitch EG Int", 0), "-4800 c");
        assert_eq!(fmt("Main Pitch EG Int", 1023), "+4800 c");
        assert_eq!(fmt("Main Filter EG Int", 1023), "+100%");
        assert_eq!(fmt("Micro Tuning", 0), "Not set");
        assert_eq!(fmt("Micro Tuning", 1), "Equal Temp");
        assert_eq!(fmt("Program Tuning", 51), "0 c");
        assert_eq!(fmt("Program Transpose", 13), "0");
        assert_eq!(fmt("Arpeggiator Rate", 11), "4th");
        assert_eq!(fmt("Arpeggiator Gate Time", 73), "100%");
        assert_eq!(fmt("Arpeggiator Tempo", 1200), "120.0 BPM");
        assert_eq!(fmt("Program Level", 102), "0.0 dB");
        assert_eq!(fmt("Mod FX Type", 1), "Chorus");
        assert_eq!(fmt("Main Controllers Mod Wheel", 16), "Cutoff");
        assert_eq!(fmt("Main Controllers Mod Wheel", 28), "LFO Int");
        assert_eq!(fmt("Main Controllers Aftertouch", 1), "Not set");
        assert_eq!(fmt("Split Point", 60), "C4");
        assert_eq!(fmt("Main VCO 2 Ring/Sync", 1), "Off");
        assert_eq!(fmt("Category", 3), "Pad/Strings");
        // Every factory program holds 2, the CC order's "Main".
        assert_eq!(fmt("Edit Timbre", 2), "Main");
        assert_eq!(fmt("Edit Timbre", 0), "Sub");
        assert_eq!(fmt("Main Controllers Aftertouch", 2), "Balance");
    }

    #[test]
    fn depth_and_lfo_rate_read_their_context() {
        let mut p = Program::init();
        let depth = field("Main Voice Depth");
        assert_eq!(format_in(&p, depth), "Poly");
        p.set(field("Main Voice Mode"), 3).unwrap();
        p.set(depth, 1023).unwrap();
        assert_eq!(format_in(&p, depth), "Maj7b5");
        assert_eq!(label_in(&p, depth), "Chord");
        assert!(matches!(
            control(&p, depth),
            Control::Bands { width: 73, .. }
        ));
        let rate = field("Sub LFO Rate");
        p.set(field("Sub LFO Mode"), 0).unwrap();
        p.set(rate, 0).unwrap();
        assert_eq!(format_in(&p, rate), "4");
    }

    #[test]
    fn sub_fields_matter_only_with_the_sub_timbre_on() {
        let mut p = Program::init();
        assert!(!relevant(&p, field("Sub Filter Cutoff")));
        p.set(field("Sub On"), 1).unwrap();
        assert!(relevant(&p, field("Sub Filter Cutoff")));
        assert!(relevant(&p, field("Main Multi Engine Noise Shape")));
        assert!(!relevant(&p, field("Main Multi Engine VPM Shape")));
    }

    #[test]
    fn pages_mirror_the_panel_with_main_before_sub() {
        let titles: Vec<_> = CURATION.pages.iter().map(|p| p.title).collect();
        assert_eq!(
            titles,
            [
                "Voice",
                "Oscillators",
                "Mixer",
                "Filter",
                "Envelopes",
                "Effects"
            ]
        );
        let env: Vec<_> = CURATION.pages[ENVELOPES]
            .sections
            .iter()
            .map(|s| s.title)
            .collect();
        assert_eq!(env[0], "Main Amp EG");
        assert_eq!(CURATION.excluded.len(), 12);
    }
}
