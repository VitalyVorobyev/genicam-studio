//! E2E test harness for GenICam Studio.
//!
//! Spawns `arv-fake-gv-camera-0.8` and `genicam-service` as child processes,
//! waits for device discovery, and provides a Zenoh session for test assertions.
//!
//! All tests are `#[ignore]` — they require external binaries and are run via:
//! ```bash
//! cargo test -p e2e-tests -- --ignored --test-threads=1
//! ```

use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;

use tokio::process::{Child, Command};
use tokio::time::{sleep, timeout};

use genicam_zenoh_api::DeviceAnnounce;

/// Environment variable for the genicam-service binary path.
const SERVICE_PATH_ENV: &str = "GENICAM_SERVICE_PATH";
/// Default service binary path (assumes genicam-rs is a sibling directory).
const SERVICE_PATH_DEFAULT: &str = "../genicam-rs/target/debug/genicam-service";

/// Environment variable for the fake camera binary path.
const FAKE_CAM_PATH_ENV: &str = "ARV_FAKE_CAMERA_PATH";
/// Default: find in PATH.
const FAKE_CAM_PATH_DEFAULT: &str = "arv-fake-gv-camera-0.8";

/// Environment variable for a Zenoh config file (JSON5).
const ZENOH_CONFIG_ENV: &str = "ZENOH_CONFIG";

/// The device ID produced by the aravis fake camera (all-zero MAC).
pub const FAKE_DEVICE_ID: &str = "cam-000000000000";

/// How long to wait for the first announce message from genicam-service.
const ANNOUNCE_TIMEOUT: Duration = Duration::from_secs(15);

/// Test harness that manages the fake camera and service lifecycle.
pub struct TestHarness {
    fake_camera: Child,
    service: Child,
    session: Arc<zenoh::Session>,
    device_id: String,
}

#[derive(Debug)]
pub enum HarnessError {
    FakeCameraSpawn(String),
    ServiceSpawn(String),
    ZenohOpen(String),
    Timeout(String),
}

impl std::fmt::Display for HarnessError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FakeCameraSpawn(e) => write!(f, "Failed to spawn fake camera: {e}"),
            Self::ServiceSpawn(e) => write!(f, "Failed to spawn service: {e}"),
            Self::ZenohOpen(e) => write!(f, "Failed to open Zenoh session: {e}"),
            Self::Timeout(e) => write!(f, "Timeout: {e}"),
        }
    }
}

impl std::error::Error for HarnessError {}

impl TestHarness {
    /// Start the fake camera, genicam-service, and a Zenoh test session.
    ///
    /// Waits for the service to publish its first announce message before returning.
    pub async fn start() -> Result<Self, HarnessError> {
        init_tracing();

        let fake_cam_path =
            std::env::var(FAKE_CAM_PATH_ENV).unwrap_or_else(|_| FAKE_CAM_PATH_DEFAULT.to_string());
        let service_path =
            std::env::var(SERVICE_PATH_ENV).unwrap_or_else(|_| SERVICE_PATH_DEFAULT.to_string());

        // 1. Start the fake GigE camera on loopback
        tracing::info!("Starting fake camera: {fake_cam_path}");
        let fake_camera = Command::new(&fake_cam_path)
            .arg("-i")
            .arg("127.0.0.1")
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| {
                HarnessError::FakeCameraSpawn(format!(
                    "{e} (path: {fake_cam_path}). Is aravis installed?"
                ))
            })?;

        // 2. Wait for the camera to be ready (brief delay for UDP socket binding)
        sleep(Duration::from_secs(2)).await;

        // 3. Start genicam-service with loopback interface
        tracing::info!("Starting service: {service_path}");
        let mut svc_cmd = Command::new(&service_path);
        svc_cmd
            .arg("--iface")
            .arg(if cfg!(target_os = "macos") {
                "lo0"
            } else {
                "lo"
            })
            .stdout(Stdio::null())
            .stderr(Stdio::piped());

        // Pass zenoh config if available
        let zenoh_config_path = std::env::var(ZENOH_CONFIG_ENV).ok();
        if let Some(ref cfg_path) = zenoh_config_path {
            svc_cmd.arg("--zenoh-config").arg(cfg_path);
        }

        let service = svc_cmd.spawn().map_err(|e| {
            HarnessError::ServiceSpawn(format!(
                "{e} (path: {service_path}). Build it with: cd ../genicam-rs && cargo build -p genicam-service"
            ))
        })?;

        // 4. Open Zenoh session (load config if provided)
        let zenoh_config = match &zenoh_config_path {
            Some(path) => zenoh::Config::from_file(path)
                .map_err(|e| HarnessError::ZenohOpen(format!("config {path}: {e}")))?,
            None => zenoh::Config::default(),
        };
        let session = Arc::new(
            zenoh::open(zenoh_config)
                .await
                .map_err(|e| HarnessError::ZenohOpen(e.to_string()))?,
        );

        // 5. Wait for the first announce message
        tracing::info!("Waiting for device announce...");
        let sub = session
            .declare_subscriber(genicam_zenoh_api::keys::ANNOUNCE_ALL)
            .await
            .map_err(|e| HarnessError::ZenohOpen(format!("subscriber: {e}")))?;

        let announce = timeout(ANNOUNCE_TIMEOUT, async {
            loop {
                if let Ok(sample) = sub.recv_async().await {
                    let bytes = sample.payload().to_bytes();
                    if let Ok(a) = serde_json::from_slice::<DeviceAnnounce>(&bytes) {
                        return a;
                    }
                }
            }
        })
        .await
        .map_err(|_| {
            HarnessError::Timeout(format!(
                "No announce received within {}s. Is genicam-service running?",
                ANNOUNCE_TIMEOUT.as_secs()
            ))
        })?;

        let device_id = announce.id.clone();
        tracing::info!("Device discovered: {device_id} ({})", announce.name);

        // Give Zenoh time to fully establish bidirectional transport
        // and propagate queryable declarations across peers.
        // pub/sub works quickly but queryables with wildcards need more time.
        sleep(Duration::from_secs(5)).await;

        Ok(Self {
            fake_camera,
            service,
            session,
            device_id,
        })
    }

    /// The device ID of the discovered fake camera.
    pub fn device_id(&self) -> &str {
        &self.device_id
    }

    /// The Zenoh session for test assertions.
    pub fn session(&self) -> &Arc<zenoh::Session> {
        &self.session
    }

    /// Kill the fake camera process (for device-lost tests).
    pub async fn kill_fake_camera(&mut self) -> Result<(), std::io::Error> {
        self.fake_camera.kill().await
    }

    /// Graceful shutdown: kill service first, then fake camera.
    pub async fn shutdown(&mut self) {
        let _ = self.service.kill().await;
        let _ = self.fake_camera.kill().await;
    }
}

impl Drop for TestHarness {
    fn drop(&mut self) {
        // Best-effort synchronous kill (async Drop not available)
        let _ = self.service.start_kill();
        let _ = self.fake_camera.start_kill();
    }
}

fn init_tracing() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "e2e_tests=info,warn".into()),
        )
        .try_init();
}

// ── Helpers for tests ────────────────────────────────────────────────────────

/// Fetch the GenICam XML for a device via Zenoh GET.
pub async fn fetch_xml(session: &zenoh::Session, device_id: &str) -> Result<String, String> {
    let key = genicam_zenoh_api::keys::xml(device_id);
    let replies = session
        .get(&key)
        .timeout(Duration::from_secs(10))
        .await
        .map_err(|e| format!("GET error: {e}"))?;

    match replies.recv_async().await {
        Ok(reply) => match reply.result() {
            Ok(sample) => {
                let bytes = sample.payload().to_bytes();
                let resp: genicam_zenoh_api::DeviceXmlResponse =
                    serde_json::from_slice(&bytes).map_err(|e| format!("parse: {e}"))?;
                Ok(resp.xml)
            }
            Err(e) => Err(format!("reply error: {e}")),
        },
        Err(_) => Err("no reply (timeout)".to_string()),
    }
}

/// Write a node value via Zenoh GET with payload.
pub async fn write_node(
    session: &zenoh::Session,
    device_id: &str,
    node_name: &str,
    value: serde_json::Value,
) -> Result<(), String> {
    let key = genicam_zenoh_api::keys::node_set(device_id, node_name);
    let payload = serde_json::to_vec(&genicam_zenoh_api::NodeSetRequest { value })
        .map_err(|e| e.to_string())?;

    let replies = session
        .get(&key)
        .payload(payload)
        .timeout(Duration::from_secs(5))
        .await
        .map_err(|e| format!("GET error: {e}"))?;

    match replies.recv_async().await {
        Ok(reply) => match reply.result() {
            Ok(sample) => {
                let bytes = sample.payload().to_bytes();
                let resp: genicam_zenoh_api::NodeOpResponse =
                    serde_json::from_slice(&bytes).map_err(|e| format!("parse: {e}"))?;
                if resp.ok {
                    Ok(())
                } else {
                    Err(resp.error.unwrap_or_else(|| "write failed".to_string()))
                }
            }
            Err(e) => Err(format!("reply error: {e}")),
        },
        Err(_) => Err("no reply (timeout)".to_string()),
    }
}

/// Bulk-read node values via Zenoh GET with payload.
pub async fn read_bulk(
    session: &zenoh::Session,
    device_id: &str,
    names: &[&str],
) -> Result<genicam_zenoh_api::BulkReadResponse, String> {
    let key = genicam_zenoh_api::keys::nodes_bulk_read(device_id);
    let payload = serde_json::to_vec(&genicam_zenoh_api::BulkReadRequest {
        names: names.iter().map(|s| s.to_string()).collect(),
    })
    .map_err(|e| e.to_string())?;

    let replies = session
        .get(&key)
        .payload(payload)
        .timeout(Duration::from_secs(5))
        .await
        .map_err(|e| format!("GET error: {e}"))?;

    match replies.recv_async().await {
        Ok(reply) => match reply.result() {
            Ok(sample) => {
                let bytes = sample.payload().to_bytes();
                serde_json::from_slice(&bytes).map_err(|e| format!("parse: {e}"))
            }
            Err(e) => Err(format!("reply error: {e}")),
        },
        Err(_) => Err("no reply (timeout)".to_string()),
    }
}

/// Send an acquisition command (Start/Stop).
pub async fn send_acquisition_command(
    session: &zenoh::Session,
    device_id: &str,
    command: genicam_zenoh_api::AcquisitionCommand,
) -> Result<(), String> {
    let key = genicam_zenoh_api::keys::acquisition_control(device_id);
    let payload = serde_json::to_vec(&genicam_zenoh_api::AcquisitionControlRequest { command })
        .map_err(|e| e.to_string())?;

    let replies = session
        .get(&key)
        .payload(payload)
        .timeout(Duration::from_secs(5))
        .await
        .map_err(|e| format!("GET error: {e}"))?;

    match replies.recv_async().await {
        Ok(reply) => match reply.result() {
            Ok(sample) => {
                let bytes = sample.payload().to_bytes();
                let resp: genicam_zenoh_api::NodeOpResponse =
                    serde_json::from_slice(&bytes).map_err(|e| format!("parse: {e}"))?;
                if resp.ok {
                    Ok(())
                } else {
                    Err(resp.error.unwrap_or_else(|| "command failed".to_string()))
                }
            }
            Err(e) => Err(format!("reply error: {e}")),
        },
        Err(_) => Err("no reply (timeout)".to_string()),
    }
}
