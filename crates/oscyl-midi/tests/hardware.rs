//! End-to-end tests against a real minilogue xd on USB.
//!
//! Ignored by default. Run them with the synth connected, one at a time:
//!
//! ```sh
//! cargo test -p oscyl-midi --test hardware -- --ignored --test-threads=1
//! ```
//!
//! They only write to the edit buffer and to the last flash group
//! (programs 497–500), and they put back exactly what was there, verified
//! by reading it again. Take a backup first (`oscyl backup`) anyway.

use oscyl_midi::Device;
use oscyl_minilogue_xd::Program;
use oscyl_minilogue_xd::msg::PROGRAM_COUNT;
use oscyl_minilogue_xd::session::{Event, Op};
use oscyl_minilogue_xd::store::plan_commit;
use oscyl_minilogue_xd::sync::plan_write;

const HW: &str = "needs a minilogue xd connected";

/// The synth; a reply lag it was found in is cleared by the session
/// before the first op.
fn open() -> Device {
    Device::open_xd(None).expect("open the synth's SOUND port")
}

fn read(port: &mut Device, slots: impl IntoIterator<Item = u16>) -> Vec<(u16, Program)> {
    let mut got = Vec::new();
    port.run_xd(Op::ReadPrograms(slots.into_iter().collect()), |e| {
        if let Event::Program(n, p) = e {
            got.push((*n, p.clone()));
        }
    })
    .expect("read programs");
    got
}

fn current(port: &mut Device) -> Program {
    let mut cur = None;
    port.run_xd(Op::ReadCurrent, |e| {
        if let Event::Current(p) = e {
            cur = Some(p.clone());
        }
    })
    .expect("read the edit buffer");
    cur.expect("a current program")
}

/// Write `wanted` over what is on the synth (`device`, all 500), the way
/// the app does: only differing programs, through a commit plan.
fn write(port: &mut Device, device: &[Option<Program>], wanted: &[Program]) -> Vec<u16> {
    let todo = plan_write(device, wanted);
    let plan = plan_commit(todo.iter().map(|&i| (i, wanted[i as usize].clone())), |n| {
        device[n as usize].clone()
    })
    .expect("plan");
    port.run_xd(Op::WritePrograms(plan), |_| {}).expect("write");
    todo
}

fn read_all(port: &mut Device) -> Vec<Option<Program>> {
    let mut all = vec![None; PROGRAM_COUNT as usize];
    for (n, p) in read(port, 0..PROGRAM_COUNT) {
        all[n as usize] = Some(p);
    }
    assert!(all.iter().all(Option::is_some), "every program read");
    all
}

#[test]
#[ignore = "needs a minilogue xd connected"]
fn identifies_and_reads_every_kind_of_data() {
    let _ = HW;
    let mut port = open();
    let fw = port.firmware().expect("firmware");
    assert!(fw.split('.').next().unwrap().parse::<u16>().unwrap() >= 2);
    // Reads are deterministic: the same program twice is the same bytes.
    let a = read(&mut port, [0, 250, 499]);
    let b = read(&mut port, [0, 250, 499]);
    assert_eq!(a, b);
    assert_eq!(a.iter().map(|(n, _)| *n).collect::<Vec<_>>(), [0, 250, 499]);

    let mut kinds = (0, 0, 0, 0);
    port.run_xd(Op::ReadGlobal, |e| {
        kinds.0 += matches!(e, Event::Global(_)) as u32
    })
    .unwrap();
    port.run_xd(Op::ReadTuning, |e| {
        kinds.1 += matches!(e, Event::Scale(..) | Event::Octave(..)) as u32
    })
    .unwrap();
    port.run_xd(Op::ReadSlots, |e| {
        kinds.2 += matches!(e, Event::Slot { .. }) as u32
    })
    .unwrap();
    let _ = current(&mut port);
    kinds.3 = 1;
    assert_eq!(kinds, (1, 12, 48, 1));
}

#[test]
#[ignore = "needs a minilogue xd connected"]
fn audition_changes_only_the_edit_buffer() {
    let mut port = open();
    let before = current(&mut port);
    let stored = read(&mut port, [0]);

    let mut probe = before.clone();
    probe.set_name("Oscyl E2E").unwrap();
    port.run_xd(Op::Audition(Box::new(probe.clone())), |_| {})
        .unwrap();
    assert_eq!(current(&mut port).name(), "Oscyl E2E");

    port.run_xd(Op::Audition(Box::new(before.clone())), |_| {})
        .unwrap();
    assert_eq!(current(&mut port), before, "edit buffer restored");
    assert_eq!(read(&mut port, [0]), stored, "nothing stored");
}

#[test]
#[ignore = "needs a minilogue xd connected"]
fn writes_the_last_group_and_puts_it_back() {
    let mut port = open();
    let device = read_all(&mut port);
    let original: Vec<Program> = device.iter().flatten().cloned().collect();

    // Two of the last group's four programs, including the firmware's
    // special case (program 500, index 499).
    let mut wanted = original.clone();
    wanted[497].set_name("E2E 498").unwrap();
    wanted[499].set_name("E2E 500").unwrap();
    assert_eq!(write(&mut port, &device, &wanted), [497, 499]);

    let after: Vec<Option<Program>> = {
        let mut v = device.clone();
        for (n, p) in read(&mut port, 496..500) {
            v[n as usize] = Some(p);
        }
        v
    };
    assert_eq!(
        after[497].as_ref().map(Program::name).as_deref(),
        Some("E2E 498")
    );
    assert_eq!(
        after[499].as_ref().map(Program::name).as_deref(),
        Some("E2E 500")
    );
    assert_eq!(after[496], device[496], "neighbours untouched");
    assert_eq!(after[498], device[498], "neighbours untouched");

    // Put everything back and prove it by reading the whole synth.
    assert_eq!(write(&mut port, &after, &original), [497, 499]);
    let restored = read_all(&mut port);
    assert!(
        plan_write(&restored, &original).is_empty(),
        "synth as it was"
    );

    // No reply lag after the commit: a fresh port opens clean.
    drop(port);
    let _ = open();
}

#[test]
#[ignore = "needs a minilogue xd connected"]
fn live_messages_reach_the_edit_buffer() {
    use oscyl_minilogue_xd::Field;
    use oscyl_minilogue_xd::live;
    let mut port = open();
    let before = current(&mut port);
    // A spread of controls, selectors and NRPNs, with values whose low bits
    // are set, so the CC 63 split is checked too.
    let mut changed = Vec::new();
    for (field, value) in [
        (Field::Cutoff, 517),
        (Field::Resonance, 301),
        (Field::AmpEgAttack, 9),
        (Field::Vco1Pitch, 700),
        (Field::Vco2Shape, 1023),
        (Field::DelayTime, 333),
        (Field::Vco1Wave, 1),
        (Field::Vco2Octave, 3),
        (Field::CutoffDrive, 2),
        (Field::LfoMode, 0),
        (Field::Sync, 1),
        (Field::ReverbOn, 1),
        (Field::ReverbSubType, 4),
        (Field::Portamento, 77),
        (Field::ProgramLevel, 100),
        (Field::MultiOctave, 2),
        (Field::EgLegato, 1),
        (Field::ShapeNoise, 517),
        (Field::JoystickRangePlus, 150),
    ] {
        let mut want = current(&mut port);
        want.set(field, value).unwrap();
        let msgs = live::messages(field, &want).expect("a live message");
        let status = 0xB0 | port.channel();
        for (cc, v) in msgs {
            port.send(&[status, cc, v]).unwrap();
        }
        let got = current(&mut port);
        changed.push((field, value, got.get(field)));
    }
    port.run_xd(Op::Audition(Box::new(before.clone())), |_| {})
        .unwrap();
    let wrong: Vec<_> = changed
        .iter()
        .filter(|(_, want, got)| want != got)
        .collect();
    assert!(wrong.is_empty(), "set, read back: {wrong:?}");
    assert_eq!(current(&mut port), before, "edit buffer restored");
}

#[test]
#[ignore = "needs a minilogue xd connected"]
fn generated_programs_audition_exactly() {
    use oscyl_minilogue_xd::Field;
    use oscyl_minilogue_xd::generate::{Character, Macros, Recipe, generate};
    let mut port = open();
    let before = current(&mut port);
    let mut wrong = Vec::new();
    for character in Character::ALL {
        let p = generate(&Recipe {
            character,
            macros: Macros::default(),
            seed: 2026,
            keep: Vec::new(),
            units: Vec::new(),
        });
        port.run_xd(Op::Audition(Box::new(p.clone())), |_| {})
            .unwrap();
        let got = current(&mut port);
        let fields: Vec<_> = Field::ALL
            .iter()
            .filter(|&&f| got.get(f) != p.get(f))
            .map(|&f| (f, p.get(f), got.get(f)))
            .collect();
        if got != p {
            wrong.push((character, got.name(), fields));
        }
    }
    // Put the edit buffer back before judging, so a failure leaves the
    // synth as it was.
    port.run_xd(Op::Audition(Box::new(before.clone())), |_| {})
        .unwrap();
    assert_eq!(current(&mut port), before, "edit buffer restored");
    assert!(wrong.is_empty(), "auditioned, read back: {wrong:?}");
}
