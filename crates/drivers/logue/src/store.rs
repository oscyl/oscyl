//! Committing program writes to flash deterministically.
//!
//! logue firmware keeps one 4 KB flash sector (a *group* of programs: 4 on
//! the xd, 12 on the prologue) in a RAM cache. A 0x4C write only updates
//! that cache; the flash save happens later, from the main loop, after
//! ~300 idle iterations. A 0x1C read that races that deferred save corrupts
//! the synth's reply bookkeeping until the next write.
//!
//! The firmware also saves synchronously, *before* acknowledging, in two
//! cases: a write into a different group, and a write past the group size
//! into the cached group (which is then dropped and the cache emptied).
//! [`plan_commit`] uses the second case: it pads every batch with
//! byte-identical copies of its last program until that write is the one
//! past the group size. The ACK of the final write therefore means "saved,
//! nothing pending", and reads are safe and coherent straight after it. No
//! timing is involved.

use std::collections::BTreeMap;

/// A model's flash store: programs per sector, and the last program, which
/// has its own firmware special case (see [`StoreModel::write`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Store {
    pub group_size: u16,
    pub last: u16,
}

impl Store {
    /// The first program of `n`'s group.
    fn group_start(self, n: u16) -> u16 {
        n / self.group_size * self.group_size
    }
}

/// What the firmware does with one 0x4C write.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Effect {
    /// Copied into the cache; a deferred save is pending.
    Cached,
    /// The previously cached group was saved first, then this group loaded.
    SavedPreviousAndCached,
    /// The cached group was saved and the cache emptied; this write's data
    /// was discarded. Nothing is pending.
    SavedAndDropped,
}

/// Host-side mirror of the synth's program cache (xd and prologue,
/// firmware 2.10).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreModel {
    store: Store,
    /// Cached group and the number of writes it has received.
    cached: Option<(u16, u16)>,
}

impl StoreModel {
    /// A store with nothing cached and nothing pending.
    pub fn clean(store: Store) -> StoreModel {
        StoreModel {
            store,
            cached: None,
        }
    }

    pub fn is_clean(&self) -> bool {
        self.cached.is_none()
    }

    /// The group in the cache, if any.
    pub fn cached_group(&self) -> Option<u16> {
        self.cached.map(|(g, _)| g)
    }

    /// Apply one program write.
    pub fn write(&mut self, program: u16) -> Effect {
        let Store { group_size, last } = self.store;
        let group = program / group_size;
        match self.cached {
            None => {
                self.cached = Some((group, 1));
                Effect::Cached
            }
            Some((g, n)) if g == group => match n + 1 {
                // Firmware special case: the last program as the
                // group-size-th write of its cache session saves, reloads and
                // caches again.
                n if n == group_size && program == last => {
                    self.cached = Some((group, 1));
                    Effect::SavedPreviousAndCached
                }
                n if n > group_size => {
                    self.cached = None;
                    Effect::SavedAndDropped
                }
                n => {
                    self.cached = Some((group, n));
                    Effect::Cached
                }
            },
            Some(_) => {
                self.cached = Some((group, 1));
                Effect::SavedPreviousAndCached
            }
        }
    }
}

/// A batch of program writes that ends with the synth's store saved and
/// clean. Only [`plan_commit`] can build one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitPlan<P> {
    writes: Vec<(u16, P)>,
    real: usize,
}

impl<P> CommitPlan<P> {
    /// Every write to send, in order (real writes, then padding).
    pub fn writes(&self) -> &[(u16, P)] {
        &self.writes
    }

    /// Number of writes that change data (the rest is commit padding).
    pub fn real(&self) -> usize {
        self.real
    }

    pub fn is_empty(&self) -> bool {
        self.writes.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PlanError {
    #[error("program number {0} is out of range (0-{1})")]
    OutOfRange(u16, u16),
    /// Only the last program is written, and the padding needs another
    /// program of its group (0-based `first..last`) whose contents are known.
    #[error(
        "writing only program {} needs the current contents of program {} to commit the \
         save; read one of them first",
        last + 1,
        neighbours(*first, *last)
    )]
    NeedsNeighbour { first: u16, last: u16 },
}

/// "497, 498 or 499": 0-based `first..last` numbered from 1.
fn neighbours(first: u16, last: u16) -> String {
    let n: Vec<String> = (first + 1..=last).map(|n| n.to_string()).collect();
    match n.split_last() {
        Some((tail, init)) if !init.is_empty() => format!("{} or {tail}", init.join(", ")),
        _ => n.concat(),
    }
}

/// Order and pad `batch` so it ends saved, assuming the store is clean when
/// the batch starts (true after any previous `CommitPlan`).
///
/// `known` returns the synth's current contents of a program, if the caller
/// has read it; it is only needed when the batch is the last program alone.
pub fn plan_commit<P: Clone>(
    store: Store,
    batch: impl IntoIterator<Item = (u16, P)>,
    known: impl Fn(u16) -> Option<P>,
) -> Result<CommitPlan<P>, PlanError> {
    let last = store.last;
    // One write per program (a write past the group size would be
    // dropped); the latest entry for an index wins.
    let unique: BTreeMap<u16, P> = batch.into_iter().collect();
    if let Some((&n, _)) = unique.iter().find(|(n, _)| **n > last) {
        return Err(PlanError::OutOfRange(n, last));
    }
    let mut writes: Vec<(u16, P)> = unique.into_iter().collect();
    if writes.is_empty() {
        return Ok(CommitPlan { writes, real: 0 });
    }
    // The last program must not be last: its special case re-dirties the
    // cache. Never pad with it either.
    if writes.last().is_some_and(|(n, _)| *n == last) {
        let l = writes.pop().expect("non-empty");
        writes.insert(0, l);
    }
    let real = writes.len();
    if writes.len() == 1 && writes[0].0 == last {
        let first = store.group_start(last);
        let (n, p) = (first..last)
            .find_map(|n| known(n).map(|p| (n, p)))
            .ok_or(PlanError::NeedsNeighbour { first, last })?;
        writes.push((n, p));
    }
    let mut model = StoreModel::clean(store);
    for (n, _) in &writes {
        let effect = model.write(*n);
        debug_assert_ne!(
            effect,
            Effect::SavedAndDropped,
            "a real write would be dropped"
        );
    }
    let pad = writes.last().expect("non-empty").clone();
    while !model.is_clean() {
        model.write(pad.0);
        writes.push(pad.clone());
    }
    Ok(CommitPlan { writes, real })
}

#[cfg(test)]
mod tests {
    use super::*;

    const XD: Store = Store {
        group_size: 4,
        last: 499,
    };
    const PROLOGUE: Store = Store {
        group_size: 12,
        last: 499,
    };

    fn indices(plan: &CommitPlan<u32>) -> Vec<u16> {
        plan.writes().iter().map(|(n, _)| *n).collect()
    }

    fn slow(store: Store, programs: impl IntoIterator<Item = u16>) -> Vec<usize> {
        let mut m = StoreModel::clean(store);
        programs
            .into_iter()
            .enumerate()
            .filter(|(_, n)| m.write(*n) != Effect::Cached)
            .map(|(i, _)| i + 1)
            .collect()
    }

    #[test]
    fn xd_store_matches_measured_ack_pattern() {
        assert_eq!(slow(XD, 490..=499), vec![3, 7, 10]);
    }

    #[test]
    fn group_of_12_slow_only_at_492() {
        // Only the crossing into group 41.
        assert_eq!(slow(PROLOGUE, 490..=499), vec![3]);
        // §7 T1: 12 fast writes, the 13th slow and dropped.
        let mut m = StoreModel::clean(PROLOGUE);
        let e: Vec<Effect> = (0..13).map(|_| m.write(300)).collect();
        assert!(e[..12].iter().all(|e| *e == Effect::Cached));
        assert_eq!(e[12], Effect::SavedAndDropped);
        assert!(m.is_clean());
    }

    #[test]
    fn program_499_as_12th_write_re_caches() {
        let mut m = StoreModel::clean(PROLOGUE);
        for _ in 0..11 {
            m.write(499);
        }
        assert_eq!(m.write(499), Effect::SavedPreviousAndCached);
        assert!(!m.is_clean());
    }

    #[test]
    fn single_edit_pads_to_13_writes() {
        let plan = plan_commit(PROLOGUE, [(17, 1u32)], |_| None).unwrap();
        assert_eq!(indices(&plan), vec![17; 13]);
        assert_eq!(plan.real(), 1);
        let plan = plan_commit(XD, [(17, 1u32)], |_| None).unwrap();
        assert_eq!(indices(&plan), vec![17; 5]);
    }

    #[test]
    fn prologue_499_alone_pads_with_a_group_41_neighbour() {
        let err = plan_commit(PROLOGUE, [(499, 1u32)], |_| None).unwrap_err();
        assert_eq!(
            err,
            PlanError::NeedsNeighbour {
                first: 492,
                last: 499
            }
        );
        assert_eq!(
            err.to_string(),
            "writing only program 500 needs the current contents of program 493, 494, 495, \
             496, 497, 498 or 499 to commit the save; read one of them first"
        );
        let plan = plan_commit(PROLOGUE, [(499, 1u32)], |n| (n == 492).then_some(7)).unwrap();
        assert_eq!(indices(&plan)[..2], [499, 492]);
        assert_eq!(plan.writes().len(), 13);
        assert!(plan.writes()[1..].iter().all(|w| *w == (492, 7)));
    }

    #[test]
    fn xd_neighbour_message_is_unchanged() {
        assert_eq!(
            plan_commit(XD, [(499, 1u32)], |_| None)
                .unwrap_err()
                .to_string(),
            "writing only program 500 needs the current contents of program 497, 498 or 499 \
             to commit the save; read one of them first"
        );
    }

    #[test]
    fn prologue_plans_end_clean_and_never_pad_with_499() {
        let near = |n: u16| Some(u32::from(n));
        let batches: Vec<Vec<u16>> = (0..500)
            .map(|n| vec![n])
            .chain((0..500).step_by(7).map(|n| vec![n, (n * 13 + 5) % 500]))
            .chain([(488..500).collect(), (492..500).collect()])
            .collect();
        for b in batches {
            let plan = plan_commit(PROLOGUE, b.iter().map(|&n| (n, 0u32)), near).unwrap();
            let mut m = StoreModel::clean(PROLOGUE);
            for (i, (n, _)) in plan.writes().iter().enumerate() {
                let e = m.write(*n);
                if i < plan.real() {
                    assert_ne!(e, Effect::SavedAndDropped, "{b:?}");
                }
                if i >= plan.real() {
                    assert_ne!(*n, 499, "{b:?} pads with 499");
                }
            }
            assert!(m.is_clean(), "{b:?} -> {:?}", indices(&plan));
        }
    }

    #[test]
    fn rejects_out_of_range() {
        assert_eq!(
            plan_commit(PROLOGUE, [(500, 0u32)], |_| None),
            Err(PlanError::OutOfRange(500, 499))
        );
    }
}
