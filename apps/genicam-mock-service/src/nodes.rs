use std::sync::Arc;

use tokio::sync::watch;
use tracing::{debug, error, info, warn};

use genicam_zenoh_api::{NodeOpResponse, NodeSetRequest};

use crate::config::MockConfig;
use crate::state::NodeStore;

/// Publish all initial node values, then broadcast changes as they happen.
pub async fn run_publisher(
    session: Arc<zenoh::Session>,
    config: MockConfig,
    store: Arc<NodeStore>,
    mut shutdown: watch::Receiver<bool>,
) {
    // Publish all initial values
    let all = store.all().await;
    for (name, update) in &all {
        let key = genicam_zenoh_api::keys::node_value(&config.device_id, name);
        let payload = match serde_json::to_vec(update) {
            Ok(p) => p,
            Err(e) => {
                warn!("Failed to serialize node {name}: {e}");
                continue;
            }
        };
        if let Err(e) = session.put(&key, payload).await {
            warn!("Failed to publish initial value for {name}: {e}");
        }
    }
    info!("Published {} initial node values", all.len());

    // Subscribe and forward changes
    let mut rx = store.subscribe();
    loop {
        tokio::select! {
            _ = shutdown.changed() => {
                if *shutdown.borrow() { break; }
            }
            result = rx.recv() => {
                match result {
                    Ok((name, update)) => {
                        let key = genicam_zenoh_api::keys::node_value(&config.device_id, &name);
                        let payload = match serde_json::to_vec(&update) {
                            Ok(p) => p,
                            Err(e) => {
                                warn!("Failed to serialize update for {name}: {e}");
                                continue;
                            }
                        };
                        debug!("Publishing node change: {name} = {:?}", update.value);
                        let _ = session.put(&key, payload).await;
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        warn!("Node publisher lagged by {n} messages");
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        break;
                    }
                }
            }
        }
    }
}

/// Handle node set requests via a wildcard queryable.
pub async fn run_set_queryable(
    session: Arc<zenoh::Session>,
    config: MockConfig,
    store: Arc<NodeStore>,
    mut shutdown: watch::Receiver<bool>,
) {
    let key_pattern = format!(
        "genicam/devices/{}/nodes/*/set",
        config.device_id
    );
    let queryable = match session.declare_queryable(&key_pattern).await {
        Ok(q) => q,
        Err(e) => {
            error!("Failed to declare set queryable: {e}");
            return;
        }
    };

    loop {
        tokio::select! {
            _ = shutdown.changed() => {
                if *shutdown.borrow() { break; }
            }
            query = queryable.recv_async() => {
                match query {
                    Ok(query) => {
                        let key_expr = query.key_expr().as_str().to_string();
                        // Extract node name from key: genicam/devices/{id}/nodes/{name}/set
                        let node_name = extract_node_name_from_key(&key_expr, "set");
                        let node_name = match node_name {
                            Some(n) => n,
                            None => {
                                warn!("Could not extract node name from key: {key_expr}");
                                let resp = NodeOpResponse { ok: false, error: Some("Invalid key".to_string()) };
                                let _ = query.reply(&key_expr, serde_json::to_vec(&resp).unwrap_or_default()).await;
                                continue;
                            }
                        };

                        let payload = query.payload()
                            .map(|p| p.to_bytes().to_vec())
                            .unwrap_or_default();

                        let request: Result<NodeSetRequest, _> = serde_json::from_slice(&payload);
                        let resp = match request {
                            Ok(req) => {
                                info!("Set {node_name} = {:?}", req.value);
                                match store.set(&node_name, req.value).await {
                                    Ok(_) => NodeOpResponse { ok: true, error: None },
                                    Err(e) => {
                                        warn!("Set {node_name} failed: {e}");
                                        NodeOpResponse { ok: false, error: Some(e) }
                                    }
                                }
                            }
                            Err(e) => {
                                warn!("Invalid set request for {node_name}: {e}");
                                NodeOpResponse { ok: false, error: Some(format!("Invalid request: {e}")) }
                            }
                        };
                        let _ = query.reply(&key_expr, serde_json::to_vec(&resp).unwrap_or_default()).await;
                    }
                    Err(e) => {
                        error!("Set queryable recv error: {e}");
                        break;
                    }
                }
            }
        }
    }
}

/// Handle command execute requests via a wildcard queryable.
pub async fn run_execute_queryable(
    session: Arc<zenoh::Session>,
    config: MockConfig,
    mut shutdown: watch::Receiver<bool>,
) {
    let key_pattern = format!(
        "genicam/devices/{}/nodes/*/execute",
        config.device_id
    );
    let queryable = match session.declare_queryable(&key_pattern).await {
        Ok(q) => q,
        Err(e) => {
            error!("Failed to declare execute queryable: {e}");
            return;
        }
    };

    loop {
        tokio::select! {
            _ = shutdown.changed() => {
                if *shutdown.borrow() { break; }
            }
            query = queryable.recv_async() => {
                match query {
                    Ok(query) => {
                        let key_expr = query.key_expr().as_str().to_string();
                        let node_name = extract_node_name_from_key(&key_expr, "execute")
                            .unwrap_or_else(|| "unknown".to_string());
                        info!("Execute command: {node_name}");
                        let resp = NodeOpResponse { ok: true, error: None };
                        let _ = query.reply(&key_expr, serde_json::to_vec(&resp).unwrap_or_default()).await;
                    }
                    Err(e) => {
                        error!("Execute queryable recv error: {e}");
                        break;
                    }
                }
            }
        }
    }
}

/// Extract node name from key expression like `genicam/devices/{id}/nodes/{name}/{suffix}`.
fn extract_node_name_from_key(key: &str, suffix: &str) -> Option<String> {
    let parts: Vec<&str> = key.split('/').collect();
    if parts.len() >= 6 && parts[parts.len() - 1] == suffix {
        Some(parts[parts.len() - 2].to_string())
    } else {
        None
    }
}
