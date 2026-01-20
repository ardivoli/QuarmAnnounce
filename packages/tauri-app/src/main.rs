// Prevents additional console window on Windows in release mode
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod state;

use state::AppState;

fn main() {
    tauri::Builder::default()
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![
            commands::load_config,
            commands::save_config,
            commands::get_config,
            commands::init_tts,
            commands::test_announcement,
            commands::start_monitoring,
            commands::stop_monitoring,
            commands::get_monitoring_status,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
