//! Generated sounds: pick a [`Character`], shape it with four [`Macros`],
//! and a seed makes a complete, playable program. [`vary`] mutates a
//! program, [`breed`] crosses two, and [`hand`] deals six different ones
//! with six different names.
//!
//! Everything is deterministic: the same recipe and seed always give the
//! same bytes. Each field draws its random numbers from its own stream
//! (seed and field), so turning a macro changes only the arithmetic, never
//! which random numbers a field gets: more `bright` can only open the
//! filter, more `space` only adds reverb.
//!
//! Every program passes a coherence pass: something always sounds, the
//! note is heard, resonance is tamed under full drive, Arp and Chord depths
//! sit mid-band, no user oscillator or effect slot is used, and the level
//! never goes above 0 dB.

use crate::present::{self, Category, Control, Stage, depth_choices, depth_raw};
use crate::{Field, Program};
use Field::*;
use oscyl_driver::Family;
use oscyl_logue::kinds::VPM_SAFE;

/// SplitMix64: tiny, fast, and good enough to make sounds.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Rng {
        Rng(seed)
    }

    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in `0.0..1.0`.
    fn unit(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n.max(1) as u64) as usize
    }
}

/// A kind of sound to generate. Rock first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Character {
    PowerChord,
    FuzzBass,
    ScreamLead,
    Riser,
    Wobble,
    Pad,
    Pluck,
    Keys,
    Drone,
    SpaceFx,
    ArpPulse,
    Bell,
    Brass,
    Strings,
    Organ,
    SubBass,
    Acid,
    SyncLead,
    FmGlass,
    FmBass,
    Chiptune,
    Kick,
    Snare,
    Laser,
    Siren,
    Wind,
    Impact,
    Alarm,
}

impl Character {
    pub const ALL: [Character; 28] = [
        Character::PowerChord,
        Character::FuzzBass,
        Character::ScreamLead,
        Character::Riser,
        Character::Wobble,
        Character::Pad,
        Character::Pluck,
        Character::Keys,
        Character::Drone,
        Character::SpaceFx,
        Character::ArpPulse,
        Character::Bell,
        Character::Brass,
        Character::Strings,
        Character::Organ,
        Character::SubBass,
        Character::Acid,
        Character::SyncLead,
        Character::FmGlass,
        Character::FmBass,
        Character::Chiptune,
        Character::Kick,
        Character::Snare,
        Character::Laser,
        Character::Siren,
        Character::Wind,
        Character::Impact,
        Character::Alarm,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Character::PowerChord => "Power Chord",
            Character::FuzzBass => "Fuzz Bass",
            Character::ScreamLead => "Scream Lead",
            Character::Riser => "Riser",
            Character::Wobble => "Wobble",
            Character::Pad => "Pad",
            Character::Pluck => "Pluck",
            Character::Keys => "Keys",
            Character::Drone => "Drone",
            Character::SpaceFx => "Space FX",
            Character::ArpPulse => "Arp Pulse",
            Character::Bell => "Bell",
            Character::Brass => "Brass",
            Character::Strings => "Strings",
            Character::Organ => "Organ",
            Character::SubBass => "Sub Bass",
            Character::Acid => "Acid",
            Character::SyncLead => "Sync Lead",
            Character::FmGlass => "FM Glass",
            Character::FmBass => "FM Bass",
            Character::Chiptune => "Chiptune",
            Character::Kick => "Kick",
            Character::Snare => "Snare",
            Character::Laser => "Laser",
            Character::Siren => "Siren",
            Character::Wind => "Wind",
            Character::Impact => "Impact",
            Character::Alarm => "Alarm",
        }
    }

    /// One line on what it sounds like.
    pub fn hint(self) -> &'static str {
        match self {
            Character::PowerChord => "Driven fifths for riffs",
            Character::FuzzBass => "Low, fuzzy and tight",
            Character::ScreamLead => "Gliding solo with bite",
            Character::Riser => "Pitch climbs into noise",
            Character::Wobble => "Tempo-locked wobble",
            Character::Pad => "Slow swell with chorus",
            Character::Pluck => "Short and percussive",
            Character::Keys => "Quick, playable chords",
            Character::Drone => "Endless low tone",
            Character::SpaceFx => "Sirens, lasers, alien air",
            Character::ArpPulse => "A self-running pattern",
            Character::Bell => "Metallic ring, long decay",
            Character::Brass => "Bold swell with a filter blat",
            Character::Strings => "Bowed ensemble",
            Character::Organ => "Drawbars, straight on and off",
            Character::SubBass => "Clean weight below",
            Character::Acid => "Squelching, sliding line",
            Character::SyncLead => "Hard sync sweep",
            Character::FmGlass => "Glassy digital keys",
            Character::FmBass => "Knocking digital bass",
            Character::Chiptune => "Old game console pulse",
            Character::Kick => "Bass drum: a pitch drop",
            Character::Snare => "Noise crack with a body",
            Character::Laser => "A fast zap",
            Character::Siren => "Rising and falling wail",
            Character::Wind => "Whistling air the keys play",
            Character::Impact => "Huge low hit with a tail",
            Character::Alarm => "Two-tone warning",
        }
    }

    /// What it is for.
    pub fn family(self) -> Family {
        use Character::*;
        match self {
            Keys | Pluck | Bell | Organ | FmGlass => Family::Keys,
            FuzzBass | Wobble | SubBass | Acid | FmBass => Family::Bass,
            PowerChord | ScreamLead | Brass | SyncLead | Chiptune => Family::Lead,
            Pad | Drone | Strings => Family::Pad,
            ArpPulse | Kick | Snare => Family::Rhythm,
            Riser | SpaceFx | Laser | Siren | Wind | Impact | Alarm => Family::Effect,
        }
    }

    /// The category [`present::category`] gives its programs.
    pub fn category(self) -> Category {
        use Character::*;
        match self {
            PowerChord | ScreamLead | Brass | SyncLead | Chiptune => Category::Lead,
            FuzzBass | SubBass | Acid | FmBass => Category::Bass,
            // Drums and effects alike: a pitch drop or noise alone.
            Riser | Wobble | SpaceFx | Kick | Snare | Laser | Siren | Wind | Impact | Alarm => {
                Category::Fx
            }
            Pad | Strings => Category::Pad,
            Pluck => Category::Pluck,
            Keys | Organ | FmGlass => Category::Keys,
            Drone => Category::Drone,
            ArpPulse => Category::Arp,
            Bell => Category::Bell,
        }
    }

    /// Words for names: `(first, second)`.
    fn words(self) -> (&'static [&'static str], &'static [&'static str]) {
        match self {
            Character::PowerChord => (
                &[
                    "Iron", "Steel", "Loud", "Raw", "Arena", "Hot", "Big", "Thick", "Amp",
                ],
                &[
                    "Fifths", "Riff", "Wall", "Chug", "Crunch", "Power", "Stack", "Anthem",
                ],
            ),
            Character::FuzzBass => (
                &[
                    "Mud", "Fuzz", "Low", "Dirt", "Tar", "Grit", "Deep", "Heavy", "Rust",
                ],
                &["Fuzz", "Bass", "Rumble", "Growl", "Sludge", "Thud", "Boots"],
            ),
            Character::ScreamLead => (
                &["Hot", "Wild", "Sky", "Burning", "Razor", "Feral", "Red"],
                &["Scream", "Wail", "Solo", "Howl", "Lead", "Shred", "Bite"],
            ),
            Character::Riser => (
                &["Glass", "Up", "Rocket", "Tidal", "Storm", "Slow", "White"],
                &["Rise", "Lift", "Climb", "Surge", "Swell", "Launch"],
            ),
            Character::Wobble => (
                &["Wub", "Bent", "Warp", "Rubber", "Drunk", "Wonky", "Liquid"],
                &["Wobble", "Wub", "Warp", "Bounce", "Bend", "Lurch"],
            ),
            Character::Pad => (
                &[
                    "Slow", "Silk", "Warm", "Velvet", "Soft", "Dusk", "Cloud", "Amber",
                ],
                &["Pad", "Bloom", "Haze", "Choir", "Field", "Veil", "Glow"],
            ),
            Character::Pluck => (
                &[
                    "Tight", "Quick", "Glass", "Wood", "Bright", "Short", "Nylon",
                ],
                &["Pluck", "Pick", "Snap", "Ping", "Drop", "Tick"],
            ),
            Character::Keys => (
                &["Warm", "Old", "Club", "Soft", "Round", "Ivory", "Smoke"],
                &["Keys", "Piano", "Organ", "Comp", "Chords", "Stab"],
            ),
            Character::Drone => (
                &["Deep", "Long", "Black", "Low", "Endless", "Monk", "Iron"],
                &["Drone", "Hum", "Tone", "Throat", "Mantra", "Void"],
            ),
            Character::SpaceFx => (
                &["Star", "Orbit", "Ion", "Comet", "Laser", "Alien", "Nova"],
                &["Ray", "Beam", "Signal", "Probe", "Drift", "Wave"],
            ),
            Character::ArpPulse => (
                &["Pulse", "Night", "Neon", "Tick", "Motor", "Chase", "City"],
                &["Arp", "Run", "Pulse", "Steps", "Motor", "Grid"],
            ),
            Character::Bell => (
                &["Glass", "Tin", "Ice", "Silver", "Temple", "Clear", "Cold"],
                &["Bell", "Chime", "Tine", "Ring", "Gong", "Toll"],
            ),
            Character::Brass => (
                &[
                    "Brass", "Gold", "Royal", "Bold", "Hot", "Big", "Horn", "Regal",
                ],
                &[
                    "Brass", "Horns", "Fanfare", "Stab", "Section", "Blare", "Swell",
                ],
            ),
            Character::Strings => (
                &["Silk", "Slow", "Wide", "Grand", "Soft", "Autumn", "Old"],
                &[
                    "Strings", "Bows", "Ensemble", "Section", "Quartet", "Legato",
                ],
            ),
            Character::Organ => (
                &["Chapel", "Rock", "Dusty", "Full", "Soul", "Night", "Pipe"],
                &["Organ", "Drawbar", "Stops", "Rotor", "Gospel", "Reed"],
            ),
            Character::SubBass => (
                &["Sub", "Deep", "Low", "Round", "Dark", "Floor", "Pure"],
                &["Sub", "Bass", "Boom", "Weight", "Floor", "Depth"],
            ),
            Character::Acid => (
                &["Acid", "Squelch", "Silver", "Sour", "Rubber", "Wet", "Lime"],
                &["Acid", "Line", "Squelch", "Bite", "Box", "Worm"],
            ),
            Character::SyncLead => (
                &["Sync", "Hard", "Tear", "Vocal", "Sharp", "Neon", "Chrome"],
                &["Sync", "Lead", "Sweep", "Rip", "Cry", "Edge"],
            ),
            Character::FmGlass => (
                &["Glass", "Crystal", "Clear", "Frost", "Mint", "Tine", "Opal"],
                &["Glass", "Keys", "Tines", "EP", "Chime", "Prism"],
            ),
            Character::FmBass => (
                &["FM", "Knock", "Rubber", "Solid", "Punch", "Slap", "Hard"],
                &["Bass", "Knock", "Thump", "Pluck", "Donk", "Low"],
            ),
            Character::Chiptune => (
                &["Chip", "8-Bit", "Pixel", "Arcade", "Tiny", "Retro", "Coin"],
                &["Lead", "Blip", "Pulse", "Square", "Tune", "Jump"],
            ),
            Character::Kick => (
                &["Deep", "Hard", "Round", "Short", "Club", "Dry", "Big"],
                &["Kick", "Thump", "Boom", "Punch", "Drum", "Knock"],
            ),
            Character::Snare => (
                &["Tight", "Crisp", "Dry", "Fat", "Tin", "Snap", "Hot"],
                &["Snare", "Crack", "Hit", "Rim", "Clap", "Slap"],
            ),
            Character::Laser => (
                &["Laser", "Zap", "Photon", "Ray", "Plasma", "Blaster", "Pew"],
                &["Zap", "Shot", "Beam", "Bolt", "Gun", "Blast"],
            ),
            Character::Siren => (
                &["Air", "Raid", "Night", "Slow", "Red", "City", "Dub"],
                &["Siren", "Wail", "Whoop", "Alert", "Howl", "Call"],
            ),
            Character::Wind => (
                &["North", "Cold", "Night", "Desert", "Ghost", "High", "Storm"],
                &["Wind", "Gale", "Breeze", "Howl", "Draft", "Whistle"],
            ),
            Character::Impact => (
                &["Big", "Deep", "Heavy", "Dark", "Giant", "Trailer", "Doom"],
                &["Impact", "Hit", "Boom", "Slam", "Crash", "Quake"],
            ),
            Character::Alarm => (
                &["Red", "Loud", "Panic", "Clock", "Fire", "Urgent", "Code"],
                &["Alarm", "Alert", "Beep", "Buzzer", "Warning", "Bleep"],
            ),
        }
    }
}

/// Four ways to shape a character, each `0.0..=1.0`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Macros {
    /// Cutoff, resonance and the second oscillator's octave.
    pub bright: f32,
    /// LFO amount and rate, EG amount, modulation effect.
    pub movement: f32,
    /// Drive, cross mod, ring and sync, noise.
    pub grit: f32,
    /// Delay and reverb amount, time and mix.
    pub space: f32,
}

impl Default for Macros {
    fn default() -> Self {
        Macros {
            bright: 0.5,
            movement: 0.5,
            grit: 0.5,
            space: 0.5,
        }
    }
}

/// What to generate.
#[derive(Debug, Clone, PartialEq)]
pub struct Recipe {
    pub character: Character,
    pub macros: Macros,
    pub seed: u64,
    /// Stages copied from the base program by [`generate_from`].
    pub keep: Vec<Stage>,
}

/// Programs per deal.
pub const HAND: usize = 6;

// Raw values, as documented in the KORG MIDI implementation.
const SQR: u16 = 0;
const TRI: u16 = 1;
const SAW: u16 = 2;
const FT16: u16 = 0;
const FT8: u16 = 1;
const FT4: u16 = 2;
const FT2: u16 = 3;
const N_HIGH: u16 = 0;
const N_LOW: u16 = 1;
const N_PEAK: u16 = 2;
const N_DECIM: u16 = 3;
const NOISE: u16 = 0;
const VPM: u16 = 1;
const ARP: u16 = 1;
const CHORD: u16 = 2;
const UNISON: u16 = 3;
const POLY: u16 = 4;
const TO_CUTOFF: u16 = 0;
const TO_SHAPE: u16 = 1;
const TO_PITCH: u16 = 2;
const EG_PITCH2: u16 = 1;
const LFO_ONE_SHOT: u16 = 0;
const LFO_BPM: u16 = 2;
const CHORUS: u16 = 1;
const ENSEMBLE: u16 = 2;
const PHASER: u16 = 3;
const FLANGER: u16 = 4;
// Delay types.
const D_STEREO: u16 = 0;
const D_MONO: u16 = 1;
const D_PING: u16 = 2;
const D_TAPE: u16 = 4;
const D_ONE_TAP: u16 = 5;
const D_STEREO_BPM: u16 = 6;
const D_MONO_BPM: u16 = 7;
const D_PING_BPM: u16 = 8;
const D_TAPE_BPM: u16 = 10;
// Reverb types.
const R_HALL: u16 = 0;
const R_SMOOTH: u16 = 1;
const R_ARENA: u16 = 2;
const R_PLATE: u16 = 3;
const R_ROOM: u16 = 4;
const R_SPACE: u16 = 6;
const R_RISER: u16 = 7;
const R_SUBMARINE: u16 = 8;
const R_HORROR: u16 = 9;
/// The last built-in delay and reverb types; above them are user slots.
const LAST_DELAY: u16 = 11;
const LAST_REVERB: u16 = 9;

/// A program being made: every random number is keyed by field, so
/// macros never shift another field's draws.
struct Make {
    p: Program,
    seed: u64,
    m: Macros,
}

impl Make {
    /// Random `0.0..1.0` for `field`'s `k`-th draw.
    fn roll(&self, field: Field, k: u64) -> f32 {
        let s = field.spec();
        let key = ((s.offset as u64) << 12) | (u64::from(s.shift) << 8) | k;
        Rng::new(self.seed ^ key.wrapping_mul(0xD1B5_4A32_D192_ED03)).unit()
    }

    /// Set `field` to `v`, rounded and clamped to its range.
    fn put(&mut self, field: Field, v: f32) {
        let s = field.spec();
        let v = v.round().clamp(f32::from(s.min), f32::from(s.max)) as u16;
        self.p.set(field, v).expect("clamped to the field's range");
    }

    fn set(&mut self, field: Field, v: u16) {
        self.put(field, f32::from(v));
    }

    /// A random value in `lo..=hi`.
    fn int(&mut self, field: Field, lo: u16, hi: u16) {
        let span = f32::from(hi - lo) + 1.0;
        let v = (f32::from(lo) + (self.roll(field, 0) * span).floor()).min(f32::from(hi));
        self.put(field, v);
    }

    /// `lo..hi` by `t`, plus up to `jitter` either way.
    fn mac(&mut self, field: Field, lo: u16, hi: u16, t: f32, jitter: u16) {
        let t = t.clamp(0.0, 1.0);
        let base = f32::from(lo) + (f32::from(hi) - f32::from(lo)) * t;
        let j = (self.roll(field, 3) * 2.0 - 1.0) * f32::from(jitter);
        self.put(field, base + j);
    }

    /// One of `options`.
    fn pick(&mut self, field: Field, options: &[u16]) {
        let i = (self.roll(field, 1) * options.len() as f32) as usize;
        self.set(field, options[i.min(options.len() - 1)]);
    }

    fn chance(&self, field: Field, p: f32) -> bool {
        self.roll(field, 2) < p
    }

    /// Bright: cutoff across `cut`, resonance across `res`.
    fn bright(&mut self, cut: (u16, u16), res: (u16, u16)) {
        let b = self.m.bright;
        self.mac(Cutoff, cut.0, cut.1, b, 110);
        self.mac(Resonance, res.0, res.1, b, 90);
    }

    /// The second oscillator an octave up when bright, else `low`.
    fn vco2_octave(&mut self, low: &[u16]) {
        if self.chance(Vco2Octave, (self.m.bright - 0.55) * 1.5) {
            self.set(Vco2Octave, FT4);
        } else {
            self.pick(Vco2Octave, low);
        }
    }

    /// Grit: drive (at least `drive`), cross mod up to `xmod`, ring and
    /// sync when very gritty, noise when `noise`, and the level a little
    /// lower when loud.
    fn grit(&mut self, drive: u16, xmod: u16, noise: bool) {
        let g = self.m.grit.clamp(0.0, 1.0);
        let by_grit = if g < 0.34 {
            0
        } else if g < 0.67 {
            1
        } else {
            2
        };
        self.set(CutoffDrive, by_grit.max(drive));
        self.mac(CrossModDepth, 0, xmod, (g - 0.45) / 0.55, 30);
        let wild = (g - 0.7) / 0.3 * 0.35;
        self.set(Ring, u16::from(self.chance(Ring, wild)));
        self.set(Sync, u16::from(self.chance(Sync, wild)));
        if noise {
            self.set(MultiType, NOISE);
            self.pick(SelectNoise, &[0, 1, 3]);
            self.int(ShapeNoise, 200, 800);
            self.mac(MultiLevel, 0, 380, (g - 0.5) * 2.0, 20);
        }
        self.put(ProgramLevel, 102.0 - ((g - 0.75).max(0.0) * 40.0));
    }

    /// Movement: the LFO on `target` across `int`, the EG across `eg`, and
    /// one of `fx` switched on above half movement.
    fn movement(&mut self, target: u16, int: (u16, u16), eg: (u16, u16), fx: &[u16]) {
        let mv = self.m.movement;
        self.set(LfoTarget, target);
        self.pick(LfoWave, &[SQR, TRI, SAW]);
        self.mac(LfoInt, int.0, int.1, mv, 20);
        self.mac(LfoRate, 150, 750, mv, 80);
        self.mac(EgInt, eg.0, eg.1, mv, 25);
        if !fx.is_empty() {
            self.pick(ModFxType, fx);
            self.int(ModFxChorus, 0, 7);
            self.int(ModFxEnsemble, 0, 2);
            self.int(ModFxPhaser, 0, 7);
            self.int(ModFxFlanger, 0, 7);
            self.mac(ModFxTime, 150, 700, mv, 60);
            self.mac(ModFxDepth, 150, 800, mv, 60);
            self.set(ModFxOn, u16::from(mv >= 0.5));
        }
    }

    /// Space: delay and reverb, with `bias` added to the space macro.
    fn space(&mut self, bias: f32, delays: &[u16], reverbs: &[u16]) {
        let sp = (self.m.space + bias).clamp(0.0, 1.0);
        self.pick(DelaySubType, delays);
        self.pick(ReverbSubType, reverbs);
        self.set(DelayOn, u16::from(sp >= 0.3));
        self.mac(DelayTime, 250, 800, sp, 60);
        self.mac(DelayDepth, 80, 600, sp, 40);
        self.mac(DelayDryWet, 60, 520, sp, 30);
        self.set(ReverbOn, u16::from(sp >= 0.12));
        self.mac(ReverbTime, 250, 1000, sp, 60);
        self.mac(ReverbDepth, 200, 900, sp, 40);
        self.mac(ReverbDryWet, 40, 640, sp, 30);
    }

    /// Two oscillators, `waves` each, slightly detuned, at full level.
    fn oscillators(&mut self, w1: &[u16], w2: &[u16]) {
        self.pick(Vco1Wave, w1);
        self.pick(Vco2Wave, w2);
        self.int(Vco1Shape, 0, 250);
        self.int(Vco2Shape, 0, 250);
        self.int(Vco2Pitch, 536, 552);
        self.int(Vco1Level, 820, 1000);
        self.int(Vco2Level, 620, 900);
        self.pick(CutoffKeyTrack, &[1, 2]);
        self.int(CutoffVelocity, 0, 60);
        self.int(AmpVelocity, 40, 110);
    }

    fn amp(&mut self, a: (u16, u16), d: (u16, u16), s: (u16, u16), r: (u16, u16)) {
        self.int(AmpEgAttack, a.0, a.1);
        self.int(AmpEgDecay, d.0, d.1);
        self.int(AmpEgSustain, s.0, s.1);
        self.int(AmpEgRelease, r.0, r.1);
    }

    fn eg(&mut self, target: u16, a: (u16, u16), d: (u16, u16)) {
        self.set(EgTarget, target);
        self.int(EgAttack, a.0, a.1);
        self.int(EgDecay, d.0, d.1);
    }

    /// The multi engine as VPM: one of `types` at `level`, its modulation
    /// depth `t` of the way through what the type takes ([`VPM_SAFE`]),
    /// feedback and noise by grit, the modulation envelope's reach by
    /// movement.
    fn vpm(&mut self, types: &[u16], t: f32, level: (u16, u16)) {
        self.set(MultiType, VPM);
        self.pick(SelectVpm, types);
        let v = &VPM_SAFE[usize::from(self.p.get(SelectVpm))];
        let (g, mv) = (self.m.grit, self.m.movement);
        self.mac(ShapeVpm, v.shape.0, v.shape.1, t, 40);
        self.int(ShiftShapeVpm, v.shift.0, v.shift.1);
        self.mac(VpmParam1, 100, v.feedback, g, 4);
        self.mac(VpmParam2, 100, v.noise, g, 2);
        self.mac(VpmParam3, v.shape_mod.0, v.shape_mod.1, mv, 5);
        self.int(VpmParam4, v.attack.0, v.attack.1);
        self.int(VpmParam5, v.decay.0, v.decay.1);
        self.int(VpmParam6, v.track.0, v.track.1);
        self.int(MultiLevel, level.0, level.1);
    }

    /// The multi engine as noise: one of `types`, coloured within `shape`.
    /// The caller sets its level.
    fn noise(&mut self, types: &[u16], shape: (u16, u16)) {
        self.set(MultiType, NOISE);
        self.pick(SelectNoise, types);
        self.int(ShapeNoise, shape.0, shape.1);
    }

    /// Where the multi engine sits: one of `octaves`, and after the filter
    /// instead of before it with chance `post`.
    fn multi_at(&mut self, octaves: &[u16], post: f32) {
        self.pick(MultiOctave, octaves);
        self.set(MultiRouting, u16::from(self.chance(MultiRouting, post)));
    }

    /// The LFO locked to the tempo: one of `bands` (KORG's note values,
    /// longest first), shorter with movement.
    fn lfo_bpm(&mut self, bands: &[u16]) {
        let last = (bands.len() - 1) as f32;
        let i = (self.m.movement * last + self.roll(LfoRate, 4) - 0.5)
            .round()
            .clamp(0.0, last) as usize;
        self.set(LfoMode, LFO_BPM);
        self.set(LfoRate, bands[i] * 64 + 32);
    }

    /// The LFO as one cycle per note, an extra envelope.
    fn lfo_once(&mut self, waves: &[u16], rate: (u16, u16)) {
        self.set(LfoMode, LFO_ONE_SHOT);
        self.pick(LfoWave, waves);
        self.int(LfoRate, rate.0, rate.1);
    }

    /// The EG on the whole pitch: the note starts `int` away (above 512 is
    /// up; further with movement) and lands over `decay`. Drums and zaps.
    fn sweep(&mut self, decay: (u16, u16), int: (u16, u16)) {
        self.eg(TO_PITCH, (0, 0), decay);
        self.mac(EgInt, int.0, int.1, self.m.movement, 10);
    }

    /// A voice mode depth entry (chord or arp pattern) from `entries`.
    fn depth_entry(&mut self, mode: u16, entries: &[u16]) {
        self.set(VoiceModeType, mode);
        self.pick(VoiceModeDepth, entries);
        let i = self.p.get(VoiceModeDepth) as usize;
        if let Some(c) = depth_choices(mode) {
            self.set(VoiceModeDepth, depth_raw(i, c));
        }
    }
}

fn power_chord(s: &mut Make) {
    // Mostly the fifth; sometimes sus2 or sus4.
    let entries: &[u16] = if s.chance(VoiceModeType, 0.8) {
        &[0]
    } else {
        &[1, 4]
    };
    s.depth_entry(CHORD, entries);
    s.int(Portamento, 4, 18);
    s.oscillators(&[SAW], &[SAW, SAW, SQR]);
    s.vco2_octave(&[FT16, FT8]);
    s.grit(1, 300, true);
    s.bright((380, 900), (60, 380));
    s.amp((0, 15), (400, 700), (700, 950), (120, 320));
    s.eg(TO_CUTOFF, (0, 10), (250, 550));
    s.movement(TO_SHAPE, (0, 420), (540, 700), &[PHASER, FLANGER]);
    s.set(ModFxOn, u16::from(s.m.movement >= 0.6));
    s.space(
        -0.1,
        &[D_STEREO, D_TAPE, D_MONO],
        &[R_ROOM, R_PLATE, R_ARENA, R_HALL],
    );
}

fn fuzz_bass(s: &mut Make) {
    s.set(VoiceModeType, UNISON);
    s.int(VoiceModeDepth, 0, 60);
    s.set(Octave, 1);
    s.oscillators(&[SQR, SAW], &[SAW, SQR]);
    s.set(Vco1Octave, FT16);
    s.pick(Vco2Octave, &[FT16, FT8]);
    s.int(Vco1Shape, 0, 400);
    s.int(Vco1Level, 900, 1023);
    s.int(Vco2Level, 700, 950);
    s.grit(2, 450, true);
    s.bright((220, 720), (40, 360));
    s.amp((0, 8), (200, 450), (400, 700), (30, 110));
    s.eg(TO_CUTOFF, (0, 10), (150, 400));
    s.movement(TO_CUTOFF, (0, 300), (600, 820), &[]);
    s.space(-0.35, &[D_MONO, D_ONE_TAP], &[R_ROOM]);
}

fn scream_lead(s: &mut Make) {
    s.set(VoiceModeType, UNISON);
    s.int(VoiceModeDepth, 150, 450);
    s.pick(Octave, &[2, 3]);
    s.int(Portamento, 25, 90);
    s.oscillators(&[SAW], &[SAW, SQR]);
    s.pick(Vco2Octave, &[FT8, FT4]);
    s.int(Vco1Level, 900, 1023);
    s.grit(1, 400, false);
    s.bright((560, 1000), (220, 600));
    s.amp((0, 30), (500, 800), (820, 1000), (200, 420));
    s.eg(TO_CUTOFF, (0, 20), (300, 700));
    // Vibrato: the LFO on pitch, gently.
    s.movement(TO_PITCH, (20, 160), (560, 720), &[FLANGER, PHASER, CHORUS]);
    // Hard sync with the EG sweeping the second oscillator: the scream.
    if s.chance(EgTarget, 0.4 + s.m.grit * 0.5) {
        s.set(Sync, 1);
        s.int(Vco2Pitch, 600, 820);
        s.set(EgTarget, EG_PITCH2);
        s.mac(EgInt, 620, 820, s.m.movement, 30);
    }
    s.space(
        0.05,
        &[D_PING, D_STEREO, D_TAPE],
        &[R_HALL, R_PLATE, R_ARENA],
    );
}

fn riser(s: &mut Make) {
    s.set(VoiceModeType, POLY);
    s.oscillators(&[SAW], &[SAW]);
    s.int(Vco2Pitch, 548, 600);
    s.vco2_octave(&[FT8]);
    s.grit(0, 600, false);
    s.set(MultiType, NOISE);
    s.set(SelectNoise, 0);
    s.int(MultiLevel, 350, 700);
    s.bright((300, 800), (300, 650));
    s.amp((300, 700), (800, 800), (950, 1023), (500, 900));
    s.movement(TO_CUTOFF, (0, 500), (760, 1000), &[FLANGER, PHASER]);
    // The climb: the EG bends the whole pitch up, slowly.
    s.eg(TO_PITCH, (650, 1000), (900, 1023));
    s.space(0.3, &[D_PING, D_STEREO], &[R_RISER, R_SPACE, R_HALL]);
}

fn wobble(s: &mut Make) {
    s.set(VoiceModeType, UNISON);
    s.int(VoiceModeDepth, 0, 120);
    s.pick(Octave, &[1, 2]);
    s.oscillators(&[SAW, SQR], &[SAW, SQR]);
    s.pick(Vco1Octave, &[FT16, FT8]);
    s.pick(Vco2Octave, &[FT16, FT8]);
    s.grit(1, 300, true);
    s.bright((200, 650), (350, 700));
    s.amp((0, 10), (600, 600), (850, 1023), (60, 200));
    if s.chance(LfoTarget, 0.5) {
        // The pitch wobbles.
        s.movement(TO_PITCH, (360, 760), (512, 512), &[]);
        s.eg(TO_CUTOFF, (0, 10), (200, 500));
    } else {
        // The filter wobbles; the pitch drops (or jumps) into it.
        s.movement(TO_CUTOFF, (500, 1000), (512, 512), &[]);
        s.eg(TO_PITCH, (0, 0), (80, 250));
        let dive = if s.chance(EgInt, 0.7) { -1.0 } else { 1.0 };
        let depth = 180.0 + s.roll(EgInt, 5) * 150.0;
        s.put(EgInt, 512.0 + dive * depth);
    }
    // Tempo-locked: 1/2, 1/4, 1/8 or 1/16.
    s.lfo_bpm(&[4, 7, 10, 12]);
    s.space(-0.2, &[D_STEREO_BPM, D_MONO_BPM], &[R_ROOM, R_PLATE]);
}

fn pad(s: &mut Make) {
    s.set(VoiceModeType, POLY);
    s.int(VoiceModeDepth, 0, 200);
    s.oscillators(&[SAW, TRI, SQR], &[SAW, TRI, SQR]);
    s.int(Vco2Pitch, 538, 560);
    s.vco2_octave(&[FT8]);
    s.grit(0, 350, false);
    // Sometimes a digital layer under the analogue pair: fat or breathy.
    if s.chance(MultiType, 0.4) {
        s.vpm(&[8, 9, 10, 11, 0], s.m.bright * 0.5, (180, 420));
        s.multi_at(&[FT8, FT4], 0.2);
    }
    s.bright((280, 780), (0, 300));
    s.amp((350, 750), (600, 900), (820, 980), (450, 880));
    s.eg(TO_CUTOFF, (400, 800), (600, 1000));
    s.movement(
        TO_SHAPE,
        (150, 650),
        (540, 680),
        &[CHORUS, ENSEMBLE, PHASER],
    );
    s.set(ModFxOn, 1);
    s.mac(LfoRate, 60, 420, s.m.movement, 40);
    s.space(0.25, &[D_STEREO, D_TAPE], &[R_HALL, R_SMOOTH, R_SPACE]);
}

fn pluck(s: &mut Make) {
    s.set(VoiceModeType, POLY);
    s.oscillators(&[SAW, SQR, TRI], &[SAW, SQR]);
    s.vco2_octave(&[FT8]);
    s.grit(0, 300, false);
    // Sometimes a digital pick on top, its modulation falling away.
    if s.chance(MultiType, 0.3) {
        s.vpm(&[12, 13, 2, 3], s.m.bright, (250, 550));
        s.multi_at(&[FT8, FT4], 0.3);
    }
    s.bright((160, 520), (100, 480));
    s.amp((0, 0), (150, 420), (0, 120), (120, 320));
    s.eg(TO_CUTOFF, (0, 0), (100, 320));
    s.movement(TO_CUTOFF, (0, 200), (700, 950), &[CHORUS, PHASER]);
    s.space(
        0.0,
        &[D_PING, D_PING_BPM, D_STEREO],
        &[R_PLATE, R_ROOM, R_HALL],
    );
}

fn keys(s: &mut Make) {
    s.set(VoiceModeType, POLY);
    s.oscillators(&[SQR, TRI, SAW], &[TRI, SQR]);
    s.vco2_octave(&[FT8]);
    s.int(Vco2Level, 400, 750);
    s.grit(0, 200, false);
    // Sometimes an electric-piano tine from the VPM engine.
    if s.chance(MultiType, 0.35) {
        s.vpm(&[0, 1, 12, 13], s.m.bright * 0.6, (220, 480));
        s.multi_at(&[FT8, FT4], 0.3);
    }
    s.bright((380, 820), (0, 260));
    s.amp((0, 15), (450, 800), (300, 680), (220, 450));
    s.eg(TO_CUTOFF, (0, 10), (300, 600));
    s.movement(TO_SHAPE, (0, 400), (540, 680), &[CHORUS, PHASER, ENSEMBLE]);
    s.space(-0.05, &[D_STEREO, D_TAPE], &[R_ROOM, R_PLATE, R_HALL]);
}

fn drone(s: &mut Make) {
    s.pick(VoiceModeType, &[POLY, UNISON]);
    s.int(VoiceModeDepth, 0, 200);
    s.pick(Octave, &[0, 1]);
    s.oscillators(&[SAW, SQR], &[SAW, SQR, TRI]);
    s.pick(Vco1Octave, &[FT16, FT8]);
    s.grit(0, 700, false);
    s.vpm(&[0, 4, 8, 10, 14, 15], s.m.grit, (300, 650));
    s.multi_at(&[FT16, FT8], 0.0);
    s.bright((220, 650), (150, 600));
    s.amp((450, 900), (800, 800), (1023, 1023), (900, 1023));
    s.eg(TO_CUTOFF, (500, 1000), (800, 1023));
    s.movement(
        TO_CUTOFF,
        (150, 600),
        (520, 650),
        &[PHASER, FLANGER, ENSEMBLE],
    );
    s.pick(LfoTarget, &[TO_CUTOFF, TO_SHAPE]);
    s.mac(LfoRate, 0, 250, s.m.movement, 30);
    s.space(
        0.3,
        &[D_TAPE, D_STEREO],
        &[R_SPACE, R_SUBMARINE, R_HALL, R_HORROR],
    );
}

fn space_fx(s: &mut Make) {
    s.set(VoiceModeType, POLY);
    s.pick(Octave, &[2, 3]);
    s.oscillators(&[TRI, SAW], &[TRI, SAW, SQR]);
    s.grit(0, 800, false);
    s.bright((300, 900), (300, 700));
    s.amp((50, 300), (700, 700), (900, 1023), (600, 1000));
    s.eg(TO_CUTOFF, (0, 300), (400, 900));
    if s.chance(Vco1Level, 0.7) {
        // Sirens and lasers: the LFO bends the pitch hard.
        s.movement(TO_PITCH, (380, 950), (512, 700), &[FLANGER, PHASER]);
        s.mac(LfoRate, 200, 900, s.m.movement, 60);
        s.set(Ring, u16::from(s.chance(Ring, 0.5)));
        s.int(CrossModDepth, 300, 800);
    } else {
        // Alien air: noise alone, swept by the LFO.
        s.movement(TO_CUTOFF, (300, 900), (512, 700), &[FLANGER, PHASER]);
        s.set(Vco1Level, 0);
        s.set(Vco2Level, 0);
        s.set(MultiType, NOISE);
        s.pick(SelectNoise, &[0, 1, 2, 3]);
        s.int(ShapeNoise, 200, 900);
        s.int(MultiLevel, 700, 1000);
    }
    s.space(
        0.35,
        &[D_PING, D_STEREO, D_TAPE],
        &[R_SPACE, R_SUBMARINE, R_HORROR, R_RISER],
    );
}

fn arp_pulse(s: &mut Make) {
    // Rise, fall, rise-fall, poly and random patterns.
    s.depth_entry(ARP, &[2, 3, 4, 6, 8, 10]);
    s.pick(ArpRate, &[4, 6, 7]);
    s.int(ArpGateTime, 18, 50);
    s.oscillators(&[SAW, SQR], &[SAW, SQR]);
    s.vco2_octave(&[FT8]);
    s.grit(0, 300, false);
    s.bright((250, 700), (150, 520));
    s.amp((0, 10), (200, 420), (0, 250), (100, 250));
    s.eg(TO_CUTOFF, (0, 0), (150, 380));
    s.movement(TO_CUTOFF, (0, 350), (620, 900), &[PHASER, FLANGER]);
    s.space(
        0.1,
        &[D_STEREO_BPM, D_PING_BPM, D_TAPE_BPM],
        &[R_PLATE, R_ROOM],
    );
}

fn bell(s: &mut Make) {
    s.set(VoiceModeType, POLY);
    s.pick(Octave, &[2, 3]);
    s.oscillators(&[TRI], &[TRI, SQR]);
    s.pick(Vco1Octave, &[FT8, FT4]);
    s.int(Vco2Pitch, 700, 980);
    s.grit(0, 0, false);
    // Metal: ring modulation, or plenty of cross mod.
    if s.chance(Vco2Wave, 0.6) {
        s.set(Ring, 1);
    } else {
        s.int(CrossModDepth, 380, 750);
    }
    s.vpm(&[0, 1, 2, 3, 12, 13], s.m.bright, (300, 600));
    s.multi_at(&[FT8, FT4, FT2], 0.3);
    s.bright((450, 950), (0, 300));
    s.amp((0, 0), (780, 1000), (0, 180), (500, 850));
    s.eg(TO_CUTOFF, (0, 10), (600, 900));
    s.movement(TO_SHAPE, (0, 300), (560, 700), &[CHORUS, ENSEMBLE]);
    s.space(0.15, &[D_STEREO, D_PING], &[R_PLATE, R_HALL, R_SMOOTH]);
}

fn brass(s: &mut Make) {
    s.set(VoiceModeType, POLY);
    // A little glide into each note, as lips do.
    s.int(Portamento, 3, 12);
    s.oscillators(&[SAW], &[SAW, SAW, SQR]);
    s.int(Vco2Pitch, 534, 548);
    s.pick(Vco2Octave, &[FT8, FT8, FT16]);
    s.grit(0, 150, false);
    s.bright((230, 640), (0, 260));
    s.int(CutoffVelocity, 50, 110);
    s.amp((20, 110), (500, 800), (700, 900), (150, 340));
    // The blat: the filter opens just after the note starts.
    s.eg(TO_CUTOFF, (60, 220), (350, 650));
    // Vibrato, never a siren.
    s.movement(TO_PITCH, (0, 130), (680, 880), &[ENSEMBLE, CHORUS]);
    s.set(LfoWave, TRI);
    s.space(0.0, &[D_STEREO, D_TAPE], &[R_HALL, R_ARENA, R_ROOM]);
}

fn strings(s: &mut Make) {
    s.set(VoiceModeType, POLY);
    s.oscillators(&[SAW], &[SAW, SAW, SQR]);
    s.int(Vco2Pitch, 538, 562);
    s.vco2_octave(&[FT8]);
    s.grit(0, 120, false);
    s.bright((400, 900), (0, 200));
    s.amp((320, 600), (600, 900), (820, 980), (400, 800));
    s.eg(TO_CUTOFF, (300, 700), (500, 900));
    // Bowing: the shapes drift slowly, and the ensemble is always on.
    s.movement(
        TO_SHAPE,
        (120, 520),
        (530, 620),
        &[ENSEMBLE, ENSEMBLE, CHORUS],
    );
    s.set(ModFxOn, 1);
    s.set(LfoWave, TRI);
    s.mac(LfoRate, 80, 380, s.m.movement, 40);
    s.space(0.2, &[D_STEREO, D_TAPE], &[R_HALL, R_SMOOTH, R_ARENA]);
}

fn organ(s: &mut Make) {
    s.set(VoiceModeType, POLY);
    // Drawbars: a fundamental, a stop one or two octaves above it in
    // tune, and a sine from the VPM engine below or between.
    s.oscillators(&[SQR, TRI], &[TRI, SQR]);
    s.set(Vco1Octave, FT8);
    s.pick(Vco2Octave, &[FT4, FT4, FT2]);
    s.int(Vco2Pitch, 505, 519);
    s.int(Vco1Shape, 0, 120);
    s.int(Vco2Shape, 0, 120);
    s.int(Vco2Level, 350, 800);
    s.grit(0, 0, false);
    s.vpm(&[0], 0.0, (250, 600));
    s.multi_at(&[FT16, FT8, FT4], 0.0);
    s.bright((480, 960), (0, 120));
    // No touch response, and gone when the key is.
    s.set(CutoffKeyTrack, 2);
    s.int(AmpVelocity, 0, 20);
    s.int(CutoffVelocity, 0, 15);
    s.amp((0, 6), (600, 600), (1023, 1023), (15, 90));
    // Key click.
    s.eg(TO_CUTOFF, (0, 0), (60, 200));
    // A rotary speaker's shimmer: fast, shallow vibrato and a mod effect.
    s.movement(TO_PITCH, (0, 90), (512, 600), &[CHORUS, PHASER, ENSEMBLE]);
    s.set(ModFxOn, u16::from(s.m.movement >= 0.3));
    s.set(LfoWave, TRI);
    s.mac(LfoRate, 600, 820, s.m.movement, 30);
    s.space(-0.1, &[D_STEREO, D_TAPE], &[R_ROOM, R_HALL, R_PLATE]);
}

fn sub_bass(s: &mut Make) {
    s.set(VoiceModeType, UNISON);
    s.int(VoiceModeDepth, 0, 25);
    s.set(Octave, 1);
    s.int(Portamento, 0, 25);
    // Close to a sine; a quiet triangle an octave up gives it a pitch on
    // small speakers.
    s.oscillators(&[TRI, TRI, SQR], &[TRI]);
    s.pick(Vco1Octave, &[FT16, FT8]);
    s.set(Vco2Octave, FT8);
    s.int(Vco2Pitch, 506, 518);
    s.int(Vco1Shape, 0, 80);
    s.int(Vco1Level, 950, 1023);
    s.int(Vco2Level, 0, 320);
    s.grit(0, 0, false);
    s.bright((140, 470), (0, 160));
    s.amp((0, 12), (400, 700), (800, 1023), (40, 160));
    s.eg(TO_CUTOFF, (0, 10), (150, 400));
    s.movement(TO_CUTOFF, (0, 120), (512, 640), &[]);
    s.space(-0.5, &[D_MONO], &[R_ROOM]);
}

fn acid(s: &mut Make) {
    s.set(VoiceModeType, UNISON);
    s.set(VoiceModeDepth, 0);
    s.set(Octave, 1);
    // One oscillator, sliding between notes.
    s.int(Portamento, 18, 60);
    s.oscillators(&[SAW, SQR], &[SAW]);
    s.set(Vco1Octave, FT8);
    s.int(Vco1Shape, 0, 300);
    s.int(Vco1Level, 900, 1023);
    s.set(Vco2Level, 0);
    s.grit(0, 0, false);
    s.set(Ring, 0);
    s.set(Sync, 0);
    // The squelch: resonance near the edge, and the EG snaps the filter
    // open on every note, harder on a hard one.
    s.bright((110, 520), (520, 840));
    s.pick(CutoffKeyTrack, &[0, 1]);
    s.int(CutoffVelocity, 70, 127);
    s.amp((0, 4), (300, 520), (650, 1000), (25, 100));
    s.eg(TO_CUTOFF, (0, 0), (110, 380));
    s.movement(TO_CUTOFF, (0, 140), (700, 960), &[]);
    s.space(
        -0.15,
        &[D_MONO_BPM, D_STEREO_BPM, D_PING_BPM],
        &[R_ROOM, R_PLATE],
    );
}

fn sync_lead(s: &mut Make) {
    s.set(VoiceModeType, UNISON);
    s.int(VoiceModeDepth, 80, 360);
    s.pick(Octave, &[2, 3]);
    s.int(Portamento, 12, 60);
    s.oscillators(&[SAW, SQR], &[SAW, SAW, SQR]);
    s.pick(Vco2Octave, &[FT8, FT4]);
    s.grit(1, 0, false);
    // Hard sync, the second oscillator tuned well above the first, and
    // the EG pulling it through its harmonics.
    s.set(Sync, 1);
    s.set(Ring, 0);
    s.int(Vco2Pitch, 620, 960);
    s.bright((520, 1000), (80, 420));
    s.amp((0, 25), (500, 800), (800, 1000), (150, 380));
    s.eg(EG_PITCH2, (0, 160), (350, 820));
    s.movement(TO_PITCH, (20, 140), (650, 900), &[FLANGER, PHASER, CHORUS]);
    s.set(LfoWave, TRI);
    // Now and then the sweep runs the other way.
    if s.chance(EgInt, 0.25) {
        s.set(EgInt, 1024 - s.p.get(EgInt));
    }
    s.space(
        0.05,
        &[D_PING, D_STEREO, D_TAPE],
        &[R_HALL, R_PLATE, R_ARENA],
    );
}

fn fm_glass(s: &mut Make) {
    s.set(VoiceModeType, POLY);
    s.pick(Octave, &[2, 2, 3]);
    // The VPM engine leads; a soft triangle may sit under it.
    s.oscillators(&[TRI], &[TRI]);
    s.int(Vco1Level, 0, 360);
    s.set(Vco2Level, 0);
    s.grit(0, 0, false);
    s.vpm(&[0, 1, 2, 3, 12, 13], s.m.bright, (820, 1000));
    s.multi_at(&[FT8, FT8, FT4], 0.3);
    s.bright((520, 1000), (0, 200));
    // Struck, then ringing down: longer than a pluck, shorter than a bell.
    s.amp((0, 0), (520, 750), (0, 200), (350, 700));
    s.eg(TO_CUTOFF, (0, 0), (300, 600));
    s.movement(TO_SHAPE, (0, 260), (540, 680), &[CHORUS, ENSEMBLE, PHASER]);
    s.space(0.12, &[D_STEREO, D_PING], &[R_PLATE, R_HALL, R_SMOOTH]);
}

fn fm_bass(s: &mut Make) {
    s.pick(VoiceModeType, &[UNISON, POLY]);
    s.int(VoiceModeDepth, 0, 40);
    s.set(Octave, 1);
    // A triangle sub under the VPM engine.
    s.oscillators(&[TRI, SQR], &[TRI]);
    s.set(Vco1Octave, FT16);
    s.int(Vco1Level, 0, 420);
    s.set(Vco2Level, 0);
    s.grit(0, 0, false);
    let depth = s.m.bright * 0.6 + s.m.grit * 0.4;
    s.vpm(&[0, 1, 4, 5, 6, 7, 8, 9], depth, (850, 1023));
    // The knock: the modulation envelope hits hard and is gone at once.
    s.mac(VpmParam3, 125, 185, s.m.movement, 6);
    s.int(VpmParam5, 45, 95);
    s.multi_at(&[FT16, FT8, FT8], 0.25);
    s.bright((330, 880), (0, 240));
    s.amp((0, 6), (280, 600), (300, 800), (40, 150));
    s.eg(TO_CUTOFF, (0, 0), (150, 400));
    s.movement(TO_CUTOFF, (0, 160), (560, 780), &[]);
    s.space(-0.3, &[D_MONO, D_ONE_TAP, D_MONO_BPM], &[R_ROOM, R_PLATE]);
}

fn chiptune(s: &mut Make) {
    s.set(VoiceModeType, UNISON);
    s.set(VoiceModeDepth, 0);
    s.pick(Octave, &[2, 3]);
    // Two pulse channels, from square to thin.
    s.oscillators(&[SQR], &[SQR]);
    s.pick(Vco1Shape, &[0, 380, 700, 900]);
    s.pick(Vco2Shape, &[0, 380, 700, 900]);
    // The second channel: an octave up, a fifth up, or resting.
    match (s.roll(Vco2Level, 6) * 3.0) as u8 {
        0 => {
            s.set(Vco2Octave, FT4);
            s.set(Vco2Pitch, 512);
        }
        1 => {
            s.set(Vco2Octave, FT8);
            s.set(Vco2Pitch, 834);
        }
        _ => s.set(Vco2Level, 0),
    }
    s.grit(0, 0, false);
    // The noise channel, crushed.
    s.noise(&[N_DECIM], (300, 900));
    s.mac(MultiLevel, 0, 420, (s.m.grit - 0.3) / 0.7, 20);
    s.bright((720, 1023), (0, 150));
    s.set(CutoffKeyTrack, 2);
    s.int(AmpVelocity, 0, 30);
    // On and off like a gate.
    s.amp((0, 0), (300, 620), (650, 1023), (0, 70));
    // A blip of pitch on each note; the LFO sweeps the pulse width,
    // trills, or plays one grace note.
    s.eg(TO_PITCH, (0, 0), (30, 120));
    match (s.roll(LfoTarget, 6) * 3.0) as u8 {
        0 => {
            s.movement(TO_SHAPE, (200, 800), (512, 620), &[]);
            s.set(LfoWave, TRI);
        }
        1 => {
            s.movement(TO_PITCH, (150, 275), (512, 620), &[]);
            s.set(LfoWave, SQR);
            s.mac(LfoRate, 500, 850, s.m.movement, 40);
        }
        _ => {
            s.movement(TO_PITCH, (180, 275), (512, 620), &[]);
            s.lfo_once(&[SQR], (300, 700));
        }
    }
    s.space(
        -0.2,
        &[D_MONO_BPM, D_PING_BPM, D_STEREO_BPM],
        &[R_ROOM, R_PLATE],
    );
}

fn kick(s: &mut Make) {
    s.set(VoiceModeType, UNISON);
    s.int(VoiceModeDepth, 0, 30);
    s.set(Octave, 1);
    // One round oscillator; the keys do not move the filter.
    s.oscillators(&[TRI, TRI, SQR], &[TRI]);
    s.pick(Vco1Octave, &[FT16, FT8]);
    s.int(Vco1Shape, 0, 60);
    s.int(Vco1Level, 950, 1023);
    s.set(Vco2Level, 0);
    s.set(CutoffKeyTrack, 0);
    s.int(AmpVelocity, 70, 127);
    s.grit(0, 0, false);
    // The beater: a touch of noise, more with grit.
    s.noise(&[N_LOW, N_HIGH, N_DECIM], (200, 700));
    s.mac(MultiLevel, 0, 260, s.m.grit, 20);
    s.bright((230, 700), (0, 200));
    s.amp((0, 0), (190, 470), (0, 0), (120, 400));
    s.movement(TO_CUTOFF, (0, 200), (512, 512), &[]);
    s.lfo_once(&[SAW, TRI], (300, 800));
    // The thump: the pitch falls fast into the note.
    s.sweep((60, 210), (760, 900));
    s.space(-0.3, &[D_MONO, D_ONE_TAP], &[R_ROOM, R_PLATE, R_ARENA]);
}

fn snare(s: &mut Make) {
    s.set(VoiceModeType, POLY);
    // The shell: two oscillators out of tune with each other, as a
    // drum's modes are.
    s.oscillators(&[TRI, SQR], &[TRI]);
    s.set(Vco1Octave, FT8);
    s.int(Vco2Pitch, 700, 900);
    s.int(Vco1Level, 380, 700);
    s.int(Vco2Level, 0, 450);
    s.set(CutoffKeyTrack, 0);
    s.int(AmpVelocity, 70, 127);
    s.grit(0, 0, false);
    // The wires: noise, louder than the shell, before or after the filter.
    s.noise(&[N_HIGH, N_PEAK, N_HIGH, N_DECIM], (300, 900));
    s.int(MultiLevel, 720, 1023);
    s.multi_at(&[FT8], 0.5);
    s.bright((380, 900), (0, 260));
    s.amp((0, 0), (140, 400), (0, 0), (110, 380));
    s.movement(TO_CUTOFF, (0, 180), (512, 512), &[]);
    s.sweep((40, 150), (690, 800));
    s.space(-0.1, &[D_MONO, D_ONE_TAP], &[R_ROOM, R_PLATE, R_ARENA]);
}

fn laser(s: &mut Make) {
    s.set(VoiceModeType, POLY);
    s.pick(Octave, &[2, 3]);
    s.oscillators(&[SAW, SQR], &[SAW, SQR, TRI]);
    s.grit(0, 500, false);
    s.bright((380, 950), (250, 700));
    s.amp((0, 0), (220, 600), (0, 0), (200, 600));
    // A fast LFO makes the beam shimmer.
    s.movement(TO_CUTOFF, (0, 600), (512, 512), &[FLANGER, PHASER]);
    s.mac(LfoRate, 450, 1000, s.m.movement, 60);
    // The zap: the pitch falls a long way; now and then it rises.
    s.sweep((120, 520), (840, 1013));
    if s.chance(EgInt, 0.25) {
        s.set(EgInt, 1024 - s.p.get(EgInt));
    }
    s.space(
        0.2,
        &[D_PING, D_STEREO, D_PING_BPM],
        &[R_SPACE, R_HALL, R_PLATE],
    );
}

fn siren(s: &mut Make) {
    s.set(VoiceModeType, UNISON);
    s.int(VoiceModeDepth, 0, 80);
    s.pick(Octave, &[2, 3]);
    s.oscillators(&[SAW, SQR], &[SAW, SQR]);
    s.int(Vco2Level, 0, 500);
    s.grit(0, 250, false);
    s.bright((450, 950), (100, 500));
    s.amp((0, 80), (700, 700), (950, 1023), (150, 600));
    s.eg(TO_CUTOFF, (0, 100), (400, 800));
    // The wail: a slow LFO carries the pitch up and down, restarting
    // with each key.
    s.movement(TO_PITCH, (420, 820), (512, 600), &[PHASER, FLANGER]);
    s.pick(LfoWave, &[TRI, TRI, SAW]);
    s.mac(LfoRate, 60, 420, s.m.movement, 40);
    s.set(LfoKeySync, 1);
    s.space(
        0.15,
        &[D_STEREO, D_TAPE, D_PING],
        &[R_HALL, R_ARENA, R_SPACE],
    );
}

fn wind(s: &mut Make) {
    s.set(VoiceModeType, POLY);
    // Noise alone.
    s.set(Vco1Level, 0);
    s.set(Vco2Level, 0);
    s.grit(0, 0, false);
    s.noise(&[N_HIGH, N_LOW, N_PEAK, N_PEAK], (150, 900));
    s.int(MultiLevel, 750, 1023);
    // The whistle: resonance sings where the cutoff sits, and the keys
    // move it.
    s.bright((260, 760), (350, 760));
    s.set(CutoffKeyTrack, 2);
    s.int(CutoffVelocity, 0, 50);
    s.int(AmpVelocity, 30, 90);
    s.amp((280, 700), (700, 700), (850, 1023), (500, 950));
    s.eg(TO_CUTOFF, (300, 800), (600, 1000));
    // Gusts.
    s.movement(TO_CUTOFF, (220, 720), (512, 640), &[FLANGER, PHASER]);
    s.set(LfoWave, TRI);
    s.mac(LfoRate, 20, 300, s.m.movement, 30);
    s.space(
        0.3,
        &[D_TAPE, D_STEREO],
        &[R_SPACE, R_HALL, R_SUBMARINE, R_HORROR],
    );
}

fn impact(s: &mut Make) {
    s.set(VoiceModeType, UNISON);
    s.int(VoiceModeDepth, 100, 500);
    s.pick(Octave, &[0, 1]);
    s.oscillators(&[SQR, SAW, TRI], &[SAW, SQR]);
    s.pick(Vco1Octave, &[FT16, FT8]);
    s.pick(Vco2Octave, &[FT16, FT8]);
    s.grit(1, 600, false);
    // Debris.
    s.noise(&[N_LOW, N_DECIM, N_HIGH], (100, 700));
    s.int(MultiLevel, 450, 900);
    s.bright((200, 700), (0, 300));
    s.amp((0, 0), (600, 950), (0, 0), (600, 950));
    // The filter closes once over the hit.
    s.movement(TO_CUTOFF, (0, 400), (512, 512), &[FLANGER, PHASER]);
    s.lfo_once(&[SAW, TRI], (60, 300));
    // The boom: a slow fall of the pitch.
    s.sweep((250, 750), (690, 860));
    s.space(
        0.45,
        &[D_TAPE, D_STEREO],
        &[R_ARENA, R_HALL, R_SPACE, R_SUBMARINE],
    );
}

fn alarm(s: &mut Make) {
    s.set(VoiceModeType, UNISON);
    s.int(VoiceModeDepth, 0, 60);
    s.pick(Octave, &[2, 3]);
    s.oscillators(&[SQR, SQR, SAW], &[SQR, SAW]);
    s.int(Vco2Level, 0, 450);
    s.grit(0, 300, false);
    s.bright((550, 1000), (0, 350));
    s.amp((0, 0), (700, 700), (1023, 1023), (20, 160));
    s.eg(TO_CUTOFF, (0, 0), (200, 500));
    // Two tones: a square LFO switches the pitch, sometimes in time
    // with the tempo (1/4, 1/8 or 1/16).
    s.movement(TO_PITCH, (340, 700), (512, 560), &[]);
    s.set(LfoWave, SQR);
    s.set(LfoKeySync, 1);
    if s.chance(LfoMode, 0.4) {
        s.lfo_bpm(&[7, 10, 12]);
    } else {
        s.mac(LfoRate, 350, 850, s.m.movement, 50);
    }
    s.space(
        -0.05,
        &[D_MONO, D_STEREO, D_PING],
        &[R_ROOM, R_PLATE, R_ARENA],
    );
}

/// Make a new program from `recipe` (its `keep` is ignored: see
/// [`generate_from`]).
pub fn generate(recipe: &Recipe) -> Program {
    let mut s = Make {
        p: Program::init(),
        seed: recipe.seed ^ (recipe.character as u64).wrapping_mul(0xA24B_AED4_963E_E407),
        m: recipe.macros,
    };
    match recipe.character {
        Character::PowerChord => power_chord(&mut s),
        Character::FuzzBass => fuzz_bass(&mut s),
        Character::ScreamLead => scream_lead(&mut s),
        Character::Riser => riser(&mut s),
        Character::Wobble => wobble(&mut s),
        Character::Pad => pad(&mut s),
        Character::Pluck => pluck(&mut s),
        Character::Keys => keys(&mut s),
        Character::Drone => drone(&mut s),
        Character::SpaceFx => space_fx(&mut s),
        Character::ArpPulse => arp_pulse(&mut s),
        Character::Bell => bell(&mut s),
        Character::Brass => brass(&mut s),
        Character::Strings => strings(&mut s),
        Character::Organ => organ(&mut s),
        Character::SubBass => sub_bass(&mut s),
        Character::Acid => acid(&mut s),
        Character::SyncLead => sync_lead(&mut s),
        Character::FmGlass => fm_glass(&mut s),
        Character::FmBass => fm_bass(&mut s),
        Character::Chiptune => chiptune(&mut s),
        Character::Kick => kick(&mut s),
        Character::Snare => snare(&mut s),
        Character::Laser => laser(&mut s),
        Character::Siren => siren(&mut s),
        Character::Wind => wind(&mut s),
        Character::Impact => impact(&mut s),
        Character::Alarm => alarm(&mut s),
    }
    let mut p = s.p;
    cohere(&mut p);
    p.set_name(&name(recipe.seed, recipe.character))
        .expect("generated names are valid");
    p
}

/// Like [`generate`], with the fields of `recipe.keep`'s stages copied
/// from `base` as they are: "a new filter on my sound".
pub fn generate_from(base: &Program, recipe: &Recipe) -> Program {
    let mut p = generate(recipe);
    for stage in &recipe.keep {
        for &f in stage.sections().iter().flat_map(|s| s.fields) {
            p.set(f, base.get(f))
                .expect("a program's own value is in range");
        }
    }
    p
}

/// Make sure the program sounds and stays within what the synth's own
/// panel offers (see the module docs). Idempotent.
fn cohere(p: &mut Program) {
    let put = |p: &mut Program, f: Field, v: u16| p.set(f, v).expect("legal value");
    if [Vco1Level, Vco2Level, MultiLevel]
        .iter()
        .all(|&f| p.get(f) < 100)
    {
        put(p, Vco1Level, 800);
    }
    if p.get(AmpEgSustain) < 64 && p.get(AmpEgDecay) < 80 {
        put(p, AmpEgDecay, 80);
    }
    let cap = match p.get(CutoffDrive) {
        2 => 700,
        1 => 850,
        _ => 1023,
    };
    if p.get(Resonance) > cap {
        put(p, Resonance, cap);
    }
    if let Some(c) = depth_choices(p.get(VoiceModeType)) {
        let i = present::depth_index(p.get(VoiceModeDepth), c);
        put(p, VoiceModeDepth, depth_raw(i, c));
    }
    if p.get(MultiType) == 2 {
        put(p, MultiType, NOISE);
    }
    if p.get(ModFxType) == 5 {
        put(p, ModFxType, CHORUS);
    }
    if p.get(DelaySubType) > LAST_DELAY {
        put(p, DelaySubType, D_STEREO);
    }
    if p.get(ReverbSubType) > LAST_REVERB {
        put(p, ReverbSubType, R_HALL);
    }
    if p.get(ProgramLevel) > 102 {
        put(p, ProgramLevel, 102);
    }
}

/// `p` after the coherence pass.
pub fn cohered(mut p: Program) -> Program {
    cohere(&mut p);
    p
}

/// Continuous fields [`vary`] moves, with how far (a share of `amount`):
/// the ones that shape the sound; pitch barely.
const MOVES: &[(Field, f32)] = &[
    (Vco1Shape, 1.0),
    (Vco2Shape, 1.0),
    (Vco2Pitch, 0.08),
    (CrossModDepth, 0.5),
    (ShapeNoise, 0.8),
    (ShiftShapeNoise, 0.5),
    (ShapeVpm, 0.8),
    (ShiftShapeVpm, 0.5),
    (Vco1Level, 0.4),
    (Vco2Level, 0.5),
    (MultiLevel, 0.4),
    (Cutoff, 1.0),
    (Resonance, 0.7),
    (AmpEgAttack, 0.3),
    (AmpEgDecay, 0.5),
    (AmpEgSustain, 0.4),
    (AmpEgRelease, 0.5),
    (EgAttack, 0.5),
    (EgDecay, 0.6),
    (EgInt, 0.4),
    (LfoRate, 0.7),
    (LfoInt, 0.5),
    (ModFxTime, 0.7),
    (ModFxDepth, 0.7),
    (DelayTime, 0.6),
    (DelayDepth, 0.6),
    (DelayDryWet, 0.5),
    (ReverbTime, 0.6),
    (ReverbDepth, 0.6),
    (ReverbDryWet, 0.5),
    (VoiceModeDepth, 0.3),
    (VpmParam1, 0.3),
    (VpmParam2, 0.2),
    (VpmParam3, 0.4),
    (VpmParam4, 0.3),
    (VpmParam5, 0.4),
    (VpmParam6, 0.3),
    (Portamento, 0.1),
    (CutoffVelocity, 0.3),
    (AmpVelocity, 0.3),
];

/// Selectors [`vary`] may flip, with their built-in values.
const FLIPS: &[(Field, u16, u16)] = &[
    (Vco1Wave, 0, 2),
    (Vco2Wave, 0, 2),
    (Vco2Octave, 0, 3),
    (Sync, 0, 1),
    (Ring, 0, 1),
    (CutoffDrive, 0, 2),
    (LfoWave, 0, 2),
    (LfoTarget, 0, 2),
    (ModFxType, 1, 4),
    (DelaySubType, 0, LAST_DELAY),
    (ReverbSubType, 0, LAST_REVERB),
    (SelectNoise, 0, 3),
    (SelectVpm, 0, 15),
    (MultiOctave, 0, 3),
    (MultiRouting, 0, 1),
    (CutoffKeyTrack, 0, 2),
    (LfoMode, 0, 2),
    (LfoKeySync, 0, 1),
];

/// A mutation of `base`: continuous fields move by up to `amount` of
/// their range, and above half one selector may flip. `0.0` changes
/// nothing; the name stays.
pub fn vary(base: &Program, amount: f32, seed: u64) -> Program {
    let amount = amount.clamp(0.0, 1.0);
    let mut r = Rng::new(seed);
    let mut p = base.clone();
    for &(f, weight) in MOVES {
        let d = (r.unit() * 2.0 - 1.0) * amount * weight;
        // Arp patterns and chords are choices, not amounts.
        if f == VoiceModeDepth && depth_choices(p.get(VoiceModeType)).is_some() {
            continue;
        }
        let s = f.spec();
        let span = f32::from(s.max - s.min);
        let v = (f32::from(p.get(f)) + d * span).round();
        p.set(f, v.clamp(f32::from(s.min), f32::from(s.max)) as u16)
            .expect("clamped");
    }
    let (flip, which, to) = (r.unit(), r.below(FLIPS.len()), r.unit());
    if flip < (amount - 0.5) * 2.0 {
        let (f, lo, hi) = FLIPS[which];
        let others = hi - lo; // every value but the current one
        let k = ((to * f32::from(others)) as u16).min(others - 1);
        let v = lo + k;
        let v = if v >= p.get(f) { v + 1 } else { v };
        p.set(f, v.min(hi)).expect("a built-in value");
        // Another VPM type takes another range of modulation.
        if f == SelectVpm {
            let safe = &VPM_SAFE[usize::from(p.get(SelectVpm))];
            for (f, (lo, hi)) in [(ShapeVpm, safe.shape), (ShiftShapeVpm, safe.shift)] {
                p.set(f, p.get(f).clamp(lo, hi)).expect("within the range");
            }
        }
    }
    cohere(&mut p);
    p
}

/// A child of `a` and `b`: continuous fields interpolate by `mix` (0 is
/// `a`, 1 is `b`), and each panel section takes its selectors from one
/// parent (`b` with probability `mix`). The name is `a`'s.
pub fn breed(a: &Program, b: &Program, mix: f32, seed: u64) -> Program {
    let mix = mix.clamp(0.0, 1.0);
    let mut r = Rng::new(seed);
    let mut p = a.clone();
    for stage in Stage::ALL {
        for section in stage.sections() {
            let from = if r.unit() < mix { b } else { a };
            for &f in section.fields {
                let dial = matches!(present::control(f), Control::Dial);
                let chosen =
                    f == VoiceModeDepth && depth_choices(from.get(VoiceModeType)).is_some();
                let v = if dial && !chosen {
                    let (x, y) = (f32::from(a.get(f)), f32::from(b.get(f)));
                    (x + (y - x) * mix).round() as u16
                } else {
                    from.get(f)
                };
                p.set(f, v).expect("between two legal values");
            }
        }
    }
    cohere(&mut p);
    p
}

/// A name for a `character` sound: two words from its lists, at most 12
/// characters.
pub fn name(seed: u64, character: Character) -> String {
    let (first, second) = character.words();
    let mut r = Rng::new(seed ^ 0x6E61_6D65);
    for _ in 0..16 {
        let (a, b) = (first[r.below(first.len())], second[r.below(second.len())]);
        if a != b && a.len() + 1 + b.len() <= 12 {
            return format!("{a} {b}");
        }
    }
    second[r.below(second.len())].to_owned()
}

/// Deal [`HAND`] different programs with different names: `make` is
/// called with a fresh seed until each one differs from those before it.
pub fn hand(character: Character, seed: u64, mut make: impl FnMut(u64) -> Program) -> Vec<Program> {
    let mut r = Rng::new(seed);
    let mut out: Vec<Program> = Vec::with_capacity(HAND);
    let differs = |out: &[Program], p: &Program| {
        out.iter()
            .all(|q| Field::ALL.iter().any(|&f| q.get(f) != p.get(f)))
    };
    while out.len() < HAND {
        let mut s = r.next_u64();
        let mut p = make(s);
        // A maker that keeps making the same sound gets nudged apart.
        let mut tries = 0;
        while !differs(&out, &p) {
            s = r.next_u64();
            p = if tries < 8 { make(s) } else { vary(&p, 0.2, s) };
            tries += 1;
        }
        let n = (0..64)
            .map(|k| name(s.wrapping_add(k), character))
            .find(|n| out.iter().all(|q| q.name() != *n))
            .unwrap_or_else(|| format!("{} {}", character.words().1[0], out.len() + 1));
        p.set_name(&n).expect("generated names are valid");
        out.push(p);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Field;
    use crate::present::{self, category};

    fn recipe(character: Character, seed: u64) -> Recipe {
        Recipe {
            character,
            macros: Macros::default(),
            seed,
            keep: Vec::new(),
        }
    }

    /// Macros spread over their whole range, from the seed.
    fn spread(seed: u64) -> Macros {
        let u = |k: u64| {
            ((seed.wrapping_mul(2_654_435_761).rotate_left(k as u32 * 13) >> 40) % 101) as f32
                / 100.0
        };
        Macros {
            bright: u(1),
            movement: u(2),
            grit: u(3),
            space: u(4),
        }
    }

    /// The rules every generated, varied or bred program follows.
    fn check(p: &Program) {
        use Field::*;
        for &f in Field::ALL {
            let s = f.spec();
            assert!(
                (s.min..=s.max).contains(&p.get(f)),
                "{} = {}",
                s.name,
                p.get(f)
            );
        }
        assert!(
            [Vco1Level, Vco2Level, MultiLevel]
                .iter()
                .any(|&f| p.get(f) >= 100),
            "something sounds"
        );
        assert!(
            p.get(AmpEgSustain) >= 64 || p.get(AmpEgDecay) >= 80,
            "the note is heard"
        );
        match p.get(CutoffDrive) {
            2 => assert!(p.get(Resonance) <= 700, "resonance tamed under full drive"),
            1 => assert!(p.get(Resonance) <= 850),
            _ => {}
        }
        if let Some(c) = present::depth_choices(p.get(VoiceModeType)) {
            let d = p.get(VoiceModeDepth);
            assert_eq!(
                d,
                present::depth_raw(present::depth_index(d, c), c),
                "depth mid-band"
            );
        }
        assert_ne!(p.get(MultiType), 2, "no user oscillator");
        assert_ne!(p.get(ModFxType), 5, "no user mod effect");
        assert!(p.get(DelaySubType) < 12, "no user delay");
        assert!(p.get(ReverbSubType) < 10, "no user reverb");
        assert!(p.get(ProgramLevel) <= 102, "never louder than 0 dB");
        let n = p.name();
        assert!(!n.is_empty() && n.len() <= 12 && n.is_ascii(), "{n:?}");
    }

    /// Sum over the typed fields of how far they moved, as a share of
    /// their range.
    fn distance(a: &Program, b: &Program) -> f32 {
        Field::ALL
            .iter()
            .map(|&f| {
                let s = f.spec();
                f32::from(a.get(f).abs_diff(b.get(f))) / f32::from(s.max - s.min)
            })
            .sum()
    }

    fn same_sound(a: &Program, b: &Program) -> bool {
        Field::ALL.iter().all(|&f| a.get(f) == b.get(f))
    }

    #[test]
    fn every_program_is_valid_and_coherent() {
        for c in Character::ALL {
            for seed in 0..2000 {
                let r = Recipe {
                    macros: spread(seed),
                    ..recipe(c, seed)
                };
                let g = generate(&r);
                check(&g);
                check(&vary(&g, (seed % 11) as f32 / 10.0, seed));
                let other = generate(&recipe(
                    Character::ALL[seed as usize % Character::ALL.len()],
                    seed + 7,
                ));
                check(&breed(&g, &other, (seed % 5) as f32 / 4.0, seed));
            }
        }
    }

    #[test]
    fn same_inputs_same_bytes() {
        for c in Character::ALL {
            let r = Recipe {
                macros: spread(42),
                ..recipe(c, 42)
            };
            let a = generate(&r);
            assert_eq!(a.as_bytes(), generate(&r).as_bytes());
            assert_eq!(vary(&a, 0.6, 9).as_bytes(), vary(&a, 0.6, 9).as_bytes());
            let b = generate(&recipe(c, 43));
            assert_eq!(
                breed(&a, &b, 0.4, 3).as_bytes(),
                breed(&a, &b, 0.4, 3).as_bytes()
            );
            assert_ne!(a.as_bytes(), generate(&recipe(c, 43)).as_bytes());
        }
    }

    #[test]
    fn characters_land_in_their_category() {
        for c in Character::ALL {
            let hits = (0..500)
                .filter(|&seed| {
                    let r = Recipe {
                        macros: spread(seed),
                        ..recipe(c, seed)
                    };
                    category(&generate(&r)) == c.category()
                })
                .count();
            assert!(hits >= 450, "{c:?}: {hits} of 500 in {:?}", c.category());
        }
    }

    #[test]
    fn rock_characters_category() {
        use Category::*;
        assert_eq!(Character::PowerChord.category(), Lead);
        assert_eq!(Character::ScreamLead.category(), Lead);
        assert_eq!(Character::FuzzBass.category(), Bass);
        assert_eq!(Character::Pad.category(), Pad);
        assert_eq!(Character::Drone.category(), Drone);
        assert_eq!(Character::ArpPulse.category(), Arp);
        for c in [Character::Riser, Character::SpaceFx, Character::Wobble] {
            assert_eq!(c.category(), Fx);
        }
    }

    #[test]
    fn power_chords_are_driven_fifths() {
        for seed in 0..200 {
            let p = generate(&recipe(Character::PowerChord, seed));
            assert_eq!(p.get(Field::VoiceModeType), 2, "chord mode");
            assert!(p.get(Field::CutoffDrive) >= 1, "driven");
        }
        let fifths = (0..200)
            .filter(|&s| {
                let p = generate(&recipe(Character::PowerChord, s));
                present::format_in(&p, Field::VoiceModeDepth) == "5th"
            })
            .count();
        assert!(fifths > 120, "mostly fifths: {fifths}");
    }

    #[test]
    fn fuzz_bass_is_low_and_fully_driven() {
        for seed in 0..200 {
            let p = generate(&recipe(Character::FuzzBass, seed));
            assert_eq!(p.get(Field::CutoffDrive), 2);
            assert!(p.get(Field::Octave) < 2 || p.get(Field::Vco1Octave) == 0);
        }
    }

    /// Averages over 200 seeds as one macro rises, the others fixed.
    fn averages(c: Character, field: Field, set: impl Fn(&mut Macros, f32)) -> Vec<f32> {
        [0.0, 0.25, 0.5, 0.75, 1.0]
            .into_iter()
            .map(|t| {
                let sum: u32 = (0..200)
                    .map(|seed| {
                        let mut r = recipe(c, seed);
                        set(&mut r.macros, t);
                        u32::from(generate(&r).get(field))
                    })
                    .sum();
                sum as f32 / 200.0
            })
            .collect()
    }

    #[test]
    fn more_bright_never_darker_more_space_never_drier() {
        for c in Character::ALL {
            let cut = averages(c, Field::Cutoff, |m, t| m.bright = t);
            assert!(cut.windows(2).all(|w| w[0] <= w[1]), "{c:?} cutoff {cut:?}");
            assert!(cut[4] > cut[0], "{c:?} bright does something");
            let wet = averages(c, Field::ReverbDryWet, |m, t| m.space = t);
            assert!(wet.windows(2).all(|w| w[0] <= w[1]), "{c:?} reverb {wet:?}");
            assert!(wet[4] > wet[0], "{c:?} space does something");
        }
    }

    #[test]
    fn vary_by_nothing_is_the_same_program() {
        for c in Character::ALL {
            for seed in 0..50 {
                let p = generate(&recipe(c, seed));
                assert_eq!(vary(&p, 0.0, seed + 1), p);
            }
        }
    }

    #[test]
    fn vary_moves_further_with_amount() {
        let mean = |amount: f32| {
            let mut total = 0.0;
            for c in Character::ALL {
                for seed in 0..100 {
                    let p = generate(&recipe(c, seed));
                    total += distance(&p, &vary(&p, amount, seed + 99));
                }
            }
            total
        };
        let (a, b, c) = (mean(0.1), mean(0.4), mean(0.9));
        assert!(0.0 < a && a < b && b < c, "{a} {b} {c}");
    }

    #[test]
    fn breed_ends_are_the_parents() {
        for seed in 0..50 {
            let a = generate(&recipe(Character::Pad, seed));
            let b = generate(&recipe(Character::ScreamLead, seed));
            assert!(same_sound(&breed(&a, &b, 0.0, seed), &a));
            assert!(same_sound(&breed(&a, &b, 1.0, seed), &b));
            let child = breed(&a, &b, 0.5, seed);
            assert!(!same_sound(&child, &a) && !same_sound(&child, &b));
        }
    }

    #[test]
    fn keep_copies_the_chosen_stages() {
        let base = generate(&recipe(Character::Bell, 5));
        let keep = vec![Stage::Filter, Stage::Envelopes];
        let r = Recipe {
            keep: keep.clone(),
            ..recipe(Character::FuzzBass, 8)
        };
        let p = generate_from(&base, &r);
        for stage in keep {
            for f in stage.sections().iter().flat_map(|s| s.fields) {
                assert_eq!(p.get(*f), base.get(*f), "{f:?} kept");
            }
        }
        assert_eq!(p.get(Field::Octave), generate(&r).get(Field::Octave));
        assert_ne!(p.get(Field::Vco1Octave), base.get(Field::Vco1Octave));
    }

    #[test]
    fn names_fit_the_synth() {
        for c in Character::ALL {
            for seed in 0..1000 {
                let n = name(seed, c);
                assert!(n.is_ascii() && (1..=12).contains(&n.len()), "{n:?}");
                Program::init().set_name(&n).unwrap();
            }
        }
    }

    #[test]
    fn a_hand_is_six_different_sounds_with_different_names() {
        for c in Character::ALL {
            for seed in 0..20 {
                let deal = hand(c, seed, |s| generate(&recipe(c, s)));
                assert_eq!(deal.len(), HAND);
                let dealt = hand(c, seed, |s| generate(&recipe(c, s)));
                assert_eq!(deal, dealt, "deterministic");
                for (i, a) in deal.iter().enumerate() {
                    check(a);
                    for b in &deal[i + 1..] {
                        assert!(!same_sound(a, b));
                        assert_ne!(a.name(), b.name());
                    }
                }
            }
        }
        // Even when every attempt makes the same program.
        let p = generate(&recipe(Character::Keys, 1));
        let deal = hand(Character::Keys, 3, |_| p.clone());
        for (i, a) in deal.iter().enumerate() {
            for b in &deal[i + 1..] {
                assert!(!same_sound(a, b));
            }
        }
    }

    #[test]
    fn titles_and_hints() {
        for c in Character::ALL {
            assert!(!c.title().is_empty() && !c.hint().is_empty());
            assert_eq!(
                Character::ALL
                    .iter()
                    .filter(|d| d.title() == c.title())
                    .count(),
                1
            );
        }
    }
}
