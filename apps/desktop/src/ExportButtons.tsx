import { useState } from "react";
import { save } from "@tauri-apps/plugin-dialog";
import { exportSession } from "./api";

export default function ExportButtons() {
  const [busy, setBusy] = useState<"csv" | "json" | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [last, setLast] = useState<string | null>(null);

  const doExport = async (fmt: "csv" | "json") => {
    setBusy(fmt);
    setError(null);
    try {
      const ts = new Date().toISOString().replace(/[:.]/g, "-");
      const path = await save({
        title: `Export session as ${fmt.toUpperCase()}`,
        defaultPath: `network-monitor-${ts}.${fmt}`,
        filters: [{ name: fmt.toUpperCase(), extensions: [fmt] }],
      });
      if (!path) return;
      const result = await exportSession(path, fmt);
      setLast(`exported ${result.flow_count} flows → ${result.path}`);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(null);
    }
  };

  return (
    <div className="export-row">
      <button
        className="secondary"
        onClick={() => doExport("csv")}
        disabled={busy !== null}
      >
        {busy === "csv" ? "exporting…" : "export csv"}
      </button>
      <button
        className="secondary"
        onClick={() => doExport("json")}
        disabled={busy !== null}
      >
        {busy === "json" ? "exporting…" : "export json"}
      </button>
      {last && <span className="muted small">{last}</span>}
      {error && <span className="status error small">{error}</span>}
    </div>
  );
}
