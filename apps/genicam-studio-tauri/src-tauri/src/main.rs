#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod error;
mod state;

use std::sync::Arc;
use tokio::sync::RwLock;
use tracing_subscriber::prelude::*;

#[tauri::command]
fn ping() -> &'static str {
    "pong"
}

/// Load Zenoh configuration, checking these sources in order:
/// 1. `ZENOH_CONFIG` environment variable (path to a JSON5 config file)
/// 2. `config/zenoh-studio.json5` relative to the workspace root (dev mode)
/// 3. Default config (multicast scouting — works on Linux, often fails on macOS)
fn load_zenoh_config() -> zenoh::Config {
    // 1. Env var
    if let Ok(path) = std::env::var("ZENOH_CONFIG") {
        match zenoh::Config::from_file(&path) {
            Ok(cfg) => {
                tracing::info!("Loaded Zenoh config from ZENOH_CONFIG={path}");
                return cfg;
            }
            Err(e) => tracing::warn!("Failed to load ZENOH_CONFIG={path}: {e}"),
        }
    }

    // 2. Dev-mode config adjacent to the workspace
    let dev_candidates = [
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../../config/zenoh-studio.json5"
        ),
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../config/zenoh-studio.json5"
        ),
    ];
    for candidate in &dev_candidates {
        if std::path::Path::new(candidate).exists() {
            match zenoh::Config::from_file(candidate) {
                Ok(cfg) => {
                    tracing::info!("Loaded Zenoh config from {candidate}");
                    return cfg;
                }
                Err(e) => tracing::warn!("Failed to load {candidate}: {e}"),
            }
        }
    }

    tracing::info!("Using default Zenoh config (multicast scouting)");
    zenoh::Config::default()
}

fn dirs_log_path() -> std::path::PathBuf {
    let base = dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
    let log_dir = base.join(".genicam-studio").join("logs");
    std::fs::create_dir_all(&log_dir).ok();
    log_dir
}

fn main() {
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "genicam_studio=info,warn".into());

    // File appender: daily rotation in ~/.genicam-studio/logs/
    let log_dir = dirs_log_path();
    let file_appender = tracing_appender::rolling::daily(&log_dir, "studio.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::registry()
        .with(env_filter)
        .with(tracing_subscriber::fmt::layer().with_writer(std::io::stderr))
        .with(
            tracing_subscriber::fmt::layer()
                .with_ansi(false)
                .with_writer(non_blocking),
        )
        .init();

    tracing::info!("Log directory: {}", log_dir.display());
    // ModelState: holds the last parsed UiGraph (used by xml_model commands).
    let model_state = RwLock::new(state::ModelState::default());

    // ZenohState: device discovery, connection, node cache, acquisition.
    // Wrapped in Arc so background tasks can hold a clone independently.
    let zenoh_state = Arc::new(state::ZenohState::new());
    let zenoh_for_setup = zenoh_state.clone();

    // SfncGroupsState: cached SFNC group config loaded once from the bundled JSON.
    let sfnc_groups_state: commands::sfnc_groups::SfncGroupsState =
        Arc::new(RwLock::new(None::<Vec<commands::sfnc_groups::SfncGroup>>));

    if let Err(err) = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .manage(model_state)
        .manage(zenoh_state)
        .manage(sfnc_groups_state)
        .setup(move |app| {
            let app_handle = app.handle().clone();
            let zenoh = zenoh_for_setup.clone();

            // Initialize Zenoh and start the device-discovery background task.
            tauri::async_runtime::spawn(async move {
                let zenoh_config = load_zenoh_config();
                match zenoh::open(zenoh_config).await {
                    Ok(session) => {
                        tracing::info!("Zenoh session open, ZID: {}", session.zid());
                        *zenoh.session.lock().await = Some(Arc::new(session));
                        commands::device::start_discovery_task(zenoh, app_handle);
                    }
                    Err(e) => {
                        tracing::error!("Failed to open Zenoh session: {e}");
                        // App still starts; Zenoh commands will return errors.
                    }
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            ping,
            // XML model (offline + fixture mode)
            commands::xml_model::parse_xml,
            commands::xml_model::get_current_model,
            commands::xml_model::list_fixtures,
            commands::xml_model::load_fixture,
            // Device discovery & connection
            commands::device::list_discovered_devices,
            commands::device::get_connection_state,
            commands::device::connect_device,
            commands::device::disconnect_device,
            // Node operations
            commands::nodes::get_node_value,
            commands::nodes::write_node,
            commands::nodes::execute_command,
            commands::nodes::read_nodes_bulk,
            // Acquisition
            commands::acquisition::get_acquisition_status,
            commands::acquisition::start_acquisition,
            commands::acquisition::stop_acquisition,
            // Recording
            commands::recording::start_recording,
            commands::recording::stop_recording,
            commands::recording::get_recording_status,
            // SFNC groups config
            commands::sfnc_groups::get_sfnc_groups,
        ])
        .run(tauri::generate_context!())
    {
        tracing::error!("error while running tauri application: {err}");
    }
}
