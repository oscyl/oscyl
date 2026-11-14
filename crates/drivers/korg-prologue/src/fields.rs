//! The program's field schema (TABLE 3 of KORG's "prologue MIDI
//! Implementation"): program-wide fields, then every timbre field twice, as
//! "Main …" (timbre 1 at byte 80) and "Sub …" (timbre 2 at byte 206).

use std::collections::HashMap;
use std::sync::LazyLock;

/// How a field is stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layout {
    Byte,
    /// Little-endian u16 (KORG labels the lower offset `H:`; the factory
    /// programs show the low byte comes first).
    Word,
    /// `bits` bits from bit `shift` of one byte.
    Bits {
        shift: u8,
        bits: u8,
    },
    /// A 2.00 byte whose documented range (2-32) has no 0 or 1, and which
    /// 1.x programs leave at `FF`: a stored `FF` or 0 reads [`NOT_SET`],
    /// and writing [`NOT_SET`] keeps whichever of the two is stored (`FF`
    /// otherwise).
    UnsetFf,
}

/// The aftertouch assign's "not set" value. 1 is the E.pedal list's
/// "Volume", which KORG's 2-32 leaves out for aftertouch.
pub const NOT_SET: u16 = 1;

/// One stored parameter, raw values `min..=max`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    /// Unique: `"Main VCO 1 Wave"`, `"Mod FX Type"`, `"Octave"`.
    pub name: &'static str,
    /// The editor section, also the schema group: `"Main VCO 1"`.
    pub group: &'static str,
    /// The section title without the timbre: `"VCO 1"`.
    pub section: &'static str,
    /// The name inside its section: `"Wave"`.
    pub label: &'static str,
    /// 0 main, 1 sub; `None` for program-wide fields.
    pub timbre: Option<u8>,
    pub offset: u16,
    pub layout: Layout,
    pub min: u16,
    pub max: u16,
}

/// Timbre 1 (main) and timbre 2 (sub) offsets.
pub const TIMBRES: [(u16, &str); 2] = [(80, "Main"), (206, "Sub")];
/// Bytes each timbre takes.
pub const TIMBRE_LEN: u16 = 126;

use Layout::*;

type Row = (&'static str, &'static str, u16, Layout, u16, u16);

/// Program-wide rows: (section, label, offset, layout, min, max). Bytes
/// 51-58 are 2.00 fields stored + 1, so 0 ("not set", as every 1.x program
/// holds) is in range.
const PROGRAM: &[Row] = &[
    ("Program", "Octave", 16, Byte, 0, 4),
    ("Program", "Program Level", 40, Byte, 12, 132),
    ("Program", "Amp Velocity", 37, Byte, 0, 127),
    ("Program", "Portamento Mode", 38, Byte, 0, 1),
    ("Program", "Category", 29, Byte, 0, 8),
    ("Program", "Program Tuning", 53, Byte, 0, 101),
    ("Program", "Program Transpose", 54, Byte, 0, 25),
    ("Program", "Micro Tuning", 51, Byte, 0, 140),
    ("Program", "Scale Key", 52, Byte, 0, 25),
    ("Timbres", "Sub On", 17, Byte, 0, 1),
    ("Timbres", "Edit Timbre", 18, Byte, 0, 2),
    ("Timbres", "Timbre Type", 19, Byte, 0, 2),
    ("Timbres", "Balance", 20, Byte, 0, 127),
    ("Timbres", "Main/Sub Position", 22, Byte, 0, 1),
    ("Timbres", "Split Point", 23, Byte, 0, 127),
    ("Arpeggiator", "Mode", 73, Byte, 0, 2),
    ("Arpeggiator", "Type", 75, Byte, 0, 15),
    ("Arpeggiator", "Range", 74, Byte, 0, 15),
    ("Arpeggiator", "Rate", 56, Byte, 0, 12),
    ("Arpeggiator", "Gate Time", 55, Byte, 0, 73),
    ("Arpeggiator", "Target", 26, Byte, 0, 2),
    ("Arpeggiator", "Tempo", 24, Word, 300, 6000),
    ("Mod FX", "On", 71, Byte, 0, 1),
    // 1-based like the xd's (factory programs hold 1-4).
    ("Mod FX", "Type", 41, Byte, 1, 5),
    ("Mod FX", "Chorus", 46, Byte, 0, 7),
    // KORG's table orders 47-49 ensemble, phaser, flanger; the factory
    // programs show 47 is the flanger's and 49 the phaser's subtype
    // (confirmed by a probe). Their order in the
    // schema stays chorus, ensemble, phaser, flanger.
    ("Mod FX", "Ensemble", 48, Byte, 0, 2),
    ("Mod FX", "Phaser", 49, Byte, 0, 7),
    ("Mod FX", "Flanger", 47, Byte, 0, 7),
    ("Mod FX", "User", 50, Byte, 0, 15),
    ("Mod FX", "Speed", 42, Word, 0, 1023),
    ("Mod FX", "Depth", 44, Word, 0, 1023),
    ("Mod FX", "Routing", 69, Byte, 0, 2),
    ("Delay/Reverb", "On", 72, Byte, 0, 1),
    ("Delay/Reverb", "Type", 62, Byte, 0, 2),
    ("Delay/Reverb", "Delay Type", 68, Byte, 0, 19),
    ("Delay/Reverb", "Reverb Type", 67, Byte, 0, 17),
    ("Delay/Reverb", "Time", 63, Word, 0, 1023),
    ("Delay/Reverb", "Depth", 65, Word, 0, 1023),
    ("Delay/Reverb", "Dry/Wet", 57, Word, 0, 1025),
    ("Delay/Reverb", "Routing", 70, Byte, 0, 2),
];

/// Timbre rows, offsets relative to the timbre's start.
const TIMBRE: &[Row] = &[
    ("Voice", "Mode", 6, Byte, 0, 3),
    ("Voice", "Depth", 4, Word, 0, 1023),
    ("Voice", "Portamento", 0, Byte, 0, 127),
    ("Voice", "Voice Spread", 2, Byte, 0, 127),
    ("Voice", "Mono Legato", 119, Byte, 0, 1),
    ("VCO 1", "Wave", 10, Byte, 0, 2),
    ("VCO 1", "Octave", 11, Byte, 0, 3),
    ("VCO 1", "Pitch", 12, Word, 0, 1023),
    ("VCO 1", "Shape", 14, Word, 0, 1023),
    ("VCO 2", "Wave", 19, Byte, 0, 2),
    ("VCO 2", "Octave", 20, Byte, 0, 3),
    ("VCO 2", "Pitch", 21, Word, 0, 1023),
    ("VCO 2", "Shape", 23, Word, 0, 1023),
    ("VCO 2", "Ring/Sync", 25, Byte, 0, 2),
    ("VCO 2", "Cross Mod", 26, Word, 0, 1023),
    ("Pitch EG", "Target", 16, Byte, 0, 2),
    ("Pitch EG", "Int", 17, Word, 0, 1023),
    ("Multi Engine", "Type", 29, Byte, 0, 2),
    ("Multi Engine", "Octave", 30, Byte, 0, 3),
    ("Multi Engine", "Routing", 28, Byte, 0, 1),
    ("Multi Engine", "Noise Type", 31, Byte, 0, 3),
    ("Multi Engine", "VPM Type", 32, Byte, 0, 15),
    ("Multi Engine", "User Osc", 33, Byte, 0, 15),
    ("Multi Engine", "Noise Shape", 34, Word, 0, 1023),
    ("Multi Engine", "VPM Shape", 107, Word, 0, 1023),
    ("Multi Engine", "VPM Shift", 109, Word, 0, 1023),
    ("Multi Engine", "User Shape", 111, Word, 0, 1023),
    ("Multi Engine", "User Shift", 113, Word, 0, 1023),
    ("Multi Engine", "Feedback", 81, Byte, 0, 200),
    ("Multi Engine", "Noise Depth", 83, Byte, 0, 200),
    ("Multi Engine", "Shape Mod", 85, Byte, 0, 200),
    ("Multi Engine", "Mod Attack", 87, Byte, 0, 200),
    ("Multi Engine", "Mod Decay", 88, Byte, 0, 200),
    ("Multi Engine", "Mod Key Track", 91, Byte, 0, 200),
    ("Multi Engine", "Param 1", 93, Byte, 0, 200),
    ("Multi Engine", "Param 2", 95, Byte, 0, 200),
    ("Multi Engine", "Param 3", 97, Byte, 0, 200),
    ("Multi Engine", "Param 4", 99, Byte, 0, 200),
    ("Multi Engine", "Param 5", 101, Byte, 0, 200),
    ("Multi Engine", "Param 6", 103, Byte, 0, 200),
    (
        "Multi Engine",
        "Param 1 Type",
        106,
        Bits { shift: 0, bits: 2 },
        0,
        3,
    ),
    (
        "Multi Engine",
        "Param 2 Type",
        106,
        Bits { shift: 2, bits: 2 },
        0,
        3,
    ),
    (
        "Multi Engine",
        "Param 3 Type",
        106,
        Bits { shift: 4, bits: 2 },
        0,
        3,
    ),
    (
        "Multi Engine",
        "Param 4 Type",
        106,
        Bits { shift: 6, bits: 2 },
        0,
        3,
    ),
    (
        "Multi Engine",
        "Param 5 Type",
        105,
        Bits { shift: 0, bits: 2 },
        0,
        3,
    ),
    (
        "Multi Engine",
        "Param 6 Type",
        105,
        Bits { shift: 2, bits: 2 },
        0,
        3,
    ),
    ("Mixer", "VCO 1", 38, Word, 0, 1023),
    ("Mixer", "VCO 2", 40, Word, 0, 1023),
    ("Mixer", "Multi", 42, Word, 0, 1023),
    ("Filter", "Cutoff", 44, Word, 0, 1023),
    ("Filter", "Resonance", 46, Word, 0, 1023),
    ("Filter", "EG Int", 48, Word, 0, 1023),
    ("Filter", "Drive", 50, Byte, 0, 2),
    ("Filter", "Low Cut", 51, Byte, 0, 1),
    ("Filter", "Key Track", 52, Byte, 0, 2),
    ("Filter", "Velocity", 53, Byte, 0, 127),
    ("Amp EG", "Attack", 54, Word, 0, 1023),
    ("Amp EG", "Decay", 56, Word, 0, 1023),
    ("Amp EG", "Sustain", 58, Word, 0, 1023),
    ("Amp EG", "Release", 60, Word, 0, 1023),
    ("EG", "Attack", 62, Word, 0, 1023),
    ("EG", "Decay", 64, Word, 0, 1023),
    ("EG", "Sustain", 66, Word, 0, 1023),
    ("EG", "Release", 68, Word, 0, 1023),
    ("LFO", "Wave", 70, Byte, 0, 2),
    ("LFO", "Mode", 71, Byte, 0, 2),
    ("LFO", "Rate", 72, Word, 0, 1023),
    ("LFO", "Int", 74, Word, 0, 1023),
    ("LFO", "Target", 76, Byte, 0, 2),
    ("LFO", "Target Osc", 118, Byte, 0, 3),
    ("LFO", "Key Sync", 116, Byte, 0, 1),
    ("LFO", "Voice Sync", 117, Byte, 0, 1),
    ("Controllers", "Mod Wheel", 77, Byte, 2, 32),
    ("Controllers", "Mod Wheel Range", 115, Byte, 0, 200),
    ("Controllers", "E.Pedal", 78, Byte, 0, 32),
    ("Controllers", "Aftertouch", 120, UnsetFf, NOT_SET, 32),
    ("Controllers", "Bend +", 79, Byte, 0, 12),
    ("Controllers", "Bend -", 80, Byte, 0, 12),
];

/// Documented reserved bytes, kept verbatim, with no field: program-wide
/// absolute offsets, then timbre-relative ones.
pub const RESERVED: &[u16] = &[
    21, 27, 28, 30, 31, 32, 33, 34, 35, 36, 39, 59, 60, 61, 76, 77, 78, 79,
];
pub const TIMBRE_RESERVED: &[u16] = &[
    1, 3, 7, 8, 9, 36, 37, 82, 84, 86, 89, 90, 92, 94, 96, 98, 100, 102, 104, 121, 122, 123, 124,
    125,
];

fn leak(s: String) -> &'static str {
    s.leak()
}

static ALL: LazyLock<Vec<Field>> = LazyLock::new(|| {
    let mut all = vec![];
    for &(section, label, offset, layout, min, max) in PROGRAM {
        let name = match section {
            "Program" | "Timbres" => label,
            _ => leak(format!("{section} {label}")),
        };
        all.push(Field {
            name,
            group: section,
            section,
            label,
            timbre: None,
            offset,
            layout,
            min,
            max,
        });
    }
    for (t, &(start, timbre)) in TIMBRES.iter().enumerate() {
        for &(section, label, offset, layout, min, max) in TIMBRE {
            all.push(Field {
                name: leak(format!("{timbre} {section} {label}")),
                group: leak(format!("{timbre} {section}")),
                section,
                label,
                timbre: Some(t as u8),
                offset: start + offset,
                layout,
                min,
                max,
            });
        }
    }
    all
});

static INDEX: LazyLock<HashMap<&'static str, usize>> =
    LazyLock::new(|| ALL.iter().enumerate().map(|(i, f)| (f.name, i)).collect());

/// Every field, in contract order.
pub fn all() -> &'static [Field] {
    &ALL
}

/// The index of field `name`; panics on a name the schema lacks.
pub fn index(name: &str) -> usize {
    *INDEX
        .get(name)
        .unwrap_or_else(|| panic!("no prologue field {name:?}"))
}

/// The field `name`.
pub fn field(name: &str) -> &'static Field {
    &ALL[index(name)]
}

/// The index of timbre field `"<section> <label>"` in `timbre` (0 main).
pub fn in_timbre(timbre: u8, section_label: &str) -> usize {
    index(&format!("{} {section_label}", TIMBRES[timbre as usize].1))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::program::PROGRAM_LEN;

    fn masks(f: &Field) -> Vec<(usize, u8)> {
        let o = f.offset as usize;
        match f.layout {
            Byte | UnsetFf => vec![(o, 0xFF)],
            Word => vec![(o, 0xFF), (o + 1, 0xFF)],
            Bits { shift, bits } => vec![(o, (((1u16 << bits) - 1) as u8) << shift)],
        }
    }

    #[test]
    fn no_overlaps_and_full_coverage() {
        let mut used = [0u8; PROGRAM_LEN];
        let mut claim = |off: usize, m: u8, what: &str| {
            assert_eq!(used[off] & m, 0, "{what} overlaps at byte {off}");
            used[off] |= m;
        };
        for off in (0..16).chain(332..336) {
            claim(off, 0xFF, "marker or name");
        }
        for &off in RESERVED {
            claim(off as usize, 0xFF, "reserved");
        }
        for (start, _) in TIMBRES {
            for &off in TIMBRE_RESERVED {
                claim((start + off) as usize, 0xFF, "timbre reserved");
            }
            // +105 bits 4-7 are reserved.
            claim(start as usize + 105, 0xF0, "timbre reserved bits");
        }
        for f in all() {
            assert!(f.min <= f.max, "{}", f.name);
            for (off, m) in masks(f) {
                claim(off, m, f.name);
            }
        }
        let uncovered: Vec<usize> = (0..PROGRAM_LEN).filter(|&i| used[i] != 0xFF).collect();
        assert!(uncovered.is_empty(), "uncovered bytes {uncovered:?}");
    }

    #[test]
    fn sub_is_main_plus_126() {
        let main: Vec<&Field> = all().iter().filter(|f| f.timbre == Some(0)).collect();
        let sub: Vec<&Field> = all().iter().filter(|f| f.timbre == Some(1)).collect();
        assert_eq!(main.len(), sub.len());
        for (m, s) in main.iter().zip(&sub) {
            assert_eq!(s.offset, m.offset + TIMBRE_LEN);
            assert_eq!(s.name.strip_prefix("Sub "), m.name.strip_prefix("Main "));
        }
        assert_eq!(field("Main Filter Cutoff").offset, 124);
        assert_eq!(field("Sub Filter Cutoff").offset, 250);
        assert_eq!(in_timbre(1, "Amp EG Attack"), index("Sub Amp EG Attack"));
    }

    #[test]
    fn mod_fx_subtypes_follow_the_factory_evidence() {
        // Ruling 6 (ledger): 46 chorus, 47 flanger, 48 ensemble, 49 phaser.
        let at = |n: &str| (field(n).offset, field(n).max);
        assert_eq!(at("Mod FX Chorus"), (46, 7));
        assert_eq!(at("Mod FX Flanger"), (47, 7));
        assert_eq!(at("Mod FX Ensemble"), (48, 2));
        assert_eq!(at("Mod FX Phaser"), (49, 7));
    }

    #[test]
    fn names_are_unique() {
        assert_eq!(INDEX.len(), all().len());
    }
}
