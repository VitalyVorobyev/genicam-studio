use std::sync::Arc;

use tauri::State;

use crate::state::device_state::{ConnectionState, NodeValueEntry, ZenohState};
use genicam_zenoh_api::{NodeOpResponse, NodeSetRequest};

#[tauri::command]
pub async fn get_node_value(
    node_name: String,
    zenoh: State<'_, Arc<ZenohState>>,
) -> Result<NodeValueEntry, String> {
    zenoh
        .node_cache
        .read()
        .await
        .get(&node_name)
        .cloned()
        .ok_or_else(|| format!("Node '{node_name}' not in cache"))
}

#[tauri::command]
pub async fn write_node(
    node_name: String,
    value: serde_json::Value,
    zenoh: State<'_, Arc<ZenohState>>,
) -> Result<(), String> {
    let session = zenoh.get_session().await?;
    let device_id = connected_device_id(&zenoh).await?;

    let key = genicam_zenoh_api::keys::node_set(&device_id, &node_name);
    let payload = serde_json::to_vec(&NodeSetRequest { value }).map_err(|e| e.to_string())?;

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
                    Err(resp.error.unwrap_or_else(|| "Write failed".to_string()))
                }
            }
            Err(e) => Err(format!("Reply error: {e}")),
        },
        Err(_) => Err(format!("No reply for write_node '{node_name}' (timeout)")),
    }
}

#[tauri::command]
pub async fn execute_command(
    node_name: String,
    zenoh: State<'_, Arc<ZenohState>>,
) -> Result<(), String> {
    let session = zenoh.get_session().await?;
    let device_id = connected_device_id(&zenoh).await?;

    let key = genicam_zenoh_api::keys::node_execute(&device_id, &node_name);

    let replies = session
        .get(&key)
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
                    Err(resp.error.unwrap_or_else(|| "Execute failed".to_string()))
                }
            }
            Err(e) => Err(format!("Reply error: {e}")),
        },
        Err(_) => Err(format!(
            "No reply for execute_command '{node_name}' (timeout)"
        )),
    }
}

async fn connected_device_id(zenoh: &ZenohState) -> Result<String, String> {
    match zenoh.connection.lock().await.clone() {
        ConnectionState::Connected { device_id, .. } => Ok(device_id),
        _ => Err("Not connected to a device".to_string()),
    }
}
