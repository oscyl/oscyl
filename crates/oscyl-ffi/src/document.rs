//! Documents (a library's programs or a synth's slots) with change
//! tracking and undo, their program menus, and library files as bytes.
//! Every method is cheap and synchronous unless it says it blocks.

use std::sync::{Arc, Mutex, MutexGuard};

use oscyl_app::program_menu::{self, Action, Candidate, Context};
use oscyl_app::slots::{slot_label, slots};
use oscyl_app::state::{Doc, Item, Reorder};

use crate::editor::GraphShape;
use crate::library::Libraries;
use crate::synth::{FfiError, driver};

/// How a program goes to another place.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum ReorderHow {
    /// Inserted there; the programs between shift by one.
    Move,
    /// Exchanged with the program there.
    Swap,
}

impl From<ReorderHow> for Reorder {
    fn from(h: ReorderHow) -> Reorder {
        match h {
            ReorderHow::Move => Reorder::Move,
            ReorderHow::Swap => Reorder::Swap,
        }
    }
}

/// One row of the program list.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ProgramRow {
    /// The slot as shown: "001", or "U2 001" on a synth with banks.
    pub slot: String,
    /// Empty until the program has been read from the synth.
    pub name: String,
    pub edited: bool,
    /// A factory slot: nothing changes it.
    pub read_only: bool,
    /// Read from the synth (always true in a library).
    pub loaded: bool,
}

struct Inner {
    doc: Doc,
    /// The synth's slots read so far; `None` for a library.
    loaded: Option<Vec<bool>>,
}

impl Inner {
    fn row(&self, i: usize) -> ProgramRow {
        let d = &self.doc;
        let loaded = self.is_loaded(i);
        ProgramRow {
            slot: slot_label(d.driver, i),
            name: if loaded { d.name(i) } else { String::new() },
            edited: d.is_edited(i),
            read_only: d.is_read_only(i),
            loaded,
        }
    }

    fn is_loaded(&self, i: usize) -> bool {
        match &self.loaded {
            Some(l) => l.get(i).copied().unwrap_or(false),
            None => i < self.doc.len(),
        }
    }
}

/// A set of programs being edited: one of Oscyl's libraries (from
/// [`Libraries::read`]), a file's (from [`import_file`]) or a synth's
/// slots ([`Document::synth`]). Every change is one undo step.
#[derive(uniffi::Object)]
pub struct Document(Mutex<Inner>);

impl Document {
    pub(crate) fn new(doc: Doc) -> Arc<Document> {
        Arc::new(Document(Mutex::new(Inner { doc, loaded: None })))
    }

    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.0.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Run `f` on the document.
    pub(crate) fn with<R>(&self, f: impl FnOnce(&Doc) -> R) -> R {
        f(&self.lock().doc)
    }

    /// Change program `i` with `f` under one lock, only once it has been
    /// read (`None` before: a synth's slot not read yet).
    pub(crate) fn edit_loaded<R>(&self, i: usize, f: impl FnOnce(&mut Doc) -> R) -> Option<R> {
        let mut inner = self.lock();
        inner.is_loaded(i).then(|| f(&mut inner.doc))
    }

    /// Change the document with `f`.
    pub(crate) fn with_mut<R>(&self, f: impl FnOnce(&mut Doc) -> R) -> R {
        f(&mut self.lock().doc)
    }
}

fn index(i: u32) -> usize {
    i as usize
}

/// One step of a document's history.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct HistoryStep {
    /// What it did, in words.
    pub title: String,
    /// The program it touched, and its slot's label.
    pub index: u32,
    pub slot: String,
    /// Done, or undone and waiting to be redone.
    pub done: bool,
}

fn count(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

#[uniffi::export]
impl Document {
    /// A library-to-be of `programs` (each checked by the driver; a bad
    /// one is refused), titled `title`: the program the synth is playing,
    /// say, to keep.
    #[uniffi::constructor]
    pub fn of_programs(
        driver_id: String,
        title: String,
        programs: Vec<Vec<u8>>,
    ) -> Result<Arc<Document>, FfiError> {
        let d = driver(&driver_id)?;
        let data = oscyl_driver::LibraryData {
            programs: programs
                .into_iter()
                .map(|data| oscyl_driver::LibProgram {
                    data,
                    ..Default::default()
                })
                .collect(),
            ..Default::default()
        };
        Doc::from_data(d, data, None, &title)
            .map(Document::new)
            .map_err(|e| FfiError::Refused { reason: e.0 })
    }

    /// Synth `driver_id`'s demo library, titled `title`: the example sounds
    /// its driver makes (nothing vendor-made is shipped), as a library to
    /// keep with `Libraries.add`.
    #[uniffi::constructor]
    pub fn demo(driver_id: String, title: String) -> Result<Arc<Document>, FfiError> {
        let d = driver(&driver_id)?;
        Doc::from_data(d, d.demo_library(), None, &title)
            .map(Document::new)
            .map_err(|e| FfiError::Refused { reason: e.0 })
    }

    /// A synth's slots, every one the init program until read: pass each
    /// program the synth sends to [`Document::receive`].
    #[uniffi::constructor]
    pub fn synth(driver_id: String, title: String) -> Result<Arc<Document>, FfiError> {
        let d = driver(&driver_id)?;
        let n = slots(d).len();
        Ok(Arc::new(Document(Mutex::new(Inner {
            doc: Doc::synth(d, vec![d.init_program(); n], &title),
            loaded: Some(vec![false; n]),
        }))))
    }

    /// The synth sent the program in `slot`: it becomes that slot's saved
    /// version (and its working copy, unless edited). Returns the slot's
    /// index, or `None` for a slot this document does not have.
    pub fn receive(&self, slot: u16, data: Vec<u8>) -> Option<u32> {
        let mut inner = self.lock();
        let i = slots(inner.doc.driver).iter().position(|&s| s == slot)?;
        inner.doc.set_base(i, data);
        if let Some(l) = inner.loaded.as_mut().and_then(|l| l.get_mut(i)) {
            *l = true;
        }
        Some(count(i))
    }

    /// What is left of a synth's slots once the synth is gone, as on the
    /// desktop (`Doc::offline`): when any was edited, every writable slot's
    /// working program, in slot order, as a library titled `title` (factory
    /// slots, which nothing edits, stay out, so it fits a library); `None`
    /// when nothing was edited.
    pub fn offline(&self, title: String) -> Option<Arc<Document>> {
        let doc = self.lock().doc.clone().offline(&title)?;
        let mut data = doc.to_data();
        let mut i = 0;
        data.programs.retain(|_| {
            i += 1;
            !doc.is_read_only(i - 1)
        });
        Doc::from_data(doc.driver, data, None, &title)
            .ok()
            .map(Document::new)
    }

    /// Every program has been read (always true in a library).
    pub fn all_read(&self) -> bool {
        let inner = self.lock();
        inner.loaded.as_ref().is_none_or(|l| l.iter().all(|x| *x))
    }

    pub fn is_loaded(&self, i: u32) -> bool {
        self.lock().is_loaded(index(i))
    }

    pub fn driver_id(&self) -> String {
        self.with(|d| d.driver.info().id.to_string())
    }

    pub fn title(&self) -> String {
        self.with(|d| d.title.clone())
    }

    pub fn len(&self) -> u32 {
        self.with(|d| count(d.len()))
    }

    pub fn is_empty(&self) -> bool {
        self.with(Doc::is_empty)
    }

    /// The most programs this document may hold.
    pub fn max_programs(&self) -> u32 {
        self.with(|d| count(d.max_programs()))
    }

    /// Program `i`'s name, empty when unreadable.
    pub fn name(&self, i: u32) -> String {
        self.with(|d| d.name(index(i)))
    }

    /// Program `i`'s slot as shown ("001", "U2 001").
    pub fn slot_label(&self, i: u32) -> String {
        self.with(|d| slot_label(d.driver, index(i)))
    }

    /// Every program as the list shows it, in one call.
    pub fn rows(&self) -> Vec<ProgramRow> {
        let inner = self.lock();
        (0..inner.doc.len()).map(|i| inner.row(i)).collect()
    }

    /// Program `i` as the list shows it (one slot just read, say).
    pub fn row(&self, i: u32) -> Option<ProgramRow> {
        let inner = self.lock();
        (index(i) < inner.doc.len()).then(|| inner.row(index(i)))
    }

    /// Program `i`'s bytes as edited, once it has been read; `None` before
    /// (and past the end). What a swipe on its row plays.
    pub fn program(&self, i: u32) -> Option<Vec<u8>> {
        let inner = self.lock();
        let i = index(i);
        inner
            .is_loaded(i)
            .then(|| inner.doc.work(i).map(<[u8]>::to_vec))
            .flatten()
    }

    /// Program `i`'s first picture (the first page that has one, curves
    /// about `points` long), for a program that has been read; `None`
    /// otherwise. The program list draws it small beside the name.
    pub fn thumbnail(&self, i: u32, points: u32) -> Option<GraphShape> {
        let inner = self.lock();
        let i = index(i);
        if !inner.is_loaded(i) {
            return None;
        }
        let d = &inner.doc;
        let p = d.work(i)?;
        let pages = d.driver.curation().pages.len();
        (0..pages)
            .find_map(|page| oscyl_app::editor::graph(d.driver, p, page, points as usize))
            .map(|g| crate::editor::shape(&g))
    }

    pub fn is_edited(&self, i: u32) -> bool {
        self.with(|d| d.is_edited(index(i)))
    }

    pub fn edited_count(&self) -> u32 {
        self.with(|d| count(d.edited_count()))
    }

    pub fn is_dirty(&self) -> bool {
        self.with(Doc::is_dirty)
    }

    /// Program `i` sits in a factory slot nothing changes.
    pub fn is_read_only(&self, i: u32) -> bool {
        self.with(|d| d.is_read_only(index(i)))
    }

    /// Changes with every edit; equal revisions mean equal programs.
    pub fn revision(&self) -> u64 {
        self.with(Doc::revision)
    }

    /// Rename program `i` (as one undo step). Refused in a factory slot or
    /// when the synth can't hold the name.
    pub fn set_name(&self, i: u32, name: String) -> Result<(), FfiError> {
        self.lock()
            .doc
            .set_name(index(i), &name)
            .map_err(|e| FfiError::Refused { reason: e.0 })
    }

    /// Add an Init Program right after `after` (at the end when `None`);
    /// returns where it went, or `None` when the document is full.
    pub fn new_program(&self, after: Option<u32>) -> Option<u32> {
        self.lock().doc.new_program(after.map(index)).map(count)
    }

    /// Take program `i` out (a library's; undo brings it back).
    pub fn remove(&self, i: u32) {
        self.lock().doc.remove(index(i));
    }

    /// Put program `from` at `to`; returns how many places changed (0:
    /// refused, as for a factory slot).
    pub fn reorder(&self, from: u32, to: u32, how: ReorderHow) -> u32 {
        count(self.lock().doc.reorder(index(from), index(to), how.into()))
    }

    /// How many places [`Document::reorder`] would change.
    pub fn reorder_changes(&self, from: u32, to: u32, how: ReorderHow) -> u32 {
        count(self.with(|d| d.reorder_changes(index(from), index(to), how.into())))
    }

    /// Programs `indices` (with their notes), for [`Document::paste`] or
    /// [`Document::paste_over`] here or in another document of the synth.
    pub fn copy(&self, indices: Vec<u32>) -> Arc<Clip> {
        self.with(|d| {
            Arc::new(Clip {
                driver: d.driver.info().id.to_string(),
                items: indices
                    .into_iter()
                    .filter_map(|i| {
                        let i = index(i);
                        Some(Item::new(
                            d.work(i)?.to_vec(),
                            d.infos.get(i).cloned().unwrap_or_default(),
                        ))
                    })
                    .collect(),
            })
        })
    }

    /// Insert `clip`'s programs at `at` as one undo step; false when they
    /// are another synth's or would not fit.
    pub fn paste(&self, at: u32, clip: &Clip) -> bool {
        let mut inner = self.lock();
        clip.driver == inner.doc.driver.info().id && inner.doc.insert(index(at), clip.items.clone())
    }

    /// Replace program `i` with `clip`'s first program as one undo step;
    /// false when nothing changed (another synth's, a factory slot, the
    /// same program).
    pub fn paste_over(&self, i: u32, clip: &Clip) -> bool {
        let mut inner = self.lock();
        let Some(first) = clip.items.first() else {
            return false;
        };
        if clip.driver != inner.doc.driver.info().id {
            return false;
        }
        let before = inner.doc.revision();
        inner.doc.set_program(index(i), first.work.clone());
        inner.doc.revision() != before
    }

    /// Put `data` (a generated sound) over program `i` as one undo step, or
    /// at the end when `i` is `None`; returns where it went. Refused on a
    /// factory slot, when the library is full, or for bytes the synth
    /// would not take.
    pub fn put(&self, i: Option<u32>, data: Vec<u8>) -> Result<u32, FfiError> {
        let mut inner = self.lock();
        let d = &mut inner.doc;
        d.driver
            .check_program(&data)
            .map_err(|e| FfiError::Refused { reason: e.0 })?;
        match i {
            Some(i) => {
                if d.is_read_only(index(i)) {
                    return Err(FfiError::Refused {
                        reason: oscyl_app::i18n::tr("This slot is read-only (a factory bank)."),
                    });
                }
                d.set_program(index(i), data);
                Ok(i)
            }
            None => {
                let at = d.len();
                if d.insert(at, vec![Item::new(data, Default::default())]) {
                    Ok(count(at))
                } else {
                    Err(FfiError::Refused {
                        reason: oscyl_app::i18n::tr(
                            "This library is full: a library holds up to {max} programs.",
                        )
                        .replace("{max}", &d.max_programs().to_string()),
                    })
                }
            }
        }
    }

    /// Put program `i` back as saved (one undo step).
    pub fn revert_program(&self, i: u32) {
        self.lock().doc.revert_program(index(i));
    }

    /// Put the init program in slot `i` (one undo step).
    pub fn clear_program(&self, i: u32) {
        let mut inner = self.lock();
        let init = inner.doc.driver.init_program();
        inner.doc.set_program(index(i), init);
    }

    pub fn can_undo(&self) -> bool {
        self.with(Doc::can_undo)
    }

    pub fn can_redo(&self) -> bool {
        self.with(Doc::can_redo)
    }

    /// Undo one step; returns the program it touched.
    pub fn undo(&self) -> Option<u32> {
        self.lock().doc.undo().map(count)
    }

    /// Redo one step; returns the program it touched.
    pub fn redo(&self) -> Option<u32> {
        self.lock().doc.redo().map(count)
    }

    /// End the current undo step (a drag of one field is one step).
    pub fn seal(&self) {
        self.lock().doc.seal();
    }

    /// The history, oldest step first, each in words: the steps done, then
    /// the steps undone.
    pub fn history(&self) -> Vec<HistoryStep> {
        self.with(|d| {
            let slot = |i: usize| slot_label(d.driver, i);
            oscyl_app::review::history(d)
                .into_iter()
                .map(|h| HistoryStep {
                    slot: slot(h.index),
                    title: h.title,
                    index: count(h.index),
                    done: h.done,
                })
                .collect()
        })
    }

    /// Undo or redo until `done` steps of [`Self::history`] are done;
    /// returns the program the last step touched.
    pub fn jump_history(&self, done: u32) -> Option<u32> {
        self.lock().doc.jump(index(done)).map(count)
    }
}

/// Programs copied from a document, with their synth.
#[derive(uniffi::Object)]
pub struct Clip {
    driver: String,
    items: Vec<Item>,
}

#[uniffi::export]
impl Clip {
    pub fn driver_id(&self) -> String {
        self.driver.clone()
    }

    pub fn len(&self) -> u32 {
        count(self.items.len())
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

/// One thing a program's menu does; mirrors `oscyl_app::program_menu::Action`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum ProgramAction {
    /// Rename the program (the desktop renames in the editor's title;
    /// until the mobile editor lands, the menu does).
    Rename,
    /// A copy right after it, in a library.
    Duplicate,
    Copy,
    PasteOver,
    CopyToLibrary,
    NewProgram,
    MoveUp,
    MoveDown,
    Revert,
    /// Take the program out of the library.
    Remove,
    /// Put the init program in the slot.
    Clear,
}

impl ProgramAction {
    fn of(a: Action) -> Option<ProgramAction> {
        Some(match a {
            Action::Copy => ProgramAction::Copy,
            Action::PasteOver => ProgramAction::PasteOver,
            Action::CopyToLibrary => ProgramAction::CopyToLibrary,
            Action::NewProgram => ProgramAction::NewProgram,
            Action::MoveUp => ProgramAction::MoveUp,
            Action::MoveDown => ProgramAction::MoveDown,
            Action::Revert => ProgramAction::Revert,
            Action::Remove => ProgramAction::Remove,
            Action::Clear => ProgramAction::Clear,
            Action::SwapUp | Action::SwapDown => return None,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct MenuEntry {
    pub action: ProgramAction,
    pub enabled: bool,
}

/// A library a program can be copied to.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct LibraryTarget {
    pub id: String,
    pub name: String,
}

/// A program's menu: its sections in order, and the libraries Copy to
/// Library offers (then a new one).
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ProgramMenu {
    pub sections: Vec<Vec<MenuEntry>>,
    pub targets: Vec<LibraryTarget>,
}

/// The menu of program `index` of `doc`, shown from library `library_id`
/// of `libraries` (`None`: the synth), with `clipboard` holding what was
/// last copied.
#[uniffi::export]
pub fn program_menu(
    doc: &Document,
    index: u32,
    libraries: Option<Arc<Libraries>>,
    library_id: Option<String>,
    clipboard: Option<Arc<Clip>>,
) -> ProgramMenu {
    let inner = doc.lock();
    let i = index as usize;
    let in_library = library_id.is_some();
    let cx = Context {
        in_library,
        can_reorder: in_library || inner.loaded.as_ref().is_some_and(|l| l.iter().all(|x| *x)),
        read: inner.is_loaded(i),
        clipboard: clipboard.as_deref().map(|c| c.driver.as_str()),
    };
    let entry = |a| MenuEntry {
        action: ProgramAction::of(a).expect("in the menu"),
        enabled: program_menu::enabled(&inner.doc, i, &cx, a),
    };
    let sections = program_menu::sections(in_library)
        .into_iter()
        .map(|s| {
            let mut out = Vec::new();
            for a in s {
                match a {
                    // Renaming changes the program as clearing does.
                    Action::Copy => out.push(MenuEntry {
                        action: ProgramAction::Rename,
                        enabled: program_menu::enabled(&inner.doc, i, &cx, Action::Clear),
                    }),
                    // A duplicate is a new program that is a copy.
                    Action::NewProgram => {
                        out.push(entry(a));
                        out.push(MenuEntry {
                            action: ProgramAction::Duplicate,
                            enabled: entry(a).enabled && entry(Action::Copy).enabled,
                        });
                        continue;
                    }
                    _ => {}
                }
                out.push(entry(a));
            }
            out
        })
        .collect();
    let candidates = libraries
        .map(|l| {
            l.metas()
                .into_iter()
                .map(|m| Candidate {
                    broken: oscyl_midi::driver(&m.driver).is_none(),
                    key: m.id,
                    name: m.name,
                    driver: m.driver,
                    managed: true,
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let targets =
        program_menu::copy_targets(candidates, library_id.as_ref(), inner.doc.driver.info().id)
            .into_iter()
            .map(|(id, name)| LibraryTarget { id, name })
            .collect();
    ProgramMenu { sections, targets }
}

/// A file read in: its synth, a name for it (the file's), and its programs.
#[derive(uniffi::Record)]
pub struct ImportedLibrary {
    pub driver_id: String,
    pub name: String,
    pub document: Arc<Document>,
}

/// A format a library can be exported to.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct FileFormatInfo {
    /// Without the dot.
    pub ext: String,
    pub name: String,
}

/// Read file `file_name`'s `bytes`: an Oscyl library or a synth's own
/// format, told by the name's extension (by the contents when it has none
/// Oscyl knows). Refused past what a library of the synth holds. Blocks
/// while it parses: call it off the main thread.
#[uniffi::export]
pub fn import_file(file_name: String, bytes: Vec<u8>) -> Result<ImportedLibrary, FfiError> {
    let (d, lib) = oscyl_app::files::read(&file_name, &bytes).map_err(FfiError::failed)?;
    oscyl_app::files::fits(d, &lib).map_err(FfiError::failed)?;
    let name = oscyl_app::files::title_of(&file_name);
    let doc = Doc::from_data(d, lib, None, &name).map_err(|e| FfiError::failed(e.0))?;
    Ok(ImportedLibrary {
        driver_id: d.info().id.to_string(),
        name,
        document: Document::new(doc),
    })
}

/// The formats a library of driver `driver_id` exports to, the usual one
/// first, Oscyl's own last.
#[uniffi::export]
pub fn export_formats(driver_id: String) -> Result<Vec<FileFormatInfo>, FfiError> {
    Ok(oscyl_app::files::formats(driver(&driver_id)?)
        .into_iter()
        .map(|f| FileFormatInfo {
            ext: f.ext.to_string(),
            name: f.name,
        })
        .collect())
}

/// `doc`'s programs as a file in the format `ext` names.
#[uniffi::export]
pub fn export_file(doc: &Document, ext: String) -> Result<Vec<u8>, FfiError> {
    doc.with(|d| oscyl_app::files::write(&ext, d.driver, &d.to_data()))
        .map_err(FfiError::failed)
}

#[cfg(test)]
mod tests {
    use super::*;

    const XD: &str = "korg-minilogue-xd";
    const REV2: &str = "sequential-prophet-rev2";

    fn demo(id: &str) -> Arc<Document> {
        let d = driver(id).unwrap();
        Document::new(Doc::from_data(d, d.demo_library(), None, "Demo").unwrap())
    }

    #[test]
    fn every_driver_has_a_demo_library() {
        for d in oscyl_midi::DRIVERS {
            let id = d.info().id.to_string();
            let demo = Document::demo(id.clone(), "Demo".into()).unwrap();
            assert!(!demo.is_empty(), "{id} has demo sounds");
        }
    }

    #[test]
    fn thumbnail_is_first_picture_and_none_when_not_loaded() {
        let doc = demo(XD);
        let shape = doc
            .thumbnail(0, 48)
            .expect("a demo xd program has a picture");
        assert!(!shape.marks.is_empty());
        assert_eq!(doc.thumbnail(doc.len(), 48), None, "no program there");
        let synth = Document::synth(XD.into(), "xd".into()).unwrap();
        assert_eq!(synth.thumbnail(0, 48), None, "a slot not read yet");
    }

    #[test]
    fn program_is_the_edit_once_read() {
        let doc = demo(XD);
        let d = driver(XD).unwrap();
        assert_eq!(
            doc.program(0).as_deref(),
            doc.with(|x| x.work(0).map(<[u8]>::to_vec)).as_deref()
        );
        assert!(d.check_program(&doc.program(0).unwrap()).is_ok());
        assert_eq!(doc.program(doc.len()), None);
        let synth = Document::synth(XD.into(), "xd".into()).unwrap();
        assert_eq!(synth.program(0), None, "a slot not read yet");
    }

    /// The spike for the program list: thumbnails must be cheap enough to draw as rows scroll in.
    #[test]
    fn thumbnails_are_cheap() {
        let doc = demo(XD);
        let n = doc.len().min(120);
        let started = std::time::Instant::now();
        for i in 0..n {
            assert!(doc.thumbnail(i, 48).is_some());
        }
        let each = started.elapsed() / n;
        eprintln!("thumbnail: {each:?} each over {n} programs");
    }

    #[test]
    fn the_history_lists_steps_and_jumps() {
        let doc = demo(XD);
        doc.set_name(0, "One".into()).unwrap();
        doc.set_name(1, "Two".into()).unwrap();
        let h = doc.history();
        assert_eq!(h.len(), 2);
        assert!(h.iter().all(|s| s.done) && h[1].index == 1 && !h[1].slot.is_empty());
        assert_eq!(doc.jump_history(0), Some(0));
        assert!(doc.history().iter().all(|s| !s.done) && !doc.can_undo());
        assert_eq!(doc.jump_history(2), Some(1));
        assert!(doc.can_undo() && !doc.can_redo());
    }

    #[test]
    fn edits_undo_and_redo() {
        let doc = demo(XD);
        let n = doc.len();
        let first = doc.name(0);
        doc.set_name(0, "Lead".into()).unwrap();
        assert!(doc.is_edited(0) && doc.is_dirty());
        assert_eq!(doc.new_program(Some(0)), Some(1));
        assert_eq!(doc.len(), n + 1);
        assert_eq!(doc.reorder(0, 2, ReorderHow::Move), 3);
        assert_eq!(doc.name(2), "Lead");
        assert_eq!(doc.undo(), Some(0));
        assert_eq!(doc.undo(), Some(1));
        assert_eq!(doc.undo(), Some(0));
        assert_eq!((doc.name(0), doc.len(), doc.is_dirty()), (first, n, false));
        assert!(doc.can_redo());
        doc.redo();
        assert_eq!(doc.name(0), "Lead");
    }

    #[test]
    fn copy_paste_between_documents_of_one_synth() {
        let (a, b) = (demo(XD), demo(XD));
        let clip = a.copy(vec![3]);
        assert_eq!(clip.len(), 1);
        let n = b.len();
        assert!(b.paste(n, &clip));
        assert_eq!(b.name(n), a.name(3));
        assert!(b.paste_over(0, &clip));
        assert_eq!(b.name(0), a.name(3));
        assert!(!demo(REV2).paste(0, &clip), "another synth's");
    }

    #[test]
    fn the_synth_reads_slot_by_slot_and_keeps_factory_slots() {
        let doc = Document::synth(REV2.into(), "Rev2".into()).unwrap();
        assert!(!doc.all_read() && !doc.is_loaded(0));
        let d = driver(REV2).unwrap();
        let mut p = d.init_program();
        d.set_name(&mut p, "Pad").unwrap();
        let slot = slots(d)[5];
        assert_eq!(doc.receive(slot, p), Some(5));
        assert!(doc.is_loaded(5) && !doc.is_edited(5));
        assert_eq!(doc.rows()[5].name, "Pad");
        assert_eq!(doc.rows()[4].name, "", "not read yet");
        let factory = doc.rows().iter().position(|r| r.read_only).unwrap() as u32;
        assert!(matches!(
            doc.set_name(factory, "Mine".into()),
            Err(FfiError::Refused { .. })
        ));
        let menu = program_menu(&doc, factory, None, None, None);
        let on: Vec<_> = menu
            .sections
            .concat()
            .into_iter()
            .filter(|e| e.enabled)
            .map(|e| e.action)
            .collect();
        assert_eq!(on, [ProgramAction::Copy]);
    }

    #[test]
    fn a_synth_gone_leaves_its_edits_as_a_library() {
        let doc = Document::synth(REV2.into(), "Rev2".into()).unwrap();
        assert!(doc.offline("x".into()).is_none(), "nothing edited");
        doc.set_name(3, "Kept".into()).unwrap();
        let kept = doc.offline("Rev2 (disconnected)".into()).unwrap();
        let d = driver(REV2).unwrap();
        assert_eq!(kept.len() as usize, oscyl_app::slots::library_max(d));
        assert_eq!(kept.name(3), "Kept");
        assert_eq!(kept.title(), "Rev2 (disconnected)");
        assert!(!kept.rows().iter().any(|r| r.read_only));
    }

    #[test]
    fn a_library_program_menu() {
        let doc = demo(XD);
        let menu = program_menu(&doc, 0, None, Some("lib".into()), None);
        let all: Vec<_> = menu.sections.concat();
        let on = |a| all.iter().find(|e| e.action == a).map(|e| e.enabled);
        assert_eq!(on(ProgramAction::Rename), Some(true));
        assert_eq!(on(ProgramAction::Duplicate), Some(true));
        assert_eq!(on(ProgramAction::Remove), Some(true));
        assert_eq!(on(ProgramAction::Clear), None);
        assert_eq!(on(ProgramAction::MoveUp), Some(false), "already first");
        assert_eq!(on(ProgramAction::PasteOver), Some(false), "nothing copied");
    }

    #[test]
    fn files_round_trip_and_bad_ones_are_errors() {
        let doc = demo(XD);
        for f in export_formats(XD.into()).unwrap() {
            let bytes = export_file(&doc, f.ext.clone()).unwrap();
            let back = import_file(format!("Gig.{}", f.ext), bytes).unwrap();
            assert_eq!((back.driver_id.as_str(), back.name.as_str()), (XD, "Gig"));
            assert_eq!(back.document.rows(), doc.rows(), "{}", f.ext);
        }
        assert!(matches!(
            import_file("x.oscyllib".into(), b"nonsense".to_vec()),
            Err(FfiError::Failed { .. })
        ));
    }
}
