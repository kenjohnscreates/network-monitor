//! Capture session: pcap handle + parser thread.
//!
//! Architecture:
//! - One OS thread owns the pcap handle and blocks on `next_packet`.
//! - Each packet is parsed into a [`PacketEvent`] and handed to the caller's
//!   `on_packet` callback.
//! - Stopping is cooperative: `stop()` flips an atomic, then closes the
//!   handle so the next read returns. We use pcap's read timeout so the loop
//!   wakes periodically even when traffic is idle.
//!
//! The callback runs on the capture thread, so keep it cheap (push to a
//! channel, send a Tauri event, etc).

use std::net::IpAddr;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use shared_types::PacketEvent;
use thiserror::Error;

use crate::direction::DirectionResolver;
use crate::parse::{parse_packet, DataLink};

/// Default snapshot length passed to libpcap. Large enough for IPv6 + TCP
/// options; small enough that we never copy useful payload by accident.
const SNAPLEN: i32 = 256;

/// Read timeout in ms. Keeps the loop responsive to `stop()` without
/// burning CPU spinning.
const READ_TIMEOUT_MS: i32 = 250;

#[derive(Debug, Error)]
pub enum CaptureError {
    #[error("pcap error: {0}")]
    Pcap(#[from] pcap::Error),

    #[error("interface `{0}` not found")]
    InterfaceNotFound(String),

    #[error("interface `{name}` uses unsupported link layer type {linktype}")]
    UnsupportedLink { name: String, linktype: i32 },

    #[error("capture session is already running")]
    AlreadyRunning,
}

/// Live counters owned by a session, cheap to read from any thread.
#[derive(Debug, Default)]
pub struct CaptureStats {
    pub packets_parsed: AtomicU64,
    pub packets_dropped_unparsed: AtomicU64,
    pub bytes_seen: AtomicU64,
}

impl CaptureStats {
    pub fn snapshot(&self) -> CaptureStatsSnapshot {
        CaptureStatsSnapshot {
            packets_parsed: self.packets_parsed.load(Ordering::Relaxed),
            packets_dropped_unparsed: self.packets_dropped_unparsed.load(Ordering::Relaxed),
            bytes_seen: self.bytes_seen.load(Ordering::Relaxed),
        }
    }
}

#[derive(Debug, Clone, Copy, serde::Serialize)]
pub struct CaptureStatsSnapshot {
    pub packets_parsed: u64,
    pub packets_dropped_unparsed: u64,
    pub bytes_seen: u64,
}

pub struct CaptureSession {
    interface: String,
    stop: Arc<AtomicBool>,
    stats: Arc<CaptureStats>,
    join: Option<JoinHandle<()>>,
}

impl CaptureSession {
    /// Open a capture on `interface_name` and start the parser loop.
    /// `on_packet` is called from the capture thread for every parsed event.
    pub fn start<F>(interface_name: &str, mut on_packet: F) -> Result<Self, CaptureError>
    where
        F: FnMut(PacketEvent) + Send + 'static,
    {
        let device = pcap::Device::list()?
            .into_iter()
            .find(|d| d.name == interface_name)
            .ok_or_else(|| CaptureError::InterfaceNotFound(interface_name.to_string()))?;

        let local_ips = device
            .addresses
            .iter()
            .map(|a| match a.addr {
                IpAddr::V4(v4) => v4.to_string(),
                IpAddr::V6(v6) => v6.to_string(),
            })
            .collect::<Vec<_>>();
        let resolver = DirectionResolver::new(local_ips);

        let cap = pcap::Capture::from_device(device)?
            .promisc(false)
            .snaplen(SNAPLEN)
            .immediate_mode(true)
            .timeout(READ_TIMEOUT_MS)
            .open()?;

        let linktype = cap.get_datalink();
        let link = DataLink::from_pcap(linktype);
        if link == DataLink::Unsupported {
            return Err(CaptureError::UnsupportedLink {
                name: interface_name.to_string(),
                linktype: linktype.0,
            });
        }

        let stop = Arc::new(AtomicBool::new(false));
        let stats = Arc::new(CaptureStats::default());

        let stop_thread = stop.clone();
        let stats_thread = stats.clone();
        let interface = interface_name.to_string();

        let join = thread::Builder::new()
            .name(format!("capture:{interface}"))
            .spawn(move || {
                run_loop(cap, link, resolver, stop_thread, stats_thread, &mut on_packet);
            })
            .expect("spawn capture thread");

        Ok(Self {
            interface,
            stop,
            stats,
            join: Some(join),
        })
    }

    pub fn interface(&self) -> &str {
        &self.interface
    }

    pub fn stats(&self) -> &Arc<CaptureStats> {
        &self.stats
    }

    /// Signal the loop to exit and join the thread.
    pub fn stop(mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(handle) = self.join.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for CaptureSession {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(handle) = self.join.take() {
            // Best-effort: don't panic in drop.
            let _ = handle.join();
        }
    }
}

fn run_loop<F>(
    mut cap: pcap::Capture<pcap::Active>,
    link: DataLink,
    resolver: DirectionResolver,
    stop: Arc<AtomicBool>,
    stats: Arc<CaptureStats>,
    on_packet: &mut F,
) where
    F: FnMut(PacketEvent),
{
    while !stop.load(Ordering::Relaxed) {
        match cap.next_packet() {
            Ok(packet) => {
                let ts_ms = packet.header.ts.tv_sec as i64 * 1000
                    + (packet.header.ts.tv_usec as i64) / 1000;
                let captured_len = packet.header.caplen;
                stats.bytes_seen.fetch_add(captured_len as u64, Ordering::Relaxed);

                if let Some(event) = parse_packet(
                    ts_ms,
                    captured_len,
                    packet.data,
                    link,
                    &resolver,
                ) {
                    stats.packets_parsed.fetch_add(1, Ordering::Relaxed);
                    on_packet(event);
                } else {
                    stats.packets_dropped_unparsed.fetch_add(1, Ordering::Relaxed);
                }
            }
            Err(pcap::Error::TimeoutExpired) => {
                // Idle tick: check stop flag and loop.
                continue;
            }
            Err(pcap::Error::NoMorePackets) => break,
            Err(_) => {
                // For any other error, sleep briefly to avoid a hot error loop,
                // then continue. The user can stop the session if it persists.
                thread::sleep(Duration::from_millis(50));
            }
        }
    }
}
