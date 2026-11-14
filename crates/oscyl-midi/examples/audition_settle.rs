//! Sends an edit-buffer dump (audition, 0x40) of program <index> from
//! <library>, then immediately polls raw program reads for 3 s, bypassing
//! the session's quiet period, to see whether auditions trigger a flash save.
use std::time::{Duration, Instant};

use oscyl_midi::Device;
use oscyl_minilogue_xd::library::Library;
use oscyl_minilogue_xd::msg::{Dump, Reply, Request, decode, encode_dump, encode_request};

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let lib = Library::read(std::fs::File::open(&a[1]).unwrap()).unwrap();
    let idx: usize = a[2].parse().unwrap();
    let p = lib.programs().unwrap()[idx].0.clone();
    let mut port = Device::open_xd(None).unwrap();
    let ch = port.channel();
    let t0 = Instant::now();
    port.send(&encode_dump(
        ch,
        &Dump::CurrentProgram(p.as_bytes().to_vec()),
    ))
    .unwrap();
    let ack = port
        .recv_until(Instant::now() + Duration::from_secs(2))
        .unwrap();
    println!(
        "{:?} audition reply {:?}",
        t0.elapsed(),
        ack.map(|m| decode(&m))
    );
    let mut bad = 0;
    let mut n = 0;
    while t0.elapsed() < Duration::from_secs(3) {
        port.send(&encode_request(ch, &Request::Program(300)))
            .unwrap();
        match port
            .recv_until(Instant::now() + Duration::from_secs(2))
            .unwrap()
            .map(|m| decode(&m))
        {
            Some(Ok(Reply::Dump(Dump::Program(300, d)))) if &d[4..10] == b"NR,137" => {}
            other => {
                bad += 1;
                println!(
                    "{:?} unexpected: {:?}",
                    t0.elapsed(),
                    other.map(|r| r.map(|_| "other dump"))
                );
            }
        }
        n += 1;
    }
    println!("{n} reads, {bad} bad");
}
