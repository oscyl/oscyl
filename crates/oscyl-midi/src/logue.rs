//! The logue family's own operations (user units, global and tuning
//! data), which the driver contract does not model, for every logue
//! driver: the minilogue xd and the prologue. A [`Device`](crate::Device)
//! runs them between its driver session's ops, pumped like them.

use std::time::Instant;

use oscyl_driver::Driver;
use oscyl_driver::engine::{Engine, OpError};
use oscyl_logue::Model;
use oscyl_logue::session::{Event, Op, Proto};

pub use oscyl_logue::msg::Module;
pub use oscyl_logue::unit::{Platform, unit_name};

/// A logue operation on user slots and settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnitOp {
    /// The status of every user slot.
    ReadSlots,
    ReadSlotData {
        module: Module,
        slot: u8,
    },
    /// Load TABLE 7 slot data (`u32 size, u32 checksum, blob`).
    UploadUnit {
        module: Module,
        slot: u8,
        data: Vec<u8>,
    },
    ClearSlot {
        module: Module,
        slot: u8,
    },
    ReadGlobal,
    /// The 6 user scales and 6 user octaves.
    ReadTuning,
}

/// What [`UnitOp`]s report, in order; each op ends with `Done` or
/// `Failed`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnitEvent {
    Slot {
        module: Module,
        slot: u8,
        header: Option<Vec<u8>>,
    },
    SlotData {
        module: Module,
        slot: u8,
        data: Vec<u8>,
    },
    Global(Vec<u8>),
    Scale(u8, Vec<u8>),
    Octave(u8, Vec<u8>),
    Done,
    Failed(OpError),
}

/// A sans-IO session speaking [`UnitOp`].
pub trait UnitSession: Send {
    fn submit(&mut self, op: UnitOp);
    fn is_idle(&self) -> bool;
    fn poll_transmit(&mut self, now: Instant) -> Option<Vec<u8>>;
    fn next_deadline(&self) -> Option<Instant>;
    fn handle_timeout(&mut self, now: Instant);
    fn handle_input(&mut self, bytes: &[u8]);
    fn poll_event(&mut self) -> Option<UnitEvent>;
}

struct Logue<M: Model>(Engine<Proto<M>>);

impl<M: Model> UnitSession for Logue<M> {
    fn submit(&mut self, op: UnitOp) {
        self.0.submit(match op {
            UnitOp::ReadSlots => Op::ReadSlots,
            UnitOp::ReadSlotData { module, slot } => Op::ReadSlotData { module, slot },
            UnitOp::UploadUnit { module, slot, data } => Op::UploadUnit { module, slot, data },
            UnitOp::ClearSlot { module, slot } => Op::ClearSlot { module, slot },
            UnitOp::ReadGlobal => Op::ReadGlobal,
            UnitOp::ReadTuning => Op::ReadTuning,
        })
    }

    fn is_idle(&self) -> bool {
        self.0.is_idle()
    }

    fn poll_transmit(&mut self, now: Instant) -> Option<Vec<u8>> {
        self.0.poll_transmit(now)
    }

    fn next_deadline(&self) -> Option<Instant> {
        self.0.next_deadline()
    }

    fn handle_timeout(&mut self, now: Instant) {
        self.0.handle_timeout(now)
    }

    fn handle_input(&mut self, bytes: &[u8]) {
        self.0.handle_input(bytes)
    }

    fn poll_event(&mut self) -> Option<UnitEvent> {
        loop {
            return Some(match self.0.poll_event()? {
                Event::Slot {
                    module,
                    slot,
                    header,
                } => UnitEvent::Slot {
                    module,
                    slot,
                    header,
                },
                Event::SlotData { module, slot, data } => {
                    UnitEvent::SlotData { module, slot, data }
                }
                Event::Global(g) => UnitEvent::Global(g),
                Event::Scale(n, v) => UnitEvent::Scale(n, v),
                Event::Octave(n, v) => UnitEvent::Octave(n, v),
                Event::OpDone => UnitEvent::Done,
                Event::OpFailed(e) => UnitEvent::Failed(e),
                // Program events come only from program ops, never
                // submitted here.
                _ => continue,
            });
        }
    }
}

type MakeSession = fn(u8) -> Box<dyn UnitSession>;

fn session<M: Model>(channel: u8) -> Box<dyn UnitSession> {
    Box::new(Logue::<M>(Engine::new(Proto::new(channel))))
}

/// `d`'s logue model: its unit session maker, SDK platform and unit file
/// extension. `None` for a synth outside the logue family.
fn model(d: &dyn Driver) -> Option<(MakeSession, Platform, &'static str)> {
    use oscyl_korg_minilogue_xd::model::XdModel;
    use oscyl_korg_prologue::model::PrologueModel;
    let id = d.info().id;
    if id == oscyl_korg_minilogue_xd::DRIVER.info().id {
        Some((session::<XdModel>, XdModel::PLATFORM, "mnlgxdunit"))
    } else if id == oscyl_korg_prologue::DRIVER.info().id {
        Some((
            session::<PrologueModel>,
            PrologueModel::PLATFORM,
            "prlgunit",
        ))
    } else {
        None
    }
}

/// A session for `d`'s logue operations on `channel`, if it is a logue
/// synth.
pub fn unit_session(d: &dyn Driver, channel: u8) -> Option<Box<dyn UnitSession>> {
    model(d).map(|(make, ..)| make(channel))
}

/// The logue SDK platform `d`'s units are built for, if it is a logue
/// synth.
pub fn platform(d: &dyn Driver) -> Option<Platform> {
    model(d).map(|(_, p, _)| p)
}

/// The extension of `d`'s logue SDK unit files (`.mnlgxdunit`,
/// `.prlgunit`), if it is a logue synth.
pub fn unit_ext(d: &dyn Driver) -> Option<&'static str> {
    model(d).map(|(.., e)| e)
}
