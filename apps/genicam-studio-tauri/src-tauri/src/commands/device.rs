use std::sync::Arc;

use tauri::{AppHandle, Emitter, State};
use tokio::sync::RwLock;

use crate::commands::xml_model::{ModelSummary, ParseXmlResponse};
use crate::state::device_state::{ConnectionState, DeviceInfo, NodeValueEntry, ZenohState};
use crate::state::ModelState;
use genicam_xml_model::parse_genicam_xml;
use genicam_zenoh_api::{DeviceAnnounce, DeviceXmlResponse, ImageMeta};

// ── IPC Commands ──────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn list_discovered_devices(
    zenoh: State<'_, Arc<ZenohState>>,
) -> Result<Vec<DeviceInfo>, String> {
    Ok(zenoh.registry.lock().await.list())
}

#[tauri::command]
pub async fn get_connection_state(
    zenoh: State<'_, Arc<ZenohState>>,
) -> Result<ConnectionState, String> {
    Ok(zenoh.connection.lock().await.clone())
}

#[tauri::command]
pub async fn connect_device(
    device_id: String,
    zenoh: State<'_, Arc<ZenohState>>,
    model: State<'_, RwLock<ModelState>>,
    app: AppHandle,
) -> Result<ParseXmlResponse, String> {
    let session = zenoh.get_session().await?;

    // Signal connecting
    *zenoh.connection.lock().await = ConnectionState::Connecting {
        device_id: device_id.clone(),
    };
    emit_connection_state(&app, &zenoh).await;

    // Fetch GenICam XML from the device service
    let xml = match fetch_device_xml(&session, &device_id).await {
        Ok(xml) => xml,
        Err(e) => {
            *zenoh.connection.lock().await = ConnectionState::Error { message: e.clone() };
            emit_connection_state(&app, &zenoh).await;
            return Err(e);
        }
    };

    // Parse the XML into a UiGraph
    let graph = parse_genicam_xml(&xml).map_err(|e| e.to_string())?;

    let summary = ModelSummary {
        node_count: graph.nodes_by_name.len(),
        category_count: graph.categories.len(),
        root_category: graph.root_category.clone(),
    };

    let response = ParseXmlResponse {
        graph: graph.clone(),
        xml: xml.clone(),
        diags: vec![],
        summary,
    };

    // Persist in model state so get_current_model() still works
    model
        .write()
        .await
        .update(xml, graph, vec![], Some(device_id.clone()));

    // Resolve display name from registry
    let (device_name, device_model) = {
        let registry = zenoh.registry.lock().await;
        (
            registry
                .get_name(&device_id)
                .unwrap_or_else(|| device_id.clone()),
            registry.get_model(&device_id).unwrap_or_default(),
        )
    };

    // Abort any stale subscription tasks and clear node cache
    abort_sub_tasks(&zenoh).await;
    zenoh.node_cache.write().await.clear();

    // Start per-connection subscriber tasks
    let z = zenoh.inner().clone();
    let tasks = vec![
        spawn_node_value_sub(session.clone(), device_id.clone(), z.clone(), app.clone()),
        spawn_status_sub(session.clone(), device_id.clone(), z.clone(), app.clone()),
        spawn_acq_status_sub(session.clone(), device_id.clone(), z.clone(), app.clone()),
        spawn_image_meta_sub(session.clone(), device_id.clone(), z.clone(), app.clone()),
    ];
    zenoh.sub_tasks.lock().await.extend(tasks);

    // Mark connected
    *zenoh.connection.lock().await = ConnectionState::Connected {
        device_id,
        device_name,
        model: device_model,
    };
    emit_connection_state(&app, &zenoh).await;

    Ok(response)
}

#[tauri::command]
pub async fn disconnect_device(
    zenoh: State<'_, Arc<ZenohState>>,
    app: AppHandle,
) -> Result<(), String> {
    abort_sub_tasks(&zenoh).await;
    stop_acquisition_child(&zenoh).await;
    zenoh.node_cache.write().await.clear();
    *zenoh.connection.lock().await = ConnectionState::Disconnected;
    emit_connection_state(&app, &zenoh).await;
    Ok(())
}

// ── Background task init (called from main.rs setup) ─────────────────────────

/// Start the global device-discovery subscriber.  Runs for the app lifetime.
pub fn start_discovery_task(state: Arc<ZenohState>, app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        run_discovery_loop(state, app).await;
    });
}

async fn run_discovery_loop(zenoh: Arc<ZenohState>, app: AppHandle) {
    let session = match zenoh.get_session().await {
        Ok(s) => s,
        Err(_) => return, // Zenoh not initialized — skip
    };

    let sub = match session
        .declare_subscriber(genicam_zenoh_api::keys::ANNOUNCE_ALL)
        .await
    {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Discovery subscriber error: {e}");
            return;
        }
    };

    let mut ticker = tokio::time::interval(std::time::Duration::from_secs(1));

    loop {
        tokio::select! {
            result = sub.recv_async() => {
                let sample = match result {
                    Ok(s) => s,
                    Err(_) => break,
                };
                let bytes = sample.payload().to_bytes();
                if let Ok(announce) = serde_json::from_slice::<DeviceAnnounce>(&bytes) {
                    let info = DeviceInfo {
                        id: announce.id.clone(),
                        name: announce.name,
                        model: announce.model,
                        serial: announce.serial,
                    };
                    let is_new = {
                        let mut registry = zenoh.registry.lock().await;
                        let is_new = !registry.contains(&announce.id);
                        registry.update(info.clone());
                        is_new
                    };
                    if is_new {
                        let _ = app.emit("device-discovered", &info);
                    }
                }
            }
            _ = ticker.tick() => {
                let expired = zenoh.registry.lock().await.expire_old(6);
                for id in expired {
                    let _ = app.emit("device-lost", serde_json::json!({ "device_id": id }));
                }
            }
        }
    }
}

// ── Subscription task spawners ───────────────────────────────────────────────

fn spawn_node_value_sub(
    session: Arc<zenoh::Session>,
    device_id: String,
    zenoh: Arc<ZenohState>,
    app: AppHandle,
) -> tauri::async_runtime::JoinHandle<()> {
    tauri::async_runtime::spawn(async move {
        let key = genicam_zenoh_api::keys::node_value_wildcard(&device_id);
        let sub = match session.declare_subscriber(&key).await {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Node value subscriber error: {e}");
                return;
            }
        };
        while let Ok(sample) = sub.recv_async().await {
            let bytes = sample.payload().to_bytes();
            if let Ok(update) = serde_json::from_slice::<genicam_zenoh_api::NodeValueUpdate>(&bytes)
            {
                let key_str = sample.key_expr().as_str();
                if let Some(node_name) = genicam_zenoh_api::keys::extract_node_name(key_str) {
                    let entry = NodeValueEntry {
                        value: update.value.clone(),
                        access_mode: update.access_mode.clone(),
                    };
                    zenoh
                        .node_cache
                        .write()
                        .await
                        .insert(node_name.to_string(), entry);
                    let _ = app.emit(
                        "node-value-changed",
                        serde_json::json!({
                            "node_name": node_name,
                            "value": update.value,
                            "access_mode": update.access_mode,
                        }),
                    );
                }
            }
        }
    })
}

fn spawn_status_sub(
    session: Arc<zenoh::Session>,
    device_id: String,
    zenoh: Arc<ZenohState>,
    app: AppHandle,
) -> tauri::async_runtime::JoinHandle<()> {
    tauri::async_runtime::spawn(async move {
        let key = genicam_zenoh_api::keys::status(&device_id);
        let sub = match session.declare_subscriber(&key).await {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Status subscriber error: {e}");
                return;
            }
        };
        while let Ok(sample) = sub.recv_async().await {
            let bytes = sample.payload().to_bytes();
            if let Ok(status) = serde_json::from_slice::<genicam_zenoh_api::DeviceStatus>(&bytes) {
                if !status.connected {
                    let msg = status
                        .error
                        .unwrap_or_else(|| "Device disconnected".to_string());
                    let new_state = ConnectionState::Error { message: msg };
                    *zenoh.connection.lock().await = new_state.clone();
                    let _ = app.emit("connection-state-changed", new_state);
                }
            }
        }
    })
}

fn spawn_acq_status_sub(
    session: Arc<zenoh::Session>,
    device_id: String,
    zenoh: Arc<ZenohState>,
    app: AppHandle,
) -> tauri::async_runtime::JoinHandle<()> {
    tauri::async_runtime::spawn(async move {
        let key = genicam_zenoh_api::keys::acquisition_status(&device_id);
        let sub = match session.declare_subscriber(&key).await {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Acquisition status subscriber error: {e}");
                return;
            }
        };
        while let Ok(sample) = sub.recv_async().await {
            let bytes = sample.payload().to_bytes();
            if let Ok(status) =
                serde_json::from_slice::<genicam_zenoh_api::AcquisitionStatus>(&bytes)
            {
                zenoh.acquisition.lock().await.status = status.clone();
                let _ = app.emit("acquisition-status", status);
            }
        }
    })
}

fn spawn_image_meta_sub(
    session: Arc<zenoh::Session>,
    device_id: String,
    zenoh: Arc<ZenohState>,
    app: AppHandle,
) -> tauri::async_runtime::JoinHandle<()> {
    tauri::async_runtime::spawn(async move {
        let key = genicam_zenoh_api::keys::image_meta(&device_id);
        let sub = match session.declare_subscriber(&key).await {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Image meta subscriber error: {e}");
                return;
            }
        };
        while let Ok(sample) = sub.recv_async().await {
            let bytes = sample.payload().to_bytes();
            if let Ok(meta) = serde_json::from_slice::<ImageMeta>(&bytes) {
                zenoh.acquisition.lock().await.image_meta = Some(meta.clone());
                let _ = app.emit("image-meta-changed", meta);
            }
        }
    })
}

// ── Helpers ───────────────────────────────────────────────────────────────────

async fn emit_connection_state(app: &AppHandle, zenoh: &ZenohState) {
    let state = zenoh.connection.lock().await.clone();
    let _ = app.emit("connection-state-changed", state);
}

async fn abort_sub_tasks(zenoh: &ZenohState) {
    let mut tasks = zenoh.sub_tasks.lock().await;
    for t in tasks.drain(..) {
        t.abort();
    }
}

pub async fn stop_acquisition_child(zenoh: &ZenohState) {
    let mut acq = zenoh.acquisition.lock().await;
    // Signal the monitor task to kill the child and exit.
    if let Some(tx) = acq.stop_tx.take() {
        let _ = tx.send(true);
        // Drop `tx` so the monitor task sees the channel close even if send was missed.
    }
    // Drop the handle (task will complete on its own after receiving the stop signal).
    acq.monitor_handle.take();
    acq.ws_url = None;
    acq.status.active = false;
}

async fn fetch_device_xml(session: &zenoh::Session, device_id: &str) -> Result<String, String> {
    let key = genicam_zenoh_api::keys::xml(device_id);
    let replies = session
        .get(&key)
        .await
        .map_err(|e| format!("Zenoh GET error: {e}"))?;

    match replies.recv_async().await {
        Ok(reply) => match reply.result() {
            Ok(sample) => {
                let bytes = sample.payload().to_bytes();
                let resp: DeviceXmlResponse = serde_json::from_slice(&bytes)
                    .map_err(|e| format!("Invalid XML response: {e}"))?;
                Ok(resp.xml)
            }
            Err(e) => Err(format!("Reply error: {e}")),
        },
        Err(_) => Err("No reply for XML request (timeout)".to_string()),
    }
}
