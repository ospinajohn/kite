use chrono::{Local, NaiveDate};
use kite::compromisos::port::CompromisosPort;
use kite::compromisos::sqlite_impl::SqliteCompromisosStorage;
use kite::compromisos::{calcular_vencimientos, confirmar_pago};
use kite::models::{Compromiso, Duracion, EstadoCompromiso, Frecuencia, Quincena};
use kite::storage::port::StoragePort;
use kite::storage::sqlite_impl::SqliteStorage;
use tempfile::tempdir;

fn compromiso_con_fecha(dia: u8, creado: NaiveDate, duracion: Duracion) -> Compromiso {
    let mut c = Compromiso::new(
        "Arriendo".into(),
        500_000,
        Frecuencia::Mensual(dia),
        duracion,
    )
    .unwrap();
    c.creado_en = creado
        .and_hms_opt(10, 0, 0)
        .unwrap()
        .and_local_timezone(Local)
        .unwrap();
    c
}

#[test]
fn vencimientos_agrupa_atrasados_hoy_proximos() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("kite.db");
    let storage = SqliteCompromisosStorage::new(&db).unwrap();
    storage.migrate().unwrap();
    let hoy = Local::now().date_naive();

    // Atrasado: día de vencimiento de ayer.
    let ayer = hoy - chrono::Days::new(1);
    let c_atrasado = compromiso_con_fecha(
        ayer.day() as u8,
        ayer - chrono::Days::new(3),
        Duracion::Indefinido,
    );
    storage.save(c_atrasado).unwrap();

    // Hoy: vence hoy.
    let c_hoy = compromiso_con_fecha(
        hoy.day() as u8,
        hoy - chrono::Days::new(3),
        Duracion::Indefinido,
    );
    storage.save(c_hoy).unwrap();

    // Próximo: vence en 5 días.
    let proximo = hoy + chrono::Days::new(5);
    let c_prox = compromiso_con_fecha(
        proximo.day() as u8,
        hoy - chrono::Days::new(3),
        Duracion::Indefinido,
    );
    storage.save(c_prox).unwrap();

    let vencimientos = calcular_vencimientos(&storage, hoy).unwrap();
    assert_eq!(vencimientos.atrasados.len(), 1);
    assert_eq!(vencimientos.hoy.len(), 1);
    assert_eq!(vencimientos.proximos.len(), 1);
    assert_eq!(vencimientos.badge(), 2);
}

#[test]
fn flujo_pago_completo_dedup_y_cuotas() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("kite.db");
    let movements = SqliteStorage::new(&db).unwrap();
    movements.migrate().unwrap();
    let compromisos = SqliteCompromisosStorage::new(&db).unwrap();
    compromisos.migrate().unwrap();

    let hoy = Local::now().date_naive();
    let c = compromisos
        .save(compromiso_con_fecha(
            hoy.day() as u8,
            hoy,
            Duracion::Cuotas(2),
        ))
        .unwrap();
    let periodo = c.periodo_key_en_mes(hoy.year(), hoy.month());

    // Primer pago: crea el gasto y marca el período.
    let mov = confirmar_pago(&compromisos, &c, periodo.clone(), 480_000, hoy).unwrap();
    assert_eq!(mov.monto_centavos, 480_000);
    assert_eq!(mov.concepto, "Arriendo");

    // Aparece en el historial como gasto normal.
    assert_eq!(movements.query(None, None).unwrap().len(), 1);
    assert_eq!(
        compromisos.periodos_pagados(c.id).unwrap(),
        vec![periodo.clone()]
    );

    // Segundo pago del mismo período → rechazado (FR-C07).
    let err = confirmar_pago(&compromisos, &c, periodo, 500_000, hoy).unwrap_err();
    assert!(err.to_string().contains("ya tiene un pago"));
    // No se guardó ningún movimiento extra.
    assert_eq!(movements.query(None, None).unwrap().len(), 1);

    // Cuotas confirmadas = 1; tras el segundo período se agota.
    let c_rec = compromisos.get_by_id(c.id).unwrap().unwrap();
    assert_eq!(c_rec.cuotas_confirmadas, 1);
}

#[test]
fn pausado_no_aparece_en_vencimientos() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("kite.db");
    let storage = SqliteCompromisosStorage::new(&db).unwrap();
    storage.migrate().unwrap();
    let hoy = Local::now().date_naive();

    let c = compromiso_con_fecha(hoy.day() as u8, hoy, Duracion::Indefinido);
    let saved = storage.save(c).unwrap();
    storage
        .set_estado(saved.id, EstadoCompromiso::Pausado)
        .unwrap();

    let vencimientos = calcular_vencimientos(&storage, hoy).unwrap();
    assert_eq!(vencimientos.badge(), 0);
    assert!(vencimientos.atrasados.is_empty());
    assert!(vencimientos.hoy.is_empty());
}

#[test]
fn quincenal_vence_y_paga_en_su_periodo() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("kite.db");
    let movements = SqliteStorage::new(&db).unwrap();
    movements.migrate().unwrap();
    let compromisos = SqliteCompromisosStorage::new(&db).unwrap();
    compromisos.migrate().unwrap();

    let hoy = Local::now().date_naive();
    let c = Compromiso::new(
        "Netflix".into(),
        20_000,
        Frecuencia::Quincenal(Quincena::Primera),
        Duracion::Indefinido,
    )
    .unwrap();
    let saved = compromisos.save(c).unwrap();
    let periodo = saved.periodo_key_en_mes(hoy.year(), hoy.month());

    confirmar_pago(&compromisos, &saved, periodo.clone(), 20_000, hoy).unwrap();
    assert_eq!(
        compromisos.periodos_pagados(saved.id).unwrap(),
        vec![periodo]
    );
    assert_eq!(movements.query(None, None).unwrap().len(), 1);
}

use chrono::Datelike;
