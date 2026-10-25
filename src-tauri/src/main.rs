#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use kite::AppState;
use tauri::Manager;

fn main() {
    let app_state = AppState::new().expect("Failed to initialize app state");

    tauri::Builder::default()
        .setup(|app| {
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                // App initialization logic here if needed
            });
            Ok(())
        })
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            kite::app::register_expense,
            kite::app::register_income,
            kite::app::parse_message,
            kite::app::confirm_proposal,
            kite::app::cancel_proposal,
            kite::app::get_history,
            kite::app::get_summary,
            kite::app::edit_movement,
            kite::app::delete_movement,
            kite::app::minimize_window,
            kite::app::toggle_maximize_window,
            kite::app::close_window,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}