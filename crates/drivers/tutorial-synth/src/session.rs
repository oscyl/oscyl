// ANCHOR: protocol
//! The session: the Tutorial Mono's protocol, sans IO.
//!
//! The request engine of `oscyl_driver::engine` keeps one request in
//! flight and times it out. This file says what to send for each
//! operation and which reply completes each request. A write is done only
//! when the synth acknowledges it, so a lost reply is a failure.

use std::time::{Duration, Instant};

use oscyl_driver::engine::{Engine, OpError, Outcome, Protocol, Step};
use oscyl_driver::{DriverError, DriverSession, SessionEvent, SessionOp};

use crate::msg::{Msg, SLOTS};
use crate::program;

/// The reply that completes a request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Expect {
    Program(u16),
    EditBuffer,
    Ack(Option<u16>),
}

/// How long to wait for a reply. It finds a dead link; it does not pace
/// the messages.
const TIMEOUT: Duration = Duration::from_secs(2);

/// The Tutorial Mono's rules for the request engine.
pub struct Mono;

impl Protocol for Mono {
    type Op = SessionOp;
    type Event = SessionEvent;
    type Expect = Expect;

    /// The requests that make up `op`, in order.
    fn expand(&self, op: SessionOp) -> Vec<Step<Expect>> {
        let step = |msg: Msg, expect, what, index| Step {
            msg: msg.encode(),
            expect,
            timeout: TIMEOUT,
            what,
            index,
        };
        match op {
            SessionOp::ReadCurrent => {
                vec![step(
                    Msg::EditRequest,
                    Expect::EditBuffer,
                    "edit buffer read",
                    None,
                )]
            }
            SessionOp::ReadPrograms(slots) => slots
                .into_iter()
                .map(|n| {
                    step(
                        Msg::ProgramRequest(n),
                        Expect::Program(n),
                        "program read",
                        Some(n),
                    )
                })
                .collect(),
            SessionOp::WritePrograms(batch) => batch
                .into_iter()
                .map(|(n, p)| {
                    let expect = Expect::Ack(Some(n));
                    step(Msg::ProgramDump(n, p), expect, "program write", Some(n))
                })
                .collect(),
            SessionOp::Audition(p) => {
                vec![step(Msg::EditDump(p), Expect::Ack(None), "audition", None)]
            }
        }
    }
    // ANCHOR_END: protocol

    // ANCHOR: replies
    /// What one message from the synth means for the request in flight.
    fn on_message(
        &self,
        bytes: &[u8],
        expect: Expect,
        what: &'static str,
        _index: Option<u16>,
    ) -> Outcome<SessionEvent> {
        match (Msg::decode(bytes), expect) {
            (Some(Msg::ProgramDump(n, p)), Expect::Program(m)) if n == m => {
                whole(p, what, |p| SessionEvent::Program(n, p))
            }
            (Some(Msg::EditDump(p)), Expect::EditBuffer) => whole(p, what, SessionEvent::Current),
            (Some(Msg::Ack(s)), Expect::Ack(t)) if s == t => Outcome::Done(None),
            // Another message: an echo of our own, or a dump sent by hand.
            _ => Outcome::Unrelated,
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

/// The request is done with the event `event(p)` when `p` is a whole
/// program. Bad bytes never reach the apps: the request fails.
fn whole(
    p: Vec<u8>,
    what: &'static str,
    event: impl FnOnce(Vec<u8>) -> SessionEvent,
) -> Outcome<SessionEvent> {
    match program::check(&p) {
        Ok(()) => Outcome::Done(Some(event(p))),
        Err(e) => Outcome::Failed(OpError::BadData { what, error: e.0 }),
    }
}
// ANCHOR_END: replies

// ANCHOR: session
/// A session with one Tutorial Mono.
pub struct Session(Engine<Mono>);

impl Default for Session {
    fn default() -> Session {
        Session(Engine::new(Mono))
    }
}

/// Refuses a slot past the last one.
fn slot(n: u16) -> Result<(), DriverError> {
    if n < SLOTS {
        Ok(())
    } else {
        Err(DriverError(format!("the Tutorial Mono has no slot {n}")))
    }
}

impl DriverSession for Session {
    /// Checks every slot and program first: a bad operation sends nothing.
    fn submit(&mut self, op: SessionOp) -> Result<(), DriverError> {
        match &op {
            SessionOp::ReadCurrent => {}
            SessionOp::ReadPrograms(slots) => slots.iter().try_for_each(|&n| slot(n))?,
            SessionOp::WritePrograms(batch) => batch
                .iter()
                .try_for_each(|(n, p)| slot(*n).and_then(|()| program::check(p)))?,
            SessionOp::Audition(p) => program::check(p)?,
        }
        self.0.submit(op);
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
// ANCHOR_END: session

// ANCHOR: tests
#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::Sim;
    use oscyl_driver::Simulator;

    #[test]
    fn a_write_ends_when_the_synth_acknowledges_it() {
        let (mut s, mut sim) = (Session::default(), Sim::default());
        s.submit(SessionOp::WritePrograms(vec![(4, program::init())]))
            .unwrap();
        let now = Instant::now();
        let request = s.poll_transmit(now).unwrap();
        assert!(s.poll_transmit(now).is_none(), "one request at a time");
        for reply in sim.handle(&request) {
            s.handle_input(&reply);
        }
        let events: Vec<_> = std::iter::from_fn(|| s.poll_event()).collect();
        assert_eq!(events.last(), Some(&SessionEvent::Done));
        assert!(s.submit(SessionOp::ReadPrograms(vec![16])).is_err());
    }
}
// ANCHOR_END: tests
