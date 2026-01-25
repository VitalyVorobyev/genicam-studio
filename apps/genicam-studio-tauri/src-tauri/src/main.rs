#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[tauri::command]
fn ping() -> &'static str {
    "pong"
}

fn main() {
    if let Err(err) = tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![ping])
        .run(tauri::generate_context!())
    {
        eprintln!("error while running tauri application: {err}");
    }
}
