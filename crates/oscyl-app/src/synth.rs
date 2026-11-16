//! The synth link: one worker thread owns every connected synth (an
//! `oscyl_midi::Device` each) and talks to the GTK main loop through
//! channels. Commands go in over a std channel and ring the worker's bell;
//! events stream back over an async channel the main loop awaits. Nothing
//! here sleeps or polls: the worker waits on its bell, which every device
//! also rings when its synth sends something, or until the earliest
//! session deadline (which only bounds a synth that stopped answering).

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use oscyl_driver::engine::OpError;
use oscyl_driver::{Driver, SessionEvent, SessionOp};
// User units are a logue-family capability the contract does not model:
// they run as `Device` unit ops on drivers that list units.
#[cfg(feature = "midir")]
use oscyl_midi::Candidate;
use oscyl_midi::logue::{Module, UnitEvent, UnitOp, unit_name};
use oscyl_midi::{DRIVERS, Device, Opened, Opening};

use crate::i18n::tr;
use crate::slots::slots;

/// A connected synth, for as long as it stays connected.
pub type DeviceId = u64;

pub enum Cmd {
    /// Connect to every synth found on USB not yet connected, or (when
    /// `simulated`) to a simulated synth of every registered driver.
    Connect {
        simulated: bool,
    },
    /// Connect to one port that appeared (hotplug).
    #[cfg(feature = "midir")]
    Open(Candidate),
    /// Connect to `driver`'s synth on a link the platform provides (its
    /// bytes moved by Android or iOS), known by `port`.
    OpenLink {
        driver: &'static dyn Driver,
        link: Box<dyn oscyl_midi::Link>,
        port: String,
    },
    /// Connect to `driver`'s simulated synth (demo mode).
    OpenDemo(&'static dyn Driver),
    /// End every synth's work and stop the worker (the app quits).
    Quit,
    /// Tests: a device opened on a link of their own (a synth on its own
    /// thread).
    #[cfg(test)]
    Attach(Box<Device>),
    /// The port named so went away: end what its synth was doing, drop it.
    Gone(String),
    Disconnect(DeviceId),
    /// Read every program, slot by slot (stopped by [`Cmd::CancelRead`]).
    ReadAll(DeviceId),
    CancelRead(DeviceId),
    /// Read the program the synth is playing (its edit buffer).
    ReadCurrent(DeviceId),
    /// Play a program from the edit buffer. A newer audition replaces one
    /// still waiting, and the live changes queued before it.
    Audition(DeviceId, Vec<u8>),
    /// Live parameter changes (`Driver::live_messages`), each inner `Vec`
    /// one MIDI message, sent in order, so notes keep playing.
    Live(DeviceId, Vec<Vec<u8>>),
    /// Write programs to slots; the driver plans the safe commit.
    Write(DeviceId, Vec<(u16, Vec<u8>)>),
    ReadSlots(DeviceId),
    /// Load a unit (TABLE 7 slot data) into a slot, then re-list slots.
    UploadUnit {
        id: DeviceId,
        module: Module,
        slot: u8,
        data: Vec<u8>,
    },
    /// Save the slot's unit to `backup`, clear the slot, then re-list.
    ClearSlot {
        id: DeviceId,
        module: Module,
        slot: u8,
        backup: PathBuf,
    },
}

#[derive(Clone)]
pub enum Ev {
    Connected {
        id: DeviceId,
        driver: &'static dyn Driver,
        port: String,
        firmware: Option<String>,
        simulated: bool,
    },
    /// Why, and what to do, for an error dialog.
    ConnectFailed(String),
    Disconnected(DeviceId),
    ReadProgress {
        id: DeviceId,
        done: usize,
        total: usize,
    },
    /// Slot `n`'s program.
    Program(DeviceId, u16, Vec<u8>),
    ReadDone {
        id: DeviceId,
        cancelled: bool,
    },
    WriteProgress {
        id: DeviceId,
        done: usize,
        total: usize,
    },
    Written {
        id: DeviceId,
        indices: Vec<u16>,
    },
    /// A write was refused or failed: nothing to mark synced.
    WriteFailed(DeviceId, String),
    /// The program the synth is playing now (its edit buffer).
    Current(DeviceId, Vec<u8>),
    Slots(DeviceId, Vec<SlotInfo>),
    Error(Option<DeviceId>, String),
}

#[derive(Debug, Clone)]
pub struct SlotInfo {
    pub module: Module,
    pub slot: u8,
    pub name: Option<String>,
}

/// The UI's handle on the worker.
#[derive(Clone)]
pub struct SynthHandle {
    tx: mpsc::Sender<Cmd>,
    bell: mpsc::Sender<()>,
}

impl SynthHandle {
    pub fn send(&self, cmd: Cmd) {
        // The worker stops only once every handle is gone, so these
        // cannot fail while the UI holds one.
        let _ = self.tx.send(cmd);
        let _ = self.bell.send(());
    }
}

/// Opens a port again for [`Recovery`]: the device inquiry goes out.
type Reopen =
    Box<dyn FnMut(&'static dyn Driver, &str) -> Result<Opening, oscyl_midi::Error> + Send>;

pub fn spawn() -> (SynthHandle, async_channel::Receiver<Ev>) {
    // Desktop ports reopen by name; a platform's links cannot be reopened.
    #[cfg(feature = "midir")]
    let reopen: Option<Reopen> = Some(Box::new(|driver, port| {
        Opening::open(&Candidate {
            driver,
            port: port.into(),
        })
    }));
    #[cfg(not(feature = "midir"))]
    let reopen: Option<Reopen> = None;
    spawn_with(reopen, RETRY)
}

fn spawn_with(reopen: Option<Reopen>, retry: Retry) -> (SynthHandle, async_channel::Receiver<Ev>) {
    let (tx, rx) = mpsc::channel();
    let (bell, rung) = mpsc::channel();
    let (etx, erx) = async_channel::unbounded();
    let ring = bell.clone();
    std::thread::Builder::new()
        .name("oscyl-synth".into())
        .spawn(move || {
            Worker {
                devices: Vec::new(),
                openings: Vec::new(),
                next_id: 1,
                events: etx,
                bell: ring,
                reopen,
                retry,
            }
            .run(rx, rung)
        })
        .expect("spawn synth worker");
    (SynthHandle { tx, bell }, erx)
}

/// What the device's next op `Done`/`Failed` belongs to.
enum Job {
    Read,
    Current,
    Audition,
    Write(Vec<u16>),
}

/// A full read in progress: one slot at a time, so a cancel or another
/// op gets its turn between slots.
struct Read {
    slots: Vec<u16>,
    done: usize,
    cancelled: bool,
}

/// Auditions and live changes waiting for the audition in flight.
#[derive(Debug, Default, PartialEq)]
struct Outbox(VecDeque<Out>);

#[derive(Debug, PartialEq)]
enum Out {
    Audition(Vec<u8>),
    Live(Vec<Vec<u8>>),
}

impl Outbox {
    /// A whole program supersedes everything waiting: older auditions and
    /// live changes that carry older values of its fields.
    fn audition(&mut self, p: Vec<u8>) {
        self.0.clear();
        self.0.push_back(Out::Audition(p));
    }

    fn live(&mut self, msgs: Vec<Vec<u8>>) {
        self.0.push_back(Out::Live(msgs));
    }

    /// What may go out now: live messages up to the next audition, which
    /// comes last; nothing while an audition is in flight.
    fn take(&mut self, auditioning: bool) -> Vec<Out> {
        let mut out = Vec::new();
        if auditioning {
            return out;
        }
        while let Some(o) = self.0.pop_front() {
            let last = matches!(o, Out::Audition(_));
            out.push(o);
            if last {
                break;
            }
        }
        out
    }
}

/// What a unit op of a device's queue is for.
enum UnitJob {
    /// List the slots; collects them.
    List(Vec<SlotInfo>),
    /// Load or clear: its listing follows as its own job.
    Change,
    /// Read a slot's unit to back it up to the path, then clear the slot.
    Backup(Module, u8, PathBuf, Option<Vec<u8>>),
}

/// How long a port stays closed between attempts to reach a synth that
/// stopped answering, and how many attempts there are (about a minute).
/// A minilogue xd asked for a program as it boots ignores SysEx until its
/// port has been closed; it answers again some seconds later.
const RETRY: Retry = Retry {
    gap: Duration::from_secs(2),
    attempts: 15,
};

/// How a synth that stopped answering is retried.
#[derive(Clone, Copy)]
struct Retry {
    /// How long its port stays closed between attempts.
    gap: Duration,
    attempts: u32,
}

/// A synth that stopped answering: its port closed, then opened again
/// until the synth answers the device inquiry.
struct Recovery {
    opening: Option<Opening>,
    /// When the port opens again (while `opening` is `None`).
    next: Instant,
    attempts_left: u32,
}

struct Dev {
    id: DeviceId,
    /// Closed while `recovery` runs.
    device: Device,
    recovery: Option<Recovery>,
    /// Reopens left before giving up; counts down across reconnects
    /// until an op completes, so a synth that answers only the inquiry is
    /// not reopened forever.
    reopens_left: u32,
    simulated: bool,
    jobs: VecDeque<Job>,
    read: Option<Read>,
    outbox: Outbox,
    units: VecDeque<UnitJob>,
}

struct Worker {
    devices: Vec<Dev>,
    /// Ports whose device inquiry is out, the answer awaited.
    openings: Vec<Opening>,
    next_id: DeviceId,
    events: async_channel::Sender<Ev>,
    /// Kept to hand to new devices; the worker never rings it itself.
    bell: mpsc::Sender<()>,
    /// `None` where ports cannot be reopened.
    reopen: Option<Reopen>,
    retry: Retry,
}

impl Worker {
    fn emit(&self, ev: Ev) {
        // The UI closing the receiver means the app is quitting.
        let _ = self.events.send_blocking(ev);
    }

    fn run(mut self, cmds: mpsc::Receiver<Cmd>, rung: mpsc::Receiver<()>) {
        loop {
            loop {
                match cmds.try_recv() {
                    Ok(Cmd::Quit) => return self.quit(),
                    Ok(cmd) => self.handle(cmd),
                    Err(mpsc::TryRecvError::Empty) => break,
                    Err(mpsc::TryRecvError::Disconnected) => return,
                }
            }
            let next = self.round();
            oscyl_midi::wait(&rung, next);
        }
    }

    /// Every synth's work ends (cancelled); the worker stops, and its bell
    /// with it.
    fn quit(mut self) {
        while !self.devices.is_empty() {
            self.drop_device(0);
        }
    }

    /// Pump every device once; returns when to come back at the latest.
    fn round(&mut self) -> Option<Instant> {
        let mut next: Option<Instant> = None;
        let mut at = |t: Instant| next = Some(next.map_or(t, |n: Instant| n.min(t)));
        // Ports being opened: their answers, without waiting for them.
        let now = Instant::now();
        for o in std::mem::take(&mut self.openings) {
            let driver = o.driver();
            match o.poll(now) {
                Opened::Ready(d) => self.add(*d, false),
                Opened::Waiting(o) => {
                    at(o.deadline());
                    self.openings.push(o);
                }
                Opened::Failed(e) => self.emit(Ev::ConnectFailed(describe(&e, Some(driver)))),
            }
        }
        let mut k = 0;
        while k < self.devices.len() {
            if self.devices[k].recovery.is_some() {
                // `None`: given up, the device was dropped.
                if let Some(t) = self.recover(k, now) {
                    at(t);
                    k += 1;
                }
                continue;
            }
            let mut evs = Vec::new();
            let pumped = self.devices[k].device.pump(&mut |e| evs.push(e.clone()));
            let mut busy = !evs.is_empty();
            for e in evs {
                self.on_event(k, e);
            }
            while let Some(e) = self.devices[k].device.poll_unit_event() {
                busy = true;
                self.on_unit_event(k, e);
            }
            match pumped {
                Ok(t) => {
                    t.into_iter().for_each(&mut at);
                    // An audition it submits goes out on the next pump: now,
                    // not when the synth next sends something (a key).
                    busy |= self.flush(k);
                    if busy {
                        // Something new was submitted: pump it now.
                        at(Instant::now());
                    }
                    k += 1;
                }
                Err(e) => {
                    let id = self.devices[k].id;
                    let driver = self.devices[k].device.driver();
                    self.emit(Ev::Error(Some(id), describe(&e, Some(driver))));
                    self.drop_device(k);
                }
            }
        }
        next
    }

    fn find(&self, id: DeviceId) -> Option<usize> {
        self.devices.iter().position(|d| d.id == id)
    }

    fn handle(&mut self, cmd: Cmd) {
        match cmd {
            Cmd::Connect { simulated: true } => {
                for &d in DRIVERS {
                    let open = self
                        .devices
                        .iter()
                        .any(|x| x.simulated && x.device.driver().info().id == d.info().id);
                    if !open {
                        self.handle(Cmd::OpenDemo(d));
                    }
                }
            }
            // The platform finds synths itself (`Cmd::OpenLink`).
            #[cfg(not(feature = "midir"))]
            Cmd::Connect { simulated: false } => self.emit(Ev::ConnectFailed(describe(
                &oscyl_midi::Error::NotFound,
                None,
            ))),
            #[cfg(feature = "midir")]
            Cmd::Connect { simulated: false } => match oscyl_midi::scan() {
                Ok(found) if found.is_empty() => self.emit(Ev::ConnectFailed(describe(
                    &oscyl_midi::Error::NotFound,
                    None,
                ))),
                Ok(found) => {
                    for c in found {
                        self.handle(Cmd::Open(c));
                    }
                }
                Err(e) => self.emit(Ev::ConnectFailed(describe(&e, None))),
            },
            #[cfg(feature = "midir")]
            Cmd::Open(c) => {
                if !self.is_open(&c.port) {
                    self.opening(c.driver, Opening::open(&c));
                }
            }
            // The platform already handed out the link's input: say why it
            // will never connect rather than drop it unseen.
            Cmd::OpenLink { driver, link, port } => {
                if self.is_open(&port) {
                    let why = tr("A synth on this port is already connected or connecting.");
                    self.emit(Ev::ConnectFailed(format!("{why}\n{port}")));
                } else {
                    self.opening(driver, Opening::start(driver, link, port));
                }
            }
            #[cfg(test)]
            Cmd::Attach(d) => self.add(*d, false),
            Cmd::OpenDemo(driver) => match Device::demo(driver) {
                Ok(d) => self.add(d, true),
                Err(e) => self.emit(Ev::ConnectFailed(describe(&e, Some(driver)))),
            },
            Cmd::Gone(port) => {
                while let Some(k) = self
                    .devices
                    .iter()
                    .position(|d| d.device.port_name() == port)
                {
                    self.drop_device(k);
                }
            }
            Cmd::Disconnect(id) => {
                if let Some(k) = self.find(id) {
                    self.drop_device(k);
                }
            }
            Cmd::Quit => unreachable!("handled by run"),
            other => self.on_device(other),
        }
    }

    /// A device or an opening is on `port` already.
    fn is_open(&self, port: &str) -> bool {
        self.devices.iter().any(|d| d.device.port_name() == port)
            || self.openings.iter().any(|o| o.port_name() == port)
    }

    /// Await the answer to an inquiry just sent.
    fn opening(&mut self, driver: &'static dyn Driver, o: Result<Opening, oscyl_midi::Error>) {
        match o {
            Ok(mut o) => {
                o.ring_on_input(self.bell.clone());
                self.openings.push(o);
            }
            Err(e) => self.emit(Ev::ConnectFailed(describe(&e, Some(driver)))),
        }
    }

    fn add(&mut self, mut device: Device, simulated: bool) {
        device.ring_on_input(self.bell.clone());
        let id = self.next_id;
        self.next_id += 1;
        self.emit(Ev::Connected {
            id,
            driver: device.driver(),
            port: device.port_name().to_string(),
            firmware: device.firmware(),
            simulated,
        });
        self.devices.push(Dev {
            id,
            device,
            recovery: None,
            reopens_left: self.retry.attempts,
            simulated,
            jobs: VecDeque::new(),
            read: None,
            outbox: Outbox::default(),
            units: VecDeque::new(),
        });
    }

    /// End everything device `k` was doing (as cancelled), then drop it.
    fn drop_device(&mut self, k: usize) {
        let mut evs = Vec::new();
        self.devices[k].device.cancel(&mut |e| evs.push(e.clone()));
        for e in evs {
            self.on_event(k, e);
        }
        while let Some(e) = self.devices[k].device.poll_unit_event() {
            self.on_unit_event(k, e);
        }
        // A read between slots (its synth being reopened) has no op to end it.
        if self.devices[k].read.take().is_some() {
            let id = self.devices[k].id;
            self.emit(Ev::ReadDone {
                id,
                cancelled: true,
            });
        }
        let d = self.devices.remove(k);
        self.emit(Ev::Disconnected(d.id));
    }

    fn on_device(&mut self, cmd: Cmd) {
        let id = match &cmd {
            Cmd::ReadAll(id)
            | Cmd::ReadCurrent(id)
            | Cmd::CancelRead(id)
            | Cmd::Audition(id, _)
            | Cmd::Live(id, _)
            | Cmd::Write(id, _)
            | Cmd::ReadSlots(id)
            | Cmd::UploadUnit { id, .. }
            | Cmd::ClearSlot { id, .. } => *id,
            _ => unreachable!("handled by the worker"),
        };
        let Some(k) = self.find(id) else {
            let why = tr("The synth is not connected.");
            // A write always ends as a write, so whoever waits for it hears.
            self.emit(if matches!(cmd, Cmd::Write(..)) {
                Ev::WriteFailed(id, why)
            } else {
                Ev::Error(Some(id), why)
            });
            return;
        };
        match cmd {
            Cmd::ReadAll(_) => {
                let all = slots(self.devices[k].device.driver());
                self.devices[k].read = Some(Read {
                    slots: all,
                    done: 0,
                    cancelled: false,
                });
                self.next_read(k);
            }
            Cmd::ReadCurrent(_) => self.submit(k, SessionOp::ReadCurrent, Job::Current),
            Cmd::CancelRead(_) => {
                if let Some(r) = self.devices[k].read.as_mut() {
                    r.cancelled = true;
                }
                // No slot of it is in flight to end it.
                if self.devices[k].recovery.is_some() {
                    self.next_read(k);
                }
            }
            Cmd::Audition(_, p) => self.devices[k].outbox.audition(p),
            Cmd::Live(_, m) => self.devices[k].outbox.live(m),
            Cmd::Write(_, batch) => {
                let indices = batch.iter().map(|(n, _)| *n).collect();
                self.submit(k, SessionOp::WritePrograms(batch), Job::Write(indices));
            }
            Cmd::ReadSlots(_) => self.unit(k, UnitOp::ReadSlots, UnitJob::List(vec![])),
            Cmd::UploadUnit {
                module, slot, data, ..
            } => {
                self.unit(
                    k,
                    UnitOp::UploadUnit { module, slot, data },
                    UnitJob::Change,
                );
                self.unit(k, UnitOp::ReadSlots, UnitJob::List(vec![]));
            }
            Cmd::ClearSlot {
                module,
                slot,
                backup,
                ..
            } => self.unit(
                k,
                UnitOp::ReadSlotData { module, slot },
                UnitJob::Backup(module, slot, backup, None),
            ),
            _ => unreachable!("not a device command"),
        }
    }

    /// Queue unit op `op` on device `k`, for `job`.
    fn unit(&mut self, k: usize, op: UnitOp, job: UnitJob) {
        let d = &mut self.devices[k];
        if d.recovery.is_some() {
            let id = d.id;
            self.emit(Ev::Error(
                Some(id),
                tr("The synth is reconnecting. Try again in a moment."),
            ));
            return;
        }
        match d.device.submit_units(op) {
            Ok(()) => d.units.push_back(job),
            Err(e) => {
                let (id, driver) = (d.id, d.device.driver());
                self.emit(Ev::Error(Some(id), describe(&e, Some(driver))));
            }
        }
    }

    fn on_unit_event(&mut self, k: usize, e: UnitEvent) {
        let id = self.devices[k].id;
        let driver = self.devices[k].device.driver();
        match (e, self.devices[k].units.front_mut()) {
            (
                UnitEvent::Slot {
                    module,
                    slot,
                    header,
                },
                Some(UnitJob::List(slots)),
            ) => slots.push(SlotInfo {
                module,
                slot,
                name: header.as_deref().map(unit_name),
            }),
            (UnitEvent::SlotData { data, .. }, Some(UnitJob::Backup(.., saved))) => {
                *saved = Some(data)
            }
            (UnitEvent::Done, _) => match self.devices[k].units.pop_front() {
                Some(UnitJob::List(slots)) => self.emit(Ev::Slots(id, slots)),
                Some(UnitJob::Backup(module, slot, backup, saved)) => {
                    // Never clear what could not be backed up first.
                    match write_backup(&backup, saved) {
                        Ok(()) => {
                            self.unit(k, UnitOp::ClearSlot { module, slot }, UnitJob::Change);
                            self.unit(k, UnitOp::ReadSlots, UnitJob::List(vec![]));
                        }
                        Err(why) => self.emit(Ev::Error(
                            Some(id),
                            tr("Slot not cleared: backup failed ({error}).")
                                .replace("{error}", &why),
                        )),
                    }
                }
                Some(UnitJob::Change) | None => {}
            },
            (UnitEvent::Failed(err), _) => {
                self.devices[k].units.pop_front();
                if !matches!(err, OpError::Cancelled) {
                    let e = oscyl_midi::Error::Op(err);
                    self.emit(Ev::Error(Some(id), describe(&e, Some(driver))));
                }
            }
            _ => {}
        }
    }

    fn submit(&mut self, k: usize, op: SessionOp, job: Job) {
        let d = &mut self.devices[k];
        let submitted = if d.recovery.is_some() {
            Err(tr("The synth is reconnecting. Try again in a moment."))
        } else {
            d.device
                .submit(op)
                .map_err(|e| describe(&e, Some(d.device.driver())))
        };
        match submitted {
            Ok(()) => d.jobs.push_back(job),
            Err(why) => {
                let id = d.id;
                if matches!(job, Job::Write(_)) {
                    self.emit(Ev::WriteFailed(id, why));
                    return;
                }
                self.emit(Ev::Error(Some(id), why));
                if matches!(job, Job::Read) {
                    self.devices[k].read = None;
                    self.emit(Ev::ReadDone {
                        id,
                        cancelled: true,
                    });
                }
            }
        }
    }

    /// Submit the read's next slot, or report it done.
    fn next_read(&mut self, k: usize) {
        let d = &mut self.devices[k];
        let Some(r) = d.read.as_ref() else { return };
        // A synth reconnecting resumes the read itself.
        if d.recovery.is_some() && !r.cancelled {
            return;
        }
        if r.cancelled || r.done >= r.slots.len() {
            let (id, cancelled) = (d.id, r.cancelled);
            d.read = None;
            self.emit(Ev::ReadDone { id, cancelled });
            return;
        }
        let slot = r.slots[r.done];
        self.submit(k, SessionOp::ReadPrograms(vec![slot]), Job::Read);
    }

    /// Send what the outbox allows now. True when it submitted an op,
    /// which goes out on the next pump.
    fn flush(&mut self, k: usize) -> bool {
        let d = &mut self.devices[k];
        let auditioning = d.jobs.iter().any(|j| matches!(j, Job::Audition));
        let mut submitted = false;
        for out in d.outbox.take(auditioning) {
            match out {
                Out::Audition(p) => {
                    self.submit(k, SessionOp::Audition(p), Job::Audition);
                    submitted = true;
                }
                Out::Live(msgs) => {
                    let channel = self.devices[k].device.channel();
                    for m in msgs {
                        if let Err(e) = self.devices[k].device.send(&on_channel(m, channel)) {
                            let (id, driver) =
                                (self.devices[k].id, self.devices[k].device.driver());
                            self.emit(Ev::Error(Some(id), describe(&e, Some(driver))));
                            break;
                        }
                    }
                }
            }
        }
        submitted
    }

    fn on_event(&mut self, k: usize, e: SessionEvent) {
        let id = self.devices[k].id;
        match e {
            SessionEvent::Program(n, p) => self.emit(Ev::Program(id, n, p)),
            SessionEvent::Progress { done, total } => {
                if matches!(self.devices[k].jobs.front(), Some(Job::Write(_))) {
                    self.emit(Ev::WriteProgress { id, done, total });
                }
            }
            SessionEvent::Current(p) => self.emit(Ev::Current(id, p)),
            SessionEvent::Done => {
                // It answers: a later stall gets every attempt again.
                self.devices[k].reopens_left = self.retry.attempts;
                let job = self.devices[k].jobs.pop_front();
                match job {
                    Some(Job::Read) => {
                        if let Some(r) = self.devices[k].read.as_mut() {
                            r.done += 1;
                            let (done, total) = (r.done, r.slots.len());
                            self.emit(Ev::ReadProgress { id, done, total });
                        }
                        self.next_read(k);
                    }
                    Some(Job::Write(indices)) => self.emit(Ev::Written { id, indices }),
                    Some(Job::Audition | Job::Current) | None => {}
                }
            }
            SessionEvent::Failed(err) => {
                let driver = self.devices[k].device.driver();
                let job = self.devices[k].jobs.pop_front();
                if matches!(err, OpError::Timeout { .. }) && self.start_recovery(k) {
                    match job {
                        // The read goes on at the same slot once the synth
                        // answers again.
                        Some(Job::Read) => return,
                        Some(Job::Write(_)) => {
                            let why = describe(&oscyl_midi::Error::Op(err), Some(driver));
                            self.emit(Ev::WriteFailed(id, why));
                        }
                        _ => {}
                    }
                    return;
                }
                let why = describe(&oscyl_midi::Error::Op(err.clone()), Some(driver));
                if matches!(job, Some(Job::Write(_))) {
                    self.emit(Ev::WriteFailed(id, why));
                } else if !matches!(err, OpError::Cancelled) {
                    self.emit(Ev::Error(Some(id), why));
                }
                if matches!(job, Some(Job::Read)) {
                    self.devices[k].read = None;
                    self.emit(Ev::ReadDone {
                        id,
                        cancelled: true,
                    });
                }
            }
        }
    }
}

impl Worker {
    /// Device `k` stopped answering: close its port and start reopening
    /// it (where ports cannot be reopened, [`Worker::recover`] drops it
    /// for the platform to open again). False for a simulated synth, or
    /// when recovery already runs.
    fn start_recovery(&mut self, k: usize) -> bool {
        if self.devices[k].simulated || self.devices[k].recovery.is_some() {
            return false;
        }
        // Whatever else was queued ends now, as cancelled.
        let mut evs = Vec::new();
        self.devices[k].device.cancel(&mut |e| evs.push(e.clone()));
        self.devices[k].device.close();
        self.devices[k].recovery = Some(Recovery {
            opening: None,
            next: Instant::now() + self.retry.gap,
            attempts_left: self.devices[k].reopens_left,
        });
        for e in evs {
            self.on_event(k, e);
        }
        while let Some(e) = self.devices[k].device.poll_unit_event() {
            self.on_unit_event(k, e);
        }
        let id = self.devices[k].id;
        self.emit(Ev::Error(
            Some(id),
            tr("The synth stopped answering. Reconnecting…"),
        ));
        true
    }

    /// One step of device `k`'s recovery: when to come back, or `None`
    /// once given up (the device is dropped).
    fn recover(&mut self, k: usize, now: Instant) -> Option<Instant> {
        // A platform's link: the platform opens the port again.
        if self.reopen.is_none() {
            self.drop_device(k);
            return None;
        }
        let d = &mut self.devices[k];
        let (driver, port) = (d.device.driver(), d.device.port_name().to_string());
        let r = d.recovery.as_mut().expect("recovering");
        if let Some(o) = r.opening.take() {
            match o.poll(now) {
                Opened::Ready(mut device) => {
                    device.ring_on_input(self.bell.clone());
                    d.device = *device;
                    // What is left carries on until an op completes.
                    d.reopens_left = r.attempts_left;
                    d.recovery = None;
                    // A read in progress goes on where it stopped.
                    self.next_read(k);
                    return Some(now);
                }
                Opened::Waiting(o) => {
                    let t = o.deadline();
                    r.opening = Some(o);
                    return Some(t);
                }
                // The port closed again with the attempt.
                Opened::Failed(_) => r.next = now + self.retry.gap,
            }
        } else if now >= r.next {
            if r.attempts_left == 0 {
                return self.give_up(k);
            }
            r.attempts_left -= 1;
            let reopen = self.reopen.as_mut().expect("recovery needs reopen");
            match reopen(driver, &port) {
                Ok(mut o) => {
                    o.ring_on_input(self.bell.clone());
                    let t = o.deadline();
                    r.opening = Some(o);
                    return Some(t);
                }
                Err(_) => r.next = now + self.retry.gap,
            }
        }
        Some(r.next)
    }

    /// The synth never answered again: end its read and drop it.
    fn give_up(&mut self, k: usize) -> Option<Instant> {
        let id = self.devices[k].id;
        self.emit(Ev::Error(
            Some(id),
            tr("The synth does not answer. Turn it off and on again, then connect."),
        ));
        if self.devices[k].read.take().is_some() {
            self.emit(Ev::ReadDone {
                id,
                cancelled: true,
            });
        }
        self.drop_device(k);
        None
    }
}

/// `msg` on MIDI `channel` (0-15) when it is a channel message: the app
/// asks the driver for channel 0, and each synth listens on its own.
fn on_channel(mut msg: Vec<u8>, channel: u8) -> Vec<u8> {
    if let Some(status @ 0x80..=0xEF) = msg.first().copied() {
        msg[0] = status & 0xF0 | channel & 0x0F;
    }
    msg
}

/// Write a slot's unit to `backup` (the inner reason when it can't).
fn write_backup(backup: &std::path::Path, saved: Option<Vec<u8>>) -> Result<(), String> {
    let data = saved.ok_or_else(|| tr("the synth sent no unit data"))?;
    if let Some(dir) = backup.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    std::fs::write(backup, data).map_err(|e| e.to_string())
}

/// A link error for the person: the likely cause and the next step, with
/// the raw error kept as a detail line. `driver` is the synth it concerns,
/// when known: some say what to check on the synth.
pub fn describe(e: &oscyl_midi::Error, driver: Option<&dyn Driver>) -> String {
    use oscyl_midi::Error;
    let advice = match e {
        Error::NotFound => tr("Connect the synth with a USB cable and switch it on."),
        // midir fails to connect when another client holds the port.
        Error::Midi(m) if m.contains("connect") => {
            tr("Another app is using the synth's MIDI port. Close it and press Connect.")
        }
        Error::Silent(_) => {
            let hint = driver.map(|d| d.setup_hint()).unwrap_or_default();
            let base = tr("The synth didn't answer. Check the cable and try again.");
            if hint.is_empty() {
                base
            } else {
                format!("{base} {}", tr(hint))
            }
        }
        Error::Op(OpError::Timeout { .. }) => {
            tr("The synth didn't answer. Check the cable and try again.")
        }
        Error::Disconnected(_) => {
            tr("The synth was disconnected. Check the cable and press Connect.")
        }
        _ => tr("Something went wrong talking to the synth. Try again."),
    };
    format!("{advice}\n{e}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn driver(id: &str) -> &'static dyn Driver {
        oscyl_midi::driver(id).unwrap()
    }

    #[test]
    fn errors_name_the_next_step_and_keep_the_detail() {
        let d = describe(&oscyl_midi::Error::NotFound, None);
        assert!(d.starts_with("Connect the synth with a USB cable"));
        assert!(d.ends_with(&oscyl_midi::Error::NotFound.to_string()));
    }

    #[test]
    fn silent_rev2_names_the_sysex_cable() {
        let e = oscyl_midi::Error::Silent("Prophet Rev2:Prophet Rev2 MIDI 1 36:0".into());
        let rev2 = driver("sequential-prophet-rev2");
        assert!(describe(&e, Some(rev2)).contains("MIDI SysEx Cable"));
        let xd = driver("korg-minilogue-xd");
        assert!(!describe(&e, Some(xd)).contains("SysEx Cable"));
    }

    #[test]
    fn live_messages_go_on_the_synth_channel() {
        assert_eq!(on_channel(vec![0xB0, 43, 9], 5), [0xB5, 43, 9]);
        assert_eq!(on_channel(vec![0xF0, 1, 0xF7], 5), [0xF0, 1, 0xF7]);
    }

    #[test]
    fn auditions_coalesce_and_live_messages_go_one_by_one() {
        let mut o = Outbox::default();
        o.audition(vec![1]);
        o.live(vec![vec![0xB0, 1, 1]]);
        o.audition(vec![2]);
        assert_eq!(o, Outbox([Out::Audition(vec![2])].into()));
        o.live(vec![vec![0xB0, 2, 2], vec![0xB0, 3, 3]]);
        // Nothing goes out while an audition is in flight.
        assert!(o.take(true).is_empty());
        assert_eq!(o.take(false), [Out::Audition(vec![2])]);
        // The live change after it waits for that audition to land.
        assert!(o.take(true).is_empty());
        assert_eq!(
            o.take(false),
            [Out::Live(vec![vec![0xB0, 2, 2], vec![0xB0, 3, 3]])]
        );
        assert!(o.take(false).is_empty());
    }

    /// A morph sends a program per finger move: while one is on its way
    /// the rest wait, and only the newest of them goes after it.
    #[test]
    fn morph_sends_newest_only() {
        let mut o = Outbox::default();
        o.audition(vec![0]);
        let mut sent = o.take(false);
        for t in 1..=10 {
            o.audition(vec![t]);
            sent.extend(o.take(true));
        }
        sent.extend(o.take(false));
        assert_eq!(sent, [Out::Audition(vec![0]), Out::Audition(vec![10])]);
    }

    use std::sync::{Arc, Mutex};

    /// A simulated synth on its own thread, as midir delivers a real one:
    /// replies arrive later and ring the bell. With a `gate`, the first
    /// message after the inquiry waits until the gate opens.
    struct Threaded {
        to_synth: mpsc::Sender<Vec<u8>>,
        from_synth: mpsc::Receiver<Vec<u8>>,
        bell: Arc<Mutex<Option<mpsc::Sender<()>>>>,
    }

    impl oscyl_midi::Link for Threaded {
        fn send(&mut self, bytes: &[u8]) -> Result<(), oscyl_midi::Error> {
            self.to_synth
                .send(bytes.to_vec())
                .map_err(|e| oscyl_midi::Error::Disconnected(e.to_string()))
        }

        fn recv_until(&mut self, deadline: Instant) -> Result<Option<Vec<u8>>, oscyl_midi::Error> {
            match self
                .from_synth
                .recv_timeout(deadline.saturating_duration_since(Instant::now()))
            {
                Ok(m) => Ok(Some(m)),
                Err(mpsc::RecvTimeoutError::Timeout) => Ok(None),
                Err(e) => Err(oscyl_midi::Error::Disconnected(e.to_string())),
            }
        }

        fn ring_on_input(&mut self, bell: mpsc::Sender<()>) {
            *self.bell.lock().unwrap() = Some(bell);
        }
    }

    fn threaded(d: &'static dyn Driver, gate: Option<mpsc::Receiver<()>>) -> Box<Device> {
        let link = threaded_link(d, gate);
        Box::new(Device::connect(d, link, format!("threaded {}", d.info().id)).unwrap())
    }

    fn threaded_link(d: &'static dyn Driver, gate: Option<mpsc::Receiver<()>>) -> Box<Threaded> {
        let (to_synth, rx) = mpsc::channel::<Vec<u8>>();
        let (tx, from_synth) = mpsc::channel();
        let bell = Arc::new(Mutex::new(None::<mpsc::Sender<()>>));
        let ring = bell.clone();
        let mut sim = d.demo_simulator();
        std::thread::spawn(move || {
            let (mut gate, mut framer) = (gate, oscyl_driver::sysex::Framer::new());
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

    /// A minilogue xd that answers the device inquiry, then nothing: as
    /// one asked for a program while it boots.
    struct Wedged(VecDeque<Vec<u8>>);

    impl oscyl_midi::Link for Wedged {
        fn send(&mut self, bytes: &[u8]) -> Result<(), oscyl_midi::Error> {
            if bytes == oscyl_driver::sysex::DEVICE_INQUIRY {
                let mut sim = driver("korg-minilogue-xd").simulator();
                self.0.extend(sim.handle(bytes));
            }
            Ok(())
        }

        fn recv_until(&mut self, _: Instant) -> Result<Option<Vec<u8>>, oscyl_midi::Error> {
            Ok(self.0.pop_front())
        }
    }

    /// A synth that stops answering mid-read is closed and opened again
    /// until it answers; the read then goes on, under the same id.
    #[test]
    fn a_synth_that_stops_answering_is_reopened_and_the_read_resumes() {
        let xd = driver("korg-minilogue-xd");
        let reopened = Arc::new(Mutex::new(0));
        let count = reopened.clone();
        let (handle, rx) = spawn_with(
            Some(Box::new(move |d, port| {
                *count.lock().unwrap() += 1;
                Opening::start(d, threaded_link(d, None), port.into())
            })),
            FAST,
        );
        let wedged = Device::connect(xd, Box::new(Wedged(VecDeque::new())), "usb xd".into());
        let id = attach(&handle, &rx, Box::new(wedged.unwrap()));
        handle.send(Cmd::ReadAll(id));
        let (mut read, mut told) = (0, false);
        loop {
            match rx.recv_blocking().expect("worker alive") {
                Ev::Error(Some(i), e) if i == id && e.contains("Reconnecting") => told = true,
                Ev::Program(i, ..) if i == id => read += 1,
                Ev::ReadDone { id: i, cancelled } if i == id => break assert!(!cancelled),
                Ev::Error(_, e) | Ev::ConnectFailed(e) => panic!("unexpected error: {e}"),
                _ => {}
            }
        }
        assert!(told);
        assert_eq!(read, 500);
        assert_eq!(*reopened.lock().unwrap(), 1);
    }

    /// Retries for tests: a short gap, two attempts.
    const FAST: Retry = Retry {
        gap: Duration::from_millis(10),
        attempts: 2,
    };

    /// A port that answers nothing, not even the device inquiry.
    struct Mute;

    impl oscyl_midi::Link for Mute {
        fn send(&mut self, _: &[u8]) -> Result<(), oscyl_midi::Error> {
            Ok(())
        }

        fn recv_until(&mut self, _: Instant) -> Result<Option<Vec<u8>>, oscyl_midi::Error> {
            Ok(None)
        }
    }

    fn wedged_xd() -> Box<Device> {
        let xd = driver("korg-minilogue-xd");
        let d = Device::connect(xd, Box::new(Wedged(VecDeque::new())), "usb xd".into());
        Box::new(d.unwrap())
    }

    /// Waits for the worker's "Reconnecting" notice about `id`.
    fn reconnecting(rx: &async_channel::Receiver<Ev>, id: DeviceId) {
        loop {
            match rx.recv_blocking().expect("worker alive") {
                Ev::Error(Some(i), e) if i == id && e.contains("Reconnecting") => return,
                Ev::Error(_, e) | Ev::ConnectFailed(e) => panic!("unexpected error: {e}"),
                _ => {}
            }
        }
    }

    /// Unplugged while being reopened: the read ends, the synth is
    /// dropped, and its port is not opened again.
    #[test]
    fn gone_during_recovery_drops_without_reopening() {
        let reopened = Arc::new(Mutex::new(0));
        let count = reopened.clone();
        let slow = Retry {
            gap: Duration::from_secs(30),
            attempts: 2,
        };
        let (handle, rx) = spawn_with(
            Some(Box::new(move |d, port| {
                *count.lock().unwrap() += 1;
                Opening::start(d, threaded_link(d, None), port.into())
            })),
            slow,
        );
        let id = attach(&handle, &rx, wedged_xd());
        handle.send(Cmd::ReadAll(id));
        reconnecting(&rx, id);
        handle.send(Cmd::Gone("usb xd".into()));
        assert!(matches!(
            next(&rx, |e| matches!(e, Ev::ReadDone { .. })),
            Ev::ReadDone { id: i, cancelled: true } if i == id
        ));
        assert!(matches!(
            next(&rx, |e| matches!(e, Ev::Disconnected(_))),
            Ev::Disconnected(i) if i == id
        ));
        assert_eq!(*reopened.lock().unwrap(), 0);
    }

    /// A write that gets no reply fails as a write; the synth is reopened
    /// and answers the next op.
    #[test]
    fn a_write_that_gets_no_reply_fails_and_the_synth_recovers() {
        let (handle, rx) = spawn_with(
            Some(Box::new(|d, port| {
                Opening::start(d, threaded_link(d, None), port.into())
            })),
            FAST,
        );
        let id = attach(&handle, &rx, wedged_xd());
        let program = driver("korg-minilogue-xd").init_program();
        handle.send(Cmd::Write(id, vec![(3, program)]));
        loop {
            match rx.recv_blocking().expect("worker alive") {
                Ev::WriteFailed(i, _) if i == id => break,
                Ev::Written { .. } => panic!("nothing answered the write"),
                _ => {}
            }
        }
        handle.send(Cmd::ReadCurrent(id));
        loop {
            match rx.recv_blocking().expect("worker alive") {
                Ev::Current(i, _) if i == id => break,
                // The reconnect still under way refuses it: ask again.
                Ev::Error(Some(i), e) if i == id && e.contains("reconnecting") => {
                    handle.send(Cmd::ReadCurrent(id))
                }
                _ => {}
            }
        }
    }

    /// Reopened and silent every time: the worker gives up after its
    /// attempts, says what to do, ends the read and drops the synth.
    #[test]
    fn silence_after_every_reopen_gives_up() {
        let reopened = Arc::new(Mutex::new(0));
        let count = reopened.clone();
        let (handle, rx) = spawn_with(
            Some(Box::new(move |_, _| {
                *count.lock().unwrap() += 1;
                Err(oscyl_midi::Error::NotFound)
            })),
            FAST,
        );
        let id = attach(&handle, &rx, wedged_xd());
        handle.send(Cmd::ReadAll(id));
        reconnecting(&rx, id);
        match rx.recv_blocking().expect("worker alive") {
            Ev::Error(Some(i), e) if i == id => assert!(e.contains("does not answer"), "{e}"),
            _ => panic!("expected the give-up notice"),
        }
        assert!(matches!(
            rx.recv_blocking().unwrap(),
            Ev::ReadDone { id: i, cancelled: true } if i == id
        ));
        assert!(matches!(rx.recv_blocking().unwrap(), Ev::Disconnected(i) if i == id));
        assert_eq!(*reopened.lock().unwrap(), 2);
    }

    /// A synth that answers the device inquiry after each reopen but never
    /// the read itself is not reopened forever: the attempts count down
    /// across reconnects until an op completes, then the worker gives up.
    #[test]
    fn a_synth_that_only_answers_the_inquiry_is_given_up() {
        let reopened = Arc::new(Mutex::new(0));
        let count = reopened.clone();
        let (handle, rx) = spawn_with(
            Some(Box::new(move |d, port| {
                *count.lock().unwrap() += 1;
                Opening::start(d, Box::new(Wedged(VecDeque::new())), port.into())
            })),
            Retry {
                gap: Duration::from_millis(10),
                attempts: 1,
            },
        );
        let id = attach(&handle, &rx, wedged_xd());
        handle.send(Cmd::ReadAll(id));
        loop {
            match rx.recv_blocking().expect("worker alive") {
                Ev::Error(Some(i), e) if i == id && e.contains("does not answer") => break,
                Ev::Program(..) => panic!("nothing answered a read"),
                _ => {}
            }
        }
        assert!(matches!(
            next(&rx, |e| matches!(e, Ev::Disconnected(_))),
            Ev::Disconnected(i) if i == id
        ));
        assert_eq!(*reopened.lock().unwrap(), 1);
    }

    /// A synth that speaks only when spoken to (no clock), its messages
    /// as they go out reported on `sent`.
    struct Quiet {
        sim: Box<dyn oscyl_driver::Simulator>,
        framer: oscyl_driver::sysex::Framer,
        replies: VecDeque<Vec<u8>>,
        bell: Option<mpsc::Sender<()>>,
        sent: mpsc::Sender<Vec<u8>>,
    }

    impl oscyl_midi::Link for Quiet {
        fn send(&mut self, bytes: &[u8]) -> Result<(), oscyl_midi::Error> {
            for m in self.framer.push(bytes) {
                let _ = self.sent.send(m.clone());
                self.replies.extend(self.sim.handle(&m));
            }
            if let Some(b) = &self.bell {
                let _ = b.send(());
            }
            Ok(())
        }

        fn recv_until(&mut self, _: Instant) -> Result<Option<Vec<u8>>, oscyl_midi::Error> {
            Ok(self.replies.pop_front())
        }

        fn ring_on_input(&mut self, bell: mpsc::Sender<()>) {
            self.bell = Some(bell);
        }
    }

    /// An audition reaches the synth at once, without waiting for the
    /// synth to send something (a key press) to wake the worker.
    #[test]
    fn an_audition_goes_out_at_once() {
        let xd = driver("korg-minilogue-xd");
        let (sent, out) = mpsc::channel();
        let link = Quiet {
            sim: xd.demo_simulator(),
            framer: oscyl_driver::sysex::Framer::new(),
            replies: VecDeque::new(),
            bell: None,
            sent,
        };
        let device = Device::connect(xd, Box::new(link), "usb xd".into()).unwrap();
        let (handle, rx) = spawn();
        let id = attach(&handle, &rx, Box::new(device));
        while out.try_recv().is_ok() {}
        handle.send(Cmd::Audition(id, xd.init_program()));
        // The current-program dump (function 0x40), possibly after the
        // lag check's reads.
        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            match out.recv_timeout(left) {
                Ok(m) if m.get(6) == Some(&0x40) => break,
                Ok(_) => {}
                Err(_) => panic!("the audition never went out"),
            }
        }
    }

    /// Where the platform moves the bytes the worker cannot reopen the
    /// port: it drops the synth, so the platform opens it again.
    #[test]
    fn a_platform_link_that_stops_answering_is_dropped_for_the_platform_to_reopen() {
        let (handle, rx) = spawn_with(None, FAST);
        let id = attach(&handle, &rx, wedged_xd());
        handle.send(Cmd::ReadAll(id));
        reconnecting(&rx, id);
        assert!(matches!(
            rx.recv_blocking().unwrap(),
            Ev::ReadDone { id: i, cancelled: true } if i == id
        ));
        assert!(matches!(rx.recv_blocking().unwrap(), Ev::Disconnected(i) if i == id));
    }

    /// Escape while the synth is being reopened ends the read at once.
    #[test]
    fn cancel_during_recovery_ends_the_read() {
        let (handle, rx) = spawn_with(
            Some(Box::new(|d, port| {
                Opening::start(d, Box::new(Mute), port.into())
            })),
            Retry {
                gap: Duration::from_secs(30),
                attempts: 2,
            },
        );
        let id = attach(&handle, &rx, wedged_xd());
        handle.send(Cmd::ReadAll(id));
        reconnecting(&rx, id);
        let t0 = Instant::now();
        handle.send(Cmd::CancelRead(id));
        assert!(matches!(
            next(&rx, |e| matches!(e, Ev::ReadDone { .. })),
            Ev::ReadDone { id: i, cancelled: true } if i == id
        ));
        assert!(t0.elapsed() < Duration::from_secs(1), "{:?}", t0.elapsed());
    }

    fn attach(
        handle: &SynthHandle,
        rx: &async_channel::Receiver<Ev>,
        device: Box<Device>,
    ) -> DeviceId {
        handle.send(Cmd::Attach(device));
        match next(rx, |e| matches!(e, Ev::Connected { .. })) {
            Ev::Connected { id, .. } => id,
            _ => unreachable!(),
        }
    }

    /// Units on one synth wait for that synth only: while a minilogue
    /// xd's unit transfer is held up, a Prophet Rev2 reads all its
    /// programs; once the xd answers, its slots arrive and its next read
    /// runs without waiting out a deadline (its bell still rings).
    #[test]
    fn unit_ops_do_not_hold_up_other_synths() {
        let (handle, rx) = spawn();
        let (gate, held) = mpsc::channel();
        let xd = attach(
            &handle,
            &rx,
            threaded(driver("korg-minilogue-xd"), Some(held)),
        );
        let rev2 = attach(
            &handle,
            &rx,
            threaded(driver("sequential-prophet-rev2"), None),
        );
        handle.send(Cmd::ReadSlots(xd));
        handle.send(Cmd::ReadAll(rev2));
        let mut rev2_read = 0;
        loop {
            match next(&rx, |e| {
                matches!(e, Ev::Program(..) | Ev::ReadDone { .. } | Ev::Slots(..))
            }) {
                Ev::Program(id, ..) if id == rev2 => rev2_read += 1,
                Ev::ReadDone { id, .. } if id == rev2 => break,
                Ev::Slots(..) => panic!("the xd is held"),
                _ => {}
            }
        }
        assert_eq!(rev2_read, 1024);
        gate.send(()).unwrap();
        assert!(matches!(
            next(&rx, |e| matches!(e, Ev::Slots(..))),
            Ev::Slots(id, s) if id == xd && s.len() == 48
        ));
        let t0 = Instant::now();
        let got = read_all(&handle, &rx, &[xd]);
        assert_eq!(got[0].len(), 500);
        assert!(
            t0.elapsed() < std::time::Duration::from_secs(2),
            "{:?}",
            t0.elapsed()
        );
    }

    /// A synth whose bytes the platform moves: the xd's simulator answers
    /// through the input sender; closing it and the port going away end
    /// the device.
    #[test]
    fn open_link_connects_through_a_push_link() {
        let xd = driver("korg-minilogue-xd");
        let (link, sender, _back) = push_xd();
        let (handle, rx) = spawn();
        handle.send(Cmd::OpenLink {
            driver: xd,
            link: Box::new(link),
            port: "usb xd".into(),
        });
        let id = match next(&rx, |e| {
            matches!(e, Ev::Connected { .. } | Ev::ConnectFailed(_))
        }) {
            Ev::Connected {
                id,
                port,
                simulated,
                ..
            } => {
                assert_eq!(port, "usb xd");
                assert!(!simulated);
                id
            }
            Ev::ConnectFailed(e) => panic!("{e}"),
            _ => unreachable!(),
        };
        sender.close();
        handle.send(Cmd::Gone("usb xd".into()));
        // The closed input may end the device before `Gone` does: then an
        // error says so first.
        loop {
            match rx.recv_blocking().expect("worker alive") {
                Ev::Disconnected(i) => break assert_eq!(i, id),
                Ev::Error(Some(i), e) if i == id && e.contains("input closed") => {}
                _ => panic!("unexpected event"),
            }
        }
    }

    /// Where the simulator behind [`push_xd`] answers; empty it to leave
    /// the test's sender the only one.
    type Back = Arc<Mutex<Option<oscyl_midi::InputSender>>>;

    /// A push link whose output feeds the xd's simulator, its replies
    /// pushed back as the platform would.
    fn push_xd() -> (oscyl_midi::PushLink, oscyl_midi::InputSender, Back) {
        let xd = driver("korg-minilogue-xd");
        let input = Back::default();
        let back = input.clone();
        let (mut sim, mut framer) = (xd.simulator(), oscyl_driver::sysex::Framer::new());
        let (link, sender) = oscyl_midi::PushLink::new(Box::new(move |bytes| {
            for msg in framer.push(bytes) {
                for reply in sim.handle(&msg) {
                    if let Some(input) = back.lock().unwrap().as_ref() {
                        input.push(&reply);
                    }
                }
            }
            Ok(())
        }));
        *input.lock().unwrap() = Some(sender.clone());
        (link, sender, input)
    }

    /// A second link on a port already open fails instead of vanishing
    /// (the platform would wait on it forever).
    #[test]
    fn open_link_on_an_open_port_fails() {
        let xd = driver("korg-minilogue-xd");
        let (handle, rx) = spawn();
        for _ in 0..2 {
            let (link, _sender, _back) = push_xd();
            handle.send(Cmd::OpenLink {
                driver: xd,
                link: Box::new(link),
                port: "usb xd".into(),
            });
        }
        // The second is refused while the first may still be opening, so
        // the two answers come in either order.
        let (mut connected, mut refused) = (false, false);
        while !(connected && refused) {
            match rx.recv_blocking().expect("worker alive") {
                Ev::Connected { .. } => connected = true,
                Ev::ConnectFailed(e) if e.contains("usb xd") && !refused => refused = true,
                Ev::Error(_, e) | Ev::ConnectFailed(e) => panic!("unexpected error: {e}"),
                _ => {}
            }
        }
    }

    /// A sink dropped without `close` ends its synth too.
    #[test]
    fn dropped_input_disconnects_an_idle_device() {
        let xd = driver("korg-minilogue-xd");
        let (handle, rx) = spawn();
        let (link, sender, back) = push_xd();
        handle.send(Cmd::OpenLink {
            driver: xd,
            link: Box::new(link),
            port: "usb xd".into(),
        });
        let id = match next(&rx, |e| matches!(e, Ev::Connected { .. })) {
            Ev::Connected { id, .. } => id,
            _ => unreachable!(),
        };
        back.lock().unwrap().take();
        drop(sender);
        loop {
            match rx.recv_blocking().expect("worker alive") {
                Ev::Disconnected(i) => break assert_eq!(i, id),
                Ev::Error(Some(i), e) if i == id && e.contains("input closed") => {}
                _ => panic!("unexpected event"),
            }
        }
    }

    /// The prologue's units list through the same path.
    #[test]
    fn prologue_units_list() {
        let (handle, rx) = spawn();
        let id = connect(&handle, &rx, driver("korg-prologue"));
        handle.send(Cmd::ReadSlots(id));
        assert!(matches!(
            next(&rx, |e| matches!(e, Ev::Slots(..))),
            Ev::Slots(_, s) if s.len() == 48
        ));
    }

    /// A batch the driver refuses is a failed write, not a sync.
    #[test]
    fn a_refused_write_fails_as_a_write() {
        let (handle, rx) = spawn();
        let rev2 = driver("sequential-prophet-rev2");
        let id = connect(&handle, &rx, rev2);
        handle.send(Cmd::Write(id, vec![(3, vec![0; 3])]));
        assert!(matches!(
            rx.recv_blocking().unwrap(),
            Ev::WriteFailed(i, _) if i == id
        ));
    }

    #[test]
    fn the_current_program_is_read_from_the_edit_buffer() {
        let (handle, rx) = spawn();
        let xd = driver("korg-minilogue-xd");
        let id = connect(&handle, &rx, xd);
        handle.send(Cmd::ReadCurrent(id));
        match next(&rx, |e| matches!(e, Ev::Current(..))) {
            Ev::Current(i, p) => {
                assert_eq!(i, id);
                assert!(xd.check_program(&p).is_ok());
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn a_write_to_a_synth_that_went_fails_as_a_write() {
        let (handle, rx) = spawn();
        let id = connect(&handle, &rx, driver("korg-minilogue-xd"));
        handle.send(Cmd::Disconnect(id));
        next(&rx, |e| matches!(e, Ev::Disconnected(i) if *i == id));
        handle.send(Cmd::Write(id, vec![(3, vec![0; 3])]));
        assert!(matches!(
            next(&rx, |e| matches!(e, Ev::WriteFailed(..))),
            Ev::WriteFailed(i, _) if i == id
        ));
    }

    fn next(rx: &async_channel::Receiver<Ev>, pred: impl Fn(&Ev) -> bool) -> Ev {
        loop {
            let ev = rx.recv_blocking().expect("worker alive");
            if pred(&ev) {
                return ev;
            }
            if let Ev::Error(_, e) | Ev::ConnectFailed(e) = &ev {
                panic!("unexpected error: {e}");
            }
        }
    }

    fn connect(
        handle: &SynthHandle,
        rx: &async_channel::Receiver<Ev>,
        d: &'static dyn Driver,
    ) -> DeviceId {
        handle.send(Cmd::OpenDemo(d));
        match next(rx, |e| matches!(e, Ev::Connected { .. })) {
            Ev::Connected { id, driver, .. } => {
                assert_eq!(driver.info().id, d.info().id);
                id
            }
            _ => unreachable!(),
        }
    }

    /// Read everything `ids` hold, interleaved: each device's programs by
    /// slot.
    fn read_all(
        handle: &SynthHandle,
        rx: &async_channel::Receiver<Ev>,
        ids: &[DeviceId],
    ) -> Vec<std::collections::BTreeMap<u16, Vec<u8>>> {
        for &id in ids {
            handle.send(Cmd::ReadAll(id));
        }
        let mut got = vec![std::collections::BTreeMap::new(); ids.len()];
        let mut done = 0;
        while done < ids.len() {
            match next(rx, |e| matches!(e, Ev::Program(..) | Ev::ReadDone { .. })) {
                Ev::Program(id, n, p) => {
                    let k = ids.iter().position(|&i| i == id).unwrap();
                    got[k].insert(n, p);
                }
                Ev::ReadDone { cancelled, .. } => {
                    assert!(!cancelled);
                    done += 1;
                }
                _ => unreachable!(),
            }
        }
        got
    }

    /// The whole flow against the simulated synth: connect, read all,
    /// write and commit an edit, read it back, audition repeatedly, list
    /// and clear units.
    #[test]
    fn simulated_synth_round_trip() {
        let (handle, rx) = spawn();
        let xd = driver("korg-minilogue-xd");
        let id = connect(&handle, &rx, xd);
        let programs = read_all(&handle, &rx, &[id]).remove(0);
        assert_eq!(programs.len(), 500);

        let cutoff = xd.fields().iter().position(|f| f.name == "Cutoff").unwrap();
        let mut edited = programs[&7].clone();
        xd.set(&mut edited, cutoff, 900).unwrap();
        handle.send(Cmd::Write(id, vec![(7, edited.clone())]));
        match next(&rx, |e| matches!(e, Ev::Written { .. })) {
            Ev::Written { indices, .. } => assert_eq!(indices, vec![7]),
            _ => unreachable!(),
        }
        assert_eq!(read_all(&handle, &rx, &[id]).remove(0)[&7], edited);

        // Many auditions in a row are coalesced, never an error.
        let resonance = xd
            .fields()
            .iter()
            .position(|f| f.name == "Resonance")
            .unwrap();
        for v in 0..50 {
            let mut p = edited.clone();
            xd.set(&mut p, resonance, v * 10).unwrap();
            handle.send(Cmd::Audition(id, p));
            handle.send(Cmd::Live(id, xd.live_messages(&edited, cutoff, 0).unwrap()));
        }
        handle.send(Cmd::ReadSlots(id));
        let slots = match next(&rx, |e| matches!(e, Ev::Slots(..))) {
            Ev::Slots(_, s) => s,
            _ => unreachable!(),
        };
        assert_eq!(slots.len(), 48);

        // Clearing backs the unit up first, then empties the slot.
        let used = slots
            .iter()
            .find(|s| s.name.is_some())
            .expect("demo units")
            .clone();
        let backup =
            std::env::temp_dir().join(format!("oscyl-test-{}.slot.bin", std::process::id()));
        handle.send(Cmd::ClearSlot {
            id,
            module: used.module,
            slot: used.slot,
            backup: backup.clone(),
        });
        match next(&rx, |e| matches!(e, Ev::Slots(..))) {
            Ev::Slots(_, s) => {
                let now = s
                    .iter()
                    .find(|x| x.module == used.module && x.slot == used.slot)
                    .expect("slot");
                assert!(now.name.is_none());
            }
            _ => unreachable!(),
        }
        let saved = std::fs::read(&backup).expect("backup written");
        assert!(saved.len() > 8);
        let _ = std::fs::remove_file(backup);
    }

    /// Two minilogue xds and a Prophet Rev2 at once: reads interleave,
    /// events name their device, and a write to one xd leaves the other
    /// as it was.
    #[test]
    fn two_xds_and_a_rev2_at_once() {
        let (handle, rx) = spawn();
        let (xd, rev2) = (
            driver("korg-minilogue-xd"),
            driver("sequential-prophet-rev2"),
        );
        let ids = [
            connect(&handle, &rx, xd),
            connect(&handle, &rx, xd),
            connect(&handle, &rx, rev2),
        ];
        let before = read_all(&handle, &rx, &ids);
        assert_eq!(
            before.iter().map(|m| m.len()).collect::<Vec<_>>(),
            [500, 500, 1024]
        );
        let mut p = xd.init_program();
        xd.set_name(&mut p, "Only One").unwrap();
        handle.send(Cmd::Write(ids[0], vec![(3, p.clone())]));
        next(&rx, |e| matches!(e, Ev::Written { .. }));
        let after = read_all(&handle, &rx, &ids[..2]);
        assert_eq!(after[0][&3], p);
        assert_eq!(after[1][&3], before[1][&3]);
        // A Rev2 program goes to the Rev2 only.
        let mut q = rev2.init_program();
        rev2.set_name(&mut q, "Rev Two").unwrap();
        handle.send(Cmd::Write(ids[2], vec![(130, q.clone())]));
        match next(&rx, |e| matches!(e, Ev::Written { .. })) {
            Ev::Written { id, indices } => assert_eq!((id, indices), (ids[2], vec![130])),
            _ => unreachable!(),
        }
    }

    /// A synth unplugged mid-read is dropped (its read ends cancelled);
    /// the other carries on to the end.
    #[test]
    fn gone_cancels_and_drops_only_that_device() {
        let (handle, rx) = spawn();
        let (xd, rev2) = (
            driver("korg-minilogue-xd"),
            driver("sequential-prophet-rev2"),
        );
        let a = connect(&handle, &rx, xd);
        let b = connect(&handle, &rx, rev2);
        handle.send(Cmd::ReadAll(a));
        handle.send(Cmd::ReadAll(b));
        handle.send(Cmd::Gone("simulated Prophet Rev2".into()));
        let (mut a_read, mut b_gone, mut a_done) = (0, false, false);
        while !(b_gone && a_done) {
            match rx.recv_blocking().unwrap() {
                Ev::Program(id, ..) if id == a => a_read += 1,
                Ev::Program(id, ..) => assert!(!b_gone, "nothing from {id} once gone"),
                Ev::ReadDone { id, cancelled } if id == a => {
                    assert!(!cancelled);
                    a_done = true;
                }
                Ev::Disconnected(id) => {
                    assert_eq!(id, b);
                    b_gone = true;
                }
                Ev::Error(_, e) => panic!("{e}"),
                _ => {}
            }
        }
        assert_eq!(a_read, 500);
        handle.send(Cmd::ReadAll(b));
        assert!(matches!(
            rx.recv_blocking().unwrap(),
            Ev::Error(Some(id), _) if id == b
        ));
    }
}
