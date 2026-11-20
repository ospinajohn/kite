#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use kite::AppState;
use std::collections::HashSet;
use std::sync::Mutex;
use std::time::Duration;
use tauri_plugin_notification::NotificationExt;

/// Revisa periódicamente los compromisos que vencen hoy y notifica una vez por día
/// mientras la app está en ejecución (FR-C08). No hay scheduler externo (plan §1).
fn notificar_compromisos_del_dia(app: tauri::AppHandle) {
    use tauri::Manager;
    let notificados: Mutex<HashSet<String>> = Mutex::new(HashSet::new());
    loop {
        let state = app.state::<AppState>();
        let hoy = chrono::Local::now().date_naive();
        match kite::compromisos::calcular_vencimientos(state.compromisos().as_ref(), hoy) {
            Ok(vencimientos) => {
                for item in &vencimientos.hoy {
                    let clave = format!("{}:{}", item.compromiso.id, hoy.format("%Y-%m-%d"));
                    let ya_notificado = {
                        let mut set = notificados.lock().unwrap();
                        if set.contains(&clave) {
                            true
                        } else {
                            set.insert(clave);
                            false
                        }
                    };
                    if ya_notificado {
                        continue;
                    }
                    let concepto = &item.compromiso.concepto;
                    let cuerpo =
                        format!("«{concepto}» vence hoy. Registralo desde la pestaña Compromisos.");
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
                // Limpia claves de días anteriores para no crecer sin límite.
                let prefijo_hoy = hoy.format("%Y-%m-%d").to_string();
                if let Ok(mut set) = notificados.lock() {
                    set.retain(|k| k.ends_with(&prefijo_hoy));
                }
            }
            Err(e) => eprintln!("[kite] Error calculando vencimientos para notificar: {e}"),
        }
        std::thread::sleep(Duration::from_secs(60));
    }
}

fn main() {
    let app_state = AppState::new().expect("Failed to initialize app state");

    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                notificar_compromisos_del_dia(handle);
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
            kite::app::get_planificacion,
            kite::app::set_dia_corte,
            kite::app::get_dia_corte,
            kite::app::list_metas,
            kite::app::add_meta,
            kite::app::update_meta,
            kite::app::cancelar_meta,
            kite::app::reactivar_meta,
            kite::app::add_apartado,
            kite::app::edit_apartado,
            kite::app::delete_apartado,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
