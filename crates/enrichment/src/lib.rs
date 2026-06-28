//! Background enrichment service for IP addresses.
//!
//! Provides best-effort hostname (reverse DNS) and country (GeoIP) lookups
//! that never block the packet capture or UI threads.
//!
//! Architecture:
//! - Callers invoke [`Enricher::lookup`] which is a non-blocking cache read.
//! - Cache misses are queued via [`Enricher::request`] for an MPSC worker
//!   thread to resolve in the background.
//! - Results land in a shared cache; subsequent ticks pick them up via
//!   [`Enricher::lookup`] or [`Enricher::enrich_in_place`].

mod geoip;
mod rdns;

use std::collections::{HashMap, HashSet};
use std::net::IpAddr;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex, RwLock};
use std::thread::{self, JoinHandle};

use shared_types::FlowRecord;

pub use geoip::GeoipError;

#[derive(Debug, Clone, Default)]
pub struct Enrichment {
    pub hostname: Option<String>,
    pub country: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct EnrichmentSnapshot {
    pub cached_entries: usize,
    pub queue_depth: usize,
    pub geoip_loaded: bool,
    pub rdns_enabled: bool,
}

/// Public handle to the enrichment subsystem. Cheap to clone.
#[derive(Clone)]
pub struct Enricher {
    inner: Arc<Inner>,
}

struct Inner {
    cache: RwLock<HashMap<IpAddr, Enrichment>>,
    pending: Mutex<HashSet<IpAddr>>,
    rdns_enabled: AtomicBool,
    geoip: RwLock<Option<geoip::GeoipDb>>,
    tx: Sender<Job>,
    worker: Mutex<Option<JoinHandle<()>>>,
    stopped: AtomicBool,
}

enum Job {
    Lookup(IpAddr),
    Stop,
}

impl Enricher {
    pub fn new(rdns_enabled: bool) -> Self {
        let (tx, rx) = mpsc::channel::<Job>();
        let inner = Arc::new(Inner {
            cache: RwLock::new(HashMap::new()),
            pending: Mutex::new(HashSet::new()),
            rdns_enabled: AtomicBool::new(rdns_enabled),
            geoip: RwLock::new(None),
            tx,
            worker: Mutex::new(None),
            stopped: AtomicBool::new(false),
        });

        let worker_inner = inner.clone();
        let handle = thread::Builder::new()
            .name("enrichment-worker".into())
            .spawn(move || run_worker(worker_inner, rx))
            .expect("spawn enrichment worker");
        *inner.worker.lock().expect("worker lock") = Some(handle);

        Self { inner }
    }

    /// Read the cache for `ip`. Never blocks on I/O.
    pub fn lookup(&self, ip: IpAddr) -> Option<Enrichment> {
        self.inner
            .cache
            .read()
            .ok()
            .and_then(|c| c.get(&ip).cloned())
    }

    /// Schedule a background lookup for `ip` if not already cached or queued.
    /// Cheap and non-blocking; the worker resolves it later.
    pub fn request(&self, ip: IpAddr) {
        if self.inner.stopped.load(Ordering::Relaxed) {
            return;
        }
        if self
            .inner
            .cache
            .read()
            .map(|c| c.contains_key(&ip))
            .unwrap_or(false)
        {
            return;
        }
        let mut pending = match self.inner.pending.lock() {
            Ok(p) => p,
            Err(_) => return,
        };
        if pending.insert(ip) {
            let _ = self.inner.tx.send(Job::Lookup(ip));
        }
    }

    pub fn set_rdns_enabled(&self, enabled: bool) {
        let prev = self.inner.rdns_enabled.swap(enabled, Ordering::Relaxed);
        // On off→on transitions, drop cache entries that lack a hostname
        // so the next snapshot re-queues them. Without this, IPs that were
        // first seen while rDNS was off would stay nameless forever.
        if enabled && !prev {
            if let Ok(mut cache) = self.inner.cache.write() {
                cache.retain(|_, e| e.hostname.is_some());
            }
        }
    }

    pub fn rdns_enabled(&self) -> bool {
        self.inner.rdns_enabled.load(Ordering::Relaxed)
    }

    /// Load (or replace) the GeoIP database from disk.
    /// Returns an error if the path cannot be opened, leaving any previously
    /// loaded DB intact.
    pub fn load_geoip(&self, path: PathBuf) -> Result<(), GeoipError> {
        let db = geoip::GeoipDb::open(&path)?;
        if let Ok(mut slot) = self.inner.geoip.write() {
            *slot = Some(db);
        }
        // Drop cache entries lacking a country so the next snapshot
        // re-queues them with the new DB. Entries that already had a
        // country resolved (from a previous DB) stay put.
        if let Ok(mut cache) = self.inner.cache.write() {
            cache.retain(|_, e| e.country.is_some());
        }
        Ok(())
    }

    pub fn clear_geoip(&self) {
        if let Ok(mut slot) = self.inner.geoip.write() {
            *slot = None;
        }
    }

    pub fn geoip_loaded(&self) -> bool {
        self.inner.geoip.read().map(|g| g.is_some()).unwrap_or(false)
    }

    pub fn clear_cache(&self) {
        if let Ok(mut cache) = self.inner.cache.write() {
            cache.clear();
        }
        if let Ok(mut pending) = self.inner.pending.lock() {
            pending.clear();
        }
    }

    pub fn stats(&self) -> EnrichmentSnapshot {
        EnrichmentSnapshot {
            cached_entries: self.inner.cache.read().map(|c| c.len()).unwrap_or(0),
            queue_depth: self.inner.pending.lock().map(|p| p.len()).unwrap_or(0),
            geoip_loaded: self.geoip_loaded(),
            rdns_enabled: self.rdns_enabled(),
        }
    }

    /// Decorate `flows` in place with cached enrichment data and queue
    /// background lookups for misses. Acceptable to call every snapshot tick.
    pub fn enrich_in_place(&self, flows: &mut [FlowRecord]) {
        for f in flows.iter_mut() {
            let Ok(ip) = f.dst_ip.parse::<IpAddr>() else {
                continue;
            };
            if is_private_or_loopback(ip) {
                // Don't waste lookups on private/link-local space.
                continue;
            }
            match self.lookup(ip) {
                Some(e) => {
                    if f.hostname.is_none() {
                        f.hostname = e.hostname;
                    }
                    if f.country.is_none() {
                        f.country = e.country;
                    }
                }
                None => self.request(ip),
            }
        }
    }
}

impl Drop for Inner {
    fn drop(&mut self) {
        self.stopped.store(true, Ordering::Relaxed);
        let _ = self.tx.send(Job::Stop);
        if let Ok(mut w) = self.worker.lock() {
            if let Some(handle) = w.take() {
                let _ = handle.join();
            }
        }
    }
}

fn run_worker(inner: Arc<Inner>, rx: mpsc::Receiver<Job>) {
    while let Ok(job) = rx.recv() {
        match job {
            Job::Stop => break,
            Job::Lookup(ip) => {
                let mut entry = Enrichment::default();
                if inner.rdns_enabled.load(Ordering::Relaxed) {
                    entry.hostname = rdns::lookup(ip);
                }
                if let Ok(slot) = inner.geoip.read() {
                    if let Some(db) = slot.as_ref() {
                        entry.country = db.lookup_country(ip);
                    }
                }
                if let Ok(mut cache) = inner.cache.write() {
                    cache.insert(ip, entry);
                }
                if let Ok(mut pending) = inner.pending.lock() {
                    pending.remove(&ip);
                }
            }
        }
    }
}

fn is_private_or_loopback(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v) => {
            v.is_loopback()
                || v.is_private()
                || v.is_link_local()
                || v.is_broadcast()
                || v.is_multicast()
                || v.is_unspecified()
        }
        IpAddr::V6(v) => {
            v.is_loopback()
                || v.is_unspecified()
                || v.is_multicast()
                // Link-local fe80::/10
                || (v.segments()[0] & 0xffc0) == 0xfe80
                // Unique-local fc00::/7
                || (v.segments()[0] & 0xfe00) == 0xfc00
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookup_returns_none_when_no_data() {
        let e = Enricher::new(false);
        let ip: IpAddr = "8.8.8.8".parse().unwrap();
        assert!(e.lookup(ip).is_none());
    }

    #[test]
    fn private_addresses_are_skipped() {
        let e = Enricher::new(false);
        let mut flows = vec![FlowRecord {
            id: "x".into(),
            src_ip: "10.0.0.1".into(),
            dst_ip: "10.0.0.2".into(),
            hostname: None,
            country: None,
            src_port: None,
            dst_port: None,
            protocol: shared_types::Protocol::Tcp,
            bytes_up: 1,
            bytes_down: 0,
            packets_up: 1,
            packets_down: 0,
            first_seen: 0,
            last_seen: 0,
        }];
        e.enrich_in_place(&mut flows);
        // No work scheduled for private space.
        let stats = e.stats();
        assert_eq!(stats.queue_depth, 0);
    }

    #[test]
    fn load_geoip_with_invalid_path_fails_softly() {
        let e = Enricher::new(false);
        let err = e.load_geoip(PathBuf::from("/nonexistent.mmdb"));
        assert!(err.is_err());
        assert!(!e.geoip_loaded());
    }

    #[test]
    fn rdns_toggle_persists() {
        let e = Enricher::new(false);
        assert!(!e.rdns_enabled());
        e.set_rdns_enabled(true);
        assert!(e.rdns_enabled());
    }
}
