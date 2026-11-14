//! A chart of one driver graph (`oscyl_driver::Graph`), with the saved shape drawn
//! faint (dashed) behind it while a stage is edited.
//!
//! Two uses:
//! - **Signal** graphs sit side by side in the chart band. Each draws a
//!   lead-in from its left edge (except the first) and a lead-out to its
//!   right edge at mid height, so neighbouring graphs join into one pen
//!   line; mix bars are joined top to top.
//! - **Handle** graphs (the selected stage's large graph) carry draggable
//!   points: drag with pointer or touch, or focus the graph and use the
//!   arrow keys (Shift for bigger steps); Tab moves between handles. A
//!   value label follows the handle while it moves. Moves are reported in
//!   data coordinates through `connect_handle_moved`; the end of a drag
//!   (or focus leaving after key moves) through `connect_handle_released`.
//!
//! Accessibility: the graph is an image whose description names the
//! current handle and its value. The editor's dials, which carry every
//! value a handle moves, remain the full accessible path.
//!
//! Colours: the graph's own CSS `color` is the pen; swatches give the
//! faint, rule, muted, edit and knob colours.

use std::cell::{Cell, OnceCell, RefCell};

use adw::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{gdk, glib, graphene, gsk, pango};
use oscyl_driver::{Graph as Shape, Note};

use super::{Swatches, em, fade, layout};
use crate::callback::Slot;
use crate::i18n::tr;

/// Echo taps, a reverb tail and a modulation sweep (see
/// [`oscyl_driver::Graph::Effects`]).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Effects {
    pub taps: Vec<(f32, f32)>,
    pub tail: Option<(f32, f32)>,
    pub sweep: Option<(f32, f32)>,
}

/// What a graph shows: a driver's [`oscyl_driver::Graph`], or nothing.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum GraphData {
    /// Samples −1..1.
    Wave(Vec<f32>),
    /// Response points, the cutoff's x (0..1) and the y of unity gain.
    Response(Vec<(f32, f32)>, f32, f32),
    /// Envelope corners.
    Envelope([(f32, f32); 5]),
    /// Source levels, one bar each.
    Levels(Vec<f32>),
    /// The dry hit and echoes, reverb tail, mod sweep.
    Effects(Effects),
    /// Each voice's notes on a piano roll.
    Voices(Vec<Note>),
    #[default]
    Empty,
}

impl From<Shape> for GraphData {
    fn from(g: Shape) -> GraphData {
        match g {
            Shape::Wave(w) => GraphData::Wave(w),
            Shape::Response {
                points,
                cutoff,
                unity,
            } => GraphData::Response(points, cutoff, unity),
            Shape::Envelope(e) => GraphData::Envelope(e),
            Shape::Levels(l) => GraphData::Levels(l),
            Shape::Effects { taps, tail, sweep } => {
                GraphData::Effects(Effects { taps, tail, sweep })
            }
            Shape::Voices(v) => GraphData::Voices(v),
        }
    }
}

impl GraphData {
    /// The shape `t` of the way from `self` to `to` (0..1), when the two
    /// are the same kind with the same number of points; `None` otherwise
    /// (the graph then shows `to` at once).
    pub fn lerp(&self, to: &GraphData, t: f32) -> Option<GraphData> {
        let l = |a: f32, b: f32| a + (b - a) * t;
        let pt = |a: (f32, f32), b: (f32, f32)| (l(a.0, b.0), l(a.1, b.1));
        Some(match (self, to) {
            (GraphData::Wave(a), GraphData::Wave(b)) if a.len() == b.len() => {
                GraphData::Wave(a.iter().zip(b).map(|(x, y)| l(*x, *y)).collect())
            }
            (GraphData::Response(a, ca, ua), GraphData::Response(b, cb, ub))
                if a.len() == b.len() =>
            {
                GraphData::Response(
                    a.iter().zip(b).map(|(x, y)| pt(*x, *y)).collect(),
                    l(*ca, *cb),
                    l(*ua, *ub),
                )
            }
            (GraphData::Envelope(a), GraphData::Envelope(b)) => {
                GraphData::Envelope(std::array::from_fn(|i| pt(a[i], b[i])))
            }
            (GraphData::Levels(a), GraphData::Levels(b)) if a.len() == b.len() => {
                GraphData::Levels(a.iter().zip(b).map(|(x, y)| l(*x, *y)).collect())
            }
            _ => return None,
        })
    }
}

/// How long a shape glides to a new one (a compare moving the page).
const GLIDE_MS: u32 = 220;

/// One draggable point, as the editor describes it.
#[derive(Debug, Clone, PartialEq)]
pub struct HandlePoint {
    /// Data position (x 0..1 left to right, y 0..1 bottom to top).
    pub at: (f32, f32),
    /// Whether dragging across / up changes anything.
    pub axes: (bool, bool),
    /// Accessible name, e.g. "Cutoff and resonance".
    pub name: String,
    /// The value label shown while moving, e.g. "618 · 255".
    pub value: String,
}

/// Maps data (x 0..1 left→right, y 0..1 bottom→top) into a pixel box.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Frame {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Frame {
    pub fn map(&self, dx: f32, dy: f32) -> (f32, f32) {
        (self.x + dx * self.w, self.y + (1.0 - dy) * self.h)
    }

    /// The inverse of `map`.
    pub fn unmap(&self, px: f32, py: f32) -> (f32, f32) {
        (
            (px - self.x) / self.w.max(1.0),
            1.0 - (py - self.y) / self.h.max(1.0),
        )
    }
}

/// Wave samples as (x, y) data points, x 0..1 and y 0..1.
pub fn wave_points(samples: &[f32]) -> Vec<(f32, f32)> {
    let last = samples.len().saturating_sub(1).max(1) as f32;
    samples
        .iter()
        .enumerate()
        .map(|(i, v)| (i as f32 / last, (v.clamp(-1.0, 1.0) + 1.0) / 2.0))
        .collect()
}

/// The index of the handle within `reach` px of `(px, py)`, nearest first.
pub fn hit(points: &[(f32, f32)], (px, py): (f32, f32), reach: f32) -> Option<usize> {
    points
        .iter()
        .enumerate()
        .map(|(i, &(x, y))| (i, (x - px).hypot(y - py)))
        .filter(|&(_, d)| d <= reach)
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(i, _)| i)
}

/// Where an arrow key moves a handle: along its axes, `step` in data
/// units. A one-axis handle takes all four arrows on its axis.
pub fn key_move(
    key: gdk::Key,
    (x, y): (f32, f32),
    (ax, ay): (bool, bool),
    step: f32,
) -> Option<(f32, f32)> {
    let (dx, dy) = match key {
        gdk::Key::Left | gdk::Key::KP_Left => (-step, 0.0),
        gdk::Key::Right | gdk::Key::KP_Right => (step, 0.0),
        gdk::Key::Down | gdk::Key::KP_Down => (0.0, -step),
        gdk::Key::Up | gdk::Key::KP_Up => (0.0, step),
        _ => return None,
    };
    let (dx, dy) = match (ax, ay) {
        (true, true) => (dx, dy),
        (true, false) => (dx + dy, 0.0),
        (false, true) => (0.0, dx + dy),
        (false, false) => return None,
    };
    Some((x + dx, y + dy))
}

// Swatch indices (see `data/widgets.css`).
const FAINT: usize = 0;
const RULE: usize = 1;
const MUTED: usize = 2;
const EDIT: usize = 3;
const KNOB: usize = 4;

/// Handle hit radius: at least 12 px (a 24 px target), more at large text.
fn reach(em: f32) -> f32 {
    (0.9 * em).max(12.0)
}

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct Graph {
        pub(super) current: RefCell<GraphData>,
        pub(super) previous: RefCell<Option<GraphData>>,
        /// A compact graph (a stage tab's picture): no scale, no hairlines.
        pub(super) compact: Cell<bool>,
        /// Minimum width in em (0: the default 4 em).
        pub(super) min_width_em: Cell<f32>,
        pub(super) handles: RefCell<Vec<HandlePoint>>,
        /// The handle keys move (and the focus ring marks).
        pub(super) active: Cell<usize>,
        /// The handle being dragged, and the pointer's offset from it.
        pub(super) dragging: Cell<Option<(usize, f32, f32)>>,
        pub(super) drag_start: Cell<(f64, f64)>,
        /// A key move is pending its release (sent when focus leaves).
        pub(super) keyed: Cell<bool>,
        pub(super) description: RefCell<String>,
        pub(super) swatches: OnceCell<Swatches>,
        pub(super) on_moved: Slot<(usize, (f32, f32))>,
        pub(super) on_released: Slot<()>,
        /// The shape a glide starts from, while it runs.
        pub(super) glide_from: RefCell<Option<GraphData>>,
        pub(super) glide: OnceCell<adw::TimedAnimation>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Graph {
        const NAME: &'static str = "OscylGraph";
        type Type = super::Graph;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.set_css_name("graph");
            klass.set_accessible_role(gtk::AccessibleRole::Img);
        }
    }

    impl ObjectImpl for Graph {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            obj.set_hexpand(true);
            let _ = self.swatches.set(Swatches::new(
                &*obj,
                &["faint", "rule", "muted", "edit", "knob"],
            ));
        }

        fn dispose(&self) {
            while let Some(child) = self.obj().first_child() {
                child.unparent();
            }
        }
    }

    impl WidgetImpl for Graph {
        fn measure(&self, orientation: gtk::Orientation, _for_size: i32) -> (i32, i32, i32, i32) {
            let em = em(&*self.obj());
            let wide = match self.min_width_em.get() {
                w if w > 0.0 => w,
                _ => 4.0,
            };
            let min = match orientation {
                gtk::Orientation::Horizontal => wide * em,
                _ => 2.5 * em,
            };
            (min.ceil() as i32, min.ceil() as i32, -1, -1)
        }

        fn snapshot(&self, snapshot: &gtk::Snapshot) {
            draw(&self.obj(), snapshot);
        }
    }
}

glib::wrapper! {
    /// A chart; see the module docs.
    pub struct Graph(ObjectSubclass<imp::Graph>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for Graph {
    fn default() -> Self {
        Self::new()
    }
}

impl Graph {
    pub fn new() -> Self {
        glib::Object::new()
    }

    /// A graph with draggable handles (see the module docs).
    pub fn with_handles() -> Self {
        let g = Self::new();
        install_controllers(&g);
        g
    }

    /// Draw as a compact picture (a stage tab): the shape alone, inset.
    pub fn set_compact(&self) {
        self.imp().compact.set(true);
        self.queue_draw();
    }

    /// A narrower minimum width, in em (a row of small pictures).
    pub fn set_min_width_em(&self, em: f32) {
        self.imp().min_width_em.set(em);
        self.queue_resize();
    }

    /// Like [`Graph::set_data`], but the shape glides from the one shown
    /// to the new one (220 ms, at once when animations are off).
    pub fn glide_data(&self, current: GraphData, previous: Option<GraphData>) {
        let imp = self.imp();
        let anim = imp.glide.get_or_init(|| {
            let weak = self.downgrade();
            let target = adw::CallbackAnimationTarget::new(move |_| {
                if let Some(g) = weak.upgrade() {
                    g.queue_draw();
                }
            });
            adw::TimedAnimation::builder()
                .widget(self)
                .value_from(0.0)
                .value_to(1.0)
                .duration(GLIDE_MS)
                .easing(adw::Easing::EaseOutCubic)
                .target(&target)
                .build()
        });
        // Turning back mid-glide (Compare released early): carry on from
        // the mix on screen, so the shape never pops.
        let playing = anim.state() == adw::AnimationState::Playing;
        let back = playing && imp.glide_from.borrow().as_ref() == Some(&current);
        let (from, start) = if back {
            (imp.current.borrow().clone(), 1.0 - anim.value())
        } else {
            (self.shown_data(), 0.0)
        };
        anim.pause();
        anim.set_value_from(start);
        anim.set_duration(((1.0 - start) * f64::from(GLIDE_MS)).round().max(1.0) as u32);
        imp.glide_from.replace(Some(from));
        self.set_data(current, previous);
        anim.play();
    }

    /// Mid-glide between shapes that cannot be interpolated (effects,
    /// voices): the old shape and how far the crossfade has come.
    fn fading_from(&self) -> Option<(GraphData, f32)> {
        let imp = self.imp();
        let a = imp.glide.get()?;
        if a.state() != adw::AnimationState::Playing {
            return None;
        }
        let from = imp.glide_from.borrow();
        let from = from.as_ref()?;
        let current = imp.current.borrow();
        from.lerp(&current, 0.0)
            .is_none()
            .then(|| (from.clone(), a.value() as f32))
    }

    /// The shape drawn now: mid-glide, between the two.
    fn shown_data(&self) -> GraphData {
        let imp = self.imp();
        let current = imp.current.borrow();
        match (imp.glide.get(), &*imp.glide_from.borrow()) {
            (Some(a), Some(from)) if a.state() == adw::AnimationState::Playing => from
                .lerp(&current, a.value() as f32)
                .unwrap_or_else(|| current.clone()),
            _ => current.clone(),
        }
    }

    /// Show `current`, with `previous` (the saved shape) faint behind it.
    pub fn set_data(&self, current: GraphData, previous: Option<GraphData>) {
        self.imp().current.replace(current);
        self.imp().previous.replace(previous);
        self.queue_draw();
    }

    /// Replace the handles (positions and labels follow every edit).
    pub fn set_handles(&self, handles: Vec<HandlePoint>) {
        let imp = self.imp();
        if handles.len() != imp.handles.borrow().len() {
            imp.active.set(0);
            imp.dragging.set(None);
        }
        self.set_focusable(!handles.is_empty());
        imp.handles.replace(handles);
        self.describe();
        self.queue_draw();
    }

    /// What the graph shows, in words (its accessible name).
    pub fn set_description(&self, text: &str) {
        self.imp().description.replace(text.to_owned());
        self.describe();
    }

    /// Called with a handle's index and its new data position.
    pub fn connect_handle_moved(&self, f: impl Fn((usize, (f32, f32))) + 'static) {
        self.imp().on_moved.connect(f);
    }

    /// Called when a drag ends, or focus leaves after key moves.
    pub fn connect_handle_released(&self, f: impl Fn(()) + 'static) {
        self.imp().on_released.connect(f);
    }

    fn describe(&self) {
        let imp = self.imp();
        let text = imp.description.borrow().clone();
        let handles = imp.handles.borrow();
        let detail = match handles.get(imp.active.get()) {
            Some(h) => format!("{text}. {}: {}", h.name, h.value),
            None => text.clone(),
        };
        self.update_property(&[
            gtk::accessible::Property::Label(&text),
            gtk::accessible::Property::Description(&detail),
        ]);
    }

    fn frame(&self) -> Frame {
        let (w, h) = (self.width() as f32, self.height() as f32);
        let em = em(self);
        // Handles need room to sit on the edges and stay grabbable.
        let pad = if self.imp().handles.borrow().is_empty() {
            4.0
        } else {
            reach(em)
        };
        // Room for the scale: a row under the graph, or a column of
        // percentages beside the bars.
        let (left, below) = match &*self.imp().current.borrow() {
            GraphData::Response(..)
            | GraphData::Envelope(_)
            | GraphData::Voices(_)
            | GraphData::Effects(_) => (0.0, 1.4 * em),
            GraphData::Levels(_) => (2.6 * em, 0.0),
            _ => (0.0, 0.0),
        };
        let pad_x = pad;
        Frame {
            x: pad_x + left,
            y: pad,
            w: (w - 2.0 * pad_x - left).max(1.0),
            h: (h - 2.0 * pad - below).max(1.0),
        }
    }

    fn pixel_points(&self) -> Vec<(f32, f32)> {
        let f = self.frame();
        let handles = self.imp().handles.borrow();
        handles.iter().map(|h| f.map(h.at.0, h.at.1)).collect()
    }

    fn handle_at(&self, x: f64, y: f64) -> Option<usize> {
        hit(&self.pixel_points(), (x as f32, y as f32), reach(em(self)))
    }

    fn move_handle(&self, i: usize, to: (f32, f32)) {
        let to = (to.0.clamp(0.0, 1.0), to.1.clamp(0.0, 1.0));
        self.imp().on_moved.emit((i, to));
    }
}

fn install_controllers(g: &Graph) {
    fn owner(c: &impl IsA<gtk::EventController>) -> Option<Graph> {
        c.widget().and_downcast::<Graph>()
    }

    let drag = gtk::GestureDrag::new();
    drag.connect_drag_begin(|d, x, y| {
        let Some(g) = owner(d) else { return };
        let Some(i) = g.handle_at(x, y) else {
            // Not on a handle: let scrolling or the page have it.
            d.set_state(gtk::EventSequenceState::Denied);
            return;
        };
        d.set_state(gtk::EventSequenceState::Claimed);
        g.grab_focus();
        let (hx, hy) = g.pixel_points()[i];
        let imp = g.imp();
        imp.active.set(i);
        imp.dragging.set(Some((i, hx - x as f32, hy - y as f32)));
        imp.drag_start.set((x, y));
        g.describe();
        g.queue_draw();
    });
    drag.connect_drag_update(|d, dx, dy| {
        let Some(g) = owner(d) else { return };
        let Some((i, ox, oy)) = g.imp().dragging.get() else {
            return;
        };
        let (sx, sy) = g.imp().drag_start.get();
        let f = g.frame();
        let to = f.unmap((sx + dx) as f32 + ox, (sy + dy) as f32 + oy);
        g.move_handle(i, to);
    });
    drag.connect_drag_end(|d, _, _| {
        let Some(g) = owner(d) else { return };
        if g.imp().dragging.take().is_some() {
            g.queue_draw();
            g.imp().on_released.emit(());
        }
    });
    g.add_controller(drag);

    // A grab cursor over handles says they move.
    let motion = gtk::EventControllerMotion::new();
    motion.connect_motion(|m, x, y| {
        let Some(g) = owner(m) else { return };
        let name = if g.handle_at(x, y).is_some() {
            Some("grab")
        } else {
            None
        };
        g.set_cursor_from_name(name);
    });
    g.add_controller(motion);

    let keys = gtk::EventControllerKey::new();
    keys.connect_key_pressed(|c, key, _, state| {
        let Some(g) = owner(c) else {
            return glib::Propagation::Proceed;
        };
        let imp = g.imp();
        let n = imp.handles.borrow().len();
        let i = imp.active.get();
        let step_back = state.contains(gdk::ModifierType::SHIFT_MASK);
        // Tab walks the handles, then leaves the graph.
        let tab = matches!(
            key,
            gdk::Key::Tab | gdk::Key::ISO_Left_Tab | gdk::Key::KP_Tab
        );
        if tab {
            let next = if step_back {
                i.checked_sub(1)
            } else {
                Some(i + 1).filter(|&j| j < n)
            };
            return match next {
                Some(j) => {
                    imp.active.set(j);
                    g.describe();
                    g.queue_draw();
                    glib::Propagation::Stop
                }
                None => glib::Propagation::Proceed,
            };
        }
        let Some(h) = imp.handles.borrow().get(i).cloned() else {
            return glib::Propagation::Proceed;
        };
        let step = if step_back { 0.1 } else { 0.01 };
        match key_move(key, h.at, h.axes, step) {
            Some(to) => {
                imp.keyed.set(true);
                g.move_handle(i, to);
                glib::Propagation::Stop
            }
            None => glib::Propagation::Proceed,
        }
    });
    g.add_controller(keys);

    let focus = gtk::EventControllerFocus::new();
    focus.connect_enter(|c| {
        if let Some(g) = owner(c) {
            g.queue_draw();
        }
    });
    focus.connect_leave(|c| {
        let Some(g) = owner(c) else { return };
        g.queue_draw();
        if g.imp().keyed.take() {
            g.imp().on_released.emit(());
        }
    });
    g.add_controller(focus);
}

fn polyline(f: &Frame, pts: &[(f32, f32)]) -> Option<gsk::Path> {
    let (first, rest) = pts.split_first()?;
    let b = gsk::PathBuilder::new();
    let (x, y) = f.map(first.0, first.1);
    b.move_to(x, y);
    for &(dx, dy) in rest {
        let (x, y) = f.map(dx, dy);
        b.line_to(x, y);
    }
    Some(b.to_path())
}

fn hline(f: &Frame, dy: f32) -> gsk::Path {
    let b = gsk::PathBuilder::new();
    let (_, y) = f.map(0.0, dy);
    let y = y.round() + 0.5;
    b.move_to(f.x, y);
    b.line_to(f.x + f.w, y);
    b.to_path()
}

fn vline(f: &Frame, dx: f32) -> gsk::Path {
    let b = gsk::PathBuilder::new();
    let (x, _) = f.map(dx, 0.0);
    b.move_to(x, f.y);
    b.line_to(x, f.y + f.h);
    b.to_path()
}

/// Draw one shape. `faint` draws the "previous" style: thin, dashed, no
/// fills or markers. `edited` marks the filter cutoff in edit (orange).
fn shape(
    s: &gtk::Snapshot,
    f: &Frame,
    data: &GraphData,
    color: &gdk::RGBA,
    (faint, edited): (bool, bool),
    sw: &Swatches,
) {
    let line = gsk::Stroke::new(if faint { 1.25 } else { 1.75 });
    line.set_line_join(gsk::LineJoin::Round);
    line.set_line_cap(gsk::LineCap::Round);
    if faint {
        line.set_dash(&[4.0, 4.0]);
    }
    match data {
        GraphData::Wave(samples) => {
            if let Some(p) = polyline(f, &wave_points(samples)) {
                s.append_stroke(&p, &line, color);
            }
        }
        GraphData::Response(pts, cutoff, _) => {
            let Some(p) = polyline(f, pts) else { return };
            if !faint {
                // Soft fill under the curve.
                let mut area = Vec::with_capacity(pts.len() + 2);
                area.extend(pts.first().map(|p| (p.0, 0.0)));
                area.extend_from_slice(pts);
                area.extend(pts.last().map(|p| (p.0, 0.0)));
                if let Some(fill) = polyline(f, &area) {
                    s.append_fill(&fill, gsk::FillRule::Winding, &fade(color, 0.08));
                }
            }
            s.append_stroke(&p, &line, color);
            if !faint {
                let marker = if edited {
                    sw.color(EDIT)
                } else {
                    sw.color(MUTED)
                };
                let dash = gsk::Stroke::new(1.0);
                dash.set_dash(&[2.0, 3.0]);
                s.append_stroke(&vline(f, *cutoff), &dash, &marker);
            }
        }
        GraphData::Envelope(pts) => {
            if let Some(p) = polyline(f, pts) {
                s.append_stroke(&p, &line, color);
            }
        }
        GraphData::Levels(levels) => {
            let slot = 1.0 / levels.len().max(1) as f32;
            for (i, &l) in levels.iter().enumerate() {
                let x0 = slot * i as f32 + slot * 0.2;
                let (px0, py) = f.map(x0, l.clamp(0.0, 1.0));
                let (px1, base) = f.map(x0 + slot * 0.6, 0.0);
                let rect = graphene::Rect::new(px0, py, px1 - px0, (base - py).max(1.0));
                let b = gsk::PathBuilder::new();
                b.add_rect(&rect);
                if faint {
                    s.append_stroke(&b.to_path(), &line, color);
                } else {
                    s.append_fill(&b.to_path(), gsk::FillRule::Winding, &fade(color, 0.22));
                    // The bar's top edge in full pen: the part you drag.
                    let top = gsk::PathBuilder::new();
                    top.move_to(px0, py);
                    top.line_to(px1, py);
                    s.append_stroke(&top.to_path(), &line, color);
                }
            }
        }
        GraphData::Effects(fx) => {
            // The reverb's tail first, as a decaying wash behind the hits.
            if let Some((len, level)) = fx.tail {
                let b = gsk::PathBuilder::new();
                let start = fx.taps.first().map_or(0.03, |t| t.0);
                let (x0, y0) = f.map(start, 0.0);
                b.move_to(x0, y0);
                for k in 0..=40 {
                    let t = k as f32 / 40.0;
                    let (x, y) = f.map(start + t * len, level * (-4.0 * t).exp());
                    b.line_to(x, y);
                }
                let (x1, _) = f.map(start + len, 0.0);
                b.line_to(x1, y0);
                b.close();
                let path = b.to_path();
                if faint {
                    s.append_stroke(&path, &gsk::Stroke::new(1.0), color);
                } else {
                    s.append_fill(&path, gsk::FillRule::Winding, &super::fade(color, 0.22));
                }
            }
            // The mod FX's sweep: a wave riding above the hits.
            if let Some((cycles, depth)) = fx.sweep {
                let b = gsk::PathBuilder::new();
                for k in 0..=120 {
                    let t = k as f32 / 120.0;
                    let y = 0.88 + 0.07 * depth * (t * cycles * std::f32::consts::TAU).sin();
                    let (x, y) = f.map(t, y);
                    if k == 0 {
                        b.move_to(x, y);
                    } else {
                        b.line_to(x, y);
                    }
                }
                let line = gsk::Stroke::new(1.25);
                if faint {
                    line.set_dash(&[3.0, 3.0]);
                }
                s.append_stroke(&b.to_path(), &line, color);
            }
            // The dry hit and the delay's echoes as bars.
            let bar = gsk::Stroke::new(if faint { 1.25 } else { 2.0 });
            bar.set_line_cap(gsk::LineCap::Round);
            if faint {
                bar.set_dash(&[3.0, 3.0]);
            }
            for &(t, a) in &fx.taps {
                let b = gsk::PathBuilder::new();
                let (x, y0) = f.map(t, 0.0);
                let (_, y1) = f.map(t, (a * 0.75).clamp(0.0, 1.0));
                b.move_to(x, y0);
                b.line_to(x, y1);
                s.append_stroke(&b.to_path(), &bar, color);
            }
        }
        GraphData::Voices(notes) => {
            // A piano roll: each voice's note as a block, time across and
            // pitch up. Blocks grow with the graph so the large view reads
            // as notes, not lines; the saved roll is outlined behind.
            let thick = (f.h * 0.07).clamp(2.5, 9.0);
            for n in notes {
                let (x0, y) = f.map(n.start, n.pitch);
                let (x1, _) = f.map(n.end, n.pitch);
                let rect = graphene::Rect::new(
                    x0 + 1.0,
                    y - thick / 2.0,
                    (x1 - x0 - 2.0).max(thick),
                    thick,
                );
                let b = gsk::PathBuilder::new();
                b.add_rounded_rect(&gsk::RoundedRect::from_rect(rect, 1.0));
                let path = b.to_path();
                if faint {
                    s.append_stroke(&path, &gsk::Stroke::new(1.0), color);
                } else {
                    s.append_fill(&path, gsk::FillRule::Winding, color);
                }
            }
        }
        GraphData::Empty => {}
    }
}

/// The ruled scale of a large graph, in ink-2: frequencies under the
/// filter curve, the envelope's phases under its segments, levels beside
/// the mix bars.
fn scale(g: &Graph, s: &gtk::Snapshot, f: &Frame, data: &GraphData, sw: &Swatches) {
    let em = em(g);
    let ink2 = sw.color(MUTED);
    let hair = gsk::Stroke::new(1.0);
    let text = |s: &gtk::Snapshot, t: &str, x: f32, y: f32, centre: bool| {
        let l = layout(g, t, 0.8, pango::Weight::Normal);
        let (tw, th) = l.pixel_size();
        let x = if centre {
            x - tw as f32 / 2.0
        } else {
            x - tw as f32
        };
        s.save();
        s.translate(&graphene::Point::new(
            x.round(),
            (y - th as f32 / 2.0).round(),
        ));
        s.append_layout(&l, &ink2);
        s.restore();
        tw as f32
    };
    let tick = |s: &gtk::Snapshot, x: f32| {
        let b = gsk::PathBuilder::new();
        let x = x.round() + 0.5;
        b.move_to(x, f.y + f.h);
        b.line_to(x, f.y + f.h + 0.3 * em);
        s.append_stroke(&b.to_path(), &hair, &ink2);
    };
    let row = f.y + f.h + 0.85 * em;
    match data {
        GraphData::Response(..) => {
            // x is log frequency over 20 Hz to 20 kHz.
            for (hz, label) in [(100.0f32, tr("100 Hz")), (1e3, tr("1k")), (1e4, tr("10k"))] {
                let (x, _) = f.map((hz / 20.0).log10() / 3.0, 0.0);
                tick(s, x);
                text(s, &label, x, row, true);
            }
        }
        GraphData::Envelope(pts) => {
            // Relative time: the phases, under the segments they span.
            let names = [tr("A"), tr("D"), tr("S"), tr("R")];
            for (i, name) in names.iter().enumerate() {
                let (a, _) = f.map(pts[i].0, 0.0);
                let (b, _) = f.map(pts[i + 1].0, 0.0);
                tick(s, a);
                if b - a > 1.2 * em {
                    text(s, name, (a + b) / 2.0, row, true);
                }
            }
            tick(s, f.map(pts[4].0, 0.0).0);
        }
        GraphData::Effects(fx) => {
            // The note, then time: what follows it is the effects.
            if let Some(&(t, _)) = fx.taps.first() {
                let (x, _) = f.map(t, 0.0);
                tick(s, x);
                text(s, &tr("Note"), x + 1.4 * em, row, true);
            }
            text(s, &tr("Time"), f.x + f.w, row, false);
        }
        GraphData::Voices(_) => {
            // The roll's axes, named: time runs right, pitch up.
            text(s, &tr("Time"), f.x + f.w, row, false);
            text(
                s,
                &tr("Pitch"),
                f.x + 0.4 * em
                    + layout(g, &tr("Pitch"), 0.8, pango::Weight::Normal)
                        .pixel_size()
                        .0 as f32,
                f.y + 0.5 * em,
                false,
            );
            for k in 1..8 {
                // Steps of the arpeggiator clock as faint ticks.
                tick(s, f.map(k as f32 / 8.0, 0.0).0);
            }
        }
        GraphData::Levels(_) => {
            for (v, label) in [(0.0f32, tr("0%")), (0.5, tr("50%")), (1.0, tr("100%"))] {
                let (_, y) = f.map(0.0, v);
                if v > 0.0 {
                    let dash = gsk::Stroke::new(1.0);
                    dash.set_dash(&[2.0, 4.0]);
                    s.append_stroke(&hline(f, v), &dash, &sw.color(RULE));
                }
                text(s, &label, f.x - 0.4 * em, y, false);
            }
        }
        _ => {}
    }
}

/// A handle: a small pen ring on the raised colour; the active one is
/// solid pen while dragged, with a focus ring when keyboard-focused.
fn draw_handle(
    s: &gtk::Snapshot,
    at: (f32, f32),
    r: f32,
    pen: &gdk::RGBA,
    sw: &Swatches,
    state: (bool, bool),
) {
    let (dragging, focused) = state;
    let b = gsk::PathBuilder::new();
    b.add_circle(&graphene::Point::new(at.0, at.1), r);
    let p = b.to_path();
    let fill = if dragging { *pen } else { sw.color(KNOB) };
    s.append_fill(&p, gsk::FillRule::Winding, &fill);
    s.append_stroke(&p, &gsk::Stroke::new(2.0), pen);
    if focused {
        let b = gsk::PathBuilder::new();
        b.add_circle(&graphene::Point::new(at.0, at.1), r + 4.0);
        s.append_stroke(&b.to_path(), &gsk::Stroke::new(2.0), pen);
    }
}

/// The value label beside a moving handle: raised chip, ink text.
fn draw_value(g: &Graph, s: &gtk::Snapshot, at: (f32, f32), text: &str, sw: &Swatches) {
    let em = em(g);
    let l = layout(g, text, 0.85, pango::Weight::Semibold);
    let (tw, th) = l.pixel_size();
    let (tw, th) = (tw as f32, th as f32);
    let (pw, ph) = (0.5 * em, 0.25 * em);
    let (bw, bh) = (tw + 2.0 * pw, th + 2.0 * ph);
    // Above and right of the handle, kept inside the widget.
    let x = (at.0 + 0.8 * em).min(g.width() as f32 - bw).max(0.0);
    let y = (at.1 - 0.8 * em - bh).max(0.0);
    let rect = graphene::Rect::new(x, y, bw, bh);
    let rr = gsk::RoundedRect::from_rect(rect, 0.3 * em);
    let b = gsk::PathBuilder::new();
    b.add_rounded_rect(&rr);
    let p = b.to_path();
    s.append_fill(&p, gsk::FillRule::Winding, &sw.color(KNOB));
    s.append_stroke(&p, &gsk::Stroke::new(1.0), &sw.color(RULE));
    s.save();
    s.translate(&graphene::Point::new((x + pw).round(), (y + ph).round()));
    s.append_layout(&l, &g.color());
    s.restore();
}

fn draw(g: &Graph, s: &gtk::Snapshot) {
    let imp = g.imp();
    let Some(sw) = imp.swatches.get() else { return };
    let (w, h) = (g.width() as f32, g.height() as f32);
    if w < 2.0 || h < 2.0 {
        return;
    }
    let em = em(g);
    let compact = imp.compact.get();
    let f = if compact {
        Frame {
            x: 0.6 * em,
            y: 3.0,
            w: (w - 1.2 * em).max(1.0),
            h: (h - 6.0).max(1.0),
        }
    } else {
        g.frame()
    };
    let current = g.shown_data();
    let pen = g.color();

    // Chart hairlines and scale, except on a compact picture.
    if !compact {
        let rule = sw.color(RULE);
        let hair = gsk::Stroke::new(1.0);
        let baseline = match &current {
            GraphData::Wave(_) => 0.5,
            _ => 0.0,
        };
        s.append_stroke(&hline(&f, baseline), &hair, &rule);
        if let GraphData::Response(_, _, unity) = &current {
            s.append_stroke(&hline(&f, *unity), &hair, &rule);
        }
        scale(g, s, &f, &current, sw);
        if h > 12.0 * em {
            // Large view: a dashed grid at tenths.
            let grid = gsk::Stroke::new(1.0);
            grid.set_dash(&[2.0, 4.0]);
            for i in 1..10 {
                s.append_stroke(&vline(&f, i as f32 / 10.0), &grid, &rule);
            }
        }
    }

    let previous = imp.previous.borrow();
    if let Some(prev) = &*previous {
        shape(s, &f, prev, &sw.color(FAINT), (true, false), sw);
    }
    // Shapes that cannot glide crossfade over the same 220 ms.
    match g.fading_from() {
        Some((from, t)) => {
            s.push_opacity(f64::from(1.0 - t));
            shape(s, &f, &from, &pen, (false, previous.is_some()), sw);
            s.pop();
            s.push_opacity(f64::from(t));
            shape(s, &f, &current, &pen, (false, previous.is_some()), sw);
            s.pop();
        }
        None => shape(s, &f, &current, &pen, (false, previous.is_some()), sw),
    }

    let handles = imp.handles.borrow();
    if handles.is_empty() {
        return;
    }
    let r = (0.35 * em).max(5.0);
    let focused = g.has_visible_focus();
    let dragging = imp.dragging.get().map(|d| d.0);
    for (i, h) in handles.iter().enumerate() {
        let at = f.map(h.at.0, h.at.1);
        let active = i == imp.active.get();
        draw_handle(s, at, r, &pen, sw, (dragging == Some(i), focused && active));
    }
    // The value follows a dragged or key-moved handle.
    let shown = dragging.or((focused && imp.keyed.get()).then_some(imp.active.get()));
    if let Some(h) = shown.and_then(|i| handles.get(i)) {
        draw_value(g, s, f.map(h.at.0, h.at.1), &h.value, sw);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shapes_glide_or_say_they_cannot() {
        let a = GraphData::Levels(vec![0.0, 1.0, 0.5]);
        let b = GraphData::Levels(vec![1.0, 0.0, 0.5]);
        assert_eq!(
            a.lerp(&b, 0.5),
            Some(GraphData::Levels(vec![0.5, 0.5, 0.5]))
        );
        assert_eq!(a.lerp(&b, 1.0), Some(b.clone()));
        // Different kinds or lengths crossfade instead.
        assert_eq!(
            GraphData::Wave(vec![0.0]).lerp(&GraphData::Wave(vec![0.0, 1.0]), 0.5),
            None
        );
        assert_eq!(
            GraphData::Voices(vec![]).lerp(&GraphData::Voices(vec![]), 0.5),
            None
        );
    }

    #[test]
    fn frame_maps_and_unmaps_with_y_up() {
        let f = Frame {
            x: 10.0,
            y: 20.0,
            w: 100.0,
            h: 50.0,
        };
        assert_eq!(f.map(0.0, 0.0), (10.0, 70.0));
        assert_eq!(f.map(1.0, 1.0), (110.0, 20.0));
        assert_eq!(f.map(0.5, 0.5), (60.0, 45.0));
        assert_eq!(f.unmap(60.0, 45.0), (0.5, 0.5));
    }

    #[test]
    fn wave_points_span_unit_box() {
        let p = wave_points(&[-1.0, 0.0, 1.0, 3.0]);
        assert_eq!(p[0], (0.0, 0.0));
        assert_eq!(p[2].1, 1.0);
        assert_eq!(p[3], (1.0, 1.0));
        assert!(wave_points(&[]).is_empty());
    }

    #[test]
    fn hits_nearest_handle_within_reach() {
        let pts = [(10.0, 10.0), (30.0, 10.0)];
        assert_eq!(hit(&pts, (12.0, 11.0), 12.0), Some(0));
        assert_eq!(hit(&pts, (25.0, 10.0), 12.0), Some(1));
        assert_eq!(hit(&pts, (60.0, 60.0), 12.0), None);
    }

    #[test]
    fn arrows_follow_the_handle_axes() {
        let k = |key, axes| key_move(key, (0.5, 0.5), axes, 0.1);
        assert_eq!(k(gdk::Key::Right, (true, true)), Some((0.6, 0.5)));
        assert_eq!(k(gdk::Key::Up, (true, true)), Some((0.5, 0.6)));
        // One axis: every arrow moves along it.
        assert_eq!(k(gdk::Key::Up, (false, true)), Some((0.5, 0.6)));
        assert_eq!(k(gdk::Key::Left, (false, true)), Some((0.5, 0.4)));
        assert_eq!(k(gdk::Key::Down, (true, false)), Some((0.4, 0.5)));
        assert_eq!(k(gdk::Key::a, (true, true)), None);
        assert_eq!(k(gdk::Key::Up, (false, false)), None);
    }
}
