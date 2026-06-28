import { useEffect, useRef, useState } from "react";
import {
  captureStatus,
  pauseCapture,
  resumeCapture,
  startCapture,
  stopCapture,
  type CaptureStatus,
} from "./api";

interface Props {
  interfaceId: string | null;
}

const POLL_MS = 750;

export default function CapturePanel({ interfaceId }: Props) {
  const [status, setStatus] = useState<CaptureStatus>({
    running: false,
    paused: false,
    interface: null,
    stats: null,
    flow_count: 0,
  });
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const pollTimer = useRef<number | null>(null);

  useEffect(() => {
    let cancelled = false;
    async function poll() {
      try {
        const s = await captureStatus();
        if (!cancelled) setStatus(s);
      } catch (e) {
        if (!cancelled) setError(String(e));
      }
    }
    void poll();
    pollTimer.current = window.setInterval(poll, POLL_MS);
    return () => {
      cancelled = true;
      if (pollTimer.current !== null) window.clearInterval(pollTimer.current);
    };
  }, []);

  async function run(action: () => Promise<unknown>) {
    setBusy(true);
    setError(null);
    try {
      await action();
      setStatus(await captureStatus());
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }

  const { running, paused } = status;
  const startDisabled = busy || running || !interfaceId;
  const stopDisabled = busy || !running;
  const pauseDisabled = busy || !running || paused;
  const resumeDisabled = busy || !running || !paused;

  return (
    <section className="card">
      <div className="card-head">
        <h2>Capture</h2>
        <div className="row gap">
          <button
            onClick={() => interfaceId && run(() => startCapture(interfaceId))}
            disabled={startDisabled}
          >
            {running ? (paused ? "paused" : "running…") : "start"}
          </button>
          <button
            onClick={() => run(pauseCapture)}
            disabled={pauseDisabled}
            className="secondary"
          >
            pause
          </button>
          <button
            onClick={() => run(resumeCapture)}
            disabled={resumeDisabled}
            className="secondary"
          >
            resume
          </button>
          <button
            onClick={() => run(stopCapture)}
            disabled={stopDisabled}
            className="secondary"
          >
            stop
          </button>
        </div>
      </div>

      {!interfaceId && !running && (
        <p className="muted">Select an interface above to start a capture.</p>
      )}

      {error && (
        <div className="status error">
          <pre>{error}</pre>
          {isPermissionError(error) && <PermissionHint />}
        </div>
      )}

      <div className="stats">
        <Stat
          label="status"
          value={running ? (paused ? "paused" : "running") : "stopped"}
        />
        <Stat label="interface" value={status.interface ?? "—"} mono />
        <Stat label="flows" value={status.flow_count.toLocaleString()} />
        <Stat
          label="packets parsed"
          value={(status.stats?.packets_parsed ?? 0).toLocaleString()}
        />
        <Stat
          label="unparsed"
          value={(status.stats?.packets_dropped_unparsed ?? 0).toLocaleString()}
        />
        <Stat
          label="bytes seen"
          value={formatBytes(status.stats?.bytes_seen ?? 0)}
        />
      </div>
    </section>
  );
}

function Stat({
  label,
  value,
  mono,
}: {
  label: string;
  value: string;
  mono?: boolean;
}) {
  return (
    <div className="stat">
      <div className="stat-label">{label}</div>
      <div className={`stat-value ${mono ? "mono" : ""}`}>{value}</div>
    </div>
  );
}

function formatBytes(n: number): string {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  if (n < 1024 * 1024 * 1024) return `${(n / 1024 / 1024).toFixed(1)} MB`;
  return `${(n / 1024 / 1024 / 1024).toFixed(2)} GB`;
}

function isPermissionError(msg: string): boolean {
  const m = msg.toLowerCase();
  return (
    m.includes("permission") ||
    m.includes("operation not permitted") ||
    m.includes("/dev/bpf") ||
    m.includes("you don't have permission")
  );
}

function PermissionHint() {
  return (
    <div className="hint">
      <strong>macOS BPF permissions required.</strong>
      <p className="muted small">
        Live capture needs read access to <span className="mono">/dev/bpf*</span>.
        Pick one:
      </p>
      <ul className="muted small">
        <li>
          One-shot:{" "}
          <span className="mono">sudo chmod o+r /dev/bpf*</span> (resets on
          reboot)
        </li>
        <li>
          Persistent: install the <span className="mono">ChmodBPF</span> launch
          daemon shipped with Wireshark
        </li>
      </ul>
    </div>
  );
}
