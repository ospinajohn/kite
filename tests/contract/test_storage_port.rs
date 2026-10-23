use pocketflow::models::{Movement, MovementType, MovementOrigin};
use pocketflow::storage::port::{StoragePort, PeriodSummary};
use chrono::{Local, NaiveDate};
use uuid::Uuid;

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

#[cfg(test)]
mod tests {
    use super::*;
    use pocketflow::storage::sqlite_impl::SqliteStorage;
    use tempfile::tempdir;

    #[test]
    fn sqlite_storage_passes_contract() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let storage = SqliteStorage::new(&db_path).unwrap();
        test_storage_port(&storage);
    }
}