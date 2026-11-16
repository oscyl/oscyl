//! A section's frame (DESIGN.md "Section panel"): a 1 px `rule` with the
//! panel's corners, a faint enamel sheen at its top, and the section's
//! legend engraved into the top rule: the rule breaks around it, 18 px
//! from the start edge, and a long legend ends in an ellipsis inside the
//! frame. The phone draws the same frame (`editor/Section.kt`).

use std::cell::{Cell, OnceCell};

use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::{glib, graphene, gsk};

use super::Swatches;

/// Where the legend starts, from the frame's start edge.
const INSET: f32 = 18.0;
/// The rule's break on each side of the legend.
const GAP: f32 = 6.0;
/// `radius_panel_px` in design/tokens.json.
const RADIUS: f32 = 10.0;
const RULE: usize = 0;
const INK: usize = 1;

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct SectionFrame {
        pub(super) legend: OnceCell<Option<gtk::Label>>,
        pub(super) content: OnceCell<gtk::Widget>,
        pub(super) swatches: OnceCell<Swatches>,
        /// The legend's allocated width, where the rule breaks.
        pub(super) legend_width: Cell<i32>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SectionFrame {
        const NAME: &'static str = "OscylSectionFrame";
        type Type = super::SectionFrame;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.set_css_name("sectionframe");
            klass.set_accessible_role(gtk::AccessibleRole::Group);
        }
    }

    impl ObjectImpl for SectionFrame {
        fn constructed(&self) {
            self.parent_constructed();
            let _ = self
                .swatches
                .set(Swatches::new(&*self.obj(), &["rule", "ink"]));
        }

        fn dispose(&self) {
            while let Some(child) = self.obj().first_child() {
                child.unparent();
            }
        }
    }

    impl WidgetImpl for SectionFrame {
        fn request_mode(&self) -> gtk::SizeRequestMode {
            gtk::SizeRequestMode::HeightForWidth
        }

        fn measure(&self, o: gtk::Orientation, for_size: i32) -> (i32, i32, i32, i32) {
            let Some(content) = self.content.get() else {
                return (0, 0, -1, -1);
            };
            if o == gtk::Orientation::Horizontal {
                let (cmin, cnat, _, _) = content.measure(o, for_size);
                // A legend too long for the frame ends in an ellipsis, so
                // only its natural width asks for room.
                let lnat = self.legend().map_or(0, natural);
                let room = (2.0 * INSET) as i32;
                (cmin, cnat.max(lnat + room), -1, -1)
            } else {
                let top = self.top();
                let (cmin, cnat, _, _) = content.measure(o, for_size);
                (cmin + top, cnat + top, -1, -1)
            }
        }

        fn size_allocate(&self, width: i32, height: i32, _baseline: i32) {
            let top = self.top();
            if let Some(l) = self.legend() {
                let (lw, lh) = self.legend_size(width);
                self.legend_width.set(lw);
                let x = if self.obj().direction() == gtk::TextDirection::Rtl {
                    width - INSET as i32 - lw
                } else {
                    INSET as i32
                };
                l.allocate(
                    lw,
                    lh,
                    -1,
                    Some(gsk::Transform::new().translate(&graphene::Point::new(x as f32, 0.0))),
                );
            }
            if let Some(c) = self.content.get() {
                c.allocate(
                    width,
                    (height - top).max(0),
                    -1,
                    Some(gsk::Transform::new().translate(&graphene::Point::new(0.0, top as f32))),
                );
            }
        }

        fn snapshot(&self, s: &gtk::Snapshot) {
            let obj = self.obj();
            let Some(sw) = self.swatches.get() else {
                return;
            };
            let (w, h) = (obj.width() as f32, obj.height() as f32);
            let top = self.top() as f32;
            let frame = graphene::Rect::new(0.0, top, w, (h - top).max(0.0));
            // The sheen: 3 % ink at the top, gone by 45 % of the height.
            s.push_rounded_clip(&gsk::RoundedRect::from_rect(frame, RADIUS));
            s.append_linear_gradient(
                &frame,
                &graphene::Point::new(0.0, top),
                &graphene::Point::new(0.0, h),
                &[
                    gsk::ColorStop::new(0.0, super::super::fade(&sw.color(INK), 0.03)),
                    gsk::ColorStop::new(0.45, super::super::fade(&sw.color(INK), 0.0)),
                ],
            );
            s.pop();
            // The rule, on whole pixels, broken where the legend sits.
            let (l, t, r, b) = (0.5, top.round() + 0.5, w - 0.5, h - 0.5);
            let k = RADIUS - 0.5;
            let path = gsk::PathBuilder::new();
            match self.legend().map(|_| self.legend_width.get()) {
                Some(lw) => {
                    let lw = lw as f32;
                    let (from, to) = if obj.direction() == gtk::TextDirection::Rtl {
                        (w - INSET - lw - GAP, w - INSET + GAP)
                    } else {
                        (INSET - GAP, INSET + lw + GAP)
                    };
                    path.move_to(to, t);
                    path.line_to(r - k, t);
                    path.arc_to(r, t, r, t + k);
                    path.line_to(r, b - k);
                    path.arc_to(r, b, r - k, b);
                    path.line_to(l + k, b);
                    path.arc_to(l, b, l, b - k);
                    path.line_to(l, t + k);
                    path.arc_to(l, t, l + k, t);
                    path.line_to(from, t);
                }
                None => path.add_rounded_rect(&gsk::RoundedRect::from_rect(
                    graphene::Rect::new(l, t, r - l, b - t),
                    k,
                )),
            }
            s.append_stroke(&path.to_path(), &gsk::Stroke::new(1.0), &sw.color(RULE));
            if let Some(l) = self.legend() {
                obj.snapshot_child(l, s);
            }
            if let Some(c) = self.content.get() {
                obj.snapshot_child(c, s);
            }
        }
    }

    impl SectionFrame {
        pub(super) fn legend(&self) -> Option<&gtk::Label> {
            self.legend.get().and_then(Option::as_ref)
        }

        /// The legend's size in a frame `width` wide: its natural width,
        /// short of the insets at both ends.
        pub(super) fn legend_size(&self, width: i32) -> (i32, i32) {
            let Some(l) = self.legend() else {
                return (0, 0);
            };
            let nat = natural(l);
            let lw = nat.min(width - (2.0 * INSET) as i32).max(0);
            // Ellipsized only when it must be: GTK ellipsizes a tracked
            // (letter-spaced) label early even at its natural width.
            l.set_ellipsize(if lw < nat {
                gtk::pango::EllipsizeMode::End
            } else {
                gtk::pango::EllipsizeMode::None
            });
            let (min, _, _, _) = l.measure(gtk::Orientation::Horizontal, -1);
            let lw = lw.max(min);
            let (_, lh, _, _) = l.measure(gtk::Orientation::Vertical, lw);
            (lw, lh)
        }

        /// The frame's top rule: through the middle of the legend.
        pub(super) fn top(&self) -> i32 {
            self.legend()
                .map_or(0, |l| l.measure(gtk::Orientation::Vertical, -1).1 / 2)
        }
    }
}

/// `l`'s width unellipsized.
fn natural(l: &gtk::Label) -> i32 {
    l.layout()
        .pixel_size()
        .0
        .max(l.measure(gtk::Orientation::Horizontal, -1).1)
}

glib::wrapper! {
    /// A framed section with its legend engraved into the top rule.
    pub struct SectionFrame(ObjectSubclass<imp::SectionFrame>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl SectionFrame {
    /// Frame `content`, with `legend` (printed uppercase) in its top rule.
    pub fn new(legend: Option<&str>, content: &impl IsA<gtk::Widget>) -> Self {
        let frame: Self = glib::Object::new();
        let label = legend.map(|text| {
            let l = gtk::Label::builder()
                .label(text)
                .xalign(0.0)
                .css_classes(["section-heading"])
                .accessible_role(gtk::AccessibleRole::Heading)
                .build();
            l.set_parent(&frame);
            frame.update_relation(&[gtk::accessible::Relation::LabelledBy(&[l.upcast_ref()])]);
            l
        });
        content.set_parent(&frame);
        let _ = frame.imp().legend.set(label);
        let _ = frame.imp().content.set(content.clone().upcast());
        frame
    }
}
