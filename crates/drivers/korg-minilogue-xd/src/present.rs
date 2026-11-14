//! Platform-neutral presentation of program parameters: how the editor
//! groups fields into pages and sections, which control each field gets,
//! compact labels, and human-readable values.
//!
//! Value meanings follow KORG's MIDI Implementation, TABLE 2 notes P2–P24 and
//! S4. Where KORG gives no
//! unit, the raw stored value is shown rather than an invented one.

use oscyl_logue::kinds::{
    ARP_RATES, CHORDS, CHORUS_TYPES, DELAY_TYPES, ENSEMBLE_TYPES, FEET, FLANGER_TYPES,
    LFO_BPM_RATES, LFO_TARGET_OSC, LFO_TARGETS, MOD_FX_TYPES, MULTI_ROUTING, MULTI_TYPES,
    NOISE_TYPES, PHASER_TYPES, REVERB_TYPES, THIRDS, USER_PARAM_TYPES, USER_SLOTS, VPM_TYPES,
    WAVES, gate_percent, level_db, microtuning, signed,
};

use crate::{Field, Program};

pub use oscyl_logue::kinds::{eg_int_percent, pitch_cents};

/// A top-level editor page.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Stage {
    Voice,
    Oscillators,
    Mixer,
    Filter,
    Envelopes,
    Effects,
}

/// A titled group of fields within a [`Stage`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Section {
    pub title: &'static str,
    pub fields: &'static [Field],
}

/// Fields deliberately absent from every stage: the sequencer (it gets its
/// own view) and the user-oscillator parameter types, which are metadata set
/// by the loaded user oscillator rather than panel controls.
pub const EXCLUDED: &[Field] = {
    use Field::*;
    &[
        SeqActiveSteps,
        SeqBpm,
        SeqStepLength,
        SeqStepResolution,
        SeqSwing,
        SeqDefaultGateTime,
        SeqStepsOn,
        SeqMotionStepsOn,
        MotionSlot1On,
        MotionSlot1Smooth,
        MotionSlot1Parameter,
        MotionSlot2On,
        MotionSlot2Smooth,
        MotionSlot2Parameter,
        MotionSlot3On,
        MotionSlot3Smooth,
        MotionSlot3Parameter,
        MotionSlot4On,
        MotionSlot4Smooth,
        MotionSlot4Parameter,
        MotionSlot1Steps,
        MotionSlot2Steps,
        MotionSlot3Steps,
        MotionSlot4Steps,
        UserParam1Type,
        UserParam2Type,
        UserParam3Type,
        UserParam4Type,
        UserParam5Type,
        UserParam6Type,
    ]
};

impl Stage {
    pub const ALL: [Stage; 6] = [
        Stage::Voice,
        Stage::Oscillators,
        Stage::Mixer,
        Stage::Filter,
        Stage::Envelopes,
        Stage::Effects,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Stage::Voice => "Voice",
            Stage::Oscillators => "Oscillators",
            Stage::Mixer => "Mixer",
            Stage::Filter => "Filter",
            Stage::Envelopes => "Envelopes",
            Stage::Effects => "Effects",
        }
    }

    /// Symbolic icon name; the apps ship matching icons.
    pub fn icon_name(self) -> &'static str {
        match self {
            Stage::Voice => "oscyl-voice-symbolic",
            Stage::Oscillators => "oscyl-oscillator-symbolic",
            Stage::Mixer => "oscyl-mixer-symbolic",
            Stage::Filter => "oscyl-filter-symbolic",
            Stage::Envelopes => "oscyl-envelope-symbolic",
            Stage::Effects => "oscyl-effects-symbolic",
        }
    }

    pub fn sections(self) -> &'static [Section] {
        use Field::*;
        match self {
            Stage::Voice => &[
                Section {
                    title: "Voice",
                    fields: &[
                        VoiceModeType,
                        VoiceModeDepth,
                        Octave,
                        Portamento,
                        PortamentoMode,
                        PortamentoBpmSync,
                        KeyTrig,
                    ],
                },
                Section {
                    title: "Arpeggiator",
                    fields: &[ArpRate, ArpGateTime],
                },
                Section {
                    title: "Program",
                    fields: &[
                        ProgramLevel,
                        ProgramTuning,
                        ProgramTranspose,
                        Microtuning,
                        ScaleKey,
                        BendRangePlus,
                        BendRangeMinus,
                    ],
                },
                Section {
                    title: "Controllers",
                    fields: &[
                        JoystickAssignPlus,
                        JoystickRangePlus,
                        JoystickAssignMinus,
                        JoystickRangeMinus,
                        CvInMode,
                        CvIn1Assign,
                        CvIn1Range,
                        CvIn2Assign,
                        CvIn2Range,
                        AftertouchAssign,
                    ],
                },
            ],
            Stage::Oscillators => &[
                Section {
                    title: "VCO 1",
                    fields: &[Vco1Wave, Vco1Octave, Vco1Pitch, Vco1Shape],
                },
                Section {
                    title: "VCO 2",
                    fields: &[
                        Vco2Wave,
                        Vco2Octave,
                        Vco2Pitch,
                        Vco2Shape,
                        Sync,
                        Ring,
                        CrossModDepth,
                    ],
                },
                Section {
                    title: "Multi Engine",
                    fields: &[
                        MultiType,
                        MultiOctave,
                        MultiRouting,
                        SelectNoise,
                        SelectVpm,
                        SelectUser,
                        ShapeNoise,
                        ShapeVpm,
                        ShapeUser,
                        ShiftShapeNoise,
                        ShiftShapeVpm,
                        ShiftShapeUser,
                        VpmParam1,
                        VpmParam2,
                        VpmParam3,
                        VpmParam4,
                        VpmParam5,
                        VpmParam6,
                        UserParam1,
                        UserParam2,
                        UserParam3,
                        UserParam4,
                        UserParam5,
                        UserParam6,
                    ],
                },
            ],
            Stage::Mixer => &[Section {
                title: "Mixer",
                fields: &[Vco1Level, Vco2Level, MultiLevel],
            }],
            Stage::Filter => &[Section {
                title: "Filter",
                fields: &[
                    Cutoff,
                    Resonance,
                    CutoffDrive,
                    CutoffKeyTrack,
                    CutoffVelocity,
                ],
            }],
            Stage::Envelopes => &[
                Section {
                    title: "Amp EG",
                    fields: &[
                        AmpEgAttack,
                        AmpEgDecay,
                        AmpEgSustain,
                        AmpEgRelease,
                        AmpVelocity,
                    ],
                },
                Section {
                    title: "EG",
                    fields: &[EgAttack, EgDecay, EgInt, EgTarget, EgLegato],
                },
                Section {
                    title: "LFO",
                    fields: &[
                        LfoWave,
                        LfoMode,
                        LfoRate,
                        LfoInt,
                        LfoTarget,
                        LfoTargetOsc,
                        LfoKeySync,
                        LfoVoiceSync,
                    ],
                },
            ],
            Stage::Effects => &[
                Section {
                    title: "Mod FX",
                    fields: &[
                        ModFxOn,
                        ModFxType,
                        ModFxChorus,
                        ModFxEnsemble,
                        ModFxPhaser,
                        ModFxFlanger,
                        ModFxUser,
                        ModFxTime,
                        ModFxDepth,
                    ],
                },
                Section {
                    title: "Delay",
                    fields: &[DelayOn, DelaySubType, DelayTime, DelayDepth, DelayDryWet],
                },
                Section {
                    title: "Reverb",
                    fields: &[
                        ReverbOn,
                        ReverbSubType,
                        ReverbTime,
                        ReverbDepth,
                        ReverbDryWet,
                    ],
                },
            ],
        }
    }
}

/// The kind of widget a field wants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Control {
    /// A continuous or wide numeric range.
    Dial,
    /// A named enumeration: `names[raw - spec.min]`. Raw values past the end
    /// (documented range wider than KORG's name list) display as numbers.
    Choice(&'static [&'static str]),
    /// Off / On.
    Toggle,
}

const ASSIGNS: &[&str] = &[
    "Gate Time",
    "Portamento",
    "V.M Depth",
    "VCO 1 Pitch",
    "VCO 1 Shape",
    "VCO 2 Pitch",
    "VCO 2 Shape",
    "Cross Mod",
    "Multi Shape",
    "VCO 1 Level",
    "VCO 2 Level",
    "Multi Level",
    "Cutoff",
    "Resonance",
    "Amp EG Attack",
    "Amp EG Decay",
    "Amp EG Sustain",
    "Amp EG Release",
    "EG Attack",
    "EG Decay",
    "EG Int",
    "LFO Rate",
    "LFO Int",
    "Mod FX Speed",
    "Mod FX Depth",
    "Reverb Time",
    "Reverb Depth",
    "Delay Time",
    "Delay Depth",
];
const ARP_PATTERNS: &[&str] = &[
    "Manual 1",
    "Manual 2",
    "Rise 1",
    "Rise 2",
    "Fall 1",
    "Fall 2",
    "Rise Fall 1",
    "Rise Fall 2",
    "Poly 1",
    "Poly 2",
    "Random 1",
    "Random 2",
    "Random 3",
];

pub fn control(field: Field) -> Control {
    use Control::*;
    use Field::*;
    match field {
        KeyTrig | Sync | Ring | ModFxOn | DelayOn | ReverbOn | LfoKeySync | LfoVoiceSync
        | EgLegato | PortamentoBpmSync | MotionSlot1On | MotionSlot1Smooth | MotionSlot2On
        | MotionSlot2Smooth | MotionSlot3On | MotionSlot3Smooth | MotionSlot4On
        | MotionSlot4Smooth => Toggle,
        Octave => Choice(&["-2", "-1", "0", "+1", "+2"]),
        VoiceModeType => Choice(&["Arp/Latch", "Chord", "Unison", "Poly"]),
        Vco1Wave | Vco2Wave | LfoWave => Choice(WAVES),
        Vco1Octave | Vco2Octave | MultiOctave => Choice(FEET),
        CutoffDrive | CutoffKeyTrack => Choice(THIRDS),
        MultiType => Choice(MULTI_TYPES),
        SelectNoise => Choice(NOISE_TYPES),
        SelectVpm => Choice(VPM_TYPES),
        SelectUser | ModFxUser => Choice(USER_SLOTS),
        MultiRouting => Choice(MULTI_ROUTING),
        PortamentoMode => Choice(&["Auto", "On"]),
        EgTarget => Choice(&["Cutoff", "Pitch 2", "Pitch"]),
        LfoMode => Choice(&["1-Shot", "Normal", "BPM"]),
        LfoTarget => Choice(LFO_TARGETS),
        LfoTargetOsc => Choice(LFO_TARGET_OSC),
        ModFxType => Choice(MOD_FX_TYPES),
        ModFxChorus => Choice(CHORUS_TYPES),
        ModFxEnsemble => Choice(ENSEMBLE_TYPES),
        ModFxPhaser => Choice(PHASER_TYPES),
        ModFxFlanger => Choice(FLANGER_TYPES),
        DelaySubType => Choice(DELAY_TYPES),
        ReverbSubType => Choice(REVERB_TYPES),
        JoystickAssignPlus | JoystickAssignMinus | CvIn1Assign | CvIn2Assign | AftertouchAssign => {
            Choice(ASSIGNS)
        }
        CvInMode => Choice(&["Modulation", "CV/Gate (+)", "CV/Gate (-)"]),
        ArpRate => Choice(ARP_RATES),
        UserParam1Type | UserParam2Type | UserParam3Type | UserParam4Type | UserParam5Type
        | UserParam6Type => Choice(USER_PARAM_TYPES),
        SeqStepResolution => Choice(&["1/16", "1/8", "1/4", "1/2", "1/1"]),
        _ => Dial,
    }
}

/// Compact label for use inside a titled [`Section`]; falls back to the
/// field's full name.
pub fn short_label(field: Field) -> &'static str {
    use Field::*;
    match field {
        VoiceModeType => "Mode",
        VoiceModeDepth => "Depth",
        PortamentoMode => "Porta Mode",
        PortamentoBpmSync => "Porta Sync",
        ArpRate => "Rate",
        ArpGateTime => "Gate",
        ProgramLevel => "Level",
        ProgramTuning => "Tuning",
        ProgramTranspose => "Transpose",
        BendRangePlus => "Bend +",
        BendRangeMinus => "Bend -",
        JoystickAssignPlus => "Joy + Assign",
        JoystickRangePlus => "Joy + Range",
        JoystickAssignMinus => "Joy - Assign",
        JoystickRangeMinus => "Joy - Range",
        CvIn1Assign => "CV 1 Assign",
        CvIn1Range => "CV 1 Range",
        CvIn2Assign => "CV 2 Assign",
        CvIn2Range => "CV 2 Range",
        AftertouchAssign => "Aftertouch",
        Vco1Wave | Vco2Wave | LfoWave => "Wave",
        Vco1Octave | Vco2Octave | MultiOctave => "Octave",
        Vco1Pitch | Vco2Pitch => "Pitch",
        Vco1Shape | Vco2Shape => "Shape",
        CrossModDepth => "Cross Mod",
        MultiType => "Type",
        MultiRouting => "Routing",
        SelectNoise => "Noise Type",
        SelectVpm => "VPM Type",
        SelectUser => "User Osc",
        ShapeNoise => "Noise Shape",
        ShapeVpm => "VPM Shape",
        ShapeUser => "User Shape",
        ShiftShapeNoise => "Noise Shift",
        ShiftShapeVpm => "VPM Shift",
        ShiftShapeUser => "User Shift",
        VpmParam1 => "Feedback",
        VpmParam2 => "Noise Depth",
        VpmParam3 => "Shape Mod",
        VpmParam4 => "Mod Attack",
        VpmParam5 => "Mod Decay",
        VpmParam6 => "Mod Key Track",
        UserParam1 => "Param 1",
        UserParam2 => "Param 2",
        UserParam3 => "Param 3",
        UserParam4 => "Param 4",
        UserParam5 => "Param 5",
        UserParam6 => "Param 6",
        Vco1Level => "VCO 1",
        Vco2Level => "VCO 2",
        MultiLevel => "Multi",
        CutoffDrive => "Drive",
        CutoffKeyTrack => "Key Track",
        CutoffVelocity | AmpVelocity => "Velocity",
        AmpEgAttack | EgAttack => "Attack",
        AmpEgDecay | EgDecay => "Decay",
        AmpEgSustain => "Sustain",
        AmpEgRelease => "Release",
        EgInt | LfoInt => "Int",
        EgTarget | LfoTarget => "Target",
        EgLegato => "Legato",
        LfoMode => "Mode",
        LfoRate => "Rate",
        LfoTargetOsc => "Target Osc",
        LfoKeySync => "Key Sync",
        LfoVoiceSync => "Voice Sync",
        ModFxOn | DelayOn | ReverbOn => "On",
        ModFxType => "Type",
        ModFxChorus => "Chorus",
        ModFxEnsemble => "Ensemble",
        ModFxPhaser => "Phaser",
        ModFxFlanger => "Flanger",
        ModFxUser => "User",
        DelaySubType | ReverbSubType => "Type",
        ModFxTime | DelayTime | ReverbTime => "Time",
        ModFxDepth | DelayDepth | ReverbDepth => "Depth",
        DelayDryWet | ReverbDryWet => "Dry/Wet",
        _ => field.spec().name,
    }
}

/// Human-readable value of `raw` for `field`, from `raw` alone. Fields whose
/// meaning depends on another field (voice mode depth, LFO rate in BPM mode,
/// user oscillator parameters) show the raw number; use [`format_in`] for
/// those.
pub fn format_value(field: Field, raw: u16) -> String {
    use Field::*;
    match field {
        Vco1Pitch | Vco2Pitch => signed(pitch_cents(raw).round() as i32, " c"),
        EgInt => signed(eg_int_percent(raw).round() as i32, "%"),
        JoystickRangePlus | JoystickRangeMinus | CvIn1Range | CvIn2Range | VpmParam1
        | VpmParam2 | VpmParam3 | VpmParam4 | VpmParam5 | VpmParam6 => {
            signed(raw as i32 - 100, "%")
        }
        ProgramLevel => level_db(raw),
        ProgramTuning => signed(raw as i32 - 50, " c"),
        ScaleKey => signed(raw as i32 - 12, ""),
        ProgramTranspose => signed(raw as i32 - 13, ""),
        BendRangePlus if raw > 0 => format!("+{raw}"),
        BendRangeMinus if raw > 0 => format!("-{raw}"),
        BendRangePlus | BendRangeMinus => "Off".into(),
        ArpGateTime | SeqDefaultGateTime => gate_percent(raw),
        SeqBpm => format!("{}.{}", raw / 10, raw % 10),
        SeqSwing => signed(raw as i32 - 75, ""),
        Microtuning => microtuning(raw),
        _ => match control(field) {
            Control::Toggle => if raw == 0 { "Off" } else { "On" }.into(),
            Control::Choice(names) => raw
                .checked_sub(field.spec().min)
                .and_then(|i| names.get(i as usize))
                .map_or_else(|| raw.to_string(), |s| (*s).into()),
            Control::Dial => raw.to_string(),
        },
    }
}

/// Like [`format_value`] but reads context from the program: voice mode depth
/// per voice mode type (note P2), LFO rate as a note length in BPM mode
/// (P11), and user oscillator parameters per their type (P24).
pub fn format_in(p: &Program, field: Field) -> String {
    use Field::*;
    let raw = p.get(field);
    let user_type = match field {
        UserParam1 => Some(UserParam1Type),
        UserParam2 => Some(UserParam2Type),
        UserParam3 => Some(UserParam3Type),
        UserParam4 => Some(UserParam4Type),
        UserParam5 => Some(UserParam5Type),
        UserParam6 => Some(UserParam6Type),
        _ => None,
    };
    if let Some(t) = user_type {
        return match p.get(t) {
            0 => format!("{raw}%"),
            1 => signed(raw as i32 - 100, ""),
            2 => (raw + 1).to_string(),
            _ => raw.to_string(),
        };
    }
    let pick = |names: &[&str], width: u16| {
        names
            .get((raw / width) as usize)
            .copied()
            .unwrap_or(names[names.len() - 1])
            .to_string()
    };
    match field {
        VoiceModeDepth => match p.get(VoiceModeType) {
            1 => pick(ARP_PATTERNS, 78),
            2 => pick(CHORDS, 73),
            3 => format!("{} c", (raw as u32 * 50 + 511) / 1023),
            // Labelled "Duo" (`label_in`): the amount, or Off.
            4 if raw < 256 => "Off".into(),
            4 => (raw - 256).to_string(),
            _ => raw.to_string(),
        },
        LfoRate if p.get(LfoMode) == 2 => pick(LFO_BPM_RATES, 64),
        _ => format_value(field, raw),
    }
}

/// In Arp and Chord modes the voice mode depth picks a named entry: the
/// names and the width of each entry's band of raw values.
pub fn depth_choices(voice_mode: u16) -> Option<(&'static [&'static str], u16)> {
    match voice_mode {
        1 => Some((ARP_PATTERNS, 78)),
        2 => Some((CHORDS, 73)),
        _ => None,
    }
}

/// Which named entry a raw depth selects.
pub fn depth_index(raw: u16, (names, width): (&[&str], u16)) -> usize {
    ((raw / width) as usize).min(names.len() - 1)
}

/// The raw depth for entry `i`: the middle of its band, so the synth reads
/// it the same way whatever its rounding.
pub fn depth_raw(i: usize, (_, width): (&[&str], u16)) -> u16 {
    (i as u16 * width + width / 2).min(1023)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn depth_entries_round_trip() {
        for mode in [1, 2] {
            let c = depth_choices(mode).unwrap();
            let mut p = Program::init();
            p.set(Field::VoiceModeType, mode).unwrap();
            for (i, name) in c.0.iter().enumerate() {
                let raw = depth_raw(i, c);
                assert_eq!(depth_index(raw, c), i);
                p.set(Field::VoiceModeDepth, raw).unwrap();
                assert_eq!(format_in(&p, Field::VoiceModeDepth), *name);
            }
        }
        assert!(depth_choices(3).is_none() && depth_choices(4).is_none());
    }

    #[test]
    fn every_field_placed_exactly_once() {
        for &f in Field::ALL {
            let n = Stage::ALL
                .iter()
                .flat_map(|s| s.sections())
                .flat_map(|s| s.fields)
                .chain(EXCLUDED)
                .filter(|&&g| g == f)
                .count();
            assert_eq!(n, 1, "{:?} placed {n} times", f);
        }
    }

    #[test]
    fn choices_cover_documented_range() {
        // Every choice list names at least the documented values KORG lists;
        // ranges wider than the list (reverb 18–19) fall back to numbers.
        for &f in Field::ALL {
            if let Control::Choice(names) = control(f) {
                let s = f.spec();
                assert!(!names.is_empty());
                assert!(names.len() <= (s.max - s.min + 1) as usize, "{:?}", f);
            }
        }
        assert_eq!(format_value(Field::ReverbSubType, 19), "19");
        assert_eq!(format_value(Field::DelaySubType, 19), "User 8");
    }

    #[test]
    fn pitch_edges() {
        let f = Field::Vco1Pitch;
        assert_eq!(format_value(f, 0), "-1200 c");
        assert_eq!(format_value(f, 4), "-1200 c");
        assert_eq!(format_value(f, 356), "-256 c");
        assert_eq!(format_value(f, 492), "0 c");
        assert_eq!(format_value(f, 512), "0 c");
        assert_eq!(format_value(f, 532), "0 c");
        assert_eq!(format_value(f, 540), "+8 c");
        assert_eq!(format_value(f, 668), "+256 c");
        assert_eq!(format_value(f, 1023), "+1200 c");
    }

    #[test]
    fn percent_and_level_edges() {
        assert_eq!(format_value(Field::EgInt, 0), "-100%");
        assert_eq!(format_value(Field::EgInt, 512), "0%");
        assert_eq!(format_value(Field::EgInt, 1023), "+100%");
        assert_eq!(format_value(Field::JoystickRangePlus, 0), "-100%");
        assert_eq!(format_value(Field::VpmParam1, 100), "0%");
        assert_eq!(format_value(Field::CvIn1Range, 200), "+100%");
        assert_eq!(format_value(Field::ProgramLevel, 12), "-18.0 dB");
        assert_eq!(format_value(Field::ProgramLevel, 102), "0.0 dB");
        assert_eq!(format_value(Field::ProgramLevel, 103), "+0.2 dB");
        assert_eq!(format_value(Field::ProgramLevel, 132), "+6.0 dB");
        assert_eq!(format_value(Field::ProgramTuning, 0), "-50 c");
        assert_eq!(format_value(Field::ArpGateTime, 72), "100%");
        assert_eq!(format_value(Field::SeqBpm, 1200), "120.0");
    }

    #[test]
    fn names_and_toggles() {
        assert_eq!(format_value(Field::VoiceModeType, 1), "Arp/Latch");
        assert_eq!(format_value(Field::VoiceModeType, 4), "Poly");
        assert_eq!(format_value(Field::Vco1Wave, 2), "Saw");
        assert_eq!(format_value(Field::Octave, 2), "0");
        assert_eq!(format_value(Field::ModFxType, 1), "Chorus");
        assert_eq!(format_value(Field::CutoffDrive, 1), "50%");
        assert_eq!(format_value(Field::Sync, 1), "On");
        assert_eq!(format_value(Field::BendRangeMinus, 0), "Off");
        assert_eq!(format_value(Field::BendRangeMinus, 12), "-12");
        assert_eq!(format_value(Field::Microtuning, 134), "User Octave 1");
        assert_eq!(format_value(Field::Cutoff, 700), "700");
    }

    #[test]
    fn context_dependent() {
        let mut p = Program::init();
        assert_eq!(format_in(&p, Field::VoiceModeDepth), "Off");
        p.set(Field::VoiceModeDepth, 1023).unwrap();
        assert_eq!(format_in(&p, Field::VoiceModeDepth), "767");
        p.set(Field::VoiceModeType, 2).unwrap();
        assert_eq!(format_in(&p, Field::VoiceModeDepth), "Maj7b5");
        p.set(Field::VoiceModeType, 1).unwrap();
        assert_eq!(format_in(&p, Field::VoiceModeDepth), "Random 3");
        p.set(Field::VoiceModeType, 3).unwrap();
        assert_eq!(format_in(&p, Field::VoiceModeDepth), "50 c");
        p.set(Field::LfoMode, 2).unwrap();
        p.set(Field::LfoRate, 1023).unwrap();
        assert_eq!(format_in(&p, Field::LfoRate), "1/36");
        p.set(Field::UserParam1Type, 1).unwrap();
        assert_eq!(format_in(&p, Field::UserParam1), "-100");
    }
}

/// Parameters editors draw with a larger dial, in field order.
pub const PRIMARY: [Field; 4] = [
    Field::Vco1Shape,
    Field::Vco2Shape,
    Field::Cutoff,
    Field::Resonance,
];

/// The label for `field` under `p`'s voice mode: the voice mode depth is a
/// pattern, a chord, a detune or a duo amount. Other fields keep their
/// [`short_label`].
pub fn label_in(p: &Program, field: Field) -> &'static str {
    match (field, p.get(Field::VoiceModeType)) {
        (Field::VoiceModeDepth, 1) => "Pattern",
        (Field::VoiceModeDepth, 2) => "Chord",
        (Field::VoiceModeDepth, 3) => "Detune",
        (Field::VoiceModeDepth, _) => "Duo",
        _ => short_label(field),
    }
}

/// A one-line description of `stage`'s state in `p`, in English: the long
/// form of the editor's stage summary.
pub fn summary(stage: Stage, p: &Program) -> String {
    summary_in(stage, p, false, &|s| s.to_string())
}

/// The editor's stage summary: long, or `compact` for narrow columns
/// ("Cut 618 · Res 255", the mixer's levels alone). Every phrase and value
/// name goes through `tr`; templates are filled after translating.
pub fn summary_in(stage: Stage, p: &Program, compact: bool, tr: &dyn Fn(&str) -> String) -> String {
    use Field::*;
    let f = |field| tr(&format_in(p, field));
    match stage {
        Stage::Voice if p.get(Octave) == Octave.spec().max / 2 => f(VoiceModeType),
        Stage::Voice if compact => format!("{} {}", f(VoiceModeType), f(Octave)),
        Stage::Voice => tr("{mode} · Octave {octave}")
            .replace("{mode}", &f(VoiceModeType))
            .replace("{octave}", &f(Octave)),
        Stage::Oscillators => format!("{} + {}", f(Vco1Wave), f(Vco2Wave)),
        Stage::Mixer => {
            let on: Vec<String> = [
                (Vco1Level, "VCO 1"),
                (Vco2Level, "VCO 2"),
                (MultiLevel, "Multi"),
            ]
            .into_iter()
            .filter(|(l, _)| p.get(*l) > 0)
            .map(|(l, n)| {
                let level = (u32::from(p.get(l)) * 100 + 511) / 1023;
                if compact {
                    format!("{level}%")
                } else {
                    format!("{} {level}%", tr(n))
                }
            })
            .collect();
            if on.is_empty() {
                tr("Silent")
            } else {
                on.join(" · ")
            }
        }
        Stage::Filter => tr(if compact {
            "Cut {cutoff} · Res {resonance}"
        } else {
            "Cutoff {cutoff} · Res {resonance}"
        })
        .replace("{cutoff}", &f(Cutoff))
        .replace("{resonance}", &f(Resonance)),
        Stage::Envelopes => tr("A {attack} · R {release}")
            .replace("{attack}", &f(AmpEgAttack))
            .replace("{release}", &f(AmpEgRelease)),
        Stage::Effects => {
            let on: Vec<String> = [(ModFxOn, "Mod"), (DelayOn, "Delay"), (ReverbOn, "Reverb")]
                .into_iter()
                .filter(|(s, _)| p.get(*s) != 0)
                .map(|(_, n)| tr(n))
                .collect();
            if on.is_empty() {
                tr("All off")
            } else {
                on.join(" · ")
            }
        }
    }
}

/// Whether `field` does anything under `p`'s current settings. Editors hide
/// controls that do not (for example the phaser type while the chorus is
/// selected), so the visible controls are exactly the ones that change the
/// sound.
pub fn relevant(p: &Program, field: Field) -> bool {
    use Field::*;
    // Multi type: 0 Noise, 1 VPM, 2 User.
    let multi = p.get(MultiType);
    // Mod FX type: 1 Chorus, 2 Ensemble, 3 Phaser, 4 Flanger, 5 User.
    let modfx = p.get(ModFxType);
    match field {
        SelectNoise | ShapeNoise | ShiftShapeNoise => multi == 0,
        SelectVpm | ShapeVpm | ShiftShapeVpm | VpmParam1 | VpmParam2 | VpmParam3 | VpmParam4
        | VpmParam5 | VpmParam6 => multi == 1,
        SelectUser | ShapeUser | ShiftShapeUser | UserParam1 | UserParam2 | UserParam3
        | UserParam4 | UserParam5 | UserParam6 => multi == 2,
        ModFxChorus => modfx == 1,
        ModFxEnsemble => modfx == 2,
        ModFxPhaser => modfx == 3,
        ModFxFlanger => modfx == 4,
        ModFxUser => modfx == 5,
        _ => true,
    }
}

#[cfg(test)]
mod relevance_tests {
    use super::*;

    #[test]
    fn only_the_selected_engine_and_effect_subtype_are_relevant() {
        let mut p = Program::init();
        p.set(Field::MultiType, 1).unwrap();
        p.set(Field::ModFxType, 3).unwrap();
        assert!(relevant(&p, Field::ShapeVpm));
        assert!(!relevant(&p, Field::ShapeNoise));
        assert!(relevant(&p, Field::ModFxPhaser));
        assert!(!relevant(&p, Field::ModFxChorus));
        assert!(relevant(&p, Field::Cutoff));
    }
}

/// What kind of sound a program is, guessed from its settings, for the
/// program list. Deterministic, so a program keeps its category until an
/// edit changes the settings it reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Category {
    Bass,
    Lead,
    Pad,
    Keys,
    Pluck,
    Bell,
    Arp,
    Fx,
    Drone,
}

/// Guess `p`'s [`Category`] from its voice mode, octaves, amp envelope,
/// pitch modulation and sources. Checked in order: arpeggiator, pitch or
/// noise effects, slow swells, low octaves, short envelopes, mono leads,
/// and keys for the rest.
// ponytail: fixed thresholds tuned on the demo library; a learned
// classifier only if real libraries prove these wrong often.
pub fn category(p: &Program) -> Category {
    use Field::*;
    let g = |f| p.get(f);
    let audible = |level| g(level) >= 100;
    let vcos_silent = !audible(Vco1Level) && !audible(Vco2Level);
    let multi_leads = g(MultiLevel) > g(Vco1Level).max(g(Vco2Level));
    // EG target 2 bends the whole pitch; LFO target 2 is pitch.
    let pitch_sweep = g(EgTarget) == 2 && g(EgInt).abs_diff(512) > 150;
    let siren = g(LfoTarget) == 2 && g(LfoInt) > 300;
    let noise_only = vcos_silent && g(MultiType) == 0 && audible(MultiLevel);
    let low = g(Octave) < 2
        || (g(Vco1Octave) == 0
            && audible(Vco1Level)
            && !(audible(Vco2Level) && g(Vco2Octave) >= 2));
    let metallic = g(Ring) == 1 || g(CrossModDepth) > 300 || (g(MultiType) == 1 && multi_leads);
    let (attack, decay, sustain, release) = (
        g(AmpEgAttack),
        g(AmpEgDecay),
        g(AmpEgSustain),
        g(AmpEgRelease),
    );
    if g(VoiceModeType) == 1 {
        Category::Arp
    } else if pitch_sweep || siren || noise_only {
        Category::Fx
    } else if attack >= 300 && sustain >= 800 {
        if g(Octave) < 2 || (sustain >= 1000 && release >= 900) {
            Category::Drone
        } else {
            Category::Pad
        }
    } else if low {
        Category::Bass
    } else if sustain < 256 && decay < 500 {
        Category::Pluck
    } else if sustain < 256 && decay >= 760 && metallic {
        Category::Bell
    } else if g(VoiceModeType) == 3 || g(Portamento) > 0 {
        Category::Lead
    } else {
        Category::Keys
    }
}

#[cfg(test)]
mod category_tests {
    use super::*;
    use Category::*;

    #[test]
    fn init_program_is_keys() {
        assert_eq!(category(&Program::init()), Keys);
    }

    #[test]
    fn voice_mode_and_octave_decide_first() {
        let mut p = Program::init();
        p.set(Field::VoiceModeType, 1).unwrap();
        assert_eq!(category(&p), Arp);
        p.set(Field::VoiceModeType, 3).unwrap();
        assert_eq!(category(&p), Lead);
        p.set(Field::Octave, 0).unwrap();
        assert_eq!(category(&p), Bass);
    }

    #[test]
    fn envelope_shapes() {
        let mut p = Program::init();
        p.set(Field::AmpEgAttack, 600).unwrap();
        assert_eq!(category(&p), Pad);
        p.set(Field::AmpEgRelease, 1000).unwrap();
        assert_eq!(category(&p), Drone);
        let mut p = Program::init();
        p.set(Field::AmpEgSustain, 0).unwrap();
        p.set(Field::AmpEgDecay, 300).unwrap();
        assert_eq!(category(&p), Pluck);
        p.set(Field::AmpEgDecay, 900).unwrap();
        assert_eq!(category(&p), Keys);
        p.set(Field::Ring, 1).unwrap();
        assert_eq!(category(&p), Bell);
    }

    #[test]
    fn pitch_drops_and_noise_are_effects() {
        let mut p = Program::init();
        p.set(Field::EgTarget, 2).unwrap();
        p.set(Field::EgInt, 900).unwrap();
        assert_eq!(category(&p), Fx);
        let mut p = Program::init();
        p.set(Field::Vco1Level, 0).unwrap();
        p.set(Field::MultiLevel, 1023).unwrap();
        assert_eq!(category(&p), Fx);
    }
}
