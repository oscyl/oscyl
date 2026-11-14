//! A minilogue xd program: 1024 unpacked bytes (KORG TABLE 2).

use std::ops::Range;

use crate::msg::PROGRAM_LEN;
use crate::{Error, Field};

/// Fixed ASCII markers and their offsets.
pub const MARKERS: [(usize, &[u8]); 3] = [(0, b"PROG"), (156, b"PRED"), (160, b"SQ")];

/// Documented regions with no typed `Field`: the name (see `Program::name`)
/// and the per-step sequencer event data (16 steps × 52 bytes, KORG note S3).
pub const RAW: [(Range<usize>, &str); 2] =
    [(4..16, "program name"), (190..1022, "step event data")];

const NAME: Range<usize> = 4..16;

pub(crate) use oscyl_logue::kinds::name_char;

#[derive(Clone, PartialEq, Eq)]
pub struct Program([u8; PROGRAM_LEN]);

impl std::fmt::Debug for Program {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Program({:?})", self.name())
    }
}

impl Program {
    /// A fresh program built from the zeroed layout plus the markers, the
    /// name "Init Program", and neutral values derived from the documented
    /// meanings (centre for bipolar parameters, the lowest legal value for
    /// parameters whose range starts above zero, all sequencer steps active).
    /// No KORG factory data is embedded.
    pub fn init() -> Program {
        let mut b = [0u8; PROGRAM_LEN];
        for (off, tag) in MARKERS {
            b[off..off + tag.len()].copy_from_slice(tag);
        }
        let mut p = Program(b);
        p.set_name("Init Program").unwrap();
        use Field::*;
        for (f, v) in [
            (Octave, 2),          // 0
            (VoiceModeType, 4),   // poly
            (Vco1Octave, 1),      // 8'
            (Vco2Octave, 1),      // 8'
            (Vco1Pitch, 512),     // 0 cent
            (Vco2Pitch, 512),     // 0 cent
            (Vco1Level, 1023),    // audible
            (Cutoff, 1023),       // open
            (AmpEgSustain, 1023), // held
            (EgInt, 512),         // 0 %
            (ModFxType, 1),       // chorus (lowest legal value)
            (JoystickRangePlus, 200),
            (JoystickRangeMinus, 200),
            (CvIn1Range, 200),
            (CvIn2Range, 200),
            (ScaleKey, 12),      // 0 notes
            (ProgramTuning, 50), // 0 cent
            (ProgramLevel, 102), // 0 dB
            (VpmParam1, 100),    // 0 %
            (VpmParam2, 100),
            (VpmParam3, 100),
            (VpmParam4, 100),
            (VpmParam5, 100),
            (VpmParam6, 100),
            (ProgramTranspose, 13), // 0 notes
            (SeqActiveSteps, 0xFFFF),
            (SeqBpm, 1200), // 120.0
            (SeqStepLength, 16),
            (SeqSwing, 75),           // 0
            (SeqDefaultGateTime, 54), // 75 %
            (ArpGateTime, 54),
        ] {
            p.set(f, v).unwrap();
        }
        p
    }

    /// Parses an unpacked program dump. Accepts firmware 1.x's `SEQD`
    /// sequencer header as well as 2.x's `SQ`.
    pub fn from_bytes(b: &[u8]) -> Result<Program, Error> {
        let b: [u8; PROGRAM_LEN] = b.try_into().map_err(|_| Error::Length {
            expected: PROGRAM_LEN,
            got: b.len(),
        })?;
        let v1 = &b[160..164] == b"SEQD";
        for (off, tag) in MARKERS {
            if &b[off..off + tag.len()] != tag && !(v1 && off == 160) {
                return Err(Error::Malformed("missing program marker"));
            }
        }
        Ok(Program(b))
    }

    pub fn as_bytes(&self) -> &[u8; PROGRAM_LEN] {
        &self.0
    }

    /// The stored name up to the first NUL, trailing spaces kept.
    pub fn name(&self) -> String {
        oscyl_logue::kinds::read_name(&self.0[NAME])
    }

    /// Sets the name (≤ 12 displayable ASCII characters), NUL-padded.
    pub fn set_name(&mut self, name: &str) -> Result<(), Error> {
        oscyl_logue::kinds::write_name(&mut self.0[NAME], name)
            .ok_or_else(|| Error::InvalidName(name.into()))
    }

    fn word(&self, off: usize, width: u8) -> u32 {
        (0..width as usize).fold(0, |w, i| w | (self.0[off + i] as u32) << (8 * i))
    }

    pub fn get(&self, f: Field) -> u16 {
        let s = f.spec();
        ((self.word(s.offset, s.width) >> s.shift) & ((1 << s.bits) - 1)) as u16
    }

    pub fn set(&mut self, f: Field, value: u16) -> Result<(), Error> {
        let s = f.spec();
        if !(s.min..=s.max).contains(&value) {
            return Err(Error::OutOfRange {
                field: s.name,
                value,
                min: s.min,
                max: s.max,
            });
        }
        let mask = ((1u32 << s.bits) - 1) << s.shift;
        let w = self.word(s.offset, s.width) & !mask | (value as u32) << s.shift;
        for i in 0..s.width as usize {
            self.0[s.offset + i] = (w >> (8 * i)) as u8;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Error, Field};

    #[test]
    fn init_has_marker_and_name() {
        let p = Program::init();
        assert_eq!(&p.as_bytes()[..4], b"PROG");
        assert_eq!(p.name(), "Init Program");
    }

    #[test]
    fn init_round_trips_and_is_in_range() {
        let p = Program::init();
        assert_eq!(
            Program::from_bytes(p.as_bytes()).unwrap().as_bytes(),
            p.as_bytes()
        );
        for &f in Field::ALL {
            let s = f.spec();
            assert!((s.min..=s.max).contains(&p.get(f)), "{}", s.name);
        }
    }

    #[test]
    fn from_bytes_checks_markers_and_length() {
        let mut b = *Program::init().as_bytes();
        b[156] = b'X';
        assert!(Program::from_bytes(&b).is_err());
        b[156] = b'P';
        b[160..164].copy_from_slice(b"SEQD");
        assert!(Program::from_bytes(&b).is_ok());
        assert!(matches!(
            Program::from_bytes(&[0; 10]),
            Err(Error::Length { .. })
        ));
    }

    #[test]
    fn microtuning_reads_offset_0x7a() {
        let mut b = *Program::init().as_bytes();
        b[0x7A] = 130;
        assert_eq!(
            Program::from_bytes(&b).unwrap().get(Field::Microtuning),
            130
        );
    }

    #[test]
    fn ten_bit_values_are_little_endian() {
        let mut p = Program::init();
        p.set(Field::Cutoff, 0x2C5).unwrap();
        assert_eq!(&p.as_bytes()[60..62], &[0xC5, 0x02]);
        assert_eq!(p.get(Field::Cutoff), 0x2C5);
    }

    #[test]
    fn bit_fields_leave_neighbours_alone() {
        let mut p = Program::init();
        p.set(Field::UserParam2Type, 2).unwrap();
        p.set(Field::UserParam4Type, 3).unwrap();
        assert_eq!(p.as_bytes()[149], 0b1100_1000);
        assert_eq!(p.get(Field::UserParam1Type), 0);
        assert_eq!(p.get(Field::UserParam2Type), 2);
    }

    #[test]
    fn set_checks_range() {
        let mut p = Program::init();
        for &f in Field::ALL {
            let s = f.spec();
            if s.max < u16::MAX {
                assert!(matches!(p.set(f, s.max + 1), Err(Error::OutOfRange { .. })));
            }
            if s.min > 0 {
                assert!(matches!(p.set(f, s.min - 1), Err(Error::OutOfRange { .. })));
            }
            p.set(f, s.max).unwrap();
            assert_eq!(p.get(f), s.max, "{}", s.name);
        }
    }

    #[test]
    fn set_name_validates() {
        let mut p = Program::init();
        assert!(p.set_name("Thirteen chars").is_err());
        assert!(p.set_name("Bad~char").is_err());
        p.set_name("Bass 1").unwrap();
        assert_eq!(p.name(), "Bass 1");
        assert_eq!(&p.as_bytes()[4..16], b"Bass 1\0\0\0\0\0\0");
    }
}
