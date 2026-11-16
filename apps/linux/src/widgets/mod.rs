//! Oscyl's custom widgets: ring dials, graphs with handles and the chart
//! band.
//!
//! Colours come from CSS, never from code. Each drawn widget owns a few
//! hidden "swatch" children whose CSS `color` names one palette role (see
//! `data/widgets.css`), so themes, high contrast and AMOLED restyle the
//! drawing like any stock widget. Sizes are in ems read from the widget's
//! Pango context, so text zoom scales the drawing too.

mod dial;
mod dock;
mod fold;
mod graph;
mod pad;
mod plate;
mod section_frame;
mod stage_strip;

pub use dial::Dial;
#[allow(unused_imports)] // The widgets demo example does not use it.
pub use dock::Dock;
#[allow(unused_imports)] // The widgets demo example does not use it.
pub use fold::{Fold, Line};
pub use graph::{Graph, GraphData, HandlePoint};
#[allow(unused_imports)] // The widgets demo example does not use it.
pub use pad::XyPad;
#[allow(unused_imports)] // The widgets demo example does not use it.
pub use plate::{Plated, SegmentTrack};
#[allow(unused_imports)] // The widgets demo example does not use it.
pub use section_frame::SectionFrame;
pub use stage_strip::StageStrip;

use std::cell::{Cell, OnceCell};
use std::rc::Rc;

use adw::prelude::*;
use gtk::{gdk, pango};

/// Hidden children whose CSS `color` is read at draw time.
pub(crate) struct Swatches(Vec<gtk::Widget>);

impl Swatches {
    pub fn new(parent: &impl IsA<gtk::Widget>, classes: &[&str]) -> Self {
        Self(
            classes
                .iter()
                .map(|class| {
                    let w = gtk::Box::builder()
                        .css_classes([*class])
                        .visible(false)
                        .accessible_role(gtk::AccessibleRole::Presentation)
                        .build();
                    w.set_parent(parent);
                    w.upcast()
                })
                .collect(),
        )
    }

    pub fn color(&self, i: usize) -> gdk::RGBA {
        self.0.get(i).map(|w| w.color()).unwrap_or(gdk::RGBA::BLACK)
    }
}

/// One em in pixels for `widget`'s current font (follows text zoom).
pub(crate) fn em(widget: &impl IsA<gtk::Widget>) -> f32 {
    let ctx = widget.pango_context();
    let Some(desc) = ctx.font_description() else {
        return 14.0;
    };
    let size = desc.size() as f32 / pango::SCALE as f32;
    if size <= 0.0 {
        return 14.0;
    }
    if desc.is_size_absolute() {
        return size;
    }
    let dpi = pangocairo::functions::context_get_resolution(&ctx);
    let dpi = if dpi > 0.0 { dpi as f32 } else { 96.0 };
    size * dpi / 72.0
}

/// A Pango layout for `text` in `widget`'s font, scaled by `scale`, at
/// `weight`, with tabular numerals.
pub(crate) fn layout(
    widget: &impl IsA<gtk::Widget>,
    text: &str,
    scale: f64,
    weight: pango::Weight,
) -> pango::Layout {
    let layout = widget.create_pango_layout(Some(text));
    let attrs = pango::AttrList::new();
    attrs.insert(pango::AttrFontFeatures::new("tnum"));
    attrs.insert(pango::AttrInt::new_weight(weight));
    attrs.insert(pango::AttrFloat::new_scale(scale));
    layout.set_attributes(Some(&attrs));
    layout
}

/// `color` with its alpha multiplied by `alpha`.
pub(crate) fn fade(color: &gdk::RGBA, alpha: f32) -> gdk::RGBA {
    let mut c = *color;
    c.set_alpha(color.alpha() * alpha);
    c
}

/// How long a mouse wheel notch glides.
const WHEEL_MS: u32 = 200;

/// Make a mouse wheel glide `scrolled` instead of jumping a notch at a
/// time: each notch moves the same distance GTK would (the page size to
/// the power ⅔), eased over 200 ms, and notches that come in while one
/// glides add to where it is going. Touchpads and touchscreens keep
/// GTK's own kinetic scrolling; reduced motion jumps as before.
pub fn smooth_wheel(scrolled: &gtk::ScrolledWindow) {
    glide_wheel(scrolled, false);
}

/// Like [`smooth_wheel`] for a row that only scrolls sideways: the wheel
/// (either axis) glides it left and right.
pub fn smooth_wheel_sideways(scrolled: &gtk::ScrolledWindow) {
    glide_wheel(scrolled, true);
}

fn glide_wheel(scrolled: &gtk::ScrolledWindow, sideways: bool) {
    let scroll = gtk::EventControllerScroll::new(if sideways {
        gtk::EventControllerScrollFlags::BOTH_AXES
    } else {
        gtk::EventControllerScrollFlags::VERTICAL
    });
    // Before the scrolled window's own handler, and before any scrolled
    // window inside it, so the outer page takes the wheel.
    scroll.set_propagation_phase(gtk::PropagationPhase::Capture);
    let going_to: Rc<Cell<f64>> = Rc::default();
    let anim: Rc<OnceCell<adw::TimedAnimation>> = Rc::default();
    let weak = scrolled.downgrade();
    scroll.connect_scroll(move |c, dx, dy| {
        let Some(sw) = weak.upgrade() else {
            return gtk::glib::Propagation::Proceed;
        };
        let d = if sideways && dy == 0.0 { dx } else { dy };
        if d == 0.0 {
            return gtk::glib::Propagation::Proceed;
        }
        let adj = if sideways {
            sw.hadjustment()
        } else {
            sw.vadjustment()
        };
        let page = adj.page_size();
        let end = (adj.upper() - page).max(adj.lower());
        // Touchpads keep GTK's kinetic scrolling (sideways rows take
        // their pixel deltas directly).
        if c.unit() != gdk::ScrollUnit::Wheel {
            if sideways {
                adj.set_value((adj.value() + d).clamp(adj.lower(), end));
                return gtk::glib::Propagation::Stop;
            }
            return gtk::glib::Propagation::Proceed;
        }
        // A dial under the pointer turns with the wheel; the page stays.
        if !sideways && over_dial(c, &sw) {
            return gtk::glib::Propagation::Proceed;
        }
        if end <= adj.lower() {
            return gtk::glib::Propagation::Proceed;
        }
        let a = anim.get_or_init(|| {
            adw::TimedAnimation::builder()
                .widget(&sw)
                .target(&adw::PropertyAnimationTarget::new(&adj, "value"))
                .duration(WHEEL_MS)
                .easing(adw::Easing::EaseOutCubic)
                .build()
        });
        let gliding = a.state() == adw::AnimationState::Playing;
        let from = if gliding { going_to.get() } else { adj.value() };
        let step = if sideways {
            page * 0.5
        } else {
            page.powf(2.0 / 3.0)
        };
        let to = (from + d * step).clamp(adj.lower(), end);
        going_to.set(to);
        a.pause();
        a.set_value_from(adj.value());
        a.set_value_to(to);
        a.play();
        gtk::glib::Propagation::Stop
    });
    scrolled.add_controller(scroll);
}

/// Whether the scroll event `c` is handling happened over a dial inside
/// `sw`.
fn over_dial(c: &gtk::EventControllerScroll, sw: &gtk::ScrolledWindow) -> bool {
    let (Some(event), Some(native)) = (c.current_event(), sw.native()) else {
        return false;
    };
    let Some((x, y)) = event.position() else {
        return false;
    };
    let (tx, ty) = native.surface_transform();
    let picked = native.pick(x - tx, y - ty, gtk::PickFlags::DEFAULT);
    std::iter::successors(picked, |w| w.parent())
        .take_while(|w| w != sw.upcast_ref::<gtk::Widget>())
        .any(|w| w.is::<Dial>())
}
