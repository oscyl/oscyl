//! User oscillator and effect units (logue family): what a unit file
//! holds and where it may go, and the sections' names. Shared by every
//! app; the worker loads and clears slots (`synth::Cmd`).

use oscyl_driver::Driver;
use oscyl_logue::unit::Unit;
use oscyl_midi::logue::{Module, platform};

use crate::i18n::tr;

/// A section of user units, for people.
pub fn module_title(m: Module) -> String {
    match m {
        Module::Osc => tr("Oscillators"),
        Module::ModFx => tr("Modulation effects"),
        Module::DelFx => tr("Delays"),
        Module::RevFx => tr("Reverbs"),
    }
}

/// A unit file, read: its name, its section and the slot data to load.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnitFile {
    pub name: String,
    pub module: Module,
    pub data: Vec<u8>,
}

/// Unit file `bytes` for synth `d`, ready for a slot of section `module`;
/// why not, in words, when it is not a unit for this synth or belongs in
/// another section.
pub fn read_unit(d: &dyn Driver, bytes: &[u8], module: Module) -> Result<UnitFile, String> {
    let p = platform(d).ok_or_else(|| tr("This synth has no user units."))?;
    // A slot backup (`.slot.bin`, what clearing a slot keeps) goes back as it is.
    if let Some(u) = slot_backup(p.id, bytes) {
        return if u.module != module {
            Err(tr("The unit “{name}” is for {section}.")
                .replace("{name}", &u.name)
                .replace("{section}", &module_title(u.module)))
        } else {
            Ok(u)
        };
    }
    let u = Unit::read(&p, std::io::Cursor::new(bytes)).map_err(|e| e.to_string())?;
    if u.module != module {
        return Err(tr("The unit “{name}” is for {section}.")
            .replace("{name}", &u.name)
            .replace("{section}", &module_title(u.module)));
    }
    Ok(UnitFile {
        data: u.slot_data(),
        name: u.name,
        module: u.module,
    })
}

/// The section a slot backup (`.slot.bin`) of synth `d` goes back into;
/// `None` when `bytes` are not one of `d`'s slot backups.
pub fn backup_section(d: &dyn Driver, bytes: &[u8]) -> Option<Module> {
    slot_backup(platform(d)?.id, bytes).map(|u| u.module)
}

/// Slot data as the synth gave it back (`u32 size, u32 checksum, blob`,
/// the blob's header naming its platform and section), when `bytes` are
/// that and for platform `id`.
fn slot_backup(id: u8, bytes: &[u8]) -> Option<UnitFile> {
    let size = u32::from_le_bytes(bytes.get(0..4)?.try_into().ok()?) as usize;
    let blob = bytes.get(8..)?;
    if size != blob.len() || blob.len() < oscyl_logue::unit::HEADER_LEN || blob[1] != id {
        return None;
    }
    let module = Module::ALL.into_iter().find(|m| *m as u8 == blob[0])?;
    Some(UnitFile {
        name: oscyl_logue::unit::unit_name(blob),
        module,
        // As the official Librarian sends it: checksum 0.
        data: oscyl_logue::unit::slot_data(blob),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_unit_is_read_for_its_own_synth_and_section() {
        let xd = oscyl_midi::driver("korg-minilogue-xd").unwrap();
        let rev2 = oscyl_midi::driver("sequential-prophet-rev2").unwrap();
        assert!(
            read_unit(rev2, b"", Module::Osc).is_err(),
            "no units on a Rev2"
        );
        assert!(read_unit(xd, b"not a zip", Module::Osc).is_err());
    }

    #[test]
    fn a_slot_backup_goes_back_into_its_own_section() {
        let xd = oscyl_midi::driver("korg-minilogue-xd").unwrap();
        // What clearing a slot keeps: size, the synth's checksum, then the blob.
        let mut blob = vec![0u8; oscyl_logue::unit::HEADER_LEN + 16];
        blob[0] = Module::Osc as u8;
        blob[1] = 2;
        blob[18..23].copy_from_slice(b"waves");
        let backup = [&(blob.len() as u32).to_le_bytes()[..], &[9, 9, 9, 9], &blob].concat();
        let u = read_unit(xd, &backup, Module::Osc).unwrap();
        assert_eq!(u.name, "waves");
        assert_eq!(
            u.data,
            oscyl_logue::unit::slot_data(&blob),
            "checksum 0, as uploads send it"
        );
        assert!(
            read_unit(xd, &backup, Module::ModFx).is_err(),
            "another section"
        );
        let prologue = oscyl_midi::driver("korg-prologue").unwrap();
        assert!(
            read_unit(prologue, &backup, Module::Osc).is_err(),
            "another synth's"
        );
    }
}
