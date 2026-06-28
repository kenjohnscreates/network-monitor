//! Minimal packet metadata parser.
//!
//! Intentionally tiny: extract only the fields listed in PRD §22 PacketEvent.
//! No payload inspection. Anything we can't parse is returned as `None` so
//! the capture loop can drop it without crashing.

use etherparse::{NetSlice, SlicedPacket, TransportSlice};
use shared_types::{PacketEvent, Protocol};

use crate::direction::DirectionResolver;

/// Subset of pcap data link types we know how to parse.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataLink {
    /// Ethernet (`DLT_EN10MB` = 1)
    Ethernet,
    /// macOS loopback (`DLT_NULL` = 0): 4-byte host-order address family prefix
    Null,
    /// BSD loopback (`DLT_LOOP` = 12): 4-byte big-endian address family prefix
    Loop,
    /// Raw IP (`DLT_RAW` = 12 on Linux / `DLT_IPV4` / `DLT_IPV6`)
    Raw,
    /// Anything else we won't try to parse.
    Unsupported,
}

impl DataLink {
    pub fn from_pcap(linktype: pcap::Linktype) -> Self {
        match linktype.0 {
            1 => Self::Ethernet,
            0 => Self::Null,
            12 => Self::Loop, // pcap uses DLT_LOOP=12 on BSDs; Linux DLT_RAW=12 too — handled identically (4-byte prefix is BE family)
            228 | 229 => Self::Raw, // DLT_IPV4 / DLT_IPV6
            _ => Self::Unsupported,
        }
    }
}

/// Parse a captured frame into a [`PacketEvent`].
///
/// Returns `None` for any traffic we don't currently support
/// (non-IP, malformed, unsupported link layer, etc).
pub fn parse_packet(
    timestamp_ms: i64,
    captured_len: u32,
    raw: &[u8],
    link: DataLink,
    direction_resolver: &DirectionResolver,
) -> Option<PacketEvent> {
    let payload = strip_link_layer(raw, link)?;
    let sliced = SlicedPacket::from_ip(payload).ok()?;

    let (src_ip, dst_ip) = match sliced.net {
        Some(NetSlice::Ipv4(ipv4)) => {
            let h = ipv4.header();
            (
                std::net::IpAddr::V4(h.source_addr()).to_string(),
                std::net::IpAddr::V4(h.destination_addr()).to_string(),
            )
        }
        Some(NetSlice::Ipv6(ipv6)) => {
            let h = ipv6.header();
            (
                std::net::IpAddr::V6(h.source_addr()).to_string(),
                std::net::IpAddr::V6(h.destination_addr()).to_string(),
            )
        }
        _ => return None,
    };

    let (protocol, src_port, dst_port) = match sliced.transport {
        Some(TransportSlice::Tcp(tcp)) => {
            (Protocol::Tcp, Some(tcp.source_port()), Some(tcp.destination_port()))
        }
        Some(TransportSlice::Udp(udp)) => {
            (Protocol::Udp, Some(udp.source_port()), Some(udp.destination_port()))
        }
        Some(TransportSlice::Icmpv4(_)) | Some(TransportSlice::Icmpv6(_)) => {
            (Protocol::Icmp, None, None)
        }
        _ => (Protocol::Other, None, None),
    };

    let direction = direction_resolver.classify(&src_ip, &dst_ip);

    Some(PacketEvent {
        timestamp: timestamp_ms,
        src_ip,
        dst_ip,
        src_port,
        dst_port,
        protocol,
        packet_length: captured_len,
        direction,
    })
}

/// Strip the link-layer framing so the returned slice starts at the IP header.
fn strip_link_layer(raw: &[u8], link: DataLink) -> Option<&[u8]> {
    match link {
        DataLink::Ethernet => {
            // Use etherparse to walk the ethernet header (handles VLAN tags too).
            let sliced = SlicedPacket::from_ethernet(raw).ok()?;
            // Reconstruct the IP slice by reading the offset of the net layer.
            let net = sliced.net?;
            let ip_bytes_start = match net {
                NetSlice::Ipv4(ipv4) => ipv4.header().slice().as_ptr(),
                NetSlice::Ipv6(ipv6) => ipv6.header().slice().as_ptr(),
                _ => return None,
            };
            // Compute offset from raw start to the IP header.
            let offset = (ip_bytes_start as usize).checked_sub(raw.as_ptr() as usize)?;
            raw.get(offset..)
        }
        DataLink::Null | DataLink::Loop => raw.get(4..),
        DataLink::Raw => Some(raw),
        DataLink::Unsupported => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared_types::Direction;

    #[test]
    fn unsupported_link_returns_none() {
        let resolver = DirectionResolver::new(Vec::new());
        let result = parse_packet(0, 0, &[], DataLink::Unsupported, &resolver);
        assert!(result.is_none());
    }

    #[test]
    fn empty_ethernet_returns_none() {
        let resolver = DirectionResolver::new(Vec::new());
        let result = parse_packet(0, 0, &[], DataLink::Ethernet, &resolver);
        assert!(result.is_none());
    }

    #[test]
    fn truncated_null_returns_none() {
        let resolver = DirectionResolver::new(Vec::new());
        let result = parse_packet(0, 0, &[1, 2], DataLink::Null, &resolver);
        assert!(result.is_none());
    }

    #[test]
    fn parses_simple_ipv4_udp_ethernet_frame() {
        // Hand-crafted minimal Ethernet/IPv4/UDP frame: src 10.0.0.1 -> 10.0.0.2 dport 53.
        use etherparse::PacketBuilder;
        let payload = [0u8; 4];
        let builder = PacketBuilder::ethernet2(
            [1, 2, 3, 4, 5, 6],
            [6, 5, 4, 3, 2, 1],
        )
        .ipv4([10, 0, 0, 1], [10, 0, 0, 2], 64)
        .udp(12345, 53);

        let mut frame = Vec::with_capacity(builder.size(payload.len()));
        builder.write(&mut frame, &payload).unwrap();

        let resolver = DirectionResolver::new(vec!["10.0.0.1".into()]);
        let event = parse_packet(
            42,
            frame.len() as u32,
            &frame,
            DataLink::Ethernet,
            &resolver,
        )
        .expect("should parse");

        assert_eq!(event.timestamp, 42);
        assert_eq!(event.src_ip, "10.0.0.1");
        assert_eq!(event.dst_ip, "10.0.0.2");
        assert_eq!(event.src_port, Some(12345));
        assert_eq!(event.dst_port, Some(53));
        assert_eq!(event.protocol, Protocol::Udp);
        assert_eq!(event.direction, Direction::Outbound);
    }
}
