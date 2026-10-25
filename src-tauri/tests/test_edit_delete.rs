use kite::models::{Movement, MovementType, MovementOrigin};
use kite::storage::port::StoragePort;
use chrono::{Local, NaiveDate};
use tempfile::tempdir;
use uuid::Uuid;

fn create_test_storage() -> kite::storage::sqlite_impl::SqliteStorage {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("test.db");
    kite::storage::sqlite_impl::SqliteStorage::new(&db_path).unwrap()
}

#[test]
fn test_edit_movement_updates_summary() {
    let storage = create_test_storage();
    storage.migrate().unwrap();

    let hoy = Local::now().date_naive();

    let m = Movement::new(MovementType::Gasto, 18_000, "Almuerzo".into(), hoy, MovementOrigin::Formulario).unwrap();
    storage.save(m.clone()).unwrap();

    // Verificar resumen inicial
    let summary1 = storage.summary(None, None).unwrap();
    assert_eq!(summary1.gastos_centavos, 18_000);

    // Editar movimiento
    let mut updated = m.clone();
    updated.monto_centavos = 19_000;
    updated.concepto = "Almuerzo corregido".into();
    updated.fecha = hoy - chrono::Duration::days(1);
    storage.update(updated).unwrap();

    // Verificar que el movimiento se actualizó
    let retrieved = storage.get_by_id(m.id).unwrap().unwrap();
    assert_eq!(retrieved.monto_centavos, 19_000);
    assert_eq!(retrieved.concepto, "Almuerzo corregido");

    // Verificar que el resumen refleja el cambio
    let summary2 = storage.summary(None, None).unwrap();
    assert_eq!(summary2.gastos_centavos, 19_000);
}

#[test]
fn test_edit_movement_preserves_id_type_origen() {
    let storage = create_test_storage();
    storage.migrate().unwrap();

    let hoy = Local::now().date_naive();

    let m = Movement::new(MovementType::Ingreso, 100_000, "Pago".into(), hoy, MovementOrigin::Mensaje).unwrap();
    storage.save(m.clone()).unwrap();

    let mut updated = m.clone();
    updated.monto_centavos = 150_000;
    updated.concepto = "Pago corregido".into();
    storage.update(updated).unwrap();

    let retrieved = storage.get_by_id(m.id).unwrap().unwrap();
    assert_eq!(retrieved.id, m.id);
    assert_eq!(retrieved.tipo, MovementType::Ingreso);
    assert_eq!(retrieved.origen, MovementOrigin::Mensaje);
}

#[test]
fn test_edit_movement_refresca_actualizado_en() {
    let storage = create_test_storage();
    storage.migrate().unwrap();

    let hoy = Local::now().date_naive();

    let m = Movement::new(MovementType::Gasto, 10_000, "Test".into(), hoy, MovementOrigin::Formulario).unwrap();
    storage.save(m.clone()).unwrap();

    let original_actualizado = m.actualizado_en;
    // Asegurar que pase al menos 1 segundo para diferencia de timestamp
    std::thread::sleep(std::time::Duration::from_millis(1100));

    // Usar Movement::update() para crear el movimiento actualizado con nuevo timestamp
    let updated = m.update(11_000, "Test".into(), hoy).unwrap();
    storage.update(updated).unwrap();

    let retrieved = storage.get_by_id(m.id).unwrap().unwrap();
    // actualizado_en debe ser estrictamente mayor (diferencia >= 1 segundo)
    assert!(retrieved.actualizado_en > original_actualizado, 
        "actualizado_en debe refrescarse: original={:?}, nuevo={:?}", 
        original_actualizado, retrieved.actualizado_en);
}

#[test]
fn test_delete_movement_with_confirmation() {
    let storage = create_test_storage();
    storage.migrate().unwrap();

    let hoy = Local::now().date_naive();

    let m = Movement::new(MovementType::Gasto, 18_000, "Almuerzo".into(), hoy, MovementOrigin::Formulario).unwrap();
    storage.save(m.clone()).unwrap();

    // Verificar que existe
    let before = storage.get_by_id(m.id).unwrap();
    assert!(before.is_some());

    // Eliminar
    let deleted = storage.delete(m.id).unwrap();
    assert!(deleted);

    // Verificar que ya no existe
    let after = storage.get_by_id(m.id).unwrap();
    assert!(after.is_none());

    // Verificar en query
    let all = storage.query(None, None).unwrap();
    assert_eq!(all.len(), 0);
}

#[test]
fn test_delete_movement_cancelled_persists() {
    let storage = create_test_storage();
    storage.migrate().unwrap();

    let hoy = Local::now().date_naive();

    let m = Movement::new(MovementType::Gasto, 18_000, "Almuerzo".into(), hoy, MovementOrigin::Formulario).unwrap();
    storage.save(m.clone()).unwrap();

    // Simular cancelación: no llamamos a delete
    // Verificar que persiste
    let all = storage.query(None, None).unwrap();
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].concepto, "Almuerzo");
}

#[test]
fn test_delete_nonexistent_returns_false() {
    let storage = create_test_storage();
    storage.migrate().unwrap();

    let deleted = storage.delete(Uuid::nil()).unwrap();
    assert!(!deleted);
}

#[test]
fn test_edit_last_movement_period_summary_zeros() {
    let storage = create_test_storage();
    storage.migrate().unwrap();

    let hoy = Local::now().date_naive();

    let m = Movement::new(MovementType::Gasto, 10_000, "Último".into(), hoy, MovementOrigin::Formulario).unwrap();
    storage.save(m.clone()).unwrap();

    // Verificar resumen antes
    let summary1 = storage.summary(None, None).unwrap();
    assert_eq!(summary1.gastos_centavos, 10_000);

    // Eliminar el único movimiento
    storage.delete(m.id).unwrap();

    // Verificar resumen en ceros
    let summary2 = storage.summary(None, None).unwrap();
    assert_eq!(summary2.gastos_centavos, 0);
    assert_eq!(summary2.ingresos_centavos, 0);
    assert_eq!(summary2.diferencia_centavos, 0);
}

#[test]
fn test_edit_movement_invalid_validation() {
    let storage = create_test_storage();
    storage.migrate().unwrap();

    let hoy = Local::now().date_naive();

    let m = Movement::new(MovementType::Gasto, 18_000, "Almuerzo".into(), hoy, MovementOrigin::Formulario).unwrap();
    storage.save(m.clone()).unwrap();

    // Intentar editar con monto inválido (0)
    let mut updated = m;
    updated.monto_centavos = 0;
    let result = updated.update(0, "Inválido".into(), hoy);

    // La validación debería fallar en el modelo
    assert!(result.is_err());
}

#[test]
fn test_delete_movement_updates_summary() {
    let storage = create_test_storage();
    storage.migrate().unwrap();

    let hoy = Local::now().date_naive();

    let m1 = Movement::new(MovementType::Gasto, 10_000, "Gasto 1".into(), hoy, MovementOrigin::Formulario).unwrap();
    let m2 = Movement::new(MovementType::Ingreso, 50_000, "Ingreso 1".into(), hoy, MovementOrigin::Formulario).unwrap();

    storage.save(m1.clone()).unwrap();
    storage.save(m2.clone()).unwrap();

    let summary1 = storage.summary(None, None).unwrap();
    assert_eq!(summary1.gastos_centavos, 10_000);
    assert_eq!(summary1.ingresos_centavos, 50_000);
    assert_eq!(summary1.diferencia_centavos, 40_000);

    // Eliminar el gasto
    storage.delete(m1.id).unwrap();

    let summary2 = storage.summary(None, None).unwrap();
    assert_eq!(summary2.gastos_centavos, 0);
    assert_eq!(summary2.ingresos_centavos, 50_000);
    assert_eq!(summary2.diferencia_centavos, 50_000);
}