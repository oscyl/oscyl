//! End-to-end sessions against the simulated synth.

use oscyl_midi::{Device, Error, SimLink};
use oscyl_minilogue_xd::Program;
use oscyl_minilogue_xd::demo::{demo_library, demo_units};
use oscyl_minilogue_xd::msg::{Dump, Module, PROGRAM_COUNT, Request, encode_dump, encode_request};
use oscyl_minilogue_xd::session::{Event, Op, OpError};
use oscyl_minilogue_xd::sim::SimDevice;
use oscyl_minilogue_xd::store::plan_commit;
use oscyl_minilogue_xd::sync::plan_write;

/// A minilogue xd device on `sim`, through the same inquiry as a real one.
fn simulated(sim: SimDevice) -> Result<Device, Error> {
    Device::connect(
        &oscyl_minilogue_xd::DRIVER,
        Box::new(SimLink::new(sim)),
        "simulated minilogue xd".into(),
    )
}

fn demo_programs() -> Vec<Program> {
    demo_library().into_iter().map(|(p, _)| p).collect()
}

fn read_all(port: &mut Device) -> Vec<Option<Program>> {
    let mut got = vec![None; PROGRAM_COUNT as usize];
    port.run_xd(Op::ReadPrograms((0..PROGRAM_COUNT).collect()), |e| {
        if let Event::Program(n, p) = e {
            got[*n as usize] = Some(p.clone());
        }
    })
    .unwrap();
    got
}

fn slots(port: &mut Device) -> Vec<(Module, u8, Option<Vec<u8>>)> {
    let mut v = Vec::new();
    port.run_xd(Op::ReadSlots, |e| {
        if let Event::Slot {
            module,
            slot,
            header,
        } = e
        {
            v.push((*module, *slot, header.clone()));
        }
    })
    .unwrap();
    v
}

#[test]
fn opens_like_a_firmware_2_10_synth() {
    let port = simulated(SimDevice::new()).unwrap();
    assert_eq!(
        (port.channel(), port.firmware().as_deref()),
        (0, Some("2.10"))
    );
}

#[test]
fn full_backup_of_the_demo_library() {
    let mut port = simulated(SimDevice::demo()).unwrap();
    let progs = read_all(&mut port);
    let demo = demo_programs();
    assert!(progs.iter().all(Option::is_some));
    for (i, p) in demo.iter().enumerate() {
        assert_eq!(progs[i].as_ref(), Some(p));
    }
    let mut global = None;
    port.run_xd(Op::ReadGlobal, |e| {
        if let Event::Global(g) = e {
            global = Some(g.clone())
        }
    })
    .unwrap();
    assert!(global.unwrap().starts_with(b"GLOB"));
    let mut tables = 0;
    port.run_xd(Op::ReadTuning, |e| {
        tables += matches!(e, Event::Scale(..) | Event::Octave(..)) as usize
    })
    .unwrap();
    assert_eq!(tables, 12);
    let occupied: Vec<_> = slots(&mut port)
        .into_iter()
        .filter(|(_, _, h)| h.is_some())
        .collect();
    assert_eq!(occupied.len(), demo_units().len());
    for (module, slot, _) in occupied {
        let mut data = None;
        port.run_xd(Op::ReadSlotData { module, slot }, |e| {
            if let Event::SlotData { data: d, .. } = e {
                data = Some(d.clone())
            }
        })
        .unwrap();
        assert!(data.unwrap().len() > 8);
    }
}

#[test]
fn diff_commit_and_read_back() {
    let mut port = simulated(SimDevice::demo()).unwrap();
    let device = read_all(&mut port);
    let mut wanted: Vec<Program> = device.iter().flatten().cloned().collect();
    for (n, name) in [
        (3, "Edited 3"),
        (4, "Edited 4"),
        (250, "Edited 250"),
        (499, "Last"),
    ] {
        wanted[n].set_name(name).unwrap();
    }
    let todo = plan_write(&device, &wanted);
    assert_eq!(todo, [3, 4, 250, 499]);
    let plan = plan_commit(todo.iter().map(|&i| (i, wanted[i as usize].clone())), |n| {
        device[n as usize].clone()
    })
    .unwrap();
    port.run_xd(Op::WritePrograms(plan), |_| {}).unwrap();
    let after = read_all(&mut port);
    assert!(plan_write(&after, &wanted).is_empty());
}

#[test]
fn audition_loads_the_edit_buffer_only() {
    let mut port = simulated(SimDevice::demo()).unwrap();
    let p = demo_programs().swap_remove(42);
    port.run_xd(Op::Audition(Box::new(p.clone())), |_| {})
        .unwrap();
    let mut cur = None;
    port.run_xd(Op::ReadCurrent, |e| {
        if let Event::Current(c) = e {
            cur = Some(c.clone())
        }
    })
    .unwrap();
    assert_eq!(cur, Some(p.clone()));
    assert_ne!(read_all(&mut port)[0].as_ref(), Some(&p));
}

#[test]
fn unit_upload_list_and_clear() {
    let mut port = simulated(SimDevice::new()).unwrap();
    assert!(slots(&mut port).iter().all(|(_, _, h)| h.is_none()));
    assert_eq!(slots(&mut port).len(), 48);
    let (_, unit) = demo_units().swap_remove(1);
    let data = unit.slot_data();
    port.run_xd(
        Op::UploadUnit {
            module: unit.module,
            slot: 10,
            data: data.clone(),
        },
        |_| {},
    )
    .unwrap();
    let listed = slots(&mut port);
    let header = listed
        .iter()
        .find(|(m, s, _)| (*m, *s) == (unit.module, 10))
        .and_then(|(_, _, h)| h.clone())
        .unwrap();
    assert_eq!(&header[18..23], unit.name.as_bytes());
    port.run_xd(
        Op::ClearSlot {
            module: unit.module,
            slot: 10,
        },
        |_| {},
    )
    .unwrap();
    assert!(slots(&mut port).iter().all(|(_, _, h)| h.is_none()));
    // A bad slot is reported with KORG's error code.
    let err = port
        .run_xd(
            Op::ClearSlot {
                module: Module::DelFx,
                slot: 9,
            },
            |_| {},
        )
        .unwrap_err();
    assert!(matches!(err, Error::Op(OpError::Device { code: 0x2D, .. })));
}

#[test]
fn recovers_a_synth_already_in_reply_lag() {
    let mut sim = SimDevice::demo();
    sim.faults.race_deferred_save = true;
    // An uncommitted write leaves a deferred save; a read races it.
    let p = sim.program(7).unwrap().clone();
    sim.handle(&encode_dump(0, &Dump::Program(7, p.as_bytes().to_vec())));
    sim.handle(&encode_request(0, &Request::Program(9)));
    assert!(sim.is_lagging());

    // The xd session clears the lag before its first op.
    let mut port = simulated(sim).unwrap();
    let progs = read_all(&mut port);
    for (i, p) in demo_programs().iter().enumerate() {
        assert_eq!(progs[i].as_ref(), Some(p), "program {i}");
    }
}

/// Programs 0 and 1 identical, the stale reply different: a, b, b alone
/// reads X, A, A and took that for healthy.
#[test]
fn identical_probe_pair_still_detects_the_lag() {
    let named = |n: &str| {
        let mut p = Program::init();
        p.set_name(n).unwrap();
        p
    };
    let lib = [named("Twin"), named("Twin"), named("Two"), named("Three")];
    let mut sim = SimDevice::with_library(&lib);
    sim.faults.race_deferred_save = true;
    sim.handle(&encode_dump(
        0,
        &Dump::Program(5, lib[0].as_bytes().to_vec()),
    ));
    sim.handle(&encode_request(0, &Request::Program(6)));
    assert!(sim.is_lagging());

    // The xd session's a, b, b, a check clears it before the first op.
    let mut port = simulated(sim).unwrap();
    let progs = read_all(&mut port);
    assert_eq!(progs[2].as_ref().map(Program::name).as_deref(), Some("Two"));
    assert_eq!(
        progs[3].as_ref().map(Program::name).as_deref(),
        Some("Three")
    );
}

#[test]
fn a_lost_reply_times_out_at_once() {
    let mut sim = SimDevice::demo();
    // The identity is reply 1, the session's lag check reads 2-5 (a, b,
    // b, a), the edit buffer read before the first xd op 6; reply 7 is
    // the read below.
    sim.faults.drop_every_nth_reply = Some(7);
    let mut port = simulated(sim).unwrap();
    let t0 = std::time::Instant::now();
    let err = port.run_xd(Op::ReadPrograms(vec![0]), |_| {}).unwrap_err();
    assert!(matches!(err, Error::Op(OpError::Timeout { .. })));
    assert!(t0.elapsed().as_secs() < 1, "must not wait out the deadline");
}
