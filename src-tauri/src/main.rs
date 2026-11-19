#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use kite::AppState;
use tauri_plugin_notification::NotificationExt;

/// Al arrancar, notifica por Windows los compromisos que vencen hoy (FR-C08).
/// La app corre de forma continua; no hay scheduler externo (plan §1).
async fn notificar_compromisos_del_dia(app: tauri::AppHandle) {
    use tauri::Manager;
    let state = app.state::<AppState>();
    let hoy = chrono::Local::now().date_naive();
    match kite::compromisos::calcular_vencimientos(state.compromisos().as_ref(), hoy) {
        Ok(vencimientos) => {
            for item in &vencimientos.hoy {
                let concepto = &item.compromiso.concepto;
                let cuerpo = format!(
                    "«{concepto}» vence hoy. Registralo desde la pestaña Compromisos."
                );
                if let Err(e) = app
                    .notification()
                    .builder()
                    .title("Kite · Compromiso vence hoy")
                    .body(&cuerpo)
                    .show()
                {
                    eprintln!("[kite] No se pudo notificar el compromiso '{concepto}': {e}");
                }
            }
        }
        Err(e) => eprintln!("[kite] Error calculando vencimientos para notificar: {e}"),
    }
}

fn main() {
    let app_state = AppState::new().expect("Failed to initialize app state");

    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                notificar_compromisos_del_dia(handle).await;
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
            kite::app::list_compromisos,
            kite::app::add_compromiso,
            kite::app::update_compromiso,
            kite::app::delete_compromiso,
            kite::app::set_compromiso_estado,
            kite::app::get_vencimientos,
            kite::app::confirmar_pago_compromiso,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}