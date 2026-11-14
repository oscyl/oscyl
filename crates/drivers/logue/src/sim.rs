//! A sans-IO simulated logue synth running firmware 2.10.
//!
//! [`Sim::handle`] takes one complete SysEx message and returns the replies
//! the synth would send. Reply bytes follow captures
//! from a real xd (identity and search replies, the extra
//! byte in 0x49 and 0x4A replies), and program writes follow the firmware's
//! store model ([`crate::store::StoreModel`] with the model's group size):
//! writes land in a cache whose deferred flash save only happens on
//! [`Sim::idle_tick`] or when a later write forces it.
//!
//! [`Faults`] injects the failures seen on hardware (xd) or found in the
//! firmware (prologue): the reply-lag fault (a 0x1C read racing the
//! deferred save) and lost replies.

use std::marker::PhantomData;

use oscyl_driver::Simulator;
use oscyl_driver::pack::{pack, unpack};

use crate::Model;
use crate::msg::{Dump, Module, OCTAVE_LEN, PROGRAM_COUNT, Reply, SCALE_LEN, decode, encode_dump};
use crate::store::{Effect, StoreModel};

pub const ACK: u8 = 0x23;
pub const NAK: u8 = 0x24;
pub const DATA_FORMAT_ERROR: u8 = 0x26;
pub const USER_DATA_SIZE_ERROR: u8 = 0x27;
pub const USER_TARGET_ERROR: u8 = 0x29;
pub const USER_MODULE_ERROR: u8 = 0x2C;
pub const USER_SLOT_ERROR: u8 = 0x2D;
/// Member id and firmware 2.10 version bytes of identity/search replies,
/// after the family id.
const ID_TAIL: [u8; 7] = [0x01, 0x00, 0x00, 0x02, 0x00, 0x0A, 0x00];
/// Scale and octave tables: 6 user tables plus the edit buffer (0x7F).
const TUNING_TABLES: usize = 7;
/// Bytes of a slot's status record at the start of its blob.
const SLOT_STATUS_LEN: usize = crate::msg::SLOT_STATUS_LEN;

/// Failures to inject, for tests. All off by default.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Faults {
    /// A 0x1C read that arrives while a deferred flash save is pending races
    /// it, as on hardware: that read gets a corrupt reply (empty on the xd,
    /// possibly a valid program of the flushed group on the prologue) and
    /// every later 0x1C reply carries the previous read's data, until the
    /// next 0x4C write. Off: the read is answered correctly.
    pub race_deferred_save: bool,
    /// Drop every nth reply, counting replies from when this is set.
    pub drop_every_nth_reply: Option<usize>,
}

/// One occupied user slot: TABLE 7 data as stored (`u32 size,
/// u32 checksum, blob`). The 32-byte status record is the blob's start.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Slot(Vec<u8>);

impl Slot {
    fn status(&self) -> &[u8] {
        &self.0[8..8 + SLOT_STATUS_LEN]
    }
}

/// A simulated synth of model `M` (firmware 2.10, MIDI channel 1).
pub struct Sim<M: Model> {
    /// Injected failures; may be changed at any time.
    pub faults: Faults,
    channel: u8,
    programs: Vec<M::Program>,
    edit_buffer: M::Program,
    global: Vec<u8>,
    scales: Vec<Vec<u8>>,
    octaves: Vec<Vec<u8>>,
    /// Indexed by `Module as usize - 1`.
    slots: [Vec<Option<Slot>>; 4],
    store: StoreModel,
    /// In the reply-lag state: 0x1C replies carry `last_read`.
    lagging: bool,
    last_read: Vec<u8>,
    /// Replies counted for `Faults::drop_every_nth_reply`.
    replies: usize,
    model: PhantomData<fn() -> M>,
}

impl<M: Model> Clone for Sim<M> {
    fn clone(&self) -> Self {
        Sim {
            faults: self.faults.clone(),
            channel: self.channel,
            programs: self.programs.clone(),
            edit_buffer: self.edit_buffer.clone(),
            global: self.global.clone(),
            scales: self.scales.clone(),
            octaves: self.octaves.clone(),
            slots: self.slots.clone(),
            store: self.store.clone(),
            lagging: self.lagging,
            last_read: self.last_read.clone(),
            replies: self.replies,
            model: PhantomData,
        }
    }
}

impl<M: Model> std::fmt::Debug for Sim<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Sim")
            .field("model", &M::NAME)
            .field("faults", &self.faults)
            .field("store", &self.store)
            .field("lagging", &self.lagging)
            .finish_non_exhaustive()
    }
}

impl<M: Model> Default for Sim<M> {
    fn default() -> Self {
        Sim::new()
    }
}

impl<M: Model> Sim<M> {
    /// A synth whose 500 programs are all `M::init()`, with empty user
    /// slots and a blank global block.
    pub fn new() -> Sim<M> {
        Sim::with_library(&[])
    }

    /// A synth holding `programs` from program 1 up; the rest are
    /// `M::init()`. Extra programs beyond 500 are ignored.
    pub fn with_library(programs: &[M::Program]) -> Sim<M> {
        let mut all: Vec<M::Program> = programs
            .iter()
            .take(PROGRAM_COUNT as usize)
            .cloned()
            .collect();
        all.resize(PROGRAM_COUNT as usize, M::init());
        let mut global = vec![0u8; M::FAMILY.global_len];
        global[..4].copy_from_slice(b"GLOB");
        Sim {
            faults: Faults::default(),
            channel: 0,
            edit_buffer: all[0].clone(),
            last_read: M::bytes(&all[0]).to_vec(),
            programs: all,
            global,
            scales: vec![vec![0; SCALE_LEN]; TUNING_TABLES],
            octaves: vec![vec![0; OCTAVE_LEN]; TUNING_TABLES],
            // ModFx, DelFx, RevFx, Osc (module id - 1).
            slots: [16, 8, 8, 16].map(|n| vec![None; n]),
            store: StoreModel::clean(M::STORE),
            lagging: false,
            replies: 0,
            model: PhantomData,
        }
    }

    /// The stored program `n` (0-based), if in range.
    pub fn program(&self, n: u16) -> Option<&M::Program> {
        self.programs.get(n as usize)
    }

    /// The edit buffer (last auditioned program).
    pub fn edit_buffer(&self) -> &M::Program {
        &self.edit_buffer
    }

    /// True while a deferred flash save is pending.
    pub fn save_pending(&self) -> bool {
        !self.store.is_clean()
    }

    /// True while every 0x1C reply carries the previous read's data.
    pub fn is_lagging(&self) -> bool {
        self.lagging
    }

    /// The main loop ran idle long enough: the deferred save completes.
    pub fn idle_tick(&mut self) {
        self.store = StoreModel::clean(M::STORE);
    }

    /// Replies to one complete incoming SysEx message. Messages for another
    /// channel or device, and ones the synth does not answer, give none.
    pub fn handle(&mut self, msg: &[u8]) -> Vec<Vec<u8>> {
        let replies = self.respond(msg);
        replies
            .into_iter()
            .filter(|_| {
                let Some(n) = self.faults.drop_every_nth_reply.filter(|&n| n > 0) else {
                    return true;
                };
                self.replies += 1;
                !self.replies.is_multiple_of(n)
            })
            .collect()
    }

    /// Switch on a fault by its [`Simulator`] name.
    pub fn inject(&mut self, fault: &str) {
        match fault {
            "race_deferred_save" => self.faults.race_deferred_save = true,
            "drop_every_3rd_reply" => self.faults.drop_every_nth_reply = Some(3),
            "lose_reply" => self.faults.drop_every_nth_reply = Some(1),
            _ => panic!("unknown fault {fault:?}"),
        }
    }

    fn respond(&mut self, msg: &[u8]) -> Vec<Vec<u8>> {
        let Some(body) = msg
            .strip_prefix(&[0xF0])
            .and_then(|m| m.strip_suffix(&[0xF7]))
        else {
            return vec![];
        };
        let ch = self.channel;
        let fam = M::FAMILY.id;
        match body {
            [0x7E, c, 0x06, 0x01] if *c == ch || *c == 0x7F => {
                vec![
                    [
                        &[0xF0, 0x7E, ch, 0x06, 0x02, 0x42, fam][..],
                        &ID_TAIL,
                        &[0xF7],
                    ]
                    .concat(),
                ]
            }
            [0x42, 0x50, 0x00, echo] => {
                vec![
                    [
                        &[0xF0, 0x42, 0x50, 0x01, ch, *echo, fam][..],
                        &ID_TAIL,
                        &[0xF7],
                    ]
                    .concat(),
                ]
            }
            [0x42, g, 0x00, 0x01, f, func, rest @ ..] if *g == 0x30 | ch && *f == fam => {
                self.function(*func, rest, msg).into_iter().collect()
            }
            _ => vec![],
        }
    }

    fn reply(&self, func: u8, data: &[u8]) -> Vec<u8> {
        [
            &[
                0xF0,
                0x42,
                0x30 | self.channel,
                0x00,
                0x01,
                M::FAMILY.id,
                func,
            ][..],
            data,
            &[0xF7],
        ]
        .concat()
    }

    fn status(&self, code: u8) -> Option<Vec<u8>> {
        Some(self.reply(code, &[]))
    }

    fn dump(&self, d: &Dump) -> Option<Vec<u8>> {
        Some(encode_dump(&M::FAMILY, self.channel, d))
    }

    fn function(&mut self, func: u8, rest: &[u8], msg: &[u8]) -> Option<Vec<u8>> {
        match (func, rest) {
            (0x10, []) => self.dump(&Dump::CurrentProgram(M::bytes(&self.edit_buffer).to_vec())),
            (0x0E, []) => self.dump(&Dump::Global(self.global.clone())),
            (0x1C, [lo, hi]) => self.read_program(*lo as u16 | (*hi as u16) << 7),
            (0x14, [s]) => {
                let t = tuning_index(*s)?;
                self.dump(&Dump::Scale(*s, self.scales[t].clone()))
            }
            (0x15, [s]) => {
                let t = tuning_index(*s)?;
                self.dump(&Dump::Octave(*s, self.octaves[t].clone()))
            }
            (0x19, [m, s]) => match self.slot(*m, *s) {
                Err(code) => self.status(code),
                Ok(None) => Some(self.reply(0x49, &[*m, *s])),
                Ok(Some(slot)) => {
                    let data = [&[*m, *s, 0][..], &pack(slot.status())].concat();
                    Some(self.reply(0x49, &data))
                }
            },
            (0x1A, [m, s]) => match self.slot(*m, *s) {
                Err(code) => self.status(code),
                // Not captured: what the synth sends for an empty slot.
                Ok(None) => self.status(NAK),
                Ok(Some(slot)) => {
                    let data = [&[*m, *s, 0][..], &pack(&slot.0)].concat();
                    Some(self.reply(0x4A, &data))
                }
            },
            (0x1B, [m, s]) => {
                let code = match self.slot_mut(*m, *s) {
                    Ok(slot) => {
                        *slot = None;
                        ACK
                    }
                    Err(code) => code,
                };
                self.status(code)
            }
            (0x1D, [m]) => {
                let code = match Module::from_id(*m) {
                    Ok(module) => {
                        self.slots[module as usize - 1].fill(None);
                        ACK
                    }
                    Err(_) => USER_MODULE_ERROR,
                };
                self.status(code)
            }
            (0x1E, [m, a, b]) => {
                let code = match (self.slot(*m, *a), self.slot(*m, *b)) {
                    (Err(code), _) | (_, Err(code)) => code,
                    _ => {
                        self.slots[*m as usize - 1].swap(*a as usize, *b as usize);
                        ACK
                    }
                };
                self.status(code)
            }
            (0x4A, [m, s, packed @ ..]) => {
                let code = self.upload(*m, *s, packed);
                self.status(code)
            }
            (0x40 | 0x4C | 0x51 | 0x44 | 0x45, _) => {
                let code = match decode(&M::FAMILY, msg) {
                    Ok(Reply::Dump(d)) => self.store_dump(d),
                    _ => NAK,
                };
                self.status(code)
            }
            _ => None,
        }
    }

    fn read_program(&mut self, n: u16) -> Option<Vec<u8>> {
        let current = M::bytes(self.programs.get(n as usize)?).to_vec();
        let data = if self.lagging {
            std::mem::replace(&mut self.last_read, current)
        } else if self.faults.race_deferred_save && self.save_pending() {
            // The idle save completes and hands its "done" token to this
            // read, which ships the read buffer as the flush left it; from
            // now on every read is one behind.
            let flushed = self.store.cached_group().unwrap_or(0) * M::STORE.group_size;
            self.idle_tick();
            self.lagging = true;
            self.last_read = current;
            if M::RACE_REPLY_VALID {
                // The flush re-read its group into the reply buffer.
                M::bytes(&self.programs[flushed as usize]).to_vec()
            } else {
                vec![0; M::FAMILY.program_len]
            }
        } else {
            self.last_read = current.clone();
            current
        };
        self.dump(&Dump::Program(n, data))
    }

    fn store_dump(&mut self, d: Dump) -> u8 {
        match d {
            Dump::CurrentProgram(b) => match M::parse(&b) {
                Ok(p) => {
                    self.edit_buffer = p;
                    ACK
                }
                Err(_) => NAK,
            },
            Dump::Program(n, b) => match M::parse(&b) {
                Ok(p) if n < PROGRAM_COUNT => {
                    // The write's ACK path consumes any stale "done" token.
                    self.lagging = false;
                    if self.store.write(n) != Effect::SavedAndDropped {
                        self.programs[n as usize] = p;
                    }
                    ACK
                }
                _ => NAK,
            },
            Dump::Global(g) if g.starts_with(b"GLOB") => {
                self.global = g;
                ACK
            }
            Dump::Scale(s, v) => match tuning_index(s) {
                Some(t) => {
                    self.scales[t] = v;
                    ACK
                }
                None => NAK,
            },
            Dump::Octave(s, v) => match tuning_index(s) {
                Some(t) => {
                    self.octaves[t] = v;
                    ACK
                }
                None => NAK,
            },
            _ => NAK,
        }
    }

    /// Validate and store a 0x4A upload (`u32 size, u32 checksum, blob`,
    /// packed, with no extra byte, as the official Librarian sends it).
    fn upload(&mut self, m: u8, s: u8, packed: &[u8]) -> u8 {
        if let Err(code) = self.slot(m, s) {
            return code;
        }
        let Ok(mut data) = unpack(packed) else {
            return DATA_FORMAT_ERROR;
        };
        if data.len() < 8 + SLOT_STATUS_LEN {
            return USER_DATA_SIZE_ERROR;
        }
        let size = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as usize;
        let blob = &data[8..];
        if size != blob.len() {
            return USER_DATA_SIZE_ERROR;
        }
        if blob[0] != m || blob[1] != M::PLATFORM.id {
            return USER_TARGET_ERROR;
        }
        // The synth stores its own checksum (its algorithm is unknown); this stand-in only makes it non-zero.
        let sum = blob.iter().fold(0x811C_9DC5u32, |h, &b| {
            (h ^ b as u32).wrapping_mul(0x0100_0193)
        });
        data[4..8].copy_from_slice(&sum.to_le_bytes());
        self.slots[m as usize - 1][s as usize] = Some(Slot(data));
        ACK
    }

    fn slot(&self, m: u8, s: u8) -> Result<Option<&Slot>, u8> {
        let module = Module::from_id(m).map_err(|_| USER_MODULE_ERROR)?;
        self.slots[module as usize - 1]
            .get(s as usize)
            .map(Option::as_ref)
            .ok_or(USER_SLOT_ERROR)
    }

    fn slot_mut(&mut self, m: u8, s: u8) -> Result<&mut Option<Slot>, u8> {
        let module = Module::from_id(m).map_err(|_| USER_MODULE_ERROR)?;
        self.slots[module as usize - 1]
            .get_mut(s as usize)
            .ok_or(USER_SLOT_ERROR)
    }
}

/// Table index for scale/octave number `s` (0–5, or 0x7F for the edit
/// buffer).
fn tuning_index(s: u8) -> Option<usize> {
    match s {
        0..=5 => Some(s as usize),
        0x7F => Some(6),
        _ => None,
    }
}

/// `lose_reply` loses every reply: the standard fault the conformance
/// suite requires.
pub const FAULTS: &[&str] = &["race_deferred_save", "drop_every_3rd_reply", "lose_reply"];

/// [`Sim`] behind the generic [`Simulator`].
pub struct LogueSimulator<M: Model>(pub Sim<M>);

impl<M: Model> Default for LogueSimulator<M> {
    fn default() -> Self {
        LogueSimulator(Sim::new())
    }
}

impl<M: Model> Simulator for LogueSimulator<M> {
    fn handle(&mut self, msg: &[u8]) -> Vec<Vec<u8>> {
        self.0.handle(msg)
    }

    fn idle_tick(&mut self) {
        self.0.idle_tick()
    }

    fn program(&self, slot: u16) -> Option<Vec<u8>> {
        self.0.program(slot).map(|p| M::bytes(p).to_vec())
    }

    fn edit_buffer(&self) -> Vec<u8> {
        M::bytes(self.0.edit_buffer()).to_vec()
    }

    fn is_settled(&self) -> bool {
        !self.0.save_pending() && !self.0.is_lagging()
    }

    fn faults(&self) -> &'static [&'static str] {
        FAULTS
    }

    fn inject(&mut self, fault: &str) {
        self.0.inject(fault)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::msg::{Request, encode_request};
    use crate::testing::{Test, program};

    fn read(sim: &mut Sim<Test>, n: u16) -> Vec<u8> {
        let m = encode_request(&Test::FAMILY, 0, &Request::Program(n));
        match decode(&Test::FAMILY, &sim.handle(&m)[0]) {
            Ok(Reply::Dump(Dump::Program(k, d))) if k == n => d,
            r => panic!("{r:?}"),
        }
    }

    fn write(sim: &mut Sim<Test>, n: u16, p: &[u8]) {
        let m = encode_dump(&Test::FAMILY, 0, &Dump::Program(n, p.to_vec()));
        assert_eq!(decode(&Test::FAMILY, &sim.handle(&m)[0]), Ok(Reply::Ack));
    }

    #[test]
    fn group_of_12_caches_until_the_13th_write() {
        let mut sim = Sim::<Test>::new();
        for k in 0..12 {
            write(&mut sim, 300, &program(k));
            assert!(sim.save_pending());
        }
        write(&mut sim, 300, &program(99));
        assert!(!sim.save_pending());
        assert_eq!(sim.program(300), Some(&program(11)));
    }

    #[test]
    fn race_reply_can_look_valid_and_then_lags() {
        let mut sim = Sim::<Test>::with_library(&[program(1), program(2), program(3)]);
        write(&mut sim, 2, &program(3)); // group 0 dirty
        sim.faults.race_deferred_save = true;
        // The racing read of 1 ships program 0 (the flushed group's first),
        // a valid program under the wrong number.
        assert_eq!(read(&mut sim, 1), program(1));
        assert!(sim.is_lagging());
        assert_eq!(read(&mut sim, 2), program(2));
        assert_eq!(read(&mut sim, 2), program(3));
        write(&mut sim, 5, &program(9));
        assert!(!sim.is_lagging());
    }

    #[test]
    fn identity_carries_the_family() {
        let mut sim = Sim::<Test>::new();
        let r = sim.handle(&oscyl_driver::sysex::DEVICE_INQUIRY);
        assert_eq!(r[0][6], 0x4B);
        assert!(matches!(
            decode(&Test::FAMILY, &r[0]),
            Ok(Reply::Identity { .. })
        ));
    }
}
