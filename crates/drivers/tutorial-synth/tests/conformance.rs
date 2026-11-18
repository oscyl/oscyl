//! The checks every driver passes against its simulator.

#[test]
fn tutorial_mono_conforms() {
    oscyl_conformance::run(&oscyl_tutorial_synth::driver::DRIVER);
}
