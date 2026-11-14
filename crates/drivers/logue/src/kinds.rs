//! Field kinds the logue models share: value lists, the meanings KORG
//! documents for raw values, CC value bands, envelope and wave shapes, and
//! the program name codec. Each model keeps its own field table; only the
//! kinds are shared.

pub const WAVES: &[&str] = &["Square", "Triangle", "Saw"];
pub const FEET: &[&str] = &["16'", "8'", "4'", "2'"];
pub const THIRDS: &[&str] = &["0%", "50%", "100%"];
pub const USER_SLOTS: &[&str] = &[
    "User 1", "User 2", "User 3", "User 4", "User 5", "User 6", "User 7", "User 8", "User 9",
    "User 10", "User 11", "User 12", "User 13", "User 14", "User 15", "User 16",
];
pub const NOISE_TYPES: &[&str] = &["High", "Low", "Peak", "Decim"];
pub const VPM_TYPES: &[&str] = &[
    "Sin 1", "Sin 2", "Sin 3", "Sin 4", "Saw 1", "Saw 2", "Squ 1", "Squ 2", "Fat 1", "Fat 2",
    "Air 1", "Air 2", "Decay 1", "Decay 2", "Creep", "Throat",
];
pub const MULTI_TYPES: &[&str] = &["Noise", "VPM", "User"];
pub const MULTI_ROUTING: &[&str] = &["Pre VCF", "Post VCF"];
pub const LFO_TARGETS: &[&str] = &["Cutoff", "Shape", "Pitch"];
pub const LFO_TARGET_OSC: &[&str] = &["All", "VCO 1+2", "VCO 2", "Multi"];
pub const MOD_FX_TYPES: &[&str] = &["Chorus", "Ensemble", "Phaser", "Flanger", "User"];
pub const CHORUS_TYPES: &[&str] = &[
    "Stereo", "Light", "Deep", "Triphase", "Harmonic", "Mono", "Feedback", "Vibrato",
];
pub const ENSEMBLE_TYPES: &[&str] = &["Stereo", "Light", "Mono"];
pub const PHASER_TYPES: &[&str] = &[
    "Stereo",
    "Fast",
    "Orange",
    "Small",
    "Small Reso",
    "Black",
    "Formant",
    "Twinkle",
];
pub const FLANGER_TYPES: &[&str] = &[
    "Stereo",
    "Light",
    "Mono",
    "High Sweep",
    "Mid Sweep",
    "Pan Sweep",
    "Mono Sweep",
    "Triphase",
];
pub const DELAY_TYPES: &[&str] = &[
    "Stereo",
    "Mono",
    "Ping Pong",
    "Hipass",
    "Tape",
    "One Tap",
    "Stereo BPM",
    "Mono BPM",
    "Ping BPM",
    "Hipass BPM",
    "Tape BPM",
    "Doubling",
    "User 1",
    "User 2",
    "User 3",
    "User 4",
    "User 5",
    "User 6",
    "User 7",
    "User 8",
];
pub const REVERB_TYPES: &[&str] = &[
    "Hall",
    "Smooth",
    "Arena",
    "Plate",
    "Room",
    "Early Ref",
    "Space",
    "Riser",
    "Submarine",
    "Horror",
    "User 1",
    "User 2",
    "User 3",
    "User 4",
    "User 5",
    "User 6",
    "User 7",
    "User 8",
];
pub const ARP_RATES: &[&str] = &[
    "64th", "48th", "32nd", "24th", "16th", "16th.", "12th", "8th", "8th.", "6th", "4th",
];
/// LFO rate names when the LFO is synced to BPM (xd note P11, prologue
/// P22), one per 64 raw steps.
pub const LFO_BPM_RATES: &[&str] = &[
    "4", "2", "1", "3/4", "1/2", "3/8", "1/3", "1/4", "3/16", "1/6", "1/8", "1/12", "1/16", "1/24",
    "1/32", "1/36",
];
pub const MICROTUNINGS: &[&str] = &[
    "Equal Temp",
    "Pure Major",
    "Pure Minor",
    "Pythagorean",
    "Werckmeister",
    "Kirnburger",
    "Slendro",
    "Pelog",
    "Ionian",
    "Dorian",
    "Aeolian",
    "Major Penta",
    "Minor Penta",
    "Reverse",
    "AFX001",
    "AFX002",
    "AFX003",
    "AFX004",
    "AFX005",
    "AFX006",
    "DC001",
    "DC002",
    "DC003",
];
pub const CHORDS: &[&str] = &[
    "5th", "sus2", "m", "Maj", "sus4", "m7", "7", "7sus4", "Maj7", "aug", "dim", "m7b5", "mMaj7",
    "Maj7b5",
];
pub const USER_PARAM_TYPES: &[&str] = &["Percent", "Bipolar", "Select"];

/// A micro tuning (0-22 presets, 128-133 user scales, 134-139 user
/// octaves), by name.
pub fn microtuning(raw: u16) -> String {
    match raw {
        0..=22 => MICROTUNINGS[raw as usize].into(),
        128..=133 => format!("User Scale {}", raw - 127),
        134..=139 => format!("User Octave {}", raw - 133),
        _ => raw.to_string(),
    }
}

/// VCO pitch in cents (xd note P5, prologue P16): piecewise linear through
/// KORG's breakpoints, flat at 0 between 492 and 532.
pub fn pitch_cents(raw: u16) -> f32 {
    const PTS: [(f32, f32); 10] = [
        (0.0, -1200.0),
        (4.0, -1200.0),
        (356.0, -256.0),
        (476.0, -16.0),
        (492.0, 0.0),
        (532.0, 0.0),
        (548.0, 16.0),
        (668.0, 256.0),
        (1020.0, 1200.0),
        (1023.0, 1200.0),
    ];
    let x = raw.min(1023) as f32;
    for w in PTS.windows(2) {
        let ((x0, y0), (x1, y1)) = (w[0], w[1]);
        if x <= x1 {
            return y0 + (y1 - y0) * (x - x0) / (x1 - x0);
        }
    }
    1200.0
}

/// EG int in percent (xd note P10, prologue cutoff EG int P20): quadratic
/// around a flat centre.
pub fn eg_int_percent(raw: u16) -> f32 {
    let v = raw as f32;
    let q = |d: f32| d * d * 4641.0 * 100.0 / 1_073_741_824.0;
    match raw {
        0..=11 => -100.0,
        12..=491 => -q(492.0 - v),
        492..=532 => 0.0,
        533..=1012 => q(v - 532.0),
        _ => 100.0,
    }
}

/// `+n<unit>` / `0<unit>` / `-n<unit>`.
pub fn signed(v: i32, unit: &str) -> String {
    if v > 0 {
        format!("+{v}{unit}")
    } else {
        format!("{v}{unit}")
    }
}

/// Program level 12-132: -18 dB to +6 dB in 0.2 dB steps, 102 = 0 dB.
pub fn level_db(raw: u16) -> String {
    let db = (raw as i32 - 102) * 2; // tenths of a dB
    let sign = if db > 0 {
        "+"
    } else if db < 0 {
        "-"
    } else {
        ""
    };
    format!("{sign}{}.{} dB", db.abs() / 10, db.abs() % 10)
}

/// Gate time 0-72 as percent.
pub fn gate_percent(raw: u16) -> String {
    format!("{}%", (raw as u32 * 100 + 36) / 72)
}

/// ADSR corner points (x time 0..1, y level 0..1) from raw 0–1023 values.
/// Attack, decay and release each get up to a quarter of the width and the
/// sustain hold a fixed quarter, so every setting fits the same box.
pub fn adsr(attack: u16, decay: u16, sustain: u16, release: u16) -> [(f32, f32); 5] {
    let q = |v: u16| v.min(1023) as f32 / 1023.0 * 0.25;
    let s = sustain.min(1023) as f32 / 1023.0;
    let a = q(attack);
    let d = a + q(decay);
    let h = d + 0.25;
    [(0.0, 0.0), (a, 1.0), (d, s), (h, s), (h + q(release), 0.0)]
}

/// One VCO or LFO sample at `phase` (0..1): wave 0 square (shape narrows
/// the pulse from 50 % to 5 %), 1 triangle (shape leans the apex towards a
/// ramp) and 2 saw (shape has no visible effect here).
pub fn wave_sample(wave: u16, shape: f32, phase: f32) -> f32 {
    match wave {
        0 => {
            if phase < 0.5 - 0.45 * shape {
                1.0
            } else {
                -1.0
            }
        }
        1 => {
            let apex = 0.5 + 0.45 * shape;
            if phase < apex {
                2.0 * phase / apex - 1.0
            } else {
                1.0 - 2.0 * (phase - apex) / (1.0 - apex)
            }
        }
        _ => 2.0 * phase - 1.0,
    }
}

/// Characters the synths can display in a program name (KORG note P1).
pub fn name_char(c: u8) -> bool {
    c.is_ascii_alphanumeric() || b" !#$%&'()*,-./:?".contains(&c)
}

/// The 12-byte name field up to the first NUL, trailing spaces kept.
pub fn read_name(field: &[u8]) -> String {
    let end = field.iter().position(|&c| c == 0).unwrap_or(field.len());
    String::from_utf8_lossy(&field[..end]).into_owned()
}

/// `name` NUL-padded into `field`, or `None` when it is too long or uses a
/// character the synth cannot display.
pub fn write_name(field: &mut [u8], name: &str) -> Option<()> {
    if name.len() > field.len() || !name.bytes().all(name_char) {
        return None;
    }
    field.fill(0);
    field[..name.len()].copy_from_slice(name.as_bytes());
    Some(())
}

/// CC values at the centre of each band the synths recognise for a choice
/// (xd notes *6-xx, prologue *5-xx).
pub mod cc {
    pub const HALVES: [u8; 2] = [0, 127];
    pub const THIRDS: [u8; 3] = [0, 64, 127];
    pub const QUARTERS: [u8; 4] = [0, 42, 84, 127];
    pub const EIGHTHS: [u8; 8] = [0, 16, 32, 48, 64, 80, 96, 127];
    pub const SIXTEENTHS: [u8; 16] = [
        0, 8, 16, 24, 32, 40, 48, 56, 64, 72, 80, 88, 96, 104, 112, 127,
    ];
    pub const DELAY_TYPES: [u8; 20] = [
        0, 7, 13, 20, 26, 32, 39, 45, 52, 58, 64, 71, 77, 84, 90, 96, 103, 109, 116, 127,
    ];
    /// The implementations name 18 reverb choices.
    pub const REVERB_TYPES: [u8; 18] = [
        0, 8, 15, 22, 29, 36, 43, 50, 57, 64, 72, 79, 86, 93, 100, 107, 117, 127,
    ];
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_codec_pads_with_nul_and_refuses_bad_names() {
        let mut f = [b' '; 12];
        write_name(&mut f, "Bass 1").unwrap();
        assert_eq!(&f, b"Bass 1\0\0\0\0\0\0");
        assert_eq!(read_name(&f), "Bass 1");
        assert_eq!(read_name(b"Space pad   "), "Space pad   ");
        assert!(write_name(&mut f, "Thirteen chrs").is_none());
        assert!(write_name(&mut f, "Bad~").is_none());
    }

    #[test]
    fn documented_value_meanings() {
        assert_eq!(pitch_cents(0), -1200.0);
        assert_eq!(pitch_cents(512), 0.0);
        assert_eq!(eg_int_percent(1023), 100.0);
        assert_eq!(level_db(102), "0.0 dB");
        assert_eq!(microtuning(134), "User Octave 1");
        assert_eq!(gate_percent(72), "100%");
    }
}
