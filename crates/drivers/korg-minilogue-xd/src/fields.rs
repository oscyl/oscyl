//! Program parameter map (TABLE 2 of KORG's "minilogue xd MIDI
//! Implementation").

/// Panel section a parameter belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Group {
    Voice,
    Vco1,
    Vco2,
    Multi,
    Mixer,
    Filter,
    AmpEg,
    Eg,
    Lfo,
    ModFx,
    Delay,
    Reverb,
    Arp,
    Sequencer,
    Misc,
}

/// Where a parameter lives: `width` bytes at `offset` read as a little-endian
/// integer, then `bits` bits starting at bit `shift`. Values are raw stored
/// values; `min..=max` is the documented stored range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FieldSpec {
    pub offset: usize,
    pub width: u8,
    pub shift: u8,
    pub bits: u8,
    pub min: u16,
    pub max: u16,
    pub name: &'static str,
    pub group: Group,
}

// (offset, width, shift, bits)
type Layout = (usize, u8, u8, u8);
const fn byte(o: usize) -> Layout {
    (o, 1, 0, 8)
}
/// Little-endian u16 (KORG's table labels the first byte `H`, but captures
/// show the low byte comes first).
const fn word(o: usize) -> Layout {
    (o, 2, 0, 16)
}
const fn bits(o: usize, shift: u8, n: u8) -> Layout {
    (o, 1, shift, n)
}

macro_rules! fields {
    ($($v:ident => $l:expr, $min:literal, $max:literal, $name:literal, $g:ident;)*) => {
        /// One program parameter.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum Field { $($v),* }

        impl Field {
            pub const ALL: &'static [Field] = &[$(Field::$v),*];

            pub fn spec(self) -> FieldSpec {
                let ((offset, width, shift, bits), min, max, name, group) = match self {
                    $(Field::$v => ($l, $min, $max, $name, Group::$g)),*
                };
                FieldSpec { offset, width, shift, bits, min, max, name, group }
            }
        }
    };
}

fields! {
    Octave => byte(16), 0, 4, "Octave", Voice;
    Portamento => byte(17), 0, 127, "Portamento", Voice;
    KeyTrig => byte(18), 0, 1, "Key Trig", Voice;
    VoiceModeDepth => word(19), 0, 1023, "Voice Mode Depth", Voice;
    VoiceModeType => byte(21), 1, 4, "Voice Mode Type", Voice;
    Vco1Wave => byte(22), 0, 2, "VCO 1 Wave", Vco1;
    Vco1Octave => byte(23), 0, 3, "VCO 1 Octave", Vco1;
    Vco1Pitch => word(24), 0, 1023, "VCO 1 Pitch", Vco1;
    Vco1Shape => word(26), 0, 1023, "VCO 1 Shape", Vco1;
    Vco2Wave => byte(28), 0, 2, "VCO 2 Wave", Vco2;
    Vco2Octave => byte(29), 0, 3, "VCO 2 Octave", Vco2;
    Vco2Pitch => word(30), 0, 1023, "VCO 2 Pitch", Vco2;
    Vco2Shape => word(32), 0, 1023, "VCO 2 Shape", Vco2;
    Sync => byte(34), 0, 1, "Sync", Vco2;
    Ring => byte(35), 0, 1, "Ring", Vco2;
    CrossModDepth => word(36), 0, 1023, "Cross Mod Depth", Vco2;
    MultiType => byte(38), 0, 2, "Multi Type", Multi;
    SelectNoise => byte(39), 0, 3, "Select Noise", Multi;
    SelectVpm => byte(40), 0, 15, "Select VPM", Multi;
    SelectUser => byte(41), 0, 15, "Select User", Multi;
    ShapeNoise => word(42), 0, 1023, "Shape Noise", Multi;
    ShapeVpm => word(44), 0, 1023, "Shape VPM", Multi;
    ShapeUser => word(46), 0, 1023, "Shape User", Multi;
    ShiftShapeNoise => word(48), 0, 1023, "Shift Shape Noise", Multi;
    ShiftShapeVpm => word(50), 0, 1023, "Shift Shape VPM", Multi;
    ShiftShapeUser => word(52), 0, 1023, "Shift Shape User", Multi;
    Vco1Level => word(54), 0, 1023, "VCO 1 Level", Mixer;
    Vco2Level => word(56), 0, 1023, "VCO 2 Level", Mixer;
    MultiLevel => word(58), 0, 1023, "Multi Level", Mixer;
    Cutoff => word(60), 0, 1023, "Cutoff", Filter;
    Resonance => word(62), 0, 1023, "Resonance", Filter;
    CutoffDrive => byte(64), 0, 2, "Cutoff Drive", Filter;
    CutoffKeyTrack => byte(65), 0, 2, "Cutoff Keyboard Track", Filter;
    AmpEgAttack => word(66), 0, 1023, "Amp EG Attack", AmpEg;
    AmpEgDecay => word(68), 0, 1023, "Amp EG Decay", AmpEg;
    AmpEgSustain => word(70), 0, 1023, "Amp EG Sustain", AmpEg;
    AmpEgRelease => word(72), 0, 1023, "Amp EG Release", AmpEg;
    EgAttack => word(74), 0, 1023, "EG Attack", Eg;
    EgDecay => word(76), 0, 1023, "EG Decay", Eg;
    EgInt => word(78), 0, 1023, "EG Int", Eg;
    EgTarget => byte(80), 0, 2, "EG Target", Eg;
    LfoWave => byte(81), 0, 2, "LFO Wave", Lfo;
    LfoMode => byte(82), 0, 2, "LFO Mode", Lfo;
    LfoRate => word(83), 0, 1023, "LFO Rate", Lfo;
    LfoInt => word(85), 0, 1023, "LFO Int", Lfo;
    LfoTarget => byte(87), 0, 2, "LFO Target", Lfo;
    ModFxOn => byte(88), 0, 1, "Mod FX On", ModFx;
    ModFxType => byte(89), 1, 5, "Mod FX Type", ModFx;
    ModFxChorus => byte(90), 0, 7, "Mod FX Chorus", ModFx;
    ModFxEnsemble => byte(91), 0, 2, "Mod FX Ensemble", ModFx;
    ModFxPhaser => byte(92), 0, 7, "Mod FX Phaser", ModFx;
    ModFxFlanger => byte(93), 0, 7, "Mod FX Flanger", ModFx;
    ModFxUser => byte(94), 0, 15, "Mod FX User", ModFx;
    ModFxTime => word(95), 0, 1023, "Mod FX Time", ModFx;
    ModFxDepth => word(97), 0, 1023, "Mod FX Depth", ModFx;
    DelayOn => byte(99), 0, 1, "Delay On", Delay;
    DelaySubType => byte(100), 0, 19, "Delay Sub Type", Delay;
    DelayTime => word(101), 0, 1023, "Delay Time", Delay;
    DelayDepth => word(103), 0, 1023, "Delay Depth", Delay;
    ReverbOn => byte(105), 0, 1, "Reverb On", Reverb;
    ReverbSubType => byte(106), 0, 19, "Reverb Sub Type", Reverb;
    ReverbTime => word(107), 0, 1023, "Reverb Time", Reverb;
    ReverbDepth => word(109), 0, 1023, "Reverb Depth", Reverb;
    BendRangePlus => byte(111), 0, 12, "Bend Range (+)", Misc;
    BendRangeMinus => byte(112), 0, 12, "Bend Range (-)", Misc;
    JoystickAssignPlus => byte(113), 0, 28, "Joystick Assign (+)", Misc;
    JoystickRangePlus => byte(114), 0, 200, "Joystick Range (+)", Misc;
    JoystickAssignMinus => byte(115), 0, 28, "Joystick Assign (-)", Misc;
    JoystickRangeMinus => byte(116), 0, 200, "Joystick Range (-)", Misc;
    CvInMode => byte(117), 0, 2, "CV In Mode", Misc;
    CvIn1Assign => byte(118), 0, 28, "CV In 1 Assign", Misc;
    CvIn1Range => byte(119), 0, 200, "CV In 1 Range", Misc;
    CvIn2Assign => byte(120), 0, 28, "CV In 2 Assign", Misc;
    CvIn2Range => byte(121), 0, 200, "CV In 2 Range", Misc;
    Microtuning => byte(122), 0, 139, "Micro Tuning", Misc;
    ScaleKey => byte(123), 0, 24, "Scale Key", Misc;
    ProgramTuning => byte(124), 0, 100, "Program Tuning", Misc;
    LfoKeySync => byte(125), 0, 1, "LFO Key Sync", Lfo;
    LfoVoiceSync => byte(126), 0, 1, "LFO Voice Sync", Lfo;
    LfoTargetOsc => byte(127), 0, 3, "LFO Target Osc", Lfo;
    CutoffVelocity => byte(128), 0, 127, "Cutoff Velocity", Filter;
    AmpVelocity => byte(129), 0, 127, "Amp Velocity", AmpEg;
    MultiOctave => byte(130), 0, 3, "Multi Octave", Multi;
    MultiRouting => byte(131), 0, 1, "Multi Routing", Multi;
    EgLegato => byte(132), 0, 1, "EG Legato", Eg;
    PortamentoMode => byte(133), 0, 1, "Portamento Mode", Voice;
    PortamentoBpmSync => byte(134), 0, 1, "Portamento BPM Sync", Voice;
    ProgramLevel => byte(135), 12, 132, "Program Level", Misc;
    VpmParam1 => byte(136), 0, 200, "VPM Feedback", Multi;
    VpmParam2 => byte(137), 0, 200, "VPM Noise Depth", Multi;
    VpmParam3 => byte(138), 0, 200, "VPM Shape Mod Int", Multi;
    VpmParam4 => byte(139), 0, 200, "VPM Mod Attack", Multi;
    VpmParam5 => byte(140), 0, 200, "VPM Mod Decay", Multi;
    VpmParam6 => byte(141), 0, 200, "VPM Mod Key Track", Multi;
    UserParam1 => byte(142), 0, 200, "User Param 1", Multi;
    UserParam2 => byte(143), 0, 200, "User Param 2", Multi;
    UserParam3 => byte(144), 0, 200, "User Param 3", Multi;
    UserParam4 => byte(145), 0, 200, "User Param 4", Multi;
    UserParam5 => byte(146), 0, 200, "User Param 5", Multi;
    UserParam6 => byte(147), 0, 200, "User Param 6", Multi;
    UserParam5Type => bits(148, 0, 2), 0, 3, "User Param 5 Type", Multi;
    UserParam6Type => bits(148, 2, 2), 0, 3, "User Param 6 Type", Multi;
    UserParam1Type => bits(149, 0, 2), 0, 3, "User Param 1 Type", Multi;
    UserParam2Type => bits(149, 2, 2), 0, 3, "User Param 2 Type", Multi;
    UserParam3Type => bits(149, 4, 2), 0, 3, "User Param 3 Type", Multi;
    UserParam4Type => bits(149, 6, 2), 0, 3, "User Param 4 Type", Multi;
    ProgramTranspose => byte(150), 1, 25, "Program Transpose", Misc;
    DelayDryWet => word(151), 0, 1024, "Delay Dry/Wet", Delay;
    ReverbDryWet => word(153), 0, 1024, "Reverb Dry/Wet", Reverb;
    AftertouchAssign => byte(155), 0, 28, "MIDI Aftertouch Assign", Misc;
    SeqActiveSteps => word(162), 0, 65535, "Seq Active Steps", Sequencer;
    SeqBpm => word(164), 100, 3000, "Seq BPM", Sequencer;
    SeqStepLength => byte(166), 1, 16, "Seq Step Length", Sequencer;
    SeqStepResolution => byte(167), 0, 4, "Seq Step Resolution", Sequencer;
    SeqSwing => byte(168), 0, 150, "Seq Swing", Sequencer;
    SeqDefaultGateTime => byte(169), 0, 72, "Seq Default Gate Time", Sequencer;
    SeqStepsOn => word(170), 0, 65535, "Seq Steps On", Sequencer;
    SeqMotionStepsOn => word(172), 0, 65535, "Seq Step Motion On", Sequencer;
    MotionSlot1On => bits(174, 0, 1), 0, 1, "Motion Slot 1 On", Sequencer;
    MotionSlot1Smooth => bits(174, 1, 1), 0, 1, "Motion Slot 1 Smooth", Sequencer;
    MotionSlot1Parameter => byte(175), 0, 129, "Motion Slot 1 Parameter", Sequencer;
    MotionSlot2On => bits(176, 0, 1), 0, 1, "Motion Slot 2 On", Sequencer;
    MotionSlot2Smooth => bits(176, 1, 1), 0, 1, "Motion Slot 2 Smooth", Sequencer;
    MotionSlot2Parameter => byte(177), 0, 129, "Motion Slot 2 Parameter", Sequencer;
    MotionSlot3On => bits(178, 0, 1), 0, 1, "Motion Slot 3 On", Sequencer;
    MotionSlot3Smooth => bits(178, 1, 1), 0, 1, "Motion Slot 3 Smooth", Sequencer;
    MotionSlot3Parameter => byte(179), 0, 129, "Motion Slot 3 Parameter", Sequencer;
    MotionSlot4On => bits(180, 0, 1), 0, 1, "Motion Slot 4 On", Sequencer;
    MotionSlot4Smooth => bits(180, 1, 1), 0, 1, "Motion Slot 4 Smooth", Sequencer;
    MotionSlot4Parameter => byte(181), 0, 129, "Motion Slot 4 Parameter", Sequencer;
    MotionSlot1Steps => word(182), 0, 65535, "Motion Slot 1 Steps On", Sequencer;
    MotionSlot2Steps => word(184), 0, 65535, "Motion Slot 2 Steps On", Sequencer;
    MotionSlot3Steps => word(186), 0, 65535, "Motion Slot 3 Steps On", Sequencer;
    MotionSlot4Steps => word(188), 0, 65535, "Motion Slot 4 Steps On", Sequencer;
    ArpGateTime => byte(1022), 0, 72, "Arp Gate Time", Arp;
    ArpRate => byte(1023), 0, 10, "Arp Rate", Arp;
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::program::{MARKERS, RAW};

    /// Per-byte bit masks claimed by one spec.
    fn masks(s: &FieldSpec) -> Vec<(usize, u8)> {
        let m = ((1u32 << s.bits) - 1) << s.shift;
        (0..s.width as usize)
            .map(|i| (s.offset + i, (m >> (8 * i)) as u8))
            .filter(|&(_, b)| b != 0)
            .collect()
    }

    #[test]
    fn no_overlaps_and_full_coverage() {
        let mut used = [0u8; 1024];
        let mut claim = |off: usize, m: u8, what: &str| {
            assert_eq!(used[off] & m, 0, "{what} overlaps at byte {off}");
            used[off] |= m;
        };
        for (off, tag) in MARKERS {
            for i in 0..tag.len() {
                claim(off + i, 0xFF, "marker");
            }
        }
        for (r, what) in RAW {
            for off in r.clone() {
                claim(off, 0xFF, what);
            }
        }
        for &f in Field::ALL {
            let s = f.spec();
            assert!(s.min <= s.max && s.bits <= 8 * s.width, "{}", s.name);
            for (off, m) in masks(&s) {
                claim(off, m, s.name);
            }
        }
        let uncovered: Vec<usize> = (0..1024).filter(|&i| used[i] == 0).collect();
        assert!(uncovered.is_empty(), "uncovered bytes {uncovered:?}");
    }
}
