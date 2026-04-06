# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

GenICam Studio is a desktop application (Tauri v2) for machine vision engineers to discover, configure, and operate GenICam-compliant industrial cameras. The app communicates with cameras through a separate camera service process over Zenoh.

Key docs:
- `docs/backlog.md` — product backlog and milestone plan
- `docs/camera-service-api.md` — camera service library API specification
- `docs/zenoh-api.md` — Zenoh wire protocol between service and app
- `docs/testing-cookbook.md` — how to test with real camera service and aravis fake camera
- `docs/adrs/` — architecture decision records

## Related Projects

- **genicam-rs** (`../genicam-rs`) — Pure Rust GenICam/GigE Vision implementation. Contains `genicam-service` binary that bridges real cameras to this app via Zenoh.
- **aravis** (`../aravis`) — C library providing `arv-fake-gv-camera-0.8` for simulated device testing.

Pre-alpha: parsing coverage is expanding; JSON contracts aim for stability. Desktop (Tauri) is the primary runtime; WASM is maintained for offline XML browsing.

## Build Commands

### Rust (workspace root)
```bash
cargo test                                          # run all tests
cargo fmt                                           # format all crates
cargo clippy --all-targets --all-features -- -D warnings  # lint (must be clean)
cargo test -p genicam_xml_model                     # single crate tests
```

### WASM adapter
```bash
./scripts/wasm-build.sh
# Outputs to: ui/genicam-studio-ui/src/wasm/genicam_xml_model_wasm/
```

### UI (browser mode)
```bash
cd ui/genicam-studio-ui
bun install
bun run wasm:build && bun run dev   # build WASM then start Vite dev server (localhost:5173)
bun run build                       # production build
```

### Desktop (Tauri v2)
```bash
cd apps/genicam-studio-tauri
cargo tauri dev    # requires: cargo install tauri-cli --version '^2'
cargo tauri build
```

### Streamer
```bash
cargo run -p genicam-ws-streamer -- \
  --image-key quiss/sensors/svcA/devices/cam0/image \
  --width 640 --height 480
# Options: --bind 127.0.0.1:8081, --path /ws, --fps-limit 30, --zenoh-config <file>
```

## Architecture

### Workspace layout
The root `Cargo.toml` defines workspace members; `apps/genicam-studio-tauri/src-tauri` is excluded (built via `cargo tauri`).

```
crates/
  genicam_xml_model/          # Core streaming XML parser → UiGraph model
  genicam_xml_model_wasm/     # wasm-bindgen wrapper (maintenance mode)
  genicam_zenoh_api/          # Shared Zenoh payload types (no zenoh dep)
  genicam_streamer/           # Zenoh frame → BMP encoder → WebSocket server (library)
apps/
  genicam-studio-tauri/       # Tauri v2 desktop shell (thin glue only)
  genicam-ws-streamer/        # Standalone streamer CLI (thin main.rs over genicam_streamer)
  genicam-mock-service/       # Mock camera service for development/testing
ui/
  genicam-studio-ui/          # React 19 + TypeScript frontend
tests/
  e2e/                        # E2E tests: fake camera + genicam-service + Zenoh assertions
docs/
  backlog.md                  # Product backlog and milestones
  camera-service-api.md       # Camera service Rust library API spec
  zenoh-api.md                # Zenoh key-expression wire protocol
  testing-cookbook.md          # How to test with real service and aravis fake camera
  adrs/                       # Architecture Decision Records
```

### Data flow

```
GenICam XML
  └→ genicam_xml_model (quick-xml streaming parser)
       └→ UiGraph JSON contract (serde)
            ├→ Browser: WASM module → WebWasmProvider → React
            └→ Desktop: Tauri IPC → TauriProvider → React (same UI)
```

The UI never parses XML — it only consumes the `UiGraph` JSON contract. Providers (`WebWasmProvider`, `TauriProvider` in `ui/genicam-studio-ui/src/xml_model/provider.ts`) abstract the backend so the React tree is identical in both modes. Platform detection uses `window.__TAURI__`.

### Image streaming

```
genicam-service (external)
  └→ Zenoh pub: genicam/devices/{id}/image  (16-byte FrameHeader + raw pixels)
       └→ genicam_streamer (embedded tokio tasks in Tauri)
            ├→ FrameHeader decode → BMP encode (Mono8/10/12/16, Bayer, RGB)
            └→ WebSocket broadcast (ws://127.0.0.1:<auto-port>/ws)
                 └→ React ViewerCanvas (createImageBitmap → canvas)
```

The streamer is embedded in the Tauri backend via `genicam_streamer::run_with_session()`, sharing the app's Zenoh session. It auto-assigns a WS port and returns the URL to the frontend. The standalone `genicam-ws-streamer` binary is kept for debugging.

### Recording

Raw frame recording to `.gsr` files (JSON header + raw frames + index) via `genicam_streamer::recording`. Tauri commands: `start_recording`, `stop_recording`, `get_recording_status`. UI shows record button with real-time frame count.

### UiGraph contract

The central type bridging Rust and TypeScript (kept in sync between `crates/genicam_xml_model/src/model.rs` and `ui/genicam-studio-ui/src/xml_model/uigraph.ts`):

- `nodes_by_name: Record<string, UiNode>` — all features indexed by name
- `categories: Record<string, UiCategory>` — category hierarchy
- `root_category: string` — entry point (usually `"Root"`)

Every `UiNode` has a `kind` (Category | Integer | Float | Boolean | String | Enumeration | Command | Register | Unknown) and a `raw: RawNode` snapshot of the original XML tag, attributes, and text children.

### Key invariants (from AGENTS.md)
- **Never drop unknown nodes** — unrecognized XML becomes `UiNodeKind::Unknown { tag }` with `RawNode` preserved.
- **No parsing in the UI** — all XML logic stays in `crates/`.
- **No logic in app shells** — `apps/` is thin glue (windowing, IPC wiring).
- **No `unwrap()`/`expect()` outside tests** — return `Result` with context.
- **JSON contract is stable** — breaking changes require explicit request.

## Fixtures & Tests

Fixtures live in `crates/genicam_xml_model/fixtures/`. Integration tests in `crates/genicam_xml_model/tests/` compare parser output against `expected_*.json` snapshots. Update snapshots intentionally when parser behavior changes. Every fixture must include at least one unknown node to keep the preservation invariant covered.

### E2E tests

E2E tests live in `tests/e2e/` and require `arv-fake-gv-camera-0.8` on PATH and a built `genicam-service` binary.

```bash
# Build service first (excluded from genicam-rs workspace)
cd ../genicam-rs/crates/genicam-service && cargo build

# Run E2E tests (discovery, node read/write, acquisition, device lost)
GENICAM_SERVICE_PATH=../genicam-rs/crates/genicam-service/target/debug/genicam-service \
  cargo test -p e2e-tests -- --ignored --test-threads=1

# Streamer integration test (no external binaries needed)
cargo test -p e2e-tests --test streamer_e2e
```

## CI Pipeline

1. `cargo fmt --check`
2. `cargo clippy --all-targets --all-features -- -D warnings`
3. `cargo test`
4. `./scripts/wasm-build.sh`
5. `cd ui/genicam-studio-ui && bun install && bun run build` (UI)
6. E2E tests (main branch only): build aravis + genicam-service, run `--ignored` tests
7. Tauri build verification (Linux + macOS matrix)
