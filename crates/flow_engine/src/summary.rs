//! Aggregate views over the flow table for dashboard rendering.
//!
//! These walk the full flow set once and emit small ranked lists. They are
//! cheap (no allocations beyond the result) and intended to be called from
//! the snapshotter on every tick.
//!
//! Counters cover lifetime traffic of the current session, not per-tick rates;
//! rates are derived in the caller by diffing successive summaries.

use std::collections::HashMap;

use serde::Serialize;
use shared_types::{FlowRecord, Protocol};

use crate::FlowAggregator;

#[derive(Debug, Clone, Serialize)]
pub struct Totals {
    pub bytes_up: u64,
    pub bytes_down: u64,
    pub packets_up: u64,
    pub packets_down: u64,
}

impl Totals {
    pub fn total_bytes(&self) -> u64 {
        self.bytes_up.saturating_add(self.bytes_down)
    }
    pub fn total_packets(&self) -> u64 {
        self.packets_up.saturating_add(self.packets_down)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct HostStat {
    pub ip: String,
    pub hostname: Option<String>,
    pub country: Option<String>,
    pub bytes: u64,
    pub packets: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProtocolStat {
    pub protocol: Protocol,
    pub bytes: u64,
    pub packets: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct PortStat {
    pub port: u16,
    pub bytes: u64,
    pub packets: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct CountryStat {
    pub country: String,
    pub bytes: u64,
    pub packets: u64,
}

impl FlowAggregator {
    pub fn totals(&self) -> Totals {
        totals(&self.snapshot())
    }
    pub fn top_hosts(&self, n: usize) -> Vec<HostStat> {
        top_hosts(&self.snapshot(), n)
    }
    pub fn top_protocols(&self, n: usize) -> Vec<ProtocolStat> {
        top_protocols(&self.snapshot(), n)
    }
    pub fn top_ports(&self, n: usize) -> Vec<PortStat> {
        top_ports(&self.snapshot(), n)
    }
    pub fn top_countries(&self, n: usize) -> Vec<CountryStat> {
        top_countries(&self.snapshot(), n)
    }
}

pub fn totals(flows: &[FlowRecord]) -> Totals {
    let mut t = Totals {
        bytes_up: 0,
        bytes_down: 0,
        packets_up: 0,
        packets_down: 0,
    };
    for f in flows {
        t.bytes_up = t.bytes_up.saturating_add(f.bytes_up);
        t.bytes_down = t.bytes_down.saturating_add(f.bytes_down);
        t.packets_up = t.packets_up.saturating_add(f.packets_up);
        t.packets_down = t.packets_down.saturating_add(f.packets_down);
    }
    t
}

pub fn top_hosts(flows: &[FlowRecord], n: usize) -> Vec<HostStat> {
    let mut by_ip: HashMap<String, HostStat> = HashMap::new();
    for f in flows {
        let total_bytes = f.bytes_up.saturating_add(f.bytes_down);
        let total_packets = f.packets_up.saturating_add(f.packets_down);
        let entry = by_ip.entry(f.dst_ip.clone()).or_insert_with(|| HostStat {
            ip: f.dst_ip.clone(),
            hostname: f.hostname.clone(),
            country: f.country.clone(),
            bytes: 0,
            packets: 0,
        });
        entry.bytes = entry.bytes.saturating_add(total_bytes);
        entry.packets = entry.packets.saturating_add(total_packets);
        if entry.hostname.is_none() {
            entry.hostname = f.hostname.clone();
        }
        if entry.country.is_none() {
            entry.country = f.country.clone();
        }
    }
    top_n_by_bytes(by_ip.into_values().collect(), n, |h| h.bytes)
}

pub fn top_protocols(flows: &[FlowRecord], n: usize) -> Vec<ProtocolStat> {
    let mut by_proto: HashMap<Protocol, ProtocolStat> = HashMap::new();
    for f in flows {
        let total_bytes = f.bytes_up.saturating_add(f.bytes_down);
        let total_packets = f.packets_up.saturating_add(f.packets_down);
        let entry = by_proto
            .entry(f.protocol)
            .or_insert_with(|| ProtocolStat {
                protocol: f.protocol,
                bytes: 0,
                packets: 0,
            });
        entry.bytes = entry.bytes.saturating_add(total_bytes);
        entry.packets = entry.packets.saturating_add(total_packets);
    }
    top_n_by_bytes(by_proto.into_values().collect(), n, |p| p.bytes)
}

pub fn top_ports(flows: &[FlowRecord], n: usize) -> Vec<PortStat> {
    let mut by_port: HashMap<u16, PortStat> = HashMap::new();
    for f in flows {
        let Some(port) = f.dst_port else { continue };
        let total_bytes = f.bytes_up.saturating_add(f.bytes_down);
        let total_packets = f.packets_up.saturating_add(f.packets_down);
        let entry = by_port.entry(port).or_insert_with(|| PortStat {
            port,
            bytes: 0,
            packets: 0,
        });
        entry.bytes = entry.bytes.saturating_add(total_bytes);
        entry.packets = entry.packets.saturating_add(total_packets);
    }
    top_n_by_bytes(by_port.into_values().collect(), n, |p| p.bytes)
}

pub fn top_countries(flows: &[FlowRecord], n: usize) -> Vec<CountryStat> {
    let mut by_country: HashMap<String, CountryStat> = HashMap::new();
    for f in flows {
        let Some(country) = f.country.as_ref() else { continue };
        let total_bytes = f.bytes_up.saturating_add(f.bytes_down);
        let total_packets = f.packets_up.saturating_add(f.packets_down);
        let entry = by_country
            .entry(country.clone())
            .or_insert_with(|| CountryStat {
                country: country.clone(),
                bytes: 0,
                packets: 0,
            });
        entry.bytes = entry.bytes.saturating_add(total_bytes);
        entry.packets = entry.packets.saturating_add(total_packets);
    }
    top_n_by_bytes(by_country.into_values().collect(), n, |c| c.bytes)
}

fn top_n_by_bytes<T, F>(mut v: Vec<T>, n: usize, key: F) -> Vec<T>
where
    F: Fn(&T) -> u64,
{
    v.sort_by(|a, b| key(b).cmp(&key(a)));
    v.truncate(n);
    v
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared_types::{Direction, PacketEvent};

    fn pkt(src: &str, sp: u16, dst: &str, dp: u16, proto: Protocol, len: u32, dir: Direction) -> PacketEvent {
        PacketEvent {
            timestamp: 0,
            src_ip: src.into(),
            dst_ip: dst.into(),
            src_port: Some(sp),
            dst_port: Some(dp),
            protocol: proto,
            packet_length: len,
            direction: dir,
        }
    }

    #[test]
    fn totals_sum_all_flows() {
        let mut agg = FlowAggregator::new();
        agg.ingest(&pkt("10.0.0.1", 5000, "1.1.1.1", 443, Protocol::Tcp, 100, Direction::Outbound));
        agg.ingest(&pkt("1.1.1.1", 443, "10.0.0.1", 5000, Protocol::Tcp, 200, Direction::Inbound));
        agg.ingest(&pkt("10.0.0.1", 6000, "8.8.8.8", 53, Protocol::Udp, 50, Direction::Outbound));
        let t = agg.totals();
        assert_eq!(t.bytes_up, 150);
        assert_eq!(t.bytes_down, 200);
        assert_eq!(t.packets_up, 2);
        assert_eq!(t.packets_down, 1);
    }

    #[test]
    fn top_hosts_orders_by_total_bytes() {
        let mut agg = FlowAggregator::new();
        agg.ingest(&pkt("10.0.0.1", 5000, "1.1.1.1", 443, Protocol::Tcp, 100, Direction::Outbound));
        agg.ingest(&pkt("10.0.0.1", 5001, "8.8.8.8", 443, Protocol::Tcp, 5000, Direction::Outbound));
        agg.ingest(&pkt("10.0.0.1", 5002, "9.9.9.9", 443, Protocol::Tcp, 800, Direction::Outbound));
        let top = agg.top_hosts(2);
        assert_eq!(top.len(), 2);
        assert_eq!(top[0].ip, "8.8.8.8");
        assert_eq!(top[1].ip, "9.9.9.9");
    }

    #[test]
    fn top_protocols_groups_correctly() {
        let mut agg = FlowAggregator::new();
        agg.ingest(&pkt("10.0.0.1", 5000, "1.1.1.1", 443, Protocol::Tcp, 100, Direction::Outbound));
        agg.ingest(&pkt("10.0.0.1", 5001, "8.8.8.8", 443, Protocol::Tcp, 200, Direction::Outbound));
        agg.ingest(&pkt("10.0.0.1", 6000, "8.8.8.8", 53, Protocol::Udp, 1000, Direction::Outbound));
        let top = agg.top_protocols(5);
        assert_eq!(top[0].protocol, Protocol::Udp);
        assert_eq!(top[0].bytes, 1000);
        assert_eq!(top[1].protocol, Protocol::Tcp);
        assert_eq!(top[1].bytes, 300);
    }

    #[test]
    fn top_ports_uses_dst_port() {
        let mut agg = FlowAggregator::new();
        agg.ingest(&pkt("10.0.0.1", 5000, "1.1.1.1", 443, Protocol::Tcp, 100, Direction::Outbound));
        agg.ingest(&pkt("10.0.0.1", 5001, "8.8.8.8", 443, Protocol::Tcp, 200, Direction::Outbound));
        agg.ingest(&pkt("10.0.0.1", 6000, "8.8.8.8", 53, Protocol::Udp, 50, Direction::Outbound));
        let top = agg.top_ports(5);
        assert_eq!(top[0].port, 443);
        assert_eq!(top[1].port, 53);
    }

    #[test]
    fn top_countries_skips_unenriched_flows() {
        let mut agg = FlowAggregator::new();
        agg.ingest(&pkt("10.0.0.1", 5000, "1.1.1.1", 443, Protocol::Tcp, 100, Direction::Outbound));
        // No country enrichment yet → empty result.
        let top = agg.top_countries(5);
        assert!(top.is_empty());
    }
}
