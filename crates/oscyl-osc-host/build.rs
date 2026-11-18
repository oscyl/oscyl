//! Compile the unit's DSP C (not its logue SDK glue) for the computer.

fn main() {
    let unit = "../../units/oscyl-osc";
    for f in ["dsp.c", "dsp.h"] {
        println!("cargo:rerun-if-changed={unit}/{f}");
    }
    println!("cargo:rerun-if-changed=src/host.c");
    cc::Build::new()
        .std("c11")
        .include(unit)
        .file(format!("{unit}/dsp.c"))
        .file("src/host.c")
        // Plain multiply then add, as the Rust port does: Apple's clang
        // fuses them by default, which drifts FM and feedback apart.
        .flag_if_supported("-ffp-contract=off")
        .warnings(true)
        .compile("oscyl_osc");
}
