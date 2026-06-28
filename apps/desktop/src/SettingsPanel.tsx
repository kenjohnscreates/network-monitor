import { useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import {
  enrichmentStatus,
  getSettings,
  saveSettings,
  type EnrichmentStatus,
  type Settings,
} from "./api";

function parsePortsCsv(s: string): number[] {
  const xs: number[] = [];
  for (const part of s.split(/[,;\s]+/)) {
    const t = part.trim();
    if (!t) continue;
    const n = Number.parseInt(t, 10);
    if (!Number.isNaN(n) && n > 0 && n <= 65535) xs.push(n);
  }
  return xs;
}

interface Props {
  onClose: () => void;
}

export default function SettingsModal({ onClose }: Props) {
  const [settings, setSettings] = useState<Settings | null>(null);
  const [draft, setDraft] = useState<Settings | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [status, setStatus] = useState<EnrichmentStatus | null>(null);

  useEffect(() => {
    let cancelled = false;
    getSettings()
      .then((s) => {
        if (cancelled) return;
        setSettings(s);
        setDraft(s);
      })
      .catch((e) => setError(String(e)));
    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    let cancelled = false;
    const tick = async () => {
      try {
        const s = await enrichmentStatus();
        if (!cancelled) setStatus(s);
      } catch {
        // ignore
      }
    };
    void tick();
    const id = window.setInterval(tick, 1500);
    return () => {
      cancelled = true;
      window.clearInterval(id);
    };
  }, []);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose]);

  if (!draft || !settings) {
    return (
      <Backdrop onClose={onClose}>
        <div className="modal">
          <h2>Settings</h2>
          <p className="muted">Loading…</p>
        </div>
      </Backdrop>
    );
  }

  const dirty = JSON.stringify(draft) !== JSON.stringify(settings);

  const setField = <K extends keyof Settings>(key: K, value: Settings[K]) => {
    setDraft((d) => (d ? { ...d, [key]: value } : d));
  };

  const setAnomaly = <K extends keyof Settings["anomalies"]>(
    key: K,
    value: Settings["anomalies"][K],
  ) => {
    setDraft((d) =>
      d
        ? {
            ...d,
            anomalies: { ...d.anomalies, [key]: value },
          }
        : d,
    );
  };

  const apply = async () => {
    if (!draft) return;
    setBusy(true);
    setError(null);
    try {
      const saved = await saveSettings({
        ...draft,
        geoip_db_path:
          draft.geoip_db_path && draft.geoip_db_path.trim() !== ""
            ? draft.geoip_db_path
            : null,
      });
      setSettings(saved);
      setDraft(saved);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  const browseGeoip = async () => {
    try {
      const picked = await open({
        title: "Select GeoIP database",
        multiple: false,
        directory: false,
        filters: [{ name: "MaxMind DB", extensions: ["mmdb"] }],
      });
      if (typeof picked === "string") setField("geoip_db_path", picked);
    } catch (e) {
      setError(String(e));
    }
  };

  return (
    <Backdrop onClose={onClose}>
      <div
        className="modal"
        role="dialog"
        aria-modal="true"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="modal-head">
          <h2>Settings</h2>
          <button className="secondary close" aria-label="close" onClick={onClose}>
            ×
          </button>
        </div>

        <div className="setting">
          <label className="check">
            <input
              type="checkbox"
              checked={draft.reverse_dns_enabled}
              onChange={(e) => setField("reverse_dns_enabled", e.target.checked)}
            />
            <span>
              Reverse DNS lookup
              <span className="muted small"> — adds hostnames to remote IPs</span>
            </span>
          </label>
        </div>

        <div className="setting">
          <label htmlFor="geoip-path" className="setting-label">
            GeoIP database path
            <span className="muted small"> — local .mmdb file (optional)</span>
          </label>
          <div className="row gap">
            <input
              id="geoip-path"
              type="text"
              className="filter-input"
              placeholder="/path/to/GeoLite2-Country.mmdb"
              value={draft.geoip_db_path ?? ""}
              onChange={(e) => setField("geoip_db_path", e.target.value)}
              spellCheck={false}
            />
            <button className="secondary" onClick={browseGeoip}>
              browse…
            </button>
          </div>
        </div>

        <div className="row gap">
          <div className="setting" style={{ flex: 1 }}>
            <label htmlFor="snap-int" className="setting-label">
              Snapshot interval (ms)
              <span className="muted small"> — UI update cadence</span>
            </label>
            <input
              id="snap-int"
              type="number"
              min={100}
              max={5000}
              step={50}
              className="filter-input"
              value={draft.snapshot_interval_ms}
              onChange={(e) =>
                setField(
                  "snapshot_interval_ms",
                  Math.max(100, parseInt(e.target.value, 10) || 0),
                )
              }
            />
          </div>
          <div className="setting" style={{ flex: 1 }}>
            <label htmlFor="stale" className="setting-label">
              Stale flow timeout (ms)
              <span className="muted small"> — prune flows after</span>
            </label>
            <input
              id="stale"
              type="number"
              min={1000}
              step={1000}
              className="filter-input"
              value={draft.stale_after_ms}
              onChange={(e) =>
                setField(
                  "stale_after_ms",
                  Math.max(1000, parseInt(e.target.value, 10) || 0),
                )
              }
            />
          </div>
        </div>

        <div className="setting">
          <p className="muted small uppercase" style={{ margin: "16px 0 8px" }}>
            Signal heuristics
          </p>
          <p className="muted small">
            Explainable checks on each snapshot — not verdicts. &quot;New remote&quot; is
            first-seen per capture session only.
          </p>
          <label className="check">
            <input
              type="checkbox"
              checked={draft.anomalies.rare_port_enabled}
              onChange={(e) =>
                setAnomaly("rare_port_enabled", e.target.checked)
              }
            />
            <span>Rare WAN destination port (vs built-in common list)</span>
          </label>
          <label className="check">
            <input
              type="checkbox"
              checked={draft.anomalies.high_upload_enabled}
              onChange={(e) =>
                setAnomaly("high_upload_enabled", e.target.checked)
              }
            />
            <span>High upload share (bytes up vs total toward WAN)</span>
          </label>
          <label className="check">
            <input
              type="checkbox"
              checked={draft.anomalies.geo_notice_enabled}
              onChange={(e) =>
                setAnomaly("geo_notice_enabled", e.target.checked)
              }
            />
            <span>
              Geo + non‑web port notice
              <span className="muted small"> — needs GeoIP path</span>
            </span>
          </label>
          <label className="check">
            <input
              type="checkbox"
              checked={draft.anomalies.session_new_remote_enabled}
              onChange={(e) =>
                setAnomaly("session_new_remote_enabled", e.target.checked)
              }
            />
            <span>Session: first observation toward a WAN remote IP</span>
          </label>
          <label htmlFor="upload-ratio" className="setting-label" style={{ marginTop: 12 }}>
            Upload ratio threshold (when total ≥ 512 B)
          </label>
          <input
            id="upload-ratio"
            type="number"
            min={0.51}
            max={0.99}
            step={0.01}
            className="filter-input"
            style={{ maxWidth: 120 }}
            value={draft.anomalies.upload_ratio_threshold}
            onChange={(e) =>
              setAnomaly(
                "upload_ratio_threshold",
                Math.min(
                  0.99,
                  Math.max(
                    0.51,
                    Number.parseFloat(e.target.value) || 0,
                  ),
                ),
              )
            }
          />
          <label htmlFor="rare-allow" className="setting-label" style={{ marginTop: 12 }}>
            Extra ports never flagged as &quot;rare&quot; (comma-separated)
          </label>
          <input
            id="rare-allow"
            type="text"
            className="filter-input"
            placeholder="8443, 1883…"
            value={draft.anomalies.rare_extra_allowlist.join(", ")}
            onChange={(e) =>
              setAnomaly(
                "rare_extra_allowlist",
                parsePortsCsv(e.target.value),
              )
            }
            spellCheck={false}
          />
        </div>

        <div className="setting">
          <label htmlFor="topn" className="setting-label">
            Top N flows in snapshot
            <span className="muted small"> — limits emitted flow rows</span>
          </label>
          <input
            id="topn"
            type="number"
            min={10}
            max={2000}
            step={10}
            className="filter-input"
            value={draft.top_n}
            onChange={(e) =>
              setField("top_n", Math.max(10, parseInt(e.target.value, 10) || 0))
            }
          />
        </div>

        {status && (
          <p className="muted small mono">
            {status.cached_entries} cached • {status.queue_depth} pending •{" "}
            {status.geoip_loaded ? "geoip ✓" : "geoip ✗"} •{" "}
            {status.rdns_enabled ? "rdns on" : "rdns off"}
          </p>
        )}

        {error && <div className="status error">{error}</div>}

        <div className="modal-actions">
          <button className="secondary" onClick={onClose} disabled={busy}>
            close
          </button>
          <button onClick={apply} disabled={!dirty || busy}>
            {busy ? "saving…" : "save"}
          </button>
        </div>
      </div>
    </Backdrop>
  );
}

function Backdrop({
  children,
  onClose,
}: {
  children: React.ReactNode;
  onClose: () => void;
}) {
  return (
    <div className="modal-backdrop" onClick={onClose}>
      {children}
    </div>
  );
}
