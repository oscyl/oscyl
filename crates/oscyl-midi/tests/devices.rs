//! Several synths at once, of any registered driver, over simulated links.

use std::collections::VecDeque;
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, channel};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use oscyl_driver::engine::OpError;
use oscyl_driver::sysex::{DEVICE_INQUIRY, Framer};
use oscyl_driver::{Driver, SessionEvent, SessionOp, Simulator};
use oscyl_midi::{Candidate, DRIVERS, Device, Error, Link, SimLink, candidates, run_all, wait};
use oscyl_minilogue_xd::demo::demo_library;
use oscyl_minilogue_xd::msg::{Dump, PROGRAM_COUNT, Request, encode_dump, encode_request};
use oscyl_minilogue_xd::sim::SimDevice;

const XD: &dyn Driver = &oscyl_minilogue_xd::DRIVER;
const REV2: &dyn Driver = &oscyl_prophet_rev2::DRIVER;
const PROLOGUE: &dyn Driver = &oscyl_korg_prologue::DRIVER;

/// A synth on its own thread, as midir delivers a real one: replies arrive
/// on a channel and ring the bell. With a `gate`, every message after the
/// first (the inquiry `Device::connect` sends) waits until the gate opens.
struct Threaded {
    to_synth: Sender<Vec<u8>>,
    from_synth: Receiver<Vec<u8>>,
    bell: Arc<Mutex<Option<Sender<()>>>>,
}

fn threaded(mut sim: Box<dyn Simulator>, gate: Option<Receiver<()>>) -> Box<Threaded> {
    let (to_synth, rx) = channel::<Vec<u8>>();
    let (tx, from_synth) = channel();
    let bell = Arc::new(Mutex::new(None::<Sender<()>>));
    let ring = bell.clone();
    std::thread::spawn(move || {
        let (mut gate, mut framer) = (gate, Framer::new());
        let msgs = rx.iter().flat_map(|bytes| framer.push(&bytes));
        for (i, msg) in msgs.enumerate() {
            if i > 0
                && let Some(g) = gate.take()
            {
                let _ = g.recv();
            }
            for reply in sim.handle(&msg) {
                if tx.send(reply).is_err() {
                    return;
                }
                if let Some(b) = ring.lock().unwrap().as_ref() {
                    let _ = b.send(());
                }
            }
        }
    });
    Box::new(Threaded {
        to_synth,
        from_synth,
        bell,
    })
}

impl Link for Threaded {
    fn send(&mut self, bytes: &[u8]) -> Result<(), Error> {
        self.to_synth
            .send(bytes.to_vec())
            .map_err(|e| Error::Disconnected(e.to_string()))
    }

    fn recv_until(&mut self, deadline: Instant) -> Result<Option<Vec<u8>>, Error> {
        match self
            .from_synth
            .recv_timeout(deadline.saturating_duration_since(Instant::now()))
        {
            Ok(m) => Ok(Some(m)),
            Err(RecvTimeoutError::Timeout) => Ok(None),
            Err(e) => Err(Error::Disconnected(e.to_string())),
        }
    }

    fn ring_on_input(&mut self, bell: Sender<()>) {
        *self.bell.lock().unwrap() = Some(bell);
    }
}

#[test]
fn threaded_links_under_run_all() {
    let mut devs: Vec<Device> = [XD, REV2, REV2]
        .into_iter()
        .map(|d| Device::connect(d, threaded(d.simulator(), None), "t".into()).unwrap())
        .collect();
    for (i, d) in devs.iter_mut().enumerate() {
        let p = named(d.driver(), &format!("Dev {i}"));
        d.submit(SessionOp::WritePrograms(vec![(3, p)])).unwrap();
        d.submit(SessionOp::ReadPrograms(vec![3])).unwrap();
    }
    let mut read = vec![vec![]; 3];
    let mut refs: Vec<&mut Device> = devs.iter_mut().collect();
    let results = run_all(&mut refs, |i, e| match e {
        SessionEvent::Program(3, p) => read[i].push(p.clone()),
        SessionEvent::Failed(e) => panic!("device {i}: {e}"),
        _ => {}
    });
    assert!(results.iter().all(Result::is_ok), "{results:?}");
    for (i, d) in devs.iter().enumerate() {
        assert_eq!(d.driver().name(&read[i][0]).unwrap(), format!("Dev {i}"));
    }
}

/// A worker serving a UI: commands come on its own channel and ring the
/// same bell as the synths' input, so a new op starts while another device
/// is still waiting for its synth.
#[test]
fn worker_takes_new_work_while_a_device_is_mid_op() {
    let start = Instant::now();
    let (gate, gate_rx) = channel();
    let mut slow = Device::connect(
        REV2,
        threaded(REV2.simulator(), Some(gate_rx)),
        "slow".into(),
    )
    .unwrap();
    let mut fast = Device::simulated(XD).unwrap();
    let (bell, rung) = channel();
    slow.ring_on_input(bell.clone());
    fast.ring_on_input(bell.clone());
    slow.submit(SessionOp::ReadPrograms(vec![0, 1])).unwrap();
    let (cmd_tx, cmd_rx) = channel();
    std::thread::spawn(move || {
        cmd_tx.send(SessionOp::ReadCurrent).unwrap();
        bell.send(()).unwrap();
    });
    let (mut done, mut gate) = (vec![], Some(gate));
    loop {
        while let Ok(op) = cmd_rx.try_recv() {
            fast.submit(op).unwrap();
        }
        let mut next: Option<Instant> = None;
        for (i, d) in [&mut slow, &mut fast].into_iter().enumerate() {
            let t = d
                .pump(&mut |e| match e {
                    SessionEvent::Done => done.push(i),
                    SessionEvent::Failed(e) => panic!("device {i}: {e}"),
                    _ => {}
                })
                .unwrap();
            next = match (next, t) {
                (Some(a), Some(b)) => Some(a.min(b)),
                (a, b) => a.or(b),
            };
        }
        if done == [1]
            && let Some(g) = gate.take()
        {
            g.send(()).unwrap();
        }
        if done.len() == 2 {
            break;
        }
        wait(&rung, next);
    }
    assert_eq!(
        done,
        [1, 0],
        "the new op finished while the slow one waited"
    );
    assert!(
        start.elapsed() < Duration::from_secs(5),
        "woken by the ring"
    );
}

#[test]
fn cancel_ends_every_queued_op_at_once() {
    // The synth never answers again, like one unplugged mid-op.
    let (_gate, gate_rx) = channel();
    let mut d = Device::connect(
        REV2,
        threaded(REV2.simulator(), Some(gate_rx)),
        "gone".into(),
    )
    .unwrap();
    for n in 0..3 {
        d.submit(SessionOp::ReadPrograms(vec![n])).unwrap();
    }
    let mut ended = vec![];
    d.pump(&mut |e| ended.push(e.clone())).unwrap();
    assert!(ended.is_empty() && !d.is_idle());
    d.cancel(&mut |e| ended.push(e.clone()));
    assert_eq!(ended, vec![SessionEvent::Failed(OpError::Cancelled); 3]);
    assert!(d.is_idle());
    assert_eq!(d.pump(&mut |_| panic!("nothing left")).unwrap(), None);
}

/// The xd's firmware can answer one request behind. A `Device` must clear
/// that before any read, never report one program's data under another's
/// number.
#[test]
fn xd_device_clears_reply_lag_before_reading() {
    let mut sim = SimDevice::demo();
    sim.faults.race_deferred_save = true;
    // An uncommitted write leaves a deferred save; a read races it.
    let p = sim.program(7).unwrap().clone();
    sim.handle(&encode_dump(0, &Dump::Program(7, p.as_bytes().to_vec())));
    sim.handle(&encode_request(0, &Request::Program(9)));
    assert!(sim.is_lagging());
    let mut xd = Device::connect(XD, Box::new(SimLink::new(sim)), "lagging".into()).unwrap();
    let mut got = vec![];
    xd.run(SessionOp::ReadPrograms((0..PROGRAM_COUNT).collect()), |e| {
        if let SessionEvent::Program(n, p) = e {
            got.push((*n, p.clone()));
        }
    })
    .unwrap();
    let demo = demo_library();
    assert_eq!(got.len(), usize::from(PROGRAM_COUNT));
    for ((n, p), (want, _)) in got.iter().zip(&demo) {
        let name = |b: &[u8]| XD.name(b).unwrap();
        assert!(
            p == want.as_bytes(),
            "program {n} read as {:?}, holds {:?}",
            name(p),
            want.name()
        );
    }
}

/// A port that loops our output back; with `sim`, a synth answers too
/// (MIDI thru left on).
struct Echo {
    sim: Option<Box<dyn Simulator>>,
    framer: Framer,
    queue: VecDeque<Vec<u8>>,
}

impl Echo {
    fn new(sim: Option<Box<dyn Simulator>>) -> Box<Echo> {
        Box::new(Echo {
            sim,
            framer: Framer::new(),
            queue: VecDeque::new(),
        })
    }
}

impl Link for Echo {
    fn send(&mut self, bytes: &[u8]) -> Result<(), Error> {
        for msg in self.framer.push(bytes) {
            self.queue.push_back(msg.clone());
            if let Some(sim) = &mut self.sim {
                self.queue.extend(sim.handle(&msg));
            }
        }
        Ok(())
    }

    fn recv_until(&mut self, _: Instant) -> Result<Option<Vec<u8>>, Error> {
        Ok(self.queue.pop_front())
    }
}

/// A simulator that answers the device inquiry and loses everything else.
struct InquiryOnly(Box<dyn Simulator>);

impl Simulator for InquiryOnly {
    fn handle(&mut self, msg: &[u8]) -> Vec<Vec<u8>> {
        if msg == DEVICE_INQUIRY {
            self.0.handle(msg)
        } else {
            vec![]
        }
    }
    fn idle_tick(&mut self) {}
    fn program(&self, slot: u16) -> Option<Vec<u8>> {
        self.0.program(slot)
    }
    fn edit_buffer(&self) -> Vec<u8> {
        self.0.edit_buffer()
    }
    fn is_settled(&self) -> bool {
        true
    }
    fn faults(&self) -> &'static [&'static str] {
        &[]
    }
    fn inject(&mut self, _: &str) {}
}

fn mute(d: &'static dyn Driver) -> Device {
    let link = SimLink::from_sim(Box::new(InquiryOnly(d.simulator())));
    Device::connect(d, Box::new(link), "mute".into()).unwrap()
}

fn named(d: &dyn Driver, name: &str) -> Vec<u8> {
    let mut p = d.init_program();
    d.set_name(&mut p, name).unwrap();
    p
}

#[test]
fn four_devices_at_once() {
    let mut devs: Vec<Device> = [XD, XD, REV2, REV2]
        .into_iter()
        .map(|d| Device::simulated(d).unwrap())
        .collect();
    for (i, d) in devs.iter_mut().enumerate() {
        let p = named(d.driver(), &format!("Dev {i}"));
        d.submit(SessionOp::WritePrograms(vec![(3, p)])).unwrap();
        d.submit(SessionOp::ReadPrograms(vec![3, 4, 5])).unwrap();
    }
    let (mut order, mut read, mut done) = (vec![], vec![vec![]; 4], [0; 4]);
    let mut refs: Vec<&mut Device> = devs.iter_mut().collect();
    let results = run_all(&mut refs, |i, e| {
        order.push(i);
        match e {
            SessionEvent::Program(3, p) => read[i].push(p.clone()),
            SessionEvent::Done => done[i] += 1,
            SessionEvent::Failed(e) => panic!("device {i}: {e}"),
            _ => {}
        }
    });
    assert!(results.iter().all(Result::is_ok), "{results:?}");
    assert_eq!(done, [2; 4]);
    for (i, d) in devs.iter().enumerate() {
        assert_eq!(read[i].len(), 1);
        assert_eq!(d.driver().name(&read[i][0]).unwrap(), format!("Dev {i}"));
        assert!(d.is_idle());
    }
    let switches = order.windows(2).filter(|w| w[0] != w[1]).count();
    assert!(switches > 3, "ops ran one device after another: {order:?}");
}

#[test]
fn lost_reply_fails_only_its_device() {
    let start = Instant::now();
    let (mut xd, mut rev2) = (Device::simulated(XD).unwrap(), mute(REV2));
    xd.submit(SessionOp::ReadPrograms(vec![0])).unwrap();
    rev2.submit(SessionOp::ReadPrograms(vec![0])).unwrap();
    let mut last = [None, None];
    let results = run_all(&mut [&mut xd, &mut rev2], |i, e| last[i] = Some(e.clone()));
    assert!(results.iter().all(Result::is_ok));
    assert_eq!(last[0], Some(SessionEvent::Done));
    assert!(
        matches!(last[1], Some(SessionEvent::Failed(OpError::Timeout { .. }))),
        "{:?}",
        last[1]
    );
    assert!(
        start.elapsed() < Duration::from_secs(5),
        "a simulated deadline passes at once"
    );
}

#[test]
fn looping_port_is_refused() {
    for sim in [None, Some(REV2.simulator())] {
        let r = Device::connect(REV2, Echo::new(sim), "loop".into());
        assert!(
            matches!(r, Err(Error::Loopback(ref p)) if p == "loop"),
            "{:?}",
            r.err()
        );
    }
}

#[test]
fn silent_port_is_reported() {
    // A Rev2 answers, but not as a minilogue xd.
    let link = SimLink::from_sim(REV2.simulator());
    let r = Device::connect(XD, Box::new(link), "quiet".into());
    assert!(
        matches!(r, Err(Error::Silent(ref p)) if p == "quiet"),
        "{:?}",
        r.err()
    );
}

#[test]
fn device_run_reports_op_failure() {
    let mut rev2 = Device::simulated(REV2).unwrap();
    let p = REV2.init_program();
    assert!(matches!(
        rev2.run(SessionOp::WritePrograms(vec![(600, p)]), |_| {}),
        Err(Error::Driver(_))
    ));
    let mut events = 0;
    rev2.run(SessionOp::ReadPrograms(vec![0, 1]), |_| events += 1)
        .unwrap();
    // Progress after the device check and each read, two programs, done.
    assert_eq!(events, 6);
    assert!(matches!(
        mute(REV2).run(SessionOp::ReadPrograms(vec![0]), |_| {}),
        Err(Error::Op(OpError::Timeout { .. }))
    ));
}

fn names(v: &[&str]) -> Vec<String> {
    v.iter().map(ToString::to_string).collect()
}

fn found(c: &[Candidate]) -> Vec<(&str, &str)> {
    c.iter()
        .map(|c| (c.driver.info().id, c.port.as_str()))
        .collect()
}

#[test]
fn candidates_from_a_fake_port_list() {
    let ports = names(&[
        "Midi Through:Midi Through Port-0 14:0",
        "minilogue xd:minilogue xd _ MIDI OUT 28:0",
        "minilogue xd:minilogue xd _ SOUND 28:1",
        "minilogue xd:minilogue xd _ SOUND 32:1",
        "Prophet Rev2:Prophet Rev2 MIDI 1 36:0",
    ]);
    assert_eq!(
        found(&candidates(&ports)),
        [
            (
                "korg-minilogue-xd",
                "minilogue xd:minilogue xd _ SOUND 28:1"
            ),
            (
                "korg-minilogue-xd",
                "minilogue xd:minilogue xd _ SOUND 32:1"
            ),
            (
                "sequential-prophet-rev2",
                "Prophet Rev2:Prophet Rev2 MIDI 1 36:0"
            ),
        ]
    );
}

#[test]
fn registry_lists_every_driver() {
    let ids: Vec<_> = DRIVERS.iter().map(|d| d.info().id).collect();
    assert_eq!(
        ids,
        [
            "korg-minilogue-xd",
            "sequential-prophet-rev2",
            "korg-prologue"
        ]
    );
}

#[test]
fn every_driver_recognises_only_its_own_simulator() {
    for d in DRIVERS {
        for reply in d.simulator().handle(&DEVICE_INQUIRY) {
            let who: Vec<_> = DRIVERS
                .iter()
                .filter(|o| o.recognize(&reply))
                .map(|o| o.info().id)
                .collect();
            assert_eq!(who, [d.info().id]);
        }
    }
}

#[test]
fn driver_lookup() {
    assert_eq!(
        oscyl_midi::driver("korg-minilogue-xd").map(|d| d.info().model),
        Some("minilogue xd")
    );
    assert!(oscyl_midi::driver("nope").is_none());
}

#[test]
fn demo_device_holds_demo_programs_and_names_its_firmware() {
    let mut d = Device::demo(XD).unwrap();
    assert_eq!(d.port_name(), "simulated minilogue xd");
    assert_eq!(d.firmware().as_deref(), Some("2.10"));
    let mut got = None;
    d.run(SessionOp::ReadPrograms(vec![0]), |e| {
        if let SessionEvent::Program(0, p) = e {
            got = Some(p.clone());
        }
    })
    .unwrap();
    assert_eq!(got, Some(demo_library()[0].0.as_bytes().to_vec()));
    assert_eq!(Device::demo(REV2).unwrap().firmware(), None);
}

/// A simulated link that records every send.
struct Recording(SimLink, Arc<Mutex<Vec<Vec<u8>>>>);

impl Link for Recording {
    fn send(&mut self, bytes: &[u8]) -> Result<(), Error> {
        self.1.lock().unwrap().push(bytes.to_vec());
        self.0.send(bytes)
    }
    fn recv_until(&mut self, deadline: Instant) -> Result<Option<Vec<u8>>, Error> {
        self.0.recv_until(deadline)
    }
    fn synchronous(&self) -> bool {
        true
    }
}

#[test]
fn send_goes_out_as_one_message() {
    let log = Arc::new(Mutex::new(vec![]));
    let link = Recording(SimLink::from_sim(XD.simulator()), log.clone());
    let mut d = Device::connect(XD, Box::new(link), "rec".into()).unwrap();
    d.send(&[0xB0, 43, 100]).unwrap();
    assert_eq!(log.lock().unwrap().last(), Some(&vec![0xB0, 43, 100]));
}

#[test]
fn run_xd_lists_48_slots_and_refuses_a_rev2() {
    use oscyl_minilogue_xd::session::{Event, Op};
    let mut d = Device::demo(XD).unwrap();
    let mut slots = 0;
    d.run_xd(Op::ReadSlots, |e| {
        slots += matches!(e, Event::Slot { .. }) as usize
    })
    .unwrap();
    assert_eq!(slots, 48);
    let mut r = Device::demo(REV2).unwrap();
    assert!(matches!(
        r.run_xd(Op::ReadSlots, |_| {}),
        Err(Error::Driver(_))
    ));
}

/// A worker's bell survives a blocking `run` (or `run_xd`) on the same
/// device: input after it still rings the worker, so a read is not left
/// waiting for its deadline.
#[test]
fn the_workers_bell_survives_a_blocking_run() {
    let mut d = Device::connect(XD, threaded(XD.demo_simulator(), None), "t".into()).unwrap();
    let (bell, rung) = channel();
    d.ring_on_input(bell.clone());
    d.run(SessionOp::ReadCurrent, |_| {}).unwrap();
    d.run_xd(oscyl_minilogue_xd::session::Op::ReadSlots, |_| {})
        .unwrap();
    let t0 = Instant::now();
    d.submit(SessionOp::ReadPrograms(vec![1])).unwrap();
    let mut done = false;
    while !done {
        let next = d
            .pump(&mut |e| done |= matches!(e, SessionEvent::Done))
            .unwrap();
        if !done {
            wait(&rung, next);
        }
    }
    assert!(
        t0.elapsed() < Duration::from_secs(1),
        "waited {:?}",
        t0.elapsed()
    );
    drop(bell);
}

/// What a pumped device reported, in order: session events and unit events.
#[derive(Debug, PartialEq)]
enum Seen {
    Program(u16),
    Done,
    Slots(usize),
    UnitDone,
}

/// Pump `d` (over a threaded link, waiting on its bell) until idle.
fn pump_all(d: &mut Device) -> Vec<Seen> {
    use oscyl_midi::logue::UnitEvent;
    let (bell, rung) = channel();
    d.ring_on_input(bell.clone());
    let mut seen = vec![];
    let mut slots = 0;
    loop {
        let next = d
            .pump(&mut |e| match e {
                SessionEvent::Program(n, _) => seen.push(Seen::Program(*n)),
                SessionEvent::Done => seen.push(Seen::Done),
                _ => {}
            })
            .unwrap();
        while let Some(e) = d.poll_unit_event() {
            match e {
                UnitEvent::Slot { .. } => slots += 1,
                UnitEvent::Done => {
                    seen.push(Seen::Slots(std::mem::take(&mut slots)));
                    seen.push(Seen::UnitDone);
                }
                UnitEvent::Failed(e) => panic!("{e}"),
                _ => {}
            }
        }
        if next.is_none() && d.is_idle() {
            return seen;
        }
        wait(&rung, next);
    }
}

/// Unit ops are pumped like session ops: they wait for the ops queued
/// before them, and ops queued after them wait for them, on a threaded
/// link, without blocking.
#[test]
fn unit_ops_take_their_turn_between_session_ops() {
    use oscyl_midi::logue::UnitOp;
    for d in [XD, PROLOGUE] {
        let mut dev = Device::connect(d, threaded(d.demo_simulator(), None), "t".into()).unwrap();
        dev.submit(SessionOp::ReadPrograms(vec![0])).unwrap();
        dev.submit_units(UnitOp::ReadSlots).unwrap();
        dev.submit(SessionOp::ReadPrograms(vec![1])).unwrap();
        let t0 = Instant::now();
        assert_eq!(
            pump_all(&mut dev),
            [
                Seen::Program(0),
                Seen::Done,
                Seen::Slots(48),
                Seen::UnitDone,
                Seen::Program(1),
                Seen::Done
            ],
            "{}",
            d.info().id
        );
        assert!(t0.elapsed() < Duration::from_secs(1));
    }
    let mut rev2 = Device::demo(REV2).unwrap();
    assert!(rev2.submit_units(UnitOp::ReadSlots).is_err());
}

/// Units first on a fresh device: the session's lag check runs, silently.
#[test]
fn unit_ops_first_run_the_lag_check_silently() {
    use oscyl_midi::logue::UnitOp;
    let mut dev = Device::connect(XD, threaded(XD.demo_simulator(), None), "t".into()).unwrap();
    dev.submit_units(UnitOp::ReadSlots).unwrap();
    assert_eq!(pump_all(&mut dev), [Seen::Slots(48), Seen::UnitDone]);
}

/// Opening a synth never blocks: the inquiry goes out, the answer is
/// polled for as it arrives; a synth that never answers fails once its
/// link says nothing more will come.
#[test]
fn opening_polls_for_the_answer() {
    use oscyl_midi::{Opened, Opening};
    let (bell, rung) = channel();
    let mut o = Opening::start(XD, threaded(XD.simulator(), None), "t".into()).unwrap();
    o.ring_on_input(bell.clone());
    let device = loop {
        match o.poll(Instant::now()) {
            Opened::Ready(d) => break d,
            Opened::Waiting(next) => {
                wait(&rung, Some(next.deadline()));
                o = next;
            }
            Opened::Failed(e) => panic!("{e}"),
        }
    };
    assert_eq!(device.driver().info().id, XD.info().id);
    let silent = Opening::start(
        XD,
        Box::new(SimLink::from_sim(REV2.simulator())),
        "r".into(),
    )
    .unwrap()
    .poll(Instant::now());
    assert!(matches!(silent, Opened::Failed(Error::Silent(_))));
}
