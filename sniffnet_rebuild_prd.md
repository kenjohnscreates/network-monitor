# PRD - Network Traffic Monitor Desktop App

## 1. Product Summary
Build a cross-platform desktop application that gives users a clean, real-time view of local network traffic on their machine. The app should help a user understand which hosts, protocols, countries, ports, and processes are generating traffic, without requiring them to use terminal-based packet analysis tools.

This product is not a pentesting tool and should not include offensive security workflows. It is a user-facing network observability app for personal, ops, and small-team debugging use cases.

## 2. Goals
- Show real-time inbound and outbound network traffic in a desktop UI
- Surface the most useful packet/session metadata in a human-readable way
- Aggregate packets into meaningful summaries, charts, and tables
- Let users filter, search, and inspect traffic quickly
- Make capture permissions and security boundaries explicit
- Support Windows, macOS, and Linux
- Build with a safer, modular architecture that minimizes privilege exposure

## 3. Non-Goals
- No packet injection
- No offensive scanning or exploitation features
- No credential harvesting or deep payload extraction as a core feature
- No remote cloud dependency for basic monitoring
- No enterprise SIEM replacement in v1
- No full Wireshark-level protocol dissection in v1

## 4. Target Users
### Primary
- Developers debugging local apps and APIs
- Founders and operators who want a simpler alternative to Wireshark
- Power users monitoring suspicious traffic or system behavior
- Small IT admins troubleshooting endpoints

### Secondary
- Privacy-conscious users checking which apps are making connections
- Students learning networking concepts visually

## 5. Core User Stories
1. As a user, I want to start monitoring traffic with minimal setup.
2. As a user, I want to see which remote IPs/domains are communicating with my machine.
3. As a user, I want to understand traffic by protocol, country, port, and process.
4. As a user, I want to filter out noise and focus on suspicious or important traffic.
5. As a user, I want charts and summaries, not raw packet dumps only.
6. As a user, I want to export logs or summaries for later review.
7. As a user, I want the app to be safe and transparent about permissions.

## 6. Product Requirements

### 6.1 Capture Engine
The app must:
- Capture packets from selected network interfaces
- Support inbound and outbound traffic
- Support live capture start/stop/pause
- Support interface switching
- Handle packet parsing without crashing the UI
- Aggregate packets into flows or sessions where possible

The app should:
- Use libpcap/Npcap or equivalent OS-native capture layer
- Run capture logic in an isolated service/process where possible
- Drop privileges after capture initialization when the OS allows it

### 6.2 Traffic Views
The UI must provide:
- Real-time traffic overview dashboard
- Top hosts by upload/download
- Top protocols
- Top ports
- Geographic map or country list of remote endpoints
- Connection list with sortable columns
- Detail view for selected connection/flow

Recommended dashboard panels:
- Total upload/download
- Active connections
- Packets per second
- Protocol breakdown
- Country breakdown
- Historical mini-graphs

### 6.3 Filters and Search
Users must be able to filter by:
- IP address
- Domain or hostname
- Protocol
- Port
- Country
- Interface
- Direction (inbound/outbound)
- Time window

Users should be able to:
- Save filter presets
- Exclude local/private traffic
- Exclude common noise like mDNS, ARP, DHCP, etc.

### 6.4 Process Attribution
If feasible per OS, the app should map connections to local processes.

This feature should:
- Show process name and PID when reliably available
- Clearly indicate when process attribution is unavailable or partial
- Use OS-native APIs for process-to-socket mapping

### 6.5 DNS and Reverse Lookup
The app should:
- Resolve IPs to hostnames when possible
- Cache lookups locally
- Let users disable reverse DNS for privacy/performance
- Never require cloud DNS enrichment for baseline functionality

### 6.6 GeoIP Enrichment
The app should:
- Use a local GeoIP database for country-level enrichment
- Update GeoIP data only with explicit user consent or clear settings
- Gracefully handle unknown IPs and failed lookups

### 6.7 Notifications
Optional notifications may include:
- New country detected
- New domain detected
- Spike in traffic volume
- Connection to a watched host or port

Guardrails:
- Disabled by default in v1
- Local notifications only by default
- Webhooks or outbound alerts only as an advanced opt-in feature
- Redact sensitive metadata before any outbound notification

### 6.8 History and Export
The app should support:
- Session history for the current run
- Export to CSV and JSON
- Optional local encrypted session storage
- Manual clear/delete of stored history

V1 can skip:
- Continuous background daemon with months of retention
- Multi-device sync

## 7. Security Requirements

### 7.1 Architecture Safety
The system should be split into:
1. Capture service
2. Parsing/aggregation layer
3. Desktop UI

Security principles:
- Least privilege by default
- No root/admin UI process if avoidable
- Strict IPC boundary between capture service and UI
- Input validation on packets, imported files, and settings
- Timeouts and memory bounds for parser safety
- Crash isolation so malformed traffic cannot take down the full app

### 7.2 Permissions
The app must:
- Explain why packet capture permissions are needed
- Request the minimum viable permissions per OS
- Avoid broad privileged execution when a capability-based model is available
- Warn users when running with elevated privileges

### 7.3 Network Safety
The app must not:
- Send traffic data externally by default
- Auto-enable webhooks
- Download and execute remote code
- Auto-import remote rules or enrichment files without verification

### 7.4 Supply Chain Safety
Development requirements:
- Lock dependencies
- Generate SBOM in CI
- Run dependency vulnerability checks in CI
- Sign releases
- Publish checksums
- Maintain reproducible builds where practical

## 8. UX Requirements

### 8.1 Design Direction
The UI should feel:
- Clean
- Fast
- Readable for non-experts
- More approachable than Wireshark

### 8.2 Primary Screens
1. Welcome / permissions setup
2. Interface selection
3. Live dashboard
4. Connections table
5. Connection detail drawer/modal
6. Filters/settings
7. Export/history

### 8.3 Empty and Error States
Must handle:
- No capture permissions
- No active traffic
- Unsupported interface
- DNS disabled
- GeoIP DB missing
- Process mapping unavailable on this OS

## 9. Technical Architecture

### Option A - Rust desktop app (recommended)
- Core capture/parsing engine: Rust
- Desktop shell/UI: Tauri + Rust backend + React frontend
- Charts/UI: React + TypeScript
- Benefits: strong performance, safer memory model, cross-platform distribution

### Option B - Electron app
- Core capture engine: Rust sidecar
- Desktop shell/UI: Electron + React
- Use only if richer desktop integrations are needed and bundle size is acceptable

### Recommended high-level architecture
- `capture-service`: privileged process responsible only for interface access and raw packet acquisition
- `parser-engine`: packet normalization, protocol parsing, flow aggregation, DNS/GeoIP enrichment
- `desktop-ui`: charts, tables, filters, settings, exports
- `storage`: local SQLite for settings and optional history
- `ipc`: typed messages over local IPC between UI and backend services

## 10. Data Model

### Packet Event
- timestamp
- interface_id
- direction
- src_ip
- dst_ip
- src_port
- dst_port
- protocol
- packet_length
- transport_flags

### Flow Record
- flow_id
- first_seen
- last_seen
- src_ip
- dst_ip
- src_port
- dst_port
- protocol
- bytes_up
- bytes_down
- packets_up
- packets_down
- hostname
- country
- process_name
- pid

### Alert Event
- alert_id
- alert_type
- created_at
- severity
- flow_id
- summary

## 11. Platform Notes

### macOS
- May require special permissions and careful notarization/signing
- Need to test packet capture experience thoroughly

### Windows
- Likely depends on Npcap or equivalent driver support
- Installer flow must explain dependency clearly

### Linux
- Prefer capabilities over full root where possible
- Package-specific permission setup must be documented

## 12. MVP Scope - Personal Use Build

This version is for personal use only. That changes the priorities:
- Optimize for fast local development over polished distribution
- Support one primary OS first, then expand
- Minimize installer and packaging complexity
- Keep security sensible, but do not overbuild enterprise controls in v1
- Favor visibility and debugging value over perfect attribution accuracy

### Must Have
- Live packet capture on one selected interface
- Dashboard with total upload/download and packets per second
- Real-time connections table
- Protocol summary
- Port summary
- Country summary using local GeoIP DB
- Basic filters: protocol, port, host/IP, direction
- Reverse DNS lookup toggle
- Export current session to CSV/JSON
- Start/stop/pause capture
- Local-only app with no cloud sync

### Nice to Have
- Process attribution
- Saved filter presets
- Session persistence between launches
- Alerts for new hosts or ports
- Dark mode polish

### Explicitly Out of Scope for Personal v1
- Team features
- Multi-device sync
- Signed production installers
- Remote webhook notifications
- Long-term daemon/background service
- Full Wireshark-level inspection
- Payload inspection beyond minimal protocol metadata

## 13. Milestones

### Phase 0 - Discovery
- Confirm OS-level packet capture requirements
- Confirm library choices and licensing
- Define privileged boundary model
- Build packet capture proof of concept

### Phase 1 - Core Backend
- Interface discovery
- Start/stop capture
- Parse basic TCP/UDP/ICMP/IP metadata
- Flow/session aggregation
- Unit tests and fuzzing on parser inputs

### Phase 2 - Enrichment
- Reverse DNS
- GeoIP local DB integration
- Private/local network classification
- Optional process attribution prototype

### Phase 3 - Desktop UI
- Dashboard
- Connections table
- Filters
- Detail drawer
- Settings

### Phase 4 - Export and Packaging
- CSV/JSON export
- Installers for all target OSes
- Code signing and checksums
- Documentation and permission walkthroughs

### Phase 5 - Hardening
- Threat model
- Security review
- Dependency audit
- Fuzz tests
- Performance profiling

## 14. Acceptance Criteria
- User can install app and monitor network traffic on at least one supported interface
- Dashboard updates in near real time with stable performance
- User can sort and filter connections meaningfully
- No outbound telemetry is required for core functionality
- App survives malformed or high-volume traffic without crashing the UI
- Security documentation clearly explains permissions and data flows

## 15. Risks
- Cross-platform permission complexity
- OS-specific process attribution reliability
- UI performance under high packet volume
- GeoIP and DNS enrichment latency/noise
- User distrust if permissions are poorly explained
- Capture service vulnerability if not isolated correctly

## 16. Open Questions
- Do we need process attribution in MVP or can it wait?
- Should packet payload inspection be entirely omitted in v1?
- Should history be ephemeral by default for privacy?
- Do we want a consumer brand or a dev-tool brand?
- Are mobile companion views or remote dashboards worth considering later?

## 17. Recommended Build Plan for Cursor - Personal v1

The goal here is to make this easy for Cursor to execute in small, reviewable steps.

### Recommended initial target
Start with one OS first:
- Best first target: macOS or Linux
- Defer Windows until the capture flow is stable

### Suggested stack for personal v1
- Desktop shell: Tauri
- Frontend: React + TypeScript
- Core engine: Rust
- Local storage: SQLite or JSON files
- Charts: Recharts
- Table: TanStack Table
- Packet capture: pcap crate backed by libpcap/Npcap
- State management: simple React context or Zustand

### Cursor execution strategy
Use a staged build plan:
1. Build a working Rust packet capture CLI first
2. Add flow/session aggregation
3. Expose backend commands/events through Tauri
4. Build the UI dashboard and connection table
5. Add enrichment (DNS + GeoIP)
6. Add export and settings
7. Do a final cleanup/hardening pass

### Definition of done for personal v1
- You can launch the desktop app locally
- Select an interface
- See traffic updating in real time
- Filter and inspect connections
- Export a session
- Shut down cleanly without crashes

## 18. Suggested Repo Structure
- `/apps/desktop-ui`
- `/crates/capture-service`
- `/crates/parser-engine`
- `/crates/shared-types`
- `/crates/exporter`
- `/docs/threat-model.md`
- `/docs/permission-model.md`
- `/docs/release-process.md`

## 19. Future Monetization Paths
- Free open-source core + paid pro features
- Paid process attribution and long-term history
- Export/reporting packs for IT teams
- Lightweight endpoint monitoring for SMBs
- White-label observability desktop app for MSPs

## 20. Final Recommendation
For your personal-use rebuild, do not overcomplicate this. Build a local-first Tauri app with a Rust backend and React frontend. Start with one OS, one interface, basic flow aggregation, DNS/GeoIP enrichment, and a clean dashboard. Skip enterprise packaging, webhooks, team features, and heavy persistence in v1.

## 21. Cursor Build Spec - Personal V1

### Product goal
Build a personal desktop app that shows live local network traffic in a much simpler visual interface than Wireshark.

### V1 success criteria
The app is successful if it can:
- capture traffic from a selected interface
- display live traffic in a clean UI
- aggregate packets into readable flows
- show top hosts, protocols, ports, and countries
- let the user filter results
- export session data

### Primary technical decisions
- Rust for packet capture and aggregation
- Tauri for desktop shell
- React + TypeScript for UI
- Local-only operation
- No cloud services
- No sign-in
- No background daemon in v1

### Core modules

#### Module 1 - Rust capture core
Responsibilities:
- enumerate network interfaces
- open capture on a selected interface
- read packets continuously
- parse minimum needed metadata:
  - timestamp
  - src/dst IP
  - src/dst port
  - protocol
  - packet size
  - direction if inferable
- emit normalized packet events

Implementation notes:
- use the `pcap` crate
- support only IPv4 in the first pass if needed
- ignore unsupported packet types cleanly
- do not inspect packet payloads in v1

Acceptance criteria:
- CLI test can capture packets for a selected interface
- logs normalized packet events without crashing

#### Module 2 - Flow aggregation engine
Responsibilities:
- group packet events into flows
- maintain counters:
  - bytes up/down
  - packets up/down
  - first seen
  - last seen
- track top hosts and protocols in memory

Flow key:
- protocol
- src_ip
- dst_ip
- src_port
- dst_port

Implementation notes:
- use an in-memory hash map
- add periodic cleanup for stale flows
- publish snapshots every 500ms to 1s to avoid UI overload

Acceptance criteria:
- packet stream aggregates into stable flows
- top hosts/protocol totals update correctly

#### Module 3 - Enrichment
Responsibilities:
- reverse DNS lookup for remote IPs
- GeoIP country lookup from local DB

Implementation notes:
- make reverse DNS optional via settings toggle
- cache DNS results locally in memory
- use a local MaxMind-style country DB or compatible file
- do not auto-download DB in v1 unless explicitly triggered

Acceptance criteria:
- known remote IPs can resolve to hostname when enabled
- country field appears when lookup succeeds

#### Module 4 - Tauri backend bridge
Responsibilities:
- expose commands:
  - list_interfaces
  - start_capture
  - stop_capture
  - pause_capture
  - export_session
  - get_settings
  - save_settings
- stream snapshot updates to frontend via Tauri events

Acceptance criteria:
- frontend can start/stop capture
- frontend receives live updates without freezing

#### Module 5 - Frontend UI
Screens:
1. interface/setup screen
2. live dashboard
3. connections table
4. settings modal

Dashboard widgets:
- total upload
- total download
- packets per second
- top protocols
- top ports
- top countries
- top hosts

Connections table columns:
- host/IP
- hostname
- country
- protocol
- src port
- dst port
- bytes up
- bytes down
- packets
- first seen
- last seen

Filters:
- search by host/IP
- protocol
- port
- direction

Acceptance criteria:
- data updates live
- filters apply instantly
- user can inspect flows without lag at normal traffic volume

#### Module 6 - Export
Formats:
- CSV
- JSON

Export fields:
- all flow-level fields visible in table
- session start/end timestamps

Acceptance criteria:
- export works from UI
- exported file opens correctly in spreadsheet/editor tools

### Recommended repo structure for Cursor
- `/apps/desktop` - Tauri app and React UI
- `/crates/capture_core` - raw packet capture
- `/crates/flow_engine` - aggregation logic
- `/crates/enrichment` - DNS and GeoIP
- `/crates/shared_types` - event and flow structs
- `/docs/` - notes, setup, known issues

### Suggested implementation order for Cursor

#### Step 1 - project bootstrap
Prompt for Cursor:
"Create a Tauri + React + TypeScript desktop app with a Rust workspace. Add crates for `capture_core`, `flow_engine`, `enrichment`, and `shared_types`. Configure the workspace so the desktop app can import and call these crates. Keep the UI minimal for now."

#### Step 2 - interface discovery
Prompt for Cursor:
"Implement Rust code to enumerate available network interfaces using the pcap-compatible stack and expose a Tauri command `list_interfaces` returning id, name, description, and whether the interface appears up/usable."

#### Step 3 - packet capture CLI proof
Prompt for Cursor:
"Inside `capture_core`, implement a CLI-testable packet capture loop for a selected interface. Parse minimal packet metadata only: timestamp, src/dst IP, src/dst port, protocol, packet length. Print normalized packet events as JSON lines. Ignore unsupported packets safely."

#### Step 4 - flow aggregation
Prompt for Cursor:
"Implement a flow aggregation engine that consumes normalized packet events and maintains flow records with bytes up/down, packets up/down, first_seen, and last_seen. Add tests for flow grouping and stale-flow cleanup."

#### Step 5 - Tauri event streaming
Prompt for Cursor:
"Wire the Rust backend into Tauri. Add commands for `start_capture`, `stop_capture`, and `pause_capture`. Stream flow snapshot updates to the frontend no more than once per second using Tauri events."

#### Step 6 - basic dashboard UI
Prompt for Cursor:
"Build a React dashboard page that can select an interface, start/stop capture, and render summary cards for total upload, total download, packets per second, top protocols, and top hosts."

#### Step 7 - connections table
Prompt for Cursor:
"Build a sortable, filterable connections table using TanStack Table. Include host/IP, protocol, ports, bytes up/down, packets, first seen, and last seen. Add text search and protocol filter."

#### Step 8 - DNS enrichment
Prompt for Cursor:
"Add an optional reverse DNS enrichment module with an in-memory cache. Make it toggleable in settings. Enrich flows asynchronously so DNS lookups never block packet capture."

#### Step 9 - GeoIP enrichment
Prompt for Cursor:
"Add local GeoIP country lookup using a local database file configured by path in settings. Enrich remote IPs only. Fail gracefully if the DB is missing."

#### Step 10 - export
Prompt for Cursor:
"Add UI and Rust backend support to export the current session flow table to CSV and JSON. Let the user choose a file path and preserve stable field names."

#### Step 11 - settings and polish
Prompt for Cursor:
"Add a settings modal for reverse DNS toggle, GeoIP DB path, snapshot interval, and stale-flow timeout. Persist settings locally. Improve empty states and error handling."

### Minimal data contracts

#### PacketEvent
- timestamp: string or i64
- src_ip: string
- dst_ip: string
- src_port: number | null
- dst_port: number | null
- protocol: string
- packet_length: number
- direction: string | null

#### FlowRecord
- id: string
- src_ip: string
- dst_ip: string
- hostname: string | null
- country: string | null
- src_port: number | null
- dst_port: number | null
- protocol: string
- bytes_up: number
- bytes_down: number
- packets_up: number
- packets_down: number
- first_seen: string
- last_seen: string

### Personal-use simplifications
Because this is just for personal use, v1 should intentionally skip:
- signed installers
- strict privilege separation into multiple OS processes
- advanced alerting
- webhook notifications
- payload inspection
- process attribution if it slows the build too much
- automated update system
- multi-OS parity on day one

### Guardrails for Cursor
Tell Cursor to follow these rules:
- keep modules small and testable
- prefer explicit types over clever abstractions
- do not add cloud services
- do not add authentication
- do not add packet payload parsing
- do not optimize prematurely
- make each step runnable before moving on
- include comments only where they clarify non-obvious networking logic

### Manual test plan
- launch app
- list interfaces successfully
- start capture on active interface
- browse the web and confirm new flows appear
- verify top hosts/protocols update
- toggle DNS on and off
- verify GeoIP works when DB path is valid
- filter by protocol and IP
- export to CSV and JSON
- stop capture and confirm app remains stable

### Nice follow-on after v1
After personal v1 works, the next best additions are:
1. process attribution
2. per-host bandwidth charts
3. session persistence
4. Linux/macOS packaging
5. stronger privileged boundary

## 22. Master Cursor Prompt - Build Personal V1

Use the following as the top-level prompt in Cursor.

```text
You are helping me build a personal-use desktop network traffic monitor. This is a local-only app for my own machine, not a commercial or enterprise product. The goal is to create a simpler visual alternative to Wireshark that shows live network traffic in a clean UI.

## Product goal
Build a Tauri desktop app with a Rust backend and React + TypeScript frontend that:
- lists available network interfaces
- captures traffic from a selected interface
- parses minimal packet metadata only
- aggregates packets into readable flows
- displays a live dashboard and connections table
- optionally enriches flows with reverse DNS and country lookup
- exports current session data to CSV and JSON

## Important constraints
This is personal-use v1, so keep the scope tight.

Do:
- optimize for local development speed and maintainability
- keep modules small and testable
- prefer explicit types and straightforward code
- make each phase runnable before moving to the next
- include tests for the flow aggregation logic
- keep packet capture and enrichment decoupled
- use event throttling so the UI is not flooded

Do not:
- add cloud services
- add sign-in or authentication
- add telemetry
- add webhook notifications
- add packet payload inspection
- add offensive security functionality
- add team or multi-device features
- overengineer enterprise packaging or release signing in v1
- introduce clever abstractions unless they clearly reduce complexity

## Stack
- Desktop shell: Tauri
- Frontend: React + TypeScript
- Backend/core: Rust workspace
- Packet capture: `pcap` crate
- Charts: Recharts
- Table: TanStack Table
- Local settings: JSON file or lightweight SQLite

## Workspace structure
Create this repo structure:
- /apps/desktop
- /crates/capture_core
- /crates/flow_engine
- /crates/enrichment
- /crates/shared_types
- /docs

## Core data types
Define shared Rust and TypeScript-compatible data models for:

PacketEvent:
- timestamp
- src_ip
- dst_ip
- src_port
- dst_port
- protocol
- packet_length
- direction

FlowRecord:
- id
- src_ip
- dst_ip
- hostname
- country
- src_port
- dst_port
- protocol
- bytes_up
- bytes_down
- packets_up
- packets_down
- first_seen
- last_seen

## Phased execution plan
Work in order. After each phase, stop and summarize what was created, which files changed, how to run it, and any known issues.

### Phase 1 - bootstrap
Create a Tauri + React + TypeScript app in /apps/desktop and a Rust workspace with crates for capture_core, flow_engine, enrichment, and shared_types. Wire the workspace so the desktop backend can call the crates cleanly. Keep the initial UI very simple.

Deliverables:
- working app bootstrap
- Rust workspace configured
- basic README with local run instructions

### Phase 2 - interface discovery
Implement backend logic to enumerate available network interfaces using the pcap-compatible stack and expose a Tauri command:
- list_interfaces

Return:
- id
- name
- description
- whether interface appears usable

Deliverables:
- Rust implementation
- Tauri command
- simple frontend interface list view

### Phase 3 - capture core
Implement packet capture in capture_core for a selected interface. Parse minimal packet metadata only:
- timestamp
- src/dst IP
- src/dst port
- protocol
- packet length
- direction if inferable

Ignore unsupported packets safely. Do not parse payloads.

Deliverables:
- capture loop
- normalized PacketEvent output
- safe handling of unsupported traffic
- ability to start and stop capture from backend

### Phase 4 - flow engine
Implement flow aggregation in flow_engine using an in-memory hash map keyed by protocol + src/dst IP + src/dst port. Maintain:
- bytes_up/down
- packets_up/down
- first_seen
- last_seen

Publish snapshots at a throttled interval between 500ms and 1s.

Deliverables:
- flow aggregation logic
- stale flow cleanup
- unit tests for grouping and cleanup

### Phase 5 - Tauri bridge
Expose backend commands:
- list_interfaces
- start_capture
- stop_capture
- pause_capture
- get_settings
- save_settings
- export_session

Use Tauri events to push throttled flow snapshots to the frontend.

Deliverables:
- working command/event bridge
- no UI freezing under normal traffic

### Phase 6 - dashboard UI
Build a clean dashboard with:
- interface selector
- start/stop/pause controls
- total upload
- total download
- packets per second
- top protocols
- top ports
- top hosts
- top countries placeholder if enrichment not done yet

Deliverables:
- dashboard page
- live updating summary cards/charts

### Phase 7 - connections table
Build a sortable, filterable connections table using TanStack Table.
Columns:
- host/IP
- hostname
- country
- protocol
- src port
- dst port
- bytes up
- bytes down
- packets
- first seen
- last seen

Filters:
- text search by host/IP
- protocol
- port
- direction

Deliverables:
- usable live table
- client-side filtering and sorting

### Phase 8 - enrichment
Implement enrichment in a non-blocking way.

Part A - reverse DNS:
- optional
- toggle in settings
- cache results in memory
- must never block packet capture

Part B - GeoIP:
- use local database file path from settings
- enrich remote IPs only
- fail gracefully if DB missing

Deliverables:
- async enrichment pipeline
- settings support
- enriched hostname/country fields when available

### Phase 9 - export and settings
Add:
- export current session to CSV and JSON
- settings modal for reverse DNS toggle, GeoIP DB path, snapshot interval, stale-flow timeout
- local settings persistence

Deliverables:
- export working from UI
- settings saved locally

### Phase 10 - cleanup
Refactor only where necessary. Improve error states, empty states, and run instructions.

Deliverables:
- cleaner code
- more robust UX
- docs for setup and known limitations

## Implementation notes
- Favor a single-process local app for v1 unless a separate capture service is clearly necessary
- Keep packet parsing minimal and safe
- Support IPv4 first if that meaningfully simplifies v1
- DNS and GeoIP must be optional and resilient
- Snapshot updates should be throttled
- Avoid expensive recomputation in the frontend

## Expected frontend screens
1. setup / interface selection
2. live dashboard
3. connections table
4. settings modal

## Manual test plan
Implement enough so I can do this manually:
- launch app
- see interface list
- select interface
- start capture
- browse websites and watch new flows appear
- verify top hosts and protocols update
- toggle DNS on/off
- set GeoIP DB path and confirm countries appear when available
- filter table by protocol and IP
- export CSV and JSON
- stop capture without crashes

## Output format for each phase
For each phase:
1. summarize what you changed
2. list files created/edited
3. explain how to run/test it
4. note any blockers or assumptions
5. then stop and wait for my review before continuing

Start with Phase 1 only.
```

## 23. Cursor Kickoff Recommendation
When you paste the master prompt into Cursor, start in a brand new repo and let it execute Phase 1 only. Review the file structure and run instructions before asking it to continue.

A good next message to Cursor after Phase 1 is:

```text
Good. Now continue with Phase 2 only. Keep changes minimal and do not touch unrelated files.
```

## 24. Suggested First Human Review Checklist
After Cursor finishes Phase 1, check:
- does the repo structure match the plan?
- does the app launch locally?
- are Rust workspace dependencies clean?
- is the Tauri app actually wired to the workspace crates?
- did Cursor keep the UI minimal instead of overbuilding?
- are the run instructions accurate?
```

