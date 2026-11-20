use crate::compromisos::port::CompromisosPort;
use crate::config::ConfigPort;
use crate::metas::port::MetasPort;
use crate::metas::{domain::EstadoMeta, service::recalcular_estado_meta};
use crate::models::{Movement, MovementOrigin, MovementType};
use crate::parser::port::{ParsedProposal, ParserPort};
use crate::storage::port::StoragePort;
use crate::ui::port::AppCommand;
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
    compromisos: Arc<dyn crate::compromisos::port::CompromisosPort>,
    config: Arc<dyn crate::config::ConfigPort>,
    metas: Arc<dyn crate::metas::port::MetasPort>,
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
        let compromisos = crate::compromisos::sqlite_impl::SqliteCompromisosStorage::new(&db_path)?;
        compromisos.migrate()?;
        let config = crate::config::SqliteConfigStorage::new(&db_path)?;
        config.migrate()?;
        let metas = crate::metas::sqlite_impl::SqliteMetasStorage::new(&db_path)?;
        metas.migrate()?;

        Ok(Self {
            storage: Arc::new(storage),
            parser: Arc::new(crate::parser::regex_impl::RegexParser::new()),
            compromisos: Arc::new(compromisos),
            config: Arc::new(config),
            metas: Arc::new(metas),
            pending_proposal: Mutex::new(None),
        })
    }

    pub fn storage(&self) -> Arc<dyn StoragePort> {
        self.storage.clone()
    }

    pub fn compromisos(&self) -> Arc<dyn crate::compromisos::port::CompromisosPort> {
        self.compromisos.clone()
    }

    pub fn parser(&self) -> Arc<dyn ParserPort> {
        self.parser.clone()
    }

    pub fn config(&self) -> Arc<dyn crate::config::ConfigPort> {
        self.config.clone()
    }

    pub fn metas(&self) -> Arc<dyn crate::metas::port::MetasPort> {
        self.metas.clone()
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
        if let Ok(Some(movement)) = self.storage.get_by_id(id) {
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

// --- Compromisos recurrentes (feature 002) ---

#[derive(serde::Serialize, serde::Deserialize)]
pub struct CompromisoUI {
    pub compromiso: crate::models::Compromiso,
    pub proximo: Option<crate::models::Vencimiento>,
    pub periodos_pagados: Vec<String>,
}

#[tauri::command]
pub async fn list_compromisos(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<CompromisoUI>, String> {
    let hoy = chrono::Local::now().date_naive();
    let storage = state.compromisos();
    let mut items = Vec::new();
    for c in storage.list().map_err(|e| e.to_string())? {
        let pagados = storage.periodos_pagados(c.id).map_err(|e| e.to_string())?;
        let proximo = crate::compromisos::proximo_vencimiento(&c, hoy, &pagados);
        items.push(CompromisoUI {
            compromiso: c,
            proximo,
            periodos_pagados: pagados,
        });
    }
    Ok(items)
}

#[tauri::command]
pub async fn add_compromiso(
    state: tauri::State<'_, AppState>,
    concepto: String,
    monto_centavos: i64,
    frecuencia: crate::models::Frecuencia,
    duracion: crate::models::Duracion,
) -> Result<crate::models::Compromiso, String> {
    let c = crate::models::Compromiso::new(concepto, monto_centavos, frecuencia, duracion)
        .map_err(|e| e.to_string())?;
    state.compromisos().save(c).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_compromiso(
    state: tauri::State<'_, AppState>,
    id: String,
    concepto: String,
    monto_centavos: i64,
    frecuencia: crate::models::Frecuencia,
    duracion: crate::models::Duracion,
) -> Result<crate::models::Compromiso, String> {
    let id = Uuid::parse_str(&id).map_err(|e| e.to_string())?;
    let actual = state
        .compromisos()
        .get_by_id(id)
        .map_err(|e| e.to_string())?
        .ok_or("Compromiso no encontrado")?;
    let actualizado = actual
        .update(concepto, monto_centavos, frecuencia, duracion)
        .map_err(|e| e.to_string())?;
    state
        .compromisos()
        .update(actualizado)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_compromiso(
    state: tauri::State<'_, AppState>,
    id: String,
) -> Result<bool, String> {
    let id = Uuid::parse_str(&id).map_err(|e| e.to_string())?;
    state.compromisos().delete(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn set_compromiso_estado(
    state: tauri::State<'_, AppState>,
    id: String,
    pausado: bool,
) -> Result<crate::models::Compromiso, String> {
    let id = Uuid::parse_str(&id).map_err(|e| e.to_string())?;
    let estado = if pausado {
        crate::models::EstadoCompromiso::Pausado
    } else {
        crate::models::EstadoCompromiso::Activo
    };
    state
        .compromisos()
        .set_estado(id, estado)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_vencimientos(
    state: tauri::State<'_, AppState>,
) -> Result<crate::compromisos::VencimientosResponse, String> {
    let hoy = chrono::Local::now().date_naive();
    crate::compromisos::calcular_vencimientos(state.compromisos().as_ref(), hoy)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn confirmar_pago_compromiso(
    state: tauri::State<'_, AppState>,
    id: String,
    periodo_key: String,
    monto_centavos: i64,
    fecha: String,
) -> Result<crate::models::Movement, String> {
    let id = Uuid::parse_str(&id).map_err(|e| e.to_string())?;
    let fecha = NaiveDate::parse_from_str(&fecha, "%Y-%m-%d").map_err(|e| e.to_string())?;
    let compromiso = state
        .compromisos()
        .get_by_id(id)
        .map_err(|e| e.to_string())?
        .ok_or("Compromiso no encontrado")?;
    crate::compromisos::confirmar_pago(
        state.storage().as_ref(),
        state.compromisos().as_ref(),
        &compromiso,
        periodo_key,
        monto_centavos,
        fecha,
    )
    .map_err(|e| e.to_string())
}

// --- Planificación quincenal (feature 003) ---

#[tauri::command]
pub async fn get_planificacion(
    state: tauri::State<'_, AppState>,
    desde: Option<String>,
    hasta: Option<String>,
) -> Result<crate::planificacion::PlanQuincena, String> {
    // La quincena se deriva de `desde` (o de hoy). Si se recibe `hasta`, debe
    // coincidir con el fin del período calculado; no se acepta un rango ajeno.
    let hasta = hasta
        .map(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d"))
        .transpose()
        .map_err(|_| "Fecha inválida: se esperaba el formato YYYY-MM-DD".to_string())?;
    let referencia = desde
        .map(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d"))
        .transpose()
        .map_err(|_| "Fecha inválida: se esperaba el formato YYYY-MM-DD".to_string())?;
    let corte = match state.config().get("dia_corte").map_err(|e| e.to_string())? {
        Some(valor) => valor
            .parse::<u32>()
            .map_err(|_| format!("El día de corte configurado no es válido: {valor}"))?,
        None => 15,
    };
    let plan = crate::planificacion::plan_de_estado(
        state.storage().as_ref(),
        state.compromisos().as_ref(),
        corte,
        referencia,
    )
    .map_err(|e| e.to_string())?;
    if let Some(hasta) = hasta {
        if hasta != plan.rango.hasta {
            return Err(
                "El parámetro 'hasta' no coincide con el rango de la quincena calculada"
                    .to_string(),
            );
        }
    }
    Ok(plan)
}

/// Valida el día de corte (1..=28, FR-P01/FR-P07). Extraída del comando para
/// poder testearla sin Tauri State.
pub fn validar_dia_corte(valor: u32) -> Result<u32, String> {
    if !(1..=28).contains(&valor) {
        return Err("El día de corte debe estar entre 1 y 28".into());
    }
    Ok(valor)
}

#[tauri::command]
pub async fn set_dia_corte(state: tauri::State<'_, AppState>, valor: u32) -> Result<(), String> {
    let valor = validar_dia_corte(valor)?;
    state
        .config()
        .set("dia_corte", &valor.to_string())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_dia_corte(state: tauri::State<'_, AppState>) -> Result<u32, String> {
    match state.config().get("dia_corte").map_err(|e| e.to_string())? {
        Some(valor) => valor
            .parse::<u32>()
            .map_err(|_| format!("El día de corte configurado no es válido: {valor}")),
        None => Ok(15),
    }
}

// --- Metas y apartados (feature 005), funciones libres testeables sin Tauri State ---

pub fn listar_metas(
    metas: &dyn MetasPort,
) -> Result<Vec<crate::metas::domain::MetaUI>, crate::metas::domain::MetasError> {
    crate::metas::service::listar_metas(metas)
}

pub fn alta_apartado(
    movs: &dyn StoragePort,
    metas: &dyn MetasPort,
    meta_id: Uuid,
    monto_centavos: i64,
    fecha: NaiveDate,
) -> Result<Movement, String> {
    let meta = metas
        .get_by_id(meta_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Meta no encontrada".to_string())?;
    if meta.estado != EstadoMeta::Activa {
        return Err("La meta no acepta apartados".to_string());
    }
    let movimiento = Movement::new_apartado(
        meta_id,
        monto_centavos,
        format!("Apartado · {}", meta.nombre),
        fecha,
    )
    .map_err(|e| e.to_string())?;
    let saved = movs.save(movimiento).map_err(|e| e.to_string())?;
    recalcular_estado_meta(metas, meta_id).map_err(|e| e.to_string())?;
    Ok(saved)
}

pub fn editar_apartado(
    movs: &dyn StoragePort,
    metas: &dyn MetasPort,
    apartado_id: Uuid,
    monto_centavos: i64,
    concepto: String,
    fecha: NaiveDate,
) -> Result<Movement, String> {
    let mov = movs
        .get_by_id(apartado_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Movimiento no encontrado".to_string())?;
    if mov.tipo != MovementType::Aporte {
        return Err("El movimiento no es un apartado".to_string());
    }
    let actualizado = mov
        .update(monto_centavos, concepto, fecha)
        .map_err(|e| e.to_string())?;
    let saved = movs.update(actualizado).map_err(|e| e.to_string())?;
    if let Some(meta_id) = saved.meta_id {
        recalcular_estado_meta(metas, meta_id).map_err(|e| e.to_string())?;
    }
    Ok(saved)
}

pub fn borrar_apartado(
    movs: &dyn StoragePort,
    metas: &dyn MetasPort,
    apartado_id: Uuid,
) -> Result<bool, String> {
    let mov = movs
        .get_by_id(apartado_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Movimiento no encontrado".to_string())?;
    if mov.tipo != MovementType::Aporte {
        return Err("El movimiento no es un apartado".to_string());
    }
    let meta_id = mov.meta_id;
    let deleted = movs.delete(apartado_id).map_err(|e| e.to_string())?;
    if let Some(meta_id) = meta_id {
        recalcular_estado_meta(metas, meta_id).map_err(|e| e.to_string())?;
    }
    Ok(deleted)
}

pub fn actualizar_meta(
    metas: &dyn MetasPort,
    id: Uuid,
    nombre: String,
    monto_objetivo_centavos: i64,
    fecha_objetivo: Option<NaiveDate>,
) -> Result<crate::metas::domain::Meta, String> {
    let meta = metas
        .get_by_id(id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Meta no encontrada".to_string())?;
    let actualizado = meta
        .update(nombre, monto_objetivo_centavos, fecha_objetivo)
        .map_err(|e| e.to_string())?;
    metas.update(actualizado).map_err(|e| e.to_string())?;
    recalcular_estado_meta(metas, id).map_err(|e| e.to_string())
}

// --- Tauri commands: metas ---

#[tauri::command]
pub async fn list_metas(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<crate::metas::domain::MetaUI>, String> {
    crate::metas::service::listar_metas(state.metas().as_ref()).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn add_meta(
    state: tauri::State<'_, AppState>,
    nombre: String,
    monto_objetivo_centavos: i64,
    fecha_objetivo: Option<String>,
) -> Result<crate::metas::domain::Meta, String> {
    let fecha = fecha_objetivo
        .as_deref()
        .map(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d"))
        .transpose()
        .map_err(|e| e.to_string())?;
    let meta = crate::metas::domain::Meta::new(nombre, monto_objetivo_centavos, fecha)
        .map_err(|e| e.to_string())?;
    state.metas().save(meta).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_meta(
    state: tauri::State<'_, AppState>,
    id: String,
    nombre: String,
    monto_objetivo_centavos: i64,
    fecha_objetivo: Option<String>,
) -> Result<crate::metas::domain::Meta, String> {
    let id = Uuid::parse_str(&id).map_err(|e| e.to_string())?;
    let fecha = fecha_objetivo
        .as_deref()
        .map(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d"))
        .transpose()
        .map_err(|e| e.to_string())?;
    actualizar_meta(
        state.metas().as_ref(),
        id,
        nombre,
        monto_objetivo_centavos,
        fecha,
    )
}

#[tauri::command]
pub async fn cancelar_meta(
    state: tauri::State<'_, AppState>,
    id: String,
) -> Result<crate::metas::domain::Meta, String> {
    let id = Uuid::parse_str(&id).map_err(|e| e.to_string())?;
    state.metas().cancelar(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn reactivar_meta(
    state: tauri::State<'_, AppState>,
    id: String,
) -> Result<crate::metas::domain::Meta, String> {
    let id = Uuid::parse_str(&id).map_err(|e| e.to_string())?;
    state.metas().reactivar(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn add_apartado(
    state: tauri::State<'_, AppState>,
    meta_id: String,
    monto_centavos: i64,
    fecha: String,
) -> Result<crate::models::Movement, String> {
    let meta_id = Uuid::parse_str(&meta_id).map_err(|e| e.to_string())?;
    let fecha = NaiveDate::parse_from_str(&fecha, "%Y-%m-%d").map_err(|e| e.to_string())?;
    alta_apartado(
        state.storage().as_ref(),
        state.metas().as_ref(),
        meta_id,
        monto_centavos,
        fecha,
    )
}

#[tauri::command]
pub async fn edit_apartado(
    state: tauri::State<'_, AppState>,
    id: String,
    monto_centavos: i64,
    concepto: String,
    fecha: String,
) -> Result<crate::models::Movement, String> {
    let id = Uuid::parse_str(&id).map_err(|e| e.to_string())?;
    let fecha = NaiveDate::parse_from_str(&fecha, "%Y-%m-%d").map_err(|e| e.to_string())?;
    editar_apartado(
        state.storage().as_ref(),
        state.metas().as_ref(),
        id,
        monto_centavos,
        concepto,
        fecha,
    )
}

#[tauri::command]
pub async fn delete_apartado(
    state: tauri::State<'_, AppState>,
    id: String,
) -> Result<bool, String> {
    let id = Uuid::parse_str(&id).map_err(|e| e.to_string())?;
    borrar_apartado(state.storage().as_ref(), state.metas().as_ref(), id)
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

    #[test]
    fn dia_corte_valido_se_acepta() {
        assert_eq!(validar_dia_corte(1).unwrap(), 1);
        assert_eq!(validar_dia_corte(15).unwrap(), 15);
        assert_eq!(validar_dia_corte(28).unwrap(), 28);
    }

    #[test]
    fn dia_corte_fuera_de_rango_se_rechaza() {
        for valor in [0u32, 29, 100] {
            let err = validar_dia_corte(valor).unwrap_err();
            assert_eq!(err, "El día de corte debe estar entre 1 y 28");
        }
    }
}
