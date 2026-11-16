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
        .warnings(true)
        .compile("oscyl_engine");
}
