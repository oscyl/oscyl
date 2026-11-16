//! Library files and backups: open, save, back up the synth, and the
//! backup taken before every sync.

use std::fmt::Display;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use adw::prelude::*;
use gtk::{gio, glib};
use oscyl_core::native;
use oscyl_driver::{Driver, LibraryData};
use oscyl_midi::DRIVERS;

use crate::app::App;
use crate::i18n::{tr, trn};
use crate::libraries::write_atomic;
use crate::sidebar::Place;
use crate::state::Doc;
use crate::synth::{Cmd, DeviceId};

/// Errors reach the person as text.
pub fn text(e: impl Display) -> String {
    e.to_string()
}

/// A file filter named `name` matching `patterns`.
pub fn filters(name: &str, patterns: &[&str]) -> gio::ListStore {
    let f = gtk::FileFilter::new();
    f.set_name(Some(name));
    for p in patterns {
        f.add_pattern(p);
    }
    let store = gio::ListStore::new::<gtk::FileFilter>();
    store.append(&f);
    store
}

/// Oscyl's libraries and every file format of `only` (of every synth
/// when `None`).
pub fn library_filters(only: Option<&dyn Driver>) -> gio::ListStore {
    let mut patterns = vec![format!("*.{}", native::EXT)];
    for d in DRIVERS
        .iter()
        .filter(|d| only.is_none_or(|o| o.info().id == d.info().id))
    {
        patterns.extend(d.file_formats().iter().map(|f| format!("*.{}", f.ext)));
    }
    let refs: Vec<&str> = patterns.iter().map(String::as_str).collect();
    filters(&tr("Libraries and programs"), &refs)
}

/// A library file: an Oscyl library, or any registered synth's own
/// format, chosen by its extension.
pub fn read_file(path: &Path) -> Result<(&'static dyn Driver, LibraryData), String> {
    let bytes = std::fs::read(path).map_err(text)?;
    oscyl_app::files::read(&path.to_string_lossy(), &bytes)
}

/// Write `lib` of synth `d` to `path` in the format its extension names,
/// atomically, so a crash never leaves a half-written file.
pub fn write_file(path: &Path, d: &dyn Driver, lib: &LibraryData) -> Result<(), String> {
    let bytes =
        oscyl_app::files::write(&oscyl_app::files::ext_of(&path.to_string_lossy()), d, lib)?;
    write_atomic(path, |w| w.write_all(&bytes).map_err(text))
}

pub use oscyl_app::files::export_ext;

/// What a file dialog is for.
pub enum Pick<'a> {
    Open,
    /// Save, suggesting this file name.
    Save(&'a str),
}

impl App {
    /// Ask for a file, then run `done` with its path.
    pub(crate) fn pick_file(
        &self,
        title: &str,
        filters: &gio::ListStore,
        pick: Pick,
        done: impl Fn(&Rc<App>, PathBuf) + 'static,
    ) {
        let dialog = gtk::FileDialog::builder()
            .title(title)
            .filters(filters)
            .modal(true)
            .build();
        let done = self.weak(move |me, path| done(me, path));
        let on_file = move |res: Result<gio::File, glib::Error>| {
            if let Some(path) = res.ok().and_then(|f| f.path()) {
                done(path);
            }
        };
        match pick {
            Pick::Open => dialog.open(Some(&self.win.window), gio::Cancellable::NONE, on_file),
            Pick::Save(name) => {
                dialog.set_initial_name(Some(name));
                dialog.save(Some(&self.win.window), gio::Cancellable::NONE, on_file);
            }
        }
    }

    pub(crate) fn open_library(&self) {
        self.pick_file(
            &tr("Open Library"),
            &library_filters(None),
            Pick::Open,
            |me, path| me.load_library(path),
        );
    }

    /// The demo library of the synth shown in the sidebar; with none, the
    /// person picks which synth's, on its nameplate.
    pub(crate) fn open_demo(&self) {
        match self.active_driver() {
            Some(d) => self.open_demo_of(d),
            None if DRIVERS.len() == 1 => self.open_demo_of(DRIVERS[0]),
            None => crate::dialogs::pick_demo(&self.win, self.weak(|me, d| me.open_demo_of(d))),
        }
    }

    /// Synth `d`'s demo library, as a library with no file yet.
    pub(crate) fn open_demo_of(&self, d: &'static dyn Driver) {
        match Doc::from_data(d, d.demo_library(), None, &tr("Demo library")) {
            Ok(d) => self.add_library(d),
            Err(e) => self.error_dialog(&tr("Couldn't open the demo library"), &text(e)),
        }
    }

    pub(crate) fn load_library(&self, path: PathBuf) {
        let result = read_file(&path).and_then(|(d, lib)| {
            Doc::from_data(d, lib, Some(path.clone()), &title_of(&path)).map_err(text)
        });
        match result {
            Ok(doc) => self.add_library(doc),
            Err(e) => self.error_dialog(&tr("Couldn't open the library"), &e),
        }
    }

    /// Save the open library, asking where when it has no file yet or
    /// `ask` is set (Save As).
    pub(crate) fn save_library(&self, ask: bool) {
        let Place::Library(i) = *self.doc_place.borrow() else {
            return;
        };
        // Oscyl's own libraries save themselves: Save As exports a copy.
        if self.managed_place() == Some(i) {
            self.export_library(i);
            return;
        }
        let (path, title, d) = {
            let libs = self.libs.borrow();
            (
                libs[i].doc.path.clone(),
                libs[i].doc.title.clone(),
                libs[i].doc.driver,
            )
        };
        match path.filter(|_| !ask) {
            Some(p) => self.write_library(i, p),
            None => self.pick_file(
                &tr("Save Library"),
                &library_filters(Some(d)),
                Pick::Save(&format!("{title}.{}", export_ext(d, false))),
                move |me, p| me.write_library(i, p),
            ),
        }
    }

    /// Write atomically, so a crash never leaves a half-written library.
    fn write_library(&self, i: usize, path: PathBuf) {
        let (d, lib) = {
            let doc = &self.libs.borrow()[i].doc;
            (doc.driver, doc.to_data())
        };
        match write_file(&path, d, &lib) {
            Ok(()) => {
                let n = {
                    let doc = &mut self.libs.borrow_mut()[i].doc;
                    let n = doc.edited_count();
                    doc.title = title_of(&path);
                    doc.path = Some(path);
                    doc.mark_saved();
                    n
                };
                // As the phone says it: how many programs the save kept.
                self.win.toast(&if n > 0 {
                    trn("Saved %d program", "Saved %d programs", n as u32)
                } else {
                    tr("Library saved")
                });
                self.after_edit();
            }
            Err(e) => self.error_dialog(&tr("Couldn't save the library"), &e),
        }
    }

    /// Save every program read from the synth as a new backup library.
    pub(crate) fn backup_synth(&self) {
        if !self.synth_fully_read() {
            self.win.toast(&tr(
                "Wait until all programs have been read from the synth.",
            ));
            return;
        }
        let Some((d, lib)) = self.with_synth(|s| {
            (
                s.doc.driver,
                s.doc.backup_of(&(0..s.doc.len()).collect::<Vec<_>>()),
            )
        }) else {
            return;
        };
        match write_backup(d, &lib) {
            Ok(path) => {
                let toast = adw::Toast::new(&tr("Backup saved"));
                self.offer_open(&toast, path);
                self.win.show_toast(toast);
            }
            Err(e) => self.error_dialog(&tr("Couldn't save the backup"), &e),
        }
    }

    /// Back up what synth `id` holds in the slots about to change, then
    /// write `batch` (slot, program); without a backup nothing is written.
    pub(crate) fn sync(&self, id: DeviceId, indices: &[usize], batch: Vec<(u16, Vec<u8>)>) {
        let Some((d, lib)) = self.with_synth_id(id, |s| (s.doc.driver, s.doc.backup_of(indices)))
        else {
            return;
        };
        match write_backup(d, &lib) {
            Ok(path) => {
                self.with_synth_id(id, |s| {
                    s.sync_backup = Some(path);
                    s.sync_batch = batch.clone();
                });
                self.synth.send(Cmd::Write(id, batch));
            }
            Err(e) => self.error_dialog(
                &tr("Couldn't back up the synth"),
                &format!("{}\n\n{e}", tr("Nothing was synced.")),
            ),
        }
    }

    /// Give `toast` a button that opens the library at `path`.
    pub(crate) fn offer_open(&self, toast: &adw::Toast, path: PathBuf) {
        toast.set_button_label(Some(&tr("Open Backup")));
        let open = self.weak(move |me, ()| me.load_library(path.clone()));
        toast.connect_button_clicked(move |_| open(()));
    }
}

pub fn title_of(path: &Path) -> String {
    oscyl_app::files::title_of(&path.to_string_lossy())
}

/// A file name in the backups folder: `<name> <date time>.<ext>`.
pub fn backup_path(name: &str, ext: &str) -> PathBuf {
    let stamp = glib::DateTime::now_local()
        .and_then(|t| t.format("%Y-%m-%d %H.%M.%S"))
        .map(|s| s.to_string())
        .unwrap_or_default();
    backups_dir().join(format!("{name} {stamp}.{ext}"))
}

/// Where backups go.
fn backups_dir() -> PathBuf {
    glib::user_data_dir().join("oscyl").join("backups")
}

/// Save `lib` of synth `d` as a new synth backup and return its path.
fn write_backup(d: &dyn Driver, lib: &LibraryData) -> Result<PathBuf, String> {
    let stamp = glib::DateTime::now_local()
        .and_then(|t| t.format("%Y-%m-%d %H.%M.%S"))
        .map(|s| s.to_string())
        .unwrap_or_default();
    oscyl_app::review::write_backup(&backups_dir(), &stamp, d, lib)
}

#[cfg(test)]
mod tests {
    use super::*;
    use oscyl_app::review::backup_ext;

    #[test]
    fn files_of_any_synth_round_trip() {
        let dir = std::env::temp_dir().join(format!("oscyl-test-files-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        for d in DRIVERS {
            let lib = d.demo_library();
            for ext in [native::EXT, export_ext(*d, false), backup_ext(*d)] {
                let path = dir.join(format!("{}.{ext}", d.info().id));
                write_file(&path, *d, &lib).unwrap();
                let (back_d, back) = read_file(&path).unwrap();
                assert_eq!(back_d.info().id, d.info().id);
                let data = |l: &LibraryData| {
                    l.programs
                        .iter()
                        .map(|p| p.data.clone())
                        .collect::<Vec<_>>()
                };
                assert_eq!(data(&back), data(&lib), "{ext}");
            }
        }
        assert_eq!(backup_ext(DRIVERS[0]), "mnlgxdlib");
        assert!(read_file(&dir.join("x.txt")).is_err());
        let _ = std::fs::remove_dir_all(dir);
    }
}
