import { useCallback, useEffect, useState } from "react";
import { listInterfaces } from "./api";
import type { InterfaceInfo } from "./types";

type LoadState =
  | { status: "idle" }
  | { status: "loading" }
  | { status: "ok"; interfaces: InterfaceInfo[] }
  | { status: "error"; message: string };

interface Props {
  selectedId: string | null;
  onSelect: (id: string) => void;
}

export default function InterfaceList({ selectedId, onSelect }: Props) {
  const [state, setState] = useState<LoadState>({ status: "idle" });

  const refresh = useCallback(async () => {
    setState({ status: "loading" });
    try {
      const interfaces = await listInterfaces();
      setState({ status: "ok", interfaces });
    } catch (err) {
      setState({ status: "error", message: String(err) });
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  return (
    <section className="card">
      <div className="card-head">
        <h2>Network interfaces</h2>
        <button onClick={refresh} disabled={state.status === "loading"}>
          {state.status === "loading" ? "loading…" : "refresh"}
        </button>
      </div>

      {state.status === "error" && (
        <pre className="status error">{state.message}</pre>
      )}

      {state.status === "ok" && state.interfaces.length === 0 && (
        <p className="muted">
          No interfaces returned. On macOS this can mean libpcap has no
          permission to enumerate devices — see <code>docs/setup.md</code>.
        </p>
      )}

      {state.status === "ok" && state.interfaces.length > 0 && (
        <ul className="iface-list">
          {state.interfaces.map((iface) => {
            const selected = selectedId === iface.id;
            return (
              <li
                key={iface.id}
                className={`iface ${selected ? "selected" : ""}`}
                onClick={() => onSelect(iface.id)}
              >
                <div className="iface-main">
                  <span className="iface-name">{iface.name}</span>
                  <span
                    className={`iface-badge ${iface.usable ? "up" : "down"}`}
                    title={iface.usable ? "appears usable" : "down or unknown"}
                  >
                    {iface.usable ? "up" : "down"}
                  </span>
                </div>
                {iface.description && (
                  <div className="iface-desc">{iface.description}</div>
                )}
              </li>
            );
          })}
        </ul>
      )}

      {state.status === "ok" && (
        <p className="muted footnote">
          {state.interfaces.length} interface
          {state.interfaces.length === 1 ? "" : "s"} •{" "}
          {selectedId ? `selected: ${selectedId}` : "click one to select"}
        </p>
      )}
    </section>
  );
}
