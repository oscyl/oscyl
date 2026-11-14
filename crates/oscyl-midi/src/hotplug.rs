//! Notice synths being plugged in or unplugged, from ALSA sequencer
//! announcements (event-driven: the caller polls the file descriptors from
//! its own main loop and calls [`PortWatcher::drain_changes`] when they
//! are ready).

use std::ffi::CString;
use std::os::fd::RawFd;

use alsa::poll::Descriptors;
use alsa::seq::{Addr, EventType, PortCap, PortSubscribe, PortType, Seq};

use crate::{Candidate, Error, candidates};

/// A synth's port came or went.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Change {
    Appeared(Candidate),
    Gone(Candidate),
}

/// What changed from `old` to `new`: appearances first, in order.
fn diff(old: &[Candidate], new: &[Candidate]) -> Vec<Change> {
    let appeared = new.iter().filter(|c| !old.contains(c));
    let gone = old.iter().filter(|c| !new.contains(c));
    appeared
        .cloned()
        .map(Change::Appeared)
        .chain(gone.cloned().map(Change::Gone))
        .collect()
}

pub struct PortWatcher {
    seq: Seq,
    ports: Vec<Candidate>,
}

fn alsa_err(e: alsa::Error) -> Error {
    Error::Midi(e.to_string())
}

impl PortWatcher {
    pub fn new() -> Result<PortWatcher, Error> {
        let seq = Seq::open(None, Some(alsa::Direction::Capture), true).map_err(alsa_err)?;
        let name = CString::new("oscyl-watch").expect("no NUL");
        seq.set_client_name(&name).map_err(alsa_err)?;
        let port = seq
            .create_simple_port(
                &name,
                PortCap::WRITE | PortCap::SUBS_WRITE | PortCap::NO_EXPORT,
                PortType::MIDI_GENERIC | PortType::APPLICATION,
            )
            .map_err(alsa_err)?;
        let sub = PortSubscribe::empty().map_err(alsa_err)?;
        sub.set_sender(Addr::system_announce());
        sub.set_dest(Addr {
            client: seq.client_id().map_err(alsa_err)?,
            port,
        });
        seq.subscribe_port(&sub).map_err(alsa_err)?;
        let mut w = PortWatcher { seq, ports: vec![] };
        w.ports = w.scan();
        Ok(w)
    }

    /// Every synth port present right now.
    pub fn candidates(&self) -> &[Candidate] {
        &self.ports
    }

    /// File descriptors to watch for readability.
    pub fn fds(&self) -> Vec<RawFd> {
        let target = (&self.seq, Some(alsa::Direction::Capture));
        let mut fds = vec![libc_pollfd(); target.count()];
        match target.fill(&mut fds) {
            Ok(n) => fds[..n].iter().map(|p| p.fd).collect(),
            Err(_) => Vec::new(),
        }
    }

    /// Read pending announcements; returns each synth port that came or
    /// went.
    pub fn drain_changes(&mut self) -> Vec<Change> {
        let mut relevant = false;
        {
            let mut input = self.seq.input();
            while let Ok(ev) = input.event_input() {
                relevant |= matches!(
                    ev.get_type(),
                    EventType::PortStart
                        | EventType::PortExit
                        | EventType::ClientStart
                        | EventType::ClientExit
                );
            }
        }
        if !relevant {
            return vec![];
        }
        let now = self.scan();
        let changes = diff(&self.ports, &now);
        self.ports = now;
        changes
    }

    /// Synth ports, named as midir names them so [`crate::Device::open`]
    /// finds them. Enumerates through this watcher's own sequencer handle:
    /// opening other clients here would itself be announced and wake the
    /// watcher again, forever.
    fn scan(&self) -> Vec<Candidate> {
        let duplex = PortCap::READ | PortCap::WRITE;
        let mut names = vec![];
        for client in alsa::seq::ClientIter::new(&self.seq) {
            let Ok(cname) = client.get_name() else {
                continue;
            };
            for port in alsa::seq::PortIter::new(&self.seq, client.get_client()) {
                if !port.get_capability().contains(duplex) {
                    continue;
                }
                if let Ok(pname) = port.get_name() {
                    let (c, p) = (port.get_client(), port.get_port());
                    names.push(format!("{cname}:{pname} {c}:{p}"));
                }
            }
        }
        candidates(&names)
    }
}

fn libc_pollfd() -> alsa::poll::pollfd {
    alsa::poll::pollfd {
        fd: -1,
        events: 0,
        revents: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diff_reports_each_device() {
        let c = |ports: &[&str]| {
            crate::candidates(&ports.iter().map(ToString::to_string).collect::<Vec<_>>())
        };
        let (xd1, xd2, rev2) = (
            "minilogue xd:minilogue xd _ SOUND 28:1",
            "minilogue xd:minilogue xd _ SOUND 32:1",
            "Prophet Rev2:Prophet Rev2 MIDI 1 36:0",
        );
        assert_eq!(
            diff(&c(&[xd1, rev2]), &c(&[xd1, xd2])),
            [
                Change::Appeared(c(&[xd2])[0].clone()),
                Change::Gone(c(&[rev2])[0].clone())
            ]
        );
        assert!(diff(&c(&[xd1]), &c(&[xd1])).is_empty());
    }

    /// Needs an ALSA sequencer; skipped where none exists (e.g. CI).
    #[test]
    fn watcher_opens_and_starts_quiet() {
        let Ok(mut w) = PortWatcher::new() else {
            return;
        };
        assert!(!w.fds().is_empty());
        assert!(
            w.drain_changes().is_empty(),
            "no announcements without a port change"
        );
        // Checking presence must not itself cause announcements.
        let _ = w.scan();
        assert!(w.drain_changes().is_empty());
    }
}
