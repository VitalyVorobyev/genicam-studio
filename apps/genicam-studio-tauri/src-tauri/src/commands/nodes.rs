use std::collections::HashMap;
use std::sync::Arc;

use tauri::State;

use crate::state::device_state::{ConnectionState, NodeValueEntry, ZenohState};
use genicam_zenoh_api::{BulkReadRequest, BulkReadResponse, NodeOpResponse, NodeSetRequest};

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

/// Parse raw bytes from a `nodes/bulk/read` reply into a `NodeValueEntry` map.
///
/// Extracted as a pure function so it can be unit-tested without a Zenoh session.
fn parse_bulk_response(bytes: &[u8]) -> Result<HashMap<String, NodeValueEntry>, String> {
    let resp = serde_json::from_slice::<BulkReadResponse>(bytes)
        .map_err(|e| format!("Parse error: {e}"))?;
    Ok(resp
        .values
        .into_iter()
        .map(|(k, v)| {
            (
                k,
                NodeValueEntry {
                    value: v.value,
                    access_mode: v.access_mode,
                },
            )
        })
        .collect())
}

#[tauri::command]
pub async fn read_nodes_bulk(
    names: Vec<String>,
    zenoh: State<'_, Arc<ZenohState>>,
) -> Result<HashMap<String, NodeValueEntry>, String> {
    let session = zenoh.get_session().await?;
    let device_id = connected_device_id(&zenoh).await?;

    let key = genicam_zenoh_api::keys::nodes_bulk_read(&device_id);
    let payload = serde_json::to_vec(&BulkReadRequest { names }).map_err(|e| e.to_string())?;

    let replies = session
        .get(&key)
        .payload(payload)
        .await
        .map_err(|e| format!("Zenoh error: {e}"))?;

    match replies.recv_async().await {
        Ok(reply) => match reply.result() {
            Ok(sample) => {
                let bytes = sample.payload().to_bytes();
                parse_bulk_response(&bytes)
            }
            Err(e) => Err(format!("Reply error: {e}")),
        },
        Err(_) => Err("No reply for read_nodes_bulk (timeout)".to_string()),
    }
}

async fn connected_device_id(zenoh: &ZenohState) -> Result<String, String> {
    match zenoh.connection.lock().await.clone() {
        ConnectionState::Connected { device_id, .. } => Ok(device_id),
        _ => Err("Not connected to a device".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::parse_bulk_response;

    #[test]
    fn test_parse_bulk_response_happy_path() {
        let json = r#"{
            "values": {
                "Width": {"value": 1920, "access_mode": "RW"},
                "Height": {"value": 1080, "access_mode": "RO"}
            }
        }"#;
        let result = parse_bulk_response(json.as_bytes()).expect("should parse successfully");
        assert_eq!(result.len(), 2);
        let width = result.get("Width").expect("Width should be present");
        assert_eq!(width.value, serde_json::json!(1920));
        assert_eq!(width.access_mode, "RW");
        let height = result.get("Height").expect("Height should be present");
        assert_eq!(height.value, serde_json::json!(1080));
        assert_eq!(height.access_mode, "RO");
    }

    #[test]
    fn test_parse_bulk_response_empty() {
        let json = r#"{"values": {}}"#;
        let result =
            parse_bulk_response(json.as_bytes()).expect("empty map should parse without error");
        assert!(result.is_empty());
    }

    #[test]
    fn test_parse_bulk_response_invalid_json() {
        let err = parse_bulk_response(b"not json").expect_err("invalid JSON should return Err");
        assert!(
            err.contains("Parse error"),
            "error message should contain 'Parse error', got: {err}"
        );
    }
}
