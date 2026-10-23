use pocketflow::models::{Movement, MovementType, MovementOrigin};
use pocketflow::storage::port::StoragePort;
use chrono::{Local, NaiveDate};
use tempfile::tempdir;

fn create_test_storage() -> pocketflow::storage::sqlite_impl::SqliteStorage {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("test.db");
    pocketflow::storage::sqlite_impl::SqliteStorage::new(&db_path).unwrap()
}

#[test]
fn test_save_multiple_reopen_query() {
    let storage = create_test_storage();
    storage.migrate().unwrap();

    let m1 = Movement::new(
        MovementType::Ingreso,
        1_500_000,
        "Pago de quincena".into(),
        NaiveDate::from_ymd_opt(2026, 10, 15).unwrap(),
        MovementOrigin::Formulario,
    ).unwrap();
    let m2 = Movement::new(
        MovementType::Gasto,
        18_000,
        "Almuerzo".into(),
        Local::now().date_naive(),
        MovementOrigin::Formulario,
    ).unwrap();
    let m3 = Movement::new(
        MovementType::Gasto,
        12_000,
        "Transporte".into(),
        Local::now().date_naive(),
        MovementOrigin::Mensaje,
    ).unwrap();

    storage.save(m1.clone()).unwrap();
    storage.save(m2.clone()).unwrap();
    storage.save(m3.clone()).unwrap();

    // Simular cierre y reapertura creando nueva conexión al mismo archivo
    let db_path = storage.conn.path().unwrap().to_path_buf();
    drop(storage);

    let storage2 = pocketflow::storage::sqlite_impl::SqliteStorage::new(&db_path).unwrap();
    let all = storage2.query(None, None).unwrap();

    assert_eq!(all.len(), 3, "Deben persistir 3 movimientos tras reabrir");

    // Verificar datos intactos
    let ingreso = all.iter().find(|m| m.tipo == MovementType::Ingreso).unwrap();
    assert_eq!(ingreso.monto_centavos, 1_500_000);
    assert_eq!(ingreso.concepto, "Pago de quincena");

    let gastos: Vec<_> = all.iter().filter(|m| m.tipo == MovementType::Gasto).collect();
    assert_eq!(gastos.len(), 2);
}

#[test]
fn test_crash_safety_transaccion_atomica() {
    let storage = create_test_storage();
    storage.migrate().unwrap();

    let m = Movement::new(
        MovementType::Gasto,
        18_000,
        "Almuerzo".into(),
        Local::now().date_naive(),
        MovementOrigin::Formulario,
    ).unwrap();

    storage.save(m.clone()).unwrap();

    // Verificar que existe
    let retrieved = storage.get_by_id(m.id).unwrap().unwrap();
    assert_eq!(retrieved.monto_centavos, 18_000);

    // Simular crash después de commit - los datos deben persistir
    let db_path = storage.conn.path().unwrap().to_path_buf();
    drop(storage);

    let storage2 = pocketflow::storage::sqlite_impl::SqliteStorage::new(&db_path).unwrap();
    let retrieved = storage2.get_by_id(m.id).unwrap().unwrap();
    assert_eq!(retrieved.monto_centavos, 18_000, "Datos deben persistir tras crash simulado");
}

#[test]
fn test_update_persiste_tras_reabrir() {
    let storage = create_test_storage();
    storage.migrate().unwrap();

    let mut m = Movement::new(
        MovementType::Gasto,
        18_000,
        "Almuerzo".into(),
        Local::now().date_naive(),
        MovementOrigin::Formulario,
    ).unwrap();
    storage.save(m.clone()).unwrap();

    m = m.update(19_000, "Almuerzo corregido".into(), m.fecha).unwrap();
    storage.update(m.clone()).unwrap();

    let db_path = storage.conn.path().unwrap().to_path_buf();
    drop(storage);

    let storage2 = pocketflow::storage::sqlite_impl::SqliteStorage::new(&db_path).unwrap();
    let retrieved = storage2.get_by_id(m.id).unwrap().unwrap();
    assert_eq!(retrieved.monto_centavos, 19_000);
    assert_eq!(retrieved.concepto, "Almuerzo corregido");
}

#[test]
fn test_delete_persiste_tras_reabrir() {
    let storage = create_test_storage();
    storage.migrate().unwrap();

    let m = Movement::new(
        MovementType::Gasto,
        18_000,
        "Almuerzo".into(),
        Local::now().date_naive(),
        MovementOrigin::Formulario,
    ).unwrap();
    storage.save(m.clone()).unwrap();
    storage.delete(m.id).unwrap();

    let db_path = storage.conn.path().unwrap().to_path_buf();
    drop(storage);

    let storage2 = pocketflow::storage::sqlite_impl::SqliteStorage::new(&db_path).unwrap();
    let retrieved = storage2.get_by_id(m.id).unwrap();
    assert!(retrieved.is_none(), "Movimiento eliminado no debe reaparecer tras reabrir");
}