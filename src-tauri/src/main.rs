#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use pocketflow::AppState;
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
            pocketflow::app::register_expense,
            pocketflow::app::register_income,
            pocketflow::app::parse_message,
            pocketflow::app::confirm_proposal,
            pocketflow::app::cancel_proposal,
            pocketflow::app::get_history,
            pocketflow::app::get_summary,
            pocketflow::app::edit_movement,
            pocketflow::app::delete_movement,
            pocketflow::app::minimize_window,
            pocketflow::app::toggle_maximize_window,
            pocketflow::app::close_window,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}