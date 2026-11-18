//! Program bytes from a file or a synth parse or fail cleanly.
#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = oscyl_korg_minilogue_xd::Program::from_bytes(data);
    let _ = oscyl_korg_prologue::Program::from_bytes(data);
    let _ = oscyl_sequential_prophet_rev2::Program::from_bytes(data);
});
