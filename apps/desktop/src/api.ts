import { invoke } from "@tauri-apps/api/core";
import type { FlowRecord, InterfaceInfo, Protocol } from "./types";

export interface CaptureStatsSnapshot {
  packets_parsed: number;
  packets_dropped_unparsed: number;
  bytes_seen: number;
}

export interface CaptureStatus {
  running: boolean;
  paused: boolean;
  interface: string | null;
  stats: CaptureStatsSnapshot | null;
  flow_count: number;
}

export interface AnomalyRuleSettings {
  rare_port_enabled: boolean;
  high_upload_enabled: boolean;
  geo_notice_enabled: boolean;
  session_new_remote_enabled: boolean;
  upload_ratio_threshold: number;
  rare_extra_allowlist: number[];
}

export type FlowAnomalySeverity = "info" | "notice" | "warn";

export interface FlowAnomaly {
  id: string;
  rule_id: string;
  severity: FlowAnomalySeverity;
  flow_id: string;
  message: string;
}

export interface Settings {
  snapshot_interval_ms: number;
  stale_after_ms: number;
  top_n: number;
  reverse_dns_enabled: boolean;
  geoip_db_path: string | null;
  anomalies: AnomalyRuleSettings;
}

export interface Totals {
  bytes_up: number;
  bytes_down: number;
  packets_up: number;
  packets_down: number;
}

export interface HostStat {
  ip: string;
  hostname: string | null;
  country: string | null;
  bytes: number;
  packets: number;
}

export interface ProtocolStat {
  protocol: Protocol;
  bytes: number;
  packets: number;
}

export interface PortStat {
  port: number;
  bytes: number;
  packets: number;
}

export interface CountryStat {
  country: string;
  bytes: number;
  packets: number;
}

export interface DashboardSummary {
  totals: Totals;
  packets_per_second: number;
  bytes_per_second_up: number;
  bytes_per_second_down: number;
  top_hosts: HostStat[];
  top_protocols: ProtocolStat[];
  top_ports: PortStat[];
  top_countries: CountryStat[];
}

export interface FlowSnapshot {
  timestamp: number;
  flow_count: number;
  flows: FlowRecord[];
  summary: DashboardSummary;
  anomalies: FlowAnomaly[];
}

export interface ExportResult {
  path: string;
  flow_count: number;
  format: string;
}

export interface EnrichmentStatus {
  cached_entries: number;
  queue_depth: number;
  geoip_loaded: boolean;
  rdns_enabled: boolean;
}

export async function listInterfaces(): Promise<InterfaceInfo[]> {
  return invoke<InterfaceInfo[]>("list_interfaces");
}

export async function startCapture(iface: string): Promise<void> {
  return invoke<void>("start_capture", { interface: iface });
}

export async function stopCapture(): Promise<void> {
  return invoke<void>("stop_capture");
}

export async function pauseCapture(): Promise<void> {
  return invoke<void>("pause_capture");
}

export async function resumeCapture(): Promise<void> {
  return invoke<void>("resume_capture");
}

export async function captureStatus(): Promise<CaptureStatus> {
  return invoke<CaptureStatus>("capture_status");
}

export async function getSettings(): Promise<Settings> {
  return invoke<Settings>("get_settings");
}

export async function saveSettings(next: Settings): Promise<Settings> {
  return invoke<Settings>("save_settings", { new: next });
}

export async function exportSession(
  path: string,
  format: "csv" | "json",
): Promise<ExportResult> {
  return invoke<ExportResult>("export_session", { path, format });
}

export async function enrichmentStatus(): Promise<EnrichmentStatus> {
  return invoke<EnrichmentStatus>("enrichment_status");
}
