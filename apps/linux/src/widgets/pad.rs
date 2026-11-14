//! The Perform page's XY pad: two parameters at once, one across and one
//! up. A ruled frame with a grid printed at tenths (every fifth stronger),
//! the point drawn as a graph handle, and while edited the saved point as a
//! dotted ghost joined to it by an orange line. Held, dashed guides run to
//! the edges and a plate says both values. A drag (or the arrow keys) moves
//! the point; a double click takes both back to saved.

use std::cell::{Cell, OnceCell, RefCell};

use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::{gdk, glib, pango};

use super::{Swatches, em, layout};
use crate::callback::Slot;

const RULE: usize = 0;
const STRONG: usize = 1;
const MUTED: usize = 2;
const EDIT: usize = 3;
const KNOB: usize = 4;

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct XyPad {
        /// The point and the saved point, as fractions (0..1, up is 1).
        pub(super) now: Cell<Option<(f32, f32)>>,
        pub(super) saved: Cell<Option<(f32, f32)>>,
        /// The axes' names and the values' plate.
        pub(super) words: RefCell<(String, String, String)>,
        pub(super) held: Cell<bool>,
        pub(super) swatches: OnceCell<Swatches>,
        pub(super) on_began: Slot<()>,
        pub(super) on_moved: Slot<(f32, f32)>,
        pub(super) on_ended: Slot<()>,
        pub(super) on_reset: Slot<()>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for XyPad {
        const NAME: &'static str = "OscylXyPad";
        type Type = super::XyPad;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.set_css_name("xypad");
            klass.set_accessible_role(gtk::AccessibleRole::Group);
        }
    }

    impl ObjectImpl for XyPad {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            obj.set_focusable(true);
            obj.set_cursor_from_name(Some("crosshair"));
            let _ = self.swatches.set(Swatches::new(
                &*obj,
                &["rule", "strong", "muted", "edit", "knob"],
            ));
            install_controllers(&obj);
        }

        fn dispose(&self) {
            while let Some(child) = self.obj().first_child() {
                child.unparent();
            }
        }
    }

    impl WidgetImpl for XyPad {
        // Square: as tall as it is wide.
        fn request_mode(&self) -> gtk::SizeRequestMode {
            gtk::SizeRequestMode::HeightForWidth
        }

        fn measure(&self, orientation: gtk::Orientation, for_size: i32) -> (i32, i32, i32, i32) {
            let em = em(&*self.obj());
            let min = (16.0 * em).ceil() as i32;
            match orientation {
                gtk::Orientation::Horizontal => (min, (26.0 * em).ceil() as i32, -1, -1),
                _ => {
                    let side = if for_size > 0 { for_size } else { min };
                    (side, side, -1, -1)
                }
            }
        }

        fn snapshot(&self, snapshot: &gtk::Snapshot) {
            draw(&self.obj(), snapshot);
        }
    }
}

glib::wrapper! {
    pub struct XyPad(ObjectSubclass<imp::XyPad>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for XyPad {
    fn default() -> Self {
        glib::Object::new()
    }
}

impl XyPad {
    /// Show the point and its saved place (fractions, up is 1); `None`:
    /// no point (an axis unassigned).
    pub fn show(&self, now: Option<(f32, f32)>, saved: Option<(f32, f32)>) {
        self.imp().now.set(now);
        self.imp().saved.set(saved);
        self.queue_draw();
    }

    /// The parameter across, the one up, and both values in words.
    pub fn set_words(&self, across: &str, up: &str, values: &str) {
        *self.imp().words.borrow_mut() = (across.into(), up.into(), values.into());
        self.update_property(&[gtk::accessible::Property::Description(values)]);
        self.queue_draw();
    }

    /// A drag or a key run starts (one undo step until it ends).
    pub fn connect_began(&self, f: impl Fn(()) + 'static) {
        self.imp().on_began.connect(f);
    }
    /// The point moved to these fractions.
    pub fn connect_moved(&self, f: impl Fn((f32, f32)) + 'static) {
        self.imp().on_moved.connect(f);
    }
    pub fn connect_ended(&self, f: impl Fn(()) + 'static) {
        self.imp().on_ended.connect(f);
    }
    /// A double click: both back to saved.
    pub fn connect_reset(&self, f: impl Fn(()) + 'static) {
        self.imp().on_reset.connect(f);
    }

    /// The pad's square of grid, inside the frame's line.
    fn frame(&self) -> (f32, f32, f32, f32) {
        let (w, h) = (self.width() as f32, self.height() as f32);
        (0.5, 0.5, (w - 1.0).max(1.0), (h - 1.0).max(1.0))
    }

    fn fraction_at(&self, x: f64, y: f64) -> (f32, f32) {
        let (x0, y0, w, h) = self.frame();
        (
            ((x as f32 - x0) / w).clamp(0.0, 1.0),
            (1.0 - (y as f32 - y0) / h).clamp(0.0, 1.0),
        )
    }
}

fn install_controllers(pad: &XyPad) {
    let drag = gtk::GestureDrag::new();
    let weak = pad.downgrade();
    drag.connect_drag_begin(move |_, x, y| {
        let Some(p) = weak.upgrade() else { return };
        if !p.is_sensitive() || p.imp().now.get().is_none() {
            return;
        }
        p.grab_focus();
        p.imp().held.set(true);
        p.imp().on_began.emit(());
        p.imp().on_moved.emit(p.fraction_at(x, y));
    });
    let weak = pad.downgrade();
    drag.connect_drag_update(move |g, dx, dy| {
        let Some(p) = weak.upgrade() else { return };
        if !p.imp().held.get() {
            return;
        }
        if let Some((x, y)) = g.start_point() {
            p.imp().on_moved.emit(p.fraction_at(x + dx, y + dy));
        }
    });
    let weak = pad.downgrade();
    drag.connect_drag_end(move |_, _, _| {
        let Some(p) = weak.upgrade() else { return };
        if p.imp().held.replace(false) {
            p.imp().on_ended.emit(());
            p.queue_draw();
        }
    });
    pad.add_controller(drag);

    let click = gtk::GestureClick::new();
    let weak = pad.downgrade();
    click.connect_pressed(move |_, n, _, _| {
        if n == 2
            && let Some(p) = weak.upgrade()
            && p.is_sensitive()
        {
            p.imp().on_reset.emit(());
        }
    });
    pad.add_controller(click);

    // Arrow keys move the point a fiftieth (a tenth with Shift), each
    // press one undo step.
    let keys = gtk::EventControllerKey::new();
    let weak = pad.downgrade();
    keys.connect_key_pressed(move |_, key, _, state| {
        let Some(p) = weak.upgrade() else {
            return glib::Propagation::Proceed;
        };
        let Some((x, y)) = p.imp().now.get() else {
            return glib::Propagation::Proceed;
        };
        let step = if state.contains(gdk::ModifierType::SHIFT_MASK) {
            0.1
        } else {
            0.02
        };
        let to = match key {
            gdk::Key::Left => (x - step, y),
            gdk::Key::Right => (x + step, y),
            gdk::Key::Up => (x, y + step),
            gdk::Key::Down => (x, y - step),
            _ => return glib::Propagation::Proceed,
        };
        p.imp().on_began.emit(());
        p.imp()
            .on_moved
            .emit((to.0.clamp(0.0, 1.0), to.1.clamp(0.0, 1.0)));
        p.imp().on_ended.emit(());
        glib::Propagation::Stop
    });
    pad.add_controller(keys);
}

fn draw(pad: &XyPad, s: &gtk::Snapshot) {
    let imp = pad.imp();
    let Some(sw) = imp.swatches.get() else { return };
    let (x0, y0, w, h) = pad.frame();
    let em = em(pad);
    let cr = s.append_cairo(&gtk::graphene::Rect::new(
        0.0,
        0.0,
        pad.width() as f32,
        pad.height() as f32,
    ));
    let set = |c: &gdk::RGBA| {
        cr.set_source_rgba(
            f64::from(c.red()),
            f64::from(c.green()),
            f64::from(c.blue()),
            f64::from(c.alpha()),
        )
    };
    let (x0, y0, w, h) = (f64::from(x0), f64::from(y0), f64::from(w), f64::from(h));
    cr.set_line_width(1.0);
    // The grid at tenths, every fifth stronger, then the frame.
    for k in 1..10 {
        set(&sw.color(if k == 5 { STRONG } else { RULE }));
        let gx = (x0 + w * f64::from(k) / 10.0).round() + 0.5;
        let gy = (y0 + h * f64::from(k) / 10.0).round() + 0.5;
        cr.move_to(gx, y0);
        cr.line_to(gx, y0 + h);
        cr.move_to(x0, gy);
        cr.line_to(x0 + w, gy);
        let _ = cr.stroke();
    }
    set(&sw.color(STRONG));
    let r = 0.7 * f64::from(em);
    rounded(&cr, x0, y0, w, h, r);
    let _ = cr.stroke();

    // The axes' names in the corners they grow toward.
    let (across, up, values) = imp.words.borrow().clone();
    let word = |text: &str, at_end: bool| {
        let l = layout(pad, text, 0.846, pango::Weight::Medium);
        let (lw, lh) = l.pixel_size();
        let pad_em = 0.5 * f64::from(em);
        let (tx, ty) = if at_end {
            (
                x0 + w - f64::from(lw) - pad_em,
                y0 + h - f64::from(lh) - pad_em,
            )
        } else {
            (x0 + pad_em, y0 + pad_em)
        };
        cr.move_to(tx.round(), ty.round());
        pangocairo::functions::show_layout(&cr, &l);
    };
    set(&sw.color(MUTED));
    word(&across, true);
    word(&up, false);

    let Some((nx, ny)) = imp.now.get() else {
        return;
    };
    let at = |(fx, fy): (f32, f32)| (x0 + w * f64::from(fx), y0 + h * (1.0 - f64::from(fy)));
    let (px, py) = at((nx, ny));
    let ink = pad.color();
    if imp.held.get() {
        // Dashed guides to the edges while held.
        set(&sw.color(MUTED));
        cr.set_dash(&[3.0, 3.0], 0.0);
        cr.move_to(px.round() + 0.5, y0);
        cr.line_to(px.round() + 0.5, y0 + h);
        cr.move_to(x0, py.round() + 0.5);
        cr.line_to(x0 + w, py.round() + 0.5);
        let _ = cr.stroke();
        cr.set_dash(&[], 0.0);
    }
    let ring = 0.55 * f64::from(em);
    if let Some(saved) = imp.saved.get().filter(|s| *s != (nx, ny)) {
        let (sx, sy) = at(saved);
        set(&sw.color(EDIT));
        cr.set_line_width(1.5);
        cr.move_to(sx, sy);
        cr.line_to(px, py);
        let _ = cr.stroke();
        set(&sw.color(MUTED));
        cr.set_dash(&[2.0, 2.0], 0.0);
        cr.arc(sx, sy, ring, 0.0, std::f64::consts::TAU);
        let _ = cr.stroke();
        cr.set_dash(&[], 0.0);
    }
    // The point: a graph handle (a ground disc in an ink ring).
    set(&sw.color(KNOB));
    cr.arc(px, py, ring, 0.0, std::f64::consts::TAU);
    let _ = cr.fill_preserve();
    set(&ink);
    cr.set_line_width(if pad.has_focus() { 2.5 } else { 1.5 });
    let _ = cr.stroke();
    if imp.held.get() && !values.is_empty() {
        // Both values on a plate by the top end corner.
        let l = layout(pad, &values, 1.0, pango::Weight::Semibold);
        let (lw, lh) = l.pixel_size();
        let m = 0.5 * f64::from(em);
        let (bw, bh) = (f64::from(lw) + 2.0 * m, f64::from(lh) + m);
        let (bx, by) = (x0 + w - bw - m, y0 + m);
        set(&sw.color(KNOB));
        rounded(&cr, bx, by, bw, bh, 0.4 * f64::from(em));
        let _ = cr.fill_preserve();
        set(&sw.color(STRONG));
        cr.set_line_width(1.0);
        let _ = cr.stroke();
        set(&ink);
        cr.move_to((bx + m).round(), (by + m / 2.0).round());
        pangocairo::functions::show_layout(&cr, &l);
    }
}

fn rounded(cr: &gtk::cairo::Context, x: f64, y: f64, w: f64, h: f64, r: f64) {
    use std::f64::consts::FRAC_PI_2;
    cr.new_sub_path();
    cr.arc(x + w - r, y + r, r, -FRAC_PI_2, 0.0);
    cr.arc(x + w - r, y + h - r, r, 0.0, FRAC_PI_2);
    cr.arc(x + r, y + h - r, r, FRAC_PI_2, 2.0 * FRAC_PI_2);
    cr.arc(x + r, y + r, r, 2.0 * FRAC_PI_2, 3.0 * FRAC_PI_2);
    cr.close_path();
}
