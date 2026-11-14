//! Normalised geometry for live mini-graphs.
//!
//! These are illustrative shapes derived from program parameters so the
//! editor can show "what a knob does" at a glance. They are NOT renderings
//! of the synth's audio: waveshapes, filter curve, envelope timing and delay
//! taps are simplified models with no claim to match the hardware.

use oscyl_logue::kinds::wave_sample as vco;

use crate::present::Stage;

use crate::{Field, Program};
pub use oscyl_logue::kinds::adsr;

fn unit(p: &Program, f: Field) -> f32 {
    p.get(f) as f32 / 1023.0
}

/// `f`'s value as 0..1 of its range.
fn span_unit(p: &Program, f: Field) -> f32 {
    let s = f.spec();
    f32::from(p.get(f).saturating_sub(s.min)) / f32::from((s.max - s.min).max(1))
}

/// VCO 1 cycles shown: enough for VCO 2 to complete a whole cycle too.
pub fn osc_cycles(p: &Program) -> u32 {
    let oct = |f| i32::from(p.get(f));
    // Octaves only: semitones and cents would make the picture aperiodic.
    let down = oct(Field::Vco1Octave) - oct(Field::Vco2Octave);
    1u32 << down.clamp(1, 3)
}

/// [`osc_cycles`] cycles of VCO 1 (`n` samples, −1..1) mixed with VCO 2 by
/// their mixer levels. VCO 2 runs at its octave ratio to VCO 1 (pitch is
/// left out so the picture repeats cleanly); ring mod multiplies it by
/// VCO 1. Normalised to peak 1, or all zeros when both levels are 0.
pub fn osc_cycle(p: &Program, n: usize) -> Vec<f32> {
    use Field::*;
    let ratio = 2f32.powi(i32::from(p.get(Vco2Octave)) - i32::from(p.get(Vco1Octave)));
    let cycles = osc_cycles(p) as f32;
    let (l1, l2) = (unit(p, Vco1Level), unit(p, Vco2Level));
    let ring = p.get(Ring) != 0;
    let last = n.saturating_sub(1).max(1) as f32;
    let mix = |t: f32| {
        let a = vco(p.get(Vco1Wave), unit(p, Vco1Shape), t.fract());
        let mut b = vco(p.get(Vco2Wave), unit(p, Vco2Shape), (t * ratio).fract());
        if ring {
            b *= a;
        }
        l1 * a + l2 * b
    };
    // Start at a rising zero crossing, so the picture begins and ends at
    // the centre line (where the chart band's signal line meets it). The
    // window holds whole cycles, so shifting the phase keeps the shape.
    let phase = (0..256)
        .map(|k| k as f32 / 256.0)
        .find(|&t| mix(t) <= 0.0 && mix(t + 1.0 / 256.0) > 0.0)
        .unwrap_or(0.0);
    let mut out: Vec<f32> = (0..n)
        .map(|i| {
            // The last sample closes the window, so the stroke ends where
            // the next cycle would begin.
            mix(phase + i as f32 / last * cycles * (1.0 - 1e-4))
        })
        .collect();
    let peak = out.iter().fold(0f32, |m, v| m.max(v.abs()));
    if peak > 0.0 {
        out.iter_mut().for_each(|v| *v /= peak);
    }
    out
}

/// dB range mapped onto y 0..1; 0 dB sits at 2/3, leaving room for the
/// resonance peak.
const DB_FLOOR: f32 = -48.0;
const DB_CEIL: f32 = 24.0;

/// Two-pole low-pass magnitude at `n` points, x = log frequency 20 Hz–20 kHz
/// (0..1), y = level 0..1 (see `DB_FLOOR`/`DB_CEIL`). Cutoff maps 0–1023
/// exponentially onto the same span; resonance raises Q from 0.707 to ~8.7.
pub fn filter_response(p: &Program, n: usize) -> Vec<(f32, f32)> {
    let fc = 20.0 * 1000f32.powf(unit(p, Field::Cutoff));
    let q = std::f32::consts::FRAC_1_SQRT_2 + 8.0 * unit(p, Field::Resonance);
    let last = n.saturating_sub(1).max(1) as f32;
    (0..n)
        .map(|i| {
            let x = i as f32 / last;
            let w = 20.0 * 1000f32.powf(x) / fc;
            let mag = 1.0 / ((1.0 - w * w).powi(2) + (w / q).powi(2)).sqrt();
            let db = 20.0 * mag.log10();
            (x, ((db - DB_FLOOR) / (DB_CEIL - DB_FLOOR)).clamp(0.0, 1.0))
        })
        .collect()
}

/// y value of 0 dB in [`filter_response`].
pub const FILTER_UNITY: f32 = -DB_FLOOR / (DB_CEIL - DB_FLOOR);

/// [`adsr`] of the program's amp EG.
pub fn amp_envelope(p: &Program) -> [(f32, f32); 5] {
    use Field::*;
    adsr(
        p.get(AmpEgAttack),
        p.get(AmpEgDecay),
        p.get(AmpEgSustain),
        p.get(AmpEgRelease),
    )
}

/// The filter peak: x at the cutoff, y at the response's level there.
fn filter_peak(p: &Program) -> (f32, f32) {
    let q = std::f32::consts::FRAC_1_SQRT_2 + 8.0 * unit(p, Field::Resonance);
    let db = 20.0 * q.log10();
    (
        unit(p, Field::Cutoff),
        ((db - DB_FLOOR) / (DB_CEIL - DB_FLOOR)).clamp(0.0, 1.0),
    )
}

/// A point on a stage graph that can be dragged to edit the program.
/// Positions are graph data coordinates (x 0..1 left to right, y 0..1
/// bottom to top), the same space the `viz` shapes are drawn in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Handle {
    /// The filter peak: across is cutoff, up is resonance.
    FilterPeak,
    /// Amp EG corners: the attack peak, the end of the decay (also
    /// sustain level), the end of the sustain hold and the release end.
    Attack,
    Decay,
    Sustain,
    Release,
    /// The top of bar `i` of a levels graph.
    Level(usize, Field),
}

/// The handles a stage's graph offers, the primary one first.
pub fn handles(stage: Stage) -> &'static [Handle] {
    use Field::*;
    use Handle::*;
    match stage {
        Stage::Mixer => &[
            Level(0, Vco1Level),
            Level(1, Vco2Level),
            Level(2, MultiLevel),
        ],
        Stage::Filter => &[FilterPeak],
        Stage::Envelopes => &[Decay, Attack, Sustain, Release],
        _ => &[],
    }
}

impl Handle {
    /// The fields moved by dragging across and up.
    pub fn fields(self) -> (Option<Field>, Option<Field>) {
        use Field::*;
        match self {
            Handle::FilterPeak => (Some(Cutoff), Some(Resonance)),
            Handle::Attack => (Some(AmpEgAttack), None),
            Handle::Decay => (Some(AmpEgDecay), Some(AmpEgSustain)),
            Handle::Sustain => (None, Some(AmpEgSustain)),
            Handle::Release => (Some(AmpEgRelease), None),
            Handle::Level(_, f) => (None, Some(f)),
        }
    }

    /// Where the handle sits for `p`.
    pub fn position(self, p: &Program) -> (f32, f32) {
        match self {
            Handle::FilterPeak => filter_peak(p),
            Handle::Attack => amp_envelope(p)[1],
            Handle::Decay => amp_envelope(p)[2],
            Handle::Sustain => amp_envelope(p)[3],
            Handle::Release => amp_envelope(p)[4],
            Handle::Level(i, f) => ((i as f32 + 0.5) / 3.0, span_unit(p, f)),
        }
    }

    /// The field values that put the handle at `to` (clamped to range).
    /// Fields whose value would not change are left out.
    pub fn values(self, p: &Program, (x, y): (f32, f32)) -> Vec<(Field, u16)> {
        use Field::*;
        let raw = |u: f32| (u.clamp(0.0, 1.0) * 1023.0).round() as u16;
        let env = amp_envelope(p);
        let out = match self {
            Handle::FilterPeak => {
                let db = DB_FLOOR + y * (DB_CEIL - DB_FLOOR);
                let q = 10f32.powf(db / 20.0);
                vec![
                    (Cutoff, raw(x)),
                    (Resonance, raw((q - std::f32::consts::FRAC_1_SQRT_2) / 8.0)),
                ]
            }
            Handle::Attack => vec![(AmpEgAttack, raw(x * 4.0))],
            Handle::Decay => vec![
                (AmpEgDecay, raw((x - env[1].0) * 4.0)),
                (AmpEgSustain, raw(y)),
            ],
            Handle::Sustain => vec![(AmpEgSustain, raw(y))],
            Handle::Release => vec![(AmpEgRelease, raw((x - env[3].0) * 4.0))],
            Handle::Level(_, f) => {
                let s = f.spec();
                let v = f32::from(s.max - s.min) * y.clamp(0.0, 1.0);
                vec![(f, s.min + v.round() as u16)]
            }
        };
        out.into_iter().filter(|&(f, v)| p.get(f) != v).collect()
    }
}

/// VCO 1, VCO 2 and multi engine levels, 0..1.
pub fn mixer_levels(p: &Program) -> [f32; 3] {
    [
        unit(p, Field::Vco1Level),
        unit(p, Field::Vco2Level),
        unit(p, Field::MultiLevel),
    ]
}

/// The dry note and its delay echoes as (time 0..1, amplitude 0..1): the
/// dry hit first, then echoes spaced by delay time, each quieter by the
/// depth (as feedback). Only the dry hit when the delay is off, so the
/// graph always shows the note the effects act on.
pub fn echo_taps(p: &Program) -> Vec<(f32, f32)> {
    let dry = (DRY_AT, 1.0);
    if p.get(Field::DelayOn) == 0 {
        return vec![dry];
    }
    let spacing = 0.05 + 0.45 * unit(p, Field::DelayTime);
    let feedback = 0.9 * unit(p, Field::DelayDepth);
    std::iter::once(dry)
        .chain(
            (1..)
                .map(|k| (DRY_AT + k as f32 * spacing, 0.8 * feedback.powi(k - 1)))
                .take_while(|&(t, a)| t <= 1.0 && a >= 0.02),
        )
        .collect()
}

/// Where the dry hit sits on the effects graph.
const DRY_AT: f32 = 0.03;

/// What the effects do to one note, for the effects graph: the dry hit and
/// delay echoes ([`echo_taps`]), the reverb's tail and the mod FX's sweep.
#[derive(Debug, Clone, PartialEq)]
pub struct Effects {
    pub taps: Vec<(f32, f32)>,
    /// Reverb: how long the tail rings (0..1 of the graph) and how loud it
    /// starts (0..1). `None` when the reverb is off.
    pub tail: Option<(f32, f32)>,
    /// Mod FX: cycles across the graph and depth (0..1). `None` when off.
    pub sweep: Option<(f32, f32)>,
}

pub fn effects(p: &Program) -> Effects {
    let tail = (p.get(Field::ReverbOn) != 0).then(|| {
        (
            0.25 + 0.7 * unit(p, Field::ReverbTime),
            0.15 + 0.55 * unit(p, Field::ReverbDryWet).max(unit(p, Field::ReverbDepth) * 0.5),
        )
    });
    let sweep = (p.get(Field::ModFxOn) != 0).then(|| {
        (
            1.0 + 7.0 * unit(p, Field::ModFxTime),
            0.15 + 0.85 * unit(p, Field::ModFxDepth),
        )
    });
    Effects {
        taps: echo_taps(p),
        tail,
        sweep,
    }
}

/// One sounding voice in [`voices`]: which of the four voices plays, when
/// (x 0..1) and at what pitch (y 0..1).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Note {
    pub voice: u8,
    pub start: f32,
    pub end: f32,
    pub pitch: f32,
}

/// Semitones above the lowest note as y (an octave spans 0.15..0.85).
fn semis(s: f32) -> f32 {
    0.15 + s / 12.0 * 0.7
}

/// Chord intervals in semitones, in the order the voice mode depth picks
/// them (5th, sus2, m, Maj, sus4, m7, 7, 7sus4, Maj7, aug, dim, m7b5,
/// mMaj7, Maj7b5).
const CHORD_NOTES: [&[u8]; 14] = [
    &[0, 7],
    &[0, 2, 7],
    &[0, 3, 7],
    &[0, 4, 7],
    &[0, 5, 7],
    &[0, 3, 7, 10],
    &[0, 4, 7, 10],
    &[0, 5, 7, 10],
    &[0, 4, 7, 11],
    &[0, 4, 8],
    &[0, 3, 6],
    &[0, 3, 6, 10],
    &[0, 3, 7, 11],
    &[0, 4, 6, 11],
];

/// How the voice mode uses the minilogue xd's four voices, as notes on a
/// piano roll (time across, pitch up, one lane per voice):
/// - Poly: four voices, each with its own note and timing; with Duo
///   (depth from 256) each note takes two voices, detuned apart.
/// - Unison: all four voices on one note, spread by the detune depth.
/// - Chord: one key plays the chord the depth picks, a voice per note
///   (lanes beyond the chord's notes stay silent).
/// - Arp/Latch: a held four-note chord stepped in the depth's pattern
///   order, one voice per step in turn, gate from the arp gate time; the
///   Poly patterns step the whole chord.
///
/// Detune is magnified (50 cents shows as a twelfth of the height) so the
/// spread reads at all.
pub fn voices(p: &Program) -> Vec<Note> {
    use Field::*;
    let depth = p.get(VoiceModeDepth);
    let note = |voice: u8, start: f32, end: f32, pitch: f32| Note {
        voice,
        start,
        end,
        pitch,
    };
    match p.get(VoiceModeType) {
        // Arp/Latch.
        1 => {
            let gate = (f32::from(p.get(ArpGateTime)) / 72.0).clamp(0.15, 1.0);
            let held = [0.0, 4.0, 7.0, 12.0];
            let order: &[usize] = match depth / 78 {
                0..=3 => &[0, 1, 2, 3, 0, 1, 2, 3],
                4 | 5 => &[3, 2, 1, 0, 3, 2, 1, 0],
                6 | 7 => &[0, 1, 2, 3, 2, 1, 0, 1],
                8 | 9 => &[],
                _ => &[2, 0, 3, 1, 3, 0, 2, 1],
            };
            let step = 1.0 / 8.0;
            let span = |k: usize| (k as f32 * step, k as f32 * step + step * gate);
            if order.is_empty() {
                // Poly patterns: the chord on every step.
                (0..8)
                    .flat_map(|k| {
                        let (a, b) = span(k);
                        (0..4).map(move |v| note(v as u8, a, b, semis(held[v])))
                    })
                    .collect()
            } else {
                order
                    .iter()
                    .enumerate()
                    .map(|(k, &i)| {
                        let (a, b) = span(k);
                        note((k % 4) as u8, a, b, semis(held[i]))
                    })
                    .collect()
            }
        }
        // Chord.
        2 => CHORD_NOTES[usize::from(depth / 73).min(13)]
            .iter()
            .enumerate()
            .map(|(v, &s)| note(v as u8, 0.05, 0.95, semis(f32::from(s))))
            .collect(),
        // Unison.
        3 => {
            let spread = f32::from(depth) / 1023.0 / 12.0;
            (0..4)
                .map(|v| {
                    let off = (v as f32 - 1.5) / 3.0 * spread;
                    note(v, 0.05, 0.95, semis(5.0) + off)
                })
                .collect()
        }
        // Poly, with Duo above 256.
        _ if depth >= 256 => {
            let spread = f32::from(depth - 256) / 767.0 / 12.0;
            [(0.0, 0.6, 0.0), (0.3, 1.0, 7.0)]
                .into_iter()
                .enumerate()
                .flat_map(|(n, (a, b, s))| {
                    [-0.5, 0.5]
                        .into_iter()
                        .enumerate()
                        .map(move |(k, off)| note((2 * n + k) as u8, a, b, semis(s) + off * spread))
                })
                .collect()
        }
        _ => [
            (0.0, 0.55, 0.0),
            (0.15, 0.95, 7.0),
            (0.35, 0.75, 4.0),
            (0.55, 1.0, 11.0),
        ]
        .into_iter()
        .enumerate()
        .map(|(v, (a, b, s))| note(v as u8, a, b, semis(s)))
        .collect(),
    }
}

#[cfg(test)]
mod phase_tests {
    use super::*;

    #[test]
    fn waves_start_and_end_near_the_centre_line() {
        for wave in 0..3 {
            let mut p = Program::init();
            p.set(Field::Vco1Wave, wave).unwrap();
            p.set(Field::Vco2Wave, wave).unwrap();
            p.set(Field::Vco1Level, 1023).unwrap();
            let w = osc_cycle(&p, 256);
            if w.iter().all(|v| *v == 0.0) {
                continue;
            }
            // A square has no continuous crossing: it starts on its edge.
            let start = if wave == 0 {
                w[0].abs().min(w[1].abs())
            } else {
                w[0].abs()
            };
            assert!(
                start < 0.2 || wave == 0 && (w[1] - w[0]).abs() > 1.0,
                "wave {wave} starts at {}",
                w[0]
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with(fs: &[(Field, u16)]) -> Program {
        let mut p = Program::init();
        for &(f, v) in fs {
            p.set(f, v).unwrap();
        }
        p
    }

    /// x where the curve first drops below −3 dB.
    fn minus3(r: &[(f32, f32)]) -> f32 {
        let y = (-3.0 - DB_FLOOR) / (DB_CEIL - DB_FLOOR);
        r.iter().find(|p| p.1 < y).map_or(1.0, |p| p.0)
    }

    #[test]
    fn osc_cycle_peak_is_one_and_in_range() {
        for wave in 0..3 {
            for shape in [0, 512, 1023] {
                let p = with(&[
                    (Field::Vco1Wave, wave),
                    (Field::Vco1Shape, shape),
                    (Field::Vco2Level, 700),
                    (Field::Vco2Octave, 2),
                    (Field::Ring, 1),
                ]);
                let c = osc_cycle(&p, 128);
                assert_eq!(c.len(), 128);
                let peak = c.iter().fold(0f32, |m, v| m.max(v.abs()));
                assert!((peak - 1.0).abs() < 1e-6);
            }
        }
        let silent = with(&[(Field::Vco1Level, 0)]);
        assert!(osc_cycle(&silent, 16).iter().all(|&v| v == 0.0));
    }

    #[test]
    fn filter_cutoff_and_resonance() {
        let r =
            |c, res| filter_response(&with(&[(Field::Cutoff, c), (Field::Resonance, res)]), 200);
        for (c, res) in [(0, 0), (500, 1023), (1023, 1023)] {
            assert!(
                r(c, res)
                    .iter()
                    .all(|&(x, y)| (0.0..=1.0).contains(&x) && (0.0..=1.0).contains(&y))
            );
        }
        assert!(minus3(&r(300, 0)) < minus3(&r(600, 0)));
        let peak = |v: Vec<(f32, f32)>| v.iter().fold(0f32, |m, p| m.max(p.1));
        assert!(peak(r(500, 800)) > peak(r(500, 0)));
        assert!((peak(r(1023, 0)) - FILTER_UNITY).abs() < 0.01);
    }

    #[test]
    fn adsr_segments_are_monotonic() {
        for (a, d, s, r) in [(0, 0, 0, 0), (1023, 1023, 1023, 1023), (100, 900, 400, 30)] {
            let pts = adsr(a, d, s, r);
            assert!(pts.windows(2).all(|w| w[0].0 <= w[1].0));
            assert!(
                pts.iter()
                    .all(|&(x, y)| (0.0..=1.0).contains(&x) && (0.0..=1.0).contains(&y))
            );
            assert!(pts[1].1 >= pts[0].1 && pts[2].1 <= pts[1].1);
            assert_eq!(pts[2].1, pts[3].1);
            assert!(pts[4].1 <= pts[3].1);
        }
    }

    /// Moving a handle to where it already is changes nothing, and moving
    /// it to another value's position yields that value (within rounding).
    #[test]
    fn handles_round_trip() {
        let programs = [
            Program::init(),
            with(&[
                (Field::Cutoff, 618),
                (Field::Resonance, 300),
                (Field::AmpEgAttack, 200),
                (Field::AmpEgDecay, 700),
                (Field::AmpEgSustain, 512),
                (Field::AmpEgRelease, 900),
                (Field::Vco2Level, 389),
            ]),
        ];
        for p in &programs {
            for stage in Stage::ALL {
                for &h in handles(stage) {
                    assert!(h.values(p, h.position(p)).is_empty(), "{h:?} moved itself");
                    let (fx, fy) = h.fields();
                    for target in [100, 777] {
                        let mut q = p.clone();
                        for f in [fx, fy].into_iter().flatten() {
                            q.set(f, target).unwrap();
                        }
                        let mut r = p.clone();
                        for (f, v) in h.values(p, h.position(&q)) {
                            r.set(f, v).unwrap();
                        }
                        for f in [fx, fy].into_iter().flatten() {
                            assert!(r.get(f).abs_diff(target) <= 1, "{h:?} {f:?} {}", r.get(f));
                        }
                    }
                }
            }
        }
        assert!(handles(Stage::Oscillators).is_empty());
    }

    /// Square, triangle and saw read as themselves: no jagged steps, one
    /// reset per saw cycle, two edges per square cycle, pulse width from
    /// shape, and the window ends where the next cycle would begin.
    #[test]
    fn osc_waves_are_clean() {
        let jumps = |c: &[f32]| c.windows(2).filter(|w| (w[1] - w[0]).abs() > 0.5).count();
        let solo = |wave, shape| {
            with(&[
                (Field::Vco1Wave, wave),
                (Field::Vco1Shape, shape),
                (Field::Vco2Level, 0),
            ])
        };
        let n = 512;
        let cycles = osc_cycles(&solo(0, 0)) as usize;
        assert_eq!(cycles, 2);
        // The window starts at a rising zero crossing: mid-ramp for a saw,
        // so each cycle's reset falls inside the window.
        assert_eq!(jumps(&osc_cycle(&solo(2, 0), n)), cycles);
        assert_eq!(jumps(&osc_cycle(&solo(1, 0), n)), 0);
        assert_eq!(jumps(&osc_cycle(&solo(1, 1023), n)), 0);
        // Starting on its rising edge, a square shows both edges of every cycle.
        assert_eq!(jumps(&osc_cycle(&solo(0, 0), n)), 2 * cycles);
        let high = |shape| {
            let c = osc_cycle(&solo(0, shape), n);
            c.iter().filter(|&&v| v > 0.0).count() as f32 / n as f32
        };
        assert!((high(0) - 0.5).abs() < 0.02);
        assert!(high(1023) < 0.1);
        // VCO 2 an octave below: enough VCO 1 cycles for a whole VCO 2 one.
        let low = with(&[(Field::Vco1Octave, 2), (Field::Vco2Octave, 0)]);
        assert_eq!(osc_cycles(&low), 4);
    }

    #[test]
    fn effects_follow_their_switches() {
        let off = effects(&Program::init());
        assert_eq!((off.tail, off.sweep), (None, None));
        let on = effects(&with(&[(Field::ReverbOn, 1), (Field::ModFxOn, 1)]));
        let (len, level) = on.tail.unwrap();
        assert!((0.0..=1.0).contains(&len) && (0.0..=1.0).contains(&level));
        let (cycles, depth) = on.sweep.unwrap();
        assert!(cycles >= 1.0 && (0.0..=1.0).contains(&depth));
    }

    #[test]
    fn mixer_and_echoes() {
        assert_eq!(mixer_levels(&Program::init()), [1.0, 0.0, 0.0]);
        assert_eq!(echo_taps(&Program::init()), [(DRY_AT, 1.0)]);
        let taps = echo_taps(&with(&[
            (Field::DelayOn, 1),
            (Field::DelayTime, 100),
            (Field::DelayDepth, 900),
        ]));
        assert!(taps.len() > 1);
        assert!(taps.windows(2).all(|w| w[0].0 < w[1].0 && w[0].1 > w[1].1));
        assert!(
            taps.iter()
                .all(|&(t, a)| t <= 1.0 && (0.0..=1.0).contains(&a))
        );
    }

    #[test]
    fn voices_follow_the_mode() {
        use Field::*;
        let in_box = |n: &[Note]| {
            n.iter().all(|n| {
                n.voice < 4
                    && 0.0 <= n.start
                    && n.start < n.end
                    && n.end <= 1.0
                    && (0.0..=1.0).contains(&n.pitch)
            })
        };
        let v = |fs: &[(Field, u16)]| voices(&with(fs));
        // Poly: four voices, four different notes.
        let poly = v(&[(VoiceModeType, 4), (VoiceModeDepth, 0)]);
        assert_eq!(poly.len(), 4);
        assert!(in_box(&poly));
        assert!(poly.windows(2).all(|w| w[0].start != w[1].start));
        // Duo: two notes, two voices each, apart by the depth.
        let duo = v(&[(VoiceModeType, 4), (VoiceModeDepth, 1023)]);
        assert_eq!(duo.len(), 4);
        assert_eq!(duo[0].start, duo[1].start);
        assert!(duo[1].pitch > duo[0].pitch);
        // Unison: one note on all four, spread grows with depth.
        let spread = |d| {
            let n = v(&[(VoiceModeType, 3), (VoiceModeDepth, d)]);
            assert!(in_box(&n) && n.len() == 4);
            assert!(n.iter().all(|x| x.start == n[0].start));
            n[3].pitch - n[0].pitch
        };
        assert_eq!(spread(0), 0.0);
        assert!(spread(1023) > spread(300));
        // Chord: one voice per chord note (Maj7: four, Maj: three).
        let chord = |d| v(&[(VoiceModeType, 2), (VoiceModeDepth, d)]);
        assert_eq!(chord(8 * 73).len(), 4);
        assert_eq!(chord(3 * 73).len(), 3);
        assert!(chord(1023).windows(2).all(|w| w[0].pitch < w[1].pitch));
        // Arp: one note at a time, voices in turn; rise climbs, fall drops.
        let arp = |d| v(&[(VoiceModeType, 1), (VoiceModeDepth, d), (ArpGateTime, 36)]);
        let rise = arp(2 * 78);
        assert!(in_box(&rise));
        assert!(rise.windows(2).all(|w| w[0].end <= w[1].start));
        assert!(
            rise.iter()
                .enumerate()
                .all(|(k, n)| n.voice as usize == k % 4)
        );
        assert!(rise[1].pitch > rise[0].pitch);
        assert!(arp(4 * 78)[1].pitch < arp(4 * 78)[0].pitch);
        assert_eq!(arp(8 * 78).len(), 32);
    }
}
