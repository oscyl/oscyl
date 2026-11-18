// ANCHOR: messages
//! The Tutorial Mono's SysEx messages.
//!
//! Every message starts with `F0 7D 54`: `7D` is the manufacturer ID that
//! MIDI keeps for non-commercial use, and `54` is the model. The synth
//! answers a dump with an acknowledgement after it stores the dump.

use crate::program;

/// The number of program slots: 0 to 15.
pub const SLOTS: u16 = 16;

const HEADER: [u8; 3] = [0xF0, 0x7D, 0x54];
const PROGRAM_REQUEST: u8 = 0x01;
const PROGRAM_DUMP: u8 = 0x02;
const EDIT_REQUEST: u8 = 0x03;
const EDIT_DUMP: u8 = 0x04;
const ACK: u8 = 0x7F;
/// The slot byte of an acknowledgement for the edit buffer.
const EDIT_BUFFER: u8 = 0x7F;

/// One message to or from the synth.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Msg {
    /// Asks for the program in a slot.
    ProgramRequest(u16),
    /// The program in a slot: to store it, or as an answer.
    ProgramDump(u16, Vec<u8>),
    /// Asks for the edit buffer: the program the synth plays now.
    EditRequest,
    /// The edit buffer: to play it, or as an answer.
    EditDump(Vec<u8>),
    /// The synth stored a dump: in slot `Some(n)`, or in the edit buffer.
    Ack(Option<u16>),
}
// ANCHOR_END: messages

// ANCHOR: encode
impl Msg {
    /// The message as bytes, from `F0` to `F7`.
    pub fn encode(&self) -> Vec<u8> {
        let body = match self {
            Msg::ProgramRequest(n) => vec![PROGRAM_REQUEST, slot_byte(*n)],
            Msg::ProgramDump(n, p) => [&[PROGRAM_DUMP, slot_byte(*n)][..], p].concat(),
            Msg::EditRequest => vec![EDIT_REQUEST],
            Msg::EditDump(p) => [&[EDIT_DUMP][..], p].concat(),
            Msg::Ack(Some(n)) => vec![ACK, slot_byte(*n)],
            Msg::Ack(None) => vec![ACK, EDIT_BUFFER],
        };
        [&HEADER[..], &body, &[0xF7]].concat()
    }

    /// The message in `bytes`, or `None` when it is not one of ours.
    pub fn decode(bytes: &[u8]) -> Option<Msg> {
        let body = bytes.strip_prefix(&HEADER[..])?.strip_suffix(&[0xF7])?;
        let slot = |b: u8| (u16::from(b) < SLOTS).then_some(u16::from(b));
        let program = |p: &[u8]| (p.len() == program::LEN).then(|| p.to_vec());
        match body {
            [PROGRAM_REQUEST, n] => Some(Msg::ProgramRequest(slot(*n)?)),
            [PROGRAM_DUMP, n, p @ ..] => Some(Msg::ProgramDump(slot(*n)?, program(p)?)),
            [EDIT_REQUEST] => Some(Msg::EditRequest),
            [EDIT_DUMP, p @ ..] => Some(Msg::EditDump(program(p)?)),
            [ACK, EDIT_BUFFER] => Some(Msg::Ack(None)),
            [ACK, n] => Some(Msg::Ack(Some(slot(*n)?))),
            _ => None,
        }
    }
}

/// A slot as one 7-bit byte. Callers keep `n` below [`SLOTS`].
fn slot_byte(n: u16) -> u8 {
    (n & 0x7F) as u8
}
// ANCHOR_END: encode

// ANCHOR: identity
/// The start of the synth's answer to the MIDI device inquiry, after the
/// channel byte: manufacturer `7D`, family `54 4D`, member `01 00`. Four
/// version bytes and `F7` follow.
const IDENTITY: [u8; 7] = [0x06, 0x02, 0x7D, 0x54, 0x4D, 0x01, 0x00];

/// The synth's answer to the device inquiry, from channel `channel`.
pub fn identity_reply(channel: u8) -> Vec<u8> {
    [&[0xF0, 0x7E, channel][..], &IDENTITY, &[1, 0, 0, 0, 0xF7]].concat()
}

/// Whether `bytes` is the Tutorial Mono's answer to the device inquiry,
/// from any channel.
pub fn is_identity(bytes: &[u8]) -> bool {
    bytes.len() == 15 && bytes[..2] == [0xF0, 0x7E] && bytes[3..10] == IDENTITY
}
// ANCHOR_END: identity

// ANCHOR: tests
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn messages_round_trip() {
        let p = program::init();
        for m in [
            Msg::ProgramRequest(15),
            Msg::ProgramDump(3, p.clone()),
            Msg::EditRequest,
            Msg::EditDump(p),
            Msg::Ack(Some(0)),
            Msg::Ack(None),
        ] {
            assert_eq!(Msg::decode(&m.encode()), Some(m));
        }
        assert_eq!(
            Msg::ProgramRequest(2).encode(),
            [0xF0, 0x7D, 0x54, 0x01, 0x02, 0xF7]
        );
        assert!(is_identity(&identity_reply(0x7F)));
    }
}
// ANCHOR_END: tests
