//! A sans-IO simulated Prophet Rev2.
//! Seeded with [`Program::init`], never factory data.

use std::collections::HashMap;

use oscyl_driver::Simulator;
use oscyl_driver::sysex::Framer;

use crate::Program;
use crate::msg::{BANK_LEN, Reply, SLOTS, decode, encode_edit_dump, encode_program_dump};

/// Device inquiry reply: channel All, OS version byte 1.
const IDENTITY: [u8; 12] = [
    0xF0, 0x7E, 0x7F, 0x06, 0x02, 0x01, 0x2F, 0x01, 0x00, 0x00, 0x01, 0xF7,
];

/// Faults [`Simulator::inject`] switches on. None is documented for the
/// hardware:
/// - `lose_reply`: every reply is lost (models a wrong MIDI SysEx Cable
///   setting).
/// - `answer_before_store`: a program request arriving in the same buffer
///   as a dump of that slot is answered with the slot's old contents.
const FAULTS: &[&str] = &["lose_reply", "answer_before_store"];

pub struct SimRev2 {
    programs: Vec<Program>,
    edit: Program,
    /// A received program dump also replaces the edit buffer, as the
    /// factory ReadMe suggests ("display will increment"). Unverified.
    pub dump_selects_program: bool,
    lose_reply: bool,
    answer_before_store: bool,
}

impl Default for SimRev2 {
    fn default() -> SimRev2 {
        SimRev2 {
            programs: vec![Program::init(); usize::from(SLOTS)],
            edit: Program::init(),
            dump_selects_program: true,
            lose_reply: false,
            answer_before_store: false,
        }
    }
}

impl SimRev2 {
    /// One message, in arrival order. `old` holds the contents of slots
    /// stored earlier in the same buffer, for `answer_before_store`.
    fn message(&mut self, msg: &[u8], old: &mut HashMap<u16, Program>) -> Option<Vec<u8>> {
        match msg {
            [0xF0, 0x7E, ch, 0x06, 0x01, 0xF7] if *ch == 0x7F || *ch < 0x10 => {
                Some(IDENTITY.to_vec())
            }
            // Out-of-range requests are ignored: unknown on hardware, and
            // silence makes the driver time out, the safe outcome.
            [0xF0, 0x01, 0x2F, 0x05, bank, prog, 0xF7] if *bank < 8 && *prog < 0x80 => {
                let n = u16::from(*bank) * BANK_LEN + u16::from(*prog);
                let p = old.get(&n).unwrap_or(&self.programs[usize::from(n)]);
                Some(encode_program_dump(n, p))
            }
            [0xF0, 0x01, 0x2F, 0x06, 0xF7] => Some(encode_edit_dump(&self.edit)),
            _ => {
                match decode(msg) {
                    Ok(Reply::Program(n, p)) => {
                        let slot = &mut self.programs[usize::from(n)];
                        if self.answer_before_store {
                            old.entry(n).or_insert(*slot);
                        }
                        *slot = p;
                        if self.dump_selects_program {
                            self.edit = p;
                        }
                    }
                    Ok(Reply::Edit(p)) => self.edit = p,
                    _ => {}
                }
                None
            }
        }
    }
}

impl Simulator for SimRev2 {
    /// `msg` may hold several messages: a write step sends a dump and a
    /// request together.
    fn handle(&mut self, msg: &[u8]) -> Vec<Vec<u8>> {
        let mut old = HashMap::new();
        let replies: Vec<Vec<u8>> = Framer::new()
            .push(msg)
            .iter()
            .filter_map(|m| self.message(m, &mut old))
            .collect();
        if self.lose_reply { vec![] } else { replies }
    }

    fn idle_tick(&mut self) {}

    fn program(&self, slot: u16) -> Option<Vec<u8>> {
        self.programs
            .get(usize::from(slot))
            .map(|p| p.as_bytes().to_vec())
    }

    fn edit_buffer(&self) -> Vec<u8> {
        self.edit.as_bytes().to_vec()
    }

    /// No pending save state is known for the Rev2.
    fn is_settled(&self) -> bool {
        true
    }

    fn faults(&self) -> &'static [&'static str] {
        FAULTS
    }

    fn inject(&mut self, fault: &str) {
        match fault {
            "lose_reply" => self.lose_reply = true,
            "answer_before_store" => self.answer_before_store = true,
            _ => panic!("unknown fault {fault:?}"),
        }
    }
}
