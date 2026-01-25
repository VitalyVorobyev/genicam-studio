//! WebSocket server that fans out the latest BMP frame to all clients.

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
    routing::get,
    Router,
};
use bytes::Bytes;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tokio::sync::watch;
use tracing::{info, warn};

use crate::error::StreamerError;

#[derive(Clone, Debug)]
pub struct StreamInfo {
    pub width: u32,
    pub height: u32,
    pub pixel_format: &'static str,
    pub encoding: &'static str,
}

impl StreamInfo {
    pub fn mono8_bmp(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            pixel_format: "Mono8",
            encoding: "BMP",
        }
    }

    pub fn to_json(&self) -> String {
        format!(
            r#"{{"type":"info","width":{},"height":{},"pixel_format":"{}","encoding":"{}"}}"#,
            self.width, self.height, self.pixel_format, self.encoding
        )
    }
}

#[derive(Clone)]
pub struct AppState {
    pub frame_tx: watch::Sender<Bytes>,
    pub info: StreamInfo,
}

pub async fn run_server(
    bind: SocketAddr,
    path: String,
    state: AppState,
    mut shutdown: watch::Receiver<bool>,
) -> Result<(), StreamerError> {
    let app = Router::new()
        .route(&path, get(ws_handler))
        .with_state(state);

    let listener = TcpListener::bind(bind).await?;
    info!("WebSocket server listening on ws://{bind}{path}");

    let shutdown_signal = async move {
        loop {
            if *shutdown.borrow() {
                break;
            }
            if shutdown.changed().await.is_err() {
                break;
            }
        }
    };

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal)
        .await
        .map_err(|err| StreamerError::Server(err.to_string()))?;

    Ok(())
}

async fn ws_handler(State(state): State<AppState>, upgrade: WebSocketUpgrade) -> impl IntoResponse {
    upgrade.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: AppState) {
    let mut rx = state.frame_tx.subscribe();
    let info_json = state.info.to_json();

    if socket.send(Message::Text(info_json)).await.is_err() {
        return;
    }

    // If a frame is already available, send it immediately.
    let initial = rx.borrow().clone();
    if !initial.is_empty() {
        if socket
            .send(Message::Binary(initial.to_vec()))
            .await
            .is_err()
        {
            return;
        }
    }

    loop {
        if rx.changed().await.is_err() {
            break;
        }
        let frame = rx.borrow().clone();
        if frame.is_empty() {
            continue;
        }
        if socket.send(Message::Binary(frame.to_vec())).await.is_err() {
            warn!("WebSocket client disconnected");
            break;
        }
    }
}
