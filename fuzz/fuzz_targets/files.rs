//! A library or unit file from anywhere opens or fails cleanly.
#![no_main]
use std::io::Cursor;

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = oscyl_core::native::read(Cursor::new(data));
    let _ = oscyl_korg_minilogue_xd::library::Library::read(Cursor::new(data));
    let _ = oscyl_korg_minilogue_xd::unit::Unit::read(Cursor::new(data));
});
