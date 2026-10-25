use crate::models::{Movement, MovementOrigin, MovementType, PeriodSummary};
use crate::parser::port::{ParsedProposal, ParserPort};
use crate::storage::port::StoragePort;
use crate::ui::port::{AppCommand, UiNotifier};
use chrono::NaiveDate;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

const DB_DIR: &str = "Kite";
const DB_FILE: &str = "Kite.db";
const LEGACY_DB_FILE: &str = "pocketflow.db";

pub struct AppState {
    storage: Arc<dyn StoragePort>,
    parser: Arc<dyn ParserPort>,
    pending_proposal: Mutex<Option<ParsedProposal>>,
}

fn default_db_path() -> PathBuf {
    directories::BaseDirs::new()
        .map(|dirs| dirs.data_dir().join(DB_DIR).join(DB_FILE))
        .unwrap_or_else(|| PathBuf::from(DB_FILE))
}

fn legacy_db_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    for organization in ["PocketFlow", "pocketflow"] {
        if let Some(dirs) = directories::ProjectDirs::from("com", organization, "PocketFlow") {
            candidates.push(dirs.data_dir().join(LEGACY_DB_FILE));
        }
    }

    if let Some(dirs) = directories::BaseDirs::new() {
        let appdata = dirs.data_dir();
        candidates.push(appdata.join("PocketFlow").join(LEGACY_DB_FILE));
        candidates.push(
            appdata
                .join("PocketFlow")
                .join("PocketFlow")
                .join("data")
                .join(LEGACY_DB_FILE),
        );
        // Ubicación usada por versiones intermedias de Kite antes de unificar la ruta.
        candidates.push(
            appdata
                .join("kite")
                .join("Kite")
                .join("data")
                .join("kite.db"),
        );
    }

    candidates
}

fn migrate_legacy_database(
    target: &Path,
    candidates: impl IntoIterator<Item = PathBuf>,
) -> std::io::Result<Option<PathBuf>> {
    if target.exists() {
        return Ok(None);
    }

    for source in candidates {
        if !source.is_file() {
            continue;
        }

        if let Some(parent) = target.parent().filter(|path| !path.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::copy(&source, target)?;

        // La base antigua puede tener movimientos pendientes en el WAL: copiarlo evita perderlos.
        let target_name = target.file_name().unwrap_or_default().to_string_lossy();
        let wal_source = source.with_extension(format!(
            "{}-wal",
            source
                .extension()
                .map(|extension| extension.to_string_lossy())
                .unwrap_or_default()
        ));
        if wal_source.is_file() {
            std::fs::copy(
                &wal_source,
                target.with_file_name(format!("{target_name}-wal")),
            )?;
        }

        return Ok(Some(source));
    }

    Ok(None)
}

impl AppState {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let db_path = default_db_path();
        match migrate_legacy_database(&db_path, legacy_db_candidates()) {
            Ok(Some(source)) => eprintln!(
                "[kite] Base de datos migrada desde '{}' a '{}'. La base original no se modificó.",
                source.display(),
                db_path.display()
            ),
            Ok(None) => {}
            Err(error) => eprintln!(
                "[kite] Aviso: no se pudo migrar la base de datos antigua a '{}': {error}",
                db_path.display()
            ),
        }

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
                println!(
                    "Summary: ingresos={}, gastos={}, diff={}",
                    summary.ingresos_centavos, summary.gastos_centavos, summary.diferencia_centavos
                );
            }
            Err(e) => {
                eprintln!("Error getting summary: {}", e);
            }
        }
    }

    fn handle_edit_movement(
        &self,
        id: Uuid,
        monto_centavos: i64,
        concepto: String,
        fecha: NaiveDate,
    ) {
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
    state
        .storage
        .summary(desde, hasta)
        .map_err(|e| e.to_string())
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
pub async fn delete_movement(state: tauri::State<'_, AppState>, id: String) -> Result<(), String> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn migra_la_base_antigua_cuando_no_existe_la_nueva() {
        let dir = tempfile::tempdir().unwrap();
        let legacy = dir.path().join("pocketflow.db");
        let legacy_wal = dir.path().join("pocketflow.db-wal");
        fs::write(&legacy, b"contenido").unwrap();
        fs::write(&legacy_wal, b"wal").unwrap();
        let target = dir.path().join("Kite").join("Kite.db");

        let migrated = migrate_legacy_database(&target, vec![legacy.clone()]).unwrap();

        assert_eq!(migrated, Some(legacy));
        assert_eq!(fs::read(&target).unwrap(), b"contenido");
        assert_eq!(
            fs::read(dir.path().join("Kite").join("Kite.db-wal")).unwrap(),
            b"wal"
        );
    }

    #[test]
    fn no_sobrescribe_la_base_nueva_existente() {
        let dir = tempfile::tempdir().unwrap();
        let legacy = dir.path().join("pocketflow.db");
        fs::write(&legacy, b"antigua").unwrap();
        let target = dir.path().join("Kite.db");
        fs::write(&target, b"actual").unwrap();

        let migrated = migrate_legacy_database(&target, vec![legacy]).unwrap();

        assert_eq!(migrated, None);
        assert_eq!(fs::read(&target).unwrap(), b"actual");
    }

    #[test]
    fn no_hace_nada_cuando_no_hay_candidatos() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("Kite.db");

        let migrated =
            migrate_legacy_database(&target, vec![dir.path().join("pocketflow.db")]).unwrap();

        assert_eq!(migrated, None);
        assert!(!target.exists());
    }

    #[test]
    fn la_ruta_por_defecto_termina_en_kite_kite_db() {
        let path = default_db_path();
        assert!(path.ends_with(Path::new("Kite").join(DB_FILE)));
    }

    #[test]
    fn conserva_los_movimientos_al_migrar_una_base_sqlite_real() {
        use crate::storage::sqlite_impl::SqliteStorage;

        let dir = tempfile::tempdir().unwrap();
        let legacy = dir.path().join("pocketflow.db");
        {
            let storage = SqliteStorage::new(&legacy).unwrap();
            storage.migrate().unwrap();
            let movement = Movement::new(
                MovementType::Gasto,
                18_000,
                "Almuerzo".into(),
                chrono::Local::now().date_naive(),
                MovementOrigin::Formulario,
            )
            .unwrap();
            storage.save(movement).unwrap();
        }

        let target = dir.path().join("Kite").join("Kite.db");
        let migrated = migrate_legacy_database(&target, vec![legacy.clone()]).unwrap();
        assert_eq!(migrated, Some(legacy));

        let storage = SqliteStorage::new(&target).unwrap();
        storage.migrate().unwrap();
        let movements = storage.query(None, None).unwrap();
        assert_eq!(movements.len(), 1);
        assert_eq!(movements[0].monto_centavos, 18_000);
        assert_eq!(movements[0].concepto, "Almuerzo");
    }
}
