// ANCHOR: fields
//! The Tutorial Mono's program: an 8-character name, then one byte for
//! each field. Every byte is 7-bit, so a SysEx dump carries the program
//! as it is.

use oscyl_driver::{DriverError, FieldInfo};

/// The program's parameters, in the order of their bytes after the name.
pub const FIELDS: [FieldInfo; 6] = [
    field("Wave", "Oscillator", 3),
    field("Glide", "Oscillator", 1),
    field("Cutoff", "Filter", 127),
    field("Resonance", "Filter", 127),
    field("Attack", "Envelope", 127),
    field("Release", "Envelope", 127),
];

/// The names of the values of field 0, Wave.
pub const WAVES: [&str; 4] = ["Saw", "Square", "Triangle", "Noise"];

/// The name's length in bytes: ASCII, padded with spaces.
pub const NAME_LEN: usize = 8;

/// A program's length in bytes.
pub const LEN: usize = NAME_LEN + FIELDS.len();

const fn field(name: &'static str, group: &'static str, max: u16) -> FieldInfo {
    FieldInfo {
        name,
        group,
        min: 0,
        max,
    }
}
// ANCHOR_END: fields

// ANCHOR: check
/// Whether `c` can be part of a name: printable ASCII.
pub fn allowed(c: char) -> bool {
    (' '..='~').contains(&c)
}

/// Refuses bytes that are not a whole program: a wrong length, a name
/// character that is not printable ASCII, or a field out of its range.
pub fn check(bytes: &[u8]) -> Result<(), DriverError> {
    if bytes.len() != LEN {
        return Err(DriverError(format!(
            "a program has {LEN} bytes, not {}",
            bytes.len()
        )));
    }
    let (name, values) = bytes.split_at(NAME_LEN);
    if !name.iter().all(|&b| allowed(char::from(b))) {
        return Err(DriverError("the name is not printable ASCII".into()));
    }
    for (f, &v) in FIELDS.iter().zip(values) {
        if u16::from(v) > f.max {
            return Err(DriverError(format!("{} = {v} is out of range", f.name)));
        }
    }
    Ok(())
}
// ANCHOR_END: check

// ANCHOR: values
/// The new program: "Init", a saw with the filter open.
pub fn init() -> Vec<u8> {
    let mut bytes = b"Init    ".to_vec();
    bytes.extend([0, 0, 127, 0, 0, 20]);
    bytes
}

/// The value of field `i` in `bytes`.
pub fn get(bytes: &[u8], i: usize) -> Result<u16, DriverError> {
    check(bytes)?;
    let at = NAME_LEN + index(i)?;
    Ok(u16::from(bytes[at]))
}

/// Sets field `i` in `bytes` to `value`, which must be in its range.
pub fn set(bytes: &mut [u8], i: usize, value: u16) -> Result<(), DriverError> {
    check(bytes)?;
    let f = &FIELDS[index(i)?];
    if value > f.max {
        return Err(DriverError(format!("{} = {value} is out of range", f.name)));
    }
    // The range check above keeps `value` at 127 or less.
    bytes[NAME_LEN + i] = value as u8;
    Ok(())
}

fn index(i: usize) -> Result<usize, DriverError> {
    if i < FIELDS.len() {
        Ok(i)
    } else {
        Err(DriverError(format!("no field {i}")))
    }
}
// ANCHOR_END: values

// ANCHOR: name
/// The program's name, without the padding.
pub fn name(bytes: &[u8]) -> Result<String, DriverError> {
    check(bytes)?;
    let name = String::from_utf8_lossy(&bytes[..NAME_LEN]);
    Ok(name.trim_end().to_string())
}

/// Sets the name: at most 8 printable ASCII characters.
pub fn set_name(bytes: &mut [u8], name: &str) -> Result<(), DriverError> {
    check(bytes)?;
    if name.len() > NAME_LEN || !name.chars().all(allowed) {
        return Err(DriverError(format!(
            "a name has at most {NAME_LEN} printable ASCII characters"
        )));
    }
    let padded = format!("{name:<NAME_LEN$}");
    bytes[..NAME_LEN].copy_from_slice(padded.as_bytes());
    Ok(())
}
// ANCHOR_END: name

// ANCHOR: tests
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_field_and_the_name_read_back() {
        let mut p = init();
        set(&mut p, 2, 64).unwrap();
        set_name(&mut p, "Bass").unwrap();
        assert_eq!(get(&p, 2), Ok(64));
        assert_eq!(name(&p).as_deref(), Ok("Bass"));
        assert!(set(&mut p, 0, 4).is_err(), "Wave has four values");
        assert!(check(&p[1..]).is_err(), "a program has 14 bytes");
    }
}
// ANCHOR_END: tests
