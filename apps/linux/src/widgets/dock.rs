//! A strip that slides down from the top edge of its overlay: laid out
//! once and moved by a transform on a critically damped spring, so it
//! never relays out the page under it, can turn back mid-move with the
//! velocity it had, and takes input only where it shows.

use std::cell::{Cell, OnceCell, RefCell};

use adw::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{glib, graphene, gsk};

/// No bounce; settles in about a quarter of a second.
fn spring() -> adw::SpringParams {
    adw::SpringParams::new(1.0, 1.0, 420.0)
}

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct Dock {
        pub(super) child: OnceCell<gtk::Widget>,
        /// 0 hidden above the edge, 1 fully down.
        pub(super) shown: Cell<f64>,
        pub(super) anim: RefCell<Option<adw::SpringAnimation>>,
        /// Where the spring is heading.
        pub(super) target: Cell<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Dock {
        const NAME: &'static str = "OscylDock";
        type Type = super::Dock;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.set_css_name("dock");
        }
    }

    impl ObjectImpl for Dock {
        fn dispose(&self) {
            while let Some(child) = self.obj().first_child() {
                child.unparent();
            }
        }
    }

    impl WidgetImpl for Dock {
        fn measure(&self, o: gtk::Orientation, for_size: i32) -> (i32, i32, i32, i32) {
            match self.child.get() {
                Some(c) => {
                    let (min, nat, _, _) = c.measure(o, for_size);
                    (min, nat, -1, -1)
                }
                None => (0, 0, -1, -1),
            }
        }

        fn size_allocate(&self, width: i32, height: i32, _baseline: i32) {
            if let Some(c) = self.child.get() {
                // Hidden, the strip sits above the edge, its shadow too.
                let y = -(1.0 - self.shown.get() as f32) * (height as f32 + 24.0);
                let t = gsk::Transform::new().translate(&graphene::Point::new(0.0, y));
                c.allocate(width, height, -1, Some(t));
            }
        }

        fn contains(&self, _x: f64, y: f64) -> bool {
            let h = f64::from(self.obj().height());
            self.shown.get() > 0.0 && y <= h * self.shown.get()
        }
    }
}

glib::wrapper! {
    /// A strip sliding down from the top edge (see the module docs).
    pub struct Dock(ObjectSubclass<imp::Dock>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Dock {
    pub fn new(child: &impl IsA<gtk::Widget>) -> Self {
        let this: Self = glib::Object::new();
        this.set_valign(gtk::Align::Start);
        child.set_parent(&this);
        let _ = this.imp().child.set(child.clone().upcast());
        this.set_visible(false);
        this
    }

    pub fn is_shown(&self) -> bool {
        self.imp().target.get()
    }

    /// Slide down into view, or back up out of it.
    pub fn set_shown(&self, shown: bool) {
        let imp = self.imp();
        if imp.target.replace(shown) == shown {
            return;
        }
        if shown {
            self.set_visible(true);
        }
        let velocity = imp.anim.borrow().as_ref().map_or(0.0, |a| {
            if a.state() == adw::AnimationState::Playing {
                a.velocity()
            } else {
                0.0
            }
        });
        if let Some(old) = imp.anim.borrow_mut().take() {
            old.pause();
        }
        let weak = self.downgrade();
        let target = adw::CallbackAnimationTarget::new(move |v| {
            if let Some(d) = weak.upgrade() {
                d.imp().shown.set(v);
                d.queue_allocate();
            }
        });
        let anim = adw::SpringAnimation::builder()
            .widget(self)
            .value_from(imp.shown.get())
            .value_to(if shown { 1.0 } else { 0.0 })
            .spring_params(&spring())
            .initial_velocity(velocity)
            .clamp(true)
            .target(&target)
            .build();
        let weak = self.downgrade();
        anim.connect_done(move |_| {
            // Up out of view: out of the focus chain and the screen
            // reader's view too.
            if let Some(d) = weak.upgrade()
                && !d.imp().target.get()
            {
                d.set_visible(false);
            }
        });
        anim.play();
        imp.anim.replace(Some(anim));
    }
}
