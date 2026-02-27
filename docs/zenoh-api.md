# GenICam Studio — Zenoh API Contract

This document defines the Zenoh key schema, payload types, and interaction semantics
between the external camera service and GenICam Studio (Tauri shell).

## Key Naming Convention

All keys are prefixed with `genicam/devices/{device_id}/` where `device_id` is a short
stable identifier for the camera device (e.g., `cam0`, a serial-number slug, or any
opaque string without `/`).

---

## Discovery

### `genicam/devices/{device_id}/announce`

- **Direction:** Camera service → App
- **Mechanism:** `put` (publisher), periodic, e.g. every 2 seconds
- **Payload (JSON):**
  ```json
  {
    "id": "cam0",
    "name": "Sony IMX421 GigE Camera",
    "model": "SPC-3000",
    "serial": "SN12345678"
  }
  ```
- **Semantics:** The app subscribes to `genicam/devices/*/announce`. Any device not seen
  for more than 6 seconds is considered lost and removed from the discovered list.

---

## Connection Lifecycle

### `genicam/devices/{device_id}/xml`

- **Direction:** App → Service (queryable GET)
- **Mechanism:** Zenoh queryable; app issues `get()` with empty payload
- **Response (JSON):**
  ```json
  { "xml": "<RegisterDescription>...</RegisterDescription>" }
  ```
- **Semantics:** Full GenICam XML string. The app parses this into the UiGraph model on
  connect. Response should be stable during a session.

### `genicam/devices/{device_id}/status`

- **Direction:** Service → App
- **Mechanism:** `put` on change
- **Payload (JSON):**
  ```json
  { "connected": true, "error": null }
  { "connected": false, "error": "USB link lost" }
  ```
- **Semantics:** App subscribes on connect. Loss of connection triggers
  `connection-state-changed` Tauri event.

---

## Node Values

### `genicam/devices/{device_id}/nodes/{node_name}/value`

- **Direction:** Service → App
- **Mechanism:** `put` on change (service-owned)
- **Payload (JSON):**
  ```json
  { "value": 1024, "access_mode": "RW" }
  { "value": "Continuous", "access_mode": "RW" }
  { "value": true, "access_mode": "RO" }
  ```
- **Semantics:** The app maintains a local `NodeValueCache` updated by these messages.
  `access_mode` is one of `RO`, `WO`, `RW`, `NA`.

### `genicam/devices/{device_id}/nodes/{node_name}/set`

- **Direction:** App → Service (queryable GET)
- **Mechanism:** App issues `get()` with JSON payload
- **Request (JSON):**
  ```json
  { "value": 2048 }
  ```
- **Response (JSON):**
  ```json
  { "ok": true, "error": null }
  { "ok": false, "error": "Value out of range" }
  ```
- **Semantics:** Synchronous write. On success the service publishes the new value to
  `nodes/{name}/value`.

### `genicam/devices/{device_id}/nodes/{node_name}/execute`

- **Direction:** App → Service (queryable GET)
- **Mechanism:** App issues `get()` with empty payload
- **Response (JSON):**
  ```json
  { "ok": true, "error": null }
  ```
- **Semantics:** Executes a GenICam Command node.

---

## Acquisition

### `genicam/devices/{device_id}/acquisition/control`

- **Direction:** App → Service (queryable GET)
- **Mechanism:** App issues `get()` with JSON payload
- **Request (JSON):**
  ```json
  { "command": "start" }
  { "command": "stop" }
  ```
- **Response (JSON):**
  ```json
  { "ok": true, "error": null }
  ```
- **Semantics:** Start/stop hardware acquisition. On success the service begins/stops
  publishing to `image`.

### `genicam/devices/{device_id}/acquisition/status`

- **Direction:** Service → App
- **Mechanism:** `put` on change
- **Payload (JSON):**
  ```json
  { "active": true, "fps": 29.97, "dropped": 0 }
  { "active": false, "fps": null, "dropped": 0 }
  ```

### `genicam/devices/{device_id}/image`

- **Direction:** Service → Streamer (NOT consumed by Tauri directly)
- **Mechanism:** `put` per frame
- **Payload:** Raw binary Mono8 image (`width × height` bytes, row-major)
- **Consumer:** `genicam-ws-streamer` subscribes to this key and broadcasts BMP-encoded
  frames over WebSocket. The Tauri app does **not** subscribe to this key directly.

---

## Timing Guarantees

| Key | Expected frequency |
|-----|--------------------|
| `announce` | Every 2 s (informational) |
| `nodes/*/value` | On parameter change (up to ~100 Hz for fast nodes) |
| `acquisition/status` | On change |
| `image` | At acquisition frame rate |

Device is considered lost if `announce` is not received for 6 seconds.

---

## Error Semantics

- All Zenoh `get()` calls to service queryables expect a single reply within 5 seconds.
- Timeout or Zenoh-level error → surfaced as `"Zenoh timeout or error"`.
- Service-level error → returned in `{ "ok": false, "error": "..." }` response body.
- The app never puts to `image` or `nodes/*/value` — those are service-owned.

---

## Tauri IPC Commands (App Internal)

| Command | Direction | Description |
|---------|-----------|-------------|
| `list_discovered_devices()` | → `Vec<DeviceInfo>` | Current discovered device list |
| `connect_device(device_id)` | → `ParseXmlResponse` | Connect and fetch XML |
| `disconnect_device()` | → `()` | Clean up subscriptions |
| `get_connection_state()` | → `ConnectionState` | Current state enum |
| `write_node(node_name, value)` | → `()` | Write node via Zenoh queryable |
| `execute_command(node_name)` | → `()` | Execute Command node |
| `get_node_value(node_name)` | → `NodeValueEntry` | Read from local cache |
| `start_acquisition()` | → `StreamerInfo` | Start acq + launch streamer sidecar |
| `stop_acquisition()` | → `()` | Stop acq + kill streamer |
| `get_acquisition_status()` | → `AcquisitionStatus` | Current acquisition state |

## Tauri Events (Backend → Frontend)

| Event | Payload | Description |
|-------|---------|-------------|
| `device-discovered` | `DeviceInfo` | New device seen on Zenoh |
| `device-lost` | `{ device_id: string }` | Device announce timed out |
| `node-value-changed` | `{ node_name, value, access_mode }` | Live node update |
| `acquisition-status` | `AcquisitionStatus` | Acquisition state change |
| `connection-state-changed` | `ConnectionState` | Connect/disconnect/error |
