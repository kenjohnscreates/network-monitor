//! Flow aggregation engine.
//!
//! Groups [`PacketEvent`]s into bidirectional flows keyed by
//! `(protocol, endpoint_a, endpoint_b)` where the endpoints are
//! lexicographically ordered so traffic in either direction lands in the
//! same bucket.
//!
//! The "up" / "down" labels follow [`Direction`]:
//! - `Outbound` packet → `bytes_up` / `packets_up`
//! - `Inbound` packet → `bytes_down` / `packets_down`
//! - `Unknown` → counted as `up` (we don't try to guess; UI can still show totals)

mod anomaly;
mod key;
mod summary;


use std::collections::HashMap;

use shared_types::{Direction, FlowRecord, PacketEvent, Protocol};

pub use anomaly::evaluate_anomalies;
pub use key::FlowKey;
pub use summary::{
    top_countries, top_hosts, top_ports, top_protocols, totals, CountryStat, HostStat, PortStat,
    ProtocolStat, Totals,
};

/// Default time after which a flow with no new packets is considered stale
/// and removed from the aggregator. PRD §22 Phase 4 calls this configurable.
pub const DEFAULT_STALE_MS: i64 = 60_000;

#[derive(Debug, Clone)]
struct FlowState {
    /// The endpoint that sent the first packet we attribute as outbound, or
    /// the canonical "a" endpoint if direction is never resolved.
    up_ip: String,
    up_port: Option<u16>,
    down_ip: String,
    down_port: Option<u16>,
    protocol: Protocol,
    bytes_up: u64,
    bytes_down: u64,
    packets_up: u64,
    packets_down: u64,
    first_seen: i64,
    last_seen: i64,
}

impl FlowState {
    fn to_record(&self, key: &FlowKey) -> FlowRecord {
        FlowRecord {
            id: key.id(),
            src_ip: self.up_ip.clone(),
            dst_ip: self.down_ip.clone(),
            hostname: None,
            country: None,
            src_port: self.up_port,
            dst_port: self.down_port,
            protocol: self.protocol,
            bytes_up: self.bytes_up,
            bytes_down: self.bytes_down,
            packets_up: self.packets_up,
            packets_down: self.packets_down,
            first_seen: self.first_seen,
            last_seen: self.last_seen,
        }
    }

    fn total_bytes(&self) -> u64 {
        self.bytes_up.saturating_add(self.bytes_down)
    }
}

#[derive(Debug, Clone)]
pub struct FlowAggregator {
    flows: HashMap<FlowKey, FlowState>,
    stale_after_ms: i64,
}

impl Default for FlowAggregator {
    fn default() -> Self {
        Self::new()
    }
}

impl FlowAggregator {
    pub fn new() -> Self {
        Self::with_stale_after_ms(DEFAULT_STALE_MS)
    }

    pub fn with_stale_after_ms(stale_after_ms: i64) -> Self {
        Self {
            flows: HashMap::new(),
            stale_after_ms,
        }
    }

    pub fn flow_count(&self) -> usize {
        self.flows.len()
    }

    /// Number of milliseconds after which a flow is considered stale.
    pub fn stale_after_ms(&self) -> i64 {
        self.stale_after_ms
    }

    /// Update the stale-after threshold (next call to [`Self::cleanup_stale`] uses it).
    pub fn set_stale_after_ms(&mut self, ms: i64) {
        self.stale_after_ms = ms;
    }

    /// Ingest one parsed packet event into the aggregator.
    pub fn ingest(&mut self, event: &PacketEvent) {
        let key = FlowKey::from_packet(event);
        let len = event.packet_length as u64;

        let entry = self.flows.entry(key).or_insert_with(|| {
            // First packet: pick "up" side based on direction.
            let (up_ip, up_port, down_ip, down_port) = match event.direction {
                Direction::Inbound => (
                    event.dst_ip.clone(),
                    event.dst_port,
                    event.src_ip.clone(),
                    event.src_port,
                ),
                _ => (
                    event.src_ip.clone(),
                    event.src_port,
                    event.dst_ip.clone(),
                    event.dst_port,
                ),
            };
            FlowState {
                up_ip,
                up_port,
                down_ip,
                down_port,
                protocol: event.protocol,
                bytes_up: 0,
                bytes_down: 0,
                packets_up: 0,
                packets_down: 0,
                first_seen: event.timestamp,
                last_seen: event.timestamp,
            }
        });

        // Determine whether this packet is going up or down for *this* flow.
        let is_up = packet_is_up(event, entry);
        if is_up {
            entry.bytes_up = entry.bytes_up.saturating_add(len);
            entry.packets_up = entry.packets_up.saturating_add(1);
        } else {
            entry.bytes_down = entry.bytes_down.saturating_add(len);
            entry.packets_down = entry.packets_down.saturating_add(1);
        }

        if event.timestamp < entry.first_seen {
            entry.first_seen = event.timestamp;
        }
        if event.timestamp > entry.last_seen {
            entry.last_seen = event.timestamp;
        }
    }

    /// Snapshot all current flows as a vec of [`FlowRecord`]s.
    pub fn snapshot(&self) -> Vec<FlowRecord> {
        self.flows
            .iter()
            .map(|(k, v)| v.to_record(k))
            .collect()
    }

    /// Snapshot the top `n` flows by total bytes.
    pub fn snapshot_top(&self, n: usize) -> Vec<FlowRecord> {
        let mut items: Vec<(&FlowKey, &FlowState)> = self.flows.iter().collect();
        items.sort_by(|a, b| b.1.total_bytes().cmp(&a.1.total_bytes()));
        items
            .into_iter()
            .take(n)
            .map(|(k, v)| v.to_record(k))
            .collect()
    }

    /// Drop flows whose `last_seen` is older than `now_ms - stale_after_ms`.
    /// Returns the number of removed flows.
    pub fn cleanup_stale(&mut self, now_ms: i64) -> usize {
        let cutoff = now_ms.saturating_sub(self.stale_after_ms);
        let before = self.flows.len();
        self.flows.retain(|_, v| v.last_seen >= cutoff);
        before - self.flows.len()
    }

    /// Reset the aggregator. Useful when the user starts a new capture session.
    pub fn clear(&mut self) {
        self.flows.clear();
    }
}

/// Decide whether `event` should be counted as "up" for an existing flow
/// state. We compare by source endpoint identity rather than relying on the
/// packet's `direction` field, because reply packets have the opposite
/// `Direction` from the first packet we saw.
fn packet_is_up(event: &PacketEvent, state: &FlowState) -> bool {
    let src_matches_up = event.src_ip == state.up_ip && event.src_port == state.up_port;
    if src_matches_up {
        return true;
    }
    let src_matches_down = event.src_ip == state.down_ip && event.src_port == state.down_port;
    if src_matches_down {
        return false;
    }
    // Shouldn't happen: same flow key, but neither endpoint matches.
    // Fall back to direction hint, defaulting to up.
    !matches!(event.direction, Direction::Inbound)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pkt(
        ts: i64,
        src_ip: &str,
        dst_ip: &str,
        src_port: u16,
        dst_port: u16,
        proto: Protocol,
        len: u32,
        dir: Direction,
    ) -> PacketEvent {
        PacketEvent {
            timestamp: ts,
            src_ip: src_ip.into(),
            dst_ip: dst_ip.into(),
            src_port: Some(src_port),
            dst_port: Some(dst_port),
            protocol: proto,
            packet_length: len,
            direction: dir,
        }
    }

    #[test]
    fn single_packet_creates_one_flow() {
        let mut agg = FlowAggregator::new();
        agg.ingest(&pkt(1, "10.0.0.1", "8.8.8.8", 5000, 443, Protocol::Tcp, 100, Direction::Outbound));
        assert_eq!(agg.flow_count(), 1);
        let snap = agg.snapshot();
        assert_eq!(snap.len(), 1);
        let f = &snap[0];
        assert_eq!(f.src_ip, "10.0.0.1");
        assert_eq!(f.dst_ip, "8.8.8.8");
        assert_eq!(f.bytes_up, 100);
        assert_eq!(f.bytes_down, 0);
        assert_eq!(f.packets_up, 1);
        assert_eq!(f.first_seen, 1);
        assert_eq!(f.last_seen, 1);
    }

    #[test]
    fn same_direction_packets_accumulate() {
        let mut agg = FlowAggregator::new();
        agg.ingest(&pkt(1, "10.0.0.1", "8.8.8.8", 5000, 443, Protocol::Tcp, 100, Direction::Outbound));
        agg.ingest(&pkt(2, "10.0.0.1", "8.8.8.8", 5000, 443, Protocol::Tcp, 200, Direction::Outbound));
        let snap = agg.snapshot();
        assert_eq!(snap.len(), 1);
        assert_eq!(snap[0].bytes_up, 300);
        assert_eq!(snap[0].packets_up, 2);
        assert_eq!(snap[0].last_seen, 2);
    }

    #[test]
    fn reply_packet_lands_on_same_flow_and_increments_down() {
        let mut agg = FlowAggregator::new();
        agg.ingest(&pkt(1, "10.0.0.1", "8.8.8.8", 5000, 443, Protocol::Tcp, 100, Direction::Outbound));
        agg.ingest(&pkt(2, "8.8.8.8", "10.0.0.1", 443, 5000, Protocol::Tcp, 250, Direction::Inbound));
        assert_eq!(agg.flow_count(), 1);
        let f = &agg.snapshot()[0];
        assert_eq!(f.bytes_up, 100);
        assert_eq!(f.bytes_down, 250);
        assert_eq!(f.packets_up, 1);
        assert_eq!(f.packets_down, 1);
        assert_eq!(f.first_seen, 1);
        assert_eq!(f.last_seen, 2);
    }

    #[test]
    fn different_protocol_is_different_flow() {
        let mut agg = FlowAggregator::new();
        agg.ingest(&pkt(1, "10.0.0.1", "8.8.8.8", 5000, 443, Protocol::Tcp, 100, Direction::Outbound));
        agg.ingest(&pkt(2, "10.0.0.1", "8.8.8.8", 5000, 443, Protocol::Udp, 100, Direction::Outbound));
        assert_eq!(agg.flow_count(), 2);
    }

    #[test]
    fn different_ports_are_different_flows() {
        let mut agg = FlowAggregator::new();
        agg.ingest(&pkt(1, "10.0.0.1", "8.8.8.8", 5000, 443, Protocol::Tcp, 100, Direction::Outbound));
        agg.ingest(&pkt(2, "10.0.0.1", "8.8.8.8", 5001, 443, Protocol::Tcp, 100, Direction::Outbound));
        assert_eq!(agg.flow_count(), 2);
    }

    #[test]
    fn cleanup_stale_removes_old_flows() {
        let mut agg = FlowAggregator::with_stale_after_ms(1_000);
        agg.ingest(&pkt(1_000, "10.0.0.1", "8.8.8.8", 5000, 443, Protocol::Tcp, 100, Direction::Outbound));
        agg.ingest(&pkt(5_000, "10.0.0.1", "1.1.1.1", 5001, 443, Protocol::Tcp, 100, Direction::Outbound));
        // now=5500: cutoff=4500, so the 1000ms flow is stale, the 5000ms is not.
        let removed = agg.cleanup_stale(5_500);
        assert_eq!(removed, 1);
        assert_eq!(agg.flow_count(), 1);
    }

    #[test]
    fn snapshot_top_sorts_by_total_bytes() {
        let mut agg = FlowAggregator::new();
        agg.ingest(&pkt(1, "10.0.0.1", "1.1.1.1", 5000, 443, Protocol::Tcp, 100, Direction::Outbound));
        agg.ingest(&pkt(2, "10.0.0.1", "8.8.8.8", 5001, 443, Protocol::Tcp, 5000, Direction::Outbound));
        agg.ingest(&pkt(3, "10.0.0.1", "9.9.9.9", 5002, 443, Protocol::Tcp, 800, Direction::Outbound));
        let top = agg.snapshot_top(2);
        assert_eq!(top.len(), 2);
        assert_eq!(top[0].dst_ip, "8.8.8.8");
        assert_eq!(top[1].dst_ip, "9.9.9.9");
    }

    #[test]
    fn unknown_direction_packet_counts_as_up() {
        let mut agg = FlowAggregator::new();
        agg.ingest(&pkt(1, "10.0.0.1", "8.8.8.8", 5000, 443, Protocol::Tcp, 100, Direction::Unknown));
        let f = &agg.snapshot()[0];
        assert_eq!(f.bytes_up, 100);
        assert_eq!(f.bytes_down, 0);
    }

    #[test]
    fn clear_resets_flows() {
        let mut agg = FlowAggregator::new();
        agg.ingest(&pkt(1, "10.0.0.1", "8.8.8.8", 5000, 443, Protocol::Tcp, 100, Direction::Outbound));
        agg.clear();
        assert_eq!(agg.flow_count(), 0);
    }
}
