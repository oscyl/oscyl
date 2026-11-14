//! The minilogue xd session and simulator behind the generic contract.
//!
//! Both are the logue family's ([`oscyl_logue::lag::LogueSession`],
//! [`oscyl_logue::sim`]) for the xd: commit planning, the read-lag check
//! and recovery are shared with the prologue.

use oscyl_driver::Simulator;
use oscyl_logue::lag::LogueSession;

use crate::model::XdModel;
use crate::sim::SimDevice;

/// The xd session: writes go through `store::plan_commit`, and the
/// firmware's "one reply behind" state is detected and cleared before the
/// first op, as `oscyl_midi::Port` does on connect.
pub type XdSession = LogueSession<XdModel>;

/// [`SimDevice`] behind the generic [`Simulator`].
#[derive(Default)]
pub struct XdSimulator(pub SimDevice);

impl Simulator for XdSimulator {
    fn handle(&mut self, msg: &[u8]) -> Vec<Vec<u8>> {
        self.0.handle(msg)
    }

    fn idle_tick(&mut self) {
        self.0.idle_tick()
    }

    fn program(&self, slot: u16) -> Option<Vec<u8>> {
        self.0.program(slot).map(|p| p.as_bytes().to_vec())
    }

    fn edit_buffer(&self) -> Vec<u8> {
        self.0.edit_buffer().as_bytes().to_vec()
    }

    fn is_settled(&self) -> bool {
        !self.0.save_pending() && !self.0.is_lagging()
    }

    fn faults(&self) -> &'static [&'static str] {
        oscyl_logue::sim::FAULTS
    }

    fn inject(&mut self, fault: &str) {
        self.0.inject(fault)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Program;
    use oscyl_driver::{DriverSession, SessionEvent, SessionOp, Simulator};
    use std::time::Instant;

    fn pump(s: &mut XdSession, sim: &mut XdSimulator) -> Vec<SessionEvent> {
        let mut now = Instant::now();
        let mut out = vec![];
        loop {
            if let Some(m) = s.poll_transmit(now) {
                for r in sim.handle(&m) {
                    s.handle_input(&r);
                }
            } else if let Some(d) = s.next_deadline() {
                now = d;
                s.handle_timeout(now);
            } else if s.is_idle() {
                break;
            }
            out.extend(std::iter::from_fn(|| s.poll_event()));
        }
        out.extend(std::iter::from_fn(|| s.poll_event()));
        out
    }

    fn named(name: &str) -> Vec<u8> {
        let mut p = Program::init();
        p.set_name(name).unwrap();
        p.as_bytes().to_vec()
    }

    #[test]
    fn read_write_round_trip() {
        let (mut s, mut sim) = (XdSession::new(0), XdSimulator::default());
        s.submit(SessionOp::WritePrograms(vec![(3, named("Three"))]))
            .unwrap();
        assert_eq!(pump(&mut s, &mut sim).last(), Some(&SessionEvent::Done));
        assert_eq!(sim.program(3), Some(named("Three")));
        assert!(sim.is_settled());
        s.submit(SessionOp::ReadPrograms(vec![3])).unwrap();
        assert!(pump(&mut s, &mut sim).contains(&SessionEvent::Program(3, named("Three"))));
    }

    #[test]
    fn audition_sets_edit_buffer() {
        let (mut s, mut sim) = (XdSession::new(0), XdSimulator::default());
        s.submit(SessionOp::Audition(named("Try"))).unwrap();
        assert_eq!(pump(&mut s, &mut sim).last(), Some(&SessionEvent::Done));
        assert_eq!(sim.edit_buffer(), named("Try"));
        s.submit(SessionOp::ReadCurrent).unwrap();
        assert!(pump(&mut s, &mut sim).contains(&SessionEvent::Current(named("Try"))));
    }

    #[test]
    fn write_499_alone_uses_neighbour_read_earlier() {
        // Programs 0 and 1 differ, so the lag check can vouch for the read
        // (on an all-identical synth it cannot, and the write is refused).
        let lib = [
            Program::init(),
            Program::from_bytes(&named("Other")).unwrap(),
        ];
        let (mut s, mut sim) = (
            XdSession::new(0),
            XdSimulator(SimDevice::with_library(&lib)),
        );
        s.submit(SessionOp::ReadPrograms(vec![498])).unwrap();
        pump(&mut s, &mut sim);
        s.submit(SessionOp::WritePrograms(vec![(499, named("Last"))]))
            .unwrap();
        assert_eq!(pump(&mut s, &mut sim).last(), Some(&SessionEvent::Done));
        assert_eq!(sim.program(499), Some(named("Last")));
        assert!(sim.is_settled());
    }

    #[test]
    fn write_499_alone_after_writing_neighbour_keeps_that_write() {
        let (mut s, mut sim) = (XdSession::new(0), XdSimulator::default());
        s.submit(SessionOp::ReadPrograms(vec![498])).unwrap();
        pump(&mut s, &mut sim);
        s.submit(SessionOp::WritePrograms(vec![(498, named("New498"))]))
            .unwrap();
        pump(&mut s, &mut sim);
        s.submit(SessionOp::WritePrograms(vec![(499, named("Last"))]))
            .unwrap();
        assert_eq!(pump(&mut s, &mut sim).last(), Some(&SessionEvent::Done));
        assert_eq!(sim.program(498), Some(named("New498")));
        assert_eq!(sim.program(499), Some(named("Last")));
    }

    #[test]
    fn failed_write_forgets_the_neighbour() {
        let (mut s, mut sim) = (XdSession::new(0), XdSimulator::default());
        s.submit(SessionOp::ReadPrograms(vec![498])).unwrap();
        pump(&mut s, &mut sim);
        sim.inject("lose_reply");
        s.submit(SessionOp::WritePrograms(vec![(498, named("New498"))]))
            .unwrap();
        assert!(matches!(
            pump(&mut s, &mut sim).last(),
            Some(SessionEvent::Failed(_))
        ));
        assert!(
            s.submit(SessionOp::WritePrograms(vec![(499, named("Last"))]))
                .is_err(),
            "498's contents are unknown after a failed write"
        );
    }

    #[test]
    fn write_499_alone_without_neighbour_is_rejected() {
        let mut s = XdSession::new(0);
        assert!(
            s.submit(SessionOp::WritePrograms(vec![(499, named("Last"))]))
                .is_err()
        );
    }

    #[test]
    fn write_out_of_range_is_rejected_before_sending() {
        let mut s = XdSession::new(0);
        assert!(
            s.submit(SessionOp::WritePrograms(vec![(500, named("X"))]))
                .is_err()
        );
        assert!(s.submit(SessionOp::ReadPrograms(vec![500])).is_err());
        assert_eq!(s.poll_transmit(Instant::now()), None);
        assert!(s.is_idle());
    }

    #[test]
    fn faults_are_injectable() {
        let mut sim = XdSimulator::default();
        assert_eq!(
            sim.faults(),
            &["race_deferred_save", "drop_every_3rd_reply", "lose_reply"]
        );
        sim.inject("drop_every_3rd_reply");
        assert_eq!(sim.0.faults.drop_every_nth_reply, Some(3));
    }
}
