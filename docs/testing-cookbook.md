# Testing Cookbook: Real Camera Service Integration

This document describes how to test GenICam Studio with the **real camera service** (`genicam-service`) from the `genicam-rs` repository, using `arv-fake-gv-camera` as a simulated GigE Vision device.

## Prerequisites

| Component | Location | Install |
|-----------|----------|---------|
| genicam-rs workspace | `../genicam-rs` | `git clone` (branch `phase2_dev`) |
| aravis (fake camera) | system / `../aravis` | `brew install aravis` |
| genicam-studio | this repo | — |

Verify aravis is installed:
```bash
which arv-fake-gv-camera-0.8  # should print a path
```

## Architecture Overview

```
┌──────────────────┐     Zenoh      ┌──────────────────┐     GVCP/GVSP    ┌──────────────────┐
│  GenICam Studio   │ ◄──────────► │  genicam-service  │ ◄──────────────► │ arv-fake-gv-cam  │
│  (Tauri app)      │   pub/sub    │  (Rust binary)    │    UDP/GigE      │ (aravis sim)     │
└──────────────────┘   queryable   └──────────────────┘                   └──────────────────┘
```

The **mock service** (`apps/genicam-mock-service`) can be swapped 1:1 with `genicam-service` — they implement the same Zenoh API contract (`docs/zenoh-api.md`).

## Quick Start: Mock Service (no camera needed)

This is the fastest way to develop and test the UI:

```bash
# Terminal 1: start mock service
cargo run -p genicam-mock-service

# Terminal 2: start studio
cd apps/genicam-studio-tauri
cargo tauri dev
```

The mock service generates synthetic test patterns (Mono8 gradient, RGB8 color bars) — no real camera required.

## Quick Start: Real Service + Fake Camera

```bash
# Terminal 1: start the fake GigE camera
# On macOS — use loopback (works) or a real NIC:
arv-fake-gv-camera-0.8 -i 127.0.0.1

# Terminal 2: start the real camera service
cd ../genicam-rs
cargo run -p genicam-service
# Or with explicit interface: cargo run -p genicam-service -- --iface en0

# Terminal 3: start studio
cd apps/genicam-studio-tauri
cargo tauri dev
```

## What to Expect

### Discovery
- Within 5 seconds, studio should show the fake camera in the device list
- Device ID: `cam-000000000000` (fake camera uses all-zero MAC)
- Model: "Fake" / Manufacturer: "Aravis"

### Connection
- Click to connect — studio fetches GenICam XML and renders the feature tree
- Available features: Width, Height, PixelFormat, ExposureTimeAbs, Gain, AcquisitionMode, TriggerMode, etc.
- Width/Height default: 512x512 (aravis fake camera defaults)

### Feature Read/Write
- Change Width, Height — readback should reflect new values
- Change PixelFormat (Mono8, RGB8, BayerRG8, Mono16) — affects streaming pixel data
- ExposureTimeAbs — adjustable float value

### Streaming (acquisition)
- Start acquisition — studio should receive frames via the WebSocket streamer
- Frame dimensions match the camera's Width/Height
- PixelFormat changes are reflected in the stream
- FPS is published via `acquisition/status` (~30 fps from fake camera)
- Stop acquisition — frames stop arriving

## Zenoh Key Reference

All keys prefixed with `genicam/devices/{device_id}/`:

| Key suffix | Direction | Description |
|------------|-----------|-------------|
| `announce` | Service → App | Periodic device announcement (every 2s) |
| `xml` | App → Service (query) | Full GenICam XML |
| `status` | Service → App | Connection status |
| `nodes/{name}/value` | Service → App | Node value updates (published on connect and after writes) |
| `nodes/{name}/set` | App → Service (query) | Write node value |
| `nodes/{name}/execute` | App → Service (query) | Execute command node |
| `nodes/bulk/read` | App → Service (query) | Batch read node values |
| `acquisition/control` | App → Service (query) | Start/stop acquisition |
| `acquisition/status` | Service → App | Acquisition state with FPS |
| `image` | Service → Streamer | Binary: 16-byte FrameHeader + raw pixels |
| `image/meta` | Service → App | JSON: pixel_format, width, height, payload_size |

Full specification: `docs/zenoh-api.md`

## Differences: Mock Service vs Real Service

| Aspect | Mock Service | Real Service (genicam-service) |
|--------|-------------|-------------------------------|
| Camera source | Synthetic patterns | Real GigE Vision device (or aravis fake) |
| Node values | Simulated in-memory | Read from actual camera registers |
| Frame data | Generated gradients/bars | Real GVSP reassembled frames |
| Node interdependencies | Simulated (MS-12) | Driven by actual camera register dependencies |
| Pixel formats | Mono8, Mono16, BayerRG8, RGB8 | Whatever the camera supports |
| Error behavior | Always succeeds | May return transport errors |
| XML source | Built-in fixture | Fetched from camera via GVCP |
| Initial node values | Published from config | Published for common SFNC features on connect |
| Device lost | Never | Detected when device disappears from discovery |

**From the app's perspective, both services are interchangeable.** The Zenoh API contract is identical.

## genicam-service Capabilities (Apr 2026)

The real service (`../genicam-rs/crates/genicam-service`) supports:

- **Discovery**: periodic GVCP broadcast, loopback support, multi-camera dedup
- **CCP**: claims Control Channel Privilege on connect (required for streaming)
- **XML**: serves raw GenICam XML fetched from the camera
- **Nodes**: read/write/execute/bulk-read via Zenoh queryables; initial SFNC values published on connect
- **Acquisition**: start/stop via AcquisitionStart/Stop commands; frame streaming with FrameHeader encoding
- **FPS tracking**: measured FPS published in AcquisitionStatus every second
- **Device lost**: detects when camera disappears from discovery, cleans up tasks
- **XML parsing**: full pValue delegation, IntReg, IntSwissKnife (hex literals), StructReg, Converter

## genicam-rs Integration Tests

All 12 integration tests pass against `arv-fake-gv-camera` on macOS (including loopback streaming):

```bash
cd ../genicam-rs
cargo test -p genicam --test fake_camera -- --ignored --test-threads=1
```

Tests cover: discovery, connection, XML fetch, feature read/write, command execution, frame streaming, frame dimension validation, full lifecycle.

## Troubleshooting

### No device discovered
- Check that the fake camera is running: `arv-fake-gv-camera-0.8 -i 127.0.0.1`
- If using `--iface`, ensure it matches the camera's subnet
- Verify with: `arv-tool-0.8 -a 127.0.0.1` (should show the device)
- Ensure no firewall blocks UDP broadcast on port 3956

### Connection fails (XML parse error)
- Check `genicam-service` logs for specific parsing errors
- The service supports all node types used by the aravis fake camera XML

### No frames during acquisition
- Service needs CCP (Control Channel Privilege) — this is handled automatically
- Check service logs for "stream build failed" or "frame stream error"
- Verify the streamer (`genicam-ws-streamer`) is running if testing through the studio UI

### Node write returns error
- Some nodes are read-only (SensorWidth, SensorHeight)
- Value out of range — check Min/Max constraints
- Node not found — may be skipped during XML parsing (check logs)
