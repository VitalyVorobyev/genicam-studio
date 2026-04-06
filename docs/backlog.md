# GenICam Studio — Product Backlog

## Current Status

**M1–M9 complete.** Working toward first release.

**Goal:** Production-ready desktop application for GenICam device operation via genicam-service + Zenoh IPC.

## Scope

GenICam Studio is the **desktop UI application**. The camera service (`genicam-service` in the `genicam-rs` repo) handles hardware communication. This backlog covers:
- **Real service integration** testing and bug fixes
- **Frame annotation** overlay on live stream
- **Recording** playback and export
- **Release packaging** (DMG, AppImage, MSI)

## Legend

- **Priority:** P0 = must-have for release, P1 = important, P2 = nice-to-have
- **Size:** S = small (1–2 days), M = medium (3–5 days), L = large (1–2 weeks), XL = extra-large (2+ weeks)
- **Status:** `planned` | `in-progress` | `done` | `blocked`

---

## Archived Milestones

<details>
<summary>M1–M9 (click to expand)</summary>

### M1: Foundation
XML parser with UiGraph contract, Feature Browser, WASM adapter, Tauri v2 shell, Zenoh API spec, Tauri backend (discovery, connect, node read/write, acquisition), WebSocket streamer (Mono8), Image Viewer, Device sidebar, keyboard shortcuts.

### M2: Mock Service & API Polish
Mock camera service with realistic SFNC nodes, synthetic multi-format images, node interdependencies, bulk read.

### M3: Image Viewer v2
Acquisition controls, exposure/gain sliders, image format controls, zoom/pan, pixel inspector, SFNC groups runtime mapping, UX design system.

### M4: Multi-Format & Image Tools
Inline frame header (ZA-04). Multi-format streamer + renderer (Mono10–16, RGB8, Bayer). Histogram, ROI selection, line profile.

### M5: Polish & Feature Browser
UX polish (device sidebar, feature browser, responsive layout, loading/error states, toast notifications). Live values in tree, batch apply, export state.

### M6: E2E Testing & CI
E2E test harness (fake camera + service), CI workflow with aravis, Tauri build verification (Linux + macOS).

### M7: Production Hardening
Structured `tracing` logging, Zenoh timeout wrappers, session health monitor, device reconnection with backoff, user-friendly error messages, service crash recovery, connection persistence, log file rotation.

### M8: Embed Streamer
Streamer runs as embedded tokio tasks in Tauri (shared Zenoh session). Standalone `genicam-ws-streamer` binary retained for debugging.

### M9: Shared XML, Recording, Feature Browser
Shared genapi-xml + genapi-core crates from genicam-rs. Recording engine with .gsr format and UI controls. Node dependency visualization and favorites.

</details>

---

## Epic 17: Real Service Integration

Testing and bug fixes for end-to-end operation with genicam-service (real camera service) and aravis fake camera.

| ID | Task | Priority | Size | Status | Notes |
|----|------|----------|------|--------|-------|
| RSI-01 | Fix `extract_node_name` for set/execute keys | P0 | S | done | `genicam_zenoh_api::keys::extract_node_name` was hardcoded to "value" suffix. Fixed to use structural "nodes" guard. Enables node writes and command execution with real service. |
| RSI-02 | Streamer integration test (synthetic frames) | P0 | S | done | `tests/e2e/tests/streamer_e2e.rs`: publishes synthetic Mono8 frame on Zenoh, verifies BMP arrives over WebSocket. Tests exact Tauri code path without external binaries. |
| RSI-03 | Fix node value type mismatch | P1 | S | done | Added `value_as_u32()` helper in `acquisition.rs` that handles both `Number(640)` and `String("640")` JSON values. Service sends strings; studio now parses them correctly for Width/Height. |
| RSI-04 | Update CI to use genicam-rs `main` branch | P0 | S | done | Updated `.github/workflows/ci.yml` from `phase2_dev` to `main`. |
| RSI-05 | Validate full pipeline with aravis fake camera | P0 | M | done | E2E tests pass against real genicam-service + aravis fake camera: discovery, node write, acquisition. Streaming fixed by CODEX: macOS needs control reconnect before AcquisitionStart, stream registers before start, trailing byte trim. `test_manual_topology_frames_and_ws_stream` validates full pipeline with TCP Zenoh topology. |

---

## Epic 18: Frame Annotation Overlay

Optional text overlay burned into the live image stream.

| ID | Task | Priority | Size | Status | Notes |
|----|------|----------|------|--------|-------|
| FA-01 | Annotation rendering engine | P2 | M | planned | Render frame ID, timestamp, FPS as text overlay on BMP frames in the streamer. Configurable fields. |
| FA-02 | Annotation toggle in toolbar | P2 | S | planned | Toggle button in ViewerToolbar. Sends config to embedded streamer via watch channel. |

---

## Epic 19: Recording Playback & Export

Load and replay recorded `.gsr` files. Export to standard formats.

| ID | Task | Priority | Size | Status | Notes |
|----|------|----------|------|--------|-------|
| REC-03 | Playback engine | P2 | L | planned | Load `.gsr`, feed frames through BMP encoder + WS pipeline. Play/pause, frame step, speed control, seek. |
| REC-04 | Export to standard format | P2 | M | planned | Convert `.gsr` to TIFF stack or AVI (uncompressed). Enables interop with ImageJ, MATLAB, etc. |

---

## Epic 20: Release & Packaging

CI builds, release artifacts, and distribution.

| ID | Task | Priority | Size | Status | Notes |
|----|------|----------|------|--------|-------|
| RP-01 | Release packaging pipeline | P1 | L | planned | GitHub Actions: build DMG (macOS), AppImage (Linux), MSI (Windows). Upload to GitHub Releases on tag push. |
| RP-02 | Bundle genicam-service binary | P1 | M | planned | Tauri external binary sidecar. Auto-start if no external service detected. Per-platform builds. |
| RP-03 | Auto-update mechanism | P2 | M | planned | Tauri v2 updater plugin. GitHub Releases as update server. Check on startup. |
| RP-04 | Performance benchmarks in CI | P2 | M | planned | Criterion benchmarks: BMP encoding throughput, UiGraph parse time, Zenoh round-trip latency. Fail on >10% regression. |

---

## Milestone Plan

### M10: Real Service Integration ← current

**Focus:** RSI-01 through RSI-05

**Goal:** Verified end-to-end operation with genicam-service and aravis fake camera. Node writes, streaming, and recording all working against real service.

**Exit criteria:**
- Node writes work with real genicam-service (RSI-01 done)
- Streamer pipeline tested with synthetic frames (RSI-02 done)
- CI uses genicam-rs main branch
- Full pipeline validated manually with fake camera

### M11: Release Preparation

**Focus:** RP-01, RP-02, FA-01, FA-02

**Goal:** Installable packages with bundled service. Frame annotation overlay.

**Exit criteria:**
- DMG/AppImage/MSI builds in CI
- Service auto-starts as Tauri sidecar
- Frame annotation toggle in toolbar

### M12: Recording & Polish

**Focus:** REC-03, REC-04, RP-03, RP-04

**Goal:** Recording playback, export, auto-updates, benchmarks.

**Exit criteria:**
- Recordings replayable with play/pause/seek
- Export to TIFF stack
- Auto-update mechanism functional
- Performance benchmarks in CI
