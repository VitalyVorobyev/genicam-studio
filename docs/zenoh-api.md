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
- **Rust type:** `DeviceAnnounce` in `genicam_zenoh_api`
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
- **Rust type:** `DeviceXmlResponse` in `genicam_zenoh_api`
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
- **Rust type:** `DeviceStatus` in `genicam_zenoh_api`
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
  { "value": 640, "access_mode": "RW", "min": 1.0, "max": 4096.0, "inc": 1.0 }
  ```
- **Rust type:** `NodeValueUpdate` in `genicam_zenoh_api`
- **Semantics:** The app maintains a local `NodeValueCache` updated by these messages.
  `access_mode` is one of `RO`, `WO`, `RW`, `NA`.
  `min`, `max`, `inc` are **optional** runtime constraint hints (ZA-06). When present,
  the UI can tighten slider ranges without re-parsing XML. Services that do not implement
  constraint propagation may omit them; the UI falls back to XML-parsed static constraints.

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
- **Rust type:** request: `NodeSetRequest`; response: `NodeOpResponse` — both in `genicam_zenoh_api`
- **Semantics:** Synchronous write. On success the service publishes the new value to
  `nodes/{name}/value`.

### `genicam/devices/{device_id}/nodes/{node_name}/execute`

- **Direction:** App → Service (queryable GET)
- **Mechanism:** App issues `get()` with empty payload
- **Response (JSON):**
  ```json
  { "ok": true, "error": null }
  ```
- **Rust type:** `NodeOpResponse` in `genicam_zenoh_api`
- **Semantics:** Executes a GenICam Command node.

### `genicam/devices/{device_id}/nodes/bulk/read`

- **Direction:** App → Service (queryable GET)
- **Request (JSON):** `{ "names": ["ExposureTime", "Gain", "Width"] }`
- **Response (JSON):**
  ```json
  {
    "values": {
      "ExposureTime": { "value": 10000.0, "access_mode": "RW" },
      "Gain":         { "value": 1.0,     "access_mode": "RW" }
    }
  }
  ```
- **Rust type:** request: `BulkReadRequest`; response: `BulkReadResponse` — both in `genicam_zenoh_api`
- **Semantics:** Batch read of multiple node values in a single round-trip. Unknown node names are silently omitted. An empty `names` list returns an empty `values` map. The per-entry shape is identical to `nodes/{name}/value`.

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
- **Rust type:** request: `AcquisitionControlRequest { command: AcquisitionCommand }`; response: `NodeOpResponse` — both in `genicam_zenoh_api`. `AcquisitionCommand` serializes as `"start"` or `"stop"` (`#[serde(rename_all = "lowercase")]`).
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
- **Rust type:** `AcquisitionStatus` in `genicam_zenoh_api`

### `genicam/devices/{device_id}/image`

- **Direction:** Service → Streamer (NOT consumed by Tauri directly)
- **Mechanism:** `put` per frame
- **Payload:** 16-byte binary [`FrameHeader`] immediately followed by raw pixel data (row-major,
  no further encoding). The header makes every frame self-describing — consumers do not need to
  rely on a prior `image/meta` subscription.
- **Consumer:** `genicam-ws-streamer` subscribes to this key and broadcasts BMP-encoded
  frames over WebSocket. The Tauri app does **not** subscribe to this key directly.

#### Binary frame header layout (16 bytes, all fields little-endian)

| Offset | Size | Field    | Description                                              |
|--------|------|----------|----------------------------------------------------------|
| 0      | 2    | `magic`  | `0x4746` LE (`[0x46, 0x47]`) — frame marker             |
| 2      | 1    | `version`| Header layout version; currently `1`                    |
| 3      | 1    | `format` | Pixel format discriminant (see table below)              |
| 4      | 4    | `width`  | Image width in pixels, `u32` LE                          |
| 8      | 4    | `height` | Image height in pixels, `u32` LE                         |
| 12     | 4    | `seq`    | Monotonically increasing frame counter, `u32` LE         |

#### Pixel format discriminant table

Codes are **only appended** — never reordered. Unknown codes map to `PixelFormat::Unknown`.

| Code | `PixelFormat` variant | Notes               |
|------|----------------------|---------------------|
|    0 | `Unknown`            | fallback / unset    |
|    1 | `Mono8`              | 1 byte/px           |
|    2 | `Mono10`             | 2 bytes/px (LE)     |
|    3 | `Mono12`             | 2 bytes/px (LE)     |
|    4 | `Mono16`             | 2 bytes/px (LE)     |
|    5 | `BayerRG8`           | 1 byte/px           |
|    6 | `BayerGR8`           | 1 byte/px           |
|    7 | `BayerBG8`           | 1 byte/px           |
|    8 | `BayerGB8`           | 1 byte/px           |
|    9 | `BayerRG10`          | 2 bytes/px (LE)     |
|   10 | `BayerGR10`          | 2 bytes/px (LE)     |
|   11 | `BayerBG10`          | 2 bytes/px (LE)     |
|   12 | `BayerGB10`          | 2 bytes/px (LE)     |
|   13 | `BayerRG12`          | 2 bytes/px (LE)     |
|   14 | `BayerGR12`          | 2 bytes/px (LE)     |
|   15 | `BayerBG12`          | 2 bytes/px (LE)     |
|   16 | `BayerGB12`          | 2 bytes/px (LE)     |
|   17 | `BayerRG16`          | 2 bytes/px (LE)     |
|   18 | `BayerGR16`          | 2 bytes/px (LE)     |
|   19 | `BayerBG16`          | 2 bytes/px (LE)     |
|   20 | `BayerGB16`          | 2 bytes/px (LE)     |
|   21 | `RGB8`               | 3 bytes/px          |
|   22 | `BGR8`               | 3 bytes/px          |
|   23 | `RGBa8`              | 4 bytes/px          |
|   24 | `YCbCr422_8`         | 2 bytes/px          |
|   25 | `YCbCr8`             | 3 bytes/px          |
|   26 | `Coord3D_C16`        | 2 bytes/px (LE)     |

- **Rust types:** `FrameHeader`, `FrameHeaderError`, `FRAME_MAGIC`, `HEADER_SIZE`,
  `pixel_format_to_u8`, `u8_to_pixel_format` — all in `genicam_zenoh_api::frame_header`.

### `genicam/devices/{device_id}/image/meta`

- **Direction:** Service → App and Streamer
- **Mechanism:** `put` on acquisition start; re-published whenever Width, Height, or
  PixelFormat node values change while acquisition is active
- **Consumers:**
  - **genicam-ws-streamer** (ST-01): subscribes to reconfigure its BMP encoder when
    dimensions or format change.
  - **Tauri app** (TB-01): subscribes and stores the latest value in
    `AcquisitionInner.image_meta`; emits `image-meta-changed` event to the frontend.
- **Payload (JSON):**
  ```json
  { "pixel_format": "Mono8", "width": 1920, "height": 1080, "payload_size": 2073600 }
  ```
- **Rust type:** `ImageMeta` in `genicam_zenoh_api`
- **Field notes:**
  - `pixel_format` — SFNC format string (e.g. `"Mono8"`, `"Mono16"`, `"BayerRG8"`,
    `"RGB8"`). Full variant list: `genicam_zenoh_api::PixelFormat`. Unknown strings
    deserialize as `PixelFormat::Unknown`.
  - `payload_size` — `width × height × bytes_per_pixel` for the given format.
    See `PixelFormat::bytes_per_pixel()` for the authoritative mapping.

---

## Timing Guarantees

| Key | Expected frequency |
|-----|--------------------|
| `announce` | Every 2 s (informational) |
| `nodes/*/value` | On parameter change (up to ~100 Hz for fast nodes) |
| `acquisition/status` | On change |
| `image` | At acquisition frame rate |
| `image/meta` | On acquisition start; on Width, Height, or PixelFormat change during active acquisition |

Device is considered lost if `announce` is not received for 6 seconds.

---

## Error Semantics

- All Zenoh `get()` calls to service queryables expect a single reply within 5 seconds.
- Timeout or Zenoh-level error → surfaced as `"Zenoh timeout or error"`.
- Service-level error → returned in `{ "ok": false, "error": "..." }` response body.
- The app never puts to `image` or `nodes/*/value` — those are service-owned.

---

## Sequence Diagrams

### Device Connect

```mermaid
sequenceDiagram
    participant App as Tauri App
    participant Z as Zenoh
    participant Svc as Camera Service

    Svc->>Z: put(announce) every 2 s
    Z-->>App: device-discovered event
    App->>Z: get(xml)
    Z->>Svc: query forwarded
    Svc-->>Z: reply DeviceXmlResponse
    Z-->>App: ParseXmlResponse (UiGraph)
    App->>Z: subscribe(nodes/*/value)
    App->>Z: subscribe(status)
    App->>Z: subscribe(acquisition/status)
    App->>Z: subscribe(image/meta)
```

### Acquisition

```mermaid
sequenceDiagram
    participant UI as Frontend
    participant App as Tauri App
    participant Str as Streamer
    participant Z as Zenoh
    participant Svc as Camera Service

    UI->>App: start_acquisition()
    App->>Z: get(acquisition/control) {command:"start"}
    Svc-->>Z: reply {ok:true}
    Svc->>Z: put(acquisition/status) {active:true}
    Svc->>Z: put(image/meta) {pixel_format,width,height,payload_size}
    App->>Str: spawn genicam-ws-streamer
    Str->>Z: subscribe(image)
    Str->>Z: subscribe(image/meta)
    Z-->>App: image-meta-changed → UI
    loop per frame
        Svc->>Z: put(image) [raw pixels]
        Z-->>Str: encode BMP → WebSocket → UI
    end
```

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
| `node-value-changed` | `{ node_name, value, access_mode, min?, max?, inc? }` | Live node update; `min`/`max`/`inc` present when service provides runtime constraints |
| `acquisition-status` | `AcquisitionStatus` | Acquisition state change |
| `connection-state-changed` | `ConnectionState` | Connect/disconnect/error |
| `image-meta-changed` | `ImageMeta` | Image format or dimension change; emitted by TB-01 on each image/meta Zenoh update |
| `streamer-status` | `StreamerStatus` | Streamer process lifecycle event: started, crashed (with restart), stopped. Fields: `running: bool`, `error: string \| null`, `restart_count: number`. |
| `disconnect-reason` | `DisconnectReason` | Emitted when service signals `{ connected: false }`. Fields: `message: string`, `device_id: string`. Used to display a reconnect prompt. |
