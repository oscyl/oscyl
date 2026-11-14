//! Render the window to a PNG for design reviews (`--screenshot`).

use std::cell::Cell;
use std::path::PathBuf;
use std::rc::Rc;

use adw::prelude::*;
use gtk::{glib, graphene};

/// Present the window, wait until it has painted a settled frame, save it
/// and quit. "Settled" means two consecutive frames with the same size, so
/// the capture follows the frame clock, not a timer.
pub fn capture_and_quit(win: &impl IsA<gtk::Window>, path: PathBuf) {
    let win = win.clone().upcast::<gtk::Window>();
    // Captures must show settled states, not a transition in progress.
    if let Some(s) = gtk::Settings::default() {
        s.set_gtk_enable_animations(false);
    }
    win.present();
    let last = Rc::new(Cell::new((0, 0)));
    let done = Rc::new(Cell::new(false));
    let w = win.clone();
    win.add_tick_callback(move |_, _| {
        let size = (w.width(), w.height());
        if done.get() {
            return glib::ControlFlow::Break;
        }
        if size.0 > 0 && size == last.get() {
            done.set(true);
            save_and_quit(w.clone(), path.clone());
            w.queue_draw();
            return glib::ControlFlow::Break;
        }
        last.set(size);
        w.queue_draw();
        glib::ControlFlow::Continue
    });
}

/// Save after a frame has been painted, trying the following painted frames
/// while the window has not rendered its last layout yet (list headers take
/// a frame more to settle); bounded by frame count, never by time.
fn save_and_quit(w: gtk::Window, path: PathBuf) {
    let Some(clock) = w.frame_clock() else {
        eprintln!("screenshot failed: no frame clock");
        return;
    };
    let tries = Rc::new(Cell::new(0u32));
    let handler: Rc<Cell<Option<glib::SignalHandlerId>>> = Rc::default();
    let h = handler.clone();
    let id = clock.connect_after_paint(move |clock| {
        tries.set(tries.get() + 1);
        match save(&w, &path) {
            Err(e) if e == NOTHING && tries.get() < 240 => {
                w.queue_draw();
                return;
            }
            Ok(()) => eprintln!("saved {}", path.display()),
            Err(e) => eprintln!("screenshot failed: {e}"),
        }
        if let Some(id) = h.take() {
            clock.disconnect(id);
        }
        if let Some(app) = w.application() {
            app.quit();
        }
    });
    handler.set(Some(id));
}

const NOTHING: &str = "nothing rendered";

/// Renders at the window's own (possibly fractional) display scale, so the
/// capture shows the pixels the screen shows.
fn save(win: &gtk::Window, path: &std::path::Path) -> Result<(), String> {
    let (w, h) = (win.width(), win.height());
    let scale = win.surface().map_or(1.0, |s| s.scale()) as f32;
    let paintable = gtk::WidgetPaintable::new(Some(win));
    let snapshot = gtk::Snapshot::new();
    snapshot.scale(scale, scale);
    paintable.snapshot(&snapshot, f64::from(w), f64::from(h));
    // Open popovers are surfaces of their own: drawn over the window under
    // the widget they point from, as the compositor places them.
    for p in popovers(win.upcast_ref()) {
        let (Some(parent), pw, ph) = (p.parent(), p.width(), p.height()) else {
            continue;
        };
        let Some(b) = parent.compute_bounds(win) else {
            continue;
        };
        let x = (b.x() + b.width() / 2.0 - pw as f32 / 2.0).clamp(0.0, (w - pw).max(0) as f32);
        let y = b.y() + b.height();
        snapshot.save();
        snapshot.translate(&graphene::Point::new(x, y));
        gtk::WidgetPaintable::new(Some(&p)).snapshot(&snapshot, f64::from(pw), f64::from(ph));
        snapshot.restore();
    }
    let node = snapshot.to_node().ok_or(NOTHING)?;
    let renderer = win.renderer().ok_or("no renderer")?;
    let bounds = graphene::Rect::new(0.0, 0.0, w as f32 * scale, h as f32 * scale);
    let texture = renderer.render_texture(node, Some(&bounds));
    texture.save_to_png(path).map_err(|e| e.to_string())
}

/// Every popover showing under `w`.
fn popovers(w: &gtk::Widget) -> Vec<gtk::Popover> {
    let mut out = Vec::new();
    let mut c = w.first_child();
    while let Some(child) = c {
        match child.downcast_ref::<gtk::Popover>() {
            Some(p) if p.is_visible() => out.push(p.clone()),
            _ => out.extend(popovers(&child)),
        }
        c = child.next_sibling();
    }
    out
}
