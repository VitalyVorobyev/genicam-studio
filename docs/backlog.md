# GenICam Studio — Product Backlog

## Legend

- **Priority:** P0 = must-have for release, P1 = important, P2 = nice-to-have
- **Size:** S = small (1-2 days), M = medium (3-5 days), L = large (1-2 weeks), XL = extra-large (2+ weeks)
- **Status:** `planned` | `in-progress` | `✓ done` | `blocked`
- **ADR:** link to relevant architecture decision record

## Scope

GenICam Studio is the **desktop UI application**. The actual camera service (GenTL, hardware communication) lives in a separate repository. This backlog covers:
- The **Zenoh API contract** between Studio and the camera service
- The **UI/UX** for camera operation
- A **mock camera service** for integration testing
- The **Tauri backend** that bridges Zenoh and the UI
- The **WebSocket streamer** for image display

---

## Epic 1: Mock Camera Service — 13/13 complete ✓

A fake Zenoh camera service for end-to-end integration testing without real hardware. Enables UI development and CI.

| ID | Task | Priority | Size | Status | ADR | Notes |
|----|------|----------|------|--------|-----|-------|
| ~~MS-01~~ | ~~Mock service binary scaffold~~ | P0 | M | ✓ done | 008 | Zenoh session, CLI config (device name, image size, node set). Graceful shutdown. |
| ~~MS-02~~ | ~~Discovery announcer~~ | P0 | S | ✓ done | 008 | Periodic `announce` publisher with configurable device info. |
| ~~MS-03~~ | ~~XML queryable~~ | P0 | S | ✓ done | 008 | Serve a bundled GenICam XML fixture on `{id}/xml` queryable. |
| ~~MS-04~~ | ~~Node value store~~ | P0 | M | ✓ done | 008 | In-memory node store seeded from XML defaults. Publish changes to `nodes/{name}/value`. |
| ~~MS-05~~ | ~~Node set/execute queryables~~ | P0 | M | ✓ done | 008 | Handle `nodes/{name}/set` and `nodes/{name}/execute`. Validate types, update store. |
| ~~MS-06~~ | ~~Status publisher~~ | P0 | S | ✓ done | 008 | Publish `{id}/status` on connect/disconnect simulation. |
| ~~MS-07~~ | ~~Synthetic image generator~~ | P0 | M | ✓ done | 005 | Generate Mono8 test patterns (gradient, checkerboard). Publish to `{id}/image` at configurable FPS. |
| ~~MS-08~~ | ~~Acquisition control~~ | P0 | S | ✓ done | 008 | Handle `acquisition/control` start/stop. Publish `acquisition/status`. |
| ~~MS-09~~ | ~~Image metadata publisher~~ | P1 | S | ✓ done | 005 | Publish `{id}/image/meta` with pixel format, dimensions. |
| ~~MS-10~~ | ~~Multi-format synthetic images~~ | P1 | M | ✓ done | 005 | Generate Mono16, BayerRG8, RGB8 test patterns. |
| ~~MS-11~~ | ~~Realistic SFNC node set~~ | P0 | M | ✓ done | 007 | Comprehensive fixture XML with standard SFNC nodes: ExposureTime, Gain, Width, Height, PixelFormat, TriggerMode, AcquisitionMode, etc. |
| ~~MS-12~~ | ~~Node interdependencies~~ | P1 | M | ✓ done | — | Simulate node side effects (e.g., changing Width updates OffsetX max, changing PixelFormat changes payload size). |
| ~~MS-13~~ | ~~Bulk node read queryable~~ | P1 | S | ✓ done | 008 | Handle `nodes/bulk/read` for batch queries. |

---

## Epic 2: Zenoh API & Shared Types — 3/7 complete

Evolve the `genicam_zenoh_api` crate and `docs/zenoh-api.md`.

| ID | Task | Priority | Size | Status | ADR | Notes |
|----|------|----------|------|--------|-----|-------|
| ZA-04 | Frame metadata in image key | P1 | S | planned | 005 | Define frame header format (inline metadata vs separate key). |
| ZA-05 | API version negotiation | P2 | M | planned | 008 | Version field in `announce`, compatibility check. |
| ZA-06 | Node constraints in value updates | P1 | S | planned | — | Extend `NodeValueUpdate` to optionally include min/max/inc so UI can adapt to runtime constraint changes. |
| ZA-07 | API spec review & documentation | P0 | M | planned | 008 | Review `docs/zenoh-api.md` for completeness. Add sequence diagrams. Ensure spec matches `genicam_zenoh_api` types exactly. |
| ~~ZA-01~~ | ~~Image metadata types~~ | P1 | S | ✓ done | 005,008 | Add `ImageMeta` struct, `image_meta()` key helper. |
| ~~ZA-02~~ | ~~Bulk read types~~ | P1 | S | ✓ done | 008 | Add `BulkReadRequest`, `BulkReadResponse` types. |
| ~~ZA-03~~ | ~~PixelFormat enum (shared)~~ | P0 | S | ✓ done | 005 | Full SFNC pixel format enum in `genicam_zenoh_api`. |

---

## Epic 3: Streamer Evolution — 0/5 complete

Extend `genicam-ws-streamer` for multi-format support and better WebSocket protocol.

| ID | Task | Priority | Size | Status | ADR | Notes |
|----|------|----------|------|--------|-----|-------|
| ST-01 | Image metadata subscription | P1 | M | planned | 005 | Subscribe to `image/meta` to auto-configure encoder. Removes need for `--width`/`--height` CLI args. |
| ST-02 | Multi-format BMP encoder | P1 | L | planned | 005 | Extend BMP encoder for Mono10-16 (downscale), RGB8, Bayer (debayer+encode). |
| ST-03 | WebSocket info frame protocol | P1 | M | planned | — | Send JSON info frame on connect with pixel format, dimensions. Client uses this to configure renderer. |
| ST-04 | PNG/JPEG encoding option | P2 | M | planned | — | Add `--format bmp|png|jpeg` for compression. Useful for remote/slow connections. |
| ST-05 | Frame annotation overlay | P2 | M | planned | — | Optional frame ID, timestamp, FPS overlay burned into the image. |

---

## Epic 4: Tauri Backend Improvements — 1/7 complete

Evolve the Tauri app's Rust backend.

| ID | Task | Priority | Size | Status | ADR | Notes |
|----|------|----------|------|--------|-----|-------|
| TB-01 | Image metadata event | P1 | S | planned | 005 | Subscribe to `image/meta`, emit `image-meta-changed` to frontend. |
| TB-02 | Bulk node read command | P1 | S | planned | — | IPC command `read_nodes_bulk(names: Vec<String>)` for batch reads. |
| TB-03 | SFNC groups config loader | P0 | M | planned | 007 | Load `sfnc-groups.json` from app resources, expose via IPC. |
| TB-04 | Node value validation | P1 | M | planned | — | Validate node writes against UiGraph constraints before sending to service. |
| TB-06 | Error recovery on disconnect | P1 | M | planned | — | Auto-cleanup on unexpected disconnect. Reconnect prompt. |
| TB-07 | Streamer lifecycle improvement | P1 | M | planned | — | Health monitoring of streamer child process. Auto-restart on crash. |
| ~~TB-05~~ | ~~Connection profiles (localStorage)~~ | P1 | S | ✓ done | — | Already partially implemented (T7.4 in prior work). |

---

## Epic 5: Image Viewer UI — 0/17 complete

Build the dedicated Image Viewer with camera controls and image analysis tools.

| ID | Task | Priority | Size | Status | ADR | Notes |
|----|------|----------|------|--------|-----|-------|
| ~~IV-01~~ | ~~Image Viewer layout redesign~~ | P0 | M | ✓ done | 006 | Split layout: canvas area + collapsible control sidebar. Replace current minimal viewer. |
| ~~IV-02~~ | ~~Acquisition control section~~ | P0 | M | ✓ done | 006 | Start/Stop, AcquisitionMode enum, frame counter display. |
| ~~IV-03~~ | ~~Exposure & Gain section~~ | P0 | M | ✓ done | 006,007 | Slider controls for ExposureTime, Gain. Auto toggles for ExposureAuto, GainAuto. |
| ~~IV-04~~ | ~~Image Format section~~ | P0 | M | ✓ done | 006,007 | Width, Height, OffsetX, OffsetY, PixelFormat, Binning/Decimation controls. |
| IV-05 | Trigger Configuration section | P1 | M | planned | 006,007 | TriggerMode, TriggerSource, TriggerActivation, TriggerDelay. |
| IV-06 | Transport Layer section | P1 | S | planned | 006,007 | GevSCPSPacketSize, GevSCPD (GigE), or relevant USB3 settings. |
| IV-07 | Auto Functions section | P1 | S | planned | 006,007 | Auto-exposure, auto-gain, auto-white-balance settings. |
| IV-08 | Color Processing section | P1 | M | planned | 006,007 | White balance, gamma, LUT (when camera supports it). |
| IV-09 | SFNC groups runtime mapping | P0 | M | planned | 007 | Load `sfnc-groups.json`, match against UiGraph, show/hide sections. |
| IV-10 | Section persistence | P1 | S | planned | 006 | Save expand/collapse state per camera model in localStorage. |
| ~~IV-11~~ | ~~Zoom and pan~~ | P0 | M | ✓ done | — | Mouse wheel zoom, click-drag pan on the canvas. |
| ~~IV-12~~ | ~~Pixel inspector (crosshair)~~ | P0 | M | ✓ done | — | Hover crosshair showing pixel coordinates and value(s). |
| IV-13 | Histogram | P1 | L | planned | — | Live histogram (grayscale or per-channel). Overlay or sidebar panel. |
| IV-14 | ROI selection tool | P1 | M | planned | — | Drag-to-select ROI on canvas. Button to apply as Width/Height/OffsetX/OffsetY. |
| IV-15 | Line profile | P1 | L | planned | — | Draw a line on the canvas, show intensity profile plot. |
| IV-16 | Snapshot save | P1 | S | planned | — | Save current frame as PNG/TIFF. File dialog. |
| IV-17 | Multi-format rendering | P1 | L | planned | 005 | Adapt canvas rendering for different pixel formats from streamer info frame. |

---

## Epic 6: Feature Browser Improvements — 0/5 complete

Enhance the existing Feature Browser.

| ID | Task | Priority | Size | Status | ADR | Notes |
|----|------|----------|------|--------|-----|-------|
| FB-01 | Live value display in tree | P1 | M | planned | — | Show live values inline in the category tree (not just in FeaturePanel). |
| FB-02 | Batch apply | P1 | M | planned | — | Apply all pending drafts in one action. Progress indicator. |
| FB-03 | Node dependency visualization | P2 | L | planned | — | Show which nodes affect which (pSelected, pInvalidator relationships from XML). |
| FB-04 | Favorites/pinned nodes | P2 | S | planned | — | Pin frequently-used nodes for quick access. |
| FB-05 | Export full node state | P1 | M | planned | — | Export all current live values (not just drafts) as a preset file. |

---

## Epic 7: UX Polish & Design — 3/9 complete

Professional look and feel for the desktop app.

| ID | Task | Priority | Size | Status | ADR | Notes |
|----|------|----------|------|--------|-----|-------|
| ~~UX-02~~ | ~~App header & navigation~~ | P0 | M | ✓ done | — | Clean header with device status, tabs, acquisition controls. |
| UX-03 | Device sidebar polish | P1 | M | planned | — | Connection state badges, device cards, loading states. |
| UX-04 | Feature Browser polish | P1 | M | planned | — | Tree styling, kind badges, better search UX. |
| UX-05 | Image Viewer chrome | P0 | M | planned | — | Toolbar, status bar, professional canvas container. |
| UX-06 | Responsive layout | P1 | M | planned | — | Handle window resize, min sizes, splitter for panes. |
| UX-07 | Loading & error states | P1 | M | planned | — | Skeleton loaders, toast notifications, inline error messages. |
| ~~UX-01~~ | ~~Design system (tokens, theme)~~ | P0 | L | ✓ done | — | Dark theme, CSS custom properties, consistent spacing/typography. "Dark Instrument" aesthetic. |
| ~~UX-08~~ | ~~Keyboard shortcuts~~ | P1 | S | ✓ done | — | Ctrl+F search, Escape, Ctrl+Enter apply. Already implemented. |
| ~~UX-09~~ | ~~Window title updates~~ | P0 | S | ✓ done | — | Already implemented (T7.7). |

---

## Epic 8: XML Parser Improvements — 0/6 complete

Improve the `genicam_xml_model` crate.

| ID | Task | Priority | Size | Status | ADR | Notes |
|----|------|----------|------|--------|-----|-------|
| XP-01 | pValue / pSelected resolution | P1 | L | planned | 009 | Resolve node cross-references (IntSwissKnife, Converter, etc.) into the UiGraph. |
| XP-02 | pInvalidator tracking | P1 | M | planned | 009 | Track invalidation relationships so UI can refresh dependent nodes. |
| XP-03 | StructReg / MaskedIntReg | P2 | M | planned | 009 | Parse structured register nodes and expose as Integer/Enum nodes. |
| XP-04 | SwissKnife expression evaluation | P2 | L | planned | — | Parse and evaluate SwissKnife math expressions for computed node values. |
| XP-05 | GenICam schema version detection | P1 | S | planned | 009 | Detect schema version from XML namespace. Adjust parsing rules. |
| XP-06 | Merge XML files | P2 | M | planned | — | Support cameras that provide multiple XML fragments (extension XMLs). |

---

## Epic 9: Infrastructure & CI — 0/4 complete

Build tooling, testing, and deployment.

| ID | Task | Priority | Size | Status | ADR | Notes |
|----|------|----------|------|--------|-----|-------|
| CI-01 | End-to-end integration tests | P0 | M | planned | — | Mock service → Zenoh → Tauri backend → assertions. |
| CI-02 | Tauri build in CI | P1 | M | planned | — | Cross-platform Tauri builds (macOS, Linux, Windows). |
| CI-03 | Release packaging | P1 | L | planned | — | DMG (macOS), AppImage (Linux), MSI (Windows). Bundle streamer binary. |
| CI-04 | Performance benchmarks | P2 | M | planned | — | Frame throughput, node update latency, UI render performance. |

---

## Milestone Plan

### M1: Foundation ✓
- [x] XML parser with UiGraph contract
- [x] Feature Browser (category tree, node editors, search, visibility filter)
- [x] WASM adapter for browser mode
- [x] Tauri v2 desktop shell
- [x] Zenoh API spec and shared types
- [x] Tauri Zenoh backend (discovery, connect, node read/write, acquisition)
- [x] WebSocket streamer (Mono8)
- [x] Image Viewer (basic canvas)
- [x] Device sidebar, diagnostics tab, keyboard shortcuts

### M2: Mock Service & API Polish ✓
Focus: MS-01 through MS-13, ZA-01 through ZA-03
Goal: A mock Zenoh camera service with realistic SFNC nodes, synthetic multi-format images, node interdependencies, and bulk read. Polished API spec.

### M3: Image Viewer v2 ← current
Focus: IV-01 through IV-04, IV-09 through IV-12, UX-01, UX-02, UX-05
Goal: Professional Image Viewer with acquisition controls, exposure/gain sliders, image format controls, zoom/pan, pixel inspector.

### M4: Multi-Format & Image Tools
Focus: ST-01 through ST-03, IV-13 through IV-17, ZA-04
Goal: Multi-format streamer. Histogram, ROI selection, line profile, snapshot save.

### M5: Polish & Feature Browser
Focus: UX-03 through UX-07, FB-01 through FB-05, IV-05 through IV-08
Goal: Full UX polish, feature browser improvements, remaining Image Viewer sections.

### M6: Parser & Advanced Features
Focus: XP-01 through XP-06, ZA-06
Goal: Advanced XML parsing (SwissKnife, cross-references). Runtime node constraint propagation.

### M7: Release
Focus: CI-01 through CI-04, ZA-05, ZA-07
Goal: CI pipeline, packaging, integration tests, release builds.
