//! The Perform page, above the editor (its pad button, Ctrl+P): a large XY
//! pad on two parameters and four macro knobs, for playing a program
//! rather than shaping it. What the pad and knobs turn is kept per synth
//! model in the settings (`play`); the core proposes the first layout
//! (`play_defaults`). The pad moves both fields as one undo step; a factory
//! slot shows the values with the controls off.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use adw::prelude::*;
use oscyl_app::editor::{Widget, widget};
use oscyl_app::settings::PlayLayout;
use oscyl_driver::Driver;

use crate::app::App;
use crate::i18n::tr;
use crate::widgets::{Dial, XyPad};

/// The page's parts.
pub struct Perform {
    pub root: gtk::ScrolledWindow,
    name: gtk::Label,
    note: gtk::Label,
    pad: XyPad,
    across: gtk::DropDown,
    up: gtk::DropDown,
    knobs: Vec<(Dial, gtk::DropDown)>,
    /// The choosers' fields: entry k + 1 of each list is `fields[k]`
    /// (entry 0 is None).
    fields: RefCell<Vec<usize>>,
    /// Whose each of `fields` is: its section's (or stage's) title.
    whose: RefCell<Vec<String>>,
    driver: Cell<Option<&'static dyn Driver>>,
    /// Set while the page shows values, so choosers and dials stay quiet.
    updating: Cell<bool>,
}

/// A chooser over the program's dials, "None" first, searchable.
fn chooser(name: &str) -> gtk::DropDown {
    let dd = gtk::DropDown::builder()
        .enable_search(true)
        .tooltip_text(tr("Choose for {name}").replace("{name}", name))
        .build();
    dd.set_expression(Some(gtk::PropertyExpression::new(
        gtk::StringObject::static_type(),
        gtk::Expression::NONE,
        "string",
    )));
    dd.update_property(&[gtk::accessible::Property::Label(
        &tr("Choose for {name}").replace("{name}", name),
    )]);
    // The chosen parameter gives way with an ellipsis on a phone-width
    // page; the list of them keeps whole names.
    dd.set_factory(Some(&names(true)));
    dd.set_list_factory(Some(&names(false)));
    dd
}

/// A dropdown's names, ellipsized at the end or whole.
fn names(ellipsize: bool) -> gtk::SignalListItemFactory {
    let f = gtk::SignalListItemFactory::new();
    f.connect_setup(move |_, item| {
        let label = gtk::Label::builder().xalign(0.0).build();
        if ellipsize {
            label.set_ellipsize(gtk::pango::EllipsizeMode::End);
        }
        if let Some(item) = item.downcast_ref::<gtk::ListItem>() {
            item.set_child(Some(&label));
        }
    });
    f.connect_bind(|_, item| {
        let Some(item) = item.downcast_ref::<gtk::ListItem>() else {
            return;
        };
        if let (Some(l), Some(s)) = (
            item.child().and_downcast::<gtk::Label>(),
            item.item().and_downcast::<gtk::StringObject>(),
        ) {
            l.set_label(&s.string());
        }
    });
    f
}

fn legend(text: &str) -> gtk::Label {
    let l = gtk::Label::builder().label(text).xalign(0.0).build();
    l.add_css_class("section-heading");
    l
}

impl Perform {
    fn new() -> Rc<Perform> {
        let page = gtk::Box::new(gtk::Orientation::Vertical, 16);
        page.add_css_class("perform");
        let back = gtk::Button::builder()
            .icon_name("go-previous-symbolic")
            .action_name("win.perform")
            .valign(gtk::Align::Center)
            .css_classes(["flat"])
            .tooltip_text(tr("Back to the editor (Escape)"))
            .build();
        back.update_property(&[gtk::accessible::Property::Label(&tr("Back to the editor"))]);
        let name = gtk::Label::builder()
            .xalign(0.0)
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .css_classes(["program-title", "display"])
            .build();
        let head = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        head.append(&back);
        head.append(&name);
        page.append(&head);
        let note = gtk::Label::builder()
            .xalign(0.0)
            .wrap(true)
            .css_classes(["dim-label"])
            .label(tr(
                "Factory slot: read-only. Copy it to a library to change it.",
            ))
            .build();
        page.append(&note);

        // The pad, square, with its two choosers under it.
        let pad = XyPad::default();
        let across = chooser(&tr("Across"));
        let up = chooser(&tr("Up"));
        let axes = gtk::Grid::builder()
            .column_homogeneous(true)
            .column_spacing(12)
            .row_spacing(6)
            .build();
        axes.attach(&legend(&tr("Across")), 0, 0, 1, 1);
        axes.attach(&legend(&tr("Up")), 1, 0, 1, 1);
        axes.attach(&across, 0, 1, 1, 1);
        axes.attach(&up, 1, 1, 1, 1);
        let pad_col = gtk::Box::new(gtk::Orientation::Vertical, 12);
        pad_col.append(&pad);
        pad_col.append(&axes);

        // Four macro knobs, two by two, each with its chooser under it.
        let grid = gtk::Grid::builder()
            .column_spacing(24)
            .row_spacing(24)
            .column_homogeneous(true)
            .valign(gtk::Align::Start)
            .css_classes(["perform-knobs"])
            .build();
        let mut knobs = Vec::new();
        for k in 0..4 {
            let knob = tr("Knob {n}").replace("{n}", &(k + 1).to_string());
            let dial = Dial::new(&knob);
            dial.set_large(true);
            let dd = chooser(&knob);
            let cell = gtk::Box::new(gtk::Orientation::Vertical, 8);
            cell.append(&legend(&knob));
            cell.append(&dial);
            cell.append(&dd);
            grid.attach(&cell, k % 2, k / 2, 1, 1);
            knobs.push((dial, dd));
        }
        let body = adw::WrapBox::builder()
            .child_spacing(40)
            .line_spacing(32)
            .build();
        body.append(&pad_col);
        body.append(&grid);
        page.append(&body);

        let root = gtk::ScrolledWindow::builder()
            .child(&page)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .build();
        crate::widgets::smooth_wheel(&root);
        Rc::new(Perform {
            root,
            name,
            note,
            pad,
            across,
            up,
            knobs,
            fields: RefCell::default(),
            whose: RefCell::default(),
            driver: Cell::new(None),
            updating: Cell::new(false),
        })
    }

    /// The choosers' lists for `d`: its dials by stage ("Filter · Cutoff").
    fn set_driver(&self, d: &'static dyn Driver) {
        if self
            .driver
            .get()
            .is_some_and(|o| o.info().id == d.info().id)
        {
            return;
        }
        self.driver.set(Some(d));
        let p = d.init_program();
        let mut fields = Vec::new();
        let mut whose = Vec::new();
        let mut names = vec![tr("None")];
        for page in &d.curation().pages {
            for (k, section) in page.sections.iter().enumerate() {
                let title = tr(oscyl_app::editor::section_heading(page, k).unwrap_or(page.title));
                for &f in &section.fields {
                    if widget(d.control(&p, f)) == Widget::Dial && !fields.contains(&f) {
                        fields.push(f);
                        names.push(format!("{title} · {}", tr(d.short_label(f))));
                        whose.push(title.clone());
                    }
                }
            }
        }
        let refs: Vec<&str> = names.iter().map(String::as_str).collect();
        self.updating.set(true);
        for dd in [&self.across, &self.up]
            .into_iter()
            .chain(self.knobs.iter().map(|(_, dd)| dd))
        {
            dd.set_model(Some(&gtk::StringList::new(&refs)));
        }
        self.updating.set(false);
        *self.fields.borrow_mut() = fields;
        *self.whose.borrow_mut() = whose;
    }

    /// Whose field `f` is ("VCO 1"), for two knobs of one name.
    fn whose(&self, f: usize) -> String {
        let k = self.fields.borrow().iter().position(|&x| x == f);
        k.and_then(|k| self.whose.borrow().get(k).cloned())
            .unwrap_or_default()
    }

    fn position(&self, f: Option<usize>) -> u32 {
        f.and_then(|f| self.fields.borrow().iter().position(|&x| x == f))
            .map_or(0, |k| k as u32 + 1)
    }

    fn field_at(&self, position: u32) -> Option<usize> {
        let k = (position as usize).checked_sub(1)?;
        self.fields.borrow().get(k).copied()
    }
}

/// `raw` of `field` as a fraction of its range.
fn fraction(d: &dyn Driver, field: usize, raw: u16) -> f32 {
    let i = d.fields()[field];
    if i.max <= i.min {
        return 0.0;
    }
    f32::from(raw.saturating_sub(i.min)) / f32::from(i.max - i.min)
}

fn raw_at(d: &dyn Driver, field: usize, t: f32) -> u16 {
    let i = d.fields()[field];
    let span = f32::from(i.max - i.min);
    i.min + (t.clamp(0.0, 1.0) * span).round() as u16
}

impl App {
    /// The page, made the first time it is asked for.
    fn perform_page(&self) -> Rc<Perform> {
        if let Some(p) = self.perform.get() {
            return p.clone();
        }
        let p = Perform::new();
        let _ = self.perform.set(p.clone());
        // The pad: one undo step per drag (or key press).
        p.pad.connect_began(self.weak(|me, ()| {
            me.with_doc(oscyl_app::state::Doc::gesture);
        }));
        p.pad.connect_moved(self.weak(|me, (fx, fy): (f32, f32)| {
            let (Some(x), Some(y)) = me.pad_fields() else {
                return;
            };
            let d = me.doc_driver();
            me.edit(x, raw_at(d, x, fx));
            me.edit(y, raw_at(d, y, fy));
        }));
        p.pad.connect_ended(self.weak(|me, ()| {
            me.with_doc(oscyl_app::state::Doc::seal);
            me.autosave();
        }));
        p.pad.connect_reset(self.weak(|me, ()| {
            let (Some(x), Some(y), Some((_, _, b))) =
                (me.pad_fields().0, me.pad_fields().1, me.current_pair())
            else {
                return;
            };
            let d = me.doc_driver();
            me.with_doc(oscyl_app::state::Doc::gesture);
            for f in [x, y] {
                if let Ok(v) = d.get(&b, f) {
                    me.edit(f, v);
                }
            }
            me.with_doc(oscyl_app::state::Doc::seal);
            me.autosave();
        }));
        // The knobs turn their fields; a run ends when focus leaves.
        for (k, (dial, dd)) in p.knobs.iter().enumerate() {
            dial.connect_value_changed({
                let run = self.weak(move |me, v: u16| {
                    if let Some(f) = me.play_layout().macros.get(k).copied().flatten() {
                        me.edit(f, v);
                    }
                });
                move |_, v| run(v)
            });
            let focus = gtk::EventControllerFocus::new();
            let seal = self.weak(|me, ()| {
                me.with_doc(oscyl_app::state::Doc::seal);
            });
            focus.connect_leave(move |_| seal(()));
            dial.add_controller(focus);
            let choose = self.weak(move |me, at: u32| {
                me.choose_play(
                    |l, f| {
                        l.macros.resize(4, None);
                        l.macros[k] = f;
                    },
                    at,
                )
            });
            dd.connect_selected_notify(move |dd| choose(dd.selected()));
        }
        let choose = self.weak(|me, at: u32| me.choose_play(|l, f| l.x = f, at));
        p.across
            .connect_selected_notify(move |dd| choose(dd.selected()));
        let choose = self.weak(|me, at: u32| me.choose_play(|l, f| l.y = f, at));
        p.up.connect_selected_notify(move |dd| choose(dd.selected()));
        // Escape goes back to the editor.
        let keys = gtk::EventControllerKey::new();
        let back = self.weak(|me, ()| me.set_performing(false));
        keys.connect_key_pressed(move |_, key, _, _| {
            if key == gtk::gdk::Key::Escape {
                back(());
                return gtk::glib::Propagation::Stop;
            }
            gtk::glib::Propagation::Proceed
        });
        p.root.add_controller(keys);
        p
    }

    /// The layout for the model shown: chosen once, else the core's.
    pub(crate) fn play_layout(&self) -> PlayLayout {
        let d = self.doc_driver();
        self.win
            .settings
            .borrow()
            .play
            .get(d.info().id)
            .cloned()
            .unwrap_or_else(|| oscyl_app::editor::play_defaults(&d.curation(), d))
    }

    /// The pad's fields, each only where the program shows it.
    fn pad_fields(&self) -> (Option<usize>, Option<usize>) {
        let l = self.play_layout();
        let shown = |f: Option<usize>| {
            f.filter(|&f| {
                self.current_pair()
                    .is_some_and(|(_, p, _)| self.doc_driver().relevant(&p, f))
            })
        };
        (shown(l.x), shown(l.y))
    }

    /// A chooser picked entry `at`: `set` puts its field into the layout,
    /// kept for this model.
    fn choose_play(&self, set: impl FnOnce(&mut PlayLayout, Option<usize>), at: u32) {
        let Some(p) = self.perform.get() else { return };
        if p.updating.get() {
            return;
        }
        let mut layout = self.play_layout();
        set(&mut layout, p.field_at(at));
        let id = self.doc_driver().info().id.to_string();
        {
            let mut s = self.win.settings.borrow_mut();
            s.play.insert(id, layout);
            if let Err(e) = s.save(&crate::appearance::settings_path()) {
                drop(s);
                self.win.toast(
                    &tr("Couldn't save preferences: {error}").replace("{error}", &e.to_string()),
                );
            }
        }
        self.refresh_perform();
    }

    /// Show the Perform page (or go back to the editor).
    pub(crate) fn set_performing(&self, on: bool) {
        let loaded = self.current.get().is_some_and(|i| self.is_loaded(i));
        let on = on && loaded;
        if self.performing.replace(on) == on && !on {
            return;
        }
        if on {
            let p = self.perform_page();
            self.editor.show_page("perform", &p.root);
            self.refresh_perform();
            p.pad.grab_focus();
        } else {
            self.refresh_program();
        }
    }

    /// The Perform page's values, for the program open.
    pub(crate) fn refresh_perform(&self) {
        let (Some(p), Some((i, prog, base))) = (self.perform.get(), self.current_pair()) else {
            return;
        };
        let d = self.doc_driver();
        p.set_driver(d);
        let layout = self.play_layout();
        let read_only = self.with_doc(|doc| doc.is_read_only(i)).unwrap_or(true);
        let shown = |f: Option<usize>| f.filter(|&f| d.relevant(&prog, f));
        let label = |f: usize| oscyl_app::editor::label_in(d, &prog, f);
        let value = |f: usize, of: &[u8]| d.get(of, f).unwrap_or(0);
        p.updating.set(true);
        p.name
            .set_label(&self.with_doc(|doc| doc.name(i)).unwrap_or_default());
        p.note.set_visible(read_only);
        p.across.set_selected(p.position(layout.x));
        p.up.set_selected(p.position(layout.y));
        let (x, y) = (shown(layout.x), shown(layout.y));
        match (x, y) {
            (Some(x), Some(y)) => {
                let at = |of: &[u8]| (fraction(d, x, value(x, of)), fraction(d, y, value(y, of)));
                p.pad.show(Some(at(&prog)), Some(at(&base)));
                let words = tr("{x} and {y}")
                    .replace("{x}", &label(x))
                    .replace("{y}", &label(y));
                let values = format!(
                    "{} · {}",
                    oscyl_app::editor::format_in(d, &prog, x, value(x, &prog)),
                    oscyl_app::editor::format_in(d, &prog, y, value(y, &prog)),
                );
                p.pad.set_words(&label(x), &label(y), &values);
                p.pad
                    .update_property(&[gtk::accessible::Property::Label(&words)]);
                p.pad.set_sensitive(!read_only);
            }
            _ => {
                p.pad.show(None, None);
                p.pad.set_words(&tr("Choose two parameters"), "", "");
                p.pad.set_sensitive(false);
            }
        }
        // Two knobs of one name say whose they are ("VCO 1 Shape").
        let names: Vec<Option<String>> = (0..4)
            .map(|k| shown(layout.macros.get(k).copied().flatten()).map(label))
            .collect();
        let knob_label = |k: usize, f: usize| {
            let twin = names
                .iter()
                .enumerate()
                .any(|(j, n)| j != k && n.is_some() && *n == names[k]);
            if twin {
                format!("{} {}", p.whose(f), label(f))
            } else {
                label(f)
            }
        };
        for (k, (dial, dd)) in p.knobs.iter().enumerate() {
            let f = layout.macros.get(k).copied().flatten();
            dd.set_selected(p.position(f));
            match shown(f) {
                Some(f) => {
                    let info = d.fields()[f];
                    dial.set_range(info.min, info.max);
                    let name = knob_label(k, f);
                    dial.set_label(&name);
                    dial.update_property(&[gtk::accessible::Property::Label(&name)]);
                    let shown_p = prog.clone();
                    dial.set_formatter(move |raw| {
                        oscyl_app::editor::format_in(d, &shown_p, f, raw)
                    });
                    dial.show(value(f, &prog), value(f, &base));
                    dial.set_sensitive(!read_only);
                }
                None => {
                    dial.set_label(&tr("None"));
                    dial.set_sensitive(false);
                }
            }
        }
        p.updating.set(false);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pad_fractions_land_on_the_ends_and_back() {
        let d = oscyl_midi::driver("korg-minilogue-xd").unwrap();
        let f = d.fields().iter().position(|f| f.name == "Cutoff").unwrap();
        let i = d.fields()[f];
        assert_eq!(raw_at(d, f, 0.0), i.min);
        assert_eq!(raw_at(d, f, 1.0), i.max);
        assert_eq!(raw_at(d, f, 7.0), i.max, "clamped");
        for raw in [i.min, 300, i.max] {
            assert_eq!(raw_at(d, f, fraction(d, f, raw)), raw);
        }
    }
}
