//! Illustrative graphs of a program, per page: oscillator mix, mixer
//! levels, envelopes and LFO waves of each sounding timbre, effects. Like
//! the xd's, these are simplified shapes, not renderings of the audio.

use oscyl_driver::Graph;
use oscyl_logue::kinds::{adsr, wave_sample};

use crate::Program;
use crate::curate::{EFFECTS, ENVELOPES, MIXER, OSCILLATORS};
use crate::fields::field;

/// Timbres that sound: main, and sub when it is on.
fn timbres(p: &Program) -> Vec<&'static str> {
    if p.get(field("Sub On")) == 1 {
        vec!["Main", "Sub"]
    } else {
        vec!["Main"]
    }
}

fn get(p: &Program, timbre: &str, name: &str) -> u16 {
    p.get(field(&format!("{timbre} {name}")))
}

fn unit(p: &Program, timbre: &str, name: &str) -> f32 {
    f32::from(get(p, timbre, name).min(1023)) / 1023.0
}

fn envelope(p: &Program, timbre: &str, eg: &str) -> Graph {
    let g = |stage| get(p, timbre, &format!("{eg} {stage}"));
    Graph::Envelope(adsr(g("Attack"), g("Decay"), g("Sustain"), g("Release")))
}

/// Two cycles of the timbre's LFO wave, scaled by its intensity.
fn lfo(p: &Program, timbre: &str, n: usize) -> Graph {
    let wave = get(p, timbre, "LFO Wave");
    let int = unit(p, timbre, "LFO Int");
    Graph::Wave(
        (0..n)
            .map(|i| wave_sample(wave, 0.0, (2.0 * i as f32 / n as f32).fract()) * int)
            .collect(),
    )
}

/// One cycle of VCO 1 mixed with VCO 2 at its octave ratio, by their
/// mixer levels, normalised to peak 1.
fn osc(p: &Program, timbre: &str, n: usize) -> Graph {
    let ratio = 2f32.powi(
        i32::from(get(p, timbre, "VCO 2 Octave")) - i32::from(get(p, timbre, "VCO 1 Octave")),
    );
    let (l1, l2) = (
        unit(p, timbre, "Mixer VCO 1"),
        unit(p, timbre, "Mixer VCO 2"),
    );
    let s: Vec<f32> = (0..n)
        .map(|i| {
            let t = i as f32 / n as f32;
            let a = wave_sample(
                get(p, timbre, "VCO 1 Wave"),
                unit(p, timbre, "VCO 1 Shape"),
                t,
            );
            let b = wave_sample(
                get(p, timbre, "VCO 2 Wave"),
                unit(p, timbre, "VCO 2 Shape"),
                (t * ratio).fract(),
            );
            a * l1 + b * l2
        })
        .collect();
    let peak = s.iter().fold(0f32, |m, v| m.max(v.abs()));
    Graph::Wave(if peak > 0.0 {
        s.iter().map(|v| v / peak).collect()
    } else {
        s
    })
}

fn effects(p: &Program) -> Graph {
    let on = |name: &str| p.get(field(name)) == 1;
    let u = |name: &str| f32::from(p.get(field(name)).min(1023)) / 1023.0;
    let kind = p.get(field("Delay/Reverb Type"));
    let delay = on("Delay/Reverb On") && kind == 1;
    let reverb = on("Delay/Reverb On") && kind == 2;
    let (time, depth) = (u("Delay/Reverb Time"), u("Delay/Reverb Depth"));
    let mut taps = vec![(0.05, 1.0)];
    if delay {
        let step = 0.05 + 0.25 * time;
        let mut level = 0.8;
        let mut at = 0.05 + step;
        while at <= 1.0 && level > 0.05 {
            taps.push((at, level));
            at += step;
            level *= 0.2 + 0.7 * depth;
        }
    }
    Graph::Effects {
        taps,
        tail: reverb.then_some((0.2 + 0.8 * time, 0.3 + 0.7 * depth)),
        sweep: on("Mod FX On").then_some((1.0 + 7.0 * u("Mod FX Speed"), u("Mod FX Depth"))),
    }
}

/// The graphs of `page` in `p`, with curves of about `n` points.
pub fn graphs(p: &Program, page: usize, n: usize) -> Vec<Graph> {
    let n = n.max(2);
    match page {
        OSCILLATORS => timbres(p).into_iter().map(|t| osc(p, t, n)).collect(),
        MIXER => vec![Graph::Levels(
            timbres(p)
                .into_iter()
                .flat_map(|t| ["Mixer VCO 1", "Mixer VCO 2", "Mixer Multi"].map(|f| unit(p, t, f)))
                .collect(),
        )],
        ENVELOPES => timbres(p)
            .into_iter()
            .flat_map(|t| [envelope(p, t, "Amp EG"), envelope(p, t, "EG"), lfo(p, t, n)])
            .collect(),
        EFFECTS => vec![effects(p)],
        _ => vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn graphs_cover_envelopes_and_lfos() {
        let mut p = Program::init();
        let g = graphs(&p, ENVELOPES, 32);
        assert!(matches!(
            g[..],
            [Graph::Envelope(_), Graph::Envelope(_), Graph::Wave(_)]
        ));
        if let Graph::Envelope(e) = &g[0] {
            assert_eq!(e[2].1, 1.0, "init holds full sustain");
        }
        p.set(field("Sub On"), 1).unwrap();
        assert_eq!(graphs(&p, ENVELOPES, 32).len(), 6);
        assert_eq!(
            graphs(&p, MIXER, 32),
            vec![Graph::Levels(vec![1.0, 0.0, 0.0, 1.0, 0.0, 0.0])]
        );
        assert!(matches!(&graphs(&p, OSCILLATORS, 16)[0], Graph::Wave(w) if w.len() == 16));
    }
}
