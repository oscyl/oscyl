//! Every window action in one table: its name, shortcuts, label in the
//! Keyboard Shortcuts dialog and handler. `install_actions` and the
//! dialog both read it, so they cannot drift apart.

use adw::prelude::*;
use gtk::gio;

use crate::app::App;
use crate::i18n::tr;
use crate::settings::ZOOM_STEP;
use crate::sidebar::Place;
use crate::synth::Cmd;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    General,
    Programs,
    Editor,
    View,
}

impl Section {
    pub const ALL: [Section; 4] = [
        Section::General,
        Section::Programs,
        Section::Editor,
        Section::View,
    ];

    pub fn title(self) -> String {
        match self {
            Section::General => tr("General"),
            Section::Programs => tr("Programs"),
            Section::Editor => tr("Editor"),
            Section::View => tr("View"),
        }
    }
}

pub enum Run {
    Do(fn(&App)),
    /// A stateful on/off action (bound to a switch).
    Toggle(fn(&App, bool)),
}

pub struct Action {
    pub name: &'static str,
    pub accels: &'static [&'static str],
    /// Where the Keyboard Shortcuts dialog lists it; `None` hides it.
    pub section: Option<Section>,
    pub label: &'static str,
    pub run: Run,
}

const fn act(
    name: &'static str,
    accels: &'static [&'static str],
    section: Option<Section>,
    label: &'static str,
    run: fn(&App),
) -> Action {
    Action {
        name,
        accels,
        section,
        label,
        run: Run::Do(run),
    }
}

use Section::{Editor, General, Programs, View};

pub const ACTIONS: &[Action] = &[
    act(
        "open",
        &["<Control>o"],
        Some(General),
        "Open library",
        |a| a.open_library(),
    ),
    act("open-demo", &[], None, "", |a| a.open_demo()),
    act(
        "primary",
        &["<Control>s", "<Control>Return"],
        Some(General),
        "Save library or sync",
        |a| a.primary(),
    ),
    act(
        "save-as",
        &["<Control><Shift>s"],
        Some(General),
        "Save library as",
        |a| a.save_library(true),
    ),
    act(
        "backup",
        &["<Control>b"],
        Some(General),
        "Back up synth",
        |a| a.backup_synth(),
    ),
    act(
        "preferences",
        &["<Control>comma"],
        Some(General),
        "Preferences",
        crate::dialogs::preferences,
    ),
    act("guide", &["F1"], Some(General), "Guide", |a| {
        a.sidebar.select(&Place::Guide)
    }),
    act(
        "shortcuts",
        &["<Control>question", "<Control>slash"],
        Some(General),
        "Keyboard shortcuts",
        |a| crate::dialogs::shortcuts(&a.win),
    ),
    act("about", &[], None, "", |a| crate::dialogs::about(&a.win)),
    act("save-playing", &[], None, "", |a| {
        if let Some(id) = a.active.get() {
            a.synth.send(Cmd::ReadCurrent(id));
        }
    }),
    act("quit", &["<Control>q"], Some(General), "Quit", |a| {
        a.win.close()
    }),
    act(
        "connect",
        &["<Control><Shift>c"],
        Some(General),
        "Connect the synth",
        |a| a.connect_synth(),
    ),
    act("disconnect", &[], None, "", |a| a.disconnect_synth()),
    act(
        "cancel-read",
        &["Escape"],
        Some(Programs),
        "Restore the graph, or stop reading programs",
        |a| {
            if !a.editor.restore_chart()
                && let Some(id) = a.active.get()
            {
                a.synth.send(Cmd::CancelRead(id))
            }
            a.update_actions();
        },
    ),
    act(
        "search",
        &["<Control>f"],
        Some(Programs),
        "Search programs",
        |a| {
            a.win.inner.set_show_content(false);
            a.list.search.grab_focus();
        },
    ),
    act(
        "generate",
        &["<Control>g"],
        Some(Programs),
        "Generate sounds",
        |a| a.sidebar.select(&Place::Generate),
    ),
    act(
        "next-program",
        &["<Control>Page_Down"],
        Some(Programs),
        "Next program",
        |a| a.step_program(1),
    ),
    act(
        "prev-program",
        &["<Control>Page_Up"],
        Some(Programs),
        "Previous program",
        |a| a.step_program(-1),
    ),
    act("undo", &["<Control>z"], Some(Programs), "Undo", |a| {
        a.undo()
    }),
    act(
        "redo",
        &["<Control><Shift>z", "<Control>y"],
        Some(Programs),
        "Redo",
        |a| a.redo(),
    ),
    act(
        "perform",
        &["<Control>p"],
        Some(Editor),
        "Perform: the XY pad and four knobs, or back to the editor",
        |a| a.set_performing(!a.performing.get()),
    ),
    act(
        "history",
        &["<Control>h"],
        Some(Programs),
        "Edit history and snapshots",
        |a| {
            if let Some(h) = a.primary.borrow().as_ref().map(|p| p.history.clone())
                && h.is_sensitive()
                && h.is_visible()
            {
                h.popup();
            }
        },
    ),
    act(
        "new-program",
        &["<Control>n"],
        Some(Programs),
        "New program in a library",
        |a| a.new_program(a.current.get()),
    ),
    act(
        "copy-to-library",
        &["<Control><Shift>l"],
        Some(Programs),
        "Copy program to a library",
        |a| a.copy_to_library_menu(),
    ),
    act(
        "revert-program",
        &["<Control><Shift>r"],
        Some(Programs),
        "Revert program",
        |a| a.revert_program(),
    ),
    Action {
        name: "audition",
        accels: &["<Control><Shift>a"],
        section: Some(Programs),
        label: "Audition live on or off",
        run: Run::Toggle(|a, on| a.set_audition(on)),
    },
    act("stage-1", &["<Alt>1"], Some(Editor), "Page 1", |a| {
        a.editor.select_stage(0)
    }),
    act("stage-2", &["<Alt>2"], Some(Editor), "Page 2", |a| {
        a.editor.select_stage(1)
    }),
    act("stage-3", &["<Alt>3"], Some(Editor), "Page 3", |a| {
        a.editor.select_stage(2)
    }),
    act("stage-4", &["<Alt>4"], Some(Editor), "Page 4", |a| {
        a.editor.select_stage(3)
    }),
    act("stage-5", &["<Alt>5"], Some(Editor), "Page 5", |a| {
        a.editor.select_stage(4)
    }),
    act("stage-6", &["<Alt>6"], Some(Editor), "Page 6", |a| {
        a.editor.select_stage(5)
    }),
    act(
        "inspect",
        &["<Control>i"],
        Some(Editor),
        "Enlarge graph",
        |a| {
            a.editor.toggle_enlarged();
            a.update_actions();
        },
    ),
    act(
        "zoom-in",
        &["<Control>plus", "<Control>equal", "<Control>KP_Add"],
        Some(View),
        "Larger interface",
        |a| zoom(a, ZOOM_STEP),
    ),
    act(
        "zoom-out",
        &["<Control>minus", "<Control>KP_Subtract"],
        Some(View),
        "Smaller interface",
        |a| zoom(a, -ZOOM_STEP),
    ),
    act(
        "zoom-reset",
        &["<Control>0", "<Control>KP_0"],
        Some(View),
        "Reset interface size",
        |a| zoom(a, 0.0),
    ),
    act(
        "toggle-sidebar",
        &["F9"],
        Some(View),
        "Show or hide sources",
        |a| a.win.outer.set_show_sidebar(!a.win.outer.shows_sidebar()),
    ),
];

/// Change the interface zoom by `by`; 0 resets it.
fn zoom(a: &App, by: f64) {
    // Keep the reader's place: where the editor was scrolled to, and the
    // selected program in the list, survive the new sizes.
    a.editor.hold_place();
    let mut s = a.win.settings.borrow_mut();
    let z = if by == 0.0 { 1.0 } else { s.zoom() + by };
    s.set_zoom(z);
    crate::appearance::apply(&s);
    if let Err(e) = s.save(&crate::appearance::settings_path()) {
        gtk::glib::g_warning!("oscyl", "cannot save settings: {e}");
    }
    a.list.reveal_selected();
    a.win.zoom_toast(s.zoom());
}

impl App {
    pub(crate) fn install_actions(&self) {
        let application = self.win.application();
        for a in ACTIONS {
            let action = match a.run {
                Run::Do(f) => {
                    let action = gio::SimpleAction::new(a.name, None);
                    let run = self.weak(move |me, ()| f(me));
                    action.connect_activate(move |_, _| run(()));
                    action
                }
                // Activating a stateful action without an activate handler
                // flips its state, which lands here.
                Run::Toggle(f) => {
                    let action = gio::SimpleAction::new_stateful(a.name, None, &false.to_variant());
                    let run = self.weak(move |me, on| f(me, on));
                    action.connect_change_state(move |_, v| {
                        if let Some(on) = v.and_then(|v| v.get::<bool>()) {
                            run(on);
                        }
                    });
                    action
                }
            };
            self.win.add_action(&action);
            if let Some(app) = &application {
                app.set_accels_for_action(&format!("win.{}", a.name), a.accels);
            }
        }
        self.update_actions();
    }
}
