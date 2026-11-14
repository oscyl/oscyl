//! KORG 7-in-8 packing: each group of up to 7 data bytes is sent as one byte
//! holding their MSBs (bit i = MSB of byte i) followed by their low 7 bits.

/// A byte that must be 7-bit had its high bit set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("byte {0:#04x} is not 7-bit")]
pub struct NotSevenBit(pub u8);

/// Packed length of `n` data bytes.
pub fn packed_len(n: usize) -> usize {
    n + n.div_ceil(7)
}

/// Pack 8-bit data into 7-bit-safe SysEx payload bytes.
pub fn pack(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(packed_len(data.len()));
    for group in data.chunks(7) {
        let msbs = group
            .iter()
            .enumerate()
            .fold(0u8, |m, (i, b)| m | ((b >> 7) << i));
        out.push(msbs);
        out.extend(group.iter().map(|b| b & 0x7F));
    }
    out
}

/// Unpack 7-bit SysEx payload bytes into 8-bit data.
pub fn unpack(data: &[u8]) -> Result<Vec<u8>, NotSevenBit> {
    if let Some(&b) = data.iter().find(|&&b| b >= 0x80) {
        return Err(NotSevenBit(b));
    }
    let mut out = Vec::with_capacity(data.len());
    for group in data.chunks(8) {
        let (msbs, rest) = group.split_first().expect("chunks are non-empty");
        out.extend(
            rest.iter()
                .enumerate()
                .map(|(i, b)| b | (((msbs >> i) & 1) << 7)),
        );
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packed_lengths_match_device_sizes() {
        assert_eq!(packed_len(1024), 1171); // program
        assert_eq!(packed_len(80), 92); // global
    }

    #[test]
    fn known_vector() {
        assert_eq!(pack(&[0x80, 0x01]), [0x01, 0x00, 0x01]);
    }

    #[test]
    fn round_trip() {
        for n in [0, 1, 7, 8, 1024] {
            let data: Vec<u8> = (0..n).map(|i| (i * 37 + 11) as u8).collect();
            let packed = pack(&data);
            assert_eq!(packed.len(), packed_len(n));
            assert!(packed.iter().all(|&b| b < 0x80));
            assert_eq!(unpack(&packed).unwrap(), data);
        }
    }

    #[test]
    fn rejects_eight_bit_input() {
        assert_eq!(unpack(&[0x00, 0x80]), Err(NotSevenBit(0x80)));
    }
}
