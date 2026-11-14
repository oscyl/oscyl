//! minilogue xd SysEx messages: encoding requests and dumps, decoding replies.
//!
//! Byte layouts follow KORG's "minilogue xd MIDI Implementation" and the
//! captures taken from the hardware. The encoding is
//! the logue family's ([`oscyl_logue::msg`]) with the xd's [`FAMILY`].

use crate::Error;
#[cfg(test)]
use crate::pack::pack;
use oscyl_logue::msg::Family;

pub use oscyl_logue::msg::{
    Dump, Module, OCTAVE_LEN, PROGRAM_COUNT, Reply, Request, SCALE_LEN, SLOT_STATUS_LEN, Version,
};

/// Unpacked sizes of fixed-size dumps.
pub const PROGRAM_LEN: usize = 1024;
pub const GLOBAL_LEN: usize = 80;

/// The xd's family id (`51 01`) and dump sizes.
pub const FAMILY: Family = Family {
    id: 0x51,
    program_len: PROGRAM_LEN,
    global_len: GLOBAL_LEN,
    foreign: "not a minilogue xd message",
};

#[cfg(test)]
fn header(ch: u8, func: u8) -> Vec<u8> {
    oscyl_logue::msg::header(&FAMILY, ch, func)
}

#[cfg(test)]
fn finish(mut m: Vec<u8>) -> Vec<u8> {
    m.push(0xF7);
    m
}

pub fn encode_request(ch: u8, r: &Request) -> Vec<u8> {
    oscyl_logue::msg::encode_request(&FAMILY, ch, r)
}

pub fn encode_dump(ch: u8, d: &Dump) -> Vec<u8> {
    oscyl_logue::msg::encode_dump(&FAMILY, ch, d)
}

/// Decode one complete SysEx message from the synth.
pub fn decode(msg: &[u8]) -> Result<Reply, Error> {
    Ok(oscyl_logue::msg::decode(&FAMILY, msg)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    const IDENTITY: [u8; 15] = [
        0xF0, 0x7E, 0x00, 0x06, 0x02, 0x42, 0x51, 0x01, 0x00, 0x00, 0x02, 0x00, 0x0A, 0x00, 0xF7,
    ];
    const SEARCH: [u8; 15] = [
        0xF0, 0x42, 0x50, 0x01, 0x00, 0x00, 0x51, 0x01, 0x00, 0x00, 0x02, 0x00, 0x0A, 0x00, 0xF7,
    ];
    const FW_2_10: Version = Version {
        major: 2,
        minor: 10,
    };

    #[test]
    fn program_request_bytes() {
        assert_eq!(
            encode_request(0, &Request::Program(300)),
            [0xF0, 0x42, 0x30, 0x00, 0x01, 0x51, 0x1C, 0x2C, 0x02, 0xF7]
        );
    }

    #[test]
    fn program_dump_length() {
        let m = encode_dump(0, &Dump::Program(0, vec![0; PROGRAM_LEN]));
        assert_eq!(m.len(), 1181);
        let m = encode_dump(0, &Dump::CurrentProgram(vec![0; PROGRAM_LEN]));
        assert_eq!(m.len(), 1179);
    }

    #[test]
    fn dumps_round_trip_on_channel_5() {
        let bytes = |n: usize| (0..n).map(|i| (i * 31) as u8).collect::<Vec<u8>>();
        let sevenbit = |n: usize| (0..n).map(|i| (i * 31) as u8 & 0x7F).collect::<Vec<u8>>();
        for d in [
            Dump::CurrentProgram(bytes(PROGRAM_LEN)),
            Dump::Program(499, bytes(PROGRAM_LEN)),
            Dump::Global(bytes(GLOBAL_LEN)),
            Dump::Scale(0x7F, sevenbit(SCALE_LEN)),
            Dump::Octave(5, sevenbit(OCTAVE_LEN)),
        ] {
            let m = encode_dump(5, &d);
            assert_eq!(m[2], 0x35);
            assert_eq!(decode(&m), Ok(Reply::Dump(d)));
        }
    }

    #[test]
    fn slot_data_reply_skips_extra_byte() {
        let data: Vec<u8> = (0..20).map(|i| i * 13).collect();
        let mut m = header(0, 0x4A);
        m.extend([0x04, 0x08, 0x00]);
        m.extend(pack(&data));
        assert_eq!(
            decode(&finish(m)),
            Ok(Reply::Dump(Dump::SlotData {
                module: Module::Osc,
                slot: 8,
                data
            }))
        );
    }

    #[test]
    fn status_replies() {
        let st = |f| [0xF0, 0x42, 0x35, 0x00, 0x01, 0x51, f, 0xF7];
        assert_eq!(decode(&st(0x23)), Ok(Reply::Ack));
        assert_eq!(decode(&st(0x24)), Ok(Reply::Nak));
        assert_eq!(decode(&st(0x27)), Ok(Reply::Error(0x27)));
    }

    #[test]
    fn identity_and_search_from_hardware() {
        assert_eq!(
            decode(&IDENTITY),
            Ok(Reply::Identity {
                channel: 0,
                version: FW_2_10
            })
        );
        assert_eq!(
            decode(&SEARCH),
            Ok(Reply::Search {
                channel: 0,
                echo: 0,
                version: FW_2_10
            })
        );
    }

    #[test]
    fn slot_status_empty_and_occupied() {
        let empty = [0xF0, 0x42, 0x30, 0x00, 0x01, 0x51, 0x49, 0x04, 0x09, 0xF7];
        assert_eq!(
            decode(&empty),
            Ok(Reply::SlotStatus {
                module: Module::Osc,
                slot: 9,
                header: None
            })
        );
        let mut rec = vec![0u8; SLOT_STATUS_LEN];
        rec[..2].copy_from_slice(&[0x04, 0x02]);
        rec[18..23].copy_from_slice(b"waves");
        let mut m = header(0, 0x49);
        m.extend([0x04, 0x08, 0x00]);
        m.extend(pack(&rec));
        let m = finish(m);
        assert_eq!(m.len(), 48);
        assert_eq!(
            decode(&m),
            Ok(Reply::SlotStatus {
                module: Module::Osc,
                slot: 8,
                header: Some(rec)
            })
        );
    }

    #[test]
    fn rejects_wrong_length_program() {
        let mut m = header(0, 0x40);
        m.extend(pack(&[0; 10]));
        assert!(matches!(decode(&finish(m)), Err(Error::Length { .. })));
    }
}
