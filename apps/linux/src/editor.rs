//! The editor, a compare bench: the program title, the stage tabs, the
//! selected stage's large graph with draggable handles and its controls,
//! each showing where the saved sound was; the review bar at its foot
//! lists the changes. Pages, sections and controls come from the driver's
//! curation (`oscyl_driver::Driver`); values flow out through `on_change`,
//! from dials, pickers and graph handles alike, as (field index, raw value).

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::{Rc, Weak};

use adw::prelude::*;
use oscyl_app::editor::{Widget, option_of, raw_of, widget};
use oscyl_driver::{Control, Curation, Driver};

use crate::callback::Slot;
use crate::i18n::tr;
use crate::widgets::{Dial, Dock, Graph, GraphData, HandlePoint, SegmentTrack, StageStrip};

/// One generated control, kept so programs can be shown without rebuilding.
enum Ctl {
    Dial(Dial),
    /// Named values, as segments.
    Choice(adw::ToggleGroup),
    /// Named values, as a menu.
    Menu(gtk::DropDown),
    /// Off / On as two segments, like the synth's two-way selectors.
    Toggle(adw::ToggleGroup),
}

/// What the editor is showing.
pub struct Shown<'a> {
    pub slot: String,
    pub program: &'a [u8],
    pub base: &'a [u8],
    pub note: String,
    /// A factory slot: shown, never edited.
    pub read_only: bool,
}

/// Each shown field's control, its cell, and the [`Control`] it was built
/// for (a program change may ask for another).
type Controls = HashMap<usize, (Ctl, gtk::Box, Control)>;
/// The shown program and its saved version. Dial formatters read it, so
/// values that depend on other fields format in context.
type Pair = Rc<RefCell<Option<(Vec<u8>, Vec<u8>)>>>;

pub struct Editor {
    pub root: gtk::Stack,
    /// The header bar's title: the slot, joined by the program name once
    /// the big title scrolls away; or the page shown instead (guide, units).
    pub header_title: adw::WindowTitle,
    slot: RefCell<String>,
    title_hidden: Cell<bool>,
    context: RefCell<Option<String>>,
    title: gtk::EditableLabel,
    note: gtk::Label,
    strip: StageStrip,
    chart: gtk::Box,
    chart_title: gtk::Label,
    enlarge: gtk::Button,
    /// The dice: roll this stage's values (one undo step).
    roll: gtk::Button,
    enlarged: Cell<bool>,
    stage_graph: Graph,
    /// The scope docked at the top while the stage graph is scrolled
    /// away: the same live shape, small, with the stage and its summary.
    dock: Dock,
    page_scroll: gtk::ScrolledWindow,
    dock_graph: Graph,
    dock_title: gtk::Label,
    dock_summary: gtk::Label,
    /// The docked scope's stage switcher: each stage's live drawing and
    /// its edited dot, in a selector whose plate slides.
    dock_stages: adw::ToggleGroup,
    dock_tiles: RefCell<Vec<(Graph, gtk::Box)>>,
    /// Set while the switcher follows a stage change made elsewhere.
    dock_syncing: Cell<bool>,
    /// Gliding the page back to the stage graph.
    back_glide: std::cell::OnceCell<adw::TimedAnimation>,
    review: Rc<crate::review::Review>,
    /// The synth whose programs are shown, and its curation.
    driver: Cell<&'static dyn Driver>,
    curation: RefCell<Curation>,
    /// The page shown.
    stage: Cell<usize>,
    controls: RefCell<Controls>,
    /// Each shown control's revert button, visible while it is edited.
    reverts: RefCell<HashMap<usize, Revert>>,
    /// Each shown section and its fields: a section with nothing that
    /// applies (layer B of a one-layer program) hides.
    sections: RefCell<Vec<(gtk::Box, Vec<usize>)>>,
    shown: Pair,
    updating: Cell<bool>,
    /// While Compare is held: the edited pair, set aside while the saved
    /// program is shown.
    compare_stash: RefCell<Option<(Vec<u8>, Vec<u8>)>>,
    /// Where the program shown stands between saved (0) and the edit (1).
    blend: Cell<f32>,
    /// Set while a compare redraws: graphs glide rather than jump.
    gliding: Cell<bool>,
    /// Whole-editor refreshes so far (tests check drags stay incremental).
    #[cfg(test)]
    full_refreshes: Cell<u32>,
    on_change: Slot<(usize, u16)>,
    on_seal: Slot<()>,
    on_gesture: Slot<()>,
    on_rename: Slot<String>,
    on_revert: Slot<Option<usize>>,
    on_roll: Slot<usize>,
    welcome: adw::StatusPage,
    /// Weak self, for widget callbacks.
    me: Weak<Editor>,
}

impl Editor {
    pub fn new(driver: &'static dyn Driver) -> Rc<Editor> {
        let root = gtk::Stack::new();
        root.set_transition_type(gtk::StackTransitionType::Crossfade);
        root.set_transition_duration(220);
        // A page that comes into view settles up into place while the old
        // one fades out; the class is set on the next frame so the
        // animation restarts.
        root.connect_visible_child_notify(|stack| {
            let Some(page) = stack.visible_child() else {
                return;
            };
            page.remove_css_class("page-enter");
            page.add_tick_callback(|w, _| {
                w.add_css_class("page-enter");
                gtk::glib::ControlFlow::Break
            });
        });

        // Welcome / empty state; the app sets its text.
        let welcome = adw::StatusPage::builder()
            .icon_name("oscyl-oscillator-symbolic")
            .build();
        let actions = gtk::Box::new(gtk::Orientation::Vertical, 12);
        actions.set_halign(gtk::Align::Center);
        for (label, action, suggested) in [
            (tr("Explore Demo Library"), "win.open-demo", true),
            (tr("Open Library…"), "win.open", false),
            (tr("Connect Synth"), "win.connect", false),
        ] {
            let b = gtk::Button::builder()
                .label(label)
                .action_name(action)
                .build();
            b.add_css_class("pill");
            if suggested {
                b.add_css_class("suggested-action");
            }
            actions.append(&b);
        }
        welcome.set_child(Some(&actions));
        root.add_named(&welcome, Some("welcome"));

        // Editor page.
        let page = gtk::Box::new(gtk::Orientation::Vertical, 0);
        page.add_css_class("editor");

        let head = gtk::Box::new(gtk::Orientation::Vertical, 4);
        head.add_css_class("editor-head");
        let title = gtk::EditableLabel::new("");
        title.add_css_class("program-title");
        title.add_css_class("display");
        title.set_tooltip_text(Some(&tr("Rename (up to 12 characters)")));
        title.update_property(&[gtk::accessible::Property::Label(&tr("Program name"))]);
        let note = gtk::Label::builder().xalign(0.0).wrap(true).build();
        note.add_css_class("dim-label");
        // The title is editable: a pencil beside it says so and starts
        // editing (as do a click on the title and F2); hovering the title
        // underlines it like a field.
        let rename = gtk::Button::builder()
            .icon_name("document-edit-symbolic")
            .valign(gtk::Align::Center)
            .tooltip_text(tr("Rename (F2)"))
            .css_classes(["flat", "rename"])
            .build();
        rename.update_property(&[gtk::accessible::Property::Label(&tr("Rename program"))]);
        {
            let title = title.clone();
            rename.connect_clicked(move |_| title.start_editing());
        }
        {
            let keys = gtk::ShortcutController::new();
            keys.set_scope(gtk::ShortcutScope::Managed);
            let title = title.clone();
            keys.add_shortcut(gtk::Shortcut::new(
                gtk::ShortcutTrigger::parse_string("F2"),
                Some(gtk::CallbackAction::new(move |_, _| {
                    title.start_editing();
                    gtk::glib::Propagation::Stop
                })),
            ));
            page.add_controller(keys);
        }
        let title_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        title_row.append(&title);
        title_row.append(&rename);
        // While Compare is held, a plate beside the title says the page
        // shows the saved program. It keeps its space, so nothing moves.
        let tag = gtk::Label::builder()
            .label(tr("Saved version"))
            .valign(gtk::Align::Center)
            .css_classes(["compare-tag"])
            .accessible_role(gtk::AccessibleRole::Presentation)
            .build();
        title_row.append(&tag);
        // The Perform page: the program played on a pad and four knobs.
        let perform_content = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        perform_content.append(&gtk::Image::from_icon_name("oscyl-pad-symbolic"));
        perform_content.append(&gtk::Label::new(Some(&tr("Perform"))));
        let perform = gtk::Button::builder()
            .child(&perform_content)
            .action_name("win.perform")
            .halign(gtk::Align::End)
            .valign(gtk::Align::Center)
            .hexpand(true)
            .css_classes(["outlined", "perform-open"])
            .tooltip_text(tr(
                "Perform this program on an XY pad and four knobs (Ctrl+P)",
            ))
            .build();
        perform.update_property(&[gtk::accessible::Property::KeyShortcuts("Control+P")]);
        title_row.append(&perform);
        head.append(&title_row);
        head.append(&note);
        page.append(&head);

        let strip = StageStrip::new();
        page.append(&strip);

        // Chart + changes, side by side when wide, stacked when narrow.
        let body = gtk::Box::new(gtk::Orientation::Horizontal, 32);
        body.add_css_class("editor-body");
        let chart_col = gtk::Box::new(gtk::Orientation::Vertical, 8);
        chart_col.set_hexpand(true);
        let chart_head = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        let chart_title = gtk::Label::builder().xalign(0.0).hexpand(true).build();
        chart_title.add_css_class("stage-heading");
        chart_title.set_accessible_role(gtk::AccessibleRole::Heading);
        chart_head.append(&chart_title);
        // The dice roll the stage shown: its fields only, one undo step.
        let roll = gtk::Button::builder()
            .icon_name("oscyl-dice-symbolic")
            .valign(gtk::Align::Center)
            .css_classes(["flat", "roll-stage"])
            .build();
        chart_head.append(&roll);
        let enlarge = gtk::Button::builder()
            .icon_name("view-fullscreen-symbolic")
            .action_name("win.inspect")
            .valign(gtk::Align::Center)
            .build();
        enlarge.add_css_class("flat");
        enlarge.add_css_class("enlarge-graph");
        chart_head.append(&enlarge);
        chart_col.append(&chart_head);
        // The stage's live picture: redrawn on every change, with the saved
        // version faint behind it while edited, and handles to drag.
        let stage_graph = Graph::with_handles();
        stage_graph.add_css_class("stage-graph");
        chart_col.append(&stage_graph);
        let chart = gtk::Box::new(gtk::Orientation::Vertical, 0);
        chart.add_css_class("controls");
        chart_col.append(&chart);
        // Lets an enlarged graph take the editor's free height.
        chart_col.set_vexpand(true);
        body.append(&chart_col);

        body.set_vexpand(true);
        page.append(&body);

        let page_scroll = gtk::ScrolledWindow::builder()
            .child(&page)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .build();
        crate::widgets::smooth_wheel(&page_scroll);
        // The review bar rides over the page's foot; the page keeps room
        // for it below its last control, so nothing hides behind it.
        let review = crate::review::Review::new();
        let over = gtk::Overlay::builder().child(&page_scroll).build();
        over.add_overlay(&review.root);
        // The docked scope: while the stage graph is out of view, its
        // shape stays in sight at the top; clicking it goes back to it.
        let dock_title = gtk::Label::builder().xalign(0.0).build();
        dock_title.add_css_class("dock-title");
        let dock_summary = gtk::Label::builder()
            .xalign(0.0)
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .max_width_chars(18)
            .build();
        dock_summary.add_css_class("dim-label");
        dock_summary.add_css_class("numeric");
        let dock_text = gtk::Box::new(gtk::Orientation::Vertical, 2);
        dock_text.set_valign(gtk::Align::Center);
        dock_text.append(&dock_title);
        dock_text.append(&dock_summary);
        let dock_graph = Graph::new();
        dock_graph.add_css_class("dock-graph");
        dock_graph.set_compact();
        let dock_row = gtk::Box::new(gtk::Orientation::Horizontal, 16);
        dock_row.append(&dock_text);
        dock_row.append(&dock_graph);
        let dock_back = gtk::Button::builder()
            .child(&dock_row)
            .hexpand(true)
            .css_classes(["flat", "dock-back"])
            .tooltip_text(tr("Back to the graph"))
            .build();
        dock_back.update_property(&[gtk::accessible::Property::Label(&tr("Back to the graph"))]);
        // Every stage at a glance, its drawing live; picking one shows it
        // and glides back up to its graph.
        let dock_stages = adw::ToggleGroup::new();
        dock_stages.add_css_class("dock-stages");
        dock_stages
            .upcast_ref::<gtk::Widget>()
            .update_property(&[gtk::accessible::Property::Label(&tr("Stages"))]);
        let dock_switch = SegmentTrack::new(&dock_stages);
        dock_switch.set_valign(gtk::Align::Center);
        let dock_card = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        dock_card.add_css_class("dock-card");
        dock_card.append(&dock_back);
        dock_card.append(&dock_switch);
        let dock = Dock::new(&dock_card);
        over.add_overlay(&dock);
        // Escape folds an open sheet from anywhere in the editor.
        {
            let keys = gtk::EventControllerKey::new();
            keys.set_propagation_phase(gtk::PropagationPhase::Capture);
            let review = review.clone();
            keys.connect_key_pressed(move |_, key, _, _| {
                if key == gtk::gdk::Key::Escape && review.fold() {
                    gtk::glib::Propagation::Stop
                } else {
                    gtk::glib::Propagation::Proceed
                }
            });
            over.add_controller(keys);
        }
        // The editor adapts to its own width (it may sit beside the list, or
        // fill a phone screen): changes go under the chart when narrow.
        let bin = adw::BreakpointBin::builder()
            .child(&over)
            .width_request(320)
            .height_request(320)
            .build();
        // A BreakpointBin applies one breakpoint at a time (the last one that
        // matches), so each combination carries all its rules:
        // - stacked: tighter panels;
        // - narrow (phones): stacked, a smaller title, no slot in the
        //   header bar;
        // - short (small screens, big zoom): the stage graph shrinks before
        //   it pushes the controls away.
        let header_title = adw::WindowTitle::new("", "");
        let phone_rules = |bp: &adw::Breakpoint| {
            bp.add_setter(&dock_switch, "visible", Some(&false.to_value()));
            bp.add_setter(&header_title, "visible", Some(&false.to_value()));
            bp.add_setter(&review.chips, "visible", Some(&false.to_value()));
        };
        let classes = |bp: &adw::Breakpoint, names: &'static [&'static str]| {
            let p = page.clone();
            bp.connect_apply(move |_| names.iter().for_each(|n| p.add_css_class(n)));
            let p = page.clone();
            bp.connect_unapply(move |_| names.iter().for_each(|n| p.remove_css_class(n)));
        };
        let condition = |c: &str| adw::BreakpointCondition::parse(c).expect("valid breakpoint");
        let short = adw::Breakpoint::new(condition("max-height: 760sp"));
        classes(&short, &["short"]);
        bin.add_breakpoint(short);
        let stacked = adw::Breakpoint::new(condition("max-width: 660sp"));
        classes(&stacked, &["stacked"]);
        bin.add_breakpoint(stacked);
        let stacked_short =
            adw::Breakpoint::new(condition("max-width: 660sp and max-height: 760sp"));
        classes(&stacked_short, &["stacked", "short"]);
        bin.add_breakpoint(stacked_short);
        let narrow = adw::Breakpoint::new(condition("max-width: 500sp"));
        phone_rules(&narrow);
        classes(&narrow, &["stacked", "narrow"]);
        bin.add_breakpoint(narrow);
        let both = adw::Breakpoint::new(condition("max-width: 500sp and max-height: 760sp"));
        phone_rules(&both);
        classes(&both, &["stacked", "narrow", "short"]);
        bin.add_breakpoint(both);
        root.add_named(&bin, Some("editor"));

        let this = Rc::new_cyclic(|me| Editor {
            root,
            header_title,
            slot: RefCell::default(),
            title_hidden: Cell::new(false),
            context: RefCell::default(),
            title,
            note,
            strip,
            chart,
            chart_title,
            enlarge,
            roll: roll.clone(),
            enlarged: Cell::new(false),
            stage_graph,
            dock: dock.clone(),
            page_scroll: page_scroll.clone(),
            dock_graph,
            dock_title,
            dock_summary,
            dock_stages: dock_stages.clone(),
            dock_tiles: RefCell::default(),
            dock_syncing: Cell::new(false),
            back_glide: std::cell::OnceCell::new(),
            review: review.clone(),
            driver: Cell::new(driver),
            curation: RefCell::new(driver.curation()),
            stage: Cell::new(0),
            controls: RefCell::new(HashMap::new()),
            reverts: RefCell::new(HashMap::new()),
            sections: RefCell::default(),
            shown: Pair::default(),
            updating: Cell::new(false),
            compare_stash: RefCell::default(),
            blend: Cell::new(1.0),
            gliding: Cell::new(false),
            #[cfg(test)]
            full_refreshes: Cell::new(0),
            on_change: Slot::default(),
            on_seal: Slot::default(),
            on_gesture: Slot::default(),
            on_rename: Slot::default(),
            on_revert: Slot::default(),
            on_roll: Slot::default(),
            welcome,
            me: me.clone(),
        });

        // The scope docks once most of the stage graph has scrolled above
        // the top, and leaves as it comes back. A scroll or a new page
        // length is checked on the next frame, once the page is laid out
        // at its new place.
        {
            let adj = page_scroll.vadjustment();
            let pending = Rc::new(Cell::new(false));
            let me = Rc::downgrade(&this);
            let watch = move || {
                let Some(me) = me.upgrade() else {
                    return;
                };
                if pending.replace(true) {
                    return;
                }
                let pending = pending.clone();
                let weak = Rc::downgrade(&me);
                me.page_scroll.add_tick_callback(move |scroll, _| {
                    pending.set(false);
                    if let Some(me) = weak.upgrade() {
                        let away = me
                            .stage_graph
                            .compute_bounds(scroll)
                            .is_some_and(|b| b.y() + b.height() * 0.6 < 0.0);
                        me.dock.set_shown(away);
                    }
                    gtk::glib::ControlFlow::Break
                });
            };
            let w = watch.clone();
            adj.connect_value_changed(move |_| w());
            adj.connect_changed(move |_| watch());
        }
        {
            let me = Rc::downgrade(&this);
            roll.connect_clicked(move |_| {
                if let Some(me) = me.upgrade() {
                    me.on_roll.emit(me.stage.get());
                }
            });
        }
        {
            let me = Rc::downgrade(&this);
            dock_back.connect_clicked(move |_| {
                if let Some(me) = me.upgrade() {
                    me.back_to_graph();
                }
            });
            let me = Rc::downgrade(&this);
            dock_stages.connect_active_notify(move |g| {
                if let Some(me) = me.upgrade()
                    && !me.dock_syncing.get()
                    && (g.active() as usize) < me.pages()
                {
                    me.select_stage(g.active() as usize);
                    me.back_to_graph();
                }
            });
        }
        {
            let me = Rc::downgrade(&this);
            this.review.connect_stage(move |stage| {
                if let Some(me) = me.upgrade() {
                    me.select_stage(stage);
                }
            });
            let me = Rc::downgrade(&this);
            this.review.connect_revert(move |field| {
                if let Some(me) = me.upgrade() {
                    me.on_revert.emit(field);
                }
            });
        }
        {
            let me = Rc::downgrade(&this);
            this.strip.connect_stage_selected(move |stage| {
                if let Some(me) = me.upgrade() {
                    me.show_stage(stage);
                }
            });
        }
        {
            let me = Rc::downgrade(&this);
            this.stage_graph.connect_handle_moved(move |(i, to)| {
                if let Some(me) = me.upgrade() {
                    me.drag_handle(i, to);
                }
            });
            let me = Rc::downgrade(&this);
            this.stage_graph.connect_handle_released(move |()| {
                if let Some(me) = me.upgrade() {
                    me.seal();
                }
            });
        }
        {
            let me = Rc::downgrade(&this);
            this.title.connect_editing_notify(move |t| {
                let Some(me) = me.upgrade() else { return };
                if !t.is_editing() && !me.updating.get() {
                    me.on_rename.emit(t.text().to_string());
                }
            });
        }
        {
            // The header names the program once its big title is gone.
            let me = Rc::downgrade(&this);
            page_scroll.vadjustment().connect_value_changed(move |adj| {
                let Some(me) = me.upgrade() else { return };
                let Some(page) = me.title.parent().and_then(|h| h.parent()) else {
                    return;
                };
                let bottom = me
                    .title
                    .compute_bounds(&page)
                    .map_or(0.0, |b| f64::from(b.y() + b.height()));
                let hidden = adj.value() > bottom;
                if hidden != me.title_hidden.replace(hidden) {
                    me.update_header();
                }
            });
            let me = Rc::downgrade(&this);
            this.root.connect_visible_child_name_notify(move |s| {
                let Some(me) = me.upgrade() else { return };
                let context = match s.visible_child_name().as_deref() {
                    Some("editor") => None,
                    Some("guide") => Some(tr("Guide")),
                    Some("units") => Some(tr("Oscillators and effects")),
                    Some("generate") => Some(tr("Generate")),
                    Some("perform") => Some(tr("Perform")),
                    _ => Some(String::new()),
                };
                me.set_header_context(context);
            });
        }
        this.set_enlarged(false);
        this.build_pages();
        this
    }

    /// The synth whose programs are shown.
    pub fn driver(&self) -> &'static dyn Driver {
        self.driver.get()
    }

    /// Show `driver`'s programs from now on: its pages replace the old
    /// synth's. Nothing happens when it is the same synth.
    pub fn set_driver(self: &Rc<Self>, driver: &'static dyn Driver) {
        if self.driver.get().info().id == driver.info().id {
            return;
        }
        self.driver.set(driver);
        *self.curation.borrow_mut() = driver.curation();
        *self.shown.borrow_mut() = None;
        self.compare_stash.borrow_mut().take();
        self.blend.set(1.0);
        self.build_pages();
    }

    /// How many pages the driver's editor has.
    pub fn pages(&self) -> usize {
        self.curation.borrow().pages.len()
    }

    /// The page titled `name` (any case), for design captures.
    pub fn page_named(&self, name: &str) -> Option<usize> {
        self.curation
            .borrow()
            .pages
            .iter()
            .position(|p| p.title.eq_ignore_ascii_case(name))
    }

    /// Show the page with symbolic icon `icon`, when there is one.
    pub fn select_icon(self: &Rc<Self>, icon: &str) {
        if let Some(i) = self.page_with_icon(icon) {
            self.select_stage(i);
        }
    }

    /// The first page with symbolic icon `icon`, if any.
    pub fn page_with_icon(&self, icon: &str) -> Option<usize> {
        self.curation
            .borrow()
            .pages
            .iter()
            .position(|p| p.icon == Some(icon))
    }

    /// The strip's tabs and the dock's tiles, one per page; then the
    /// oscillators page (or the first).
    fn build_pages(self: &Rc<Self>) {
        let titles: Vec<String> = self
            .curation
            .borrow()
            .pages
            .iter()
            .map(|p| tr(p.title))
            .collect();
        self.strip.set_pages(&titles);
        self.dock_syncing.set(true);
        self.dock_stages.remove_all();
        let mut tiles = Vec::new();
        for (i, title) in titles.iter().enumerate() {
            let g = Graph::new();
            g.set_compact();
            g.set_min_width_em(2.8);
            g.add_css_class("dock-stage-graph");
            g.set_can_target(false);
            let dot = gtk::Box::builder()
                .css_classes(["edit-tick", "unlit"])
                .halign(gtk::Align::End)
                .valign(gtk::Align::Start)
                .can_target(false)
                .build();
            let tile = gtk::Overlay::builder().child(&g).build();
            tile.add_overlay(&dot);
            self.dock_stages.add(
                adw::Toggle::builder()
                    .child(&tile)
                    .name(i.to_string())
                    .label(title.as_str())
                    .tooltip(title.as_str())
                    .build(),
            );
            tiles.push((g, dot));
        }
        *self.dock_tiles.borrow_mut() = tiles;
        self.dock_syncing.set(false);
        let first = oscyl_app::editor::first_page(&self.curation.borrow());
        self.strip.select(first);
        self.show_stage(first);
    }

    pub fn connect_change(&self, f: impl Fn((usize, u16)) + 'static) {
        self.on_change.connect(f);
    }
    /// Called when a gesture ends (one undo step).
    pub fn connect_seal(&self, f: impl Fn(()) + 'static) {
        self.on_seal.connect(f);
    }
    /// Called before a graph handle moves several fields at once: until
    /// the next seal they are one undo step.
    pub fn connect_gesture(&self, f: impl Fn(()) + 'static) {
        self.on_gesture.connect(f);
    }

    /// Put the program's name up for editing.
    pub fn start_rename(&self) {
        self.title.grab_focus();
        self.title.start_editing();
    }

    pub fn connect_rename(&self, f: impl Fn(String) + 'static) {
        self.on_rename.connect(f);
    }
    /// The dice of stage `page` were pressed.
    pub fn connect_roll(&self, f: impl Fn(usize) + 'static) {
        self.on_roll.connect(f);
    }

    /// `None` reverts the whole program.
    pub fn connect_revert(&self, f: impl Fn(Option<usize>) + 'static) {
        self.on_revert.connect(f);
    }

    /// Name the page shown instead of a program in the header, or `None`
    /// for the program itself.
    pub fn set_header_context(&self, context: Option<String>) {
        *self.context.borrow_mut() = context;
        self.update_header();
    }

    fn update_header(&self) {
        let t = &self.header_title;
        if let Some(c) = &*self.context.borrow() {
            t.set_title(c);
            t.set_subtitle("");
        } else if self.title_hidden.get() {
            t.set_title(&self.title.text());
            t.set_subtitle(&self.slot.borrow());
        } else {
            // A lone slot reads as the title, not an orphaned subtitle.
            t.set_title(&self.slot.borrow());
            t.set_subtitle("");
        }
    }

    /// The page shown.
    pub fn stage(&self) -> usize {
        self.stage.get()
    }

    pub fn select_stage(self: &Rc<Self>, stage: usize) {
        if stage >= self.pages() {
            return;
        }
        self.strip.select(stage);
        if self.stage.get() != stage {
            self.show_stage(stage);
        }
    }

    /// Show another page (units, guide) in the editor area.
    pub fn show_page(&self, name: &str, page: &impl IsA<gtk::Widget>) {
        set_page(&self.root, name, page);
    }

    pub fn show_welcome(&self, title: &str, description: &str) {
        self.show_status(title, description, true);
    }

    /// The status page, with or without the open/connect buttons.
    pub fn show_status(&self, title: &str, description: &str, actions: bool) {
        self.welcome.set_title(title);
        self.welcome.set_description(Some(description));
        if let Some(c) = self.welcome.child() {
            c.set_visible(actions);
        }
        self.root.set_visible_child_name("welcome");
    }

    /// Hold to compare: show the saved program in every control, plate and
    /// graph (they glide there together), or come back to the edited one.
    /// Edits are ignored while comparing.
    #[cfg(test)]
    pub fn compare(&self, on: bool) {
        self.set_blend(if on { 0.0 } else { 1.0 }, true);
    }

    /// Show the program between saved (0) and the edit (1), the core's
    /// [`oscyl_app::editor::blend`]: below 1 every value and picture shows
    /// the blend and edits are ignored. `glide` moves there slowly (a
    /// press or a release); a drag's steps land in the frame they come.
    pub fn set_blend(&self, t: f32, glide: bool) {
        let t = t.clamp(0.0, 1.0);
        if (self.blend.get() - t).abs() < 1e-4 {
            return;
        }
        // The edited pair: set aside while a blend shows.
        let pair = if self.comparing() {
            self.compare_stash.borrow().clone()
        } else {
            self.shown.borrow().clone()
        };
        let Some(pair) = pair else {
            return;
        };
        let d = self.driver();
        let shown = if t >= 1.0 {
            self.compare_stash.borrow_mut().take();
            pair.clone()
        } else {
            *self.compare_stash.borrow_mut() = Some(pair.clone());
            (
                oscyl_app::editor::blend(d, &pair.1, &pair.0, t),
                pair.1.clone(),
            )
        };
        *self.shown.borrow_mut() = Some(shown);
        self.blend.set(t);
        if let Some(page) = self.root.child_by_name("editor") {
            if t < 1.0 {
                page.add_css_class("comparing");
            } else {
                page.remove_css_class("comparing");
            }
        }
        // Only dials that will move glide slowly; the others would keep
        // the slow glide into the next drag.
        let (p, base) = pair;
        for (field, (ctl, _, _)) in self.controls.borrow().iter() {
            if let Ctl::Dial(dial) = ctl {
                dial.set_slow_glide(glide && d.get(&p, *field) != d.get(&base, *field));
            }
        }
        self.updating.set(true);
        self.gliding.set(glide);
        self.refresh_values();
        self.refresh_stages(0..self.pages());
        self.gliding.set(false);
        self.updating.set(false);
    }

    /// Where the program shown stands between saved (0) and the edit (1).
    pub fn blend(&self) -> f32 {
        self.blend.get()
    }

    /// Keep the page's place across a change of sizes (interface size):
    /// the same share of the page stays at the top while the next few
    /// frames lay it out again.
    pub fn hold_place(&self) {
        let adj = self.page_scroll.vadjustment();
        let range = adj.upper() - adj.page_size() - adj.lower();
        if range <= 0.0 {
            return;
        }
        let share = (adj.value() - adj.lower()) / range;
        let frames = Cell::new(0u32);
        self.page_scroll.add_tick_callback(move |sw, _| {
            let a = sw.vadjustment();
            let range = a.upper() - a.page_size() - a.lower();
            if range > 0.0 {
                a.set_value(a.lower() + share * range);
            }
            frames.set(frames.get() + 1);
            if frames.get() < 3 {
                gtk::glib::ControlFlow::Continue
            } else {
                gtk::glib::ControlFlow::Break
            }
        });
    }

    /// Glide the page back so the stage heading sits just under the top.
    fn back_to_graph(&self) {
        let scroll = &self.page_scroll;
        let adj = scroll.vadjustment();
        let Some(b) = self.chart_title.compute_bounds(scroll) else {
            return;
        };
        let to = (adj.value() + f64::from(b.y()) - 16.0).max(adj.lower());
        let a = self.back_glide.get_or_init(|| {
            adw::TimedAnimation::builder()
                .widget(scroll)
                .target(&adw::PropertyAnimationTarget::new(&adj, "value"))
                .duration(320)
                .easing(adw::Easing::EaseOutCubic)
                .build()
        });
        a.pause();
        a.set_value_from(adj.value());
        a.set_value_to(to);
        a.play();
    }

    /// Design captures: open `field`'s dial for typing.
    pub fn demo_type(&self, field: usize) {
        if let Some((Ctl::Dial(d), _, _)) = self.controls.borrow().get(&field) {
            d.start_typing(Some("7"));
        }
    }

    /// Design captures: keep the page scrolled to its end.
    pub fn demo_scroll_to_end(&self) {
        self.page_scroll.add_tick_callback(|sw, _| {
            let a = sw.vadjustment();
            a.set_value(a.upper() - a.page_size());
            gtk::glib::ControlFlow::Continue
        });
    }

    /// Open the review bar's sheet of changes.
    pub fn open_review(&self) {
        self.review.open(true);
    }

    pub fn comparing(&self) -> bool {
        self.compare_stash.borrow().is_some()
    }

    /// A new program or edit arrived: stop comparing, without redrawing.
    fn end_compare(&self) {
        self.blend.set(1.0);
        if self.compare_stash.borrow_mut().take().is_some()
            && let Some(page) = self.root.child_by_name("editor")
        {
            page.remove_css_class("comparing");
        }
    }

    fn emit(&self, field: usize, value: u16) {
        if !self.updating.get() && !self.comparing() {
            self.on_change.emit((field, value));
        }
    }

    fn seal(&self) {
        if !self.updating.get() && !self.comparing() {
            self.on_seal.emit(());
        }
    }

    /// Build the chart for page `stage`.
    fn show_stage(self: &Rc<Self>, stage: usize) {
        self.stage.set(stage);
        self.dock_syncing.set(true);
        self.dock_stages.set_active(stage as u32);
        self.dock_syncing.set(false);
        let (title, help) = self
            .curation
            .borrow()
            .pages
            .get(stage)
            .map_or(("", ""), |p| (p.title, p.help));
        self.chart_title.set_label(&tr(title));
        let roll = tr("Roll {stage}").replace("{stage}", &tr(title));
        self.roll.set_tooltip_text(Some(&roll));
        self.roll
            .update_property(&[gtk::accessible::Property::Label(&roll)]);
        self.chart_title
            .set_tooltip_text((!help.is_empty()).then(|| tr(help)).as_deref());
        while let Some(c) = self.chart.first_child() {
            self.chart.remove(&c);
        }
        let mut controls = HashMap::new();
        self.reverts.borrow_mut().clear();
        self.sections.borrow_mut().clear();
        self.build_sections(stage, &self.chart, &mut controls);
        *self.controls.borrow_mut() = controls;
        self.updating.set(true);
        self.refresh_values();
        self.refresh_stages([stage]);
        self.updating.set(false);
        // The new stage's controls settle in (a short rise and fade); the
        // class is set on the next frame so the animation restarts.
        self.chart.remove_css_class("enter");
        self.chart.add_tick_callback(|w, _| {
            w.add_css_class("enter");
            gtk::glib::ControlFlow::Break
        });
    }

    /// The program the controls are built for: the one shown, or the
    /// driver's init program.
    fn program(&self) -> Vec<u8> {
        self.shown
            .borrow()
            .as_ref()
            .map_or_else(|| self.driver().init_program(), |(p, _)| p.clone())
    }

    /// Whether every control is still the one the program asks for (a
    /// depth is a dial in one voice mode and names chords in another).
    fn controls_current(&self) -> bool {
        let d = self.driver();
        let p = self.program();
        self.controls
            .borrow()
            .iter()
            .all(|(f, (_, _, built))| d.control(&p, *f) == *built)
    }

    /// Append page `stage`'s sections and their controls to `into`.
    fn build_sections(self: &Rc<Self>, stage: usize, into: &gtk::Box, controls: &mut Controls) {
        let Some(page) = self.curation.borrow().pages.get(stage).cloned() else {
            return;
        };
        let program = self.program();
        for (s, section) in page.sections.iter().enumerate() {
            let group = gtk::Box::new(gtk::Orientation::Vertical, 8);
            group.add_css_class("chart-section");
            // A section named like its stage needs no second heading.
            if let Some(heading) = oscyl_app::editor::section_heading(&page, s) {
                let h = gtk::Label::builder().label(tr(heading)).xalign(0.0).build();
                h.add_css_class("section-heading");
                h.set_accessible_role(gtk::AccessibleRole::Heading);
                group.append(&h);
            }
            // The dials lead; the selectors (wave, octave, on/off) stack in
            // a column beside them, and the two wrap one under the other
            // when the panel is too narrow for both.
            let flow = |class: &str| {
                let f = adw::WrapBox::builder()
                    .child_spacing(24)
                    .line_spacing(20)
                    .valign(gtk::Align::Start)
                    .build();
                f.add_css_class(class);
                f
            };
            let dials = flow("dials");
            let mut pickers: Vec<gtk::Box> = Vec::new();
            for &field in &section.fields {
                let kind = self.driver().control(&program, field);
                let (widget, ctl) = self.make_control(field, kind, &program);
                // A selector's "was" line: under the control, in space the
                // control keeps for it, so it never moves the layout, with
                // the edited dot by its legend. Dials draw their own.
                let widget = if !matches!(&ctl, Ctl::Dial(_))
                    && let Some(head) = widget.first_child().and_downcast::<gtk::Box>()
                {
                    let revert = self.revert_line(field);
                    head.append(&revert.dot);
                    widget.add_css_class("revertable");
                    let over = gtk::Overlay::builder().child(&widget).build();
                    over.add_overlay(&revert.line);
                    self.reverts.borrow_mut().insert(field, revert);
                    over.upcast()
                } else {
                    widget
                };
                widget.set_halign(gtk::Align::Start);
                // Dials of both sizes centre on one line, so their value
                // numerals share it.
                // Selectors align at the top, so their legends share one
                // line whatever the control's height.
                widget.set_valign(if matches!(&ctl, Ctl::Dial(_)) {
                    gtk::Align::Center
                } else {
                    gtk::Align::Start
                });
                let fb = gtk::Box::new(gtk::Orientation::Vertical, 0);
                fb.set_valign(widget.valign());
                fb.append(&widget);
                if matches!(&ctl, Ctl::Dial(_)) {
                    dials.append(&fb);
                } else {
                    pickers.push(fb.clone());
                }
                controls.insert(field, (ctl, fb, kind));
            }
            // The controls sit in a framed panel under the legend, so a
            // section's start and end are always visible.
            let panel = gtk::Box::new(gtk::Orientation::Vertical, 16);
            panel.add_css_class("section-panel");
            // The panel takes the column's width, never its content's, so
            // an edit's longer caption cannot resize the frame.
            panel.set_hexpand(true);
            panel.set_halign(gtk::Align::Fill);
            let has_dials = dials.first_child().is_some();
            let column: gtk::Widget = if has_dials {
                let b = gtk::Box::new(gtk::Orientation::Vertical, 20);
                pickers.iter().for_each(|p| b.append(p));
                b.upcast()
            } else {
                let f = flow("pickers");
                pickers.iter().for_each(|p| f.append(p));
                f.upcast()
            };
            column.add_css_class("pickers");
            column.set_valign(gtk::Align::Start);
            let row = adw::WrapBox::builder()
                .child_spacing(48)
                .line_spacing(20)
                .build();
            if has_dials {
                row.append(&dials);
            }
            if !pickers.is_empty() {
                row.append(&column);
            }
            panel.append(&row);
            group.append(&panel);
            into.append(&group);
            self.sections
                .borrow_mut()
                .push((group, section.fields.clone()));
        }
    }

    /// The "was" line for selector `field`: an undo icon and the saved
    /// value in the action colour, the way a dial shows it; clicking it
    /// restores the saved value. Hidden until the field is edited.
    fn revert_line(self: &Rc<Self>, field: usize) -> Revert {
        let text = gtk::Label::builder()
            .xalign(0.0)
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .build();
        let content = gtk::Box::new(gtk::Orientation::Horizontal, 4);
        content.append(&gtk::Image::from_icon_name("edit-undo-symbolic"));
        content.append(&text);
        let b = gtk::Button::builder()
            .child(&content)
            .css_classes(["flat", "revert-one"])
            .tooltip_text(tr("Revert to the saved value"))
            .build();
        b.set_cursor_from_name(Some("pointer"));
        b.update_property(&[gtk::accessible::Property::Label(
            &tr("Revert {parameter}").replace("{parameter}", &tr(self.driver().short_label(field))),
        )]);
        let me = self.me.clone();
        b.connect_clicked(move |_| {
            if let Some(me) = me.upgrade() {
                me.on_revert.emit(Some(field));
            }
        });
        let line = gtk::Revealer::builder()
            .child(&b)
            .transition_type(gtk::RevealerTransitionType::Crossfade)
            .transition_duration(150)
            .halign(gtk::Align::Start)
            .valign(gtk::Align::End)
            .build();
        let dot = gtk::Box::builder()
            .css_classes(["edited-dot", "unlit"])
            .valign(gtk::Align::Center)
            .accessible_role(gtk::AccessibleRole::Presentation)
            .build();
        Revert { line, text, dot }
    }

    /// Show each selector's "was" line and edited dot only while edited.
    fn sync_reverts(&self, p: &[u8], base: &[u8]) {
        for (field, r) in self.reverts.borrow().iter() {
            r.show(self.driver(), *field, p, base);
        }
    }

    fn make_control(
        self: &Rc<Self>,
        field: usize,
        kind: Control,
        program: &[u8],
    ) -> (gtk::Widget, Ctl) {
        let d = self.driver();
        let Some(spec) = d.fields().get(field).copied() else {
            return (
                gtk::Box::new(gtk::Orientation::Vertical, 0).upcast(),
                Ctl::Menu(gtk::DropDown::default()),
            );
        };
        let label = tr(&d.label_in(program, field));
        let h = d.help(field);
        let mut tooltip = [h.summary, h.detail]
            .into_iter()
            .filter(|t| !t.is_empty())
            .map(tr)
            .collect::<Vec<_>>()
            .join("\n");
        // Pickers and switches emit and end the undo step at once.
        let me = self.me.clone();
        let emit_now = move |v: u16| {
            if let Some(me) = me.upgrade() {
                me.emit(field, v);
                me.seal();
            }
        };
        let (widget, ctl): (gtk::Widget, Ctl) = match kind {
            Control::Continuous => {
                let dial = Dial::new(&label);
                dial.set_range(spec.min, spec.max);
                dial.set_large(d.emphasized(field));
                // The range lives here rather than under the dial.
                let range = tr("Range {min} to {max}")
                    .replace("{min}", &tr(&d.format_raw(field, spec.min)))
                    .replace("{max}", &tr(&d.format_raw(field, spec.max)));
                let range = format!(
                    "{range}\n{}",
                    tr("Double-click the value, or press Enter, to type it")
                );
                tooltip = if tooltip.is_empty() {
                    range
                } else {
                    format!("{tooltip}\n{range}")
                };
                let shown = self.shown.clone();
                dial.set_formatter(move |raw| format_in_context(d, &shown, field, raw));
                // Typed text as the core reads it: a value's written form
                // (a name too) or the nearest written number.
                let shown = self.shown.clone();
                dial.set_parser(move |text| {
                    let p = shown.borrow().as_ref().map(|(p, _)| p.clone())?;
                    oscyl_app::editor::parse_in(d, &p, field, text)
                });
                let span = spec.max - spec.min;
                dial.set_step_hint(1.max(span / 1023), (span / 16).max(1));
                let me = self.me.clone();
                dial.connect_value_changed(move |_, v| {
                    if let Some(me) = me.upgrade() {
                        me.emit(field, v);
                    }
                });
                // A drag or key run ends when focus leaves: one undo step.
                let focus = gtk::EventControllerFocus::new();
                let me = self.me.clone();
                focus.connect_leave(move |_| {
                    if let Some(me) = me.upgrade() {
                        me.seal();
                    }
                });
                dial.add_controller(focus);
                dial.set_tooltip_text(Some(&tooltip));
                return (dial.clone().upcast(), Ctl::Dial(dial));
            }
            _ => {
                // The core says which widget and what each option sets.
                let pick = move |option: u32| {
                    if let Some(raw) = raw_of(kind, spec.min, spec.max, option as usize) {
                        emit_now(raw);
                    }
                };
                match widget(kind) {
                    Widget::Segments(names) => {
                        let g = adw::ToggleGroup::new();
                        for (i, n) in names.iter().enumerate() {
                            g.add(
                                adw::Toggle::builder()
                                    .label(tr(n))
                                    .name(i.to_string())
                                    .build(),
                            );
                        }
                        g.connect_active_notify(move |g| pick(g.active()));
                        (SegmentTrack::new(&g).upcast(), Ctl::Choice(g))
                    }
                    Widget::Menu(names) => {
                        let translated: Vec<String> = names.iter().map(|n| tr(n)).collect();
                        let refs: Vec<&str> = translated.iter().map(String::as_str).collect();
                        let dd = gtk::DropDown::from_strings(&refs);
                        dd.connect_selected_notify(move |dd| pick(dd.selected()));
                        (dd.clone().upcast(), Ctl::Menu(dd))
                    }
                    Widget::Toggle | Widget::Dial => {
                        let g = adw::ToggleGroup::new();
                        for (i, n) in [tr("Off"), tr("On")].into_iter().enumerate() {
                            g.add(adw::Toggle::builder().label(n).name(i.to_string()).build());
                        }
                        g.set_halign(gtk::Align::Start);
                        g.connect_active_notify(move |g| pick(g.active()));
                        (SegmentTrack::new(&g).upcast(), Ctl::Toggle(g))
                    }
                }
            }
        };
        (labelled(&label, &widget, &tooltip).upcast(), ctl)
    }

    /// Show a program. `base` is the saved/synced version (for edited marks).
    pub fn show(&self, s: Shown) {
        self.end_compare();
        self.updating.set(true);
        self.root.set_visible_child_name("editor");
        if !self.title.is_editing() {
            self.title
                .set_text(&self.driver().name(s.program).unwrap_or_default());
        }
        *self.slot.borrow_mut() = s.slot;
        self.update_header();
        let note = if s.read_only {
            let ro = tr("Factory slot: read-only. Copy it to a library to change it.");
            if s.note.is_empty() {
                ro
            } else {
                format!("{ro}\n{}", s.note)
            }
        } else {
            s.note
        };
        self.note.set_label(&note);
        self.note.set_visible(!note.is_empty());
        // Nothing on a read-only slot takes an edit.
        self.title.set_editable(!s.read_only);
        self.roll.set_sensitive(!s.read_only);
        self.chart.set_sensitive(!s.read_only);
        self.stage_graph.set_sensitive(!s.read_only);
        *self.shown.borrow_mut() = Some((s.program.to_vec(), s.base.to_vec()));
        #[cfg(test)]
        self.full_refreshes.set(self.full_refreshes.get() + 1);
        self.rebuild_if_stale();
        self.refresh_values();
        self.refresh_stages(0..self.pages());
        self.refresh_changes();
        self.updating.set(false);
    }

    /// Build the page's controls again when the program asks for other
    /// ones.
    fn rebuild_if_stale(&self) {
        if !self.controls_current()
            && let Some(me) = self.me.upgrade()
        {
            me.show_stage(self.stage.get());
        }
    }

    /// One field of the shown program changed (a drag step, a key press).
    /// A dial's value redraws its dial, the graphs and its change row;
    /// anything else may change which controls apply and how others read.
    pub fn update_field(&self, field: usize, program: &[u8], base: &[u8]) {
        self.end_compare();
        *self.shown.borrow_mut() = Some((program.to_vec(), base.to_vec()));
        self.updating.set(true);
        let d = self.driver();
        let dial = matches!(
            self.controls.borrow().get(&field),
            Some((Ctl::Dial(_), _, Control::Continuous))
        ) && d.control(program, field) == Control::Continuous;
        if dial && self.controls_current() {
            if let Some((Ctl::Dial(dial), _, _)) = self.controls.borrow().get(&field) {
                dial.show(
                    d.get(program, field).unwrap_or(0),
                    d.get(base, field).unwrap_or(0),
                );
            }
            self.refresh_stages(0..self.pages());
            self.update_change(field, program, base);
        } else {
            #[cfg(test)]
            self.full_refreshes.set(self.full_refreshes.get() + 1);
            self.rebuild_if_stale();
            self.refresh_values();
            self.refresh_stages(0..self.pages());
            self.refresh_changes();
        }
        self.updating.set(false);
    }

    /// Put the shown program into the chart's controls.
    fn refresh_values(&self) {
        let shown = self.shown.borrow();
        let Some((p, base)) = shown.as_ref() else {
            return;
        };
        let d = self.driver();
        apply_values(d, &self.controls.borrow(), p, base);
        self.sync_reverts(p, base);
        for (group, fields) in self.sections.borrow().iter() {
            group.set_visible(oscyl_app::editor::section_shown(d, fields, p));
        }
    }

    /// Graphs, summaries and edited marks of `stages`, on the band and the
    /// chart; each graph is computed once.
    fn refresh_stages(&self, stages: impl IntoIterator<Item = usize>) {
        let shown = self.shown.borrow();
        let Some((p, base)) = shown.as_ref() else {
            return;
        };
        let d = self.driver();
        let curation = self.curation.borrow();
        for stage in stages {
            let Some(page) = curation.pages.get(stage) else {
                continue;
            };
            let (now, before) = (graph_of(d, p, stage), graph_of(d, base, stage));
            let edits = oscyl_app::editor::stage_edits(d, page, p, base);
            let edited = edits > 0;
            let text = d.page_summary_in(p, stage, false, &|s| tr(s));
            let short = d.page_summary_in(p, stage, true, &|s| tr(s));
            let differs = now != before;
            if stage == self.stage.get() {
                let large = large_graph_of(d, p, stage);
                // A page with no picture has no graph row.
                let pictured = large != GraphData::Empty;
                self.stage_graph.set_visible(pictured);
                self.enlarge.set_visible(pictured);
                if !pictured && self.enlarged.get() {
                    self.set_enlarged(false);
                }
                let large_before = differs.then(|| large_graph_of(d, base, stage));
                if self.gliding.get() {
                    self.stage_graph
                        .glide_data(large.clone(), large_before.clone());
                    self.dock_graph.glide_data(large, large_before);
                } else {
                    self.stage_graph
                        .set_data(large.clone(), large_before.clone());
                    self.dock_graph.set_data(large, large_before);
                }
                self.dock_title.set_label(&tr(page.title));
                self.dock_summary.set_label(&text);
                self.stage_graph.set_description(
                    &tr("{stage} graph: {summary}")
                        .replace("{stage}", &tr(page.title))
                        .replace("{summary}", &text),
                );
                self.stage_graph.set_handles(handle_points(d, stage, p));
            }
            if let Some((g, dot)) = self.dock_tiles.borrow().get(stage) {
                let previous = edited.then(|| before.clone());
                if self.gliding.get() {
                    g.glide_data(now.clone(), previous);
                } else {
                    g.set_data(now.clone(), previous);
                }
                if edited {
                    dot.remove_css_class("unlit");
                } else {
                    dot.add_css_class("unlit");
                }
            }
            self.strip
                .set_graph(stage, now, edited.then_some(before), self.gliding.get());
            self.strip.set_edits(stage, edits);
            self.strip.set_summary(stage, &text, &short);
        }
    }

    /// Rebuild the review bar from the shown program.
    fn refresh_changes(&self) {
        if let Some((p, base)) = self.shown.borrow().as_ref() {
            self.review
                .set(self.driver(), &self.curation.borrow(), p, base);
        }
    }

    /// Keep the review bar in step with one field.
    fn update_change(&self, field: usize, p: &[u8], base: &[u8]) {
        self.review
            .update(self.driver(), &self.curation.borrow(), field, p, base);
    }

    /// A graph handle moved to `to`: emit its fields like dial steps.
    fn drag_handle(&self, i: usize, to: (f32, f32)) {
        // Copy the program out: each emit updates `shown`.
        let Some(p) = self.shown.borrow().as_ref().map(|(p, _)| p.clone()) else {
            return;
        };
        if !self.updating.get() && !self.comparing() {
            self.on_gesture.emit(());
        }
        for (field, value) in self.driver().drag(&p, self.stage.get(), i, to) {
            self.emit(field, value);
        }
    }

    /// Grow the stage graph to most of the editor height, or restore it
    /// (Ctrl+I, the Enlarge button).
    pub fn toggle_enlarged(&self) {
        // Nothing to enlarge on a page without a graph.
        if !self.enlarge.is_visible() {
            return;
        }
        self.set_enlarged(!self.enlarged.get());
    }

    /// The stage graph is enlarged.
    pub fn enlarged(&self) -> bool {
        self.enlarged.get()
    }

    /// Restore an enlarged graph; false if it wasn't (Escape).
    pub fn restore_chart(&self) -> bool {
        let was = self.enlarged.get();
        self.set_enlarged(false);
        was
    }

    fn set_enlarged(&self, on: bool) {
        self.enlarged.set(on);
        let (icon, tip) = if on {
            ("view-restore-symbolic", tr("Restore graph (Escape)"))
        } else {
            ("view-fullscreen-symbolic", tr("Enlarge graph (Ctrl+I)"))
        };
        self.enlarge.set_icon_name(icon);
        self.enlarge.set_tooltip_text(Some(&tip));
        let name = if on {
            tr("Restore graph")
        } else {
            tr("Enlarge graph")
        };
        self.enlarge
            .update_property(&[gtk::accessible::Property::Label(&name)]);
        // Enlarged: the graph fills the editor's free height through normal
        // expansion and the dials step aside; the graph's handles stay live.
        self.chart.set_visible(!on);
        self.stage_graph.set_vexpand(on);
    }
}

/// Put a program's values into a set of controls (without emitting).
fn apply_values(d: &dyn Driver, controls: &Controls, p: &[u8], base: &[u8]) {
    for (&field, (ctl, cell, kind)) in controls {
        cell.set_visible(d.relevant(p, field));
        let min = d.fields().get(field).map_or(0, |f| f.min);
        let v = d.get(p, field).unwrap_or(min);
        let b = d.get(base, field).unwrap_or(min);
        let option = |raw| option_of(*kind, min, raw).unwrap_or(0) as u32;
        match ctl {
            Ctl::Dial(dial) => {
                // A depth reads as a detune in one mode and a duo amount
                // in another.
                let label = tr(&d.label_in(p, field));
                dial.set_label(&label);
                dial.update_property(&[gtk::accessible::Property::Label(&label)]);
                dial.show(v, b);
            }
            Ctl::Choice(g) | Ctl::Toggle(g) => {
                g.set_active(option(v));
                mark_saved(g, (v != b).then(|| option(b)));
            }
            Ctl::Menu(dd) => dd.set_selected(option(v)),
        }
    }
}

/// Show which of a selector's options is the saved one, under it.
fn mark_saved(g: &adw::ToggleGroup, saved: Option<u32>) {
    if let Some(track) = g.parent().and_downcast::<SegmentTrack>() {
        track.set_saved(saved);
    }
}

/// `raw` for `field`, read in the context of the shown program (a voice
/// mode depth names a chord or an arpeggio depending on the mode).
fn format_in_context(d: &dyn Driver, shown: &Pair, field: usize, raw: u16) -> String {
    match shown.borrow().as_ref() {
        Some((p, _)) => oscyl_app::editor::format_in(d, p, field, raw),
        None => tr(&d.format_raw(field, raw)),
    }
}

/// "560 to 618": a changed field's saved and current values.
pub(crate) fn change_text(before: &str, after: &str) -> String {
    tr("{before} to {after}")
        .replace("{before}", before)
        .replace("{after}", after)
}

/// Page `page`'s picture of `p` on the strip and in tiles.
pub fn graph_of(d: &dyn Driver, p: &[u8], page: usize) -> GraphData {
    oscyl_app::editor::graph(d, p, page, 128)
        .map(GraphData::from)
        .unwrap_or_default()
}

/// The page's large graph: the band's shape, drawn finer.
fn large_graph_of(d: &dyn Driver, p: &[u8], page: usize) -> GraphData {
    oscyl_app::editor::graph(d, p, page, 512)
        .map(GraphData::from)
        .unwrap_or_default()
}

/// The large graph's handles for `p`, with their names and values.
fn handle_points(d: &dyn Driver, page: usize, p: &[u8]) -> Vec<HandlePoint> {
    d.handles(p, page)
        .into_iter()
        .map(|h| {
            let value = oscyl_app::editor::handle_value(d, p, &h);
            HandlePoint {
                at: h.at,
                axes: (h.x.is_some(), h.y.is_some()),
                name: tr(h.name),
                value,
            }
        })
        .collect()
}

/// A selector's way back: its "was" line and the edited dot by its legend.
struct Revert {
    line: gtk::Revealer,
    text: gtk::Label,
    dot: gtk::Box,
}

impl Revert {
    fn show(&self, d: &dyn Driver, field: usize, p: &[u8], base: &[u8]) {
        let edited = oscyl_app::editor::is_edited(d, field, p, base);
        if edited {
            let was = tr(&d.format_value(base, field));
            self.text
                .set_label(&tr("was {value}").replace("{value}", &was));
        }
        self.line.set_reveal_child(edited);
        // The dot keeps its space either way, so the legend never shifts.
        if edited {
            self.dot.remove_css_class("unlit");
        } else {
            self.dot.add_css_class("unlit");
        }
    }
}

/// Replace (or add) the stack child `name` and show it.
pub fn set_page(stack: &gtk::Stack, name: &str, page: &impl IsA<gtk::Widget>) {
    if let Some(old) = stack.child_by_name(name) {
        stack.remove(&old);
    }
    stack.add_named(page, Some(name));
    stack.set_visible_child_name(name);
}

/// A control with its label above it.
/// A control with its legend above it; the legend row also holds the
/// control's revert button.
fn labelled(label: &str, widget: &gtk::Widget, tooltip: &str) -> gtk::Box {
    let b = gtk::Box::new(gtk::Orientation::Vertical, 6);
    let head = gtk::Box::new(gtk::Orientation::Horizontal, 4);
    head.add_css_class("control-head");
    let l = gtk::Label::builder().label(label).xalign(0.0).build();
    l.add_css_class("control-label");
    widget.update_property(&[gtk::accessible::Property::Label(label)]);
    widget.set_tooltip_text(Some(tooltip));
    head.append(&l);
    b.append(&head);
    b.append(widget);
    b
}

#[cfg(test)]
mod tests {
    use super::*;
    use oscyl_minilogue_xd::{Field, Program};

    const XD: &dyn Driver = &oscyl_minilogue_xd::DRIVER;

    fn idx(f: Field) -> usize {
        Field::ALL.iter().position(|g| *g == f).unwrap()
    }

    fn page(icon: &str) -> usize {
        XD.curation()
            .pages
            .iter()
            .position(|p| p.icon == Some(icon))
            .unwrap()
    }

    fn shown<'a>(p: &'a [u8], base: &'a [u8]) -> Shown<'a> {
        Shown {
            slot: String::new(),
            program: p,
            base,
            note: String::new(),
            read_only: false,
        }
    }

    /// A factory slot shows, but takes no edit.
    #[test]
    fn a_read_only_slot_takes_no_edit() {
        crate::on_gtk_thread(|| {
            let editor = Editor::new(XD);
            let p = XD.init_program();
            editor.show(Shown {
                read_only: true,
                ..shown(&p, &p)
            });
            assert!(!editor.chart.is_sensitive() && !editor.title.is_editable());
            editor.show(shown(&p, &p));
            assert!(editor.chart.is_sensitive() && editor.title.is_editable());
        });
    }

    /// A drag step updates one dial, its stages and its change row; it
    /// never redraws the whole editor. Needs a display; skipped without.
    #[test]
    fn dial_edits_stay_incremental() {
        crate::on_gtk_thread(|| {
            let editor = Editor::new(XD);
            let base = Program::init();
            editor.show(shown(base.as_bytes(), base.as_bytes()));
            editor.select_stage(page("oscyl-filter-symbolic"));
            let full = editor.full_refreshes.get();
            let mut p = base.clone();
            for v in [600, 610, 620] {
                p.set(Field::Cutoff, v).unwrap();
                editor.update_field(idx(Field::Cutoff), p.as_bytes(), base.as_bytes());
            }
            assert_eq!(editor.full_refreshes.get(), full);
            assert_eq!(editor.review.rows(), 1);
            let ctls = editor.controls.borrow();
            let Some((Ctl::Dial(d), _, _)) = ctls.get(&idx(Field::Cutoff)) else {
                panic!("cutoff dial");
            };
            assert_eq!(d.value(), 620);
        });
    }

    /// Switching the voice mode turns the depth dial into a chord picker
    /// and back: the control follows `Driver::control`.
    #[test]
    fn a_control_follows_the_program() {
        crate::on_gtk_thread(|| {
            let editor = Editor::new(XD);
            let base = Program::init();
            editor.show(shown(base.as_bytes(), base.as_bytes()));
            editor.select_stage(page("oscyl-voice-symbolic"));
            let depth = idx(Field::VoiceModeDepth);
            let is_dial =
                |e: &Editor| matches!(e.controls.borrow().get(&depth), Some((Ctl::Dial(_), _, _)));
            assert!(is_dial(&editor));
            let mut p = base.clone();
            p.set(Field::VoiceModeType, 2).unwrap();
            editor.update_field(idx(Field::VoiceModeType), p.as_bytes(), base.as_bytes());
            assert!(matches!(
                editor.controls.borrow().get(&depth),
                Some((Ctl::Menu(..), _, _))
            ));
            editor.update_field(idx(Field::VoiceModeType), base.as_bytes(), base.as_bytes());
            assert!(is_dial(&editor));
        });
    }

    /// A Rev2 program shows on the Rev2's own pages.
    #[test]
    fn another_driver_brings_its_pages() {
        crate::on_gtk_thread(|| {
            let rev2 = oscyl_midi::driver("sequential-prophet-rev2").unwrap();
            let editor = Editor::new(XD);
            editor.set_driver(rev2);
            assert_eq!(editor.pages(), rev2.curation().pages.len());
            let p = rev2.init_program();
            editor.show(shown(&p, &p));
            for i in 0..editor.pages() {
                editor.select_stage(i);
                assert_eq!(editor.stage.get(), i);
            }
        });
    }

    /// The docked scope's stage switcher and the editor's stage follow
    /// each other both ways.
    #[test]
    fn dock_switcher_follows_the_stage() {
        crate::on_gtk_thread(|| {
            let editor = Editor::new(XD);
            let base = Program::init();
            editor.show(shown(base.as_bytes(), base.as_bytes()));
            let (filter, effects) = (
                page("oscyl-filter-symbolic"),
                page("oscyl-effects-symbolic"),
            );
            editor.select_stage(filter);
            assert_eq!(editor.dock_stages.active(), filter as u32);
            editor.dock_stages.set_active(effects as u32);
            assert_eq!(editor.stage.get(), effects);
            assert_eq!(editor.strip.selected(), effects);
        });
    }

    /// Holding Compare shows the saved values and ignores edits; letting go
    /// brings the edited values back, and a new program ends a compare.
    #[test]
    fn compare_shows_saved_values_and_returns() {
        crate::on_gtk_thread(|| {
            let editor = Editor::new(XD);
            let base = Program::init();
            let mut p = base.clone();
            p.set(Field::Cutoff, 700).unwrap();
            editor.select_stage(page("oscyl-filter-symbolic"));
            editor.show(shown(p.as_bytes(), base.as_bytes()));
            let emitted = Rc::new(Cell::new(0));
            {
                let emitted = emitted.clone();
                editor.connect_change(move |_| emitted.set(emitted.get() + 1));
            }
            let cutoff = || match editor.controls.borrow().get(&idx(Field::Cutoff)) {
                Some((Ctl::Dial(d), _, _)) => d.clone(),
                _ => panic!("cutoff dial"),
            };
            editor.compare(true);
            assert!(editor.comparing());
            assert_eq!(cutoff().value(), base.get(Field::Cutoff));
            editor.emit(idx(Field::Cutoff), 300);
            assert_eq!(emitted.get(), 0, "edits are ignored while comparing");
            // Half way: a blend between saved and edited, edits still ignored.
            editor.set_blend(0.5, false);
            let saved = base.get(Field::Cutoff);
            let mid = cutoff().value();
            assert!(mid > saved.min(700) && mid < saved.max(700), "{mid}");
            editor.emit(idx(Field::Cutoff), 300);
            assert_eq!(emitted.get(), 0, "edits are ignored while blending");
            editor.compare(false);
            assert_eq!(cutoff().value(), 700);
            // A dial the compare did not move never keeps the slow glide.
            match editor.controls.borrow().get(&idx(Field::Resonance)) {
                Some((Ctl::Dial(d), _, _)) => assert!(!d.slow_glide()),
                _ => panic!("resonance dial"),
            }
            editor.compare(true);
            editor.show(shown(p.as_bytes(), base.as_bytes()));
            assert!(!editor.comparing());
            assert_eq!(cutoff().value(), 700);
        });
    }

    /// Dragging the filter peak emits cutoff and resonance through the
    /// same path as the dials.
    #[test]
    fn handle_drags_emit_like_dials() {
        crate::on_gtk_thread(|| {
            let editor = Editor::new(XD);
            let base = Program::init();
            editor.show(shown(base.as_bytes(), base.as_bytes()));
            editor.select_stage(page("oscyl-filter-symbolic"));
            let changes = Rc::new(RefCell::new(Vec::new()));
            let c = changes.clone();
            editor.connect_change(move |fv| c.borrow_mut().push(fv));
            editor.drag_handle(0, (0.5, 0.8));
            let fields: Vec<usize> = changes.borrow().iter().map(|c| c.0).collect();
            assert_eq!(fields, [idx(Field::Cutoff), idx(Field::Resonance)]);
        });
    }

    /// A page with no picture shows no empty graph frame and no Enlarge.
    #[test]
    fn a_page_without_a_graph_hides_the_graph_row() {
        crate::on_gtk_thread(|| {
            let rev2 = oscyl_midi::driver("sequential-prophet-rev2").unwrap();
            let p = rev2.init_program();
            let bare = (0..rev2.curation().pages.len())
                .find(|&i| rev2.graphs(&p, i, 512).is_empty())
                .expect("a Rev2 page without a graph");
            let drawn = (0..rev2.curation().pages.len())
                .find(|&i| !rev2.graphs(&p, i, 512).is_empty())
                .expect("a Rev2 page with a graph");
            let editor = Editor::new(rev2);
            editor.show(shown(&p, &p));
            editor.select_stage(bare);
            assert!(!editor.stage_graph.is_visible() && !editor.enlarge.is_visible());
            editor.select_stage(drawn);
            assert!(editor.stage_graph.is_visible() && editor.enlarge.is_visible());
        });
    }
}
