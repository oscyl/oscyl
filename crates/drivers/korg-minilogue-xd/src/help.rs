//! Plain-language help for every program parameter, for tooltips and help
//! popovers. British English; written to read well aloud.

use crate::fields::Field;
use crate::present::Stage;

/// Help text for one parameter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Help {
    /// One line, at most 90 characters: what the control does to the sound.
    pub summary: &'static str,
    /// A non-obvious tip or interaction, or empty.
    pub detail: &'static str,
}

const fn h(summary: &'static str, detail: &'static str) -> Help {
    Help { summary, detail }
}

/// Introduction to a stage of the signal path, shown under its heading.
pub fn stage_help(stage: Stage) -> &'static str {
    match stage {
        Stage::Voice => {
            "Choose how the four voices play: Poly, Unison, Chord or Arp/Latch. Glide and program-wide tuning are here too."
        }
        Stage::Oscillators => {
            "VCO 1 and VCO 2 are analogue; the multi engine is digital and plays noise, VPM or a user oscillator."
        }
        Stage::Mixer => {
            "Set the level of each source going into the filter. Higher levels also drive the filter harder."
        }
        Stage::Filter => {
            "A low-pass filter. The EG and LFO can both move the cutoff while a note plays."
        }
        Stage::Envelopes => {
            "The Amp EG shapes the volume of each note. The EG and LFO move cutoff, pitch or shape."
        }
        Stage::Effects => "Mod FX, delay and reverb each have their own switch, type and settings.",
    }
}

/// Help for one parameter. `detail` is empty when there is nothing to add.
pub fn help(field: Field) -> Help {
    use Field::*;
    match field {
        Octave => h("Shifts the whole keyboard up or down by octaves.", ""),
        Portamento => h("Glide time between notes.", "Zero turns glide off."),
        KeyTrig => h(
            "Restarts the arpeggio from its first step when you press a new key.",
            "",
        ),
        VoiceModeDepth => h(
            "Sets the detail of the voice mode; its job changes with the mode.",
            "Poly: above the first quarter it stacks a second, detuned voice (Duo). Unison: detune, up to 50 cents. Chord: the chord type. Arp/Latch: the arpeggio pattern.",
        ),
        VoiceModeType => h(
            "How the four voices are used: Poly, Unison, Chord or Arp/Latch.",
            "Unison puts all four voices on one note, so the program plays one note at a time.",
        ),
        Vco1Wave => h("Waveform of VCO 1: square, triangle or sawtooth.", ""),
        Vco1Octave => h("Pitch range of VCO 1, from 16' (lowest) to 2'.", ""),
        Vco1Pitch => h(
            "Fine-tunes VCO 1, up to an octave either way.",
            "Small offsets against VCO 2 make the two beat against each other.",
        ),
        Vco1Shape => h(
            "Reshapes the VCO 1 wave.",
            "On square it sets the pulse width; on triangle and sawtooth it adds harmonics.",
        ),
        Vco2Wave => h("Waveform of VCO 2: square, triangle or sawtooth.", ""),
        Vco2Octave => h("Pitch range of VCO 2, from 16' (lowest) to 2'.", ""),
        Vco2Pitch => h(
            "Fine-tunes VCO 2, up to an octave either way.",
            "With Sync on, sweep this for the classic sync sound.",
        ),
        Vco2Shape => h(
            "Reshapes the VCO 2 wave.",
            "Set LFO Target to Shape to move it over time.",
        ),
        Sync => h(
            "Restarts VCO 2's cycle with every cycle of VCO 1.",
            "The note follows VCO 1; VCO 2 Pitch then changes the tone instead of the pitch.",
        ),
        Ring => h(
            "Ring-modulates VCO 2 with VCO 1 for metallic, bell-like tones.",
            "Change VCO 2 Pitch to change the overtones.",
        ),
        CrossModDepth => h(
            "Lets VCO 1 modulate the pitch of VCO 2.",
            "Low amounts add bite; high amounts turn noisy and metallic.",
        ),
        MultiType => h(
            "Source for the multi engine: Noise, VPM or a user oscillator.",
            "",
        ),
        SelectNoise => h(
            "Noise type: High, Low, Peak or Decim.",
            "High and Low are high-pass and low-pass filtered noise; Decim is noise at a reduced sample rate.",
        ),
        SelectVpm => h(
            "VPM waveform, from plain sines to Air, Decay and Throat types.",
            "",
        ),
        SelectUser => h(
            "Which installed user oscillator unit the multi engine plays.",
            "Empty slots make no sound.",
        ),
        ShapeNoise => h(
            "Main noise control; what it sets depends on the noise type.",
            "High and Low: the filter cutoff. Decim: the sample rate.",
        ),
        ShapeVpm => h("Modulation depth of the VPM tone, from pure to harsh.", ""),
        ShapeUser => h(
            "Main control of the user oscillator; the unit decides what it does.",
            "",
        ),
        ShiftShapeNoise => h(
            "Second noise control (Shift and Shape).",
            "On High and Low it sets the resonance of the noise filter.",
        ),
        ShiftShapeVpm => h(
            "VPM modulator ratio, which sets the overtones you hear.",
            "Settings between whole-number ratios give inharmonic, bell-like tones.",
        ),
        ShiftShapeUser => h(
            "Second control of the user oscillator (Shift and Shape).",
            "",
        ),
        Vco1Level => h("Level of VCO 1 going into the filter.", ""),
        Vco2Level => h(
            "Level of VCO 2 going into the filter.",
            "Turn it to zero to use VCO 2 only for Sync, Ring or Cross Mod.",
        ),
        MultiLevel => h(
            "How loud the multi engine is in the mix.",
            "A little noise under the VCOs adds breath to the attack.",
        ),
        Cutoff => h("Filter cutoff: lower settings darken the sound.", ""),
        Resonance => h(
            "Boosts frequencies around the cutoff.",
            "High settings make filter sweeps whistle.",
        ),
        CutoffDrive => h("Overdrives the filter input: 0%, 50% or 100%.", ""),
        CutoffKeyTrack => h(
            "Opens the filter further for higher notes: 0%, 50% or 100%.",
            "At 0% high notes sound duller than low ones.",
        ),
        AmpEgAttack => h("Time for the volume to rise after you press a key.", ""),
        AmpEgDecay => h(
            "Time for the volume to fall from its peak to the sustain level.",
            "No effect unless Sustain is below maximum.",
        ),
        AmpEgSustain => h(
            "Volume held while you keep a key down.",
            "At zero every note dies away after the decay, like a plucked string.",
        ),
        AmpEgRelease => h("Time for the sound to fade after you let go.", ""),
        EgAttack => h("Rise time of the EG after a key press.", ""),
        EgDecay => h(
            "Fall time of the EG back to zero.",
            "The EG has no sustain stage, so this sets the length of the sweep.",
        ),
        EgInt => h(
            "Amount and direction of the EG's effect on its target.",
            "Centre is zero; below centre the sweep is inverted.",
        ),
        EgTarget => h(
            "What the EG moves: Cutoff, Pitch 2 (VCO 2 only) or Pitch.",
            "Pitch 2 with Sync on gives a sync sweep on every note.",
        ),
        LfoWave => h("LFO waveform: square, triangle or sawtooth.", ""),
        LfoMode => h(
            "LFO behaviour: 1-Shot, Normal or BPM.",
            "1-Shot runs one cycle per note, like an extra envelope. BPM locks the rate to the tempo.",
        ),
        LfoRate => h(
            "How fast the LFO cycles.",
            "In BPM mode it picks a note value instead.",
        ),
        LfoInt => h("How far the LFO moves its target.", ""),
        LfoTarget => h(
            "What the LFO moves: Cutoff, Shape or Pitch.",
            "Pitch gives vibrato; Shape on a square wave gives pulse-width modulation.",
        ),
        ModFxOn => h("Switches Mod FX on or off.", ""),
        ModFxType => h(
            "Mod FX type: Chorus, Ensemble, Phaser, Flanger or User.",
            "",
        ),
        ModFxChorus => h(
            "Chorus flavour, from Light and Deep to Triphase and Vibrato.",
            "",
        ),
        ModFxEnsemble => h("Ensemble variation: Stereo, Light or Mono.", ""),
        ModFxPhaser => h(
            "Phaser flavour, from a fast sweep to vowel-like Formant.",
            "",
        ),
        ModFxFlanger => h(
            "Flanger flavour, from Light to jet-like High and Pan sweeps.",
            "",
        ),
        ModFxUser => h(
            "Which installed user Mod FX unit is used.",
            "Empty slots pass the sound through.",
        ),
        ModFxTime => h("How fast the chorus, phaser or flanger sweeps.", ""),
        ModFxDepth => h(
            "How strongly the sweep bends the sound, from a gentle shimmer to a deep swirl.",
            "",
        ),
        DelayOn => h("Switches the delay on or off.", ""),
        DelaySubType => h(
            "Delay type, from stereo and ping-pong to tape and tempo-synced versions.",
            "BPM types set the delay time as a note value at the program tempo.",
        ),
        DelayTime => h("Time between echoes.", ""),
        DelayDepth => h("Feedback: how long the echoes keep repeating.", ""),
        DelayDryWet => h("Balance between the dry sound and the echoes.", ""),
        ReverbOn => h("Switches the reverb on or off.", ""),
        ReverbSubType => h("Reverb type, such as Hall, Plate, Room or Space.", ""),
        ReverbTime => h("How long the reverb rings on after the sound stops.", ""),
        ReverbDepth => h(
            "How dense the reverb is: higher settings blur the tail into a wash.",
            "",
        ),
        ReverbDryWet => h("Balance between the dry sound and the reverb.", ""),
        BendRangePlus => h(
            "Upward pitch-bend range, in semitones, when you push the joystick right.",
            "",
        ),
        BendRangeMinus => h(
            "Downward pitch-bend range, in semitones, when you push the joystick left.",
            "",
        ),
        JoystickAssignPlus => h("Parameter the joystick moves when pushed up.", ""),
        JoystickRangePlus => h(
            "How far pushing the joystick up moves its parameter.",
            "Below centre the joystick moves the parameter the other way.",
        ),
        JoystickAssignMinus => h("Parameter the joystick moves when pulled down.", ""),
        JoystickRangeMinus => h("How far pulling the joystick down moves its parameter.", ""),
        CvInMode => h(
            "Use of the rear CV inputs: Modulation, or CV/Gate with either polarity.",
            "In CV/Gate mode you can play the synth from a modular system.",
        ),
        CvIn1Assign => h(
            "Parameter that CV IN 1 controls.",
            "Only used when CV In Mode is Modulation.",
        ),
        CvIn1Range => h("How far CV IN 1 moves its parameter.", ""),
        CvIn2Assign => h(
            "Parameter that CV IN 2 controls.",
            "Only used when CV In Mode is Modulation.",
        ),
        CvIn2Range => h("How far CV IN 2 moves its parameter.", ""),
        Microtuning => h(
            "Tuning system: equal temperament, a preset scale or a user tuning.",
            "",
        ),
        ScaleKey => h(
            "Root note of the microtuning scale.",
            "No effect in equal temperament.",
        ),
        ProgramTuning => h(
            "Fine-tunes the whole program by up to 50 cents either way.",
            "",
        ),
        LfoKeySync => h("Restarts the LFO cycle on every new note.", ""),
        LfoVoiceSync => h(
            "Keeps the LFO in the same phase across all voices.",
            "Off lets each voice move on its own.",
        ),
        LfoTargetOsc => h(
            "Which sources the LFO affects: All, VCO 1+2, VCO 2 or Multi.",
            "Only used when LFO Target is Pitch or Shape.",
        ),
        CutoffVelocity => h("How much harder playing opens the filter.", ""),
        AmpVelocity => h(
            "How much harder playing makes notes louder.",
            "At zero every note plays at the same level.",
        ),
        MultiOctave => h("Pitch range of the multi engine, from 16' to 2'.", ""),
        MultiRouting => h(
            "Sends the multi engine before the filter (Pre VCF) or after it (Post VCF).",
            "Post VCF keeps the multi engine bright while the VCOs are filtered.",
        ),
        EgLegato => h("Stops the EG restarting on notes played legato.", ""),
        PortamentoMode => h(
            "Auto glides only between overlapping notes; On glides on every note.",
            "",
        ),
        PortamentoBpmSync => h("Sets glide time as a note value at the program tempo.", ""),
        ProgramLevel => h(
            "Output level of this program.",
            "Use it to match the volume of programs you switch between.",
        ),
        VpmParam1 => h("VPM feedback: adds edge to the VPM tone.", ""),
        VpmParam2 => h("Adds noise to the VPM modulation.", ""),
        VpmParam3 => h(
            "How far the VPM envelope moves VPM Shape during each note.",
            "Raise it for struck attacks that settle into a softer tone.",
        ),
        VpmParam4 => h("Attack time of the VPM envelope.", ""),
        VpmParam5 => h("Decay time of the VPM envelope.", ""),
        VpmParam6 => h("How much VPM Shape follows the note you play.", ""),
        UserParam1 | UserParam2 | UserParam3 | UserParam4 | UserParam5 | UserParam6 => h(
            "A parameter of the current user oscillator.",
            "The unit sets its name and meaning.",
        ),
        ProgramTranspose => h("Transposes this program by up to an octave either way.", ""),
        AftertouchAssign => h(
            "Parameter moved by aftertouch from an external MIDI controller.",
            "The minilogue xd keyboard does not send aftertouch.",
        ),
        ArpGateTime => h("Length of each arpeggio note.", ""),
        ArpRate => h("Arpeggio note value, relative to the tempo.", ""),
        // Excluded from the panel (see `present::EXCLUDED`); brief text for completeness.
        SeqActiveSteps | SeqStepsOn | SeqMotionStepsOn | SeqStepLength => {
            h("Which sequencer steps play.", SEQ)
        }
        SeqBpm => h(
            "Program tempo.",
            "Also sets the timing for synced LFO, delay and arpeggio.",
        ),
        SeqStepResolution => h("Note length of each sequencer step.", SEQ),
        SeqSwing => h("Delays every other step for a shuffled groove.", SEQ),
        SeqDefaultGateTime => h("Default length of notes recorded into the sequencer.", SEQ),
        MotionSlot1On | MotionSlot2On | MotionSlot3On | MotionSlot4On | MotionSlot1Smooth
        | MotionSlot2Smooth | MotionSlot3Smooth | MotionSlot4Smooth | MotionSlot1Parameter
        | MotionSlot2Parameter | MotionSlot3Parameter | MotionSlot4Parameter | MotionSlot1Steps
        | MotionSlot2Steps | MotionSlot3Steps | MotionSlot4Steps => {
            h("Recorded knob movement played back by the sequencer.", SEQ)
        }
        UserParam1Type | UserParam2Type | UserParam3Type | UserParam4Type | UserParam5Type
        | UserParam6Type => h(
            "How a user oscillator parameter is displayed.",
            "Set by the user oscillator unit.",
        ),
    }
}

const SEQ: &str = "Set on the synth's sequencer; Oscyl keeps it unchanged.";

#[cfg(test)]
mod tests {
    use super::*;
    use crate::present::EXCLUDED;
    use std::collections::HashSet;

    #[test]
    fn every_panel_field_has_help() {
        let mut seen = HashSet::new();
        for &f in Field::ALL.iter().filter(|f| !EXCLUDED.contains(f)) {
            let Help { summary, detail } = help(f);
            assert!(
                !summary.is_empty() && summary.chars().count() <= 90,
                "{f:?}: {summary}"
            );
            assert!(detail.chars().count() <= 320, "{f:?}");
            let user = matches!(
                f,
                Field::UserParam1
                    | Field::UserParam2
                    | Field::UserParam3
                    | Field::UserParam4
                    | Field::UserParam5
                    | Field::UserParam6
            );
            assert!(user || seen.insert(summary), "duplicate summary for {f:?}");
        }
        for s in Stage::ALL {
            assert!(!stage_help(s).is_empty());
        }
    }

    #[test]
    fn copy_follows_the_house_style() {
        let mut all: Vec<&str> = Stage::ALL.iter().map(|&s| stage_help(s)).collect();
        for &f in Field::ALL {
            all.extend([help(f).summary, help(f).detail]);
        }
        for s in all {
            for bad in ["\u{2014}", "&", "patch", "color", "center", "eight voices"] {
                assert!(!s.contains(bad), "{bad:?} in {s:?}");
            }
        }
    }
}
