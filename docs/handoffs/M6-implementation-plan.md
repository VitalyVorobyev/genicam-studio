# M6–M10 Implementation Plan

## Overview

This document lays out the implementation order, dependencies, and key technical decisions for milestones M6 through M10. Each section covers one milestone with task sequencing, file-level changes, and verification steps.

---

## M6: E2E Testing & Production Hardening

**Duration estimate:** 4–5 weeks
**Parallel tracks:** Two independent tracks that can be worked simultaneously.

### Track A: Production Hardening (PH-01, PH-02, PH-05)

These three tasks are independent of E2E infrastructure and can start immediately.

#### PH-01: Structured logging with `tracing`

**Files to modify:**
- `apps/genicam-studio-tauri/src-tauri/Cargo.toml` — add `tracing = "0.1"`, `tracing-subscriber = { version = "0.3", features = ["env-filter"] }`
- `apps/genicam-studio-tauri/src-tauri/src/main.rs` — init subscriber in `main()` before Tauri build (lines 35–80). Replace 2 `eprintln!` calls.
- `apps/genicam-studio-tauri/src-tauri/src/commands/device.rs` — replace 5 `eprintln!` → `tracing::error!`/`warn!`
- `apps/genicam-studio-tauri/src-tauri/src/commands/acquisition.rs` — replace 3 `eprintln!` → `tracing::error!`/`warn!`

**Init pattern:**
```rust
tracing_subscriber::fmt()
    .with_env_filter(
        tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "genicam_studio=info,warn".into()),
    )
    .init();
```

**Verify:** `RUST_LOG=debug cargo tauri dev` shows structured log output. No `eprintln!` remains in `src-tauri/src/`.

#### PH-02: Zenoh query timeout wrappers

**Files to modify:**
- `apps/genicam-studio-tauri/src-tauri/src/commands/device.rs` — `fetch_device_xml()` (line ~406)
- `apps/genicam-studio-tauri/src-tauri/src/commands/nodes.rs` — `write_node()` (~183), `execute_command()` (~217), `read_nodes_bulk()` (~277)
- `apps/genicam-studio-tauri/src-tauri/src/commands/acquisition.rs` — `send_acquisition_command()` (~280)

**Pattern:** Add `.timeout(std::time::Duration::from_secs(5))` to all 5 `session.get()` calls:
```rust
let replies = session
    .get(&key)
    .timeout(Duration::from_secs(5))
    .await
    .map_err(|e| format!("Zenoh GET error: {e}"))?;
```

**Verify:** Disconnect the service, attempt a node write from the UI → should see timeout error within 5s, not hang indefinitely.

#### PH-05: User-friendly error messages

**Files to create:**
- `apps/genicam-studio-tauri/src-tauri/src/error.rs` — `humanize_error(raw: &str) -> String` function with pattern matching on common error substrings

**Files to modify:**
- `apps/genicam-studio-tauri/src-tauri/src/commands/device.rs` — wrap all `Err(format!(...))` returns through `humanize_error`
- `apps/genicam-studio-tauri/src-tauri/src/commands/nodes.rs` — same
- `apps/genicam-studio-tauri/src-tauri/src/commands/acquisition.rs` — same
- `apps/genicam-studio-tauri/src-tauri/src/main.rs` — add `mod error;`

**Error mapping table:**

| Raw pattern | User message |
|-------------|-------------|
| `"No reply"` + `"timeout"` | "Camera service did not respond. Is it running?" |
| `"Zenoh GET error"` | "Communication error with camera service" |
| `"Node '...' not in cache"` | "Feature '{name}' is not available on this camera" |
| `"Access mode"` + `"RO"` | "Feature '{name}' is read-only" |
| `"out of range"` / `"Range"` | "Value is outside the allowed range ({min}–{max})" |
| `"Not connected"` | "No camera connected" |

**Verify:** Trigger each error condition from the UI. No raw Rust/Zenoh error strings visible.

### Track B: E2E Testing (E2E-01 through E2E-07)

#### E2E-01: Test harness

**Where to put tests:** New crate or test binary. Options:
- `tests/e2e/` directory at workspace root with a `Cargo.toml` (test-only crate)
- Or `apps/genicam-mock-service/tests/e2e.rs` extending existing test infra

**Recommended:** New directory `tests/e2e/` as a workspace member with `[lib] test = true` and `[[test]]` targets. This keeps E2E tests separate from unit tests.

**Files to create:**
- `tests/e2e/Cargo.toml` — depends on `genicam_zenoh_api`, `zenoh`, `tokio`, `serde_json`
- `tests/e2e/src/lib.rs` — `TestHarness` struct

**TestHarness design:**
```rust
pub struct TestHarness {
    fake_camera: Child,     // arv-fake-gv-camera-0.8 process
    service: Child,         // genicam-service process
    session: Arc<Session>,  // Zenoh client session for test assertions
}

impl TestHarness {
    pub async fn start(service_path: &str) -> Result<Self, HarnessError> {
        // 1. Start arv-fake-gv-camera-0.8 -i 127.0.0.1
        // 2. Wait for camera ready (poll GVCP discovery on 127.0.0.1:3956)
        // 3. Start genicam-service (with --iface lo0 or loopback)
        // 4. Open Zenoh session
        // 5. Wait for first announce message (with timeout)
        Ok(Self { ... })
    }
    
    pub fn device_id(&self) -> &str { "cam-000000000000" }
    pub fn session(&self) -> &Arc<Session> { &self.session }
}

impl Drop for TestHarness {
    fn drop(&mut self) {
        // Kill service, then fake camera
        let _ = self.service.kill();
        let _ = self.fake_camera.kill();
    }
}
```

**Service binary path:** Read from `GENICAM_SERVICE_PATH` env var (set in CI). Default: `../genicam-rs/target/debug/genicam-service`.

**Fake camera binary:** Read from `ARV_FAKE_CAMERA_PATH` env var. Default: find `arv-fake-gv-camera-0.8` in `$PATH`.

**Verify:** `cargo test -p e2e-tests -- --ignored` spawns both processes, connects via Zenoh, sees announce, and tears down cleanly.

#### E2E-02 through E2E-05: Test cases

All tests use `TestHarness::start()` and follow the pattern from mock-service integration tests:

```rust
#[tokio::test(flavor = "multi_thread")]
#[ignore] // Requires external binaries
async fn test_e2e_discovery() {
    let harness = TestHarness::start(None).await.unwrap();
    let key = format!("genicam/devices/{}/announce", harness.device_id());
    let sub = harness.session().declare_subscriber(&key).await.unwrap();
    
    let sample = timeout(Duration::from_secs(10), sub.recv_async())
        .await.unwrap().unwrap();
    let announce: DeviceAnnounce = serde_json::from_slice(&sample.payload().to_bytes()).unwrap();
    
    assert_eq!(announce.id, harness.device_id());
    assert!(announce.api_version.is_some());
}
```

**E2E-02:** Discovery + connect + XML fetch
**E2E-03:** Node read/write (Width change + readback, invalid value error)
**E2E-04:** Acquisition start → frame reception → stop (timeout-guarded)
**E2E-05:** Kill fake camera process → assert device lost detection

#### E2E-06: CI workflow

**File to modify:** `.github/workflows/ci.yml`

**New job** (separate from existing fast job):
```yaml
e2e-tests:
  runs-on: ubuntu-latest
  needs: check  # Run after lint/test pass
  steps:
    - uses: actions/checkout@v4
    - uses: actions/checkout@v4
      with:
        repository: <owner>/genicam-rs
        path: genicam-rs
        ref: phase2_dev
    
    # Install aravis
    - name: Install aravis
      run: |
        sudo apt-get update
        sudo apt-get install -y meson ninja-build libxml2-dev libglib2.0-dev
        git clone --depth 1 --branch ARAVIS_0_8_33 https://github.com/AravisProject/aravis.git /tmp/aravis
        cd /tmp/aravis
        meson setup build -Dviewer=disabled -Dgst-plugin=disabled -Dusb=disabled
        ninja -C build
        sudo ninja -C build install
        sudo ldconfig
    
    # Build genicam-service
    - name: Build genicam-service
      run: cargo build -p genicam-service --manifest-path genicam-rs/Cargo.toml
    
    # Run E2E tests
    - name: Run E2E tests
      env:
        GENICAM_SERVICE_PATH: genicam-rs/target/debug/genicam-service
      run: cargo test -p e2e-tests -- --ignored --test-threads=1
```

**Note on aravis in CI:** aravis is not in standard Ubuntu repos. We build from source using meson (~2 min). The fake camera binary (`arv-fake-gv-camera-0.8`) is built as part of aravis. Alternative: cache the aravis build directory between runs.

**Note on loopback streaming:** aravis fake camera streaming may not work over loopback on Linux (UDP multicast). E2E-04 (frame reception) may need a workaround or be marked `#[ignore]` on CI with a comment. Discovery and node operations work fine on loopback.

#### E2E-07: Tauri build verification

**File to modify:** `.github/workflows/ci.yml`

**New job:**
```yaml
tauri-build:
  runs-on: ${{ matrix.os }}
  strategy:
    matrix:
      os: [ubuntu-latest, macos-latest]
  steps:
    - uses: actions/checkout@v4
    - name: Install system deps (Linux)
      if: runner.os == 'Linux'
      run: sudo apt-get install -y libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev
    - name: Build Tauri
      run: |
        cargo install tauri-cli --version '^2'
        cd apps/genicam-studio-tauri
        cargo tauri build
```

### M6 Task Order

```
Week 1-2: PH-01, PH-02, PH-05 (parallel, independent)
Week 2-3: E2E-01 (test harness)
Week 3-4: E2E-02, E2E-03, E2E-04, E2E-05 (parallel, all depend on E2E-01)
Week 4-5: E2E-06, E2E-07 (CI integration)
```

---

## M7: Embed Streamer & Hardening Phase 2

**Duration estimate:** 4–5 weeks
**Depends on:** M6 complete (logging, timeouts in place)

### Embed Streamer Track (ES-01 through ES-04)

**Sequencing:** ES-01 → ES-02 + ES-03 (parallel) → ES-04

#### ES-01: Extract streamer into library crate

**Files to create:**
- `crates/genicam_streamer/Cargo.toml`
- `crates/genicam_streamer/src/lib.rs` — re-exports

**Files to move:**
- `apps/genicam-ws-streamer/src/bmp.rs` → `crates/genicam_streamer/src/bmp.rs`
- `apps/genicam-ws-streamer/src/ws.rs` → `crates/genicam_streamer/src/ws.rs`
- `apps/genicam-ws-streamer/src/zenoh_source.rs` → `crates/genicam_streamer/src/zenoh_source.rs`
- `apps/genicam-ws-streamer/src/meta.rs` → `crates/genicam_streamer/src/meta.rs`
- `apps/genicam-ws-streamer/src/error.rs` → `crates/genicam_streamer/src/error.rs`

**Files to modify:**
- `Cargo.toml` (workspace) — add `crates/genicam_streamer` to members
- `apps/genicam-ws-streamer/Cargo.toml` — depend on `genicam_streamer`
- `apps/genicam-ws-streamer/src/main.rs` — thin wrapper calling `genicam_streamer::run()`

**Key design:** `zenoh_source::run()` gains a second constructor:
```rust
// Existing: creates own session
pub async fn run(config: zenoh::Config, ...) -> Result<(), StreamerError>

// New: accepts pre-opened session
pub async fn run_with_session(session: Arc<zenoh::Session>, ...) -> Result<(), StreamerError>
```

**Verify:** `cargo run -p genicam-ws-streamer` still works exactly as before. `cargo test -p genicam_streamer` passes.

#### ES-02: Embed streamer tasks in Tauri backend

**Files to modify:**
- `apps/genicam-studio-tauri/src-tauri/Cargo.toml` — add `genicam_streamer`, `axum`, `bytes`
- `apps/genicam-studio-tauri/src-tauri/src/commands/acquisition.rs` — major rewrite

**Replace subprocess management with:**
```rust
pub async fn start_acquisition(
    zenoh: &ZenohState,
    app: &AppHandle,
    device_id: String,
) -> Result<StreamerInfo, String> {
    let session = zenoh.get_session().await?;
    let image_key = keys::image(&device_id);
    
    // Create channels
    let (frame_tx, _) = watch::channel(Bytes::new());
    let (info_tx, _) = watch::channel(StreamInfo::default());
    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    
    // Bind WS server to :0 (auto-assign port)
    let listener = TcpListener::bind("127.0.0.1:0").await.map_err(|e| ...)?;
    let port = listener.local_addr().map_err(|e| ...)?.port();
    
    // Spawn Zenoh source task
    let zenoh_handle = tokio::spawn(genicam_streamer::zenoh_source::run_with_session(
        session.clone(),
        ZenohSourceConfig { image_key, ... },
        shared_meta.clone(),
        frame_tx.clone(),
        info_tx.clone(),
        shutdown_rx.clone(),
    ));
    
    // Spawn WS server task
    let ws_handle = tokio::spawn(genicam_streamer::ws::run_server_with_listener(
        listener,
        "/ws".to_string(),
        AppState { frame_tx, info_tx },
        shutdown_rx,
    ));
    
    // Store handles for cleanup
    let mut acq = zenoh.acquisition.lock().await;
    acq.shutdown_tx = Some(shutdown_tx);
    acq.task_handles = vec![zenoh_handle, ws_handle];
    acq.ws_url = Some(format!("ws://127.0.0.1:{port}/ws"));
    
    // Send acquisition start command to service
    send_acquisition_command(&session, &device_id, AcquisitionCommand::Start).await?;
    
    Ok(StreamerInfo { ws_url: acq.ws_url.clone().unwrap(), port })
}
```

**Verify:** Start acquisition from UI → image appears in viewer → stop acquisition → WS connection closes cleanly.

#### ES-03: Share Zenoh session

Handled as part of ES-02 — `run_with_session()` accepts the existing `Arc<Session>` from `ZenohState`.

#### ES-04: Remove process management code

**Files to modify:**
- `apps/genicam-studio-tauri/src-tauri/src/commands/acquisition.rs` — delete `spawn_streamer_child`, `run_streamer_monitor`, `build_streamer_args`, `resolve_streamer_path`, `StreamerArgs`, `MAX_RESTARTS`, `RESTART_DELAY_MS`
- `apps/genicam-studio-tauri/src-tauri/src/state/device_state.rs` — simplify `AcquisitionInner`: replace `stop_tx: Option<watch::Sender<bool>>` + `monitor_handle` with `shutdown_tx` + `task_handles: Vec<JoinHandle<()>>`
- `ui/genicam-studio-ui/src/` — remove `streamer-status` event listener if no longer needed (the embedded tasks are managed internally)

**Verify:** No references to `tokio::process` remain. `cargo clippy` clean.

### Hardening Phase 2 (PH-03, PH-04, PH-06)

#### PH-03: Zenoh session health monitor

**Files to modify:**
- `apps/genicam-studio-tauri/src-tauri/src/commands/device.rs` — new `spawn_session_monitor()` function
- `apps/genicam-studio-tauri/src-tauri/src/state/device_state.rs` — add session monitor handle

**Design:** Periodic liveliness check (self-put + self-subscribe). If session is dead, emit event + attempt reopen.

#### PH-04: Device reconnection

**Files to modify:**
- `apps/genicam-studio-tauri/src-tauri/src/state/device_state.rs` — add `ConnectionState::Reconnecting { device_id, attempt, max_attempts }`
- `apps/genicam-studio-tauri/src-tauri/src/commands/device.rs` — reconnection logic in status subscriber
- `ui/genicam-studio-ui/src/device/useDevice.ts` — handle `Reconnecting` state in UI

#### PH-06: Service crash recovery

Already partially implemented via `do_emergency_disconnect`. Enhance with:
- Distinguish "device lost" (camera disappears) from "service lost" (announce stops for all devices)
- Different UI states and recovery options for each

### M7 Task Order

```
Week 1:   ES-01 (extract library crate)
Week 2-3: ES-02 + ES-03 (embed in Tauri) | PH-03 (session monitor)
Week 3-4: ES-04 (cleanup) | PH-04 (reconnection)
Week 4-5: PH-06 (crash recovery) + integration testing
```

---

## M8: Shared XML Crate

**Duration estimate:** 5–7 weeks (cross-repo coordination)
**Depends on:** Coordination with genicam-rs. See separate handoff: `docs/handoffs/SX-genicam-rs-handoff.md`

### Studio-side work (SX-03, SX-04, SX-05)

Blocked on SX-01 + SX-02 completing in genicam-rs.

#### SX-03: Integrate shared crate

**Files to modify:**
- `crates/genicam_xml_model/Cargo.toml` — add `genapi-xml = { path = "../../genicam-rs/crates/genapi-xml" }`, `genapi-core` similarly
- `crates/genicam_xml_model/src/lib.rs` — new `parse_full()` function using genapi-xml, alongside existing `parse_genicam_xml()`
- `crates/genicam_xml_model/src/model.rs` — extend `UiNode` with optional resolved fields (resolved_min, resolved_max, dependencies)

**Backward compatibility:** Existing `UiGraph` fields unchanged. New fields are `Option<T>` — old consumers unaffected.

#### SX-04: Offline browsing

**Files to modify:**
- `crates/genicam_xml_model_wasm/src/lib.rs` — use `parse_full()` for richer UiGraph in WASM mode
- UI components to render dependency information when available

### M8 Task Order

```
Week 1-2: SX-01 (API design, ADR) — cross-repo
Week 2-5: SX-02 (extract crates) — genicam-rs work
Week 4-6: SX-03 (integrate into studio) — starts when SX-02 stabilizes
Week 6-7: SX-04 (offline browsing) + SX-05 (deprecate old epic)
```

---

## M9: Recording & Polish

**Duration estimate:** 5–6 weeks

### Recording Track (REC-01, REC-02)

**Files to create:**
- `crates/genicam_streamer/src/recording.rs` — recording engine
- New IPC commands in `acquisition.rs`

**Recording format (.gsr):**
```
[JSON header (variable length, null-terminated)]
[Frame 0: FrameHeader (16 bytes) + pixel data]
[Frame 1: FrameHeader (16 bytes) + pixel data]
...
[Frame index (JSON, appended on stop)]
```

### Polish Track (FB-03, FB-04, ES-05, ES-06, PH-07, PH-08)

All independent tasks, can be parallelized.

---

## M10: Release

**Duration estimate:** 4–5 weeks

RP-01 through RP-04, REC-03, REC-04. See backlog for details.

---

## Cross-Cutting Concerns

### Testing strategy

| Level | What | When | CI? |
|-------|------|------|-----|
| Unit tests | Individual functions | Every PR | Yes (fast job) |
| Mock integration | Studio ↔ mock-service | Every PR | Yes (fast job) |
| E2E with fake camera | Studio ↔ genicam-service ↔ aravis | Nightly / manual | Yes (slow job) |
| Manual with real camera | Full pipeline | Before release | No |

### Dependency graph across milestones

```
M6 (E2E + hardening basics)
 ├─→ M7 (embed streamer + hardening phase 2)
 │    └─→ M9 (recording uses embedded streamer)
 │         └─→ M10 (release packaging)
 └─→ M8 (shared XML crate — independent of M7)
      └─→ M9 (FB-03 depends on SX-03)
```

M7 and M8 can run in parallel after M6.
