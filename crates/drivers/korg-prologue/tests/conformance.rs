#[test]
fn prologue_conforms() {
    oscyl_conformance::run(&oscyl_korg_prologue::DRIVER);
}

/// Each driver answers only its own synth's identity reply: family
/// `4B 01` (prologue) and `51 01` (minilogue xd).
#[test]
fn identity_is_prologue_only() {
    use oscyl_driver::Driver;
    use oscyl_driver::sysex::DEVICE_INQUIRY;
    let prologue = oscyl_korg_prologue::DRIVER
        .simulator()
        .handle(&DEVICE_INQUIRY);
    let xd = oscyl_minilogue_xd::DRIVER
        .simulator()
        .handle(&DEVICE_INQUIRY);
    assert_eq!(prologue[0][5..8], [0x42, 0x4B, 0x01]);
    assert!(oscyl_korg_prologue::DRIVER.recognize(&prologue[0]));
    assert!(!oscyl_korg_prologue::DRIVER.recognize(&xd[0]));
    assert!(oscyl_minilogue_xd::DRIVER.recognize(&xd[0]));
    assert!(!oscyl_minilogue_xd::DRIVER.recognize(&prologue[0]));
}
