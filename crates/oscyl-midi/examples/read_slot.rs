//! Read-only: fetch one user slot's data (0x1A) and save it.
//! Usage: cargo run -p oscyl-midi --example read_slot -- <module 1-4> <slot> <out.bin>
use std::time::{Duration, Instant};

use oscyl_korg_minilogue_xd::msg::{Dump, Module, Reply, Request, decode, encode_request};
use oscyl_midi::Device;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let module = Module::from_id(a[1].parse().unwrap()).unwrap();
    let slot: u8 = a[2].parse().unwrap();
    let mut port = Device::open_xd(None).unwrap();
    let t0 = Instant::now();
    port.send(&encode_request(
        port.channel(),
        &Request::SlotData { module, slot },
    ))
    .unwrap();
    while let Some(m) = port
        .recv_until(Instant::now() + Duration::from_secs(30))
        .unwrap()
    {
        match decode(&m) {
            Ok(Reply::Dump(Dump::SlotData { data, .. })) => {
                println!("{} bytes in {:?}", data.len(), t0.elapsed());
                std::fs::write(&a[3], &data).unwrap();
                return;
            }
            other => println!("other: {other:?}"),
        }
    }
    println!("timeout");
}
