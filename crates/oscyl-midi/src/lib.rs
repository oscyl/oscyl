//! USB-MIDI transport for Oscyl synth drivers.
//!
//! [`candidates`] picks the ports any registered driver ([`DRIVERS`])
//! claims; a [`Device`] drives one synth's driver session over a [`Link`]:
//! `MidirLink` for a real synth on desktop (feature `midir`: ALSA on Linux, CoreMIDI on macOS, WinMM on Windows;
//! input arrives through midir's callback as byte chunks and is framed into
//! SysEx messages), [`PushLink`] on platforms that move the bytes
//! themselves (Android, iOS), or [`SimLink`] for a simulated one. One
//! thread can serve several devices at once.

mod device;
#[cfg(all(feature = "midir", target_os = "linux"))]
pub mod hotplug;
pub mod logue;
mod parse;
mod push;
#[cfg(feature = "midir")]
pub use device::scan;
pub use device::{Candidate, DRIVERS, Device, Opened, Opening, candidates, driver, run_all, wait};
pub use push::{InputSender, PushLink};
use std::collections::VecDeque;
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, channel};
use std::sync::{Arc, Mutex};
use std::time::Instant;

#[cfg(feature = "midir")]
use midir::{Ignore, MidiInput, MidiInputConnection, MidiOutput, MidiOutputConnection};
use oscyl_driver::engine::OpError;
use oscyl_driver::sysex::Framer;
use oscyl_driver::{DriverError, Simulator};
use oscyl_korg_minilogue_xd::adapter::XdSimulator;
use oscyl_korg_minilogue_xd::sim::SimDevice;

#[cfg(feature = "midir")]
const CLIENT: &str = "oscyl";

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("no supported synth found")]
    NotFound,
    #[error("more than one MIDI port matches {0:?}")]
    Ambiguous(String),
    #[error("MIDI port went away: {0}")]
    Disconnected(String),
    #[error("MIDI error: {0}")]
    Midi(String),
    #[error(transparent)]
    Op(#[from] OpError),
    #[error(
        "{0} gave no recognised answer to the device inquiry; check that the synth is \
         on and accepts SysEx on this port"
    )]
    Silent(String),
    #[error(
        "our own messages come back on {0} (MIDI thru or a loop), so replies cannot be \
         trusted; turn the loop off"
    )]
    Loopback(String),
    #[error(transparent)]
    Driver(#[from] DriverError),
}

#[cfg(feature = "midir")]
fn midi_err(e: impl std::fmt::Display) -> Error {
    Error::Midi(e.to_string())
}

/// A port's name as Oscyl spells it on every platform. Windows' class
/// driver names a device's later ports "MIDIIN2 (minilogue xd)" and
/// "MIDIOUT2 (minilogue xd)"; both become "minilogue xd _ MIDI 2", the
/// name such a port gets on Android (the mobile app's `midiPortName`), so
/// the input and output pair up and the drivers' hints match.
pub fn port_label(name: &str) -> String {
    let numbered = name
        .strip_prefix("MIDIIN")
        .or_else(|| name.strip_prefix("MIDIOUT"))
        .and_then(|rest| rest.split_once(" ("))
        .and_then(|(n, rest)| Some((n, rest.strip_suffix(')')?)));
    match numbered {
        Some((n, product)) if !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()) => {
            format!("{product} _ MIDI {n}")
        }
        _ => name.to_string(),
    }
}

#[cfg(feature = "midir")]
/// Names of all MIDI input and output ports, for diagnostics.
pub fn list_ports() -> Result<Vec<String>, Error> {
    let input = MidiInput::new(CLIENT).map_err(midi_err)?;
    let output = MidiOutput::new(CLIENT).map_err(midi_err)?;
    let mut names = Vec::new();
    for p in input.ports() {
        names.push(format!(
            "in:  {}",
            port_label(&input.port_name(&p).map_err(midi_err)?)
        ));
    }
    for p in output.ports() {
        names.push(format!(
            "out: {}",
            port_label(&output.port_name(&p).map_err(midi_err)?)
        ));
    }
    Ok(names)
}

#[cfg(feature = "midir")]
/// The single index whose name `matches`; `pattern` names it in errors.
fn select(names: &[String], pattern: &str, matches: &dyn Fn(&str) -> bool) -> Result<usize, Error> {
    let mut hits = names.iter().enumerate().filter(|(_, n)| matches(n));
    match (hits.next(), hits.next()) {
        (Some((i, _)), None) => Ok(i),
        (None, _) => Err(Error::NotFound),
        (Some(_), Some(_)) => Err(Error::Ambiguous(pattern.to_string())),
    }
}

/// A bidirectional SysEx connection to a synth.
pub trait Link: Send {
    /// Send complete messages (one or more SysEx messages back to back).
    fn send(&mut self, bytes: &[u8]) -> Result<(), Error>;
    /// Next complete SysEx message, or `None` when none arrives by
    /// `deadline`.
    fn recv_until(&mut self, deadline: Instant) -> Result<Option<Vec<u8>>, Error>;
    /// Send `()` on `bell` after input arrives (the input first), so one
    /// thread can wait on several links. A link whose input arrives on its
    /// own (not [`Link::synchronous`]) must implement it.
    fn ring_on_input(&mut self, _bell: Sender<()>) {}
    /// True when input only ever arrives during a send (a simulator): all
    /// of it is there once the send returns, so a deadline passes at once
    /// instead of being waited for.
    fn synchronous(&self) -> bool {
        false
    }
}

#[cfg(feature = "midir")]
/// The real synth over midir (ALSA on Linux).
pub struct MidirLink {
    out: Out,
    _in: MidiInputConnection<()>,
    inbox: Inbox,
}

#[cfg(feature = "midir")]
impl MidirLink {
    /// Connect to the input and output named exactly `name`.
    pub fn open_exact(name: &str) -> Result<MidirLink, Error> {
        match MidirLink::open_matching(name, &|n| n == name) {
            Ok((link, _)) => Ok(link),
            Err(Error::NotFound) => Err(Error::Disconnected(name.into())),
            Err(e) => Err(e),
        }
    }

    fn open_matching(
        pattern: &str,
        matches: &dyn Fn(&str) -> bool,
    ) -> Result<(MidirLink, String), Error> {
        let mut input = MidiInput::new(CLIENT).map_err(midi_err)?;
        input.ignore(Ignore::None);
        let output = MidiOutput::new(CLIENT).map_err(midi_err)?;
        let (in_ports, out_ports) = (input.ports(), output.ports());
        let names = |f: &dyn Fn(usize) -> String, n: usize| (0..n).map(f).collect::<Vec<_>>();
        let in_names = names(
            &|i| port_label(&input.port_name(&in_ports[i]).unwrap_or_default()),
            in_ports.len(),
        );
        let out_names = names(
            &|i| port_label(&output.port_name(&out_ports[i]).unwrap_or_default()),
            out_ports.len(),
        );
        let (i, o) = (
            select(&in_names, pattern, matches)?,
            select(&out_names, pattern, matches)?,
        );

        let (inbox, feed) = inbox();
        let conn_in = input
            .connect(
                &in_ports[i],
                "in",
                move |_, bytes, _| {
                    // Clock and active sensing never reach a session: they
                    // would wake the reader 48 times a second for nothing.
                    if bytes.iter().all(|&b| b >= 0xF8) {
                        return;
                    }
                    trace("<-", bytes);
                    feed.push(bytes)
                },
                (),
            )
            .map_err(midi_err)?;
        let out = match Out::raw(&out_ports[o].id()) {
            Some(raw) => raw,
            None => Out::Seq(output.connect(&out_ports[o], "out").map_err(midi_err)?),
        };
        let link = MidirLink {
            out,
            _in: conn_in,
            inbox,
        };
        Ok((link, out_names[o].clone()))
    }
}

/// Where a link's output goes.
#[cfg(feature = "midir")]
enum Out {
    /// midir's connection (the ALSA sequencer on Linux).
    Seq(MidiOutputConnection),
    /// The port's ALSA rawmidi device. The sequencer's bridge to a
    /// hardware port drops any message bigger than its output buffer
    /// (4 KiB; the kernel logs "seq_midi: MIDI output buffer overrun"),
    /// and a unit upload is bigger. Writing here waits for room (through
    /// poll; the device is opened non-blocking, since a blocking open
    /// would wait for whoever holds the port).
    #[cfg(target_os = "linux")]
    Raw(alsa::rawmidi::Rawmidi),
}

#[cfg(feature = "midir")]
impl Out {
    /// The rawmidi device behind sequencer port `id` (`client:port`), when
    /// it is a hardware port no one else is writing to.
    #[cfg(target_os = "linux")]
    fn raw(id: &str) -> Option<Out> {
        let (client, port) = id.split_once(':')?;
        let (client, port): (i32, i32) = (client.parse().ok()?, port.parse().ok()?);
        let seq = alsa::seq::Seq::open(None, None, false).ok()?;
        let card = seq.get_any_client_info(client).ok()?.get_card().ok()?;
        // The sequencer numbers the first device's subdevices first, so
        // port N is subdevice N there, or not there at all (the open
        // fails and the sequencer is used).
        let name = format!("hw:{card},0,{port}");
        alsa::rawmidi::Rawmidi::new(&name, alsa::Direction::Playback, true)
            .ok()
            .map(Out::Raw)
    }

    #[cfg(not(target_os = "linux"))]
    fn raw(_: &str) -> Option<Out> {
        None
    }

    fn send(&mut self, msg: &[u8]) -> Result<(), Error> {
        let lost = |e: &dyn std::fmt::Display| Error::Disconnected(e.to_string());
        match self {
            Out::Seq(c) => c.send(msg).map_err(|e| lost(&e)),
            #[cfg(target_os = "linux")]
            Out::Raw(r) => {
                use alsa::poll::Descriptors;
                let mut rest = msg;
                while !rest.is_empty() {
                    match std::io::Write::write(&mut r.io(), rest) {
                        Ok(n) => rest = &rest[n..],
                        // rawmidi's write returns -EAGAIN when full.
                        Err(e) if e.raw_os_error() == Some(-11) => {
                            // Full: wait until the synth has taken some.
                            // A stall this long is a synth gone quiet.
                            let mut fds = r.get().map_err(|e| lost(&e))?;
                            if alsa::poll::poll(&mut fds, 5000).map_err(|e| lost(&e))? == 0 {
                                return Err(lost(&"the synth stopped taking data"));
                            }
                        }
                        Err(e) => return Err(lost(&e)),
                    }
                }
                Ok(())
            }
        }
    }
}

#[cfg(feature = "midir")]
impl Link for MidirLink {
    /// midir 0.11.1's ALSA backend encodes only the first MIDI event of
    /// each `send`, so each SysEx message of `bytes` goes out on its own.
    fn send(&mut self, bytes: &[u8]) -> Result<(), Error> {
        for chunk in split_messages(bytes) {
            trace("->", chunk);
            self.out.send(chunk)?;
        }
        Ok(())
    }

    fn recv_until(&mut self, deadline: Instant) -> Result<Option<Vec<u8>>, Error> {
        self.inbox.recv_until(deadline)
    }

    fn ring_on_input(&mut self, bell: Sender<()>) {
        self.inbox.ring_on_input(bell);
    }
}

/// `OSCYL_MIDI_TRACE=1`: every message sent or received (not clock or
/// active sensing) on stderr, with milliseconds since the first one.
#[cfg(feature = "midir")]
fn trace(dir: &str, bytes: &[u8]) {
    use std::sync::LazyLock;
    static START: LazyLock<Option<Instant>> =
        LazyLock::new(|| std::env::var_os("OSCYL_MIDI_TRACE").map(|_| Instant::now()));
    let Some(start) = *START else { return };
    let head: Vec<String> = bytes.iter().take(12).map(|b| format!("{b:02X}")).collect();
    eprintln!(
        "{:>9.3} {dir} {} ({} bytes)",
        start.elapsed().as_secs_f64(),
        head.join(" "),
        bytes.len()
    );
}

type Bell = Arc<Mutex<Option<Sender<()>>>>;

/// The input half of a link whose bytes arrive on another thread: chunks
/// queued by its [`Feed`], framed into SysEx messages on the reader side.
struct Inbox {
    rx: Receiver<Vec<u8>>,
    /// SysEx and channel messages (a panel's program change).
    parser: parse::Parser,
    pending: VecDeque<Vec<u8>>,
    bell: Bell,
}

/// Where an [`Inbox`]'s input is pushed; dropping it closes the input.
struct Feed {
    tx: Sender<Vec<u8>>,
    bell: Bell,
}

fn inbox() -> (Inbox, Feed) {
    let (tx, rx) = channel();
    let bell = Bell::default();
    let feed = Feed {
        tx,
        bell: bell.clone(),
    };
    let inbox = Inbox {
        rx,
        parser: parse::Parser::default(),
        pending: VecDeque::new(),
        bell,
    };
    (inbox, feed)
}

impl Feed {
    /// Queue `bytes`, then ring the bell.
    fn push(&self, bytes: &[u8]) {
        drop(self.tx.send(bytes.to_vec()));
        ring(&self.bell);
    }
}

/// Gone (closed, or dropped by the platform): the input closes, then the
/// bell rings so a waiting reader sees it.
impl Drop for Feed {
    fn drop(&mut self) {
        drop(std::mem::replace(&mut self.tx, channel().0));
        ring(&self.bell);
    }
}

fn ring(bell: &Bell) {
    if let Some(bell) = bell.lock().ok().as_deref().and_then(Option::as_ref) {
        let _ = bell.send(());
    }
}

impl Inbox {
    fn recv_until(&mut self, deadline: Instant) -> Result<Option<Vec<u8>>, Error> {
        loop {
            if let Some(m) = self.pending.pop_front() {
                return Ok(Some(m));
            }
            let wait = deadline.saturating_duration_since(Instant::now());
            match self.rx.recv_timeout(wait) {
                Ok(bytes) => self.pending.extend(self.parser.push(&bytes)),
                Err(RecvTimeoutError::Timeout) => return Ok(None),
                Err(RecvTimeoutError::Disconnected) => {
                    return Err(Error::Disconnected("input closed".into()));
                }
            }
        }
    }

    fn ring_on_input(&mut self, bell: Sender<()>) {
        if let Ok(mut b) = self.bell.lock() {
            *b = Some(bell);
        }
    }
}

#[cfg(feature = "midir")]
/// `bytes` cut into complete SysEx messages; bytes outside a message (CC or
/// NRPN runs) pass through as one chunk each.
fn split_messages(bytes: &[u8]) -> Vec<&[u8]> {
    let (mut out, mut start) = (vec![], 0);
    for (i, &b) in bytes.iter().enumerate() {
        match b {
            0xF0 if i > start => {
                out.push(&bytes[start..i]);
                start = i;
            }
            0xF7 => {
                out.push(&bytes[start..=i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    if start < bytes.len() {
        out.push(&bytes[start..]);
    }
    out
}

/// The simulated synth. Replies are computed when a message is sent and
/// queued; receiving never waits.
pub struct SimLink {
    sim: Box<dyn Simulator>,
    framer: Framer,
    replies: VecDeque<Vec<u8>>,
}

impl SimLink {
    pub fn new(sim: SimDevice) -> SimLink {
        SimLink::from_sim(Box::new(XdSimulator(sim)))
    }

    /// Any driver's simulator.
    pub fn from_sim(sim: Box<dyn Simulator>) -> SimLink {
        SimLink {
            sim,
            framer: Framer::new(),
            replies: VecDeque::new(),
        }
    }
}

impl Link for SimLink {
    fn synchronous(&self) -> bool {
        true
    }

    fn send(&mut self, bytes: &[u8]) -> Result<(), Error> {
        for msg in self.framer.push(bytes) {
            self.replies.extend(self.sim.handle(&msg));
        }
        Ok(())
    }

    /// A queued reply, or `None` at once: nothing more will ever arrive
    /// for what was sent.
    fn recv_until(&mut self, _deadline: Instant) -> Result<Option<Vec<u8>>, Error> {
        Ok(self.replies.pop_front())
    }
}

#[cfg(all(test, feature = "midir"))]
mod tests {
    #[test]
    fn windows_numbered_ports_pair_up_under_the_android_name() {
        use super::port_label;
        assert_eq!(
            port_label("MIDIIN2 (minilogue xd)"),
            "minilogue xd _ MIDI 2"
        );
        assert_eq!(
            port_label("MIDIOUT2 (minilogue xd)"),
            "minilogue xd _ MIDI 2"
        );
        for kept in [
            "minilogue xd",
            "minilogue xd:minilogue xd _ SOUND 24:1",
            "MIDIIN (x)",
            "MIDIINx (y)",
        ] {
            assert_eq!(port_label(kept), kept);
        }
    }

    use super::*;

    fn names(v: &[&str]) -> Vec<String> {
        v.iter().map(std::string::ToString::to_string).collect()
    }

    #[test]
    fn selects_the_sound_port() {
        let n = names(&[
            "Midi Through:Midi Through Port-0 14:0",
            "minilogue xd:minilogue xd _ MIDI OUT 28:0",
            "minilogue xd:minilogue xd _ SOUND 28:1",
        ]);
        fn has(p: &str) -> impl Fn(&str) -> bool + use<'_> {
            move |n| n.contains(p)
        }
        let sound = "minilogue xd _ SOUND";
        assert_eq!(select(&n, sound, &has(sound)).unwrap(), 2);
        assert!(matches!(
            select(&n, "nothing", &has("nothing")),
            Err(Error::NotFound)
        ));
        assert!(matches!(
            select(&n, "minilogue xd", &has("minilogue xd")),
            Err(Error::Ambiguous(_))
        ));
    }

    #[test]
    fn split_sends_each_message() {
        let (a, b) = (
            [0xF0, 0x01, 0x2F, 0x05, 0, 1, 0xF7],
            [0xF0, 0x01, 0x2F, 0x06, 0xF7],
        );
        let both = [&a[..], &b[..]].concat();
        assert_eq!(split_messages(&both), vec![&a[..], &b[..]]);
        assert_eq!(split_messages(&a), vec![&a[..]]);
        assert_eq!(
            split_messages(&[0xB0, 0x63, 0x01]),
            vec![&[0xB0, 0x63, 0x01][..]]
        );
    }
}
