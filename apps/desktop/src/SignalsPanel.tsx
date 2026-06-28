import type { FlowSnapshot } from "./api";

type Props = {
  snapshot: FlowSnapshot | null;
  onHighlightFlow: (flowId: string | null) => void;
};

export default function SignalsPanel({ snapshot, onHighlightFlow }: Props) {
  const list = snapshot?.anomalies ?? [];

  return (
    <section className="card signals-card">
      <div className="card-head">
        <h2>Signals</h2>
        <span className="muted small">
          {list.length.toLocaleString()} heuristic cue
          {list.length === 1 ? "" : "s"}
        </span>
      </div>
      <p className="muted small">
        Transparent rules — not antivirus verdicts. Disable or tune in Settings.
      </p>
      {!snapshot ? (
        <p className="muted small">Start capture to evaluate flows.</p>
      ) : list.length === 0 ? (
        <p className="muted small">No cues this tick (or all rules disabled).</p>
      ) : (
        <ul className="signals-list">
          {list.slice(0, 50).map((a) => (
            <li key={a.id}>
              <button
                type="button"
                className="signal-msg"
                onClick={() => onHighlightFlow(a.flow_id)}
              >
                {a.message}
              </button>
              <span className="muted tiny mono">{a.rule_id}</span>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}
