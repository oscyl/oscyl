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

pub use oscyl_app::units::module_title;

/// What a slot row asks the app to do.
pub enum SlotAction {
    Load(Module, u8),
    Clear(Module, u8, String),
}

/// Show the 48 slots as grouped lists in the editor area.
pub fn show(editor: &Editor, slots: &[SlotInfo], act: impl Fn(SlotAction) + 'static) {
    let act = Rc::new(act);
    let page = adw::PreferencesPage::new();
    page.set_title(&tr("Oscillators and effects"));
    let intro = adw::PreferencesGroup::builder()
        .title(tr("Oscillators and effects"))
        .description(tr("Custom oscillators and effects made with the logue SDK live in these slots. Loading replaces only the chosen slot."))
        .build();
    page.add(&intro);
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
            let row = adw::ActionRow::builder()
                .title(s.name.clone().unwrap_or_else(|| tr("Empty")))
                .build();
            let n = gtk::Label::new(Some(&number));
            n.add_css_class("slot-number");
            n.add_css_class("dim-label");
            row.add_prefix(&n);
            let (module, slot) = (s.module, s.slot);
            match &s.name {
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
                    let (act, name) = (act.clone(), name.clone());
                    b.connect_clicked(move |_| act(SlotAction::Clear(module, slot, name.clone())));
                    row.add_suffix(&b);
                }
                None => {
                    row.add_css_class("dim-label");
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
                    row.add_suffix(&b);
                }
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
        }
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
