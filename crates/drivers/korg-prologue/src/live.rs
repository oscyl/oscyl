//! Live parameter changes as Control Change messages (MIDI Implementation
//! rev. 1.01, CC table and notes *5-01…*5-20). The prologue has no NRPN.
//!
//! - Continuous 10-bit fields send their upper 7 bits (the prologue has no
//!   CC 63 for the lower bits), so a CC lands on the nearest multiple of 8.
//! - Choices send the centre of their value band.
//! - Only the main timbre: its CCs go on the global channel the contract
//!   passes. The sub timbre listens on the "MIDI Sub CC Ch" global setting,
//!   which the contract does not carry, so sub-timbre fields return `None`
//!   and the app auditions the whole program instead.
//! - Fields without a CC (the name, VPM and user parameters, most program
//!   settings) return `None` too.
//!
//! Every message needs the synth's "MIDI Rx CC" global setting On.

use oscyl_logue::kinds::cc::{
    DELAY_TYPES, EIGHTHS, HALVES, QUARTERS, REVERB_TYPES, SIXTEENTHS, THIRDS,
};

use crate::Program;
use crate::fields::{Field, field};

/// One Control Change: (controller, value).
pub type Cc = (u8, u8);

/// The CCs that set `f` to its value in `p`, or `None`.
pub fn messages(f: &Field, p: &Program) -> Option<Vec<Cc>> {
    if f.timbre == Some(1) {
        return None;
    }
    let v = p.get(f);
    let wide = |cc: u8| Some(vec![(cc, (v.min(1023) >> 3) as u8)]);
    let seven = |cc: u8| Some(vec![(cc, v.min(127) as u8)]);
    let pick = |cc: u8, table: &[u8]| {
        table
            .get(usize::from(v.saturating_sub(f.min)))
            .map(|&value| vec![(cc, value)])
    };
    let main = |name: &str| p.get(field(&format!("Main {name}")));
    if f.timbre.is_none() {
        let modfx = p.get(field("Mod FX Type"));
        let dlrv = p.get(field("Delay/Reverb Type"));
        return match f.name {
            "Balance" => seven(8),
            "Timbre Type" => pick(86, &THIRDS),
            "Mod FX Speed" => wide(28),
            "Mod FX Depth" => wide(29),
            // 1-based: chorus, ensemble, phaser, flanger; user has no band.
            "Mod FX Type" => pick(88, &[0, 42, 70, 99]),
            "Mod FX On" => pick(92, &HALVES),
            "Mod FX Chorus" if modfx == 1 => pick(96, &EIGHTHS),
            "Mod FX Ensemble" if modfx == 2 => pick(96, &THIRDS),
            "Mod FX Phaser" if modfx == 3 => pick(96, &EIGHTHS),
            "Mod FX Flanger" if modfx == 4 => pick(96, &EIGHTHS),
            "Mod FX User" if modfx == 5 => pick(96, &SIXTEENTHS),
            "Delay/Reverb Time" => wide(30),
            "Delay/Reverb Depth" => wide(31),
            "Delay/Reverb On" => pick(94, &HALVES),
            // Stored 0 off, 1 delay, 2 reverb; the CC knows delay, reverb.
            "Delay/Reverb Type" => (v > 0).then(|| vec![(89, HALVES[usize::from(v - 1)])]),
            "Delay/Reverb Delay Type" if dlrv == 1 => pick(97, &DELAY_TYPES),
            "Delay/Reverb Reverb Type" if dlrv == 2 => pick(97, &REVERB_TYPES),
            // Stored + 1 since 2.00; 0 is "not set" and has no CC value.
            "Delay/Reverb Dry/Wet" if v > 0 => Some(vec![(111, ((v - 1).min(1023) >> 3) as u8)]),
            _ => None,
        };
    }
    let multi = main("Multi Engine Type");
    match (f.section, f.label) {
        ("Voice", "Portamento") => seven(5),
        ("Voice", "Voice Spread") => seven(14),
        ("Voice", "Depth") => wide(27),
        ("Amp EG", "Attack") => wide(16),
        ("Amp EG", "Decay") => wide(17),
        ("Amp EG", "Sustain") => wide(18),
        ("Amp EG", "Release") => wide(19),
        ("EG", "Attack") => wide(20),
        ("EG", "Decay") => wide(21),
        ("EG", "Sustain") => wide(22),
        ("EG", "Release") => wide(23),
        ("LFO", "Rate") => wide(24),
        ("LFO", "Int") => wide(26),
        ("LFO", "Target") => pick(56, &THIRDS),
        ("LFO", "Wave") => pick(57, &THIRDS),
        ("LFO", "Mode") => pick(58, &THIRDS),
        ("Mixer", "Multi") => wide(33),
        ("Mixer", "VCO 1") => wide(39),
        ("Mixer", "VCO 2") => wide(40),
        ("VCO 1", "Pitch") => wide(34),
        ("VCO 2", "Pitch") => wide(35),
        ("VCO 1", "Shape") => wide(36),
        ("VCO 2", "Shape") => wide(37),
        ("VCO 1", "Octave") => pick(48, &QUARTERS),
        ("VCO 2", "Octave") => pick(49, &QUARTERS),
        ("VCO 1", "Wave") => pick(50, &THIRDS),
        ("VCO 2", "Wave") => pick(51, &THIRDS),
        ("VCO 2", "Cross Mod") => wide(41),
        ("VCO 2", "Ring/Sync") => pick(80, &THIRDS),
        ("Pitch EG", "Int") => wide(42),
        ("Pitch EG", "Target") => pick(81, &THIRDS),
        ("Filter", "Cutoff") => wide(43),
        ("Filter", "Resonance") => wide(44),
        ("Filter", "EG Int") => wide(45),
        ("Filter", "Low Cut") => pick(82, &HALVES),
        ("Filter", "Key Track") => pick(83, &THIRDS),
        ("Filter", "Drive") => pick(84, &THIRDS),
        ("Multi Engine", "Octave") => pick(52, &QUARTERS),
        ("Multi Engine", "Type") => pick(53, &THIRDS),
        // CC 54 and 104 move the shape of the engine in use; a hidden
        // engine's shape changes nothing audible, so it waits for a dump.
        ("Multi Engine", "Noise Shape") if multi == 0 => wide(54),
        ("Multi Engine", "VPM Shape") if multi == 1 => wide(54),
        ("Multi Engine", "User Shape") if multi == 2 => wide(54),
        ("Multi Engine", "VPM Shift") if multi == 1 => wide(104),
        ("Multi Engine", "User Shift") if multi == 2 => wide(104),
        ("Multi Engine", "Noise Type") if multi == 0 => pick(103, &EIGHTHS[..4]),
        ("Multi Engine", "VPM Type") if multi == 1 => pick(103, &SIXTEENTHS),
        ("Multi Engine", "User Osc") if multi == 2 => pick(103, &SIXTEENTHS),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fields::all;

    fn with(name: &str, v: u16) -> (Program, &'static Field) {
        let mut p = Program::init();
        let f = field(name);
        p.set(f, v).unwrap();
        (p, f)
    }

    #[test]
    fn cc_map_matches_the_midi_implementation() {
        let (p, f) = with("Main Filter Cutoff", 1023);
        assert_eq!(messages(f, &p), Some(vec![(43, 127)]));
        let (p, f) = with("Main VCO 1 Wave", 2);
        assert_eq!(messages(f, &p), Some(vec![(50, 127)]));
        let (p, f) = with("Main VCO 2 Ring/Sync", 0);
        assert_eq!(messages(f, &p), Some(vec![(80, 0)]));
        let (p, f) = with("Main EG Release", 512);
        assert_eq!(messages(f, &p), Some(vec![(23, 64)]));
        let (p, f) = with("Delay/Reverb Type", 2);
        assert_eq!(messages(f, &p), Some(vec![(89, 127)]));
        let (p, f) = with("Delay/Reverb Type", 0);
        assert_eq!(messages(f, &p), None);
        let (p, f) = with("Mod FX Type", 5);
        assert_eq!(messages(f, &p), None, "user has no CC band");
        let (p, f) = with("Delay/Reverb Dry/Wet", 0);
        assert_eq!(messages(f, &p), None, "not set");
    }

    #[test]
    fn noise_types_use_16_wide_bands() {
        // *5-20: HIGH 0-15, LOW 16-31, PEAK 32-47, DECIM 48-63.
        let got: Vec<_> = (0..4)
            .map(|v| {
                let (p, f) = with("Main Multi Engine Noise Type", v);
                messages(f, &p).unwrap()[0]
            })
            .collect();
        assert_eq!(got, [(103, 0), (103, 16), (103, 32), (103, 48)]);
    }

    #[test]
    fn mod_fx_type_and_subtypes_send_from_their_own_bytes() {
        // Values inside the prologue's 4 bands and the xd's 5.
        let types: Vec<_> = (1..=4)
            .map(|v| {
                let (p, f) = with("Mod FX Type", v);
                messages(f, &p).unwrap()[0]
            })
            .collect();
        assert_eq!(types, [(88, 0), (88, 42), (88, 70), (88, 99)]);
        // The flanger subtype is byte 47 and goes out on CC 96 only while
        // the flanger is the type.
        let mut p = Program::init();
        p.set(field("Mod FX Type"), 4).unwrap();
        p.set(field("Mod FX Flanger"), 7).unwrap();
        assert_eq!(p.as_bytes()[47], 7);
        assert_eq!(messages(field("Mod FX Flanger"), &p), Some(vec![(96, 127)]));
        assert_eq!(messages(field("Mod FX Phaser"), &p), None);
        p.set(field("Mod FX Type"), 2).unwrap();
        p.set(field("Mod FX Ensemble"), 2).unwrap();
        assert_eq!(p.as_bytes()[48], 2);
        assert_eq!(
            messages(field("Mod FX Ensemble"), &p),
            Some(vec![(96, 127)])
        );
    }

    #[test]
    fn sub_timbre_has_no_live_messages() {
        let p = Program::init();
        for f in all().iter().filter(|f| f.timbre == Some(1)) {
            assert_eq!(messages(f, &p), None, "{}", f.name);
        }
        assert!(messages(field("Main Filter Cutoff"), &p).is_some());
    }

    #[test]
    fn hidden_engine_shapes_wait_for_a_dump() {
        let (p, f) = with("Main Multi Engine Type", 1);
        assert!(messages(field("Main Multi Engine VPM Shape"), &p).is_some());
        assert_eq!(messages(field("Main Multi Engine Noise Shape"), &p), None);
        assert_eq!(messages(f, &p), Some(vec![(53, 64)]));
    }
}
