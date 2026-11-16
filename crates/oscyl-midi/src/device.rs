//! Any registered synth, several at once: which drivers exist, which MIDI
//! ports look like their synths, and a generic connection to each.

use std::collections::VecDeque;
use std::fmt;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::{Duration, Instant};

#[cfg(feature = "midir")]
use midir::{MidiInput, MidiOutput};
use oscyl_driver::engine::OpError;
use oscyl_driver::sysex::DEVICE_INQUIRY;
use oscyl_driver::{Driver, DriverError, DriverSession, SessionEvent, SessionOp};
use oscyl_minilogue_xd::session as xd;

use crate::logue::{UnitEvent, UnitOp, UnitSession, unit_session};
#[cfg(feature = "midir")]
use crate::{CLIENT, MidirLink, midi_err};
use crate::{Error, Link, SimLink};

/// Upper bound on waiting for the identity reply (normally a few ms). Only
/// reached when the port is not a responsive synth of the driver.
const INQUIRY_BOUND: Duration = Duration::from_secs(2);

/// Every driver this transport can find synths for.
pub static DRIVERS: &[&dyn Driver] = &[
    &oscyl_minilogue_xd::DRIVER,
    &oscyl_prophet_rev2::DRIVER,
    &oscyl_korg_prologue::DRIVER,
];

/// The registered driver with this [`oscyl_driver::Info::id`].
pub fn driver(id: &str) -> Option<&'static dyn Driver> {
    DRIVERS.iter().copied().find(|d| d.info().id == id)
}

/// A port whose name matches a driver's hint; not yet confirmed by the
/// device inquiry ([`crate::Device::open`] does that).
#[derive(Clone)]
pub struct Candidate {
    pub driver: &'static dyn Driver,
    /// The full port name. On ALSA it ends in `client:port`, so two synths
    /// of the same model differ.
    pub port: String,
}

impl PartialEq for Candidate {
    fn eq(&self, other: &Candidate) -> bool {
        self.driver.info().id == other.driver.info().id && self.port == other.port
    }
}

impl Eq for Candidate {}

impl fmt::Debug for Candidate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} on {:?}", self.driver.info().id, self.port)
    }
}

/// Every (port, driver) pair where the port's name contains one of the
/// driver's hints, in port order.
pub fn candidates(ports: &[String]) -> Vec<Candidate> {
    ports
        .iter()
        .flat_map(|port| {
            DRIVERS
                .iter()
                .filter(|d| d.port_hint().iter().any(|h| port.contains(h)))
                .map(|&driver| Candidate {
                    driver,
                    port: port.clone(),
                })
        })
        .collect()
}

#[cfg(feature = "midir")]
/// Candidates among the connected ports that are both an input and an
/// output.
pub fn scan() -> Result<Vec<Candidate>, Error> {
    let input = MidiInput::new(CLIENT).map_err(midi_err)?;
    let output = MidiOutput::new(CLIENT).map_err(midi_err)?;
    let ins: Vec<String> = input
        .ports()
        .iter()
        .filter_map(|p| input.port_name(p).ok())
        .collect();
    let duplex: Vec<String> = output
        .ports()
        .iter()
        .filter_map(|p| output.port_name(p).ok())
        .filter(|n| ins.contains(n))
        .collect();
    Ok(candidates(&duplex))
}

/// An open connection to one synth, through its driver's session.
///
/// A worker thread serving several devices loops: [`Device::submit`] new
/// commands, [`Device::pump`] every device, then [`wait`] on one bell that
/// every device ([`Device::ring_on_input`]) and the command sender ring,
/// until the earliest time a pump returned. [`run_all`] is that loop for a
/// fixed set of ops.
pub struct Device {
    driver: &'static dyn Driver,
    port_name: String,
    channel: u8,
    session: Box<dyn DriverSession>,
    link: Box<dyn Link>,
    /// Ops submitted that have not yet reported `Done` or `Failed`.
    outstanding: usize,
    /// The synth's answer to the device inquiry.
    identity: Vec<u8>,
    /// What input rings now, so a blocking run can put it back.
    bell: Option<Sender<()>>,
    /// The driver session has run an op, so a logue session's reply-lag
    /// check is done before unit ops (see [`Device::submit_units`]).
    checked: bool,
    /// The check's own op, as how many user ops finish before it.
    internal_at: Option<usize>,
    /// Logue unit ops; they run while the driver session is idle.
    units: Option<Box<dyn UnitSession>>,
    unit_outstanding: usize,
    unit_events: VecDeque<UnitEvent>,
    /// Session ops submitted while unit ops run: they go next.
    held: VecDeque<SessionOp>,
}

impl Device {
    #[cfg(feature = "midir")]
    /// Open a candidate's port and confirm the synth by the device inquiry.
    pub fn open(c: &Candidate) -> Result<Device, Error> {
        let link = MidirLink::open_exact(&c.port)?;
        Device::connect(c.driver, Box::new(link), c.port.clone())
    }

    /// The driver's simulator as a device, for tests and demo mode.
    pub fn simulated(driver: &'static dyn Driver) -> Result<Device, Error> {
        let link = SimLink::from_sim(driver.simulator());
        let name = format!("simulated {}", driver.info().model);
        Device::connect(driver, Box::new(link), name)
    }

    /// The driver's demo simulator ([`Driver::demo_simulator`]) as a
    /// device, for demo mode.
    pub fn demo(driver: &'static dyn Driver) -> Result<Device, Error> {
        let link = SimLink::from_sim(driver.demo_simulator());
        let name = format!("simulated {}", driver.info().model);
        Device::connect(driver, Box::new(link), name)
    }

    /// Send the device inquiry on `link` and wait for an answer `driver`
    /// recognises. Our own inquiry coming back means a looping port.
    pub fn connect(
        driver: &'static dyn Driver,
        link: Box<dyn Link>,
        port_name: String,
    ) -> Result<Device, Error> {
        let mut o = Opening::start(driver, link, port_name)?;
        loop {
            let deadline = o.deadline;
            match o.link_mut().recv_until(deadline)? {
                Some(msg) => {
                    if let Some(done) = o.answer(msg) {
                        return done;
                    }
                }
                None => return Err(Error::Silent(o.port_name)),
            }
        }
    }

    /// The device `driver` recognised in identity reply `msg`.
    fn identified(
        driver: &'static dyn Driver,
        link: Box<dyn Link>,
        port_name: String,
        msg: Vec<u8>,
    ) -> Device {
        let channel = msg.get(2).copied().filter(|c| *c < 16).unwrap_or(0);
        Device {
            driver,
            port_name,
            channel,
            session: driver.session(channel),
            link,
            outstanding: 0,
            identity: msg,
            bell: None,
            checked: false,
            internal_at: None,
            units: None,
            unit_outstanding: 0,
            unit_events: VecDeque::new(),
            held: VecDeque::new(),
        }
    }

    pub fn driver(&self) -> &'static dyn Driver {
        self.driver
    }

    pub fn port_name(&self) -> &str {
        &self.port_name
    }

    /// The MIDI channel from the identity reply. 0 also when the synth
    /// listens on all channels (it answered on `7F`); either way requests
    /// sent on it reach the synth.
    pub fn channel(&self) -> u8 {
        self.channel
    }

    /// The firmware version the synth named, for people.
    pub fn firmware(&self) -> Option<String> {
        self.driver.firmware(&self.identity)
    }

    /// Send one complete MIDI message outside the session (a live
    /// parameter change from [`Driver::live_messages`]).
    pub fn send(&mut self, msg: &[u8]) -> Result<(), Error> {
        self.link.send(msg)
    }

    /// The next complete SysEx message from the synth outside the session,
    /// or `None` once `deadline` passes (protocol probes). Only while idle.
    pub fn recv_until(&mut self, deadline: Instant) -> Result<Option<Vec<u8>>, Error> {
        self.link.recv_until(deadline)
    }

    #[cfg(feature = "midir")]
    /// The minilogue xd on `port` (the first one found when `None`), for
    /// probes and hardware tests.
    pub fn open_xd(port: Option<&str>) -> Result<Device, Error> {
        let xd = oscyl_minilogue_xd::DRIVER.info().id;
        let c = crate::scan()?
            .into_iter()
            .find(|c| c.driver.info().id == xd && port.is_none_or(|p| c.port.contains(p)))
            .ok_or(Error::NotFound)?;
        Device::open(&c)
    }

    /// Close the port now, ending its link; the device answers nothing
    /// until replaced. A minilogue xd asked for a program while it boots
    /// stops answering SysEx until its port has been closed.
    pub fn close(&mut self) {
        self.link = Box::new(Closed);
    }

    /// Ring `bell` whenever this device's synth sends something.
    pub fn ring_on_input(&mut self, bell: Sender<()>) {
        self.link.ring_on_input(bell.clone());
        self.bell = Some(bell);
    }

    /// Queue an op; it runs on the following pumps (after the unit ops
    /// queued before it).
    pub fn submit(&mut self, op: SessionOp) -> Result<(), Error> {
        if self.units_busy() {
            self.held.push_back(op);
        } else {
            self.session.submit(op)?;
        }
        self.outstanding += 1;
        Ok(())
    }

    fn units_busy(&self) -> bool {
        self.units.as_ref().is_some_and(|u| !u.is_idle())
    }

    /// Queue a logue unit op (logue synths only). It runs once the ops
    /// queued before it are done; its events come from
    /// [`Device::poll_unit_event`]. The first time, the driver session
    /// reads the edit buffer first, so a logue session's reply-lag check
    /// has cleared the firmware fault before unit reads.
    pub fn submit_units(&mut self, op: UnitOp) -> Result<(), Error> {
        if self.units.is_none() {
            self.units = Some(unit_session(self.driver, self.channel).ok_or_else(|| {
                Error::Driver(DriverError(format!(
                    "the {} has no user units",
                    self.driver.info().model
                )))
            })?);
        }
        if !self.checked && self.internal_at.is_none() && !self.units_busy() {
            self.session.submit(SessionOp::ReadCurrent)?;
            self.internal_at = Some(self.outstanding - self.held.len());
        }
        self.units.as_mut().expect("made above").submit(op);
        self.unit_outstanding += 1;
        Ok(())
    }

    /// The next event of the unit ops.
    pub fn poll_unit_event(&mut self) -> Option<UnitEvent> {
        self.unit_events.pop_front()
    }

    /// Nothing queued or in flight.
    pub fn is_idle(&self) -> bool {
        self.session.is_idle() && !self.units_busy() && self.held.is_empty()
    }

    /// End every queued and in-flight op at once, each reported as
    /// [`OpError::Cancelled`] (e.g. hotplug says the synth is gone). The
    /// device starts afresh; the caller may drop it.
    pub fn cancel(&mut self, on_event: &mut dyn FnMut(&SessionEvent)) {
        self.session = self.driver.session(self.channel);
        self.held.clear();
        self.internal_at = None;
        for _ in 0..std::mem::take(&mut self.outstanding) {
            on_event(&SessionEvent::Failed(OpError::Cancelled));
        }
        self.units = None;
        for _ in 0..std::mem::take(&mut self.unit_outstanding) {
            self.unit_events
                .push_back(UnitEvent::Failed(OpError::Cancelled));
        }
    }

    /// One non-blocking round: send what the session allows, handle input
    /// that has arrived and expired deadlines, report events. Returns when
    /// to pump again at the latest (input rings sooner), or `None` when
    /// idle.
    pub fn pump(
        &mut self,
        on_event: &mut dyn FnMut(&SessionEvent),
    ) -> Result<Option<Instant>, Error> {
        let now = Instant::now();
        // Unit ops run while the driver session has nothing to do.
        if self.units_busy() && self.session.is_idle() && self.checked {
            return self.pump_units(now);
        }
        while let Some(msg) = self.session.poll_transmit(now) {
            self.link.send(&msg)?;
        }
        while let Some(msg) = self.link.recv_until(now)? {
            self.session.handle_input(&msg);
        }
        self.session.handle_timeout(now);
        self.events(on_event);
        if self.session.is_idle() {
            // Unit ops waiting go on the next round.
            return Ok(self.units_busy().then_some(now));
        }
        match self.session.next_deadline() {
            // Everything the synth will say to what was sent is in: the
            // deadline passes now.
            Some(deadline) if self.link.synchronous() => {
                self.session.handle_timeout(deadline);
                self.events(on_event);
                Ok(Some(now))
            }
            Some(deadline) => Ok(Some(deadline)),
            // A step completed; the next one goes out on the next round.
            None => Ok(Some(now)),
        }
    }

    fn pump_units(&mut self, now: Instant) -> Result<Option<Instant>, Error> {
        let units = self.units.as_mut().expect("busy");
        while let Some(msg) = units.poll_transmit(now) {
            self.link.send(&msg)?;
        }
        while let Some(msg) = self.link.recv_until(now)? {
            units.handle_input(&msg);
        }
        units.handle_timeout(now);
        if let Some(deadline) = units.next_deadline()
            && self.link.synchronous()
        {
            // Everything the synth will say is in: the deadline passes now.
            units.handle_timeout(deadline);
        }
        let deadline = units.next_deadline();
        while let Some(e) = units.poll_event() {
            if matches!(e, UnitEvent::Done | UnitEvent::Failed(_)) {
                self.unit_outstanding = self.unit_outstanding.saturating_sub(1);
            }
            self.unit_events.push_back(e);
        }
        if !self.units_busy() {
            // The session ops held back go now.
            for op in std::mem::take(&mut self.held) {
                self.session.submit(op)?;
            }
            return Ok(Some(now));
        }
        Ok(Some(match deadline {
            Some(d) if !self.link.synchronous() => d,
            _ => now,
        }))
    }

    /// Run a logue unit op to completion, blocking (the CLI); see
    /// [`Device::submit_units`]. A failed op is an [`Error::Op`].
    pub fn run_units(
        &mut self,
        op: UnitOp,
        mut on_event: impl FnMut(&UnitEvent),
    ) -> Result<(), Error> {
        self.submit_units(op)?;
        let (bell, rung) = channel();
        let before = self.bell.clone();
        self.ring_on_input(bell);
        let result = loop {
            let next = match self.pump(&mut |_| {}) {
                Ok(n) => n,
                Err(e) => break Err(e),
            };
            let mut end = None;
            while let Some(e) = self.poll_unit_event() {
                on_event(&e);
                match e {
                    UnitEvent::Done if self.unit_outstanding == 0 => end = Some(Ok(())),
                    UnitEvent::Failed(err) => end = Some(Err(Error::Op(err))),
                    _ => {}
                }
            }
            if let Some(r) = end {
                break r;
            }
            wait(&rung, next);
        };
        if let Some(b) = before {
            self.ring_on_input(b);
        }
        result
    }

    /// Probes and xd session tests: run any minilogue xd op (programs
    /// too, outside the driver session) to completion on this device's
    /// link, blocking. Apps use [`Device::submit_units`] instead. Only while idle, so no reply of a
    /// session op can be taken for one of this op. The first time, a
    /// [`SessionOp::ReadCurrent`] runs first, so the xd session's
    /// reply-lag check has cleared the firmware fault before these reads.
    pub fn run_xd(
        &mut self,
        op: xd::Op,
        mut on_event: impl FnMut(&xd::Event),
    ) -> Result<(), Error> {
        if self.driver.info().id != oscyl_minilogue_xd::DRIVER.info().id {
            return Err(Error::Driver(DriverError(format!(
                "the {} has no user units",
                self.driver.info().model
            ))));
        }
        if !self.is_idle() {
            return Err(Error::Driver(DriverError(
                "the synth is busy with another operation".into(),
            )));
        }
        if !self.checked {
            self.run(SessionOp::ReadCurrent, |_| {})?;
            self.checked = true;
        }
        let mut session = xd::Session::new(xd::Xd {
            channel: self.channel,
        });
        session.submit(op);
        loop {
            while let Some(msg) = session.poll_transmit(Instant::now()) {
                self.link.send(&msg)?;
            }
            if let Some(deadline) = session.next_deadline() {
                match self.link.recv_until(deadline)? {
                    Some(msg) => {
                        session.handle_input(&msg);
                        session.handle_timeout(Instant::now());
                    }
                    // Nothing arrives by the deadline.
                    None => session.handle_timeout(deadline),
                }
            }
            while let Some(ev) = session.poll_event() {
                on_event(&ev);
                match ev {
                    xd::Event::OpDone => return Ok(()),
                    xd::Event::OpFailed(e) => return Err(e.into()),
                    _ => {}
                }
            }
        }
    }

    fn events(&mut self, on_event: &mut dyn FnMut(&SessionEvent)) {
        while let Some(e) = self.session.poll_event() {
            let ends = matches!(e, SessionEvent::Done | SessionEvent::Failed(_));
            // The check's own op reports nothing.
            if self.internal_at == Some(0) {
                if ends {
                    self.internal_at = None;
                    self.checked = true;
                }
                continue;
            }
            if ends {
                // The session has run an op: its lag check is done.
                self.checked = true;
                self.outstanding = self.outstanding.saturating_sub(1);
                if let Some(k) = self.internal_at.as_mut() {
                    *k -= 1;
                }
            }
            on_event(&e);
        }
    }

    /// Run `op` (and anything queued before it) to completion. A failed op
    /// is an [`Error::Op`].
    pub fn run(
        &mut self,
        op: SessionOp,
        mut on_event: impl FnMut(&SessionEvent),
    ) -> Result<(), Error> {
        self.submit(op)?;
        let mut failed = None;
        let mut results = run_all(&mut [self], |_, e| {
            if let SessionEvent::Failed(err) = e {
                failed.get_or_insert(err.clone());
            }
            on_event(e);
        });
        results.pop().expect("one device")?;
        failed.map_or(Ok(()), |e| Err(Error::Op(e)))
    }
}

/// The link of a closed [`Device`].
struct Closed;

impl Link for Closed {
    fn send(&mut self, _: &[u8]) -> Result<(), Error> {
        Err(Error::Disconnected("closed".into()))
    }

    fn recv_until(&mut self, _: Instant) -> Result<Option<Vec<u8>>, Error> {
        Err(Error::Disconnected("closed".into()))
    }
}

/// A synth being opened: the device inquiry is out, the answer awaited
/// without blocking ([`Opening::poll`] from a pumping loop).
pub struct Opening {
    driver: &'static dyn Driver,
    /// Taken by the device once recognised.
    link: Option<Box<dyn Link>>,
    port_name: String,
    deadline: Instant,
}

/// Where an [`Opening`] stands.
pub enum Opened {
    Ready(Box<Device>),
    Waiting(Opening),
    Failed(Error),
}

impl Opening {
    #[cfg(feature = "midir")]
    /// Open a candidate's port and send the inquiry.
    pub fn open(c: &Candidate) -> Result<Opening, Error> {
        let link = MidirLink::open_exact(&c.port)?;
        Opening::start(c.driver, Box::new(link), c.port.clone())
    }

    /// Send the inquiry on `link`.
    pub fn start(
        driver: &'static dyn Driver,
        mut link: Box<dyn Link>,
        port_name: String,
    ) -> Result<Opening, Error> {
        link.send(&DEVICE_INQUIRY)?;
        Ok(Opening {
            driver,
            link: Some(link),
            port_name,
            deadline: Instant::now() + INQUIRY_BOUND,
        })
    }

    pub fn driver(&self) -> &'static dyn Driver {
        self.driver
    }

    pub fn port_name(&self) -> &str {
        &self.port_name
    }

    /// Ring `bell` when the synth answers.
    pub fn ring_on_input(&mut self, bell: Sender<()>) {
        self.link_mut().ring_on_input(bell);
    }

    /// The device, an error, or `None` for a message that says nothing.
    fn answer(&mut self, msg: Vec<u8>) -> Option<Result<Device, Error>> {
        if msg == DEVICE_INQUIRY {
            return Some(Err(Error::Loopback(self.port_name.clone())));
        }
        self.driver.recognize(&msg).then(|| {
            let link = self.link.take().expect("answered once");
            Ok(Device::identified(
                self.driver,
                link,
                std::mem::take(&mut self.port_name),
                msg,
            ))
        })
    }

    /// Read what has arrived: the device once recognised, a failure, or
    /// still waiting (come back by [`Opening::deadline`] at the latest).
    pub fn poll(mut self, now: Instant) -> Opened {
        loop {
            match self.link_mut().recv_until(now) {
                Ok(Some(msg)) => match self.answer(msg) {
                    Some(Ok(d)) => return Opened::Ready(Box::new(d)),
                    Some(Err(e)) => return Opened::Failed(e),
                    None => {}
                },
                Ok(None) if now >= self.deadline || self.link_mut().synchronous() => {
                    return Opened::Failed(Error::Silent(self.port_name));
                }
                Ok(None) => return Opened::Waiting(self),
                Err(e) => return Opened::Failed(e),
            }
        }
    }

    fn link_mut(&mut self) -> &mut Box<dyn Link> {
        self.link.as_mut().expect("open until answered")
    }

    /// When the inquiry is given up.
    pub fn deadline(&self) -> Instant {
        self.deadline
    }
}

/// Block until `bell` rings or `until` passes (`None`: until it rings).
/// Rings that piled up meanwhile are drained: their input is already
/// queued for the next pump. Keep a sender of `bell` alive, or a `None`
/// wait returns at once.
pub fn wait(bell: &Receiver<()>, until: Option<Instant>) {
    let _ = match until {
        Some(t) => bell
            .recv_timeout(t.saturating_duration_since(Instant::now()))
            .ok(),
        None => bell.recv().ok(),
    };
    while bell.try_recv().is_ok() {}
}

/// Run every device's queued ops at once in this thread, interleaved, until
/// all are idle. `on_event` gets each event with its device's index. The
/// results hold transport errors per device (such a device stops; cancel
/// it); op failures are [`SessionEvent::Failed`] events.
pub fn run_all(
    devices: &mut [&mut Device],
    mut on_event: impl FnMut(usize, &SessionEvent),
) -> Vec<Result<(), Error>> {
    let (bell, rung) = channel();
    // A device's own bell (a worker's) rings again once this returns.
    let before: Vec<Option<Sender<()>>> = devices.iter().map(|d| d.bell.clone()).collect();
    for d in devices.iter_mut() {
        d.ring_on_input(bell.clone());
    }
    let mut results: Vec<Result<(), Error>> = devices.iter().map(|_| Ok(())).collect();
    loop {
        let mut next: Option<Instant> = None;
        for (i, d) in devices.iter_mut().enumerate() {
            if results[i].is_err() {
                continue;
            }
            match d.pump(&mut |e| on_event(i, e)) {
                Ok(Some(t)) => next = Some(next.map_or(t, |n| n.min(t))),
                Ok(None) => {}
                Err(e) => results[i] = Err(e),
            }
        }
        if next.is_none() {
            for (d, b) in devices.iter_mut().zip(before) {
                if let Some(b) = b {
                    d.ring_on_input(b);
                }
            }
            return results;
        }
        wait(&rung, next);
    }
}
