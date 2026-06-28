//! CLI sanity check for the capture loop.
//!
//! Usage:
//!   sudo ./target/debug/examples/capture en0 [seconds]
//!
//! Prints one JSON PacketEvent per line. Requires BPF read permission;
//! see docs/setup.md.

use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use capture_core::CaptureSession;

fn main() {
    let mut args = std::env::args().skip(1);
    let interface = args.next().unwrap_or_else(|| {
        eprintln!("usage: capture <interface> [seconds]");
        std::process::exit(2);
    });
    let seconds: u64 = args
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(5);

    let count = Arc::new(Mutex::new(0u64));
    let count_cb = count.clone();

    let session = match CaptureSession::start(&interface, move |event| {
        *count_cb.lock().unwrap() += 1;
        match serde_json::to_string(&event) {
            Ok(line) => println!("{line}"),
            Err(_) => {}
        }
    }) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    };

    let stats = session.stats().clone();
    let deadline = Instant::now() + Duration::from_secs(seconds);
    while Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(100));
    }

    session.stop();

    let parsed = stats.packets_parsed.load(Ordering::Relaxed);
    let dropped = stats.packets_dropped_unparsed.load(Ordering::Relaxed);
    let bytes = stats.bytes_seen.load(Ordering::Relaxed);
    eprintln!(
        "captured {} packets ({} parsed, {} unparsed, {} bytes seen) in {}s",
        parsed + dropped,
        parsed,
        dropped,
        bytes,
        seconds,
    );
}
