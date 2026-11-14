//! A sans-IO simulated minilogue xd running firmware 2.10.
//!
//! [`Sim::handle`](oscyl_logue::sim::Sim::handle) takes one complete SysEx message and returns the
//! replies the synth would send. Reply bytes follow captures
//! from a real synth (identity and search replies, the extra
//! byte in 0x49 and 0x4A replies), and program writes follow the firmware's
//! store model (`store::StoreModel`):
//! writes land in a 4-program cache whose deferred flash save only happens on
//! [`Sim::idle_tick`](oscyl_logue::sim::Sim::idle_tick) or when a later write forces it. The simulator
//! is the logue family's ([`oscyl_logue::sim::Sim`]) for the xd; it
//! dereferences to it.
//!
//! [`Faults`] injects the failures seen on hardware: the reply-lag fault
//! (a 0x1C read racing the deferred save) and lost replies.

use std::ops::{Deref, DerefMut};

use oscyl_logue::sim::Sim;

use crate::Program;
use crate::model::XdModel;
use crate::msg::{Dump, encode_dump};
#[cfg(test)]
use crate::msg::{GLOBAL_LEN, Module, OCTAVE_LEN, Reply, SCALE_LEN, SLOT_STATUS_LEN, decode};
#[cfg(test)]
use oscyl_logue::sim::{
    USER_DATA_SIZE_ERROR, USER_MODULE_ERROR, USER_SLOT_ERROR, USER_TARGET_ERROR,
};

pub use oscyl_logue::sim::Faults;

/// A simulated minilogue xd (firmware 2.10, MIDI channel 1).
#[derive(Debug, Clone, Default)]
pub struct SimDevice(Sim<XdModel>);

impl Deref for SimDevice {
    type Target = Sim<XdModel>;
    fn deref(&self) -> &Sim<XdModel> {
        &self.0
    }
}

impl DerefMut for SimDevice {
    fn deref_mut(&mut self) -> &mut Sim<XdModel> {
        &mut self.0
    }
}

impl SimDevice {
    /// A synth whose 500 programs are all `Program::init()`, with empty
    /// user slots and a blank global block.
    pub fn new() -> SimDevice {
        SimDevice(Sim::new())
    }

    /// A synth holding `programs` from program 1 up; the rest are
    /// `Program::init()`. Extra programs beyond 500 are ignored.
    pub fn with_library(programs: &[Program]) -> SimDevice {
        SimDevice(Sim::with_library(programs))
    }

    /// The demo synth: [`crate::demo::demo_library`] plus
    /// [`crate::demo::demo_units`] in their slots.
    pub fn demo() -> SimDevice {
        let lib: Vec<Program> = crate::demo::demo_library()
            .into_iter()
            .map(|(p, _)| p)
            .collect();
        let mut sim = SimDevice::with_library(&lib);
        for (slot, unit) in crate::demo::demo_units() {
            let up = encode_dump(
                0,
                &Dump::SlotData {
                    module: unit.module,
                    slot,
                    data: unit.slot_data(),
                },
            );
            sim.handle(&up);
        }
        sim
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::msg::{Request, Version, encode_request};

    fn named(name: &str) -> Program {
        let mut p = Program::init();
        p.set_name(name).unwrap();
        p
    }

    fn one(sim: &mut SimDevice, msg: &[u8]) -> Reply {
        let mut r = sim.handle(msg);
        assert_eq!(r.len(), 1, "{msg:02x?}");
        decode(&r.remove(0)).unwrap()
    }

    fn req(sim: &mut SimDevice, r: Request) -> Reply {
        one(sim, &encode_request(0, &r))
    }

    fn read(sim: &mut SimDevice, n: u16) -> Vec<u8> {
        match req(sim, Request::Program(n)) {
            Reply::Dump(Dump::Program(m, d)) if m == n => d,
            r => panic!("{r:?}"),
        }
    }

    fn write(sim: &mut SimDevice, n: u16, p: &Program) -> Reply {
        one(
            sim,
            &encode_dump(0, &Dump::Program(n, p.as_bytes().to_vec())),
        )
    }

    #[test]
    fn identity_and_search_match_hardware_bytes() {
        let mut sim = SimDevice::new();
        assert_eq!(
            sim.handle(&encode_request(0, &Request::Identity)),
            [[
                0xF0, 0x7E, 0x00, 0x06, 0x02, 0x42, 0x51, 0x01, 0x00, 0x00, 0x02, 0x00, 0x0A, 0x00,
                0xF7
            ]]
        );
        assert_eq!(
            sim.handle(&encode_request(0, &Request::Search(0))),
            [[
                0xF0, 0x42, 0x50, 0x01, 0x00, 0x00, 0x51, 0x01, 0x00, 0x00, 0x02, 0x00, 0x0A, 0x00,
                0xF7
            ]]
        );
        let fw = Version {
            major: 2,
            minor: 10,
        };
        assert_eq!(
            req(&mut sim, Request::Search(0x11)),
            Reply::Search {
                channel: 0,
                echo: 0x11,
                version: fw
            }
        );
    }

    #[test]
    fn ignores_other_channels_and_garbage() {
        let mut sim = SimDevice::new();
        assert!(sim.handle(&encode_request(3, &Request::Global)).is_empty());
        assert!(sim.handle(&[0x90, 0x40, 0x7F]).is_empty());
        assert!(
            sim.handle(&[0xF0, 0x42, 0x30, 0x00, 0x01, 0x51, 0x7E, 0xF7])
                .is_empty()
        );
    }

    #[test]
    fn reads_library_global_and_tuning() {
        let mut sim = SimDevice::with_library(&[named("A"), named("B")]);
        assert_eq!(Program::from_bytes(&read(&mut sim, 1)).unwrap().name(), "B");
        assert_eq!(&read(&mut sim, 499)[..], Program::init().as_bytes());
        assert!(
            matches!(req(&mut sim, Request::Global), Reply::Dump(Dump::Global(g)) if g.starts_with(b"GLOB"))
        );
        assert!(matches!(
            req(&mut sim, Request::CurrentProgram),
            Reply::Dump(Dump::CurrentProgram(_))
        ));
        for s in 0..6 {
            assert!(
                matches!(req(&mut sim, Request::Scale(s)), Reply::Dump(Dump::Scale(t, v)) if t == s && v.len() == SCALE_LEN)
            );
            assert!(
                matches!(req(&mut sim, Request::Octave(s)), Reply::Dump(Dump::Octave(t, v)) if t == s && v.len() == OCTAVE_LEN)
            );
        }
        assert!(
            sim.handle(&encode_request(0, &Request::Program(500)))
                .is_empty()
        );
    }

    #[test]
    fn writes_ack_and_bad_data_naks() {
        let mut sim = SimDevice::new();
        let audition = encode_dump(0, &Dump::CurrentProgram(named("Aud").as_bytes().to_vec()));
        assert_eq!(one(&mut sim, &audition), Reply::Ack);
        assert_eq!(sim.edit_buffer().name(), "Aud");
        assert!(!sim.save_pending(), "auditions never touch the store");
        assert_eq!(write(&mut sim, 500, &Program::init()), Reply::Nak);
        let junk = encode_dump(0, &Dump::Program(3, vec![0; 1024]));
        assert_eq!(one(&mut sim, &junk), Reply::Nak);
        let mut g = vec![1; GLOBAL_LEN];
        assert_eq!(
            one(&mut sim, &encode_dump(0, &Dump::Global(g.clone()))),
            Reply::Nak
        );
        g[..4].copy_from_slice(b"GLOB");
        assert_eq!(
            one(&mut sim, &encode_dump(0, &Dump::Global(g.clone()))),
            Reply::Ack
        );
        assert_eq!(req(&mut sim, Request::Global), Reply::Dump(Dump::Global(g)));
        let sc = vec![5; SCALE_LEN];
        assert_eq!(
            one(&mut sim, &encode_dump(0, &Dump::Scale(2, sc.clone()))),
            Reply::Ack
        );
        assert_eq!(
            req(&mut sim, Request::Scale(2)),
            Reply::Dump(Dump::Scale(2, sc))
        );
    }

    #[test]
    fn store_model_caches_saves_and_drops_the_fifth_write() {
        let mut sim = SimDevice::new();
        for k in 0..4 {
            assert_eq!(write(&mut sim, 17, &named(&format!("v{k}"))), Reply::Ack);
            assert!(sim.save_pending());
        }
        // The 5th write into the cached group saves and is dropped.
        assert_eq!(write(&mut sim, 17, &named("dropped")), Reply::Ack);
        assert!(!sim.save_pending());
        assert_eq!(sim.program(17).unwrap().name(), "v3");
        // A write into another group saves the previous one.
        write(&mut sim, 1, &named("x"));
        write(&mut sim, 30, &named("y"));
        assert!(sim.save_pending());
        sim.idle_tick();
        assert!(!sim.save_pending());
        // 499 as the 4th write of its group re-caches instead of saving.
        for n in 496..=499 {
            write(&mut sim, n, &named("z"));
        }
        assert!(sim.save_pending());
    }

    #[test]
    fn commit_plan_leaves_reads_safe_even_with_the_race_fault() {
        let mut sim = SimDevice::new();
        sim.faults.race_deferred_save = true;
        let plan = crate::store::plan_commit([(42, named("Kept"))], |_| None).unwrap();
        for (n, p) in plan.writes() {
            assert_eq!(write(&mut sim, *n, p), Reply::Ack);
        }
        assert!(!sim.save_pending());
        assert_eq!(
            Program::from_bytes(&read(&mut sim, 42)).unwrap().name(),
            "Kept"
        );
        assert!(!sim.is_lagging());
    }

    #[test]
    fn read_racing_a_deferred_save_lags_until_the_next_write() {
        let mut sim = SimDevice::with_library(&[named("P0"), named("P1"), named("P2")]);
        write(&mut sim, 2, &named("P2")); // deferred save pending
        // Without the fault, the read is simply answered.
        assert_eq!(
            Program::from_bytes(&read(&mut sim, 0)).unwrap().name(),
            "P0"
        );
        sim.faults.race_deferred_save = true;
        assert!(
            Program::from_bytes(&read(&mut sim, 0)).is_err(),
            "corrupt reply"
        );
        assert!(sim.is_lagging() && !sim.save_pending());
        let name = |d: Vec<u8>| Program::from_bytes(&d).unwrap().name();
        // Reading 1, 1, 2: each reply carries the previous read's data.
        assert_eq!(name(read(&mut sim, 1)), "P0");
        assert_eq!(name(read(&mut sim, 1)), "P1");
        assert_eq!(name(read(&mut sim, 2)), "P1");
        // Identity/global/current do not clear it.
        req(&mut sim, Request::Identity);
        req(&mut sim, Request::Global);
        assert_eq!(name(read(&mut sim, 0)), "P2");
        // A program write does.
        write(&mut sim, 5, &named("W"));
        assert!(!sim.is_lagging());
        sim.faults.race_deferred_save = false;
        assert_eq!(name(read(&mut sim, 1)), "P1");
    }

    #[test]
    fn drops_every_nth_reply() {
        let mut sim = SimDevice::new();
        sim.faults.drop_every_nth_reply = Some(3);
        let got: Vec<usize> = (0..6)
            .map(|_| sim.handle(&encode_request(0, &Request::Global)).len())
            .collect();
        assert_eq!(got, [1, 1, 0, 1, 1, 0]);
    }

    fn upload(sim: &mut SimDevice, module: Module, slot: u8, data: Vec<u8>) -> Reply {
        one(sim, &encode_dump(0, &Dump::SlotData { module, slot, data }))
    }

    #[test]
    fn unit_slots_upload_status_read_swap_clear() {
        let mut sim = SimDevice::new();
        let (_, unit) = crate::demo::demo_units()
            .into_iter()
            .find(|(_, u)| u.name == "waves")
            .unwrap();
        let slot = |m, slot| Request::SlotStatus { module: m, slot };
        assert_eq!(
            sim.handle(&encode_request(0, &slot(Module::Osc, 9))),
            [[0xF0, 0x42, 0x30, 0x00, 0x01, 0x51, 0x49, 0x04, 0x09, 0xF7]]
        );
        assert_eq!(
            upload(&mut sim, Module::Osc, 9, unit.slot_data()),
            Reply::Ack
        );
        let occupied = sim
            .handle(&encode_request(0, &slot(Module::Osc, 9)))
            .remove(0);
        assert_eq!(occupied.len(), 48);
        assert_eq!(occupied[9], 0, "extra byte");
        assert_eq!(
            decode(&occupied).unwrap(),
            Reply::SlotStatus {
                module: Module::Osc,
                slot: 9,
                header: Some(unit.header[..SLOT_STATUS_LEN].to_vec())
            }
        );
        let back = sim
            .handle(&encode_request(
                0,
                &Request::SlotData {
                    module: Module::Osc,
                    slot: 9,
                },
            ))
            .remove(0);
        assert_eq!(back[9], 0, "extra byte");
        let Reply::Dump(Dump::SlotData { data, .. }) = decode(&back).unwrap() else {
            panic!()
        };
        assert_eq!(data[8..], unit.slot_data()[8..]);
        assert_ne!(data[4..8], [0; 4], "the synth stores its own checksum");

        let swap = Request::SwapSlots {
            module: Module::Osc,
            a: 9,
            b: 3,
        };
        assert_eq!(req(&mut sim, swap), Reply::Ack);
        assert!(matches!(
            req(&mut sim, slot(Module::Osc, 3)),
            Reply::SlotStatus {
                header: Some(_),
                ..
            }
        ));
        let clear = Request::ClearSlot {
            module: Module::Osc,
            slot: 3,
        };
        assert_eq!(req(&mut sim, clear), Reply::Ack);
        assert!(matches!(
            req(&mut sim, slot(Module::Osc, 3)),
            Reply::SlotStatus { header: None, .. }
        ));
        assert_eq!(
            req(
                &mut sim,
                Request::SlotData {
                    module: Module::Osc,
                    slot: 3
                }
            ),
            Reply::Nak
        );
    }

    #[test]
    fn unit_errors_use_korg_status_codes() {
        let mut sim = SimDevice::new();
        let (_, unit) = crate::demo::demo_units().remove(0);
        let raw = |m: u8, s: u8| [0xF0, 0x42, 0x30, 0x00, 0x01, 0x51, 0x19, m, s, 0xF7];
        assert_eq!(one(&mut sim, &raw(7, 0)), Reply::Error(USER_MODULE_ERROR));
        let clear = Request::ClearSlot {
            module: Module::DelFx,
            slot: 8,
        };
        assert_eq!(req(&mut sim, clear), Reply::Error(USER_SLOT_ERROR));
        assert_eq!(
            upload(&mut sim, Module::Osc, 16, unit.slot_data()),
            Reply::Error(USER_SLOT_ERROR)
        );
        assert_eq!(
            upload(&mut sim, Module::RevFx, 0, unit.slot_data()),
            Reply::Error(USER_TARGET_ERROR)
        );
        let mut bad = unit.slot_data();
        bad[0] ^= 1;
        assert_eq!(
            upload(&mut sim, Module::Osc, 0, bad),
            Reply::Error(USER_DATA_SIZE_ERROR)
        );
    }

    #[test]
    fn demo_device_holds_the_demo_library_and_units() {
        let mut sim = SimDevice::demo();
        assert_eq!(sim.program(0).unwrap().name(), "Glass Choir");
        assert!(!sim.save_pending());
        let st = Request::SlotStatus {
            module: Module::Osc,
            slot: 0,
        };
        assert!(matches!(
            req(&mut sim, st),
            Reply::SlotStatus {
                header: Some(_),
                ..
            }
        ));
    }
}
