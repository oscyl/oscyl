//! Writes programs lo..=hi from <library> back to back (same slots), then
//! commits, and reads them back immediately. Prints per-write ACK times so
//! the firmware's synchronous saves can be compared with `store::StoreModel`.
//! Usage: rewrite_range <library> <lo> <hi>
use std::time::Instant;

use oscyl_midi::Device;
use oscyl_minilogue_xd::library::Library;
use oscyl_minilogue_xd::session::{Event, Op};

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let lib = Library::read(std::fs::File::open(&a[1]).unwrap()).unwrap();
    let (lo, hi): (u16, u16) = (a[2].parse().unwrap(), a[3].parse().unwrap());
    let progs = lib.programs().unwrap();
    let want: Vec<_> = (lo..=hi)
        .map(|i| (i, progs[i as usize].0.clone()))
        .collect();
    let mut port = Device::open_xd(None).unwrap();
    let t0 = Instant::now();
    let mut acks = Vec::new();
    let plan = oscyl_minilogue_xd::store::plan_commit(want.clone(), |_| None).unwrap();
    println!(
        "plan: {:?}",
        plan.writes().iter().map(|(n, _)| n).collect::<Vec<_>>()
    );
    port.run_xd(Op::WritePrograms(plan), |e| {
        if let Event::Progress { .. } = e {
            acks.push(t0.elapsed().as_millis())
        }
    })
    .unwrap();
    println!("wrote {} programs, ack times ms: {acks:?}", want.len());
    let mut got = Vec::new();
    port.run_xd(Op::ReadPrograms((lo..=hi).collect()), |e| {
        if let Event::Program(n, p) = e {
            got.push((*n, p.clone()))
        }
    })
    .unwrap();
    println!("read back at {:?}", t0.elapsed());
    let bad: Vec<u16> = got
        .iter()
        .zip(&want)
        .filter(|(g, w)| g != w)
        .map(|(g, _)| g.0)
        .collect();
    println!("mismatched: {bad:?}");
}
