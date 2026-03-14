use std::collections::HashMap;
use std::sync::Arc;

use tauri::State;
use tokio::sync::RwLock;

use crate::state::device_state::{ConnectionState, NodeValueEntry, ZenohState};
use crate::state::ModelState;
use genicam_xml_model::{UiNode, UiNodeKind};
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

/// Validate that `value` is a legal write for `node`, using runtime constraints from `live`
/// (ZA-06) with fallback to static XML constraints.
///
/// Returns `Ok(())` when the write is valid, or an `Err` with a human-readable message.
fn validate_node_write(
    node: &UiNode,
    live: Option<&NodeValueEntry>,
    value: &serde_json::Value,
) -> Result<(), String> {
    // Access-mode check: prefer live cache, fall back to static XML declaration.
    let access_mode = live
        .map(|e| e.access_mode.as_str())
        .or(node.access_mode.as_deref())
        .unwrap_or("RW");

    if access_mode == "RO" || access_mode == "NA" {
        return Err(format!(
            "Node '{}' is not writable (access_mode={access_mode})",
            node.name
        ));
    }

    match &node.kind {
        UiNodeKind::Integer => {
            let n = value
                .as_f64()
                .ok_or_else(|| format!("Node '{}' requires an integer value", node.name))?;
            let min = live
                .and_then(|e| e.min)
                .or_else(|| node.constraints.as_ref().and_then(|c| c.min));
            let max = live
                .and_then(|e| e.max)
                .or_else(|| node.constraints.as_ref().and_then(|c| c.max));
            let inc = live
                .and_then(|e| e.inc)
                .or_else(|| node.constraints.as_ref().and_then(|c| c.inc));
            validate_numeric_constraints(min, max, inc, n, &node.name)?;
        }
        UiNodeKind::Float => {
            let n = value
                .as_f64()
                .ok_or_else(|| format!("Node '{}' requires a numeric value", node.name))?;
            let min = live
                .and_then(|e| e.min)
                .or_else(|| node.constraints.as_ref().and_then(|c| c.min));
            let max = live
                .and_then(|e| e.max)
                .or_else(|| node.constraints.as_ref().and_then(|c| c.max));
            let inc = live
                .and_then(|e| e.inc)
                .or_else(|| node.constraints.as_ref().and_then(|c| c.inc));
            validate_numeric_constraints(min, max, inc, n, &node.name)?;
        }
        UiNodeKind::Boolean => {
            if !value.is_boolean() {
                return Err(format!(
                    "Node '{}' requires a boolean value (true/false)",
                    node.name
                ));
            }
        }
        UiNodeKind::String => {
            if !value.is_string() {
                return Err(format!("Node '{}' requires a string value", node.name));
            }
        }
        UiNodeKind::Enumeration => {
            let s = value
                .as_str()
                .ok_or_else(|| format!("Node '{}' requires a string enum entry", node.name))?;
            if !node.enum_entries.is_empty() && !node.enum_entries.iter().any(|e| e.name == s) {
                return Err(format!(
                    "'{}' is not a valid entry for enumeration node '{}'",
                    s, node.name
                ));
            }
        }
        UiNodeKind::Command => {
            return Err(format!(
                "Node '{}' is a Command node; use execute_command instead of write_node",
                node.name
            ));
        }
        UiNodeKind::Category | UiNodeKind::Register | UiNodeKind::Unknown { .. } => {
            return Err(format!(
                "Node '{}' (kind {:?}) is not writable",
                node.name, node.kind
            ));
        }
    }

    Ok(())
}

/// Check that `n` satisfies optional min/max/inc constraints.
fn validate_numeric_constraints(
    min: Option<f64>,
    max: Option<f64>,
    inc: Option<f64>,
    n: f64,
    node_name: &str,
) -> Result<(), String> {
    if let Some(min) = min {
        if n < min {
            return Err(format!(
                "Value {n} is below minimum {min} for node '{node_name}'"
            ));
        }
    }
    if let Some(max) = max {
        if n > max {
            return Err(format!(
                "Value {n} exceeds maximum {max} for node '{node_name}'"
            ));
        }
    }
    if let Some(inc) = inc {
        if inc > 0.0 {
            let base = min.unwrap_or(0.0);
            let remainder = ((n - base) / inc).fract().abs();
            // Allow a small floating-point epsilon on either side of an integer step.
            if remainder > 1e-9 && (1.0 - remainder) > 1e-9 {
                return Err(format!(
                    "Value {n} is not aligned to increment {inc} for node '{node_name}'"
                ));
            }
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn write_node(
    node_name: String,
    value: serde_json::Value,
    zenoh: State<'_, Arc<ZenohState>>,
    model: State<'_, RwLock<ModelState>>,
) -> Result<(), String> {
    // Pre-flight: validate against UiGraph constraints when a model is loaded.
    // If no model is present (e.g., pure Zenoh mode without XML) we skip silently.
    {
        let model_guard = model.read().await;
        if let Some(graph) = &model_guard.graph {
            if let Some(node) = graph.nodes_by_name.get(&node_name) {
                let cache = zenoh.node_cache.read().await;
                let live = cache.get(&node_name);
                validate_node_write(node, live, &value)?;
            }
        }
    }

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
                    min: v.min,
                    max: v.max,
                    inc: v.inc,
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
    use super::{parse_bulk_response, validate_node_write, validate_numeric_constraints};
    use crate::state::device_state::NodeValueEntry;
    use genicam_xml_model::{EnumEntry, NumericConstraints, RawNode, UiNode, UiNodeKind};
    use std::collections::HashMap;

    fn raw_node(tag: &str) -> RawNode {
        RawNode {
            tag: tag.to_string(),
            attributes: HashMap::new(),
            children_text: HashMap::new(),
        }
    }

    fn integer_node(name: &str, min: Option<f64>, max: Option<f64>, inc: Option<f64>) -> UiNode {
        UiNode {
            name: name.to_string(),
            kind: UiNodeKind::Integer,
            display_name: None,
            comment: None,
            tooltip: None,
            description: None,
            visibility: None,
            access_mode: Some("RW".to_string()),
            unit: None,
            representation: None,
            constraints: Some(NumericConstraints {
                min,
                max,
                inc,
                value: None,
            }),
            enum_entries: vec![],
            raw: raw_node("Integer"),
        }
    }

    fn float_node(name: &str) -> UiNode {
        UiNode {
            name: name.to_string(),
            kind: UiNodeKind::Float,
            display_name: None,
            comment: None,
            tooltip: None,
            description: None,
            visibility: None,
            access_mode: Some("RW".to_string()),
            unit: None,
            representation: None,
            constraints: Some(NumericConstraints {
                min: Some(0.0),
                max: Some(100.0),
                inc: None,
                value: None,
            }),
            enum_entries: vec![],
            raw: raw_node("Float"),
        }
    }

    fn bool_node(name: &str) -> UiNode {
        UiNode {
            name: name.to_string(),
            kind: UiNodeKind::Boolean,
            display_name: None,
            comment: None,
            tooltip: None,
            description: None,
            visibility: None,
            access_mode: Some("RW".to_string()),
            unit: None,
            representation: None,
            constraints: None,
            enum_entries: vec![],
            raw: raw_node("Boolean"),
        }
    }

    fn enum_node(name: &str, entries: &[&str]) -> UiNode {
        UiNode {
            name: name.to_string(),
            kind: UiNodeKind::Enumeration,
            display_name: None,
            comment: None,
            tooltip: None,
            description: None,
            visibility: None,
            access_mode: Some("RW".to_string()),
            unit: None,
            representation: None,
            constraints: None,
            enum_entries: entries
                .iter()
                .map(|e| EnumEntry {
                    name: e.to_string(),
                    value: None,
                    display_name: None,
                })
                .collect(),
            raw: raw_node("Enumeration"),
        }
    }

    // ── validate_numeric_constraints ────────────────────────────────────────────

    #[test]
    fn test_validate_numeric_constraints_no_constraints() {
        assert!(validate_numeric_constraints(None, None, None, 999.0, "X").is_ok());
    }

    #[test]
    fn test_validate_numeric_constraints_inc_violation() {
        // min=0, max=100, inc=8 — value 10 is not on the grid (0,8,16,…)
        let err = validate_numeric_constraints(Some(0.0), Some(100.0), Some(8.0), 10.0, "Width")
            .expect_err("should reject off-grid value");
        assert!(
            err.contains("increment"),
            "error should mention increment: {err}"
        );
    }

    #[test]
    fn test_validate_numeric_constraints_inc_valid() {
        // 16 is on the 0+8k grid
        assert!(
            validate_numeric_constraints(Some(0.0), Some(100.0), Some(8.0), 16.0, "Width").is_ok()
        );
    }

    // ── validate_node_write — Integer ───────────────────────────────────────────

    #[test]
    fn test_validate_node_write_integer_valid() {
        let node = integer_node("Width", Some(1.0), Some(4096.0), Some(1.0));
        assert!(validate_node_write(&node, None, &serde_json::json!(512)).is_ok());
    }

    #[test]
    fn test_validate_node_write_integer_above_max() {
        let node = integer_node("Width", Some(1.0), Some(4096.0), None);
        let err = validate_node_write(&node, None, &serde_json::json!(5000))
            .expect_err("should reject value above max");
        assert!(err.contains("maximum"), "error: {err}");
    }

    #[test]
    fn test_validate_node_write_integer_below_min() {
        let node = integer_node("Width", Some(1.0), Some(4096.0), None);
        let err = validate_node_write(&node, None, &serde_json::json!(0))
            .expect_err("should reject value below min");
        assert!(err.contains("minimum"), "error: {err}");
    }

    #[test]
    fn test_validate_node_write_integer_wrong_type() {
        let node = integer_node("Width", None, None, None);
        let err = validate_node_write(&node, None, &serde_json::json!("hello"))
            .expect_err("should reject string for Integer");
        assert!(err.contains("integer"), "error: {err}");
    }

    #[test]
    fn test_validate_node_write_float_valid_with_runtime_constraints() {
        let node = float_node("ExposureTime");
        // Runtime constraints (ZA-06) tighten the range to 100–500 µs.
        let live = NodeValueEntry {
            value: serde_json::json!(200.0),
            access_mode: "RW".to_string(),
            min: Some(100.0),
            max: Some(500.0),
            inc: None,
        };
        assert!(validate_node_write(&node, Some(&live), &serde_json::json!(300.0)).is_ok());
        let err = validate_node_write(&node, Some(&live), &serde_json::json!(50.0))
            .expect_err("runtime min should block 50.0");
        assert!(err.contains("minimum"), "error: {err}");
    }

    // ── validate_node_write — Boolean ───────────────────────────────────────────

    #[test]
    fn test_validate_node_write_boolean_wrong_type() {
        let node = bool_node("ReverseX");
        let err = validate_node_write(&node, None, &serde_json::json!("true"))
            .expect_err("string is not a bool");
        assert!(err.contains("boolean"), "error: {err}");
    }

    // ── validate_node_write — Enumeration ───────────────────────────────────────

    #[test]
    fn test_validate_node_write_enumeration_valid() {
        let node = enum_node("PixelFormat", &["Mono8", "Mono16", "RGB8"]);
        assert!(validate_node_write(&node, None, &serde_json::json!("Mono16")).is_ok());
    }

    #[test]
    fn test_validate_node_write_enumeration_invalid_entry() {
        let node = enum_node("PixelFormat", &["Mono8", "Mono16", "RGB8"]);
        let err = validate_node_write(&node, None, &serde_json::json!("JPEG"))
            .expect_err("unknown entry should fail");
        assert!(err.contains("JPEG"), "error: {err}");
    }

    // ── validate_node_write — access mode ───────────────────────────────────────

    #[test]
    fn test_validate_node_write_readonly_access_mode() {
        let mut node = integer_node("TemperatureAbs", None, None, None);
        node.access_mode = Some("RO".to_string());
        let err = validate_node_write(&node, None, &serde_json::json!(42))
            .expect_err("RO node should be rejected");
        assert!(err.contains("not writable"), "error: {err}");
    }

    // ── validate_node_write — Command ───────────────────────────────────────────

    #[test]
    fn test_validate_node_write_command_rejected() {
        let node = UiNode {
            name: "AcquisitionStart".to_string(),
            kind: UiNodeKind::Command,
            display_name: None,
            comment: None,
            tooltip: None,
            description: None,
            visibility: None,
            access_mode: Some("WO".to_string()),
            unit: None,
            representation: None,
            constraints: None,
            enum_entries: vec![],
            raw: raw_node("Command"),
        };
        let err = validate_node_write(&node, None, &serde_json::json!(null))
            .expect_err("Command node should be rejected");
        assert!(
            err.contains("execute_command"),
            "error should mention execute_command: {err}"
        );
    }

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
