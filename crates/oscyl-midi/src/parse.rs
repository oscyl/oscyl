//! Complete MIDI messages from a raw input byte stream: SysEx (`F0 … F7`)
//! and channel messages (`8n`–`En` with their data, running status
//! included). Realtime bytes (`F8`–`FF`) are dropped wherever they fall;
//! other system messages are skipped.

use oscyl_driver::sysex::Framer;

#[derive(Debug, Default)]
pub struct Parser {
    sysex: Framer,
    in_sysex: bool,
    /// The status of the channel message being read, kept for running
    /// status; `None` after a system message.
    status: Option<u8>,
    data: Vec<u8>,
}

/// Data bytes after a channel status byte.
fn data_len(status: u8) -> usize {
    match status & 0xF0 {
        0xC0 | 0xD0 => 1,
        _ => 2,
    }
}

impl Parser {
    /// Feed raw bytes; returns every message completed by them, in order.
    pub fn push(&mut self, bytes: &[u8]) -> Vec<Vec<u8>> {
        let mut done = Vec::new();
        for &b in bytes {
            match b {
                0xF8.. => {}
                0xF0 => {
                    self.in_sysex = true;
                    self.status = None;
                    done.extend(self.sysex.push(&[b]));
                }
                0xF7 if self.in_sysex => {
                    self.in_sysex = false;
                    done.extend(self.sysex.push(&[b]));
                }
                0x80..=0xEF => {
                    // A status byte ends an unterminated SysEx.
                    if self.in_sysex {
                        self.in_sysex = false;
                        self.sysex.push(&[b]);
                    }
                    self.status = Some(b);
                    self.data.clear();
                }
                // System common: no running status after it.
                0xF1..=0xF7 => {
                    self.status = None;
                    self.data.clear();
                }
                _ if self.in_sysex => done.extend(self.sysex.push(&[b])),
                _ => {
                    let Some(status) = self.status else { continue };
                    self.data.push(b);
                    if self.data.len() == data_len(status) {
                        let mut m = vec![status];
                        m.append(&mut self.data);
                        done.push(m);
                    }
                }
            }
        }
        done
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn channel_and_sysex_messages_in_order() {
        let mut p = Parser::default();
        let got = p.push(&[0xB0, 0x00, 0x00, 0xF0, 0x42, 0xF8, 0x01, 0xF7, 0xC0, 0x05]);
        assert_eq!(
            got,
            vec![
                vec![0xB0, 0x00, 0x00],
                vec![0xF0, 0x42, 0x01, 0xF7],
                vec![0xC0, 0x05]
            ]
        );
    }

    #[test]
    fn running_status_and_split_chunks() {
        let mut p = Parser::default();
        assert_eq!(p.push(&[0xB0, 0x00]), Vec::<Vec<u8>>::new());
        assert_eq!(
            p.push(&[0x00, 0x20, 0x02, 0xF8, 0xC3]),
            vec![vec![0xB0, 0x00, 0x00], vec![0xB0, 0x20, 0x02]]
        );
        assert_eq!(
            p.push(&[0x07, 0x08]),
            vec![vec![0xC3, 0x07], vec![0xC3, 0x08]]
        );
    }

    #[test]
    fn data_without_a_status_is_skipped() {
        let mut p = Parser::default();
        assert!(p.push(&[0x05, 0x06, 0xF6, 0x07]).is_empty());
        assert_eq!(p.push(&[0x90, 0x40, 0x7F]), vec![vec![0x90, 0x40, 0x7F]]);
    }

    #[test]
    fn a_status_byte_aborts_an_unterminated_sysex() {
        let mut p = Parser::default();
        assert_eq!(
            p.push(&[0xF0, 0x42, 0xC0, 0x05, 0xF7]),
            vec![vec![0xC0, 0x05]]
        );
        assert_eq!(p.push(&[0xF0, 0x01, 0xF7]), vec![vec![0xF0, 0x01, 0xF7]]);
    }
}
