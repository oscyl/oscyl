use assert_cmd::Command;

fn oscyl() -> Command {
    Command::cargo_bin("oscyl").unwrap()
}

#[test]
fn help_lists_subcommands() {
    let out = oscyl()
        .arg("--help")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(out).unwrap();
    for cmd in [
        "ports", "info", "current", "backup", "diff", "restore", "audition", "units",
    ] {
        assert!(text.contains(cmd), "missing {cmd}");
    }
}

#[test]
fn restore_of_missing_file_fails_before_touching_the_synth() {
    let out = oscyl()
        .args(["restore", "missing.mnlgxdlib"])
        .assert()
        .failure()
        .get_output()
        .stderr
        .clone();
    assert!(
        String::from_utf8(out)
            .unwrap()
            .contains("cannot open missing.mnlgxdlib")
    );
}

#[test]
fn simulator_info() {
    let out = oscyl()
        .args(["--simulator", "info"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(out).unwrap();
    assert!(text.contains("simulated"), "{text}");
    assert!(text.contains("firmware: 2.10"), "{text}");
}

#[test]
fn simulator_rev2_info() {
    let out = oscyl()
        .args(["--simulator", "--synth", "sequential-prophet-rev2", "info"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(out).unwrap();
    assert!(text.contains("Prophet Rev2"), "{text}");
}

#[test]
fn units_are_refused_on_a_rev2() {
    let out = oscyl()
        .args([
            "--simulator",
            "--synth",
            "sequential-prophet-rev2",
            "units",
            "list",
        ])
        .assert()
        .failure()
        .get_output()
        .stderr
        .clone();
    assert!(String::from_utf8(out).unwrap().contains("Prophet Rev2"));
}

#[test]
fn simulator_from_env_lists_units() {
    let out = oscyl()
        .env("OSCYL_SIMULATOR", "1")
        .args(["units", "list"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(out).unwrap();
    assert_eq!(text.lines().count(), 48, "{text}");
    assert!(
        text.contains("waves") && text.contains("MorphTbl"),
        "{text}"
    );
}

#[test]
fn prologue_units_list_through_the_logue_path() {
    let out = oscyl()
        .args(["--simulator", "--synth", "korg-prologue", "units", "list"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    assert_eq!(String::from_utf8(out).unwrap().lines().count(), 48);
}
