#[test]
fn prophet_rev2_conforms() {
    oscyl_conformance::run(&oscyl_sequential_prophet_rev2::DRIVER);
}
