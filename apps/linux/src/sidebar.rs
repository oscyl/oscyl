//! Sources sidebar: synth status, the synth's places and the libraries:
//! Oscyl's own, in a tree of folders (renamed in place, ordered and moved
//! by keys or by dragging, with row menus), and files opened from
//! elsewhere.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use adw::prelude::*;
use gtk::{gdk, gio, glib};

use crate::i18n::{tr, trn};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SynthStatus {
    Disconnected,
    Connecting,
    Connected {
        /// Empty when the synth does not say.
        firmware: String,
        simulated: bool,
    },
    Reading {
        done: usize,
        total: usize,
    },
    Writing {
        done: usize,
        total: usize,
    },
}

/// A row the sidebar can select.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Place {
    Synth,
    Library(usize),
    Units,
    Generate,
    Guide,
}

/// What a library row shows.
#[derive(Debug, Clone, Default)]
pub struct LibRow {
    pub title: String,
    /// One of Oscyl's own libraries (with a program count and a menu).
    pub managed: bool,
    pub count: usize,
    /// Changes not saved yet (or an autosave that failed).
    pub unsaved: bool,
    /// The file could not be read.
    pub broken: bool,
}

/// One row of the libraries tree.
#[derive(Debug, Clone)]
pub struct TreeRow {
    /// A library's or folder's id; a file opened from elsewhere has its
    /// own key.
    pub key: String,
    pub depth: usize,
    /// Every folder around it is open.
    pub shown: bool,
    pub kind: RowKind,
}

#[derive(Debug, Clone)]
pub enum RowKind {
    Folder {
        name: String,
        open: bool,
    },
    /// Library `index` of the app's list.
    Library {
        index: usize,
        row: LibRow,
    },
}

/// What was dropped on the libraries tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Dropped {
    /// A library or folder, by key.
    Node(String),
    /// Program `i` of the list shown.
    Program(usize),
}

/// Where it was dropped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Spot {
    /// Last in a folder (`None`: the top level), or onto a library.
    Into(Option<String>),
    Before(String),
    After(String),
}

/// A tree row's parts.
struct TreeWidgets {
    key: String,
    row: gtk::ListBoxRow,
    fold: crate::widgets::Fold,
    /// The guides for each level it is indented by.
    guides: gtk::Box,
    /// What the row looks like: the hover and the plate are its size.
    body: gtk::Box,
    /// A folder's chevron, or a library's icon.
    lead: gtk::Image,
    label: gtk::Label,
    caption: gtk::Label,
    dot: gtk::Widget,
    /// The name, or the field renaming it.
    stack: gtk::Stack,
    entry: gtk::Entry,
    depth: Cell<usize>,
    /// Library index, or `None` for a folder.
    library: Cell<Option<usize>>,
    managed: Cell<bool>,
    open: Cell<bool>,
    shown: Cell<bool>,
    /// Opening this closed folder while something is held over it.
    hover: RefCell<Option<gtk::TickCallbackId>>,
}

impl TreeWidgets {
    fn is_folder(&self) -> bool {
        self.library.get().is_none()
    }

    /// Mark where a drop would go, or nothing.
    fn mark(&self, spot: Option<&Spot>) {
        let x = self
            .body
            .compute_point(&self.fold, &gtk::graphene::Point::zero())
            .map_or(0.0, |p| p.x());
        let line = |at| crate::widgets::Line { x, at };
        match spot {
            Some(Spot::Before(_)) => self.fold.set_line(Some(line(0.0)), true),
            Some(Spot::After(_)) => self.fold.set_line(Some(line(1.0)), true),
            _ => self.fold.set_line(None, true),
        }
        if matches!(spot, Some(Spot::Into(_))) {
            self.body.add_css_class("drop-into");
        } else {
            self.body.remove_css_class("drop-into");
        }
    }

    fn stop_hover(&self) {
        if let Some(id) = self.hover.take() {
            id.remove();
        }
    }
}

/// How long something held over a closed folder waits before opening
/// it, measured on the frame clock.
const HOVER_OPEN_US: i64 = 600_000;

/// Rows above the libraries, then the libraries, then New Library and
/// Open Library…: the list's sort ranks.
const RANK_TREE: u32 = 100;
const RANK_NEW: u32 = 1_000_000;

type FolderMenu = Box<dyn Fn(&str) -> gio::Menu>;
type LibraryMenu = Box<dyn Fn(usize) -> gio::Menu>;

pub struct Sidebar {
    pub root: gtk::Box,
    status_title: gtk::Label,
    /// With two synths or more, the title is this picker of them.
    picker: gtk::DropDown,
    /// The picker's synths, in its order.
    picker_ids: RefCell<Vec<u64>>,
    /// Set while the picker follows the app, so it does not report back.
    picker_quiet: Cell<bool>,
    on_pick: crate::callback::Slot<u64>,
    status_detail: gtk::Label,
    status_dot: gtk::Box,
    progress: gtk::ProgressBar,
    connect: gtk::Button,
    cancel: gtk::Button,
    list: gtk::ListBox,
    /// The tree's rows by key, and their order.
    tree: RefCell<std::collections::HashMap<String, Rc<TreeWidgets>>>,
    order: RefCell<Vec<String>>,
    /// Each row's place in the list.
    ranks: Rc<RefCell<std::collections::HashMap<gtk::ListBoxRow, u32>>>,
    /// The New Library row; library rows go above it.
    new_row: gtk::ListBoxRow,
    lib_anchor: gtk::ListBoxRow,
    places: RefCell<Vec<(gtk::ListBoxRow, Place)>>,
    on_place: crate::callback::Slot<Place>,
    /// Selecting a row only moves the plate (the place is already shown).
    quiet: Cell<bool>,
    /// A library or folder was renamed in place: its key and new name.
    on_rename: crate::callback::Slot<(String, String)>,
    /// Library `i`'s menu is about to open, or a key acts on it.
    on_menu: crate::callback::Slot<usize>,
    /// A folder's row menu, by key.
    folder_menu: RefCell<Option<FolderMenu>>,
    /// Library `i`'s row menu.
    library_menu: RefCell<Option<LibraryMenu>>,
    /// A folder opened or closed: its key and whether it is open.
    on_toggle: crate::callback::Slot<(String, bool)>,
    on_drop: crate::callback::Slot<(Dropped, Spot)>,
}

fn header(text: &str) -> gtk::Label {
    let l = gtk::Label::builder().label(text).xalign(0.0).build();
    l.add_css_class("sidebar-heading");
    l.add_css_class("caption-heading");
    l.add_css_class("dim-label");
    l
}

/// A sidebar row: icon and label.
fn place_row(icon: &str, label: &str) -> gtk::ListBoxRow {
    let b = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    b.append(&gtk::Image::from_icon_name(icon));
    let l = gtk::Label::builder()
        .label(label)
        .xalign(0.0)
        .hexpand(true)
        .wrap(true)
        .wrap_mode(pango::WrapMode::WordChar)
        .lines(2)
        .ellipsize(pango::EllipsizeMode::End)
        .build();
    b.append(&l);
    gtk::ListBoxRow::builder().child(&b).build()
}

/// What is being dragged, from a drop's value.
fn dropped(value: Option<glib::Value>) -> Option<Dropped> {
    let s = value?.get::<String>().ok()?;
    if let Some(i) = s.strip_prefix("program:") {
        return i.parse().ok().map(Dropped::Program);
    }
    s.strip_prefix("node:")
        .map(|k| Dropped::Node(k.to_string()))
}

impl Sidebar {
    pub fn new() -> Rc<Sidebar> {
        let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
        root.add_css_class("sources");

        // Synth status: a flat row (dot, name, state) with its one action
        // at the end, and a progress bar under it while transferring.
        let status = gtk::Box::new(gtk::Orientation::Vertical, 0);
        status.add_css_class("synth-status");
        let top = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        let status_dot = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        status_dot.add_css_class("status-dot");
        status_dot.set_valign(gtk::Align::Center);
        // The name (or, with several synths, the picker) shares one row with
        // the dot and the action, so all three line up whichever shows; the
        // state reads under the name.
        let head = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        head.set_hexpand(true);
        head.set_valign(gtk::Align::Center);
        let status_title = gtk::Label::builder()
            .label(tr("Synth"))
            .xalign(0.0)
            .ellipsize(pango::EllipsizeMode::End)
            .build();
        status_title.add_css_class("heading");
        let status_detail = gtk::Label::builder()
            .label(tr("Not connected"))
            .xalign(0.0)
            .wrap(true)
            .build();
        status_detail.add_css_class("caption");
        status_detail.add_css_class("dim-label");
        // Several synths: the title picks which one the sidebar shows.
        let picker = gtk::DropDown::from_strings(&[]);
        picker.add_css_class("flat");
        picker.add_css_class("synth-picker");
        picker.set_halign(gtk::Align::Start);
        picker.set_visible(false);
        picker.set_tooltip_text(Some(&tr("Which synth to show and sync with")));
        picker.update_property(&[gtk::accessible::Property::Label(&tr("Synth"))]);
        head.append(&status_title);
        head.append(&picker);
        // Indented by the dot (6px) and the row's spacing (8px).
        status_detail.set_margin_start(14);
        let connect = gtk::Button::builder()
            .label(tr("Connect"))
            .action_name("win.connect")
            .valign(gtk::Align::Center)
            .build();
        let cancel = gtk::Button::builder()
            .icon_name("media-playback-stop-symbolic")
            .action_name("win.cancel-read")
            .tooltip_text(tr("Stop reading"))
            .valign(gtk::Align::Center)
            .visible(false)
            .build();
        cancel.update_property(&[gtk::accessible::Property::Label(&tr("Stop reading"))]);
        cancel.add_css_class("flat");
        cancel.add_css_class("circular");
        top.append(&status_dot);
        top.append(&head);
        top.append(&connect);
        top.append(&cancel);
        status.append(&top);
        status.append(&status_detail);
        let progress = gtk::ProgressBar::new();
        progress.set_margin_top(8);
        progress.set_visible(false);
        progress.update_property(&[gtk::accessible::Property::Label(&tr(
            "Synth transfer progress",
        ))]);
        status.append(&progress);
        root.append(&status);

        let list = gtk::ListBox::new();
        list.add_css_class("navigation-sidebar");
        list.update_property(&[gtk::accessible::Property::Label(&tr("Sources"))]);
        let ranks: Rc<RefCell<std::collections::HashMap<gtk::ListBoxRow, u32>>> = Rc::default();
        {
            let ranks = ranks.clone();
            list.set_sort_func(move |a, b| {
                let r = ranks.borrow();
                let rank = |row: &gtk::ListBoxRow| r.get(row).copied().unwrap_or(u32::MAX);
                rank(a).cmp(&rank(b)).into()
            });
        }
        // The selected place is the plate, sliding between rows; under a
        // library it is the size of the row's body (rows in folders are
        // indented).
        let plated = {
            let l = list.downgrade();
            crate::widgets::Plated::new(&list, 6.0, "raised", move || {
                let row = l.upgrade()?.selected_row()?;
                if row.has_css_class("tree-row") {
                    row.child()
                        .and_downcast::<crate::widgets::Fold>()?
                        .child()?
                        .last_child()
                } else {
                    Some(row.upcast())
                }
            })
        };
        {
            let plated = plated.downgrade();
            list.connect_selected_rows_changed(move |_| {
                if let Some(p) = plated.upgrade() {
                    p.place(false);
                }
            });
        }
        let scroll = gtk::ScrolledWindow::builder()
            .child(&plated)
            .vexpand(true)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .build();
        crate::widgets::smooth_wheel(&scroll);
        root.append(&scroll);

        let this = Rc::new(Sidebar {
            root,
            status_title,
            picker: picker.clone(),
            picker_ids: RefCell::default(),
            picker_quiet: Cell::new(false),
            on_pick: Default::default(),
            status_detail,
            status_dot,
            progress,
            connect,
            cancel,
            list: list.clone(),
            tree: RefCell::default(),
            order: RefCell::default(),
            ranks,
            new_row: gtk::ListBoxRow::new(),
            lib_anchor: gtk::ListBoxRow::new(),
            places: RefCell::new(Vec::new()),
            on_place: Default::default(),
            quiet: Cell::new(false),
            on_rename: Default::default(),
            on_menu: Default::default(),
            folder_menu: RefCell::default(),
            library_menu: RefCell::default(),
            on_toggle: Default::default(),
            on_drop: Default::default(),
        });

        let rank = Cell::new(0);
        let append = |row: &gtk::ListBoxRow| {
            this.ranks.borrow_mut().insert(row.clone(), rank.get());
            rank.set(rank.get() + 1);
            list.append(row);
        };
        let add_header = |text: &str| {
            let row = gtk::ListBoxRow::builder()
                .child(&header(text))
                .selectable(false)
                .activatable(false)
                .build();
            append(&row);
        };
        add_header(&tr("Synth"));
        for (icon, label, place) in [
            ("oscyl-synth-symbolic", tr("Programs"), Place::Synth),
            ("oscyl-generator-symbolic", tr("Generate"), Place::Generate),
            (
                "oscyl-unit-symbolic",
                tr("Oscillators and effects"),
                Place::Units,
            ),
        ] {
            let row = place_row(icon, &label);
            append(&row);
            this.places.borrow_mut().push((row, place));
        }
        add_header(&tr("Libraries"));
        let quiet_row = |row: &gtk::ListBoxRow, icon: &str, label: &str, r: u32| {
            row.set_selectable(false);
            row.set_activatable(true);
            let b = gtk::Box::new(gtk::Orientation::Horizontal, 12);
            b.append(&gtk::Image::from_icon_name(icon));
            b.append(&gtk::Label::builder().label(label).xalign(0.0).build());
            b.add_css_class("dim-label");
            row.set_child(Some(&b));
            this.ranks.borrow_mut().insert(row.clone(), r);
            list.append(row);
        };
        quiet_row(
            &this.new_row,
            "list-add-symbolic",
            &tr("New Library"),
            RANK_NEW,
        );
        this.lib_anchor.set_action_name(Some("win.open"));
        quiet_row(
            &this.lib_anchor,
            "document-open-symbolic",
            &tr("Open Library…"),
            RANK_NEW + 1,
        );
        // New Library offers where the programs come from, or a folder.
        let new_menu = gio::Menu::new();
        let make = gio::Menu::new();
        make.append(Some(&tr("Empty Library")), Some("win.library-new"));
        make.append(Some(&tr("From the Synth")), Some("win.library-from-synth"));
        let import = gio::MenuItem::new(Some(&tr("Import File…")), None);
        import.set_action_and_target_value(Some("win.library-import"), Some(&(-1i32).to_variant()));
        make.append_item(&import);
        new_menu.append_section(None, &make);
        let folder = gio::MenuItem::new(Some(&tr("New Folder")), None);
        folder.set_action_and_target_value(Some("win.folder-new-folder"), Some(&"".to_variant()));
        new_menu.append_item(&folder);
        {
            let new_row = this.new_row.clone();
            let me = Rc::downgrade(&this);
            list.connect_row_activated(move |_, row| {
                if *row == new_row {
                    crate::programs::popup(row.upcast_ref(), &new_menu, None);
                } else if let Some(me) = me.upgrade()
                    && let Some(w) = me.widgets_of(row).filter(|w| w.is_folder())
                {
                    me.on_toggle.emit((w.key.clone(), !w.open.get()));
                }
            });
        }
        // Dropping on New Library puts a library or folder last at the
        // top level.
        this.drop_on_new_row();
        this.connect_library_keys();

        {
            let me = Rc::downgrade(&this);
            picker.connect_selected_notify(move |p| {
                let Some(me) = me.upgrade() else { return };
                if me.picker_quiet.get() {
                    return;
                }
                let id = me.picker_ids.borrow().get(p.selected() as usize).copied();
                if let Some(id) = id {
                    me.on_pick.emit(id);
                }
            });
        }
        let me = Rc::downgrade(&this);
        list.connect_row_selected(move |_, row| {
            let (Some(me), Some(row)) = (me.upgrade(), row) else {
                return;
            };
            if me.quiet.get() {
                return;
            }
            if let Some(place) = me.place_of(row) {
                me.on_place.emit(place);
            }
        });
        this
    }

    fn widgets_of(&self, row: &gtk::ListBoxRow) -> Option<Rc<TreeWidgets>> {
        self.tree.borrow().values().find(|w| w.row == *row).cloned()
    }

    fn widgets(&self, key: &str) -> Option<Rc<TreeWidgets>> {
        self.tree.borrow().get(key).cloned()
    }

    /// Library `i`'s row.
    fn library_widgets(&self, i: usize) -> Option<Rc<TreeWidgets>> {
        self.tree
            .borrow()
            .values()
            .find(|w| w.library.get() == Some(i))
            .cloned()
    }

    fn place_of(&self, row: &gtk::ListBoxRow) -> Option<Place> {
        if let Some((_, p)) = self.places.borrow().iter().find(|(r, _)| r == row) {
            return Some(p.clone());
        }
        self.widgets_of(row)?.library.get().map(Place::Library)
    }

    pub fn connect_place(&self, f: impl Fn(Place) + 'static) {
        self.on_place.connect(f);
    }

    pub fn connect_rename(&self, f: impl Fn((String, String)) + 'static) {
        self.on_rename.connect(f);
    }

    pub fn connect_menu(&self, f: impl Fn(usize) + 'static) {
        self.on_menu.connect(f);
    }

    pub fn connect_toggle(&self, f: impl Fn((String, bool)) + 'static) {
        self.on_toggle.connect(f);
    }

    pub fn connect_drop(&self, f: impl Fn((Dropped, Spot)) + 'static) {
        self.on_drop.connect(f);
    }

    /// The row menus: a folder's (by key) and library `i`'s.
    pub fn set_menus(
        &self,
        folder: impl Fn(&str) -> gio::Menu + 'static,
        library: impl Fn(usize) -> gio::Menu + 'static,
    ) {
        *self.folder_menu.borrow_mut() = Some(Box::new(folder));
        *self.library_menu.borrow_mut() = Some(Box::new(library));
    }

    /// Open the row menu of `w` (at the pointer, when there is one).
    fn popup_menu(&self, w: &TreeWidgets, at: Option<(f64, f64)>) {
        let menu = match w.library.get() {
            Some(i) if w.managed.get() => {
                self.on_menu.emit(i);
                self.library_menu.borrow().as_ref().map(|f| f(i))
            }
            Some(_) => None,
            None => self.folder_menu.borrow().as_ref().map(|f| f(&w.key)),
        };
        if let Some(menu) = menu {
            crate::programs::popup(w.row.upcast_ref(), &menu, at);
        }
    }

    /// The focused tree row's keys: F2 renames, Alt+Up and Alt+Down move
    /// it, Delete deletes it (with Undo), Menu or Shift+F10 opens its
    /// menu; on a folder Left closes it and Right opens it. Not while its
    /// name is being typed.
    fn connect_library_keys(self: &Rc<Self>) {
        let keys = gtk::EventControllerKey::new();
        let me = Rc::downgrade(self);
        keys.connect_key_pressed(move |_, key, _, state| {
            let Some(me) = me.upgrade() else {
                return glib::Propagation::Proceed;
            };
            let typing = me
                .list
                .root()
                .and_then(|r| r.focus())
                .is_some_and(|f| f.is::<gtk::Text>());
            let Some(w) = me
                .list
                .focus_child()
                .and_downcast::<gtk::ListBoxRow>()
                .and_then(|r| me.widgets_of(&r))
                .filter(|w| (w.managed.get() || w.is_folder()) && !typing)
            else {
                return glib::Propagation::Proceed;
            };
            let alt = state.contains(gdk::ModifierType::ALT_MASK);
            let shift = state.contains(gdk::ModifierType::SHIFT_MASK);
            let folder = w.is_folder();
            let action = match key {
                gdk::Key::F2 => "rename",
                gdk::Key::Up | gdk::Key::KP_Up if alt => "move-up",
                gdk::Key::Down | gdk::Key::KP_Down if alt => "move-down",
                gdk::Key::Delete | gdk::Key::KP_Delete => "delete",
                gdk::Key::Left | gdk::Key::KP_Left if folder && w.open.get() => {
                    me.on_toggle.emit((w.key.clone(), false));
                    return glib::Propagation::Stop;
                }
                gdk::Key::Right | gdk::Key::KP_Right if folder && !w.open.get() => {
                    me.on_toggle.emit((w.key.clone(), true));
                    return glib::Propagation::Stop;
                }
                gdk::Key::Menu => "",
                gdk::Key::F10 if shift => "",
                _ => return glib::Propagation::Proceed,
            };
            if action.is_empty() {
                me.popup_menu(&w, None);
                return glib::Propagation::Stop;
            }
            let _ = match w.library.get() {
                Some(i) => {
                    me.on_menu.emit(i);
                    w.row.activate_action(
                        &format!("win.library-{action}"),
                        Some(&(i as i32).to_variant()),
                    )
                }
                None => w
                    .row
                    .activate_action(&format!("win.folder-{action}"), Some(&w.key.to_variant())),
            };
            glib::Propagation::Stop
        });
        self.list.add_controller(keys);
    }

    /// A new tree row, with its menu on right click and long press, its
    /// name field, and dragging and dropping.
    fn tree_row(self: &Rc<Self>, key: &str, folder: bool) -> Rc<TreeWidgets> {
        let body = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        body.add_css_class("tree-body");
        body.set_hexpand(true);
        let lead = gtk::Image::from_icon_name(if folder {
            "pan-end-symbolic"
        } else {
            "oscyl-library-symbolic"
        });
        if folder {
            lead.add_css_class("chevron");
        }
        body.append(&lead);
        let texts = gtk::Box::new(gtk::Orientation::Vertical, 0);
        texts.set_valign(gtk::Align::Center);
        let label = gtk::Label::builder()
            .xalign(0.0)
            .ellipsize(pango::EllipsizeMode::End)
            .build();
        let caption = gtk::Label::builder().xalign(0.0).visible(false).build();
        caption.add_css_class("legend");
        caption.add_css_class("numeric");
        texts.append(&label);
        texts.append(&caption);
        let entry = gtk::Entry::builder().valign(gtk::Align::Center).build();
        entry.update_property(&[gtk::accessible::Property::Label(&if folder {
            tr("Folder name")
        } else {
            tr("Library name")
        })]);
        let stack = gtk::Stack::builder()
            .hexpand(true)
            .hhomogeneous(false)
            .vhomogeneous(false)
            .transition_type(gtk::StackTransitionType::Crossfade)
            .build();
        stack.add_named(&texts, Some("label"));
        stack.add_named(&entry, Some("entry"));
        body.append(&stack);
        let dot = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        dot.add_css_class("edited-dot");
        dot.set_valign(gtk::Align::Center);
        dot.set_visible(false);
        body.append(&dot);
        let guides = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        let line = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        line.append(&guides);
        line.append(&body);
        let fold = crate::widgets::Fold::new(&line);
        let row = gtk::ListBoxRow::builder()
            .child(&fold)
            .selectable(!folder)
            .build();
        row.add_css_class("tree-row");
        row.add_css_class(if folder { "folder-row" } else { "library-row" });
        let w = Rc::new(TreeWidgets {
            key: key.to_string(),
            row,
            fold,
            guides,
            body,
            lead,
            label,
            caption,
            dot: dot.upcast(),
            stack,
            entry,
            depth: Cell::new(0),
            library: Cell::new(None),
            managed: Cell::new(false),
            open: Cell::new(true),
            shown: Cell::new(true),
            hover: RefCell::default(),
        });

        let me = Rc::downgrade(self);
        let open = {
            let me = me.clone();
            let w = Rc::downgrade(&w);
            move |x: f64, y: f64| {
                if let (Some(me), Some(w)) = (me.upgrade(), w.upgrade()) {
                    me.popup_menu(&w, Some((x, y)));
                }
            }
        };
        let click = gtk::GestureClick::builder()
            .button(gdk::BUTTON_SECONDARY)
            .build();
        {
            let open = open.clone();
            click.connect_pressed(move |_, _, x, y| open(x, y));
        }
        w.row.add_controller(click);
        let press = gtk::GestureLongPress::new();
        press.set_touch_only(true);
        press.connect_pressed(move |_, x, y| open(x, y));
        w.row.add_controller(press);

        // The name field: Enter or leaving it keeps the name, Escape
        // keeps the old one.
        let finish = {
            let me = me.clone();
            let w = Rc::downgrade(&w);
            move |keep: bool| {
                let (Some(me), Some(w)) = (me.upgrade(), w.upgrade()) else {
                    return;
                };
                if w.stack.visible_child_name().as_deref() != Some("entry") {
                    return;
                }
                w.stack.set_visible_child_name("label");
                let name = w.entry.text().trim().to_string();
                let focus = w.entry.has_focus() || w.entry.focus_child().is_some();
                if focus {
                    w.row.grab_focus();
                }
                if keep && !name.is_empty() && name != w.label.label() {
                    me.on_rename.emit((w.key.clone(), name));
                }
            }
        };
        let finish = Rc::new(finish);
        {
            let finish = finish.clone();
            w.entry.connect_activate(move |_| finish(true));
        }
        let leave = gtk::EventControllerFocus::new();
        {
            let finish = finish.clone();
            leave.connect_leave(move |_| finish(true));
        }
        w.entry.add_controller(leave);
        let escape = gtk::EventControllerKey::new();
        escape.connect_key_pressed(move |_, key, _, _| {
            if key == gdk::Key::Escape {
                finish(false);
                return glib::Propagation::Stop;
            }
            glib::Propagation::Proceed
        });
        w.entry.add_controller(escape);
        self.drag_and_drop(&w);
        w
    }

    /// A managed library or a folder can be picked up; anything of Oscyl's
    /// can be dropped on a row, and a program onto a library.
    fn drag_and_drop(self: &Rc<Self>, w: &Rc<TreeWidgets>) {
        let drag = gtk::DragSource::new();
        drag.set_actions(gdk::DragAction::MOVE);
        {
            let w = Rc::downgrade(w);
            drag.connect_prepare(move |d, x, y| {
                let w = w.upgrade().filter(|w| w.managed.get() || w.is_folder())?;
                // Not while its name is being typed.
                if w.stack.visible_child_name().as_deref() == Some("entry") {
                    return None;
                }
                d.set_icon(
                    Some(&gtk::WidgetPaintable::new(Some(&w.body))),
                    x as i32,
                    y as i32,
                );
                Some(gdk::ContentProvider::for_value(
                    &format!("node:{}", w.key).to_value(),
                ))
            });
        }
        w.row.add_controller(drag);

        let drop = gtk::DropTarget::new(
            glib::Type::STRING,
            gdk::DragAction::MOVE | gdk::DragAction::COPY,
        );
        drop.set_preload(true);
        let spot_at = {
            let me = Rc::downgrade(self);
            let w = Rc::downgrade(w);
            move |t: &gtk::DropTarget, y: f64| -> Option<(Dropped, Spot)> {
                let (me, w) = (me.upgrade()?, w.upgrade()?);
                let what = dropped(t.value())?;
                let h = f64::from(w.row.height()).max(1.0);
                let at = y / h;
                let spot = match &what {
                    Dropped::Program(_) => {
                        let ok = w.managed.get() && !w.is_folder();
                        return ok.then(|| (what.clone(), Spot::Into(Some(w.key.clone()))));
                    }
                    Dropped::Node(key) => {
                        // Nothing goes into or next to itself or what it
                        // holds.
                        if me.holds(key, &w.key) {
                            return None;
                        }
                        let has_children = me.first_child_of(&w.key).is_some();
                        if !w.managed.get() && !w.is_folder() {
                            return None;
                        }
                        if w.is_folder() {
                            if at < 0.25 {
                                Spot::Before(w.key.clone())
                            } else if at > 0.75 && !(w.open.get() && has_children) {
                                Spot::After(w.key.clone())
                            } else {
                                Spot::Into(Some(w.key.clone()))
                            }
                        } else if at < 0.5 {
                            Spot::Before(w.key.clone())
                        } else {
                            Spot::After(w.key.clone())
                        }
                    }
                };
                Some((what, spot))
            }
        };
        let spot_at = Rc::new(spot_at);
        let motion = {
            let (spot_at, w, me) = (spot_at.clone(), Rc::downgrade(w), Rc::downgrade(self));
            move |t: &gtk::DropTarget, _x: f64, y: f64| {
                let (Some(w), Some(me)) = (w.upgrade(), me.upgrade()) else {
                    return gdk::DragAction::empty();
                };
                let found = spot_at(t, y);
                w.mark(found.as_ref().map(|(_, s)| s));
                // Held over a closed folder: it opens after a moment.
                if w.is_folder() && !w.open.get() && found.is_some() {
                    me.hover_open(&w);
                } else {
                    w.stop_hover();
                }
                match found {
                    Some((Dropped::Program(_), _)) => gdk::DragAction::COPY,
                    Some(_) => gdk::DragAction::MOVE,
                    None => gdk::DragAction::empty(),
                }
            }
        };
        let motion = Rc::new(motion);
        {
            let motion = motion.clone();
            drop.connect_enter(move |t, x, y| motion(t, x, y));
        }
        drop.connect_motion(move |t, x, y| motion(t, x, y));
        {
            let w = Rc::downgrade(w);
            drop.connect_leave(move |_| {
                if let Some(w) = w.upgrade() {
                    w.mark(None);
                    w.stop_hover();
                }
            });
        }
        {
            let (w, me) = (Rc::downgrade(w), Rc::downgrade(self));
            drop.connect_drop(move |t, _, _, y| {
                let (Some(w), Some(me)) = (w.upgrade(), me.upgrade()) else {
                    return false;
                };
                w.mark(None);
                w.stop_hover();
                let Some(found) = spot_at(t, y) else {
                    return false;
                };
                me.on_drop.emit(found);
                true
            });
        }
        w.row.add_controller(drop);
    }

    /// New Library takes a library or folder dropped on it, last at the
    /// top level.
    fn drop_on_new_row(self: &Rc<Self>) {
        let drop = gtk::DropTarget::new(glib::Type::STRING, gdk::DragAction::MOVE);
        drop.set_preload(true);
        let row = self.new_row.clone();
        let accept =
            move |t: &gtk::DropTarget| matches!(dropped(t.value()), Some(Dropped::Node(_)));
        let accept = Rc::new(accept);
        {
            let (accept, row) = (accept.clone(), row.clone());
            drop.connect_motion(move |t, _, _| {
                if accept(t) {
                    row.add_css_class("drop-before");
                    gdk::DragAction::MOVE
                } else {
                    gdk::DragAction::empty()
                }
            });
        }
        {
            let row = row.clone();
            drop.connect_leave(move |_| row.remove_css_class("drop-before"));
        }
        let me = Rc::downgrade(self);
        drop.connect_drop(move |t, _, _, _| {
            row.remove_css_class("drop-before");
            let (Some(me), Some(Dropped::Node(key))) = (me.upgrade(), dropped(t.value())) else {
                return false;
            };
            me.on_drop.emit((Dropped::Node(key), Spot::Into(None)));
            true
        });
        self.new_row.add_controller(drop);
    }

    /// Screenshots: show where a drop would go, as while dragging.
    pub fn demo_mark(&self, spot: Spot) {
        let (Spot::Before(key) | Spot::After(key) | Spot::Into(Some(key))) = &spot else {
            return;
        };
        if let Some(w) = self.widgets(key) {
            // The line's indent needs the row laid out: mark on a frame.
            let w = w.clone();
            self.list.add_tick_callback(move |_, _| {
                if w.body.width() == 0 {
                    return glib::ControlFlow::Continue;
                }
                w.mark(Some(&spot));
                glib::ControlFlow::Break
            });
        }
    }

    /// Whether node `outer` is `inner` or holds it.
    fn holds(&self, outer: &str, inner: &str) -> bool {
        if outer == inner {
            return true;
        }
        let order = self.order.borrow();
        let tree = self.tree.borrow();
        let Some(start) = order.iter().position(|k| k == outer) else {
            return false;
        };
        let depth = tree.get(outer).map_or(0, |w| w.depth.get());
        order[start + 1..]
            .iter()
            .take_while(|k| tree.get(*k).is_some_and(|w| w.depth.get() > depth))
            .any(|k| k == inner)
    }

    /// The first row inside folder `key`, if any.
    fn first_child_of(&self, key: &str) -> Option<String> {
        let order = self.order.borrow();
        let tree = self.tree.borrow();
        let i = order.iter().position(|k| k == key)?;
        let depth = tree.get(key)?.depth.get();
        order
            .get(i + 1)
            .filter(|k| tree.get(*k).is_some_and(|w| w.depth.get() > depth))
            .cloned()
    }

    /// Open the closed folder `w` once something has been held over it
    /// for a moment, measured on the frame clock.
    fn hover_open(self: &Rc<Self>, w: &Rc<TreeWidgets>) {
        if w.hover.borrow().is_some() {
            return;
        }
        let start = Cell::new(None);
        let (me, key) = (Rc::downgrade(self), w.key.clone());
        let id = w.row.add_tick_callback(move |_, clock| {
            let now = clock.frame_time();
            let since = start.get().unwrap_or(now);
            start.set(Some(since));
            if now - since < HOVER_OPEN_US {
                return glib::ControlFlow::Continue;
            }
            if let Some(me) = me.upgrade() {
                if let Some(w) = me.widgets(&key) {
                    w.hover.take();
                }
                me.on_toggle.emit((key.clone(), true));
            }
            glib::ControlFlow::Break
        });
        *w.hover.borrow_mut() = Some(id);
    }

    /// Select `place`'s row without showing it again (it already shows;
    /// its row moved).
    pub fn show_selected(&self, place: &Place) {
        if *place == Place::Guide {
            return;
        }
        self.quiet.set(true);
        self.select(place);
        self.quiet.set(false);
    }

    /// Move keyboard focus to the row of a library or folder.
    pub fn focus_key(&self, key: &str) {
        if let Some(w) = self.widgets(key) {
            w.row.grab_focus();
        }
    }

    /// Put library `i`'s name in an editable field, all selected.
    pub fn start_rename(&self, i: usize) {
        if let Some(w) = self.library_widgets(i) {
            Self::rename_in_place(&w);
        }
    }

    /// Put folder `key`'s name in an editable field, all selected.
    pub fn start_rename_key(&self, key: &str) {
        if let Some(w) = self.widgets(key) {
            Self::rename_in_place(&w);
        }
    }

    fn rename_in_place(w: &TreeWidgets) {
        w.entry.set_text(&w.label.label());
        w.stack.set_visible_child_name("entry");
        w.entry.grab_focus();
        w.entry.select_region(0, -1);
    }

    /// Select `place`'s row. The guide has no row (it lives in the main
    /// menu and F1): selecting it clears the selection and shows it.
    pub fn select(&self, place: &Place) {
        if *place == Place::Guide {
            self.list.unselect_all();
            self.on_place.emit(Place::Guide);
            return;
        }
        let row = match place {
            Place::Library(i) => self.library_widgets(*i).map(|w| w.row.clone()),
            p => self
                .places
                .borrow()
                .iter()
                .find(|(_, q)| q == p)
                .map(|(r, _)| r.clone()),
        };
        if let Some(row) = row {
            self.list.select_row(Some(&row));
        }
    }

    /// Show the libraries tree. Rows keep their widgets by key: rows that
    /// come fold open, rows that go fold shut, rows inside a folder that
    /// opens or closes fold with it, and rows that change places glide
    /// there (all at once unless `animate`).
    pub fn set_tree(self: &Rc<Self>, rows: &[TreeRow], animate: bool) {
        // Nothing to animate before the window shows (the first load).
        let animate = animate && self.list.is_mapped();
        let keys: Vec<String> = rows.iter().map(|r| r.key.clone()).collect();
        let reordered = {
            let old = self.order.borrow();
            let kept: Vec<&String> = old.iter().filter(|k| keys.contains(k)).collect();
            let new: Vec<&String> = keys.iter().filter(|k| old.contains(k)).collect();
            kept != new
        };
        // Rows that stay remember where they are, to glide from there.
        if reordered && animate {
            for w in self.tree.borrow().values() {
                if w.row.is_visible() {
                    w.fold.remember_place(&self.list);
                }
            }
        }
        // Gone: fold shut, then leave the list.
        let gone: Vec<Rc<TreeWidgets>> = {
            let mut tree = self.tree.borrow_mut();
            let gone: Vec<String> = tree.keys().filter(|k| !keys.contains(k)).cloned().collect();
            gone.iter().filter_map(|k| tree.remove(k)).collect()
        };
        for w in gone {
            w.stop_hover();
            let (list, ranks, row) = (self.list.clone(), self.ranks.clone(), w.row.clone());
            w.fold
                .set_open(false, animate && w.row.is_visible(), move || {
                    if row.parent().is_some() {
                        list.remove(&row);
                    }
                    ranks.borrow_mut().remove(&row);
                });
        }
        for (k, r) in rows.iter().enumerate() {
            let folder = matches!(r.kind, RowKind::Folder { .. });
            let existing = self.widgets(&r.key).filter(|w| w.is_folder() == folder);
            let (w, new) = match existing {
                Some(w) => (w, false),
                None => {
                    let w = self.tree_row(&r.key, folder);
                    w.shown.set(false);
                    w.fold.set_open(false, false, || {});
                    w.row.set_visible(false);
                    self.tree.borrow_mut().insert(r.key.clone(), w.clone());
                    self.ranks
                        .borrow_mut()
                        .insert(w.row.clone(), RANK_TREE + k as u32);
                    self.list.append(&w.row);
                    (w, true)
                }
            };
            self.ranks
                .borrow_mut()
                .insert(w.row.clone(), RANK_TREE + k as u32);
            self.fill(&w, r);
            if w.shown.replace(r.shown) != r.shown || new {
                if r.shown {
                    w.row.set_visible(true);
                    w.fold.set_open(true, animate, || {});
                } else {
                    // Focus inside a folder that closes goes to the folder.
                    if w.row.has_focus() || w.row.focus_child().is_some() {
                        self.focus_parent_of(rows, k);
                    }
                    let row = w.row.downgrade();
                    let fold = w.fold.downgrade();
                    w.fold.set_open(false, animate, move || {
                        // Hidden for good once folded (unless it reopened
                        // meanwhile), so the keys skip it.
                        if let (Some(row), Some(fold)) = (row.upgrade(), fold.upgrade())
                            && !fold.is_open()
                        {
                            row.set_visible(false);
                        }
                    });
                }
            }
        }
        *self.order.borrow_mut() = keys;
        self.list.invalidate_sort();
    }

    /// Focus the folder around row `k` of `rows`.
    fn focus_parent_of(&self, rows: &[TreeRow], k: usize) {
        let depth = rows[k].depth;
        if let Some(parent) = rows[..k].iter().rev().find(|r| r.depth < depth && r.shown) {
            self.focus_key(&parent.key);
        }
    }

    /// Show row `r` in `w`.
    fn fill(&self, w: &TreeWidgets, r: &TreeRow) {
        if w.depth.replace(r.depth) != r.depth || w.guides.first_child().is_none() {
            while let Some(c) = w.guides.first_child() {
                w.guides.remove(&c);
            }
            for _ in 0..r.depth {
                let g = gtk::Box::new(gtk::Orientation::Vertical, 0);
                g.add_css_class("tree-guide");
                w.guides.append(&g);
            }
        }
        match &r.kind {
            RowKind::Folder { name, open } => {
                w.open.set(*open);
                w.label.set_label(name);
                if *open {
                    w.lead.add_css_class("open");
                } else {
                    w.lead.remove_css_class("open");
                }
                w.row
                    .update_property(&[gtk::accessible::Property::Label(name)]);
                w.row
                    .update_state(&[gtk::accessible::State::Expanded(Some(*open))]);
            }
            RowKind::Library { index, row: lib } => {
                w.library.set(Some(*index));
                w.managed.set(lib.managed);
                w.label.set_label(&lib.title);
                w.lead.set_icon_name(Some(if lib.broken {
                    "dialog-warning-symbolic"
                } else {
                    "oscyl-library-symbolic"
                }));
                let count = trn("%d program", "%d programs", lib.count as u32);
                w.caption.set_label(&if lib.broken {
                    tr("Couldn't read")
                } else {
                    count
                });
                w.caption.set_visible(lib.managed);
                w.dot.set_visible(lib.unsaved);
                let mut said = lib.title.clone();
                if lib.broken {
                    said = tr("{title}, couldn't read").replace("{title}", &lib.title);
                } else if lib.unsaved {
                    said = tr("{title}, unsaved edits").replace("{title}", &lib.title);
                }
                w.row
                    .update_property(&[gtk::accessible::Property::Label(&said)]);
            }
        }
    }

    /// The connected synths (id and name) and the one shown; the picker
    /// shows with two or more.
    pub fn set_synths(&self, synths: &[(u64, String)], active: Option<u64>) {
        self.picker_quiet.set(true);
        let names: Vec<&str> = synths.iter().map(|(_, n)| n.as_str()).collect();
        self.picker.set_model(Some(&gtk::StringList::new(&names)));
        *self.picker_ids.borrow_mut() = synths.iter().map(|(id, _)| *id).collect();
        if let Some(i) = synths.iter().position(|(id, _)| Some(*id) == active) {
            self.picker.set_selected(i as u32);
        }
        let several = synths.len() > 1;
        self.picker.set_visible(several);
        self.status_title.set_visible(!several);
        self.picker_quiet.set(false);
    }

    /// The user picked synth `id` in the picker.
    pub fn connect_pick(&self, f: impl Fn(u64) + 'static) {
        self.on_pick.connect(f);
    }

    /// Show or hide the user units place (a synth without units).
    pub fn set_units_shown(&self, shown: bool) {
        if let Some((row, _)) = self
            .places
            .borrow()
            .iter()
            .find(|(_, p)| *p == Place::Units)
        {
            row.set_visible(shown);
        }
    }

    /// The status card: `title` names the synth shown.
    pub fn set_status(&self, title: &str, s: &SynthStatus) {
        let count = |text: String, done: &usize, total: &usize| {
            text.replace("{done}", &done.to_string())
                .replace("{total}", &total.to_string())
        };
        // The row names the synth; the state line says how it is reached.
        let mut firmware_tip = None;
        let (detail, class, busy) = match s {
            SynthStatus::Disconnected => (tr("Not connected"), "off", None),
            SynthStatus::Connecting => (tr("Connecting…"), "busy", None),
            SynthStatus::Connected {
                firmware,
                simulated,
                ..
            } => {
                firmware_tip = (!firmware.is_empty())
                    .then(|| tr("Firmware {firmware}").replace("{firmware}", firmware));
                (
                    if *simulated {
                        tr("Simulated, no sound")
                    } else {
                        tr("Connected")
                    },
                    "on",
                    None,
                )
            }
            SynthStatus::Reading { done, total } => (
                count(tr("Reading {done} of {total}"), done, total),
                "busy",
                Some(*done as f64 / (*total).max(1) as f64),
            ),
            SynthStatus::Writing { done, total } => (
                count(tr("Syncing {done} of {total}"), done, total),
                "busy",
                Some(*done as f64 / (*total).max(1) as f64),
            ),
        };
        self.status_title.set_label(title);
        self.status_detail.set_label(&detail);
        self.status_detail.set_tooltip_text(firmware_tip.as_deref());
        for c in ["on", "off", "busy"] {
            self.status_dot.remove_css_class(c);
        }
        self.status_dot.add_css_class(class);
        self.progress.set_visible(busy.is_some());
        self.progress.set_fraction(busy.unwrap_or(0.0));
        let connected = !matches!(s, SynthStatus::Disconnected | SynthStatus::Connecting);
        self.connect.set_label(&if connected {
            tr("Disconnect")
        } else {
            tr("Connect")
        });
        self.connect.set_action_name(Some(if connected {
            "win.disconnect"
        } else {
            "win.connect"
        }));
        self.connect
            .set_sensitive(!matches!(s, SynthStatus::Connecting));
        // While transferring, the only action is stopping a read.
        self.connect.set_visible(busy.is_none());
        self.cancel
            .set_visible(matches!(s, SynthStatus::Reading { .. }));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lib(key: &str, index: usize, depth: usize, shown: bool) -> TreeRow {
        TreeRow {
            key: key.into(),
            depth,
            shown,
            kind: RowKind::Library {
                index,
                row: LibRow {
                    title: key.to_uppercase(),
                    managed: true,
                    count: 3,
                    ..LibRow::default()
                },
            },
        }
    }

    fn folder(key: &str, depth: usize, open: bool, shown: bool) -> TreeRow {
        TreeRow {
            key: key.into(),
            depth,
            shown,
            kind: RowKind::Folder {
                name: key.to_uppercase(),
                open,
            },
        }
    }

    /// Tree rows follow the rows given (in order, indented one guide per
    /// level, hidden inside a closed folder), and renaming in place
    /// reports the key and the new name (Escape keeps the old one). Needs
    /// a display; skipped without.
    #[test]
    fn tree_rows_indent_collapse_and_rename_in_place() {
        crate::on_gtk_thread(|| {
            let s = Sidebar::new();
            let renamed = Rc::new(RefCell::new(Vec::new()));
            let r = renamed.clone();
            s.connect_rename(move |x| r.borrow_mut().push(x));
            s.set_tree(
                &[
                    lib("a", 0, 0, true),
                    folder("f", 0, true, true),
                    lib("b", 1, 1, true),
                    folder("g", 1, false, true),
                    lib("c", 2, 2, false),
                ],
                false,
            );
            let w = |k: &str| s.widgets(k).expect("a row");
            assert_eq!(s.tree.borrow().len(), 5);
            let guides = |k: &str| {
                let mut n = 0;
                let mut c = w(k).guides.first_child();
                while let Some(g) = c {
                    n += 1;
                    c = g.next_sibling();
                }
                n
            };
            assert_eq!((guides("a"), guides("b"), guides("c")), (0, 1, 2));
            assert_eq!(w("b").caption.label(), "3 programs");
            assert!(w("b").row.is_visible() && !w("c").row.is_visible());
            assert!(w("f").lead.has_css_class("open"));
            assert!(!w("g").lead.has_css_class("open"));
            let index = |k: &str| w(k).row.index();
            assert!(index("a") < index("f") && index("f") < index("b"));
            assert!(index("c") < s.new_row.index(), "above New Library");
            assert_eq!(s.place_of(&w("b").row), Some(Place::Library(1)));
            assert_eq!(s.place_of(&w("f").row), None, "folders aren't places");
            assert!(s.holds("f", "c") && !s.holds("g", "b") && s.holds("g", "g"));

            // Closing F folds B, G and C away; moving A into G reorders.
            s.set_tree(
                &[
                    folder("f", 0, false, true),
                    lib("b", 0, 1, false),
                    folder("g", 1, false, false),
                    lib("c", 1, 2, false),
                    lib("a", 2, 2, false),
                ],
                false,
            );
            assert!(!w("b").row.is_visible() && !w("g").row.is_visible());
            assert!(!w("f").lead.has_css_class("open"));
            assert!(index("f") < index("a") && index("c") < index("a"));
            assert_eq!(guides("a"), 2);

            s.start_rename_key("f");
            let f = w("f");
            assert_eq!(f.stack.visible_child_name().as_deref(), Some("entry"));
            f.entry.set_text("Live");
            f.entry.emit_activate();
            assert_eq!(*renamed.borrow(), [("f".to_string(), "Live".to_string())]);
            assert_eq!(f.stack.visible_child_name().as_deref(), Some("label"));

            s.set_tree(&[lib("a", 0, 0, true)], false);
            assert_eq!(s.tree.borrow().len(), 1);
            assert_eq!(s.place_of(&w("a").row), Some(Place::Library(0)));
        });
    }
}
