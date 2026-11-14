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
    mut make: impl FnMut(u64) -> Result<Vec<u8>, DriverError>,
) -> Result<Vec<Vec<u8>>, DriverError> {
    let fields = d.fields().len();
    let differs = |out: &[Vec<u8>], p: &[u8]| {
        out.iter()
            .all(|q| (0..fields).any(|i| d.get(q, i) != d.get(p, i)))
    };
    let mut r = Rng(seed);
    let mut out: Vec<Vec<u8>> = Vec::with_capacity(n);
    while out.len() < n {
        let mut s = r.next_u64();
        let mut p = make(s)?;
        let mut tries = 0;
        while !differs(&out, &p) {
            s = r.next_u64();
            p = if tries < 8 {
                make(s)?
            } else {
                d.vary(&p, 0.2, s)?
            };
            tries += 1;
        }
        let taken = |name: &str| out.iter().any(|q| d.name(q).is_ok_and(|m| m == name));
        let name = (0..64)
            .map(|k| d.generated_name(character, s.wrapping_add(k)))
            .find(|name| !taken(name))
            .unwrap_or_else(|| {
                let title = d.characters().get(character).map_or("Sound", |c| c.title);
                format!("{title} {}", out.len() + 1)
            });
        d.set_name(&mut p, &name)?;
        out.push(p);
    }
    Ok(out)
}
