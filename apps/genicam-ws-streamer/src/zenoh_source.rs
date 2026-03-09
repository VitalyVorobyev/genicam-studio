//! Zenoh subscription loop that converts frames into BMP bytes.
//!
//! Two subscribers run concurrently inside a single `select!` loop:
//!
//! - **meta subscriber** — listens on `image/meta`; updates the shared `Arc<RwLock<ImageMeta>>`
//!   and rebuilds the `BmpEncoder` when dimensions change. Retained for the Tauri app's
//!   `image-meta-changed` event path.
//! - **frame subscriber** — receives framed payloads (`16-byte FrameHeader + raw pixels`),
//!   decodes the inline header, validates payload size, encodes to BMP, and pushes into the
//!   watch channel (latest frame overwrites older ones — backpressure).
//!
//! Because every frame carries an inline `FrameHeader`, the streamer no longer depends on
//! `image/meta` subscription ordering to know the format or dimensions of a frame.

use std::sync::Arc;
use std::time::{Duration, Instant};

use bytes::Bytes;
use genicam_zenoh_api::{FrameHeader, ImageMeta, PixelFormat};
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
    info_tx: watch::Sender<crate::ws::StreamInfo>,
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

                let stream_info = crate::ws::StreamInfo::from_image_meta(&meta);
                *shared_meta.write().await = meta;

                if info_tx.send(stream_info).is_err() {
                    warn!("No WS clients subscribed to info updates");
                }
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

                let raw = sample.payload().to_bytes();

                // Decode the inline frame header. Frames without a valid
                // header (e.g. from an older service version) are dropped with
                // a warning rather than treated as fatal errors.
                let (header, pixel_data) = match FrameHeader::decode(raw.as_ref()) {
                    Ok(pair) => pair,
                    Err(err) => {
                        warn!("Dropped frame: invalid frame header: {err}");
                        continue;
                    }
                };

                // Validate that the pixel data length matches what the header
                // claims, using bytes_per_pixel for the declared format.
                let bpp = header.pixel_format.bytes_per_pixel();
                let expected_pixels = (header.width as usize)
                    .checked_mul(header.height as usize)
                    .ok_or_else(|| StreamerError::Config("width*height overflows usize".into()))?;
                // Truncate fractional bpp to whole bytes (e.g. YCbCr422 = 2.0).
                let expected_bytes = (expected_pixels as f32 * bpp) as usize;

                if pixel_data.len() != expected_bytes {
                    warn!(
                        "Dropped frame seq={}: pixel data size mismatch (expected {}, got {})",
                        header.seq,
                        expected_bytes,
                        pixel_data.len()
                    );
                    continue;
                }

                // Rebuild the BMP encoder when dimensions change.
                {
                    let old = shared_meta.read().await;
                    if old.width != header.width || old.height != header.height {
                        info!(
                            "Frame header dimensions changed to {}x{}; rebuilding BMP encoder",
                            header.width, header.height
                        );
                        encoder = BmpEncoder::new(header.width, header.height);
                    }
                }

                if header.pixel_format != PixelFormat::Mono8 {
                    warn!(
                        "Encoding non-Mono8 frame seq={} ({:?}) as Mono8; output may be incorrect until ST-02",
                        header.seq, header.pixel_format
                    );
                }

                let frame = encoder.encode_gray8(pixel_data)?;

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
