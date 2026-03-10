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
  /** Optional runtime minimum constraint reported by the camera service. */
  min?: number;
  /** Optional runtime maximum constraint reported by the camera service. */
  max?: number;
  /** Optional runtime increment (step) reported by the camera service. */
  inc?: number;
}

export interface ImageMeta {
  pixel_format: string;
  width: number;
  height: number;
  payload_size: number;
}

export interface SfncFeature {
  node: string;
  widget: "float_slider" | "int_slider" | "int_select" | "enum_select" | "bool_toggle" | "command_button" | string;
}

export interface SfncGroup {
  id: string;
  title: string;
  icon: string;
  default_open: boolean;
  features: SfncFeature[];
}

export interface DisconnectReason {
  message: string;
  device_id: string;
}

export interface StreamerStatus {
  running: boolean;
  error: string | null;
  restart_count: number;
}
