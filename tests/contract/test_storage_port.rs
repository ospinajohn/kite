use kite::models::{Movement, MovementType, MovementOrigin};
use kite::storage::port::{StoragePort, PeriodSummary};
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

pub fn test_storage_port_query_ranges<S: StoragePort>(storage: &S) {
    storage.migrate().expect("migrate ok");

    let hoy = Local::now().date_naive();
    let ayer = hoy.pred_opt().unwrap();
    let anteayer = ayer.pred_opt().unwrap();
    let hace_una_semana = hoy - chrono::Duration::days(7);

    // Crear movimientos en diferentes fechas
    let m1 = Movement::new(MovementType::Gasto, 10_000, "Hoy".into(), hoy, MovementOrigin::Formulario).unwrap();
    let m2 = Movement::new(MovementType::Gasto, 20_000, "Ayer".into(), ayer, MovementOrigin::Formulario).unwrap();
    let m3 = Movement::new(MovementType::Gasto, 30_000, "Anteayer".into(), anteayer, MovementOrigin::Formulario).unwrap();
    let m4 = Movement::new(MovementType::Ingreso, 100_000, "Semana pasada".into(), hace_una_semana, MovementOrigin::Formulario).unwrap();

    storage.save(m1.clone()).unwrap();
    storage.save(m2.clone()).unwrap();
    storage.save(m3.clone()).unwrap();
    storage.save(m4.clone()).unwrap();

    // 1. Query sin rango -> todos (4)
    let all = storage.query(None, None).unwrap();
    assert_eq!(all.len(), 4, "query(None, None) debe retornar todos");

    // 2. Query solo desde -> desde fecha hasta hoy
    let desde_hoy = storage.query(Some(hoy), None).unwrap();
    assert_eq!(desde_hoy.len(), 1, "query(Some(hoy), None) debe retornar 1");
    assert_eq!(desde_hoy[0].concepto, "Hoy");

    // 3. Query solo hasta -> desde inicio hasta fecha
    let hasta_ayer = storage.query(None, Some(ayer)).unwrap();
    assert_eq!(hasta_ayer.len(), 3, "query(None, Some(ayer)) debe retornar 3 (ayer, anteayer, semana pasada)");

    // 4. Query rango exacto [ayer, hoy]
    let rango_ayer_hoy = storage.query(Some(ayer), Some(hoy)).unwrap();
    assert_eq!(rango_ayer_hoy.len(), 2, "query(Some(ayer), Some(hoy)) debe retornar 2");
    let conceptos: Vec<_> = rango_ayer_hoy.iter().map(|m| m.concepto.as_str()).collect();
    assert!(conceptos.contains(&"Hoy"));
    assert!(conceptos.contains(&"Ayer"));

    // 5. Query rango [anteayer, anteayer] -> solo 1
    let solo_anteayer = storage.query(Some(anteayer), Some(anteayer)).unwrap();
    assert_eq!(solo_anteayer.len(), 1);
    assert_eq!(solo_anteayer[0].concepto, "Anteayer");

    // 6. Rango sin resultados (futuro)
    let futuro = hoy + chrono::Duration::days(30);
    let rango_futuro = storage.query(Some(futuro), Some(futuro + chrono::Duration::days(1))).unwrap();
    assert_eq!(rango_futuro.len(), 0, "rango futuro debe retornar vacío");

    // 7. Rango invertido (desde > hasta) -> debe manejarse sin error (auto-swap o vacío)
    // La implementación decide: auto-swap o retornar vacío. Aquí verificamos que no pánico.
    let rango_invertido = storage.query(Some(hoy), Some(ayer)).unwrap();
    // Aceptamos ambas opciones: 0 (vacío) o 2 (auto-swap a [ayer, hoy])
    assert!(rango_invertido.len() == 0 || rango_invertido.len() == 2);

    // 8. Fecha muy antigua (antes de 1900) -> sin resultados
    let muy_antigua = NaiveDate::from_ymd_opt(1800, 1, 1).unwrap();
    let rango_antiguo = storage.query(Some(muy_antigua), Some(muy_antigua + chrono::Duration::days(1))).unwrap();
    assert_eq!(rango_antiguo.len(), 0);

    // 9. Verificar orden: fecha DESC, luego creado_en DESC
    // m1 (hoy) debe estar antes que m2 (ayer), etc.
    let ordenados = storage.query(None, None).unwrap();
    for i in 1..ordenados.len() {
        assert!(
            ordenados[i-1].fecha >= ordenados[i].fecha,
            "Orden debe ser fecha DESC: {} >= {}",
            ordenados[i-1].fecha, ordenados[i].fecha
        );
        if ordenados[i-1].fecha == ordenados[i].fecha {
            assert!(
                ordenados[i-1].creado_en >= ordenados[i].creado_en,
                "Mismo día: orden debe ser creado_en DESC"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kite::storage::sqlite_impl::SqliteStorage;
    use tempfile::tempdir;

    #[test]
    fn sqlite_storage_passes_contract() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let storage = SqliteStorage::new(&db_path).unwrap();
        test_storage_port(&storage);
    }

    #[test]
    fn sqlite_storage_passes_query_ranges_contract() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let storage = SqliteStorage::new(&db_path).unwrap();
        test_storage_port_query_ranges(&storage);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kite::storage::sqlite_impl::SqliteStorage;
    use tempfile::tempdir;

    #[test]
    fn sqlite_storage_passes_contract() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let storage = SqliteStorage::new(&db_path).unwrap();
        test_storage_port(&storage);
    }
}