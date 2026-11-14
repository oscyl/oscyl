//! A message from a synth, however malformed, decodes or fails cleanly.
#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = oscyl_minilogue_xd::msg::decode(data);
    let _ = oscyl_prophet_rev2::msg::decode(data);
});
