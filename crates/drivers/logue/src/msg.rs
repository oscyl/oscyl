//! logue-family SysEx messages: encoding requests and dumps, decoding
//! replies. One implementation for every model; a [`Family`] carries what
//! differs (family id, dump sizes).
//!
//! Byte layouts follow KORG's MIDI Implementations for the minilogue xd and
//! the prologue, and captures taken from an xd.

use crate::Error;
use oscyl_driver::pack::{pack, unpack};

const KORG: u8 = 0x42;

/// Unpacked sizes of the dumps every model shares.
pub const SCALE_LEN: usize = 384;
pub const OCTAVE_LEN: usize = 36;
pub const SLOT_STATUS_LEN: usize = 32;

/// Number of program memories (both the xd and the prologue).
pub const PROGRAM_COUNT: u16 = 500;

/// What tells one logue model's messages from another's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Family {
    /// The family id's low byte, after `00 01` in the header (xd `0x51`,
    /// prologue `0x4B`); the high byte is `01` for both.
    pub id: u8,
    /// Unpacked program size.
    pub program_len: usize,
    /// Unpacked global dump size.
    pub global_len: usize,
    /// The error text for a message from another model.
    pub foreign: &'static str,
}

/// logue SDK user module.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Module {
    ModFx = 1,
    DelFx = 2,
    RevFx = 3,
    Osc = 4,
}

impl Module {
    pub const ALL: [Module; 4] = [Module::Osc, Module::ModFx, Module::DelFx, Module::RevFx];

    /// Short lowercase name, as the CLI spells it and backup files use it.
    pub fn name(self) -> &'static str {
        match self {
            Module::Osc => "osc",
            Module::ModFx => "modfx",
            Module::DelFx => "delfx",
            Module::RevFx => "revfx",
        }
    }

    /// Number of user slots for this module (the same on xd and prologue).
    pub fn slots(self) -> u8 {
        match self {
            Module::ModFx | Module::Osc => 16,
            Module::DelFx | Module::RevFx => 8,
        }
    }

    pub fn from_id(id: u8) -> Result<Module, Error> {
        match id {
            1 => Ok(Module::ModFx),
            2 => Ok(Module::DelFx),
            3 => Ok(Module::RevFx),
            4 => Ok(Module::Osc),
            _ => Err(Error::Malformed("unknown user module id")),
        }
    }
}

/// Messages that ask the synth for data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Request {
    /// Universal Device Inquiry.
    Identity,
    /// KORG Search Device, with an echo id.
    Search(u8),
    Global,
    CurrentProgram,
    Program(u16),
    Scale(u8),
    Octave(u8),
    SlotStatus {
        module: Module,
        slot: u8,
    },
    SlotData {
        module: Module,
        slot: u8,
    },
    ClearSlot {
        module: Module,
        slot: u8,
    },
    SwapSlots {
        module: Module,
        a: u8,
        b: u8,
    },
}

/// Data messages, sent by the synth in reply or by us to store data.
/// Payloads are unpacked (8-bit).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Dump {
    CurrentProgram(Vec<u8>),
    Program(u16, Vec<u8>),
    Global(Vec<u8>),
    /// Scale index 0–5, or 0x7F for the edit buffer.
    Scale(u8, Vec<u8>),
    /// Octave index 0–5, or 0x7F for the edit buffer.
    Octave(u8, Vec<u8>),
    /// TABLE 7 slot data: `u32 size, u32 crc32, payload`.
    SlotData {
        module: Module,
        slot: u8,
        data: Vec<u8>,
    },
}

/// Firmware version reported by the device.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Version {
    pub major: u16,
    pub minor: u16,
}

/// Anything the synth sends that we understand.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reply {
    Identity {
        channel: u8,
        version: Version,
    },
    /// The channel is the low nibble of byte 4. The prologue sets bit 4
    /// there when its SysEx filter is off; it is masked out.
    Search {
        channel: u8,
        echo: u8,
        version: Version,
    },
    Dump(Dump),
    /// `None` for an empty slot, else the 32-byte TABLE 6 record.
    SlotStatus {
        module: Module,
        slot: u8,
        header: Option<Vec<u8>>,
    },
    Ack,
    /// 0x24: data load error, also sent when the synth is busy.
    Nak,
    /// 0x26–0x2F: specific error status (see MIDI Implementation NOTE 2).
    Error(u8),
}

/// `F0 42 3g 00 01 <family> <func>`.
pub fn header(f: &Family, ch: u8, func: u8) -> Vec<u8> {
    vec![0xF0, KORG, 0x30 | (ch & 0x0F), 0x00, 0x01, f.id, func]
}

fn finish(mut m: Vec<u8>) -> Vec<u8> {
    m.push(0xF7);
    m
}

fn prog_index(n: u16) -> [u8; 2] {
    [(n & 0x7F) as u8, (n >> 7) as u8]
}

pub fn encode_request(f: &Family, ch: u8, r: &Request) -> Vec<u8> {
    use Request::*;
    let header = |func| header(f, ch, func);
    match *r {
        Identity => oscyl_driver::sysex::DEVICE_INQUIRY.to_vec(),
        Search(echo) => vec![0xF0, KORG, 0x50, 0x00, echo & 0x7F, 0xF7],
        Global => finish(header(0x0E)),
        CurrentProgram => finish(header(0x10)),
        Program(n) => {
            let mut m = header(0x1C);
            m.extend(prog_index(n));
            finish(m)
        }
        Scale(s) => finish([header(0x14), vec![s]].concat()),
        Octave(s) => finish([header(0x15), vec![s]].concat()),
        SlotStatus { module, slot } => finish([header(0x19), vec![module as u8, slot]].concat()),
        SlotData { module, slot } => finish([header(0x1A), vec![module as u8, slot]].concat()),
        ClearSlot { module, slot } => finish([header(0x1B), vec![module as u8, slot]].concat()),
        SwapSlots { module, a, b } => finish([header(0x1E), vec![module as u8, a, b]].concat()),
    }
}

pub fn encode_dump(f: &Family, ch: u8, d: &Dump) -> Vec<u8> {
    use Dump::*;
    let header = |func| header(f, ch, func);
    let (mut m, data, packed) = match d {
        CurrentProgram(p) => (header(0x40), p, true),
        Program(n, p) => ([header(0x4C), prog_index(*n).to_vec()].concat(), p, true),
        Global(g) => (header(0x51), g, true),
        // Scale and octave data are sent as raw 7-bit bytes, not packed.
        Scale(s, v) => ([header(0x44), vec![*s]].concat(), v, false),
        Octave(s, v) => ([header(0x45), vec![*s]].concat(), v, false),
        SlotData { module, slot, data } => (
            [header(0x4A), vec![*module as u8, *slot]].concat(),
            data,
            true,
        ),
    };
    if packed {
        m.extend(pack(data));
    } else {
        m.extend(data.iter().map(|b| b & 0x7F));
    }
    finish(m)
}

fn version(b: &[u8]) -> Version {
    // KORG's tables label bytes 10-11 "minor" and 12-13 "major", but the xd
    // on firmware 2.10 sends `02 00 0A 00`, so the order is major then minor.
    Version {
        major: b[0] as u16 | (b[1] as u16) << 7,
        minor: b[2] as u16 | (b[3] as u16) << 7,
    }
}

fn exact(data: Vec<u8>, len: usize) -> Result<Vec<u8>, Error> {
    if data.len() == len {
        Ok(data)
    } else {
        Err(Error::Length {
            expected: len,
            got: data.len(),
        })
    }
}

/// Decode one complete SysEx message from a synth of family `f`.
pub fn decode(f: &Family, msg: &[u8]) -> Result<Reply, Error> {
    let body = match msg {
        [0xF0, body @ .., 0xF7] => body,
        _ => return Err(Error::Malformed("not a SysEx message")),
    };
    let id = f.id;
    match body {
        [0x7E, ch, 0x06, 0x02, KORG, fam, 0x01, _, _, v @ ..] if *fam == id && v.len() == 4 => {
            Ok(Reply::Identity {
                channel: *ch & 0x0F,
                version: version(v),
            })
        }
        [KORG, 0x50, 0x01, ch, echo, fam, 0x01, _, _, v @ ..] if *fam == id && v.len() == 4 => {
            Ok(Reply::Search {
                channel: *ch & 0x0F,
                echo: *echo,
                version: version(v),
            })
        }
        [KORG, g, 0x00, 0x01, fam, func, rest @ ..] if *fam == id && g & 0xF0 == 0x30 => {
            decode_func(f, *func, rest)
        }
        _ => Err(Error::Malformed(f.foreign)),
    }
}

fn decode_func(f: &Family, func: u8, rest: &[u8]) -> Result<Reply, Error> {
    use Dump::*;
    let d = match (func, rest) {
        (0x23, []) => return Ok(Reply::Ack),
        (0x24, []) => return Ok(Reply::Nak),
        (0x26..=0x2F, []) => return Ok(Reply::Error(func)),
        (0x40, p) => CurrentProgram(exact(unpack(p)?, f.program_len)?),
        (0x4C, [lo, hi, p @ ..]) => Program(
            *lo as u16 | (*hi as u16) << 7,
            exact(unpack(p)?, f.program_len)?,
        ),
        (0x51, p) => Global(exact(unpack(p)?, f.global_len)?),
        (0x44, [s, v @ ..]) => Scale(*s, exact(v.to_vec(), SCALE_LEN)?),
        (0x45, [s, v @ ..]) => Octave(*s, exact(v.to_vec(), OCTAVE_LEN)?),
        // Like 0x49, the synth puts one extra byte (0 so far) before the
        // packed data. Uploads from the official Librarian do not.
        (0x4A, [m, s, _, p @ ..]) => SlotData {
            module: Module::from_id(*m)?,
            slot: *s,
            data: unpack(p)?,
        },
        (0x49, [m, s]) => {
            return Ok(Reply::SlotStatus {
                module: Module::from_id(*m)?,
                slot: *s,
                header: None,
            });
        }
        // Occupied: one extra byte (always 0 so far), then 37 packed bytes.
        (0x49, [m, s, _, p @ ..]) => {
            return Ok(Reply::SlotStatus {
                module: Module::from_id(*m)?,
                slot: *s,
                header: Some(exact(unpack(p)?, SLOT_STATUS_LEN)?),
            });
        }
        _ => return Err(Error::Malformed("unknown or truncated function")),
    };
    Ok(Reply::Dump(d))
}

#[cfg(test)]
mod tests {
    use super::*;

    const XD: Family = Family {
        id: 0x51,
        program_len: 1024,
        global_len: 80,
        foreign: "not a minilogue xd message",
    };
    const PROLOGUE: Family = Family {
        id: 0x4B,
        program_len: 336,
        global_len: 32,
        foreign: "not a prologue message",
    };

    fn identity(fam: u8) -> [u8; 15] {
        [
            0xF0, 0x7E, 0x00, 0x06, 0x02, 0x42, fam, 0x01, 0x00, 0x00, 0x02, 0x00, 0x0A, 0x00, 0xF7,
        ]
    }

    #[test]
    fn family_parametrises_header_and_identity() {
        assert_eq!(
            encode_request(&PROLOGUE, 0, &Request::Program(300)),
            [0xF0, 0x42, 0x30, 0x00, 0x01, 0x4B, 0x1C, 0x2C, 0x02, 0xF7]
        );
        let fw = Version {
            major: 2,
            minor: 10,
        };
        let want = Ok(Reply::Identity {
            channel: 0,
            version: fw,
        });
        assert_eq!(decode(&PROLOGUE, &identity(0x4B)), want);
        assert_eq!(decode(&XD, &identity(0x51)), want);
        assert_eq!(
            decode(&XD, &identity(0x4B)),
            Err(Error::Malformed("not a minilogue xd message"))
        );
        assert!(decode(&PROLOGUE, &identity(0x51)).is_err());
        let xd_ack = [0xF0, 0x42, 0x30, 0x00, 0x01, 0x51, 0x23, 0xF7];
        assert!(decode(&PROLOGUE, &xd_ack).is_err());
    }

    #[test]
    fn prologue_dump_sizes() {
        let p = encode_dump(&PROLOGUE, 0, &Dump::Program(0, vec![0; 336]));
        assert_eq!(p.len(), 394);
        let c = encode_dump(&PROLOGUE, 0, &Dump::CurrentProgram(vec![0; 336]));
        assert_eq!(c.len(), 392);
        let g = encode_dump(&PROLOGUE, 0, &Dump::Global(vec![0; 32]));
        assert_eq!(g.len(), 45);
        assert_eq!(
            decode(&PROLOGUE, &p),
            Ok(Reply::Dump(Dump::Program(0, vec![0; 336])))
        );
        let xd_sized = encode_dump(&PROLOGUE, 0, &Dump::Program(0, vec![0; 1024]));
        assert!(matches!(
            decode(&PROLOGUE, &xd_sized),
            Err(Error::Length { expected: 336, .. })
        ));
    }

    #[test]
    fn search_reply_masks_the_filter_flag() {
        let m = [
            0xF0, 0x42, 0x50, 0x01, 0x13, 0x07, 0x4B, 0x01, 0x00, 0x00, 0x02, 0x00, 0x0A, 0x00,
            0xF7,
        ];
        assert!(matches!(
            decode(&PROLOGUE, &m),
            Ok(Reply::Search {
                channel: 3,
                echo: 7,
                ..
            })
        ));
    }
}
