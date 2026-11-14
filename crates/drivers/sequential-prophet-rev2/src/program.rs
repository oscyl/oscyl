//! A Prophet Rev2 program: 2046 unpacked bytes, layer A then layer B
//!.

use std::ops::Range;

use crate::fields::LAYER_B;
use crate::{Error, Field};

pub const PROGRAM_LEN: usize = 2046;

/// Layer A's name, which is the program name. Layer B's is 1024 further.
const NAME: Range<usize> = 235..255;

/// Characters a name may hold: printable ASCII, as in every factory program.
pub fn name_char(c: char) -> bool {
    matches!(c, ' '..='~')
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Program([u8; PROGRAM_LEN]);

impl std::fmt::Debug for Program {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Program({:?})", self.name())
    }
}

impl Program {
    /// A neutral, synthetic program named "Init Program" in both layers.
    /// No factory data is embedded.
    ///
    /// Bytes whose meaning is unknown but which factory programs fill
    /// get the value the
    /// format notes document: 233 = 2 (every factory program), 255 = 100
    /// (511 of 512), 121 = 3 (the lowest of the documented 3, 4, 6, 7).
    /// The rest stay 0, a value the notes list as in use. Revisit once
    /// hardware tells what they mean.
    pub fn init() -> Program {
        let mut p = Program([0; PROGRAM_LEN]);
        for layer in [0, usize::from(LAYER_B)] {
            for (offset, value) in [
                (0, 24),    // osc 1 frequency
                (1, 24),    // osc 2 frequency
                (2, 50),    // osc 1 fine tune: centre
                (3, 50),    // osc 2 fine tune: centre
                (4, 1),     // osc 1 shape: saw
                (5, 1),     // osc 2 shape: saw
                (10, 1),    // osc 1 keyboard on
                (11, 1),    // osc 2 keyboard on
                (14, 64),   // osc mix: even
                (20, 2),    // pitch bend range
                (22, 164),  // filter cutoff: open
                (28, 127),  // program volume
                (32, 127),  // filter env amount: zero (bipolar)
                (33, 127),  // amp env amount
                (34, 127),  // env 3 amount: zero (bipolar)
                (48, 127),  // amp env sustain
                (101, 127), // mod wheel amount: zero (bipolar)
                (103, 127), // pressure amount: zero
                (105, 127), // breath amount: zero
                (107, 127), // velocity amount: zero
                (109, 127), // foot amount: zero
                (121, 3),
                (130, 120), // BPM
                (233, 2),
                (255, 100),
            ] {
                p.0[layer + offset] = value;
            }
            // Mod 1-8 amounts: zero (bipolar).
            p.0[layer + 85..layer + 93].fill(127);
            p.0[layer + NAME.start..layer + NAME.end].copy_from_slice(b"Init Program        ");
        }
        p.0[232] = 60; // split point: middle C
        p
    }

    /// Any 2046 bytes: values are kept as given, in range or not.
    pub fn from_bytes(bytes: &[u8]) -> Result<Program, Error> {
        bytes.try_into().map(Program).map_err(|_| {
            Error(format!(
                "a Prophet Rev2 program is {PROGRAM_LEN} bytes, got {}",
                bytes.len()
            ))
        })
    }

    pub fn as_bytes(&self) -> &[u8; PROGRAM_LEN] {
        &self.0
    }

    /// Layer A's name, trailing spaces removed.
    pub fn name(&self) -> String {
        let s: String = self.0[NAME]
            .iter()
            .map(|&b| if name_char(b as char) { b as char } else { ' ' })
            .collect();
        s.trim_end().to_string()
    }

    /// Set layer A's name, space padded. Layer B's name is left alone.
    pub fn set_name(&mut self, name: &str) -> Result<(), Error> {
        if name.chars().count() > NAME.len() || !name.chars().all(name_char) {
            return Err(Error(format!(
                "invalid program name {name:?}: up to {} printable ASCII characters",
                NAME.len()
            )));
        }
        let field = &mut self.0[NAME];
        field.fill(b' ');
        field[..name.len()].copy_from_slice(name.as_bytes());
        Ok(())
    }

    pub fn get(&self, f: &Field) -> u16 {
        self.0[usize::from(f.offset)].into()
    }

    pub fn set(&mut self, f: &Field, value: u16) -> Result<(), Error> {
        if !(f.min..=f.max).contains(&value) {
            return Err(Error(format!(
                "{} = {value} is outside {}..={}",
                f.name, f.min, f.max
            )));
        }
        // Every field's max is at most 255.
        self.0[usize::from(f.offset)] = value as u8;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fields::all;

    #[test]
    fn init_is_in_range_for_every_field() {
        let p = Program::init();
        for f in all() {
            let v = p.get(f);
            assert!((f.min..=f.max).contains(&v), "{} = {v}", f.name);
        }
        assert_eq!(p.name(), "Init Program");
        assert_eq!(
            p.get(all().iter().find(|f| f.name == "A BPM").unwrap()),
            120
        );
    }

    #[test]
    fn opaque_bytes_survive_field_edits() {
        let bytes: Vec<u8> = (0..PROGRAM_LEN).map(|i| (i * 37 + 11) as u8).collect();
        let mut p = Program::from_bytes(&bytes).unwrap();
        for f in all() {
            p.set(f, f.min).unwrap();
            assert_eq!(p.get(f), f.min, "{}", f.name);
            p.set(f, f.max).unwrap();
            assert_eq!(p.get(f), f.max, "{}", f.name);
            assert!(p.set(f, f.max + 1).is_err(), "{}", f.name);
        }
        let is_field: Vec<bool> = (0..PROGRAM_LEN)
            .map(|i| all().iter().any(|f| f.offset as usize == i))
            .collect();
        let untouched = (0..PROGRAM_LEN).filter(|&i| !is_field[i]);
        let mut n = 0;
        for i in untouched {
            assert_eq!(p.as_bytes()[i], bytes[i], "byte {i}");
            n += 1;
        }
        // Unknown bytes in use, unused gaps and both names.
        assert!(
            n >= 2 * (1 + 5 + 1 + 4 + 21 + 2 + 20 + 1),
            "{n} opaque bytes"
        );
        assert!(Program::from_bytes(&bytes[1..]).is_err());
    }

    #[test]
    fn name_round_trip_and_limits() {
        let mut p = Program::init();
        p.set_name("LosVangelis~ 20 char").unwrap();
        assert_eq!(p.name(), "LosVangelis~ 20 char");
        assert_eq!(&p.as_bytes()[235..255], b"LosVangelis~ 20 char");
        p.set_name("Pad").unwrap();
        assert_eq!(p.name(), "Pad");
        assert_eq!(&p.as_bytes()[235..255], b"Pad                 ");
        assert!(p.set_name("123456789012345678901").is_err());
        assert!(p.set_name("caf\u{e9}").is_err());
        assert!(p.set_name("tab\t").is_err());
        assert_eq!(p.name(), "Pad");
    }
}
