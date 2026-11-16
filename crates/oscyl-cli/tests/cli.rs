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
        "ports", "info", "current", "backup", "diff", "restore", "audition", "generate", "units",
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

/// `oscyl <args>`'s standard output, which must succeed.
fn stdout(args: &[&str]) -> String {
    let out = oscyl()
        .args(args)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    String::from_utf8(out).unwrap()
}

/// `oscyl <args>`'s standard error, which must fail.
fn failure(args: &[&str]) -> String {
    let out = oscyl()
        .args(args)
        .assert()
        .failure()
        .get_output()
        .stderr
        .clone();
    String::from_utf8(out).unwrap()
}

/// Lines of `name  code` as `(name, code)`.
fn sounds(text: &str) -> Vec<(String, String)> {
    text.lines()
        .map(|l| {
            let (name, code) = l.split_at(l.len().min(14));
            (name.trim().to_string(), code.trim().to_string())
        })
        .collect()
}

#[test]
fn generate_lists_characters_per_synth() {
    let xd = stdout(&["generate", "--list"]);
    assert_eq!(xd.lines().count(), 28, "{xd}");
    assert!(xd.contains("fm-glass") && xd.contains("rhythm") && xd.contains("effect"));
    let rev2 = stdout(&["--synth", "sequential-prophet-rev2", "generate", "--list"]);
    assert!(
        rev2.lines().count() >= 10 && rev2.contains("sync-lead"),
        "{rev2}"
    );
}

#[test]
fn generate_deals_named_sounds_with_codes_that_make_them_again() {
    let args = [
        "generate",
        "--character",
        "acid",
        "--bright",
        "80",
        "--space",
        "0",
        "--seed",
        "7",
    ];
    let text = stdout(&args);
    let dealt = sounds(&text);
    assert_eq!(dealt.len(), 6, "{text}");
    assert_eq!(text, stdout(&args), "the same seed deals the same sounds");
    let mut names: Vec<&String> = dealt.iter().map(|(n, _)| n).collect();
    names.sort();
    names.dedup();
    assert_eq!(names.len(), 6, "{text}");
    for (name, code) in &dealt {
        assert!(code.starts_with("acid-"), "{text}");
        // A code alone deals that one sound, name and all.
        assert_eq!(
            sounds(&stdout(&["generate", "--code", code])),
            [(name.clone(), code.clone())]
        );
    }
    // Another seed, other sounds; --count sets how many.
    let other = stdout(&[
        "generate",
        "--character",
        "acid",
        "--seed",
        "8",
        "--count",
        "3",
    ]);
    assert_eq!(other.lines().count(), 3);
    assert!(sounds(&other).iter().all(|s| !dealt.contains(s)));
}

#[test]
fn generate_refuses_what_it_cannot_deal() {
    assert!(failure(&["generate"]).contains("--character"));
    assert!(failure(&["generate", "--character", "nope"]).contains("generate --list"));
    assert!(failure(&["generate", "--code", "acid-nonsense"]).contains("recipe code"));
    assert!(failure(&["generate", "--character", "pad", "--bright", "101"]).contains("101"));
    // A code names a character; another synth may lack it.
    let code = &sounds(&stdout(&[
        "generate",
        "--character",
        "fm-glass",
        "--count",
        "1",
    ]))[0]
        .1;
    let lacks = failure(&["--synth", "korg-prologue", "generate", "--code", code]);
    assert!(lacks.contains("fm-glass"), "{lacks}");
}

#[test]
fn generate_writes_program_files_then_varies_breeds_and_auditions_them() {
    let dir = std::env::temp_dir().join(format!("oscyl-cli-generate-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let out = dir.to_str().unwrap();
    let text = stdout(&[
        "generate",
        "--character",
        "kick",
        "--seed",
        "3",
        "--count",
        "2",
        "--out",
        out,
    ]);
    let files: Vec<String> = sounds(&text)
        .iter()
        .map(|(name, _)| format!("{out}/{name}.mnlgxdprog"))
        .collect();
    assert!(
        files.iter().all(|f| std::path::Path::new(f).is_file()),
        "{files:?}"
    );
    // Variations and children are named after the parent and have no code.
    let varied = stdout(&[
        "generate", "--vary", &files[0], "--seed", "1", "--count", "4",
    ]);
    assert_eq!(varied.lines().count(), 4, "{varied}");
    assert!(
        sounds(&varied)
            .iter()
            .all(|(n, c)| n.ends_with(char::is_numeric) && c.is_empty())
    );
    let bred = stdout(&["generate", "--breed", &files[0], &files[1], "--seed", "1"]);
    assert_eq!(bred.lines().count(), 6, "{bred}");
    // A file is for one synth.
    let wrong = failure(&["--synth", "korg-prologue", "generate", "--vary", &files[0]]);
    assert!(wrong.contains("minilogue xd"), "{wrong}");
    // Audition plays the first sound; here on the simulated synth.
    let played = oscyl()
        .args([
            "--simulator",
            "generate",
            "--character",
            "kick",
            "--seed",
            "3",
            "--audition",
        ])
        .assert()
        .success()
        .get_output()
        .stderr
        .clone();
    let name = &sounds(&text)[0].0;
    assert!(
        String::from_utf8(played)
            .unwrap()
            .contains(&format!("auditioning \"{name}\""))
    );
    // Other synths write their own format.
    let rev2 = dir.join("rev2");
    stdout(&[
        "--synth",
        "sequential-prophet-rev2",
        "generate",
        "--character",
        "bell",
        "--count",
        "1",
        "--seed",
        "5",
        "--out",
        rev2.to_str().unwrap(),
    ]);
    assert!(rev2.join("Bell 0005.syx").is_file());
    std::fs::remove_dir_all(&dir).unwrap();
}
