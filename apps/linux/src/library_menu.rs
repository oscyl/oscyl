//! Oscyl's own libraries in the app: loading them, saving them as they
//! change, new ones, the row menu (rename, duplicate, send to the synth,
//! export, delete with Undo), ordering them, and getting programs in.

use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::rc::Rc;

use adw::prelude::*;
use gtk::{gio, glib};
use oscyl_driver::{Driver, LibProgram, LibraryData};

use crate::app::App;
use crate::files::{Pick, library_filters, text, title_of};
use crate::i18n::tr;
use crate::libraries::{Lib, LineKind, Node, Store, write_atomic};
use crate::sidebar::{Dropped, Place, Spot};
use crate::state::{Doc, Info, Item, Program};

/// Library `i`'s row menu, its actions targeted at `i`. `folders` are
/// the folders it can move to (id and path).
pub fn model(i: usize, folders: &[(String, String)]) -> gio::Menu {
    let target = (i as i32).to_variant();
    let item = |label: String, action: &str| {
        let it = gio::MenuItem::new(Some(&label), None);
        it.set_action_and_target_value(Some(action), Some(&target));
        it
    };
    let menu = gio::Menu::new();
    let edit = gio::Menu::new();
    edit.append_item(&item(tr("Rename"), "win.library-rename"));
    edit.append_item(&item(tr("Duplicate"), "win.library-duplicate"));
    menu.append_section(None, &edit);
    let out = gio::Menu::new();
    out.append_item(&item(tr("Send to Synth…"), "win.library-send"));
    out.append_item(&item(tr("Export…"), "win.library-export"));
    menu.append_section(None, &out);
    let order = gio::Menu::new();
    order.append_item(&item(tr("Move Up"), "win.library-move-up"));
    order.append_item(&item(tr("Move Down"), "win.library-move-down"));
    let to = gio::Menu::new();
    let place = |label: &str, folder: &str| {
        let it = gio::MenuItem::new(Some(label), None);
        it.set_action_and_target_value(
            Some("win.library-to-folder"),
            Some(&(i as i32, folder).to_variant()),
        );
        it
    };
    to.append_item(&place(&tr("Top Level"), ""));
    let inside = gio::Menu::new();
    for (id, path) in folders {
        inside.append_item(&place(path, id));
    }
    to.append_section(None, &inside);
    order.append_submenu(Some(&tr("Move to Folder")), &to);
    menu.append_section(None, &order);
    let delete = gio::Menu::new();
    delete.append_item(&item(tr("Delete"), "win.library-delete"));
    menu.append_section(None, &delete);
    menu
}

/// Folder `key`'s row menu.
pub fn folder_model(key: &str) -> gio::Menu {
    let target = key.to_variant();
    let item = |label: String, action: &str| {
        let it = gio::MenuItem::new(Some(&label), None);
        it.set_action_and_target_value(Some(action), Some(&target));
        it
    };
    let menu = gio::Menu::new();
    let new = gio::Menu::new();
    new.append_item(&item(tr("New Library Here"), "win.folder-new-library"));
    new.append_item(&item(tr("New Folder Here"), "win.folder-new-folder"));
    menu.append_section(None, &new);
    let edit = gio::Menu::new();
    edit.append_item(&item(tr("Rename"), "win.folder-rename"));
    edit.append_item(&item(tr("Move Up"), "win.folder-move-up"));
    edit.append_item(&item(tr("Move Down"), "win.folder-move-down"));
    menu.append_section(None, &edit);
    let delete = gio::Menu::new();
    delete.append_item(&item(tr("Delete"), "win.folder-delete"));
    menu.append_section(None, &delete);
    menu
}

/// Which library is shown, in a way that survives libraries moving.
#[derive(Debug, Clone, PartialEq, Eq)]
enum LibKey {
    /// One of Oscyl's own, by id.
    Id(String),
    /// A file opened from elsewhere, by its place among those.
    File(usize),
}

/// What follows making a library.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Then {
    /// Show it and put its name up for editing.
    Rename,
    Show,
    /// Stay where the person is (a toast offers to open it).
    Stay,
}

/// Where Oscyl keeps its libraries.
fn folder() -> PathBuf {
    glib::user_data_dir().join("oscyl").join("libraries")
}

impl App {
    pub(crate) fn install_library_actions(&self) {
        // Targeted at a library's index; -1 (none) where that means "new".
        let add = |name: &str, run: fn(&App, Option<usize>)| {
            let action = gio::SimpleAction::new(name, Some(glib::VariantTy::INT32));
            let run = self.weak(move |me, i: i32| run(me, usize::try_from(i).ok()));
            action.connect_activate(move |_, v| {
                if let Some(i) = v.and_then(|v| v.get::<i32>()) {
                    run(i);
                }
            });
            self.win.add_action(&action);
        };
        add("library-rename", |me, i| {
            if let Some(i) = i {
                me.sidebar.start_rename(i);
            }
        });
        add("library-duplicate", |me, i| {
            if let Some(i) = i {
                me.duplicate_library(i);
            }
        });
        add("library-send", |me, i| {
            if let Some(i) = i {
                me.send_library(i);
            }
        });
        add("library-export", |me, i| {
            if let Some(i) = i {
                me.export_library(i);
            }
        });
        add("library-delete", |me, i| {
            if let Some(i) = i {
                me.delete_library(i);
            }
        });
        add("library-move-up", |me, i| {
            if let Some(id) = i.and_then(|i| me.libs.borrow().get(i)?.id.clone()) {
                me.step_node(&id, -1);
            }
        });
        add("library-move-down", |me, i| {
            if let Some(id) = i.and_then(|i| me.libs.borrow().get(i)?.id.clone()) {
                me.step_node(&id, 1);
            }
        });
        // Targeted at a folder's id; "" (none) where that means the top
        // level.
        let folder = |name: &str, run: fn(&App, Option<String>)| {
            let action = gio::SimpleAction::new(name, Some(glib::VariantTy::STRING));
            let run = self.weak(move |me, key: String| {
                run(me, (!key.is_empty()).then_some(key));
            });
            action.connect_activate(move |_, v| {
                if let Some(key) = v.and_then(|v| v.get::<String>()) {
                    run(key);
                }
            });
            self.win.add_action(&action);
        };
        folder("folder-new-library", |me, key| {
            let d = me.new_library_driver();
            me.create_library(
                &me.next_name(),
                d,
                LibraryData::default(),
                Then::Rename,
                key,
            );
        });
        folder("folder-new-folder", App::new_folder);
        folder("folder-rename", |me, key| {
            if let Some(key) = key {
                me.sidebar.start_rename_key(&key);
            }
        });
        folder("folder-move-up", |me, key| {
            if let Some(key) = key {
                me.step_node(&key, -1);
            }
        });
        folder("folder-move-down", |me, key| {
            if let Some(key) = key {
                me.step_node(&key, 1);
            }
        });
        folder("folder-delete", |me, key| {
            if let Some(key) = key {
                me.delete_node(&key);
            }
        });
        let to = gio::SimpleAction::new(
            "library-to-folder",
            Some(glib::VariantTy::new("(is)").expect("valid type")),
        );
        let run = self.weak(|me, (i, folder): (i32, String)| {
            let Some(id) = usize::try_from(i)
                .ok()
                .and_then(|i| me.libs.borrow().get(i)?.id.clone())
            else {
                return;
            };
            me.place_node(&id, (!folder.is_empty()).then_some(folder), usize::MAX);
        });
        to.connect_activate(move |_, v| {
            if let Some(t) = v.and_then(|v| v.get::<(i32, String)>()) {
                run(t);
            }
        });
        self.win.add_action(&to);
        add("library-import", App::import_library);
        add("keep-in-library", App::keep_in_library);
        for (name, run) in [
            ("library-new", App::new_library as fn(&App)),
            ("library-from-synth", App::library_from_synth),
        ] {
            let action = gio::SimpleAction::new(name, None);
            let run = self.weak(move |me, ()| run(me));
            action.connect_activate(move |_, _| run(()));
            self.win.add_action(&action);
        }
    }

    /// Enable the row actions that make sense for library `i` now.
    pub(crate) fn update_library_actions(&self, i: usize) {
        let (readable, len, id) = self.libs.borrow().get(i).map_or((false, 0, None), |l| {
            (l.broken.is_none(), l.doc.len(), l.id.clone())
        });
        let (at, siblings) = id
            .and_then(|id| self.store.borrow().as_ref()?.siblings(&id))
            .unwrap_or((0, 0));
        let set = |name: &str, on: bool| {
            if let Some(a) = self
                .win
                .lookup_action(name)
                .and_downcast::<gio::SimpleAction>()
            {
                a.set_enabled(on);
            }
        };
        set("library-duplicate", readable);
        set(
            "library-send",
            readable && len > 0 && self.connected() && self.synth_fully_read(),
        );
        set("library-move-up", at > 0);
        set("library-move-down", at + 1 < siblings);
    }

    /// How many of the sidebar's libraries are Oscyl's own (they come
    /// first).
    fn managed_count(&self) -> usize {
        self.libs
            .borrow()
            .iter()
            .take_while(|l| l.is_managed())
            .count()
    }

    /// Read Oscyl's libraries into the sidebar. A library that cannot be
    /// read is listed, marked, and left as it is.
    pub(crate) fn load_libraries(&self) {
        let store = match Store::open(folder()) {
            Ok(s) => s,
            Err(e) => {
                self.win
                    .toast(&tr("Couldn't open your libraries: {error}").replace("{error}", &e));
                return;
            }
        };
        let libs = store.entries().iter().map(|m| {
            let doc = store
                .read(&m.id)
                .and_then(|(d, lib)| Doc::from_data(d, lib, None, &m.name).map_err(text));
            match doc {
                Ok(doc) => Lib::managed(doc, m.id.clone()),
                Err(e) => {
                    // Kept for its file (export, delete) under whatever
                    // synth it names, or the first one Oscyl knows.
                    let d = oscyl_midi::driver(&m.driver).unwrap_or(oscyl_midi::DRIVERS[0]);
                    let empty = Doc::synth(d, Vec::new(), &m.name);
                    Lib {
                        broken: Some(e),
                        ..Lib::managed(empty, m.id.clone())
                    }
                }
            }
        });
        self.libs.borrow_mut().splice(0..0, libs);
        *self.store.borrow_mut() = Some(store);
        self.refresh_sidebar();
    }

    /// Write every library of Oscyl's whose programs changed since last
    /// written. A failure is said once and marks the library unsaved; the
    /// next change tries again.
    pub(crate) fn autosave(&self) {
        let mut flipped = false;
        let mut failures = Vec::new();
        {
            let mut store = self.store.borrow_mut();
            let Some(store) = store.as_mut() else { return };
            for lib in self.libs.borrow_mut().iter_mut() {
                let Some(id) = &lib.id else { continue };
                let rev = lib.doc.revision();
                if lib.broken.is_some() || rev == lib.saved {
                    continue;
                }
                match store.save(id, &lib.doc.to_data()) {
                    Ok(()) => {
                        lib.saved = rev;
                        flipped |= std::mem::replace(&mut lib.failed, false);
                    }
                    Err(e) => {
                        if !std::mem::replace(&mut lib.failed, true) {
                            flipped = true;
                            failures.push((lib.doc.title.clone(), e));
                        }
                    }
                }
            }
        }
        for (name, e) in failures {
            self.win.toast(
                &tr("Couldn't save “{name}”: {error}")
                    .replace("{name}", &name)
                    .replace("{error}", &e),
            );
        }
        if flipped {
            self.refresh_sidebar();
        }
    }

    /// The app is quitting: deleted libraries go for good.
    pub(crate) fn quit_libraries(&self) {
        if let Some(s) = self.store.borrow().as_ref() {
            s.purge_all();
        }
        // The synths' work ends with the app.
        self.synth.send(crate::synth::Cmd::Quit);
    }

    /// "Library 1", "Library 2"…: the first not taken. (Without the
    /// libraries folder nothing can be made, so the name doesn't matter.)
    fn next_name(&self) -> String {
        self.store
            .borrow()
            .as_ref()
            .map(|s| s.new_library_name())
            .unwrap_or_default()
    }

    /// Libraries moved in the sidebar: point what is shown at its new
    /// index (`None`: it is gone, show the synth), and redraw the rows.
    fn relocate(&self, map: impl Fn(usize) -> Option<usize>) {
        let mut gone = false;
        for cell in [&self.place, &self.doc_place] {
            let mut p = cell.borrow_mut();
            if let Place::Library(k) = *p {
                match map(k) {
                    Some(n) => *p = Place::Library(n),
                    None => {
                        *p = Place::Synth;
                        gone = true;
                    }
                }
            }
        }
        self.refresh_sidebar();
        if gone {
            self.sidebar.select(&Place::Synth);
        } else {
            let place = self.place.borrow().clone();
            self.sidebar.show_selected(&place);
        }
    }

    fn lib_key(&self, k: usize) -> Option<LibKey> {
        let libs = self.libs.borrow();
        let l = libs.get(k)?;
        Some(match &l.id {
            Some(id) => LibKey::Id(id.clone()),
            None => LibKey::File(k - libs.iter().filter(|l| l.is_managed()).count()),
        })
    }

    fn find_lib(&self, key: &LibKey) -> Option<usize> {
        let libs = self.libs.borrow();
        match key {
            LibKey::Id(id) => libs.iter().position(|l| l.id.as_deref() == Some(id)),
            LibKey::File(n) => {
                let k = libs.iter().filter(|l| l.is_managed()).count() + n;
                (k < libs.len()).then_some(k)
            }
        }
    }

    /// Change the sidebar's libraries with `change` (which may add some
    /// or take some away), put Oscyl's own in the tree's order, and keep
    /// what is shown pointing at the same library (the synth when it has
    /// gone).
    fn arrange(&self, change: impl FnOnce(&mut Vec<Lib>)) {
        let keys = [&self.place, &self.doc_place].map(|cell| match *cell.borrow() {
            Place::Library(k) => self.lib_key(k),
            _ => None,
        });
        {
            let order: std::collections::HashMap<String, usize> = self
                .store
                .borrow()
                .as_ref()
                .map(|s| {
                    s.entries()
                        .iter()
                        .enumerate()
                        .map(|(n, m)| (m.id.clone(), n))
                        .collect()
                })
                .unwrap_or_default();
            let mut libs = self.libs.borrow_mut();
            change(&mut libs);
            let (mut own, files): (Vec<Lib>, Vec<Lib>) = libs.drain(..).partition(Lib::is_managed);
            own.sort_by_key(|l| l.id.as_ref().and_then(|id| order.get(id)).copied());
            libs.extend(own);
            libs.extend(files);
        }
        let mut gone = false;
        for (cell, key) in [&self.place, &self.doc_place].into_iter().zip(keys) {
            let Some(key) = key else { continue };
            match self.find_lib(&key) {
                Some(k) => *cell.borrow_mut() = Place::Library(k),
                None => {
                    *cell.borrow_mut() = Place::Synth;
                    gone = true;
                }
            }
        }
        self.refresh_sidebar();
        if gone {
            self.sidebar.select(&Place::Synth);
        } else {
            let place = self.place.borrow().clone();
            self.sidebar.show_selected(&place);
        }
    }

    /// The index of the library with `id`.
    fn library_with(&self, id: &str) -> Option<usize> {
        self.find_lib(&LibKey::Id(id.to_string()))
    }

    /// Make sure folder `key` shows its contents.
    fn open_folder(&self, key: Option<&str>) {
        if let (Some(key), Some(s)) = (key, self.store.borrow_mut().as_mut()) {
            let _ = s.set_open(key, true);
        }
    }

    /// Add a library of Oscyl's named `name` holding `lib`, last in folder
    /// `parent` (`None`: the top level).
    fn create_library(
        &self,
        name: &str,
        driver: &'static dyn Driver,
        lib: LibraryData,
        then: Then,
        parent: Option<String>,
    ) -> Option<usize> {
        let doc = match Doc::from_data(driver, lib.clone(), None, name) {
            Ok(d) => d,
            Err(e) => {
                self.error_dialog(&tr("Couldn't make the library"), &text(e));
                return None;
            }
        };
        let created = match self.store.borrow_mut().as_mut() {
            Some(s) => s.create(name, driver, &lib, parent.as_deref()),
            None => Err(tr("Your libraries folder couldn't be opened.")),
        };
        let id = match created {
            Ok(id) => id,
            Err(e) => {
                self.error_dialog(&tr("Couldn't make the library"), &e);
                return None;
            }
        };
        if then != Then::Stay {
            self.open_folder(parent.as_deref());
        }
        let lib = Lib::managed(doc, id.clone());
        self.arrange(move |libs| libs.push(lib));
        let at = self.library_with(&id)?;
        if then != Then::Stay {
            self.current.set(None);
            self.sidebar.select(&Place::Library(at));
        }
        if then == Then::Rename
            && (self.win.outer.shows_sidebar() || !self.win.outer.is_collapsed())
        {
            self.sidebar.start_rename(at);
        }
        Some(at)
    }

    pub(crate) fn new_library(&self) {
        let d = self.new_library_driver();
        self.create_library(
            &self.next_name(),
            d,
            LibraryData::default(),
            Then::Rename,
            None,
        );
    }

    /// The synth a new empty library is for: the synth shown in the
    /// sidebar, else the model of the document shown.
    fn new_library_driver(&self) -> &'static dyn Driver {
        self.active_driver().unwrap_or_else(|| self.doc_driver())
    }

    /// A new folder, last in folder `parent` (`None`: the top level),
    /// named "Folder 1", "Folder 2"… and put up for renaming.
    fn new_folder(&self, parent: Option<String>) {
        let name = self
            .store
            .borrow()
            .as_ref()
            .map(|s| s.new_folder_name())
            .unwrap_or_default();
        self.open_folder(parent.as_deref());
        let made = match self.store.borrow_mut().as_mut() {
            Some(s) => s.new_folder(&name, parent.as_deref()),
            None => Err(tr("Your libraries folder couldn't be opened.")),
        };
        match made {
            Ok(id) => {
                self.refresh_sidebar();
                if self.win.outer.shows_sidebar() || !self.win.outer.is_collapsed() {
                    self.sidebar.start_rename_key(&id);
                }
            }
            Err(e) => self.error_dialog(&tr("Couldn't make the folder"), &e),
        }
    }

    /// The folders a library can move to: id and path ("Live / Ambient").
    pub(crate) fn folder_paths(&self) -> Vec<(String, String)> {
        let Some(store) = self.store.borrow().as_ref().map(Store::outline) else {
            return Vec::new();
        };
        let mut path: Vec<String> = Vec::new();
        let mut out = Vec::new();
        for line in store {
            path.truncate(line.depth);
            if let LineKind::Folder { id, name, .. } = line.kind {
                path.push(name);
                out.push((id, path.join(" / ")));
            }
        }
        out
    }

    /// A folder opened or closed (remembered for the next run).
    pub(crate) fn toggle_folder(&self, key: &str, open: bool) {
        let set = self
            .store
            .borrow_mut()
            .as_mut()
            .map(|s| s.set_open(key, open));
        if let Some(Err(e)) = set {
            self.win
                .toast(&tr("Couldn't save the libraries' order: {error}").replace("{error}", &e));
        }
        self.refresh_sidebar();
    }

    /// Put node `key` (a library or folder) in folder `parent` (`None`:
    /// the top level) before what is now at `at` there.
    fn place_node(&self, key: &str, parent: Option<String>, at: usize) {
        let placed = match self.store.borrow_mut().as_mut() {
            Some(s) => s.place(key, parent.as_deref(), at),
            None => return,
        };
        match placed {
            Ok(()) => {
                self.arrange(|_| {});
                self.sidebar.focus_key(key);
            }
            Err(_) => self.win.window.error_bell(),
        }
    }

    /// Something was dropped on the libraries tree.
    pub(crate) fn dropped_on_tree(&self, what: Dropped, spot: Spot) {
        match (what, spot) {
            (Dropped::Program(i), Spot::Into(Some(id))) => {
                if let Some(j) = self.library_with(&id) {
                    self.copy_to_library(i, Some(j));
                }
            }
            (Dropped::Program(_), _) => {}
            (Dropped::Node(key), Spot::Into(folder)) => self.place_node(&key, folder, usize::MAX),
            (Dropped::Node(key), spot) => {
                let (next, after) = match spot {
                    Spot::Before(next) => (next, 0),
                    Spot::After(next) => (next, 1),
                    Spot::Into(_) => unreachable!("matched above"),
                };
                let Some((parent, at)) = self.store.borrow().as_ref().and_then(|s| s.locate(&next))
                else {
                    return;
                };
                self.place_node(&key, parent, at + after);
            }
        }
    }

    /// A new library holding every program of the synth as the list shows
    /// them now.
    fn library_from_synth(&self) {
        if !self.synth_fully_read() {
            self.win.toast(&tr(
                "Wait until all programs have been read from the synth.",
            ));
            return;
        }
        let Some((driver, lib)) = self.with_synth(|s| {
            let d = &s.doc;
            // Writable slots only: a library goes back to these.
            let n = d.len().min(d.max_programs());
            let lib = LibraryData {
                programs: (0..n)
                    .filter_map(|i| d.work(i))
                    .map(|p| LibProgram {
                        data: p.to_vec(),
                        ..LibProgram::default()
                    })
                    .collect(),
                ..LibraryData::default()
            };
            (d.driver, lib)
        }) else {
            return;
        };
        self.create_library(&self.next_name(), driver, lib, Then::Rename, None);
    }

    /// The program synth `id` plays (Save Playing Sound), kept as a new
    /// library and shown.
    pub(crate) fn keep_playing(&self, id: crate::synth::DeviceId, data: Vec<u8>) {
        let Some((driver, model)) = self.with_synth_id(id, |s| (s.doc.driver, s.name.clone()))
        else {
            return;
        };
        let lib = LibraryData {
            programs: vec![LibProgram {
                data,
                ..LibProgram::default()
            }],
            ..LibraryData::default()
        };
        let name = tr("Current program ({model})").replace("{model}", &model);
        self.create_library(&name, driver, lib, Then::Show, None);
    }

    /// A library or folder was renamed in the sidebar.
    pub(crate) fn rename_node(&self, key: &str, name: &str) {
        match self.library_with(key) {
            Some(i) => self.rename_library(i, name),
            None => {
                let renamed = match self.store.borrow_mut().as_mut() {
                    Some(s) => s.rename(key, name),
                    None => return,
                };
                match renamed {
                    Ok(()) => self.refresh_sidebar(),
                    Err(e) => self.error_dialog(&tr("Couldn't rename the folder"), &e),
                }
            }
        }
    }

    pub(crate) fn rename_library(&self, i: usize, name: &str) {
        let Some(id) = self.libs.borrow().get(i).and_then(|l| l.id.clone()) else {
            return;
        };
        let renamed = match self.store.borrow_mut().as_mut() {
            Some(s) => s.rename(&id, name),
            None => return,
        };
        match renamed {
            Ok(()) => {
                self.libs.borrow_mut()[i].doc.title = name.to_string();
                if *self.doc_place.borrow() == Place::Library(i) {
                    self.win.list_page.set_title(name);
                }
                self.refresh_sidebar();
            }
            Err(e) => self.error_dialog(&tr("Couldn't rename the library"), &e),
        }
    }

    fn duplicate_library(&self, i: usize) {
        self.autosave();
        let Some((id, title)) = self
            .libs
            .borrow()
            .get(i)
            .filter(|l| l.broken.is_none())
            .and_then(|l| Some((l.id.clone()?, l.doc.title.clone())))
        else {
            return;
        };
        let name = tr("{name} copy").replace("{name}", &title);
        let copied = {
            let mut store = self.store.borrow_mut();
            let Some(s) = store.as_mut() else { return };
            s.duplicate(&id, &name)
                .and_then(|new| Ok((s.read(&new)?, new)))
        };
        let doc = copied.and_then(|((d, lib), new)| {
            Ok((Doc::from_data(d, lib, None, &name).map_err(text)?, new))
        });
        match doc {
            Ok((doc, new)) => {
                let lib = Lib::managed(doc, new.clone());
                self.arrange(move |libs| libs.push(lib));
                if let Some(k) = self.library_with(&new) {
                    self.current.set(None);
                    self.sidebar.select(&Place::Library(k));
                }
            }
            Err(e) => self.error_dialog(&tr("Couldn't duplicate the library"), &e),
        }
    }

    /// Move a library or folder one place up (-1) or down (1) in its
    /// folder; at either end, the bell.
    fn step_node(&self, key: &str, by: isize) {
        let stepped = match self.store.borrow_mut().as_mut() {
            Some(s) => s.step(key, by),
            None => return,
        };
        match stepped {
            Ok(true) => {
                self.arrange(|_| {});
                self.sidebar.focus_key(key);
            }
            Ok(false) => self.win.window.error_bell(),
            Err(e) => self.error_dialog(&tr("Couldn't move the library"), &e),
        }
    }

    fn delete_library(&self, i: usize) {
        if let Some(id) = self.libs.borrow().get(i).and_then(|l| l.id.clone()) {
            self.delete_node(&id);
        }
    }

    /// Delete a library, or a folder with everything inside; the files
    /// wait in the trash while the toast offers Undo, which brings it all
    /// back where it was.
    fn delete_node(&self, key: &str) {
        let name = self
            .store
            .borrow()
            .as_ref()
            .and_then(|s| {
                s.outline().into_iter().find_map(|l| match l.kind {
                    LineKind::Folder { id, name, .. } if id == key => Some(name),
                    LineKind::Library(id) if id == key => s
                        .entries()
                        .iter()
                        .find(|m| m.id == id)
                        .map(|m| m.name.clone()),
                    _ => None,
                })
            })
            .unwrap_or_default();
        let trashed = match self.store.borrow_mut().as_mut() {
            Some(s) => s.delete(key),
            None => return,
        };
        let trashed = match trashed {
            Ok(t) => t,
            Err(e) => {
                self.error_dialog(&tr("Couldn't delete the library"), &e);
                return;
            }
        };
        // The libraries inside wait with their documents (and undo).
        let mut taken = Vec::new();
        self.arrange(|libs| {
            let (gone, kept): (Vec<Lib>, Vec<Lib>) = libs.drain(..).partition(|l| {
                l.id.as_ref()
                    .is_some_and(|id| trashed.metas.iter().any(|m| m.id == *id))
            });
            *libs = kept;
            taken = gone;
        });
        let toast = adw::Toast::builder()
            .title(tr("Deleted “{name}”").replace("{name}", &name))
            .button_label(tr("Undo"))
            .build();
        let restored = Rc::new(Cell::new(false));
        let kept = RefCell::new(Some((taken, trashed.clone())));
        {
            let restored = restored.clone();
            let undo = self.weak(move |me, ()| {
                let Some((libs, t)) = kept.borrow_mut().take() else {
                    return;
                };
                let key = t.node.id().to_string();
                let folder = matches!(t.node, Node::Folder(_));
                let back = match me.store.borrow_mut().as_mut() {
                    Some(s) => s.restore(t),
                    None => return,
                };
                if let Err(e) = back {
                    me.error_dialog(&tr("Couldn't restore the library"), &e);
                    return;
                }
                restored.set(true);
                me.arrange(move |all| all.extend(libs));
                if folder {
                    me.sidebar.focus_key(&key);
                } else if let Some(k) = me.library_with(&key) {
                    me.current.set(None);
                    me.sidebar.select(&Place::Library(k));
                }
            });
            toast.connect_button_clicked(move |_| undo(()));
        }
        let purge = self.weak(move |me, ()| {
            if !restored.get()
                && let Some(s) = me.store.borrow().as_ref()
            {
                s.purge(&trashed);
            }
        });
        toast.connect_dismissed(move |_| purge(()));
        self.win.show_toast(toast);
    }

    pub(crate) fn export_library(&self, i: usize) {
        let Some((title, broken)) = self
            .libs
            .borrow()
            .get(i)
            .map(|l| (l.doc.title.clone(), l.broken.is_some()))
        else {
            return;
        };
        let d = self.libs.borrow()[i].doc.driver;
        self.pick_file(
            &tr("Export Library"),
            &library_filters(Some(d)),
            Pick::Save(&export_name(d, &title, broken)),
            move |me, path| {
                // A library that couldn't be read goes out as its file is.
                let written = {
                    let libs = me.libs.borrow();
                    let Some(l) = libs.get(i) else { return };
                    match (&l.broken, &l.id, me.store.borrow().as_ref()) {
                        (Some(_), Some(id), Some(s)) => {
                            std::fs::read(s.path(id)).map_err(text).and_then(|b| {
                                write_atomic(&path, |w| {
                                    std::io::Write::write_all(w, &b).map_err(text)
                                })
                            })
                        }
                        _ => crate::files::write_file(&path, l.doc.driver, &l.doc.to_data()),
                    }
                };
                match written {
                    Ok(()) => me.win.toast(&tr("Library exported")),
                    Err(e) => me.error_dialog(&tr("Couldn't export the library"), &e),
                }
            },
        );
    }

    /// Import a library or program file: as a new library, or (`into`)
    /// added to the end of that one.
    fn import_library(&self, into: Option<usize>) {
        let only = into.and_then(|j| self.libs.borrow().get(j).map(|l| l.doc.driver));
        self.pick_file(
            &tr("Import File"),
            &library_filters(only),
            Pick::Open,
            move |me, path| {
                let (d, lib) = match crate::files::read_file(&path) {
                    Ok(l) => l,
                    Err(e) => {
                        me.error_dialog(&tr("Couldn't open the file"), &e);
                        return;
                    }
                };
                if let Err(e) = oscyl_app::files::fits(d, &lib) {
                    me.error_dialog(&tr("Too many programs"), &e);
                    return;
                }
                match into {
                    Some(j) => {
                        if only.is_some_and(|o| o.info().id != d.info().id) {
                            me.error_dialog(
                                &tr("Couldn't open the file"),
                                &tr("It holds programs for another synth ({model}).")
                                    .replace("{model}", d.info().model),
                            );
                            return;
                        }
                        let items = lib
                            .programs
                            .into_iter()
                            .map(|p| {
                                let info = Info {
                                    author: p.author,
                                    comment: p.comment,
                                };
                                Item::new(p.data, info)
                            })
                            .collect();
                        me.add_programs(j, items);
                    }
                    None => {
                        me.create_library(&title_of(&path), d, lib, Then::Show, None);
                    }
                }
            },
        );
    }

    /// Add `items` at the end of library `j` as one undo step there.
    fn add_programs(&self, j: usize, items: Vec<Item>) -> bool {
        let Some((added, name)) = self
            .libs
            .borrow_mut()
            .get_mut(j)
            .map(|l| (l.doc.insert(l.doc.len(), items), l.doc.title.clone()))
        else {
            return false;
        };
        if !added {
            let max = self.libs.borrow()[j].doc.max_programs();
            self.win.toast(
                &tr("“{name}” is full: a library holds up to {max} programs.")
                    .replace("{name}", &name)
                    .replace("{max}", &max.to_string()),
            );
            return false;
        }
        if *self.doc_place.borrow() == Place::Library(j) {
            if *self.place.borrow() == Place::Library(j) {
                self.go(Place::Library(j));
            } else {
                self.after_edit();
            }
        } else {
            self.autosave();
            self.refresh_sidebar();
        }
        true
    }

    /// Copy program `i` of the list shown to the end of library `j` (or
    /// a new one), saying where with a way to open it.
    pub(crate) fn copy_to_library(&self, i: usize, j: Option<usize>) {
        if !self.is_loaded(i) {
            return;
        }
        let Some((d, p, info)) = self
            .with_doc(|d| {
                Some((
                    d.driver,
                    d.work(i)?.to_vec(),
                    d.infos.get(i).cloned().unwrap_or_default(),
                ))
            })
            .flatten()
        else {
            return;
        };
        self.put_in_library(j, d, p, info);
    }

    /// Generate's Keep in Library.
    fn keep_in_library(&self, j: Option<usize>) {
        if let Some((d, p)) = self.generator.get().and_then(|g| g.selected_program()) {
            self.warn_about_units(d, &p, false);
            self.put_in_library(j, d, p, Info::default());
        }
    }

    /// Library `j` (a new one when `None`) gets program `p` of synth `d`;
    /// a library of another synth refuses it.
    fn put_in_library(&self, j: Option<usize>, d: &'static dyn Driver, p: Program, info: Info) {
        let j = match j {
            Some(j) => {
                let other = self
                    .libs
                    .borrow()
                    .get(j)
                    .is_some_and(|l| l.doc.driver.info().id != d.info().id);
                if other {
                    self.win.toast(&tr("That library is for another synth."));
                    return;
                }
                if !self.add_programs(j, vec![Item::new(p, info)]) {
                    return;
                }
                j
            }
            None => {
                let lib = LibraryData {
                    programs: vec![LibProgram {
                        data: p,
                        author: info.author,
                        comment: info.comment,
                    }],
                    ..LibraryData::default()
                };
                let Some(at) = self.create_library(&self.next_name(), d, lib, Then::Stay, None)
                else {
                    return;
                };
                at
            }
        };
        let Some((id, name)) = self
            .libs
            .borrow()
            .get(j)
            .and_then(|l| Some((l.id.clone()?, l.doc.title.clone())))
        else {
            return;
        };
        let toast = adw::Toast::builder()
            .title(tr("Copied to “{name}”").replace("{name}", &name))
            .button_label(tr("Open"))
            .build();
        let open = self.weak(move |me, ()| {
            let found = me
                .libs
                .borrow()
                .iter()
                .position(|l| l.id.as_deref() == Some(id.as_str()));
            if let Some(k) = found {
                me.sidebar.select(&Place::Library(k));
                let n = me.with_doc(|d| d.len()).unwrap_or(0);
                if n > 0 {
                    me.show_program(n - 1);
                    me.win.inner.set_show_content(true);
                }
            }
        });
        toast.connect_button_clicked(move |_| open(()));
        self.win.show_toast(toast);
    }

    /// Ctrl+Shift+L: the libraries the selected program can be copied to,
    /// as a menu at its row.
    pub(crate) fn copy_to_library_menu(&self) {
        let Some(i) = self.current.get().filter(|&i| self.is_loaded(i)) else {
            return;
        };
        self.update_program_actions(i);
        let menu = crate::program_menu::library_targets(
            &self.copy_targets(self.managed_place()),
            "win.program-to-library",
            |j| (i as i32, j).to_variant(),
        );
        let at = self
            .list
            .row_of(i)
            .unwrap_or_else(|| self.editor.header_title.clone().upcast());
        crate::programs::popup(&at, &menu, None);
    }

    /// Turn a library with no file yet (the demo) into one of Oscyl's.
    pub(crate) fn keep_as_library(&self, i: usize) {
        let Some((name, d, lib)) = self
            .libs
            .borrow()
            .get(i)
            .filter(|l| !l.is_managed())
            .map(|l| (l.doc.title.clone(), l.doc.driver, l.doc.to_data()))
        else {
            return;
        };
        let created = match self.store.borrow_mut().as_mut() {
            Some(s) => s.create(&name, d, &lib, None),
            None => Err(tr("Your libraries folder couldn't be opened.")),
        };
        let id = match created {
            Ok(id) => id,
            Err(e) => {
                self.error_dialog(&tr("Couldn't make the library"), &e);
                return;
            }
        };
        let m = self.managed_count();
        {
            let mut libs = self.libs.borrow_mut();
            let mut lib = libs.remove(i);
            lib.saved = lib.doc.revision();
            lib.id = Some(id);
            libs.insert(m, lib);
        }
        self.relocate(|k| {
            Some(match k {
                k if k == i => m,
                k if k >= m && k < i => k + 1,
                k => k,
            })
        });
        self.update_actions();
        self.win
            .toast(&tr("Kept “{name}” in your libraries").replace("{name}", &name));
    }

    /// Send library `i` to the synth: program n becomes slot n's edit
    /// (one undo step), then the usual Sync confirmation; cancelling it
    /// takes the step back.
    pub(crate) fn send_library(&self, i: usize) {
        let Some(d) = self.libs.borrow().get(i).map(|l| l.doc.driver) else {
            return;
        };
        let Some(target) = self.target_synth(d) else {
            self.win.toast(
                &tr("Connect a {model} to send this library to it.")
                    .replace("{model}", d.info().model),
            );
            return;
        };
        // The library syncs with this synth: show it from now on.
        self.set_active(target);
        if !self.synth_fully_read() {
            self.win.toast(&tr(
                "Wait until all programs have been read from the synth.",
            ));
            return;
        }
        let programs: Vec<Program> = {
            let libs = self.libs.borrow();
            let Some(l) = libs.get(i).filter(|l| l.broken.is_none()) else {
                return;
            };
            (0..l.doc.len().min(l.doc.max_programs()))
                .filter_map(|n| l.doc.work(n).map(<[u8]>::to_vec))
                .collect()
        };
        let staged = self.with_synth(|s| s.doc.stage(&programs)).unwrap_or(0);
        if staged == 0 {
            self.win
                .toast(&tr("The synth already holds these programs."));
            return;
        }
        self.confirm_sync(true);
    }

    /// An empty library: what to do to fill it.
    pub(crate) fn show_empty_library(&self, i: usize) {
        let page = adw::StatusPage::builder()
            .paintable(&crate::widgets::LineArt::new(crate::widgets::Art::Shelf))
            .css_classes(["line-art"])
            .title(tr("No programs yet"))
            .description(tr(
                "Start a new program (Ctrl+N), copy programs here from the synth or another library with Copy to Library in a program's menu (Ctrl+Shift+L), or keep sounds from Generate.",
            ))
            .build();
        let import = gtk::Button::builder()
            .label(tr("Import File…"))
            .css_classes(["pill"])
            .action_name("win.library-import")
            .action_target(&(i as i32).to_variant())
            .build();
        let new = gtk::Button::builder()
            .label(tr("New Program"))
            .css_classes(["pill"])
            .action_name("win.new-program")
            .build();
        let buttons = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        buttons.set_halign(gtk::Align::Center);
        buttons.append(&new);
        buttons.append(&import);
        page.set_child(Some(&buttons));
        self.win
            .editor_page
            .set_title(&self.with_doc(|d| d.title.clone()).unwrap_or_default());
        self.editor.show_page("library-empty", &page);
    }

    /// Screenshots: two libraries made from the demo's sounds, the first
    /// one shown.
    pub(crate) fn demo_libraries(&self) {
        let d = self.doc_driver();
        let demo = d.demo_library().programs;
        let mut first = None;
        // A set of mixed sounds (as a gig set is), and a run of pads.
        let sets: [(String, Vec<usize>); 2] = [
            (tr("Gig set"), (0..14).map(|k| k * 8 + 1).collect()),
            (tr("Pads and drones"), (40..52).collect()),
        ];
        for (name, picks) in sets {
            let lib = LibraryData {
                programs: picks.iter().filter_map(|&k| demo.get(k).cloned()).collect(),
                ..LibraryData::default()
            };
            let at = self.create_library(&name, d, lib, Then::Show, None);
            first = first.or(at);
        }
        if let Some(i) = first {
            self.sidebar.select(&Place::Library(i));
            self.show_program(2);
        }
    }

    /// Screenshots, after [`App::demo_libraries`]: the libraries in
    /// folders, one inside another and closed, with the first library
    /// shown.
    pub(crate) fn demo_folders(&self) {
        let ids: Vec<String> = self
            .libs
            .borrow()
            .iter()
            .filter_map(|l| l.id.clone())
            .collect();
        let d = self.doc_driver();
        let sketch =
            self.create_library(&tr("Sketches"), d, LibraryData::default(), Then::Stay, None);
        {
            let mut store = self.store.borrow_mut();
            let Some(s) = store.as_mut() else { return };
            let (Ok(live), Some(gig), Some(pads)) =
                (s.new_folder(&tr("Live"), None), ids.first(), ids.get(1))
            else {
                return;
            };
            let Ok(encore) = s.new_folder(&tr("Encore"), Some(&live)) else {
                return;
            };
            let _ = s.place(gig, Some(&live), 0);
            let _ = s.place(pads, Some(&encore), 0);
            let _ = s.set_open(&encore, false);
            let _ = s.new_folder(&tr("Studio"), None);
        }
        let _ = sketch;
        self.arrange(|_| {});
    }

    /// Screenshots, after [`App::demo_folders`]: a library held between
    /// rows (`line`) or over the closed folder.
    pub(crate) fn demo_drop(&self, line: bool) {
        let outline = self
            .store
            .borrow()
            .as_ref()
            .map(Store::outline)
            .unwrap_or_default();
        let folder = |n: usize| {
            outline
                .iter()
                .filter_map(|l| match &l.kind {
                    LineKind::Folder { id, .. } => Some(id.clone()),
                    LineKind::Library(_) => None,
                })
                .nth(n)
        };
        let spot = if line {
            folder(2).map(Spot::Before)
        } else {
            folder(1).map(|k| Spot::Into(Some(k)))
        };
        if let Some(spot) = spot {
            self.sidebar.demo_mark(spot);
        }
    }
}

/// The suggested file name for exporting a library: in its synth's own
/// format. One Oscyl could not read goes out as its stored file, so it
/// keeps the native extension.
fn export_name(d: &dyn Driver, title: &str, broken: bool) -> String {
    format!("{title}.{}", crate::files::export_ext(d, broken))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_broken_library_exports_under_its_own_format() {
        let xd = oscyl_midi::driver("korg-minilogue-xd").unwrap();
        let rev2 = oscyl_midi::driver("sequential-prophet-rev2").unwrap();
        assert_eq!(export_name(xd, "Live", false), "Live.mnlgxdlib");
        assert_eq!(export_name(xd, "Live", true), "Live.oscyllib");
        assert_eq!(export_name(rev2, "Live", false), "Live.syx");
    }
}
