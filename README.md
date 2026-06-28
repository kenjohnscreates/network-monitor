# Network Monitor

Personal-use desktop app for monitoring local network traffic on macOS. A simpler, opinionated alternative to Wireshark for everyday "what's my machine talking to" questions.

> **Status:** v1.0 (Phases 1–10 complete). Live capture, flow aggregation, dashboard, connections table, optional rDNS + GeoIP enrichment, CSV/JSON export, persistent settings.

> **Inspired by [Sniffnet](https://github.com/GyulyVGC/sniffnet).** Built from scratch in Rust + Tauri as a learning project and a personal daily-use tool, focused on asset visibility rather than full protocol forensics.

## Stack

- **Desktop shell:** Tauri 2
- **Frontend:** React 18 + TypeScript + Vite + TanStack Table + Recharts
- **Backend:** Rust workspace
  - `capture_core` — `libpcap` + `etherparse` packet capture and parsing
  - `flow_engine` — bidirectional flow aggregation
  - `enrichment` — non-blocking rDNS + GeoIP
  - `shared_types` — serde contracts crossing the Tauri bridge
- **Package manager:** pnpm

## Repo layout

```
apps/desktop/            # Tauri 2 + React app
  src/                   # React frontend
  src-tauri/             # Tauri Rust backend
crates/
  shared_types/          # PacketEvent, FlowRecord, Direction, Protocol
  capture_core/          # Live capture pipeline
  flow_engine/           # Flow keys, aggregation, top-K helpers
  enrichment/            # rDNS + GeoIP worker
docs/
  setup.md               # macOS BPF permissions + run notes
```

## Prerequisites (macOS)

- Xcode Command Line Tools: `xcode-select --install`
- Rust (stable) via [rustup.rs](https://rustup.rs)
- Node 20+
- pnpm 10+ (`curl -fsSL https://get.pnpm.io/install.sh | sh -`)
- BPF read access — see [`docs/setup.md`](docs/setup.md). Without it, the app will list interfaces but live capture will fail with a permission error.

## Run in dev

```bash
pnpm install
cargo build --workspace

cd apps/desktop
pnpm tauri dev
```

Then in the window:
1. Pick a usable interface (e.g. `en0`).
2. Click **start**. Within ~1s the dashboard fills in: total upload/download, packets/sec sparkline, top hosts/protocols/ports.
3. Use the connections table to filter by host, protocol, port, or direction — try **focus: wan-ish** to hide obvious local multicast and pure RFC1918↔RFC1918 chatter while you orient.
4. Open **Signals** for rule-based heuristic cues (toggle rules under **settings**); cues are investigative hints, not malware verdicts.
5. Optional: open **settings** → enable Reverse DNS, browse to a GeoLite2 `.mmdb` file → settings persist across launches.
6. **export csv** / **export json** writes the current session to a file.

### First five minutes (orientation)

Quick path for newcomers: select an interface → **start** → Connections **focus: wan-ish** → sort by **total** → skim **Signals** for rare ports or upload-heavy WAN flows → if an unfamiliar remote IP bothers you, look it up on your terms (many people use external reputation tools such as [VirusTotal IP search](https://www.virustotal.com/gui/home/search); that sends data to VirusTotal).

More setup detail (BPF, GeoIP downloads) stays in [`docs/setup.md`](docs/setup.md).

### Sensitive issue attachments

Avoid attaching exports with live IPs/credentials to public issues — see [`SECURITY.md`](SECURITY.md).

## Build a release bundle

```bash
cd apps/desktop
pnpm tauri build
```

Replace `apps/desktop/src-tauri/icons/icon.png` with a real RGBA 512×512 PNG before distributing.

## CLI examples

Quick sanity checks without launching the GUI:

```bash
cargo run -p capture_core --example list           # list interfaces
sudo cargo run -p capture_core --example capture en0 5   # 5s of JSONL packet events
sudo cargo run -p flow_engine --example aggregate en0 10 5  # top flows for 10s, refreshing every 5
```

## Tests

```bash
cargo test --workspace
```

Covers flow keying, aggregation, top-K helpers, direction inference, packet parsing, enrichment cache behaviour, and anomaly heuristics (`flow_engine::anomaly`).

## Privacy

This app:
- captures only packet headers (no payloads)
- runs entirely locally — no network calls outside of optional reverse DNS and your locally provided GeoIP DB
- persists only your settings (`<app config dir>/settings.json`); flows live in memory and are cleared on stop or quit
- computes **Signals** heuristics on-device from the same aggregates the UI sees; disable or tune rules under settings (they’re explainable cues, not AV verdicts)

## GeoLite2 redistribution

MaxMind GeoLite2 databases have [license constraints](https://dev.maxmind.com/geoip/geolite2-free-geolocation-data). Download your own `.mmdb` from MaxMind — do **not** commit GeoIP files to git (see [.gitignore](.gitignore)).

## Known limitations

- **macOS-first.** Code paths that depend on `/dev/bpf*` and Wireshark's ChmodBPF are macOS-specific. Linux likely works (libpcap + cap_net_raw) but is untested.
- **No process attribution.** Mapping flows to PIDs requires `lsof` polling or private APIs; deferred from v1.
- **IPv6 supported but not prioritized** in dashboards.
- **VPN interfaces (`utun*`)** often need extra permissions and may not yield meaningful packets.
- **Snapshot-based UI.** The connections table shows up to `top_n` (default 250) flows; the dashboard totals/top-K are computed over the *full* aggregator state, so they remain accurate even with that cap. **Signals** may reference a flow that is outside the current top-N table — scroll/highlight only applies when that row is visible.
- **Reverse DNS** uses the OS resolver synchronously on a worker thread; results are cached in-process for the lifetime of the app.

## Roadmap

v1.0 is complete. All ten original build phases shipped:

- [x] Phase 1 — Tauri + Rust workspace bootstrap
- [x] Phase 2 — Interface discovery via pcap
- [x] Phase 3 — Live packet capture
- [x] Phase 4 — Flow aggregation engine
- [x] Phase 5 — Tauri command/event bridge
- [x] Phase 6 — Live dashboard UI
- [x] Phase 7 — Connections table
- [x] Phase 8 — DNS + GeoIP enrichment
- [x] Phase 9 — Export + settings
- [x] Phase 10 — Cleanup + hardening

## How it was built

This project was built with LLM-assisted development in Cursor. The architecture, PRD, decisions, and review were mine. The implementation was a collaboration between me and the model, with every change reviewed and tested. I am open about this because the field is moving fast and the README should reflect how the code actually came to be.

## License

[MIT](LICENSE)

## Contributing

This is a personal project. Issues are welcome. I am not actively seeking PRs but will consider them.
