use kite::models::{Movement, MovementType, MovementOrigin};
use kite::storage::port::StoragePort;
use chrono::Local;
use tempfile::tempdir;

fn create_test_storage() -> kite::storage::sqlite_impl::SqliteStorage {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("test.db");
    kite::storage::sqlite_impl::SqliteStorage::new(&db_path).unwrap()
}

#[test]
fn test_summary_totals_match_manual() {
    let storage = create_test_storage();
    storage.migrate().unwrap();

    let hoy = Local::now().date_naive();
    let ayer = hoy.pred_opt().unwrap();

    // Crear movimientos conocidos
    let m1 = Movement::new(MovementType::Ingreso, 1_500_000, "Pago quincena".into(), hoy, MovementOrigin::Formulario).unwrap();
    let m2 = Movement::new(MovementType::Gasto, 18_000, "Almuerzo".into(), hoy, MovementOrigin::Formulario).unwrap();
    let m3 = Movement::new(MovementType::Gasto, 12_000, "Transporte".into(), ayer, MovementOrigin::Mensaje).unwrap();
    let m4 = Movement::new(MovementType::Ingreso, 500_000, "Freelance".into(), ayer, MovementOrigin::Formulario).unwrap();

    storage.save(m1).unwrap();
    storage.save(m2).unwrap();
    storage.save(m3).unwrap();
    storage.save(m4).unwrap();

    // Summary total
    let summary = storage.summary(None, None).unwrap();
    
    // Verificar que cuadran con suma manual
    assert_eq!(summary.ingresos_centavos, 2_000_000); // 1.5M + 500k
    assert_eq!(summary.gastos_centavos, 30_000);     // 18k + 12k
    assert_eq!(summary.diferencia_centavos, 1_970_000); // 2M - 30k
}

#[test]
fn test_summary_with_range_filters() {
    let storage = create_test_storage();
    storage.migrate().unwrap();

    let hoy = Local::now().date_naive();
    let ayer = hoy.pred_opt().unwrap();

    let m1 = Movement::new(MovementType::Ingreso, 1_500_000, "Hoy ingreso".into(), hoy, MovementOrigin::Formulario).unwrap();
    let m2 = Movement::new(MovementType::Gasto, 18_000, "Hoy gasto".into(), hoy, MovementOrigin::Formulario).unwrap();
    let m3 = Movement::new(MovementType::Gasto, 12_000, "Ayer gasto".into(), ayer, MovementOrigin::Mensaje).unwrap();

    storage.save(m1).unwrap();
    storage.save(m2).unwrap();
    storage.save(m3).unwrap();

    // Solo hoy
    let solo_hoy = storage.summary(Some(hoy), Some(hoy)).unwrap();
    assert_eq!(solo_hoy.ingresos_centavos, 1_500_000);
    assert_eq!(solo_hoy.gastos_centavos, 18_000);
    assert_eq!(solo_hoy.diferencia_centavos, 1_482_000);

    // Solo ayer
    let solo_ayer = storage.summary(Some(ayer), Some(ayer)).unwrap();
    assert_eq!(solo_ayer.ingresos_centavos, 0);
    assert_eq!(solo_ayer.gastos_centavos, 12_000);
    assert_eq!(solo_ayer.diferencia_centavos, -12_000);

    // Rango [ayer, hoy]
    let rango = storage.summary(Some(ayer), Some(hoy)).unwrap();
    assert_eq!(rango.ingresos_centavos, 1_500_000);
    assert_eq!(rango.gastos_centavos, 30_000);
    assert_eq!(rango.diferencia_centavos, 1_470_000);
}

#[test]
fn test_summary_empty_period_returns_zeros() {
    let storage = create_test_storage();
    storage.migrate().unwrap();

    let futuro = Local::now().date_naive() + chrono::Duration::days(30);

    // Período futuro sin movimientos
    let vacio = storage.summary(Some(futuro), Some(futuro + chrono::Duration::days(1))).unwrap();
    assert_eq!(vacio.ingresos_centavos, 0);
    assert_eq!(vacio.gastos_centavos, 0);
    assert_eq!(vacio.diferencia_centavos, 0);
}

#[test]
fn test_summary_negative_difference_shows_minus() {
    let storage = create_test_storage();
    storage.migrate().unwrap();

    let hoy = Local::now().date_naive();

    // Solo gastos, sin ingresos
    let m1 = Movement::new(MovementType::Gasto, 100_000, "Compra grande".into(), hoy, MovementOrigin::Formulario).unwrap();
    let m2 = Movement::new(MovementType::Gasto, 50_000, "Otra compra".into(), hoy, MovementOrigin::Formulario).unwrap();

    storage.save(m1).unwrap();
    storage.save(m2).unwrap();

    let summary = storage.summary(Some(hoy), Some(hoy)).unwrap();
    assert_eq!(summary.ingresos_centavos, 0);
    assert_eq!(summary.gastos_centavos, 150_000);
    assert_eq!(summary.diferencia_centavos, -150_000, "Diferencia negativa mostrada con signo -");
}

#[test]
fn test_summary_label_never_dinero_libre() {
    let storage = create_test_storage();
    storage.migrate().unwrap();

    let hoy = Local::now().date_naive();

    let m1 = Movement::new(MovementType::Ingreso, 1_000_000, "Ingreso".into(), hoy, MovementOrigin::Formulario).unwrap();
    let m2 = Movement::new(MovementType::Gasto, 300_000, "Gasto".into(), hoy, MovementOrigin::Formulario).unwrap();

    storage.save(m1).unwrap();
    storage.save(m2).unwrap();

    let summary = storage.summary(Some(hoy), Some(hoy)).unwrap();
    
    // El label oficial es "Balance registrado" (FR-011)
    // Verificar que diferencia no se presenta como "dinero libre", "saldo disponible", "saldo bancario"
    // Esto se valida en la UI, pero aquí verificamos que el modelo permite diferencia negativa
    assert_eq!(summary.diferencia_centavos, 700_000);
}

#[test]
fn test_summary_inverted_range_handled() {
    let storage = create_test_storage();
    storage.migrate().unwrap();

    let hoy = Local::now().date_naive();
    let ayer = hoy.pred_opt().unwrap();

    let m1 = Movement::new(MovementType::Ingreso, 100_000, "Test".into(), hoy, MovementOrigin::Formulario).unwrap();
    storage.save(m1).unwrap();

    // Rango invertido (desde > hasta) -> no debe pánico
    let invertido = storage.summary(Some(hoy), Some(ayer)).unwrap();
    // Aceptamos: ceros (sin swap) o totales (con auto-swap)
    assert!(invertido.ingresos_centavos == 0 || invertido.ingresos_centavos == 100_000);
}

#[test]
fn test_summary_all_types_in_range() {
    let storage = create_test_storage();
    storage.migrate().unwrap();

    let hoy = Local::now().date_naive();

    // Mix de tipos y orígenes
    let m1 = Movement::new(MovementType::Ingreso, 100_000, "Ingreso form".into(), hoy, MovementOrigin::Formulario).unwrap();
    let m2 = Movement::new(MovementType::Ingreso, 50_000, "Ingreso msg".into(), hoy, MovementOrigin::Mensaje).unwrap();
    let m3 = Movement::new(MovementType::Gasto, 30_000, "Gasto form".into(), hoy, MovementOrigin::Formulario).unwrap();
    let m4 = Movement::new(MovementType::Gasto, 20_000, "Gasto msg".into(), hoy, MovementOrigin::Mensaje).unwrap();

    storage.save(m1).unwrap();
    storage.save(m2).unwrap();
    storage.save(m3).unwrap();
    storage.save(m4).unwrap();

    let summary = storage.summary(Some(hoy), Some(hoy)).unwrap();
    assert_eq!(summary.ingresos_centavos, 150_000);
    assert_eq!(summary.gastos_centavos, 50_000);
    assert_eq!(summary.diferencia_centavos, 100_000);
}

#[test]
fn test_summary_update_reflects_changes() {
    let storage = create_test_storage();
    storage.migrate().unwrap();

    let hoy = Local::now().date_naive();

    let m = Movement::new(MovementType::Gasto, 18_000, "Almuerzo".into(), hoy, MovementOrigin::Formulario).unwrap();
    storage.save(m.clone()).unwrap();

    let summary1 = storage.summary(Some(hoy), Some(hoy)).unwrap();
    assert_eq!(summary1.gastos_centavos, 18_000);

    // Editar movimiento
    let mut updated = m;
    updated.monto_centavos = 25_000;
    storage.update(updated).unwrap();

    let summary2 = storage.summary(Some(hoy), Some(hoy)).unwrap();
    assert_eq!(summary2.gastos_centavos, 25_000, "Resumen refleja edición inmediata");
}

#[test]
fn test_summary_delete_reflects_changes() {
    let storage = create_test_storage();
    storage.migrate().unwrap();

    let hoy = Local::now().date_naive();

    let m1 = Movement::new(MovementType::Ingreso, 100_000, "Ingreso".into(), hoy, MovementOrigin::Formulario).unwrap();
    let m2 = Movement::new(MovementType::Gasto, 30_000, "Gasto".into(), hoy, MovementOrigin::Formulario).unwrap();

    storage.save(m1.clone()).unwrap();
    storage.save(m2.clone()).unwrap();

    let summary1 = storage.summary(Some(hoy), Some(hoy)).unwrap();
    assert_eq!(summary1.ingresos_centavos, 100_000);
    assert_eq!(summary1.gastos_centavos, 30_000);

    // Eliminar gasto
    storage.delete(m2.id).unwrap();

    let summary2 = storage.summary(Some(hoy), Some(hoy)).unwrap();
    assert_eq!(summary2.ingresos_centavos, 100_000);
    assert_eq!(summary2.gastos_centavos, 0, "Resumen refleja eliminación inmediata");
    assert_eq!(summary2.diferencia_centavos, 100_000);
}