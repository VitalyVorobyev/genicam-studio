use std::sync::Arc;
use std::time::Duration;

use tokio::sync::watch;
use tracing::{debug, error, info};

use genicam_zenoh_api::{
    AcquisitionCommand, AcquisitionControlRequest, AcquisitionStatus, NodeOpResponse,
};

use crate::config::MockConfig;
use crate::state::NodeStore;

/// Generate an animated Mono8 gradient test pattern.
/// Brightness reacts to ExposureTime and Gain values.
fn generate_mono8(
    width: u32,
    height: u32,
    frame_id: u64,
    brightness_scale: f64,
) -> Vec<u8> {
    let mut pixels = vec![0u8; (width * height) as usize];
    for y in 0..height {
        for x in 0..width {
            let idx = (y * width + x) as usize;
            let bar = ((x as u64 + frame_id * 2) % 256) as u8;
            let gradient = ((y as f32 / height as f32) * 255.0) as u8;
            let raw = bar.wrapping_add(gradient) as f64;
            let scaled = (raw * brightness_scale).clamp(0.0, 255.0) as u8;
            pixels[idx] = scaled;
        }
    }
    pixels
}

/// Compute brightness scale from ExposureTime and Gain node values.
async fn compute_brightness(store: &NodeStore) -> f64 {
    // ExposureTime: default 10000us. Normalize so 10000 -> 1.0
    let exposure = store
        .get_value("ExposureTime")
        .await
        .and_then(|v| v.as_f64())
        .unwrap_or(10000.0);
    let exposure_factor = (exposure / 10000.0).clamp(0.01, 10.0);

    // Gain: default 1.0 dB. Each 6dB doubles brightness.
    let gain_db = store
        .get_value("Gain")
        .await
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    let gain_factor = (10.0_f64).powf(gain_db / 20.0);

    exposure_factor * gain_factor
}

/// Get current image dimensions from the node store.
async fn get_dimensions(store: &NodeStore, config: &MockConfig) -> (u32, u32) {
    let width = store
        .get_value("Width")
        .await
        .and_then(|v| v.as_u64())
        .unwrap_or(config.width as u64) as u32;
    let height = store
        .get_value("Height")
        .await
        .and_then(|v| v.as_u64())
        .unwrap_or(config.height as u64) as u32;
    (width, height)
}

pub async fn run(
    session: Arc<zenoh::Session>,
    config: MockConfig,
    store: Arc<NodeStore>,
    mut shutdown: watch::Receiver<bool>,
) {
    let control_key = genicam_zenoh_api::keys::acquisition_control(&config.device_id);
    let status_key = genicam_zenoh_api::keys::acquisition_status(&config.device_id);
    let image_key = genicam_zenoh_api::keys::image(&config.device_id);

    let queryable = match session.declare_queryable(&control_key).await {
        Ok(q) => q,
        Err(e) => {
            error!("Failed to declare acquisition control queryable: {e}");
            return;
        }
    };

    let (acq_tx, mut acq_rx) = watch::channel(false);
    let mut frame_id: u64 = 0;

    // Publish initial status
    let initial_status = AcquisitionStatus {
        active: false,
        fps: None,
        dropped: 0,
    };
    let _ = session
        .put(
            &status_key,
            serde_json::to_vec(&initial_status).unwrap_or_default(),
        )
        .await;

    let frame_interval = Duration::from_secs_f64(1.0 / config.fps as f64);
    let mut ticker = tokio::time::interval(frame_interval);
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            _ = shutdown.changed() => {
                if *shutdown.borrow() { break; }
            }
            query = queryable.recv_async() => {
                match query {
                    Ok(query) => {
                        let payload = query.payload()
                            .map(|p| p.to_bytes().to_vec())
                            .unwrap_or_default();
                        let request: Result<AcquisitionControlRequest, _> = serde_json::from_slice(&payload);
                        let resp = match request {
                            Ok(req) => {
                                match req.command {
                                    AcquisitionCommand::Start => {
                                        info!("Acquisition started");
                                        let _ = acq_tx.send(true);
                                        let status = AcquisitionStatus {
                                            active: true,
                                            fps: Some(config.fps),
                                            dropped: 0,
                                        };
                                        let _ = session.put(&status_key, serde_json::to_vec(&status).unwrap_or_default()).await;
                                        NodeOpResponse { ok: true, error: None }
                                    }
                                    AcquisitionCommand::Stop => {
                                        info!("Acquisition stopped");
                                        let _ = acq_tx.send(false);
                                        let status = AcquisitionStatus {
                                            active: false,
                                            fps: None,
                                            dropped: 0,
                                        };
                                        let _ = session.put(&status_key, serde_json::to_vec(&status).unwrap_or_default()).await;
                                        NodeOpResponse { ok: true, error: None }
                                    }
                                }
                            }
                            Err(e) => {
                                NodeOpResponse { ok: false, error: Some(format!("Invalid request: {e}")) }
                            }
                        };
                        let _ = query.reply(&control_key, serde_json::to_vec(&resp).unwrap_or_default()).await;
                    }
                    Err(e) => {
                        error!("Acquisition control queryable recv error: {e}");
                        break;
                    }
                }
            }
            _ = ticker.tick(), if *acq_rx.borrow() => {
                let brightness = compute_brightness(&store).await;
                let (width, height) = get_dimensions(&store, &config).await;
                let pixels = generate_mono8(width, height, frame_id, brightness);
                frame_id = frame_id.wrapping_add(1);

                debug!("Publishing frame {frame_id}: {width}x{height}, brightness={brightness:.2}");
                let _ = session.put(&image_key, pixels).await;
            }
            _ = acq_rx.changed() => {
                // Acquisition state changed, loop will re-evaluate select conditions
            }
        }
    }
}
