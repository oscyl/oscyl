//! Research only: deliberately trigger the firmware 2.10 reply-lag fault to
//! test recovery. Writes program 0 back unchanged WITHOUT a commit, then
//! reads until a reply without the PROG marker shows the read raced the
//! deferred flash save.
use std::time::{Duration, Instant};

use oscyl_midi::Device;
use oscyl_minilogue_xd::msg::{Dump, Reply, Request, decode, encode_dump, encode_request};

fn main() {
    let mut port = Device::open_xd(None).unwrap();
    let ch = port.channel();
    let next = |port: &mut Device| {
        port.recv_until(Instant::now() + Duration::from_secs(2))
            .unwrap()
    };
    port.send(&encode_request(ch, &Request::Program(0)))
        .unwrap();
    let Some(Ok(Reply::Dump(Dump::Program(0, p0)))) = next(&mut port).map(|m| decode(&m)) else {
        panic!("could not read program 0")
    };
    port.send(&encode_dump(ch, &Dump::Program(0, p0))).unwrap();
    println!("write ack: {:?}", next(&mut port).map(|m| decode(&m)));
    let t0 = Instant::now();
    loop {
        port.send(&encode_request(ch, &Request::Program(1)))
            .unwrap();
        match next(&mut port).map(|m| decode(&m)) {
            Some(Ok(Reply::Dump(Dump::Program(_, d)))) if d.starts_with(b"PROG") => {}
            other => {
                println!(
                    "{:?}: raced the save: {:?}",
                    t0.elapsed(),
                    other.map(|r| r.is_ok())
                );
                return;
            }
        }
        if t0.elapsed() > Duration::from_secs(5) {
            println!("no race observed");
            return;
        }
    }
}
