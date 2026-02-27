#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod state;

use std::sync::Arc;
use tokio::sync::RwLock;

#[tauri::command]
fn ping() -> &'static str {
    "pong"
}

fn main() {
    // ModelState: holds the last parsed UiGraph (used by xml_model commands).
    let model_state = RwLock::new(state::ModelState::default());

    // ZenohState: device discovery, connection, node cache, acquisition.
    // Wrapped in Arc so background tasks can hold a clone independently.
    let zenoh_state = Arc::new(state::ZenohState::new());
    let zenoh_for_setup = zenoh_state.clone();

    if let Err(err) = tauri::Builder::default()
        .manage(model_state)
        .manage(zenoh_state)
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
                        eprintln!("Failed to open Zenoh session: {e}");
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
            // Acquisition
            commands::acquisition::get_acquisition_status,
            commands::acquisition::start_acquisition,
            commands::acquisition::stop_acquisition,
        ])
        .run(tauri::generate_context!())
    {
        eprintln!("error while running tauri application: {err}");
    }
}
