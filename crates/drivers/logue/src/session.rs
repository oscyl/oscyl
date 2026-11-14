//! A logue synth's request/reply rules for the generic engine.
//!
//! Exactly one message is in flight at a time: the xd drops requests sent
//! while it is busy (seen on the hardware); the
//! prologue's parser stalls the same way while a write ACK is pending. Program
//! writes are only accepted as a [`CommitPlan`], so every write batch ends
//! with the synth's flash save complete and reads are safe afterwards.

use std::marker::PhantomData;
use std::time::Duration;

use oscyl_driver::engine::{OpError, Outcome, Protocol, Step};

use crate::Model;
use crate::msg::{Dump, Module, Reply, Request, decode, encode_dump, encode_request};
use crate::store::CommitPlan;

/// Timeouts used by the official xd Librarian; assumed the same on the
/// prologue, whose Librarian is the same program.
pub const READ_TIMEOUT: Duration = Duration::from_secs(5);
pub const WRITE_TIMEOUT: Duration = Duration::from_secs(3);
pub const UNIT_TIMEOUT: Duration = Duration::from_secs(30);

/// A high-level operation on programs of type `P`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Op<P> {
    /// Read the edit buffer (unsaved panel state).
    ReadCurrent,
    ReadPrograms(Vec<u16>),
    /// Store programs and commit them to flash; stops at the first failure.
    WritePrograms(CommitPlan<P>),
    /// Load a program into the edit buffer without storing it.
    Audition(Box<P>),
    ReadGlobal,
    /// Read the 6 user scales and 6 user octaves.
    ReadTuning,
    /// Read the status of every user slot.
    ReadSlots,
    ReadSlotData {
        module: Module,
        slot: u8,
    },
    /// Upload TABLE 7 slot data (`u32 size, u32 checksum, blob`).
    UploadUnit {
        module: Module,
        slot: u8,
        data: Vec<u8>,
    },
    ClearSlot {
        module: Module,
        slot: u8,
    },
}

/// Results reported by the session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event<P> {
    Current(P),
    Program(u16, P),
    Global(Vec<u8>),
    Scale(u8, Vec<u8>),
    Octave(u8, Vec<u8>),
    Slot {
        module: Module,
        slot: u8,
        header: Option<Vec<u8>>,
    },
    SlotData {
        module: Module,
        slot: u8,
        data: Vec<u8>,
    },
    Progress {
        done: usize,
        total: usize,
    },
    OpDone,
    OpFailed(OpError),
}

/// What reply completes a step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Expect {
    Current,
    Program(u16),
    Global,
    Scale(u8),
    Octave(u8),
    Slot(Module, u8),
    SlotData(Module, u8),
    Ack,
}

/// Model `M`'s request/reply rules on one MIDI channel.
pub struct Proto<M> {
    /// MIDI channel, 0-15.
    pub channel: u8,
    model: PhantomData<fn() -> M>,
}

impl<M> Proto<M> {
    pub fn new(channel: u8) -> Proto<M> {
        Proto {
            channel,
            model: PhantomData,
        }
    }
}

impl<M: Model> Protocol for Proto<M> {
    type Op = Op<M::Program>;
    type Event = Event<M::Program>;
    type Expect = Expect;

    fn expand(&self, op: Self::Op) -> Vec<Step<Expect>> {
        self.expand_op(op)
    }

    fn on_message(
        &self,
        msg: &[u8],
        expect: Expect,
        what: &'static str,
        index: Option<u16>,
    ) -> Outcome<Self::Event> {
        // Messages we cannot decode are ignored (e.g. panel dumps the user
        // triggers by hand).
        let Ok(reply) = decode(&M::FAMILY, msg) else {
            return Outcome::Unrelated;
        };
        let bad = |e: M::Error| {
            Outcome::Failed(OpError::BadData {
                what,
                error: e.to_string(),
            })
        };
        let event = match (expect, reply) {
            (_, Reply::Nak) => return Outcome::Failed(OpError::Nak { what, index }),
            (_, Reply::Error(code)) => {
                return Outcome::Failed(OpError::Device { code, what, index });
            }
            (Expect::Ack, Reply::Ack) => None,
            (Expect::Current, Reply::Dump(Dump::CurrentProgram(p))) => match M::parse(&p) {
                Ok(p) => Some(Event::Current(p)),
                Err(e) => return bad(e),
            },
            (Expect::Program(n), Reply::Dump(Dump::Program(m, p))) if n == m => {
                match M::parse(&p) {
                    Ok(p) => Some(Event::Program(n, p)),
                    Err(e) => return bad(e),
                }
            }
            (Expect::Global, Reply::Dump(Dump::Global(g))) => Some(Event::Global(g)),
            (Expect::Scale(s), Reply::Dump(Dump::Scale(t, v))) if s == t => {
                Some(Event::Scale(s, v))
            }
            (Expect::Octave(s), Reply::Dump(Dump::Octave(t, v))) if s == t => {
                Some(Event::Octave(s, v))
            }
            (
                Expect::Slot(m, s),
                Reply::SlotStatus {
                    module,
                    slot,
                    header,
                },
            ) if (m, s) == (module, slot) => Some(Event::Slot {
                module,
                slot,
                header,
            }),
            (Expect::SlotData(m, s), Reply::Dump(Dump::SlotData { module, slot, data }))
                if (m, s) == (module, slot) =>
            {
                Some(Event::SlotData { module, slot, data })
            }
            _ => return Outcome::Unrelated,
        };
        Outcome::Done(event)
    }

    fn progress(done: usize, total: usize) -> Self::Event {
        Event::Progress { done, total }
    }

    fn done() -> Self::Event {
        Event::OpDone
    }

    fn failed(e: OpError) -> Self::Event {
        Event::OpFailed(e)
    }
}

impl<M: Model> Proto<M> {
    fn req(
        &self,
        r: Request,
        expect: Expect,
        what: &'static str,
        index: Option<u16>,
    ) -> Step<Expect> {
        Step {
            msg: encode_request(&M::FAMILY, self.channel, &r),
            expect,
            timeout: READ_TIMEOUT,
            what,
            index,
        }
    }

    fn expand_op(&self, op: Op<M::Program>) -> Vec<Step<Expect>> {
        let ch = self.channel;
        let write = |d: Dump, timeout, what, index| Step {
            msg: encode_dump(&M::FAMILY, ch, &d),
            expect: Expect::Ack,
            timeout,
            what,
            index,
        };
        match op {
            Op::ReadCurrent => [self.req(
                Request::CurrentProgram,
                Expect::Current,
                "current program read",
                None,
            )]
            .into(),
            Op::ReadPrograms(ns) => ns
                .into_iter()
                .map(|n| {
                    self.req(
                        Request::Program(n),
                        Expect::Program(n),
                        "program read",
                        Some(n),
                    )
                })
                .collect(),
            Op::WritePrograms(plan) => plan
                .writes()
                .iter()
                .map(|(n, p)| {
                    write(
                        Dump::Program(*n, M::bytes(p).to_vec()),
                        WRITE_TIMEOUT,
                        "program write",
                        Some(*n),
                    )
                })
                .collect(),
            Op::Audition(p) => {
                let s = write(
                    Dump::CurrentProgram(M::bytes(&p).to_vec()),
                    READ_TIMEOUT,
                    "audition",
                    None,
                );
                [s].into()
            }
            Op::ReadGlobal => {
                [self.req(Request::Global, Expect::Global, "global read", None)].into()
            }
            Op::ReadTuning => (0..6)
                .flat_map(|s| {
                    [
                        self.req(Request::Scale(s), Expect::Scale(s), "scale read", None),
                        self.req(Request::Octave(s), Expect::Octave(s), "octave read", None),
                    ]
                })
                .collect(),
            Op::ReadSlots => Module::ALL
                .into_iter()
                .flat_map(|m| (0..m.slots()).map(move |s| (m, s)))
                .map(|(module, slot)| {
                    self.req(
                        Request::SlotStatus { module, slot },
                        Expect::Slot(module, slot),
                        "slot status read",
                        None,
                    )
                })
                .collect(),
            Op::ReadSlotData { module, slot } => {
                let mut s = self.req(
                    Request::SlotData { module, slot },
                    Expect::SlotData(module, slot),
                    "slot data read",
                    None,
                );
                s.timeout = UNIT_TIMEOUT;
                [s].into()
            }
            Op::UploadUnit { module, slot, data } => [write(
                Dump::SlotData { module, slot, data },
                UNIT_TIMEOUT,
                "unit upload",
                None,
            )]
            .into(),
            Op::ClearSlot { module, slot } => {
                let mut s = self.req(
                    Request::ClearSlot { module, slot },
                    Expect::Ack,
                    "slot clear",
                    None,
                );
                s.timeout = UNIT_TIMEOUT;
                [s].into()
            }
        }
    }
}
