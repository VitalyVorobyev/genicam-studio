#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod error;
mod state;

use std::sync::Arc;
use tokio::sync::RwLock;

#[tauri::command]
fn ping() -> &'static str {
    "pong"
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "genicam_studio=info,warn".into()),
        )
        .init();
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
                match zenoh::open(zenoh::Config::default()).await {
                    Ok(session) => {
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
            // SFNC groups config
            commands::sfnc_groups::get_sfnc_groups,
        ])
        .run(tauri::generate_context!())
    {
        tracing::error!("error while running tauri application: {err}");
    }
}
