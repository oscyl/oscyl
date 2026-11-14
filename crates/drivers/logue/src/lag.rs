//! A logue session behind the generic [`DriverSession`], with the read-lag
//! check and recovery the xd needs on hardware and the prologue's firmware
//! has too.

use std::collections::{HashMap, VecDeque};
use std::time::Instant;

use oscyl_driver::engine::{Engine, OpError};
use oscyl_driver::{DriverError, DriverSession, SessionEvent, SessionOp};

use crate::Model;
use crate::msg::PROGRAM_COUNT;
use crate::session::{Event, Op, Proto};
use crate::store::plan_commit;

/// Program pairs read `a, b, b, a` to detect the reply lag (see
/// [`verdict`]). A pair of identical programs cannot tell, so several are
/// tried; the last spans the bank, for a synth whose upper programs are
/// all initialised.
pub const LAG_PAIRS: [(u16, u16); 5] = [(0, 1), (100, 101), (200, 201), (300, 301), (0, 499)];

/// The reads that probe pair `(a, b)`.
pub fn probe_reads((a, b): (u16, u16)) -> Vec<u16> {
    vec![a, b, b, a]
}

/// What the replies to [`probe_reads`] say.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// Every reply carried its own program, and a and b differ.
    Healthy,
    /// Some reply carried another request's data.
    Lagging,
    /// a and b are identical and so are all replies: no way to tell.
    Undecided,
}

/// Classify the replies to `a, b, b, a`. A healthy synth answers A, B, B,
/// A; a lagging one answers X, A, B, B (one behind, X stale). When a and b
/// are identical a lagging synth answers X, A, A, A, which `a, b, b` alone
/// took for healthy: the second read of a tells it apart unless X is A too.
pub fn verdict<P: PartialEq>(got: &[P]) -> Verdict {
    match got {
        [r0, r1, r2, r3] if r1 == r2 && r0 == r3 => {
            if r0 != r1 {
                Verdict::Healthy
            } else {
                Verdict::Undecided
            }
        }
        _ => Verdict::Lagging,
    }
}
/// Program read and written back to clear the lag (never 499, see `store`).
const RECOVERY_PROGRAM: u16 = 0;

/// Where the reply-lag check stands.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Health<P> {
    Unchecked,
    /// Reading `LAG_PAIRS[pair]`; `again` after a recovery.
    Probe {
        pair: usize,
        again: bool,
    },
    /// Reading the recovery program; one entry per read so far.
    Recover(Vec<Option<P>>),
    RecoverWrite,
    Ready,
    /// The lag could not be cleared over MIDI.
    Broken(OpError),
}

fn lag_error<M: Model>() -> OpError {
    OpError::BadData {
        what: "reply-lag check",
        error: format!(
            "the synth keeps answering with the previous request's data and could not \
             be reset over MIDI; power-cycle the {}",
            M::NAME
        ),
    }
}

type Writes<P> = Vec<(u16, P)>;
type Held<P> = VecDeque<(Op<P>, Writes<P>)>;

/// The logue engine speaking [`SessionOp`]. Writes always go through
/// [`plan_commit`]; what the synth is known to hold (programs read, and
/// writes that completed) feeds its `known`. A slot being written is
/// unknown until its write reports done.
///
/// Before the first op runs, the firmware's "one reply behind" state (a
/// read raced a deferred flash save)
/// is detected and cleared: read programs a, b, b, a and take the
/// [`verdict`]. When no pair tells (every pair identical), ops run but
/// reads stay untrusted: they reach events, not `known`, until a write
/// completes, so a write that needs a neighbour's contents is refused with
/// that reason. A reply that only looks valid
/// (the prologue can ship a real program of the wrong number) is caught the
/// same way, since the check compares replies, not markers. Recovery reads
/// the recovery program until two reads agree, writes it back unchanged and
/// checks again. Ops wait until then; the check's reads never reach events
/// or `known`.
pub struct LogueSession<M: Model> {
    session: Engine<Proto<M>>,
    known: HashMap<u16, M::Program>,
    /// Per op submitted to `session`, in order: the programs it writes.
    writes: VecDeque<Writes<M::Program>>,
    health: Health<M::Program>,
    /// Ops waiting for the check, with what they write.
    held: Held<M::Program>,
    /// Programs the check's current read returned.
    probe: Vec<M::Program>,
    /// Whether reads can be taken at their word: a probe pair proved the
    /// synth healthy, or a write completed (a 0x4C clears any lag). Until
    /// then reads still reach events but not `known`.
    trusted: bool,
    events: VecDeque<SessionEvent>,
}

impl<M: Model> LogueSession<M> {
    pub fn new(channel: u8) -> LogueSession<M> {
        LogueSession {
            session: Engine::new(Proto::new(channel)),
            known: HashMap::new(),
            writes: VecDeque::new(),
            health: Health::Unchecked,
            held: VecDeque::new(),
            probe: vec![],
            trusted: false,
            events: VecDeque::new(),
        }
    }

    fn check(&mut self, health: Health<M::Program>, op: Op<M::Program>) {
        self.health = health;
        self.probe.clear();
        self.session.submit(op);
    }

    fn probe(&mut self, pair: usize, again: bool) {
        self.check(
            Health::Probe { pair, again },
            Op::ReadPrograms(probe_reads(LAG_PAIRS[pair])),
        );
    }

    fn ready(&mut self) {
        self.health = Health::Ready;
        for (op, written) in std::mem::take(&mut self.held) {
            self.session.submit(op);
            self.writes.push_back(written);
        }
    }

    /// Fail the waiting ops. A broken synth stays broken; otherwise the
    /// next op checks again.
    fn fail_held(&mut self, e: OpError, broken: bool) {
        for _ in self.held.drain(..) {
            self.events.push_back(SessionEvent::Failed(e.clone()));
        }
        self.health = if broken {
            Health::Broken(e)
        } else {
            Health::Unchecked
        };
    }

    /// The check's current read or write ended.
    fn advance(&mut self, result: Result<(), OpError>) {
        let got = std::mem::take(&mut self.probe);
        match (self.health.clone(), result) {
            (Health::Probe { pair, again }, Ok(())) => match verdict(&got) {
                Verdict::Lagging if again => self.fail_held(lag_error::<M>(), true),
                Verdict::Lagging => self.check(
                    Health::Recover(vec![]),
                    Op::ReadPrograms(vec![RECOVERY_PROGRAM]),
                ),
                Verdict::Healthy => {
                    self.trusted = true;
                    self.ready();
                }
                // No pair told: go on, but reads stay untrusted.
                Verdict::Undecided if pair + 1 == LAG_PAIRS.len() => self.ready(),
                Verdict::Undecided => self.probe(pair + 1, again),
            },
            // A garbled read is expected while lagging.
            (Health::Recover(mut reads), result) => {
                reads.push(result.ok().and(got.into_iter().next()));
                if reads.len() < 3 {
                    self.check(
                        Health::Recover(reads),
                        Op::ReadPrograms(vec![RECOVERY_PROGRAM]),
                    );
                    return;
                }
                // Each reply carries the previous request's data, so the
                // last two reads agreeing give the true contents.
                let current = reads
                    .windows(2)
                    .rev()
                    .find(|w| w[0].is_some() && w[0] == w[1])
                    .and_then(|w| w[1].clone());
                match current {
                    Some(p) => {
                        let plan = plan_commit(M::STORE, [(RECOVERY_PROGRAM, p)], |_| None)
                            .expect("recovery program is not the last");
                        self.check(Health::RecoverWrite, Op::WritePrograms(plan));
                    }
                    None => self.fail_held(lag_error::<M>(), true),
                }
            }
            (Health::RecoverWrite, Ok(())) => {
                // The write consumed the stale token.
                self.trusted = true;
                self.probe(0, true)
            }
            (_, Err(e)) => self.fail_held(e, false),
            (h, Ok(())) => unreachable!("no check runs in {h:?}"),
        }
    }

    fn checking(&self) -> bool {
        matches!(
            self.health,
            Health::Probe { .. } | Health::Recover(_) | Health::RecoverWrite
        )
    }
}

fn program<M: Model>(bytes: &[u8]) -> Result<M::Program, DriverError> {
    M::parse(bytes).map_err(|e| DriverError(e.to_string()))
}

fn in_range(n: u16) -> Result<u16, DriverError> {
    if n < PROGRAM_COUNT {
        Ok(n)
    } else {
        Err(DriverError(format!(
            "program number {n} is out of range (0-{})",
            PROGRAM_COUNT - 1
        )))
    }
}

impl<M: Model> DriverSession for LogueSession<M> {
    fn submit(&mut self, op: SessionOp) -> Result<(), DriverError> {
        let mut written = vec![];
        let op = match op {
            SessionOp::ReadCurrent => Op::ReadCurrent,
            SessionOp::ReadPrograms(ns) => {
                Op::ReadPrograms(ns.into_iter().map(in_range).collect::<Result<_, _>>()?)
            }
            SessionOp::WritePrograms(batch) => {
                // Validated here: the prologue's firmware stores whatever a
                // 0x4C carries.
                let batch = batch
                    .into_iter()
                    .map(|(n, b)| Ok((in_range(n)?, program::<M>(&b)?)))
                    .collect::<Result<Vec<_>, DriverError>>()?;
                let plan = plan_commit(M::STORE, batch.iter().cloned(), |n| {
                    self.known.get(&n).cloned()
                })
                .map_err(|e| {
                    let why = if self.trusted {
                        String::new()
                    } else {
                        "; programs read so far do not count, because the reply-lag check \
                         found no two programs that differ and cannot vouch for reads until \
                         a write completes"
                            .into()
                    };
                    DriverError(format!("{e}{why}"))
                })?;
                for (n, _) in &batch {
                    self.known.remove(n);
                }
                written = batch;
                Op::WritePrograms(plan)
            }
            SessionOp::Audition(b) => Op::Audition(Box::new(program::<M>(&b)?)),
        };
        match &self.health {
            Health::Ready => {
                self.session.submit(op);
                self.writes.push_back(written);
            }
            Health::Broken(e) => self.events.push_back(SessionEvent::Failed(e.clone())),
            Health::Unchecked => {
                self.held.push_back((op, written));
                self.probe(0, false);
            }
            _ => self.held.push_back((op, written)),
        }
        Ok(())
    }

    fn is_idle(&self) -> bool {
        self.session.is_idle() && self.held.is_empty()
    }

    fn poll_transmit(&mut self, now: Instant) -> Option<Vec<u8>> {
        self.session.poll_transmit(now)
    }

    fn next_deadline(&self) -> Option<Instant> {
        self.session.next_deadline()
    }

    fn handle_timeout(&mut self, now: Instant) {
        self.session.handle_timeout(now)
    }

    fn handle_input(&mut self, bytes: &[u8]) {
        self.session.handle_input(bytes)
    }

    fn poll_event(&mut self) -> Option<SessionEvent> {
        loop {
            if let Some(e) = self.events.pop_front() {
                return Some(e);
            }
            let event = self.session.poll_event()?;
            if self.checking() {
                match event {
                    Event::Program(_, p) => self.probe.push(p),
                    Event::OpDone => self.advance(Ok(())),
                    Event::OpFailed(e) => self.advance(Err(e)),
                    _ => {}
                }
                continue;
            }
            return Some(match event {
                Event::Current(p) => SessionEvent::Current(M::bytes(&p).to_vec()),
                Event::Program(n, p) => {
                    let bytes = M::bytes(&p).to_vec();
                    if self.trusted {
                        self.known.insert(n, p);
                    }
                    SessionEvent::Program(n, bytes)
                }
                Event::Progress { done, total } => SessionEvent::Progress { done, total },
                Event::OpDone => {
                    let written = self.writes.pop_front().unwrap_or_default();
                    // A completed write consumed any stale "done" token.
                    self.trusted |= !written.is_empty();
                    self.known.extend(written);
                    SessionEvent::Done
                }
                Event::OpFailed(e) => {
                    self.writes.pop_front();
                    SessionEvent::Failed(e)
                }
                // Only produced by ops never submitted here.
                _ => continue,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::msg::{Dump, Request, encode_dump, encode_request};
    use crate::sim::LogueSimulator;
    use crate::testing::{Test, program};
    use oscyl_driver::Simulator;

    fn pump(s: &mut LogueSession<Test>, sim: &mut LogueSimulator<Test>) -> Vec<SessionEvent> {
        let mut now = Instant::now();
        let mut out = vec![];
        loop {
            if let Some(m) = s.poll_transmit(now) {
                for r in sim.handle(&m) {
                    s.handle_input(&r);
                }
            } else if let Some(d) = s.next_deadline() {
                now = d;
                s.handle_timeout(now);
            } else if s.is_idle() {
                break;
            }
            out.extend(std::iter::from_fn(|| s.poll_event()));
        }
        out.extend(std::iter::from_fn(|| s.poll_event()));
        out
    }

    #[test]
    fn lag_with_valid_looking_reply_is_detected_and_cleared() {
        let mut sim = LogueSimulator(crate::sim::Sim::<Test>::with_library(&[
            program(10),
            program(11),
        ]));
        let f = Test::FAMILY;
        // A write leaves group 0 dirty; a racing read makes the synth lag.
        sim.handle(&encode_dump(&f, 0, &Dump::Program(5, program(5))));
        sim.inject("race_deferred_save");
        let first = sim.handle(&encode_request(&f, 0, &Request::Program(1)));
        let Ok(crate::msg::Reply::Dump(Dump::Program(1, shipped))) =
            crate::msg::decode(&f, &first[0])
        else {
            panic!("no reply");
        };
        assert_eq!(shipped, program(10), "a valid program, of the wrong number");
        assert!(!sim.is_settled());

        let mut s = LogueSession::<Test>::new(0);
        s.submit(SessionOp::ReadPrograms(vec![0, 1])).unwrap();
        let ev = pump(&mut s, &mut sim);
        assert!(ev.contains(&SessionEvent::Program(0, program(10))));
        assert!(ev.contains(&SessionEvent::Program(1, program(11))));
        assert_eq!(ev.last(), Some(&SessionEvent::Done));
        assert!(sim.is_settled());
    }

    #[test]
    fn single_edit_commits_with_13_writes() {
        let (mut s, mut sim) = (LogueSession::<Test>::new(0), LogueSimulator::default());
        s.submit(SessionOp::WritePrograms(vec![(300, program(7))]))
            .unwrap();
        let ev = pump(&mut s, &mut sim);
        let writes = ev
            .iter()
            .filter_map(|e| match e {
                SessionEvent::Progress { total, .. } => Some(*total),
                _ => None,
            })
            .next_back();
        assert_eq!(writes, Some(13));
        assert_eq!(sim.program(300), Some(program(7)));
        assert!(sim.is_settled());
    }

    fn programs(ev: &[SessionEvent]) -> Vec<(u16, Vec<u8>)> {
        ev.iter()
            .filter_map(|e| match e {
                SessionEvent::Program(n, p) => Some((*n, p.clone())),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn identical_probe_pair_with_lag_is_not_healthy() {
        let a = program(1);
        let mut sim = LogueSimulator(crate::sim::Sim::<Test>::with_library(&[
            a.clone(),
            a,
            program(2),
            program(3),
        ]));
        let f = Test::FAMILY;
        // Lag with a stale reply (program 6, init) unlike programs 0 and 1,
        // which are identical: a, b, b alone reads X, A, A and looks healthy.
        sim.handle(&encode_dump(&f, 0, &Dump::Program(5, program(5))));
        sim.inject("race_deferred_save");
        sim.handle(&encode_request(&f, 0, &Request::Program(6)));
        assert!(sim.0.is_lagging());
        let mut s = LogueSession::<Test>::new(0);
        s.submit(SessionOp::ReadPrograms(vec![2, 3])).unwrap();
        let ev = pump(&mut s, &mut sim);
        assert_eq!(programs(&ev), [(2, program(2)), (3, program(3))]);
        assert_eq!(ev.last(), Some(&SessionEvent::Done));
        assert!(sim.is_settled());
    }

    #[test]
    fn undecided_check_refuses_writes_that_need_read_data() {
        // Every program identical: no probe pair can tell a lag.
        let (mut s, mut sim) = (LogueSession::<Test>::new(0), LogueSimulator::default());
        s.submit(SessionOp::ReadPrograms(vec![495])).unwrap();
        pump(&mut s, &mut sim);
        let e = s
            .submit(SessionOp::WritePrograms(vec![(499, program(9))]))
            .unwrap_err();
        assert!(e.0.contains("reply-lag check"), "{e}");
        // A completed write clears any lag, so later reads are trusted.
        s.submit(SessionOp::WritePrograms(vec![(3, program(3))]))
            .unwrap();
        pump(&mut s, &mut sim);
        s.submit(SessionOp::ReadPrograms(vec![495])).unwrap();
        pump(&mut s, &mut sim);
        s.submit(SessionOp::WritePrograms(vec![(499, program(9))]))
            .unwrap();
        assert_eq!(pump(&mut s, &mut sim).last(), Some(&SessionEvent::Done));
        assert_eq!(sim.program(499), Some(program(9)));
    }

    #[test]
    fn verdicts() {
        use Verdict::*;
        let v = |g: [u8; 4]| verdict(&g);
        assert_eq!(v([1, 2, 2, 1]), Healthy);
        assert_eq!(v([9, 1, 2, 2]), Lagging);
        assert_eq!(v([9, 1, 1, 1]), Lagging);
        assert_eq!(v([1, 1, 1, 1]), Undecided);
    }

    #[test]
    fn invalid_program_is_refused_before_sending() {
        let mut s = LogueSession::<Test>::new(0);
        let bad = vec![0u8; crate::testing::LEN];
        assert!(s.submit(SessionOp::WritePrograms(vec![(3, bad)])).is_err());
        assert!(s.poll_transmit(Instant::now()).is_none());
    }
}
