//! Compile the engine's C (not the unit's logue SDK glue) for the computer.

fn main() {
    let unit = "../../units/oscyl-engine";
    for f in ["engine.c", "engine.h"] {
        println!("cargo:rerun-if-changed={unit}/{f}");
    }
    println!("cargo:rerun-if-changed=src/host.c");
    cc::Build::new()
        .std("c11")
        .include(unit)
        .file(format!("{unit}/engine.c"))
        .file("src/host.c")
        // Plain multiply then add, as the Rust port does: Apple's clang
        // fuses them by default, which drifts FM and feedback apart.
        .flag_if_supported("-ffp-contract=off")
        .warnings(true)
        .compile("oscyl_engine");
}
