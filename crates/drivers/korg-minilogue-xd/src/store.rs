//! Committing program writes to flash deterministically.
//!
//! Firmware 2.10 keeps one 4 KB flash sector (a *group* of 4 programs) in a
//! RAM cache. A 0x4C write only updates that cache; the flash save happens
//! later, from the main loop, after ~300 idle iterations. A 0x1C read that
//! races that deferred save corrupts the synth's reply bookkeeping until the
//! next write.
//!
//! The firmware also saves synchronously, *before* acknowledging, in two
//! cases: a write into a different group, and a 5th write into the cached
//! group (which is then dropped and the cache emptied). [`plan_commit`]
//! uses the second case: it pads every batch with byte-identical copies of
//! its last program until that write is the 5th into its group. The ACK of
//! the final write therefore means "saved, nothing pending", and reads are
//! safe and coherent straight after it. No timing is involved.
//!
//! The logic is the logue family's ([`oscyl_logue::store`]) with the xd's
//! [`STORE`].

use oscyl_logue::store::Store;

use crate::Program;

pub use oscyl_logue::store::Effect;

/// Programs per flash sector (4), and program 499's special case.
pub const STORE: Store = Store {
    group_size: 4,
    last: 499,
};

/// Host-side mirror of the synth's program cache (firmware 2.10).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreModel(oscyl_logue::store::StoreModel);

impl Default for StoreModel {
    fn default() -> Self {
        StoreModel::clean()
    }
}

impl StoreModel {
    /// A store with nothing cached and nothing pending.
    pub fn clean() -> StoreModel {
        StoreModel(oscyl_logue::store::StoreModel::clean(STORE))
    }

    pub fn is_clean(&self) -> bool {
        self.0.is_clean()
    }

    /// Apply one program write.
    pub fn write(&mut self, program: u16) -> Effect {
        self.0.write(program)
    }
}

/// A batch of program writes that ends with the synth's store saved and
/// clean. Only [`plan_commit`] can build one.
pub type CommitPlan = oscyl_logue::store::CommitPlan<Program>;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PlanError {
    #[error("program number {0} is out of range (0-499)")]
    OutOfRange(u16),
    #[error(
        "writing only program 500 needs the current contents of program 497, 498 or 499 \
         to commit the save; read one of them first"
    )]
    NeedsNeighbour,
}

/// Order and pad `batch` so it ends saved, assuming the store is clean when
/// the batch starts (true after any previous `CommitPlan`).
///
/// `known` returns the synth's current contents of a program, if the caller
/// has read it; it is only needed when the batch is program 499 alone.
pub fn plan_commit(
    batch: impl IntoIterator<Item = (u16, Program)>,
    known: impl Fn(u16) -> Option<Program>,
) -> Result<CommitPlan, PlanError> {
    use oscyl_logue::store::PlanError as E;
    oscyl_logue::store::plan_commit(STORE, batch, known).map_err(|e| match e {
        E::OutOfRange(n, _) => PlanError::OutOfRange(n),
        E::NeedsNeighbour { .. } => PlanError::NeedsNeighbour,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prog(tag: &str) -> Program {
        let mut p = Program::init();
        p.set_name(tag).unwrap();
        p
    }

    fn indices(plan: &CommitPlan) -> Vec<u16> {
        plan.writes().iter().map(|(n, _)| *n).collect()
    }

    fn none(_: u16) -> Option<Program> {
        None
    }

    #[test]
    fn model_matches_measured_ack_pattern() {
        // Programs 490-499 from a clean store: slow ACKs (synchronous saves)
        // were measured at writes #3, #7 and #10.
        let mut m = StoreModel::clean();
        let slow: Vec<usize> = (490..=499)
            .enumerate()
            .filter(|(_, n)| m.write(*n) != Effect::Cached)
            .map(|(i, _)| i + 1)
            .collect();
        assert_eq!(slow, vec![3, 7, 10]);
    }

    #[test]
    fn single_edit_is_padded_to_a_fifth_write() {
        let plan = plan_commit([(17, prog("a"))], none).unwrap();
        assert_eq!(indices(&plan), vec![17; 5]);
        assert_eq!(plan.real(), 1);
    }

    #[test]
    fn padding_counts_writes_already_in_the_last_group() {
        let plan = plan_commit([(16, prog("a")), (17, prog("b")), (30, prog("c"))], none).unwrap();
        assert_eq!(indices(&plan), vec![16, 17, 30, 30, 30, 30, 30]);
        let plan = plan_commit([(28, prog("a")), (29, prog("b")), (30, prog("c"))], none).unwrap();
        assert_eq!(indices(&plan), vec![28, 29, 30, 30, 30]);
    }

    #[test]
    fn full_group_needs_one_padding_write() {
        let plan = plan_commit((8..12).map(|n| (n, prog("x"))), none).unwrap();
        assert_eq!(indices(&plan), vec![8, 9, 10, 11, 11]);
    }

    #[test]
    fn duplicates_collapse_and_latest_wins() {
        let plan = plan_commit([(5, prog("old")), (5, prog("new"))], none).unwrap();
        assert_eq!(plan.real(), 1);
        assert!(plan.writes().iter().all(|(_, p)| p.name() == "new"));
    }

    #[test]
    fn program_499_moves_first() {
        let plan = plan_commit([(3, prog("a")), (499, prog("z"))], none).unwrap();
        assert_eq!(indices(&plan), vec![499, 3, 3, 3, 3, 3]);
    }

    #[test]
    fn program_499_alone_pads_with_a_known_neighbour() {
        assert_eq!(
            plan_commit([(499, prog("z"))], none),
            Err(PlanError::NeedsNeighbour)
        );
        let plan = plan_commit([(499, prog("z"))], |n| (n == 498).then(|| prog("y"))).unwrap();
        assert_eq!(indices(&plan), vec![499, 498, 498, 498, 498]);
        assert_eq!(plan.real(), 1);
        assert_eq!(plan.writes()[1].1.name(), "y");
    }

    #[test]
    fn every_plan_ends_clean_and_drops_nothing_real() {
        // Exhaustive over all single programs and a spread of pairs.
        let near = |n: u16| Some(prog(&format!("{n}")));
        let batches: Vec<Vec<u16>> = (0..500)
            .map(|n| vec![n])
            .chain((0..500).step_by(7).map(|n| vec![n, (n * 13 + 5) % 500]))
            .collect();
        for b in batches {
            let plan = plan_commit(b.iter().map(|&n| (n, prog("x"))), near).unwrap();
            let mut m = StoreModel::clean();
            for (i, (n, _)) in plan.writes().iter().enumerate() {
                let e = m.write(*n);
                if i < plan.real() {
                    assert_ne!(e, Effect::SavedAndDropped, "{b:?}");
                }
            }
            assert!(m.is_clean(), "{b:?} -> {:?}", indices(&plan));
            assert_eq!(
                plan.writes().last().map(|(n, _)| *n == 499),
                Some(false),
                "{b:?}"
            );
        }
    }

    #[test]
    fn rejects_out_of_range() {
        assert_eq!(
            plan_commit([(500, prog("x"))], none),
            Err(PlanError::OutOfRange(500))
        );
    }
}
