//! The app's line drawings, in the front panel's hand (DESIGN.md "Empty
//! states"; the phone draws the same ones, `parts/EmptyState.kt`): a panel
//! at rest, a plug before its socket, a shelf with a gap, a knob on its
//! scale, a sounding trace, a hand of tiles, a sync and a unit in its slot.
//!
//! A symbolic paintable, 120 by 64: an image (a status page's icon) draws
//! it in its CSS `color` (`legend` parts) and the first two colours of its
//! `-gtk-icon-palette`, `error` for outlines (`rule-strong`) and `warning`
//! for the dotted rule of what is missing (`rule`); see `data/widgets.css`.
//! Silent: the words beside it are what is said.

use std::cell::Cell;

use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::{gdk, glib, graphene, gsk};

/// Which drawing.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Art {
    /// The panel at rest: nothing open yet.
    #[default]
    Panel,
    /// A plug a dotted step from its socket: no synth.
    Cable,
    /// A shelf with one place dotted: nothing here yet.
    Shelf,
    /// A unit over its empty slot: the synth's user units.
    Chip,
}

const W: f32 = 120.0;
const H: f32 = 64.0;

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct LineArt {
        pub(super) art: Cell<Art>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for LineArt {
        const NAME: &'static str = "OscylLineArt";
        type Type = super::LineArt;
        type Interfaces = (gdk::Paintable, gtk::SymbolicPaintable);
    }

    impl ObjectImpl for LineArt {}

    impl PaintableImpl for LineArt {
        fn intrinsic_width(&self) -> i32 {
            W as i32
        }

        fn intrinsic_height(&self) -> i32 {
            H as i32
        }

        fn flags(&self) -> gdk::PaintableFlags {
            gdk::PaintableFlags::STATIC_SIZE | gdk::PaintableFlags::STATIC_CONTENTS
        }

        fn snapshot(&self, s: &gdk::Snapshot, width: f64, height: f64) {
            let grey = gdk::RGBA::new(0.5, 0.5, 0.5, 1.0);
            self.snapshot_symbolic(s, width, height, &[grey, grey, grey]);
        }
    }

    impl SymbolicPaintableImpl for LineArt {
        fn snapshot_symbolic(
            &self,
            s: &gdk::Snapshot,
            width: f64,
            height: f64,
            colors: &[gdk::RGBA],
        ) {
            let Some(s) = s.downcast_ref::<gtk::Snapshot>() else {
                return;
            };
            let legend = colors.first().copied().unwrap_or(gdk::RGBA::BLACK);
            let strong = colors.get(1).copied().unwrap_or(legend);
            let rule = colors.get(2).copied().unwrap_or(legend);
            let k = (width as f32 / W).min(height as f32 / H);
            s.save();
            s.translate(&graphene::Point::new(
                (width as f32 - W * k) / 2.0,
                (height as f32 - H * k) / 2.0,
            ));
            s.scale(k, k);
            draw(s, self.art.get(), legend, strong, rule);
            s.restore();
        }
    }
}

glib::wrapper! {
    /// One of the app's line drawings, as a themed symbolic paintable.
    pub struct LineArt(ObjectSubclass<imp::LineArt>)
        @implements gdk::Paintable, gtk::SymbolicPaintable;
}

impl LineArt {
    pub fn new(art: Art) -> Self {
        let this: Self = glib::Object::new();
        this.imp().art.set(art);
        this
    }
}

/// Draw `art` in the 120 x 64 box (the phone's coordinates, in dp there).
fn draw(s: &gtk::Snapshot, art: Art, legend: gdk::RGBA, strong: gdk::RGBA, rule: gdk::RGBA) {
    let line = gsk::Stroke::new(1.5);
    line.set_line_cap(gsk::LineCap::Round);
    line.set_line_join(gsk::LineJoin::Round);
    let dots = line.clone();
    dots.set_dash(&[0.1, 4.0]);
    let stroke = |b: gsk::PathBuilder, color: &gdk::RGBA, how: &gsk::Stroke| {
        s.append_stroke(&b.to_path(), how, color);
    };
    let rect = |color: &gdk::RGBA, x: f32, y: f32, w: f32, h: f32, r: f32, how: &gsk::Stroke| {
        let b = gsk::PathBuilder::new();
        b.add_rounded_rect(&gsk::RoundedRect::from_rect(
            graphene::Rect::new(x, y, w, h),
            r,
        ));
        stroke(b, color, how);
    };
    let seg = |color: &gdk::RGBA, x1: f32, y1: f32, x2: f32, y2: f32, how: &gsk::Stroke| {
        let b = gsk::PathBuilder::new();
        b.move_to(x1, y1);
        b.line_to(x2, y2);
        stroke(b, color, how);
    };
    let circle = |color: &gdk::RGBA, x: f32, y: f32, r: f32| {
        let b = gsk::PathBuilder::new();
        b.add_circle(&graphene::Point::new(x, y), r);
        stroke(b, color, &line);
    };
    let knob = |x: f32, y: f32, r: f32| {
        circle(&legend, x, y, r);
        seg(&legend, x, y, x, y - r * 0.55, &line);
    };
    match art {
        // The panel at rest: its display's flat trace on a dotted rule,
        // its knobs at noon.
        Art::Panel => {
            rect(&strong, 1.0, 1.0, 118.0, 62.0, 8.0, &line);
            rect(&legend, 12.0, 14.0, 52.0, 36.0, 3.0, &line);
            seg(&rule, 19.0, 32.0, 57.0, 32.0, &dots);
            seg(&legend, 24.0, 32.0, 44.0, 32.0, &line);
            knob(82.0, 24.0, 9.0);
            knob(105.0, 24.0, 9.0);
            knob(82.0, 46.0, 5.0);
            knob(105.0, 46.0, 5.0);
        }
        // The plug on its cable, a dotted step from the synth's socket;
        // the lamp unlit.
        Art::Cable => {
            rect(&strong, 78.0, 8.0, 41.0, 48.0, 8.0, &line);
            rect(&legend, 88.0, 24.0, 11.0, 16.0, 2.0, &line);
            circle(&legend, 108.0, 19.0, 3.0);
            rect(&legend, 40.0, 24.0, 16.0, 16.0, 3.0, &line);
            seg(&legend, 56.0, 29.0, 62.0, 29.0, &line);
            seg(&legend, 56.0, 35.0, 62.0, 35.0, &line);
            let b = gsk::PathBuilder::new();
            b.move_to(40.0, 32.0);
            b.cubic_to(22.0, 32.0, 24.0, 52.0, 2.0, 52.0);
            stroke(b, &legend, &line);
            seg(&rule, 68.0, 32.0, 84.0, 32.0, &dots);
        }
        // A shelf of spines, one place dotted: what is still to come.
        Art::Shelf => {
            seg(&strong, 8.0, 57.0, 112.0, 57.0, &line);
            rect(&legend, 26.0, 22.0, 11.0, 35.0, 2.0, &line);
            rect(&legend, 40.0, 14.0, 11.0, 43.0, 2.0, &line);
            rect(&rule, 54.0, 20.0, 11.0, 37.0, 2.0, &dots);
            rect(&legend, 68.0, 28.0, 11.0, 29.0, 2.0, &line);
            rect(&legend, 82.0, 18.0, 11.0, 39.0, 2.0, &line);
        }
        // A unit over its slot.
        Art::Chip => {
            rect(&legend, 46.0, 10.0, 28.0, 28.0, 3.0, &line);
            for k in 0..3 {
                let k = k as f32;
                seg(&legend, 53.0 + 7.0 * k, 5.0, 53.0 + 7.0 * k, 10.0, &line);
                seg(&legend, 41.0, 17.0 + 7.0 * k, 46.0, 17.0 + 7.0 * k, &line);
                seg(&legend, 74.0, 17.0 + 7.0 * k, 79.0, 17.0 + 7.0 * k, &line);
            }
            rect(&rule, 40.0, 46.0, 40.0, 12.0, 3.0, &dots);
            seg(&strong, 24.0, 58.0, 96.0, 58.0, &line);
        }
    }
}
