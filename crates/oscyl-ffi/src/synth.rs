//! The synth worker as a UniFFI object: commands are methods, events go to
//! a foreign [`SynthListener`] from a Rust thread of their own, so a slow
//! listener never holds up the worker.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;

use oscyl_app::synth::{Cmd, Ev, SynthHandle, spawn};
use oscyl_driver::Driver;
use oscyl_driver::sysex::Framer;
use oscyl_midi::{InputSender, PushLink};

/// The platform's MIDI output to one synth (Android `MidiInputPort`,
/// CoreMIDI). Each call carries one or more whole messages.
#[uniffi::export(with_foreign)]
pub trait MidiOut: Send + Sync {
    fn send(&self, bytes: Vec<u8>) -> Result<(), LinkError>;
}

/// Why the platform could not send.
#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum LinkError {
    #[error("{reason}")]
    Closed { reason: String },
}

impl From<uniffi::UnexpectedUniFFICallbackError> for LinkError {
    fn from(e: uniffi::UnexpectedUniFFICallbackError) -> LinkError {
        LinkError::Closed { reason: e.reason }
    }
}

/// Why a call into the core failed. `reason` is for the person (through
/// the core's translation hook where the core words it; file system errors
/// keep the system's words).
#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum FfiError {
    #[error("no driver {id:?}")]
    UnknownDriver { id: String },
    /// A library, folder or file could not be read or written.
    #[error("{reason}")]
    Failed { reason: String },
    /// The change is not allowed (a factory slot, a name the synth can't hold).
    #[error("{reason}")]
    Refused { reason: String },
}

impl FfiError {
    pub(crate) fn failed(reason: impl Into<String>) -> FfiError {
        FfiError::Failed {
            reason: reason.into(),
        }
    }
}

/// Gets every event the worker sends, in order, on a Rust thread.
#[uniffi::export(with_foreign)]
pub trait SynthListener: Send + Sync {
    fn on_event(&self, event: SynthEvent);
}

/// A program for one slot of a write.
#[derive(Debug, Clone, uniffi::Record)]
pub struct SlotProgram {
    pub slot: u16,
    pub data: Vec<u8>,
}

/// One user-unit slot (logue family).
#[derive(Debug, Clone, uniffi::Record)]
pub struct SlotInfo {
    /// The module's short name (`osc`, `modfx`, `delfx`, `revfx`).
    pub module: String,
    pub slot: u8,
    pub name: Option<String>,
}

/// What the worker reports; mirrors `oscyl_app::synth::Ev`.
#[derive(Debug, Clone, uniffi::Enum)]
pub enum SynthEvent {
    Connected {
        id: u64,
        driver_id: String,
        port: String,
        firmware: Option<String>,
        simulated: bool,
    },
    /// Why, and what to do, for an error dialog.
    ConnectFailed {
        message: String,
    },
    Disconnected {
        id: u64,
    },
    ReadProgress {
        id: u64,
        done: u64,
        total: u64,
    },
    Program {
        id: u64,
        slot: u16,
        data: Vec<u8>,
    },
    ReadDone {
        id: u64,
        cancelled: bool,
    },
    WriteProgress {
        id: u64,
        done: u64,
        total: u64,
    },
    Written {
        id: u64,
        indices: Vec<u16>,
    },
    WriteFailed {
        id: u64,
        message: String,
    },
    Slots {
        id: u64,
        slots: Vec<SlotInfo>,
    },
    /// The program the synth is playing now (its edit buffer).
    Current {
        id: u64,
        data: Vec<u8>,
    },
    Error {
        id: Option<u64>,
        message: String,
    },
    /// The synth's own panel picked program `slot` (it plays it now).
    Selected {
        id: u64,
        slot: u16,
    },
    /// Control Changes the synth's panel sent (knobs turned), in order:
    /// for a `PanelDecoder`.
    Panel {
        id: u64,
        controls: Vec<ControlChange>,
    },
}

/// One Control Change.
#[derive(Debug, Clone, uniffi::Record)]
pub struct ControlChange {
    pub cc: u8,
    pub value: u8,
}

impl From<Ev> for SynthEvent {
    fn from(ev: Ev) -> SynthEvent {
        match ev {
            Ev::Connected {
                id,
                driver,
                port,
                firmware,
                simulated,
            } => SynthEvent::Connected {
                id,
                driver_id: driver.info().id.into(),
                port,
                firmware,
                simulated,
            },
            Ev::ConnectFailed(message) => SynthEvent::ConnectFailed { message },
            Ev::Disconnected(id) => SynthEvent::Disconnected { id },
            Ev::ReadProgress { id, done, total } => SynthEvent::ReadProgress {
                id,
                done: done as u64,
                total: total as u64,
            },
            Ev::Program(id, slot, data) => SynthEvent::Program { id, slot, data },
            Ev::ReadDone { id, cancelled } => SynthEvent::ReadDone { id, cancelled },
            Ev::WriteProgress { id, done, total } => SynthEvent::WriteProgress {
                id,
                done: done as u64,
                total: total as u64,
            },
            Ev::Written { id, indices } => SynthEvent::Written { id, indices },
            Ev::WriteFailed(id, message) => SynthEvent::WriteFailed { id, message },
            Ev::Slots(id, slots) => SynthEvent::Slots {
                id,
                slots: slots
                    .into_iter()
                    .map(|s| SlotInfo {
                        module: s.module.name().into(),
                        slot: s.slot,
                        name: s.name,
                    })
                    .collect(),
            },
            Ev::Error(id, message) => SynthEvent::Error { id, message },
            Ev::Current(id, data) => SynthEvent::Current { id, data },
            Ev::Selected(id, slot) => SynthEvent::Selected { id, slot },
            Ev::Panel(id, ccs) => SynthEvent::Panel {
                id,
                controls: ccs
                    .into_iter()
                    .map(|(cc, value)| ControlChange { cc, value })
                    .collect(),
            },
        }
    }
}

/// Where the platform pushes what one synth sends.
#[derive(uniffi::Object)]
pub struct MidiInputSink(InputSender);

#[uniffi::export]
impl MidiInputSink {
    /// Received bytes, in any fragments. Ignored once disconnected.
    pub fn push(&self, bytes: Vec<u8>) {
        self.0.push(&bytes);
    }

    /// The port is gone; the synth disconnects once its input is read.
    /// Idempotent, and a late `push` is ignored rather than failing. (Not
    /// `close`: on Kotlin that is `AutoCloseable.close`, which releases the
    /// object, after which any call throws.)
    pub fn disconnect(&self) {
        self.0.close();
    }
}

/// The worker that owns every connected synth.
///
/// Call [`Synth::quit`] when done with it (`onCleared`, `close`): that is
/// the shutdown. Dropping it also quits, but only as a fallback: the event
/// thread holds the listener and the worker holds every `MidiOut`, so a
/// listener or output that refers back to the Synth keeps it alive and the
/// drop never comes.
#[derive(uniffi::Object)]
pub struct Synth {
    handle: SynthHandle,
    #[cfg(test)]
    events: async_channel::Receiver<Ev>,
    #[cfg(test)]
    drain: std::sync::Mutex<Option<std::thread::JoinHandle<()>>>,
}

/// A unit section by its short name.
pub(crate) fn module_of(name: &str) -> Result<oscyl_midi::logue::Module, FfiError> {
    oscyl_midi::logue::Module::ALL
        .into_iter()
        .find(|m| m.name() == name)
        .ok_or_else(|| FfiError::Refused {
            reason: format!("no unit section {name:?}"),
        })
}

/// A driver's simulated synth answering raw MIDI: what a platform's tests
/// put behind a fake port, so it answers as the synth would (its demo
/// programs).
#[derive(uniffi::Object)]
pub struct Simulator(std::sync::Mutex<(Box<dyn oscyl_driver::Simulator>, Framer)>);

#[uniffi::export]
impl Simulator {
    #[uniffi::constructor]
    pub fn new(driver_id: String) -> Result<Arc<Simulator>, FfiError> {
        let sim = driver(&driver_id)?.demo_simulator();
        Ok(Arc::new(Simulator(std::sync::Mutex::new((
            sim,
            Framer::new(),
        )))))
    }

    /// The synth's replies to `bytes` (any part of a message: it is framed
    /// with what came before), one message each.
    pub fn handle(&self, bytes: Vec<u8>) -> Vec<Vec<u8>> {
        let mut guard = self.0.lock().unwrap_or_else(|e| e.into_inner());
        let (sim, framer) = &mut *guard;
        framer
            .push(&bytes)
            .iter()
            .flat_map(|m| sim.handle(m))
            .collect()
    }
}

pub(crate) fn driver(id: &str) -> Result<&'static dyn Driver, FfiError> {
    oscyl_midi::driver(id).ok_or_else(|| FfiError::UnknownDriver { id: id.into() })
}

#[uniffi::export]
impl Synth {
    /// Start the worker; `listener` gets its events until it stops.
    #[uniffi::constructor]
    pub fn new(listener: Arc<dyn SynthListener>) -> Arc<Synth> {
        let (handle, events) = spawn();
        #[cfg(test)]
        let mine = events.clone();
        // Ends when the worker does (its sender goes); a panicking
        // listener loses that event only.
        let drain = std::thread::Builder::new()
            .name("oscyl-events".into())
            .spawn(move || {
                while let Ok(ev) = events.recv_blocking() {
                    let _ = catch_unwind(AssertUnwindSafe(|| listener.on_event(ev.into())));
                }
            })
            .expect("spawn event thread");
        #[cfg(not(test))]
        drop(drain);
        Arc::new(Synth {
            handle,
            #[cfg(test)]
            events: mine,
            #[cfg(test)]
            drain: std::sync::Mutex::new(Some(drain)),
        })
    }

    /// Connect to the driver's simulated synth (demo mode).
    pub fn open_demo(&self, driver_id: String) -> Result<(), FfiError> {
        self.handle.send(Cmd::OpenDemo(driver(&driver_id)?));
        Ok(())
    }

    /// Connect to the driver's synth on a port the platform opened: Rust
    /// sends through `out`, the platform pushes received bytes into the
    /// returned sink. `port` names it for [`Synth::gone`].
    pub fn open_link(
        &self,
        driver_id: String,
        port: String,
        out: Arc<dyn MidiOut>,
    ) -> Result<Arc<MidiInputSink>, FfiError> {
        let driver = driver(&driver_id)?;
        let (link, input) = PushLink::new(Box::new(move |bytes: &[u8]| {
            out.send(bytes.to_vec())
                .map_err(|LinkError::Closed { reason }| oscyl_midi::Error::Disconnected(reason))
        }));
        self.handle.send(Cmd::OpenLink {
            driver,
            link: Box::new(link),
            port,
        });
        Ok(Arc::new(MidiInputSink(input)))
    }

    /// The port went away: end what its synth was doing, drop it.
    pub fn gone(&self, port: String) {
        self.handle.send(Cmd::Gone(port));
    }

    pub fn disconnect(&self, id: u64) {
        self.handle.send(Cmd::Disconnect(id));
    }

    /// Read every program (stopped by [`Synth::cancel_read`]).
    pub fn read_all(&self, id: u64) {
        self.handle.send(Cmd::ReadAll(id));
    }

    pub fn cancel_read(&self, id: u64) {
        self.handle.send(Cmd::CancelRead(id));
    }

    /// Play a program from the edit buffer.
    pub fn audition(&self, id: u64, program: Vec<u8>) {
        self.handle.send(Cmd::Audition(id, program));
    }

    /// Live parameter changes, one MIDI message each, sent in order.
    pub fn live(&self, id: u64, messages: Vec<Vec<u8>>) {
        self.handle.send(Cmd::Live(id, messages));
    }

    /// Write programs to slots; the driver plans the safe commit.
    pub fn write(&self, id: u64, programs: Vec<SlotProgram>) {
        let programs = programs.into_iter().map(|p| (p.slot, p.data)).collect();
        self.handle.send(Cmd::Write(id, programs));
    }

    /// List the user-unit slots (logue family).
    pub fn read_slots(&self, id: u64) {
        self.handle.send(Cmd::ReadSlots(id));
    }

    /// Read the program the synth is playing (a `Current` event follows).
    pub fn read_current(&self, id: u64) {
        self.handle.send(Cmd::ReadCurrent(id));
    }

    /// Load unit slot data (from [`crate::read_unit`]) into slot `slot` of
    /// section `module` (`osc`, `modfx`, `delfx`, `revfx`); the slots are
    /// listed again after.
    pub fn upload_unit(
        &self,
        id: u64,
        module: String,
        slot: u8,
        data: Vec<u8>,
    ) -> Result<(), FfiError> {
        self.handle.send(Cmd::UploadUnit {
            id,
            module: module_of(&module)?,
            slot,
            data,
        });
        Ok(())
    }

    /// Load `file` of the shelf into the first free slot of its section on
    /// synth `id` (an `Error` event follows when the section is full); the
    /// slots are listed again after. The unit's name. Blocks (reads it).
    pub fn load_from_shelf(
        &self,
        id: u64,
        driver_id: String,
        shelf: Arc<crate::units::Shelf>,
        file: String,
    ) -> Result<String, FfiError> {
        let unit = shelf
            .0
            .unit(driver(&driver_id)?, &file)
            .map_err(|reason| FfiError::Refused { reason })?;
        self.handle.send(Cmd::LoadUnit {
            id,
            module: unit.module,
            data: unit.data,
        });
        Ok(unit.name)
    }

    /// Save the slot's unit to the file `backup`, then clear the slot (never
    /// without the backup); the slots are listed again after.
    pub fn clear_slot(
        &self,
        id: u64,
        module: String,
        slot: u8,
        backup: String,
    ) -> Result<(), FfiError> {
        self.handle.send(Cmd::ClearSlot {
            id,
            module: module_of(&module)?,
            slot,
            backup: backup.into(),
        });
        Ok(())
    }

    /// End every synth's work and stop the worker and the event thread
    /// (once the listener returns). Required; later calls do nothing.
    pub fn quit(&self) {
        self.handle.send(Cmd::Quit);
    }
}

impl Drop for Synth {
    fn drop(&mut self) {
        self.handle.send(Cmd::Quit);
    }
}

#[cfg(test)]
impl Synth {
    fn take_drain(&self) -> std::thread::JoinHandle<()> {
        self.drain.lock().unwrap().take().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    /// The simulator answers what a real minilogue xd answers, framed: the
    /// identity, then program 0 when asked for it.
    #[test]
    fn simulator_answers_like_the_synth() {
        let sim = Simulator::new(XD.into()).unwrap();
        let identity = sim.handle(vec![0xF0, 0x7E, 0x7F, 0x06, 0x01, 0xF7]);
        assert_eq!(identity.len(), 1);
        assert_eq!(&identity[0][..6], &[0xF0, 0x7E, 0x00, 0x06, 0x02, 0x42]);
        // Split across two sends, as a platform may deliver it.
        assert!(sim.handle(vec![0xF0, 0x42, 0x30, 0x00]).is_empty());
        let program = sim.handle(vec![0x01, 0x51, 0x1C, 0x00, 0x00, 0xF7]);
        assert_eq!(
            &program[0][..7],
            &[0xF0, 0x42, 0x30, 0x00, 0x01, 0x51, 0x4C]
        );
        assert!(Simulator::new("nope".into()).is_err());
    }
    use std::sync::{Arc, Mutex};

    const XD: &str = "korg-minilogue-xd";

    /// Sends every event into a channel.
    struct Collect(Mutex<mpsc::Sender<SynthEvent>>);

    impl SynthListener for Collect {
        fn on_event(&self, event: SynthEvent) {
            let _ = self.0.lock().unwrap().send(event);
        }
    }

    fn collecting() -> (Arc<Collect>, mpsc::Receiver<SynthEvent>) {
        let (tx, rx) = mpsc::channel();
        (Arc::new(Collect(Mutex::new(tx))), rx)
    }

    fn next(rx: &mpsc::Receiver<SynthEvent>, pred: impl Fn(&SynthEvent) -> bool) -> SynthEvent {
        loop {
            let ev = rx.recv().expect("synth alive");
            if pred(&ev) {
                return ev;
            }
        }
    }

    #[test]
    fn demo_connects_and_reads() {
        let (listener, rx) = collecting();
        let synth = Synth::new(listener);
        synth.open_demo(XD.into()).unwrap();
        let id = match next(&rx, |e| matches!(e, SynthEvent::Connected { .. })) {
            SynthEvent::Connected {
                id,
                driver_id,
                simulated,
                ..
            } => {
                assert_eq!(driver_id, XD);
                assert!(simulated);
                id
            }
            _ => unreachable!(),
        };
        synth.read_all(id);
        assert!(matches!(
            next(&rx, |e| matches!(e, SynthEvent::ReadDone { .. })),
            SynthEvent::ReadDone { id: i, cancelled: false } if i == id
        ));
        synth.quit();
    }

    #[test]
    fn push_after_disconnect_is_ignored() {
        use oscyl_midi::{Error, Link};
        use std::time::{Duration, Instant};

        let (mut link, input) = PushLink::new(Box::new(|_| Ok(())));
        let sink = MidiInputSink(input);
        sink.push(vec![0xF0, 0x01, 0xF7]);
        sink.disconnect();
        sink.disconnect();
        sink.push(vec![0xF0, 0x02, 0xF7]);
        let soon = || Instant::now() + Duration::from_secs(1);
        assert_eq!(
            link.recv_until(soon()).unwrap(),
            Some(vec![0xF0, 0x01, 0xF7])
        );
        assert!(matches!(
            link.recv_until(soon()),
            Err(Error::Disconnected(_))
        ));
    }

    #[test]
    fn unknown_driver_is_an_error() {
        let (listener, _rx) = collecting();
        let synth = Synth::new(listener);
        assert!(matches!(
            synth.open_demo("nope".into()),
            Err(FfiError::UnknownDriver { id }) if id == "nope"
        ));
        assert!(matches!(
            synth.open_link("nope".into(), "p".into(), Arc::new(Closed)),
            Err(FfiError::UnknownDriver { .. })
        ));
    }

    /// The device went away on the platform side.
    struct Closed;

    impl MidiOut for Closed {
        fn send(&self, _bytes: Vec<u8>) -> Result<(), LinkError> {
            Err(LinkError::Closed {
                reason: "unplugged".into(),
            })
        }
    }

    #[test]
    fn failing_out_disconnects() {
        let (listener, rx) = collecting();
        let synth = Synth::new(listener);
        synth
            .open_link(XD.into(), "usb xd".into(), Arc::new(Closed))
            .unwrap();
        match next(&rx, |_| true) {
            SynthEvent::ConnectFailed { message } => assert!(message.contains("unplugged")),
            e => panic!("{e:?}"),
        }
    }

    /// Holds its first event until the test lets it go: says it has
    /// one on `entered`, then waits on `release`.
    type Hold = (mpsc::Sender<()>, mpsc::Receiver<()>);
    struct Slow(Mutex<Option<Hold>>);

    impl SynthListener for Slow {
        fn on_event(&self, _event: SynthEvent) {
            let first = self.0.lock().unwrap().take();
            if let Some((entered, release)) = first {
                entered.send(()).unwrap();
                let _ = release.recv();
            }
        }
    }

    #[test]
    fn slow_listener_does_not_block_quit() {
        let (entered, holding) = mpsc::channel();
        let (release, held) = mpsc::channel();
        let synth = Synth::new(Arc::new(Slow(Mutex::new(Some((entered, held))))));
        let (events, drain) = (synth.events.clone(), synth.take_drain());
        synth.open_demo(XD.into()).unwrap();
        holding.recv().unwrap();
        synth.quit();
        // The worker ends (its event channel closes) while the listener
        // still holds its first event.
        while events.recv_blocking().is_ok() {}
        release.send(()).unwrap();
        drain.join().unwrap();
    }

    #[test]
    fn drop_without_quit_stops_threads() {
        let (listener, _rx) = collecting();
        let synth = Synth::new(listener.clone());
        synth.open_demo(XD.into()).unwrap();
        let drain = synth.take_drain();
        drop(synth);
        drain.join().unwrap();
        assert_eq!(Arc::strong_count(&listener), 1);
    }

    /// A listener that panics does not stop later events.
    struct Panicky(Mutex<mpsc::Sender<SynthEvent>>);

    impl SynthListener for Panicky {
        fn on_event(&self, event: SynthEvent) {
            if matches!(event, SynthEvent::Connected { .. }) {
                panic!("listener bug");
            }
            let _ = self.0.lock().unwrap().send(event);
        }
    }

    #[test]
    fn panicking_listener_keeps_receiving() {
        let (tx, rx) = mpsc::channel();
        let synth = Synth::new(Arc::new(Panicky(Mutex::new(tx))));
        synth.open_demo(XD.into()).unwrap();
        synth.disconnect(1);
        assert!(matches!(
            next(&rx, |e| matches!(e, SynthEvent::Disconnected { .. })),
            SynthEvent::Disconnected { id: 1 }
        ));
    }
}
