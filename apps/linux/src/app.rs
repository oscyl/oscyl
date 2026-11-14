//! The app core: documents, the selected program and edits. Files, synth
//! events, actions and dialogs live in their own modules as further
//! `impl App` blocks.

use std::cell::{Cell, OnceCell, RefCell};
use std::path::PathBuf;
use std::rc::{Rc, Weak};

use adw::prelude::*;
use gtk::glib;
use oscyl_driver::{Control, Driver};

use crate::editor::{Editor, Shown};
use crate::generator::{Ask, Generator, Target};
use crate::i18n::{tr, trn};
use crate::libraries::LineKind;
use crate::libraries::{Lib, Store};
use crate::programs::{Category, ProgramList, RowData, RowSource};
use crate::sidebar::{LibRow, Place, RowKind, Sidebar, SynthStatus, TreeRow};
use crate::state::{Doc, Program};
use crate::synth::{self, Cmd, DeviceId, SynthHandle};
use crate::window::OscylWindow;

pub struct App {
    pub win: Rc<OscylWindow>,
    pub(crate) sidebar: Rc<Sidebar>,
    pub(crate) list: Rc<ProgramList>,
    pub(crate) editor: Rc<Editor>,
    pub(crate) synth: SynthHandle,
    /// Every connected synth, in the order they connected.
    pub(crate) synths: RefCell<Vec<Synth>>,
    /// The synth the sidebar shows (its programs are Place::Synth).
    pub(crate) active: Cell<Option<DeviceId>>,
    /// The sidebar's libraries: Oscyl's own first, in their order, then
    /// files opened from elsewhere.
    pub(crate) libs: RefCell<Vec<Lib>>,
    /// Oscyl's libraries folder; `None` when it could not be opened.
    pub(crate) store: RefCell<Option<Store>>,
    /// What the editor area shows (a document, the guide, units).
    pub(crate) place: RefCell<Place>,
    /// The document behind the list and editor (Synth or a library).
    pub(crate) doc_place: RefCell<Place>,
    pub(crate) current: Cell<Option<usize>>,
    /// Connecting, before the first synth answers.
    pub(crate) connecting: Cell<bool>,
    pub(crate) audition: Cell<bool>,
    /// Snapshots A, B and C of the program open.
    pub(crate) snapshots: RefCell<crate::history::Snapshots>,
    /// Backslash is held (Compare from the keyboard).
    pub(crate) compare_key: Cell<bool>,
    pub(crate) primary: RefCell<Option<crate::window::Primary>>,
    pub(crate) allow_close: Cell<bool>,
    /// A program copied from the list, with its synth's driver id, ready
    /// to paste over another slot of that synth.
    pub(crate) clipboard: RefCell<Option<(&'static str, Program)>>,
    /// The Generate page, made the first time it is shown.
    pub(crate) generator: OnceCell<Rc<Generator>>,
    /// The Perform page, once opened, and whether it shows.
    pub(crate) perform: OnceCell<Rc<crate::perform::Perform>>,
    pub(crate) performing: Cell<bool>,
    me: Weak<App>,
}

/// One connected synth as the app sees it.
pub(crate) struct Synth {
    pub id: DeviceId,
    /// What the sidebar calls it ("minilogue xd", "minilogue xd 2").
    pub name: String,
    pub firmware: Option<String>,
    pub simulated: bool,
    pub port: String,
    /// Its programs; `loaded[i]` once program `i` has been read.
    pub doc: Doc,
    pub loaded: Vec<bool>,
    /// Its programs are being read (Escape stops it).
    pub reading: bool,
    /// What its status card says while it is shown.
    pub status: SynthStatus,
    /// Its unit slots as last listed, shown at once on return.
    pub slots: Vec<crate::synth::SlotInfo>,
    /// The backup taken before its sync in flight.
    pub sync_backup: Option<PathBuf>,
    /// What the sync in flight writes, slot by slot: what each slot holds
    /// once the synth confirms it.
    pub sync_batch: Vec<(u16, Vec<u8>)>,
}

/// The synth among `synths` a library of `driver` syncs with: the active
/// one when it is of that model, else the first connected one that is.
pub(crate) fn target_of(
    synths: &[(DeviceId, &'static str)],
    active: Option<DeviceId>,
    driver: &str,
) -> Option<DeviceId> {
    oscyl_app::editor::target_of(synths.iter().copied(), active, driver)
}

/// A new synth's name: its model, numbered when that model is already
/// connected under the plain name or a number ("minilogue xd 2").
pub(crate) fn synth_name(model: &str, taken: &[String]) -> String {
    (1..)
        .map(|n| {
            if n == 1 {
                model.to_string()
            } else {
                format!("{model} {n}")
            }
        })
        .find(|name| !taken.contains(name))
        .expect("a free name")
}

impl App {
    pub fn new(win: Rc<OscylWindow>) -> Rc<App> {
        let (synth, events) = synth::spawn();
        let list_title = win.list_title.clone();
        let app: Rc<App> = Rc::new_cyclic(|me: &Weak<App>| App {
            win,
            sidebar: Sidebar::new(),
            list: ProgramList::new(me.clone(), list_title),
            editor: Editor::new(oscyl_midi::DRIVERS[0]),
            synth,
            synths: RefCell::new(Vec::new()),
            active: Cell::new(None),
            libs: RefCell::new(Vec::new()),
            store: RefCell::new(None),
            place: RefCell::new(Place::Guide),
            doc_place: RefCell::new(Place::Guide),
            current: Cell::new(None),
            connecting: Cell::new(false),
            audition: Cell::new(false),
            compare_key: Cell::new(false),
            snapshots: RefCell::default(),
            primary: RefCell::new(None),
            allow_close: Cell::new(false),
            clipboard: RefCell::new(None),
            generator: OnceCell::new(),
            perform: OnceCell::new(),
            performing: Cell::new(false),
            me: me.clone(),
        });
        *app.primary.borrow_mut() = Some(app.win.set_panes(
            &app.sidebar.root,
            &app.list.root,
            &app.editor.root,
            &app.editor.header_title,
        ));
        app.connect_signals();
        app.install_compare();
        // The history is drawn as it opens, for the document shown then.
        if let Some((button, body)) = app
            .primary
            .borrow()
            .as_ref()
            .map(|p| (p.history.clone(), p.history_body.clone()))
            && let Some(popover) = button.popover()
        {
            let fill = app.weak(move |me, ()| me.fill_history(&body));
            popover.connect_show(move |_| fill(()));
        }
        app.install_actions();
        app.install_program_actions();
        app.install_library_actions();
        app.load_libraries();
        app.set_facet(app.editor.driver());
        app.refresh_status();
        // Nothing holds focus at launch (GTK would put it in the search, so
        // the first key typed would land there): once shown, clear it,
        // unless a library's name is already being typed.
        let sidebar = app.sidebar.root.clone();
        app.win.window.connect_map(move |w| {
            let (w, sidebar) = (w.clone(), sidebar.clone());
            glib::idle_add_local_once(move || {
                if !GtkWindowExt::focus(&w).is_some_and(|f| f.is_ancestor(&sidebar)) {
                    GtkWindowExt::set_focus(&w, None::<&gtk::Widget>);
                }
            });
        });

        // Synth events stream into the main loop as they happen.
        let me = app.me.clone();
        glib::spawn_future_local(async move {
            while let Ok(ev) = events.recv().await {
                match me.upgrade() {
                    Some(app) => app.on_synth(ev),
                    None => break,
                }
            }
        });

        app.editor.show_welcome(
            &tr("Welcome to Oscyl"),
            &tr("Everything works offline; you only need the synth to listen and sync."),
        );
        app
    }

    /// A callback that runs `f` on the app while it is alive.
    pub fn weak<A>(&self, f: impl Fn(&Rc<App>, A) + 'static) -> impl Fn(A) + 'static {
        let me = self.me.clone();
        move |a| {
            if let Some(me) = me.upgrade() {
                f(&me, a);
            }
        }
    }

    fn connect_signals(&self) {
        // Program actions belong to the program: away from it (guide,
        // units, welcome) the header drops them.
        let me = self.me.clone();
        self.editor
            .root
            .connect_visible_child_name_notify(move |s| {
                let Some(me) = me.upgrade() else { return };
                let page = s.visible_child_name();
                let on = page.as_deref() == Some("editor");
                // Keeping a generated sound is an edit: its Sync count
                // shows on the Generate page too.
                let generate = page.as_deref() == Some("generate");
                if let Some(p) = me.primary.borrow().as_ref() {
                    p.button.set_visible(on || generate);
                    p.audition.set_visible(on);
                    p.compare.set_visible(on);
                }
            });
        let update = self.weak(|me, ()| me.update_actions());
        let update = std::rc::Rc::new(update);
        {
            let update = update.clone();
            self.win.narrow.connect_apply(move |_| update(()));
        }
        self.win.narrow.connect_unapply(move |_| update(()));
        self.sidebar
            .connect_place(self.weak(|me, place| me.go(place)));
        self.sidebar
            .connect_pick(self.weak(|me, id| me.set_active(id)));
        self.sidebar.connect_rename(
            self.weak(|me, (key, name): (String, String)| me.rename_node(&key, &name)),
        );
        self.sidebar.connect_toggle(
            self.weak(|me, (key, open): (String, bool)| me.toggle_folder(&key, open)),
        );
        self.sidebar
            .connect_drop(self.weak(|me, (what, spot)| me.dropped_on_tree(what, spot)));
        {
            let me = self.me.clone();
            self.sidebar
                .set_menus(crate::library_menu::folder_model, move |i| {
                    let folders = me.upgrade().map(|me| me.folder_paths()).unwrap_or_default();
                    crate::library_menu::model(i, &folders)
                });
        }
        self.sidebar
            .connect_menu(self.weak(|me, i| me.update_library_actions(i)));
        self.list.connect_select(self.weak(|me, i| {
            me.select_program(i);
            // Moving by keys keeps the list in view on a phone; a click
            // or Enter opens the editor.
            if !me.list.take_key_nav() {
                me.win.inner.set_show_content(true);
            }
        }));
        self.list
            .connect_open(self.weak(|me, _| me.win.inner.set_show_content(true)));
        self.list
            .connect_reorder(self.weak(|me, (from, to, how)| me.reorder_program(from, to, how)));
        self.editor
            .connect_change(self.weak(|me, (field, value)| me.edit(field, value)));
        self.editor
            .connect_gesture(self.weak(|me, ()| me.with_doc(Doc::gesture).unwrap_or(())));
        self.editor.connect_seal(self.weak(|me, ()| {
            me.with_doc(Doc::seal);
            me.autosave();
        }));
        self.editor.connect_rename(self.weak(|me, name: String| {
            let Some(i) = me.current.get() else { return };
            if let Some(Err(_)) = me.with_doc(|d| d.set_name(i, &name)) {
                me.win.toast(&tr(
                    "Names can have up to 12 letters, digits and simple symbols.",
                ));
            }
            me.after_edit();
        }));
        self.editor
            .connect_roll(self.weak(|me, page: usize| me.roll_stage(page)));
        self.editor.connect_revert(self.weak(|me, field| {
            let Some(i) = me.current.get() else { return };
            me.with_doc(|d| match field {
                Some(f) => d.revert_field(i, f),
                None => d.revert_program(i),
            });
            me.after_edit();
        }));
    }

    // ----- documents -------------------------------------------------------

    pub(crate) fn with_doc<R>(&self, f: impl FnOnce(&mut Doc) -> R) -> Option<R> {
        match &*self.doc_place.borrow() {
            Place::Synth => self.with_synth(|s| f(&mut s.doc)),
            Place::Library(i) => self.libs.borrow_mut().get_mut(*i).map(|l| f(&mut l.doc)),
            _ => None,
        }
    }

    /// The index of the library shown, when it is one of Oscyl's own.
    pub(crate) fn managed_place(&self) -> Option<usize> {
        match *self.doc_place.borrow() {
            Place::Library(i) if self.libs.borrow().get(i).is_some_and(Lib::is_managed) => Some(i),
            _ => None,
        }
    }

    pub(crate) fn is_loaded(&self, i: usize) -> bool {
        !matches!(*self.doc_place.borrow(), Place::Synth)
            || self
                .with_synth(|s| s.loaded.get(i).copied().unwrap_or(false))
                .unwrap_or(false)
    }

    /// Run `f` on the active synth.
    pub(crate) fn with_synth<R>(&self, f: impl FnOnce(&mut Synth) -> R) -> Option<R> {
        self.with_synth_id(self.active.get()?, f)
    }

    /// Run `f` on synth `id`, while connected.
    pub(crate) fn with_synth_id<R>(
        &self,
        id: DeviceId,
        f: impl FnOnce(&mut Synth) -> R,
    ) -> Option<R> {
        self.synths
            .borrow_mut()
            .iter_mut()
            .find(|s| s.id == id)
            .map(f)
    }

    /// The active synth's model.
    pub(crate) fn active_driver(&self) -> Option<&'static dyn Driver> {
        self.with_synth(|s| s.doc.driver)
    }

    /// The connected synth a library of `driver` syncs with.
    pub(crate) fn target_synth(&self, driver: &dyn Driver) -> Option<DeviceId> {
        let all: Vec<(DeviceId, &'static str)> = self
            .synths
            .borrow()
            .iter()
            .map(|s| (s.id, s.doc.driver.info().id))
            .collect();
        target_of(&all, self.active.get(), driver.info().id)
    }

    /// The synth that plays what is shown: the active synth for its own
    /// programs, the library's target synth for a library's.
    pub(crate) fn audition_target(&self) -> Option<DeviceId> {
        match *self.doc_place.borrow() {
            Place::Library(_) => self.target_synth(self.doc_driver()),
            _ => self.active.get(),
        }
    }

    /// The synth model of the document shown (the editor's when none).
    ///
    /// Read-only borrows only, so it is safe while a caller holds the
    /// libraries or the synths borrowed (`with_doc` borrows them mutably).
    pub(crate) fn doc_driver(&self) -> &'static dyn Driver {
        let shown = match &*self.doc_place.borrow() {
            Place::Synth => self.active.get().and_then(|id| {
                self.synths
                    .borrow()
                    .iter()
                    .find(|s| s.id == id)
                    .map(|s| s.doc.driver)
            }),
            Place::Library(i) => self.libs.borrow().get(*i).map(|l| l.doc.driver),
            _ => None,
        };
        shown.unwrap_or_else(|| self.editor.driver())
    }

    /// Program `i`'s slot label in the document shown.
    pub(crate) fn slot_label(&self, i: usize) -> String {
        crate::slots::slot_label(self.doc_driver(), i)
    }

    /// The editor and the list follow the document's synth model.
    fn follow_driver(&self) {
        let d = self.doc_driver();
        if self.editor.driver().info().id != d.info().id {
            self.editor.set_driver(d);
            self.set_facet(d);
        }
    }

    /// The list's facet filter, from `d`'s [`Driver::list_facet`].
    fn set_facet(&self, d: &'static dyn Driver) {
        let facet = d.list_facet().and_then(|f| {
            let spec = d.fields().get(f)?;
            match d.control(&d.init_program(), f) {
                Control::Choice(names) => Some((
                    tr(d.short_label(f)),
                    names
                        .iter()
                        .enumerate()
                        .map(|(k, n)| (tr(n), spec.min + k as u16))
                        .collect(),
                )),
                _ => None,
            }
        });
        let (title, choices) = facet.unwrap_or_default();
        self.list.set_facet(&title, choices);
    }

    pub(crate) fn go(&self, place: Place) {
        self.autosave();
        self.performing.set(false);
        *self.place.borrow_mut() = place.clone();
        match place {
            Place::Synth | Place::Library(_) => {
                *self.doc_place.borrow_mut() = place.clone();
                self.follow_driver();
                let n = self.with_doc(|d| d.len()).unwrap_or(0);
                self.list.set_len(n);
                self.win
                    .list_page
                    .set_title(&self.with_doc(|d| d.title.clone()).unwrap_or_default());
                let broken = match place {
                    Place::Library(i) => self.libs.borrow().get(i).and_then(|l| l.broken.clone()),
                    _ => None,
                };
                if let Some(error) = broken {
                    self.current.set(None);
                    self.editor.show_status(
                        &tr("Couldn't read this library"),
                        &format!(
                            "{}\n\n{error}",
                            tr("You can still export its file or delete it from the library's menu.")
                        ),
                        false,
                    );
                } else if n == 0 {
                    self.current.set(None);
                    match place {
                        Place::Synth => self.editor.show_welcome(
                            &tr("No synth connected"),
                            &tr("Connect your synth over USB, or turn on the simulated synths in Preferences."),
                        ),
                        Place::Library(i) => self.show_empty_library(i),
                        _ => {}
                    }
                } else {
                    let i = self.current.get().filter(|i| *i < n).unwrap_or(0);
                    self.show_program(i);
                }
            }
            Place::Guide => crate::guide::show(&self.win, self.editor.driver()),
            Place::Generate => {
                let g = self
                    .generator
                    .get_or_init(|| {
                        let g = Generator::new(self.doc_driver());
                        g.connect_ask(self.weak(|me, ask| me.on_generator(ask)));
                        g.set_libraries(&self.copy_targets(None));
                        g
                    })
                    .clone();
                if self.editor.root.child_by_name("generate").is_some() {
                    self.editor.root.set_visible_child_name("generate");
                } else {
                    self.editor.show_page("generate", &g.root);
                }
                self.refresh_generator();
                g.focus();
            }
            Place::Units => {
                // Connected: the last listing at once (refreshed when the
                // new one arrives), or a quiet reading state the first
                // time; never the "connect the synth" page.
                let cached = self.with_synth(|s| s.slots.clone());
                match (self.active.get(), cached) {
                    (Some(_), Some(c)) if !c.is_empty() => {
                        crate::units::show(&self.editor, &c, self.weak(|me, a| me.slot_action(a)))
                    }
                    (Some(_), _) => self.editor.show_status(
                        &tr("Oscillators and effects"),
                        &tr("Reading the synth's user slots…"),
                        false,
                    ),
                    (None, _) => self.editor.show_welcome(
                        &tr("Oscillators and effects"),
                        &tr("User oscillators and effects in the synth's user slots. Connect the synth to list them."),
                    ),
                }
                self.editor
                    .set_header_context(Some(tr("Oscillators and effects")));
                if let Some(id) = self.active.get() {
                    self.synth.send(Cmd::ReadSlots(id));
                }
            }
        }
        self.update_actions();
    }

    /// Select program `i` in the list and the editor.
    pub(crate) fn show_program(&self, i: usize) {
        self.list.select(i);
        self.select_program(i);
    }

    fn select_program(&self, i: usize) {
        // Picking a program while the guide or units show returns to it;
        // on the Generate page it picks the slot to keep sounds in.
        let doc = self.doc_place.borrow().clone();
        let place = self.place.borrow().clone();
        if place != doc && place != Place::Generate {
            *self.place.borrow_mut() = doc.clone();
            self.sidebar.select(&doc);
        }
        if self.current.get() != Some(i) {
            self.with_doc(Doc::seal);
            self.autosave();
            self.forget_snapshots();
        }
        self.current.set(Some(i));
        self.refresh_program();
        self.audition_current();
    }

    /// Show the program `delta` rows away in the list as searched and
    /// filtered; at either end, stay and say so with the bell.
    pub(crate) fn step_program(&self, delta: isize) {
        let Some(i) = self.current.get() else {
            return;
        };
        match self.list.neighbour(i, delta) {
            Some(j) => self.show_program(j),
            None => self.win.window.error_bell(),
        }
    }

    /// The current program and its saved version, copied out of the
    /// document so no borrow is held while widgets update.
    pub(crate) fn current_pair(&self) -> Option<(usize, Program, Program)> {
        let i = self.current.get()?;
        self.with_doc(|d| Some((i, d.work(i)?.to_vec(), d.base(i)?.to_vec())))
            .flatten()
    }

    pub(crate) fn refresh_program(&self) {
        if *self.place.borrow() == Place::Generate {
            self.refresh_generator();
            self.update_actions();
            return;
        }
        let Some((i, p, b)) = self.current_pair() else {
            self.update_actions();
            return;
        };
        if !self.is_loaded(i) {
            // An unread slot holds a placeholder; editing it could
            // overwrite the real program on sync.
            self.win.editor_page.set_title(&tr("Reading…"));
            self.editor.show_status(
                &tr("Reading…"),
                &tr("Slot {number} opens as soon as it has been read from the synth.")
                    .replace("{number}", &self.slot_label(i)),
                false,
            );
            self.update_actions();
            return;
        }
        if self.performing.get()
            && let Some(p) = self.perform.get()
        {
            if self.editor.root.visible_child_name().as_deref() != Some("perform") {
                self.editor.show_page("perform", &p.root);
            }
            self.refresh_perform();
            self.update_actions();
            return;
        }
        let note = self
            .with_doc(|d| {
                // Only what the user wrote: no author line.
                d.infos
                    .get(i)
                    .map(|info| info.comment.clone())
                    .unwrap_or_default()
            })
            .unwrap_or_default();
        let name = self.with_doc(|d| d.name(i)).unwrap_or_default();
        self.win.editor_page.set_title(&name);
        self.editor.show(Shown {
            slot: tr("Slot {number}").replace("{number}", &self.slot_label(i)),
            program: &p,
            base: &b,
            note,
            read_only: self.with_doc(|d| d.is_read_only(i)).unwrap_or(false),
        });
        self.update_actions();
    }

    /// Tell the Generate page (if made) the selected slot and the link.
    pub(crate) fn refresh_generator(&self) {
        let Some(g) = self.generator.get() else {
            return;
        };
        g.set_driver(self.doc_driver());
        let target = self.current.get().and_then(|i| {
            let loaded = self.is_loaded(i);
            let (p, name) = self
                .with_doc(|d| Some((d.work(i)?.to_vec(), d.name(i))))
                .flatten()?;
            Some(Target {
                slot: self.slot_label(i),
                name: if loaded { name } else { tr("Reading…") },
                base: loaded.then_some(p),
            })
        });
        g.set_target(target);
        g.set_connected(self.audition_target().is_some());
    }

    fn on_generator(&self, ask: Ask) {
        match ask {
            // The page's promise: every sound plays as it is picked,
            // whether or not live audition is on for the editor.
            Ask::Audition(p) => {
                if let Some(id) = self.audition_target() {
                    self.synth.send(Cmd::Audition(id, p));
                }
            }
            Ask::Keep(d, p) => {
                let Some(i) = self.current.get().filter(|&i| self.is_loaded(i)) else {
                    return;
                };
                if d.info().id != self.doc_driver().info().id {
                    return;
                }
                let name = d.name(&p).unwrap_or_default();
                let message = tr("Kept “{name}” in slot {slot}")
                    .replace("{name}", &name)
                    .replace("{slot}", &self.slot_label(i));
                self.replace_program(i, p, &message);
            }
            Ask::Open => {
                let doc = self.doc_place.borrow().clone();
                self.sidebar.select(&doc);
            }
            Ask::PickSlot => {
                self.win
                    .toast(&tr("Select a slot in the program list first."));
                self.win.inner.set_show_content(false);
            }
        }
    }

    /// One field of the current program changed (every step of a drag):
    /// update only what that can affect.
    pub(crate) fn edit(&self, field: usize, value: u16) {
        let Some(i) = self.current.get().filter(|&i| self.is_loaded(i)) else {
            return;
        };
        let result = self.with_doc(|d| {
            let before = (d.is_edited(i), d.is_dirty());
            d.set_field(i, field, value)
                .map(|()| (before, (d.is_edited(i), d.is_dirty())))
        });
        let ((was_edited, was_dirty), (edited, dirty)) = match result {
            Some(Ok(flags)) => flags,
            Some(Err(e)) => {
                self.win.toast(
                    &tr("Couldn't change that value: {error}").replace("{error}", &e.to_string()),
                );
                return;
            }
            None => return,
        };
        let Some((_, p, b)) = self.current_pair() else {
            return;
        };
        self.editor.update_field(field, &p, &b);
        if self.performing.get() {
            self.refresh_perform();
        }
        let d = self.doc_driver();
        self.list
            .changed(i, was_edited != edited, d.list_facet() == Some(field));
        if was_dirty != dirty {
            self.refresh_sidebar();
        }
        self.update_actions();
        // Live: a parameter the synth takes as a Control Change goes out as
        // one, so held notes keep sounding; anything else reloads the
        // whole program in the edit buffer.
        match (
            self.audition_target(),
            oscyl_app::editor::audition_of(d, &p, field),
        ) {
            (Some(id), oscyl_app::editor::Audition::Live(msgs)) if self.audition.get() => {
                self.synth.send(Cmd::Live(id, msgs));
                self.blink();
            }
            _ => self.audition(p),
        }
    }

    /// Whether synth `id` (the active one when `None`) has had all its
    /// programs read.
    pub(crate) fn fully_read(&self, id: Option<DeviceId>) -> bool {
        id.or(self.active.get())
            .and_then(|id| self.with_synth_id(id, |s| s.loaded.iter().all(|l| *l)))
            .unwrap_or(false)
    }

    /// The active synth's programs have all been read.
    pub(crate) fn synth_fully_read(&self) -> bool {
        self.fully_read(None)
    }

    /// Redraw everything that shows the current document, and save the
    /// libraries that changed.
    pub(crate) fn after_edit(&self) {
        self.autosave();
        // Programs may have come or gone in a library.
        if matches!(*self.doc_place.borrow(), Place::Library(_)) {
            let n = self.with_doc(|d| d.len()).unwrap_or(0);
            self.list.resize(n);
            if n == 0 {
                self.current.set(None);
                if *self.place.borrow() == *self.doc_place.borrow() {
                    let place = self.doc_place.borrow().clone();
                    self.refresh_sidebar();
                    self.go(place);
                    return;
                }
            } else if self.current.get().is_none_or(|i| i >= n) {
                self.current.set(Some(n - 1));
                self.list.select(n - 1);
            }
        }
        self.refresh_program();
        self.list.refresh();
        self.refresh_sidebar();
        self.audition_current();
    }

    /// Undo or redo; in a library, show the program it touched (programs
    /// come, go and move there).
    pub(crate) fn step(&self, f: impl FnOnce(&mut Doc) -> Option<usize>) {
        let Some(i) = self.with_doc(f).flatten() else {
            return;
        };
        let reorders = self.managed_place().is_some();
        let n = self.with_doc(|d| d.len()).unwrap_or(0);
        let before = self.list.len();
        self.after_edit();
        // A program back in a library fades in, one gone lets the rows
        // below glide up.
        if n > before {
            self.list.inserted(i);
        } else if n < before {
            self.list.removed(i);
        }
        if reorders && i < n && self.current.get() != Some(i) {
            self.show_program(i);
        }
    }

    pub(crate) fn undo(&self) {
        self.step(Doc::undo);
    }

    pub(crate) fn redo(&self) {
        self.step(Doc::redo);
    }

    pub(crate) fn revert_program(&self) {
        if let Some(i) = self.current.get() {
            self.with_doc(|d| d.revert_program(i));
            self.after_edit();
        }
    }

    pub(crate) fn refresh_sidebar(&self) {
        let rows: Vec<TreeRow> = {
            let libs = self.libs.borrow();
            let row = |index: usize, l: &Lib| RowKind::Library {
                index,
                row: LibRow {
                    title: l.doc.title.clone(),
                    managed: l.is_managed(),
                    count: l.doc.len(),
                    unsaved: l.unsaved(),
                    broken: l.broken.is_some(),
                },
            };
            let outline = self
                .store
                .borrow()
                .as_ref()
                .map(crate::libraries::Store::outline)
                .unwrap_or_default();
            let mut rows: Vec<TreeRow> = outline
                .into_iter()
                .filter_map(|line| {
                    let (key, kind) = match line.kind {
                        LineKind::Folder { id, name, open } => (id, RowKind::Folder { name, open }),
                        LineKind::Library(id) => {
                            let i = libs.iter().position(|l| l.id.as_deref() == Some(&id))?;
                            (id, row(i, &libs[i]))
                        }
                    };
                    Some(TreeRow {
                        key,
                        depth: line.depth,
                        shown: line.shown,
                        kind,
                    })
                })
                .collect();
            let own = libs.iter().filter(|l| l.is_managed()).count();
            rows.extend(libs.iter().enumerate().skip(own).map(|(i, l)| TreeRow {
                key: format!("file:{}", i - own),
                depth: 0,
                shown: true,
                kind: row(i, l),
            }));
            rows
        };
        self.sidebar.set_tree(&rows, true);
        if let Some(g) = self.generator.get() {
            g.set_libraries(&self.copy_targets(None));
        }
    }

    fn audition_current(&self) {
        if let Some((i, p, _)) = self.current_pair()
            && self.is_loaded(i)
        {
            self.audition(p);
        }
    }

    fn audition(&self, p: Program) {
        if let Some(id) = self.audition_target().filter(|_| self.audition.get()) {
            self.synth.send(Cmd::Audition(id, p));
            self.blink();
        }
    }

    /// Drag this far sideways while holding Compare to blend the whole way
    /// to the edit (the phone's 160 dp).
    const BLEND_PX: f64 = 160.0;

    /// Hold the Compare button, or backslash in the editor, to hear and
    /// see the saved program; letting go returns to the edited one.
    fn install_compare(&self) {
        let Some(button) = self.primary.borrow().as_ref().map(|p| p.compare.clone()) else {
            return;
        };
        // The gesture claims the press before the button's own, so the
        // hold lasts until release and never turns into a click. Dragged
        // sideways (either way, BLEND_PX for the whole way) it blends the
        // saved program into the edit; letting go glides to the edit.
        let press = gtk::GestureDrag::new();
        press.set_propagation_phase(gtk::PropagationPhase::Capture);
        let me = self.me.clone();
        press.connect_drag_begin(move |g, _, _| {
            g.set_state(gtk::EventSequenceState::Claimed);
            if let Some(me) = me.upgrade() {
                me.set_blend(0.0, true);
            }
        });
        let me = self.me.clone();
        press.connect_drag_update(move |_, dx, _| {
            if let Some(me) = me.upgrade() {
                me.set_blend((dx.abs() / Self::BLEND_PX) as f32, false);
            }
        });
        let me = self.me.clone();
        press.connect_drag_end(move |_, _, _| {
            if let Some(me) = me.upgrade() {
                me.set_blend(1.0, true);
            }
        });
        let me = self.me.clone();
        press.connect_cancel(move |_, _| {
            if let Some(me) = me.upgrade() {
                me.set_blend(1.0, true);
            }
        });
        button.add_controller(press);
        // Backslash: while held, not while typing in a field. Key repeat
        // sends more presses; only the first one counts.
        let keys = gtk::EventControllerKey::new();
        keys.set_propagation_phase(gtk::PropagationPhase::Capture);
        let typing = |c: &gtk::EventControllerKey| {
            c.widget()
                .and_downcast::<gtk::Window>()
                .and_then(|w| gtk::prelude::GtkWindowExt::focus(&w))
                .is_some_and(|f| f.is::<gtk::Text>() || f.is::<gtk::Editable>())
        };
        let me = self.me.clone();
        keys.connect_key_pressed(move |c, key, _, state| {
            let Some(me) = me.upgrade() else {
                return gtk::glib::Propagation::Proceed;
            };
            // Left and Right while backslash is held blend by tenths.
            let step = match key {
                gtk::gdk::Key::Left => -0.1,
                gtk::gdk::Key::Right => 0.1,
                _ => 0.0,
            };
            if step != 0.0 && me.editor.comparing() && me.compare_key.get() {
                me.set_blend(me.editor.blend() + step, false);
                return gtk::glib::Propagation::Stop;
            }
            if key != gtk::gdk::Key::backslash || !state.is_empty() || typing(c) {
                return gtk::glib::Propagation::Proceed;
            }
            if me.editor.root.visible_child_name().as_deref() == Some("editor") {
                me.compare_key.set(true);
                me.compare(true);
                return gtk::glib::Propagation::Stop;
            }
            gtk::glib::Propagation::Proceed
        });
        let me = self.me.clone();
        keys.connect_key_released(move |_, key, _, _| {
            if key == gtk::gdk::Key::backslash
                && let Some(me) = me.upgrade()
            {
                me.compare_key.set(false);
                me.compare(false);
            }
        });
        self.win.window.add_controller(keys);
    }

    /// Start or end a compare: the editor shows the saved program and the
    /// synth plays it (when auditioning), then both return to the edit.
    pub(crate) fn compare(&self, on: bool) {
        self.set_blend(if on { 0.0 } else { 1.0 }, true);
    }

    /// Show and play the program between saved (0) and the edit (1); see
    /// [`crate::editor::Editor::set_blend`].
    pub(crate) fn set_blend(&self, t: f32, glide: bool) {
        let t = t.clamp(0.0, 1.0);
        if (self.editor.blend() - t).abs() < 1e-4 {
            return;
        }
        let Some((i, p, base)) = self.current_pair() else {
            return;
        };
        if !self.is_loaded(i) {
            return;
        }
        self.editor.set_blend(t, glide);
        if let Some(pr) = self.primary.borrow().as_ref() {
            if t < 1.0 {
                pr.compare.add_css_class("held");
            } else {
                pr.compare.remove_css_class("held");
            }
            pr.blend.set_fraction(f64::from(t));
            pr.blend.set_visible(t > 0.0 && t < 1.0);
        }
        let d = self.doc_driver();
        self.audition(if t >= 1.0 {
            p
        } else {
            oscyl_app::editor::blend(d, &base, &p, t)
        });
    }

    /// Show on the audition lamp that a change went to the synth.
    fn blink(&self) {
        if self.simulated() {
            return;
        }
        if let Some(p) = self.primary.borrow().as_ref() {
            p.blink();
        }
    }

    /// Turn live audition on or off (the switch, Ctrl+Shift+A, onboarding).
    pub(crate) fn set_audition(&self, on: bool) {
        if let Some(a) = self
            .win
            .lookup_action("audition")
            .and_downcast::<gtk::gio::SimpleAction>()
        {
            a.set_state(&on.to_variant());
        }
        self.audition.set(on);
        if on && self.audition_target().is_none() {
            self.win.toast(&tr("Connect the synth to audition."));
        } else if on && self.simulated() {
            self.win.toast(&tr(
                "The simulated synth makes no sound. Connect a real synth to hear changes.",
            ));
        }
        self.audition_current();
    }

    pub(crate) fn add_library(&self, doc: Doc) {
        self.libs.borrow_mut().push(Lib::file(doc));
        self.refresh_sidebar();
        let i = self.libs.borrow().len() - 1;
        self.current.set(None);
        self.sidebar.select(&Place::Library(i));
    }

    pub(crate) fn update_actions(&self) {
        let (can_undo, can_redo, dirty) = self
            .with_doc(|d| (d.can_undo(), d.can_redo(), d.is_dirty()))
            .unwrap_or((false, false, false));
        let set = |name: &str, on: bool| {
            if let Some(a) = self
                .win
                .lookup_action(name)
                .and_downcast::<gtk::gio::SimpleAction>()
            {
                a.set_enabled(on);
            }
        };
        set("undo", can_undo);
        set("redo", can_redo);
        // Undo and redo say which step they take, in the history's words.
        if let Some(p) = self.primary.borrow().as_ref() {
            let (undo, redo) = self
                .with_doc(|d| {
                    let h = oscyl_app::review::history(d);
                    let done = d.done();
                    (
                        done.checked_sub(1)
                            .and_then(|k| h.get(k))
                            .map(|s| s.title.clone()),
                        h.get(done).map(|s| s.title.clone()),
                    )
                })
                .unwrap_or_default();
            let undo = match undo {
                Some(w) => tr("Undo: {step}").replace("{step}", &w),
                None => tr("Undo"),
            };
            let redo = match redo {
                Some(w) => tr("Redo: {step}").replace("{step}", &w),
                None => tr("Redo"),
            };
            p.undo.set_tooltip_text(Some(&format!("{undo} (Ctrl+Z)")));
            p.redo
                .set_tooltip_text(Some(&format!("{redo} (Ctrl+Shift+Z)")));
        }
        set("revert-program", self.current.get().is_some() && dirty);
        set("new-program", self.managed_place().is_some());
        // Escape is the window's only while it has something to do, so
        // fields, menus and sheets keep it otherwise.
        let reading = self.with_synth(|s| s.reading).unwrap_or(false);
        set("cancel-read", self.editor.enlarged() || reading);
        set("backup", self.synth_fully_read());
        // The program the synth plays, kept as a library of its own.
        set(
            "save-playing",
            self.with_synth(|s| matches!(s.status, SynthStatus::Connected { .. }))
                .unwrap_or(false)
                && self.store.borrow().is_some(),
        );
        set(
            "library-from-synth",
            self.synth_fully_read() && self.store.borrow().is_some(),
        );
        // The primary action carries the one red count of edited
        // programs; when clean it reads as a quiet "Synced"/"Saved" ✓.
        let doc_place = self.doc_place.borrow().clone();
        let edited = |d: Option<&Doc>| d.map(Doc::edited_count).unwrap_or(0);
        // One of Oscyl's libraries saves itself: its action sends it to the
        // synth. A library with no file yet (the demo) is kept as one.
        let kind = match doc_place {
            Place::Library(i) => self.libs.borrow().get(i).map(|l| {
                (
                    l.is_managed(),
                    l.doc.path.is_none(),
                    l.doc.len(),
                    l.broken.is_some(),
                )
            }),
            _ => None,
        };
        if let Some((true, _, len, broken)) = kind {
            let d = self.doc_driver();
            let target = self.target_synth(d);
            let name = target
                .and_then(|id| self.with_synth_id(id, |s| s.name.clone()))
                .unwrap_or_default();
            let (enabled, tip) = if target.is_none() {
                (
                    false,
                    tr("Connect a {model} to send this library to it.")
                        .replace("{model}", d.info().model),
                )
            } else if !self.fully_read(target) {
                (
                    false,
                    tr("Wait until all programs have been read from the synth."),
                )
            } else if len == 0 || broken {
                (false, tr("This library has no programs to send."))
            } else {
                (
                    true,
                    trn(
                        "Write its %d program to slot 1 of {synth}",
                        "Write its %d programs to {synth} from slot 1",
                        len as u32,
                    )
                    .replace("{synth}", &name),
                )
            };
            set("primary", enabled);
            if let Some(p) = self.primary.borrow().as_ref() {
                // A phone's header has room for the verb only.
                p.label.set_label(&if self.win.is_narrow.get() {
                    tr("Send")
                } else {
                    tr("Send to Synth")
                });
                p.icon.set_icon_name(Some("oscyl-sync-symbolic"));
                p.count.set_visible(false);
                p.button.set_tooltip_text(Some(&tip));
                p.button.remove_css_class("flat");
                p.button.add_css_class("suggested-action");
                p.button
                    .update_property(&[gtk::accessible::Property::Label(&tr("Send to Synth"))]);
            }
            return;
        }
        let keep = matches!(kind, Some((false, true, _, _)));
        let (n, enabled, library) = match doc_place {
            Place::Synth => {
                let n = self.with_synth(|s| s.doc.edited_count()).unwrap_or(0);
                (n, n > 0, false)
            }
            Place::Library(i) => (
                edited(self.libs.borrow().get(i).map(|l| &l.doc)),
                dirty || keep,
                true,
            ),
            _ => (0, false, false),
        };
        let clean = n == 0 && !(library && dirty) && !keep;
        let (label, name) = match (library, clean) {
            (true, false) if keep => (
                // A phone's header has room for the verb only.
                if self.win.is_narrow.get() {
                    tr("Keep")
                } else {
                    tr("Keep as Library")
                },
                tr("Keep as one of your libraries"),
            ),
            (false, true) => (tr("Synced"), tr("Synced")),
            (false, false) => (
                tr("Sync"),
                trn(
                    "Sync %d edited program",
                    "Sync %d edited programs",
                    n as u32,
                ),
            ),
            (true, true) => (tr("Saved"), tr("Saved")),
            (true, false) if n > 0 => (
                tr("Save"),
                trn(
                    "Save %d edited program",
                    "Save %d edited programs",
                    n as u32,
                ),
            ),
            (true, false) => (tr("Save"), tr("Save")),
        };
        set("primary", enabled);
        if let Some(p) = self.primary.borrow().as_ref() {
            p.label.set_label(&label);
            p.icon.set_icon_name(Some(match (clean, library) {
                (true, _) => "oscyl-saved-symbolic",
                (false, true) => "oscyl-save-symbolic",
                (false, false) => "oscyl-sync-symbolic",
            }));
            p.count.set_label(&n.to_string());
            p.count.set_visible(n > 0);
            p.button.set_tooltip_text((!clean).then_some(name.as_str()));
            // Edited: the control plate carrying the action count. Clean:
            // a quiet flat "Synced" with a check.
            if clean {
                p.button.add_css_class("flat");
                p.button.remove_css_class("suggested-action");
            } else {
                p.button.remove_css_class("flat");
                p.button.add_css_class("suggested-action");
            }
            p.button
                .update_property(&[gtk::accessible::Property::Label(&name)]);
        }
    }

    /// Save the open library, or sync the synth's edits.
    pub(crate) fn primary(&self) {
        let place = self.doc_place.borrow().clone();
        match place {
            Place::Synth => self.confirm_sync(false),
            Place::Library(i) => {
                let (managed, keep) = self
                    .libs
                    .borrow()
                    .get(i)
                    .map_or((false, false), |l| (l.is_managed(), l.doc.path.is_none()));
                if managed {
                    self.send_library(i);
                } else if keep {
                    self.keep_as_library(i);
                } else {
                    self.save_library(false);
                }
            }
            _ => {}
        }
    }
}

impl RowSource for App {
    fn row(&self, i: usize) -> RowData {
        let loaded = self.is_loaded(i);
        self.with_doc(|d| {
            if i >= d.len() {
                return RowData::default();
            }
            // The first page with a picture, drawn small (a read program).
            let picture = d.work(i).filter(|_| loaded).and_then(|p| {
                (0..d.driver.curation().pages.len()).find_map(|page| {
                    oscyl_app::editor::graph(d.driver, p, page, 48)
                        .map(crate::widgets::GraphData::from)
                })
            });
            RowData {
                slot: crate::slots::slot_label(d.driver, i),
                name: if loaded { d.name(i) } else { tr("Reading…") },
                edited: d.is_edited(i),
                picture,
            }
        })
        .unwrap_or_default()
    }

    fn facts(&self, i: usize) -> (Option<u16>, bool) {
        self.with_doc(|d| {
            let facet = d
                .driver
                .list_facet()
                .and_then(|f| d.driver.get(d.work(i)?, f).ok());
            (facet, d.is_edited(i))
        })
        .unwrap_or_default()
    }

    fn slot(&self, i: usize) -> String {
        self.slot_label(i)
    }

    fn category(&self, i: usize) -> Option<Category> {
        if !self.is_loaded(i) {
            return None;
        }
        self.with_doc(|d| d.driver.category(d.work(i)?)).flatten()
    }

    fn prepare_menu(&self, i: usize) {
        self.update_program_actions(i);
    }

    fn menu(&self, i: usize) -> gtk::gio::Menu {
        self.program_menu(i)
    }

    fn can_drag(&self, i: usize) -> bool {
        self.is_loaded(i)
            && self
                .with_doc(|d| i < d.len() && !d.is_read_only(i))
                .unwrap_or(false)
    }

    fn reorderable(&self) -> Result<(), String> {
        self.can_reorder()
    }

    fn reorder_changes(&self, from: usize, to: usize, how: crate::state::Reorder) -> usize {
        self.with_doc(|d| d.reorder_changes(from, to, how))
            .unwrap_or(0)
    }

    fn name(&self, i: usize) -> String {
        if !self.is_loaded(i) {
            return tr("Reading…");
        }
        self.with_doc(|d| d.name(i)).unwrap_or_default()
    }
}
