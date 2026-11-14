//! The edit history, from the header's History button: three snapshots of
//! the program open (A, B, C, kept in memory until another program opens),
//! then every step of the document in the core's words, newest first, down
//! to "As saved". A click on a step goes there, so many undos or redos as
//! one move; Store and Recall keep and put back a whole program (a recall
//! is one undo step, "Recalled snapshot A").

use adw::prelude::*;

use crate::app::App;
use crate::i18n::tr;
use crate::sidebar::Place;
use crate::state::Program;

const LETTERS: [&str; 3] = ["A", "B", "C"];

/// The snapshots of one program: whose they are, and the three slots.
#[derive(Default)]
pub struct Snapshots {
    key: Option<(Place, usize)>,
    slots: [Option<Program>; 3],
}

impl App {
    /// The program open, when it can take a snapshot or a recall: read,
    /// writable, and not being compared.
    fn snapshot_target(&self) -> Option<usize> {
        let i = self.current.get().filter(|&i| self.is_loaded(i))?;
        let writable = !self.with_doc(|d| d.is_read_only(i)).unwrap_or(true);
        writable.then_some(i)
    }

    /// The snapshots, emptied first when they belong to another program.
    fn snapshots_of(&self, i: usize) -> std::cell::RefMut<'_, Snapshots> {
        let key = Some((self.doc_place.borrow().clone(), i));
        let mut s = self.snapshots.borrow_mut();
        if s.key != key {
            *s = Snapshots {
                key,
                ..Snapshots::default()
            };
        }
        s
    }

    /// Another program opened: its snapshots start empty.
    pub(crate) fn forget_snapshots(&self) {
        *self.snapshots.borrow_mut() = Snapshots::default();
    }

    fn store_snapshot(&self, k: usize) {
        let Some(i) = self.snapshot_target() else {
            return;
        };
        let Some(p) = self.with_doc(|d| d.work(i).map(<[u8]>::to_vec)).flatten() else {
            return;
        };
        if self.snapshots_of(i).slots[k].replace(p).is_some() {
            self.win
                .toast(&tr("Snapshot {letter} replaced").replace("{letter}", LETTERS[k]));
        }
    }

    /// Design captures: snapshot A of the program open.
    pub(crate) fn store_demo_snapshot(&self) {
        self.store_snapshot(0);
    }

    fn recall_snapshot(&self, k: usize) {
        let Some(i) = self.snapshot_target() else {
            return;
        };
        if self.editor.comparing() {
            self.win.window.error_bell();
            return;
        }
        let Some(p) = self.snapshots_of(i).slots[k].clone() else {
            return;
        };
        let d = self.doc_driver();
        if let Err(e) = d.check_program(&p) {
            self.win.toast(&e.to_string());
            return;
        }
        let label = oscyl_app::review::recalled_label(LETTERS[k]);
        self.with_doc(|doc| doc.set_program_named(i, p, &label));
        self.after_edit();
    }

    /// Fill the History popover's `body` for the document shown now.
    pub(crate) fn fill_history(&self, body: &gtk::Box) {
        while let Some(c) = body.first_child() {
            body.remove(&c);
        }
        let refill = {
            let body = body.clone();
            self.weak(move |me, ()| me.fill_history(&body))
        };
        let refill = std::rc::Rc::new(refill);

        // Snapshots, for a program open, read and writable.
        if let Some(i) = self.snapshot_target() {
            body.append(&legend(&tr("Snapshots")));
            let grid = gtk::Grid::builder()
                .column_spacing(8)
                .row_spacing(6)
                .css_classes(["snapshots"])
                .build();
            let stored: Vec<bool> = self
                .snapshots_of(i)
                .slots
                .iter()
                .map(Option::is_some)
                .collect();
            for (k, letter) in LETTERS.iter().enumerate() {
                let name = tr("Snapshot {letter}").replace("{letter}", letter);
                let plate = gtk::Label::builder()
                    .label(*letter)
                    .css_classes(["snapshot-plate"])
                    .build();
                if stored[k] {
                    plate.add_css_class("stored");
                }
                let state = gtk::Label::builder()
                    .label(if stored[k] {
                        tr("Stored")
                    } else {
                        tr("Not stored")
                    })
                    .xalign(0.0)
                    .hexpand(true)
                    .css_classes(["dim-label"])
                    .build();
                let store = gtk::Button::builder()
                    .label(tr("Store"))
                    .tooltip_text(tr("Keep the program as it is now"))
                    .build();
                store.update_property(&[gtk::accessible::Property::Label(
                    &tr("Store {snapshot}").replace("{snapshot}", &name),
                )]);
                let recall = gtk::Button::builder()
                    .label(tr("Recall"))
                    .sensitive(stored[k])
                    .tooltip_text(tr("Put it back, as one step you can undo"))
                    .build();
                recall.update_property(&[gtk::accessible::Property::Label(
                    &tr("Recall {snapshot}").replace("{snapshot}", &name),
                )]);
                {
                    let refill = refill.clone();
                    let run = self.weak(move |me, ()| {
                        me.store_snapshot(k);
                        refill(());
                    });
                    store.connect_clicked(move |_| run(()));
                }
                {
                    let refill = refill.clone();
                    let run = self.weak(move |me, ()| {
                        me.recall_snapshot(k);
                        refill(());
                    });
                    recall.connect_clicked(move |_| run(()));
                }
                let row = k as i32;
                grid.attach(&plate, 0, row, 1, 1);
                grid.attach(&state, 1, row, 1, 1);
                grid.attach(&store, 2, row, 1, 1);
                grid.attach(&recall, 3, row, 1, 1);
            }
            body.append(&grid);
        }

        // The steps, newest first, down to the saved state.
        body.append(&legend(&tr("Steps")));
        let steps = self
            .with_doc(|d| {
                oscyl_app::review::history(d)
                    .into_iter()
                    .map(|h| (oscyl_app::slots::slot_label(d.driver, h.index), h))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        if steps.is_empty() {
            let empty = adw::StatusPage::builder()
                .title(tr("No edits yet"))
                .description(tr(
                    "Every change to these programs is listed here; a click goes back to it.",
                ))
                .css_classes(["compact", "history-empty"])
                .build();
            body.append(&empty);
            return;
        }
        let done = steps.iter().filter(|(_, h)| h.done).count();
        let list = gtk::ListBox::builder()
            .selection_mode(gtk::SelectionMode::None)
            .css_classes(["history"])
            .build();
        list.update_property(&[gtk::accessible::Property::Label(&tr("Steps"))]);
        // Row `r` (newest first) leaves the document after `targets[r]` steps.
        let mut targets = Vec::new();
        for (at, (slot, h)) in steps.iter().enumerate().rev() {
            list.append(&step_row(slot, &h.title, at + 1 == done, !h.done));
            targets.push(at + 1);
        }
        list.append(&step_row("", &tr("As saved"), done == 0, false));
        targets.push(0);
        let jump = self.weak(move |me, to: usize| {
            me.jump_history(to);
            refill(());
        });
        list.connect_row_activated(move |_, row| {
            if let Some(&to) = usize::try_from(row.index())
                .ok()
                .and_then(|r| targets.get(r))
            {
                jump(to);
            }
        });
        let scroll = gtk::ScrolledWindow::builder()
            .child(&list)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .propagate_natural_height(true)
            .max_content_height(360)
            .build();
        body.append(&scroll);
    }

    /// Undo or redo until `done` steps of the history are done.
    fn jump_history(&self, done: usize) {
        self.step(move |d| d.jump(done));
    }
}

fn legend(text: &str) -> gtk::Label {
    let l = gtk::Label::builder().label(text).xalign(0.0).build();
    l.add_css_class("section-heading");
    l.set_accessible_role(gtk::AccessibleRole::Heading);
    l
}

/// One step: the index line where the document stands, the slot, the words;
/// a step undone waits in `legend`.
fn step_row(slot: &str, title: &str, now: bool, undone: bool) -> gtk::ListBoxRow {
    let mark = gtk::Box::builder()
        .css_classes(["history-mark"])
        .opacity(if now { 1.0 } else { 0.0 })
        .build();
    let slot_label = gtk::Label::builder()
        .label(slot)
        .width_chars(3)
        .xalign(1.0)
        .css_classes(["slot-number"])
        .build();
    let text = gtk::Label::builder()
        .label(title)
        .xalign(0.0)
        .hexpand(true)
        .ellipsize(gtk::pango::EllipsizeMode::End)
        .max_width_chars(36)
        .build();
    let row_box = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    row_box.append(&mark);
    row_box.append(&slot_label);
    row_box.append(&text);
    let row = gtk::ListBoxRow::builder().child(&row_box).build();
    if undone {
        row.add_css_class("undone");
    }
    let mut said = if slot.is_empty() {
        title.to_string()
    } else {
        format!("{slot} {title}")
    };
    if now {
        said = format!("{said}, {}", tr("Now"));
    } else if undone {
        said = format!("{said}, {}", tr("Undone"));
    }
    row.update_property(&[gtk::accessible::Property::Label(&said)]);
    row.set_tooltip_text(Some(title));
    row
}
