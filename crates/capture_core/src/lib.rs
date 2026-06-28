//! Raw packet capture.
//!
//! Phase 2: interface enumeration via `list_interfaces`.
//! Phase 3: live capture loop via [`CaptureSession`] emitting [`shared_types::PacketEvent`]s.

pub mod capture;
pub mod direction;
pub mod parse;

use shared_types::InterfaceInfo;
use thiserror::Error;

pub use capture::{CaptureError as SessionError, CaptureSession, CaptureStatsSnapshot};

#[derive(Debug, Error)]
pub enum ListError {
    #[error("pcap error: {0}")]
    Pcap(#[from] pcap::Error),
}

/// Enumerate available network interfaces.
pub fn list_interfaces() -> Result<Vec<InterfaceInfo>, ListError> {
    let devices = pcap::Device::list()?;
    Ok(devices.into_iter().map(map_device).collect())
}

fn map_device(d: pcap::Device) -> InterfaceInfo {
    let usable = is_usable(&d.flags);
    let description = d.desc.filter(|s| !s.is_empty());
    InterfaceInfo {
        id: d.name.clone(),
        name: d.name,
        description,
        usable,
    }
}

fn is_usable(flags: &pcap::DeviceFlags) -> bool {
    use pcap::ConnectionStatus;
    let connected = matches!(
        flags.connection_status,
        ConnectionStatus::Connected | ConnectionStatus::Unknown
    );
    let up = flags.is_up();
    connected && up
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_interfaces_does_not_panic() {
        let _ = list_interfaces();
    }
}
