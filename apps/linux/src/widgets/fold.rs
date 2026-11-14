//! A row's motion: [`Fold`] wraps a list row's content so it can fold
//! open and shut (its height follows, so the rows below move with it),
//! slide from where it was to where it is now, fade in, and carry a 2 px
//! ink insertion line. Every move runs on the plate's spring (critically
//! damped, about a quarter of a second, carrying its speed into a new
//! target), so rows, the plate and the line move as one; reduced motion
//! makes every move instant.

use std::cell::{Cell, OnceCell, RefCell};

use adw::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{glib, graphene};

use super::Swatches;
use super::plate::spring;

/// One value moved by the spring.
#[derive(Default)]
pub(crate) struct Sprung {
    value: Cell<f64>,
    anim: RefCell<Option<adw::SpringAnimation>>,
}

impl Sprung {
    fn new(value: f64) -> Self {
        Self {
            value: Cell::new(value),
            anim: RefCell::default(),
        }
    }

    pub(crate) fn get(&self) -> f64 {
        self.value.get()
    }

    /// Stop where it is.
    fn stop(&self) {
        if let Some(a) = self.anim.borrow_mut().take() {
            a.pause();
        }
    }

    /// Move to `to` (at once unless `animate`), then run `done`. `field`
    /// finds this value on `owner`; `layout` asks for a relayout on each
    /// step, not only a redraw.
    fn to(
        &self,
        owner: &Fold,
        field: fn(&imp::Fold) -> &Sprung,
        to: f64,
        animate: bool,
        layout: bool,
        done: Option<Box<dyn Fn()>>,
    ) {
        let velocity = self
            .anim
            .borrow_mut()
            .take()
            .filter(|a| a.state() == adw::AnimationState::Playing)
            .map_or(0.0, |a| {
                a.pause();
                a.velocity()
            });
        let redraw = move |w: &Fold| {
            if layout {
                w.queue_resize();
            } else {
                w.queue_draw();
            }
        };
        if !animate || (self.value.get() - to).abs() < 0.001 {
            self.value.set(to);
            redraw(owner);
            if let Some(done) = done {
                done();
            }
            return;
        }
        let weak = owner.downgrade();
        let target = adw::CallbackAnimationTarget::new(move |v| {
            if let Some(w) = weak.upgrade() {
                field(w.imp()).value.set(v);
                redraw(&w);
            }
        });
        let a = adw::SpringAnimation::builder()
            .widget(owner)
            .value_from(self.value.get())
            .value_to(to)
            .spring_params(&spring())
            .initial_velocity(velocity)
            .clamp(layout)
            .target(&target)
            .build();
        if let Some(done) = done {
            a.connect_done(move |_| done());
        }
        self.anim.replace(Some(a.clone()));
        a.play();
    }
}

/// Where the insertion line sits: `x` px in from the start, at `at` of
/// the height (0 top, 0.5 middle, 1 bottom).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Line {
    pub x: f32,
    pub at: f32,
}

mod imp {
    use super::*;

    pub struct Fold {
        pub(super) child: OnceCell<gtk::Widget>,
        /// How much of the height shows (1 open, 0 shut).
        pub(super) reveal: Sprung,
        /// Drawn this far down from its place, in px.
        pub(super) shift: Sprung,
        pub(super) fade: Sprung,
        pub(super) line_alpha: Sprung,
        pub(super) line: Cell<Option<Line>>,
        pub(super) swatches: OnceCell<Swatches>,
        /// Where the row was, in `anchor`'s coordinates, before a reorder:
        /// the next frame slides it from there.
        pub(super) was: RefCell<Option<(glib::WeakRef<gtk::Widget>, f32)>>,
    }

    impl Default for Fold {
        fn default() -> Self {
            Self {
                child: OnceCell::new(),
                reveal: Sprung::new(1.0),
                shift: Sprung::new(0.0),
                fade: Sprung::new(1.0),
                line_alpha: Sprung::new(0.0),
                line: Cell::new(None),
                swatches: OnceCell::new(),
                was: RefCell::new(None),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Fold {
        const NAME: &'static str = "OscylFold";
        type Type = super::Fold;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.set_css_name("fold");
        }
    }

    impl ObjectImpl for Fold {
        fn dispose(&self) {
            for s in [&self.reveal, &self.shift, &self.fade, &self.line_alpha] {
                s.stop();
            }
            while let Some(child) = self.obj().first_child() {
                child.unparent();
            }
        }
    }

    impl WidgetImpl for Fold {
        fn measure(&self, o: gtk::Orientation, for_size: i32) -> (i32, i32, i32, i32) {
            let Some(c) = self.child.get() else {
                return (0, 0, -1, -1);
            };
            let (min, nat, _, _) = c.measure(o, for_size);
            let r = self.reveal.get();
            if o == gtk::Orientation::Vertical && r < 1.0 {
                let r = r.max(0.0);
                let nat = (f64::from(nat) * r).round() as i32;
                let min = ((f64::from(min) * r).round() as i32).min(nat);
                return (min, nat, -1, -1);
            }
            (min, nat, -1, -1)
        }

        fn request_mode(&self) -> gtk::SizeRequestMode {
            gtk::SizeRequestMode::HeightForWidth
        }

        fn size_allocate(&self, width: i32, height: i32, baseline: i32) {
            if let Some(c) = self.child.get() {
                // Folding: the child keeps its full height, clipped.
                let h = if self.reveal.get() < 1.0 {
                    height.max(c.measure(gtk::Orientation::Vertical, width).1)
                } else {
                    height
                };
                c.allocate(width, h, baseline, None);
            }
        }

        fn snapshot(&self, s: &gtk::Snapshot) {
            let obj = self.obj();
            obj.flip();
            let (w, h) = (obj.width() as f32, obj.height() as f32);
            let folding = self.reveal.get() < 1.0;
            if folding {
                s.push_clip(&graphene::Rect::new(0.0, 0.0, w, h));
            }
            let fade = self.fade.get() as f32;
            if fade < 1.0 {
                s.push_opacity(f64::from(fade.max(0.0)));
            }
            s.save();
            s.translate(&graphene::Point::new(0.0, self.shift.get() as f32));
            if let Some(c) = self.child.get() {
                obj.snapshot_child(c, s);
            }
            s.restore();
            if fade < 1.0 {
                s.pop();
            }
            let alpha = self.line_alpha.get() as f32;
            if let (Some(line), Some(sw)) = (self.line.get(), self.swatches.get())
                && alpha > 0.0
            {
                let y =
                    (line.at * h - 1.0).clamp(0.0, (h - 2.0).max(0.0)) + self.shift.get() as f32;
                s.append_color(
                    &super::super::fade(&sw.color(0), alpha.min(1.0)),
                    &graphene::Rect::new(line.x, y, (w - line.x).max(0.0), 2.0),
                );
            }
            if folding {
                s.pop();
            }
        }
    }
}

glib::wrapper! {
    /// A row's content that folds, slides, fades and shows an insertion
    /// line, each on the plate's spring.
    pub struct Fold(ObjectSubclass<imp::Fold>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Fold {
    pub fn new(child: &impl IsA<gtk::Widget>) -> Self {
        let this: Self = glib::Object::new();
        let imp = this.imp();
        let _ = imp.swatches.set(Swatches::new(&this, &["ink"]));
        child.set_parent(&this);
        let _ = imp.child.set(child.clone().upcast());
        this
    }

    pub fn child(&self) -> Option<gtk::Widget> {
        self.imp().child.get().cloned()
    }

    pub fn is_open(&self) -> bool {
        self.imp().reveal.get() >= 1.0
    }

    /// Fold open or shut, then run `done`.
    pub fn set_open(&self, open: bool, animate: bool, done: impl Fn() + 'static) {
        self.imp().reveal.to(
            self,
            |i| &i.reveal,
            if open { 1.0 } else { 0.0 },
            animate,
            true,
            Some(Box::new(done)),
        );
    }

    /// Draw the content `dy` px down from its place, then glide home.
    pub fn slide_from(&self, dy: f64) {
        let imp = self.imp();
        imp.shift.stop();
        imp.shift.value.set(imp.shift.get() + dy);
        imp.shift.to(self, |i| &i.shift, 0.0, true, false, None);
    }

    /// Glide to `dy` px from its place (rows making room), or jump there.
    pub fn shift_to(&self, dy: f64, animate: bool) {
        self.imp()
            .shift
            .to(self, |i| &i.shift, dy, animate, false, None);
    }

    pub fn shift(&self) -> f64 {
        self.imp().shift.get()
    }

    /// Fade from clear to solid (a row arriving), or to `to` at once.
    pub fn fade_in(&self) {
        let imp = self.imp();
        imp.fade.stop();
        imp.fade.value.set(0.0);
        imp.fade.to(self, |i| &i.fade, 1.0, true, false, None);
    }

    pub fn fade_to(&self, to: f64, animate: bool) {
        self.imp()
            .fade
            .to(self, |i| &i.fade, to, animate, false, None);
    }

    /// Show the insertion line (fading in), or fade it out.
    pub fn set_line(&self, line: Option<Line>, animate: bool) {
        let imp = self.imp();
        if let Some(l) = line {
            imp.line.set(Some(l));
        }
        imp.line_alpha.to(
            self,
            |i| &i.line_alpha,
            if line.is_some() { 1.0 } else { 0.0 },
            animate,
            false,
            None,
        );
        self.queue_draw();
    }

    /// Remember where the row is in `anchor` before it moves; the next
    /// frame draws it there and glides it to its new place.
    pub fn remember_place(&self, anchor: &impl IsA<gtk::Widget>) {
        if let Some(p) = self.compute_point(anchor, &graphene::Point::zero()) {
            self.imp().was.replace(Some((
                anchor.as_ref().downgrade(),
                p.y() - self.shift() as f32,
            )));
            self.queue_draw();
        }
    }

    /// After a reorder, laid out in its new place: slide from the old one.
    fn flip(&self) {
        let Some((anchor, was)) = self.imp().was.take() else {
            return;
        };
        let Some(now) = anchor
            .upgrade()
            .and_then(|a| self.compute_point(&a, &graphene::Point::zero()))
        else {
            return;
        };
        let dy = f64::from(was - now.y());
        if dy.abs() < 0.5 || !adw::is_animations_enabled(self) {
            return;
        }
        // Drawn at the old place this frame; the glide starts on the next
        // turn of the main loop (an animation can't start mid-snapshot).
        let imp = self.imp();
        imp.shift.stop();
        imp.shift.value.set(dy);
        let weak = self.downgrade();
        glib::idle_add_local_once(move || {
            if let Some(f) = weak.upgrade() {
                f.imp().shift.to(&f, |i| &i.shift, 0.0, true, false, None);
            }
        });
    }
}
