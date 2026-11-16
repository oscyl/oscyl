//! The synth side of the app: hotplug, connecting, worker events and the
//! sidebar's status card, for every connected synth.

use gtk::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

use gtk::glib;
use oscyl_midi::hotplug::{Change, PortWatcher};

use crate::app::{App, Synth, reattach_index, synth_name};
use crate::i18n::{tr, trn};
use crate::sidebar::{Place, SynthStatus};
use crate::slots::slots;
use crate::state::Doc;
use crate::synth::{Cmd, DeviceId, Ev};

impl App {
    /// Follow synths being plugged in and out (ALSA announcements,
    /// watched from the main loop). Plugging one in connects it;
    /// unplugging drops it at once instead of waiting for a request to
    /// fail.
    pub(crate) fn watch_hotplug(&self) {
        // Simulated synths connect at once; real ones as they are found.
        if self.win.settings.borrow().simulator() {
            self.connect_synth();
        }
        let watcher = match PortWatcher::new() {
            Ok(w) => Rc::new(RefCell::new(w)),
            Err(e) => {
                glib::g_warning!("oscyl", "cannot watch MIDI ports: {e}");
                return;
            }
        };
        let present = !watcher.borrow().candidates().is_empty();
        if present && !self.win.settings.borrow().simulator() {
            self.connect_synth();
        }
        for fd in watcher.borrow().fds() {
            let watcher = watcher.clone();
            let on_change = self.weak(move |me, ()| {
                let changes = watcher.borrow_mut().drain_changes();
                if me.win.settings.borrow().simulator() {
                    return;
                }
                for change in changes {
                    match change {
                        Change::Appeared(c) => {
                            me.win.toast(
                                &tr("{model} plugged in").replace("{model}", c.driver.info().model),
                            );
                            me.connecting.set(true);
                            me.refresh_status();
                            me.synth.send(Cmd::Open(c));
                        }
                        Change::Gone(c) => {
                            let known = me.synths.borrow().iter().any(|s| s.port == c.port);
                            if known {
                                me.win.toast(
                                    &tr("{model} unplugged. Your edits are kept.")
                                        .replace("{model}", c.driver.info().model),
                                );
                                me.synth.send(Cmd::Gone(c.port));
                            }
                        }
                    }
                }
            });
            glib_unix::unix_fd_add_local(fd, glib::IOCondition::IN, move |_, _| {
                on_change(());
                glib::ControlFlow::Continue
            });
        }
    }

    /// Connect every synth found (or, in simulated mode, a simulated synth
    /// of every kind Oscyl knows).
    pub(crate) fn connect_synth(&self) {
        self.connecting.set(true);
        self.refresh_status();
        let simulated = self.win.settings.borrow().simulator();
        self.synth.send(Cmd::Connect { simulated });
    }

    /// Disconnect the synth the sidebar shows.
    pub(crate) fn disconnect_synth(&self) {
        if let Some(id) = self.active.get() {
            self.synth.send(Cmd::Disconnect(id));
        }
    }

    /// Show synth `id` in the sidebar: its programs become Place::Synth.
    pub(crate) fn set_active(&self, id: DeviceId) {
        if self.active.get() == Some(id) {
            return;
        }
        self.active.set(Some(id));
        self.refresh_status();
        self.show_simulated();
        let place = self.place.borrow().clone();
        if *self.doc_place.borrow() == Place::Synth || place == Place::Units {
            self.current.set(None);
            self.go(place);
        }
        self.refresh_generator();
        self.update_actions();
    }

    /// The status card for the active synth, and the picker of synths.
    pub(crate) fn refresh_status(&self) {
        let synths = self.synths.borrow();
        let names: Vec<(DeviceId, String)> =
            synths.iter().map(|s| (s.id, s.name.clone())).collect();
        let active = synths.iter().find(|s| Some(s.id) == self.active.get());
        let (title, status) = match active {
            Some(s) => (s.name.clone(), s.status.clone()),
            None if self.connecting.get() => (tr("Synth"), SynthStatus::Connecting),
            None => (tr("Synth"), SynthStatus::Disconnected),
        };
        let units = match active {
            Some(s) => !s.doc.driver.capabilities().units.is_empty(),
            None => oscyl_midi::DRIVERS
                .iter()
                .any(|d| !d.capabilities().units.is_empty()),
        };
        drop(synths);
        self.sidebar.set_status(&title, &status);
        self.sidebar.set_synths(&names, self.active.get());
        self.sidebar.set_units_shown(units);
        if !units && *self.place.borrow() == Place::Units {
            self.sidebar.select(&Place::Synth);
        }
    }

    /// Set synth `id`'s status, shown when it is the active one.
    fn set_synth_status(&self, id: DeviceId, status: SynthStatus) {
        self.with_synth_id(id, |s| s.status = status);
        if self.active.get() == Some(id) {
            self.refresh_status();
        }
    }

    /// Synth `id`'s status back to Connected after a transfer.
    fn restore_status(&self, id: DeviceId) {
        let status = self.with_synth_id(id, |s| SynthStatus::Connected {
            firmware: s.firmware.clone().unwrap_or_default(),
            simulated: s.simulated,
        });
        if let Some(st) = status {
            self.set_synth_status(id, st);
        }
    }

    /// Position of slot `n` in synth `id`'s document.
    fn position(&self, id: DeviceId, n: u16) -> Option<usize> {
        self.with_synth_id(id, |s| slots(s.doc.driver).iter().position(|&x| x == n))
            .flatten()
    }

    /// A real synth that went away keeps its entry, offline: its programs,
    /// edits and open program stay, ready for it to come back. False for
    /// a simulated synth (it leaves as before) or an unknown id.
    fn go_offline(&self, id: DeviceId) -> bool {
        let kept = self.with_synth_id(id, |s| {
            if s.simulated {
                return false;
            }
            s.online = false;
            s.reading = false;
            // A sync cut short stored nothing it can vouch for.
            s.sync_batch.clear();
            s.sync_backup = None;
            s.status = SynthStatus::Disconnected;
            true
        });
        if kept != Some(true) {
            return false;
        }
        self.refresh_status();
        if self.active.get() == Some(id) && *self.doc_place.borrow() == Place::Synth {
            // Unread rows now say "Not read".
            self.list.refresh();
            self.refresh_program();
        }
        self.refresh_generator();
        self.update_actions();
        true
    }

    /// Show the Synth page. Selecting its row again emits nothing, so a
    /// page already chosen (showing "No synth connected") is shown afresh.
    pub(crate) fn show_synth_page(&self) {
        if *self.place.borrow() == Place::Synth {
            self.go(Place::Synth);
        } else {
            self.sidebar.select(&Place::Synth);
        }
    }

    /// Synth `id` connected: when an offline entry of its model is
    /// waiting, it becomes that entry again (programs and edits kept) and
    /// is read afresh. False when there is none.
    fn reattach(
        &self,
        id: DeviceId,
        driver: &'static dyn oscyl_driver::Driver,
        port: &str,
        firmware: Option<String>,
        simulated: bool,
    ) -> bool {
        let old = {
            let mut synths = self.synths.borrow_mut();
            let entries: Vec<(bool, bool, &str, &str)> = synths
                .iter()
                .map(|s| {
                    (
                        s.online,
                        s.simulated,
                        s.doc.driver.info().id,
                        s.port.as_str(),
                    )
                })
                .collect();
            let Some(k) = reattach_index(&entries, driver.info().id, port, simulated) else {
                return false;
            };
            let s = &mut synths[k];
            let old = s.id;
            s.id = id;
            s.online = true;
            s.reading = true;
            s.port = port.to_string();
            s.status = SynthStatus::Connected {
                firmware: firmware.clone().unwrap_or_default(),
                simulated,
            };
            s.firmware = firmware;
            old
        };
        if self.active.get() == Some(old) {
            self.active.set(Some(id));
        }
        self.synth.send(Cmd::ReadAll(id));
        self.refresh_status();
        self.refresh_generator();
        self.update_actions();
        true
    }

    pub(crate) fn on_synth(&self, ev: Ev) {
        match ev {
            Ev::Connected {
                id,
                driver,
                port,
                firmware,
                simulated,
            } => {
                self.connecting.set(false);
                if self.reattach(id, driver, &port, firmware.clone(), simulated) {
                    return;
                }
                let taken: Vec<String> = self
                    .synths
                    .borrow()
                    .iter()
                    .map(|s| s.name.clone())
                    .collect();
                let name = synth_name(driver.info().model, &taken);
                let n = slots(driver).len();
                let status = SynthStatus::Connected {
                    firmware: firmware.clone().unwrap_or_default(),
                    simulated,
                };
                self.synths.borrow_mut().push(Synth {
                    id,
                    online: true,
                    name: name.clone(),
                    firmware,
                    simulated,
                    port,
                    doc: Doc::synth(driver, vec![driver.init_program(); n], &name),
                    loaded: vec![false; n],
                    reading: true,
                    status,
                    slots: Vec::new(),
                    sync_backup: None,
                    sync_batch: Vec::new(),
                });
                self.synth.send(Cmd::ReadAll(id));
                // The first synth shows at once; later ones wait in the
                // picker, so the view never jumps away from the work.
                if self.active.get().is_none() {
                    self.active.set(Some(id));
                    self.show_simulated();
                    self.refresh_status();
                    self.refresh_generator();
                    self.update_actions();
                    self.show_synth_page();
                } else {
                    self.refresh_status();
                    self.update_actions();
                }
            }
            Ev::ConnectFailed(e) => {
                self.connecting.set(false);
                self.refresh_status();
                self.error_dialog(&tr("Couldn't connect to the synth"), &e);
            }
            Ev::Disconnected(id) => {
                if self.go_offline(id) {
                    return;
                }
                let gone = {
                    let mut synths = self.synths.borrow_mut();
                    let k = synths.iter().position(|s| s.id == id);
                    k.map(|k| synths.remove(k))
                };
                // Edits not synced stay, as a library with no file yet:
                // shown, copyable, and named when quitting.
                let kept = gone.and_then(|s| {
                    let title = tr("{synth} (disconnected)").replace("{synth}", &s.name);
                    s.doc.offline(&title)
                });
                let showing =
                    self.active.get() == Some(id) && *self.doc_place.borrow() == Place::Synth;
                if let Some(doc) = kept {
                    self.libs
                        .borrow_mut()
                        .push(crate::libraries::Lib::file(doc));
                    self.refresh_sidebar();
                    if showing {
                        let i = self.libs.borrow().len() - 1;
                        self.active.set(self.synths.borrow().first().map(|s| s.id));
                        self.current.set(None);
                        self.sidebar.select(&Place::Library(i));
                    }
                }
                if self.active.get() == Some(id) {
                    let next = self.synths.borrow().first().map(|s| s.id);
                    self.active.set(next);
                    let place = self.place.borrow().clone();
                    if *self.doc_place.borrow() == Place::Synth || place == Place::Units {
                        self.current.set(None);
                        self.go(place);
                    }
                }
                self.refresh_status();
                self.show_simulated();
                self.refresh_generator();
                self.update_actions();
            }
            Ev::ReadProgress { id, done, total } => {
                self.set_synth_status(id, SynthStatus::Reading { done, total });
            }
            Ev::Program(id, n, p) => {
                let Some(i) = self.position(id, n) else {
                    return;
                };
                self.with_synth_id(id, |s| {
                    s.doc.set_base(i, p);
                    if let Some(l) = s.loaded.get_mut(i) {
                        *l = true;
                    }
                });
                if self.active.get() == Some(id) && *self.doc_place.borrow() == Place::Synth {
                    self.list.refresh_one(i);
                    if self.current.get() == Some(i) {
                        self.refresh_program();
                    }
                }
            }
            Ev::ReadDone { id, cancelled } => {
                self.with_synth_id(id, |s| s.reading = false);
                self.restore_status(id);
                if cancelled {
                    self.win.toast(&tr(
                        "Reading stopped. Programs not yet read show as “Not read”.",
                    ));
                    if self.active.get() == Some(id) && *self.doc_place.borrow() == Place::Synth {
                        self.list.refresh();
                        self.refresh_program();
                    }
                }
                self.update_actions();
            }
            Ev::WriteProgress { id, done, total } => {
                self.set_synth_status(id, SynthStatus::Writing { done, total });
            }
            Ev::Written { id, indices } => {
                let pairs: Vec<(usize, u16)> = indices
                    .iter()
                    .filter_map(|&n| Some((self.position(id, n)?, n)))
                    .collect();
                let idx: Vec<usize> = pairs.iter().map(|(i, _)| *i).collect();
                // What was written becomes each slot's saved version; an edit
                // made while the write ran stays an edit.
                self.with_synth_id(id, |s| {
                    let batch = std::mem::take(&mut s.sync_batch);
                    let stored: Vec<_> = batch
                        .into_iter()
                        .filter(|(slot, _)| indices.contains(slot))
                        .collect();
                    s.doc.stored(&stored);
                });
                self.restore_status(id);
                let toast = adw::Toast::new(&trn(
                    "Synced %d program",
                    "Synced %d programs",
                    idx.len() as u32,
                ));
                if let Some(path) = self.with_synth_id(id, |s| s.sync_backup.take()).flatten() {
                    self.offer_open(&toast, path);
                }
                self.win.toasts.add_toast(toast);
                self.after_edit();
            }
            Ev::WriteFailed(id, e) => {
                // Nothing was synced: the backup taken for it is not this
                // sync's any more.
                self.with_synth_id(id, |s| {
                    s.sync_backup = None;
                    s.sync_batch.clear();
                });
                self.restore_status(id);
                self.error_dialog(
                    &tr("Couldn't sync"),
                    &format!(
                        "{e}\n\n{}",
                        tr("Your edits are kept. Check the cable and sync again.")
                    ),
                );
                self.update_actions();
            }
            // Save Playing Sound: the edit buffer as a library of its own.
            Ev::Current(id, data) => self.keep_playing(id, data),
            Ev::Slots(id, slots) => {
                self.with_synth_id(id, |s| s.slots = slots.clone());
                if self.active.get() == Some(id) && *self.place.borrow() == Place::Units {
                    crate::units::show(&self.editor, &slots, self.weak(|me, a| me.slot_action(a)));
                }
            }
            Ev::Error(id, e) => {
                if let Some(id) = id {
                    self.restore_status(id);
                }
                self.win.toast(&e.replace('\n', " "));
            }
        }
    }

    /// Any synth is connected.
    pub(crate) fn connected(&self) -> bool {
        self.active.get().is_some()
    }

    /// The synth that plays what is shown is simulated (it makes no
    /// sound).
    pub(crate) fn simulated(&self) -> bool {
        self.audition_target()
            .and_then(|id| self.with_synth_id(id, |s| s.simulated))
            .unwrap_or(false)
    }

    /// Say on the audition button when there is nothing to hear: against
    /// a simulated synth its lamp stays a ring and never blinks.
    pub(crate) fn show_simulated(&self) {
        let Some(p) = self.primary.borrow().as_ref().map(|p| p.audition.clone()) else {
            return;
        };
        if self.simulated() {
            p.add_css_class("silent");
            p.set_tooltip_text(Some(&tr(
                "The simulated synth makes no sound: audition needs a real synth (Ctrl+Shift+A)",
            )));
        } else {
            p.remove_css_class("silent");
            p.set_tooltip_text(Some(&tr(
                "Audition live: play every change on the synth without storing it (Ctrl+Shift+A)",
            )));
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::app::{reattach_index, synth_name, target_of};

    const XD: &str = "korg-minilogue-xd";

    /// A synth that comes back takes the entry it left on the same port;
    /// on another port (re-plugged elsewhere), the first of its model.
    #[test]
    fn reattach_prefers_same_port() {
        let entries = [
            (false, false, XD, "xd A 28:1"),
            (false, false, XD, "xd B 32:1"),
        ];
        assert_eq!(reattach_index(&entries, XD, "xd B 32:1", false), Some(1));
        assert_eq!(reattach_index(&entries, XD, "xd C 40:1", false), Some(0));
    }

    /// Only an offline real synth of the same model is taken; a simulated
    /// connection never takes one.
    #[test]
    fn reattach_skips_online_simulated_and_other_models() {
        let entries = [
            (true, false, XD, "xd A 28:1"),
            (false, false, "sequential-prophet-rev2", "rev2 36:0"),
            (false, true, XD, ""),
        ];
        assert_eq!(reattach_index(&entries, XD, "xd A 28:1", false), None);
        assert_eq!(
            reattach_index(&[(false, false, XD, "xd")], XD, "", true),
            None
        );
    }

    #[test]
    fn send_targets_a_synth_of_the_library_driver() {
        let synths = [
            (1, "korg-minilogue-xd"),
            (2, "sequential-prophet-rev2"),
            (3, "korg-minilogue-xd"),
        ];
        let xd = "korg-minilogue-xd";
        assert_eq!(target_of(&synths, Some(3), xd), Some(3), "the active one");
        assert_eq!(
            target_of(&synths, Some(2), xd),
            Some(1),
            "else the first of the model"
        );
        assert_eq!(target_of(&synths, None, "sequential-prophet-rev2"), Some(2));
        assert_eq!(target_of(&synths, Some(1), "korg-prologue"), None);
    }

    #[test]
    fn same_model_names_are_numbered() {
        let taken = |v: &[&str]| v.iter().map(ToString::to_string).collect::<Vec<_>>();
        assert_eq!(synth_name("minilogue xd", &[]), "minilogue xd");
        assert_eq!(
            synth_name("minilogue xd", &taken(&["minilogue xd"])),
            "minilogue xd 2"
        );
        assert_eq!(
            synth_name("minilogue xd", &taken(&["minilogue xd 2", "Prophet Rev2"])),
            "minilogue xd"
        );
    }
}
