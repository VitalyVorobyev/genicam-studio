# GenICam Studio — Product Backlog

## Current Status

**M1–M5 complete.** M6 is next: E2E testing with real camera service and production hardening.

**Goal:** Production-ready desktop application for GenICam device operation via genicam-service + Zenoh IPC.

## Scope

GenICam Studio is the **desktop UI application**. The camera service (`genicam-service` in the `genicam-rs` repo) handles hardware communication. This backlog covers:
- **E2E testing** with genicam-service + arv-fake-gv-camera
- **Production hardening** (logging, timeouts, reconnection, error UX)
- **Embed streamer** into Tauri backend (eliminate subprocess)
- **Shared XML crate** extracted from genicam-rs for offline browsing
- **Recording** raw frames to disk
- **Release packaging** (DMG, AppImage, MSI)

## Legend

- **Priority:** P0 = must-have for release, P1 = important, P2 = nice-to-have
- **Size:** S = small (1–2 days), M = medium (3–5 days), L = large (1–2 weeks), XL = extra-large (2+ weeks)
- **Status:** `planned` | `in-progress` | `done` | `blocked`

---

## Archived: M1–M5 (complete)

<details>
<summary>Completed milestones and epics (click to expand)</summary>

### M1: Foundation ✓
XML parser with UiGraph contract, Feature Browser, WASM adapter, Tauri v2 shell, Zenoh API spec, Tauri backend (discovery, connect, node read/write, acquisition), WebSocket streamer (Mono8), Image Viewer, Device sidebar, keyboard shortcuts.

### M2: Mock Service & API Polish ✓
Epic 1 (Mock Camera Service) — 13/13 complete. Realistic SFNC node set, synthetic multi-format images, node interdependencies, bulk read.

### M3: Image Viewer v2 ✓
Image Viewer layout redesign, acquisition controls, exposure/gain sliders, image format controls, zoom/pan, pixel inspector. SFNC groups runtime mapping. UX design system.

### M4: Multi-Format & Image Tools ✓
Inline frame header (ZA-04). Multi-format streamer + renderer (Mono10–16, RGB8, Bayer). Streamer lifecycle monitoring. Error recovery. Histogram, ROI selection, line profile.

### M5: Polish & Feature Browser ✓
UX polish (device sidebar, feature browser, responsive layout, loading/error states, toast notifications). Live values in tree, batch apply, export state.

### Completed Epics
- **Epic 1: Mock Camera Service** — 13/13 ✓
- **Epic 2: Zenoh API & Shared Types** — 7/7 ✓
- **Epic 4: Tauri Backend Improvements** — 7/7 ✓
- **Epic 5: Image Viewer UI** — 17/17 ✓
- **Epic 7: UX Polish & Design** — 9/9 ✓
- **Epic 3: Streamer Evolution** — 3/5 ✓ (ST-04, ST-05 carry forward to Epic 12)
- **Epic 6: Feature Browser** — 3/5 ✓ (FB-03, FB-04 carry forward to Epic 14)
- **Epic 8: XML Parser** — superseded by Epic 13 (Shared XML Crate)
- **Epic 9: Infrastructure & CI** — 1/4 ✓ (CI-01 done; CI-02–04 carry forward)

</details>

---

## Epic 10: E2E Testing & CI — 0/7 complete

Automated end-to-end tests with genicam-service + arv-fake-gv-camera. Extends CI beyond unit/integration tests.

| ID | Task | Priority | Size | Status | Notes |
|----|------|----------|------|--------|-------|
| E2E-01 | Test harness: spawn fake camera + service | P0 | M | planned | Rust helper that starts `arv-fake-gv-camera-0.8 -i 127.0.0.1` and `genicam-service` as child processes, polls discovery announce for readiness, tears down via `Drop`. Configurable timeout. |
| E2E-02 | E2E test: discovery + connect + XML fetch | P0 | M | planned | Open Zenoh session, subscribe to announce, assert fake camera fields. Query XML, parse with `genicam_xml_model`, assert key SFNC nodes exist. |
| E2E-03 | E2E test: node read/write cycle | P0 | M | planned | Bulk-read Width+Height (default 512×512). Write Width=320, read back, assert. Write invalid value, assert error. Test command execution. |
| E2E-04 | E2E test: acquisition + frame reception | P0 | L | planned | Start acquisition, subscribe to image key, assert frames with valid FrameHeader. Assert acquisition/status reports active + nonzero FPS. Stop, assert frames stop. |
| E2E-05 | E2E test: device lost detection | P1 | M | planned | Kill arv-fake-gv-camera while connected. Assert service publishes disconnect status within timeout. |
| E2E-06 | CI workflow: E2E job with aravis | P0 | L | planned | GitHub Actions job: install aravis, build genicam-service (checkout genicam-rs), run E2E tests. Ubuntu. Cache cargo. Separate from fast CI job. |
| E2E-07 | CI workflow: Tauri build verification | P1 | M | planned | Add `cargo tauri build` to CI. Install system deps (webkit2gtk). macOS + Linux matrix. |

---

## Epic 11: Production Hardening — 8/8 complete

Error handling, reconnection, logging, and reliability for daily use with real cameras.

| ID | Task | Priority | Size | Status | Notes |
|----|------|----------|------|--------|-------|
| PH-01 | Structured logging with `tracing` | P0 | M | done | Replace all `eprintln!` in Tauri backend with `tracing`. Add `tracing-subscriber` with `EnvFilter`. |
| PH-02 | Zenoh query timeout wrappers | P0 | M | done | Explicit `.timeout(5s)` on all `session.get()` calls. User-friendly timeout error messages. |
| PH-03 | Zenoh session health monitor | P1 | M | done | Background task with liveliness token check every 10s. Emits `zenoh-session-lost`/`zenoh-session-restored` events. |
| PH-04 | Device reconnection on network glitch | P1 | L | done | `ConnectionState::Reconnecting` with exponential backoff (1–15s, 5 attempts). Auto-reconnects when device reappears in registry. UI shows attempt counter. |
| PH-05 | User-friendly error messages | P0 | M | done | Error mapping layer: "Camera service not running", "Camera busy", "Value out of range", "Network unreachable". No raw Rust errors shown to users. |
| PH-06 | Graceful service crash recovery | P1 | M | done | Emergency disconnect triggers auto-reconnect loop. Cleans up acquisition state. UI shows reconnecting banner with reason + attempt counter. Falls back to Error state after max attempts. |
| PH-07 | Connection state persistence | P2 | M | done | Persists last-connected device_id in localStorage. Shows toast when device reappears after restart. |
| PH-08 | Log file sink | P2 | S | done | Daily-rotating `~/.genicam-studio/logs/studio.log.YYYY-MM-DD` via `tracing-appender`. Non-blocking dual output (stderr + file). |

---

## Epic 12: Embed Streamer — 4/6 complete

Move WebSocket image streamer from separate subprocess into Tauri backend process. Keep WS protocol to frontend.

| ID | Task | Priority | Size | Status | Notes |
|----|------|----------|------|--------|-------|
| ES-01 | Extract streamer into library crate | P0 | M | done | Create `crates/genicam_streamer/` with BMP, WS, Zenoh source modules. Existing binary becomes thin `main.rs` calling `genicam_streamer::run()`. |
| ES-02 | Embed streamer tasks in Tauri backend | P0 | L | done | Replaced subprocess with tokio tasks using `genicam_streamer::run_with_session()` and `run_server_with_listener()`. Auto-assigns WS port. |
| ES-03 | Share Zenoh session | P0 | M | done | Modify streamer to accept `Arc<zenoh::Session>` instead of opening its own. Eliminates duplicate session. |
| ES-04 | Remove process management code | P1 | S | done | Deleted `StreamerStatus`, `StreamerArgs`, `spawn_streamer_child`, `run_streamer_monitor`, `resolve_streamer_path`. Simplified `AcquisitionInner` to use `shutdown_tx` + `task_handles`. |
| ES-05 | PNG/JPEG encoding option | P2 | M | planned | Carry-forward ST-04. Add `--format bmp|png|jpeg`. Configurable via IPC in embedded mode. |
| ES-06 | Frame annotation overlay | P2 | M | planned | Carry-forward ST-05. Optional frame ID, timestamp, FPS text burned into image. Toggle from toolbar. |

---

## Epic 13: Shared XML Crate — 5/5 complete

Extract genapi-xml/core from genicam-rs into a standalone crate. Both repos depend on it. Enables offline XML browsing with full node resolution.

| ID | Task | Priority | Size | Status | Notes |
|----|------|----------|------|--------|-------|
| SX-01 | Design shared crate API surface | P0 | M | done | Defined in `docs/handoffs/SX-genicam-rs-handoff.md`. genapi-xml + genapi-core stay in genicam-rs, studio depends via path. |
| SX-02 | Extract genapi-xml + genapi-core | P0 | XL | done | Crates in genicam-rs with full serde, introspection API, NullIo, WASM compat. |
| SX-03 | Integrate into genicam-studio | P1 | L | done | `parse_full()` in genicam_xml_model uses genapi-xml + genapi-core. UiNode extended with dependencies, dependents, expression, int_min/max/inc. |
| SX-04 | Offline XML browsing with full resolution | P1 | L | done | WASM adapter exposes `parse_xml_full()` using genapi-xml + genapi-core. WebWasmProvider auto-selects full parser. Fallback to streaming parser on error. |
| SX-05 | Deprecate old Epic 8 | P1 | S | done | Epic 8 (XML Parser) superseded by Epic 13 (Shared XML Crate). genapi-xml + genapi-core from genicam-rs now handle all parsing. |

---

## Epic 14: Feature Browser Enhancements — 2/2 complete

Carry-forward from Epic 6.

| ID | Task | Priority | Size | Status | Notes |
|----|------|----------|------|--------|-------|
| FB-03 | Node dependency visualization | P2 | L | done | Dependencies/dependents shown as clickable links in FeaturePanel. Expression strings displayed for SwissKnife/Converter nodes. Links navigate to referenced nodes. |
| FB-04 | Favorites/pinned nodes | P2 | S | done | Pin nodes to "Favorites" section. Persist in localStorage. Star icon toggle on hover, collapsible Favorites category at tree top. |

---

## Epic 15: Release & Packaging — 0/4 complete

CI builds, release artifacts, and distribution.

| ID | Task | Priority | Size | Status | Notes |
|----|------|----------|------|--------|-------|
| RP-01 | Release packaging pipeline | P1 | L | planned | GitHub Actions: build DMG (macOS), AppImage (Linux), MSI (Windows). Upload to GitHub Releases on tag push. |
| RP-02 | Bundle genicam-service binary | P1 | M | planned | Tauri external binary sidecar. Auto-start if no external service detected. Per-platform builds. |
| RP-03 | Auto-update mechanism | P2 | M | planned | Tauri v2 updater plugin. GitHub Releases as update server. Check on startup. |
| RP-04 | Performance benchmarks in CI | P2 | M | planned | Criterion benchmarks: BMP encoding throughput, UiGraph parse time, Zenoh round-trip latency. Fail on >10% regression. |

---

## Epic 16: Recording & Playback — 0/4 complete

Raw frame recording to disk for post-processing and analysis.

| ID | Task | Priority | Size | Status | Notes |
|----|------|----------|------|--------|-------|
| REC-01 | Recording engine: raw frames to disk | P1 | L | done | `genicam_streamer::recording` module with .gsr format (JSON header + raw frames + index). Background writer with mpsc channel. 3 unit tests. |
| REC-02 | Recording UI controls | P1 | M | done | Tauri commands: `start_recording`, `stop_recording`, `get_recording_status`. Record button in ViewerToolbar with pulse animation. `useRecording` hook with status polling. |
| REC-03 | Playback engine | P2 | L | planned | Load `.gsr`, feed frames through BMP encoder + WS pipeline. Play/pause, frame step, speed control, seek. |
| REC-04 | Export to standard format | P2 | M | planned | Convert `.gsr` to TIFF stack or AVI (uncompressed). Enables interop with ImageJ, MATLAB, etc. |

---

## Milestone Plan

### M6: E2E Testing & Production Hardening ← next

**Focus:** E2E-01 through E2E-07, PH-01, PH-02, PH-05

**Goal:** Automated E2E test suite in CI against arv-fake-gv-camera + genicam-service. Structured logging. Timeout-protected Zenoh queries. User-friendly error messages. Tauri build verification.

**Exit criteria:**
- CI green with E2E tests covering discovery → connect → stream → disconnect
- All `eprintln!` replaced with `tracing`
- No raw Rust errors visible to users
- Tauri binary builds in CI (Linux + macOS)

### M7: Embed Streamer & Hardening Phase 2

**Focus:** ES-01 through ES-04, PH-03, PH-04, PH-06

**Goal:** Streamer runs as embedded tokio tasks in Tauri (no child process). Zenoh health monitoring. Device reconnection with backoff. Service crash detection.

**Exit criteria:**
- Acquisition works end-to-end with embedded streamer
- `genicam-ws-streamer` still builds standalone (optional use)
- App survives network glitch and reconnects
- App detects service crash with actionable error

### M8: Shared XML Crate

**Focus:** SX-01 through SX-05

**Goal:** Standalone `genapi-parser` crate from genicam-rs. Studio uses it for full XML parsing. Offline browsing with resolved cross-references.

**Exit criteria:**
- Shared crate compiles and passes all tests independently
- genicam-rs depends on shared crate (12 integration tests pass)
- Studio UiGraph includes resolved cross-references
- Offline XML browsing shows complete feature tree

### M9: Recording & Polish

**Focus:** REC-01, REC-02, FB-03, FB-04, ES-05, ES-06, PH-07, PH-08

**Goal:** Raw frame recording. Node dependency visualization. Favorites. PNG/JPEG encoding. Connection persistence. File logging.

**Exit criteria:**
- Record button captures raw frames during acquisition
- Dependency edges visible in feature browser
- Optional PNG/JPEG encoding

### M10: Release

**Focus:** RP-01 through RP-04, REC-03, REC-04

**Goal:** Release packaging, bundled service, auto-updates, benchmarks. Recording playback and export.

**Exit criteria:**
- Installable packages for macOS, Linux, Windows
- Auto-update mechanism functional
- Performance benchmarks in CI
- Recordings replayable and exportable
