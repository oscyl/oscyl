//! Preferences, keyboard shortcuts, About and the app's alerts.

use std::rc::Rc;

use adw::prelude::*;
use gtk::{gio, glib};

use crate::actions::{ACTIONS, Section};
use crate::app::App;
use crate::i18n::{tr, trn};
use crate::settings::{Settings, Theme, ZOOM_MAX, ZOOM_MIN, ZOOM_STEP};
use crate::sidebar::Place;
use crate::window::OscylWindow;

pub fn preferences(app: &App) {
    let win = &app.win;
    let page = adw::PreferencesPage::new();

    let look = adw::PreferencesGroup::builder()
        .title(tr("Appearance"))
        .build();
    let themes = gtk::StringList::new(&[]);
    for t in Theme::ALL {
        themes.append(&t.label());
    }
    let theme_row = adw::ComboRow::builder()
        .title(tr("Style"))
        .subtitle(tr(
            "AMOLED uses true black, for OLED screens and dark studios",
        ))
        .model(&themes)
        .build();
    let current = win.settings.borrow().theme();
    theme_row.set_selected(Theme::ALL.iter().position(|t| *t == current).unwrap_or(0) as u32);
    let w = Rc::downgrade(win);
    theme_row.connect_selected_notify(move |r| {
        if let Some(w) = w.upgrade() {
            update(&w, |s| s.set_theme(Theme::ALL[r.selected() as usize]));
        }
    });
    look.add(&theme_row);

    let zoom = adw::SpinRow::builder()
        .title(tr("Interface size"))
        .subtitle(tr("Also Ctrl+Plus and Ctrl+Minus; Ctrl+0 resets"))
        .adjustment(&gtk::Adjustment::new(
            win.settings.borrow().zoom() * 100.0,
            ZOOM_MIN * 100.0,
            ZOOM_MAX * 100.0,
            ZOOM_STEP * 100.0,
            ZOOM_STEP * 100.0,
            0.0,
        ))
        .build();
    let w = Rc::downgrade(win);
    zoom.connect_value_notify(move |r| {
        if let Some(w) = w.upgrade() {
            update(&w, |s| s.set_zoom(r.value() / 100.0));
        }
    });
    look.add(&zoom);

    let motion = adw::SwitchRow::builder()
        .title(tr("Reduce motion"))
        .active(win.settings.borrow().reduce_motion)
        .build();
    let w = Rc::downgrade(win);
    motion.connect_active_notify(move |r| {
        if let Some(w) = w.upgrade() {
            update(&w, |s| s.reduce_motion = r.is_active());
        }
    });
    look.add(&motion);
    page.add(&look);

    let synth = adw::PreferencesGroup::builder()
        .title(tr("Synth"))
        .description(tr("Oscyl talks to your synths over USB-MIDI. No synth? The simulated synths answer like real ones, so you can try reading, syncing and saving, but they make no sound."))
        .build();
    let sim = adw::SwitchRow::builder()
        .title(tr("Use simulated synth"))
        .subtitle(tr("No sound. Takes effect the next time you connect"))
        .active(win.settings.borrow().simulator())
        .build();
    let w = Rc::downgrade(win);
    sim.connect_active_notify(move |r| {
        if let Some(w) = w.upgrade() {
            update(&w, |s| s.set_simulator(r.is_active()));
        }
    });
    synth.add(&sim);
    page.add(&synth);

    let dialog = adw::PreferencesDialog::new();
    dialog.add(&page);
    dialog.present(Some(&win.window));
}

fn update(win: &OscylWindow, f: impl FnOnce(&mut Settings)) {
    let mut s = win.settings.borrow_mut();
    f(&mut s);
    crate::appearance::apply(&s);
    if let Err(e) = s.save(&crate::appearance::settings_path()) {
        win.toast(&tr("Couldn't save preferences: {error}").replace("{error}", &e.to_string()));
    }
}

pub fn shortcuts(win: &OscylWindow) {
    let dialog = adw::ShortcutsDialog::new();
    for section in Section::ALL {
        let group = adw::ShortcutsSection::new(Some(&section.title()));
        for a in ACTIONS.iter().filter(|a| a.section == Some(section)) {
            // Keypad duplicates add nothing to read.
            let accels: Vec<&str> = a
                .accels
                .iter()
                .copied()
                .filter(|k| !k.contains("KP_"))
                .collect();
            group.add(adw::ShortcutsItem::new(&tr(a.label), &accels.join(" ")));
        }
        // Keys of the focused row, handled by the sidebar and the list.
        let rows: &[(String, &str)] = match section {
            Section::General => &[
                (tr("Rename library or folder"), "F2"),
                (tr("Move library or folder up or down"), "<Alt>Up <Alt>Down"),
                (tr("Close or open a folder"), "Left Right"),
            ],
            Section::Programs => &[
                (tr("Move program up or down"), "<Alt>Up <Alt>Down"),
                (
                    tr("Swap program with the one above or below"),
                    "<Control><Alt>Up <Control><Alt>Down",
                ),
                (tr("Remove program from a library"), "Delete"),
            ],
            _ => &[],
        };
        for (label, accel) in rows {
            group.add(adw::ShortcutsItem::new(label, accel));
        }
        if section == Section::Editor {
            // Dial keys, handled by the focused dial rather than actions.
            for (label, accel) in [
                (tr("Change value by a step"), "Up Down"),
                (tr("Change value in large steps"), "Page_Up Page_Down"),
                (tr("Minimum or maximum"), "Home End"),
            ] {
                group.add(adw::ShortcutsItem::new(&label, accel));
            }
        }
        dialog.add(group);
    }
    dialog.present(Some(&win.window));
}

pub fn about(win: &OscylWindow) {
    let about = adw::AboutDialog::builder()
        .application_name("Oscyl")
        .application_icon("io.github.oscyl.Oscyl")
        .developer_name("Giuseppe Maggio")
        .version(env!("CARGO_PKG_VERSION"))
        .license_type(gtk::License::MitX11)
        .comments(tr(
            "Library, editor and sync for hardware synths. Not affiliated with any synth maker.",
        ))
        .website("https://github.com/oscyl")
        .issue_url("https://github.com/oscyl/oscyl/issues")
        .translator_credits(tr("translator-credits"))
        .build();
    about.add_legal_section(
        "Albert Sans",
        Some("Copyright 2021 The Albert Sans Project Authors"),
        gtk::License::Custom,
        Some("SIL Open Font License 1.1"),
    );
    // The Rust crates the app is built from, one section per licence text.
    match gio::resources_lookup_data(
        "/io/github/oscyl/Oscyl/licences/rust.json",
        gio::ResourceLookupFlags::NONE,
    )
    .ok()
    .and_then(|bytes| RustLicences::parse(bytes.as_ref()).ok())
    {
        Some(licences) => {
            for (crates, text) in licences.groups() {
                let names: Vec<&str> = crates.iter().map(|c| c.name.as_str()).collect();
                // A short title; every crate with its version beneath it.
                let all: Vec<String> = crates
                    .iter()
                    .map(|c| format!("{} {}", c.name, c.version))
                    .collect();
                about.add_legal_section(
                    &section_title(&names),
                    Some(&glib::markup_escape_text(&all.join(", "))),
                    gtk::License::Custom,
                    Some(&glib::markup_escape_text(&text.text)),
                );
            }
        }
        None => glib::g_warning!("oscyl", "bundled licences/rust.json missing or unreadable"),
    }
    about.present(Some(&win.window));
}

/// `data/licences/rust.json`, written by `scripts/licences.sh` from cargo-about:
/// the third-party crates and the licence texts they use.
#[derive(serde::Deserialize)]
struct RustLicences {
    crates: Vec<RustCrate>,
    licences: Vec<LicenceText>,
}

#[derive(serde::Deserialize)]
struct RustCrate {
    name: String,
    version: String,
    /// Indexes into `licences`.
    texts: Vec<usize>,
}

#[derive(serde::Deserialize)]
struct LicenceText {
    text: String,
}

/// A legal section's title: up to three crates by name, else the first two
/// and how many more ("gtk4, glib and 12 more").
fn section_title(names: &[&str]) -> String {
    match names {
        [] => String::new(),
        [_] | [_, _] | [_, _, _] => names.join(", "),
        [a, b, rest @ ..] => tr("{first}, {second} and {count} more")
            .replace("{first}", a)
            .replace("{second}", b)
            .replace("{count}", &rest.len().to_string()),
    }
}

impl RustLicences {
    fn parse(bytes: &[u8]) -> serde_json::Result<RustLicences> {
        serde_json::from_slice(bytes)
    }

    /// Each licence text with the crates under it, in the file's order.
    fn groups(&self) -> Vec<(Vec<&RustCrate>, &LicenceText)> {
        self.licences
            .iter()
            .enumerate()
            .map(|(i, text)| {
                let crates = self
                    .crates
                    .iter()
                    .filter(|c| c.texts.contains(&i))
                    .collect();
                (crates, text)
            })
            .filter(|(crates, _): &(Vec<&RustCrate>, _)| !crates.is_empty())
            .collect()
    }
}

impl App {
    pub(crate) fn error_dialog(&self, heading: &str, body: &str) {
        let d = adw::AlertDialog::new(Some(heading), Some(body));
        d.add_response("ok", &tr("OK"));
        d.present(Some(&self.win.window));
    }

    /// Ask, then back up and write the synth's edited programs. `staged`:
    /// a library was just staged onto the synth as one undo step, which
    /// cancelling discards; syncing then shows the synth.
    pub(crate) fn confirm_sync(&self, staged: bool) {
        let Some(id) = self.active.get() else {
            return;
        };
        let unstage = move |me: &App| {
            if staged {
                me.with_synth_id(id, |s| s.doc.discard());
                me.after_edit();
            }
        };
        // The driver plans the commit (with the neighbours it has read);
        // the app sends what changed.
        let Some(plan) = self.with_synth_id(id, |s| {
            oscyl_app::review::sync_plan(&s.doc, |i| s.loaded.get(i).copied().unwrap_or(false))
                .map(|p| (oscyl_app::review::sync_names(&s.doc, &p.indices), p))
        }) else {
            return;
        };
        let (
            names,
            oscyl_app::review::SyncPlan {
                indices: edited,
                batch,
            },
        ) = match plan {
            Ok(p) => p,
            Err(why) => {
                unstage(self);
                self.error_dialog(&tr("Can't sync yet"), &why);
                return;
            }
        };
        if batch.is_empty() {
            unstage(self);
            return;
        }
        let dialog = adw::AlertDialog::new(
            Some(&trn(
                "Sync %d program?",
                "Sync %d programs?",
                names.len() as u32,
            )),
            Some(&format!(
                "{}\n\n{}",
                names.join("\n"),
                tr("Only these slots change. Oscyl backs up what they hold now first.")
            )),
        );
        dialog.add_response("cancel", &tr("Cancel"));
        dialog.add_response("sync", &tr("Sync"));
        dialog.set_response_appearance("sync", adw::ResponseAppearance::Suggested);
        dialog.set_default_response(Some("sync"));
        dialog.set_close_response("cancel");
        let respond = self.weak(move |me, sync: bool| {
            if !sync {
                unstage(me);
                return;
            }
            me.sync(id, &edited, batch.clone());
            if staged {
                me.sidebar.select(&Place::Synth);
            }
        });
        dialog.connect_response(None, move |_, r| respond(r == "sync"));
        dialog.present(Some(&self.win.window));
    }

    /// Ask before closing with unsaved edits.
    pub fn close_request(&self) -> glib::Propagation {
        self.autosave();
        if self.allow_close.get() {
            self.quit_libraries();
            return glib::Propagation::Proceed;
        }
        let unsaved: Vec<String> = self
            .libs
            .borrow()
            .iter()
            .filter(|l| l.unsaved())
            .map(|l| l.doc.title.clone())
            .chain(
                self.synths
                    .borrow()
                    .iter()
                    .filter(|s| s.doc.is_dirty())
                    .map(|s| tr("Programs on {synth} (not synced)").replace("{synth}", &s.name)),
            )
            .collect();
        if unsaved.is_empty() {
            self.quit_libraries();
            return glib::Propagation::Proceed;
        }
        let dialog = adw::AlertDialog::new(
            Some(&tr("Quit without saving?")),
            Some(&format!(
                "{}\n\n{}",
                tr("These have edits that are not saved or synced:"),
                unsaved.join("\n")
            )),
        );
        dialog.add_response("cancel", &tr("Keep Editing"));
        dialog.add_response("discard", &tr("Quit and Lose Edits"));
        dialog.set_response_appearance("discard", adw::ResponseAppearance::Destructive);
        dialog.set_default_response(Some("cancel"));
        dialog.set_close_response("cancel");
        let quit = self.weak(|me, ()| {
            me.allow_close.set(true);
            me.win.close();
        });
        dialog.connect_response(None, move |_, r| {
            if r == "discard" {
                quit(());
            }
        });
        dialog.present(Some(&self.win.window));
        glib::Propagation::Stop
    }
}

/// Explore Demo Library with no synth shown: which synth's demo, each on
/// its nameplate (an abstract panel drawn from the driver's own pages and
/// fields by the core, the model, the maker and its voices).
pub fn pick_demo(win: &OscylWindow, open: impl Fn(&'static dyn oscyl_driver::Driver) + 'static) {
    let dialog = adw::Dialog::builder()
        .title(tr("Explore a Demo Library"))
        .content_width(560)
        .build();
    let view = adw::ToolbarView::new();
    view.add_top_bar(&adw::HeaderBar::new());
    let tiles = gtk::FlowBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .min_children_per_line(2)
        .max_children_per_line(3)
        .column_spacing(12)
        .row_spacing(12)
        .homogeneous(true)
        .css_classes(["nameplates"])
        .build();
    let open = Rc::new(open);
    for d in oscyl_midi::DRIVERS {
        let d: &'static dyn oscyl_driver::Driver = *d;
        let info = d.info();
        let panel = gtk::DrawingArea::builder()
            .content_height(64)
            .css_classes(["nameplate-panel"])
            .accessible_role(gtk::AccessibleRole::Presentation)
            .build();
        let drawing = oscyl_app::panel::panel(d);
        panel.set_draw_func(move |area, cr, w, h| draw_panel(area, cr, w, h, &drawing));
        let model = gtk::Label::builder()
            .label(info.model)
            .xalign(0.0)
            .css_classes(["nameplate-model"])
            .build();
        let voices = oscyl_app::panel::voices_text(d.voices());
        let about = if voices.is_empty() {
            info.vendor.to_string()
        } else {
            tr("{maker} · {voices}")
                .replace("{maker}", info.vendor)
                .replace("{voices}", &voices)
        };
        let maker = gtk::Label::builder()
            .label(&about)
            .xalign(0.0)
            .css_classes(["dim-label", "caption"])
            .build();
        let content = gtk::Box::new(gtk::Orientation::Vertical, 6);
        content.append(&panel);
        content.append(&model);
        content.append(&maker);
        let tile = gtk::Button::builder()
            .child(&content)
            .css_classes(["nameplate"])
            .tooltip_text(tr("Open the {model} demo library").replace("{model}", info.model))
            .build();
        tile.update_property(&[gtk::accessible::Property::Label(
            &tr("{model} by {maker}")
                .replace("{model}", info.model)
                .replace("{maker}", info.vendor),
        )]);
        let (open, dialog) = (open.clone(), dialog.clone());
        tile.connect_clicked(move |_| {
            dialog.close();
            open(d);
        });
        tiles.append(&tile);
    }
    let body = gtk::Box::new(gtk::Orientation::Vertical, 12);
    body.add_css_class("pick-demo");
    body.append(
        &gtk::Label::builder()
            .label(tr("Example sounds to explore and edit, no synth needed."))
            .xalign(0.0)
            .wrap(true)
            .css_classes(["dim-label"])
            .build(),
    );
    body.append(&tiles);
    view.set_content(Some(&body));
    dialog.set_child(Some(&view));
    dialog.present(Some(&win.window));
}

/// A nameplate's panel, as the phone draws it: its frame, the name display,
/// a knob per field column with its index at noon, in shades of the
/// widget's ink.
fn draw_panel(
    area: &gtk::DrawingArea,
    cr: &gtk::cairo::Context,
    w: i32,
    h: i32,
    p: &oscyl_app::panel::Panel,
) {
    use std::f64::consts::{FRAC_PI_2, PI, TAU};
    let c = area.color();
    let ink = |a: f64| {
        cr.set_source_rgba(
            f64::from(c.red()),
            f64::from(c.green()),
            f64::from(c.blue()),
            f64::from(c.alpha()) * a,
        )
    };
    let (w, h) = (f64::from(w), f64::from(h));
    cr.set_line_width(1.0);
    let r = 6.0;
    let (x0, y0, x1, y1) = (0.5, 0.5, w - 0.5, h - 0.5);
    cr.new_sub_path();
    cr.arc(x1 - r, y0 + r, r, -FRAC_PI_2, 0.0);
    cr.arc(x1 - r, y1 - r, r, 0.0, FRAC_PI_2);
    cr.arc(x0 + r, y1 - r, r, FRAC_PI_2, PI);
    cr.arc(x0 + r, y0 + r, r, PI, 1.5 * PI);
    cr.close_path();
    ink(0.35);
    let _ = cr.stroke();
    let pad = 6.0;
    let (iw, ih) = (w - 2.0 * pad, h - 2.0 * pad);
    if let Some(d) = p.display {
        cr.rectangle(
            pad + f64::from(d.x) * iw,
            pad + f64::from(d.y) * ih,
            f64::from(d.w) * iw,
            f64::from(d.h) * ih,
        );
        ink(0.85);
        let _ = cr.fill();
    }
    // Knobs keep round: their radius follows the shorter side.
    let unit = iw.min(ih);
    for k in &p.knobs {
        let (x, y) = (pad + f64::from(k.x) * iw, pad + f64::from(k.y) * ih);
        let kr = (f64::from(k.r) * unit).max(3.5);
        cr.arc(x, y, kr, 0.0, TAU);
        ink(0.45);
        let _ = cr.stroke();
        cr.set_line_width(1.5);
        cr.move_to(x, y);
        cr.line_to(x, y - kr * 0.8);
        ink(1.0);
        let _ = cr.stroke();
        cr.set_line_width(1.0);
    }
}

#[cfg(test)]
mod licence_tests {
    use super::*;

    #[test]
    fn legal_titles_stay_short() {
        assert_eq!(section_title(&["gtk4"]), "gtk4");
        assert_eq!(section_title(&["a", "b", "c"]), "a, b, c");
        assert_eq!(
            section_title(&["gtk4", "glib", "x", "y"]),
            "gtk4, glib and 2 more"
        );
    }

    #[test]
    fn rust_json_parses_and_lists_the_shipped_crates() {
        let licences = RustLicences::parse(include_bytes!("../data/licences/rust.json")).unwrap();
        assert!(licences.crates.iter().any(|c| c.name == "gtk4"));
        let groups = licences.groups();
        // Every crate is in a group, and every group has its licence text.
        let grouped: usize = groups.iter().map(|(crates, _)| crates.len()).sum();
        assert!(grouped >= licences.crates.len());
        assert!(
            groups
                .iter()
                .all(|(crates, text)| !crates.is_empty() && !text.text.is_empty())
        );
    }
}
