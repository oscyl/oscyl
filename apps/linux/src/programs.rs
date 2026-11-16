//! The program list: search, filters and a virtualised list of slots.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::{Rc, Weak};

use adw::prelude::*;
use gtk::subclass::prelude::ObjectSubclassIsExt;
use gtk::{gdk, gio, glib};

use crate::i18n::{tr, trn};
use crate::state::{Reorder, arrangement};
use crate::widgets::{Fold, Graph, GraphData, Line};

/// Where rows get their data, by program index.
pub trait RowSource {
    /// Everything a row shows.
    fn row(&self, i: usize) -> RowData;
    /// The value of the driver's list facet (a voice mode) and the edited
    /// flag: asked for every program on each refilter, so no formatting.
    fn facts(&self, i: usize) -> (Option<u16>, bool);
    fn name(&self, i: usize) -> String;
    /// The slot label, as searched.
    fn slot(&self, i: usize) -> String;
    /// The program's category, once it is loaded.
    fn category(&self, i: usize) -> Option<Category>;
    /// Program `i`'s menu is about to open, or a key acts on it: enable
    /// what applies to it.
    fn prepare_menu(&self, i: usize);
    /// Program `i`'s menu.
    fn menu(&self, i: usize) -> gio::Menu;
    /// Program `i` can be picked up (to copy to a library, or move).
    fn can_drag(&self, i: usize) -> bool;
    /// Programs can be put in another order, or why not.
    fn reorderable(&self) -> Result<(), String>;
    /// How many places change once program `from` goes to `to`.
    fn reorder_changes(&self, from: usize, to: usize, how: Reorder) -> usize;
}

/// What a row shows. Provided by the app for an index on demand.
#[derive(Debug, Clone, Default)]
pub struct RowData {
    pub slot: String,
    pub name: String,
    pub edited: bool,
    /// The program's first picture (a read program only).
    pub picture: Option<GraphData>,
}

/// A category name, as the driver gives it.
pub type Category = &'static str;

/// A row arriving on its next bind: sliding from `dy` px, or fading in.
#[derive(Debug, Clone, Copy)]
enum Arrive {
    Slide(f64),
    Fade,
}

/// Where a dragged program would land.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Target {
    /// Between rows: before the program now at this index.
    Gap(usize),
    /// Onto a row: exchanged with that program.
    Swap(usize),
}

impl Target {
    /// The move it makes for program `from`, unless it leaves it in place.
    fn reorder(self, from: usize) -> Option<(usize, Reorder)> {
        let (to, how) = match self {
            Target::Gap(g) if g > from => (g - 1, Reorder::Move),
            Target::Gap(g) => (g, Reorder::Move),
            Target::Swap(q) => (q, Reorder::Swap),
        };
        (to != from).then_some((to, how))
    }
}

/// A program being dragged.
#[derive(Default)]
struct Drag {
    /// The program picked up.
    from: Option<usize>,
    /// Why it can't be reordered here (it can still go to a library).
    refused: Option<String>,
    target: Option<Target>,
    /// The picked-up row's height: what the rows move apart by.
    height: f64,
    /// The pointer, in the list's coordinates, while over it.
    pointer: Option<f64>,
    /// Says what a drop would do, under the dragged row.
    legend: Option<gtk::Label>,
    /// Scrolling near the top or bottom edge: px per second.
    speed: f64,
    scroll: Option<gtk::TickCallbackId>,
    last_frame: i64,
}

/// Near this many px from the list's top or bottom, a drag scrolls it.
const EDGE: f64 = 48.0;
/// The fastest it scrolls, at the very edge, in px per second.
const EDGE_SPEED: f64 = 900.0;

/// What the list shows: programs with one value of the driver's facet
/// (a voice mode) or all, and optionally only edits.
#[derive(Debug, Clone, Copy, Default)]
struct Filter {
    voice: Option<u16>,
    edited_only: bool,
}

/// A category's name as the list shows it.
pub fn category_name(c: Category) -> String {
    tr(c)
}

type Source = Weak<dyn RowSource>;

/// How the visible rows group into category runs.
#[derive(Default)]
struct Groups {
    /// Each program's category, by index.
    cats: Vec<Option<Category>>,
    /// Programs whose category differs from the visible row above.
    first: Vec<bool>,
    /// The library is ordered by category: runs get section headers and
    /// rows no subtitle. Otherwise a row names its category only where it
    /// differs from the row above.
    grouped: bool,
}

impl Groups {
    /// The category and the subtitle program `i` shows.
    fn of(&self, i: usize) -> (Option<Category>, String) {
        let c = self.cats.get(i).copied().flatten();
        let sub = match c {
            Some(c) if !self.grouped && self.first.get(i) == Some(&true) => category_name(c),
            _ => String::new(),
        };
        (c, sub)
    }
}

pub struct ProgramList {
    pub root: gtk::Box,
    pub search: gtk::SearchEntry,
    store: gio::ListStore,
    filter: gtk::CustomFilter,
    selection: gtk::SingleSelection,
    stack: gtk::Stack,
    runs: Runs,
    groups: Rc<RefCell<Groups>>,
    to_top: Rc<Cell<bool>>,
    /// The pane's title; its subtitle counts the programs shown.
    title: adw::WindowTitle,
    source: Source,
    bound: Rc<RefCell<HashMap<usize, gtk::ListItem>>>,
    shown: Rc<RefCell<Filter>>,
    list: gtk::ListView,
    plated: crate::widgets::Plated,
    voice: gtk::DropDown,
    /// The facet value of each dropdown entry after "All".
    facet: RefCell<Vec<u16>>,
    edited: gtk::ToggleButton,
    /// The last selection change came from the keyboard (arrows, Page
    /// keys): it updates the editor without leaving the list on a phone.
    key_nav: Rc<Cell<bool>>,
    on_select: crate::callback::Slot<usize>,
    /// A program was opened (click, Enter): show the editor.
    on_open: crate::callback::Slot<usize>,
    /// A program was dropped at a new place: from, to, how.
    on_reorder: crate::callback::Slot<(usize, usize, Reorder)>,
    drag: RefCell<Drag>,
    /// Rows to animate when they are next bound, and the frame they were
    /// asked for in (older ones are stale).
    arriving: Rc<RefCell<(i64, HashMap<usize, Arrive>)>>,
}

impl ProgramList {
    pub fn new(source: Source, title: adw::WindowTitle) -> Rc<ProgramList> {
        let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
        root.add_css_class("program-list");

        let search = gtk::SearchEntry::builder()
            .placeholder_text(tr("Search programs"))
            .margin_start(12)
            .margin_end(12)
            .margin_top(8)
            .margin_bottom(8)
            .build();
        search.update_property(&[gtk::accessible::Property::Label(&tr("Search programs"))]);
        root.append(&search);

        // One row: the voice mode, then an on/off for edited programs.
        let filters = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        filters.add_css_class("filter-chips");
        // The driver's facet fills it (`set_facet`); hidden without one.
        let voice = gtk::DropDown::from_strings(&[]);
        voice.set_visible(false);
        let edited = gtk::ToggleButton::builder()
            .label(tr("Edited only"))
            .build();
        edited.add_css_class("chip");
        filters.append(&voice);
        filters.append(&edited);
        root.append(&filters);

        let store = gio::ListStore::new::<glib::BoxedAnyObject>();
        let shown = Rc::new(RefCell::new(Filter::default()));
        let query = Rc::new(RefCell::new(String::new()));
        let filter = {
            let (source, shown, query) = (source.clone(), shown.clone(), query.clone());
            gtk::CustomFilter::new(move |obj| {
                let (Some(i), Some(src)) = (index_of(obj), source.upgrade()) else {
                    return false;
                };
                let f = *shown.borrow();
                let filter_ok = if f.voice.is_some() || f.edited_only {
                    let (voice, edited) = src.facts(i);
                    f.voice.is_none_or(|v| Some(v) == voice) && (edited || !f.edited_only)
                } else {
                    true
                };
                let q = query.borrow();
                filter_ok
                    && (q.is_empty()
                        || src.slot(i).contains(q.as_str())
                        || src.name(i).to_lowercase().contains(q.as_str()))
            })
        };
        let filtered = gtk::FilterListModel::new(Some(store.clone()), Some(filter.clone()));
        let runs = Runs::new(&filtered);
        let selection = gtk::SingleSelection::builder()
            .model(&runs)
            .autoselect(false)
            .can_unselect(false)
            .build();

        // The list itself, once made, for the rows' drag handlers.
        let me: Rc<std::cell::OnceCell<Weak<ProgramList>>> = Rc::default();
        let factory = gtk::SignalListItemFactory::new();
        {
            let (source, me) = (source.clone(), me.clone());
            factory.connect_setup(move |_, item| {
                let item = item.downcast_ref::<gtk::ListItem>().expect("list item");
                let row = row_widget();
                let fold = drag_row(&row, item, &source, &me);
                // Right click or long press opens the program's menu at the
                // pointer.
                let open = {
                    let (source, item) = (source.clone(), item.downgrade());
                    move |row: &gtk::Widget, x: f64, y: f64| {
                        let (Some(item), Some(src)) = (item.upgrade(), source.upgrade()) else {
                            return;
                        };
                        if let Some(i) = item.item().as_ref().and_then(index_of) {
                            src.prepare_menu(i);
                            popup(row, &src.menu(i), Some((x, y)));
                        }
                    }
                };
                let click = gtk::GestureClick::builder()
                    .button(gdk::BUTTON_SECONDARY)
                    .build();
                {
                    let open = open.clone();
                    click.connect_pressed(move |g, _, x, y| {
                        if let Some(w) = g.widget() {
                            open(&w, x, y);
                        }
                    });
                }
                row.add_controller(click);
                let press = gtk::GestureLongPress::new();
                press.set_touch_only(true);
                press.connect_pressed(move |g, x, y| {
                    if let Some(w) = g.widget() {
                        open(&w, x, y);
                    }
                });
                row.add_controller(press);
                item.set_child(Some(&fold));
            });
        }
        // Rows on screen, by program index, so a change to one program
        // updates its row in place (GTK keeps rows bound to unchanged items).
        let bound: Rc<RefCell<HashMap<usize, gtk::ListItem>>> = Default::default();
        let groups: Rc<RefCell<Groups>> = Default::default();
        let arriving: Rc<RefCell<(i64, HashMap<usize, Arrive>)>> = Rc::default();
        {
            let (source, bound, groups) = (source.clone(), bound.clone(), groups.clone());
            let (arriving, me) = (arriving.clone(), me.clone());
            factory.connect_bind(move |_, item| {
                let item = item.downcast_ref::<gtk::ListItem>().expect("list item");
                let (Some(i), Some(src)) =
                    (item.item().as_ref().and_then(index_of), source.upgrade())
                else {
                    return;
                };
                fill(item, &src.row(i), groups.borrow().of(i));
                bound.borrow_mut().insert(i, item.clone());
                // A recycled row starts at rest, unless it is arriving or
                // a drag is moving rows apart.
                let Some(fold) = item.child().and_downcast::<Fold>() else {
                    return;
                };
                fold.shift_to(0.0, false);
                fold.fade_to(1.0, false);
                fold.set_line(None, false);
                let frame = fold.frame_clock().map_or(0, |c| c.frame_counter());
                let arrive = {
                    let mut a = arriving.borrow_mut();
                    // Asked for in this frame or the next one only.
                    if frame > a.0 + 1 {
                        a.1.clear();
                    }
                    a.1.remove(&i)
                };
                match arrive {
                    Some(Arrive::Slide(dy)) => fold.slide_from(dy),
                    Some(Arrive::Fade) => fold.fade_in(),
                    None => {}
                }
                if let Some(me) = me.get().and_then(Weak::upgrade)
                    && me.drag.borrow().from.is_some()
                {
                    me.layout_drag(false);
                }
            });
        }
        {
            let bound = bound.clone();
            factory.connect_unbind(move |_, item| {
                let item = item.downcast_ref::<gtk::ListItem>().expect("list item");
                bound.borrow_mut().retain(|_, v| v != item);
            });
        }
        let list = gtk::ListView::builder()
            .model(&selection)
            .factory(&factory)
            .single_click_activate(false)
            .build();
        list.add_css_class("navigation-sidebar");
        list.update_property(&[gtk::accessible::Property::Label(&tr("Programs"))]);
        // Moving through the list by keys only updates the editor; a
        // click or Enter opens it (on a phone the list stays in view).
        let key_nav = Rc::new(Cell::new(false));
        {
            let nav = gtk::EventControllerKey::new();
            nav.set_propagation_phase(gtk::PropagationPhase::Capture);
            let key_nav = key_nav.clone();
            nav.connect_key_pressed(move |_, key, _, _| {
                use gdk::Key as K;
                if matches!(
                    key,
                    K::Up
                        | K::Down
                        | K::Page_Up
                        | K::Page_Down
                        | K::Home
                        | K::End
                        | K::KP_Up
                        | K::KP_Down
                        | K::KP_Page_Up
                        | K::KP_Page_Down
                        | K::KP_Home
                        | K::KP_End
                ) {
                    key_nav.set(true);
                }
                glib::Propagation::Proceed
            });
            list.add_controller(nav);
        }
        // The selected program's keys: Delete clears it (or, in a library,
        // removes it), Alt+Up and Alt+Down move it, Ctrl+Alt+Up and
        // Ctrl+Alt+Down swap it with its neighbour, Ctrl+C
        // and Ctrl+V copy and paste, Menu or Shift+F10 opens its menu.
        {
            let keys = gtk::EventControllerKey::new();
            let (source, selection) = (source.clone(), selection.clone());
            keys.connect_key_pressed(move |c, key, _, state| {
                let Some(i) = selection.selected_item().as_ref().and_then(index_of) else {
                    return glib::Propagation::Proceed;
                };
                let ctrl = state.contains(gdk::ModifierType::CONTROL_MASK);
                let shift = state.contains(gdk::ModifierType::SHIFT_MASK);
                let alt = state.contains(gdk::ModifierType::ALT_MASK);
                let action = match key {
                    gdk::Key::Delete | gdk::Key::KP_Delete if !ctrl => Some("program-delete"),
                    gdk::Key::Up | gdk::Key::KP_Up if alt && ctrl => Some("program-swap-up"),
                    gdk::Key::Down | gdk::Key::KP_Down if alt && ctrl => Some("program-swap-down"),
                    gdk::Key::Up | gdk::Key::KP_Up if alt => Some("program-move-up"),
                    gdk::Key::Down | gdk::Key::KP_Down if alt => Some("program-move-down"),
                    gdk::Key::c if ctrl => Some("program-copy"),
                    gdk::Key::v if ctrl => Some("program-paste"),
                    gdk::Key::Menu => None,
                    gdk::Key::F10 if shift => None,
                    _ => return glib::Propagation::Proceed,
                };
                let Some(src) = source.upgrade() else {
                    return glib::Propagation::Proceed;
                };
                src.prepare_menu(i);
                let Some(w) = c.widget() else {
                    return glib::Propagation::Proceed;
                };
                match action {
                    Some(a) => {
                        let _ =
                            w.activate_action(&format!("win.{a}"), Some(&(i as i32).to_variant()));
                    }
                    None => {
                        // Open at the focused row.
                        let row = w.focus_child().unwrap_or(w.clone());
                        popup(&row, &src.menu(i), None);
                    }
                }
                glib::Propagation::Stop
            });
            list.add_controller(keys);
        }
        // Section headers name each category run while grouped; otherwise
        // the list is one section whose header stays empty.
        let headers = gtk::SignalListItemFactory::new();
        headers.connect_setup(|_, h| {
            let h = h.downcast_ref::<gtk::ListHeader>().expect("list header");
            let label = gtk::Label::builder().xalign(0.0).build();
            label.add_css_class("program-section");
            label.add_css_class("caption-heading");
            label.add_css_class("dim-label");
            h.set_child(Some(&label));
        });
        {
            let groups = groups.clone();
            headers.connect_bind(move |_, h| {
                let h = h.downcast_ref::<gtk::ListHeader>().expect("list header");
                let g = groups.borrow();
                let c = h
                    .item()
                    .as_ref()
                    .and_then(index_of)
                    .and_then(|i| g.of(i).0)
                    .filter(|_| g.grouped);
                if let Some(label) = h.child().and_downcast::<gtk::Label>() {
                    label.set_label(&c.map(category_name).unwrap_or_default());
                    label.set_visible(c.is_some());
                }
            });
        }
        let scroll = gtk::ScrolledWindow::builder()
            .child(&list)
            .vexpand(true)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .build();
        crate::widgets::smooth_wheel(&scroll);

        // A new library opens at its top, first header included: GTK
        // anchors on the first row and would leave its header above, so
        // scroll once the list has laid out the new rows.
        let to_top = Rc::new(Cell::new(false));
        if let Some(adj) = list.vadjustment() {
            let to_top = to_top.clone();
            adj.connect_changed(move |adj| {
                if to_top.replace(false) {
                    adj.set_value(adj.lower());
                }
            });
        }

        list.set_header_factory(Some(&headers));
        let clear = gtk::Button::builder()
            .label(tr("Clear Search and Filters"))
            .halign(gtk::Align::Center)
            .build();
        clear.add_css_class("pill");
        let empty = adw::StatusPage::builder()
            .icon_name("system-search-symbolic")
            .title(tr("No matching programs"))
            .description(tr(
                "No program matches the search and the filters together.",
            ))
            .child(&clear)
            .build();
        empty.add_css_class("compact");
        // The selected row is the plate, sliding from the old row to the
        // new one and riding along when the list scrolls.
        let plated = {
            let (selection, bound) = (selection.downgrade(), bound.clone());
            crate::widgets::Plated::new(&scroll, 0.0, "raised", move || {
                let i = selection
                    .upgrade()?
                    .selected_item()
                    .as_ref()
                    .and_then(index_of)?;
                bound.borrow().get(&i)?.child()?.parent()
            })
        };
        if let Some(adj) = list.vadjustment() {
            plated.follow_scroll(&adj);
        }
        {
            let plated = plated.downgrade();
            selection.connect_selected_item_notify(move |_| {
                if let Some(p) = plated.upgrade() {
                    p.place(false);
                }
            });
        }
        let stack = gtk::Stack::new();
        stack.add_named(&plated, Some("list"));
        stack.add_named(&empty, Some("empty"));
        stack.set_vexpand(true);
        root.append(&stack);

        let this = Rc::new(ProgramList {
            root,
            search,
            store,
            filter,
            selection,
            stack,
            runs,
            groups,
            to_top,
            title,
            source,
            bound,
            shown,
            list: list.clone(),
            plated: plated.clone(),
            voice: voice.clone(),
            facet: RefCell::default(),
            edited: edited.clone(),
            key_nav,
            on_select: Default::default(),
            on_open: Default::default(),
            on_reorder: Default::default(),
            drag: RefCell::default(),
            arriving,
        });
        let _ = me.set(Rc::downgrade(&this));
        this.install_drop();
        {
            let me = Rc::downgrade(&this);
            clear.connect_clicked(move |_| {
                if let Some(me) = me.upgrade() {
                    me.clear_filters();
                }
            });
        }
        {
            let me = Rc::downgrade(&this);
            list.connect_activate(move |_, pos| {
                if let Some(me) = me.upgrade()
                    && let Some(i) = me.selection.item(pos).as_ref().and_then(index_of)
                {
                    me.on_open.emit(i);
                }
            });
        }
        // Enter or Down in the search goes to the first match; typing
        // in the list searches.
        {
            let me = Rc::downgrade(&this);
            this.search.connect_activate(move |_| {
                if let Some(me) = me.upgrade() {
                    me.focus_first();
                }
            });
            let keys = gtk::EventControllerKey::new();
            let me = Rc::downgrade(&this);
            keys.connect_key_pressed(move |_, key, _, _| {
                if matches!(key, gdk::Key::Down | gdk::Key::KP_Down)
                    && let Some(me) = me.upgrade()
                {
                    me.focus_first();
                    return glib::Propagation::Stop;
                }
                glib::Propagation::Proceed
            });
            this.search.add_controller(keys);
            this.search.set_key_capture_widget(Some(&list));
        }

        {
            let me = Rc::downgrade(&this);
            voice.connect_selected_notify(move |d| {
                if let Some(me) = me.upgrade() {
                    let i = d.selected() as usize;
                    let v = i
                        .checked_sub(1)
                        .and_then(|i| me.facet.borrow().get(i).copied());
                    me.shown.borrow_mut().voice = v;
                    me.refilter();
                }
            });
        }
        {
            let me = Rc::downgrade(&this);
            edited.connect_toggled(move |b| {
                if let Some(me) = me.upgrade() {
                    me.shown.borrow_mut().edited_only = b.is_active();
                    me.refilter();
                }
            });
        }
        {
            let me = Rc::downgrade(&this);
            this.search.connect_search_changed(move |e| {
                if let Some(me) = me.upgrade() {
                    *query.borrow_mut() = e.text().to_lowercase();
                    me.refilter();
                }
            });
        }
        {
            let me = Rc::downgrade(&this);
            this.selection.connect_selected_item_notify(move |sel| {
                if let (Some(me), Some(i)) = (
                    me.upgrade(),
                    sel.selected_item().as_ref().and_then(index_of),
                ) {
                    me.on_select.emit(i);
                }
            });
        }
        this
    }

    pub fn connect_select(&self, f: impl Fn(usize) + 'static) {
        self.on_select.connect(f);
    }

    /// A program was dragged to a new place: from, to, how.
    pub fn connect_reorder(&self, f: impl Fn((usize, usize, Reorder)) + 'static) {
        self.on_reorder.connect(f);
    }

    /// Program `i`'s row content, if it is bound.
    fn fold(&self, i: usize) -> Option<Fold> {
        self.bound.borrow().get(&i)?.child().and_downcast()
    }

    /// Program `i`'s row: top and height in the list's coordinates.
    fn span(&self, i: usize) -> Option<(f64, f64)> {
        let row = self.fold(i)?.parent()?;
        let b = row.compute_bounds(&self.list)?;
        Some((f64::from(b.y()), f64::from(b.height())))
    }

    /// Animate rows that will show other programs once laid out: each
    /// slides from `dy` px or fades in. Rows on screen start now; rows the
    /// list binds in this frame or the next start when bound.
    fn arrive(&self, rows: impl IntoIterator<Item = (usize, Arrive)>) {
        let frame = self.list.frame_clock().map_or(0, |c| c.frame_counter());
        let mut later = HashMap::new();
        for (i, a) in rows {
            match (self.fold(i), a) {
                (Some(f), Arrive::Slide(dy)) => f.slide_from(dy),
                (Some(f), Arrive::Fade) => f.fade_in(),
                (None, a) => {
                    later.insert(i, a);
                }
            }
        }
        *self.arriving.borrow_mut() = (frame, later);
    }

    /// A row's height for animating program `i`'s neighbours (rows the
    /// list hasn't laid out take the height of one that has).
    fn row_height(&self, i: usize) -> f64 {
        self.span(i)
            .or_else(|| {
                let b = self.bound.borrow();
                let k = *b.keys().next()?;
                drop(b);
                self.span(k)
            })
            .map_or(0.0, |(_, h)| h)
    }

    /// Program `from` went to `to` (Alt+Up, Alt+Down, Ctrl+Alt+Up,
    /// Ctrl+Alt+Down): the programs slide to their new places instead of
    /// changing in place.
    pub fn moved(&self, from: usize, to: usize, how: Reorder) {
        if from == to {
            return;
        }
        let h = self.row_height(to);
        let rows: Vec<_> = arrangement(from, to, how)
            .into_iter()
            .map(|(p, was)| (p, Arrive::Slide((was as f64 - p as f64) * h)))
            .collect();
        self.arrive(rows);
    }

    /// A program was added at `at`: it fades in and the rows below slide
    /// down to make room.
    pub fn inserted(&self, at: usize) {
        let h = self.row_height(at);
        let n = self.store.n_items() as usize;
        let rows =
            std::iter::once((at, Arrive::Fade)).chain((at + 1..n).map(|p| (p, Arrive::Slide(-h))));
        self.arrive(rows.collect::<Vec<_>>());
    }

    /// The program at `at` was removed: the rows below slide up into its
    /// place.
    pub fn removed(&self, at: usize) {
        let h = self.row_height(at);
        let n = self.store.n_items() as usize;
        self.arrive((at..n).map(|p| (p, Arrive::Slide(h))).collect::<Vec<_>>());
    }

    /// Programs can be dragged to a new order now, or why not: the
    /// document must allow it, and the list must show every program in
    /// order (no search, filter or category runs).
    fn can_reorder(&self) -> Result<(), String> {
        self.source
            .upgrade()
            .map_or(Err(String::new()), |s| s.reorderable())?;
        let f = *self.shown.borrow();
        if f.voice.is_some() || f.edited_only || !self.search.text().is_empty() {
            return Err(tr("Clear the search and filters to reorder"));
        }
        if self.groups.borrow().grouped {
            return Err(tr("Programs grouped by category keep their order"));
        }
        Ok(())
    }

    /// The list takes a dragged program of its own at a new place.
    fn install_drop(self: &Rc<Self>) {
        let drop = gtk::DropTarget::new(glib::Type::STRING, gdk::DragAction::MOVE);
        let me = Rc::downgrade(self);
        drop.connect_motion(move |_, _, y| {
            let Some(me) = me.upgrade().filter(|m| m.drag.borrow().from.is_some()) else {
                return gdk::DragAction::empty();
            };
            me.drag.borrow_mut().pointer = Some(y);
            if me.drag.borrow().refused.is_some() {
                return gdk::DragAction::empty();
            }
            me.autoscroll(y);
            me.layout_drag(true);
            gdk::DragAction::MOVE
        });
        let me = Rc::downgrade(self);
        drop.connect_leave(move |_| {
            if let Some(me) = me.upgrade() {
                me.stop_scroll();
                me.drag.borrow_mut().pointer = None;
                me.layout_drag(true);
            }
        });
        let me = Rc::downgrade(self);
        drop.connect_drop(move |_, _, _, _| {
            let Some(me) = me.upgrade() else { return false };
            me.stop_scroll();
            let (from, target) = {
                let mut d = me.drag.borrow_mut();
                (d.from.take(), d.target.take())
            };
            let Some((from, (to, how))) = from.and_then(|f| Some((f, target?.reorder(f)?))) else {
                me.settle(true);
                return true;
            };
            match how {
                // The rows already stand in the new order: the drop shows
                // it as it is, and the landed row fades in where the line
                // was.
                Reorder::Move => {
                    me.settle(false);
                    me.on_reorder.emit((from, to, how));
                    me.arrive([(to, Arrive::Fade)]);
                }
                // The two programs glide to each other's place.
                Reorder::Swap => {
                    me.settle(false);
                    me.on_reorder.emit((from, to, how));
                    me.moved(from, to, how);
                }
            }
            true
        });
        self.list.add_controller(drop);
    }

    /// Every row back at rest, gliding or at once, and the plate shown.
    fn settle(&self, animate: bool) {
        self.plated.set_hidden(false);
        for item in self.bound.borrow().values() {
            if let Some(f) = item.child().and_downcast::<Fold>() {
                f.shift_to(0.0, animate);
                f.fade_to(1.0, animate);
                f.set_line(None, animate);
            }
            if let Some(row) = item.child().as_ref().and_then(row_box) {
                row.remove_css_class("swap-target");
            }
        }
    }

    /// A row was picked up; `legend` sits under it on the pointer.
    fn drag_begin(&self, i: usize, legend: gtk::Label) {
        let refused = self.can_reorder().err();
        legend.set_label(refused.as_deref().unwrap_or_default());
        legend.set_visible(refused.as_deref().is_some_and(|r| !r.is_empty()));
        let height = self.span(i).map_or(0.0, |(_, h)| h);
        // The rows move under the plate: it waits, then glides to where
        // the program lands.
        self.plated.set_hidden(refused.is_none());
        *self.drag.borrow_mut() = Drag {
            from: Some(i),
            refused,
            height,
            legend: Some(legend),
            ..Drag::default()
        };
    }

    /// The drag ended; unless it landed in the list, everything glides
    /// back.
    fn drag_end(&self) {
        self.stop_scroll();
        let d = std::mem::take(&mut *self.drag.borrow_mut());
        if d.from.is_some() {
            self.settle(true);
        }
    }

    /// Where the pointer at `y` would put program `from`: between rows
    /// near a row's top or bottom quarter, onto it in its middle half.
    fn target_at(spans: &[(usize, f64, f64)], from: usize, y: f64) -> Option<Target> {
        let first = spans.first()?;
        if y < first.1 {
            return Some(Target::Gap(first.0));
        }
        let Some(&(i, top, h)) = spans.iter().find(|s| y < s.1 + s.2) else {
            return spans.last().map(|s| Target::Gap(s.0 + 1));
        };
        let at = (y - top) / h.max(1.0);
        Some(if at < 0.25 {
            Target::Gap(i)
        } else if at > 0.75 {
            Target::Gap(i + 1)
        } else if i == from {
            Target::Gap(from)
        } else {
            Target::Swap(i)
        })
    }

    /// Show where the picked-up program would land: between rows, the
    /// rows move apart and the insertion line sits in the gap; onto a row,
    /// that row is outlined. The legend says what a drop would change.
    /// Outside the list, everything is back at rest.
    fn layout_drag(&self, animate: bool) {
        let (Some(from), pointer, h, None) = ({
            let d = self.drag.borrow();
            (d.from, d.pointer, d.height, d.refused.clone())
        }) else {
            return;
        };
        let mut spans: Vec<(usize, f64, f64)> = self
            .bound
            .borrow()
            .keys()
            .filter_map(|&i| self.span(i).map(|(y, h)| (i, y, h)))
            .collect();
        spans.sort_by_key(|s| s.0);
        let target = pointer.and_then(|y| Self::target_at(&spans, from, y));
        self.drag.borrow_mut().target = target;
        self.say_drop(from, target);
        let gap = match target {
            Some(Target::Gap(g)) => Some(g),
            _ => None,
        };
        let top_of = |i: usize| {
            spans
                .iter()
                .find(|s| s.0 == i)
                .map(|s| s.1)
                .or_else(|| spans.iter().find(|s| s.0 + 1 == i).map(|s| s.1 + s.2))
        };
        for &(i, y, _) in &spans {
            let Some(f) = self.fold(i) else { continue };
            if let Some(row) = f.child().as_ref().and_then(|_| {
                self.bound
                    .borrow()
                    .get(&i)?
                    .child()
                    .as_ref()
                    .and_then(row_box)
            }) {
                if target == Some(Target::Swap(i)) {
                    row.add_css_class("swap-target");
                } else {
                    row.remove_css_class("swap-target");
                }
            }
            if i == from {
                // The picked-up row shows as the line in the gap, or a
                // ghost in its place.
                match gap.and_then(|g| {
                    let top = top_of(g)?;
                    Some(if g > from { top - h } else { top })
                }) {
                    Some(gap_top) => {
                        f.fade_to(0.0, animate);
                        f.shift_to(gap_top - y, animate);
                        f.set_line(Some(Line { x: 0.0, at: 0.5 }), animate);
                    }
                    None => {
                        f.fade_to(0.35, animate);
                        f.shift_to(0.0, animate);
                        f.set_line(None, animate);
                    }
                }
                continue;
            }
            let dy = match gap {
                Some(g) if from < i && i < g => -h,
                Some(g) if g <= i && i < from => h,
                _ => 0.0,
            };
            f.shift_to(dy, animate);
        }
    }

    /// Screenshots: program `from` held over row `over`, between rows
    /// (just above it) or onto it, once the rows are laid out.
    pub fn demo_drag(self: &Rc<Self>, from: usize, over: usize, swap: bool) {
        let me = Rc::downgrade(self);
        self.list.add_tick_callback(move |_, _| {
            let Some(me) = me.upgrade() else {
                return glib::ControlFlow::Break;
            };
            let (Some((_, h)), Some((top, oh))) = (me.span(from), me.span(over)) else {
                return glib::ControlFlow::Continue;
            };
            if h <= 0.0 {
                return glib::ControlFlow::Continue;
            }
            me.plated.set_hidden(true);
            *me.drag.borrow_mut() = Drag {
                from: Some(from),
                height: h,
                pointer: Some(top + if swap { oh / 2.0 } else { oh * 0.1 }),
                ..Drag::default()
            };
            me.layout_drag(false);
            glib::ControlFlow::Break
        });
    }

    /// The legend under the dragged row: "Move to 003 · 38 slots change".
    fn say_drop(&self, from: usize, target: Option<Target>) {
        let Some(legend) = self.drag.borrow().legend.clone() else {
            return;
        };
        let text = target.and_then(|t| t.reorder(from)).and_then(|(to, how)| {
            let n = self.source.upgrade()?.reorder_changes(from, to, how) as u32;
            let what = match how {
                Reorder::Move => trn(
                    "Move to {slot} · %d slot changes",
                    "Move to {slot} · %d slots change",
                    n,
                ),
                Reorder::Swap => trn(
                    "Swap with {slot} · %d slot changes",
                    "Swap with {slot} · %d slots change",
                    n,
                ),
            };
            Some(what.replace("{slot}", &self.source.upgrade()?.slot(to)))
        });
        legend.set_visible(text.is_some());
        legend.set_label(text.as_deref().unwrap_or_default());
    }

    /// Near the top or bottom edge, scroll on the frame clock, faster the
    /// closer the pointer is to the edge.
    fn autoscroll(self: &Rc<Self>, y: f64) {
        let height = f64::from(self.list.height());
        let speed = if y < EDGE {
            -EDGE_SPEED * (EDGE - y) / EDGE
        } else if y > height - EDGE {
            EDGE_SPEED * (y - (height - EDGE)) / EDGE
        } else {
            0.0
        };
        self.drag.borrow_mut().speed = speed;
        if speed == 0.0 {
            self.stop_scroll();
            return;
        }
        if self.drag.borrow().scroll.is_some() {
            return;
        }
        let me = Rc::downgrade(self);
        let id = self.list.add_tick_callback(move |list, clock| {
            let Some(me) = me.upgrade() else {
                return glib::ControlFlow::Break;
            };
            let now = clock.frame_time();
            let (speed, last) = {
                let mut d = me.drag.borrow_mut();
                (d.speed, std::mem::replace(&mut d.last_frame, now))
            };
            if let Some(adj) = list.vadjustment()
                && last > 0
            {
                let dt = (now - last) as f64 / 1e6;
                adj.set_value(adj.value() + speed * dt);
                me.layout_drag(false);
            }
            glib::ControlFlow::Continue
        });
        self.drag.borrow_mut().scroll = Some(id);
    }

    fn stop_scroll(&self) {
        let mut d = self.drag.borrow_mut();
        d.last_frame = 0;
        if let Some(id) = d.scroll.take() {
            id.remove();
        }
    }

    /// A program was opened by click or Enter.
    pub fn connect_open(&self, f: impl Fn(usize) + 'static) {
        self.on_open.connect(f);
    }

    /// Whether the last selection change came from moving by keys (and
    /// forget it).
    pub fn take_key_nav(&self) -> bool {
        self.key_nav.replace(false)
    }

    /// The program `delta` rows from `i` in the list as filtered now, or
    /// `None` past either end.
    pub fn neighbour(&self, i: usize, delta: isize) -> Option<usize> {
        let model = self.selection.model()?;
        let n = model.n_items();
        let pos = (0..n).find(|&p| model.item(p).as_ref().and_then(index_of) == Some(i))?;
        let to = u32::try_from(pos as isize + delta)
            .ok()
            .filter(|&p| p < n)?;
        model.item(to).as_ref().and_then(index_of)
    }

    /// Select the first program shown and move focus to the list.
    fn focus_first(&self) {
        if self.selection.n_items() == 0 {
            return;
        }
        self.list.scroll_to(
            0,
            gtk::ListScrollFlags::FOCUS | gtk::ListScrollFlags::SELECT,
            None,
        );
    }

    /// Bring the selected program into view once the list has laid out
    /// again (after a change of sizes), on the next frame.
    pub fn reveal_selected(&self) {
        let pos = self.selection.selected();
        if pos == gtk::INVALID_LIST_POSITION {
            return;
        }
        self.list.add_tick_callback(move |list, _| {
            list.scroll_to(pos, gtk::ListScrollFlags::NONE, None);
            glib::ControlFlow::Break
        });
    }

    /// What the facet filter offers: `title` (the field's name) and its
    /// named values, or nothing (the filter hides). Resets the filter.
    pub fn set_facet(&self, title: &str, choices: Vec<(String, u16)>) {
        let mut names = vec![tr("All")];
        names.extend(choices.iter().map(|(n, _)| n.clone()));
        let refs: Vec<&str> = names.iter().map(String::as_str).collect();
        self.voice.set_model(Some(&gtk::StringList::new(&refs)));
        self.voice.set_selected(0);
        self.voice.set_visible(!choices.is_empty());
        self.voice.set_tooltip_text(Some(title));
        self.voice
            .update_property(&[gtk::accessible::Property::Label(title)]);
        *self.facet.borrow_mut() = choices.into_iter().map(|(_, v)| v).collect();
        self.shown.borrow_mut().voice = None;
    }

    /// Empty the search and reset both filters.
    fn clear_filters(&self) {
        self.search.set_text("");
        self.voice.set_selected(0);
        self.edited.set_active(false);
    }

    /// Show `n` programs (indices 0..n).
    pub fn set_len(&self, n: usize) {
        let items: Vec<glib::BoxedAnyObject> = (0..n).map(glib::BoxedAnyObject::new).collect();
        self.store.splice(0, self.store.n_items(), &items);
        self.refresh();
        // Armed until the list lays out the new rows (consumed by the
        // adjustment handler); if no relayout happens, it is disarmed after
        // the next painted frame so a later user scroll is never hijacked.
        self.to_top.set(true);
        let to_top = self.to_top.clone();
        let frames = Cell::new(0u8);
        self.root.add_tick_callback(move |_, _| {
            frames.set(frames.get() + 1);
            if frames.get() < 2 {
                return glib::ControlFlow::Continue;
            }
            to_top.set(false);
            glib::ControlFlow::Break
        });
    }

    /// Show `n` programs without moving the list (programs were added to
    /// or removed from a library).
    pub fn resize(&self, n: usize) {
        let have = self.store.n_items() as usize;
        if n < have {
            self.store
                .splice(n as u32, (have - n) as u32, &[] as &[glib::Object]);
        } else if n > have {
            let items: Vec<glib::BoxedAnyObject> =
                (have..n).map(glib::BoxedAnyObject::new).collect();
            self.store.extend_from_slice(&items);
        }
    }

    /// How many programs the list holds (shown or not).
    pub fn len(&self) -> usize {
        self.store.n_items() as usize
    }

    /// Program `i`'s row on screen, if it is.
    pub fn row_of(&self, i: usize) -> Option<gtk::Widget> {
        self.bound
            .borrow()
            .get(&i)?
            .child()
            .filter(|w| w.is_mapped())
    }

    /// Re-read every visible row's data (after syncs, undo, saving).
    pub fn refresh(&self) {
        if let Some(src) = self.source.upgrade() {
            let n = self.store.n_items() as usize;
            self.groups.borrow_mut().cats = (0..n).map(|i| src.category(i)).collect();
        }
        self.refilter();
    }

    /// Re-read one program's row; cheap enough to call per received program.
    pub fn refresh_one(&self, i: usize) {
        let Some(src) = self.source.upgrade() else {
            return;
        };
        let c = src.category(i);
        let moved = self
            .groups
            .borrow_mut()
            .cats
            .get_mut(i)
            .is_some_and(|old| std::mem::replace(old, c) != c);
        if moved {
            // Runs and the rows around this one change: regroup (O(shown)).
            self.regroup();
        } else if let Some(item) = self.bound.borrow().get(&i) {
            fill(item, &src.row(i), self.groups.borrow().of(i));
        }
    }

    /// Recompute the category runs of the visible rows, then refill the
    /// rows on screen.
    fn regroup(&self) {
        let n = self.runs.n_items();
        let mut starts = Vec::new();
        let grouped = {
            let mut g = self.groups.borrow_mut();
            let runs = 1 + g.cats.windows(2).filter(|w| w[0] != w[1]).count();
            // ponytail: "ordered by category" = at most one run per 4
            // programs; tune if real libraries land on the wrong side.
            // Unread programs have no category yet: group only once all
            // are known, or their empty run gets a blank header.
            // A category may head only one run: a library whose categories
            // are split (Pad, Arp, Pad) shows per-row legends instead.
            let distinct = g
                .cats
                .iter()
                .collect::<std::collections::HashSet<_>>()
                .len();
            g.grouped =
                runs * 4 <= g.cats.len() && runs == distinct && g.cats.iter().all(Option::is_some);
            let mut first = vec![false; g.cats.len()];
            let mut prev = None;
            for pos in 0..n {
                let Some(i) = self.runs.item(pos).as_ref().and_then(index_of) else {
                    continue;
                };
                let c = g.cats.get(i).copied().flatten();
                if prev != Some(c) {
                    starts.push(pos);
                    first[i] = true;
                }
                prev = Some(c);
            }
            g.first = first;
            g.grouped
        };
        // Rebinds the headers, which read `groups`. Ungrouped: one section
        // whose header stays empty.
        self.runs
            .set_starts(if grouped { starts } else { Vec::new() });
        if let Some(src) = self.source.upgrade() {
            let g = self.groups.borrow();
            for (i, item) in self.bound.borrow().iter() {
                fill(item, &src.row(*i), g.of(*i));
            }
        }
    }

    /// Program `i` was edited: update its row, and refilter only when the
    /// active filter depends on what changed.
    pub fn changed(&self, i: usize, edited_flipped: bool, voice_changed: bool) {
        self.refresh_one(i);
        let f = *self.shown.borrow();
        if (f.voice.is_some() && voice_changed) || (f.edited_only && edited_flipped) {
            self.refilter();
        }
    }

    /// Select program `i` if it is visible under the current filter.
    pub fn select(&self, i: usize) {
        let model = self.selection.model().expect("model");
        for pos in 0..model.n_items() {
            if model.item(pos).as_ref().and_then(index_of) == Some(i) {
                self.selection.set_selected(pos);
                return;
            }
        }
    }

    fn refilter(&self) {
        self.filter.changed(gtk::FilterChange::Different);
        self.regroup();
        // The drag handles show where programs can be reordered.
        if self.can_reorder().is_ok() {
            self.root.add_css_class("reorderable");
        } else {
            self.root.remove_css_class("reorderable");
        }
        let shown = self.selection.n_items();
        let total = self.store.n_items();
        self.stack
            .set_visible_child_name(if shown == 0 && total > 0 {
                "empty"
            } else {
                "list"
            });
        self.title.set_subtitle(&if shown == total {
            trn("%d program", "%d programs", total)
        } else {
            trn("{shown} of %d program", "{shown} of %d programs", total)
                .replace("{shown}", &shown.to_string())
        });
    }
}

fn index_of(obj: &glib::Object) -> Option<usize> {
    obj.downcast_ref::<glib::BoxedAnyObject>()
        .and_then(|b| b.try_borrow::<usize>().ok().map(|i| *i))
}

/// Open `model` as a menu on `row`, at the pointer when there is one.
pub fn popup(row: &gtk::Widget, model: &gio::Menu, at: Option<(f64, f64)>) {
    let menu = gtk::PopoverMenu::from_model(Some(model));
    menu.set_has_arrow(false);
    menu.set_halign(gtk::Align::Start);
    menu.set_parent(row);
    if let Some((x, y)) = at {
        menu.set_pointing_to(Some(&gdk::Rectangle::new(x as i32, y as i32, 1, 1)));
    }
    menu.connect_closed(|m| {
        // Unparent after the close has finished, on the main loop's next
        // turn (an idle, not a timer).
        let m = m.clone();
        glib::idle_add_local_once(move || m.unparent());
    });
    menu.popup();
}

/// Wrap a row's content so it can move ([`Fold`]), with a drag handle
/// at its start, and make the whole row draggable: to copy the program to
/// a library in the sidebar, or (in one of Oscyl's libraries) to move it.
fn drag_row(
    row: &gtk::Box,
    item: &gtk::ListItem,
    source: &Source,
    me: &Rc<std::cell::OnceCell<Weak<ProgramList>>>,
) -> Fold {
    let handle = gtk::Image::builder()
        .icon_name("list-drag-handle-symbolic")
        .pixel_size(14)
        .halign(gtk::Align::Start)
        .valign(gtk::Align::Center)
        .margin_start(3)
        .can_target(false)
        .build();
    handle.add_css_class("drag-handle");
    let overlay = gtk::Overlay::builder().child(row).build();
    overlay.add_overlay(&handle);
    let fold = Fold::new(&overlay);
    let drag = gtk::DragSource::new();
    drag.set_actions(gdk::DragAction::COPY | gdk::DragAction::MOVE);
    // Where the row was taken, so it stays under the pointer.
    let grip = Rc::new(Cell::new((0.0, 0.0)));
    {
        let (source, item, grip) = (source.clone(), item.downgrade(), grip.clone());
        drag.connect_prepare(move |d, x, y| {
            let i = item.upgrade()?.item().as_ref().and_then(index_of)?;
            if !source.upgrade()?.can_drag(i) {
                if let Some(w) = d.widget() {
                    w.error_bell();
                }
                return None;
            }
            grip.set((x, y));
            Some(gdk::ContentProvider::for_value(
                &format!("program:{i}").to_value(),
            ))
        });
    }
    {
        let (item, me, row) = (item.downgrade(), me.clone(), row.downgrade());
        drag.connect_drag_begin(move |_, d| {
            let (Some(i), Some(me), Some(row)) = (
                item.upgrade()
                    .and_then(|it| it.item())
                    .as_ref()
                    .and_then(index_of),
                me.get().and_then(Weak::upgrade),
                row.upgrade(),
            ) else {
                return;
            };
            // The row under the pointer, and under it what a drop would do.
            let icon = gtk::Box::new(gtk::Orientation::Vertical, 4);
            icon.add_css_class("drag-icon");
            let picture = gtk::Picture::for_paintable(&gtk::WidgetPaintable::new(Some(&row)));
            picture.set_can_shrink(false);
            picture.set_size_request(row.width(), row.height());
            let legend = gtk::Label::builder().xalign(0.0).visible(false).build();
            legend.add_css_class("legend");
            legend.add_css_class("drag-legend");
            icon.append(&picture);
            icon.append(&legend);
            gtk::DragIcon::for_drag(d).set_child(Some(&icon));
            let (x, y) = grip.get();
            d.set_hotspot(x as i32, y as i32);
            me.drag_begin(i, legend);
        });
    }
    {
        let me = me.clone();
        drag.connect_drag_end(move |_, _, _| {
            if let Some(me) = me.get().and_then(Weak::upgrade) {
                me.drag_end();
            }
        });
    }
    fold.add_controller(drag);
    fold
}

/// The row's own content inside its [`Fold`] and handle overlay.
fn row_box(child: &gtk::Widget) -> Option<gtk::Widget> {
    child
        .downcast_ref::<Fold>()?
        .child()?
        .downcast_ref::<gtk::Overlay>()?
        .child()
}

fn row_widget() -> gtk::Box {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    row.add_css_class("program-row");
    // The edit mark: an action-colour dot after the name, before the
    // picture, as on the phone (the accessible label says "edited", so
    // colour is never the only cue).
    let tick = gtk::Box::new(gtk::Orientation::Vertical, 0);
    tick.add_css_class("edit-tick");
    tick.set_valign(gtk::Align::Center);
    let slot = gtk::Label::builder().xalign(1.0).width_chars(3).build();
    slot.add_css_class("slot-number");
    let text = gtk::Box::new(gtk::Orientation::Vertical, 2);
    text.set_hexpand(true);
    text.set_valign(gtk::Align::Center);
    let name = gtk::Label::builder()
        .xalign(0.0)
        .ellipsize(pango::EllipsizeMode::End)
        .build();
    name.add_css_class("program-name");
    let sub = gtk::Label::builder()
        .xalign(0.0)
        .ellipsize(pango::EllipsizeMode::End)
        .build();
    sub.add_css_class("caption");
    sub.add_css_class("dim-label");
    text.append(&name);
    text.append(&sub);
    // The program's first picture, small, last in the row, so pictures
    // stand in one column whatever comes before.
    let picture = Graph::new();
    picture.set_compact();
    picture.set_min_width_em(3.6);
    picture.add_css_class("mini");
    picture.add_css_class("row-picture");
    picture.set_can_target(false);
    picture.set_valign(gtk::Align::Center);
    picture.set_hexpand(false);
    picture.set_accessible_role(gtk::AccessibleRole::Presentation);
    row.append(&slot);
    row.append(&text);
    row.append(&tick);
    row.append(&picture);
    row
}

fn fill(item: &gtk::ListItem, d: &RowData, (cat, sub): (Option<Category>, String)) {
    if let Some(row) = item.child().as_ref().and_then(row_box) {
        bind_row(&row, d, &sub);
    }
    let mut label = format!("{} {}", d.slot, d.name);
    if let Some(c) = cat {
        label = tr("{program}, {category}")
            .replace("{program}", &label)
            .replace("{category}", &category_name(c));
    }
    if d.edited {
        label = tr("{program}, edited").replace("{program}", &label);
    }
    item.set_accessible_label(&label);
}

fn bind_row(child: &gtk::Widget, d: &RowData, subtitle: &str) {
    let slot = child
        .first_child()
        .and_downcast::<gtk::Label>()
        .expect("slot");
    let text = slot.next_sibling().expect("text");
    let tick = text.next_sibling().expect("tick");
    let picture = tick
        .next_sibling()
        .and_downcast::<Graph>()
        .expect("picture");
    let name = text
        .first_child()
        .and_downcast::<gtk::Label>()
        .expect("name");
    let sub = name
        .next_sibling()
        .and_downcast::<gtk::Label>()
        .expect("subtitle");
    // Hidden by opacity, not visibility, so the name column stays put.
    tick.set_opacity(if d.edited { 1.0 } else { 0.0 });
    // Kept in place when there is none, so pictures stand in one column.
    picture.set_opacity(if d.picture.is_some() { 1.0 } else { 0.0 });
    picture.set_data(d.picture.clone().unwrap_or_default(), None);
    slot.set_label(&d.slot);
    name.set_label(&d.name);
    sub.set_label(subtitle);
    sub.set_visible(!subtitle.is_empty());
}

/// The filtered programs, split into sections at category runs (see
/// [`ProgramList::regroup`]) so the list can give each run a header.
mod runs {
    use std::cell::{OnceCell, RefCell};

    use gtk::subclass::prelude::*;
    use gtk::{gio, glib, prelude::*};

    #[derive(Default)]
    pub struct Runs {
        pub inner: OnceCell<gio::ListModel>,
        /// Position where each section starts, ascending.
        pub starts: RefCell<Vec<u32>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Runs {
        const NAME: &'static str = "OscylProgramRuns";
        type Type = super::Runs;
        type Interfaces = (gio::ListModel, gtk::SectionModel);
    }

    impl ObjectImpl for Runs {}

    impl ListModelImpl for Runs {
        fn item_type(&self) -> glib::Type {
            glib::BoxedAnyObject::static_type()
        }
        fn n_items(&self) -> u32 {
            self.inner.get().map_or(0, |m| m.n_items())
        }
        fn item(&self, position: u32) -> Option<glib::Object> {
            self.inner.get()?.item(position)
        }
    }

    impl SectionModelImpl for Runs {
        fn section(&self, position: u32) -> (u32, u32) {
            let n = self.n_items();
            if position >= n {
                return (n, u32::MAX);
            }
            let s = self.starts.borrow();
            let k = s.partition_point(|&x| x <= position);
            let start = k.checked_sub(1).map_or(0, |k| s[k]);
            (start, s.get(k).copied().unwrap_or(n).min(n))
        }
    }
}

glib::wrapper! {
    pub struct Runs(ObjectSubclass<runs::Runs>)
        @implements gio::ListModel, gtk::SectionModel;
}

impl Runs {
    fn new(inner: &impl IsA<gio::ListModel>) -> Runs {
        let me: Runs = glib::Object::new();
        let _ = me.imp().inner.set(inner.clone().upcast());
        let weak = me.downgrade();
        inner.connect_items_changed(move |_, pos, removed, added| {
            if let Some(me) = weak.upgrade() {
                me.items_changed(pos, removed, added);
            }
        });
        me
    }

    fn set_starts(&self, starts: Vec<u32>) {
        *self.imp().starts.borrow_mut() = starts;
        if self.n_items() > 0 {
            self.sections_changed(0, self.n_items());
        }
    }
}
