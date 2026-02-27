use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use genicam_zenoh_api::AcquisitionStatus;
use serde::{Deserialize, Serialize};

// ── Types exposed to the UI via Tauri IPC ───────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub id: String,
    pub name: String,
    pub model: String,
    pub serial: String,
}

/// Connection state, serialized with a `kind` tag so TypeScript can switch on it.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ConnectionState {
    Disconnected,
    Connecting {
        device_id: String,
    },
    Connected {
        device_id: String,
        device_name: String,
        model: String,
    },
    Error {
        message: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeValueEntry {
    pub value: serde_json::Value,
    pub access_mode: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct StreamerInfo {
    pub ws_url: String,
    pub width: u32,
    pub height: u32,
}

// ── Internal registry ────────────────────────────────────────────────────────

pub struct DiscoveredDevice {
    pub info: DeviceInfo,
    pub last_seen: Instant,
}

#[derive(Default)]
pub struct DeviceRegistry {
    pub devices: HashMap<String, DiscoveredDevice>,
}

impl DeviceRegistry {
    pub fn update(&mut self, info: DeviceInfo) {
        self.devices.insert(
            info.id.clone(),
            DiscoveredDevice {
                info,
                last_seen: Instant::now(),
            },
        );
    }

    /// Remove devices not seen for `timeout_secs` and return their IDs.
    pub fn expire_old(&mut self, timeout_secs: u64) -> Vec<String> {
        let now = Instant::now();
        let expired: Vec<String> = self
            .devices
            .iter()
            .filter(|(_, d)| now.duration_since(d.last_seen).as_secs() > timeout_secs)
            .map(|(id, _)| id.clone())
            .collect();
        for id in &expired {
            self.devices.remove(id);
        }
        expired
    }

    pub fn list(&self) -> Vec<DeviceInfo> {
        self.devices.values().map(|d| d.info.clone()).collect()
    }

    pub fn contains(&self, id: &str) -> bool {
        self.devices.contains_key(id)
    }

    pub fn get_name(&self, id: &str) -> Option<String> {
        self.devices.get(id).map(|d| d.info.name.clone())
    }

    pub fn get_model(&self, id: &str) -> Option<String> {
        self.devices.get(id).map(|d| d.info.model.clone())
    }
}

// ── Acquisition inner state ──────────────────────────────────────────────────

pub struct AcquisitionInner {
    pub child: Option<tokio::process::Child>,
    pub ws_url: Option<String>,
    pub status: AcquisitionStatus,
    pub width: u32,
    pub height: u32,
}

impl AcquisitionInner {
    pub fn new() -> Self {
        Self {
            child: None,
            ws_url: None,
            status: AcquisitionStatus {
                active: false,
                fps: None,
                dropped: 0,
            },
            width: 640,
            height: 480,
        }
    }
}

// ── Shared Zenoh application state ──────────────────────────────────────────

/// All Zenoh-related state for the Tauri backend.
///
/// Wrapped in `Arc` so it can be cloned into async background tasks while
/// Tauri holds the canonical reference via `.manage()`.
pub struct ZenohState {
    pub session: tokio::sync::Mutex<Option<Arc<zenoh::Session>>>,
    pub registry: tokio::sync::Mutex<DeviceRegistry>,
    pub connection: tokio::sync::Mutex<ConnectionState>,
    pub node_cache: tokio::sync::RwLock<HashMap<String, NodeValueEntry>>,
    pub acquisition: tokio::sync::Mutex<AcquisitionInner>,
    /// Handles for per-connection subscriber tasks; aborted on disconnect.
    pub sub_tasks: tokio::sync::Mutex<Vec<tauri::async_runtime::JoinHandle<()>>>,
}

impl ZenohState {
    pub fn new() -> Self {
        Self {
            session: tokio::sync::Mutex::new(None),
            registry: tokio::sync::Mutex::new(DeviceRegistry::default()),
            connection: tokio::sync::Mutex::new(ConnectionState::Disconnected),
            node_cache: tokio::sync::RwLock::new(HashMap::new()),
            acquisition: tokio::sync::Mutex::new(AcquisitionInner::new()),
            sub_tasks: tokio::sync::Mutex::new(Vec::new()),
        }
    }

    pub async fn get_session(&self) -> Result<Arc<zenoh::Session>, String> {
        self.session
            .lock()
            .await
            .clone()
            .ok_or_else(|| "Zenoh session not initialized".to_string())
    }
}
