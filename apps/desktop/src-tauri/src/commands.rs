//! Tauri command surface.

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::Ordering;

use capture_core::{CaptureSession, CaptureStatsSnapshot};
use serde::Serialize;
use shared_types::{FlowRecord, InterfaceInfo, Protocol};
use tauri::{AppHandle, State};

use crate::persist;
use crate::snapshotter::Snapshotter;
use crate::state::{AppState, Settings};

#[derive(Debug, Serialize)]
pub struct CaptureStatus {
    pub running: bool,
    pub paused: bool,
    pub interface: Option<String>,
    pub stats: Option<CaptureStatsSnapshot>,
    pub flow_count: usize,
}

#[tauri::command]
pub fn list_interfaces() -> Result<Vec<InterfaceInfo>, String> {
    capture_core::list_interfaces().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn start_capture(
    interface: String,
    app: AppHandle,
    state: State<AppState>,
) -> Result<(), String> {
    {
        let session = state.session.lock().map_err(|e| e.to_string())?;
        if session.is_some() {
            return Err("a capture session is already running".into());
        }
    }

    let aggregator = state.aggregator.clone();
    let paused = state.paused.clone();
    paused.store(false, Ordering::Relaxed);

    {
        let mut agg = aggregator.lock().map_err(|e| e.to_string())?;
        agg.clear();
        let stale = state
            .settings
            .lock()
            .map_err(|e| e.to_string())?
            .stale_after_ms;
        agg.set_stale_after_ms(stale);
    }

    let agg_ingest = aggregator.clone();
    let paused_ingest = paused.clone();

    let session = CaptureSession::start(&interface, move |event| {
        if paused_ingest.load(Ordering::Relaxed) {
            return;
        }
        if let Ok(mut agg) = agg_ingest.lock() {
            agg.ingest(&event);
        }
    })
    .map_err(|e| e.to_string())?;

    let (interval_ms, top_n) = {
        let s = state.settings.lock().map_err(|e| e.to_string())?;
        (s.snapshot_interval_ms, s.top_n)
    };

    let snap =
        Snapshotter::start(app, aggregator, state.enricher.clone(), interval_ms, top_n, state.settings.clone());

    *state.session.lock().map_err(|e| e.to_string())? = Some(session);
    *state.snapshotter.lock().map_err(|e| e.to_string())? = Some(snap);
    Ok(())
}

#[tauri::command]
pub fn stop_capture(state: State<AppState>) -> Result<(), String> {
    if let Some(snap) = state.snapshotter.lock().map_err(|e| e.to_string())?.take() {
        snap.stop();
    }
    if let Some(session) = state.session.lock().map_err(|e| e.to_string())?.take() {
        session.stop();
    }
    state.paused.store(false, Ordering::Relaxed);
    Ok(())
}

#[tauri::command]
pub fn pause_capture(state: State<AppState>) -> Result<(), String> {
    state.paused.store(true, Ordering::Relaxed);
    Ok(())
}

#[tauri::command]
pub fn resume_capture(state: State<AppState>) -> Result<(), String> {
    state.paused.store(false, Ordering::Relaxed);
    Ok(())
}

#[tauri::command]
pub fn capture_status(state: State<AppState>) -> Result<CaptureStatus, String> {
    let session_guard = state.session.lock().map_err(|e| e.to_string())?;
    let agg_guard = state.aggregator.lock().map_err(|e| e.to_string())?;
    let status = match session_guard.as_ref() {
        Some(session) => CaptureStatus {
            running: true,
            paused: state.paused.load(Ordering::Relaxed),
            interface: Some(session.interface().to_string()),
            stats: Some(session.stats().snapshot()),
            flow_count: agg_guard.flow_count(),
        },
        None => CaptureStatus {
            running: false,
            paused: false,
            interface: None,
            stats: None,
            flow_count: agg_guard.flow_count(),
        },
    };
    Ok(status)
}

#[tauri::command]
pub fn get_settings(state: State<AppState>) -> Result<Settings, String> {
    state
        .settings
        .lock()
        .map(|s| s.clone())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_settings(
    new: Settings,
    app: AppHandle,
    state: State<AppState>,
) -> Result<Settings, String> {
    let mut current = state.settings.lock().map_err(|e| e.to_string())?;
    let prev = current.clone();
    *current = new.clone();
    drop(current);

    // Apply runtime-relevant changes immediately.
    if let Ok(mut agg) = state.aggregator.lock() {
        agg.set_stale_after_ms(new.stale_after_ms);
    }

    state.enricher.set_rdns_enabled(new.reverse_dns_enabled);

    if new.geoip_db_path != prev.geoip_db_path {
        match new.geoip_db_path.as_ref() {
            Some(path) => {
                state
                    .enricher
                    .load_geoip(PathBuf::from(path))
                    .map_err(|e| format!("failed to load GeoIP DB: {e}"))?;
            }
            None => state.enricher.clear_geoip(),
        }
    }

    if let Err(e) = persist::save(&app, &new) {
        eprintln!("warning: failed to persist settings: {e}");
    }
    Ok(new)
}

#[derive(Debug, Serialize)]
pub struct EnrichmentStatus {
    pub cached_entries: usize,
    pub queue_depth: usize,
    pub geoip_loaded: bool,
    pub rdns_enabled: bool,
}

#[tauri::command]
pub fn enrichment_status(state: State<AppState>) -> Result<EnrichmentStatus, String> {
    let s = state.enricher.stats();
    Ok(EnrichmentStatus {
        cached_entries: s.cached_entries,
        queue_depth: s.queue_depth,
        geoip_loaded: s.geoip_loaded,
        rdns_enabled: s.rdns_enabled,
    })
}

#[derive(Debug, Serialize)]
pub struct ExportResult {
    pub path: String,
    pub flow_count: usize,
    pub format: String,
}

#[tauri::command]
pub fn export_session(
    path: String,
    format: String,
    state: State<AppState>,
) -> Result<ExportResult, String> {
    let path_buf = PathBuf::from(&path);
    let agg = state.aggregator.lock().map_err(|e| e.to_string())?;
    let flows = agg.snapshot();

    match format.to_lowercase().as_str() {
        "json" => {
            let body = serde_json::to_string_pretty(&flows).map_err(|e| e.to_string())?;
            fs::write(&path_buf, body).map_err(|e| e.to_string())?;
        }
        "csv" => {
            fs::write(&path_buf, flows_to_csv(&flows)).map_err(|e| e.to_string())?;
        }
        other => return Err(format!("unsupported format: {other}")),
    }

    Ok(ExportResult {
        path,
        flow_count: flows.len(),
        format,
    })
}

fn flows_to_csv(flows: &[FlowRecord]) -> String {
    let mut out = String::from(
        "id,src_ip,src_port,dst_ip,dst_port,protocol,bytes_up,bytes_down,packets_up,packets_down,hostname,country,first_seen,last_seen\n",
    );
    for f in flows {
        out.push_str(&format!(
            "{},{},{},{},{},{},{},{},{},{},{},{},{},{}\n",
            csv_escape(&f.id),
            csv_escape(&f.src_ip),
            f.src_port.map_or(String::new(), |p| p.to_string()),
            csv_escape(&f.dst_ip),
            f.dst_port.map_or(String::new(), |p| p.to_string()),
            protocol_to_str(f.protocol),
            f.bytes_up,
            f.bytes_down,
            f.packets_up,
            f.packets_down,
            csv_escape(f.hostname.as_deref().unwrap_or("")),
            csv_escape(f.country.as_deref().unwrap_or("")),
            f.first_seen,
            f.last_seen,
        ));
    }
    out
}

fn protocol_to_str(p: Protocol) -> &'static str {
    match p {
        Protocol::Tcp => "TCP",
        Protocol::Udp => "UDP",
        Protocol::Icmp => "ICMP",
        Protocol::Other => "OTHER",
    }
}

fn csv_escape(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}
