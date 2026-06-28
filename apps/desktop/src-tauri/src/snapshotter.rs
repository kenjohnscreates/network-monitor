//! Throttled snapshot publisher.
//!
//! A dedicated thread wakes on a fixed interval, takes a top-N snapshot of
//! the [`FlowAggregator`] plus a [`DashboardSummary`], and emits both as a
//! `flow_snapshot` Tauri event. This is what keeps the UI from being
//! flooded by raw packet events.

use std::sync::atomic::{AtomicBool, Ordering};
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use enrichment::Enricher;
use flow_engine::{
    self as fe, CountryStat, FlowAggregator, HostStat, PortStat, ProtocolStat, Totals,
};
use serde::Serialize;
use shared_types::{AnomalyRuleSettings, FlowAnomaly, FlowRecord};
use tauri::{AppHandle, Emitter};

const STALE_CLEANUP_INTERVAL_MS: u64 = 5_000;
const TOP_LIST_LEN: usize = 8;

#[derive(Debug, Clone, Serialize)]
pub struct DashboardSummary {
    pub totals: Totals,
    pub packets_per_second: f64,
    pub bytes_per_second_up: f64,
    pub bytes_per_second_down: f64,
    pub top_hosts: Vec<HostStat>,
    pub top_protocols: Vec<ProtocolStat>,
    pub top_ports: Vec<PortStat>,
    pub top_countries: Vec<CountryStat>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FlowSnapshot {
    /// Unix epoch milliseconds.
    pub timestamp: i64,
    pub flow_count: usize,
    pub flows: Vec<FlowRecord>,
    pub summary: DashboardSummary,
    pub anomalies: Vec<FlowAnomaly>,
}

pub struct Snapshotter {
    stop: Arc<AtomicBool>,
    join: Option<JoinHandle<()>>,
}

impl Snapshotter {
    pub fn start(
        app: AppHandle,
        aggregator: Arc<Mutex<FlowAggregator>>,
        enricher: Arc<Enricher>,
        interval_ms: u64,
        top_n: usize,
        settings_mtx: Arc<Mutex<crate::state::Settings>>,
    ) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let stop_thread = stop.clone();
        let interval = Duration::from_millis(interval_ms.max(100));

        let join = thread::Builder::new()
            .name("flow-snapshotter".into())
            .spawn(move || {
                let mut last_cleanup = Instant::now();
                let mut prev: Option<(i64, Totals)> = None;
                let mut session_seen: HashSet<String> = HashSet::new();

                while !stop_thread.load(Ordering::Relaxed) {
                    let tick_start = Instant::now();
                    let now_ms = chrono::Utc::now().timestamp_millis();

                    let rules = settings_mtx
                        .lock()
                        .ok()
                        .map(|s| s.anomalies.clone())
                        .unwrap_or_default();

                    let payload = match build_payload(
                        &aggregator,
                        &enricher,
                        now_ms,
                        top_n,
                        &mut prev,
                        &rules,
                        &mut session_seen,
                    ) {
                        Some(p) => p,
                        None => continue,
                    };

                    if let Ok(mut agg) = aggregator.lock() {
                        if last_cleanup.elapsed()
                            >= Duration::from_millis(STALE_CLEANUP_INTERVAL_MS)
                        {
                            agg.cleanup_stale(now_ms);
                            last_cleanup = Instant::now();
                        }
                    }

                    let _ = app.emit("flow_snapshot", &payload);

                    let elapsed = tick_start.elapsed();
                    if elapsed < interval {
                        thread::sleep(interval - elapsed);
                    }
                }
            })
            .expect("spawn snapshotter thread");

        Self {
            stop,
            join: Some(join),
        }
    }

    pub fn stop(mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(handle) = self.join.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for Snapshotter {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(handle) = self.join.take() {
            let _ = handle.join();
        }
    }
}

fn build_payload(
    aggregator: &Arc<Mutex<FlowAggregator>>,
    enricher: &Arc<Enricher>,
    now_ms: i64,
    top_n: usize,
    prev: &mut Option<(i64, Totals)>,
    anomaly_rules: &AnomalyRuleSettings,
    session_seen: &mut HashSet<String>,
) -> Option<FlowSnapshot> {
    let (mut all_flows, flow_count) = {
        let agg = aggregator.lock().ok()?;
        (agg.snapshot(), agg.flow_count())
    };

    enricher.enrich_in_place(&mut all_flows);

    let mut anomalies = fe::evaluate_anomalies(&all_flows, anomaly_rules, session_seen);
    anomalies.truncate(100);

    let totals = fe::totals(&all_flows);
    let summary = DashboardSummary {
        packets_per_second: rate(
            prev.as_ref().map(|(t, p)| (*t, p.total_packets())),
            now_ms,
            totals.total_packets(),
        ),
        bytes_per_second_up: rate(
            prev.as_ref().map(|(t, p)| (*t, p.bytes_up)),
            now_ms,
            totals.bytes_up,
        ),
        bytes_per_second_down: rate(
            prev.as_ref().map(|(t, p)| (*t, p.bytes_down)),
            now_ms,
            totals.bytes_down,
        ),
        top_hosts: fe::top_hosts(&all_flows, TOP_LIST_LEN),
        top_protocols: fe::top_protocols(&all_flows, TOP_LIST_LEN),
        top_ports: fe::top_ports(&all_flows, TOP_LIST_LEN),
        top_countries: fe::top_countries(&all_flows, TOP_LIST_LEN),
        totals: totals.clone(),
    };

    all_flows.sort_by(|a, b| {
        (b.bytes_up + b.bytes_down).cmp(&(a.bytes_up + a.bytes_down))
    });
    all_flows.truncate(top_n);

    let payload = FlowSnapshot {
        timestamp: now_ms,
        flow_count,
        flows: all_flows,
        summary,
        anomalies,
    };
    *prev = Some((now_ms, totals));
    Some(payload)
}

fn rate(previous: Option<(i64, u64)>, now_ms: i64, current: u64) -> f64 {
    let Some((prev_ms, prev_value)) = previous else {
        return 0.0;
    };
    let dt_ms = now_ms.saturating_sub(prev_ms);
    if dt_ms <= 0 {
        return 0.0;
    }
    let delta = current.saturating_sub(prev_value) as f64;
    delta * 1000.0 / dt_ms as f64
}
