//! UniFFI's bindings generator, built from this crate so its version
//! matches the library's (`generate --library <lib> --language kotlin`).

fn main() {
    uniffi::uniffi_bindgen_main()
}
