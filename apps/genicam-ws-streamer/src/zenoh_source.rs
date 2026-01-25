//! Zenoh subscription loop that converts Mono8 frames into BMP bytes.
//!
//! The resulting BMP is pushed into a watch channel so the latest frame
//! overwrites older ones. This keeps memory bounded and provides
//! backpressure (slow clients just see the newest frame).

use std::time::{Duration, Instant};

use bytes::Bytes;
use tokio::sync::watch;
use tracing::{info, warn};

use crate::bmp::BmpEncoder;
use crate::error::StreamerError;

#[derive(Clone, Copy, Debug)]
pub enum PixelFormat {
    Mono8,
}

impl PixelFormat {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Mono8 => "Mono8",
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct StreamSpec {
    pub width: u32,
    pub height: u32,
    pub pixel_format: PixelFormat,
}

#[derive(Debug)]
pub struct FrameEncoder {
    spec: StreamSpec,
    bmp: BmpEncoder,
}

impl FrameEncoder {
    pub fn new(spec: StreamSpec) -> Self {
        let bmp = BmpEncoder::new(spec.width, spec.height);
        Self { spec, bmp }
    }

    pub fn expected_payload_len(&self) -> Result<usize, StreamerError> {
        (self.spec.width as usize)
            .checked_mul(self.spec.height as usize)
            .ok_or_else(|| StreamerError::Config("width*height overflows usize".into()))
    }

    pub fn encode(&self, pixels: &[u8]) -> Result<Bytes, StreamerError> {
        match self.spec.pixel_format {
            PixelFormat::Mono8 => Ok(self.bmp.encode_gray8(pixels)?),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ZenohSourceConfig {
    pub key_expr: String,
    pub fps_limit: Option<u32>,
}

pub async fn run(
    config: zenoh::Config,
    source: ZenohSourceConfig,
    encoder: FrameEncoder,
    frame_tx: watch::Sender<Bytes>,
    mut shutdown: watch::Receiver<bool>,
) -> Result<(), StreamerError> {
    let expected = encoder.expected_payload_len()?;
    let min_interval = source
        .fps_limit
        .map(|fps| Duration::from_secs_f64(1.0 / fps as f64));

    let session = zenoh::open(config).await?;
    let subscriber = session.declare_subscriber(&source.key_expr).await?;

    info!(
        "Subscribed to zenoh key '{}' (expected {} bytes per frame)",
        source.key_expr, expected
    );

    let mut last_emit: Option<Instant> = None;

    loop {
        tokio::select! {
            _ = shutdown.changed() => {
                if *shutdown.borrow() {
                    break;
                }
            }
            sample = subscriber.recv_async() => {
                let sample = match sample {
                    Ok(sample) => sample,
                    Err(err) => {
                        warn!("Zenoh subscriber closed: {err}");
                        break;
                    }
                };

                if let Some(min_interval) = min_interval {
                    if let Some(last_emit) = last_emit {
                        if last_emit.elapsed() < min_interval {
                            // Drop frames above the configured FPS.
                            continue;
                        }
                    }
                }

                let payload = sample.payload().to_bytes();
                if payload.len() != expected {
                    warn!(
                        "Dropped frame: payload size mismatch (expected {}, got {})",
                        expected,
                        payload.len()
                    );
                    continue;
                }

                let frame = encoder.encode(payload.as_ref())?;
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
