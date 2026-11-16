//! Oscyl's oscillator engine unit (`units/oscyl-engine`), its C compiled
//! for the computer: what a patch sounds like on the synth, for tests.
//! Nothing ships it.
//!
//! Its tests hold the engine to account: every family's oscillators sound
//! right, the Rust port (`oscyl_logue::engine::render`) matches the C, a
//! bad patch plays a saw, and the committed payloads are built from the
//! sources as they are.

unsafe extern "C" {
    fn oscyl_host_render(
        patch: *const u8,
        size: u32,
        pitch: u16,
        knobs: *const u16,
        out: *mut f32,
        frames: u32,
    );
}

/// `frames` samples of the patch region `patch`, from a note's start at
/// `pitch` (`note << 8 | fine`), with `knobs` the unit's six parameters
/// (0-100), Shape and Shift-Shape (0-1023).
pub fn render(patch: &[u8], pitch: u16, knobs: [u16; 8], frames: usize) -> Vec<f32> {
    let mut out = vec![0.0; frames];
    // SAFETY: the C reads at most `size` bytes of `patch` and eight knobs,
    // and writes `frames` floats into `out`.
    unsafe {
        oscyl_host_render(
            patch.as_ptr(),
            u32::try_from(patch.len()).expect("a patch region"),
            pitch,
            knobs.as_ptr(),
            out.as_mut_ptr(),
            u32::try_from(frames).expect("a short render"),
        );
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use oscyl_logue::engine::{self, Family, Patch, Play, render::probes};

    fn c(p: &Patch, play: &Play, frames: usize) -> Vec<f32> {
        let [a, b, c, d, e, f] = play.params;
        render(
            &p.to_bytes(),
            play.pitch,
            [a, b, c, d, e, f, play.shape, play.shift_shape],
            frames,
        )
    }

    fn peak(s: &[f32]) -> f32 {
        s.iter().fold(0.0, |m, x| m.max(x.abs()))
    }

    fn rms(s: &[f32]) -> f32 {
        (s.iter().map(|x| x * x).sum::<f32>() / s.len() as f32).sqrt()
    }

    #[test]
    fn every_family_sounds_right_on_the_unit() {
        let probes = probes();
        for family in Family::ALL {
            for seed in 0..200 {
                let p = engine::generate(family, seed);
                let what = format!("{family:?} {seed} {}", p.name);
                for play in &probes {
                    let s = c(&p, play, 4096);
                    let body = &s[1024..];
                    let mean = body.iter().sum::<f32>() / body.len() as f32;
                    assert!(rms(body) > 0.02, "{what}: silent at {play:?}");
                    assert!(peak(&s) <= 0.95, "{what}: clips ({}) at {play:?}", peak(&s));
                    assert!(mean.abs() < 0.05, "{what}: DC {mean} at {play:?}");
                    assert!(peak(&s) >= 0.04, "{what}: too quiet at {play:?}");
                }
            }
        }
    }

    #[test]
    fn the_rust_port_plays_what_the_unit_plays() {
        for family in Family::ALL {
            for seed in 0..6 {
                let p = engine::generate(family, seed);
                for play in probes() {
                    let (rust, c) = (engine::render(&p, &play, 2048), c(&p, &play, 2048));
                    let worst = rust
                        .iter()
                        .zip(&c)
                        .map(|(a, b)| (a - b).abs())
                        .fold(0.0, f32::max);
                    assert!(worst < 1e-3, "{family:?} {seed}: apart by {worst}");
                }
            }
        }
    }

    #[test]
    fn a_bad_or_empty_patch_plays_a_saw() {
        let note = 60 << 8;
        let saw = |region: &[u8]| {
            let s = render(region, note, [50; 8], 4096);
            assert!(rms(&s[1024..]) > 0.1 && peak(&s) <= 0.8, "a plain saw");
            s
        };
        // As built: the marker, then nothing.
        let mut region = vec![0u8; 4608];
        region[..8].copy_from_slice(b"OSCYLPAT");
        let plain = saw(&region);
        // Garbage, a broken check sum, a patch with nothing to sound.
        assert_eq!(saw(&[0x5A; 4608]), plain);
        let mut p = engine::generate(Family::Fm, 1).to_bytes();
        p[40] ^= 0xFF;
        p.resize(4608, 0);
        assert_eq!(saw(&p), plain);
        let mut quiet = engine::generate(Family::Fold, 1);
        quiet.blocks.retain(|b| !b.kind.is_source());
        quiet.mods.clear();
        assert_eq!(saw(&quiet.to_bytes()), plain);
        assert_eq!(
            engine::render(&quiet, &Play::at(60, 50.0 / 100.0), 4096),
            plain,
            "the port too"
        );
    }

    /// POSIX `cksum`: CRC-32 (0x04C11DB7, most significant bit first) of
    /// the bytes and then their length.
    fn cksum(bytes: &[u8]) -> String {
        fn step(c: u32, b: u8) -> u32 {
            (0..8).fold(c ^ (u32::from(b) << 24), |c, _| {
                if c & 0x8000_0000 != 0 {
                    (c << 1) ^ 0x04C1_1DB7
                } else {
                    c << 1
                }
            })
        }
        let mut c = bytes.iter().fold(0, |c, b| step(c, *b));
        let mut n = bytes.len();
        while n > 0 {
            c = step(c, n as u8);
            n >>= 8;
        }
        format!("{} {}", !c, bytes.len())
    }

    #[test]
    fn the_committed_payloads_are_built_from_these_sources() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        // As scripts/build-units.sh lists them.
        let sources = [
            "engine.h",
            "engine.c",
            "unit.c",
            "osc_api.h",
            "sdk/userosc.h",
            "sdk/_unit.c",
            "sdk/ld/userosc.ld",
            "sdk/ld/rules.ld",
            "sdk/minilogue-xd/userprg.h",
            "sdk/prologue/userprg.h",
        ]
        .map(|f| format!("units/oscyl-engine/{f}"));
        let mut all = vec![];
        for f in sources.iter().map(String::as_str).chain(["scripts/build-units.sh"]) {
            all.extend(std::fs::read(root.join(f)).unwrap());
        }
        let committed =
            std::fs::read_to_string(root.join("units/oscyl-engine/payload/SOURCE")).unwrap();
        assert_eq!(
            committed.trim(),
            cksum(&all),
            "a unit's source changed since its payloads were built: run scripts/build-units.sh"
        );
    }
}
