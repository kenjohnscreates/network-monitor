import { useEffect, useRef, useState } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { FlowSnapshot } from "./api";

const HISTORY_LIMIT = 60;

export interface RateSample {
  timestamp: number;
  pps: number;
  bps_up: number;
  bps_down: number;
}

export interface FlowSnapshotState {
  snapshot: FlowSnapshot | null;
  history: RateSample[];
}

/**
 * Subscribe to throttled flow snapshots from the Rust backend.
 * Returns the latest snapshot plus a rolling rate history (up to
 * `HISTORY_LIMIT` samples) suitable for sparklines.
 */
export function useFlowSnapshot(): FlowSnapshotState {
  const [snapshot, setSnapshot] = useState<FlowSnapshot | null>(null);
  const [history, setHistory] = useState<RateSample[]>([]);
  const buf = useRef<RateSample[]>([]);

  useEffect(() => {
    let unlisten: UnlistenFn | null = null;
    let cancelled = false;
    void listen<FlowSnapshot>("flow_snapshot", (event) => {
      if (cancelled) return;
      const snap = event.payload;
      setSnapshot(snap);
      const sample: RateSample = {
        timestamp: snap.timestamp,
        pps: snap.summary.packets_per_second,
        bps_up: snap.summary.bytes_per_second_up,
        bps_down: snap.summary.bytes_per_second_down,
      };
      const next = [...buf.current, sample].slice(-HISTORY_LIMIT);
      buf.current = next;
      setHistory(next);
    }).then((un) => {
      if (cancelled) un();
      else unlisten = un;
    });
    return () => {
      cancelled = true;
      if (unlisten) unlisten();
    };
  }, []);

  return { snapshot, history };
}
