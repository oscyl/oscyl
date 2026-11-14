//! A prologue program: 336 unpacked bytes (KORG TABLE 3), main and sub
//! timbre, no sequencer.

use std::ops::Range;

use oscyl_logue::Error;
use oscyl_logue::kinds::{read_name, write_name};

use crate::fields::{Field, Layout, NOT_SET, TIMBRES, field};

pub const PROGRAM_LEN: usize = 336;
const NAME: Range<usize> = 4..16;
/// `PROG` at 0, `PRED` at 332: the only check the firmware makes (on 0x40;
/// it stores a 0x4C unchecked, so the driver checks first).
const MARKERS: [(usize, &[u8; 4]); 2] = [(0, b"PROG"), (332, b"PRED")];

#[derive(Clone, PartialEq, Eq)]
pub struct Program([u8; PROGRAM_LEN]);

impl std::fmt::Debug for Program {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Program({:?})", self.name())
    }
}

impl Program {
    /// A fresh program: the markers, the name "Init Program", the reserved
    /// bytes the factory set holds as `FF`, the 2.00 fields "not set" (as in
    /// every 1.x program), and neutral values from the documented meanings
    /// (centred pitches and EG ints, one open VCO, poly). No KORG factory
    /// data is embedded.
    pub fn init() -> Program {
        let mut b = [0u8; PROGRAM_LEN];
        for (off, tag) in MARKERS {
            b[off..off + 4].copy_from_slice(tag);
        }
        for off in [21, 35, 36] {
            b[off] = 0xFF;
        }
        for (start, _) in TIMBRES {
            for off in [1, 3, 90, 120, 121, 122, 123, 124, 125] {
                b[(start + off) as usize] = 0xFF;
            }
        }
        let mut p = Program(b);
        p.set_name("Init Program").expect("valid name");
        for (name, v) in [
            ("Octave", 2),
            ("Edit Timbre", 2),
            ("Balance", 64),
            ("Split Point", 60),
            ("Program Level", 102),
            ("Arpeggiator Tempo", 1200),
            ("Mod FX Type", 1),
            ("Delay/Reverb Type", 1),
        ] {
            p.set(field(name), v).expect("in range");
        }
        for timbre in ["Main", "Sub"] {
            for (name, v) in [
                ("VCO 1 Octave", 1),
                ("VCO 2 Octave", 1),
                ("VCO 1 Pitch", 512),
                ("VCO 2 Pitch", 512),
                ("VCO 2 Ring/Sync", 1), // off
                ("Pitch EG Int", 512),
                ("Mixer VCO 1", 1023),
                ("Filter Cutoff", 1023),
                ("Filter EG Int", 512),
                ("Amp EG Sustain", 1023),
                ("LFO Wave", 1),
                ("LFO Mode", 1),
                ("LFO Key Sync", 1),
                ("LFO Voice Sync", 1),
                ("Controllers Mod Wheel", 28),
                ("Controllers Mod Wheel Range", 200),
                ("Controllers Bend +", 2),
                ("Controllers Bend -", 2),
                ("Multi Engine Feedback", 100),
                ("Multi Engine Noise Depth", 100),
                ("Multi Engine Shape Mod", 100),
                ("Multi Engine Mod Attack", 100),
                ("Multi Engine Mod Decay", 100),
                ("Multi Engine Mod Key Track", 100),
            ] {
                p.set(field(&format!("{timbre} {name}")), v)
                    .expect("in range");
            }
        }
        p
    }

    /// Parses an unpacked program: 336 bytes with both markers. Field
    /// values are not checked: a 1.x program's 0 in a 2.00 field, or an
    /// undocumented value, is kept as it is.
    pub fn from_bytes(b: &[u8]) -> Result<Program, Error> {
        let b: [u8; PROGRAM_LEN] = b.try_into().map_err(|_| Error::Length {
            expected: PROGRAM_LEN,
            got: b.len(),
        })?;
        if MARKERS.iter().any(|(off, tag)| &b[*off..off + 4] != *tag) {
            return Err(Error::Malformed("missing program marker"));
        }
        Ok(Program(b))
    }

    pub fn as_bytes(&self) -> &[u8; PROGRAM_LEN] {
        &self.0
    }

    /// The stored name up to the first NUL, trailing spaces kept.
    pub fn name(&self) -> String {
        read_name(&self.0[NAME])
    }

    /// Sets the name (at most 12 displayable ASCII characters), NUL-padded.
    pub fn set_name(&mut self, name: &str) -> Result<(), Error> {
        write_name(&mut self.0[NAME], name).ok_or_else(|| Error::InvalidName(name.into()))
    }

    pub fn get(&self, f: &Field) -> u16 {
        let o = f.offset as usize;
        let b = self.0[o];
        match f.layout {
            Layout::Byte => b.into(),
            Layout::Word => u16::from_le_bytes([b, self.0[o + 1]]),
            Layout::Bits { shift, bits } => ((b >> shift) & ((1 << bits) - 1)).into(),
            Layout::UnsetFf if b == 0xFF || b == 0 => NOT_SET,
            Layout::UnsetFf => b.into(),
        }
    }

    pub fn set(&mut self, f: &Field, value: u16) -> Result<(), Error> {
        if !(f.min..=f.max).contains(&value) {
            return Err(Error::OutOfRange {
                field: f.name,
                value,
                min: f.min,
                max: f.max,
            });
        }
        let o = f.offset as usize;
        match f.layout {
            Layout::Byte => self.0[o] = value as u8,
            Layout::Word => self.0[o..o + 2].copy_from_slice(&value.to_le_bytes()),
            Layout::Bits { shift, bits } => {
                let m = ((1u8 << bits) - 1) << shift;
                self.0[o] = self.0[o] & !m | (value as u8) << shift & m;
            }
            Layout::UnsetFf => {
                let stored_unset = matches!(self.0[o], 0 | 0xFF);
                self.0[o] = match value {
                    NOT_SET if stored_unset => self.0[o],
                    NOT_SET => 0xFF,
                    v => v as u8,
                }
            }
        }
        Ok(())
    }

    /// Copy `f`'s stored bits from `src`, in range or not.
    pub fn copy_field(&mut self, src: &Program, f: &Field) {
        let o = f.offset as usize;
        match f.layout {
            Layout::Word => self.0[o..o + 2].copy_from_slice(&src.0[o..o + 2]),
            Layout::Bits { shift, bits } => {
                let m = ((1u8 << bits) - 1) << shift;
                self.0[o] = self.0[o] & !m | src.0[o] & m;
            }
            Layout::Byte | Layout::UnsetFf => self.0[o] = src.0[o],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fields::all;

    #[test]
    fn init_is_valid_and_in_range() {
        let p = Program::init();
        assert_eq!(&p.as_bytes()[..4], b"PROG");
        assert_eq!(&p.as_bytes()[332..], b"PRED");
        assert_eq!(p.name(), "Init Program");
        assert_eq!(Program::from_bytes(p.as_bytes()).unwrap(), p);
        for f in all() {
            assert!((f.min..=f.max).contains(&p.get(f)), "{}", f.name);
        }
    }

    #[test]
    fn from_bytes_checks_markers_and_length() {
        let mut b = *Program::init().as_bytes();
        b[332] = b'X';
        assert!(Program::from_bytes(&b).is_err());
        assert!(matches!(
            Program::from_bytes(&[0; 1024]),
            Err(Error::Length { expected: 336, .. })
        ));
    }

    #[test]
    fn ten_bit_little_endian() {
        let mut p = Program::init();
        p.set(field("Main Filter Cutoff"), 0x2C5).unwrap();
        assert_eq!(&p.as_bytes()[124..126], &[0xC5, 0x02]);
        p.set(field("Arpeggiator Tempo"), 1200).unwrap();
        assert_eq!(&p.as_bytes()[24..26], &1200u16.to_le_bytes());
    }

    #[test]
    fn pre_2_00_zeros_are_kept_and_valid() {
        // A 1.x program: 0 in bytes 51-58 (micro tuning, scale key,
        // tuning, transpose, arp gate and rate, dry/wet) and FF aftertouch.
        let mut b = *Program::init().as_bytes();
        b[51..59].fill(0);
        b[80 + 120] = 0xFF;
        let p = Program::from_bytes(&b).unwrap();
        for name in [
            "Micro Tuning",
            "Scale Key",
            "Program Tuning",
            "Program Transpose",
            "Arpeggiator Gate Time",
            "Arpeggiator Rate",
            "Delay/Reverb Dry/Wet",
        ] {
            let f = field(name);
            assert_eq!(p.get(f), 0, "{name}");
            assert!(f.min == 0, "{name} must accept 0");
        }
        assert_eq!(p.get(field("Main Controllers Aftertouch")), NOT_SET);
        assert_eq!(p.as_bytes(), &b, "kept byte for byte");
    }

    #[test]
    fn aftertouch_keeps_its_stored_not_set_value() {
        // KORG's range is 2-32, so 1 (the P24 "Volume") is the schema's
        // "not set", stored as FF (1.x) or 0.
        let f = field("Sub Controllers Aftertouch");
        assert_eq!((f.min, f.max), (NOT_SET, 32));
        let mut p = Program::init();
        assert_eq!(p.as_bytes()[326], 0xFF);
        assert_eq!(p.get(f), NOT_SET);
        p.set(f, 16).unwrap();
        assert_eq!((p.as_bytes()[326], p.get(f)), (16, 16));
        p.set(f, NOT_SET).unwrap();
        assert_eq!(p.as_bytes()[326], 0xFF);
        // A stored 0 reads "not set" and stays 0 when set "not set".
        let mut b = *Program::init().as_bytes();
        b[326] = 0;
        let mut p = Program::from_bytes(&b).unwrap();
        assert_eq!(p.get(f), NOT_SET);
        p.set(f, NOT_SET).unwrap();
        assert_eq!(p.as_bytes()[326], 0);
        assert!(p.set(f, 0).is_err());
    }

    #[test]
    fn bit_fields_leave_neighbours_alone() {
        let mut p = Program::init();
        p.set(field("Main Multi Engine Param 2 Type"), 2).unwrap();
        p.set(field("Main Multi Engine Param 4 Type"), 3).unwrap();
        assert_eq!(p.as_bytes()[80 + 106], 0b1100_1000);
        assert_eq!(p.get(field("Main Multi Engine Param 1 Type")), 0);
    }
}
