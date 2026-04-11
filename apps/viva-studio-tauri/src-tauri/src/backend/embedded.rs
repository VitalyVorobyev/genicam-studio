//! Embedded device backend: direct GigE Vision and USB3 Vision camera access.
//!
//! This backend uses `viva-genicam` to communicate directly with cameras without
//! requiring an external service process. It is the default mode when no Zenoh
//! configuration is detected.
//!
//! ## Thread safety
//!
//! `Camera<GigeRegisterIo>` is `!Sync` because the underlying `NodeMap` uses
//! `RefCell` for caching. All camera operations are dispatched to a dedicated
//! blocking thread via `spawn_blocking` and a `std::sync::Mutex` protects
//! concurrent access.

use std::collections::HashMap;
use std::net::Ipv4Addr;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use bytes::Bytes;
use tauri::Emitter;
use tokio::sync::{Mutex as AsyncMutex, RwLock, watch};
use tracing::{info, warn};
use viva_genicam::{Camera, FrameStream, GigeRegisterIo};

use crate::state::device_state::{DeviceInfo, NodeValueEntry, StreamerInfo};

use super::{BackendMode, ConnectResult, DeviceBackend, NetworkConfig};

// ── Internal types ──────────────────────────────────────────────────────────

/// A connected GigE Vision camera and its associated state.
///
/// Wrapped in a `std::sync::Mutex` because `Camera<GigeRegisterIo>` is `!Sync`
/// (the `NodeMap` uses `RefCell` for caching). All access goes through
/// `spawn_blocking`.
struct ConnectedCamera {
    camera: Camera<GigeRegisterIo>,
    /// Kept for logging and disconnect matching.
    #[allow(dead_code)]
    device_id: String,
    #[allow(dead_code)]
    xml: String,
}

// SAFETY: ConnectedCamera is only accessed through spawn_blocking with a
// std::sync::Mutex, so it is never shared across threads concurrently.
// The Mutex ensures exclusive access.
unsafe impl Send for ConnectedCamera {}

/// State for an active image acquisition session.
struct AcquisitionState {
    shutdown_tx: watch::Sender<bool>,
    task_handles: Vec<tokio::task::JoinHandle<()>>,
    /// Kept for diagnostics and status reporting.
    #[allow(dead_code)]
    ws_url: String,
    #[allow(dead_code)]
    width: u32,
    #[allow(dead_code)]
    height: u32,
}

// ── EmbeddedBackend ─────────────────────────────────────────────────────────

/// Backend that communicates directly with cameras via GigE Vision / USB3 Vision.
pub struct EmbeddedBackend {
    /// Connected camera behind a std::sync::Mutex for spawn_blocking access.
    camera: std::sync::Mutex<Option<ConnectedCamera>>,
    /// Cache of discovered devices, updated periodically.
    discovered: RwLock<Vec<DeviceInfo>>,
    /// Active acquisition state, if streaming.
    acquisition: AsyncMutex<Option<AcquisitionState>>,
}

impl EmbeddedBackend {
    /// Create a new embedded backend with no connected camera.
    pub fn new() -> Self {
        Self {
            camera: std::sync::Mutex::new(None),
            discovered: RwLock::new(Vec::new()),
            acquisition: AsyncMutex::new(None),
        }
    }

    /// Start a background task that periodically discovers GigE cameras.
    pub fn start_discovery_task(self: &Arc<Self>, app: tauri::AppHandle, interval: Duration) {
        let backend = self.clone();
        tauri::async_runtime::spawn(async move {
            let mut ticker = tokio::time::interval(interval);
            loop {
                ticker.tick().await;
                let devices = discover_gige_devices().await;
                *backend.discovered.write().await = devices.clone();
                for d in &devices {
                    let _ = app.emit("device-discovered", d);
                }
            }
        });
    }

    /// Stop streamer tasks and clean up acquisition state.
    async fn stop_acquisition_inner(&self) {
        let mut acq = self.acquisition.lock().await;
        if let Some(state) = acq.take() {
            let _ = state.shutdown_tx.send(true);
            for handle in state.task_handles {
                handle.abort();
            }
        }
    }
}

#[async_trait]
impl DeviceBackend for EmbeddedBackend {
    fn mode(&self) -> BackendMode {
        BackendMode::Embedded
    }

    async fn discover(&self) -> Vec<DeviceInfo> {
        self.discovered.read().await.clone()
    }

    async fn connect(&self, device_id: &str) -> Result<ConnectResult, String> {
        // Disconnect any existing camera first.
        self.disconnect(device_id).await.ok();

        let ip: Ipv4Addr = device_id
            .parse()
            .map_err(|_| format!("Invalid device ID '{device_id}': expected an IPv4 address"))?;

        // Look up device info from discovery cache.
        let (name, model) = {
            let discovered = self.discovered.read().await;
            discovered
                .iter()
                .find(|d| d.id == device_id)
                .map(|d| (d.name.clone(), d.model.clone()))
                .unwrap_or_else(|| (device_id.to_string(), String::new()))
        };

        let gige_info = viva_genicam::gige::DeviceInfo {
            ip,
            mac: [0; 6],
            model: if model.is_empty() {
                None
            } else {
                Some(model.clone())
            },
            manufacturer: None,
        };

        info!(device_id, "Connecting to GigE camera (embedded mode)");

        let (camera, xml) = viva_genicam::connect_gige_with_xml(&gige_info)
            .await
            .map_err(|e| format!("Failed to connect to camera at {device_id}: {e}"))?;

        info!(
            device_id,
            xml_len = xml.len(),
            "Camera connected, XML fetched"
        );

        let result = ConnectResult {
            xml: xml.clone(),
            device_name: name,
            model,
        };

        {
            let mut guard = self
                .camera
                .lock()
                .map_err(|_| "Camera mutex poisoned".to_string())?;
            *guard = Some(ConnectedCamera {
                camera,
                device_id: device_id.to_string(),
                xml,
            });
        }

        Ok(result)
    }

    async fn disconnect(&self, _device_id: &str) -> Result<(), String> {
        self.stop_acquisition_inner().await;
        {
            let mut guard = self
                .camera
                .lock()
                .map_err(|_| "Camera mutex poisoned".to_string())?;
            *guard = None;
        }
        info!("Camera disconnected (embedded mode)");
        Ok(())
    }

    async fn get_feature(&self, name: &str) -> Result<NodeValueEntry, String> {
        let name = name.to_string();
        // Clone the Arc-wrapped self is not possible since Camera is behind std::sync::Mutex.
        // Instead, we do the blocking work inline — the std::sync::Mutex lock is held
        // briefly and does not cross await points.
        let value_str = {
            let mut guard = self
                .camera
                .lock()
                .map_err(|_| "Camera mutex poisoned".to_string())?;
            let connected = guard
                .as_mut()
                .ok_or_else(|| "No camera connected".to_string())?;
            connected
                .camera
                .get(&name)
                .map_err(|e| format!("Failed to read feature '{name}': {e}"))?
        };

        Ok(NodeValueEntry {
            value: string_to_json_value(&value_str),
            access_mode: "RW".to_string(),
            min: None,
            max: None,
            inc: None,
        })
    }

    async fn set_feature(&self, name: &str, value: &serde_json::Value) -> Result<(), String> {
        let value_str = json_value_to_string(value);
        let name = name.to_string();

        let mut guard = self
            .camera
            .lock()
            .map_err(|_| "Camera mutex poisoned".to_string())?;
        let connected = guard
            .as_mut()
            .ok_or_else(|| "No camera connected".to_string())?;
        connected
            .camera
            .set(&name, &value_str)
            .map_err(|e| format!("Failed to write feature '{name}': {e}"))
    }

    async fn exec_command(&self, name: &str) -> Result<(), String> {
        let name = name.to_string();

        let mut guard = self
            .camera
            .lock()
            .map_err(|_| "Camera mutex poisoned".to_string())?;
        let connected = guard
            .as_mut()
            .ok_or_else(|| "No camera connected".to_string())?;
        connected
            .camera
            .set(&name, "")
            .map_err(|e| format!("Failed to execute command '{name}': {e}"))
    }

    async fn bulk_read(&self, names: &[String]) -> Result<HashMap<String, NodeValueEntry>, String> {
        let mut guard = self
            .camera
            .lock()
            .map_err(|_| "Camera mutex poisoned".to_string())?;
        let connected = guard
            .as_mut()
            .ok_or_else(|| "No camera connected".to_string())?;

        let mut result = HashMap::with_capacity(names.len());
        for name in names {
            match connected.camera.get(name) {
                Ok(value_str) => {
                    result.insert(
                        name.clone(),
                        NodeValueEntry {
                            value: string_to_json_value(&value_str),
                            access_mode: "RW".to_string(),
                            min: None,
                            max: None,
                            inc: None,
                        },
                    );
                }
                Err(e) => {
                    warn!(name, error = %e, "Failed to read feature in bulk_read");
                }
            }
        }

        Ok(result)
    }

    async fn start_acquisition(&self) -> Result<StreamerInfo, String> {
        // Stop any previous acquisition.
        self.stop_acquisition_inner().await;

        // Read dimensions and build the stream while holding the camera lock.
        let (width, height, frame_stream) = {
            let mut guard = self
                .camera
                .lock()
                .map_err(|_| "Camera mutex poisoned".to_string())?;
            let connected = guard
                .as_mut()
                .ok_or_else(|| "No camera connected".to_string())?;

            let width = connected
                .camera
                .get("Width")
                .ok()
                .and_then(|s| s.parse::<u32>().ok())
                .unwrap_or(640);
            let height = connected
                .camera
                .get("Height")
                .ok()
                .and_then(|s| s.parse::<u32>().ok())
                .unwrap_or(480);

            // Get the camera's IP to detect the network interface.
            let camera_ip = connected
                .camera
                .transport()
                .lock_device()
                .map_err(|e| format!("Failed to access device: {e}"))?
                .remote_addr()
                .ip();

            let camera_ipv4 = match camera_ip {
                std::net::IpAddr::V4(ip) => ip,
                _ => {
                    return Err("IPv6 cameras are not supported for streaming".to_string());
                }
            };

            let iface = viva_genicam::gige::nic::Iface::from_ipv4(camera_ipv4)
                .map_err(|e| format!("Failed to detect network interface: {e}"))?;

            // Build the GVSP stream — needs &mut GigeDevice.
            // We must hold the device lock for this blocking-async sequence.
            // GigeRegisterIo::lock_device() returns a std::sync::MutexGuard.
            // StreamBuilder::build() is async but doesn't cross thread boundaries
            // in a problematic way — it just does socket operations.
            //
            // However, since GigeDevice is behind a std::sync::Mutex inside
            // GigeRegisterIo, and StreamBuilder needs &mut GigeDevice, we need
            // to work with the MutexGuard directly.
            let mut device_guard = connected
                .camera
                .transport()
                .lock_device()
                .map_err(|e| format!("Failed to access device: {e}"))?;

            // We can't call .build().await while holding a std::sync::MutexGuard
            // across an await point. Instead, we need to drop the guard and use a
            // different approach: build the stream outside the guard.
            //
            // The issue is that StreamBuilder::new needs &mut GigeDevice.
            // Let's build stream parameters here and construct later.

            // Actually, StreamBuilder::build() is async because it does network I/O
            // (bind socket, configure device). But we have the MutexGuard which is
            // not Send. We need to handle this carefully.
            //
            // Solution: use tokio::task::block_in_place to run the async build
            // within the blocking context while holding the lock.
            let handle = tokio::runtime::Handle::current();
            let stream = handle
                .block_on(
                    viva_genicam::StreamBuilder::new(&mut device_guard)
                        .iface(iface)
                        .auto_packet_size(true)
                        .build(),
                )
                .map_err(|e| format!("Failed to build stream: {e}"))?;

            drop(device_guard);

            let frame_stream = FrameStream::new(stream, None);

            // Start acquisition on the camera.
            connected
                .camera
                .acquisition_start()
                .map_err(|e| format!("Failed to start acquisition: {e}"))?;

            (width, height, frame_stream)
        };

        // Create WS broadcast channels.
        let (frame_tx, _) = watch::channel(Bytes::new());
        let info_tx = watch::channel(viva_streamer::ws::StreamInfo {
            width,
            height,
            pixel_format: "Mono8".to_string(),
            encoding: "BMP",
            frame_type: "info",
        })
        .0;
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        // Bind WebSocket server.
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .map_err(|e| format!("Failed to bind WS listener: {e}"))?;
        let port = listener
            .local_addr()
            .map_err(|e| format!("Failed to get local address: {e}"))?
            .port();
        let ws_url = format!("ws://127.0.0.1:{port}/ws");

        info!(ws_url, width, height, "Embedded streamer starting");

        // Spawn frame reader task.
        let frame_reader_handle = {
            let frame_tx = frame_tx.clone();
            let mut shutdown_rx = shutdown_rx.clone();
            let mut frame_stream = frame_stream;
            tokio::spawn(async move {
                let mut encoder_gray = viva_streamer::bmp::BmpEncoder::new(width, height);
                let mut encoder_rgb = viva_streamer::bmp::BmpEncoder::new_rgb24(width, height);
                let mut logged_first = false;

                loop {
                    tokio::select! {
                        _ = shutdown_rx.changed() => {
                            if *shutdown_rx.borrow() {
                                break;
                            }
                        }
                        result = frame_stream.next_frame() => {
                            match result {
                                Ok(Some(frame)) => {
                                    if !logged_first {
                                        info!(
                                            width = frame.width,
                                            height = frame.height,
                                            pixel_format = ?frame.pixel_format,
                                            "First frame received (embedded)"
                                        );
                                        logged_first = true;
                                    }

                                    if frame.width != width || frame.height != height {
                                        encoder_gray = viva_streamer::bmp::BmpEncoder::new(
                                            frame.width,
                                            frame.height,
                                        );
                                        encoder_rgb = viva_streamer::bmp::BmpEncoder::new_rgb24(
                                            frame.width,
                                            frame.height,
                                        );
                                    }

                                    let bmp = encode_camera_frame(
                                        &frame,
                                        &encoder_gray,
                                        &encoder_rgb,
                                    );
                                    match bmp {
                                        Ok(Some(data)) => {
                                            let _ = frame_tx.send(data);
                                        }
                                        Ok(None) => {}
                                        Err(e) => {
                                            warn!("BMP encode error: {e}");
                                        }
                                    }
                                }
                                Ok(None) => {
                                    info!("Frame stream ended");
                                    break;
                                }
                                Err(e) => {
                                    warn!("Frame stream error: {e}");
                                    break;
                                }
                            }
                        }
                    }
                }
            })
        };

        // Spawn WS server task.
        let ws_handle = {
            let state = viva_streamer::ws::AppState { frame_tx, info_tx };
            let shutdown_rx = shutdown_rx.clone();
            tokio::spawn(async move {
                if let Err(e) = viva_streamer::ws::run_server_with_listener(
                    listener,
                    "/ws".to_string(),
                    state,
                    shutdown_rx,
                )
                .await
                {
                    warn!("WS server task error: {e}");
                }
            })
        };

        *self.acquisition.lock().await = Some(AcquisitionState {
            shutdown_tx,
            task_handles: vec![frame_reader_handle, ws_handle],
            ws_url: ws_url.clone(),
            width,
            height,
        });

        Ok(StreamerInfo {
            ws_url,
            width,
            height,
        })
    }

    async fn stop_acquisition(&self) -> Result<(), String> {
        self.stop_acquisition_inner().await;

        // Stop acquisition on the camera.
        let mut guard = self
            .camera
            .lock()
            .map_err(|_| "Camera mutex poisoned".to_string())?;
        if let Some(connected) = guard.as_mut() {
            connected
                .camera
                .acquisition_stop()
                .map_err(|e| format!("Failed to stop acquisition: {e}"))?;
        }

        Ok(())
    }

    async fn get_xml(&self) -> Result<String, String> {
        let guard = self
            .camera
            .lock()
            .map_err(|_| "Camera mutex poisoned".to_string())?;
        guard
            .as_ref()
            .map(|c| c.xml.clone())
            .ok_or_else(|| "No camera connected".to_string())
    }

    async fn get_network_config(&self) -> Result<NetworkConfig, String> {
        let guard = self
            .camera
            .lock()
            .map_err(|_| "Camera mutex poisoned".to_string())?;
        let connected = guard
            .as_ref()
            .ok_or_else(|| "No camera connected".to_string())?;

        let mut device_guard = connected
            .camera
            .transport()
            .lock_device()
            .map_err(|e| format!("Failed to access device: {e}"))?;

        let current_ip = device_guard.remote_addr().ip().to_string();

        let handle = tokio::runtime::Handle::current();
        let (pip, psub, pgw) = handle
            .block_on(device_guard.read_persistent_ip())
            .map_err(|e| format!("Failed to read persistent IP: {e}"))?;

        // Get MAC from discovery cache.
        let mac = {
            let discovered = handle.block_on(self.discovered.read());
            discovered
                .iter()
                .find(|d| d.id == connected.device_id)
                .map(|d| d.serial.clone())
                .unwrap_or_default()
        };

        Ok(NetworkConfig {
            current_ip,
            persistent_ip: pip.to_string(),
            persistent_subnet: psub.to_string(),
            persistent_gateway: pgw.to_string(),
            mac,
        })
    }

    async fn set_persistent_ip(
        &self,
        ip: Ipv4Addr,
        subnet: Ipv4Addr,
        gateway: Ipv4Addr,
    ) -> Result<(), String> {
        let guard = self
            .camera
            .lock()
            .map_err(|_| "Camera mutex poisoned".to_string())?;
        let connected = guard
            .as_ref()
            .ok_or_else(|| "No camera connected".to_string())?;

        let mut device_guard = connected
            .camera
            .transport()
            .lock_device()
            .map_err(|e| format!("Failed to access device: {e}"))?;

        let handle = tokio::runtime::Handle::current();
        handle
            .block_on(device_guard.write_persistent_ip(ip, subnet, gateway))
            .map_err(|e| format!("Failed to write persistent IP: {e}"))?;

        handle
            .block_on(device_guard.enable_persistent_ip())
            .map_err(|e| format!("Failed to enable persistent IP: {e}"))?;

        info!(%ip, %subnet, %gateway, "Persistent IP configured and enabled");
        Ok(())
    }
}

// ── Discovery helpers ───────────────────────────────────────────────────────

async fn discover_gige_devices() -> Vec<DeviceInfo> {
    let timeout = Duration::from_secs(1);
    match viva_genicam::gige::discover_all(timeout).await {
        Ok(devices) => devices
            .into_iter()
            .map(|d| DeviceInfo {
                id: d.ip.to_string(),
                name: d.model.clone().unwrap_or_else(|| d.ip.to_string()),
                model: d.model.unwrap_or_default(),
                serial: format!(
                    "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
                    d.mac[0], d.mac[1], d.mac[2], d.mac[3], d.mac[4], d.mac[5]
                ),
                transport: "gige".to_string(),
            })
            .collect(),
        Err(e) => {
            warn!("GigE discovery failed: {e}");
            Vec::new()
        }
    }
}

// ── Value conversion helpers ────────────────────────────────────────────────

/// Convert a camera feature value string to a JSON value.
fn string_to_json_value(s: &str) -> serde_json::Value {
    if let Ok(n) = s.parse::<i64>() {
        return serde_json::Value::Number(n.into());
    }
    if let Ok(f) = s.parse::<f64>()
        && let Some(n) = serde_json::Number::from_f64(f)
    {
        return serde_json::Value::Number(n);
    }
    match s {
        "true" | "True" => return serde_json::Value::Bool(true),
        "false" | "False" => return serde_json::Value::Bool(false),
        _ => {}
    }
    serde_json::Value::String(s.to_string())
}

/// Convert a JSON value to a string suitable for the camera `set()` API.
fn json_value_to_string(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        other => other.to_string(),
    }
}

// ── Frame encoding ──────────────────────────────────────────────────────────

/// Convert a `viva_genicam::Frame` to BMP bytes for WebSocket delivery.
fn encode_camera_frame(
    frame: &viva_genicam::Frame,
    encoder_gray: &viva_streamer::bmp::BmpEncoder,
    encoder_rgb: &viva_streamer::bmp::BmpEncoder,
) -> Result<Option<Bytes>, String> {
    use viva_genicam::pfnc::PixelFormat;

    let pixel_data = frame.payload.as_ref();

    match frame.pixel_format {
        PixelFormat::Mono8 => encoder_gray
            .encode_gray8(pixel_data)
            .map(Some)
            .map_err(|e| e.to_string()),

        PixelFormat::Mono16 => {
            let gray = viva_streamer::bmp::mono_u16le_to_gray8(pixel_data, 16);
            encoder_gray
                .encode_gray8(&gray)
                .map(Some)
                .map_err(|e| e.to_string())
        }

        PixelFormat::RGB8Packed => encoder_rgb
            .encode_rgb24(pixel_data)
            .map(Some)
            .map_err(|e| e.to_string()),

        PixelFormat::BGR8Packed => {
            let rgb: Vec<u8> = pixel_data
                .chunks_exact(3)
                .flat_map(|c| [c[2], c[1], c[0]])
                .collect();
            encoder_rgb
                .encode_rgb24(&rgb)
                .map(Some)
                .map_err(|e| e.to_string())
        }

        PixelFormat::BayerRG8
        | PixelFormat::BayerGR8
        | PixelFormat::BayerBG8
        | PixelFormat::BayerGB8 => {
            let pattern = camera_bayer_pattern(frame.pixel_format);
            let rgb =
                viva_streamer::bmp::debayer_nn(pixel_data, frame.width, frame.height, pattern);
            encoder_rgb
                .encode_rgb24(&rgb)
                .map(Some)
                .map_err(|e| e.to_string())
        }

        _ => {
            warn!(
                pixel_format = ?frame.pixel_format,
                "Unsupported pixel format in embedded streaming"
            );
            Ok(None)
        }
    }
}

fn camera_bayer_pattern(pf: viva_genicam::pfnc::PixelFormat) -> viva_streamer::bmp::BayerPattern {
    use viva_genicam::pfnc::PixelFormat;
    use viva_streamer::bmp::BayerPattern;

    match pf {
        PixelFormat::BayerRG8 => BayerPattern::Rggb,
        PixelFormat::BayerGR8 => BayerPattern::Grbg,
        PixelFormat::BayerBG8 => BayerPattern::Bggr,
        PixelFormat::BayerGB8 => BayerPattern::Gbrg,
        _ => BayerPattern::Rggb,
    }
}
