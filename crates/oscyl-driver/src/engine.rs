//! Generic sans-IO request/reply engine.
//!
//! Callers submit ops, move bytes with [`Engine::poll_transmit`] and
//! [`Engine::handle_input`], drive time with [`Engine::handle_timeout`], and
//! read results from [`Engine::poll_event`]. A [`Protocol`] expands each op
//! into [`Step`]s and classifies replies; the engine keeps exactly one step
//! in flight, abandons the rest of an op on its first failure, and reports
//! progress after every completed step.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use crate::sysex::Framer;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum OpError {
    #[error("synth rejected {what}{}", at(*index))]
    Nak {
        what: &'static str,
        index: Option<u16>,
    },
    #[error("synth reported error {code:#04x} for {what}{}", at(*index))]
    Device {
        code: u8,
        what: &'static str,
        index: Option<u16>,
    },
    #[error("no reply to {what}{}", at(*index))]
    Timeout {
        what: &'static str,
        index: Option<u16>,
    },
    #[error("malformed data in reply to {what}: {error}")]
    BadData { what: &'static str, error: String },
    /// The op was dropped before it ended (e.g. its synth was unplugged).
    #[error("cancelled")]
    Cancelled,
}

fn at(index: Option<u16>) -> String {
    index.map(|i| format!(" (program {i})")).unwrap_or_default()
}

/// One message to send and the reply that completes it.
#[derive(Debug, Clone)]
pub struct Step<X> {
    pub msg: Vec<u8>,
    pub expect: X,
    /// Bounds a dead link; never paces traffic.
    pub timeout: Duration,
    pub what: &'static str,
    pub index: Option<u16>,
}

/// How a received message relates to the step in flight.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome<E> {
    /// Not an answer to this step (e.g. a dump the user sent by hand).
    Unrelated,
    /// The step completed, optionally with a result.
    Done(Option<E>),
    /// The step failed; the rest of the op is abandoned.
    Failed(OpError),
}

/// A synth's request/reply rules.
pub trait Protocol {
    type Op;
    type Event;
    type Expect: Copy;
    fn expand(&self, op: Self::Op) -> Vec<Step<Self::Expect>>;
    /// Classify one complete SysEx message against the step in flight.
    fn on_message(
        &self,
        msg: &[u8],
        expect: Self::Expect,
        what: &'static str,
        index: Option<u16>,
    ) -> Outcome<Self::Event>;
    fn progress(done: usize, total: usize) -> Self::Event;
    fn done() -> Self::Event;
    fn failed(e: OpError) -> Self::Event;
}

pub struct Engine<P: Protocol> {
    protocol: P,
    queue: VecDeque<P::Op>,
    steps: VecDeque<Step<P::Expect>>,
    total: usize,
    in_flight: Option<(Step<P::Expect>, Instant)>,
    framer: Framer,
    events: VecDeque<P::Event>,
}

impl<P: Protocol> Engine<P> {
    pub fn new(protocol: P) -> Engine<P> {
        Engine {
            protocol,
            queue: VecDeque::new(),
            steps: VecDeque::new(),
            total: 0,
            in_flight: None,
            framer: Framer::new(),
            events: VecDeque::new(),
        }
    }

    pub fn protocol(&self) -> &P {
        &self.protocol
    }

    pub fn submit(&mut self, op: P::Op) {
        self.queue.push_back(op);
    }

    /// True when nothing is queued or in flight.
    pub fn is_idle(&self) -> bool {
        self.queue.is_empty() && self.steps.is_empty() && self.in_flight.is_none()
    }

    /// Next message to send, if the line is free.
    pub fn poll_transmit(&mut self, now: Instant) -> Option<Vec<u8>> {
        if self.in_flight.is_some() {
            return None;
        }
        while self.steps.is_empty() {
            let op = self.queue.pop_front()?;
            self.steps = self.protocol.expand(op).into();
            self.total = self.steps.len();
            if self.steps.is_empty() {
                self.events.push_back(P::done());
            }
        }
        let step = self.steps.pop_front()?;
        let msg = step.msg.clone();
        self.in_flight = Some((step, now));
        Some(msg)
    }

    /// When the in-flight request stops waiting for its reply.
    pub fn next_deadline(&self) -> Option<Instant> {
        self.in_flight.as_ref().map(|(s, t)| *t + s.timeout)
    }

    pub fn handle_timeout(&mut self, now: Instant) {
        let expired = self
            .in_flight
            .as_ref()
            .is_some_and(|(s, t)| now >= *t + s.timeout);
        if expired {
            let (step, _) = self.in_flight.take().expect("checked above");
            self.fail(OpError::Timeout {
                what: step.what,
                index: step.index,
            });
        }
    }

    pub fn handle_input(&mut self, bytes: &[u8]) {
        for msg in self.framer.push(bytes) {
            self.on_message(&msg);
        }
    }

    pub fn poll_event(&mut self) -> Option<P::Event> {
        self.events.pop_front()
    }

    fn on_message(&mut self, msg: &[u8]) {
        let Some((step, _)) = &self.in_flight else {
            return;
        };
        match self
            .protocol
            .on_message(msg, step.expect, step.what, step.index)
        {
            Outcome::Unrelated => {}
            Outcome::Failed(e) => {
                self.in_flight = None;
                self.fail(e);
            }
            Outcome::Done(event) => {
                self.in_flight = None;
                self.events.extend(event);
                let done = self.total - self.steps.len();
                self.events.push_back(P::progress(done, self.total));
                if self.steps.is_empty() {
                    self.events.push_back(P::done());
                }
            }
        }
    }

    /// Abandon the rest of the current op.
    fn fail(&mut self, e: OpError) {
        self.steps.clear();
        self.events.push_back(P::failed(e));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    /// Test protocol: an op is a list of bytes; each byte `b` is sent as
    /// `F0 b F7` and completed by the same message coming back.
    struct Echo;

    #[derive(Debug, PartialEq)]
    enum Ev {
        Got(u8),
        Progress(usize, usize),
        Done,
        Failed(OpError),
    }

    impl Protocol for Echo {
        type Op = Vec<u8>;
        type Event = Ev;
        type Expect = u8;
        fn expand(&self, op: Vec<u8>) -> Vec<Step<u8>> {
            op.into_iter()
                .map(|b| Step {
                    msg: vec![0xF0, b, 0xF7],
                    expect: b,
                    timeout: Duration::from_secs(1),
                    what: "echo",
                    index: Some(b as u16),
                })
                .collect()
        }
        fn on_message(
            &self,
            msg: &[u8],
            expect: u8,
            _what: &'static str,
            _index: Option<u16>,
        ) -> Outcome<Ev> {
            match msg {
                [0xF0, b, 0xF7] if *b == expect => Outcome::Done(Some(Ev::Got(*b))),
                _ => Outcome::Unrelated,
            }
        }
        fn progress(done: usize, total: usize) -> Ev {
            Ev::Progress(done, total)
        }
        fn done() -> Ev {
            Ev::Done
        }
        fn failed(e: OpError) -> Ev {
            Ev::Failed(e)
        }
    }

    fn events(e: &mut Engine<Echo>) -> Vec<Ev> {
        std::iter::from_fn(|| e.poll_event()).collect()
    }

    #[test]
    fn one_in_flight() {
        let t = Instant::now();
        let mut e = Engine::new(Echo);
        e.submit(vec![1, 2]);
        assert_eq!(e.poll_transmit(t), Some(vec![0xF0, 1, 0xF7]));
        assert_eq!(e.poll_transmit(t), None);
        e.handle_input(&[0xF0, 1, 0xF7]);
        assert_eq!(e.poll_transmit(t), Some(vec![0xF0, 2, 0xF7]));
    }

    #[test]
    fn progress_then_done() {
        let t = Instant::now();
        let mut e = Engine::new(Echo);
        e.submit(vec![1, 2]);
        e.poll_transmit(t);
        e.handle_input(&[0xF0, 1, 0xF7]);
        e.poll_transmit(t);
        e.handle_input(&[0xF0, 2, 0xF7]);
        assert_eq!(
            events(&mut e),
            vec![
                Ev::Got(1),
                Ev::Progress(1, 2),
                Ev::Got(2),
                Ev::Progress(2, 2),
                Ev::Done
            ]
        );
        assert!(e.is_idle());
    }

    #[test]
    fn unrelated_message_is_ignored() {
        let t = Instant::now();
        let mut e = Engine::new(Echo);
        e.submit(vec![1]);
        e.poll_transmit(t);
        e.handle_input(&[0xF0, 9, 0xF7]);
        assert!(events(&mut e).is_empty());
        assert!(!e.is_idle());
    }

    #[test]
    fn timeout_abandons_rest_of_op_then_next_op_runs() {
        let t = Instant::now();
        let mut e = Engine::new(Echo);
        e.submit(vec![1, 2]);
        e.submit(vec![3]);
        e.poll_transmit(t);
        assert_eq!(e.next_deadline(), Some(t + Duration::from_secs(1)));
        e.handle_timeout(t + Duration::from_secs(1));
        assert_eq!(
            events(&mut e),
            vec![Ev::Failed(OpError::Timeout {
                what: "echo",
                index: Some(1)
            })]
        );
        assert_eq!(e.poll_transmit(t), Some(vec![0xF0, 3, 0xF7]));
    }
}
