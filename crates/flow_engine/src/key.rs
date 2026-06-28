//! Canonical bidirectional flow key.
//!
//! Two packets with swapped (src, dst) endpoints map to the same key, so the
//! aggregator buckets request and reply traffic together.

use shared_types::{PacketEvent, Protocol};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FlowKey {
    pub protocol: Protocol,
    pub a_ip: String,
    pub a_port: Option<u16>,
    pub b_ip: String,
    pub b_port: Option<u16>,
}

impl FlowKey {
    pub fn from_packet(event: &PacketEvent) -> Self {
        let a = (event.src_ip.as_str(), event.src_port);
        let b = (event.dst_ip.as_str(), event.dst_port);
        let (a, b) = if (a.0, a.1) <= (b.0, b.1) { (a, b) } else { (b, a) };
        Self {
            protocol: event.protocol,
            a_ip: a.0.to_string(),
            a_port: a.1,
            b_ip: b.0.to_string(),
            b_port: b.1,
        }
    }

    pub fn id(&self) -> String {
        let proto = match self.protocol {
            Protocol::Tcp => "tcp",
            Protocol::Udp => "udp",
            Protocol::Icmp => "icmp",
            Protocol::Other => "other",
        };
        let ap = self.a_port.map_or(String::from("-"), |p| p.to_string());
        let bp = self.b_port.map_or(String::from("-"), |p| p.to_string());
        format!("{}:{}:{}-{}:{}", proto, self.a_ip, ap, self.b_ip, bp)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared_types::Direction;

    fn pkt(src_ip: &str, src_port: u16, dst_ip: &str, dst_port: u16) -> PacketEvent {
        PacketEvent {
            timestamp: 0,
            src_ip: src_ip.into(),
            dst_ip: dst_ip.into(),
            src_port: Some(src_port),
            dst_port: Some(dst_port),
            protocol: Protocol::Tcp,
            packet_length: 0,
            direction: Direction::Unknown,
        }
    }

    #[test]
    fn swapped_endpoints_produce_same_key() {
        let a = FlowKey::from_packet(&pkt("10.0.0.1", 5000, "8.8.8.8", 443));
        let b = FlowKey::from_packet(&pkt("8.8.8.8", 443, "10.0.0.1", 5000));
        assert_eq!(a, b);
    }

    #[test]
    fn id_is_stable_for_canonical_key() {
        let a = FlowKey::from_packet(&pkt("10.0.0.1", 5000, "8.8.8.8", 443));
        let b = FlowKey::from_packet(&pkt("8.8.8.8", 443, "10.0.0.1", 5000));
        assert_eq!(a.id(), b.id());
    }
}
