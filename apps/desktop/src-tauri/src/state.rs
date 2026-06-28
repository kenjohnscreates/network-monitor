//! Shared application state.
//!
//! Held in Tauri's managed state so commands can access it via `State<AppState>`.

use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use capture_core::CaptureSession;
use enrichment::Enricher;
use flow_engine::FlowAggregator;
use serde::{Deserialize, Serialize};
use shared_types::AnomalyRuleSettings;

use crate::snapshotter::Snapshotter;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub snapshot_interval_ms: u64,
    pub stale_after_ms: i64,
    pub top_n: usize,
    pub reverse_dns_enabled: bool,
    pub geoip_db_path: Option<String>,
    #[serde(default)]
    pub anomalies: AnomalyRuleSettings,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            snapshot_interval_ms: 750,
            stale_after_ms: 60_000,
            top_n: 250,
            reverse_dns_enabled: false,
            geoip_db_path: None,
            anomalies: AnomalyRuleSettings::default(),
        }
    }
}

pub struct AppState {
    pub session: Mutex<Option<CaptureSession>>,
    pub snapshotter: Mutex<Option<Snapshotter>>,
    pub aggregator: Arc<Mutex<FlowAggregator>>,
    pub paused: Arc<AtomicBool>,
    pub settings: Arc<Mutex<Settings>>,
    pub enricher: Arc<Enricher>,
}

impl Default for AppState {
    fn default() -> Self {
        let settings_val = Settings::default();
        let enricher = Arc::new(Enricher::new(settings_val.reverse_dns_enabled));
        if let Some(path) = settings_val.geoip_db_path.as_ref() {
            let _ = enricher.load_geoip(path.into());
        }
        Self {
            session: Mutex::new(None),
            snapshotter: Mutex::new(None),
            aggregator: Arc::new(Mutex::new(FlowAggregator::new())),
            paused: Arc::new(AtomicBool::new(false)),
            settings: Arc::new(Mutex::new(settings_val)),
            enricher,
        }
    }
}
