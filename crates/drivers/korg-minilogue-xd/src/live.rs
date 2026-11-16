//! Live parameter changes as MIDI Control Change and NRPN messages.
//!
//! Auditioning a whole program (a current-program dump) makes the synth
//! reload it, which cuts held notes. Most parameters can instead be sent
//! the way the synth's own panel sends them, as Control Change messages,
//! and the synth applies them to the sounding voices without a gap. This
//! module maps a field's value to those messages, following the KORG
//! minilogue xd MIDI Implementation (rev. 1.01):
//! - continuous 10-bit values: CC 63 carries the lower 3 bits, then the
//!   parameter's CC carries the upper 7 (note *5-4);
//! - selectors: the CC value at the centre of each choice's band (notes
//!   *6-01..);
//! - NRPN parameters: CC 99/98 select the parameter, CC 6 carries the value
//!   (CC 63 first for the lower bits of 8 and 10-bit values, notes
//!   *3-1/*3-2), then the NRPN selection is reset to null.
//!
//! A field with no live message (or a choice the synth's table does not
//! name) returns `None`; the caller then auditions the whole program.
//! Every message needs the synth's "MIDI Rx CC" global setting On (the
//! default).

use crate::fields::Field;
use crate::program::Program;
use oscyl_logue::kinds::cc::{
    DELAY_TYPES, EIGHTHS, HALVES, QUARTERS, REVERB_TYPES, SIXTEENTHS, THIRDS,
};

/// One Control Change: (controller, value), both 0..=127. The channel is
/// added by the transport.
pub type Cc = (u8, u8);

/// CC 63: the lower bits of the next value.
const LSB: u8 = 63;

/// The messages that set `field` to its value in `p` on a live voice, or
/// `None` when it can only be auditioned as a whole program.
pub fn messages(field: Field, p: &Program) -> Option<Vec<Cc>> {
    use Field::*;
    let v = p.get(field);
    let wide = |cc: u8| Some(ten_bit(cc, v.min(1023)));
    let pick = |cc: u8, table: &[u8]| {
        table
            .get(usize::from(v.saturating_sub(field.spec().min)))
            .map(|&value| vec![(cc, value)])
    };
    match field {
        // Continuous controls with their own CC.
        AmpEgAttack => wide(16),
        AmpEgDecay => wide(17),
        AmpEgSustain => wide(18),
        AmpEgRelease => wide(19),
        EgAttack => wide(20),
        EgDecay => wide(21),
        EgInt => wide(22),
        LfoRate => wide(24),
        LfoInt => wide(26),
        VoiceModeDepth => wide(27),
        ModFxTime => wide(28),
        ModFxDepth => wide(29),
        MultiLevel => wide(33),
        Vco1Pitch => wide(34),
        Vco2Pitch => wide(35),
        Vco1Shape => wide(36),
        Vco2Shape => wide(37),
        Vco1Level => wide(39),
        Vco2Level => wide(40),
        CrossModDepth => wide(41),
        Cutoff => wide(43),
        Resonance => wide(44),
        DelayTime => wide(105),
        DelayDepth => wide(106),
        DelayDryWet => wide(107),
        ReverbTime => wide(108),
        ReverbDepth => wide(109),
        ReverbDryWet => wide(110),
        Portamento => Some(vec![(5, v.min(127) as u8)]),

        // Selectors with their own CC.
        EgTarget => pick(23, &THIRDS),
        Vco1Octave => pick(48, &QUARTERS),
        Vco2Octave => pick(49, &QUARTERS),
        Vco1Wave => pick(50, &THIRDS),
        Vco2Wave => pick(51, &THIRDS),
        MultiType => pick(53, &THIRDS),
        LfoTarget => pick(56, &THIRDS),
        LfoWave => pick(57, &THIRDS),
        LfoMode => pick(58, &THIRDS),
        Sync => pick(80, &HALVES),
        Ring => pick(81, &HALVES),
        CutoffKeyTrack => pick(83, &THIRDS),
        CutoffDrive => pick(84, &THIRDS),
        ModFxType => pick(88, &[13, 38, 64, 89, 115]),
        DelaySubType => pick(89, &DELAY_TYPES),
        ReverbSubType => pick(90, &REVERB_TYPES),
        ModFxOn => pick(92, &HALVES),
        DelayOn => pick(93, &HALVES),
        ReverbOn => pick(94, &HALVES),

        // A sub type is live only for the effect or engine type in use;
        // a hidden one changes nothing audible, so it waits for a dump.
        ModFxChorus if p.get(ModFxType) == 1 => pick(96, &EIGHTHS),
        ModFxEnsemble if p.get(ModFxType) == 2 => pick(96, &THIRDS),
        ModFxPhaser if p.get(ModFxType) == 3 => pick(96, &EIGHTHS),
        ModFxFlanger if p.get(ModFxType) == 4 => pick(96, &EIGHTHS),
        ModFxUser if p.get(ModFxType) == 5 => pick(96, &SIXTEENTHS),
        SelectNoise if p.get(MultiType) == 0 => pick(103, &QUARTERS),
        SelectVpm if p.get(MultiType) == 1 => pick(103, &SIXTEENTHS),
        SelectUser if p.get(MultiType) == 2 => pick(103, &SIXTEENTHS),

        // NRPN parameters.
        ShapeNoise => Some(nrpn_wide(0x14, v)),
        ShapeVpm => Some(nrpn_wide(0x15, v)),
        ShapeUser => Some(nrpn_wide(0x16, v)),
        ShiftShapeNoise => Some(nrpn_wide(0x17, v)),
        ShiftShapeVpm => Some(nrpn_wide(0x18, v)),
        ShiftShapeUser => Some(nrpn_wide(0x19, v)),
        BendRangePlus => Some(nrpn(0x20, v)),
        BendRangeMinus => Some(nrpn(0x21, v)),
        JoystickAssignPlus => Some(nrpn(0x22, v)),
        JoystickRangePlus => Some(nrpn_byte(0x23, v)),
        JoystickAssignMinus => Some(nrpn(0x24, v)),
        JoystickRangeMinus => Some(nrpn_byte(0x25, v)),
        CvInMode => Some(nrpn(0x28, v)),
        CvIn1Assign => Some(nrpn(0x29, v)),
        CvIn1Range => Some(nrpn_byte(0x2A, v)),
        CvIn2Assign => Some(nrpn(0x2B, v)),
        CvIn2Range => Some(nrpn_byte(0x2C, v)),
        ScaleKey => Some(nrpn(0x31, v)),
        ProgramTuning => Some(nrpn(0x32, v)),
        LfoKeySync => Some(nrpn(0x34, v)),
        LfoVoiceSync => Some(nrpn(0x35, v)),
        LfoTargetOsc => Some(nrpn(0x36, v)),
        CutoffVelocity => Some(nrpn(0x38, v)),
        AmpVelocity => Some(nrpn(0x39, v)),
        MultiOctave => Some(nrpn(0x3A, v)),
        MultiRouting => Some(nrpn(0x3B, v)),
        EgLegato => Some(nrpn(0x3C, v)),
        PortamentoMode => Some(nrpn(0x3D, v)),
        PortamentoBpmSync => Some(nrpn(0x3E, v)),
        // Stored as 12..=132 (-18 dB..+6 dB), sent as 0..=120.
        ProgramLevel => Some(nrpn(0x3F, v.saturating_sub(12))),
        _ => None,
    }
}

/// What the synth's panel sends (its transmitted Control Changes, MIDI
/// Implementation 1-1; "MIDI Tx CC" On), decoded to the field it sets and
/// its value. Fields are found through [`messages`], so what the panel
/// sends and what the app sends can never disagree. Keeps the CC 63 lower
/// bits and the NRPN selection between messages.
#[derive(Debug, Default)]
pub struct Listener {
    lsb: Option<u8>,
    nrpn_msb: Option<u8>,
    nrpn: Option<u8>,
}

/// How a field's live messages name it: its own CC, or an NRPN.
#[derive(Debug, PartialEq, Clone, Copy)]
enum Key {
    Cc(u8),
    Nrpn(u8),
}

fn key(msgs: &[Cc]) -> Option<Key> {
    match msgs {
        [(99, 0), (98, n), ..] => Some(Key::Nrpn(*n)),
        [.., (cc, _)] => Some(Key::Cc(*cc)),
        [] => None,
    }
}

/// The value byte of a field's live messages (not the NRPN selection).
fn value_byte(msgs: &[Cc]) -> Option<u8> {
    msgs.iter()
        .rev()
        .find(|&&(cc, _)| !matches!(cc, 98 | 99 | LSB))
        .map(|&(_, v)| v)
}

impl Listener {
    /// One Control Change from the panel: the field it sets in `p` and
    /// its value, once complete.
    pub fn control(&mut self, cc: u8, v: u8, p: &Program) -> Option<(Field, u16)> {
        match cc {
            LSB => {
                self.lsb = Some(v);
                return None;
            }
            99 => {
                self.nrpn_msb = Some(v);
                self.nrpn = None;
                return None;
            }
            98 => {
                self.nrpn = (self.nrpn_msb == Some(0) && v != 127).then_some(v);
                return None;
            }
            _ => {}
        }
        let lsb = self.lsb.take();
        let (wanted, field) = match cc {
            6 => (Key::Nrpn(self.nrpn?), None),
            // The multi engine's shape knobs set the engine in use's shape.
            54 | 104 => (Key::Cc(cc), multi_shape(cc == 104, p.get(Field::MultiType))),
            // Voice mode depth, sent again without the display.
            59 => (Key::Cc(27), None),
            _ => (Key::Cc(cc), None),
        };
        let f = field.or_else(|| {
            Field::ALL
                .iter()
                .copied()
                .find(|&f| messages(f, p).as_deref().and_then(key) == Some(wanted))
        })?;
        Some((f, decode(f, p, lsb, v)?))
    }
}

/// The shape (or shift shape) field of multi engine `engine`.
fn multi_shape(shift: bool, engine: u16) -> Option<Field> {
    use Field::*;
    Some(match (shift, engine) {
        (false, 0) => ShapeNoise,
        (false, 1) => ShapeVpm,
        (false, 2) => ShapeUser,
        (true, 0) => ShiftShapeNoise,
        (true, 1) => ShiftShapeVpm,
        (true, 2) => ShiftShapeUser,
        _ => return None,
    })
}

/// Field `f`'s value from its value byte `m` (and CC 63's lower bits):
/// for a wide value the bits, else the value whose byte is nearest (a
/// switch's choice, an offset level).
fn decode(f: Field, p: &Program, lsb: Option<u8>, m: u8) -> Option<u16> {
    let s = f.spec();
    let sent = |v: u16| {
        let mut q = p.clone();
        q.set(f, v).ok()?;
        messages(f, &q)
    };
    let wide = sent(s.min)?.iter().any(|&(cc, _)| cc == LSB);
    if wide {
        let m = u16::from(m);
        // Without its lower bits, 7 bits spread over the whole range.
        let low = lsb.map_or(m >> 4, |l| u16::from(l & 7));
        return Some(((m << 3) | low).clamp(s.min, s.max));
    }
    (s.min..=s.max).min_by_key(|&v| {
        sent(v)
            .as_deref()
            .and_then(value_byte)
            .map_or(u8::MAX, |b| b.abs_diff(m))
    })
}

/// A 10-bit value: the lower 3 bits on CC 63, then the upper 7.
fn ten_bit(cc: u8, v: u16) -> Vec<Cc> {
    vec![(LSB, (v & 0x7) as u8), (cc, (v >> 3) as u8)]
}

/// Select NRPN `lsb` (MSB 0), send `data`, then deselect.
fn nrpn_with(lsb: u8, data: &[Cc]) -> Vec<Cc> {
    let mut out = vec![(99, 0), (98, lsb)];
    out.extend_from_slice(data);
    out.extend_from_slice(&[(99, 127), (98, 127)]);
    out
}

/// A 7-bit NRPN value.
fn nrpn(lsb: u8, v: u16) -> Vec<Cc> {
    nrpn_with(lsb, &[(6, v.min(127) as u8)])
}

/// An 8-bit NRPN value (0..=200): lower 3 bits on CC 63, upper 5 on CC 6.
fn nrpn_byte(lsb: u8, v: u16) -> Vec<Cc> {
    let v = v.min(255);
    nrpn_with(lsb, &[(LSB, (v & 0x7) as u8), (6, (v >> 3) as u8)])
}

/// A 10-bit NRPN value: lower 3 bits on CC 63, upper 7 on CC 6.
fn nrpn_wide(lsb: u8, v: u16) -> Vec<Cc> {
    let v = v.min(1023);
    nrpn_with(lsb, &[(LSB, (v & 0x7) as u8), (6, (v >> 3) as u8)])
}

#[cfg(test)]
mod tests {
    use super::*;

    /// What the panel sends, decoded in order against `p`.
    fn heard(p: &Program, ccs: &[Cc]) -> Vec<(Field, u16)> {
        let mut l = Listener::default();
        ccs.iter()
            .filter_map(|&(cc, v)| l.control(cc, v, p))
            .collect()
    }

    /// A knob's 10 bits: the lower 3 on CC 63 first, else the 7 alone
    /// spread over the range.
    #[test]
    fn panel_knobs_decode_with_and_without_lsb() {
        let p = Program::init();
        assert_eq!(heard(&p, &[(63, 5), (43, 85)]), [(Field::Cutoff, 685)]);
        assert_eq!(heard(&p, &[(43, 127)]), [(Field::Cutoff, 1023)]);
        assert_eq!(heard(&p, &[(43, 0)]), [(Field::Cutoff, 0)]);
        assert_eq!(heard(&p, &[(5, 40)]), [(Field::Portamento, 40)]);
    }

    /// Switches send fixed values; each picks its nearest choice.
    #[test]
    fn panel_switches_pick_their_choice() {
        let p = Program::init();
        let min = |f: Field| f.spec().min;
        assert_eq!(
            heard(&p, &[(50, 64)]),
            [(Field::Vco1Wave, min(Field::Vco1Wave) + 1)]
        );
        assert_eq!(
            heard(&p, &[(88, 84)]),
            [(Field::ModFxType, min(Field::ModFxType) + 3)]
        );
        assert_eq!(
            heard(&p, &[(48, 127)]),
            [(Field::Vco1Octave, min(Field::Vco1Octave) + 3)]
        );
    }

    /// The multi engine's shape knobs set the shape of the engine in use.
    #[test]
    fn panel_multi_shape_follows_the_engine() {
        let p = with(Field::MultiType, 1);
        assert_eq!(heard(&p, &[(63, 1), (54, 64)]), [(Field::ShapeVpm, 513)]);
        assert_eq!(heard(&p, &[(104, 0)]), [(Field::ShiftShapeVpm, 0)]);
    }

    /// NRPN parameters: selected on CC 99/98, the value on CC 6 (with CC 63
    /// first for wider ones); the null selection ends it.
    #[test]
    fn panel_nrpn_parameters_decode() {
        let p = Program::init();
        assert_eq!(
            heard(&p, &[(99, 0), (98, 0x38), (6, 1), (99, 127), (98, 127)]),
            [(Field::CutoffVelocity, 1)]
        );
        assert_eq!(
            heard(&p, &[(99, 0), (98, 0x3F), (6, 108)]),
            [(Field::ProgramLevel, 120)]
        );
        assert!(heard(&p, &[(99, 127), (98, 127), (6, 5)]).is_empty());
    }

    /// Performance controls (joystick, hold) and unknown controls set nothing.
    #[test]
    fn panel_performance_controls_set_nothing() {
        let p = Program::init();
        assert!(heard(&p, &[(1, 64), (2, 3), (64, 127), (118, 9)]).is_empty());
    }

    /// Whatever the app sends for a field, decoded, gives the field back
    /// at a value that sends the same (a few ranges end past what 10 bits
    /// carry: Delay Dry/Wet's 1024 is sent as 1023).
    #[test]
    fn every_live_field_round_trips() {
        let p = Program::init();
        for &f in Field::ALL {
            let s = f.spec();
            for v in [s.min, s.max, s.min + (s.max - s.min) / 3] {
                let mut q = p.clone();
                if q.set(f, v).is_err() {
                    continue;
                }
                let Some(sent) = messages(f, &q) else {
                    continue;
                };
                let got = heard(&q, &sent);
                let &(g, w) = got
                    .last()
                    .unwrap_or_else(|| panic!("{f:?} = {v}: {sent:?}"));
                assert_eq!(g, f, "{f:?} = {v}: {sent:?}");
                let mut r = q.clone();
                r.set(f, w).unwrap();
                assert_eq!(messages(f, &r), Some(sent), "{f:?} = {v} came back as {w}");
            }
        }
    }

    fn with(field: Field, v: u16) -> Program {
        let mut p = Program::init();
        p.set(field, v).unwrap();
        p
    }

    #[test]
    fn ten_bit_values_split_lsb_first() {
        let p = with(Field::Cutoff, 0b10_1010_1101);
        assert_eq!(
            messages(Field::Cutoff, &p),
            Some(vec![(63, 0b101), (43, 0b101_0101)])
        );
        let p = with(Field::Cutoff, 1023);
        assert_eq!(messages(Field::Cutoff, &p), Some(vec![(63, 7), (43, 127)]));
    }

    #[test]
    fn every_value_byte_is_seven_bit() {
        for &f in Field::ALL {
            let s = f.spec();
            for v in [s.min, (s.min + s.max) / 2, s.max] {
                let mut p = Program::init();
                if p.set(f, v).is_err() {
                    continue;
                }
                for (cc, val) in messages(f, &p).unwrap_or_default() {
                    assert!(cc < 128 && val < 128, "{f:?} = {v}: ({cc}, {val})");
                }
            }
        }
    }

    #[test]
    fn selectors_use_band_centres() {
        let p = with(Field::Vco1Octave, 2);
        assert_eq!(messages(Field::Vco1Octave, &p), Some(vec![(48, 84)]));
        let p = with(Field::ModFxType, 5);
        assert_eq!(messages(Field::ModFxType, &p), Some(vec![(88, 115)]));
        // Unnamed reverb choices fall back to a program dump.
        let p = with(Field::ReverbSubType, 19);
        assert_eq!(messages(Field::ReverbSubType, &p), None);
    }

    #[test]
    fn hidden_sub_types_wait_for_a_dump() {
        let mut p = Program::init();
        p.set(Field::ModFxType, 1).unwrap();
        p.set(Field::ModFxPhaser, 3).unwrap();
        assert_eq!(messages(Field::ModFxPhaser, &p), None);
        assert!(messages(Field::ModFxChorus, &p).is_some());
    }

    #[test]
    fn nrpn_selects_sends_and_deselects() {
        let p = with(Field::ProgramLevel, 102);
        assert_eq!(
            messages(Field::ProgramLevel, &p),
            Some(vec![(99, 0), (98, 0x3F), (6, 90), (99, 127), (98, 127)])
        );
        let p = with(Field::ShapeVpm, 517);
        assert_eq!(
            messages(Field::ShapeVpm, &p),
            Some(vec![
                (99, 0),
                (98, 0x15),
                (63, 5),
                (6, 64),
                (99, 127),
                (98, 127)
            ])
        );
    }

    #[test]
    fn voice_mode_and_names_need_a_dump() {
        let p = Program::init();
        assert_eq!(messages(Field::VoiceModeType, &p), None);
    }
}
