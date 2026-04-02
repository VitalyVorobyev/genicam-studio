# Testing Cookbook: Real Camera Service Integration

This document describes how to test GenICam Studio with the **real camera service** (`genicam-service`) from the `genicam-rs` repository, using `arv-fake-gv-camera` as a simulated GigE Vision device.

## Prerequisites

| Component | Location | Install |
|-----------|----------|---------|
| genicam-rs workspace | `../genicam-rs` | `git clone` |
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
# Terminal 1: start the fake GigE camera on the local network interface
arv-fake-gv-camera-0.8 -i $(ipconfig getifaddr en0)
# Or on a specific interface:
# arv-fake-gv-camera-0.8 -i 192.168.1.100

# Terminal 2: start the real camera service
cd ../genicam-rs
cargo run -p genicam-service -- --iface en0

# Terminal 3: start studio
cd apps/genicam-studio-tauri
cargo tauri dev
```

**Important:** The fake camera must bind to a **real network interface** (not 127.0.0.1) because GVSP streaming does not work reliably on macOS loopback.

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
- Stop acquisition — frames stop arriving

## Zenoh Key Reference

All keys prefixed with `genicam/devices/{device_id}/`:

| Key suffix | Direction | Description |
|------------|-----------|-------------|
| `announce` | Service → App | Periodic device announcement (every 2s) |
| `xml` | App → Service (query) | Full GenICam XML |
| `status` | Service → App | Connection status |
| `nodes/{name}/value` | Service → App | Node value updates |
| `nodes/{name}/set` | App → Service (query) | Write node value |
| `nodes/{name}/execute` | App → Service (query) | Execute command node |
| `nodes/bulk/read` | App → Service (query) | Batch read node values |
| `acquisition/control` | App → Service (query) | Start/stop acquisition |
| `acquisition/status` | Service → App | Acquisition state |
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

**From the app's perspective, both services are interchangeable.** The Zenoh API contract is identical.

## Troubleshooting

### No device discovered
- Check that the fake camera is bound to the same subnet as the service's `--iface`
- Verify with: `arv-tool-0.8` (should list the device)
- Ensure no firewall blocks UDP broadcast on port 3956

### Connection fails (XML parse error)
- The fake camera's XML uses `<pValue>` delegation and `IntReg` nodes — genicam-rs supports these as of Apr 2026
- Check `genicam-service` logs for specific XML parsing errors
- Known limitation: `IntSwissKnife` hex literals (`0xFF`) not yet supported in expression parser (the `PayloadSize` node is skipped)

### No frames during acquisition
- GVSP streaming does not work on macOS loopback (127.0.0.1)
- Use a real NIC: `arv-fake-gv-camera-0.8 -i $(ipconfig getifaddr en0)`
- Stream channel register offsets must match (bootstrap registers at 0x0d00)
- Check service logs for "stream build failed" errors

### Node write returns error
- Some nodes are read-only (SensorWidth, SensorHeight)
- Value out of range — check Min/Max constraints
- Node not found — may be skipped during XML parsing (check logs)

## Running genicam-rs Integration Tests

These tests validate the genicam-rs library directly against the fake camera (no Zenoh involved):

```bash
cd ../genicam-rs
cargo test -p genicam --test fake_camera -- --ignored --test-threads=1
```

**Expected results:** 8/11 pass. 3 streaming tests timeout on macOS loopback (known limitation).
