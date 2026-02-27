// Types mirroring the Rust device_state.rs enums and structs.
// Kept in sync with the Tauri backend serialization.

export interface DeviceInfo {
  id: string;
  name: string;
  model: string;
  serial: string;
}

export type ConnectionState =
  | { kind: "disconnected" }
  | { kind: "connecting"; device_id: string }
  | { kind: "connected"; device_id: string; device_name: string; model: string }
  | { kind: "error"; message: string };

export interface AcquisitionStatus {
  active: boolean;
  fps: number | null;
  dropped: number;
}

export interface StreamerInfo {
  ws_url: string;
  width: number;
  height: number;
}

export interface NodeValueEntry {
  value: number | string | boolean;
  access_mode: string;
}
