use kite::models::{Movement, MovementType, MovementOrigin};
use kite::storage::port::StoragePort;
use chrono::{Local, NaiveDate};
use tempfile::tempdir;

fn create_test_storage() -> kite::storage::sqlite_impl::SqliteStorage {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("test.db");
    kite::storage::sqlite_impl::SqliteStorage::new(&db_path).unwrap()
}

#[test]
fn test_history_query_all() {
    let storage = create_test_storage();
    storage.migrate().unwrap();

    let hoy = Local::now().date_naive();
    let ayer = hoy.pred_opt().unwrap();

    let m1 = Movement::new(MovementType::Gasto, 10_000, "Hoy".into(), hoy, MovementOrigin::Formulario).unwrap();
    let m2 = Movement::new(MovementType::Ingreso, 100_000, "Ayer ingreso".into(), ayer, MovementOrigin::Formulario).unwrap();

    storage.save(m1).unwrap();
    storage.save(m2).unwrap();

    let all = storage.query(None, None).unwrap();
    assert_eq!(all.len(), 2);
    // Orden: más reciente primero
    assert_eq!(all[0].concepto, "Hoy");
    assert_eq!(all[1].concepto, "Ayer ingreso");
}

#[test]
fn test_history_query_with_desde() {
    let storage = create_test_storage();
    storage.migrate().unwrap();

    let hoy = Local::now().date_naive();
    let ayer = hoy.pred_opt().unwrap();
    let anteayer = ayer.pred_opt().unwrap();

    let m1 = Movement::new(MovementType::Gasto, 10_000, "Hoy".into(), hoy, MovementOrigin::Formulario).unwrap();
    let m2 = Movement::new(MovementType::Gasto, 20_000, "Ayer".into(), ayer, MovementOrigin::Formulario).unwrap();
    let m3 = Movement::new(MovementType::Gasto, 30_000, "Anteayer".into(), anteayer, MovementOrigin::Formulario).unwrap();

    storage.save(m1).unwrap();
    storage.save(m2).unwrap();
    storage.save(m3).unwrap();

    // Query desde hoy
    let desde_hoy = storage.query(Some(hoy), None).unwrap();
    assert_eq!(desde_hoy.len(), 1);
    assert_eq!(desde_hoy[0].concepto, "Hoy");

    // Query desde ayer
    let desde_ayer = storage.query(Some(ayer), None).unwrap();
    assert_eq!(desde_ayer.len(), 2);
    assert!(desde_ayer.iter().any(|m| m.concepto == "Hoy"));
    assert!(desde_ayer.iter().any(|m| m.concepto == "Ayer"));
}

#[test]
fn test_history_query_with_hasta() {
    let storage = create_test_storage();
    storage.migrate().unwrap();

    let hoy = Local::now().date_naive();
    let ayer = hoy.pred_opt().unwrap();
    let anteayer = ayer.pred_opt().unwrap();

    let m1 = Movement::new(MovementType::Gasto, 10_000, "Hoy".into(), hoy, MovementOrigin::Formulario).unwrap();
    let m2 = Movement::new(MovementType::Gasto, 20_000, "Ayer".into(), ayer, MovementOrigin::Formulario).unwrap();
    let m3 = Movement::new(MovementType::Gasto, 30_000, "Anteayer".into(), anteayer, MovementOrigin::Formulario).unwrap();

    storage.save(m1).unwrap();
    storage.save(m2).unwrap();
    storage.save(m3).unwrap();

    // Query hasta ayer
    let hasta_ayer = storage.query(None, Some(ayer)).unwrap();
    assert_eq!(hasta_ayer.len(), 2);
    assert!(hasta_ayer.iter().any(|m| m.concepto == "Ayer"));
    assert!(hasta_ayer.iter().any(|m| m.concepto == "Anteayer"));
}

#[test]
fn test_history_query_range_exact() {
    let storage = create_test_storage();
    storage.migrate().unwrap();

    let hoy = Local::now().date_naive();
    let ayer = hoy.pred_opt().unwrap();
    let anteayer = ayer.pred_opt().unwrap();

    let m1 = Movement::new(MovementType::Gasto, 10_000, "Hoy".into(), hoy, MovementOrigin::Formulario).unwrap();
    let m2 = Movement::new(MovementType::Gasto, 20_000, "Ayer".into(), ayer, MovementOrigin::Formulario).unwrap();
    let m3 = Movement::new(MovementType::Gasto, 30_000, "Anteayer".into(), anteayer, MovementOrigin::Formulario).unwrap();

    storage.save(m1).unwrap();
    storage.save(m2).unwrap();
    storage.save(m3).unwrap();

    // Rango [ayer, hoy]
    let rango = storage.query(Some(ayer), Some(hoy)).unwrap();
    assert_eq!(rango.len(), 2);
    let conceptos: Vec<_> = rango.iter().map(|m| m.concepto.as_str()).collect();
    assert!(conceptos.contains(&"Hoy"));
    assert!(conceptos.contains(&"Ayer"));
}

#[test]
fn test_history_query_single_day() {
    let storage = create_test_storage();
    storage.migrate().unwrap();

    let hoy = Local::now().date_naive();

    let m1 = Movement::new(MovementType::Gasto, 10_000, "Hoy 1".into(), hoy, MovementOrigin::Formulario).unwrap();
    let m2 = Movement::new(MovementType::Gasto, 20_000, "Hoy 2".into(), hoy, MovementOrigin::Formulario).unwrap();

    storage.save(m1).unwrap();
    storage.save(m2).unwrap();

    let solo_hoy = storage.query(Some(hoy), Some(hoy)).unwrap();
    assert_eq!(solo_hoy.len(), 2);
    // Mismo día -> orden por creado_en DESC
    assert_eq!(solo_hoy[0].concepto, "Hoy 2"); // último insertado primero
    assert_eq!(solo_hoy[1].concepto, "Hoy 1");
}

#[test]
fn test_history_query_empty_range() {
    let storage = create_test_storage();
    storage.migrate().unwrap();

    let hoy = Local::now().date_naive();
    let futuro = hoy + chrono::Duration::days(30);

    let m1 = Movement::new(MovementType::Gasto, 10_000, "Hoy".into(), hoy, MovementOrigin::Formulario).unwrap();
    storage.save(m1).unwrap();

    // Rango futuro -> vacío
    let rango_futuro = storage.query(Some(futuro), Some(futuro + chrono::Duration::days(1))).unwrap();
    assert_eq!(rango_futuro.len(), 0);
}

#[test]
fn test_history_query_inverted_range() {
    let storage = create_test_storage();
    storage.migrate().unwrap();

    let hoy = Local::now().date_naive();
    let ayer = hoy.pred_opt().unwrap();

    let m1 = Movement::new(MovementType::Gasto, 10_000, "Hoy".into(), hoy, MovementOrigin::Formulario).unwrap();
    let m2 = Movement::new(MovementType::Gasto, 20_000, "Ayer".into(), ayer, MovementOrigin::Formulario).unwrap();

    storage.save(m1).unwrap();
    storage.save(m2).unwrap();

    // Rango invertido (desde > hasta) -> no debe panicar
    // Aceptamos 0 (vacío) o 2 (auto-swap)
    let invertido = storage.query(Some(hoy), Some(ayer)).unwrap();
    assert!(invertido.len() == 0 || invertido.len() == 2);
}

#[test]
fn test_history_query_very_old_date() {
    let storage = create_test_storage();
    storage.migrate().unwrap();

    let muy_antigua = NaiveDate::from_ymd_opt(1800, 1, 1).unwrap();

    let rango_antiguo = storage.query(Some(muy_antigua), Some(muy_antigua + chrono::Duration::days(1))).unwrap();
    assert_eq!(rango_antiguo.len(), 0);
}

#[test]
fn test_history_order_fecha_desc_creado_desc() {
    let storage = create_test_storage();
    storage.migrate().unwrap();

    let hoy = Local::now().date_naive();
    let ayer = hoy.pred_opt().unwrap();

    // Insertar en orden: hoy, ayer, hoy (de nuevo)
    let m1 = Movement::new(MovementType::Gasto, 10_000, "Hoy 1".into(), hoy, MovementOrigin::Formulario).unwrap();
    let m2 = Movement::new(MovementType::Gasto, 20_000, "Ayer".into(), ayer, MovementOrigin::Formulario).unwrap();
    let m3 = Movement::new(MovementType::Gasto, 30_000, "Hoy 2".into(), hoy, MovementOrigin::Formulario).unwrap();

    storage.save(m1).unwrap();
    storage.save(m2).unwrap();
    storage.save(m3).unwrap();

    let all = storage.query(None, None).unwrap();
    assert_eq!(all.len(), 3);

    // Orden: fecha DESC (hoy antes que ayer)
    // Dentro de hoy: creado_en DESC (m3 antes que m1)
    assert_eq!(all[0].concepto, "Hoy 2");
    assert_eq!(all[1].concepto, "Hoy 1");
    assert_eq!(all[2].concepto, "Ayer");

    // Verificar fechas
    assert_eq!(all[0].fecha, hoy);
    assert_eq!(all[1].fecha, hoy);
    assert_eq!(all[2].fecha, ayer);

    // Verificar creado_en dentro del mismo día
    assert!(all[0].creado_en >= all[1].creado_en);
}