import { Area, AreaChart, ResponsiveContainer, Tooltip, YAxis } from "recharts";
import type {
  CountryStat,
  DashboardSummary,
  FlowSnapshot,
  HostStat,
  PortStat,
  ProtocolStat,
} from "./api";
import type { RateSample } from "./useFlowSnapshot";

interface Props {
  snapshot: FlowSnapshot | null;
  history: RateSample[];
}

export default function Dashboard({ snapshot, history }: Props) {
  if (!snapshot) {
    return (
      <section className="card">
        <div className="card-head">
          <h2>Dashboard</h2>
          <span className="muted">waiting for snapshot…</span>
        </div>
        <p className="muted">
          Start a capture to populate totals, top hosts, top protocols and
          rate sparklines.
        </p>
      </section>
    );
  }

  const { summary } = snapshot;

  return (
    <section className="dashboard">
      <KpiRow summary={summary} history={history} />

      <div className="dash-grid">
        <TopHosts hosts={summary.top_hosts} />
        <TopProtocols protocols={summary.top_protocols} />
        <TopPorts ports={summary.top_ports} />
        <TopCountries countries={summary.top_countries} />
      </div>
    </section>
  );
}

function KpiRow({
  summary,
  history,
}: {
  summary: DashboardSummary;
  history: RateSample[];
}) {
  const { totals, packets_per_second, bytes_per_second_up, bytes_per_second_down } = summary;
  return (
    <div className="kpi-row">
      <Kpi
        label="upload"
        value={formatBytes(totals.bytes_up)}
        sub={`${formatRate(bytes_per_second_up)}`}
        chart={<RateSparkline data={history} field="bps_up" color="#7fff7f" />}
      />
      <Kpi
        label="download"
        value={formatBytes(totals.bytes_down)}
        sub={`${formatRate(bytes_per_second_down)}`}
        chart={<RateSparkline data={history} field="bps_down" color="#00ff66" />}
      />
      <Kpi
        label="packets"
        value={(totals.packets_up + totals.packets_down).toLocaleString()}
        sub={`${packets_per_second.toFixed(1)} pps`}
        chart={<RateSparkline data={history} field="pps" color="#00aa44" />}
      />
    </div>
  );
}

function Kpi({
  label,
  value,
  sub,
  chart,
}: {
  label: string;
  value: string;
  sub: string;
  chart?: React.ReactNode;
}) {
  return (
    <div className="kpi">
      <div className="kpi-text">
        <div className="kpi-label">{label}</div>
        <div className="kpi-value mono">{value}</div>
        <div className="kpi-sub mono">{sub}</div>
      </div>
      {chart && <div className="kpi-chart">{chart}</div>}
    </div>
  );
}

function RateSparkline({
  data,
  field,
  color,
}: {
  data: RateSample[];
  field: keyof Pick<RateSample, "pps" | "bps_up" | "bps_down">;
  color: string;
}) {
  if (data.length < 2) {
    return <div className="spark-empty muted">…</div>;
  }
  return (
    <ResponsiveContainer width="100%" height="100%">
      <AreaChart data={data} margin={{ top: 4, right: 0, bottom: 0, left: 0 }}>
        <defs>
          <linearGradient id={`g-${field}`} x1="0" y1="0" x2="0" y2="1">
            <stop offset="5%" stopColor={color} stopOpacity={0.6} />
            <stop offset="95%" stopColor={color} stopOpacity={0.05} />
          </linearGradient>
        </defs>
        <YAxis hide domain={[0, "dataMax"]} />
        <Tooltip
          cursor={{ stroke: "rgba(0,255,102,0.4)", strokeDasharray: "2 2" }}
          contentStyle={{
            background: "#001a00",
            border: "1px solid rgba(0,255,102,0.4)",
            fontSize: "11px",
            padding: "4px 6px",
            fontFamily: "'JetBrains Mono', ui-monospace, monospace",
            color: "#00ff66",
            boxShadow: "0 0 12px rgba(0,255,102,0.2)",
          }}
          formatter={(value) => {
            const v = typeof value === "number" ? value : 0;
            return field === "pps" ? `${v.toFixed(1)} pps` : formatRate(v);
          }}
          labelFormatter={() => ""}
        />
        <Area
          type="monotone"
          dataKey={field}
          stroke={color}
          strokeWidth={1.5}
          fill={`url(#g-${field})`}
          isAnimationActive={false}
        />
      </AreaChart>
    </ResponsiveContainer>
  );
}

function TopHosts({ hosts }: { hosts: HostStat[] }) {
  const max = hosts[0]?.bytes ?? 0;
  return (
    <Card title="Top hosts" empty={hosts.length === 0}>
      {hosts.map((h) => (
        <BarRow
          key={h.ip}
          label={h.hostname ?? h.ip}
          sub={h.hostname ? h.ip : undefined}
          value={formatBytes(h.bytes)}
          ratio={max ? h.bytes / max : 0}
        />
      ))}
    </Card>
  );
}

function TopProtocols({ protocols }: { protocols: ProtocolStat[] }) {
  const max = protocols[0]?.bytes ?? 0;
  return (
    <Card title="Top protocols" empty={protocols.length === 0}>
      {protocols.map((p) => (
        <BarRow
          key={p.protocol}
          label={p.protocol}
          value={formatBytes(p.bytes)}
          ratio={max ? p.bytes / max : 0}
        />
      ))}
    </Card>
  );
}

function TopPorts({ ports }: { ports: PortStat[] }) {
  const max = ports[0]?.bytes ?? 0;
  return (
    <Card title="Top ports" empty={ports.length === 0}>
      {ports.map((p) => (
        <BarRow
          key={p.port}
          label={`${p.port} ${PORT_NAMES[p.port] ?? ""}`.trim()}
          value={formatBytes(p.bytes)}
          ratio={max ? p.bytes / max : 0}
        />
      ))}
    </Card>
  );
}

function TopCountries({ countries }: { countries: CountryStat[] }) {
  const max = countries[0]?.bytes ?? 0;
  return (
    <Card
      title="Top countries"
      empty={countries.length === 0}
      placeholder="Load a GeoLite2 .mmdb in Settings to populate countries."
    >
      {countries.map((c) => (
        <BarRow
          key={c.country}
          label={c.country}
          value={formatBytes(c.bytes)}
          ratio={max ? c.bytes / max : 0}
        />
      ))}
    </Card>
  );
}

function Card({
  title,
  children,
  empty,
  placeholder,
}: {
  title: string;
  children: React.ReactNode;
  empty: boolean;
  placeholder?: string;
}) {
  return (
    <div className="card">
      <div className="card-head">
        <h3>{title}</h3>
      </div>
      {empty ? (
        <p className="muted small">{placeholder ?? "No data yet."}</p>
      ) : (
        <div className="bar-list">{children}</div>
      )}
    </div>
  );
}

function BarRow({
  label,
  sub,
  value,
  ratio,
}: {
  label: string;
  sub?: string;
  value: string;
  ratio: number;
}) {
  return (
    <div className="bar-row">
      <div className="bar-label">
        <span className="mono ellipsis">{label}</span>
        {sub && <span className="muted small mono ellipsis">{sub}</span>}
      </div>
      <div className="bar-track">
        <div
          className="bar-fill"
          style={{ width: `${Math.max(ratio * 100, 1)}%` }}
        />
      </div>
      <div className="bar-value mono">{value}</div>
    </div>
  );
}

const PORT_NAMES: Record<number, string> = {
  20: "ftp-data",
  21: "ftp",
  22: "ssh",
  23: "telnet",
  25: "smtp",
  53: "dns",
  67: "dhcp",
  68: "dhcp",
  80: "http",
  110: "pop3",
  123: "ntp",
  143: "imap",
  443: "https",
  465: "smtps",
  587: "smtp",
  993: "imaps",
  995: "pop3s",
  3306: "mysql",
  3389: "rdp",
  5432: "postgres",
  5900: "vnc",
  6379: "redis",
  8080: "http-alt",
  8443: "https-alt",
  27017: "mongo",
};

function formatBytes(n: number): string {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  if (n < 1024 * 1024 * 1024) return `${(n / 1024 / 1024).toFixed(1)} MB`;
  return `${(n / 1024 / 1024 / 1024).toFixed(2)} GB`;
}

function formatRate(bps: number): string {
  return `${formatBytes(bps)}/s`;
}
