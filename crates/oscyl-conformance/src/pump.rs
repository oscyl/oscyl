//! Drives a session against a simulator on a virtual clock: time only
//! jumps to the session's next deadline, so a lost reply times out at once
//! and nothing ever sleeps.

use std::time::Instant;

use oscyl_driver::{DriverSession, SessionEvent, Simulator};

/// Run until the session has nothing queued or in flight. `who` names the
/// driver in the panic raised if the session stalls.
pub fn run_until_idle(
    session: &mut dyn DriverSession,
    sim: &mut dyn Simulator,
    who: &str,
) -> Vec<SessionEvent> {
    let mut now = Instant::now();
    let mut events = vec![];
    loop {
        if let Some(msg) = session.poll_transmit(now) {
            for reply in sim.handle(&msg) {
                session.handle_input(&reply);
            }
        } else if let Some(deadline) = session.next_deadline() {
            now = deadline;
            session.handle_timeout(now);
        } else if session.is_idle() {
            events.extend(std::iter::from_fn(|| session.poll_event()));
            return events;
        } else {
            panic!("{who}: session is busy but has nothing to send and no deadline");
        }
        events.extend(std::iter::from_fn(|| session.poll_event()));
    }
}

/// Idle the simulator until it reports settled.
pub fn settle(sim: &mut dyn Simulator, who: &str) {
    for _ in 0..1000 {
        if sim.is_settled() {
            return;
        }
        sim.idle_tick();
    }
    panic!("{who}: simulator never settled");
}

#[cfg(test)]
mod tests {
    use super::*;
    use oscyl_driver::{DriverError, SessionOp};

    /// Busy forever: not idle, nothing to send, no deadline.
    struct Stuck;
    impl DriverSession for Stuck {
        fn submit(&mut self, _: SessionOp) -> Result<(), DriverError> {
            Ok(())
        }
        fn is_idle(&self) -> bool {
            false
        }
        fn poll_transmit(&mut self, _: Instant) -> Option<Vec<u8>> {
            None
        }
        fn next_deadline(&self) -> Option<Instant> {
            None
        }
        fn handle_timeout(&mut self, _: Instant) {}
        fn handle_input(&mut self, _: &[u8]) {}
        fn poll_event(&mut self) -> Option<SessionEvent> {
            None
        }
    }

    struct Silent;
    impl Simulator for Silent {
        fn handle(&mut self, _: &[u8]) -> Vec<Vec<u8>> {
            vec![]
        }
        fn idle_tick(&mut self) {}
        fn program(&self, _: u16) -> Option<Vec<u8>> {
            None
        }
        fn edit_buffer(&self) -> Vec<u8> {
            vec![]
        }
        fn is_settled(&self) -> bool {
            true
        }
        fn faults(&self) -> &'static [&'static str] {
            &[]
        }
        fn inject(&mut self, _: &str) {}
    }

    #[test]
    #[should_panic(
        expected = "stuck-driver: session is busy but has nothing to send and no deadline"
    )]
    fn stuck_session_fails_instead_of_spinning() {
        run_until_idle(&mut Stuck, &mut Silent, "stuck-driver");
    }
}
