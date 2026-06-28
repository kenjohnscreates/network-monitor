# Setup notes — macOS

Live capture on macOS goes through Berkeley Packet Filter (BPF) character devices: `/dev/bpf0`, `/dev/bpf1`, … These are owned by `root:wheel` and not readable by your user by default, so any libpcap-based program (Wireshark, tcpdump, this app) needs either elevated privileges or a one-time permission fix.

You can still **list** interfaces without these permissions; only **starting a capture** requires them.

## Option 1 — install ChmodBPF (recommended)

The Wireshark project ships a small launchd job that fixes BPF device ownership at boot. This is the standard libpcap-on-macOS approach.

```bash
brew install --cask wireshark
# or download just the ChmodBPF installer from https://www.wireshark.org/
```

Log out and back in once after installation.

## Option 2 — manual chmod (per session)

```bash
sudo chmod o+r /dev/bpf*
```

Resets on reboot.

## Option 3 — run as root (avoid for dev)

`sudo`-launching `pnpm tauri dev` is painful (hot reload + sudo + WebKit) and not recommended. It's fine for one-off CLI examples:

```bash
sudo cargo run -p capture_core --example capture en0 5
```

## Verifying capture works

```bash
# list interfaces (no privileges required)
cargo run -p capture_core --example list

# 5 seconds of JSONL packet events on en0 (requires permission)
sudo cargo run -p capture_core --example capture en0 5

# 10 seconds of aggregated flows, top 5 every refresh
sudo cargo run -p flow_engine --example aggregate en0 10 5
```

If the app shows a "permission" error in the Capture panel, click the inline hint or follow Option 1/2 above.

## First five minutes in the GUI

1. Choose a real interface (usually Wi‑Fi / `en0`).
2. **Start** capture; set Connections **focus** to **wan-ish** so link-local and obvious local noise is easier to ignore.
3. Sort by **total** bytes to see heavy talkers.
4. Skim **Signals** for explainable heuristics (tweak or disable under **settings**).
5. If you need country tags, download [GeoLite2 Country](https://dev.maxmind.com/geoip/geolite2-free-geolocation-data) yourself and set the path in Settings — do not commit `.mmdb` files to this repo.

## GeoIP database (optional)

Country enrichment requires a local MaxMind `.mmdb` file. The free [GeoLite2-Country](https://dev.maxmind.com/geoip/geolite2-free-geolocation-data) database works. Download it, then in **Settings → GeoIP database path**, browse to the `.mmdb` file. The app keeps the file path only — the DB is read from disk on demand.

## Reverse DNS (optional)

Toggle in **Settings → Reverse DNS lookup**. Resolution happens in a background worker thread using the OS resolver and is cached in-process; the capture path is never blocked on DNS.

## Where settings live

`<app config dir>/settings.json`. On macOS that's typically:

```
~/Library/Application Support/<bundle-id>/settings.json
```

Delete the file to reset to defaults.

## Known limitations

- **No process attribution.** macOS requires `lsof` polling or private APIs to map flows to PIDs.
- **IPv6** is parsed and aggregated but dashboards favour IPv4 in v1.
- **VPN interfaces** (`utun*`) often need extra permissions and may not surface useful traffic.
- **Linux/Windows** are untested. The capture pipeline is libpcap-based and should port without large changes, but isn't verified.
