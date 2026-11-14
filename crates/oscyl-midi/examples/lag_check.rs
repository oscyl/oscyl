//! Read-only: after optional reset attempts, read 5 then 300 and report
//! whether replies are one request behind. Reads are raw requests, so no
//! session check repairs the state before it is seen.
use std::time::{Duration, Instant};

use oscyl_midi::Device;
use oscyl_minilogue_xd::Program;
use oscyl_minilogue_xd::msg::{Dump, Reply, Request, decode, encode_request};

fn name(port: &mut Device, n: u16) -> String {
    let ch = port.channel();
    port.send(&encode_request(ch, &Request::Program(n)))
        .unwrap();
    match port
        .recv_until(Instant::now() + Duration::from_secs(2))
        .unwrap()
        .map(|m| decode(&m))
    {
        Some(Ok(Reply::Dump(Dump::Program(_, d)))) => Program::from_bytes(&d)
            .map(|p| p.name())
            .unwrap_or_else(|_| "?".into()),
        _ => "?".into(),
    }
}

fn main() {
    let mut port = Device::open_xd(None).unwrap();
    let ch = port.channel();
    for attempt in std::env::args().skip(1) {
        let req = match attempt.as_str() {
            "global" => Request::Global,
            "current" => Request::CurrentProgram,
            "identity" => Request::Identity,
            _ => panic!("unknown attempt"),
        };
        port.send(&encode_request(ch, &req)).unwrap();
        while port
            .recv_until(Instant::now() + Duration::from_millis(500))
            .unwrap()
            .is_some()
        {}
    }
    let a = name(&mut port, 5);
    let b = name(&mut port, 300);
    let c = name(&mut port, 300);
    let lag = b != c;
    println!("5={a:?} 300={b:?} 300={c:?} lagging={lag}");
}
