import { useEffect, useMemo, useState } from "react";
import {
  type ColumnDef,
  type ColumnFiltersState,
  type SortingState,
  flexRender,
  getCoreRowModel,
  getFilteredRowModel,
  getSortedRowModel,
  useReactTable,
} from "@tanstack/react-table";
import type { FlowSnapshot } from "./api";
import type { FlowRecord, Protocol } from "./types";
import { passesFocusPreset, type FocusPreset } from "./trafficFilters";

interface Props {
  snapshot: FlowSnapshot | null;
  highlightFlowId?: string | null;
}

type DirectionFilter = "all" | "outbound" | "inbound" | "mixed";

const PROTOCOLS: Array<Protocol | "all"> = ["all", "TCP", "UDP", "ICMP", "OTHER"];

export default function ConnectionsTable({ snapshot, highlightFlowId }: Props) {
  const [globalFilter, setGlobalFilter] = useState("");
  const [protoFilter, setProtoFilter] = useState<Protocol | "all">("all");
  const [portFilter, setPortFilter] = useState("");
  const [dirFilter, setDirFilter] = useState<DirectionFilter>("all");
  const [focusPreset, setFocusPreset] = useState<FocusPreset>("all");
  const [sorting, setSorting] = useState<SortingState>([
    { id: "bytes", desc: true },
  ]);

  const data = useMemo(() => snapshot?.flows ?? [], [snapshot]);

  const columns = useMemo<ColumnDef<FlowRecord>[]>(
    () => [
      {
        id: "host",
        header: "host",
        accessorFn: (f) => f.hostname ?? f.dst_ip,
        cell: (c) => {
          const f = c.row.original;
          return (
            <div className="cell-host">
              <span className="mono ellipsis">{f.hostname ?? f.dst_ip}</span>
              {f.hostname && (
                <span className="muted small mono ellipsis">{f.dst_ip}</span>
              )}
            </div>
          );
        },
      },
      {
        id: "country",
        header: "country",
        accessorFn: (f) => f.country ?? "—",
        cell: (c) => <span className="mono small">{c.getValue() as string}</span>,
        size: 70,
      },
      {
        id: "protocol",
        header: "proto",
        accessorKey: "protocol",
        cell: (c) => (
          <span className="mono small">
            {(c.getValue() as string).toLowerCase()}
          </span>
        ),
        size: 60,
      },
      {
        id: "src_port",
        header: "src",
        accessorFn: (f) => f.src_port ?? 0,
        cell: (c) => (
          <span className="mono small num">
            {c.row.original.src_port ?? "—"}
          </span>
        ),
        size: 60,
      },
      {
        id: "dst_port",
        header: "dst",
        accessorFn: (f) => f.dst_port ?? 0,
        cell: (c) => (
          <span className="mono small num">
            {c.row.original.dst_port ?? "—"}
          </span>
        ),
        size: 60,
      },
      {
        id: "bytes_up",
        header: "up",
        accessorKey: "bytes_up",
        cell: (c) => (
          <span className="mono small num">
            {formatBytes(c.getValue() as number)}
          </span>
        ),
        size: 80,
      },
      {
        id: "bytes_down",
        header: "down",
        accessorKey: "bytes_down",
        cell: (c) => (
          <span className="mono small num">
            {formatBytes(c.getValue() as number)}
          </span>
        ),
        size: 80,
      },
      {
        id: "bytes",
        header: "total",
        accessorFn: (f) => f.bytes_up + f.bytes_down,
        cell: (c) => (
          <span className="mono small num">
            {formatBytes(c.getValue() as number)}
          </span>
        ),
        size: 80,
      },
      {
        id: "packets",
        header: "pkts",
        accessorFn: (f) => f.packets_up + f.packets_down,
        cell: (c) => (
          <span className="mono small num">
            {(c.getValue() as number).toLocaleString()}
          </span>
        ),
        size: 70,
      },
      {
        id: "first_seen",
        header: "first",
        accessorKey: "first_seen",
        cell: (c) => (
          <span className="mono small">
            {formatTime(c.getValue() as number)}
          </span>
        ),
        size: 80,
      },
      {
        id: "last_seen",
        header: "last",
        accessorKey: "last_seen",
        cell: (c) => (
          <span className="mono small">
            {formatTime(c.getValue() as number)}
          </span>
        ),
        size: 80,
      },
    ],
    [],
  );

  const columnFilters = useMemo<ColumnFiltersState>(() => {
    const f: ColumnFiltersState = [];
    if (protoFilter !== "all") f.push({ id: "protocol", value: protoFilter });
    return f;
  }, [protoFilter]);

  const filteredData = useMemo(() => {
    const port = portFilter.trim() === "" ? null : parseInt(portFilter, 10);
    const portOk = (f: FlowRecord) => {
      if (port === null || Number.isNaN(port)) return true;
      return f.src_port === port || f.dst_port === port;
    };
    const dirOk = (f: FlowRecord) => {
      switch (dirFilter) {
        case "all":
          return true;
        case "outbound":
          return f.bytes_up > 0 && f.bytes_down === 0;
        case "inbound":
          return f.bytes_down > 0 && f.bytes_up === 0;
        case "mixed":
          return f.bytes_up > 0 && f.bytes_down > 0;
      }
    };
    const focusOk = (f: FlowRecord) => passesFocusPreset(f, focusPreset);
    return data.filter((f) => focusOk(f) && portOk(f) && dirOk(f));
  }, [data, portFilter, dirFilter, focusPreset]);

  useEffect(() => {
    if (!highlightFlowId) return;
    window.requestAnimationFrame(() => {
      const el = document.querySelector(`[data-flow-id="${CSS.escape(highlightFlowId)}"]`);
      el?.scrollIntoView({ block: "nearest", behavior: "smooth" });
    });
  }, [highlightFlowId]);

  const table = useReactTable({
    data: filteredData,
    columns,
    state: { globalFilter, sorting, columnFilters },
    onGlobalFilterChange: setGlobalFilter,
    onSortingChange: setSorting,
    globalFilterFn: textFilter,
    getCoreRowModel: getCoreRowModel(),
    getSortedRowModel: getSortedRowModel(),
    getFilteredRowModel: getFilteredRowModel(),
  });

  const total = snapshot?.flow_count ?? 0;
  const shown = table.getRowModel().rows.length;

  return (
    <section className="card">
      <div className="card-head">
        <h2>Connections</h2>
        <span className="muted small">
          {shown.toLocaleString()} of {total.toLocaleString()} flows
        </span>
      </div>

      <div className="filters">
        <input
          type="search"
          placeholder="search host, IP, hostname…"
          value={globalFilter}
          onChange={(e) => setGlobalFilter(e.target.value)}
          className="filter-input"
        />
        <select
          value={protoFilter}
          onChange={(e) => setProtoFilter(e.target.value as Protocol | "all")}
        >
          {PROTOCOLS.map((p) => (
            <option key={p} value={p}>
              {p === "all" ? "any proto" : p}
            </option>
          ))}
        </select>
        <input
          type="text"
          inputMode="numeric"
          placeholder="port"
          value={portFilter}
          onChange={(e) => setPortFilter(e.target.value)}
          className="filter-input port"
        />
        <select
          value={focusPreset}
          onChange={(e) => setFocusPreset(e.target.value as FocusPreset)}
          title="wan-ish hides link-local IPv6, multicast, mDNS, and LAN-only RFC1918↔RFC1918 chatter"
        >
          <option value="all">focus: all</option>
          <option value="internet_focus">focus: wan-ish</option>
        </select>
        <select
          value={dirFilter}
          onChange={(e) => setDirFilter(e.target.value as DirectionFilter)}
        >
          <option value="all">any direction</option>
          <option value="outbound">outbound</option>
          <option value="inbound">inbound</option>
          <option value="mixed">mixed</option>
        </select>
        {(globalFilter ||
          protoFilter !== "all" ||
          portFilter ||
          dirFilter !== "all" ||
          focusPreset !== "all") && (
          <button
            className="secondary"
            onClick={() => {
              setGlobalFilter("");
              setProtoFilter("all");
              setPortFilter("");
              setDirFilter("all");
              setFocusPreset("all");
            }}
          >
            clear
          </button>
        )}
      </div>

      {focusPreset === "internet_focus" && (
        <p className="muted small footnote">
          WAN-ish focus hides link-local IPv6 (fe80::), multicast, mDNS (
          <span title="Multicast DNS / Bonjour">UDP 5353 / 224.0.0.251</span>
          ), and pure LAN RFC1918-to-RFC1918 flows.
        </p>
      )}

      {!snapshot ? (
        <p className="muted">Waiting for first snapshot…</p>
      ) : shown === 0 ? (
        <p className="muted">No flows match the current filters.</p>
      ) : (
        <div className="table-wrap">
          <table className="connections">
            <thead>
              {table.getHeaderGroups().map((hg) => (
                <tr key={hg.id}>
                  {hg.headers.map((h) => {
                    const sorted = h.column.getIsSorted();
                    const headTips: Record<string, string> = {
                      host:
                        "Shows the remote side (dst IP or hostname). WAN focus hides obvious local noise.",
                      country:
                        "GeoIP country code when a MaxMind DB path is configured.",
                      protocol: "L4 protocol observed for this flow aggregate.",
                      src_port: "Local ephemeral port when visible.",
                      dst_port: "Destination service port (443=HTTPS, 53=DNS, …).",
                      bytes_up:
                        "Bytes attributed to outbound direction for this flow aggregate.",
                      bytes_down: "Bytes attributed to inbound replies.",
                      bytes: "Combined bytes for sorting heavy talkers.",
                      packets: "Packets seen for this flow aggregate.",
                      first_seen:
                        "First packet time in this capture session for the flow.",
                      last_seen:
                        "Most recent packet observed for this aggregated flow.",
                    };
                    const tip = headTips[h.column.id] ?? "";
                    return (
                      <th
                        key={h.id}
                        style={{ width: h.getSize() }}
                        title={tip}
                        onClick={h.column.getToggleSortingHandler()}
                        className={sorted ? "sorted" : undefined}
                      >
                        {flexRender(h.column.columnDef.header, h.getContext())}
                        {sorted === "asc" && " ↑"}
                        {sorted === "desc" && " ↓"}
                      </th>
                    );
                  })}
                </tr>
              ))}
            </thead>
            <tbody>
              {table.getRowModel().rows.map((row) => (
                <tr
                  key={row.id}
                  data-flow-id={row.original.id}
                  className={
                    highlightFlowId === row.original.id ? "row-highlight" : undefined
                  }
                >
                  {row.getVisibleCells().map((cell) => (
                    <td key={cell.id} style={{ width: cell.column.getSize() }}>
                      {flexRender(cell.column.columnDef.cell, cell.getContext())}
                    </td>
                  ))}
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </section>
  );
}

function textFilter(row: { original: FlowRecord }, _id: string, value: string): boolean {
  if (!value) return true;
  const q = value.toLowerCase();
  const f = row.original;
  return (
    f.src_ip.toLowerCase().includes(q) ||
    f.dst_ip.toLowerCase().includes(q) ||
    (f.hostname?.toLowerCase().includes(q) ?? false) ||
    (f.country?.toLowerCase().includes(q) ?? false)
  );
}

function formatBytes(n: number): string {
  if (n < 1024) return `${n}`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)}K`;
  if (n < 1024 * 1024 * 1024) return `${(n / 1024 / 1024).toFixed(1)}M`;
  return `${(n / 1024 / 1024 / 1024).toFixed(2)}G`;
}

function formatTime(ms: number): string {
  if (!ms) return "—";
  return new Date(ms).toLocaleTimeString();
}
