import type { FlowSnapshot } from "./api";

interface Props {
  snapshot: FlowSnapshot | null;
}

/**
 * One-line heuristic digest for newcomers (not forensic truth).
 */
export default function TrafficHealthStrip({ snapshot }: Props) {
  if (!snapshot) {
    return (
      <section className="card traffic-health">
        <p className="muted small">
          Capture traffic to populate a digest (dominant ports, protocols).
        </p>
      </section>
    );
  }

  const topProto = snapshot.summary.top_protocols[0];
  const topPort = snapshot.summary.top_ports[0];

  let line =
    "Top traffic mix varies — sort Connections by TOTAL to find heavy talkers.";
  if (topPort?.port === 443) {
    line =
      "Plenty of bytes sit on tcp/443 destinations — typical HTTPS (browsers and sync tooling).";
  } else if (topPort?.port === 53 || topPort?.port === 853) {
    line =
      "DNS-shaped ports dominate — browsers and apps resolve names constantly.";
  } else if (topProto?.protocol === "UDP" && snapshot.summary.top_ports[1]?.port === 1900) {
    line =
      "UDP chatter includes discovery-style ports — often local multicast/SSDP bursts.";
  }

  const ac =
    snapshot.anomalies.length > 0
      ? ` Signals panel flagged ${snapshot.anomalies.length} heuristic cue(s) — investigate, don't panic.`
      : "";

  return (
    <section className="card traffic-health">
      <p className="mono small">{line + ac}</p>
    </section>
  );
}
