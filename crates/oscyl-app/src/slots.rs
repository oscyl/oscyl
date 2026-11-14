//! A synth's program slots, from its banks: where a library's programs go
//! and how slots are named. GTK-free.

use oscyl_driver::Driver;

/// Every slot, bank by bank: a synth document's program `i` is slot
/// `slots(d)[i]`.
pub fn slots(d: &dyn Driver) -> Vec<u16> {
    d.capabilities()
        .banks
        .iter()
        .flat_map(|b| b.first..b.first + b.len)
        .collect()
}

/// The slots Oscyl may write, bank by bank. Program `i` of a library goes
/// to `writable(d)[i]`.
pub fn writable(d: &dyn Driver) -> Vec<u16> {
    d.capabilities()
        .banks
        .iter()
        .filter(|b| b.writable)
        .flat_map(|b| b.first..b.first + b.len)
        .collect()
}

/// The most programs a library holds: the synth's writable slots, so
/// program n of a library can go to slot n.
pub fn library_max(d: &dyn Driver) -> usize {
    writable(d).len()
}

/// Program `i`'s slot as shown and searched: "001" on a synth with one
/// bank, "U2 001" with several.
pub fn slot_label(d: &dyn Driver, i: usize) -> String {
    let banks = d.capabilities().banks;
    let mut at = i;
    for b in banks {
        if at < usize::from(b.len) {
            return if banks.len() == 1 {
                format!("{:03}", at + 1)
            } else {
                format!("{} {:03}", b.name, at + 1)
            };
        }
        at -= usize::from(b.len);
    }
    format!("{:03}", i + 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn driver(id: &str) -> &'static dyn Driver {
        oscyl_midi::driver(id).unwrap()
    }

    #[test]
    fn library_max_is_the_writable_slot_count() {
        assert_eq!(library_max(driver("korg-minilogue-xd")), 500);
        assert_eq!(library_max(driver("sequential-prophet-rev2")), 512);
        assert_eq!(slots(driver("sequential-prophet-rev2")).len(), 1024);
    }

    #[test]
    fn slot_labels_follow_banks() {
        let xd = driver("korg-minilogue-xd");
        assert_eq!(slot_label(xd, 0), "001");
        assert_eq!(slot_label(xd, 499), "500");
        let rev2 = driver("sequential-prophet-rev2");
        assert_eq!(slot_label(rev2, 0), "U1 001");
        assert_eq!(slot_label(rev2, 128), "U2 001");
        assert_eq!(slot_label(rev2, 1023), "F4 128");
    }
}
