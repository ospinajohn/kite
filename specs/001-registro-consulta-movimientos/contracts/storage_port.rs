//! StoragePort — Contrato de persistencia para movimientos
//!
//! Define la frontera entre la lógica de dominio y el almacenamiento.
//! Implementaciones: `SqliteStorage` (producción), `MockStorage` (tests).

use crate::models::movement::{Movement, MovementType};
use crate::models::summary::PeriodSummary;
use chrono::NaiveDate;
use std::error::Error;
use uuid::Uuid;

/// Errores del puerto de almacenamiento
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("Movimiento no encontrado: {0}")]
    NotFound(Uuid),
    #[error("Violación de integridad: {0}")]
    Integrity(String),
    #[error("Error de base de datos: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("Error de migración: {0}")]
    Migration(String),
}

/// Puerto de almacenamiento (trait object safe)
pub trait StoragePort: Send + Sync {
    /// Guarda un movimiento nuevo. Asigna `id`, `creado_en`, `actualizado_en`.
    fn save(&self, movement: Movement) -> Result<Movement, StorageError>;

    /// Actualiza monto, concepto y/o fecha de un movimiento existente.
    /// Refresca `actualizado_en`. No permite cambiar `tipo` ni `origen`.
    fn update(&self, movement: Movement) -> Result<Movement, StorageError>;

    /// Elimina un movimiento por ID. Retorna true si existía.
    fn delete(&self, id: Uuid) -> Result<bool, StorageError>;

    /// Recupera un movimiento por ID.
    fn get_by_id(&self, id: Uuid) -> Result<Option<Movement>, StorageError>;

    /// Lista movimientos en rango de fechas (inclusive).
    /// Sin rango = todos. Orden: fecha DESC, creado_en DESC.
    fn query(
        &self,
        desde: Option<NaiveDate>,
        hasta: Option<NaiveDate>,
    ) -> Result<Vec<Movement>, StorageError>;

    /// Calcula totales en rango de fechas (inclusive).
    /// Sin rango = acumulado total.
    fn summary(
        &self,
        desde: Option<NaiveDate>,
        hasta: Option<NaiveDate>,
    ) -> Result<PeriodSummary, StorageError>;

    /// Inicializa/migra esquema a versión actual.
    fn migrate(&self) -> Result<(), StorageError>;
}

/// Resumen de período (vista derivada)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeriodSummary {
    pub ingresos_centavos: i64,
    pub gastos_centavos: i64,
    pub diferencia_centavos: i64,
    pub desde: Option<NaiveDate>,
    pub hasta: Option<NaiveDate>,
}

impl PeriodSummary {
    pub fn new(
        ingresos_centavos: i64,
        gastos_centavos: i64,
        desde: Option<NaiveDate>,
        hasta: Option<NaiveDate>,
    ) -> Self {
        Self {
            ingresos_centavos,
            gastos_centavos,
            diferencia_centavos: ingresos_centavos - gastos_centavos,
            desde,
            hasta,
        }
    }

    /// Formato legible para UI (COP con separador de miles)
    pub fn format_cop(&self, centavos: i64) -> String {
        let sign = if centavos >= 0 { "" } else { "-" };
        let abs = centavos.abs();
        let miles = abs / 1_000;
        let resto = abs % 1_000;
        if miles > 0 {
            format!("{sign}${miles}.{resto:03}")
        } else {
            format!("{sign}${resto}")
        }
    }
}

#[cfg(test)]
mod contract_tests {
    use super::*;
    use crate::models::movement::{Movement, MovementType, MovementOrigin};
    use chrono::{Local, NaiveDate};

    /// Test genérico contra cualquier StoragePort
    pub fn test_storage_port<S: StoragePort>(storage: &S) {
        // migrate
        storage.migrate().expect("migrate ok");

        // save ingreso
        let ingreso = Movement::new(
            MovementType::Ingreso,
            1_500_000, // centavos
            "Pago de quincena".to_string(),
            NaiveDate::from_ymd_opt(2026, 10, 15).unwrap(),
            MovementOrigin::Formulario,
        );
        let saved = storage.save(ingreso.clone()).expect("save ingreso");
        assert_ne!(saved.id, Uuid::nil());
        assert_eq!(saved.tipo, MovementType::Ingreso);
        assert_eq!(saved.monto_centavos, 1_500_000);

        // save gasto
        let gasto = Movement::new(
            MovementType::Gasto,
            18_000,
            "Almuerzo".to_string(),
            Local::now().date_naive(),
            MovementOrigin::Formulario,
        );
        let saved_gasto = storage.save(gasto.clone()).expect("save gasto");
        assert_eq!(saved_gasto.tipo, MovementType::Gasto);

        // query sin filtro → 2 movimientos, orden desc por fecha
        let all = storage.query(None, None).expect("query all");
        assert_eq!(all.len(), 2);
        assert!(all[0].fecha >= all[1].fecha); // desc

        // query con filtro
        let filtered = storage.query(
            Some(NaiveDate::from_ymd_opt(2026, 10, 1).unwrap()),
            Some(NaiveDate::from_ymd_opt(2026, 10, 31).unwrap()),
        ).expect("query range");
        assert!(filtered.len() <= 2);

        // summary sin filtro
        let summary = storage.summary(None, None).expect("summary all");
        assert_eq!(summary.ingresos_centavos, 1_500_000);
        assert_eq!(summary.gastos_centavos, 18_000);
        assert_eq!(summary.diferencia_centavos, 1_482_000);

        // update gasto
        let mut updated = saved_gasto.clone();
        updated.monto_centavos = 19_000;
        let saved_updated = storage.update(updated).expect("update");
        assert_eq!(saved_updated.monto_centavos, 19_000);

        // summary refleja update
        let summary2 = storage.summary(None, None).expect("summary after update");
        assert_eq!(summary2.gastos_centavos, 19_000);

        // delete
        let deleted = storage.delete(saved_gasto.id).expect("delete");
        assert!(deleted);
        let after_delete = storage.query(None, None).expect("query after delete");
        assert_eq!(after_delete.len(), 1);

        // delete inexistente → false
        let not_deleted = storage.delete(Uuid::nil()).expect("delete not found");
        assert!(!not_deleted);
    }
}