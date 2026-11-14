//! The in-app guide (F1): short, practical chapters.

use adw::prelude::*;
use oscyl_driver::Driver;

use crate::i18n::tr;
use crate::window::OscylWindow;

struct Chapter {
    title: &'static str,
    body: &'static str,
}

const CHAPTERS: &[Chapter] = &[
    Chapter {
        title: "Connecting",
        body: "Plug your synths into this computer with USB cables and switch them on, then press Connect. \
               Oscyl reads every program in the background while you keep working. \
               With several synths connected, pick the one to show and sync with at the top of the sidebar. \
               No synth? Turn on the simulated synths in Preferences to try reading, syncing and saving. \
               They answer like real ones but make no sound, so auditioning needs a real synth.",
    },
    Chapter {
        title: "Working offline",
        body: "Open your synth's own library and program files, edit them anywhere, and save. \
               Oscyl keeps everything it does not edit exactly as it was, so files stay compatible.",
    },
    Chapter {
        title: "Editing",
        body: "Pick a program. The band across the top shows the synth's pages, from the voice to the effects. \
               Click a page to open its large graph and its controls. \
               Drag the handles on the graph: the filter peak, the envelope points, the mixer bars. \
               Drag a dial up or down, scroll over it, or focus it and use the arrow keys; hold Shift for fine steps, \
               and double-click to put it back. Ctrl+I enlarges the graph. \
               Red ticks mark what you changed: the program in the list, the stage on the band and each line under Changes. \
               An edited dial shows what it was.",
    },
    Chapter {
        title: "Auditioning",
        body: "Turn on Audition live (Ctrl+Shift+A) and every change plays on the synth instantly, without storing anything. \
               Play the keys to hear it. Nothing is written to the synth's memory until you sync.",
    },
    Chapter {
        title: "Generating sounds",
        body: "Generate (Ctrl+G) makes new sounds for you. Pick a character, from power chords and fuzz bass to bells and space effects, \
               and shape it with four dials: Bright opens the filter, Movement adds the LFO and modulation, \
               Grit adds drive and noise, Ambience adds delay and reverb. Press Generate (or Space) for six sounds, \
               and pick one, or move with the arrow keys, to hear it on the synth. \
               Keep puts it in the slot selected in the list, as an edit you can undo and sync like any other. \
               Vary deals six variations of a sound, Breed With crosses it with another, and Alt+Left goes back to an earlier hand. \
               To give one of your sounds a new filter or new effects, select it and choose which of its stages to keep.",
    },
    Chapter {
        title: "Syncing",
        body: "Sync shows exactly which slots will change before it writes, \
               writes only those, and confirms each one is saved. Back up the whole synth any time with Ctrl+B. \
               Undo (Ctrl+Z) works on edits; to reverse a sync, undo and sync again.",
    },
    Chapter {
        title: "Managing units",
        body: "Synths of the logue family have slots for user oscillators and effects made with the logue SDK. \
               Oscyl lists what is loaded in each slot.",
    },
];

/// Replace the editor area with the guide; `driver` names the editor's
/// pages.
pub fn show(win: &OscylWindow, driver: &dyn Driver) {
    let page = adw::PreferencesPage::new();
    // A page title, then one list of chapters (the first one open).
    let chapters = adw::PreferencesGroup::builder()
        .title(tr("Getting started"))
        .build();
    for (i, c) in CHAPTERS.iter().enumerate() {
        let row = adw::ExpanderRow::builder()
            .title(tr(c.title))
            .expanded(i == 0)
            .build();
        let body = gtk::Label::builder()
            .label(tr(c.body))
            .wrap(true)
            .xalign(0.0)
            .selectable(true)
            .margin_top(12)
            .margin_bottom(12)
            .margin_start(12)
            .margin_end(12)
            .build();
        row.add_row(&body);
        chapters.add(&row);
    }
    page.add(&chapters);
    let stages = adw::PreferencesGroup::builder()
        .title(tr("The editor's pages"))
        .build();
    for p in driver.curation().pages {
        let row = adw::ActionRow::builder()
            .title(tr(p.title))
            .subtitle(tr(p.help))
            .subtitle_lines(0)
            .build();
        stages.add(&row);
    }
    page.add(&stages);
    let keys = adw::PreferencesGroup::new();
    let shortcuts = adw::ButtonRow::builder()
        .title(tr("Keyboard Shortcuts"))
        .action_name("win.shortcuts")
        .build();
    keys.add(&shortcuts);
    page.add(&keys);
    win.editor_page.set_title(&tr("Guide"));
    if let Some(view) = win.editor_page.child().and_downcast::<adw::ToolbarView>()
        && let Some(stack) = view.content().and_downcast::<gtk::Stack>()
    {
        crate::editor::set_page(&stack, "guide", &page);
    }
    win.inner.set_show_content(true);
}
