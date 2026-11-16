//! User oscillator and effect slots: list, load and clear.
//!
//! User units are a logue-family capability the driver contract does not
//! model (`Capabilities::units` only lists them), so this page speaks the
//! logue SDK's unit types (`oscyl_midi::logue`). The sidebar shows it only
//! for a synth with units.

use std::rc::Rc;

use adw::prelude::*;
use oscyl_midi::logue::{Module, unit_ext};

use crate::app::App;
use crate::editor::Editor;
use crate::files::{Pick, backup_path, filters, text};
use crate::i18n::tr;
use crate::synth::{Cmd, DeviceId, SlotInfo};
use oscyl_app::shelf::{Shelf, ShelfUnit};

pub use oscyl_app::units::module_title;

/// What a row of the page asks the app to do.
pub enum SlotAction {
    Load(Module, u8),
    Clear(Module, u8, String),
    /// Pick a unit file for the shelf.
    ShelfAdd,
    /// Load the shelf's file (and the unit's name) into the first free
    /// slot of its section.
    ShelfLoad(String, String),
    /// Take the file (and the unit's name) off the shelf.
    ShelfRemove(String, String),
}

/// A row titled `title` (and `subtitle`): one that opens onto the unit's
/// parameters, each with its range, when it declares any; a plain row
/// otherwise. `suffixes` go at its end.
fn unit_row(
    title: &str,
    subtitle: &str,
    params: &[(String, String)],
    suffixes: &[gtk::Widget],
) -> adw::PreferencesRow {
    if params.is_empty() {
        let row = adw::ActionRow::builder()
            .title(title)
            .subtitle(subtitle)
            .build();
        for s in suffixes {
            row.add_suffix(s);
        }
        return row.upcast();
    }
    let row = adw::ExpanderRow::builder()
        .title(title)
        .subtitle(subtitle)
        .build();
    row.set_tooltip_text(Some(&tr("Show its parameters")));
    for s in suffixes {
        row.add_suffix(s);
    }
    for (name, range) in params {
        let p = adw::ActionRow::builder()
            .title(name)
            .subtitle(range)
            .build();
        p.add_css_class("property");
        row.add_row(&p);
    }
    row.upcast()
}

/// The shelf, then the 48 slots as grouped lists, in the editor area.
pub fn show(
    editor: &Editor,
    slots: &[SlotInfo],
    shelf: &[ShelfUnit],
    act: impl Fn(SlotAction) + 'static,
) {
    let act = Rc::new(act);
    let page = adw::PreferencesPage::new();
    page.set_title(&tr("Oscillators and effects"));
    let intro = adw::PreferencesGroup::builder()
        .title(tr("Oscillators and effects"))
        .description(tr("Custom oscillators and effects made with the logue SDK live in these slots. Loading replaces only the chosen slot."))
        .build();
    page.add(&intro);

    // The shelf: units kept by Oscyl, one click from a free slot.
    let kept = adw::PreferencesGroup::builder()
        .title(tr("Shelf"))
        .description(tr("Unit files Oscyl keeps for this synth. Load puts one into the first free slot of its section."))
        .build();
    let add = gtk::Button::builder()
        .label(tr("Add…"))
        .valign(gtk::Align::Center)
        .tooltip_text(tr("Keep a unit file on the shelf"))
        .build();
    let a = act.clone();
    add.connect_clicked(move |_| a(SlotAction::ShelfAdd));
    kept.set_header_suffix(Some(&add));
    if shelf.is_empty() {
        let row = adw::ActionRow::builder()
            .title(tr("Nothing on the shelf yet"))
            .subtitle(tr("Add a unit file to keep it here."))
            .build();
        row.add_css_class("dim-label");
        kept.add(&row);
    }
    for u in shelf {
        let name = &u.info.name;
        let load = gtk::Button::builder()
            .label(tr("Load"))
            .valign(gtk::Align::Center)
            .tooltip_text(tr("Load into the first free slot of its section"))
            .build();
        load.update_property(&[gtk::accessible::Property::Label(
            &tr("Load {name} into the first free slot of {section}")
                .replace("{name}", name)
                .replace("{section}", &u.section),
        )]);
        let (a, file, n) = (act.clone(), u.file.clone(), name.clone());
        load.connect_clicked(move |_| a(SlotAction::ShelfLoad(file.clone(), n.clone())));
        let remove = gtk::Button::builder()
            .icon_name("user-trash-symbolic")
            .valign(gtk::Align::Center)
            .tooltip_text(tr("Take it off the shelf"))
            .css_classes(["flat"])
            .build();
        remove.update_property(&[gtk::accessible::Property::Label(
            &tr("Take {name} off the shelf").replace("{name}", name),
        )]);
        let (a, file, n) = (act.clone(), u.file.clone(), name.clone());
        remove.connect_clicked(move |_| a(SlotAction::ShelfRemove(file.clone(), n.clone())));
        kept.add(&unit_row(
            name,
            &u.section,
            &oscyl_app::palette::param_words(&u.info),
            &[load.upcast(), remove.upcast()],
        ));
    }
    page.add(&kept);

    for module in Module::ALL {
        let used = slots
            .iter()
            .filter(|s| s.module == module && s.name.is_some())
            .count();
        let group = adw::PreferencesGroup::builder()
            .title(module_title(module))
            .description(format!("{used} / {}", module.slots()))
            .build();
        for s in slots.iter().filter(|s| s.module == module) {
            let number = format!("{:02}", s.slot + 1);
            let n = gtk::Label::new(Some(&number));
            n.add_css_class("slot-number");
            n.add_css_class("dim-label");
            let (module, slot) = (s.module, s.slot);
            let row: adw::PreferencesRow = match &s.name {
                Some(name) => {
                    let b = gtk::Button::builder()
                        .icon_name("user-trash-symbolic")
                        .valign(gtk::Align::Center)
                        .tooltip_text(tr("Clear this slot"))
                        .build();
                    b.add_css_class("flat");
                    b.update_property(&[gtk::accessible::Property::Label(
                        &tr("Clear {name}").replace("{name}", name),
                    )]);
                    let (act, cleared) = (act.clone(), name.clone());
                    b.connect_clicked(move |_| {
                        act(SlotAction::Clear(module, slot, cleared.clone()))
                    });
                    let params = s
                        .info
                        .as_ref()
                        .map(oscyl_app::palette::param_words)
                        .unwrap_or_default();
                    unit_row(name, "", &params, &[b.upcast()])
                }
                None => {
                    let b = gtk::Button::builder()
                        .label(tr("Load…"))
                        .valign(gtk::Align::Center)
                        .build();
                    b.update_property(&[gtk::accessible::Property::Label(
                        &tr("Load unit into {section}, slot {number}")
                            .replace("{section}", &module_title(module))
                            .replace("{number}", &number),
                    )]);
                    let act = act.clone();
                    b.connect_clicked(move |_| act(SlotAction::Load(module, slot)));
                    let row = unit_row(&tr("Empty"), "", &[], &[b.upcast()]);
                    row.add_css_class("dim-label");
                    row
                }
            };
            // The slot's number leads either kind of row.
            if let Some(r) = row.downcast_ref::<adw::ActionRow>() {
                r.add_prefix(&n);
            } else if let Some(r) = row.downcast_ref::<adw::ExpanderRow>() {
                r.add_prefix(&n);
            }
            group.add(&row);
        }
        page.add(&group);
    }
    editor.show_page("units", &page);
}

impl App {
    /// A slot's action, for the synth shown now: the dialog that follows
    /// acts on that synth even if another is picked meanwhile.
    pub(crate) fn slot_action(&self, action: SlotAction) {
        let Some((id, d)) = self.active.get().zip(self.active_driver()) else {
            return;
        };
        if !self.active_online() {
            return;
        }
        match action {
            SlotAction::Load(module, slot) => {
                let ext = unit_ext(d).unwrap_or_default();
                self.pick_file(
                    &tr("Load Unit"),
                    // A unit file, or a slot's backup (what Clear Slot keeps) to put back.
                    &filters(
                        &tr("logue SDK units for {model}").replace("{model}", d.info().model),
                        &[&format!("*.{ext}"), "*.slot.bin"],
                    ),
                    Pick::Open,
                    move |me, path| me.load_unit(id, d, module, slot, &path),
                )
            }
            SlotAction::Clear(module, slot, name) => self.confirm_clear(id, module, slot, name),
            SlotAction::ShelfAdd => {
                let ext = unit_ext(d).unwrap_or_default();
                self.pick_file(
                    &tr("Add to Shelf"),
                    &filters(
                        &tr("logue SDK units for {model}").replace("{model}", d.info().model),
                        &[&format!("*.{ext}")],
                    ),
                    Pick::Open,
                    move |me, path| {
                        let added = std::fs::read(&path)
                            .map_err(text)
                            .and_then(|bytes| me.shelf().add(d, &bytes));
                        match added {
                            Ok(u) => {
                                me.win.toast(
                                    &tr("“{name}” is on the shelf").replace("{name}", &u.info.name),
                                );
                                me.show_units();
                            }
                            Err(e) => me.error_dialog(&tr("Couldn't read the unit"), &e),
                        }
                    },
                )
            }
            SlotAction::ShelfLoad(file, name) => match self.shelf().unit(d, &file) {
                Ok(u) => {
                    self.win
                        .toast(&tr("Loading “{name}”…").replace("{name}", &name));
                    self.synth.send(Cmd::LoadUnit {
                        id,
                        module: u.module,
                        data: u.data,
                    });
                }
                Err(e) => self.error_dialog(&tr("Couldn't read the unit"), &e),
            },
            SlotAction::ShelfRemove(file, name) => match self.shelf().remove(d, &file) {
                Ok(()) => {
                    self.win
                        .toast(&tr("“{name}” is off the shelf").replace("{name}", &name));
                    self.show_units();
                }
                Err(e) => self.error_dialog(&tr("Couldn't remove the unit"), &e),
            },
        }
    }

    /// Oscyl's shelf of unit files, in the app's data.
    pub(crate) fn shelf(&self) -> Shelf {
        Shelf::at(&gtk::glib::user_data_dir().join("oscyl").join("units"))
    }

    /// The units page from the active synth's last listing, with the shelf.
    pub(crate) fn show_units(&self) {
        let (Some(d), Some(slots)) = (self.active_driver(), self.with_synth(|s| s.slots.clone()))
        else {
            return;
        };
        let shelf = self.shelf().list(d);
        show(
            &self.editor,
            &slots,
            &shelf,
            self.weak(|me, a| me.slot_action(a)),
        );
    }

    fn load_unit(
        &self,
        id: DeviceId,
        d: &'static dyn oscyl_driver::Driver,
        module: Module,
        slot: u8,
        path: &std::path::Path,
    ) {
        let unit = std::fs::read(path)
            .map_err(text)
            .and_then(|bytes| oscyl_app::units::read_unit(d, &bytes, module));
        match unit {
            Ok(u) => {
                self.win
                    .toast(&tr("Loading “{name}”…").replace("{name}", &u.name));
                self.synth.send(Cmd::UploadUnit {
                    id,
                    module,
                    slot,
                    data: u.data,
                });
            }
            Err(e) => self.error_dialog(&tr("Couldn't read the unit"), &e),
        }
    }

    fn confirm_clear(&self, id: DeviceId, module: Module, slot: u8, name: String) {
        let dialog = adw::AlertDialog::new(
            Some(&tr("Clear “{name}”?").replace("{name}", &name)),
            Some(&tr(
                "The slot is emptied on the synth. Oscyl saves a backup of the unit first: Load Unit puts it back.",
            )),
        );
        dialog.add_response("cancel", &tr("Cancel"));
        dialog.add_response("clear", &tr("Clear Slot"));
        dialog.set_response_appearance("clear", adw::ResponseAppearance::Destructive);
        dialog.set_close_response("cancel");
        let clear = self.weak(move |me, ()| {
            me.synth.send(Cmd::ClearSlot {
                id,
                module,
                slot,
                backup: backup_path(&name, "slot.bin"),
            });
        });
        dialog.connect_response(None, move |_, r| {
            if r == "clear" {
                clear(());
            }
        });
        dialog.present(Some(&self.win.window));
    }
}
