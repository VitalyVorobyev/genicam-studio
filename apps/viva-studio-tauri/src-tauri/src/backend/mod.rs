//! Backend abstraction for device communication.
//!
//! The app supports two operating modes:
//! - **Embedded** (default): direct GigE/USB3 camera access via `viva-genicam`.
//! - **Remote**: camera access through a separate service process via Zenoh.
//!
//! The [`DeviceBackend`] trait defines the interface that both modes implement,
//! allowing Tauri commands to delegate without knowing the transport.

pub mod embedded;
pub mod remote;

use std::collections::HashMap;

use async_trait::async_trait;

use crate::state::device_state::{DeviceInfo, NodeValueEntry, StreamerInfo};

/// Which backend mode the application is currently using.
#[derive(Debug, Clone, serde::Serialize)]
pub enum BackendMode {
    /// Direct camera communication (GigE + USB3) embedded in the app process.
    Embedded,
    /// Camera access via an external service process over Zenoh.
    Remote,
}

/// Result of a successful device connection.
pub struct ConnectResult {
    /// Raw GenICam XML fetched from the device.
    pub xml: String,
    /// Human-readable device name (user-defined name or model fallback).
    pub device_name: String,
    /// Device model string.
    pub model: String,
}

/// Trait abstracting device communication for both embedded and remote modes.
#[async_trait]
pub trait DeviceBackend: Send + Sync + 'static {
    /// Return the current backend mode.
    fn mode(&self) -> BackendMode;

    /// Discover available cameras on the network / USB bus.
    async fn discover(&self) -> Vec<DeviceInfo>;

    /// Connect to a device by its identifier (IP address for GigE, serial for USB3).
    async fn connect(&self, device_id: &str) -> Result<ConnectResult, String>;

    /// Disconnect from the currently connected device.
    async fn disconnect(&self, device_id: &str) -> Result<(), String>;

    /// Read a single feature value from the connected camera.
    async fn get_feature(&self, name: &str) -> Result<NodeValueEntry, String>;

    /// Write a feature value on the connected camera.
    async fn set_feature(&self, name: &str, value: &serde_json::Value) -> Result<(), String>;

    /// Execute a command feature on the connected camera.
    async fn exec_command(&self, name: &str) -> Result<(), String>;

    /// Read multiple feature values in a single round-trip.
    async fn bulk_read(&self, names: &[String]) -> Result<HashMap<String, NodeValueEntry>, String>;

    /// Start image acquisition and return the WebSocket URL for the stream.
    async fn start_acquisition(&self) -> Result<StreamerInfo, String>;

    /// Stop image acquisition.
    async fn stop_acquisition(&self) -> Result<(), String>;

    /// Return the raw GenICam XML of the connected device.
    async fn get_xml(&self) -> Result<String, String>;
}
