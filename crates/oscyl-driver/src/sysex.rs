//! Reassembles complete SysEx messages from a raw MIDI byte stream.

/// Universal device inquiry, all channels. Synths answer with an identity
/// reply that [`crate::Driver::recognize`] classifies.
pub const DEVICE_INQUIRY: [u8; 6] = [0xF0, 0x7E, 0x7F, 0x06, 0x01, 0xF7];

/// Collects bytes until a full `F0 … F7` message is available.
///
/// Realtime bytes (`F8`–`FF`, such as clock and active sensing) may legally
/// appear inside a SysEx message and are dropped. Bytes outside a message are
/// ignored.
#[derive(Debug, Default)]
pub struct Framer {
    buf: Vec<u8>,
    in_sysex: bool,
}

impl Framer {
    /// A framer outside any message.
    pub fn new() -> Self {
        Self::default()
    }

    /// Feed raw bytes; returns every message completed by them.
    pub fn push(&mut self, bytes: &[u8]) -> Vec<Vec<u8>> {
        let mut done = Vec::new();
        for &b in bytes {
            match b {
                0xF8.. => {}
                0xF0 => {
                    self.buf.clear();
                    self.buf.push(b);
                    self.in_sysex = true;
                }
                0xF7 if self.in_sysex => {
                    self.buf.push(b);
                    done.push(std::mem::take(&mut self.buf));
                    self.in_sysex = false;
                }
                // Any other status byte aborts an unterminated message.
                0x80.. => {
                    self.buf.clear();
                    self.in_sysex = false;
                }
                _ if self.in_sysex => self.buf.push(b),
                _ => {}
            }
        }
        done
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drops_realtime_bytes_inside_sysex() {
        let mut f = Framer::new();
        let out = f.push(&[0xF0, 0x42, 0xF8, 0x30, 0xFE, 0x01, 0xF7]);
        assert_eq!(out, vec![vec![0xF0, 0x42, 0x30, 0x01, 0xF7]]);
    }

    #[test]
    fn reassembles_split_message() {
        let mut f = Framer::new();
        assert!(f.push(&[0x90, 0x40, 0xF0, 0x42]).is_empty());
        assert!(f.push(&[0x30, 0x00]).is_empty());
        assert_eq!(
            f.push(&[0x01, 0xF7, 0xF0]),
            vec![vec![0xF0, 0x42, 0x30, 0x00, 0x01, 0xF7]]
        );
    }

    #[test]
    fn status_byte_aborts_unterminated_message() {
        let mut f = Framer::new();
        assert!(f.push(&[0xF0, 0x42, 0x90, 0x40, 0x7F, 0xF7]).is_empty());
    }
}
