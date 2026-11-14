//! Documents: a set of programs being edited, from a library file or the
//! synth, with per-field change tracking and undo/redo. Programs are the
//! driver's native bytes. GTK-free and unit tested.

use std::collections::BTreeMap;
use std::fmt;
use std::path::PathBuf;

use oscyl_driver::{Driver, DriverError, LibProgram, LibraryData};

use crate::i18n::tr;
use crate::slots::{library_max, slot_label, slots, writable};

/// A program's bytes, in its driver's layout.
pub type Program = Vec<u8>;

/// A program's notes: who made it and what the user wrote about it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Info {
    pub author: String,
    pub comment: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Edit {
    index: usize,
    field: Option<usize>,
    before: Program,
    after: Program,
    /// What the history calls it, when its changes are not the words for it
    /// ("Rolled Filter", "Recalled snapshot A").
    label: Option<String>,
}

/// A program with its saved version and notes, as moved in and out of a
/// library.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    pub work: Program,
    pub base: Program,
    pub info: Info,
}

impl Item {
    /// A program new to the document: nothing to compare it with yet.
    pub fn new(p: Program, info: Info) -> Item {
        Item {
            base: p.clone(),
            work: p,
            info,
        }
    }
}

/// One undo step.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Step {
    Edit(Box<Edit>),
    /// Several programs replaced at once (a library staged onto the synth).
    Batch(Vec<Edit>),
    /// Programs inserted at `at` (or, when `insert` is false, removed).
    Splice {
        at: usize,
        items: Vec<Item>,
        insert: bool,
    },
    Move {
        from: usize,
        to: usize,
    },
    /// Two programs exchanged, with their saved versions and notes.
    Swap {
        a: usize,
        b: usize,
    },
}

/// One step of a document's history, as a list of it tells it apart
/// ([`Doc::steps`]); positions are as the step saw them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StepInfo<'a> {
    /// Program `index` went from `before` to `after` (a field, its name,
    /// or the whole program).
    Edit {
        index: usize,
        before: &'a [u8],
        after: &'a [u8],
        /// Its own words, when it has them.
        label: Option<&'a str>,
    },
    /// So many programs replaced at once.
    Replaced(usize),
    /// So many programs added at a position; the first one's name.
    Added(usize, usize, String),
    /// So many programs removed from a position; the first one's name.
    Removed(usize, usize, String),
    /// A program moved from one position to another.
    Moved(usize, usize),
    /// Two programs exchanged.
    Swapped(usize, usize),
}

/// How a program goes to another place.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reorder {
    /// Inserted there; the programs between shift by one.
    Move,
    /// Exchanged with the program there.
    Swap,
}

/// Where each position from `from` to `to` (both included) takes its
/// program from once program `from` goes to `to`: (position, old index).
pub fn arrangement(from: usize, to: usize, how: Reorder) -> Vec<(usize, usize)> {
    match how {
        Reorder::Swap => vec![(from, to), (to, from)],
        Reorder::Move if from <= to => (from..=to)
            .map(|p| (p, if p == to { from } else { p + 1 }))
            .collect(),
        Reorder::Move => (to..=from)
            .map(|p| (p, if p == to { from } else { p - 1 }))
            .collect(),
    }
}

#[derive(Clone)]
pub struct Doc {
    /// The synth model whose programs these are.
    pub driver: &'static dyn Driver,
    /// The library's file; `None` for the synth and for unsaved libraries.
    pub path: Option<PathBuf>,
    pub title: String,
    /// Programs as last saved to the file or committed to the synth.
    base: Vec<Program>,
    /// Programs as currently edited.
    work: Vec<Program>,
    /// `edited[i]` caches `work[i] != base[i]`, so dirty checks during a
    /// drag cost nothing.
    edited: Vec<bool>,
    pub infos: Vec<Info>,
    /// What the source file held besides programs, kept verbatim.
    extras: BTreeMap<String, Vec<u8>>,
    undo: Vec<Step>,
    redo: Vec<Step>,
    /// When false, the next field edit starts a new undo step.
    merging: bool,
    /// A gesture that moves several fields (a graph handle): until the
    /// next seal, all its field changes are one undo step.
    gesture: bool,
    /// Counts changes to the working copy, so autosave knows what it wrote.
    revision: u64,
    /// The synth: its slots stay put, so reordering changes what each
    /// slot holds (edits to sync) rather than moving programs with their
    /// saved versions.
    fixed: bool,
    /// The synth's read-only (factory) slots, by position: nothing edits
    /// them. Always false in a library.
    read_only: Vec<bool>,
}

impl fmt::Debug for Doc {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Doc")
            .field("driver", &self.driver.info().id)
            .field("title", &self.title)
            .field("programs", &self.work.len())
            .finish_non_exhaustive()
    }
}

impl Doc {
    /// A library's programs; every one must be a program of `driver`.
    pub fn from_data(
        driver: &'static dyn Driver,
        lib: LibraryData,
        path: Option<PathBuf>,
        title: &str,
    ) -> Result<Doc, DriverError> {
        for p in &lib.programs {
            driver.check_program(&p.data)?;
        }
        let (programs, infos): (Vec<Program>, Vec<Info>) = lib
            .programs
            .into_iter()
            .map(|p| {
                (
                    p.data,
                    Info {
                        author: p.author,
                        comment: p.comment,
                    },
                )
            })
            .unzip();
        Ok(Doc {
            driver,
            path,
            title: title.to_string(),
            edited: vec![false; programs.len()],
            base: programs.clone(),
            work: programs,
            infos,
            extras: lib.extras,
            undo: Vec::new(),
            redo: Vec::new(),
            merging: false,
            gesture: false,
            revision: 0,
            fixed: false,
            read_only: Vec::new(),
        })
    }

    pub fn synth(driver: &'static dyn Driver, programs: Vec<Program>, title: &str) -> Doc {
        Doc {
            driver,
            path: None,
            title: title.to_string(),
            infos: vec![Info::default(); programs.len()],
            edited: vec![false; programs.len()],
            base: programs.clone(),
            work: programs,
            extras: BTreeMap::new(),
            undo: Vec::new(),
            redo: Vec::new(),
            merging: false,
            gesture: false,
            revision: 0,
            fixed: true,
            read_only: {
                let w = writable(driver);
                slots(driver).iter().map(|s| !w.contains(s)).collect()
            },
        }
    }

    /// Program `i` sits in a slot Oscyl never writes (a factory bank).
    pub fn is_read_only(&self, i: usize) -> bool {
        self.read_only.get(i).copied().unwrap_or(false)
    }

    fn refuse_read_only(&self, i: usize) -> Result<(), DriverError> {
        if self.is_read_only(i) {
            Err(DriverError(tr("This slot is read-only (a factory bank).")))
        } else {
            Ok(())
        }
    }

    /// What is left of a synth's document once the synth is gone: its
    /// edits, as a document titled `title`; `None` when there are none.
    pub fn offline(mut self, title: &str) -> Option<Doc> {
        self.is_dirty().then(|| {
            self.title = title.to_string();
            self
        })
    }

    pub fn len(&self) -> usize {
        self.work.len()
    }

    pub fn is_empty(&self) -> bool {
        self.work.is_empty()
    }

    pub fn work(&self, i: usize) -> Option<&[u8]> {
        self.work.get(i).map(Vec::as_slice)
    }

    pub fn base(&self, i: usize) -> Option<&[u8]> {
        self.base.get(i).map(Vec::as_slice)
    }

    /// Program `i`'s name, empty when unreadable.
    pub fn name(&self, i: usize) -> String {
        self.work(i)
            .and_then(|p| self.driver.name(p).ok())
            .unwrap_or_default()
    }

    /// The most programs this document may hold.
    pub fn max_programs(&self) -> usize {
        library_max(self.driver)
    }

    pub fn is_edited(&self, i: usize) -> bool {
        self.edited.get(i).copied().unwrap_or(false)
    }

    pub fn edited_indices(&self) -> Vec<usize> {
        (0..self.edited.len()).filter(|&i| self.edited[i]).collect()
    }

    pub fn edited_count(&self) -> usize {
        self.edited.iter().filter(|e| **e).count()
    }

    pub fn is_dirty(&self) -> bool {
        self.edited.contains(&true)
    }

    fn mark(&mut self, i: usize) {
        self.edited[i] = self.work[i] != self.base[i];
    }

    /// Program `i` as read from the synth: it becomes the base, and the
    /// working copy too unless it was already edited.
    pub fn set_base(&mut self, i: usize, p: Program) {
        if i >= self.len() {
            return;
        }
        if !self.edited[i] {
            self.work[i] = p.clone();
        }
        self.base[i] = p;
        self.mark(i);
    }

    /// Set one field. Consecutive changes to the same field of the same
    /// program merge into one undo step until [`Doc::seal`] (a drag is one
    /// step, not hundreds); inside a [`Doc::gesture`], changes to any field
    /// of it merge.
    pub fn set_field(&mut self, i: usize, field: usize, value: u16) -> Result<(), DriverError> {
        self.refuse_read_only(i)?;
        let Some(current) = self.work.get(i) else {
            return Ok(());
        };
        if self.driver.get(current, field) == Ok(value) {
            return Ok(());
        }
        let mut next = current.clone();
        self.driver.set(&mut next, field, value)?;
        self.apply(i, Some(field), next);
        Ok(())
    }

    pub fn set_name(&mut self, i: usize, name: &str) -> Result<(), DriverError> {
        self.refuse_read_only(i)?;
        let Some(current) = self.work.get(i) else {
            return Ok(());
        };
        let mut next = current.clone();
        self.driver.set_name(&mut next, name)?;
        if &next != current {
            self.seal();
            self.apply(i, None, next);
            self.seal();
        }
        Ok(())
    }

    /// Replace a whole program as one undo step.
    pub fn set_program(&mut self, i: usize, p: Program) {
        if self.is_read_only(i) || self.driver.check_program(&p).is_err() {
            return;
        }
        if self.work.get(i) != Some(&p) {
            self.seal();
            self.apply(i, None, p);
            self.seal();
        }
    }

    /// [`Doc::set_program`] as one undo step the history calls `label`.
    pub fn set_program_named(&mut self, i: usize, p: Program, label: &str) {
        let steps = self.undo.len();
        self.set_program(i, p);
        if self.undo.len() > steps
            && let Some(Step::Edit(e)) = self.undo.last_mut()
        {
            e.label = Some(label.to_string());
        }
    }

    fn apply(&mut self, index: usize, field: Option<usize>, after: Program) {
        let before = self.work[index].clone();
        self.redo.clear();
        match self.undo.last_mut() {
            Some(Step::Edit(top))
                if self.merging
                    && field.is_some()
                    && top.index == index
                    && (top.field == field || (self.gesture && top.field.is_some())) =>
            {
                top.after = after.clone();
            }
            _ => self.undo.push(Step::Edit(Box::new(Edit {
                index,
                field,
                before,
                after: after.clone(),
                label: None,
            }))),
        }
        self.merging = field.is_some();
        self.work[index] = after;
        self.mark(index);
        self.revision += 1;
    }

    /// A change to the working copy since the document was made; equal
    /// revisions mean equal programs.
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// Start a new undo step with `step`, then run it.
    fn push(&mut self, step: Step) -> usize {
        self.seal();
        self.redo.clear();
        let i = self.run(&step, true);
        self.undo.push(step);
        i
    }

    /// Run `step` forward (do, redo) or backward (undo); returns the
    /// program index it leaves in view.
    fn run(&mut self, step: &Step, forward: bool) -> usize {
        self.revision += 1;
        match step {
            Step::Edit(e) => {
                self.put(e, forward);
                e.index
            }
            Step::Batch(edits) => {
                for e in edits {
                    self.put(e, forward);
                }
                edits.first().map_or(0, |e| e.index)
            }
            &Step::Splice {
                at,
                ref items,
                insert,
            } => {
                if insert == forward {
                    for (k, it) in items.iter().enumerate() {
                        self.work.insert(at + k, it.work.clone());
                        self.base.insert(at + k, it.base.clone());
                        self.infos.insert(at + k, it.info.clone());
                        self.edited.insert(at + k, it.work != it.base);
                    }
                } else {
                    let end = at + items.len();
                    self.work.drain(at..end);
                    self.base.drain(at..end);
                    self.infos.drain(at..end);
                    self.edited.drain(at..end);
                }
                at.min(self.len().saturating_sub(1))
            }
            Step::Move { from, to } => {
                let (from, to) = if forward { (*from, *to) } else { (*to, *from) };
                let w = self.work.remove(from);
                self.work.insert(to, w);
                let b = self.base.remove(from);
                self.base.insert(to, b);
                let n = self.infos.remove(from);
                self.infos.insert(to, n);
                let e = self.edited.remove(from);
                self.edited.insert(to, e);
                to
            }
            &Step::Swap { a, b } => {
                self.work.swap(a, b);
                self.base.swap(a, b);
                self.infos.swap(a, b);
                self.edited.swap(a, b);
                if forward { b } else { a }
            }
        }
    }

    fn put(&mut self, e: &Edit, forward: bool) {
        self.work[e.index] = if forward { &e.after } else { &e.before }.clone();
        self.mark(e.index);
    }

    /// Add `items` at `at` as one undo step. Refused (false) past
    /// [`Doc::max_programs`].
    pub fn insert(&mut self, at: usize, items: Vec<Item>) -> bool {
        let foreign = items
            .iter()
            .any(|it| self.driver.check_program(&it.work).is_err());
        if foreign
            || items.is_empty()
            || at > self.len()
            || self.len() + items.len() > self.max_programs()
        {
            return false;
        }
        self.push(Step::Splice {
            at,
            items,
            insert: true,
        });
        true
    }

    /// Add an Init Program right after program `after` (at the end when
    /// none), as one undo step; returns where it went, or `None` when the
    /// library is full.
    pub fn new_program(&mut self, after: Option<usize>) -> Option<usize> {
        let at = after.map_or(self.len(), |i| (i + 1).min(self.len()));
        self.insert(
            at,
            vec![Item::new(self.driver.init_program(), Info::default())],
        )
        .then_some(at)
    }

    /// Remove program `i` as one undo step.
    pub fn remove(&mut self, i: usize) {
        if i < self.len() {
            let items = vec![Item {
                work: self.work[i].clone(),
                base: self.base[i].clone(),
                info: self.infos[i].clone(),
            }];
            self.push(Step::Splice {
                at: i,
                items,
                insert: false,
            });
        }
    }

    /// How many places would hold another program once program `from`
    /// goes to `to`.
    pub fn reorder_changes(&self, from: usize, to: usize, how: Reorder) -> usize {
        let touched = arrangement(from.min(self.len()), to.min(self.len()), how);
        if touched.iter().any(|&(p, _)| self.is_read_only(p)) {
            return 0;
        }
        if from == to || from >= self.len() || to >= self.len() {
            return 0;
        }
        arrangement(from, to, how)
            .into_iter()
            .filter(|&(p, old)| self.work[p] != self.work[old])
            .count()
    }

    /// Put program `from` at `to` as one undo step; returns how many
    /// places changed. In a library a program moves with its saved
    /// version and notes. On the synth the slots stay put: each changed
    /// slot is an edit, written at the next sync like any other.
    pub fn reorder(&mut self, from: usize, to: usize, how: Reorder) -> usize {
        let n = self.reorder_changes(from, to, how);
        if n == 0 {
            return 0;
        }
        if from == to || from >= self.len() || to >= self.len() {
            return 0;
        }
        if self.fixed {
            let edits: Vec<Edit> = arrangement(from, to, how)
                .into_iter()
                .filter(|&(p, old)| self.work[p] != self.work[old])
                .map(|(p, old)| Edit {
                    index: p,
                    field: None,
                    before: self.work[p].clone(),
                    after: self.work[old].clone(),
                    label: None,
                })
                .collect();
            if !edits.is_empty() {
                self.push(Step::Batch(edits));
            }
        } else {
            self.push(match how {
                Reorder::Move => Step::Move { from, to },
                Reorder::Swap => Step::Swap { a: from, b: to },
            });
        }
        n
    }

    /// Make `programs` the working copy of programs 0, 1, … as one undo
    /// step; programs that already match are left alone. Returns how many
    /// changed.
    pub fn stage(&mut self, programs: &[Program]) -> usize {
        let edits: Vec<Edit> = programs
            .iter()
            .zip(&self.work)
            .enumerate()
            .filter(|(i, (p, w))| p != w && !self.is_read_only(*i))
            .map(|(index, (p, w))| Edit {
                index,
                field: None,
                before: w.clone(),
                after: p.clone(),
                label: None,
            })
            .collect();
        let n = edits.len();
        if n > 0 {
            self.push(Step::Batch(edits));
        }
        n
    }

    /// Undo the last step and forget it (a cancelled staging).
    pub fn discard(&mut self) {
        if self.undo().is_some() {
            self.redo.pop();
        }
    }

    /// Start a gesture that moves several fields at once (a graph handle):
    /// every field change from here to the next [`Doc::seal`] is one step.
    pub fn gesture(&mut self) {
        self.gesture = true;
    }

    /// End the current undo step.
    pub fn seal(&mut self) {
        self.merging = false;
        self.gesture = false;
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    /// Undo one step; returns the program index it touched.
    pub fn undo(&mut self) -> Option<usize> {
        self.seal();
        let s = self.undo.pop()?;
        let i = self.run(&s, false);
        self.redo.push(s);
        Some(i)
    }

    pub fn redo(&mut self) -> Option<usize> {
        self.seal();
        let s = self.redo.pop()?;
        let i = self.run(&s, true);
        self.undo.push(s);
        Some(i)
    }

    /// Every step of the history, oldest first: the steps done (the undo
    /// side), then the steps undone (the redo side, the next to redo
    /// first). [`Doc::done`] of them are done.
    pub fn steps(&self) -> Vec<StepInfo<'_>> {
        self.undo
            .iter()
            .chain(self.redo.iter().rev())
            .map(|s| match s {
                Step::Edit(e) => StepInfo::Edit {
                    index: e.index,
                    before: &e.before,
                    after: &e.after,
                    label: e.label.as_deref(),
                },
                Step::Batch(edits) => StepInfo::Replaced(edits.len()),
                Step::Splice { at, items, insert } => {
                    let name = items
                        .first()
                        .and_then(|it| self.driver.name(&it.work).ok())
                        .unwrap_or_default();
                    if *insert {
                        StepInfo::Added(*at, items.len(), name)
                    } else {
                        StepInfo::Removed(*at, items.len(), name)
                    }
                }
                Step::Move { from, to } => StepInfo::Moved(*from, *to),
                Step::Swap { a, b } => StepInfo::Swapped(*a, *b),
            })
            .collect()
    }

    /// How many of [`Doc::steps`] are done: where the history stands.
    pub fn done(&self) -> usize {
        self.undo.len()
    }

    /// Undo or redo until `done` steps are done (a jump in the history);
    /// returns the program the last step touched, `None` when already there.
    pub fn jump(&mut self, done: usize) -> Option<usize> {
        let mut last = None;
        while self.undo.len() > done {
            last = Some(self.undo()?);
        }
        while self.undo.len() < done {
            last = Some(self.redo()?);
        }
        last
    }

    /// Put one field back to its saved value (as an undoable step).
    pub fn revert_field(&mut self, i: usize, field: usize) {
        if let Some(Ok(b)) = self.base.get(i).map(|b| self.driver.get(b, field)) {
            self.seal();
            let _ = self.set_field(i, field, b);
            self.seal();
        }
    }

    pub fn revert_program(&mut self, i: usize) {
        if let Some(b) = self.base.get(i).cloned() {
            self.set_program(i, b);
        }
    }

    /// Everything is saved/synced: the working copy becomes the base.
    pub fn mark_saved(&mut self) {
        self.base = self.work.clone();
        self.edited.fill(false);
        self.seal();
    }

    /// The synth stored `written` (slot, program): each becomes that slot's
    /// saved version. An edit made while the write ran stays an edit,
    /// unlike [`Doc::mark_saved_programs`], which takes the working copy.
    pub fn stored(&mut self, written: &[(u16, Program)]) {
        let at = slots(self.driver);
        for (slot, p) in written {
            if let Some(i) = at.iter().position(|s| s == slot) {
                self.set_base(i, p.clone());
            }
        }
        self.seal();
    }

    /// Mark only some programs as saved (a partial sync).
    pub fn mark_saved_programs(&mut self, indices: &[usize]) {
        for &i in indices {
            if let Some(w) = self.work.get(i).cloned() {
                self.base[i] = w;
                self.edited[i] = false;
            }
        }
        self.seal();
    }

    /// The saved versions of programs `indices`, in that order, as a
    /// library. A library has no slot numbers, so each program's comment
    /// names its slot ("Slot 124").
    pub fn backup_of(&self, indices: &[usize]) -> LibraryData {
        LibraryData {
            programs: indices
                .iter()
                .filter_map(|&i| {
                    Some(LibProgram {
                        data: self.base.get(i)?.clone(),
                        author: String::new(),
                        comment: tr("Slot {number}")
                            .replace("{number}", &slot_label(self.driver, i)),
                    })
                })
                .collect(),
            extras: BTreeMap::new(),
        }
    }

    /// The document as library data, keeping whatever the source file
    /// held besides programs.
    pub fn to_data(&self) -> LibraryData {
        LibraryData {
            programs: self
                .work
                .iter()
                .zip(&self.infos)
                .map(|(p, info)| LibProgram {
                    data: p.clone(),
                    author: info.author.clone(),
                    comment: info.comment.clone(),
                })
                .collect(),
            extras: self.extras.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_edit_made_during_a_sync_stays_an_edit() {
        let d = oscyl_midi::driver("korg-minilogue-xd").unwrap();
        let mut doc = Doc::synth(d, vec![d.init_program(); 3], "xd");
        doc.set_name(1, "Sent").unwrap();
        let written = vec![(slots(d)[1], doc.work(1).unwrap().to_vec())];
        // While the write runs, the person edits on.
        doc.set_name(1, "Later").unwrap();
        doc.stored(&written);
        assert!(doc.is_edited(1), "the later edit was never written");
        assert_eq!(d.name(doc.base(1).unwrap()).unwrap(), "Sent");
    }

    #[test]
    fn a_two_field_drag_is_one_undo_step() {
        let d = oscyl_midi::driver("korg-minilogue-xd").unwrap();
        let mut doc = Doc::synth(d, vec![d.init_program(); 3], "xd");
        let base = doc.work(1).unwrap().to_vec();
        let (a, b) = (
            d.fields().iter().position(|f| f.name == "Cutoff").unwrap(),
            d.fields()
                .iter()
                .position(|f| f.name == "Resonance")
                .unwrap(),
        );
        for v in [100, 200, 300] {
            doc.gesture();
            doc.set_field(1, a, v).unwrap();
            doc.set_field(1, b, v).unwrap();
        }
        doc.seal();
        doc.set_field(1, a, 5).unwrap();
        doc.seal();
        doc.undo();
        assert_eq!(
            d.get(doc.work(1).unwrap(), b),
            Ok(300),
            "the last gesture alone"
        );
        doc.undo();
        assert_eq!(
            doc.work(1).unwrap(),
            base.as_slice(),
            "the whole drag at once"
        );
        assert!(!doc.can_undo());
    }
    use oscyl_minilogue_xd::Field;

    fn xd() -> &'static dyn Driver {
        &oscyl_minilogue_xd::DRIVER
    }

    fn init() -> Program {
        xd().init_program()
    }

    /// `f`'s index in the xd's schema.
    fn fi(f: Field) -> usize {
        Field::ALL.iter().position(|g| *g == f).unwrap()
    }

    fn get(p: &[u8], f: Field) -> u16 {
        xd().get(p, fi(f)).unwrap()
    }

    fn set(p: &mut [u8], f: Field, v: u16) {
        xd().set(p, fi(f), v).unwrap()
    }

    fn doc(n: usize) -> Doc {
        Doc::synth(xd(), vec![init(); n], "test")
    }

    #[test]
    fn drag_is_one_undo_step() {
        let mut d = doc(3);
        for v in [100, 200, 300] {
            d.set_field(1, fi(Field::Cutoff), v).unwrap();
        }
        d.seal();
        d.set_field(1, fi(Field::Resonance), 50).unwrap();
        assert!(d.is_edited(1));
        assert_eq!(d.undo(), Some(1));
        assert_eq!(
            get(&d.work[1], Field::Resonance),
            get(&init(), Field::Resonance)
        );
        assert_eq!(get(&d.work[1], Field::Cutoff), 300);
        d.undo();
        assert!(!d.is_dirty());
        d.redo();
        assert_eq!(get(&d.work[1], Field::Cutoff), 300);
    }

    #[test]
    fn the_history_lists_every_step_and_a_jump_lands_on_it() {
        let mut d = doc(2);
        d.set_field(0, fi(Field::Cutoff), 100).unwrap();
        d.seal();
        d.set_name(1, "Lead").unwrap();
        d.set_field(0, fi(Field::Resonance), 50).unwrap();
        d.seal();
        assert_eq!((d.steps().len(), d.done()), (3, 3));
        assert!(matches!(d.steps()[1], StepInfo::Edit { index: 1, .. }));
        // Back to after the first step: the rename and the resonance are undone, and stay listed.
        assert_eq!(d.jump(1), Some(1));
        assert_eq!((d.steps().len(), d.done()), (3, 1));
        assert_eq!(get(&d.work[0], Field::Cutoff), 100);
        assert_ne!(get(&d.work[0], Field::Resonance), 50);
        assert_ne!(d.name(1), "Lead");
        // The redo side keeps its order: the rename is still the second step.
        assert!(matches!(d.steps()[1], StepInfo::Edit { index: 1, .. }));
        assert_eq!(d.jump(3), Some(0));
        assert_eq!(get(&d.work[0], Field::Resonance), 50);
        assert_eq!(d.name(1), "Lead");
        assert_eq!(d.jump(3), None);
        assert_eq!(d.jump(0), Some(0));
        assert!(!d.can_undo() && d.edited_indices().is_empty());
    }

    #[test]
    fn different_fields_do_not_merge() {
        let mut d = doc(1);
        d.set_field(0, fi(Field::Cutoff), 100).unwrap();
        d.set_field(0, fi(Field::Resonance), 100).unwrap();
        d.undo();
        assert_eq!(get(&d.work[0], Field::Cutoff), 100);
    }

    #[test]
    fn revert_and_save() {
        let mut d = doc(2);
        d.set_field(0, fi(Field::Cutoff), 10).unwrap();
        d.set_field(1, fi(Field::Cutoff), 20).unwrap();
        assert_eq!(d.edited_indices(), vec![0, 1]);
        d.revert_field(0, fi(Field::Cutoff));
        assert_eq!(d.edited_indices(), vec![1]);
        d.mark_saved_programs(&[1]);
        assert!(!d.is_dirty());
        d.undo();
        assert!(d.is_dirty(), "undo after save makes it dirty again");
    }

    #[test]
    fn edited_flags_follow_every_change() {
        let mut d = doc(2);
        d.set_field(0, fi(Field::Cutoff), 10).unwrap();
        assert_eq!((d.edited_count(), d.is_dirty()), (1, true));
        d.set_field(0, fi(Field::Cutoff), get(&init(), Field::Cutoff))
            .unwrap();
        assert!(!d.is_dirty(), "dragging back to the saved value");
        d.set_field(0, fi(Field::Cutoff), 20).unwrap();
        assert!(d.is_edited(0));
        let mut read = init();
        set(&mut read, Field::Resonance, 7);
        d.set_base(1, read.clone());
        assert_eq!(
            d.work(1),
            Some(read.as_slice()),
            "untouched programs follow the synth"
        );
        d.set_base(0, read);
        assert!(d.is_edited(0), "edits survive a late read");
        d.mark_saved();
        assert_eq!(d.edited_count(), 0);
    }

    #[test]
    fn out_of_range_is_rejected_without_changing_state() {
        let mut d = doc(1);
        assert!(d.set_field(0, fi(Field::Vco1Wave), 9).is_err());
        assert!(!d.is_dirty());
        assert!(!d.can_undo());
    }

    #[test]
    fn backup_holds_the_saved_programs_labelled_by_slot() {
        let mut d = doc(200);
        let saved = named("Saved");
        d.set_base(123, saved.clone());
        d.set_field(123, fi(Field::Cutoff), 5).unwrap();
        let progs = d.backup_of(&[123, 4]).programs;
        assert_eq!(progs.len(), 2);
        assert_eq!(progs[0].data, saved, "the synth's version, not the edit");
        assert_eq!(progs[0].comment, "Slot 124");
        assert_eq!(progs[1].comment, "Slot 005");
    }

    #[test]
    fn library_round_trip_keeps_extras() {
        let mut lib = LibraryData::default();
        lib.programs.push(LibProgram {
            data: init(),
            author: "Ann".into(),
            comment: "Warm".into(),
        });
        lib.extras.insert("FavoriteData.fav_data".into(), vec![1]);
        let d = Doc::from_data(xd(), lib.clone(), None, "x").unwrap();
        assert_eq!(d.infos[0].author, "Ann");
        assert_eq!(d.to_data(), lib);
    }

    fn named(name: &str) -> Program {
        let mut p = init();
        xd().set_name(&mut p, name).unwrap();
        p
    }

    /// A library holding programs named `names`.
    fn lib_doc(names: &[&str]) -> Doc {
        let lib = LibraryData {
            programs: names
                .iter()
                .map(|n| LibProgram {
                    data: named(n),
                    ..LibProgram::default()
                })
                .collect(),
            ..LibraryData::default()
        };
        Doc::from_data(xd(), lib, None, "lib").unwrap()
    }

    fn names(d: &Doc) -> Vec<String> {
        (0..d.len()).map(|i| d.name(i)).collect()
    }

    #[test]
    fn staging_marks_only_differing_slots_and_cancel_restores() {
        let mut synth = doc(5);
        let before = synth.clone();
        let mut lib = vec![init(); 3];
        lib[1] = named("Lead");
        assert_eq!(synth.stage(&lib), 1);
        assert_eq!(
            synth.edited_indices(),
            vec![1],
            "slots 1 and 3 already match"
        );
        assert_eq!(synth.work(1), Some(lib[1].as_slice()));
        assert_ne!(synth.revision(), before.revision());
        synth.discard();
        assert_eq!(synth.work, before.work);
        assert!(!synth.is_dirty());
        assert!(
            !synth.can_undo() && !synth.can_redo(),
            "cancel leaves no step"
        );
        assert_eq!(synth.stage(&[init()]), 0, "nothing to send");
        assert!(!synth.can_undo());
    }

    #[test]
    fn copy_remove_and_move_are_undoable_steps() {
        let mut d = lib_doc(&["A", "B"]);
        assert!(d.insert(2, vec![Item::new(named("C"), Info::default())]));
        assert_eq!(names(&d), ["A", "B", "C"]);
        assert!(!d.is_edited(2), "a copied program is not an edit");
        assert_eq!(d.to_data().programs.len(), 3);
        d.set_field(0, fi(Field::Cutoff), 10).unwrap();
        d.reorder(0, 2, Reorder::Move);
        assert_eq!(names(&d), ["B", "C", "A"]);
        assert!(d.is_edited(2), "the edit moves with its program");
        d.remove(1);
        assert_eq!(names(&d), ["B", "A"]);
        assert_eq!(d.undo(), Some(1));
        assert_eq!(names(&d), ["B", "C", "A"]);
        d.undo();
        assert_eq!(names(&d), ["A", "B", "C"]);
        assert!(d.is_edited(0));
        d.undo();
        d.undo();
        assert_eq!(names(&d), ["A", "B"]);
        assert_eq!(d.infos.len(), 2);
        d.redo();
        assert_eq!(names(&d), ["A", "B", "C"]);
    }

    #[test]
    fn a_library_holds_at_most_500_programs() {
        let mut d = Doc::synth(xd(), vec![init(); 499], "lib");
        let item = || vec![Item::new(init(), Info::default())];
        assert!(d.insert(d.len(), item()));
        assert!(!d.insert(d.len(), item()));
        assert_eq!(d.len(), 500);
    }

    #[test]
    fn a_rev2_doc_edits_and_undoes() {
        let rev2 = oscyl_midi::driver("sequential-prophet-rev2").unwrap();
        let mut d = Doc::synth(rev2, vec![rev2.init_program(); 3], "rev2");
        let f = 5;
        let max = rev2.fields()[f].max;
        d.set_field(1, f, max).unwrap();
        assert_eq!(d.edited_indices(), vec![1]);
        assert!(d.set_field(1, f, max + 1).is_err());
        d.set_name(2, "Wide Pad").unwrap();
        assert_eq!(d.name(2), "Wide Pad");
        d.undo();
        d.undo();
        assert!(!d.is_dirty());
        assert_eq!(d.max_programs(), 512);
        assert_eq!(d.new_program(None), Some(3));
        assert_eq!(d.work(3), Some(rev2.init_program().as_slice()));
    }

    #[test]
    fn a_new_program_goes_after_the_selected_one_as_one_step() {
        let mut d = lib_doc(&["A", "B"]);
        let init_name = xd().name(&init()).unwrap();
        assert_eq!(d.new_program(Some(0)), Some(1));
        assert_eq!(names(&d), ["A", init_name.as_str(), "B"]);
        assert_eq!(d.new_program(None), Some(3), "none selected: at the end");
        assert_eq!(d.new_program(Some(9)), Some(4), "past the end: at the end");
        assert!(!d.is_edited(1), "a new program is not an edit");
        d.undo();
        d.undo();
        assert_eq!(names(&d), ["A", init_name.as_str(), "B"]);
        d.undo();
        assert_eq!(names(&d), ["A", "B"]);
        let mut full = Doc::synth(xd(), vec![init(); 500], "full");
        assert_eq!(full.new_program(None), None);
        assert!(!full.can_undo());
    }

    #[test]
    fn moving_a_program_far_is_one_step_and_slots_follow() {
        let mut d = lib_doc(&["A", "B", "C", "D", "E"]);
        d.reorder(0, 3, Reorder::Move);
        assert_eq!(names(&d), ["B", "C", "D", "A", "E"]);
        assert_eq!(
            crate::slots::slot_label(xd(), 3),
            "004",
            "slot labels are positions"
        );
        d.reorder(4, 1, Reorder::Move);
        assert_eq!(names(&d), ["B", "E", "C", "D", "A"]);
        assert_eq!(d.undo(), Some(4));
        assert_eq!(names(&d), ["B", "C", "D", "A", "E"]);
        assert_eq!(d.undo(), Some(0));
        assert_eq!(names(&d), ["A", "B", "C", "D", "E"]);
        assert!(!d.can_undo());
    }

    #[test]
    fn a_library_swap_is_one_step_carrying_saved_versions() {
        let mut d = lib_doc(&["A", "B", "C"]);
        d.set_field(0, fi(Field::Cutoff), 10).unwrap();
        assert_eq!(d.reorder_changes(0, 2, Reorder::Swap), 2);
        assert_eq!(d.reorder(0, 2, Reorder::Swap), 2);
        assert_eq!(names(&d), ["C", "B", "A"]);
        assert_eq!(d.edited_indices(), vec![2], "the edit travels");
        assert_eq!(d.undo(), Some(0));
        assert_eq!(names(&d), ["A", "B", "C"]);
        assert_eq!(d.edited_indices(), vec![0]);
    }

    #[test]
    fn reordering_the_synth_edits_exactly_the_changed_slots() {
        let mut synth = Doc::synth(xd(), ["A", "B", "C", "D", "E"].map(named).to_vec(), "synth");
        // Move: the slots in between shift by one, each an edit.
        assert_eq!(synth.reorder_changes(1, 3, Reorder::Move), 3);
        assert_eq!(synth.reorder(1, 3, Reorder::Move), 3);
        assert_eq!(names(&synth), ["A", "C", "D", "B", "E"]);
        assert_eq!(synth.edited_indices(), vec![1, 2, 3]);
        assert_eq!(
            synth.base(1),
            Some(named("B").as_slice()),
            "slots keep their saved programs"
        );
        synth.undo();
        assert!(!synth.is_dirty());
        assert!(!synth.can_undo(), "one step");
        // Swap: two slots.
        assert_eq!(synth.reorder(4, 0, Reorder::Swap), 2);
        assert_eq!(names(&synth), ["E", "B", "C", "D", "A"]);
        assert_eq!(synth.edited_indices(), vec![0, 4]);
        synth.undo();
        assert!(!synth.is_dirty());
        // Moving onto itself, or between equal programs, changes nothing.
        let mut same = Doc::synth(xd(), vec![init(); 4], "synth");
        assert_eq!(same.reorder(0, 3, Reorder::Move), 0);
        assert_eq!(same.reorder(2, 2, Reorder::Swap), 0);
        assert!(!same.is_dirty() && !same.can_undo());
        // Moving back by hand clears the edits.
        synth.reorder(0, 1, Reorder::Swap);
        synth.reorder(0, 1, Reorder::Swap);
        assert!(!synth.is_dirty());
    }

    #[test]
    fn arrangement_says_where_each_place_takes_its_program_from() {
        assert_eq!(arrangement(1, 3, Reorder::Move), [(1, 2), (2, 3), (3, 1)]);
        assert_eq!(arrangement(3, 1, Reorder::Move), [(1, 3), (2, 1), (3, 2)]);
        assert_eq!(arrangement(3, 1, Reorder::Swap), [(3, 1), (1, 3)]);
    }

    #[test]
    fn factory_slots_of_a_synth_are_read_only() {
        let rev2 = oscyl_midi::driver("sequential-prophet-rev2").unwrap();
        let init = rev2.init_program();
        let mut d = Doc::synth(rev2, vec![init.clone(); 1024], "rev2");
        assert!(!d.is_read_only(10) && d.is_read_only(600));
        assert!(d.set_field(600, 5, rev2.fields()[5].max).is_err());
        assert!(d.set_name(600, "Mine").is_err());
        let mut named = init.clone();
        rev2.set_name(&mut named, "Other").unwrap();
        d.set_program(600, named.clone());
        d.revert_program(600);
        assert!(
            !d.is_dirty() && !d.can_undo(),
            "nothing changes a factory slot"
        );
        d.set_program(20, named.clone());
        assert_eq!(d.reorder_changes(20, 600, Reorder::Swap), 0);
        assert_eq!(d.reorder(20, 600, Reorder::Move), 0);
        assert_eq!(d.reorder(20, 10, Reorder::Swap), 2, "user slots still move");
        let mut lib = vec![init.clone(); 1024];
        lib[700] = named.clone();
        lib[5] = named;
        let mut fresh = Doc::synth(rev2, vec![init; 1024], "rev2");
        assert_eq!(fresh.stage(&lib), 1, "staging skips factory slots");
        assert_eq!(fresh.edited_indices(), vec![5]);
        let xd = Doc::synth(xd(), vec![self::init(); 500], "xd");
        assert!((0..500).all(|i| !xd.is_read_only(i)));
    }

    #[test]
    fn a_disconnected_synths_edits_stay_as_a_document() {
        let mut d = doc(3);
        assert!(d.clone().offline("gone").is_none(), "nothing to keep");
        d.set_field(1, fi(Field::Cutoff), 10).unwrap();
        let kept = d.offline("minilogue xd (disconnected)").unwrap();
        assert_eq!(kept.title, "minilogue xd (disconnected)");
        assert_eq!(kept.edited_indices(), vec![1]);
        assert!(kept.can_undo());
    }

    #[test]
    fn another_synths_program_is_refused() {
        let rev2 = oscyl_midi::driver("sequential-prophet-rev2").unwrap();
        let mut d = lib_doc(&["A"]);
        assert!(!d.insert(1, vec![Item::new(rev2.init_program(), Info::default())]));
        d.set_program(0, rev2.init_program());
        assert!(!d.can_undo());
    }
}
