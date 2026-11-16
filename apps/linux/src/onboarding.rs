//! How the app starts: the first-run welcome dialog, which asks what the
//! person mainly does and sets up the workspace for it, and the scripted
//! states used for design screenshots.

use std::rc::Rc;

use adw::prelude::*;

use crate::i18n::tr;
use crate::settings::Role;
use crate::sidebar::{Place, SynthStatus};
use crate::state::Doc;

pub struct Choice {
    pub role: Role,
    pub demo: bool,
    pub simulator: bool,
}

/// Present the welcome dialog; `done` runs once with the person's choice,
/// or `None` when they skip.
pub fn present(parent: &impl IsA<gtk::Widget>, done: impl Fn(Option<Choice>) + 'static) {
    let dialog = adw::Dialog::builder()
        .title(tr("Welcome"))
        .content_width(600)
        .build();
    let view = adw::ToolbarView::new();
    // The page's own heading names the dialog; a second title would repeat it.
    view.add_top_bar(&adw::HeaderBar::builder().show_title(false).build());

    let page = adw::PreferencesPage::new();
    let intro = adw::PreferencesGroup::builder()
        .title(tr("Welcome to Oscyl"))
        .description(tr(
            "What do you mainly want to do? Oscyl sets up the workspace for it.",
        ))
        .build();
    let roles = [
        (
            Role::SoundDesign,
            tr("Design sounds"),
            tr("Start in the editor at the oscillators, with live audition on."),
        ),
        (
            Role::Organise,
            tr("Organise my library"),
            tr("Start with your programs: search, back up and sync."),
        ),
        (
            Role::Perform,
            tr("Prepare a performance"),
            tr("Start at voice and arpeggiator settings, ready to audition."),
        ),
    ];
    let mut first: Option<gtk::CheckButton> = None;
    let mut buttons = Vec::new();
    for (role, title, subtitle) in roles {
        let check = gtk::CheckButton::new();
        if let Some(f) = &first {
            check.set_group(Some(f));
        } else {
            check.set_active(true);
            first = Some(check.clone());
        }
        let row = adw::ActionRow::builder()
            .title(title)
            .subtitle(subtitle)
            .activatable_widget(&check)
            .build();
        row.add_prefix(&check);
        intro.add(&row);
        buttons.push((role, check));
    }
    page.add(&intro);

    let start = adw::PreferencesGroup::builder()
        .title(tr("Getting started"))
        .build();
    let demo = adw::SwitchRow::builder()
        .title(tr("Open the demo library"))
        .subtitle(tr("120 example sounds to explore and edit"))
        .active(true)
        .build();
    let sim = adw::SwitchRow::builder()
        .title(tr("Use the simulated synth"))
        .subtitle(tr(
            "No synth? Try reading and syncing with a simulated one (no sound)",
        ))
        .build();
    start.add(&demo);
    start.add(&sim);
    page.add(&start);

    let go = gtk::Button::builder()
        .label(tr("Get Started"))
        .halign(gtk::Align::Center)
        .css_classes(["suggested-action", "pill"])
        .build();
    let actions = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    actions.set_halign(gtk::Align::Center);
    actions.set_margin_top(12);
    actions.set_margin_bottom(18);
    actions.append(&go);
    view.set_content(Some(&page));
    view.add_bottom_bar(&actions);
    dialog.set_child(Some(&view));
    dialog.set_default_widget(Some(&go));

    // Closing the dialog any other way counts as skipping it.
    let chosen = Rc::new(std::cell::Cell::new(false));
    let done = Rc::new(done);
    {
        let (chosen, done) = (chosen.clone(), done.clone());
        dialog.connect_closed(move |_| {
            if !chosen.get() {
                done(None);
            }
        });
    }
    {
        let d = dialog.clone();
        go.connect_clicked(move |_| {
            let role = buttons
                .iter()
                .find(|(_, b)| b.is_active())
                .map(|(r, _)| *r)
                .unwrap_or(Role::SoundDesign);
            chosen.set(true);
            done(Some(Choice {
                role,
                demo: demo.is_active(),
                simulator: sim.is_active(),
            }));
            d.close();
        });
    }
    dialog.present(Some(parent));
}

impl crate::app::App {
    /// Offer the welcome dialog once, then set up the chosen workspace;
    /// later launches start at the chosen role's stage.
    pub fn first_run(&self) {
        let (onboarded, role) = {
            let s = self.win.settings.borrow();
            (s.onboarded, s.role)
        };
        if onboarded {
            if let Some(role) = role {
                self.editor.select_icon(role.start_page_icon());
            }
            return;
        }
        present(
            &self.win.window,
            self.weak(|me, choice: Option<Choice>| {
                {
                    let mut s = me.win.settings.borrow_mut();
                    s.onboarded = true;
                    if let Some(c) = &choice {
                        s.role = Some(c.role);
                        s.use_simulator |= c.simulator;
                    }
                    if let Err(e) = s.save(&crate::appearance::settings_path()) {
                        me.win.toast(
                            &tr("Couldn't save preferences: {error}")
                                .replace("{error}", &e.to_string()),
                        );
                    }
                }
                let Some(c) = choice else { return };
                if c.demo {
                    me.open_demo();
                }
                if c.simulator {
                    me.connect_synth();
                }
                me.editor.select_icon(c.role.start_page_icon());
                if c.role == crate::settings::Role::SoundDesign && c.simulator {
                    me.set_audition(true);
                }
                if c.role == crate::settings::Role::Organise {
                    me.win.inner.set_show_content(false);
                }
            }),
        );
    }

    /// Scripted state for design screenshots: `demo`, optionally
    /// `:synth=<driver id>`, followed by any of
    /// `:<stage>`, `:edited`, `:enlarge`, `:reading`, `:arp`, `:fx`, `:audition`, `:compare`, `:blend` (40% of the way to the edit), `:roll` (the stage shown, rolled), `:history` (snapshot A stored, the History popover open), `:perform` (the Perform page), `:demos` (the demo chooser's nameplates), `:welcome`, `:generate` (or `:generate-empty`, no hand), `:new-oscillators` (six new oscillators), `:keep`, `:libraries` (two of Oscyl's libraries, one shown), `:folders` (those in nested folders, one closed), `:drop-line` and `:drop-into` (a drop's mark in the tree), `:new-library` (an empty one, being named), `:new-program` (an Init Program after the one shown, being named), `:reorder` and `:swap` (a program held between rows, or onto one), `:sources` (the sidebar over a phone's window), `:units` (a shelf and slots, no synth needed), `:guide` and `:about`
    /// (stages by title, e.g. `demo:filter:edited`).
    pub fn run_demo(&self, spec: &str) {
        let mut parts = spec.split(':');
        if parts.next() != Some("demo") {
            return;
        }
        // `synth=<driver id>` first picks the synth whose demo shows.
        let mut parts = parts.peekable();
        let driver = parts
            .next_if(|p| p.starts_with("synth="))
            .and_then(|p| oscyl_midi::driver(&p["synth=".len()..]));
        // Captures never ask which synth: the first Oscyl knows.
        self.open_demo_of(driver.unwrap_or(oscyl_midi::DRIVERS[0]));
        self.show_program(3);
        // Fields by schema name; a driver without one skips it.
        let d = self.editor.driver();
        let field = |name: &str| d.fields().iter().position(|f| f.name == name);
        let edit = |name: &str, v: u16| {
            if let Some(f) = field(name) {
                self.edit(f, v);
            }
        };
        for part in parts {
            if let Some(stage) = self.editor.page_named(part) {
                self.editor.select_stage(stage);
            } else if part == "edited" {
                edit("Cutoff", 618);
                self.with_doc(Doc::seal);
                edit("VCO 2 Pitch", 560);
                edit("VCO 1 Wave", 0);
            } else if part == "enlarge" {
                self.editor.toggle_enlarged();
            } else if part == "reading" {
                // Design review only: the simulator reads too fast to catch
                // this state, so show the real component at a fixed point.
                let model = self.doc_driver().info().model;
                self.sidebar.set_status(
                    model,
                    &SynthStatus::Reading {
                        done: 312,
                        total: 500,
                    },
                );
            } else if part == "type" {
                if let Some(f) = field("Cutoff") {
                    self.editor.demo_type(f);
                }
            } else if part == "bottom" {
                self.editor.demo_scroll_to_end();
            } else if part == "backdrop" {
                // Design captures: the window as it looks unfocused.
                self.win
                    .window
                    .set_state_flags(gtk::StateFlags::BACKDROP, false);
            } else if part == "review" {
                self.editor.open_review();
            } else if part == "compare" {
                self.compare(true);
            } else if part == "roll" {
                // A fixed program to roll from is enough: the seed varies.
                self.roll_stage(self.editor.stage());
            } else if part == "history" {
                // The edits so far and a snapshot, the popover open.
                self.store_demo_snapshot();
                if let Some(h) = self.primary.borrow().as_ref().map(|p| p.history.clone()) {
                    h.popup();
                }
            } else if part == "demos" {
                // Explore Demo Library with no synth: the nameplates (over
                // the demo already open).
                crate::dialogs::pick_demo(&self.win, |_| {});
            } else if part == "perform" {
                self.set_performing(true);
            } else if part == "blend" {
                self.set_blend(0.4, false);
            } else if part == "audition" {
                self.set_audition(true);
            } else if part == "fx" {
                for (f, v) in [
                    ("Mod FX On", 1),
                    ("Mod FX Type", 2),
                    ("Delay On", 1),
                    ("Reverb On", 1),
                    ("Delay Time", 380),
                    ("Delay Depth", 700),
                    ("Reverb Time", 600),
                ] {
                    edit(f, v);
                }
            } else if part == "arp" {
                edit("Voice Mode Type", 1);
                edit("Voice Mode Depth", 200);
            } else if part == "welcome" {
                present(&self.win.window, |_| {});
            } else if part == "generate" || part == "generate-empty" {
                // A fixed seed, so captures are the same every run.
                self.sidebar.select(&Place::Generate);
                if let Some(g) = self.generator.get().filter(|_| part == "generate") {
                    g.set_seed(7);
                    g.deal();
                }
            } else if part == "new-oscillators" {
                // New oscillators dealt, in view, with a fixed seed.
                self.sidebar.select(&Place::Generate);
                if let Some(g) = self.generator.get() {
                    g.set_seed(7);
                    g.deal_oscillators();
                }
            } else if part == "keep" {
                if let Some(g) = self.generator.get() {
                    g.keep_selected();
                }
            } else if part == "libraries" {
                self.demo_libraries();
            } else if part == "folders" {
                self.demo_folders();
            } else if part == "drop-line" || part == "drop-into" {
                self.demo_drop(part == "drop-line");
            } else if part == "new-program" {
                self.new_program(self.current.get());
            } else if part == "reorder" || part == "swap" {
                // A program of the library shown held over the list.
                self.list.demo_drag(2, 6, part == "swap");
            } else if part == "new-library" {
                self.sidebar.select(&Place::Synth);
                self.new_library();
            } else if part == "sources" {
                // Collapsing hides the sidebar: show it once collapsed.
                self.win.outer.connect_collapsed_notify(|o| {
                    if o.is_collapsed() {
                        o.set_show_sidebar(true);
                    }
                });
            } else if part == "toast" {
                // A notice with its action, as an edit's Undo shows it.
                self.win.show_toast(
                    adw::Toast::builder()
                        .title(tr("Removed “{program}”").replace("{program}", "Glass Choir"))
                        .button_label(tr("Undo"))
                        .build(),
                );
            } else if part == "units" {
                self.sidebar.select(&Place::Units);
                self.demo_units();
            } else if part == "guide" {
                self.sidebar.select(&Place::Guide);
            } else if part == "about" {
                crate::dialogs::about(&self.win);
            }
        }
    }
}
