//! The unit's DSP (`units/oscyl-osc/engine.c`) in Rust, operation for
//! operation, for the apps' wave pictures and the patch maker's level
//! checks. Keep the two in step: `oscyl-osc-host` renders both and
//! fails when they part.

use super::{Kind, Patch, TABLE_LEN};

const RATE: f32 = 48000.0;
const CHUNK: usize = 64;
const ONE_255: f32 = 1.0 / 255.0;
const VOWELS: [[f32; 2]; 5] = [
    [730.0, 1090.0],
    [530.0, 1840.0],
    [270.0, 2290.0],
    [570.0, 840.0],
    [300.0, 870.0],
];
const SPREAD: [f32; 5] = [-1.0, -0.42, 0.0, 0.42, 1.0];

/// How the synth plays the unit: the note (`note << 8 | fine`), Shape and
/// Shift-Shape (0-1023) and the six parameters (0-100), as the logue SDK
/// hands them over.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Play {
    pub pitch: u16,
    pub shape: u16,
    pub shift_shape: u16,
    pub params: [u16; 6],
}

impl Play {
    /// `note`, every knob at `knob` (0 to 1).
    pub fn at(note: u8, knob: f32) -> Play {
        let k = knob.clamp(0.0, 1.0);
        Play {
            pitch: u16::from(note) << 8,
            shape: (k * 1023.0).round() as u16,
            shift_shape: (k * 1023.0).round() as u16,
            params: [(k * 100.0).round() as u16; 6],
        }
    }
}

/// The settings a patch's level is checked at: every knob low, middle,
/// high and mixed, at three notes.
pub fn probes() -> Vec<Play> {
    let mut out = vec![];
    for note in [36, 60, 84] {
        for k in [0.0, 0.5, 1.0] {
            out.push(Play::at(note, k));
        }
        out.push(Play {
            pitch: u16::from(note) << 8,
            shape: 1023,
            shift_shape: 0,
            params: [100, 0, 100, 0, 100, 0],
        });
    }
    out
}

fn clamp01(x: f32) -> f32 {
    x.clamp(0.0, 1.0)
}

fn floor_i(x: f32) -> i32 {
    let i = x as i32;
    if i as f32 > x { i - 1 } else { i }
}

fn frac(x: f32) -> f32 {
    x - floor_i(x) as f32
}

fn sin2pi(p: f32) -> f32 {
    let mut p = frac(p);
    let mut sign = 1.0;
    if p >= 0.5 {
        p -= 0.5;
        sign = -1.0;
    }
    let mut s = 16.0 * p * (0.5 - p);
    s = 0.225 * (s * s - s) + s;
    sign * s
}

fn exp2(x: f32) -> f32 {
    let i = floor_i(x);
    let f = x - i as f32;
    let m = 1.0 + f * (0.696_065_64 + f * (0.224_494_33 + f * 0.079_440_23));
    let i = i.clamp(-126, 127);
    m * f32::from_bits(((i + 127) as u32) << 23)
}

fn blep(mut t: f32, mut dt: f32) -> f32 {
    if dt > 0.5 {
        dt = 0.5;
    }
    if t < dt {
        t /= dt;
        return t + t - t * t - 1.0;
    }
    if t > 1.0 - dt {
        t = (t - 1.0) / dt;
        return t * t + t + t + 1.0;
    }
    0.0
}

fn saw(t: f32, dt: f32) -> f32 {
    2.0 * t - 1.0 - blep(t, dt)
}

fn noise(x: &mut u32) -> f32 {
    let mut v = *x;
    v ^= v << 13;
    v ^= v >> 17;
    v ^= v << 5;
    *x = v;
    (v >> 8) as f32 * (2.0 / 16_777_216.0) - 1.0
}

fn seconds(b: u8) -> f32 {
    0.001 * exp2(f32::from(b) * ONE_255 * 11.0)
}

#[derive(Clone, Copy, Default)]
struct Block {
    kind: u8,
    base: [f32; 7],
    v: [f32; 7],
    k: [f32; 4],
    s: [f32; 6],
    n: u32,
}

struct Engine<'a> {
    blocks: Vec<Block>,
    mods: Vec<(u8, u8, u8, f32)>,
    tables: &'a [[i16; TABLE_LEN]],
    gain: f32,
    lfo_rate: f32,
    lfo_shape: u8,
    env_attack: f32,
    env_decay: f32,
    env_sustain: f32,
    shape: f32,
    shift_shape: f32,
    params: [f32; 6],
    phase: f32,
    sub_phase: f32,
    lfo_phase: f32,
    lfo_value: f32,
    env: f32,
    env_stage: u8,
    dc_x: f32,
    dc_y: f32,
    noise: u32,
}

fn table_at(t: &[i16; TABLE_LEN], phase: f32) -> f32 {
    let idx = phase * TABLE_LEN as f32;
    let j = floor_i(idx);
    let f = idx - j as f32;
    let s0 = f32::from(t[(j & 255) as usize]);
    let s1 = f32::from(t[((j + 1) & 255) as usize]);
    (s0 + (s1 - s0) * f) * (1.0 / 32768.0)
}

impl<'a> Engine<'a> {
    /// As `oscyl_engine_load`; a patch with no source plays a saw.
    fn load(p: &'a Patch) -> Engine<'a> {
        let mut e = Engine {
            blocks: p
                .blocks
                .iter()
                .map(|b| Block {
                    kind: b.kind as u8,
                    base: b.k.map(|c| f32::from(c) * ONE_255),
                    ..Block::default()
                })
                .collect(),
            mods: p
                .mods
                .iter()
                .map(|m| {
                    (
                        m.source.to_u8(),
                        m.block,
                        m.param,
                        f32::from(m.amount) * (1.0 / 64.0),
                    )
                })
                .collect(),
            tables: &p.tables,
            gain: f32::from(p.gain) * (1.0 / 64.0),
            lfo_rate: 0.05 * exp2(f32::from(p.lfo_rate) * ONE_255 * 9.0),
            lfo_shape: p.lfo_shape,
            env_attack: seconds(p.attack),
            env_decay: seconds(p.decay),
            env_sustain: f32::from(p.sustain) * ONE_255,
            shape: 0.0,
            shift_shape: 0.0,
            params: [0.0; 6],
            phase: 0.0,
            sub_phase: 0.0,
            lfo_phase: 0.0,
            lfo_value: 0.0,
            env: 0.0,
            env_stage: 0,
            dc_x: 0.0,
            dc_y: 0.0,
            noise: 0x1234_5678,
        };
        if !p.blocks.iter().any(|b| b.kind.is_source()) {
            let mut saw = Block {
                kind: Kind::Saw as u8,
                ..Block::default()
            };
            saw.base[0] = 1.0;
            e.blocks = vec![saw];
            e.mods.clear();
            e.tables = &[];
            e.gain = 0.5;
            e.lfo_rate = 1.0;
            e.env_attack = 0.001;
            e.env_decay = 0.001;
            e.env_sustain = 1.0;
        }
        e.reset();
        e
    }

    fn reset(&mut self) {
        self.phase = 0.0;
        self.sub_phase = 0.0;
        self.env = 0.0;
        self.env_stage = 0;
        for b in &mut self.blocks {
            b.s = [0.0; 6];
            b.n = 0;
            if b.kind == Kind::Supersaw as u8 {
                for (i, s) in b.s.iter_mut().take(5).enumerate() {
                    *s = 0.2 * i as f32;
                }
            }
        }
    }

    fn param(&mut self, index: u16, value: u16) {
        match index {
            0..=5 => self.params[usize::from(index)] = clamp01(f32::from(value) * 0.01),
            6 => self.shape = clamp01(f32::from(value) * (1.0 / 1023.0)),
            7 => self.shift_shape = clamp01(f32::from(value) * (1.0 / 1023.0)),
            _ => {}
        }
    }

    fn resolve(&mut self, note: f32, w0: f32, shape_lfo: f32, frames: usize) {
        let dt = frames as f32 * (1.0 / RATE);
        self.lfo_phase += self.lfo_rate * dt;
        if self.lfo_phase >= 1.0 {
            self.lfo_phase = frac(self.lfo_phase);
            if self.lfo_shape >= 2 {
                self.lfo_value = noise(&mut self.noise);
            }
        }
        if self.lfo_shape == 0 {
            self.lfo_value = sin2pi(self.lfo_phase);
        } else if self.lfo_shape == 1 {
            self.lfo_value = 4.0 * (self.lfo_phase - 0.5).abs() - 1.0;
        }
        if self.env_stage == 0 {
            self.env += dt / self.env_attack;
            if self.env >= 1.0 {
                self.env = 1.0;
                self.env_stage = 1;
            }
        } else {
            let c = (1.0 - dt / self.env_decay).max(0.0);
            self.env = self.env_sustain + (self.env - self.env_sustain) * c;
        }

        let mut src = [0.0f32; 11];
        src[0] = clamp01(self.shape + shape_lfo);
        src[1] = self.shift_shape;
        src[2] = self.lfo_value;
        src[3] = self.env;
        src[4..10].copy_from_slice(&self.params);
        src[10] = (note - 60.0) * (1.0 / 64.0);

        for b in &mut self.blocks {
            b.v = b.base;
        }
        let n_blocks = self.blocks.len();
        for &(s, block, param, amount) in &self.mods {
            if usize::from(s) < src.len() && usize::from(block) < n_blocks && param < 7 {
                self.blocks[usize::from(block)].v[usize::from(param)] +=
                    amount * src[usize::from(s)];
            }
        }
        let f0 = w0 * RATE;
        let n_tables = self.tables.len() as i32;
        for b in &mut self.blocks {
            for v in &mut b.v {
                *v = clamp01(*v);
            }
            let (v, k) = (b.v, &mut b.k);
            match Kind::ALL.into_iter().find(|t| *t as u8 == b.kind) {
                Some(Kind::Wavetable) => {
                    let pos = v[0] * (if n_tables > 0 { n_tables - 1 } else { 0 }) as f32;
                    let mut i0 = floor_i(pos);
                    if n_tables > 1 && i0 >= n_tables - 1 {
                        i0 = n_tables - 2;
                    }
                    if i0 < 0 {
                        i0 = 0;
                    }
                    k[0] = i0 as f32;
                    k[1] = if n_tables > 1 { pos - i0 as f32 } else { 0.0 };
                }
                Some(Kind::Fm) => {
                    k[0] = (1 + floor_i(v[0] * 7.99)) as f32;
                    k[1] = v[1] * v[1] * 0.8;
                    k[2] = v[2] * 0.2;
                }
                Some(Kind::Supersaw) => k[0] = v[0] * 0.04,
                Some(Kind::Noise) => k[0] = (1 + floor_i(v[1] * v[1] * 63.0)) as f32,
                Some(Kind::PhaseDist) => {
                    let d = 0.5 - 0.48 * v[0];
                    k[0] = d;
                    k[1] = 0.5 / d;
                    k[2] = 0.5 / (1.0 - d);
                }
                Some(Kind::Sync) => k[0] = 1.0 + 7.0 * v[0],
                Some(Kind::Fold) => {
                    k[0] = 0.25 * (1.0 + 7.0 * v[0]);
                    k[1] = 0.125 * v[1];
                }
                Some(Kind::Crush) => {
                    k[0] = exp2(2.0 + (1.0 - v[0]) * 10.0);
                    k[1] = (1 + floor_i(v[1] * v[1] * 31.0)) as f32;
                }
                Some(Kind::Lowpass) => {
                    let fc = (f0 * exp2(v[0] * 7.0)).min(20000.0);
                    let w = std::f32::consts::TAU * fc * (1.0 / RATE);
                    k[0] = w / (1.0 + w);
                }
                Some(Kind::Formant) => {
                    let pos = v[0] * 4.0;
                    let a = floor_i(pos).min(3) as usize;
                    let f = pos - a as f32;
                    let shift = exp2(v[3] - 0.5);
                    for n in 0..2 {
                        let hz = (VOWELS[a][n] + (VOWELS[a + 1][n] - VOWELS[a][n]) * f) * shift;
                        k[n] = 2.0 * sin2pi(0.5 * hz * (1.0 / RATE));
                    }
                    k[2] = 1.0 - 0.9 * v[1];
                }
                _ => {}
            }
        }
    }

    fn render(&mut self, pitch: u16, shape_lfo: i32, out: &mut [f32]) {
        let note = f32::from((pitch >> 8) as u8) + f32::from((pitch & 0xFF) as u8) * (1.0 / 256.0);
        let mut w0 = 440.0 * exp2((note - 69.0) * (1.0 / 12.0)) * (1.0 / RATE);
        if w0 > 0.45 {
            w0 = 0.45;
        }
        self.resolve(
            note,
            w0,
            shape_lfo as f32 * (1.0 / 2_147_483_648.0),
            out.len(),
        );
        let tables = self.tables;
        for o in out.iter_mut() {
            let mut ph = self.phase + w0;
            if ph >= 1.0 {
                ph -= 1.0;
                self.sub_phase = if self.sub_phase >= 0.5 { 0.0 } else { 0.5 };
            }
            self.phase = ph;
            let (mut rp, mut dt, mut x) = (ph, w0, 0.0f32);
            for b in &mut self.blocks {
                let (v, k, s) = (b.v, b.k, &mut b.s);
                match Kind::ALL.into_iter().find(|t| *t as u8 == b.kind) {
                    Some(Kind::Saw) => x += v[0] * saw(rp, dt),
                    Some(Kind::Wavetable) => {
                        let y = if tables.is_empty() {
                            sin2pi(rp)
                        } else {
                            let i = k[0] as usize;
                            let mut y = table_at(&tables[i], rp);
                            if k[1] > 0.0 {
                                y += (table_at(&tables[i + 1], rp) - y) * k[1];
                            }
                            y
                        };
                        x += v[1] * y;
                    }
                    Some(Kind::Fm) => {
                        let m = sin2pi(rp * k[0] + k[2] * s[0]);
                        s[0] = m;
                        x += v[3] * sin2pi(rp + k[1] * m);
                    }
                    Some(Kind::Supersaw) => {
                        let mut y = 0.0;
                        for j in 0..5 {
                            let d = w0 * (1.0 + k[0] * SPREAD[j]);
                            let mut p = s[j] + d;
                            if p >= 1.0 {
                                p -= 1.0;
                            }
                            s[j] = p;
                            y += saw(p, d);
                        }
                        x += v[1] * y * 0.36;
                    }
                    Some(Kind::Sub) => {
                        let sp = self.sub_phase + 0.5 * ph;
                        let sq = if sp < 0.5 { 1.0 } else { -1.0 };
                        x += v[0] * (sin2pi(sp) + (sq - sin2pi(sp)) * v[1]);
                    }
                    Some(Kind::Noise) => {
                        if b.n as f32 >= k[0] - 0.5 || b.n == 0 {
                            s[0] = noise(&mut self.noise);
                            b.n = 0;
                        }
                        b.n += 1;
                        x += v[0] * s[0];
                    }
                    Some(Kind::PhaseDist) => {
                        rp = if rp < k[0] {
                            rp * k[1]
                        } else {
                            0.5 + (rp - k[0]) * k[2]
                        };
                    }
                    Some(Kind::Sync) => {
                        rp = frac(rp * k[0]);
                        dt *= k[0];
                    }
                    Some(Kind::Fold) => {
                        let y = sin2pi(k[0] * x + k[1]);
                        x += (y - x) * v[2];
                    }
                    Some(Kind::Crush) => {
                        if b.n as f32 >= k[1] - 0.5 || b.n == 0 {
                            let q = x * k[0];
                            s[0] = (q + if q >= 0.0 { 0.5 } else { -0.5 }) as i32 as f32 / k[0];
                            b.n = 0;
                        }
                        b.n += 1;
                        x += (s[0] - x) * v[2];
                    }
                    Some(Kind::Lowpass) => {
                        s[0] += k[0] * (x - s[0]);
                        x = s[0];
                    }
                    Some(Kind::Formant) => {
                        let mut y = 0.0;
                        for j in 0..2 {
                            s[2 * j] += k[j] * s[2 * j + 1];
                            let hp = x - s[2 * j] - k[2] * s[2 * j + 1];
                            s[2 * j + 1] += k[j] * hp;
                            y += s[2 * j + 1] * if j == 0 { 1.0 } else { 0.7 };
                        }
                        x += (y * k[2] - x) * v[2];
                    }
                    None => {}
                }
            }
            let y = x * self.gain;
            let mut hp = y - self.dc_x + 0.995 * self.dc_y;
            self.dc_x = y;
            self.dc_y = hp;
            let mut a = hp.abs();
            if a > 0.75 {
                let z = (a - 0.75) * 4.0;
                let t = if z >= 3.0 {
                    1.0
                } else {
                    z * (27.0 + z * z) / (27.0 + 9.0 * z * z)
                };
                a = 0.75 + 0.25 * t;
                hp = if hp < 0.0 { -a } else { a };
            }
            *o = hp;
        }
    }
}

/// `frames` samples of `patch` played as `play` from a note's start,
/// rendered as the unit does (64 at a time), each within -1 to 1.
pub fn render(patch: &Patch, play: &Play, frames: usize) -> Vec<f32> {
    let mut e = Engine::load(patch);
    for (i, v) in play.params.iter().enumerate() {
        e.param(i as u16, *v);
    }
    e.param(6, play.shape);
    e.param(7, play.shift_shape);
    e.reset();
    let mut out = vec![0.0; frames];
    for chunk in out.chunks_mut(CHUNK) {
        e.render(play.pitch, 0, chunk);
    }
    out
}

/// One cycle of `patch` as a picture, `points` long: a C below middle C
/// with every knob at half, once it has settled, scaled so its peak is 1
/// (a picture shows the shape; the level is the gain's business).
pub fn picture(patch: &Patch, points: usize) -> Vec<f32> {
    let note = 48u8;
    let period = RATE / (440.0 * 2f32.powf((f32::from(note) - 69.0) / 12.0));
    let s = render(patch, &Play::at(note, 0.5), 4096);
    let start = s.len() as f32 - period - 1.0;
    let cycle: Vec<f32> = (0..points)
        .map(|i| {
            let at = start + period * i as f32 / points as f32;
            let j = at as usize;
            let f = at - j as f32;
            s[j] + (s[j + 1] - s[j]) * f
        })
        .collect();
    let peak = cycle.iter().fold(0.0f32, |m, x| m.max(x.abs()));
    if peak > 1e-6 {
        cycle.iter().map(|x| x / peak).collect()
    } else {
        cycle
    }
}
