//! What a program row offers on right click, long press, or its keys:
//! copy, paste over, copy to a library, revert, and clear to an empty
//! program. The synth's slots are fixed, so there "delete" clears a slot
//! to the init program; in one of Oscyl's libraries it removes the program,
//! and programs move up and down. Every change is one undo step, and the
//! toast after a clear or a removal offers Undo.

use adw::prelude::*;
use gtk::{gio, glib};

use crate::app::App;
use crate::i18n::tr;
use crate::sidebar::Place;
use crate::state::{Program, Reorder};
use oscyl_app::program_menu::{Action, Candidate, Context, copy_targets, enabled, sections};

/// The libraries a program can be copied to: index and name of each of
/// Oscyl's own (but the one shown), then New Library, as targets of
/// `action` with `target(library)` (-1 for a new one).
pub fn library_targets(
    libraries: &[(usize, String)],
    action: &str,
    target: impl Fn(i32) -> glib::Variant,
) -> gio::Menu {
    let menu = gio::Menu::new();
    let names = gio::Menu::new();
    for (j, name) in libraries {
        let it = gio::MenuItem::new(Some(name), None);
        it.set_action_and_target_value(Some(action), Some(&target(*j as i32)));
        names.append_item(&it);
    }
    menu.append_section(None, &names);
    let new = gio::MenuItem::new(Some(&tr("New Library")), None);
    new.set_action_and_target_value(Some(action), Some(&target(-1)));
    menu.append_item(&new);
    menu
}

/// The menu for program `i`, with its actions targeted at `i`. `libraries`
/// are the places it can be copied to; `in_library` adds new programs and
/// removing (the synth clears instead).
pub fn model(i: usize, libraries: &[(usize, String)], in_library: bool) -> gio::Menu {
    let target = (i as i32).to_variant();
    let item = |label: String, action: &str| {
        let it = gio::MenuItem::new(Some(&label), None);
        it.set_action_and_target_value(Some(action), Some(&target));
        it
    };
    let menu = gio::Menu::new();
    for actions in sections(in_library) {
        let section = gio::Menu::new();
        for a in actions {
            match a {
                Action::CopyToLibrary => {
                    let to = library_targets(libraries, "win.program-to-library", |j| {
                        (i as i32, j).to_variant()
                    });
                    section.append_submenu(Some(&tr("Copy to Another Library")), &to);
                }
                _ => section.append_item(&item(label(a), &format!("win.{}", name(a)))),
            }
        }
        menu.append_section(None, &section);
    }
    menu
}

/// An action's menu label.
fn label(a: Action) -> String {
    match a {
        Action::Copy => tr("Copy Sound"),
        Action::PasteOver => tr("Paste Sound Here"),
        Action::CopyToLibrary => tr("Copy to Another Library"),
        Action::NewProgram => tr("New Program"),
        Action::MoveUp | Action::SwapUp => tr("Move Up"),
        Action::MoveDown | Action::SwapDown => tr("Move Down"),
        Action::Revert => tr("Revert Changes"),
        Action::Remove => tr("Remove from Library"),
        Action::Clear => tr("Clear to Init Program"),
    }
}

/// An action's window action.
fn name(a: Action) -> &'static str {
    match a {
        Action::Copy => "program-copy",
        Action::PasteOver => "program-paste",
        Action::CopyToLibrary => "program-to-library",
        Action::NewProgram => "program-new",
        Action::MoveUp => "program-move-up",
        Action::MoveDown => "program-move-down",
        Action::SwapUp => "program-swap-up",
        Action::SwapDown => "program-swap-down",
        Action::Revert => "program-revert",
        Action::Remove => "program-remove",
        Action::Clear => "program-clear",
    }
}

impl App {
    pub(crate) fn install_program_actions(&self) {
        let add = |name: &str, run: fn(&App, usize)| {
            let action = gio::SimpleAction::new(name, Some(glib::VariantTy::INT32));
            let run = self.weak(move |me, i: usize| run(me, i));
            action.connect_activate(move |_, v| {
                if let Some(i) = v
                    .and_then(|v| v.get::<i32>())
                    .and_then(|i| usize::try_from(i).ok())
                {
                    run(i);
                }
            });
            self.win.add_action(&action);
        };
        add("program-copy", App::copy_program);
        add("program-paste", App::paste_program);
        add("program-revert", App::revert_program_at);
        add("program-clear", App::clear_program);
        add("program-remove", App::remove_program);
        add("program-delete", |me, i| {
            if me.managed_place().is_some() {
                me.remove_program(i);
            } else {
                me.clear_program(i);
            }
        });
        add("program-new", |me, i| me.new_program(Some(i)));
        add("program-move-up", |me, i| {
            me.move_program(i, -1, Reorder::Move)
        });
        add("program-move-down", |me, i| {
            me.move_program(i, 1, Reorder::Move)
        });
        add("program-swap-up", |me, i| {
            me.move_program(i, -1, Reorder::Swap)
        });
        add("program-swap-down", |me, i| {
            me.move_program(i, 1, Reorder::Swap)
        });
        let to = gio::SimpleAction::new(
            "program-to-library",
            Some(glib::VariantTy::new("(ii)").expect("valid type")),
        );
        let run = self.weak(|me, (i, j): (i32, i32)| {
            if let Ok(i) = usize::try_from(i) {
                me.copy_to_library(i, usize::try_from(j).ok());
            }
        });
        to.connect_activate(move |_, v| {
            if let Some(t) = v.and_then(|v| v.get::<(i32, i32)>()) {
                run(t);
            }
        });
        self.win.add_action(&to);
    }

    /// Enable the row actions that make sense for program `i` now.
    pub(crate) fn update_program_actions(&self, i: usize) {
        let clipboard = self.clipboard.borrow().as_ref().map(|(id, _)| *id);
        let in_library = self.managed_place().is_some();
        let cx = Context {
            in_library,
            can_reorder: self.can_reorder().is_ok(),
            read: self.is_loaded(i),
            clipboard,
        };
        let on = |a| self.with_doc(|d| enabled(d, i, &cx, a)).unwrap_or(false);
        let set = |name: &str, on: bool| {
            if let Some(a) = self
                .win
                .lookup_action(name)
                .and_downcast::<gio::SimpleAction>()
            {
                a.set_enabled(on);
            }
        };
        for a in [
            Action::Copy,
            Action::PasteOver,
            Action::CopyToLibrary,
            Action::MoveUp,
            Action::MoveDown,
            Action::SwapUp,
            Action::SwapDown,
            Action::Revert,
            Action::Remove,
            Action::Clear,
        ] {
            set(name(a), on(a));
        }
        set(
            "program-delete",
            on(if in_library {
                Action::Remove
            } else {
                Action::Clear
            }),
        );
    }

    /// Program `i`'s menu, offering the libraries it can be copied to.
    pub(crate) fn program_menu(&self, i: usize) -> gio::Menu {
        model(
            i,
            &self.copy_targets(self.managed_place()),
            self.managed_place().is_some(),
        )
    }

    /// Oscyl's libraries a program can go to: all readable ones of the
    /// synth shown but `except`.
    pub(crate) fn copy_targets(&self, except: Option<usize>) -> Vec<(usize, String)> {
        // Asked first: the document shown may be a library, and the list below
        // borrows them all (a borrow held across both used to panic).
        let driver = self.doc_driver().info().id;
        let candidates = self
            .libs
            .borrow()
            .iter()
            .enumerate()
            .map(|(j, l)| Candidate {
                key: j,
                name: l.doc.title.clone(),
                driver: l.doc.driver.info().id.to_string(),
                managed: l.is_managed(),
                broken: l.broken.is_some(),
            })
            .collect::<Vec<_>>();
        copy_targets(candidates, except.as_ref(), driver)
    }

    /// Add an Init Program to the library shown, right after program
    /// `after` (at the end when none), show it and put its name up for
    /// editing. At 500 programs, the bell and a toast.
    pub(crate) fn new_program(&self, after: Option<usize>) {
        if self.managed_place().is_none() || self.can_reorder().is_err() {
            return;
        }
        let Some(at) = self.with_doc(|d| d.new_program(after)).flatten() else {
            let max = self.with_doc(|d| d.max_programs()).unwrap_or(0);
            self.win.window.error_bell();
            self.win.toast(
                &tr("This library is full: a library holds up to {max} programs.")
                    .replace("{max}", &max.to_string()),
            );
            return;
        };
        let first = self.with_doc(|d| d.len() == 1).unwrap_or(false);
        if first {
            // The empty-library page gives way to the list and editor.
            let place = self.doc_place.borrow().clone();
            self.refresh_sidebar();
            self.go(place);
        } else {
            self.after_edit();
        }
        self.list.inserted(at);
        self.show_program(at);
        self.list.reveal_selected();
        self.win.inner.set_show_content(true);
        self.editor.start_rename();
    }

    /// Remove program `i` from the library shown, with Undo.
    fn remove_program(&self, i: usize) {
        let Some(name) = self
            .with_doc(|d| (i < d.len()).then(|| d.name(i)))
            .flatten()
        else {
            return;
        };
        self.with_doc(|d| d.remove(i));
        self.after_edit();
        self.list.removed(i);
        let toast = adw::Toast::builder()
            .title(tr("Removed “{program}”").replace("{program}", &name))
            .button_label(tr("Undo"))
            .action_name("win.undo")
            .build();
        self.win.show_toast(toast);
    }

    /// Programs can be put in another order here, or why not: one of
    /// Oscyl's libraries, or the synth once all of it has been read.
    pub(crate) fn can_reorder(&self) -> Result<(), String> {
        match *self.doc_place.borrow() {
            Place::Synth if self.synth_fully_read() => Ok(()),
            Place::Synth => Err(tr("Wait until all programs have been read from the synth.")),
            Place::Library(i)
                if self
                    .libs
                    .borrow()
                    .get(i)
                    .is_some_and(|l| l.is_managed() && l.broken.is_none()) =>
            {
                Ok(())
            }
            _ => Err(tr("Keep this library to put its programs in order")),
        }
    }

    /// Move program `i` one place up (-1) or down (1), or swap it with
    /// that neighbour.
    fn move_program(&self, i: usize, by: isize, how: Reorder) {
        let len = self.with_doc(|d| d.len()).unwrap_or(0);
        let Some(to) = i.checked_add_signed(by).filter(|&to| to < len) else {
            self.win.window.error_bell();
            return;
        };
        if self.can_reorder().is_err() {
            self.win.window.error_bell();
            return;
        }
        self.reorder_program(i, to, how);
        self.list.moved(i, to, how);
    }

    /// Put program `from` at `to` as one undo step. On the synth this
    /// only edits the slots in memory: nothing is written until Sync.
    pub(crate) fn reorder_program(&self, from: usize, to: usize, how: Reorder) {
        self.with_doc(|d| d.reorder(from, to, how));
        self.after_edit();
        self.show_program(to);
    }

    fn copy_program(&self, i: usize) {
        let Some((d, p)) = self
            .with_doc(|d| Some((d.driver, d.work(i)?.to_vec())))
            .flatten()
        else {
            return;
        };
        let name = d.name(&p).unwrap_or_default();
        *self.clipboard.borrow_mut() = Some((d.info().id, p));
        self.win
            .toast(&tr("Copied “{program}”").replace("{program}", &name));
    }

    fn paste_program(&self, i: usize) {
        let Some((id, p)) = self.clipboard.borrow().clone() else {
            return;
        };
        if id != self.doc_driver().info().id {
            return;
        }
        self.replace_program(
            i,
            p,
            &tr("Pasted over slot {slot}").replace("{slot}", &self.slot_label(i)),
        );
    }

    fn revert_program_at(&self, i: usize) {
        self.with_doc(|d| d.revert_program(i));
        self.after_edit();
    }

    fn clear_program(&self, i: usize) {
        self.replace_program(
            i,
            self.doc_driver().init_program(),
            &tr("Slot {slot} cleared").replace("{slot}", &self.slot_label(i)),
        );
    }

    /// Replace program `i` as one undo step and say so, with Undo.
    /// Roll stage `page` of the program shown (the dice): its fields only,
    /// one undo step the history calls "Rolled Filter". Refused on a
    /// factory slot, a slot not read yet, and while comparing.
    pub(crate) fn roll_stage(&self, page: usize) {
        let Some(i) = self.current.get().filter(|&i| self.is_loaded(i)) else {
            return;
        };
        if self.editor.comparing() || self.with_doc(|d| d.is_read_only(i)).unwrap_or(true) {
            self.win.window.error_bell();
            return;
        }
        let d = self.doc_driver();
        let title = d
            .curation()
            .pages
            .get(page)
            .map(|p| tr(p.title))
            .unwrap_or_default();
        let Some(p) = self
            .with_doc(|doc| doc.work(i).map(<[u8]>::to_vec))
            .flatten()
        else {
            return;
        };
        let seed = (u64::from(glib::random_int()) << 32) | u64::from(glib::random_int());
        match oscyl_app::generator::roll_page(d, &p, page, seed) {
            Ok(rolled) => {
                self.replace_program(i, rolled, &oscyl_app::generator::rolled_label(&title))
            }
            Err(e) => self.win.toast(&e.to_string()),
        }
    }

    pub(crate) fn replace_program(&self, i: usize, p: Program, message: &str) {
        if !self.is_loaded(i) {
            return;
        }
        // The history calls the step by the toast's words.
        self.with_doc(|d| d.set_program_named(i, p, message));
        self.after_edit();
        let toast = adw::Toast::builder()
            .title(message)
            .button_label(tr("Undo"))
            .action_name("win.undo")
            .build();
        self.win.show_toast(toast);
    }
}
