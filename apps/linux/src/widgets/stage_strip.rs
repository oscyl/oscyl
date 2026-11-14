//! The chart band: one ruled band across the editor, a column per stage
//! separated by hairline rules, with a single pen line running through
//! every column's mini graph (wave, mix bars, filter curve, envelope,
//! echo taps). It behaves as a single-selection tab list.
//!
//! Each column is a toggle button (accessible role Tab) holding the title
//! (never truncated), an orange count of its edits, the live graph and a
//! one-line summary; no tooltip, so nothing covers the graph while you
//! move along the tabs (the stage heading carries its description). The
//! selected column is marked by a pen title and underline only.
//! Left/Right/Home/End move between tabs; Alt+1…6 jump to a stage from
//! anywhere in the window. Narrower than its columns, the band scrolls
//! sideways without a visible scrollbar: an end with more tabs past it
//! fades into the ground under an arrow that glides there, and the chosen
//! tab always scrolls clear of that zone.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use adw::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{gdk, glib};

use super::{Graph, GraphData};
use crate::i18n::tr;

/// Called with the page index the user picked.
type StageFn = Rc<dyn Fn(usize)>;

struct Tile {
    /// The page's title, translated.
    title: String,
    button: gtk::ToggleButton,
    /// The summary, and its short form for narrow columns.
    summary: gtk::Label,
    short: gtk::Label,
    graph: Graph,
    tick: gtk::Label,
}

/// Index of the tab a navigation key moves to from `current`.
pub fn key_tab(key: gdk::Key, current: usize, count: usize) -> Option<usize> {
    let last = count.checked_sub(1)?;
    match key {
        gdk::Key::Left | gdk::Key::KP_Left => Some(current.saturating_sub(1)),
        gdk::Key::Right | gdk::Key::KP_Right => Some((current + 1).min(last)),
        gdk::Key::Home | gdk::Key::KP_Home => Some(0),
        gdk::Key::End | gdk::Key::KP_End => Some(last),
        _ => None,
    }
}

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct StageStrip {
        pub(super) tiles: RefCell<Vec<Tile>>,
        pub(super) selected: Cell<usize>,
        /// Set while `select()` runs, so programmatic changes don't notify.
        pub(super) quiet: Cell<bool>,
        /// Scroll the selected tile into view after the next layout.
        pub(super) reveal: Cell<bool>,
        /// The width of the last layout; a new width re-reveals.
        pub(super) width: Cell<i32>,
        pub(super) on_selected: RefCell<Vec<StageFn>>,
        /// The selected tab's plate, sliding between tabs.
        pub(super) plate: super::super::plate::Glide,
        pub(super) swatches: std::cell::OnceCell<super::super::Swatches>,
        /// The tabs scrolled: the next frame moves the plate with them at
        /// once (the scroll is laid out by then), never gliding after.
        pub(super) scrolled: Cell<bool>,
        /// Arrows at the ends that say there is more, and scroll to it.
        pub(super) arrows: std::cell::OnceCell<[gtk::Button; 2]>,
        /// The scroll animation the arrows start.
        pub(super) scroll_anim: std::cell::OnceCell<adw::TimedAnimation>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for StageStrip {
        const NAME: &'static str = "OscylStageStrip";
        type Type = super::StageStrip;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.set_css_name("stagestrip");
        }
    }

    impl ObjectImpl for StageStrip {
        fn constructed(&self) {
            self.parent_constructed();
            build(&self.obj());
            // After the scroller, which stays the first child.
            let _ = self.swatches.set(super::super::Swatches::new(
                &*self.obj(),
                &["plate", "ground", "shadow"],
            ));
            let _ = self.arrows.set(arrows(&self.obj()));
        }

        fn dispose(&self) {
            while let Some(child) = self.obj().first_child() {
                child.unparent();
            }
        }
    }

    impl WidgetImpl for StageStrip {
        fn snapshot(&self, s: &gtk::Snapshot) {
            // The plate under the tabs, clipped to the scroller's view.
            // Catch up with layout changes the selection signal did not
            // report (a tab shown after a resize, a scroll).
            let obj = self.obj();
            obj.place_plate(self.scrolled.take());
            self.plate.tick();
            let Some(scroller) = obj.first_child() else {
                return;
            };
            let view = scroller.compute_bounds(&*obj);
            if let (Some(rect), Some(sw), Some(view)) =
                (self.plate.current(), self.swatches.get(), view)
            {
                s.push_clip(&view);
                super::super::plate::draw_plate(s, &rect, 6.0, &sw.color(0), Some(&sw.color(2)));
                s.pop();
            }
            obj.snapshot_child(&scroller, s);
            // Where tabs run past an edge, they fade into the ground under
            // an arrow, so a cut-off tab reads as "more this way".
            if let (Some(view), Some(sw), Some(arrows)) =
                (view, self.swatches.get(), self.arrows.get())
            {
                let ground = sw.color(1);
                let clear = super::super::fade(&ground, 0.0);
                let w = (3.2 * super::super::em(&*obj)).min(view.width() / 3.0);
                for (arrow, left) in arrows.iter().zip([true, false]) {
                    if !arrow.is_visible() {
                        continue;
                    }
                    let (x, from, to) = if left {
                        (view.x(), view.x(), view.x() + w)
                    } else {
                        (
                            view.x() + view.width() - w,
                            view.x() + view.width(),
                            view.x() + view.width() - w,
                        )
                    };
                    let rect = gtk::graphene::Rect::new(x, view.y(), w, view.height());
                    s.append_linear_gradient(
                        &rect,
                        &gtk::graphene::Point::new(from, 0.0),
                        &gtk::graphene::Point::new(to, 0.0),
                        &[
                            gtk::gsk::ColorStop::new(0.0, ground),
                            gtk::gsk::ColorStop::new(0.35, super::super::fade(&ground, 0.85)),
                            gtk::gsk::ColorStop::new(1.0, clear),
                        ],
                    );
                    obj.snapshot_child(arrow, s);
                }
            }
        }

        fn measure(&self, orientation: gtk::Orientation, for_size: i32) -> (i32, i32, i32, i32) {
            match self.obj().first_child() {
                Some(scroller) => {
                    let (min, nat, _, _) = scroller.measure(orientation, for_size);
                    (min, nat, -1, -1)
                }
                None => (0, 0, -1, -1),
            }
        }

        fn size_allocate(&self, width: i32, height: i32, baseline: i32) {
            if let Some(scroller) = self.obj().first_child() {
                scroller.allocate(width, height, baseline, None);
            }
            // The arrows sit on the frame's inner edges, centred.
            if let Some(arrows) = self.arrows.get() {
                for (arrow, left) in arrows.iter().zip([true, false]) {
                    let (_, w, _, _) = arrow.measure(gtk::Orientation::Horizontal, -1);
                    let (_, h, _, _) = arrow.measure(gtk::Orientation::Vertical, w);
                    let x = if left { 4 } else { width - w - 4 };
                    let t = gtk::gsk::Transform::new().translate(&gtk::graphene::Point::new(
                        x as f32,
                        ((height - h) / 2) as f32,
                    ));
                    arrow.allocate(w, h, -1, Some(t));
                }
            }
            // After a first layout or a width change (a breakpoint, a
            // resize), bring the selected tile back into view once the
            // scroller knows its new range: on the next frame, driven by
            // the frame clock rather than a timer.
            let resized = self.width.get() != width;
            self.obj().place_plate(resized);
            if self.reveal.take() || self.width.replace(width) != width {
                self.obj().add_tick_callback(|strip, _| {
                    strip.reveal_selected();
                    glib::ControlFlow::Break
                });
            }
        }
    }
}

glib::wrapper! {
    /// The stage tab strip; see the module docs.
    pub struct StageStrip(ObjectSubclass<imp::StageStrip>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for StageStrip {
    fn default() -> Self {
        Self::new()
    }
}

impl StageStrip {
    pub fn new() -> Self {
        glib::Object::new()
    }

    /// One tab per page, titled `titles`, replacing the tabs there were.
    pub fn set_pages(&self, titles: &[String]) {
        let Some(flow) = self.flow().and_downcast::<gtk::Box>() else {
            return;
        };
        while let Some(c) = flow.first_child() {
            flow.remove(&c);
        }
        self.imp().tiles.borrow_mut().clear();
        self.imp().selected.set(0);
        let mut tiles: Vec<Tile> = Vec::new();
        for (i, title) in titles.iter().enumerate() {
            let first = tiles.first().map(|t| t.button.clone());
            let t = tile(self, i, title, first.as_ref());
            flow.append(&t.button);
            tiles.push(t);
        }
        self.imp().tiles.replace(tiles);
    }

    /// Select page `stage` without notifying `connect_stage_selected`.
    pub fn select(&self, stage: usize) {
        self.imp().quiet.set(true);
        self.activate_tab(stage, false);
        self.imp().quiet.set(false);
    }

    pub fn selected(&self) -> usize {
        self.imp().selected.get()
    }

    /// Called when the user picks a page (click, keys, Alt+N).
    pub fn connect_stage_selected(&self, f: impl Fn(usize) + 'static) {
        self.imp().on_selected.borrow_mut().push(Rc::new(f));
    }

    /// The column's summary; `short` shows when `text` doesn't fit.
    pub fn set_summary(&self, stage: usize, text: &str, short: &str) {
        if let Some(t) = self.imp().tiles.borrow().get(stage) {
            t.summary.set_label(text);
            t.short.set_label(short);
        }
        self.describe(stage);
    }

    /// The tab's picture; `glide` moves it from the old shape (a compare).
    pub fn set_graph(
        &self,
        stage: usize,
        current: GraphData,
        previous: Option<GraphData>,
        glide: bool,
    ) {
        if let Some(t) = self.imp().tiles.borrow().get(stage) {
            if glide {
                t.graph.glide_data(current, previous);
            } else {
                t.graph.set_data(current, previous);
            }
        }
    }

    /// How many of the stage's parameters are edited (0: none). The count
    /// keeps its space either way, so tabs never change width.
    pub fn set_edits(&self, stage: usize, n: usize) {
        if let Some(t) = self.imp().tiles.borrow().get(stage) {
            t.tick.set_label(&n.max(1).to_string());
            if n > 0 {
                t.tick.remove_css_class("unlit");
            } else {
                t.tick.add_css_class("unlit");
            }
        }
        self.describe(stage);
    }

    /// The tab's accessible description: its summary, plus "edited".
    fn describe(&self, stage: usize) {
        if let Some(t) = self.imp().tiles.borrow().get(stage) {
            let mut text = t.summary.label().to_string();
            if !t.tick.has_css_class("unlit") {
                text = if text.is_empty() {
                    tr("Edited")
                } else {
                    tr("{summary}, edited").replace("{summary}", &text)
                };
            }
            t.graph
                .set_description(&tr("{stage} graph").replace("{stage}", &t.title));
            t.button
                .update_property(&[gtk::accessible::Property::Description(&text)]);
        }
    }

    /// Aim the plate at the selected tab; `snap` places it without a
    /// glide (layout changes, scrolling).
    fn place_plate(&self, snap: bool) {
        let tile = self
            .imp()
            .tiles
            .borrow()
            .get(self.imp().selected.get())
            .map(|t| t.button.clone());
        // Not laid out yet: the first allocation places the plate.
        let tile = tile.filter(|b| b.width() > 0);
        if let Some(rect) = tile.and_then(|b| b.compute_bounds(self)) {
            let first = self.imp().plate.current().is_none();
            self.imp().plate.retarget(self, rect, snap || first);
        }
    }

    /// Show each end's arrow only while tabs run past that end.
    fn sync_arrows(&self) {
        let (Some(adj), Some(arrows)) = (self.hadjustment(), self.imp().arrows.get()) else {
            return;
        };
        let more = adj.upper() - adj.page_size() - adj.lower() > 1.0;
        arrows[0].set_visible(more && adj.value() > adj.lower() + 1.0);
        arrows[1].set_visible(more && adj.value() < adj.upper() - adj.page_size() - 1.0);
    }

    fn hadjustment(&self) -> Option<gtk::Adjustment> {
        Some(
            self.first_child()
                .and_downcast::<gtk::ScrolledWindow>()?
                .hadjustment(),
        )
    }

    /// Glide the tabs most of a view towards `dir` (-1 left, 1 right).
    fn scroll_by_page(&self, dir: f64) {
        if let Some(adj) = self.hadjustment() {
            self.glide_to(adj.value() + dir * adj.page_size() * 0.75, true);
        }
    }

    /// Scroll to `value`, gliding or at once.
    fn glide_to(&self, value: f64, glide: bool) {
        let Some(adj) = self.hadjustment() else {
            return;
        };
        let end = value.clamp(
            adj.lower(),
            (adj.upper() - adj.page_size()).max(adj.lower()),
        );
        if !glide {
            if let Some(a) = self.imp().scroll_anim.get() {
                a.pause();
            }
            adj.set_value(end);
            return;
        }
        let anim = self.imp().scroll_anim.get_or_init(|| {
            adw::TimedAnimation::builder()
                .widget(self)
                .target(&adw::PropertyAnimationTarget::new(&adj, "value"))
                .duration(220)
                .easing(adw::Easing::EaseOutCubic)
                .build()
        });
        anim.pause();
        anim.set_value_from(adj.value());
        anim.set_value_to(end);
        anim.play();
    }

    /// Scroll the selected tile fully into view (no motion if it is).
    fn reveal_selected(&self) {
        let tile = self
            .imp()
            .tiles
            .borrow()
            .get(self.imp().selected.get())
            .map(|t| t.button.clone());
        if let Some(tile) = tile {
            self.reveal(&tile, false);
        }
    }

    /// Bring `tile` into view clear of the end arrows and fades, so the
    /// chosen tab is never half hidden under them.
    fn reveal(&self, tile: &gtk::ToggleButton, glide: bool) {
        let (Some(adj), Some(flow)) = (self.hadjustment(), self.flow()) else {
            return;
        };
        let Some(b) = tile.compute_bounds(&flow) else {
            return;
        };
        let (x, w) = (f64::from(b.x()), f64::from(b.width()));
        let (first, last) = (x <= adj.lower() + 1.0, x + w >= adj.upper() - 1.0);
        // The arrow zone, unless the tile is already at that end.
        let pad = 2.6 * f64::from(super::em(self));
        let (lo, hi) = (
            if first { adj.lower() } else { x - pad },
            if last { adj.upper() } else { x + w + pad },
        );
        let v = adj.value();
        if lo < v {
            self.glide_to(lo, glide);
        } else if hi > v + adj.page_size() {
            self.glide_to(hi - adj.page_size(), glide);
        }
    }

    /// The scrolled tile row.
    fn flow(&self) -> Option<gtk::Widget> {
        self.first_child()
            .and_downcast::<gtk::ScrolledWindow>()?
            .child()
            .and_downcast::<gtk::Viewport>()?
            .child()
    }

    fn activate_tab(&self, i: usize, focus: bool) {
        let button = self.imp().tiles.borrow().get(i).map(|t| t.button.clone());
        if let Some(b) = button {
            b.set_active(true);
            if focus {
                b.grab_focus();
            }
            // Bring the tile into view: now if laid out, else at the first
            // layout.
            if b.width() > 0 {
                self.reveal(&b, true);
            } else {
                self.imp().reveal.set(true);
            }
        }
    }

    fn on_toggled(&self, i: usize, active: bool) {
        if let Some(t) = self.imp().tiles.borrow().get(i) {
            t.button
                .update_state(&[gtk::accessible::State::Selected(Some(active))]);
        }
        if !active {
            return;
        }
        self.imp().selected.set(i);
        self.place_plate(false);
        if self.imp().quiet.get() {
            return;
        }
        let callbacks = self.imp().on_selected.borrow().clone();
        callbacks.iter().for_each(|f| f(i));
    }
}

fn tile(strip: &StageStrip, i: usize, name: &str, group: Option<&gtk::ToggleButton>) -> Tile {
    let title = gtk::Label::builder()
        .label(name)
        .xalign(0.0)
        .css_classes(["stage-title"])
        .build();
    // The edit count: an orange box at the tab's top right.
    let tick = gtk::Label::builder()
        .css_classes(["stage-count", "numeric", "unlit"])
        .valign(gtk::Align::Center)
        .accessible_role(gtk::AccessibleRole::Presentation)
        .build();
    let head = gtk::Box::builder()
        .spacing(6)
        .css_classes(["stage-head"])
        .build();
    title.set_hexpand(true);
    head.append(&title);
    head.append(&tick);

    let (summary_box, summary, short) = summary_pair();
    let graph = Graph::new();
    graph.add_css_class("mini");
    graph.set_compact();

    let body = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .build();
    body.append(&head);
    body.append(&graph);
    body.append(&summary_box);

    let button = gtk::ToggleButton::builder()
        .child(&body)
        .css_classes(["stage-col"])
        .accessible_role(gtk::AccessibleRole::Tab)
        .hexpand(true)
        .build();
    button.set_group(group);
    button.update_property(&[gtk::accessible::Property::Label(name)]);
    button.update_state(&[gtk::accessible::State::Selected(Some(false))]);
    button.connect_toggled(glib::clone!(
        #[weak]
        strip,
        move |b| strip.on_toggled(i, b.is_active())
    ));

    Tile {
        title: name.to_string(),
        button,
        summary,
        short,
        graph,
        tick,
    }
}

/// The summary line: the full text where it fits, else the short form,
/// never cut off. The column's minimum width is the shorter one's; the
/// one that doesn't show stays laid out but transparent, so switching
/// never asks for a new layout. The tab's description carries the text.
fn summary_pair() -> (gtk::Widget, gtk::Label, gtk::Label) {
    let label = || {
        // Summaries never set a tab's width: they shorten, then ellipsize,
        // so switching programs or effects never resizes the strip (or the
        // window).
        gtk::Label::builder()
            .xalign(0.0)
            .ellipsize(pango::EllipsizeMode::End)
            .accessible_role(gtk::AccessibleRole::Presentation)
            .build()
    };
    let (long, short) = (label(), label());
    let b: Summary = glib::Object::new();
    b.set_css_classes(&["stage-summary", "dim-label", "numeric"]);
    b.set_overflow(gtk::Overflow::Hidden);
    long.set_parent(&b);
    short.set_parent(&b);
    (b.upcast(), long, short)
}

mod summary {
    use super::*;

    #[derive(Default)]
    pub struct Summary;

    #[glib::object_subclass]
    impl ObjectSubclass for Summary {
        const NAME: &'static str = "OscylStageSummary";
        type Type = super::Summary;
        type ParentType = gtk::Widget;
    }

    impl ObjectImpl for Summary {
        fn dispose(&self) {
            while let Some(child) = self.obj().first_child() {
                child.unparent();
            }
        }
    }

    fn size(w: &gtk::Widget, o: gtk::Orientation) -> (i32, i32) {
        let (min, nat, _, _) = w.measure(o, -1);
        (min, nat)
    }

    impl WidgetImpl for Summary {
        fn measure(&self, o: gtk::Orientation, _for_size: i32) -> (i32, i32, i32, i32) {
            let obj = self.obj();
            let (Some(l), Some(s)) = (obj.first_child(), obj.last_child()) else {
                return (0, 0, -1, -1);
            };
            let ((lm, ln), (sm, sn)) = (size(&l, o), size(&s, o));
            match o {
                gtk::Orientation::Horizontal => (lm.min(sm), ln.max(sn), -1, -1),
                _ => (lm.max(sm), ln.max(sn), -1, -1),
            }
        }

        fn size_allocate(&self, w: i32, h: i32, _baseline: i32) {
            let obj = self.obj();
            let (Some(l), Some(s)) = (obj.first_child(), obj.last_child()) else {
                return;
            };
            let fits = size(&l, gtk::Orientation::Horizontal).1 <= w;
            for (c, on) in [(&l, fits), (&s, !fits)] {
                c.set_opacity(if on { 1.0 } else { 0.0 });
                let cw = w.max(size(c, gtk::Orientation::Horizontal).0);
                c.allocate(cw, h, -1, None);
            }
        }
    }
}

glib::wrapper! {
    pub struct Summary(ObjectSubclass<summary::Summary>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

/// The two end arrows; hidden until tabs run past that end.
fn arrows(strip: &StageStrip) -> [gtk::Button; 2] {
    [
        ("go-previous-symbolic", -1.0, tr("Earlier stages")),
        ("go-next-symbolic", 1.0, tr("Later stages")),
    ]
    .map(|(icon, dir, label)| {
        let b = gtk::Button::builder()
            .icon_name(icon)
            .css_classes(["flat", "circular", "strip-arrow"])
            .focusable(false)
            .tooltip_text(label.as_str())
            .visible(false)
            .build();
        // Keyboard users move through tabs with the arrow keys; the
        // buttons are for pointer and touch.
        b.update_property(&[gtk::accessible::Property::Label(&label)]);
        b.connect_clicked(glib::clone!(
            #[weak]
            strip,
            move |_| strip.scroll_by_page(dir)
        ));
        b.set_parent(strip);
        b
    })
}

fn build(strip: &StageStrip) {
    // Equal tabs: their widths never follow their text.
    let flow = gtk::Box::builder()
        .homogeneous(true)
        .css_classes(["flow"])
        .accessible_role(gtk::AccessibleRole::TabList)
        .build();
    flow.update_property(&[gtk::accessible::Property::Label(&tr("Signal flow"))]);

    let keys = gtk::EventControllerKey::new();
    keys.connect_key_pressed(glib::clone!(
        #[weak]
        strip,
        #[upgrade_or]
        glib::Propagation::Proceed,
        move |_, key, _, state| {
            let focused = strip
                .imp()
                .tiles
                .borrow()
                .iter()
                .position(|t| t.button.has_focus());
            let Some(current) = focused else {
                return glib::Propagation::Proceed;
            };
            if !state.is_empty() {
                return glib::Propagation::Proceed;
            }
            let count = strip.imp().tiles.borrow().len();
            match key_tab(key, current, count) {
                Some(i) => {
                    strip.activate_tab(i, true);
                    glib::Propagation::Stop
                }
                None => glib::Propagation::Proceed,
            }
        }
    ));
    flow.add_controller(keys);

    let shortcuts = gtk::ShortcutController::new();
    shortcuts.set_scope(gtk::ShortcutScope::Global);
    let digits = [
        gdk::Key::_1,
        gdk::Key::_2,
        gdk::Key::_3,
        gdk::Key::_4,
        gdk::Key::_5,
        gdk::Key::_6,
        gdk::Key::_7,
        gdk::Key::_8,
        gdk::Key::_9,
    ];
    for (i, key) in digits.into_iter().enumerate() {
        let action = gtk::CallbackAction::new(glib::clone!(
            #[weak]
            strip,
            #[upgrade_or]
            glib::Propagation::Proceed,
            move |_, _| {
                strip.activate_tab(i, true);
                glib::Propagation::Stop
            }
        ));
        shortcuts.add_shortcut(gtk::Shortcut::new(
            Some(gtk::KeyvalTrigger::new(key, gdk::ModifierType::ALT_MASK)),
            Some(action),
        ));
    }
    strip.add_controller(shortcuts);

    let scroller = gtk::ScrolledWindow::builder()
        // Scrolls (touch, wheel, keys follow the selection) but shows no
        // scrollbar: edge fades and arrows say there is more.
        .hscrollbar_policy(gtk::PolicyType::External)
        .vscrollbar_policy(gtk::PolicyType::Never)
        .child(&flow)
        .build();
    scroller.set_parent(strip);
    // Scrolling moves the tabs; on the next frame, once they are laid
    // out at their new place, the plate moves with them at once.
    let adj = scroller.hadjustment();
    adj.connect_value_changed(glib::clone!(
        #[weak]
        strip,
        move |_| {
            strip.imp().scrolled.set(true);
            strip.sync_arrows();
            strip.queue_draw();
        }
    ));
    adj.connect_changed(glib::clone!(
        #[weak]
        strip,
        move |_| strip.sync_arrows()
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arrow_keys_move_between_tabs_and_stop_at_ends() {
        assert_eq!(key_tab(gdk::Key::Right, 0, 6), Some(1));
        assert_eq!(key_tab(gdk::Key::Right, 5, 6), Some(5));
        assert_eq!(key_tab(gdk::Key::Left, 0, 6), Some(0));
        assert_eq!(key_tab(gdk::Key::Home, 3, 6), Some(0));
        assert_eq!(key_tab(gdk::Key::End, 3, 6), Some(5));
        assert_eq!(key_tab(gdk::Key::Up, 3, 6), None);
        assert_eq!(key_tab(gdk::Key::Right, 0, 0), None);
    }
}
