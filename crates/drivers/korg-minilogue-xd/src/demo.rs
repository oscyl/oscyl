//! An authored demo library, built in code from `Program::init()`.
//!
//! Used by the simulator and for screenshots. No KORG factory data and no
//! user backups are involved: every value comes from a category recipe, the
//! program's own settings below and a tiny per-program xorshift, so the
//! library is identical on every run.

use serde_json::json;

use crate::library::ProgInfo;
use crate::msg::Module;
use crate::unit::{Unit, build_header};
use crate::{Field, Program};
use Field::*;

const PROGRAMMER: &str = "Oscyl demo";

// Raw values, as documented in the KORG MIDI implementation.
const SQR: u16 = 0;
const TRI: u16 = 1;
const SAW: u16 = 2;
const FT16: u16 = 0;
const FT8: u16 = 1;
const FT4: u16 = 2;
const NOISE: u16 = 0;
const VPM: u16 = 1;
const USER: u16 = 2;
const ARP: u16 = 1;
const CHORD: u16 = 2;
const UNISON: u16 = 3;
const POLY: u16 = 4;
const EG_CUTOFF: u16 = 0;
const EG_PITCH2: u16 = 1;
const EG_PITCH: u16 = 2;
const LFO_SHAPE: u16 = 1;
const LFO_PITCH: u16 = 2;
const LFO_BPM: u16 = 2;
const ENSEMBLE: u16 = 2;
const PHASER: u16 = 3;
const FLANGER: u16 = 4;
const USER_FX: u16 = 5;

/// xorshift64, seeded per program.
struct Rng(u64);

impl Rng {
    fn new(seed: usize) -> Rng {
        Rng((seed as u64 + 1).wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }

    /// Uniform-ish value in `lo..=hi`.
    fn range(&mut self, lo: u16, hi: u16) -> u16 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        lo + (self.0 % (hi - lo + 1) as u64) as u16
    }
}

#[derive(Clone, Copy)]
enum Cat {
    Pad,
    Bass,
    Lead,
    Keys,
    Pluck,
    Arp,
    Fx,
    Drone,
}

type Settings = &'static [(Field, u16)];

const PWM: Settings = &[
    (Vco1Wave, SQR),
    (Vco2Wave, SQR),
    (LfoTarget, LFO_SHAPE),
    (LfoTargetOsc, 1),
    (LfoWave, TRI),
    (LfoRate, 350),
    (LfoInt, 450),
];

/// `(category, name, playing tip, settings over the category recipe)`.
#[rustfmt::skip]
const PROGRAMS: [(Cat, &str, &str, Settings); 120] = [
    // Pads
    (Cat::Pad, "Glass Choir", "", &[(Vco1Wave, TRI), (Vco2Wave, TRI), (Vco2Octave, FT4), (MultiType, VPM), (SelectVpm, 10), (MultiLevel, 500), (ModFxType, ENSEMBLE)]),
    (Cat::Pad, "Slow Bloom", "Slow attack: hold chords and let them swell.", &[]),
    (Cat::Pad, "High Canopy", "", &[(Vco2Octave, FT4)]),
    (Cat::Pad, "Turning Tide", "", &[(ModFxType, PHASER)]),
    (Cat::Pad, "Hollow Hall", "", &[(Vco1Wave, SQR), (Vco2Wave, SQR)]),
    (Cat::Pad, "Felt", "", &[(Vco1Wave, TRI), (Vco2Wave, TRI), (Cutoff, 300)]),
    (Cat::Pad, "Breathing", "", PWM),
    (Cat::Pad, "Silk Weave", "", &[(Vco1Wave, SQR), (Vco2Wave, SQR), (LfoTarget, LFO_SHAPE), (LfoTargetOsc, 1), (LfoRate, 520), (LfoInt, 600), (ModFxType, ENSEMBLE)]),
    (Cat::Pad, "Old Strings", "", &[(Vco1Wave, SQR), (Vco2Wave, SQR), (LfoTarget, LFO_SHAPE), (LfoTargetOsc, 1), (LfoRate, 450), (LfoInt, 500), (ModFxType, ENSEMBLE), (AmpEgAttack, 320), (AmpEgRelease, 550)]),
    (Cat::Pad, "Bowed Dusk", "", &[(ModFxType, ENSEMBLE), (AmpEgAttack, 350), (Cutoff, 600)]),
    (Cat::Pad, "String Room", "", &[(Vco2Wave, SQR), (ModFxType, ENSEMBLE), (Cutoff, 700)]),
    (Cat::Pad, "Weather", "The LFO sweeps the filter slowly. Hold a chord to hear it.", &[(LfoTarget, 0), (LfoWave, TRI), (LfoRate, 80), (LfoInt, 500), (Resonance, 450)]),
    (Cat::Pad, "Min7 Haze", "Each key plays a minor seventh chord.", &[(VoiceModeType, CHORD), (VoiceModeDepth, 400)]),
    (Cat::Pad, "Air Veil", "", &[(MultiType, NOISE), (SelectNoise, 1), (MultiLevel, 400)]),
    (Cat::Pad, "Wall of Saws", "Uses the SuperSaw user oscillator in User 3.", &[(MultiType, USER), (SelectUser, 2), (MultiLevel, 800), (Vco1Level, 400), (Vco2Level, 400)]),
    // Basses
    (Cat::Bass, "Deep Floor", "", &[(Vco1Wave, TRI), (Vco2Level, 0), (Cutoff, 300), (Resonance, 0), (EgInt, 560)]),
    (Cat::Bass, "Undertow", "", &[(Vco1Wave, SQR), (Vco1Shape, 0), (Vco2Wave, TRI), (Cutoff, 260)]),
    (Cat::Bass, "Workhorse", "", &[]),
    (Cat::Bass, "Two Floors", "", &[(Vco2Octave, FT8)]),
    (Cat::Bass, "Short Fuse", "", &[(AmpEgSustain, 0), (AmpEgDecay, 400)]),
    (Cat::Bass, "Boxy", "", &[(Vco1Wave, SQR), (Vco2Wave, SQR)]),
    (Cat::Bass, "Narrow Pulse", "", &[(Vco1Wave, SQR), (Vco1Shape, 700), (Vco2Wave, SQR)]),
    (Cat::Bass, "Thumb", "", &[(AmpEgSustain, 0), (AmpEgDecay, 300), (EgInt, 900)]),
    (Cat::Bass, "Squelch", "Glides only between overlapping notes.", &[(Vco2Level, 0), (Cutoff, 200), (Resonance, 750), (EgInt, 900), (EgDecay, 300), (Portamento, 30), (PortamentoMode, 0)]),
    (Cat::Bass, "Acid Rain", "", &[(Vco1Wave, SQR), (Vco2Level, 0), (Cutoff, 220), (Resonance, 700), (EgInt, 880), (Portamento, 25)]),
    (Cat::Bass, "Snarl", "", &[(Sync, 1), (Vco2Octave, FT8), (EgTarget, EG_PITCH2), (EgInt, 800)]),
    (Cat::Bass, "Stack of 4", "All four voices play each note, detuned.", &[(VoiceModeDepth, 500)]),
    (Cat::Bass, "Hot Wire", "", &[(CutoffDrive, 2), (Resonance, 300)]),
    (Cat::Bass, "FM Low End", "", &[(MultiType, VPM), (SelectVpm, 0), (MultiOctave, FT16), (MultiLevel, 800), (Vco2Level, 0)]),
    (Cat::Bass, "Eighth Wub", "The LFO opens the filter in eighth notes at the program tempo.", &[(LfoMode, LFO_BPM), (LfoRate, 660), (LfoTarget, 0), (LfoWave, TRI), (LfoInt, 600), (AmpEgSustain, 900)]),
    // Leads
    (Cat::Lead, "First Chair", "", &[]),
    (Cat::Lead, "Piercer", "", &[(Vco2Octave, FT4)]),
    (Cat::Lead, "Clarinetish", "", &[(Vco1Wave, SQR), (Vco2Wave, SQR)]),
    (Cat::Lead, "Whistle", "", &[(Vco1Wave, TRI), (Vco2Wave, TRI), (Cutoff, 500)]),
    (Cat::Lead, "Razor", "", &[(Sync, 1), (Vco2Pitch, 800)]),
    (Cat::Lead, "Sync Swoop", "The EG sweeps VCO 2 pitch at the start of each note.", &[(Sync, 1), (EgTarget, EG_PITCH2), (EgInt, 850), (EgDecay, 500)]),
    (Cat::Lead, "Stick Scream", "Push the joystick up to sweep VCO 2 pitch.", &[(Sync, 1), (JoystickAssignPlus, 5)]),
    (Cat::Lead, "Thick as Tar", "", &[(VoiceModeDepth, 600)]),
    (Cat::Lead, "Slide Rule", "Every note glides from the previous one.", &[(Portamento, 80), (PortamentoMode, 1)]),
    (Cat::Lead, "Warbler", "", &[(Vco1Wave, SQR), (LfoTarget, LFO_PITCH), (LfoWave, TRI), (LfoRate, 550), (LfoInt, 40)]),
    (Cat::Lead, "Tin Voice", "", &[(Ring, 1), (Vco2Pitch, 700)]),
    (Cat::Lead, "Crossfire", "", &[(CrossModDepth, 400)]),
    (Cat::Lead, "Hot Lead", "", &[(CutoffDrive, 2), (Resonance, 400)]),
    (Cat::Lead, "Opening Act", "Push the joystick up to open the filter.", &[(JoystickAssignPlus, 12), (Cutoff, 400)]),
    (Cat::Lead, "FM Solo", "", &[(MultiType, VPM), (SelectVpm, 4), (MultiLevel, 700), (Vco2Level, 300)]),
    // Keys
    (Cat::Keys, "Drawbars", "Same level however hard you play.", &[(Vco1Wave, TRI), (Vco2Wave, TRI), (Vco2Octave, FT4), (Vco2Pitch, 512), (AmpEgSustain, 1023), (AmpEgRelease, 80), (AmpVelocity, 0), (Cutoff, 900), (EgInt, 512), (ModFxOn, 1), (ModFxType, 1), (ModFxChorus, 7)]),
    (Cat::Keys, "Combo Organ", "", &[(Vco1Wave, SQR), (Vco1Shape, 0), (Vco2Wave, SQR), (Vco2Octave, FT4), (Vco2Pitch, 512), (AmpEgSustain, 1023), (AmpEgRelease, 80), (AmpVelocity, 0), (Cutoff, 750), (EgInt, 512)]),
    (Cat::Keys, "Spun Organ", "", &[(Vco1Wave, TRI), (Vco2Wave, SQR), (Vco2Octave, FT4), (AmpEgSustain, 1023), (AmpEgRelease, 100), (AmpVelocity, 0), (EgInt, 512), (ModFxOn, 1), (ModFxType, PHASER)]),
    (Cat::Keys, "Tine Piano", "", &[(MultiType, VPM), (SelectVpm, 1), (ShapeVpm, 400), (MultiLevel, 900), (Vco1Level, 300), (Vco2Level, 0), (AmpEgSustain, 200)]),
    (Cat::Keys, "Phased Tines", "", &[(MultiType, VPM), (SelectVpm, 2), (ShapeVpm, 550), (MultiLevel, 900), (Vco1Wave, TRI), (Vco1Level, 400), (Vco2Level, 0), (ModFxOn, 1), (ModFxType, PHASER)]),
    (Cat::Keys, "Polysynth", "", &[]),
    (Cat::Keys, "Square Comp", "", &[(Vco1Wave, SQR), (Vco2Wave, SQR)]),
    (Cat::Keys, "House Stab", "Each key plays a major chord.", &[(VoiceModeType, CHORD), (VoiceModeDepth, 250), (AmpEgSustain, 0), (AmpEgDecay, 520)]),
    (Cat::Keys, "Fanfare", "", &[(EgAttack, 150), (EgDecay, 600), (EgInt, 750), (AmpEgSustain, 800), (Cutoff, 350)]),
    (Cat::Keys, "Horn Section", "", &[(Vco2Wave, SQR), (EgAttack, 200), (EgDecay, 650), (EgInt, 720), (AmpEgSustain, 850), (Cutoff, 400), (ModFxOn, 1), (ModFxType, ENSEMBLE)]),
    (Cat::Keys, "Hit Harder", "Play harder for a brighter, louder note.", &[(CutoffVelocity, 110), (AmpVelocity, 100), (Cutoff, 350)]),
    (Cat::Keys, "Temple Bell", "", &[(MultiType, VPM), (SelectVpm, 0), (ShiftShapeVpm, 700), (ShapeVpm, 600), (MultiLevel, 900), (Vco1Level, 0), (Vco2Level, 0), (AmpEgSustain, 0), (AmpEgDecay, 800), (AmpEgRelease, 700)]),
    (Cat::Keys, "Glockenspiel", "", &[(MultiType, VPM), (SelectVpm, 3), (ShiftShapeVpm, 820), (ShapeVpm, 500), (MultiLevel, 900), (Vco1Level, 200), (Vco2Level, 0), (AmpEgSustain, 0), (AmpEgDecay, 850), (AmpEgRelease, 750)]),
    (Cat::Keys, "Gamelan", "", &[(Ring, 1), (Vco1Wave, TRI), (Vco2Wave, TRI), (Vco2Pitch, 760), (AmpEgSustain, 0), (AmpEgDecay, 800), (AmpEgRelease, 700), (Cutoff, 900)]),
    (Cat::Keys, "Clock Tower", "", &[(CrossModDepth, 600), (Vco1Wave, TRI), (Vco2Wave, TRI), (Vco2Octave, FT4), (AmpEgSustain, 0), (AmpEgDecay, 800), (AmpEgRelease, 650)]),
    // Plucks
    (Cat::Pluck, "Pizzicato", "", &[]),
    (Cat::Pluck, "Harpsichord", "", &[(Vco2Octave, FT4)]),
    (Cat::Pluck, "Pixel Pick", "", &[(Vco1Wave, SQR), (Vco2Wave, SQR)]),
    (Cat::Pluck, "Kalimba", "", &[(Vco1Wave, TRI), (Vco2Wave, TRI), (Resonance, 100)]),
    (Cat::Pluck, "Droplet", "", &[(Resonance, 700)]),
    (Cat::Pluck, "Snapback", "", &[(Sync, 1), (Vco2Pitch, 760)]),
    (Cat::Pluck, "Tin Harp", "", &[(Ring, 1), (Vco2Pitch, 700)]),
    (Cat::Pluck, "Sus4 Strum", "Each key plays a sus4 chord.", &[(VoiceModeType, CHORD), (VoiceModeDepth, 330)]),
    (Cat::Pluck, "FM Koto", "", &[(MultiType, VPM), (SelectVpm, 12), (MultiLevel, 800), (Vco2Level, 0)]),
    (Cat::Pluck, "Glass Pick", "", &[(MultiType, VPM), (SelectVpm, 13), (MultiLevel, 800), (Vco1Wave, TRI)]),
    (Cat::Pluck, "Marimba", "", &[(MultiType, VPM), (SelectVpm, 0), (ShapeVpm, 300), (MultiLevel, 1000), (Vco1Level, 0), (Vco2Level, 0), (Cutoff, 900)]),
    (Cat::Pluck, "Vibes", "", &[(MultiType, VPM), (SelectVpm, 1), (ShapeVpm, 450), (MultiLevel, 1000), (Vco1Wave, TRI), (Vco1Level, 300), (Vco2Level, 0), (Cutoff, 900)]),
    (Cat::Pluck, "Gravel", "", &[(MultiType, NOISE), (SelectNoise, 2), (MultiLevel, 600)]),
    (Cat::Pluck, "Crunch", "", &[(CutoffDrive, 2)]),
    (Cat::Pluck, "Canyon", "Long delay feedback: leave space between notes.", &[(DelayDepth, 650), (DelayDryWet, 500)]),
    // Arpeggios (Voice Mode Depth picks the pattern)
    (Cat::Arp, "Staircase", "Hold a chord; the notes play upward.", &[(VoiceModeDepth, 190)]),
    (Cat::Arp, "Waterfall", "", &[(VoiceModeDepth, 350)]),
    (Cat::Arp, "Seesaw", "", &[(VoiceModeDepth, 500)]),
    (Cat::Arp, "Dice Roll", "Hold a chord; the notes play in random order.", &[(VoiceModeDepth, 820)]),
    (Cat::Arp, "Chordwalker", "", &[(VoiceModeDepth, 660)]),
    (Cat::Arp, "Motor", "", &[]),
    (Cat::Arp, "8-Bit Steps", "", &[(Vco1Wave, SQR), (Vco2Wave, SQR)]),
    (Cat::Arp, "Picker", "", &[(AmpEgSustain, 0), (EgInt, 900), (Cutoff, 300)]),
    (Cat::Arp, "Gears", "", &[(Sync, 1), (Vco2Pitch, 780)]),
    (Cat::Arp, "Music Box", "", &[(Ring, 1), (Vco2Pitch, 720)]),
    (Cat::Arp, "FM Ripple", "", &[(MultiType, VPM), (SelectVpm, 6), (MultiLevel, 800), (Vco2Level, 0)]),
    (Cat::Arp, "Acid Loop", "", &[(Vco2Level, 0), (Resonance, 750), (Cutoff, 250), (EgInt, 900)]),
    (Cat::Arp, "Bassline", "", &[(Vco2Octave, FT16), (Vco2Wave, SQR)]),
    (Cat::Arp, "Echo Chamber", "The delay is synced to the tempo.", &[(DelayDepth, 600), (DelayDryWet, 500)]),
    (Cat::Arp, "Slinky", "", &[(Portamento, 40), (PortamentoMode, 1)]),
    // Effects and percussion
    (Cat::Fx, "Closed Hat", "", &[(Vco1Level, 0), (Vco2Level, 0), (MultiType, NOISE), (SelectNoise, 0), (ShapeNoise, 700), (MultiLevel, 1023), (AmpEgDecay, 80), (AmpEgRelease, 80)]),
    (Cat::Fx, "Open Hat", "", &[(Vco1Level, 0), (Vco2Level, 0), (MultiType, NOISE), (SelectNoise, 0), (ShapeNoise, 600), (MultiLevel, 1023), (AmpEgDecay, 300), (AmpEgRelease, 300)]),
    (Cat::Fx, "Snare", "", &[(Vco1Wave, TRI), (Vco1Octave, FT4), (Vco1Level, 600), (Vco2Level, 0), (MultiType, NOISE), (SelectNoise, 0), (MultiLevel, 900), (AmpEgDecay, 200), (EgTarget, EG_PITCH), (EgInt, 700), (EgDecay, 80)]),
    (Cat::Fx, "Kick Drum", "Pitch drops quickly at the start of each note.", &[(Vco1Wave, TRI), (Vco1Octave, FT16), (Vco2Level, 0), (EgTarget, EG_PITCH), (EgInt, 900), (EgDecay, 120), (AmpEgDecay, 300), (Cutoff, 400)]),
    (Cat::Fx, "Low Tom", "", &[(Vco1Wave, TRI), (Vco2Level, 0), (EgTarget, EG_PITCH), (EgInt, 750), (EgDecay, 250), (AmpEgDecay, 400)]),
    (Cat::Fx, "Zap", "", &[(Vco1Wave, SQR), (EgTarget, EG_PITCH), (EgInt, 1000), (EgDecay, 200)]),
    (Cat::Fx, "Pew Pew", "", &[(EgTarget, EG_PITCH), (EgInt, 100), (EgDecay, 400), (AmpEgDecay, 450)]),
    (Cat::Fx, "Siren", "", &[(Vco1Wave, TRI), (Vco2Wave, TRI), (LfoTarget, LFO_PITCH), (LfoWave, TRI), (LfoRate, 300), (LfoInt, 800), (AmpEgSustain, 1023)]),
    (Cat::Fx, "Alarm", "", &[(Vco1Wave, SQR), (Vco2Wave, SQR), (LfoTarget, LFO_PITCH), (LfoWave, SQR), (LfoRate, 450), (LfoInt, 500), (AmpEgSustain, 1023)]),
    (Cat::Fx, "Gale", "", &[(Vco1Level, 0), (Vco2Level, 0), (MultiType, NOISE), (SelectNoise, 1), (ShapeNoise, 400), (MultiLevel, 1023), (LfoTarget, 0), (LfoRate, 60), (LfoInt, 500), (Resonance, 600), (AmpEgAttack, 700), (AmpEgSustain, 1023), (AmpEgRelease, 800)]),
    (Cat::Fx, "Shoreline", "", &[(Vco1Level, 0), (Vco2Level, 0), (MultiType, NOISE), (SelectNoise, 0), (MultiLevel, 1023), (LfoTarget, 0), (LfoWave, TRI), (LfoRate, 40), (LfoInt, 600), (AmpEgAttack, 500), (AmpEgSustain, 1023), (AmpEgRelease, 700)]),
    (Cat::Fx, "Liftoff", "Hold the key while the pitch rises.", &[(EgTarget, EG_PITCH), (EgAttack, 1000), (EgInt, 900), (MultiType, NOISE), (MultiLevel, 400), (AmpEgSustain, 1023)]),
    (Cat::Fx, "Scrap Metal", "", &[(Ring, 1), (CrossModDepth, 700), (Vco2Pitch, 850), (EgTarget, EG_PITCH), (EgInt, 820), (EgDecay, 150), (AmpEgDecay, 600), (AmpEgRelease, 500)]),
    (Cat::Fx, "Broken Radio", "", &[(Vco1Level, 0), (Vco2Level, 0), (MultiType, NOISE), (SelectNoise, 3), (MultiLevel, 900), (AmpEgSustain, 1023)]),
    (Cat::Fx, "Sonar", "", &[(Vco1Wave, SQR), (Vco1Octave, FT4), (AmpEgDecay, 60), (EgTarget, EG_PITCH), (EgInt, 300), (DelayOn, 1), (DelayDepth, 650), (DelayDryWet, 500)]),
    // Drones
    (Cat::Drone, "Monolith", "", &[]),
    (Cat::Drone, "Mains Hum", "", &[(Vco1Wave, SQR), (Vco2Wave, SQR)]),
    (Cat::Drone, "Abyss", "", &[(Vco2Octave, FT16), (Vco2Wave, TRI)]),
    (Cat::Drone, "Static Sea", "", &[(MultiType, NOISE), (SelectNoise, 1), (MultiLevel, 700)]),
    (Cat::Drone, "Singing Bowl", "", &[(Ring, 1), (Vco2Pitch, 680)]),
    (Cat::Drone, "Tangle", "", &[(CrossModDepth, 500)]),
    (Cat::Drone, "Engine Room", "", &[(Sync, 1), (Vco2Pitch, 700)]),
    (Cat::Drone, "Nebula", "", &[(MultiType, VPM), (SelectVpm, 14), (MultiLevel, 700)]),
    (Cat::Drone, "Swarm", "Mod FX uses the user effect in User 1.", &[(VoiceModeType, UNISON), (VoiceModeDepth, 400), (ModFxOn, 1), (ModFxType, USER_FX), (ModFxUser, 0)]),
    (Cat::Drone, "Open Fifth", "Each key plays a fifth.", &[(VoiceModeType, CHORD), (VoiceModeDepth, 30)]),
    (Cat::Drone, "Slow Orbit", "", &[(ModFxOn, 1), (ModFxType, PHASER), (ModFxTime, 150)]),
    (Cat::Drone, "Jet Wash", "", &[(ModFxOn, 1), (ModFxType, FLANGER), (ModFxTime, 150)]),
    (Cat::Drone, "Lo-Fi Haze", "", &[(MultiType, NOISE), (SelectNoise, 3), (MultiLevel, 500)]),
    (Cat::Drone, "Tide Pool", "", PWM),
    (Cat::Drone, "Night Shift", "", &[(Cutoff, 150)]),
];

/// 120 authored programs (15 each of pads, basses, leads, keys, plucks,
/// arpeggios, effects and drones), with their programmer note. Deterministic.
pub fn demo_library() -> Vec<(Program, ProgInfo)> {
    PROGRAMS
        .iter()
        .enumerate()
        .map(|(i, &(cat, name, tip, own))| build(i, cat, name, tip, own))
        .collect()
}

fn build(i: usize, cat: Cat, name: &str, tip: &str, own: Settings) -> (Program, ProgInfo) {
    let mut r = Rng::new(i);
    let mut v: Vec<(Field, u16)> = vec![
        // Two slightly detuned sawtooth VCOs, multi engine silent.
        (Vco1Wave, SAW),
        (Vco2Wave, SAW),
        (Vco1Shape, r.range(0, 250)),
        (Vco2Shape, r.range(0, 250)),
        (Vco2Pitch, r.range(536, 546)),
        (Vco1Level, r.range(800, 1000)),
        (Vco2Level, r.range(600, 900)),
        // Drive and key track cycle so every value occurs.
        (CutoffDrive, (i / 2 % 3) as u16),
        (CutoffKeyTrack, (i / 4 % 3) as u16),
        (CutoffVelocity, r.range(0, 60)),
        (AmpVelocity, r.range(40, 127)),
        (EgTarget, EG_CUTOFF),
        (EgDecay, r.range(300, 600)),
        (LfoWave, (i % 3) as u16),
        (LfoMode, 1),
        (LfoRate, r.range(300, 600)),
        (ModFxChorus, r.range(0, 7)),
        (ModFxEnsemble, r.range(0, 2)),
        (ModFxPhaser, r.range(0, 7)),
        (ModFxFlanger, r.range(0, 7)),
        (ModFxTime, r.range(200, 600)),
        (ModFxDepth, r.range(200, 600)),
        (DelaySubType, r.range(0, 2)),
        (DelayTime, r.range(300, 800)),
        (DelayDepth, r.range(150, 450)),
        (DelayDryWet, r.range(150, 350)),
        (ReverbSubType, r.range(0, 6)),
        (ReverbTime, r.range(300, 900)),
        (ReverbDepth, r.range(300, 800)),
        (ReverbDryWet, r.range(150, 450)),
        (SeqBpm, r.range(900, 1400)),
    ];
    v.extend(match cat {
        Cat::Pad => vec![
            (VoiceModeType, POLY),
            (AmpEgAttack, r.range(450, 800)),
            (AmpEgDecay, r.range(500, 900)),
            (AmpEgSustain, r.range(800, 1023)),
            (AmpEgRelease, r.range(600, 900)),
            (Cutoff, r.range(350, 650)),
            (Resonance, r.range(0, 300)),
            (EgAttack, r.range(400, 800)),
            (EgDecay, r.range(600, 1000)),
            (EgInt, r.range(560, 680)),
            (ModFxOn, 1),
            (ReverbOn, 1),
            (ReverbTime, r.range(600, 1000)),
        ],
        Cat::Bass => vec![
            (VoiceModeType, UNISON),
            (VoiceModeDepth, r.range(0, 60)),
            (Vco1Octave, FT16),
            (Vco2Octave, FT16),
            (AmpEgAttack, r.range(0, 20)),
            (AmpEgDecay, r.range(250, 550)),
            (AmpEgSustain, r.range(400, 800)),
            (AmpEgRelease, r.range(30, 150)),
            (Cutoff, r.range(200, 450)),
            (Resonance, r.range(100, 400)),
            (EgDecay, r.range(200, 450)),
            (EgInt, r.range(650, 850)),
        ],
        Cat::Lead => vec![
            (VoiceModeType, UNISON),
            (VoiceModeDepth, r.range(0, 150)),
            (Portamento, r.range(10, 50)),
            (AmpEgAttack, r.range(0, 60)),
            (AmpEgSustain, r.range(800, 1023)),
            (AmpEgRelease, r.range(150, 350)),
            (Cutoff, r.range(550, 850)),
            (Resonance, r.range(150, 450)),
            (EgInt, r.range(560, 720)),
            (DelayOn, 1),
        ],
        Cat::Keys => vec![
            (VoiceModeType, POLY),
            (AmpEgAttack, r.range(0, 15)),
            (AmpEgDecay, r.range(450, 750)),
            (AmpEgSustain, r.range(300, 650)),
            (AmpEgRelease, r.range(250, 450)),
            (Cutoff, r.range(450, 800)),
            (Resonance, r.range(0, 250)),
            (EgInt, r.range(560, 700)),
            (ReverbOn, 1),
            (ReverbSubType, 4), // room
        ],
        Cat::Pluck => vec![
            (VoiceModeType, POLY),
            (AmpEgAttack, 0),
            (AmpEgDecay, r.range(150, 380)),
            (AmpEgSustain, 0),
            (AmpEgRelease, r.range(150, 320)),
            (Cutoff, r.range(200, 450)),
            (Resonance, r.range(100, 450)),
            (EgAttack, 0),
            (EgDecay, r.range(120, 320)),
            (EgInt, r.range(750, 950)),
            (DelayOn, 1),
        ],
        Cat::Arp => vec![
            (VoiceModeType, ARP),
            (VoiceModeDepth, r.range(0, 1023)),
            (AmpEgAttack, r.range(0, 30)),
            (AmpEgDecay, r.range(250, 450)),
            (AmpEgSustain, r.range(0, 300)),
            (AmpEgRelease, r.range(100, 250)),
            (Cutoff, r.range(350, 700)),
            (Resonance, r.range(150, 500)),
            (EgInt, r.range(620, 850)),
            (ArpGateTime, r.range(20, 60)),
            (DelayOn, 1),
            (DelaySubType, r.range(6, 10)), // tempo-synced types
        ],
        Cat::Fx => vec![
            (VoiceModeType, POLY),
            (AmpEgAttack, 0),
            (AmpEgDecay, r.range(200, 500)),
            (AmpEgSustain, 0),
            (AmpEgRelease, r.range(100, 400)),
            (Cutoff, r.range(600, 1000)),
            (Resonance, r.range(100, 500)),
            (EgInt, 512),
        ],
        Cat::Drone => vec![
            (VoiceModeType, POLY),
            (Octave, 1),
            (AmpEgAttack, r.range(700, 1000)),
            (AmpEgSustain, 1023),
            (AmpEgRelease, r.range(800, 1023)),
            (Cutoff, r.range(250, 550)),
            (Resonance, r.range(200, 600)),
            (LfoTarget, 0),
            (LfoRate, r.range(0, 150)),
            (LfoInt, r.range(150, 400)),
            (ReverbOn, 1),
            (ReverbTime, r.range(800, 1023)),
        ],
    });
    v.extend_from_slice(own);
    let mut p = Program::init();
    p.set_name(name).expect("demo names are valid");
    for (f, x) in v {
        p.set(f, x).expect("demo values are in range");
    }
    let info = ProgInfo {
        programmer: PROGRAMMER.into(),
        comment: tip.into(),
    };
    (p, info)
}

/// A few small logue SDK units for the simulator's user slots, as
/// `(slot, unit)`. Their payloads are placeholders, not runnable code; the
/// status header (name, module, versions) is what a synth reports.
pub fn demo_units() -> Vec<(u8, Unit)> {
    [
        (Module::Osc, 0, "MorphTbl"),
        (Module::Osc, 1, "waves"),
        (Module::Osc, 2, "SuperSaw"),
        (Module::ModFx, 0, "Wobbler"),
        (Module::DelFx, 0, "TapeEcho"),
        (Module::RevFx, 0, "Shimmer"),
    ]
    .into_iter()
    .map(|(module, slot, name)| {
        let (key, magic) = match module {
            Module::Osc => ("osc", b"UOSC"),
            Module::ModFx => ("modfx", b"UMOD"),
            Module::DelFx => ("delfx", b"UDEL"),
            Module::RevFx => ("revfx", b"UREV"),
        };
        let payload = [&magic[..], &[0; 60]].concat();
        let manifest = json!({"header": {
            "platform": "minilogue-xd", "module": key, "api": "1.1-0",
            "dev_id": 0, "prg_id": slot, "version": "1.0-0", "name": name,
            "params": []
        }});
        let (module, name, header) =
            build_header(&manifest, payload.len()).expect("demo manifests are valid");
        let unit = Unit {
            module,
            name,
            header,
            payload,
        };
        (slot, unit)
    })
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn programs_are_valid_unique_and_in_range() {
        let lib = demo_library();
        assert_eq!(lib.len(), 120);
        let mut names = BTreeSet::new();
        for (p, info) in &lib {
            assert_eq!(&Program::from_bytes(p.as_bytes()).unwrap(), p);
            assert!(names.insert(p.name()), "duplicate {}", p.name());
            assert!(p.name().len() <= 12);
            assert_eq!(info.programmer, PROGRAMMER);
            assert!(!info.comment.contains('\n'));
            for &f in Field::ALL {
                let s = f.spec();
                assert!(
                    (s.min..=s.max).contains(&p.get(f)),
                    "{} {}",
                    p.name(),
                    s.name
                );
            }
        }
    }

    #[test]
    fn covers_every_mode_wave_and_effect_switch() {
        let lib = demo_library();
        let values = |f: Field| lib.iter().map(|(p, _)| p.get(f)).collect::<BTreeSet<_>>();
        let all = |f: Field| (f.spec().min..=f.spec().max).collect::<BTreeSet<_>>();
        for f in [
            VoiceModeType,
            Vco1Wave,
            Vco2Wave,
            MultiType,
            CutoffDrive,
            CutoffKeyTrack,
            ModFxOn,
            ModFxType,
            DelayOn,
            ReverbOn,
            EgTarget,
            LfoWave,
        ] {
            assert_eq!(values(f), all(f), "{}", f.spec().name);
        }
    }

    /// Each program sounds like its section: the recipe holds after the
    /// program's own settings, and `present::category` files it there.
    #[test]
    fn programs_match_their_section() {
        use crate::present::{Category, category};
        let mut misses = Vec::new();
        for (&(cat, ..), (p, _)) in PROGRAMS.iter().zip(demo_library()) {
            let name = p.name();
            let get = |f: Field| p.get(f);
            match cat {
                Cat::Bass => {
                    assert!(get(Vco1Octave) <= FT8 && get(Vco2Octave) <= FT8, "{name}");
                    assert!(get(AmpEgRelease) <= 200, "{name}");
                    assert_eq!(get(VoiceModeType), UNISON, "{name}");
                }
                Cat::Pad => {
                    assert!(
                        get(AmpEgAttack) >= 300 && get(AmpEgRelease) >= 500,
                        "{name}"
                    );
                    assert_eq!(get(ReverbOn), 1, "{name}");
                }
                Cat::Drone => assert_eq!(get(AmpEgSustain), 1023, "{name}"),
                Cat::Pluck => assert_eq!(get(AmpEgSustain), 0, "{name}"),
                Cat::Arp => assert_eq!(get(VoiceModeType), ARP, "{name}"),
                Cat::Lead | Cat::Keys | Cat::Fx => {}
            }
            let want = match cat {
                Cat::Pad => Category::Pad,
                Cat::Bass => Category::Bass,
                Cat::Lead => Category::Lead,
                Cat::Keys => Category::Keys,
                Cat::Pluck => Category::Pluck,
                Cat::Arp => Category::Arp,
                Cat::Fx => Category::Fx,
                Cat::Drone => Category::Drone,
            };
            let got = category(&p);
            if got != want && !(want == Category::Keys && got == Category::Bell) {
                misses.push(format!("{name}: {got:?}, in {want:?}"));
            }
        }
        // The list groups programs by category, so a stray reads as a
        // misfiled program.
        assert!(misses.is_empty(), "{misses:#?}");
    }

    #[test]
    fn is_deterministic() {
        assert_eq!(demo_library(), demo_library());
    }

    #[test]
    fn units_build() {
        let units = demo_units();
        assert!(units.iter().any(|(_, u)| u.name == "waves"));
        for (slot, u) in units {
            assert!(slot < u.module.slots());
        }
    }
}
