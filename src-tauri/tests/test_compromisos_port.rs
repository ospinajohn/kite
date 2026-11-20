use chrono::{Local, NaiveDate};
use kite::compromisos::port::{CompromisosError, CompromisosPort};
use kite::compromisos::sqlite_impl::SqliteCompromisosStorage;
use kite::models::{
    Compromiso, Duracion, EstadoCompromiso, Frecuencia, Movement, MovementOrigin, MovementType,
    Quincena,
};
use kite::storage::port::StoragePort;
use kite::storage::sqlite_impl::SqliteStorage;
use tempfile::tempdir;
use uuid::Uuid;

fn nuevo_compromiso_mensual(dia: u8, duracion: Duracion) -> Compromiso {
    Compromiso::new(
        "Arriendo".into(),
        500_000,
        Frecuencia::Mensual(dia),
        duracion,
    )
    .unwrap()
}

fn movimiento_pago() -> Movement {
    Movement::new(
        MovementType::Gasto,
        480_000,
        "Arriendo".into(),
        Local::now().date_naive(),
        MovementOrigin::Formulario,
    )
    .unwrap()
}

#[test]
fn crud_roundtrip() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("kite.db");
    // La tabla compromisos_pagos referencia movements(id): se migra antes.
    SqliteStorage::new(&db).unwrap().migrate().unwrap();
    let storage = SqliteCompromisosStorage::new(&db).unwrap();
    storage.migrate().unwrap();

    let c = nuevo_compromiso_mensual(5, Duracion::Indefinido);
    let saved = storage.save(c.clone()).unwrap();
    assert_ne!(saved.id, Uuid::nil());

    let loaded = storage.get_by_id(saved.id).unwrap().unwrap();
    assert_eq!(loaded.concepto, "Arriendo");
    assert_eq!(loaded.monto_centavos, 500_000);
    assert_eq!(loaded.frecuencia, Frecuencia::Mensual(5));
    assert_eq!(loaded.duracion, Duracion::Indefinido);
    assert_eq!(loaded.estado, EstadoCompromiso::Activo);
    assert_eq!(loaded.creado_en, c.creado_en);

    // Quincenal y cuotas sobreviven el roundtrip.
    let cq = Compromiso::new(
        "Netflix".into(),
        20_000,
        Frecuencia::Quincenal(Quincena::Segunda),
        Duracion::Cuotas(12),
    )
    .unwrap();
    let saved_q = storage.save(cq.clone()).unwrap();
    let loaded_q = storage.get_by_id(saved_q.id).unwrap().unwrap();
    assert_eq!(
        loaded_q.frecuencia,
        Frecuencia::Quincenal(Quincena::Segunda)
    );
    assert_eq!(loaded_q.duracion, Duracion::Cuotas(12));

    let cf = Compromiso::new(
        "Seguro".into(),
        90_000,
        Frecuencia::Mensual(20),
        Duracion::FechaFin(NaiveDate::from_ymd_opt(2027, 6, 30).unwrap()),
    )
    .unwrap();
    let saved_f = storage.save(cf.clone()).unwrap();
    let loaded_f = storage.get_by_id(saved_f.id).unwrap().unwrap();
    assert_eq!(
        loaded_f.duracion,
        Duracion::FechaFin(NaiveDate::from_ymd_opt(2027, 6, 30).unwrap())
    );

    assert_eq!(storage.list().unwrap().len(), 3);

    // Update.
    let upd = loaded
        .update(
            "Arriendo nuevo".into(),
            550_000,
            Frecuencia::Mensual(5),
            Duracion::Indefinido,
        )
        .unwrap();
    let saved_upd = storage.update(upd.clone()).unwrap();
    let reloaded = storage.get_by_id(saved_upd.id).unwrap().unwrap();
    assert_eq!(reloaded.monto_centavos, 550_000);
    assert_eq!(reloaded.concepto, "Arriendo nuevo");

    // Update inexistente → NotFound.
    let mut fantasma = upd.clone();
    fantasma.id = Uuid::new_v4();
    match storage.update(fantasma) {
        Err(CompromisosError::NotFound(_)) => {}
        other => panic!("esperaba NotFound, obtuve: {other:?}"),
    }

    // Pausar / reactivar.
    let pausado = storage
        .set_estado(saved_upd.id, EstadoCompromiso::Pausado)
        .unwrap();
    assert_eq!(pausado.estado, EstadoCompromiso::Pausado);
    let activo = storage
        .set_estado(saved_upd.id, EstadoCompromiso::Activo)
        .unwrap();
    assert_eq!(activo.estado, EstadoCompromiso::Activo);
    match storage.set_estado(Uuid::new_v4(), EstadoCompromiso::Pausado) {
        Err(CompromisosError::NotFound(_)) => {}
        other => panic!("esperaba NotFound, obtuve: {other:?}"),
    }

    // Delete.
    assert!(storage.delete(saved_q.id).unwrap());
    assert!(storage.get_by_id(saved_q.id).unwrap().is_none());
    assert!(!storage.delete(saved_q.id).unwrap());
}

#[test]
fn pago_duplicado_rechazado_y_historial() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("kite.db");
    let movements = SqliteStorage::new(&db).unwrap();
    movements.migrate().unwrap();
    let storage = SqliteCompromisosStorage::new(&db).unwrap();
    storage.migrate().unwrap();

    let c = storage
        .save(nuevo_compromiso_mensual(5, Duracion::Cuotas(3)))
        .unwrap();

    let mov = movements.save(movimiento_pago()).unwrap();
    let pago1 = storage
        .registrar_pago(c.id, "2026-11".into(), mov.clone())
        .unwrap();
    assert_eq!(pago1.monto_pagado, 480_000);
    assert_eq!(pago1.periodo_key, "2026-11");

    // Deduplicación FR-C07: mismo período → Integrity.
    match storage.registrar_pago(c.id, "2026-11".into(), mov) {
        Err(CompromisosError::Integrity(msg)) => assert!(msg.contains("ya tiene un pago")),
        other => panic!("esperaba Integrity, obtuve: {other:?}"),
    }

    // Otro período sí pasa.
    let mov2 = movements.save(movimiento_pago()).unwrap();
    storage
        .registrar_pago(c.id, "2026-12".into(), mov2)
        .unwrap();

    assert_eq!(
        storage.periodos_pagados(c.id).unwrap(),
        vec!["2026-11", "2026-12"]
    );
    assert_eq!(storage.historial_pagos(c.id).unwrap().len(), 2);

    // El contador de cuotas incrementó con cada pago (FR-C11).
    let recargado = storage.get_by_id(c.id).unwrap().unwrap();
    assert_eq!(recargado.cuotas_confirmadas, 2);

    // Pago de compromiso inexistente → NotFound.
    let mov3 = movements.save(movimiento_pago()).unwrap();
    match storage.registrar_pago(Uuid::new_v4(), "2026-11".into(), mov3) {
        Err(CompromisosError::NotFound(_)) => {}
        other => panic!("esperaba NotFound, obtuve: {other:?}"),
    }
}

#[test]
fn delete_cascada_pagos_y_contador_reinicia() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("kite.db");
    let movements = SqliteStorage::new(&db).unwrap();
    movements.migrate().unwrap();
    let storage = SqliteCompromisosStorage::new(&db).unwrap();
    storage.migrate().unwrap();

    let c = storage
        .save(nuevo_compromiso_mensual(5, Duracion::Cuotas(3)))
        .unwrap();
    let mov = movements.save(movimiento_pago()).unwrap();
    storage.registrar_pago(c.id, "2026-11".into(), mov).unwrap();

    assert!(storage.delete(c.id).unwrap());
    assert!(storage.periodos_pagados(c.id).unwrap().is_empty());

    // El movimiento del pago sobrevive en el historial (spec: los pagos confirmados no se pierden).
    let historial = movements.query(None, None).unwrap();
    assert_eq!(historial.len(), 1);
}

#[test]
fn migrate_bump_user_version() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("kite.db");
    let storage = SqliteCompromisosStorage::new(&db).unwrap();
    storage.migrate().unwrap();
    // Segunda llamada idempotente.
    storage.migrate().unwrap();

    let conn = rusqlite::Connection::open(&db).unwrap();
    let version: i64 = conn
        .query_row("PRAGMA user_version", [], |r| r.get(0))
        .unwrap();
    assert!(version >= 2);
}
