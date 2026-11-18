//! Golden digests of the xd's observable bytes, recorded before the
//! logue-family code moved into `oscyl-logue`. Any change in a commit
//! plan, an encoding, a decoded reply, a simulator conversation, a library
//! or a unit changes a digest. A digest may change only with a reviewed
//! behaviour change, never in a refactor.

use std::io::Cursor;

use oscyl_driver::{DriverSession, SessionOp, Simulator};
use oscyl_korg_minilogue_xd::adapter::{XdSession, XdSimulator};
use oscyl_korg_minilogue_xd::library::{Entry, Library, ProgInfo, from_data, to_data};
use oscyl_korg_minilogue_xd::msg::{Dump, Module, Request, decode, encode_dump, encode_request};
use oscyl_korg_minilogue_xd::sim::SimDevice;
use oscyl_korg_minilogue_xd::store::{StoreModel, plan_commit};
use oscyl_korg_minilogue_xd::unit::{Unit, build_header};
use oscyl_korg_minilogue_xd::{Field, Program};

/// FNV-1a, 64 bit: stable across Rust releases, unlike `DefaultHasher`.
fn fnv(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, &b| {
        (h ^ u64::from(b)).wrapping_mul(0x0000_0100_0000_01B3)
    })
}

/// A log of everything a test observed, digested at the end.
#[derive(Default)]
struct Log(Vec<u8>);

impl Log {
    fn bytes(&mut self, b: &[u8]) {
        self.0.extend((b.len() as u32).to_le_bytes());
        self.0.extend(b);
    }
    fn text(&mut self, t: impl std::fmt::Debug) {
        self.bytes(format!("{t:?}").as_bytes());
    }
    fn check(&self, want: u64) {
        let got = fnv(&self.0);
        assert_eq!(got, want, "digest {got:#018x} over {} bytes", self.0.len());
    }
}

fn named(name: &str) -> Program {
    let mut p = Program::init();
    p.set_name(name).unwrap();
    p
}

fn batches() -> Vec<Vec<u16>> {
    (0..500)
        .map(|n| vec![n])
        .chain((0..500).step_by(7).map(|n| vec![n, (n * 13 + 5) % 500]))
        .chain([
            (490..=499).collect(),
            (8..12).collect(),
            vec![3, 499],
            vec![499],
        ])
        .chain([vec![5, 5], vec![16, 17, 30], vec![28, 29, 30]])
        .collect()
}

#[test]
fn commit_plans() {
    let mut log = Log::default();
    let known = |n: u16| n.is_multiple_of(3).then(|| named(&format!("K{n}")));
    for b in batches() {
        let batch = b.iter().map(|&n| (n, named(&format!("P{n}"))));
        for plan in [
            plan_commit(batch.clone(), known),
            plan_commit(batch, |_| None),
        ] {
            match plan {
                Ok(p) => {
                    log.text(p.real());
                    for (n, prog) in p.writes() {
                        log.text(n);
                        log.bytes(prog.as_bytes());
                    }
                }
                Err(e) => log.text(e.to_string()),
            }
        }
    }
    log.text(plan_commit([(500, Program::init())], |_| None).map_err(|e| e.to_string()));
    log.check(0x8ce3_9fb6_4227_3f48);
}

#[test]
fn store_effects() {
    let mut log = Log::default();
    for b in batches() {
        let mut m = StoreModel::clean();
        for &n in b.iter().chain(&b).chain(&b) {
            log.text((m.write(n), m.is_clean()));
        }
    }
    log.check(0x41ea_dc63_af8a_1a9d);
}

fn requests() -> Vec<Request> {
    use Request::*;
    let mut r = vec![Identity, Search(0x11), Global, CurrentProgram];
    r.extend([0, 1, 127, 128, 300, 499].map(Program));
    r.extend([0, 5, 0x7F].map(Scale));
    r.extend([0, 5, 0x7F].map(Octave));
    for module in Module::ALL {
        r.push(SlotStatus { module, slot: 3 });
        r.push(SlotData { module, slot: 7 });
        r.push(ClearSlot { module, slot: 1 });
        r.push(SwapSlots { module, a: 2, b: 5 });
    }
    r
}

fn dumps() -> Vec<Dump> {
    let bytes = |n: usize| (0..n).map(|i| (i * 31) as u8).collect::<Vec<u8>>();
    let seven = |n: usize| (0..n).map(|i| (i * 31) as u8 & 0x7F).collect::<Vec<u8>>();
    vec![
        Dump::CurrentProgram(named("Cur").as_bytes().to_vec()),
        Dump::Program(0, named("Zero").as_bytes().to_vec()),
        Dump::Program(499, named("Last").as_bytes().to_vec()),
        Dump::Program(300, bytes(1024)),
        Dump::Global(bytes(80)),
        Dump::Scale(0x7F, seven(384)),
        Dump::Octave(5, seven(36)),
        Dump::SlotData {
            module: Module::Osc,
            slot: 9,
            data: bytes(77),
        },
    ]
}

#[test]
fn encodings() {
    let mut log = Log::default();
    for ch in [0, 5, 15] {
        for r in requests() {
            let m = encode_request(ch, &r);
            log.bytes(&m);
            log.text(decode(&m));
        }
        for d in dumps() {
            let m = encode_dump(ch, &d);
            log.bytes(&m);
            log.text(decode(&m));
        }
        for f in [0x23, 0x24, 0x26, 0x27, 0x2F, 0x30] {
            log.text(decode(&[0xF0, 0x42, 0x30 | ch, 0x00, 0x01, 0x51, f, 0xF7]));
        }
    }
    for m in [
        &[
            0xF0, 0x7E, 0x00, 0x06, 0x02, 0x42, 0x51, 0x01, 0x00, 0x00, 0x02, 0x00, 0x0A, 0x00,
            0xF7,
        ][..],
        &[
            0xF0, 0x42, 0x50, 0x01, 0x13, 0x07, 0x51, 0x01, 0x00, 0x00, 0x02, 0x00, 0x0A, 0x00,
            0xF7,
        ],
        &[
            0xF0, 0x7E, 0x00, 0x06, 0x02, 0x42, 0x4B, 0x01, 0x00, 0x00, 0x02, 0x00, 0x0A, 0x00,
            0xF7,
        ],
        &[0xF0, 0x42, 0x30, 0x00, 0x01, 0x51, 0x49, 0x04, 0x09, 0xF7],
        &[
            0xF0, 0x42, 0x30, 0x00, 0x01, 0x51, 0x4A, 0x04, 0x09, 0x00, 0x01, 0x02, 0xF7,
        ],
        &[0xF0, 0x42, 0x30, 0x00, 0x01, 0x51, 0x40, 0x00, 0x01, 0xF7],
        &[0xF0, 0x42, 0x30, 0x00, 0x01, 0x51, 0x49, 0x09, 0x09, 0xF7],
        &[0x90, 0x40, 0x7F],
        &[],
    ] {
        log.text(decode(m));
    }
    log.check(0x8d5c_b7af_9e81_1105);
}

fn waves() -> serde_json::Value {
    serde_json::json!({"header": {
        "platform": "minilogue-xd", "module": "osc", "api": "1.1-0",
        "dev_id": 7, "prg_id": 9, "version": "1.0-1", "name": "waves", "num_param": 6,
        "params": [["Wave A", 0, 45, ""], ["Wave B", 0, 43, ""], ["Sub Wave", 0, 15, ""],
                   ["Sub Mix", 0, 100, "%"], ["Ring Mix", -100, 100, "%"], ["Bit Crush", 0, 100, "%"]]
    }})
}

#[test]
fn sim_conversation() {
    let mut log = Log::default();
    let mut sim = SimDevice::with_library(&[named("P0"), named("P1"), named("P2")]);
    let say = |sim: &mut SimDevice, m: Vec<u8>, log: &mut Log| {
        for r in sim.handle(&m) {
            log.bytes(&r);
        }
        log.text((sim.save_pending(), sim.is_lagging()));
    };
    for ch in [0, 3] {
        for r in requests() {
            say(&mut sim, encode_request(ch, &r), &mut log);
        }
    }
    for d in dumps() {
        say(&mut sim, encode_dump(0, &d), &mut log);
    }
    for k in 0..6 {
        let d = Dump::Program(17, named(&format!("v{k}")).as_bytes().to_vec());
        say(&mut sim, encode_dump(0, &d), &mut log);
    }
    for n in 490..=499 {
        let d = Dump::Program(n, named(&format!("w{n}")).as_bytes().to_vec());
        say(&mut sim, encode_dump(0, &d), &mut log);
    }
    sim.faults.race_deferred_save = true;
    for n in [0, 1, 1, 2, 0] {
        say(&mut sim, encode_request(0, &Request::Program(n)), &mut log);
    }
    say(&mut sim, encode_request(0, &Request::Global), &mut log);
    let d = Dump::Program(5, named("W").as_bytes().to_vec());
    say(&mut sim, encode_dump(0, &d), &mut log);
    sim.idle_tick();
    for n in [1, 2] {
        say(&mut sim, encode_request(0, &Request::Program(n)), &mut log);
    }
    let (module, name, header) = build_header(&waves(), 8).unwrap();
    let unit = Unit {
        module,
        name,
        header,
        payload: b"UOSCxxxx".to_vec(),
    };
    for (m, s) in [(Module::Osc, 9), (Module::Osc, 16), (Module::RevFx, 0)] {
        let d = Dump::SlotData {
            module: m,
            slot: s,
            data: unit.slot_data(),
        };
        say(&mut sim, encode_dump(0, &d), &mut log);
    }
    for r in [
        Request::SlotStatus {
            module: Module::Osc,
            slot: 9,
        },
        Request::SlotData {
            module: Module::Osc,
            slot: 9,
        },
        Request::SwapSlots {
            module: Module::Osc,
            a: 9,
            b: 3,
        },
        Request::SlotStatus {
            module: Module::Osc,
            slot: 3,
        },
        Request::ClearSlot {
            module: Module::Osc,
            slot: 3,
        },
        Request::SlotData {
            module: Module::Osc,
            slot: 3,
        },
        Request::ClearSlot {
            module: Module::DelFx,
            slot: 8,
        },
    ] {
        say(&mut sim, encode_request(0, &r), &mut log);
    }
    say(
        &mut sim,
        vec![0xF0, 0x42, 0x30, 0x00, 0x01, 0x51, 0x19, 7, 0, 0xF7],
        &mut log,
    );
    say(
        &mut sim,
        vec![0xF0, 0x42, 0x30, 0x00, 0x01, 0x51, 0x1D, 4, 0xF7],
        &mut log,
    );
    sim.faults.drop_every_nth_reply = Some(3);
    for _ in 0..6 {
        say(&mut sim, encode_request(0, &Request::Global), &mut log);
    }
    for n in 0..500 {
        log.bytes(sim.program(n).unwrap().as_bytes());
    }
    log.bytes(sim.edit_buffer().as_bytes());
    let demo = SimDevice::demo();
    log.bytes(demo.program(0).unwrap().as_bytes());
    log.check(0x5593_3af3_7ba0_f004);
}

fn pump(s: &mut dyn DriverSession, sim: &mut dyn Simulator, log: &mut Log) {
    let mut now = std::time::Instant::now();
    loop {
        if let Some(m) = s.poll_transmit(now) {
            log.bytes(&m);
            for r in sim.handle(&m) {
                log.bytes(&r);
                s.handle_input(&r);
            }
        } else if let Some(d) = s.next_deadline() {
            now = d;
            s.handle_timeout(now);
        } else if s.is_idle() {
            break;
        }
        while let Some(e) = s.poll_event() {
            log.text(e);
        }
    }
    while let Some(e) = s.poll_event() {
        log.text(e);
    }
}

#[test]
fn session_events() {
    let mut log = Log::default();
    let ops = || {
        vec![
            SessionOp::ReadPrograms(vec![498]),
            SessionOp::WritePrograms(vec![
                (3, named("Three").as_bytes().to_vec()),
                (499, named("Last").as_bytes().to_vec()),
            ]),
            SessionOp::WritePrograms(vec![(499, named("Alone").as_bytes().to_vec())]),
            SessionOp::ReadPrograms(vec![3, 499, 0]),
            SessionOp::Audition(named("Try").as_bytes().to_vec()),
            SessionOp::ReadCurrent,
        ]
    };
    let faults = XdSimulator::default().faults();
    for fault in [None].into_iter().chain(faults.iter().map(Some)) {
        let (mut s, mut sim) = (XdSession::new(2), XdSimulator::default());
        if let Some(f) = fault {
            sim.inject(f);
        }
        for op in ops() {
            log.text(s.submit(op).map_err(|e| e.to_string()));
            pump(&mut s, &mut sim, &mut log);
        }
        log.text(sim.is_settled());
    }
    // A synth that lags before the session opens.
    let (mut s, mut sim) = (XdSession::new(0), XdSimulator::default());
    let w = encode_dump(0, &Dump::Program(2, named("P2").as_bytes().to_vec()));
    sim.handle(&w);
    sim.inject("race_deferred_save");
    sim.handle(&encode_request(0, &Request::Program(0)));
    s.submit(SessionOp::ReadPrograms(vec![0, 1])).unwrap();
    pump(&mut s, &mut sim, &mut log);
    log.text(sim.is_settled());
    log.check(0x15f4_45fc_b0f9_1455);
}

#[test]
fn library_bytes() {
    let mut log = Log::default();
    let mut lib = Library::default();
    let info = |p: &str, c: &str| ProgInfo {
        programmer: p.into(),
        comment: c.into(),
    };
    let mut edited = named("Two");
    edited.set(Field::Cutoff, 300).unwrap();
    lib.push_program(&named("One"), &info("Ann & Co", "<first>"));
    lib.push_program(&edited, &info("Bob", ""));
    lib.push_program(&Program::init(), &ProgInfo::default());
    lib.entries.push(Entry {
        kind: "FavoriteData".into(),
        fields: vec![("FavoriteBinary".into(), "Fav_000.fav_data".into())],
    });
    lib.files.insert("Fav_000.fav_data".into(), vec![1, 2, 3]);
    lib.files.insert("Unknown/member.bin".into(), vec![9]);
    let mut out = Cursor::new(Vec::new());
    lib.write(&mut out).unwrap();
    let bytes = out.into_inner();
    log.bytes(&bytes);
    let back = Library::read(Cursor::new(bytes)).unwrap();
    let data = to_data(&back).unwrap();
    for p in &data.programs {
        log.bytes(&p.data);
        log.text((&p.author, &p.comment));
    }
    log.text(&data.extras);
    let again = from_data(&data).unwrap();
    let mut out = Cursor::new(Vec::new());
    again.write(&mut out).unwrap();
    log.bytes(&out.into_inner());
    log.text(Library::read(Cursor::new(vec![1, 2, 3])).map(|_| ()));
    log.check(0xf133_ef27_2ce6_e654);
}

#[test]
fn unit_bytes() {
    let mut log = Log::default();
    let (module, name, header) = build_header(&waves(), 2016).unwrap();
    log.text((module, &name));
    log.bytes(&header);
    let unit = Unit {
        module,
        name,
        header,
        payload: b"UOSCxxxx".to_vec(),
    };
    log.bytes(&unit.blob());
    log.bytes(&unit.slot_data());
    let mut m = waves();
    m["header"]["platform"] = serde_json::json!("prologue");
    log.text(build_header(&m, 0).map(|_| ()));
    let mut m = waves();
    m["header"]["name"] = serde_json::json!("fourteen chars");
    log.text(build_header(&m, 0).map(|_| ()));
    log.text(Unit::read(Cursor::new(vec![0u8; 4])).map(|_| ()));
    log.check(0xb7bc_4440_b9f9_2aad);
}
