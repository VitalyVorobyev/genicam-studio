use std::sync::Arc;

use tauri::{AppHandle, Emitter, State};

use crate::commands::device::stop_acquisition_child;
use crate::state::device_state::{ConnectionState, StreamerInfo, StreamerStatus, ZenohState};
use genicam_zenoh_api::{
    AcquisitionCommand, AcquisitionControlRequest, AcquisitionStatus, NodeOpResponse,
};

// ── Streamer args ─────────────────────────────────────────────────────────────

/// Everything needed to (re-)spawn a `genicam-ws-streamer` process.
struct StreamerArgs {
    path: String,
    image_key: String,
    width: u32,
    height: u32,
    port: u16,
}

/// Build the CLI argument list for `genicam-ws-streamer`.
///
/// Pure function — no I/O, suitable for unit tests.
fn build_streamer_args(image_key: &str, width: u32, height: u32, port: u16) -> Vec<String> {
    vec![
        "--image-key".to_string(),
        image_key.to_string(),
        "--width".to_string(),
        width.to_string(),
        "--height".to_string(),
        height.to_string(),
        "--bind".to_string(),
        format!("127.0.0.1:{port}"),
    ]
}

/// Attempt to spawn the streamer child process.
fn spawn_streamer_child(args: &StreamerArgs) -> Result<tokio::process::Child, String> {
    let cli_args = build_streamer_args(&args.image_key, args.width, args.height, args.port);
    tokio::process::Command::new(&args.path)
        .args(&cli_args)
        .spawn()
        .map_err(|e| format!("Failed to spawn genicam-ws-streamer ({}): {e}", args.path))
}

// ── Monitor task ──────────────────────────────────────────────────────────────

/// Maximum number of automatic restart attempts before giving up.
const MAX_RESTARTS: u32 = 5;
/// Fixed delay between restart attempts.
const RESTART_DELAY_MS: u64 = 500;

/// Monitor task: owns the streamer child and handles auto-restart on crash.
///
/// Runs until:
/// - A `true` value is received on `stop_rx` (intentional stop → kills child, exits).
/// - `stop_rx` sender is dropped (interpreted as stop → kills child, exits).
/// - The child crashes `MAX_RESTARTS` times (gives up, emits final error event).
async fn run_streamer_monitor(
    args: StreamerArgs,
    app: AppHandle,
    mut stop_rx: tokio::sync::watch::Receiver<bool>,
) {
    let mut restart_count: u32 = 0;

    loop {
        // Check stop signal before attempting (re-)spawn.
        if *stop_rx.borrow() {
            return;
        }

        let mut child = match spawn_streamer_child(&args) {
            Ok(c) => c,
            Err(e) => {
                let _ = app.emit(
                    "streamer-status",
                    StreamerStatus {
                        running: false,
                        error: Some(e.clone()),
                        restart_count,
                    },
                );
                eprintln!("Streamer spawn failed: {e}");
                return;
            }
        };

        let _ = app.emit(
            "streamer-status",
            StreamerStatus {
                running: true,
                error: None,
                restart_count,
            },
        );

        tokio::select! {
            result = child.wait() => {
                // Child exited — unexpected if stop was not requested.
                let _ = child.kill().await; // no-op if already exited
                let exit_desc = match result {
                    Ok(s) => format!("exit status {s}"),
                    Err(e) => e.to_string(),
                };
                let _ = app.emit(
                    "streamer-status",
                    StreamerStatus {
                        running: false,
                        error: Some(format!("Streamer crashed ({exit_desc})")),
                        restart_count,
                    },
                );
                eprintln!("genicam-ws-streamer exited unexpectedly ({exit_desc}), restart_count={restart_count}");

                if restart_count >= MAX_RESTARTS {
                    eprintln!("Streamer exceeded max restart count; giving up");
                    return;
                }

                restart_count += 1;

                // Backoff sleep, interruptible by stop signal.
                tokio::select! {
                    _ = tokio::time::sleep(std::time::Duration::from_millis(RESTART_DELAY_MS)) => {}
                    result = stop_rx.changed() => {
                        // Stop requested or sender dropped.
                        let _ = result; // channel may be closed
                        return;
                    }
                }
            }
            result = stop_rx.changed() => {
                // Stop requested or sender dropped — kill child and exit.
                let _ = result;
                let _ = child.kill().await;
                let _ = app.emit(
                    "streamer-status",
                    StreamerStatus {
                        running: false,
                        error: None,
                        restart_count,
                    },
                );
                return;
            }
        }
    }
}

// ── IPC Commands ──────────────────────────────────────────────────────────────

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

    // Stop any previously running monitor before starting a new one.
    stop_acquisition_child(&zenoh).await;

    // Create a watch channel: send `true` to stop the monitor task.
    let (stop_tx, stop_rx) = tokio::sync::watch::channel(false);

    let args = StreamerArgs {
        path: streamer_path,
        image_key,
        width,
        height,
        port: ws_port,
    };

    let app_clone = app.clone();
    let monitor_handle = tauri::async_runtime::spawn(async move {
        run_streamer_monitor(args, app_clone, stop_rx).await;
    });

    {
        let mut acq = zenoh.acquisition.lock().await;
        acq.stop_tx = Some(stop_tx);
        acq.monitor_handle = Some(monitor_handle);
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

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_streamer_args_contains_image_key() {
        let args = build_streamer_args("genicam/devices/cam0/image", 640, 480, 8081);
        let pos = args
            .iter()
            .position(|a| a == "--image-key")
            .expect("--image-key missing");
        assert_eq!(args[pos + 1], "genicam/devices/cam0/image");
    }

    #[test]
    fn test_build_streamer_args_contains_bind() {
        let args = build_streamer_args("some/key", 1920, 1080, 9090);
        let pos = args
            .iter()
            .position(|a| a == "--bind")
            .expect("--bind missing");
        assert_eq!(args[pos + 1], "127.0.0.1:9090");
    }

    #[test]
    fn test_build_streamer_args_dimensions() {
        let args = build_streamer_args("k", 320, 240, 8081);
        let w_pos = args
            .iter()
            .position(|a| a == "--width")
            .expect("--width missing");
        let h_pos = args
            .iter()
            .position(|a| a == "--height")
            .expect("--height missing");
        assert_eq!(args[w_pos + 1], "320");
        assert_eq!(args[h_pos + 1], "240");
    }
}
