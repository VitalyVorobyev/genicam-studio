//! Zenoh subscription loop that converts Mono8 frames into BMP bytes.
//!
//! Two subscribers run concurrently inside a single `select!` loop:
//!
//! - **meta subscriber** — listens on `image/meta`; updates the shared
//!   `Arc<RwLock<ImageMeta>>` and rebuilds the `BmpEncoder` when dimensions
//!   change.
//! - **frame subscriber** — receives raw pixel payloads, validates size using
//!   the current meta, encodes to BMP, and pushes into the watch channel so
//!   the latest frame overwrites older ones (backpressure).

use std::sync::Arc;
use std::time::{Duration, Instant};

use bytes::Bytes;
use genicam_zenoh_api::{ImageMeta, PixelFormat};
use tokio::sync::{watch, RwLock};
use tracing::{info, warn};

use crate::bmp::BmpEncoder;
use crate::error::StreamerError;

#[derive(Clone, Debug)]
pub struct ZenohSourceConfig {
    pub key_expr: String,
    pub meta_key: String,
    pub fps_limit: Option<u32>,
}

pub async fn run(
    config: zenoh::Config,
    source: ZenohSourceConfig,
    shared_meta: Arc<RwLock<ImageMeta>>,
    frame_tx: watch::Sender<Bytes>,
    mut shutdown: watch::Receiver<bool>,
) -> Result<(), StreamerError> {
    let min_interval = source
        .fps_limit
        .map(|fps| Duration::from_secs_f64(1.0 / fps as f64));

    let session = zenoh::open(config).await?;
    let frame_sub = session.declare_subscriber(&source.key_expr).await?;
    let meta_sub = session.declare_subscriber(&source.meta_key).await?;

    // Cache the current dimensions so we can detect changes and rebuild the
    // encoder only when necessary.
    let (init_width, init_height) = {
        let m = shared_meta.read().await;
        (m.width, m.height)
    };
    let mut encoder = BmpEncoder::new(init_width, init_height);

    info!(
        "Subscribed to '{}' (meta: '{}')",
        source.key_expr, source.meta_key
    );

    let mut last_emit: Option<Instant> = None;

    loop {
        tokio::select! {
            _ = shutdown.changed() => {
                if *shutdown.borrow() {
                    break;
                }
            }

            // ── Meta subscriber ───────────────────────────────────────────
            sample = meta_sub.recv_async() => {
                let sample = match sample {
                    Ok(s) => s,
                    Err(err) => {
                        warn!("Meta subscriber closed: {err}");
                        break;
                    }
                };

                let payload = sample.payload().to_bytes();
                let meta: ImageMeta = match crate::meta::parse_meta_payload(&payload) {
                    Ok(m) => m,
                    Err(err) => {
                        warn!("Failed to parse image/meta payload: {err}");
                        continue;
                    }
                };

                if meta.pixel_format != PixelFormat::Mono8 {
                    warn!(
                        "Received pixel_format != Mono8 ({:?}); encoding as Mono8 anyway (ST-02 will handle multi-format)",
                        meta.pixel_format
                    );
                }

                // Rebuild the encoder only when dimensions change.
                // Safety: read-then-write is not atomic, but the select! loop is
                // single-threaded — no concurrent writers exist while this arm executes.
                // If this loop is ever split into separate tasks, consolidate into one
                // write-lock acquisition.
                let needs_rebuild = {
                    let old = shared_meta.read().await;
                    old.width != meta.width || old.height != meta.height
                };

                if needs_rebuild {
                    info!(
                        "Dimensions changed to {}x{}; rebuilding BMP encoder",
                        meta.width, meta.height
                    );
                    encoder = BmpEncoder::new(meta.width, meta.height);
                }

                *shared_meta.write().await = meta;
            }

            // ── Frame subscriber ──────────────────────────────────────────
            sample = frame_sub.recv_async() => {
                let sample = match sample {
                    Ok(s) => s,
                    Err(err) => {
                        warn!("Frame subscriber closed: {err}");
                        break;
                    }
                };

                if let Some(interval) = min_interval {
                    if let Some(last) = last_emit {
                        if last.elapsed() < interval {
                            // Drop frames above the configured FPS.
                            continue;
                        }
                    }
                }

                let (expected, px_format) = {
                    let m = shared_meta.read().await;
                    let expected = (m.width as usize)
                        .checked_mul(m.height as usize)
                        .ok_or_else(|| StreamerError::Config("width*height overflows usize".into()))?;
                    (expected, m.pixel_format.clone())
                };

                let payload = sample.payload().to_bytes();
                if payload.len() != expected {
                    warn!(
                        "Dropped frame: payload size mismatch (expected {}, got {})",
                        expected,
                        payload.len()
                    );
                    continue;
                }

                if px_format != PixelFormat::Mono8 {
                    warn!(
                        "Encoding non-Mono8 frame ({:?}) as Mono8; output may be incorrect until ST-02",
                        px_format
                    );
                }

                let frame = encoder.encode_gray8(payload.as_ref())?;

                if frame_tx.send(frame).is_err() {
                    warn!("No WebSocket clients are listening for frames");
                }
                last_emit = Some(Instant::now());
            }
        }
    }

    if let Err(err) = session.close().await {
        warn!("Failed to close zenoh session cleanly: {err}");
    }

    Ok(())
}
