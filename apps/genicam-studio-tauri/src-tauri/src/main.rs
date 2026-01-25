#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod state;

use tokio::sync::RwLock;

#[tauri::command]
fn ping() -> &'static str {
    "pong"
}

fn main() {
    // Store the last parsed model in Rust so Tauri mode can reload without
    // re-parsing and keep the UiGraph contract authoritative on the backend.
    let model_state = RwLock::new(state::ModelState::default());

    if let Err(err) = tauri::Builder::default()
        .manage(model_state)
        .invoke_handler(tauri::generate_handler![
            ping,
            commands::xml_model::parse_xml,
            commands::xml_model::get_current_model,
            commands::xml_model::list_fixtures,
            commands::xml_model::load_fixture
        ])
        .run(tauri::generate_context!())
    {
        eprintln!("error while running tauri application: {err}");
    }
}
