//! Prophet Rev2 SysEx messages.

use oscyl_driver::pack;

use crate::{Error, Program};

/// A message from the synth.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reply {
    /// Program dump (`02`) of a slot.
    Program(u16, Program),
    /// Edit buffer dump (`03`).
    Edit(Program),
    /// Device inquiry reply from a Rev2.
    Identity,
}

/// Programs per bank; slot = bank * 128 + program.
pub const BANK_LEN: u16 = 128;
/// Banks U1-U4 then F1-F4.
pub const SLOTS: u16 = 8 * BANK_LEN;

/// `01` = DSI/Sequential, `2F` = Rev2.
const HEADER: [u8; 3] = [0xF0, 0x01, 0x2F];
const PROGRAM_DUMP: u8 = 0x02;
const EDIT_DUMP: u8 = 0x03;
const PROGRAM_REQUEST: u8 = 0x05;
const EDIT_REQUEST: u8 = 0x06;
/// Device inquiry reply up to the OS version: universal non-realtime,
/// any channel, `06 02`, DSI, family `2F 01`, member `00 00`.
const IDENTITY: [u8; 10] = [0xF0, 0x7E, 0, 0x06, 0x02, 0x01, 0x2F, 0x01, 0x00, 0x00];

/// Bank and program bytes. Callers keep `slot` below [`SLOTS`].
fn bank_program(slot: u16) -> [u8; 2] {
    [(slot / BANK_LEN) as u8, (slot % BANK_LEN) as u8]
}

fn message(func: u8, args: &[u8], payload: &[u8]) -> Vec<u8> {
    [&HEADER[..], &[func], args, payload, &[0xF7]].concat()
}

pub fn encode_program_request(slot: u16) -> Vec<u8> {
    message(PROGRAM_REQUEST, &bank_program(slot), &[])
}

pub fn encode_edit_request() -> Vec<u8> {
    message(EDIT_REQUEST, &[], &[])
}

pub fn encode_program_dump(slot: u16, p: &Program) -> Vec<u8> {
    message(PROGRAM_DUMP, &bank_program(slot), &pack::pack(p.as_bytes()))
}

pub fn encode_edit_dump(p: &Program) -> Vec<u8> {
    message(EDIT_DUMP, &[], &pack::pack(p.as_bytes()))
}

fn malformed(what: &str) -> Error {
    Error(format!("not a Prophet Rev2 message: {what}"))
}

fn program(packed: &[u8]) -> Result<Program, Error> {
    let bytes = pack::unpack(packed).map_err(|e| malformed(&e.to_string()))?;
    Program::from_bytes(&bytes)
}

/// Decode one complete SysEx message from the synth.
pub fn decode(msg: &[u8]) -> Result<Reply, Error> {
    let body = msg
        .strip_suffix(&[0xF7])
        .ok_or_else(|| malformed("no F7"))?;
    if body.len() >= IDENTITY.len()
        && body[..2] == IDENTITY[..2]
        && body[3..IDENTITY.len()] == IDENTITY[3..]
    {
        // Any number of version bytes may follow.
        return Ok(Reply::Identity);
    }
    let rest = body
        .strip_prefix(&HEADER[..])
        .ok_or_else(|| malformed("header"))?;
    match rest {
        [PROGRAM_DUMP, bank, prog, payload @ ..] if *bank < 8 && *prog < 0x80 => {
            Ok(Reply::Program(
                u16::from(*bank) * BANK_LEN + u16::from(*prog),
                program(payload)?,
            ))
        }
        [EDIT_DUMP, payload @ ..] => Ok(Reply::Edit(program(payload)?)),
        _ => Err(malformed("unknown function")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Program {
        let bytes: Vec<u8> = (0..crate::program::PROGRAM_LEN)
            .map(|i| (i * 37 + 11) as u8)
            .collect();
        Program::from_bytes(&bytes).unwrap()
    }

    #[test]
    fn request_bytes_match_protocol_doc() {
        assert_eq!(
            encode_program_request(0),
            [0xF0, 0x01, 0x2F, 0x05, 0x00, 0x00, 0xF7]
        );
        assert_eq!(
            encode_program_request(130),
            [0xF0, 0x01, 0x2F, 0x05, 0x01, 0x02, 0xF7]
        );
        assert_eq!(
            encode_program_request(1023),
            [0xF0, 0x01, 0x2F, 0x05, 0x07, 0x7F, 0xF7]
        );
        assert_eq!(encode_edit_request(), [0xF0, 0x01, 0x2F, 0x06, 0xF7]);
    }

    #[test]
    fn packed_dump_length_is_2346() {
        let d = encode_program_dump(517, &sample());
        assert_eq!(d.len(), 2346);
        assert_eq!(d[..6], [0xF0, 0x01, 0x2F, 0x02, 0x04, 0x05]);
        assert_eq!(d.last(), Some(&0xF7));
        assert!(d[1..d.len() - 1].iter().all(|&b| b < 0x80));
        let e = encode_edit_dump(&sample());
        assert_eq!(e.len(), 2344);
        assert_eq!(e[..4], [0xF0, 0x01, 0x2F, 0x03]);
    }

    #[test]
    fn dump_round_trip() {
        let p = sample();
        assert_eq!(
            decode(&encode_program_dump(1023, &p)),
            Ok(Reply::Program(1023, p))
        );
        assert_eq!(decode(&encode_edit_dump(&p)), Ok(Reply::Edit(p)));
        let identity = [
            0xF0, 0x7E, 0x7F, 0x06, 0x02, 0x01, 0x2F, 0x01, 0x00, 0x00, 0x12, 0xF7,
        ];
        assert_eq!(decode(&identity), Ok(Reply::Identity));
        let mut short = encode_program_dump(3, &p);
        short.remove(10);
        assert!(decode(&short).is_err());
        let mut other = encode_program_dump(3, &p);
        other[2] = 0x2D;
        assert!(decode(&other).is_err());
        assert!(decode(&encode_program_request(3)).is_err());
    }
}
