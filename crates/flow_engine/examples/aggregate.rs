//! End-to-end CLI sanity check: capture + aggregate + print top flows.
//!
//! Usage:
//!   sudo ./target/debug/examples/aggregate en0 [seconds] [top_n]
//!
//! Requires BPF read permission; see docs/setup.md.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use capture_core::CaptureSession;
use flow_engine::FlowAggregator;

fn main() {
    let mut args = std::env::args().skip(1);
    let interface = args.next().unwrap_or_else(|| {
        eprintln!("usage: aggregate <interface> [seconds] [top_n]");
        std::process::exit(2);
    });
    let seconds: u64 = args.next().and_then(|s| s.parse().ok()).unwrap_or(10);
    let top_n: usize = args.next().and_then(|s| s.parse().ok()).unwrap_or(10);

    let agg = Arc::new(Mutex::new(FlowAggregator::new()));
    let agg_cb = agg.clone();

    let session = match CaptureSession::start(&interface, move |event| {
        if let Ok(mut a) = agg_cb.lock() {
            a.ingest(&event);
        }
    }) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    };

    let deadline = Instant::now() + Duration::from_secs(seconds);
    while Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(1_000));
        if let Ok(a) = agg.lock() {
            eprintln!(
                "[{:>3}s] {} flows tracked",
                (deadline - Instant::now()).as_secs(),
                a.flow_count()
            );
        }
    }

    session.stop();

    let snap = agg.lock().unwrap().snapshot_top(top_n);
    println!("\ntop {} flows by total bytes:", snap.len());
    println!(
        "{:<6} {:<22} {:<6} {:<22} {:<6} {:>10} {:>10}",
        "proto", "src_ip", "sport", "dst_ip", "dport", "bytes_up", "bytes_dn"
    );
    for f in snap {
        println!(
            "{:<6} {:<22} {:<6} {:<22} {:<6} {:>10} {:>10}",
            format!("{:?}", f.protocol).to_lowercase(),
            f.src_ip,
            f.src_port.map_or("-".into(), |p| p.to_string()),
            f.dst_ip,
            f.dst_port.map_or("-".into(), |p| p.to_string()),
            f.bytes_up,
            f.bytes_down,
        );
    }
}
