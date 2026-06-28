//! Shared data contracts crossing the Tauri bridge.
//!
//! Everything here must derive `Serialize` + `Deserialize` so the same struct
//! can be emitted by Rust and consumed by the React UI as a TypeScript object.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    Inbound,
    Outbound,
    Unknown,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "UPPERCASE")]
pub enum Protocol {
    Tcp,
    Udp,
    Icmp,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterfaceInfo {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub usable: bool,
}

/// One parsed packet flowing through the capture pipeline.
/// Payload bytes are intentionally never captured.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PacketEvent {
    /// Unix epoch milliseconds.
    pub timestamp: i64,
    pub src_ip: String,
    pub dst_ip: String,
    pub src_port: Option<u16>,
    pub dst_port: Option<u16>,
    pub protocol: Protocol,
    pub packet_length: u32,
    pub direction: Direction,
}

/// Aggregated bidirectional flow keyed by (protocol, src_ip, dst_ip, src_port, dst_port).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowRecord {
    pub id: String,
    pub src_ip: String,
    pub dst_ip: String,
    pub hostname: Option<String>,
    pub country: Option<String>,
    pub src_port: Option<u16>,
    pub dst_port: Option<u16>,
    pub protocol: Protocol,
    pub bytes_up: u64,
    pub bytes_down: u64,
    pub packets_up: u64,
    pub packets_down: u64,
    /// Unix epoch milliseconds.
    pub first_seen: i64,
    pub last_seen: i64,
}

/// Rule toggles persisted with settings (transparent heuristics, not antivirus verdicts).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AnomalyRuleSettings {
    pub rare_port_enabled: bool,
    pub high_upload_enabled: bool,
    pub geo_notice_enabled: bool,
    pub session_new_remote_enabled: bool,
    /// Bytes-up / total when total >= 512 bytes (TCP/UDP only).
    pub upload_ratio_threshold: f64,
    /// Ports never flagged as rare (merged with builtin common list server-side).
    pub rare_extra_allowlist: Vec<u16>,
}

impl Default for AnomalyRuleSettings {
    fn default() -> Self {
        Self {
            rare_port_enabled: true,
            high_upload_enabled: true,
            geo_notice_enabled: false,
            session_new_remote_enabled: true,
            upload_ratio_threshold: 0.88,
            rare_extra_allowlist: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FlowAnomalySeverity {
    Info,
    Notice,
    Warn,
}

/// One surfaced heuristic finding tied to an aggregated [`FlowRecord`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowAnomaly {
    pub id: String,
    pub rule_id: String,
    pub severity: FlowAnomalySeverity,
    pub flow_id: String,
    pub message: String,
}
