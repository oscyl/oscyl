//! Wave makers: single-cycle tables for the engine's wavetable block, from
//! harmonics, drawn points, a single-cycle WAV file, or a seeded family of
//! tables that morph into each other.

use std::f32::consts::TAU;

use super::{TABLE_LEN, Table};
use crate::Error;

/// `samples` (one cycle, any length) as a table: resampled, its DC taken
/// out, its peak brought to 95 %. A silent cycle is a silent table.
fn table(samples: &[f32]) -> Table {
    let n = samples.len().max(1);
    let at = |i: usize| {
        let x = i as f32 * n as f32 / TABLE_LEN as f32;
        let j = x as usize;
        let f = x - j as f32;
        let a = samples.get(j % n).copied().unwrap_or(0.0);
        let b = samples.get((j + 1) % n).copied().unwrap_or(0.0);
        a + (b - a) * f
    };
    let raw: Vec<f32> = (0..TABLE_LEN).map(at).collect();
    let dc = raw.iter().sum::<f32>() / TABLE_LEN as f32;
    let peak = raw.iter().fold(0.0f32, |m, x| m.max((x - dc).abs()));
    let scale = if peak > 1e-6 { 0.95 * 32767.0 / peak } else { 0.0 };
    std::array::from_fn(|i| ((raw[i] - dc) * scale).round() as i16)
}

/// A table of sine harmonics: `amps[0]` the fundamental's level, `amps[1]`
/// the second harmonic's, and so on (at most 64 are used).
pub fn harmonics(amps: &[f32]) -> Table {
    let cycle: Vec<f32> = (0..TABLE_LEN)
        .map(|i| {
            let p = i as f32 / TABLE_LEN as f32;
            amps.iter()
                .take(64)
                .enumerate()
                .map(|(h, a)| a * (TAU * (h + 1) as f32 * p).sin())
                .sum()
        })
        .collect();
    table(&cycle)
}

/// A table through drawn points, evenly spaced over one cycle, joined by
/// straight lines (two points at least make a wave).
pub fn drawn(points: &[f32]) -> Table {
    table(points)
}

/// A table from a WAV file holding one cycle: PCM (8, 16, 24 or 32 bits)
/// or 32-bit float, its first channel.
pub fn wav(bytes: &[u8]) -> Result<Table, Error> {
    let bad = |why| Error::Malformed(why);
    if bytes.len() < 12 || &bytes[..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err(bad("not a WAV file"));
    }
    let (mut format, mut data) = (None, None);
    let mut at = 12;
    while at + 8 <= bytes.len() {
        let id = &bytes[at..at + 4];
        let len = u32::from_le_bytes(bytes[at + 4..at + 8].try_into().expect("4")) as usize;
        let body = &bytes[at + 8..(at + 8 + len).min(bytes.len())];
        match id {
            b"fmt " if body.len() >= 16 => format = Some(body),
            b"data" => data = Some(body),
            _ => {}
        }
        at += 8 + len + (len & 1);
    }
    let (format, data) = format.zip(data).ok_or_else(|| bad("a WAV file without sound"))?;
    let tag = u16::from_le_bytes([format[0], format[1]]);
    let channels = usize::from(u16::from_le_bytes([format[2], format[3]])).max(1);
    let bits = u16::from_le_bytes([format[14], format[15]]);
    let width = usize::from(bits / 8);
    if width == 0 || width > 4 {
        return Err(bad("a WAV sample size Oscyl does not read"));
    }
    let sample = |b: &[u8]| -> Option<f32> {
        Some(match (tag, bits) {
            (3, 32) => f32::from_le_bytes(b.try_into().ok()?),
            (1 | 0xFFFE, 8) => (f32::from(b[0]) - 128.0) / 128.0,
            (1 | 0xFFFE, 16) => f32::from(i16::from_le_bytes([b[0], b[1]])) / 32768.0,
            (1 | 0xFFFE, 24) => (i32::from_le_bytes([0, b[0], b[1], b[2]]) >> 8) as f32 / 8_388_608.0,
            (1 | 0xFFFE, 32) => i32::from_le_bytes(b.try_into().ok()?) as f32 / 2_147_483_648.0,
            _ => return None,
        })
    };
    let cycle = data
        .chunks_exact(width * channels)
        .map(|frame| sample(&frame[..width]))
        .collect::<Option<Vec<f32>>>()
        .ok_or_else(|| bad("a WAV format Oscyl does not read"))?;
    if cycle.len() < 2 {
        return Err(bad("a WAV file too short for a cycle"));
    }
    Ok(table(&cycle))
}

/// A seeded family of `n` tables (one to eight) that morph from one to the
/// next: spectra that drift, a pulse that narrows, a resonance that sweeps
/// up the harmonics, or drawn shapes that smooth out.
pub fn family(seed: u64, n: usize) -> Vec<Table> {
    let mut r = super::make::Rng::new(seed ^ 0x7761_7665);
    let n = n.clamp(1, 8);
    let at = |i: usize| if n == 1 { 0.0 } else { i as f32 / (n - 1) as f32 };
    match r.below(4) {
        0 => {
            let spectrum = |r: &mut super::make::Rng| -> Vec<f32> {
                let tilt = 0.6 + r.f() * 1.2;
                (1..=48)
                    .map(|h| r.f().powi(2) / (h as f32).powf(tilt))
                    .collect()
            };
            let (a, b) = (spectrum(&mut r), spectrum(&mut r));
            (0..n)
                .map(|i| {
                    let t = at(i);
                    let amps: Vec<f32> = a.iter().zip(&b).map(|(x, y)| x + (y - x) * t).collect();
                    harmonics(&amps)
                })
                .collect()
        }
        1 => {
            let (from, to) = (0.5 - r.f() * 0.1, 0.05 + r.f() * 0.15);
            (0..n)
                .map(|i| {
                    let w = from + (to - from) * at(i);
                    // A band-limited pulse of width w.
                    let amps: Vec<f32> = (1..=48)
                        .map(|h| (std::f32::consts::PI * h as f32 * w).sin() / h as f32)
                        .collect();
                    harmonics(&amps)
                })
                .collect()
        }
        2 => {
            let (lo, hi) = (1.0 + r.f() * 3.0, 8.0 + r.f() * 24.0);
            let width = 1.0 + r.f() * 3.0;
            (0..n)
                .map(|i| {
                    let centre = lo + (hi - lo) * at(i);
                    let amps: Vec<f32> = (1..=48)
                        .map(|h| {
                            let d = (h as f32 - centre) / width;
                            0.3 / h as f32 + (-d * d).exp()
                        })
                        .collect();
                    harmonics(&amps)
                })
                .collect()
        }
        _ => {
            let points: Vec<f32> = (0..4 + r.below(9)).map(|_| r.f() * 2.0 - 1.0).collect();
            (0..n)
                .map(|i| {
                    // Each table keeps fewer of the drawing's harmonics.
                    let keep = 48.0 - 44.0 * at(i);
                    smooth(&drawn(&points), keep as usize)
                })
                .collect()
        }
    }
}

/// `t` with only its first `keep` harmonics.
fn smooth(t: &Table, keep: usize) -> Table {
    let amps: Vec<(f32, f32)> = (1..=keep.max(1))
        .map(|h| {
            let (mut re, mut im) = (0.0, 0.0);
            for (i, s) in t.iter().enumerate() {
                let a = TAU * h as f32 * i as f32 / TABLE_LEN as f32;
                re += f32::from(*s) * a.cos();
                im += f32::from(*s) * a.sin();
            }
            (re, im)
        })
        .collect();
    let cycle: Vec<f32> = (0..TABLE_LEN)
        .map(|i| {
            amps.iter()
                .enumerate()
                .map(|(h, (re, im))| {
                    let a = TAU * (h + 1) as f32 * i as f32 / TABLE_LEN as f32;
                    re * a.cos() + im * a.sin()
                })
                .sum()
        })
        .collect();
    table(&cycle)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn peak(t: &Table) -> i16 {
        t.iter().map(|s| s.saturating_abs()).max().unwrap()
    }

    #[test]
    fn makers_give_full_dc_free_tables() {
        let sine = harmonics(&[1.0]);
        assert!((sine[64] - 31128).abs() < 50, "{}", sine[64]);
        assert!(sine[0].abs() < 50);
        let tri = drawn(&[0.0, 1.0, 0.0, -1.0]);
        assert!((peak(&tri) - 31129).abs() <= 1, "{}", peak(&tri));
        for seed in 0..40 {
            for t in family(seed, 6) {
                let dc: i32 = t.iter().map(|s| i32::from(*s)).sum::<i32>() / 256;
                assert!(dc.abs() < 200, "seed {seed}: DC {dc}");
                assert!(peak(&t) > 30000, "seed {seed}");
            }
        }
        assert_eq!(family(3, 5), family(3, 5));
        assert_ne!(family(3, 5), family(4, 5));
    }

    #[test]
    fn a_wav_cycle_reads_in_any_common_format() {
        fn wav16(samples: &[i16]) -> Vec<u8> {
            let data: Vec<u8> = samples.iter().flat_map(|s| s.to_le_bytes()).collect();
            let mut b = b"RIFF\0\0\0\0WAVEfmt ".to_vec();
            b.extend_from_slice(&16u32.to_le_bytes());
            b.extend_from_slice(&[1, 0, 1, 0]);
            b.extend_from_slice(&48000u32.to_le_bytes());
            b.extend_from_slice(&96000u32.to_le_bytes());
            b.extend_from_slice(&[2, 0, 16, 0]);
            b.extend_from_slice(b"data");
            b.extend_from_slice(&(data.len() as u32).to_le_bytes());
            b.extend_from_slice(&data);
            b
        }
        let square: Vec<i16> = (0..600).map(|i| if i < 300 { 9000 } else { -9000 }).collect();
        let t = wav(&wav16(&square)).unwrap();
        assert!(t[10] > 30000 && t[200] < -30000);
        assert!(wav(b"RIFF....WAVE").is_err());
        assert!(wav(&wav16(&[1])).is_err());
    }
}
