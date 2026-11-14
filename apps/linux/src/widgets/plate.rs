//! The sliding selection plate: in a selector (a toggle group) and in the
//! stage tabs, the chosen option's plate glides from the old option to the
//! new one instead of jumping.
//!
//! [`Glide`] holds the plate's rectangle and moves it on the frame clock
//! with a critically damped libadwaita spring, which also honours the
//! system's "reduce animation" setting (the plate then moves at once). A
//! new target mid-move carries the plate's speed into the new move, so a
//! held arrow key flows from row to row instead of restarting each time. The owner
//! draws the plate in its snapshot, under its children, so a move costs
//! one rounded rectangle per frame.
//!
//! [`SegmentTrack`] wraps an `AdwToggleGroup` and draws its plate.

use std::cell::{Cell, OnceCell, RefCell};

use adw::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{glib, graphene, gsk};

use super::Swatches;

/// The plate's spring: no bounce, settling in about a quarter of a second,
/// long enough to read as motion, short enough never to lag a click.
pub(crate) fn spring() -> adw::SpringParams {
    adw::SpringParams::new(1.0, 1.0, 600.0)
}

/// How far apart two plates are, in px (their corners' travel).
fn distance(a: &graphene::Rect, b: &graphene::Rect) -> f64 {
    let d = |x: f32, y: f32| f64::from(x - y);
    (d(a.x(), b.x()).hypot(d(a.y(), b.y())) + d(a.width(), b.width()).abs()).max(0.0)
}

/// A rectangle that glides to new targets.
#[derive(Default)]
pub struct Glide {
    from: Cell<Option<graphene::Rect>>,
    to: Cell<Option<graphene::Rect>>,
    progress: Cell<f64>,
    animation: RefCell<Option<adw::SpringAnimation>>,
}

fn mix(a: &graphene::Rect, b: &graphene::Rect, t: f32) -> graphene::Rect {
    let l = |x: f32, y: f32| x + (y - x) * t;
    graphene::Rect::new(
        l(a.x(), b.x()),
        l(a.y(), b.y()),
        l(a.width(), b.width()),
        l(a.height(), b.height()),
    )
}

impl Glide {
    /// Where the plate is now.
    pub fn current(&self) -> Option<graphene::Rect> {
        match (self.from.get(), self.to.get()) {
            (Some(a), Some(b)) => Some(mix(&a, &b, self.progress.get() as f32)),
            (None, b) => b,
            (a, None) => a,
        }
    }

    /// Move to `target`, gliding from where the plate is unless `snap`
    /// (first placement, resizes, scrolling) or the target is unchanged.
    pub fn retarget(&self, owner: &impl IsA<gtk::Widget>, target: graphene::Rect, snap: bool) {
        if self.to.get().is_some_and(|t| t == target) {
            return;
        }
        let start = if snap { None } else { self.current() };
        // The plate's speed now, in px per second, carried into the move.
        let speed = match (
            self.animation.borrow_mut().take(),
            self.from.get(),
            self.to.get(),
        ) {
            (Some(a), Some(f), Some(t)) if a.state() == adw::AnimationState::Playing => {
                a.pause();
                a.velocity() * distance(&f, &t)
            }
            (Some(a), ..) => {
                a.pause();
                0.0
            }
            _ => 0.0,
        };
        self.from.set(start.or(Some(target)));
        self.to.set(Some(target));
        let Some(start) = start else {
            self.progress.set(1.0);
            owner.as_ref().queue_draw();
            return;
        };
        self.progress.set(0.0);
        let span = distance(&start, &target);
        let weak = owner.downgrade();
        let redraw = adw::CallbackAnimationTarget::new(move |_| {
            if let Some(w) = weak.upgrade() {
                w.queue_draw();
            }
        });
        let a = adw::SpringAnimation::builder()
            .widget(owner.as_ref())
            .value_from(0.0)
            .value_to(1.0)
            .spring_params(&spring())
            .initial_velocity(if span > 0.5 { speed / span } else { 0.0 })
            .clamp(true)
            .target(&redraw)
            .build();
        a.play();
        self.animation.replace(Some(a));
    }

    /// Forget the plate; the next target is placed at once.
    pub fn clear(&self) {
        if let Some(a) = self.animation.borrow_mut().take() {
            a.pause();
        }
        self.from.set(None);
        self.to.set(None);
    }

    /// Read the animation's value; call at the start of the owner's
    /// snapshot.
    pub fn tick(&self) {
        if let Some(a) = self.animation.borrow().as_ref()
            && a.state() == adw::AnimationState::Playing
        {
            self.progress.set(a.value());
        } else if self.to.get().is_some() {
            self.progress.set(1.0);
        }
    }
}

/// Fill `rect` with `color` and `radius` corners. A lifted plate is
/// enamel: a soft shadow under it and a faint light-to-dark sweep from top
/// to bottom, so it reads as a raised part rather than a flat fill.
pub fn draw_plate(
    s: &gtk::Snapshot,
    rect: &graphene::Rect,
    radius: f32,
    color: &gtk::gdk::RGBA,
    lift: Option<&gtk::gdk::RGBA>,
) {
    let rr = gsk::RoundedRect::from_rect(*rect, radius);
    let Some(shadow) = lift else {
        s.push_rounded_clip(&rr);
        s.append_color(color, rect);
        s.pop();
        return;
    };
    // The shadow token: soft on enamel, none on AMOLED black.
    s.append_outset_shadow(&rr, shadow, 0.0, 1.0, 0.0, 3.0);
    s.push_rounded_clip(&rr);
    let mix = |k: f32| {
        let t = |c: f32| {
            if k > 0.0 {
                c + (1.0 - c) * k
            } else {
                c * (1.0 + k)
            }
        };
        gtk::gdk::RGBA::new(
            t(color.red()),
            t(color.green()),
            t(color.blue()),
            color.alpha(),
        )
    };
    s.append_linear_gradient(
        rect,
        &graphene::Point::new(rect.x(), rect.y()),
        &graphene::Point::new(rect.x(), rect.y() + rect.height()),
        &[
            gsk::ColorStop::new(0.0, mix(0.06)),
            gsk::ColorStop::new(1.0, mix(-0.05)),
        ],
    );
    s.pop();
}

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct SegmentTrack {
        pub(super) glide: Glide,
        pub(super) group: RefCell<Option<adw::ToggleGroup>>,
        pub(super) swatches: OnceCell<Swatches>,
        pub(super) width: Cell<i32>,
        /// The saved option's index while it is not the chosen one.
        pub(super) saved: Cell<Option<u32>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SegmentTrack {
        const NAME: &'static str = "OscylSegmentTrack";
        type Type = super::SegmentTrack;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.set_css_name("segmenttrack");
        }
    }

    impl ObjectImpl for SegmentTrack {
        fn constructed(&self) {
            self.parent_constructed();
            let _ = self
                .swatches
                .set(Swatches::new(&*self.obj(), &["plate", "act", "shadow"]));
        }

        fn dispose(&self) {
            while let Some(child) = self.obj().first_child() {
                child.unparent();
            }
        }
    }

    impl WidgetImpl for SegmentTrack {
        fn measure(&self, o: gtk::Orientation, for_size: i32) -> (i32, i32, i32, i32) {
            match &*self.group.borrow() {
                Some(g) => g.measure(o, for_size),
                None => (0, 0, -1, -1),
            }
        }

        fn size_allocate(&self, width: i32, height: i32, baseline: i32) {
            if let Some(g) = &*self.group.borrow() {
                g.allocate(width, height, baseline, None);
            }
            // A new size moves the plate with the layout, without a glide.
            let resized = self.width.replace(width) != width;
            self.obj().place(resized);
        }

        fn snapshot(&self, s: &gtk::Snapshot) {
            // A selector shown after its value changed (another effect
            // type, another program) places its plate here, at once.
            self.obj().place(self.glide.current().is_none());
            self.glide.tick();
            if let (Some(rect), Some(sw)) = (self.glide.current(), self.swatches.get()) {
                draw_plate(s, &rect, 4.0, &sw.color(0), Some(&sw.color(2)));
            }
            self.parent_snapshot(s);
            // The saved option, while another is chosen: a short bar in the
            // action colour under it, outside the frame.
            if let (Some(i), Some(sw)) = (self.saved.get(), self.swatches.get())
                && let Some(t) = self.obj().toggle(i)
                && let Some(b) = t.compute_bounds(&*self.obj())
            {
                let w = 12.0f32.min(b.width());
                let notch = graphene::Rect::new(
                    b.x() + (b.width() - w) / 2.0,
                    self.obj().height() as f32 + 3.0,
                    w,
                    3.0,
                );
                s.push_rounded_clip(&gsk::RoundedRect::from_rect(notch, 1.5));
                s.append_color(&sw.color(1), &notch);
                s.pop();
            }
        }
    }
}

glib::wrapper! {
    /// A toggle group whose chosen option's plate slides between options.
    pub struct SegmentTrack(ObjectSubclass<imp::SegmentTrack>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl SegmentTrack {
    pub fn new(group: &adw::ToggleGroup) -> Self {
        let track: Self = glib::Object::new();
        group.set_parent(&track);
        group.add_css_class("tracked");
        track.imp().group.replace(Some(group.clone()));
        let weak = track.downgrade();
        group.connect_active_notify(move |_| {
            if let Some(t) = weak.upgrade() {
                t.place(false);
            }
        });
        track
    }

    /// Mark option `saved` as the saved one (drawn while another is
    /// chosen), or none.
    pub fn set_saved(&self, saved: Option<u32>) {
        if self.imp().saved.replace(saved) != saved {
            self.queue_draw();
        }
    }

    /// The group's `i`th toggle widget (separators excluded).
    fn toggle(&self, i: u32) -> Option<gtk::Widget> {
        let group = self.imp().group.borrow().clone()?;
        std::iter::successors(group.first_child(), |c| c.next_sibling())
            .filter(|c| c.css_name() == "toggle")
            .nth(i as usize)
    }

    /// Aim the plate at the active toggle's box.
    fn place(&self, snap: bool) {
        let Some(group) = self.imp().group.borrow().clone() else {
            return;
        };
        let toggle = self.toggle(group.active());
        // Not laid out yet: the first allocation places the plate.
        let toggle = toggle.filter(|t| t.width() > 0);
        if let Some(rect) = toggle.and_then(|t| t.compute_bounds(self)) {
            let first = self.imp().glide.current().is_none();
            self.imp().glide.retarget(self, rect, snap || first);
        }
    }
}

mod plated_imp {
    use super::*;

    #[derive(Default)]
    pub struct Plated {
        pub(super) glide: Glide,
        pub(super) child: OnceCell<gtk::Widget>,
        pub(super) selected: OnceCell<Box<dyn Fn() -> Option<gtk::Widget>>>,
        pub(super) radius: Cell<f32>,
        pub(super) swatches: OnceCell<Swatches>,
        pub(super) width: Cell<i32>,
        /// The next placement follows the content at once (it scrolled).
        pub(super) snap_next: Cell<bool>,
        /// Not drawn for now (a row is being dragged); it keeps following
        /// the selection, so it glides from there when shown again.
        pub(super) hidden: Cell<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Plated {
        const NAME: &'static str = "OscylPlated";
        type Type = super::Plated;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.set_css_name("plated");
        }
    }

    impl ObjectImpl for Plated {
        fn dispose(&self) {
            while let Some(child) = self.obj().first_child() {
                child.unparent();
            }
        }
    }

    impl WidgetImpl for Plated {
        fn measure(&self, o: gtk::Orientation, for_size: i32) -> (i32, i32, i32, i32) {
            match self.child.get() {
                Some(c) => {
                    let (min, nat, _, _) = c.measure(o, for_size);
                    (min, nat, -1, -1)
                }
                None => (0, 0, -1, -1),
            }
        }

        fn request_mode(&self) -> gtk::SizeRequestMode {
            self.child
                .get()
                .map_or(gtk::SizeRequestMode::ConstantSize, |c| c.request_mode())
        }

        fn size_allocate(&self, width: i32, height: i32, baseline: i32) {
            if let Some(c) = self.child.get() {
                c.allocate(width, height, baseline, None);
            }
            let resized = self.width.replace(width) != width;
            self.obj().place(resized);
        }

        fn snapshot(&self, s: &gtk::Snapshot) {
            let obj = self.obj();
            let shown = obj.place(self.glide.current().is_none() || self.snap_next.take());
            self.glide.tick();
            if let (true, false, Some(rect), Some(sw)) = (
                shown,
                self.hidden.get(),
                self.glide.current(),
                self.swatches.get(),
            ) {
                let (w, h) = (obj.width() as f32, obj.height() as f32);
                s.push_clip(&graphene::Rect::new(0.0, 0.0, w, h));
                let lift = obj.has_css_class("lifted");
                let shadow = sw.color(1);
                draw_plate(
                    s,
                    &rect,
                    self.radius.get(),
                    &sw.color(0),
                    lift.then_some(&shadow),
                );
                s.pop();
            }
            self.parent_snapshot(s);
        }
    }
}

glib::wrapper! {
    /// A container of choices (a grid, a list) whose selected choice is the
    /// plate, sliding from the old choice to the new one.
    pub struct Plated(ObjectSubclass<plated_imp::Plated>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Plated {
    /// Wrap `child`; `selected` names the widget the plate sits under now,
    /// if any is shown. Call [`Plated::place`] when the selection changes.
    /// The plate's colour is the CSS `color` of `plated > .<swatch>`
    /// (`plate` or `raised`), so it never depends on where the widget sits.
    pub fn new(
        child: &impl IsA<gtk::Widget>,
        radius: f32,
        swatch: &str,
        selected: impl Fn() -> Option<gtk::Widget> + 'static,
    ) -> Self {
        let this: Self = glib::Object::new();
        let imp = this.imp();
        let _ = imp.swatches.set(Swatches::new(&this, &[swatch, "shadow"]));
        child.set_parent(&this);
        let _ = imp.child.set(child.clone().upcast());
        let _ = imp.selected.set(Box::new(selected));
        imp.radius.set(radius);
        this
    }

    /// A wrapping grid whose selected child is the plate.
    pub fn flow(flow: &gtk::FlowBox) -> Self {
        let f = flow.downgrade();
        let this = Self::new(flow, 6.0, "plate", move || {
            f.upgrade()?
                .selected_children()
                .into_iter()
                .next()
                .map(|c| c.upcast())
        });
        this.add_css_class("lifted");
        let weak = this.downgrade();
        flow.connect_selected_children_changed(move |_| {
            if let Some(t) = weak.upgrade() {
                t.place(false);
            }
        });
        this
    }

    /// Follow `adjustment`: scrolling moves the plate with the content,
    /// never gliding.
    pub fn follow_scroll(&self, adjustment: &gtk::Adjustment) {
        let weak = self.downgrade();
        adjustment.connect_value_changed(move |_| {
            if let Some(t) = weak.upgrade() {
                t.imp().snap_next.set(true);
                t.queue_draw();
            }
        });
    }

    /// Stop drawing the plate for now, or draw it again.
    pub fn set_hidden(&self, hidden: bool) {
        self.imp().hidden.set(hidden);
        self.queue_draw();
    }

    /// Put the plate under the selection at once (a new set of children).
    pub fn snap(&self) {
        self.place(true);
    }

    /// Aim the plate at the selected widget; false when none is shown.
    pub fn place(&self, snap: bool) -> bool {
        let imp = self.imp();
        let target = imp
            .selected
            .get()
            .and_then(|f| f())
            // Not laid out yet: the first allocation places the plate.
            .filter(|w| w.width() > 0 && w.is_drawable())
            .and_then(|w| w.compute_bounds(self));
        match target {
            Some(rect) => {
                let first = imp.glide.current().is_none();
                imp.glide.retarget(self, rect, snap || first);
                true
            }
            None => {
                // Nothing to sit under (scrolled away, unselected): the
                // plate comes back at once where the selection reappears.
                imp.glide.clear();
                self.queue_draw();
                false
            }
        }
    }
}
