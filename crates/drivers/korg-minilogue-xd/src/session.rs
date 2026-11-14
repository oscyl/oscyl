//! Sans-IO request/reply engine.
//!
//! Callers submit [`Op`]s, move bytes with [`Session::poll_transmit`] and
//! [`Session::handle_input`], drive time with [`Session::handle_timeout`],
//! and read results from [`Session::poll_event`]. Exactly one message is in
//! flight at a time: the synth drops requests sent while it is busy
//! (seen on the hardware). Program writes are
//! only accepted as a [`CommitPlan`](crate::store::CommitPlan), so every write batch ends with the
//! synth's flash save complete and reads are safe afterwards. The rules
//! are the logue family's ([`oscyl_logue::session`]) for the xd.

pub use oscyl_driver::engine::OpError;
use oscyl_driver::engine::{Engine, Outcome, Protocol, Step};
use oscyl_logue::session::Proto;

use crate::Program;
use crate::model::XdModel;
#[cfg(test)]
use crate::msg::{Dump, Module, Request, encode_request};
#[cfg(test)]
use crate::store::CommitPlan;
#[cfg(test)]
use std::time::Duration;

pub use oscyl_logue::session::Expect;

/// A high-level operation.
pub type Op = oscyl_logue::session::Op<Program>;

/// Results reported by the session.
pub type Event = oscyl_logue::session::Event<Program>;

/// The minilogue xd's request/reply rules for the generic engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Xd {
    /// MIDI channel, 0-15.
    pub channel: u8,
}

pub type Session = Engine<Xd>;

impl Xd {
    fn proto(&self) -> Proto<XdModel> {
        Proto::new(self.channel)
    }
}

impl Protocol for Xd {
    type Op = Op;
    type Event = Event;
    type Expect = Expect;

    fn expand(&self, op: Op) -> Vec<Step<Expect>> {
        self.proto().expand(op)
    }

    fn on_message(
        &self,
        msg: &[u8],
        expect: Expect,
        what: &'static str,
        index: Option<u16>,
    ) -> Outcome<Event> {
        self.proto().on_message(msg, expect, what, index)
    }

    fn progress(done: usize, total: usize) -> Event {
        Event::Progress { done, total }
    }

    fn done() -> Event {
        Event::OpDone
    }

    fn failed(e: OpError) -> Event {
        Event::OpFailed(e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::msg::encode_dump;
    use std::time::Instant;

    fn ack() -> Vec<u8> {
        vec![0xF0, 0x42, 0x30, 0x00, 0x01, 0x51, 0x23, 0xF7]
    }
    fn nak() -> Vec<u8> {
        vec![0xF0, 0x42, 0x30, 0x00, 0x01, 0x51, 0x24, 0xF7]
    }
    fn prog_dump(n: u16) -> Vec<u8> {
        encode_dump(0, &Dump::Program(n, Program::init().as_bytes().to_vec()))
    }
    fn plan(ns: &[u16]) -> CommitPlan {
        crate::store::plan_commit(ns.iter().map(|&n| (n, Program::init())), |_| None).unwrap()
    }
    fn events(s: &mut Session) -> Vec<Event> {
        std::iter::from_fn(|| s.poll_event()).collect()
    }

    #[test]
    fn reads_one_request_at_a_time() {
        let t = Instant::now();
        let mut s = Session::new(Xd { channel: 0 });
        s.submit(Op::ReadPrograms(vec![0, 1, 2]));
        assert_eq!(
            s.poll_transmit(t),
            Some(encode_request(0, &Request::Program(0)))
        );
        assert_eq!(s.poll_transmit(t), None, "must wait for the reply");
        s.handle_input(&prog_dump(0));
        assert_eq!(
            s.poll_transmit(t),
            Some(encode_request(0, &Request::Program(1)))
        );
        s.handle_input(&prog_dump(1));
        s.poll_transmit(t);
        s.handle_input(&prog_dump(2));
        let ev = events(&mut s);
        assert!(ev.contains(&Event::Program(2, Program::init())));
        assert_eq!(ev.last(), Some(&Event::OpDone));
        assert!(s.is_idle());
    }

    #[test]
    fn ignores_reply_for_another_program() {
        let t = Instant::now();
        let mut s = Session::new(Xd { channel: 0 });
        s.submit(Op::ReadPrograms(vec![5]));
        s.poll_transmit(t);
        s.handle_input(&prog_dump(4));
        assert_eq!(s.poll_transmit(t), None);
        assert!(events(&mut s).is_empty());
    }

    #[test]
    fn nak_stops_a_write() {
        let t = Instant::now();
        let mut s = Session::new(Xd { channel: 0 });
        s.submit(Op::WritePrograms(plan(&[3, 4, 5])));
        s.poll_transmit(t);
        s.handle_input(&ack());
        s.poll_transmit(t);
        s.handle_input(&nak());
        assert_eq!(s.poll_transmit(t), None, "no further programs after a NAK");
        assert!(events(&mut s).contains(&Event::OpFailed(OpError::Nak {
            what: "program write",
            index: Some(4)
        })));
    }

    #[test]
    fn device_error_code_is_reported() {
        let t = Instant::now();
        let mut s = Session::new(Xd { channel: 0 });
        s.submit(Op::ClearSlot {
            module: Module::Osc,
            slot: 3,
        });
        s.poll_transmit(t);
        s.handle_input(&[0xF0, 0x42, 0x30, 0x00, 0x01, 0x51, 0x2D, 0xF7]);
        assert!(matches!(
            events(&mut s)[..],
            [Event::OpFailed(OpError::Device { code: 0x2D, .. })]
        ));
    }

    #[test]
    fn write_times_out_after_three_seconds() {
        let t = Instant::now();
        let mut s = Session::new(Xd { channel: 0 });
        s.submit(Op::WritePrograms(plan(&[7])));
        s.poll_transmit(t);
        s.handle_timeout(t + Duration::from_millis(2900));
        assert!(events(&mut s).is_empty());
        s.handle_timeout(t + Duration::from_millis(3100));
        assert_eq!(
            events(&mut s),
            vec![Event::OpFailed(OpError::Timeout {
                what: "program write",
                index: Some(7)
            })]
        );
        assert!(s.is_idle());
    }

    #[test]
    fn audition_sends_only_current_program_dump() {
        let t = Instant::now();
        let mut s = Session::new(Xd { channel: 2 });
        s.submit(Op::Audition(Box::new(Program::init())));
        let m = s.poll_transmit(t).unwrap();
        assert_eq!(&m[..7], &[0xF0, 0x42, 0x32, 0x00, 0x01, 0x51, 0x40]);
        assert_eq!(m.len(), 1179);
    }

    #[test]
    fn commit_plan_is_sent_whole_then_reads_follow_at_once() {
        let t = Instant::now();
        let mut s = Session::new(Xd { channel: 0 });
        s.submit(Op::WritePrograms(plan(&[1])));
        s.submit(Op::ReadPrograms(vec![1]));
        for _ in 0..5 {
            let m = s.poll_transmit(t).unwrap();
            assert_eq!(m[6], 0x4C);
            s.handle_input(&ack());
        }
        assert_eq!(
            s.poll_transmit(t),
            Some(encode_request(0, &Request::Program(1)))
        );
    }

    #[test]
    fn auditions_do_not_wait() {
        let t = Instant::now();
        let mut s = Session::new(Xd { channel: 0 });
        s.submit(Op::Audition(Box::new(Program::init())));
        s.submit(Op::ReadCurrent);
        s.poll_transmit(t);
        s.handle_input(&ack());
        assert_eq!(
            s.poll_transmit(t),
            Some(encode_request(0, &Request::CurrentProgram))
        );
    }

    #[test]
    fn writes_may_follow_writes() {
        let t = Instant::now();
        let mut s = Session::new(Xd { channel: 0 });
        s.submit(Op::WritePrograms(plan(&[1, 2])));
        s.poll_transmit(t);
        s.handle_input(&ack());
        assert!(s.poll_transmit(t).is_some());
    }

    #[test]
    fn uses_device_channel() {
        let mut s = Session::new(Xd { channel: 9 });
        s.submit(Op::ReadGlobal);
        assert_eq!(s.poll_transmit(Instant::now()).unwrap()[2], 0x39);
    }

    #[test]
    fn read_slots_covers_all_48() {
        let mut s = Session::new(Xd { channel: 0 });
        s.submit(Op::ReadSlots);
        let t = Instant::now();
        let mut sim = crate::sim::SimDevice::new();
        let mut steps = 0;
        while let Some(m) = s.poll_transmit(t) {
            steps += 1;
            for r in sim.handle(&m) {
                s.handle_input(&r);
            }
        }
        assert_eq!(steps, 48);
    }
}
