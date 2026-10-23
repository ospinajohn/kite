use crate::models::{Movement, MovementType, MovementOrigin, PeriodSummary};
use crate::parser::port::{ParserPort, ParsedProposal};
use crate::storage::port::StoragePort;
use crate::ui::port::{AppCommand, UiNotifier};
use chrono::NaiveDate;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

pub struct AppState {
    storage: Arc<dyn StoragePort>,
    parser: Arc<dyn ParserPort>,
    pending_proposal: Mutex<Option<ParsedProposal>>,
}

impl AppState {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let db_path = directories::ProjectDirs::from("com", "pocketflow", "PocketFlow")
            .map(|dirs| dirs.data_dir().join("pocketflow.db"))
            .unwrap_or_else(|| std::path::PathBuf::from("pocketflow.db"));

        let storage = crate::storage::sqlite_impl::SqliteStorage::new(&db_path)?;
        storage.migrate()?;

        Ok(Self {
            storage: Arc::new(storage),
            parser: Arc::new(crate::parser::regex_impl::RegexParser::new()),
            pending_proposal: Mutex::new(None),
        })
    }

    pub fn storage(&self) -> Arc<dyn StoragePort> {
        self.storage.clone()
    }

    pub fn parser(&self) -> Arc<dyn ParserPort> {
        self.parser.clone()
    }

    pub fn take_pending_proposal(&self) -> Option<ParsedProposal> {
        self.pending_proposal.lock().unwrap().take()
    }

    pub fn set_pending_proposal(&self, proposal: ParsedProposal) {
        *self.pending_proposal.lock().unwrap() = Some(proposal);
    }
}

impl AppCommand for AppState {
    fn handle_new_expense(&self, monto_centavos: i64, concepto: String, fecha: NaiveDate) {
        let movement = Movement::new(
            MovementType::Gasto,
            monto_centavos,
            concepto,
            fecha,
            MovementOrigin::Formulario,
        );
        match movement {
            Ok(m) => {
                if let Err(e) = self.storage.save(m.clone()) {
                    eprintln!("Error saving expense: {}", e);
                }
            }
            Err(e) => {
                eprintln!("Validation error: {}", e);
            }
        }
    }

    fn handle_new_income(&self, monto_centavos: i64, concepto: String, fecha: NaiveDate) {
        let movement = Movement::new(
            MovementType::Ingreso,
            monto_centavos,
            concepto,
            fecha,
            MovementOrigin::Formulario,
        );
        match movement {
            Ok(m) => {
                if let Err(e) = self.storage.save(m.clone()) {
                    eprintln!("Error saving income: {}", e);
                }
            }
            Err(e) => {
                eprintln!("Validation error: {}", e);
            }
        }
    }

    fn handle_parse_message(&self, text: String) {
        match self.parser.parse(&text) {
            Ok(Some(proposal)) => {
                self.set_pending_proposal(proposal.clone());
            }
            Ok(None) => {
                eprintln!("No se pudo interpretar el mensaje");
            }
            Err(e) => {
                eprintln!("Error parsing: {}", e);
            }
        }
    }

    fn handle_confirm_proposal(&self, proposal: ParsedProposal) {
        let movement = Movement::new(
            proposal.tipo,
            proposal.monto_centavos,
            proposal.concepto,
            proposal.fecha,
            proposal.origen,
        );
        match movement {
            Ok(m) => {
                if let Err(e) = self.storage.save(m.clone()) {
                    eprintln!("Error saving from proposal: {}", e);
                }
            }
            Err(e) => {
                eprintln!("Validation error from proposal: {}", e);
            }
        }
    }

    fn handle_open_history(&self, desde: Option<NaiveDate>, hasta: Option<NaiveDate>) {
        match self.storage.query(desde, hasta) {
            Ok(movements) => {
                println!("History query returned {} movements", movements.len());
            }
            Err(e) => {
                eprintln!("Error querying history: {}", e);
            }
        }
    }

    fn handle_open_summary(&self, desde: Option<NaiveDate>, hasta: Option<NaiveDate>) {
        match self.storage.summary(desde, hasta) {
            Ok(summary) => {
                println!("Summary: ingresos={}, gastos={}, diff={}",
                    summary.ingresos_centavos, summary.gastos_centavos, summary.diferencia_centavos);
            }
            Err(e) => {
                eprintln!("Error getting summary: {}", e);
            }
        }
    }

    fn handle_edit_movement(&self, id: Uuid, monto_centavos: i64, concepto: String, fecha: NaiveDate) {
        if let Ok(Some(mut movement)) = self.storage.get_by_id(id) {
            match movement.update(monto_centavos, concepto, fecha) {
                Ok(updated) => {
                    if let Err(e) = self.storage.update(updated.clone()) {
                        eprintln!("Error updating movement: {}", e);
                    }
                }
                Err(e) => {
                    eprintln!("Validation error: {}", e);
                }
            }
        } else {
            eprintln!("Movement not found");
        }
    }

    fn handle_delete_movement(&self, id: Uuid) {
        if let Err(e) = self.storage.delete(id) {
            eprintln!("Error deleting movement: {}", e);
        }
    }

    fn shutdown(&self) {
        println!("Shutting down...");
    }
}

#[tauri::command]
pub async fn register_expense(
    state: tauri::State<'_, AppState>,
    monto_centavos: i64,
    concepto: String,
    fecha: String,
) -> Result<(), String> {
    let fecha = NaiveDate::parse_from_str(&fecha, "%Y-%m-%d").map_err(|e| e.to_string())?;
    state.handle_new_expense(monto_centavos, concepto, fecha);
    Ok(())
}

#[tauri::command]
pub async fn register_income(
    state: tauri::State<'_, AppState>,
    monto_centavos: i64,
    concepto: String,
    fecha: String,
) -> Result<(), String> {
    let fecha = NaiveDate::parse_from_str(&fecha, "%Y-%m-%d").map_err(|e| e.to_string())?;
    state.handle_new_income(monto_centavos, concepto, fecha);
    Ok(())
}

#[tauri::command]
pub async fn parse_message(
    state: tauri::State<'_, AppState>,
    text: String,
) -> Result<Option<crate::parser::port::ParsedProposal>, String> {
    state.handle_parse_message(text);
    Ok(state.take_pending_proposal())
}

#[tauri::command]
pub async fn confirm_proposal(
    state: tauri::State<'_, AppState>,
    proposal: crate::parser::port::ParsedProposal,
) -> Result<(), String> {
    state.handle_confirm_proposal(proposal);
    Ok(())
}

#[tauri::command]
pub async fn cancel_proposal(state: tauri::State<'_, AppState>) -> Result<(), String> {
    state.pending_proposal.lock().unwrap().take();
    Ok(())
}

#[tauri::command]
pub async fn get_history(
    state: tauri::State<'_, AppState>,
    desde: Option<String>,
    hasta: Option<String>,
) -> Result<Vec<crate::models::Movement>, String> {
    let desde = desde.and_then(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok());
    let hasta = hasta.and_then(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok());
    state.storage.query(desde, hasta).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_summary(
    state: tauri::State<'_, AppState>,
    desde: Option<String>,
    hasta: Option<String>,
) -> Result<crate::models::summary::PeriodSummary, String> {
    let desde = desde.and_then(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok());
    let hasta = hasta.and_then(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok());
    state.storage.summary(desde, hasta).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn edit_movement(
    state: tauri::State<'_, AppState>,
    id: String,
    monto_centavos: i64,
    concepto: String,
    fecha: String,
) -> Result<(), String> {
    let id = Uuid::parse_str(&id).map_err(|e| e.to_string())?;
    let fecha = NaiveDate::parse_from_str(&fecha, "%Y-%m-%d").map_err(|e| e.to_string())?;
    state.handle_edit_movement(id, monto_centavos, concepto, fecha);
    Ok(())
}

#[tauri::command]
pub async fn delete_movement(
    state: tauri::State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    let id = Uuid::parse_str(&id).map_err(|e| e.to_string())?;
    state.handle_delete_movement(id);
    Ok(())
}

#[tauri::command]
pub async fn minimize_window(window: tauri::Window) -> Result<(), String> {
    window.minimize().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn toggle_maximize_window(window: tauri::Window) -> Result<(), String> {
    if window.is_maximized().unwrap_or(false) {
        window.unmaximize().map_err(|e| e.to_string())
    } else {
        window.maximize().map_err(|e| e.to_string())
    }
}

#[tauri::command]
pub async fn close_window(window: tauri::Window) -> Result<(), String> {
    window.close().map_err(|e| e.to_string())
}