//! Transparent heuristics — not malware verdicts.

use std::collections::HashSet;
use std::net::{Ipv4Addr, Ipv6Addr};

use shared_types::{
    AnomalyRuleSettings, FlowAnomaly, FlowAnomalySeverity, FlowRecord, Protocol,
};

const COMMON_PORTS: &[u16] = &[
    20, 21, 22, 23, 25, 53, 80, 110, 123, 143, 443, 465, 587, 993, 995, 8080,
    8443, 853, 5228,
];

/// Evaluates heuristic rules across the entire flow snapshot (pre top-N truncation in UI).
#[must_use]
pub fn evaluate_anomalies(
    flows: &[FlowRecord],
    rules: &AnomalyRuleSettings,
    session_seen_remotes: &mut HashSet<String>,
) -> Vec<FlowAnomaly> {
    let mut out = Vec::new();

    for f in flows {
        if rules.rare_port_enabled {
            if let Some(a) = eval_rare_port(f, rules) {
                out.push(a);
            }
        }
        if rules.high_upload_enabled {
            if let Some(a) = eval_high_upload(f, rules) {
                out.push(a);
            }
        }
        if rules.geo_notice_enabled {
            if let Some(a) = eval_geo_notice(f) {
                out.push(a);
            }
        }
        if rules.session_new_remote_enabled {
            if let Some(a) = eval_session_new_remote(f, session_seen_remotes) {
                out.push(a);
            }
        }
    }

    if out.len() > 300 {
        out.truncate(300);
    }

    out.sort_by_key(|a| match a.severity {
        FlowAnomalySeverity::Warn => 0,
        FlowAnomalySeverity::Notice => 1,
        FlowAnomalySeverity::Info => 2,
    });
    out
}

fn format_proto(p: Protocol) -> &'static str {
    match p {
        Protocol::Tcp => "tcp",
        Protocol::Udp => "udp",
        Protocol::Icmp => "icmp",
        Protocol::Other => "other",
    }
}

fn is_allowed_dst_port(dp: u16, rules: &AnomalyRuleSettings) -> bool {
    COMMON_PORTS.contains(&dp)
        || rules
            .rare_extra_allowlist
            .iter()
            .copied()
            .any(|p| p == dp)
}

fn classify_v4(ip: &str) -> Option<Ipv4Addr> {
    ip.parse::<Ipv4Addr>().ok()
}

fn classify_v6(ip: &str) -> Option<Ipv6Addr> {
    let s = ip.trim_matches(|c| c == '[' || c == ']');
    s.parse::<Ipv6Addr>().ok()
}

fn is_reasonable_inet_target(ip: &str) -> bool {
    if ip.contains(':') {
        let Some(ip6) = classify_v6(ip) else {
            return false;
        };
        !(ip6.is_loopback()
            || ip6.is_unique_local()
            || ip6.is_unicast_link_local()
            || ip6.is_multicast())
    } else if let Some(ip4) = classify_v4(ip) {
        !(ip4.is_loopback()
            || ip4.is_broadcast()
            || ip4.is_multicast()
            || ip4.is_private()
            || ip4.octets()[0] == 0)
    } else {
        false
    }
}

fn eval_rare_port(f: &FlowRecord, rules: &AnomalyRuleSettings) -> Option<FlowAnomaly> {
    if !matches!(f.protocol, Protocol::Tcp | Protocol::Udp) {
        return None;
    }
    let dp = f.dst_port?;
    if is_allowed_dst_port(dp, rules) {
        return None;
    }
    let remote = f.dst_ip.as_str();
    if !is_reasonable_inet_target(remote) {
        return None;
    }

    Some(FlowAnomaly {
        id: format!("rare_port-{}", f.id),
        rule_id: "rare_destination_port".into(),
        severity: FlowAnomalySeverity::Notice,
        flow_id: f.id.clone(),
        message: format!(
            "Uncommon destination port {dp} ({}) toward {remote} — verify it matches an app you trust.",
            format_proto(f.protocol),
        ),
    })
}

fn eval_high_upload(f: &FlowRecord, rules: &AnomalyRuleSettings) -> Option<FlowAnomaly> {
    if !matches!(f.protocol, Protocol::Tcp | Protocol::Udp) {
        return None;
    }
    let total = f.bytes_up.saturating_add(f.bytes_down);
    if total < 512 {
        return None;
    }
    let ratio = f.bytes_up as f64 / total as f64;
    if ratio < rules.upload_ratio_threshold {
        return None;
    }
    let remote = f.dst_ip.as_str();
    if !is_reasonable_inet_target(remote) {
        return None;
    }

    Some(FlowAnomaly {
        id: format!("upload_ratio-{}", f.id),
        rule_id: "high_upload_share".into(),
        severity: FlowAnomalySeverity::Notice,
        flow_id: f.id.clone(),
        message: format!(
            "~{pct:.0}% of bytes leaving your machine toward {remote} are upload-heavy ({proto}) — can be benign sync/backups.",
            pct = ratio * 100.0,
            proto = format_proto(f.protocol),
        ),
    })
}

fn eval_geo_notice(f: &FlowRecord) -> Option<FlowAnomaly> {
    let cc = f.country.as_deref()?.trim();
    if cc.is_empty() {
        return None;
    }
    let dp = f.dst_port?;
    let web_ok = dp == 80 || dp == 443 || dp == 8080 || dp == 8443;
    if web_ok || dp == 53 || dp == 853 {
        return None;
    }

    let remote = f.dst_ip.as_str();
    Some(FlowAnomaly {
        id: format!("geo-{}", f.id),
        rule_id: "geo_non_web_service_port".into(),
        severity: FlowAnomalySeverity::Info,
        flow_id: f.id.clone(),
        message: format!(
            "Geo tags {remote} ({cc}), but destination port={dp}/{proto} is not a typical CDN/DNS pairing.",
            proto = format_proto(f.protocol),
        ),
    })
}

fn eval_session_new_remote(
    f: &FlowRecord,
    seen: &mut HashSet<String>,
) -> Option<FlowAnomaly> {
    let remote = f.dst_ip.as_str();
    if !is_reasonable_inet_target(remote) {
        return None;
    }
    let first = seen.insert(remote.to_string());
    first.then_some(FlowAnomaly {
        id: format!("new_remote-{}", f.id),
        rule_id: "session_new_remote".into(),
        severity: FlowAnomalySeverity::Info,
        flow_id: f.id.clone(),
        message: format!("First observation this capture session toward {remote}.",),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared_types::Protocol;

    fn sample_tcp_flow(port: u16, up: u64, down: u64) -> FlowRecord {
        FlowRecord {
            id: "t1".into(),
            src_ip: "192.168.1.50".into(),
            dst_ip: "8.8.8.8".into(),
            hostname: None,
            country: None,
            src_port: Some(50_000),
            dst_port: Some(port),
            protocol: Protocol::Tcp,
            bytes_up: up,
            bytes_down: down,
            packets_up: 1,
            packets_down: 1,
            first_seen: 0,
            last_seen: 0,
        }
    }

    #[test]
    fn rare_port_triggers() {
        let f = sample_tcp_flow(1337, 100, 100);
        let mut seen = HashSet::new();
        let rules = AnomalyRuleSettings::default();
        let out = evaluate_anomalies(&[f], &rules, &mut seen);
        assert!(out.iter().any(|a| a.rule_id == "rare_destination_port"));
    }

    #[test]
    fn high_upload_triggers() {
        let f = sample_tcp_flow(443, 9000, 200);
        let mut seen = HashSet::new();
        let rules = AnomalyRuleSettings::default();
        let out = evaluate_anomalies(&[f], &rules, &mut seen);
        assert!(out.iter().any(|a| a.rule_id == "high_upload_share"));
    }
}
