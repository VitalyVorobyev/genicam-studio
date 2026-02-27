//! Shared Zenoh API payload types for GenICam Studio.
//!
//! This crate has no Zenoh dependency — it is a pure data contract that can be
//! used by both the camera service and the Tauri app.

use serde::{Deserialize, Serialize};

// ── Discovery ────────────────────────────────────────────────────────────────

/// Periodic announcement published by the camera service.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceAnnounce {
    pub id: String,
    pub name: String,
    pub model: String,
    pub serial: String,
}

// ── Connection Lifecycle ─────────────────────────────────────────────────────

/// Device connection status pushed by the service on change.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceStatus {
    pub connected: bool,
    pub error: Option<String>,
}

/// Response to `genicam/devices/{id}/xml` queryable.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceXmlResponse {
    pub xml: String,
}

// ── Node Values ──────────────────────────────────────────────────────────────

/// Live node value update published by the service on change.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeValueUpdate {
    pub value: serde_json::Value,
    pub access_mode: String,
}

/// Request payload for the `nodes/{name}/set` queryable.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeSetRequest {
    pub value: serde_json::Value,
}

/// Generic response for node write, execute, and acquisition control.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeOpResponse {
    pub ok: bool,
    pub error: Option<String>,
}

// ── Acquisition ──────────────────────────────────────────────────────────────

/// Request payload for the `acquisition/control` queryable.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcquisitionControlRequest {
    pub command: AcquisitionCommand,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AcquisitionCommand {
    Start,
    Stop,
}

/// Acquisition status pushed by the service on change.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcquisitionStatus {
    pub active: bool,
    pub fps: Option<f32>,
    pub dropped: u64,
}

// ── Key Expressions ──────────────────────────────────────────────────────────

/// Key expression constants and helpers for the GenICam Zenoh API.
pub mod keys {
    /// Wildcard subscription for all device announcements.
    pub const ANNOUNCE_ALL: &str = "genicam/devices/*/announce";

    pub fn announce(device_id: &str) -> String {
        format!("genicam/devices/{device_id}/announce")
    }

    pub fn xml(device_id: &str) -> String {
        format!("genicam/devices/{device_id}/xml")
    }

    pub fn status(device_id: &str) -> String {
        format!("genicam/devices/{device_id}/status")
    }

    pub fn node_value(device_id: &str, node_name: &str) -> String {
        format!("genicam/devices/{device_id}/nodes/{node_name}/value")
    }

    pub fn node_value_wildcard(device_id: &str) -> String {
        format!("genicam/devices/{device_id}/nodes/*/value")
    }

    pub fn node_set(device_id: &str, node_name: &str) -> String {
        format!("genicam/devices/{device_id}/nodes/{node_name}/set")
    }

    pub fn node_execute(device_id: &str, node_name: &str) -> String {
        format!("genicam/devices/{device_id}/nodes/{node_name}/execute")
    }

    pub fn acquisition_control(device_id: &str) -> String {
        format!("genicam/devices/{device_id}/acquisition/control")
    }

    pub fn acquisition_status(device_id: &str) -> String {
        format!("genicam/devices/{device_id}/acquisition/status")
    }

    pub fn image(device_id: &str) -> String {
        format!("genicam/devices/{device_id}/image")
    }

    /// Extract node name from `genicam/devices/{id}/nodes/{name}/value`.
    pub fn extract_node_name(key: &str) -> Option<&str> {
        // Split by '/' and expect at least 6 segments ending in "value"
        let parts: Vec<&str> = key.split('/').collect();
        if parts.len() >= 6 && parts[parts.len() - 1] == "value" {
            Some(parts[parts.len() - 2])
        } else {
            None
        }
    }
}
