//! A dial for one numeric parameter, drawn like a control on 1960s audio
//! equipment.
//!
//! Drawing: a printed scale of 21 ticks over 270° with 0 and 10 at its
//! ends and 5 above its top (`.tick` swatch); an enamel disc (`.disc`, lighter at the
//! top) with a 1 px rim (`.rim`) and a soft shadow (`.shadow`); an ink
//! index running in from the rim; under it the formatted value in tabular
//! numerals and, only while edited, a caption ("was 560") over the value
//! and the label.
//!
//! The saved value never leaves the scale: while edited, a dotted ghost
//! index marks it and an arc in the action colour runs from it to the
//! current value, ending in a dot; another dot follows the label. The
//! "was" line carries an undo icon and restores the saved value when
//! clicked (or with Backspace); the accessible description says "edited".
//! The index glides to a new value on the frame clock (about 90 ms, or
//! about 200 ms while comparing) and the value rolls with it, unless
//! animations are off. The range lives in the tooltip.

use std::cell::{Cell, OnceCell, RefCell};
use std::f32::consts::PI;
use std::sync::OnceLock;

use gtk::glib::subclass::Signal;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{gdk, glib, graphene, gsk, pango};

use super::{Swatches, em, layout};
use crate::i18n::tr;

/// Where the ring starts (0° = 3 o'clock, clockwise on screen) and how far
/// it sweeps: from 7:30 round to 4:30.
pub const START_DEG: f32 = 135.0;
pub const SWEEP_DEG: f32 = 270.0;
/// Scale diameter in ems, for ordinary and primary (large) dials.
const RING_EM: f32 = 4.75;
const LARGE_RING_EM: f32 = 6.25;
/// Room above the scale for its printed "5".
const FIVE_EM: f32 = 0.7;
/// Printed scale: ticks over the sweep; every fifth is long.
const TICKS: usize = 21;
/// Index glide time constant: about 95 % of the way in 90 ms.
const GLIDE_TAU_S: f64 = 0.03;
/// While comparing, the whole page glides together: about 95 % in 200 ms.
const COMPARE_TAU_S: f64 = 0.066;
/// Pixels of vertical drag for the full range (×10 with Shift).
const DRAG_PX: f64 = 200.0;

/// Position of `v` within `min..=max`, 0..1.
pub fn fraction(v: u16, min: u16, max: u16) -> f32 {
    if max <= min {
        return 0.0;
    }
    f32::from(v.clamp(min, max) - min) / f32::from(max - min)
}

/// Screen angle (radians, clockwise from 3 o'clock) of a position 0..1
/// on the scale.
pub fn angle(f: f32) -> f32 {
    (START_DEG + SWEEP_DEG * f) * PI / 180.0
}

/// New unrounded drag value after the pointer moved (`dx`, `dy`) px.
/// Up and right both raise the value, so a dial follows whichever way the
/// hand moves, like a hardware knob pulled with one finger.
pub fn drag_value(acc: f64, (dx, dy): (f64, f64), fine: bool, min: u16, max: u16) -> f64 {
    let px = if fine { DRAG_PX * 10.0 } else { DRAG_PX };
    let range = f64::from(max.saturating_sub(min));
    (acc + (dx - dy) / px * range).clamp(f64::from(min), f64::from(max))
}

/// Default coarse step: a sixteenth of the range (64 for 0–1023).
pub fn default_coarse(min: u16, max: u16) -> u16 {
    max.saturating_sub(min).div_ceil(16).max(1)
}

/// The value a key press asks for, or `None` if the key isn't ours.
pub fn key_target(
    key: gdk::Key,
    shift: bool,
    v: u16,
    (min, max): (u16, u16),
    (fine, coarse): (u16, u16),
) -> Option<u16> {
    let step = i32::from(if shift { fine.saturating_mul(10) } else { fine });
    let delta = match key {
        gdk::Key::Up | gdk::Key::Right | gdk::Key::KP_Up | gdk::Key::KP_Right => step,
        gdk::Key::Down | gdk::Key::Left | gdk::Key::KP_Down | gdk::Key::KP_Left => -step,
        gdk::Key::Page_Up | gdk::Key::KP_Page_Up => i32::from(coarse),
        gdk::Key::Page_Down | gdk::Key::KP_Page_Down => -i32::from(coarse),
        gdk::Key::Home | gdk::Key::KP_Home => return Some(min),
        gdk::Key::End | gdk::Key::KP_End => return Some(max),
        _ => return None,
    };
    Some((i32::from(v) + delta).clamp(i32::from(min), i32::from(max)) as u16)
}

// Swatch indices (see `data/widgets.css`).
const TICK: usize = 0;
const DISC: usize = 1;
const RIM: usize = 2;
const MUTED: usize = 3;
const ACT: usize = 4;
const SHADOW: usize = 5;

type Formatter = Box<dyn Fn(u16) -> String>;
type Parser = Box<dyn Fn(&str) -> Option<u16>>;

mod imp {
    use super::*;

    pub struct Dial {
        pub(super) label: RefCell<String>,
        pub(super) range: Cell<(u16, u16)>,
        pub(super) value: Cell<u16>,
        pub(super) original: Cell<u16>,
        pub(super) large: Cell<bool>,
        /// A program value: it keeps room for its "was" line. A setting
        /// (Generate's macros) is never edited and keeps none.
        pub(super) revertible: Cell<bool>,
        /// (fine, coarse); coarse 0 means "derive from the range".
        pub(super) steps: Cell<(u16, u16)>,
        pub(super) formatter: RefCell<Option<Formatter>>,
        /// Reads typed text (the core's `parse_in`), names as well as numbers.
        pub(super) parser: RefCell<Option<Parser>>,
        pub(super) drag_acc: Cell<f64>,
        pub(super) drag_last: Cell<(f64, f64)>,
        pub(super) scroll_acc: Cell<f64>,
        pub(super) swatches: OnceCell<Swatches>,
        /// The index's drawn position (0..1), gliding towards the value.
        pub(super) shown: Cell<f32>,
        /// Frame time of the last glide step, while a glide runs.
        pub(super) glide: Cell<Option<i64>>,
        /// The slower glide of a compare.
        pub(super) slow: Cell<bool>,
        /// Where the "was" line is drawn while edited: clicking it reverts.
        pub(super) caption_at: Cell<Option<graphene::Rect>>,
        /// Where the value is drawn; a double-click there types a value.
        pub(super) value_at: Cell<Option<graphene::Rect>>,
        /// The field a value is typed into, over the value, while typing.
        pub(super) entry: OnceCell<gtk::Entry>,
    }

    impl Default for Dial {
        fn default() -> Self {
            Self {
                label: RefCell::default(),
                range: Cell::new((0, 1023)),
                value: Cell::new(0),
                original: Cell::new(0),
                large: Cell::new(false),
                revertible: Cell::new(true),
                steps: Cell::new((1, 0)),
                formatter: RefCell::default(),
                parser: RefCell::default(),
                drag_acc: Cell::new(0.0),
                drag_last: Cell::new((0.0, 0.0)),
                scroll_acc: Cell::new(0.0),
                swatches: OnceCell::new(),
                shown: Cell::new(0.0),
                glide: Cell::new(None),
                slow: Cell::new(false),
                caption_at: Cell::new(None),
                value_at: Cell::new(None),
                entry: OnceCell::new(),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Dial {
        const NAME: &'static str = "OscylDial";
        type Type = super::Dial;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.set_css_name("dial");
            klass.set_accessible_role(gtk::AccessibleRole::Slider);
        }
    }

    impl ObjectImpl for Dial {
        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![
                    Signal::builder("value-changed")
                        .param_types([u32::static_type()])
                        .build(),
                ]
            })
        }

        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            obj.set_focusable(true);
            obj.set_cursor_from_name(Some("ns-resize"));
            let _ = self.swatches.set(Swatches::new(
                &*obj,
                &["tick", "disc", "rim", "muted", "act", "shadow"],
            ));
            install_controllers(&obj);
            let _ = self.entry.set(value_entry(&obj));
        }

        fn dispose(&self) {
            while let Some(child) = self.obj().first_child() {
                child.unparent();
            }
        }
    }

    impl WidgetImpl for Dial {
        fn measure(&self, orientation: gtk::Orientation, _for_size: i32) -> (i32, i32, i32, i32) {
            let obj = self.obj();
            let em = em(&*obj);
            let ring = obj.ring_em() * em;
            match orientation {
                gtk::Orientation::Horizontal => {
                    // Room for the label, so the grid keeps one rhythm
                    // whether or not a dial shows its caption.
                    let label = layout(&*obj, &self.label.borrow(), 1.0, pango::Weight::Medium);
                    let natural = (label.pixel_size().0 as f32 + 1.0 * em).max(ring + 1.5 * em);
                    (ring.ceil() as i32, natural.ceil() as i32, -1, -1)
                }
                _ => {
                    // Every dial reserves a large ring's height, so labels
                    // share one line across a row of mixed sizes.
                    let h = ((LARGE_RING_EM + FIVE_EM) * em + obj.text_block(em)).ceil() as i32;
                    (h, h, -1, -1)
                }
            }
        }

        fn snapshot(&self, snapshot: &gtk::Snapshot) {
            draw(&self.obj(), snapshot);
            if let Some(e) = self
                .entry
                .get()
                .filter(|e| gtk::prelude::WidgetExt::is_visible(*e))
            {
                self.obj().snapshot_child(e, snapshot);
            }
        }

        fn size_allocate(&self, width: i32, _height: i32, _baseline: i32) {
            // The typing field sits over the value, a little wider.
            let Some(e) = self.entry.get() else {
                return;
            };
            let at = self.obj().value_rect();
            if !gtk::prelude::WidgetExt::is_visible(e) {
                return;
            }
            let (_, h, _, _) = e.measure(gtk::Orientation::Vertical, -1);
            let w = (5.5 * em(&*self.obj())).min(width as f32) as i32;
            let x = (width - w) / 2;
            let y = (at.y() + at.height() / 2.0) as i32 - h / 2;
            e.allocate(
                w,
                h,
                -1,
                Some(gsk::Transform::new().translate(&graphene::Point::new(x as f32, y as f32))),
            );
        }
    }
}

glib::wrapper! {
    /// A ring dial; see the module docs.
    pub struct Dial(ObjectSubclass<imp::Dial>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Dial {
    pub fn new(label: &str) -> Self {
        let dial: Self = glib::Object::new();
        dial.imp().label.replace(label.to_owned());
        dial.sync();
        dial
    }

    /// Rename the dial (a parameter whose meaning follows another one).
    pub fn set_label(&self, label: &str) {
        if *self.imp().label.borrow() != label {
            self.imp().label.replace(label.to_owned());
            self.sync();
            self.queue_resize();
        }
    }

    pub fn set_range(&self, min: u16, max: u16) {
        let (min, max) = (min.min(max), min.max(max));
        self.imp().range.set((min, max));
        let imp = self.imp();
        imp.value.set(imp.value.get().clamp(min, max));
        self.sync();
    }

    /// Show `value`, edited against `original` (the last saved or synced
    /// value), without emitting `value-changed`.
    pub fn show(&self, value: u16, original: u16) {
        let (min, max) = self.imp().range.get();
        self.imp().value.set(value.clamp(min, max));
        self.imp().original.set(original);
        self.sync();
    }

    /// Glide slowly (a compare moving the whole page) or briskly.
    pub fn set_slow_glide(&self, slow: bool) {
        self.imp().slow.set(slow);
    }

    #[cfg(test)]
    pub fn slow_glide(&self) -> bool {
        self.imp().slow.get()
    }

    /// A primary parameter: a larger ring.
    pub fn set_large(&self, large: bool) {
        self.imp().large.set(large);
        self.queue_resize();
    }

    fn ring_em(&self) -> f32 {
        if self.imp().large.get() {
            LARGE_RING_EM
        } else {
            RING_EM
        }
    }

    pub fn value(&self) -> u16 {
        self.imp().value.get()
    }

    pub fn set_formatter(&self, f: impl Fn(u16) -> String + 'static) {
        self.imp().formatter.replace(Some(Box::new(f)));
        self.sync();
    }

    /// Read typed text with `f` (the value it names, or `None`) instead of
    /// matching numbers: names can be typed too.
    pub fn set_parser(&self, f: impl Fn(&str) -> Option<u16> + 'static) {
        self.imp().parser.replace(Some(Box::new(f)));
    }

    /// Arrow-key step (×10 with Shift) and Page Up/Down step. A coarse
    /// step of 0 restores the default (a sixteenth of the range).
    pub fn set_step_hint(&self, fine: u16, coarse: u16) {
        self.imp().steps.set((fine.max(1), coarse));
    }

    /// Emitted when the user changes the value (never by `show`).
    pub fn connect_value_changed<F: Fn(&Self, u16) + 'static>(
        &self,
        f: F,
    ) -> glib::SignalHandlerId {
        self.connect_closure(
            "value-changed",
            false,
            glib::closure_local!(move |dial: &Self, v: u32| f(dial, v as u16)),
        )
    }

    fn format(&self, v: u16) -> String {
        match &*self.imp().formatter.borrow() {
            Some(f) => f(v),
            None => v.to_string(),
        }
    }

    /// The line under the label while edited: what it was.
    fn caption(&self) -> String {
        tr("was {value}").replace("{value}", &self.format(self.imp().original.get()))
    }

    /// The "was" line's room under the disc, in px.
    fn was_room(&self, em: f32) -> f32 {
        if self.imp().revertible.get() {
            WAS_EM * em
        } else {
            0.0
        }
    }

    fn text_block(&self, em: f32) -> f32 {
        text_block(em, self.was_room(em))
    }

    /// Never edited, keeping no room for a "was" line (a setting, not a
    /// program value).
    pub fn set_revertible(&self, revertible: bool) {
        self.imp().revertible.set(revertible);
        self.queue_resize();
    }

    fn is_edited(&self) -> bool {
        self.imp().value.get() != self.imp().original.get()
    }

    fn steps(&self) -> (u16, u16) {
        let (fine, coarse) = self.imp().steps.get();
        let (min, max) = self.imp().range.get();
        (
            fine,
            if coarse == 0 {
                default_coarse(min, max)
            } else {
                coarse
            },
        )
    }

    /// The raw value whose shown text is nearest the number typed, or
    /// `None` when the text is no number or the dial shows names rather
    /// than numbers (those are picked by turning). Out of range snaps to
    /// the nearest end; ties go to the value nearest the current one.
    pub fn parse(&self, text: &str) -> Option<u16> {
        if let Some(p) = self.imp().parser.borrow().as_ref() {
            return p(text);
        }
        let n = number(text)?;
        let (min, max) = self.imp().range.get();
        let cur = i32::from(self.value());
        let mut best: Option<(f64, u16)> = None;
        for raw in min..=max {
            let d = (number(&self.format(raw))? - n).abs();
            let closer = |b: u16| (i32::from(raw) - cur).abs() < (i32::from(b) - cur).abs();
            match best {
                Some((bd, b)) if d > bd + 1e-9 || ((d - bd).abs() <= 1e-9 && !closer(b)) => {}
                _ => best = Some((d, raw)),
            }
        }
        best.map(|(_, raw)| raw)
    }

    /// Where the value line sits: under the scale, full width, one line of
    /// the value's type.
    fn value_rect(&self) -> graphene::Rect {
        let em = em(self);
        let h = self.height() as f32;
        let slot = ((LARGE_RING_EM + FIVE_EM) * em)
            .min(h - self.text_block(em))
            .max(em);
        let line = layout(self, "0", 1.1, pango::Weight::Semibold)
            .pixel_size()
            .1 as f32;
        graphene::Rect::new(
            0.0,
            value_y(slot, em, self.was_room(em)),
            self.width() as f32,
            line,
        )
    }

    /// Whether a value can be typed (every value it shows is a number).
    pub fn typable(&self) -> bool {
        if self.imp().parser.borrow().is_some() {
            return true;
        }
        let (min, max) = self.imp().range.get();
        number(&self.format(min)).is_some() && number(&self.format(max)).is_some()
    }

    /// Open the typing field over the value, holding `start` (a key just
    /// typed) or the current value, selected.
    pub fn start_typing(&self, start: Option<&str>) {
        let Some(e) = self.imp().entry.get() else {
            return;
        };
        if !self.typable() {
            self.error_bell();
            return;
        }
        e.remove_css_class("error");
        e.set_tooltip_text(None);
        e.set_visible(true);
        self.queue_allocate();
        self.queue_draw();
        e.grab_focus();
        match start {
            Some(t) => {
                e.set_text(t);
                e.set_position(-1);
            }
            None => {
                e.set_text(&self.format(self.value()));
                e.select_region(0, -1);
            }
        }
    }

    /// Close the typing field; `commit` sets the typed value first, and an
    /// invalid one keeps the field open, marked.
    fn stop_typing(&self, commit: bool) {
        let Some(e) = self
            .imp()
            .entry
            .get()
            .filter(|e| gtk::prelude::WidgetExt::is_visible(*e))
        else {
            return;
        };
        if commit {
            match self.parse(&e.text()) {
                Some(v) => self.user_set(v),
                None => {
                    // Said, not only marked: the phone's words for it.
                    e.add_css_class("error");
                    e.set_tooltip_text(Some(&tr("No such value")));
                    e.announce(
                        &tr("No such value"),
                        gtk::AccessibleAnnouncementPriority::Medium,
                    );
                    self.error_bell();
                    return;
                }
            }
        }
        e.set_visible(false);
        self.queue_draw();
        self.grab_focus();
    }

    /// Change the value on the user's behalf and emit `value-changed`.
    fn user_set(&self, v: u16) {
        let (min, max) = self.imp().range.get();
        let v = v.clamp(min, max);
        if v == self.value() {
            return;
        }
        self.imp().value.set(v);
        self.sync();
        self.emit_by_name::<()>("value-changed", &[&u32::from(v)]);
    }

    /// Bring CSS state, accessibility and drawing in line with the value.
    fn sync(&self) {
        let imp = self.imp();
        let (min, max) = imp.range.get();
        let v = imp.value.get();
        let edited = self.is_edited();
        if edited {
            self.add_css_class("edited");
        } else {
            self.remove_css_class("edited");
        }
        let text = self.format(v);
        self.update_property(&[
            gtk::accessible::Property::Label(&imp.label.borrow()),
            gtk::accessible::Property::ValueMin(f64::from(min)),
            gtk::accessible::Property::ValueMax(f64::from(max)),
            gtk::accessible::Property::ValueNow(f64::from(v)),
            gtk::accessible::Property::ValueText(&text),
        ]);
        if edited {
            let was =
                tr("edited, was {value}").replace("{value}", &self.format(imp.original.get()));
            self.update_property(&[gtk::accessible::Property::Description(&was)]);
        } else {
            self.reset_property(gtk::AccessibleProperty::Description);
        }
        self.glide_to(fraction(v, min, max));
        self.queue_draw();
    }

    /// Move the drawn index towards `target` on the frame clock, or at once
    /// when animations are off or the dial is not on screen.
    fn glide_to(&self, target: f32) {
        let imp = self.imp();
        let animate = self.is_mapped()
            && gtk::Settings::default().is_none_or(|s| s.is_gtk_enable_animations());
        // No glide starts: a slow glide asked for now must not linger
        // into the next drag.
        if !animate || (imp.glide.get().is_none() && (imp.shown.get() - target).abs() < 0.001) {
            imp.shown.set(target);
            imp.slow.set(false);
            return;
        }
        if imp.glide.get().is_some() {
            return; // The running glide reads the new value each frame.
        }
        imp.glide.set(Some(0));
        self.add_tick_callback(|dial, clock| {
            let imp = dial.imp();
            let (min, max) = imp.range.get();
            let target = fraction(imp.value.get(), min, max);
            let now = clock.frame_time();
            let last = imp.glide.get().unwrap_or(0);
            let dt = if last == 0 {
                1.0 / 60.0
            } else {
                (now - last) as f64 / 1e6
            };
            let tau = if imp.slow.get() {
                COMPARE_TAU_S
            } else {
                GLIDE_TAU_S
            };
            let k = 1.0 - (-dt.max(0.0) / tau).exp();
            let shown = imp.shown.get();
            let next = shown + (target - shown) * k as f32;
            dial.queue_draw();
            if (target - next).abs() < 0.001 {
                imp.shown.set(target);
                imp.glide.set(None);
                imp.slow.set(false);
                glib::ControlFlow::Break
            } else {
                imp.shown.set(next);
                imp.glide.set(Some(now));
                glib::ControlFlow::Continue
            }
        });
    }
}

/// Height of the "was" line, value and label under the disc, in px.
fn text_block(em: f32, room: f32) -> f32 {
    WAS_TOP_EM * em + room + 1.45 * em + 1.3 * em
}

/// The "was" line's gap under the scale's slot, and the room it always
/// keeps (an edit never moves the value), in ems.
const WAS_TOP_EM: f32 = 0.1;
const WAS_EM: f32 = 1.2;

/// Where the value line starts: under the "was" line's room.
fn value_y(slot: f32, em: f32, room: f32) -> f32 {
    slot + WAS_TOP_EM * em + room
}

fn install_controllers(dial: &Dial) {
    fn owner(c: &impl IsA<gtk::EventController>) -> Option<Dial> {
        c.widget().and_downcast::<Dial>()
    }
    fn shift(c: &impl IsA<gtk::EventController>) -> bool {
        c.current_event_state()
            .contains(gdk::ModifierType::SHIFT_MASK)
    }

    let drag = gtk::GestureDrag::new();
    drag.connect_drag_begin(|g, _, _| {
        let Some(d) = owner(g) else { return };
        d.grab_focus();
        d.imp().drag_acc.set(f64::from(d.value()));
        d.imp().drag_last.set((0.0, 0.0));
        d.add_css_class("dragging");
        // The tooltip would cover the dial being turned: none while held.
        d.set_has_tooltip(false);
    });
    drag.connect_drag_end(|g, _, _| {
        if let Some(d) = owner(g) {
            d.remove_css_class("dragging");
            d.set_has_tooltip(true);
        }
    });
    drag.connect_cancel(|g, _| {
        if let Some(d) = owner(g) {
            d.remove_css_class("dragging");
            d.set_has_tooltip(true);
        }
    });
    drag.connect_drag_update(|g, dx, dy| {
        let Some(d) = owner(g) else { return };
        // Keep a surrounding scrolled view from panning with us.
        g.set_state(gtk::EventSequenceState::Claimed);
        let imp = d.imp();
        let (min, max) = imp.range.get();
        let (lx, ly) = imp.drag_last.get();
        let acc = drag_value(imp.drag_acc.get(), (dx - lx, dy - ly), shift(g), min, max);
        imp.drag_acc.set(acc);
        imp.drag_last.set((dx, dy));
        d.user_set(acc.round() as u16);
    });
    dial.add_controller(drag);

    let click = gtk::GestureClick::new();
    click.connect_pressed(|g, n, x, y| {
        let Some(d) = owner(g) else { return };
        d.grab_focus();
        let on_caption = d
            .imp()
            .caption_at
            .get()
            .is_some_and(|r| r.contains_point(&graphene::Point::new(x as f32, y as f32)));
        let on_value = d
            .imp()
            .value_at
            .get()
            .is_some_and(|r| r.contains_point(&graphene::Point::new(x as f32, y as f32)));
        if n == 2 && on_value && d.typable() {
            d.start_typing(None);
        } else if n == 2 || on_caption {
            d.user_set(d.imp().original.get());
        }
    });
    dial.add_controller(click);

    let scroll = gtk::EventControllerScroll::new(gtk::EventControllerScrollFlags::BOTH_AXES);
    scroll.connect_scroll(|c, dx, dy| {
        let Some(d) = owner(c) else {
            return glib::Propagation::Proceed;
        };
        // Turning by wheel hides the tooltip until the pointer leaves.
        d.set_has_tooltip(false);
        // Shift+wheel arrives as horizontal scroll on some systems.
        let mut delta = if dy != 0.0 { dy } else { dx };
        if c.unit() == gdk::ScrollUnit::Surface {
            delta /= 20.0;
        }
        let imp = d.imp();
        let acc = imp.scroll_acc.get() - delta;
        let notches = acc.trunc();
        imp.scroll_acc.set(acc - notches);
        let (fine, _) = d.steps();
        let step = if shift(c) {
            fine
        } else {
            fine.saturating_mul(10)
        };
        let target = f64::from(d.value()) + notches * f64::from(step);
        d.user_set(target.clamp(0.0, f64::from(u16::MAX)) as u16);
        glib::Propagation::Stop
    });
    dial.add_controller(scroll);

    // A wheel turn hid the tooltip; leaving the dial brings it back.
    let motion = gtk::EventControllerMotion::new();
    motion.connect_motion(|c, x, y| {
        if let Some(d) = owner(c) {
            let on_caption = d
                .imp()
                .caption_at
                .get()
                .is_some_and(|r| r.contains_point(&graphene::Point::new(x as f32, y as f32)));
            d.set_cursor_from_name(Some(if on_caption { "pointer" } else { "ns-resize" }));
        }
    });
    motion.connect_leave(|c| {
        if let Some(d) = owner(c) {
            d.set_has_tooltip(true);
        }
    });
    dial.add_controller(motion);

    let keys = gtk::EventControllerKey::new();
    keys.connect_key_pressed(|c, key, _, state| {
        let Some(d) = owner(c) else {
            return glib::Propagation::Proceed;
        };
        // Enter, or a digit, a sign or a point, starts typing a value.
        if state.difference(gdk::ModifierType::SHIFT_MASK).is_empty() {
            if matches!(key, gdk::Key::Return | gdk::Key::KP_Enter) {
                d.start_typing(None);
                return glib::Propagation::Stop;
            }
            if let Some(ch) = key
                .to_unicode()
                .filter(|c| c.is_ascii_digit() || "-+.,".contains(*c))
            {
                d.start_typing(Some(&ch.to_string()));
                return glib::Propagation::Stop;
            }
        }
        if matches!(key, gdk::Key::BackSpace) && d.is_edited() {
            d.user_set(d.imp().original.get());
            return glib::Propagation::Stop;
        }
        let shift = state.contains(gdk::ModifierType::SHIFT_MASK);
        match key_target(key, shift, d.value(), d.imp().range.get(), d.steps()) {
            Some(v) => {
                d.user_set(v);
                glib::Propagation::Stop
            }
            None => glib::Propagation::Proceed,
        }
    });
    dial.add_controller(keys);
}

fn draw(dial: &Dial, s: &gtk::Snapshot) {
    let imp = dial.imp();
    let Some(sw) = imp.swatches.get() else { return };
    let (w, h) = (dial.width() as f32, dial.height() as f32);
    let em = em(dial);
    // The scale sits centred in a large dial's slot; text goes below it.
    let slot = ((LARGE_RING_EM + FIVE_EM) * em)
        .min(h - dial.text_block(em))
        .max(em);
    // The scale's "5" sits above it: the ring centres in the room below.
    let five = layout(dial, "5", 0.6, pango::Weight::Medium);
    let (five_w, five_h) = five.pixel_size();
    let top = five_h as f32 + 1.0;
    let ring = (dial.ring_em() * em).min(w).min(slot - top);
    let (cx, cy) = ((w / 2.0).round(), (top + (slot - top) / 2.0).round());
    let r_out = ring / 2.0 - 1.0;
    let r_disc = (r_out * 0.70).round();
    let edited = dial.is_edited();
    let ink = dial.color();
    let radial = |at: f32, r0: f32, r1: f32| {
        let b = gsk::PathBuilder::new();
        b.move_to(cx + r0 * at.cos(), cy + r0 * at.sin());
        b.line_to(cx + r1 * at.cos(), cy + r1 * at.sin());
        b.to_path()
    };

    // The printed scale, inside the arc's track.
    let r_scale = r_out - 3.0;
    let tick = gsk::Stroke::new(1.0);
    tick.set_line_cap(gsk::LineCap::Butt);
    let tick_color = sw.color(TICK);
    for i in 0..TICKS {
        let at = angle(i as f32 / (TICKS - 1) as f32);
        let long = i % 5 == 0;
        let r0 = if long { r_scale * 0.80 } else { r_scale * 0.86 };
        s.append_stroke(&radial(at, r0, r_scale), &tick, &tick_color);
    }
    // 5 is printed above the scale's top, clear of the arc's track.
    s.save();
    s.translate(&graphene::Point::new(
        (cx - five_w as f32 / 2.0).round(),
        (cy - r_out - top).round(),
    ));
    s.append_layout(&five, &tick_color);
    s.restore();
    // The scale's ends carry 0 and 10, printed just below their ticks.
    for (f, text) in [(0.0, "0"), (1.0, "10")] {
        let at = angle(f);
        let l = layout(dial, text, 0.6, pango::Weight::Medium);
        let (tw, _) = l.pixel_size();
        let (x, y) = (cx + r_scale * at.cos(), cy + r_scale * at.sin());
        let x = if f == 0.0 {
            x - tw as f32 - 1.0
        } else {
            x + 1.0
        };
        s.save();
        s.translate(&graphene::Point::new(x.round(), (y + 1.0).round()));
        s.append_layout(&l, &tick_color);
        s.restore();
    }

    // The enamel disc: a soft shadow under it, lighter at the top, a rim.
    let center = graphene::Point::new(cx, cy);
    let round = gsk::RoundedRect::from_rect(
        graphene::Rect::new(cx - r_disc, cy - r_disc, 2.0 * r_disc, 2.0 * r_disc),
        r_disc,
    );
    s.append_outset_shadow(&round, &sw.color(SHADOW), 0.0, 1.5, 0.0, 4.0);
    let disc = gsk::PathBuilder::new();
    disc.add_circle(&center, r_disc);
    let disc = disc.to_path();
    let base = sw.color(DISC);
    let shade = |c: &gdk::RGBA, k: f32| {
        gdk::RGBA::new(
            (c.red() + (if k > 0.0 { 1.0 - c.red() } else { c.red() }) * k).clamp(0.0, 1.0),
            (c.green() + (if k > 0.0 { 1.0 - c.green() } else { c.green() }) * k).clamp(0.0, 1.0),
            (c.blue() + (if k > 0.0 { 1.0 - c.blue() } else { c.blue() }) * k).clamp(0.0, 1.0),
            c.alpha(),
        )
    };
    s.push_fill(&disc, gsk::FillRule::Winding);
    s.append_linear_gradient(
        &graphene::Rect::new(cx - r_disc, cy - r_disc, 2.0 * r_disc, 2.0 * r_disc),
        &graphene::Point::new(cx, cy - r_disc),
        &graphene::Point::new(cx, cy + r_disc),
        &[
            gsk::ColorStop::new(0.0, shade(&base, 0.07)),
            gsk::ColorStop::new(1.0, shade(&base, -0.10)),
        ],
    );
    s.pop();
    s.append_stroke(&disc, &gsk::Stroke::new(1.0), &sw.color(RIM));

    let (min, max) = imp.range.get();
    let a = angle(imp.shown.get());
    if edited {
        let act = sw.color(ACT);
        let saved = fraction(imp.original.get(), min, max);
        // The ghost index: where the saved value sits, dotted.
        let ghost = gsk::Stroke::new((ring * 0.03).max(2.0));
        ghost.set_line_cap(gsk::LineCap::Round);
        ghost.set_dash(&[0.1, 3.2]);
        let g = angle(saved);
        s.append_stroke(
            &radial(g, r_disc * 0.62, r_disc * 1.08),
            &ghost,
            &sw.color(MUTED),
        );
        // The arc from the saved value to the current one, on the track
        // outside the scale, ending in a dot.
        let (f0, f1) = (saved.min(imp.shown.get()), saved.max(imp.shown.get()));
        let r_arc = r_out - 0.5;
        let steps = ((f1 - f0) * 60.0).ceil().max(1.0) as usize;
        let arc = gsk::PathBuilder::new();
        for i in 0..=steps {
            let at = angle(f0 + (f1 - f0) * i as f32 / steps as f32);
            let (x, y) = (cx + r_arc * at.cos(), cy + r_arc * at.sin());
            if i == 0 {
                arc.move_to(x, y);
            } else {
                arc.line_to(x, y);
            }
        }
        let track = gsk::Stroke::new((ring * 0.028).max(2.0));
        track.set_line_cap(gsk::LineCap::Round);
        s.append_stroke(&arc.to_path(), &track, &act);
        let dot = gsk::PathBuilder::new();
        dot.add_circle(
            &graphene::Point::new(cx + r_arc * a.cos(), cy + r_arc * a.sin()),
            (ring * 0.03).max(2.5),
        );
        s.append_fill(&dot.to_path(), gsk::FillRule::Winding, &act);
    }

    // The index line, at the glided position, running in from the rim.
    let index = gsk::Stroke::new((ring * 0.035).max(2.0));
    index.set_line_cap(gsk::LineCap::Round);
    s.append_stroke(&radial(a, r_disc * 0.38, r_disc * 0.84), &index, &ink);

    let place = |s: &gtk::Snapshot, l: &pango::Layout, x: f32, y: f32, color: &gdk::RGBA| {
        s.save();
        s.translate(&graphene::Point::new(x.round(), y.round()));
        s.append_layout(l, color);
        s.restore();
    };
    let fit = |l: &pango::Layout, width: f32| {
        l.set_width((width.max(1.0) * pango::SCALE as f32) as i32);
        l.set_ellipsize(pango::EllipsizeMode::End);
    };

    // While edited, the "was" line under the disc, then the value, then
    // the label with the action dot after it (DESIGN.md "Dial"; the phone
    // draws the same order). The "was" line is also the way back: an undo
    // icon and the saved value, in the action colour; clicking it (or
    // Backspace) restores the saved value. Its room is always kept.
    let y = slot + WAS_TOP_EM * em;
    if edited {
        let act = sw.color(ACT);
        let cap = layout(dial, &dial.caption(), 0.85, pango::Weight::Medium);
        let (cw, ch) = cap.pixel_size();
        let icon = (ch as f32 * 0.8).round();
        let gap = 0.25 * em;
        fit(&cap, w - icon - gap);
        let cw = (cw as f32).min(w - icon - gap);
        let total = (icon + gap + cw).min(w);
        let x = ((w - total) / 2.0).max(0.0).round();
        let iy = (y + (ch as f32 - icon) / 2.0).round();
        let theme = gtk::IconTheme::for_display(&dial.display());
        let paintable = theme.lookup_icon(
            "edit-undo-symbolic",
            &[],
            icon as i32,
            dial.scale_factor(),
            gtk::TextDirection::None,
            gtk::IconLookupFlags::empty(),
        );
        s.save();
        s.translate(&graphene::Point::new(x, iy));
        paintable.snapshot_symbolic(s, f64::from(icon), f64::from(icon), &[act]);
        s.restore();
        place(s, &cap, x + icon + gap, y, &act);
        imp.caption_at.set(Some(graphene::Rect::new(
            x - 4.0,
            y - 2.0,
            total + 8.0,
            ch as f32 + 4.0,
        )));
    } else {
        imp.caption_at.set(None);
    }

    let mut y = value_y(slot, em, dial.was_room(em));
    // While the index glides, the value rolls with it.
    let rolling = if imp.glide.get().is_some() {
        let f = imp.shown.get();
        (f32::from(min) + f * f32::from(max - min)).round() as u16
    } else {
        imp.value.get()
    };
    let value = layout(dial, &dial.format(rolling), 1.1, pango::Weight::Semibold);
    fit(&value, w);
    let (vw, vh) = value.pixel_size();
    let typing = imp
        .entry
        .get()
        .is_some_and(gtk::prelude::WidgetExt::is_visible);
    if !typing {
        place(s, &value, (w - vw as f32) / 2.0, y, &ink);
    }
    imp.value_at.set(Some(graphene::Rect::new(
        ((w - vw as f32) / 2.0 - 0.3 * em).max(0.0),
        y,
        (vw as f32 + 0.6 * em).min(w),
        vh as f32,
    )));
    y += vh as f32 + 0.05 * em;

    let label = layout(dial, &imp.label.borrow(), 1.0, pango::Weight::Normal);
    let dot_gap = if edited { 0.45 * em + 6.0 } else { 0.0 };
    fit(&label, w - dot_gap);
    let (lw, lh) = label.pixel_size();
    let x0 = ((w - lw as f32 - dot_gap) / 2.0).max(0.0);
    place(s, &label, x0, y, &sw.color(MUTED));
    if edited {
        let dot = gsk::PathBuilder::new();
        dot.add_circle(
            &graphene::Point::new(x0 + lw as f32 + 0.45 * em + 3.0, y + lh as f32 / 2.0),
            3.0,
        );
        s.append_fill(&dot.to_path(), gsk::FillRule::Winding, &sw.color(ACT));
    }
}
/// The number at the start of `s` ("+40 c" is 40, "-2.4 dB" is -2.4, a
/// decimal comma counts), or `None` for a name or a fraction ("1/16").
pub fn number(s: &str) -> Option<f64> {
    let s = s.trim().replace('\u{2212}', "-").replace(',', ".");
    let end = s
        .char_indices()
        .find(|&(i, c)| !(c.is_ascii_digit() || c == '.' || (i == 0 && (c == '-' || c == '+'))))
        .map_or(s.len(), |(i, _)| i);
    let n: f64 = s[..end].parse().ok()?;
    let rest = &s[end..];
    (!rest.chars().any(|c| c.is_ascii_digit() || c == '/')).then_some(n)
}

/// The field a dial's value is typed into: Enter sets it, Escape or
/// leaving without a valid number puts the dial back as it was.
fn value_entry(dial: &Dial) -> gtk::Entry {
    let e = gtk::Entry::builder()
        .xalign(0.5)
        .width_chars(5)
        .max_length(12)
        .visible(false)
        .css_classes(["dial-entry", "numeric"])
        .build();
    e.update_property(&[gtk::accessible::Property::Label(&tr("Type a value"))]);
    e.set_parent(dial);
    let weak = dial.downgrade();
    e.connect_activate(move |_| {
        if let Some(d) = weak.upgrade() {
            d.stop_typing(true);
        }
    });
    let keys = gtk::EventControllerKey::new();
    let weak = dial.downgrade();
    keys.connect_key_pressed(move |_, key, _, _| {
        if key == gdk::Key::Escape
            && let Some(d) = weak.upgrade()
        {
            d.stop_typing(false);
            return glib::Propagation::Stop;
        }
        glib::Propagation::Proceed
    });
    e.add_controller(keys);
    let focus = gtk::EventControllerFocus::new();
    let weak = dial.downgrade();
    focus.connect_leave(move |_| {
        if let Some(d) = weak.upgrade() {
            // Switching to another window keeps the field open.
            if d.root()
                .and_downcast::<gtk::Window>()
                .is_some_and(|w| !w.is_active())
            {
                return;
            }
            // Clicking away keeps a valid number and drops anything else.
            let ok = d
                .imp()
                .entry
                .get()
                .and_then(|e| d.parse(&e.text()))
                .is_some();
            d.stop_typing(ok);
        }
    });
    e.add_controller(focus);
    e
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_read_from_shown_text() {
        assert_eq!(number("618"), Some(618.0));
        assert_eq!(number("+40 c"), Some(40.0));
        assert_eq!(number("-2.4 dB"), Some(-2.4));
        assert_eq!(number("\u{2212}3"), Some(-3.0));
        assert_eq!(number("12,5%"), Some(12.5));
        assert_eq!(number("1/16"), None);
        assert_eq!(number("Off"), None);
        assert_eq!(number("Duo 3"), None);
        assert_eq!(number(""), None);
    }

    /// A typed number lands on the raw value whose text is nearest it,
    /// in the dial's own units; names are never typable.
    #[test]
    fn typed_values_map_through_the_formatter() {
        crate::on_gtk_thread(|| {
            let d = Dial::new("Cutoff");
            d.set_range(0, 1023);
            assert_eq!(d.parse("618"), Some(618));
            assert_eq!(d.parse("5000"), Some(1023), "out of range snaps to the end");
            assert_eq!(d.parse("cutoff"), None);
            d.set_formatter(|r| {
                let c = i32::from(r) - 512;
                if c > 0 {
                    format!("+{c} c")
                } else {
                    format!("{c} c")
                }
            });
            assert_eq!(d.parse("+40"), Some(552));
            assert_eq!(d.parse("-12 c"), Some(500));
            assert!(d.typable());
            d.set_formatter(|r| if r == 0 { "Off".into() } else { r.to_string() });
            assert!(!d.typable());
            assert_eq!(d.parse("5"), None);
        });
    }

    #[test]
    fn angle_spans_270_degrees_from_7_30() {
        assert_eq!(fraction(0, 0, 1023), 0.0);
        assert_eq!(fraction(1023, 0, 1023), 1.0);
        assert_eq!(fraction(5000, 0, 1023), 1.0);
        assert_eq!(fraction(3, 5, 5), 0.0);
        assert!((angle(fraction(0, 0, 100)) - 135f32.to_radians()).abs() < 1e-5);
        assert!((angle(fraction(50, 0, 100)) - 270f32.to_radians()).abs() < 1e-5);
        assert!((angle(fraction(100, 0, 100)) - 405f32.to_radians()).abs() < 1e-5);
    }

    #[test]
    fn drag_maps_200px_to_full_range_and_shift_is_fine() {
        assert_eq!(drag_value(0.0, (0.0, -200.0), false, 0, 1000), 1000.0);
        assert_eq!(drag_value(0.0, (200.0, 0.0), false, 0, 1000), 1000.0);
        assert_eq!(drag_value(500.0, (0.0, 20.0), false, 0, 1000), 400.0);
        assert_eq!(drag_value(500.0, (-20.0, 0.0), false, 0, 1000), 400.0);
        assert_eq!(drag_value(500.0, (0.0, 20.0), true, 0, 1000), 490.0);
        assert_eq!(drag_value(10.0, (0.0, 400.0), false, 0, 1000), 0.0);
    }

    #[test]
    fn keys_step_fine_shift_coarse_and_ends() {
        let r = (0, 1023);
        let st = (1, default_coarse(0, 1023));
        assert_eq!(st.1, 64);
        assert_eq!(key_target(gdk::Key::Up, false, 10, r, st), Some(11));
        assert_eq!(key_target(gdk::Key::Left, false, 10, r, st), Some(9));
        assert_eq!(key_target(gdk::Key::Right, true, 10, r, st), Some(20));
        assert_eq!(key_target(gdk::Key::Down, true, 5, r, st), Some(0));
        assert_eq!(
            key_target(gdk::Key::Page_Up, false, 1000, r, st),
            Some(1023)
        );
        assert_eq!(key_target(gdk::Key::Page_Down, false, 100, r, st), Some(36));
        assert_eq!(key_target(gdk::Key::Home, false, 100, r, st), Some(0));
        assert_eq!(key_target(gdk::Key::End, false, 100, r, st), Some(1023));
        assert_eq!(key_target(gdk::Key::a, false, 100, r, st), None);
        assert_eq!(default_coarse(0, 4), 1);
    }
}
