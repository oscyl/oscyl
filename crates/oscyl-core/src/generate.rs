//! Driver-independent parts of generation: dealing a hand of distinct
//! programs from any driver's generate, vary or breed.

use oscyl_driver::{Driver, DriverError};

/// SplitMix64, the same stream drivers seed from.
struct Rng(u64);

impl Rng {
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
}

/// A dealt program, and the seed that makes it again when one does.
pub type Dealt = (Vec<u8>, Option<u64>);

/// Deal `n` programs that differ in at least one field and carry
/// different names. `make` is called with a fresh seed derived from
/// `seed` until each program differs from those before it; a maker that
/// keeps repeating itself is nudged apart with [`Driver::vary`]. Names
/// come from [`Driver::generated_name`] for `character`. Deterministic.
pub fn hand(
    d: &dyn Driver,
    character: usize,
    seed: u64,
    n: usize,
    make: impl FnMut(u64) -> Result<Vec<u8>, DriverError>,
) -> Result<Vec<Vec<u8>>, DriverError> {
    let dealt = deal(d, character, seed, (u64::MAX, None, false), n, make)?;
    Ok(dealt.into_iter().map(|(p, _)| p).collect())
}

/// [`hand`], telling for each program the seed `make` made it from, so a
/// caller can make it again. For that, a program whose own name
/// ([`Driver::generated_name`] of its seed) is taken is made from another
/// seed, not renamed; `None` for the rare one that had to be renamed or
/// nudged apart all the same. Seeds are cut to `mask` (short seeds make
/// short recipe codes); the first program is made from `first` when given.
pub fn hand_seeded(
    d: &dyn Driver,
    character: usize,
    seed: u64,
    mask: u64,
    first: Option<u64>,
    n: usize,
    make: impl FnMut(u64) -> Result<Vec<u8>, DriverError>,
) -> Result<Vec<Dealt>, DriverError> {
    deal(d, character, seed, (mask, first, true), n, make)
}

/// Both hands: `(mask, first, own_names)`, the last for [`hand_seeded`]'s
/// rule on names.
fn deal(
    d: &dyn Driver,
    character: usize,
    seed: u64,
    (mask, first, own_names): (u64, Option<u64>, bool),
    n: usize,
    mut make: impl FnMut(u64) -> Result<Vec<u8>, DriverError>,
) -> Result<Vec<Dealt>, DriverError> {
    let fields = d.fields().len();
    let differs = |out: &[Dealt], p: &[u8]| {
        out.iter()
            .all(|(q, _)| (0..fields).any(|i| d.get(q, i) != d.get(p, i)))
    };
    let mut r = Rng(seed);
    let mut out: Vec<Dealt> = Vec::with_capacity(n);
    while out.len() < n {
        let mut s = r.next_u64() & mask;
        if let (true, Some(first)) = (out.is_empty(), first) {
            s = first;
        }
        let mut p = make(s)?;
        let mut tries = 0;
        let taken =
            |out: &[Dealt], name: &str| out.iter().any(|(q, _)| d.name(q).is_ok_and(|m| m == name));
        let own = |s: u64| d.generated_name(character, s);
        while !differs(&out, &p) || (own_names && tries < 8 && taken(&out, &own(s))) {
            s = r.next_u64() & mask;
            p = if tries < 8 {
                make(s)?
            } else {
                d.vary(&p, 0.2, s)?
            };
            tries += 1;
        }
        let name = (0..64)
            .map(|k| own(s.wrapping_add(k)))
            .find(|name| !taken(&out, name))
            .unwrap_or_else(|| {
                let title = d.characters().get(character).map_or("Sound", |c| c.title);
                format!("{title} {}", out.len() + 1)
            });
        d.set_name(&mut p, &name)?;
        out.push((p, (tries <= 8 && name == own(s)).then_some(s)));
    }
    Ok(out)
}
