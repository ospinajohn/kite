use crate::models::Movement;
use crate::models::summary::PeriodSummary;
use chrono::NaiveDate;
use std::error::Error;
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("Movimiento no encontrado: {0}")]
    NotFound(Uuid),
    #[error("Violación de integridad: {0}")]
    Integrity(String),
    #[error("Error de base de datos: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("Error de E/S: {0}")]
    Io(#[from] std::io::Error),
    #[error("Error de migración: {0}")]
    Migration(String),
}

pub trait StoragePort: Send + Sync {
    fn save(&self, movement: Movement) -> Result<Movement, StorageError>;
    fn update(&self, movement: Movement) -> Result<Movement, StorageError>;
    fn delete(&self, id: Uuid) -> Result<bool, StorageError>;
    fn get_by_id(&self, id: Uuid) -> Result<Option<Movement>, StorageError>;
    fn query(
        &self,
        desde: Option<NaiveDate>,
        hasta: Option<NaiveDate>,
    ) -> Result<Vec<Movement>, StorageError>;
    fn summary(
        &self,
        desde: Option<NaiveDate>,
        hasta: Option<NaiveDate>,
    ) -> Result<PeriodSummary, StorageError>;
    fn migrate(&self) -> Result<(), StorageError>;
}

#[cfg(test)]
mod contract_tests {
    use super::*;
    use crate::models::{Movement, MovementType, MovementOrigin};
    use chrono::{Local, NaiveDate};

    pub fn test_storage_port<S: StoragePort>(storage: &S) {
        storage.migrate().expect("migrate ok");

        let ingreso = Movement::new(
            MovementType::Ingreso,
            1_500_000,
            "Pago de quincena".to_string(),
            NaiveDate::from_ymd_opt(2026, 10, 15).unwrap(),
            MovementOrigin::Formulario,
        ).unwrap();
        let saved = storage.save(ingreso.clone()).expect("save ingreso");
        assert_ne!(saved.id, Uuid::nil());
        assert_eq!(saved.tipo, MovementType::Ingreso);
        assert_eq!(saved.monto_centavos, 1_500_000);

        let gasto = Movement::new(
            MovementType::Gasto,
            18_000,
            "Almuerzo".to_string(),
            Local::now().date_naive(),
            MovementOrigin::Formulario,
        ).unwrap();
        let saved_gasto = storage.save(gasto.clone()).expect("save gasto");
        assert_eq!(saved_gasto.tipo, MovementType::Gasto);

        let all = storage.query(None, None).expect("query all");
        assert_eq!(all.len(), 2);
        assert!(all[0].fecha >= all[1].fecha);

        let filtered = storage.query(
            Some(NaiveDate::from_ymd_opt(2026, 10, 1).unwrap()),
            Some(NaiveDate::from_ymd_opt(2026, 10, 31).unwrap()),
        ).expect("query range");
        assert!(filtered.len() <= 2);

        let summary = storage.summary(None, None).expect("summary all");
        assert_eq!(summary.ingresos_centavos, 1_500_000);
        assert_eq!(summary.gastos_centavos, 18_000);
        assert_eq!(summary.diferencia_centavos, 1_482_000);

        let mut updated = saved_gasto.clone();
        updated.monto_centavos = 19_000;
        let saved_updated = storage.update(updated).expect("update");
        assert_eq!(saved_updated.monto_centavos, 19_000);

        let summary2 = storage.summary(None, None).expect("summary after update");
        assert_eq!(summary2.gastos_centavos, 19_000);

        let deleted = storage.delete(saved_gasto.id).expect("delete");
        assert!(deleted);
        let after_delete = storage.query(None, None).expect("query after delete");
        assert_eq!(after_delete.len(), 1);

        let not_deleted = storage.delete(Uuid::nil()).expect("delete not found");
        assert!(!not_deleted);
    }
}