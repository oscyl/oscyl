//! The Rev2's request/reply rules for the generic engine, and the
//! contract's session.
//!
//! The Rev2 answers no dump, so every write step is the dump followed by a
//! request for the same slot (or the edit buffer), and completes on the
//! reply: equal bytes mean done, different bytes fail the step. A step can
//! report a false failure, never a false success.

use std::time::{Duration, Instant};

use oscyl_driver::engine::{Engine, OpError, Outcome, Protocol, Step};
use oscyl_driver::{DriverError, DriverSession, SessionEvent, SessionOp};

use crate::Program;
use crate::msg::{
    BANK_LEN, Reply, SLOTS, decode, encode_edit_dump, encode_edit_request, encode_program_dump,
    encode_program_request,
};

/// Bounds a dead link only. A write step is about 1.5 s of wire time over
/// 5-pin MIDI.
const TIMEOUT: Duration = Duration::from_secs(5);
/// U1-U4. F1-F4 accept SysEx writes, but Oscyl keeps them read-only.
pub const WRITABLE: u16 = 4 * BANK_LEN;

fn below(n: u16, limit: u16, what: &str) -> Result<u16, DriverError> {
    if n < limit {
        Ok(n)
    } else {
        Err(DriverError(format!(
            "cannot {what} slot {n}: only slots 0-{} allow it",
            limit - 1
        )))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Op {
    ReadCurrent,
    ReadPrograms(Vec<u16>),
    WritePrograms(Vec<(u16, Program)>),
    Audition(Box<Program>),
    /// `op` after a device check that refuses a port echoing our output.
    Checked(Box<Op>),
}

use oscyl_driver::sysex::DEVICE_INQUIRY as INQUIRY;

/// The reply that completes a step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Expect {
    /// `None`: the edit buffer.
    pub slot: Option<u16>,
    /// What a write sent; the reply must hold the same bytes.
    pub sent: Option<Program>,
    /// The device check: completed by the Rev2's identity reply.
    pub check: bool,
}

pub struct Rev2;

impl Protocol for Rev2 {
    type Op = Op;
    type Event = SessionEvent;
    type Expect = Expect;

    fn expand(&self, op: Op) -> Vec<Step<Expect>> {
        let step = |msg, slot, sent, what| Step {
            msg,
            expect: Expect {
                slot,
                sent,
                check: false,
            },
            timeout: TIMEOUT,
            what,
            index: slot,
        };
        match op {
            Op::ReadCurrent => vec![step(encode_edit_request(), None, None, "edit buffer read")],
            Op::ReadPrograms(slots) => slots
                .into_iter()
                .map(|n| step(encode_program_request(n), Some(n), None, "program read"))
                .collect(),
            Op::WritePrograms(programs) => programs
                .into_iter()
                .map(|(n, p)| {
                    let msg = [encode_program_dump(n, &p), encode_program_request(n)].concat();
                    step(msg, Some(n), Some(p), "program write")
                })
                .collect(),
            Op::Audition(p) => {
                let msg = [encode_edit_dump(&p), encode_edit_request()].concat();
                vec![step(msg, None, Some(*p), "audition")]
            }
            Op::Checked(op) => {
                // A write completes on a dump equal to what was sent, which is
                // also what a port looping our output back would deliver.
                // Our own inquiry coming back exposes such a port first.
                let check = Step {
                    msg: INQUIRY.to_vec(),
                    expect: Expect {
                        slot: None,
                        sent: None,
                        check: true,
                    },
                    timeout: TIMEOUT,
                    what: "device check",
                    index: None,
                };
                std::iter::once(check).chain(self.expand(*op)).collect()
            }
        }
    }

    fn on_message(
        &self,
        msg: &[u8],
        expect: Expect,
        what: &'static str,
        _index: Option<u16>,
    ) -> Outcome<SessionEvent> {
        if expect.check {
            return match decode(msg) {
                _ if msg == INQUIRY => Outcome::Failed(OpError::BadData {
                    what,
                    error: "our own messages come back on this MIDI port (MIDI thru or a \
                            loop), so writes cannot be confirmed; turn the loop off"
                        .into(),
                }),
                Ok(Reply::Identity) => Outcome::Done(None),
                _ => Outcome::Unrelated,
            };
        }
        let (slot, p) = match decode(msg) {
            Ok(Reply::Program(n, p)) if expect.slot == Some(n) => (Some(n), p),
            Ok(Reply::Edit(p)) if expect.slot.is_none() => (None, p),
            // Identity replies, dumps of other slots, foreign messages.
            _ => return Outcome::Unrelated,
        };
        match (expect.sent, slot) {
            (Some(sent), _) if sent == p => Outcome::Done(None),
            (Some(_), _) => Outcome::Failed(OpError::BadData {
                what,
                error: "read-back differs from what was sent".into(),
            }),
            (None, Some(n)) => Outcome::Done(Some(SessionEvent::Program(n, p.as_bytes().to_vec()))),
            (None, None) => Outcome::Done(Some(SessionEvent::Current(p.as_bytes().to_vec()))),
        }
    }

    fn progress(done: usize, total: usize) -> SessionEvent {
        SessionEvent::Progress { done, total }
    }

    fn done() -> SessionEvent {
        SessionEvent::Done
    }

    fn failed(e: OpError) -> SessionEvent {
        SessionEvent::Failed(e)
    }
}

pub struct Rev2Session(Engine<Rev2>);

impl Default for Rev2Session {
    fn default() -> Rev2Session {
        Rev2Session(Engine::new(Rev2))
    }
}

impl DriverSession for Rev2Session {
    /// Refuses writes outside U1-U4, reads past F4 and wrong-length
    /// programs before anything is queued.
    fn submit(&mut self, op: SessionOp) -> Result<(), DriverError> {
        let op = match op {
            SessionOp::ReadCurrent => Op::ReadCurrent,
            SessionOp::ReadPrograms(slots) => Op::ReadPrograms(
                slots
                    .into_iter()
                    .map(|n| below(n, SLOTS, "read"))
                    .collect::<Result<_, _>>()?,
            ),
            SessionOp::WritePrograms(programs) => Op::WritePrograms(
                programs
                    .into_iter()
                    .map(|(n, b)| Ok((below(n, WRITABLE, "write")?, Program::from_bytes(&b)?)))
                    .collect::<Result<_, DriverError>>()?,
            ),
            SessionOp::Audition(b) => Op::Audition(Box::new(Program::from_bytes(&b)?)),
        };
        self.0.submit(Op::Checked(Box::new(op)));
        Ok(())
    }

    fn is_idle(&self) -> bool {
        self.0.is_idle()
    }

    fn poll_transmit(&mut self, now: Instant) -> Option<Vec<u8>> {
        self.0.poll_transmit(now)
    }

    fn next_deadline(&self) -> Option<Instant> {
        self.0.next_deadline()
    }

    fn handle_timeout(&mut self, now: Instant) {
        self.0.handle_timeout(now)
    }

    fn handle_input(&mut self, bytes: &[u8]) {
        self.0.handle_input(bytes)
    }

    fn poll_event(&mut self) -> Option<SessionEvent> {
        self.0.poll_event()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::msg::{
        encode_edit_dump, encode_edit_request, encode_program_dump, encode_program_request,
    };
    use crate::sim::SimRev2;
    use oscyl_driver::Simulator;

    /// Run until idle; time jumps to the next deadline, nothing sleeps.
    /// Returns the events and every message sent.
    fn pump(s: &mut Rev2Session, sim: &mut SimRev2) -> (Vec<SessionEvent>, Vec<Vec<u8>>) {
        let (mut now, mut events, mut sent) = (Instant::now(), vec![], vec![]);
        while !s.is_idle() {
            if let Some(m) = s.poll_transmit(now) {
                for r in sim.handle(&m) {
                    s.handle_input(&r);
                }
                sent.push(m);
            } else if let Some(d) = s.next_deadline() {
                now = d;
                s.handle_timeout(now);
            }
            events.extend(std::iter::from_fn(|| s.poll_event()));
        }
        events.extend(std::iter::from_fn(|| s.poll_event()));
        (events, sent)
    }

    /// A port that loops our output back to our input, with no synth.
    fn echo(s: &mut Rev2Session) -> Vec<SessionEvent> {
        let (mut now, mut events) = (Instant::now(), vec![]);
        while !s.is_idle() {
            if let Some(m) = s.poll_transmit(now) {
                s.handle_input(&m);
            } else if let Some(d) = s.next_deadline() {
                now = d;
                s.handle_timeout(now);
            }
            events.extend(std::iter::from_fn(|| s.poll_event()));
        }
        events
    }

    #[test]
    fn an_echoing_port_never_completes_a_write() {
        let mut s = Rev2Session::default();
        s.submit(SessionOp::WritePrograms(vec![(
            3,
            named("Echo").as_bytes().to_vec(),
        )]))
        .unwrap();
        let ev = echo(&mut s);
        assert!(!ev.contains(&SessionEvent::Done), "{ev:?}");
        assert!(
            matches!(ev.last(), Some(SessionEvent::Failed(OpError::BadData { error, .. })) if error.contains("loop")),
            "{ev:?}"
        );
    }

    #[test]
    fn an_echoing_port_never_completes_an_audition() {
        let mut s = Rev2Session::default();
        s.submit(SessionOp::Audition(named("Echo").as_bytes().to_vec()))
            .unwrap();
        assert!(!echo(&mut s).contains(&SessionEvent::Done));
    }

    fn named(name: &str) -> Program {
        let mut p = Program::init();
        p.set_name(name).unwrap();
        p
    }

    fn bytes(p: &Program) -> Vec<u8> {
        p.as_bytes().to_vec()
    }

    #[test]
    fn write_completes_on_equal_read_back() {
        let (mut s, mut sim) = (Rev2Session::default(), SimRev2::default());
        let p = named("Five");
        s.submit(SessionOp::WritePrograms(vec![(5, bytes(&p))]))
            .unwrap();
        let (ev, sent) = pump(&mut s, &mut sim);
        assert_eq!(
            ev,
            [
                SessionEvent::Progress { done: 1, total: 2 },
                SessionEvent::Progress { done: 2, total: 2 },
                SessionEvent::Done
            ]
        );
        assert_eq!(
            sent,
            [
                INQUIRY.to_vec(),
                [encode_program_dump(5, &p), encode_program_request(5)].concat()
            ]
        );
        assert_eq!(sim.program(5), Some(bytes(&p)));
    }

    #[test]
    fn stale_read_back_fails_the_write() {
        let (mut s, mut sim) = (Rev2Session::default(), SimRev2::default());
        sim.inject("answer_before_store");
        let p = named("Five");
        s.submit(SessionOp::WritePrograms(vec![
            (5, bytes(&p)),
            (6, bytes(&p)),
        ]))
        .unwrap();
        let (ev, _) = pump(&mut s, &mut sim);
        assert!(
            matches!(
                ev[..],
                [
                    SessionEvent::Progress { done: 1, .. },
                    SessionEvent::Failed(OpError::BadData { .. })
                ]
            ),
            "{ev:?}"
        );
        assert_eq!(
            sim.program(6),
            Some(bytes(&Program::init())),
            "op abandoned"
        );
    }

    #[test]
    fn audition_reads_back_edit_buffer() {
        let (mut s, mut sim) = (Rev2Session::default(), SimRev2::default());
        let p = named("Try");
        s.submit(SessionOp::Audition(bytes(&p))).unwrap();
        let (ev, sent) = pump(&mut s, &mut sim);
        assert_eq!(ev.last(), Some(&SessionEvent::Done));
        assert_eq!(
            sent,
            [
                INQUIRY.to_vec(),
                [encode_edit_dump(&p), encode_edit_request()].concat()
            ]
        );
        assert_eq!(sim.edit_buffer(), bytes(&p));
        s.submit(SessionOp::ReadCurrent).unwrap();
        let (ev, _) = pump(&mut s, &mut sim);
        assert!(ev.contains(&SessionEvent::Current(bytes(&p))));
    }

    #[test]
    fn read_returns_the_slot() {
        let (mut s, mut sim) = (Rev2Session::default(), SimRev2::default());
        s.submit(SessionOp::ReadPrograms(vec![1023])).unwrap();
        let (ev, sent) = pump(&mut s, &mut sim);
        assert_eq!(sent, [INQUIRY.to_vec(), encode_program_request(1023)]);
        assert!(ev.contains(&SessionEvent::Program(1023, bytes(&Program::init()))));
    }

    #[test]
    fn factory_slot_write_refused() {
        let mut s = Rev2Session::default();
        let p = bytes(&Program::init());
        for slot in [512, 1023, 1024] {
            assert!(
                s.submit(SessionOp::WritePrograms(vec![
                    (0, p.clone()),
                    (slot, p.clone())
                ]))
                .is_err()
            );
        }
        assert!(s.submit(SessionOp::ReadPrograms(vec![1024])).is_err());
        assert!(s.submit(SessionOp::Audition(p[1..].to_vec())).is_err());
        assert_eq!(s.poll_transmit(Instant::now()), None);
        assert!(s.is_idle());
    }
}
