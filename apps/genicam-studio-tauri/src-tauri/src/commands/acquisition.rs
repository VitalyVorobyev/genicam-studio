use std::sync::Arc;

use tauri::{AppHandle, Emitter, State};

use crate::commands::device::stop_acquisition_child;
use crate::state::device_state::{ConnectionState, StreamerInfo, ZenohState};
use genicam_zenoh_api::{
    AcquisitionCommand, AcquisitionControlRequest, AcquisitionStatus, NodeOpResponse,
};

#[tauri::command]
pub async fn get_acquisition_status(
    zenoh: State<'_, Arc<ZenohState>>,
) -> Result<AcquisitionStatus, String> {
    Ok(zenoh.acquisition.lock().await.status.clone())
}

#[tauri::command]
pub async fn start_acquisition(
    zenoh: State<'_, Arc<ZenohState>>,
    app: AppHandle,
) -> Result<StreamerInfo, String> {
    let session = zenoh.get_session().await?;
    let device_id = connected_device_id(&zenoh).await?;

    // Read image dimensions from cached node values (Width / Height SFNC nodes)
    let (width, height) = {
        let cache = zenoh.node_cache.read().await;
        let w = cache
            .get("Width")
            .and_then(|e| e.value.as_u64())
            .unwrap_or(640) as u32;
        let h = cache
            .get("Height")
            .and_then(|e| e.value.as_u64())
            .unwrap_or(480) as u32;
        (w, h)
    };

    // Send start command to device service
    send_acquisition_command(&session, &device_id, AcquisitionCommand::Start).await?;

    // Resolve streamer binary path: prefer workspace target, fall back to PATH
    let streamer_path = resolve_streamer_path();

    let image_key = genicam_zenoh_api::keys::image(&device_id);
    let ws_port: u16 = 8081;
    let ws_url = format!("ws://127.0.0.1:{ws_port}/ws");

    let child = tokio::process::Command::new(&streamer_path)
        .args([
            "--image-key",
            &image_key,
            "--width",
            &width.to_string(),
            "--height",
            &height.to_string(),
            "--bind",
            &format!("127.0.0.1:{ws_port}"),
        ])
        .spawn()
        .map_err(|e| format!("Failed to spawn genicam-ws-streamer ({streamer_path}): {e}"))?;

    {
        let mut acq = zenoh.acquisition.lock().await;
        acq.child = Some(child);
        acq.ws_url = Some(ws_url.clone());
        acq.width = width;
        acq.height = height;
        acq.status.active = true;
    }

    let _ = app.emit(
        "acquisition-status",
        AcquisitionStatus {
            active: true,
            fps: None,
            dropped: 0,
        },
    );

    Ok(StreamerInfo {
        ws_url,
        width,
        height,
    })
}

#[tauri::command]
pub async fn stop_acquisition(
    zenoh: State<'_, Arc<ZenohState>>,
    app: AppHandle,
) -> Result<(), String> {
    let session = zenoh.get_session().await?;
    let device_id = connected_device_id(&zenoh).await?;

    // Signal device service to stop — best-effort, don't block on error
    let _ = send_acquisition_command(&session, &device_id, AcquisitionCommand::Stop).await;

    stop_acquisition_child(&zenoh).await;

    let _ = app.emit(
        "acquisition-status",
        AcquisitionStatus {
            active: false,
            fps: None,
            dropped: 0,
        },
    );

    Ok(())
}

// ── Helpers ───────────────────────────────────────────────────────────────────

async fn connected_device_id(zenoh: &ZenohState) -> Result<String, String> {
    match zenoh.connection.lock().await.clone() {
        ConnectionState::Connected { device_id, .. } => Ok(device_id),
        _ => Err("Not connected to a device".to_string()),
    }
}

async fn send_acquisition_command(
    session: &zenoh::Session,
    device_id: &str,
    command: AcquisitionCommand,
) -> Result<(), String> {
    let key = genicam_zenoh_api::keys::acquisition_control(device_id);
    let payload =
        serde_json::to_vec(&AcquisitionControlRequest { command }).map_err(|e| e.to_string())?;

    let replies = session
        .get(&key)
        .payload(payload)
        .await
        .map_err(|e| format!("Zenoh error: {e}"))?;

    match replies.recv_async().await {
        Ok(reply) => match reply.result() {
            Ok(sample) => {
                let bytes = sample.payload().to_bytes();
                let resp: NodeOpResponse =
                    serde_json::from_slice(&bytes).map_err(|e| format!("Parse error: {e}"))?;
                if resp.ok {
                    Ok(())
                } else {
                    Err(resp
                        .error
                        .unwrap_or_else(|| "Acquisition command failed".to_string()))
                }
            }
            Err(e) => Err(format!("Reply error: {e}")),
        },
        Err(_) => Err("No reply for acquisition command (timeout)".to_string()),
    }
}

/// Resolve the path to the genicam-ws-streamer binary.
/// In development, prefer the workspace target/debug directory.
fn resolve_streamer_path() -> String {
    // Check workspace debug build first (cargo tauri dev scenario)
    let dev_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../../target/debug/genicam-ws-streamer"
    );
    if std::path::Path::new(dev_path).exists() {
        return dev_path.to_string();
    }
    // Fall back to PATH lookup (installed / bundled binary)
    "genicam-ws-streamer".to_string()
}
