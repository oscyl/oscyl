// ANCHOR: state
//! A simulated Tutorial Mono. It answers every message as the synth
//! would, with no sound and no MIDI port: the conformance suite and demo
//! mode talk to it.

use oscyl_driver::Simulator;
use oscyl_driver::sysex::DEVICE_INQUIRY;

use crate::msg::{Msg, SLOTS, identity_reply};
use crate::program;

/// The synth's memory: one program per slot, and the edit buffer.
pub struct Sim {
    programs: Vec<Vec<u8>>,
    edit: Vec<u8>,
    /// The fault `lose_reply`: the synth answers nothing.
    lose_reply: bool,
}

impl Default for Sim {
    fn default() -> Sim {
        Sim {
            programs: vec![program::init(); usize::from(SLOTS)],
            edit: program::init(),
            lose_reply: false,
        }
    }
}
// ANCHOR_END: state

// ANCHOR: answer
impl Sim {
    /// The synth's answer to one of its own messages, if it answers.
    fn answer(&mut self, bytes: &[u8]) -> Option<Msg> {
        match Msg::decode(bytes)? {
            Msg::ProgramRequest(n) => {
                let p = self.programs[usize::from(n)].clone();
                Some(Msg::ProgramDump(n, p))
            }
            Msg::EditRequest => Some(Msg::EditDump(self.edit.clone())),
            Msg::ProgramDump(n, p) if program::check(&p).is_ok() => {
                self.programs[usize::from(n)] = p;
                Some(Msg::Ack(Some(n)))
            }
            Msg::EditDump(p) if program::check(&p).is_ok() => {
                self.edit = p;
                Some(Msg::Ack(None))
            }
            // A bad program, or a message only the synth sends.
            _ => None,
        }
    }
}
// ANCHOR_END: answer

// ANCHOR: simulator
impl Simulator for Sim {
    fn handle(&mut self, msg: &[u8]) -> Vec<Vec<u8>> {
        let reply = if msg == DEVICE_INQUIRY {
            Some(identity_reply(0x7F))
        } else {
            self.answer(msg).map(|m| m.encode())
        };
        if self.lose_reply {
            return vec![];
        }
        reply.into_iter().collect()
    }

    /// The Tutorial Mono stores a dump at once: no work waits for idle time.
    fn idle_tick(&mut self) {}

    fn program(&self, slot: u16) -> Option<Vec<u8>> {
        self.programs.get(usize::from(slot)).cloned()
    }

    fn edit_buffer(&self) -> Vec<u8> {
        self.edit.clone()
    }

    fn is_settled(&self) -> bool {
        true
    }

    fn faults(&self) -> &'static [&'static str] {
        &["lose_reply"]
    }

    fn inject(&mut self, fault: &str) {
        match fault {
            "lose_reply" => self.lose_reply = true,
            _ => panic!("unknown fault {fault:?}"),
        }
    }
}
// ANCHOR_END: simulator

// ANCHOR: tests
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_dump_is_stored_and_acknowledged() {
        let mut sim = Sim::default();
        let mut p = program::init();
        program::set_name(&mut p, "Stored").unwrap();
        let reply = sim.handle(&Msg::ProgramDump(5, p.clone()).encode());
        assert_eq!(reply, [Msg::Ack(Some(5)).encode()]);
        assert_eq!(sim.program(5), Some(p));
    }
}
// ANCHOR_END: tests
