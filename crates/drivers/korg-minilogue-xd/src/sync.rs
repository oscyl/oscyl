//! Deciding what to send.

use crate::Program;

/// Indices whose wanted program differs from the device copy, or whose
/// device copy is unknown. Only these need writing.
pub fn plan_write(on_device: &[Option<Program>], wanted: &[Program]) -> Vec<u16> {
    wanted
        .iter()
        .enumerate()
        .filter(|(i, w)| on_device.get(*i).and_then(Option::as_ref) != Some(*w))
        .map(|(i, _)| i as u16)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_changed_or_unknown_programs() {
        let base = Program::init();
        let mut changed = base.clone();
        changed.set_name("Changed").unwrap();
        let mut device = vec![Some(base.clone()); 500];
        let mut wanted = vec![base; 500];
        wanted[17] = changed.clone();
        wanted[499] = changed;
        device[250] = None;
        assert_eq!(plan_write(&device, &wanted), vec![17, 250, 499]);
    }
}
