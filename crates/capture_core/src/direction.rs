//! Inbound/outbound classification.
//!
//! Phase 3 keeps this dead simple: if the packet's `src_ip` matches one of the
//! local addresses bound to the capture interface, treat it as outbound.
//! If the `dst_ip` matches, inbound. Anything else is `Unknown`
//! (e.g. broadcast/multicast or two non-local hosts on a span port).

use shared_types::Direction;
use std::collections::HashSet;

#[derive(Debug, Clone, Default)]
pub struct DirectionResolver {
    local_ips: HashSet<String>,
}

impl DirectionResolver {
    pub fn new(local_ips: Vec<String>) -> Self {
        Self { local_ips: local_ips.into_iter().collect() }
    }

    pub fn classify(&self, src_ip: &str, dst_ip: &str) -> Direction {
        let src_local = self.local_ips.contains(src_ip);
        let dst_local = self.local_ips.contains(dst_ip);
        match (src_local, dst_local) {
            (true, _) => Direction::Outbound,
            (false, true) => Direction::Inbound,
            _ => Direction::Unknown,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outbound_when_src_is_local() {
        let r = DirectionResolver::new(vec!["192.168.1.10".into()]);
        assert_eq!(r.classify("192.168.1.10", "1.1.1.1"), Direction::Outbound);
    }

    #[test]
    fn inbound_when_dst_is_local() {
        let r = DirectionResolver::new(vec!["192.168.1.10".into()]);
        assert_eq!(r.classify("1.1.1.1", "192.168.1.10"), Direction::Inbound);
    }

    #[test]
    fn unknown_when_neither_local() {
        let r = DirectionResolver::new(vec!["192.168.1.10".into()]);
        assert_eq!(r.classify("8.8.8.8", "1.1.1.1"), Direction::Unknown);
    }
}
