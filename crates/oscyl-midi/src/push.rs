//! A link whose bytes the platform moves (Android `MidiManager`, CoreMIDI):
//! output goes to a closure, input is pushed in through an [`InputSender`].

use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use crate::{Error, Feed, Inbox, Link, inbox};

/// Where a [`PushLink`]'s output goes.
pub type Output = Box<dyn FnMut(&[u8]) -> Result<(), Error> + Send>;

/// Output through a closure; input pushed by the platform through the
/// paired [`InputSender`] and framed into SysEx messages here.
pub struct PushLink {
    send: Output,
    inbox: Inbox,
}

/// The input end of a [`PushLink`]: the platform pushes received bytes
/// (in any fragments) and closes it when the port goes away.
#[derive(Clone)]
pub struct InputSender(Arc<Mutex<Option<Feed>>>);

impl PushLink {
    /// A link that sends through `send` (one call per [`Link::send`],
    /// bytes unchanged), with the sender its input is pushed through.
    pub fn new(send: Output) -> (PushLink, InputSender) {
        let (inbox, feed) = inbox();
        (
            PushLink { send, inbox },
            InputSender(Arc::new(Mutex::new(Some(feed)))),
        )
    }
}

impl Link for PushLink {
    fn send(&mut self, bytes: &[u8]) -> Result<(), Error> {
        (self.send)(bytes)
    }

    fn recv_until(&mut self, deadline: Instant) -> Result<Option<Vec<u8>>, Error> {
        self.inbox.recv_until(deadline)
    }

    fn ring_on_input(&mut self, bell: Sender<()>) {
        self.inbox.ring_on_input(bell);
    }
}

impl InputSender {
    /// Queue received bytes and ring the link's bell. Ignored once closed.
    pub fn push(&self, bytes: &[u8]) {
        if let Some(feed) = self.0.lock().unwrap_or_else(|e| e.into_inner()).as_ref() {
            feed.push(bytes);
        }
    }

    /// The port is gone: once queued input is read, the link reports
    /// [`Error::Disconnected`]. The bell rings so a waiting worker sees it.
    /// Dropping the last sender does the same.
    pub fn close(&self) {
        let feed = self.0.lock().unwrap_or_else(|e| e.into_inner()).take();
        drop(feed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc::channel;
    use std::time::Duration;

    fn link() -> (PushLink, InputSender) {
        PushLink::new(Box::new(|_| Ok(())))
    }

    fn soon() -> Instant {
        Instant::now() + Duration::from_secs(1)
    }

    #[test]
    fn fragments_are_framed_in_order() {
        let (mut link, input) = link();
        input.push(&[0xF0, 0x42, 0x30]);
        input.push(&[0x00, 0x01, 0xF7, 0xF0, 0x7E]);
        input.push(&[0x01, 0xF7]);
        assert_eq!(
            link.recv_until(soon()).unwrap(),
            Some(vec![0xF0, 0x42, 0x30, 0x00, 0x01, 0xF7])
        );
        assert_eq!(
            link.recv_until(soon()).unwrap(),
            Some(vec![0xF0, 0x7E, 0x01, 0xF7])
        );
        assert_eq!(link.recv_until(Instant::now()).unwrap(), None);
    }

    #[test]
    fn close_disconnects_after_queued_input() {
        let (mut link, input) = link();
        input.push(&[0xF0, 0x01, 0xF7]);
        input.clone().close();
        assert_eq!(
            link.recv_until(soon()).unwrap(),
            Some(vec![0xF0, 0x01, 0xF7])
        );
        assert!(matches!(
            link.recv_until(soon()),
            Err(Error::Disconnected(_))
        ));
        // Pushing after close is ignored.
        input.push(&[0xF0, 0x02, 0xF7]);
        assert!(matches!(
            link.recv_until(soon()),
            Err(Error::Disconnected(_))
        ));
    }

    #[test]
    fn push_rings_the_bell() {
        let (mut link, input) = link();
        let (tx, rx) = channel();
        link.ring_on_input(tx);
        let t = std::thread::spawn(move || input.push(&[0xF0, 0x01, 0xF7]));
        rx.recv().unwrap();
        t.join().unwrap();
        assert_eq!(
            link.recv_until(soon()).unwrap(),
            Some(vec![0xF0, 0x01, 0xF7])
        );
    }

    #[test]
    fn send_goes_to_closure() {
        let sent = Arc::new(Mutex::new(Vec::<Vec<u8>>::new()));
        let log = sent.clone();
        let (mut link, _input) = PushLink::new(Box::new(move |b| {
            log.lock().unwrap().push(b.to_vec());
            Ok(())
        }));
        link.send(&[1, 2]).unwrap();
        assert_eq!(*sent.lock().unwrap(), vec![vec![1, 2]]);
    }
}
