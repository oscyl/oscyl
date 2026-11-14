//! Oscyl's own libraries: a folder of `.oscyllib` files with their order,
//! names and folders. Every method reads or writes that folder, so it
//! blocks: call them off the main thread.

use std::sync::{Arc, Mutex, MutexGuard};

use oscyl_app::libraries::{LineKind, Meta, Store, Trashed as StoreTrashed};
use oscyl_app::state::Doc;
use oscyl_driver::LibraryData;

use crate::document::Document;
use crate::synth::{FfiError, driver};

/// What a line of the outline is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum NodeKind {
    Folder,
    Library,
}

/// One line of the libraries tree, depth first.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct OutlineLine {
    pub depth: u32,
    pub kind: NodeKind,
    pub id: String,
    pub name: String,
    /// A folder shows what is inside it (always true for a library).
    pub open: bool,
    /// Every folder around it is open.
    pub shown: bool,
    /// First or last in its folder: it can't move up or down.
    pub first: bool,
    pub last: bool,
    /// A library's synth (`None` for a folder).
    pub driver_id: Option<String>,
    /// How many programs a library holds (`None` for a folder or a
    /// library that can't be read).
    pub programs: Option<u32>,
    /// Why a library can't be read (a synth this build doesn't know, a
    /// damaged file).
    pub broken: Option<String>,
}

/// Something deleted, kept until [`Libraries::restore`] brings it back or
/// [`Libraries::purge`] removes it for good.
#[derive(uniffi::Object)]
pub struct Trashed(Mutex<Option<StoreTrashed>>);

/// The libraries folder.
#[derive(uniffi::Object)]
pub struct Libraries(Mutex<Store>);

impl Libraries {
    fn lock(&self) -> MutexGuard<'_, Store> {
        self.0.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Every library's details, in the tree's order.
    pub(crate) fn metas(&self) -> Vec<Meta> {
        self.lock().entries().to_vec()
    }

    fn change<R>(&self, f: impl FnOnce(&mut Store) -> Result<R, String>) -> Result<R, FfiError> {
        f(&mut self.lock()).map_err(FfiError::failed)
    }
}

#[uniffi::export]
impl Libraries {
    /// Open (making it if needed) the libraries folder `dir`: converts
    /// libraries in the old format once (originals kept in
    /// `before-oscyllib/`) and empties the last run's trash.
    #[uniffi::constructor]
    pub fn open(dir: String) -> Result<Arc<Libraries>, FfiError> {
        let store = Store::open(dir.into()).map_err(FfiError::failed)?;
        Ok(Arc::new(Libraries(Mutex::new(store))))
    }

    /// The tree as lines, depth first. Reads every library to count its
    /// programs.
    pub fn outline(&self) -> Vec<OutlineLine> {
        let store = self.lock();
        store
            .outline()
            .into_iter()
            .map(|l| {
                let depth = u32::try_from(l.depth).unwrap_or(u32::MAX);
                let id = match &l.kind {
                    LineKind::Folder { id, .. } | LineKind::Library(id) => id,
                };
                let (at, len) = store.siblings(id).unwrap_or((0, 1));
                let (first, last) = (at == 0, at + 1 >= len);
                match l.kind {
                    LineKind::Folder { id, name, open } => OutlineLine {
                        depth,
                        kind: NodeKind::Folder,
                        id,
                        name,
                        open,
                        shown: l.shown,
                        first,
                        last,
                        driver_id: None,
                        programs: None,
                        broken: None,
                    },
                    LineKind::Library(id) => {
                        let meta = store.entries().iter().find(|m| m.id == id);
                        let read = store.read(&id);
                        OutlineLine {
                            depth,
                            kind: NodeKind::Library,
                            name: meta.map(|m| m.name.clone()).unwrap_or_default(),
                            driver_id: meta.map(|m| m.driver.clone()),
                            programs: read
                                .as_ref()
                                .ok()
                                .map(|(_, data)| u32::try_from(data.programs.len()).unwrap_or(0)),
                            broken: read.err(),
                            id,
                            open: true,
                            shown: l.shown,
                            first,
                            last,
                        }
                    }
                }
            })
            .collect()
    }

    /// "Library 1", "Library 2"…: the first name no library has.
    pub fn new_library_name(&self) -> String {
        self.lock().new_library_name()
    }

    /// "Folder 1", "Folder 2"…: the first name no folder has.
    pub fn new_folder_name(&self) -> String {
        self.lock().new_folder_name()
    }

    /// A new empty library of driver `driver_id` named `name`, last in
    /// folder `parent` (`None`: the top level); returns its id.
    pub fn create(
        &self,
        name: String,
        driver_id: String,
        parent: Option<String>,
    ) -> Result<String, FfiError> {
        let d = driver(&driver_id)?;
        self.change(|s| s.create(&name, d, &LibraryData::default(), parent.as_deref()))
    }

    /// A new library holding `doc`'s programs (an imported file), last in
    /// folder `parent`; returns its id.
    pub fn add(
        &self,
        name: String,
        doc: &Document,
        parent: Option<String>,
    ) -> Result<String, FfiError> {
        let (d, data) = doc.with(|d| (d.driver, d.to_data()));
        self.change(|s| s.create(&name, d, &data, parent.as_deref()))
    }

    /// A new empty folder last in folder `parent`; returns its id.
    pub fn new_folder(&self, name: String, parent: Option<String>) -> Result<String, FfiError> {
        self.change(|s| s.new_folder(&name, parent.as_deref()))
    }

    /// Rename a library or a folder.
    pub fn rename(&self, id: String, name: String) -> Result<(), FfiError> {
        self.change(|s| s.rename(&id, &name))
    }

    /// Copy library `id` as `name`, right after it; returns the copy's id.
    pub fn duplicate(&self, id: String, name: String) -> Result<String, FfiError> {
        self.change(|s| s.duplicate(&id, &name))
    }

    /// Move a library, or a folder with everything inside, to the trash.
    pub fn delete(&self, id: String) -> Result<Arc<Trashed>, FfiError> {
        let t = self.change(|s| s.delete(&id))?;
        Ok(Arc::new(Trashed(Mutex::new(Some(t)))))
    }

    /// Bring back something deleted where it was (at the top level when its
    /// folder has gone). Once only.
    pub fn restore(&self, trashed: &Trashed) -> Result<(), FfiError> {
        let t = trashed
            .0
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take()
            .ok_or_else(|| FfiError::failed("already restored or purged"))?;
        self.change(|s| s.restore(t))
    }

    /// Remove something deleted for good (its undo has gone).
    pub fn purge(&self, trashed: &Trashed) {
        if let Some(t) = trashed.0.lock().unwrap_or_else(|e| e.into_inner()).take() {
            self.lock().purge(&t);
        }
    }

    /// Put node `id` in folder `parent` (`None`: the top level) before what
    /// is now at `at` there (past the end: last).
    pub fn place(&self, id: String, parent: Option<String>, at: u32) -> Result<(), FfiError> {
        self.change(|s| s.place(&id, parent.as_deref(), at as usize))
    }

    /// Move node `id` one place up (-1) or down (1) in its folder; false
    /// when it is already first or last there.
    pub fn step(&self, id: String, by: i32) -> Result<bool, FfiError> {
        self.change(|s| s.step(&id, by as isize))
    }

    /// Open or close folder `id` (remembered).
    pub fn set_open(&self, id: String, open: bool) -> Result<(), FfiError> {
        self.change(|s| s.set_open(&id, open))
    }

    /// Library `id`'s programs, to edit.
    pub fn read(&self, id: String) -> Result<Arc<Document>, FfiError> {
        let store = self.lock();
        let name = store
            .entries()
            .iter()
            .find(|m| m.id == id)
            .map(|m| m.name.clone())
            .unwrap_or_default();
        let (d, data) = store.read(&id).map_err(FfiError::failed)?;
        let doc = Doc::from_data(d, data, None, &name).map_err(|e| FfiError::failed(e.0))?;
        Ok(Document::new(doc))
    }

    /// Write `doc`'s programs as library `id`.
    pub fn save(&self, id: String, doc: &Document) -> Result<(), FfiError> {
        let data = doc.with(Doc::to_data);
        self.change(|s| s.save(&id, &data))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const XD: &str = "korg-minilogue-xd";

    /// A fresh folder under the system temp dir, removed on drop.
    struct TempDir(std::path::PathBuf);

    impl TempDir {
        fn new(name: &str) -> TempDir {
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let dir = std::env::temp_dir().join(format!(
                "oscyl-ffi-test-{name}-{}-{nanos}",
                std::process::id()
            ));
            std::fs::create_dir_all(&dir).unwrap();
            TempDir(dir)
        }

        fn path(&self) -> String {
            self.0.to_string_lossy().into_owned()
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn names(l: &Libraries) -> Vec<String> {
        l.outline().into_iter().map(|l| l.name).collect()
    }

    #[test]
    fn libraries_and_folders_persist() {
        let t = TempDir::new("store");
        let libs = Libraries::open(t.path()).unwrap();
        let name = libs.new_library_name();
        let a = libs.create(name, XD.into(), None).unwrap();
        let f = libs.new_folder(libs.new_folder_name(), None).unwrap();
        let b = libs.create("B".into(), XD.into(), Some(f.clone())).unwrap();
        libs.rename(a.clone(), "Gig".into()).unwrap();
        assert_eq!(names(&libs), ["Gig", "Folder 1", "B"]);
        let doc = libs.read(a.clone()).unwrap();
        doc.new_program(None).unwrap();
        doc.set_name(0, "Lead".into()).unwrap();
        libs.save(a.clone(), &doc).unwrap();
        libs.place(a.clone(), Some(f.clone()), 0).unwrap();
        libs.set_open(f.clone(), false).unwrap();
        let again = Libraries::open(t.path()).unwrap();
        let lines = again.outline();
        assert_eq!(names(&again), ["Folder 1", "Gig", "B"]);
        assert_eq!((lines[0].kind, lines[0].open), (NodeKind::Folder, false));
        assert_eq!((lines[0].first, lines[0].last), (true, true));
        assert_eq!(
            (lines[1].first, lines[1].last, lines[2].last),
            (true, false, true)
        );
        assert_eq!(
            (lines[1].depth, lines[1].shown, lines[1].programs),
            (1, false, Some(1))
        );
        assert_eq!(again.read(a).unwrap().name(0), "Lead");
        let trashed = again.delete(f).unwrap();
        assert!(again.outline().is_empty());
        again.restore(&trashed).unwrap();
        assert_eq!(names(&again), ["Folder 1", "Gig", "B"]);
        assert!(again.restore(&trashed).is_err(), "once only");
        let _ = b;
    }

    #[test]
    fn an_unknown_driver_is_an_error() {
        let t = TempDir::new("driver");
        let libs = Libraries::open(t.path()).unwrap();
        assert!(matches!(
            libs.create("X".into(), "nope".into(), None),
            Err(FfiError::UnknownDriver { .. })
        ));
    }

    #[test]
    fn legacy_libraries_migrate_on_open() {
        let t = TempDir::new("legacy");
        let d = driver(XD).unwrap();
        let bytes = d.export("mnlgxdlib", &d.demo_library()).unwrap();
        std::fs::write(t.0.join("0123456789abcdef.mnlgxdlib"), bytes).unwrap();
        std::fs::write(
            t.0.join("index.json"),
            r#"{"libraries":[{"id":"0123456789abcdef","name":"Old","created":1,"changed":1}]}"#,
        )
        .unwrap();
        let libs = Libraries::open(t.path()).unwrap();
        let lines = libs.outline();
        assert_eq!(names(&libs), ["Old"]);
        assert_eq!(lines[0].driver_id.as_deref(), Some(XD));
        assert!(
            t.0.join("before-oscyllib/0123456789abcdef.mnlgxdlib")
                .is_file()
        );
    }
}
