//! The program's field schema.

use std::sync::LazyLock;

use crate::program::PROGRAM_LEN;

/// One stored parameter: one byte at `offset`, raw values `min..=max`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Field {
    /// `"A …"` / `"B …"` for per-layer fields.
    pub name: &'static str,
    /// `"<Layer> · <section>"`, or `"Program"` for program-wide fields.
    pub group: &'static str,
    pub offset: u16,
    pub min: u16,
    pub max: u16,
    /// NRPN parameter number (User's Guide 1.2.4, pp. 89-94).
    pub nrpn: u16,
}

/// Layer B's copy of a layer A byte; NRPN numbers move by [`LAYER_B_NRPN`].
pub const LAYER_B: u16 = 1024;
pub const LAYER_B_NRPN: u16 = 2048;

/// A layer A parameter, `min..=max` raw values.
struct Row {
    name: String,
    section: &'static str,
    offset: u16,
    min: u16,
    max: u16,
    nrpn: u16,
}

const OSC: &str = "Oscillators";
const ENV: &str = "Envelopes";
const MOD: &str = "Modulation";
const SEQ: &str = "Sequencer";
const VOICE: &str = "Voice";
const ARP: &str = "Clock and arp";
const POLY: &str = "Poly sequencer";

/// Layer A rows.
fn layer_a() -> Vec<Row> {
    let mut rows = vec![];
    let mut row = |name: String, section, offset, max, nrpn| {
        rows.push(Row {
            name,
            section,
            offset,
            min: 0,
            max,
            nrpn,
        })
    };
    // Per-oscillator pairs: osc 1 at `offset`, osc 2 at `offset + 1`.
    for (what, offset, max, nrpn) in [
        ("frequency", 0, 120, [0, 5]),
        ("fine tune", 2, 100, [1, 6]),
        ("shape", 4, 4, [2, 7]),
        ("shape mod", 6, 99, [102, 103]),
        ("glide", 8, 127, [3, 8]),
        ("keyboard", 10, 1, [4, 9]),
        ("note reset", 12, 1, [99, 104]),
    ] {
        for i in 0..2 {
            row(
                format!("Osc {} {what}", i + 1),
                OSC,
                offset + i,
                max,
                nrpn[i as usize],
            );
        }
    }
    for (name, section, offset, max, nrpn) in [
        ("Osc mix", OSC, 14, 127, 13),
        ("Sub osc level", OSC, 15, 127, 110),
        ("Noise level", OSC, 16, 127, 14),
        ("Sync", OSC, 17, 1, 10),
        ("Glide mode", OSC, 18, 3, 11),
        ("Glide", OSC, 19, 1, 111),
        ("Pitch bend range", OSC, 20, 12, 113),
        ("Osc slop", OSC, 21, 127, 12),
        ("Filter cutoff", "Filter", 22, 164, 15),
        ("Resonance", "Filter", 23, 127, 16),
        ("Filter key amount", "Filter", 24, 127, 17),
        ("Filter audio mod", "Filter", 25, 127, 18),
        ("Filter poles", "Filter", 26, 1, 19),
        ("VCA level", "Amp", 27, 127, 98),
        ("Program volume", "Amp", 28, 127, 29),
        ("Pan spread", "Amp", 29, 127, 28),
        ("Env 3 destination", ENV, 30, 52, 57),
        ("Env 3 repeat", ENV, 31, 1, 97),
    ] {
        row(name.into(), section, offset, max, nrpn);
    }
    // Envelope stages, interleaved filter / amp / env 3 from 32.
    for (k, stage) in [
        "amount", "velocity", "delay", "attack", "decay", "sustain", "release",
    ]
    .into_iter()
    .enumerate()
    {
        for (e, env) in ["Filter env", "Amp env", "Env 3"].into_iter().enumerate() {
            // Filter and env 3 amounts are bipolar (127 = zero).
            let max = if k == 0 && e != 1 { 254 } else { 127 };
            let (k, e) = (k as u16, e as u16);
            let nrpn = [20, 30, 58][e as usize] + k;
            row(format!("{env} {stage}"), ENV, 32 + 3 * k + e, max, nrpn);
        }
    }
    for (k, (what, max)) in [
        ("rate", 150),
        ("shape", 5),
        ("amount", 127),
        ("destination", 52),
        ("clock sync", 1),
    ]
    .into_iter()
    .enumerate()
    {
        let k = k as u16;
        for l in 0..4 {
            let name = format!("LFO {} {what}", l + 1);
            row(name, "LFOs", 53 + 4 * k + l, max, 37 + 5 * l + k);
        }
    }
    for l in 0..4 {
        row(
            format!("LFO {} key sync", l + 1),
            "LFOs",
            73 + l,
            1,
            105 + l,
        );
    }
    for (k, (what, max)) in [("source", 22), ("amount", 254), ("destination", 52)]
        .into_iter()
        .enumerate()
    {
        let k = k as u16;
        for m in 0..8 {
            let name = format!("Mod {} {what}", m + 1);
            row(name, MOD, 77 + 8 * k + m, max, 65 + 3 * m + k);
        }
    }
    for (i, src) in ["Mod wheel", "Pressure", "Breath", "Velocity", "Foot"]
        .into_iter()
        .enumerate()
    {
        let i = i as u16;
        row(format!("{src} amount"), MOD, 101 + 2 * i, 254, 116 + 2 * i);
        row(
            format!("{src} destination"),
            MOD,
            102 + 2 * i,
            52,
            117 + 2 * i,
        );
    }
    for t in 0..4 {
        // Tracks 2 and 4 add 53 = slew.
        let max = if t % 2 == 1 { 53 } else { 52 };
        let name = format!("Gated seq track {} destination", t + 1);
        row(name, SEQ, 111 + t, max, 184 + t);
    }
    for (name, section, offset, max, nrpn) in [
        ("FX select", "Effects", 115, 13, 154),
        ("FX on", "Effects", 116, 1, 153),
        ("FX mix", "Effects", 117, 127, 155),
        ("FX param 1", "Effects", 118, 255, 156),
        ("FX param 2", "Effects", 119, 127, 157),
        ("FX clock sync", "Effects", 120, 1, 158),
        ("Key mode", VOICE, 122, 5, 170),
        ("Unison", VOICE, 123, 1, 168),
        ("Unison mode", VOICE, 124, 16, 169),
        ("BPM", ARP, 130, 250, 179),
        ("Clock divide", ARP, 131, 12, 175),
        ("Arp mode", ARP, 132, 4, 173),
        ("Arp range", ARP, 133, 2, 174),
        ("Arp repeats", ARP, 134, 3, 177),
        ("Arp relatch", ARP, 135, 1, 178),
        ("Arp on", ARP, 136, 1, 172),
        ("Sequencer mode", SEQ, 138, 4, 182),
        ("Sequencer type", SEQ, 139, 1, 183),
    ] {
        row(name.into(), section, offset, max, nrpn);
    }
    for i in 0..64 {
        // 0-125 value, 126 reset, 127 rest.
        let name = format!("Gated seq track {} step {}", i / 16 + 1, i % 16 + 1);
        row(name, SEQ, 140 + i, 127, 192 + i);
    }
    row("Unison detune".into(), VOICE, 208, 16, 167);
    row("Pan mode".into(), "Amp", 209, 1, 114);
    for t in 0..6 {
        // Notes: low 7 bits note, bit 7 tie. Velocities: 0 reset,
        // 128 rest, 129-255 velocity 1-127.
        for (what, base) in [("note", 256), ("velocity", 320)] {
            for s in 0..64 {
                let offset = base + 128 * t + s;
                let name = format!("Poly seq track {} {what} {}", t + 1, s + 1);
                row(name, POLY, offset, 255, offset + 20);
            }
        }
    }
    for r in &mut rows {
        if r.name == "BPM" {
            r.min = 30;
        }
    }
    rows
}

static FIELDS: LazyLock<Vec<Field>> = LazyLock::new(|| {
    let rows = layer_a();
    let mut out = vec![];
    for (layer, shift, nrpn_shift) in [("A", 0, 0), ("B", LAYER_B, LAYER_B_NRPN)] {
        for r in &rows {
            // Layer B's copy runs two bytes past the program (poly seq
            // track 6 velocities 63-64): those values are not stored.
            if usize::from(r.offset + shift) >= PROGRAM_LEN {
                continue;
            }
            out.push(Field {
                // Built once per process; the schema lives as long as it.
                name: format!("{layer} {}", r.name).leak(),
                group: format!("{layer} · {}", r.section).leak(),
                offset: r.offset + shift,
                min: r.min,
                max: r.max,
                nrpn: r.nrpn + nrpn_shift,
            });
        }
    }
    for (name, offset, max, nrpn) in [("A/B mode", 231, 2, 163), ("Split point", 232, 120, 171)] {
        out.push(Field {
            name,
            group: "Program",
            offset,
            min: 0,
            max,
            nrpn,
        });
    }
    out
});

/// Every field: layer A, layer B, then the program-wide ones.
pub fn all() -> &'static [Field] {
    &FIELDS
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::program::PROGRAM_LEN;

    fn named(name: &str) -> Option<&'static Field> {
        all().iter().find(|f| f.name == name)
    }

    #[test]
    fn layer_b_schema_skips_missing_steps() {
        assert!(all().iter().all(|f| (f.offset as usize) < PROGRAM_LEN));
        assert_eq!(
            named("A Poly seq track 6 velocity 64").map(|f| f.offset),
            Some(1023)
        );
        assert_eq!(
            named("B Poly seq track 6 velocity 62").map(|f| f.offset),
            Some(2045)
        );
        assert!(named("B Poly seq track 6 velocity 63").is_none());
        assert!(named("B Poly seq track 6 velocity 64").is_none());
        let count = |p: &str| all().iter().filter(|f| f.name.starts_with(p)).count();
        assert_eq!(count("A "), count("B ") + 2);
        for b in all().iter().filter(|f| f.name.starts_with("B ")) {
            let a = named(&format!("A {}", &b.name[2..])).expect("layer A twin");
            assert_eq!((b.offset, b.nrpn), (a.offset + 1024, a.nrpn + 2048));
            assert_eq!((b.min, b.max), (a.min, a.max));
            assert!(b.group.starts_with("B · "), "{}", b.group);
        }
        let wide: Vec<_> = all().iter().filter(|f| f.group == "Program").collect();
        assert_eq!(
            wide.iter().map(|f| (f.offset, f.nrpn)).collect::<Vec<_>>(),
            [(231, 163), (232, 171)]
        );
    }

    #[test]
    fn offsets_are_unique() {
        let mut offs: Vec<u16> = all().iter().map(|f| f.offset).collect();
        offs.sort();
        let n = offs.len();
        offs.dedup();
        assert_eq!(offs.len(), n);
        assert!(n > 1900, "{n} fields");
    }

    #[test]
    fn spot_checks_from_program_format() {
        let f = named("A Filter cutoff").unwrap();
        assert_eq!((f.offset, f.min, f.max, f.nrpn), (22, 0, 164, 15));
        let f = named("A BPM").unwrap();
        assert_eq!((f.offset, f.min, f.max, f.nrpn), (130, 30, 250, 179));
        let f = named("A Mod 8 destination").unwrap();
        assert_eq!((f.offset, f.max, f.nrpn), (100, 52, 67 + 3 * 7));
        let f = named("A Gated seq track 4 step 16").unwrap();
        assert_eq!((f.offset, f.max, f.nrpn), (203, 127, 255));
        let f = named("A Gated seq track 2 destination").unwrap();
        assert_eq!((f.offset, f.max, f.nrpn), (112, 53, 185));
        let f = named("A Poly seq track 2 note 1").unwrap();
        assert_eq!((f.offset, f.max, f.nrpn), (384, 255, 404));
        assert_eq!(named("A LFO 3 shape").unwrap().group, "A · LFOs");
    }
}
