//! Feature 005 «Ahorro y metas» (T3/T4/T5 — S2, S3, S6, S7):
//! - Unit de las funciones puras `recalcular_estado` y `a_ritmo`.
//! - Contract de `MetasPort` (CRUD, cancelar/reactivar conserva acumulado).
//! - Integration del flujo completo del spec con SQLite real y la lógica de
//!   los comandos de `app.rs` (funciones libres testeables sin Tauri State).

use chrono::{Local, NaiveDate};
use kite::app::{actualizar_meta, alta_apartado, borrar_apartado, editar_apartado, listar_metas};
use kite::compromisos::port::CompromisosPort;
use kite::compromisos::sqlite_impl::SqliteCompromisosStorage;
use kite::config::{ConfigPort, SqliteConfigStorage};
use kite::metas::{
    a_ritmo, recalcular_estado, recalcular_estado_meta, EstadoMeta, Meta, MetasError, MetasPort,
    SqliteMetasStorage,
};
use kite::models::{Movement, MovementOrigin, MovementType};
use kite::planificacion::plan_de_estado;
use kite::storage::port::StoragePort;
use kite::storage::sqlite_impl::SqliteStorage;
use tempfile::tempdir;
use uuid::Uuid;

fn d(y: i32, m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, day).unwrap()
}

// --- Unit: recalcular_estado (S6/FR-M04) ---

#[test]
fn recalcular_estado_casos() {
    assert_eq!(recalcular_estado(0, 2_000_000), EstadoMeta::Activa);
    assert_eq!(recalcular_estado(1_999_999, 2_000_000), EstadoMeta::Activa);
    assert_eq!(
        recalcular_estado(2_000_000, 2_000_000),
        EstadoMeta::Completada
    );
    // Meta sobrepasada: sigue completada (edge case del spec).
    assert_eq!(
        recalcular_estado(2_500_000, 2_000_000),
        EstadoMeta::Completada
    );
}

// --- Unit: a_ritmo (S4/FR-M08) ---
// Periodo de referencia: inicio 2026-11-01, fecha objetivo 2026-12-01 (30 días).

#[test]
fn a_ritmo_sin_apartados_con_plazo_corrido_es_atrasado() {
    // 10 de 30 días transcurridos: se espera 100.000 de 300.000.
    assert!(!a_ritmo(
        0,
        300_000,
        d(2026, 11, 1),
        d(2026, 11, 11),
        d(2026, 12, 1)
    ));
}

#[test]
fn a_ritmo_exacto_y_por_debajo() {
    assert!(a_ritmo(
        100_000,
        300_000,
        d(2026, 11, 1),
        d(2026, 11, 11),
        d(2026, 12, 1)
    ));
    assert!(!a_ritmo(
        99_999,
        300_000,
        d(2026, 11, 1),
        d(2026, 11, 11),
        d(2026, 12, 1)
    ));
}

#[test]
fn a_ritmo_meta_sobrepasada_siempre_a_ritmo() {
    assert!(a_ritmo(
        400_000,
        300_000,
        d(2026, 11, 1),
        d(2026, 11, 11),
        d(2026, 12, 1)
    ));
    // Incluso al inicio del plazo.
    assert!(a_ritmo(
        300_000,
        300_000,
        d(2026, 11, 1),
        d(2026, 11, 1),
        d(2026, 12, 1)
    ));
}

#[test]
fn a_ritmo_al_inicio_no_exige_nada() {
    assert!(a_ritmo(
        0,
        300_000,
        d(2026, 11, 1),
        d(2026, 11, 1),
        d(2026, 12, 1)
    ));
}

#[test]
fn a_ritmo_con_el_plazo_vencido_exige_el_objetivo() {
    let vencido = d(2026, 12, 1);
    assert!(!a_ritmo(299_999, 300_000, d(2026, 11, 1), vencido, vencido));
    assert!(a_ritmo(300_000, 300_000, d(2026, 11, 1), vencido, vencido));
    // Después de la fecha objetivo también.
    assert!(!a_ritmo(
        100_000,
        300_000,
        d(2026, 11, 1),
        d(2026, 12, 10),
        vencido
    ));
}

#[test]
fn a_ritmo_plazo_no_positivo_se_resuelve_con_el_objetivo() {
    // fecha_objetivo <= inicio: sin plazo válido, solo cuenta alcanzar el objetivo.
    assert!(!a_ritmo(
        100_000,
        300_000,
        d(2026, 11, 1),
        d(2026, 11, 1),
        d(2026, 11, 1)
    ));
    assert!(a_ritmo(
        300_000,
        300_000,
        d(2026, 11, 1),
        d(2026, 11, 1),
        d(2026, 11, 1)
    ));
}

// --- Contract: MetasPort sobre SQLite real ---

struct Entorno {
    movs: SqliteStorage,
    comps: SqliteCompromisosStorage,
    metas: SqliteMetasStorage,
    _config: SqliteConfigStorage,
    _dir: tempfile::TempDir,
}

fn entorno() -> Entorno {
    let dir = tempdir().unwrap();
    let db = dir.path().join("kite.db");
    // movements primero: compromisos_pagos referencia movements(id) y la
    // migración 004 reconstruye movements.
    let movs = SqliteStorage::new(&db).unwrap();
    movs.migrate().unwrap();
    let comps = SqliteCompromisosStorage::new(&db).unwrap();
    comps.migrate().unwrap();
    let config = SqliteConfigStorage::new(&db).unwrap();
    config.migrate().unwrap();
    let metas = SqliteMetasStorage::new(&db).unwrap();
    metas.migrate().unwrap();
    Entorno {
        movs,
        comps,
        metas,
        _config: config,
        _dir: dir,
    }
}

fn meta_viaje() -> Meta {
    Meta::new("Viaje a la costa".into(), 2_000_000, None).unwrap()
}

#[test]
fn crud_roundtrip_metas() {
    let e = entorno();
    let m = meta_viaje();
    let saved = e.metas.save(m.clone()).unwrap();
    assert_ne!(saved.id, Uuid::nil());
    assert_eq!(saved.estado, EstadoMeta::Activa);

    let loaded = e.metas.get_by_id(saved.id).unwrap().unwrap();
    assert_eq!(loaded.nombre, "Viaje a la costa");
    assert_eq!(loaded.monto_objetivo_centavos, 2_000_000);
    assert_eq!(loaded.fecha_objetivo, None);
    assert_eq!(loaded.creado_en, m.creado_en);

    // Con fecha objetivo futura sobrevive el roundtrip.
    let futura = Local::now().date_naive() + chrono::Days::new(30);
    let mf = Meta::new("Fondo".into(), 1_000_000, Some(futura)).unwrap();
    let saved_f = e.metas.save(mf).unwrap();
    assert_eq!(
        e.metas
            .get_by_id(saved_f.id)
            .unwrap()
            .unwrap()
            .fecha_objetivo,
        Some(futura)
    );

    assert_eq!(e.metas.list().unwrap().len(), 2);

    // Update.
    let upd = loaded
        .update("Viaje al Caribe".into(), 2_500_000, Some(futura))
        .unwrap();
    e.metas.update(upd.clone()).unwrap();
    let reloaded = e.metas.get_by_id(saved.id).unwrap().unwrap();
    assert_eq!(reloaded.nombre, "Viaje al Caribe");
    assert_eq!(reloaded.monto_objetivo_centavos, 2_500_000);
    assert_eq!(reloaded.fecha_objetivo, Some(futura));
    assert_eq!(reloaded.creado_en, m.creado_en);

    // Update inexistente → NotFound.
    let mut fantasma = upd;
    fantasma.id = Uuid::new_v4();
    assert!(matches!(
        e.metas.update(fantasma),
        Err(MetasError::NotFound(_))
    ));
    assert!(matches!(e.metas.get_by_id(Uuid::new_v4()).unwrap(), None));
    assert!(matches!(
        e.metas.cancelar(Uuid::new_v4()),
        Err(MetasError::NotFound(_))
    ));
    assert!(matches!(
        e.metas.reactivar(Uuid::new_v4()),
        Err(MetasError::NotFound(_))
    ));
}

#[test]
fn validaciones_de_meta() {
    assert!(matches!(
        Meta::new("   ".into(), 1000, None),
        Err(MetasError::InvalidNombre(_))
    ));
    assert!(matches!(
        Meta::new("X".into(), 0, None),
        Err(MetasError::InvalidMonto(_))
    ));
    assert!(matches!(
        Meta::new("X".into(), -5, None),
        Err(MetasError::InvalidMonto(_))
    ));
    let hoy = Local::now().date_naive();
    assert!(matches!(
        Meta::new("X".into(), 1000, Some(hoy)),
        Err(MetasError::InvalidFecha(_))
    ));
    assert!(matches!(
        Meta::new("X".into(), 1000, Some(hoy - chrono::Days::new(1))),
        Err(MetasError::InvalidFecha(_))
    ));
    // El nombre se recorta (igual que el concepto de un movimiento).
    assert_eq!(
        Meta::new("  Viaje  ".into(), 1000, None).unwrap().nombre,
        "Viaje"
    );
}

#[test]
fn cancelar_conserva_apartados_y_reactivar_recalcula_estado() {
    let e = entorno();
    let hoy = Local::now().date_naive();
    let m = e.metas.save(meta_viaje()).unwrap();
    alta_apartado(&e.movs, &e.metas, m.id, 2_000_000, hoy).unwrap();
    assert_eq!(
        e.metas.get_by_id(m.id).unwrap().unwrap().estado,
        EstadoMeta::Completada
    );

    // Cancelar solo archiva: los apartados quedan como están (C4/FR-M06).
    let cancelada = e.metas.cancelar(m.id).unwrap();
    assert_eq!(cancelada.estado, EstadoMeta::Cancelada);
    let apartados = e.metas.apartados_de_meta(m.id).unwrap();
    assert_eq!(apartados.len(), 1);
    assert_eq!(apartados[0].monto_centavos, 2_000_000);

    // Cancelar dos veces se rechaza.
    assert!(matches!(
        e.metas.cancelar(m.id),
        Err(MetasError::NoPermitido(_))
    ));

    // Reactivar conserva el acumulado: como alcanza el objetivo, vuelve completada.
    let reactivada = e.metas.reactivar(m.id).unwrap();
    assert_eq!(reactivada.estado, EstadoMeta::Completada);
    assert_eq!(
        e.metas.apartados_de_meta(m.id).unwrap()[0].monto_centavos,
        2_000_000
    );

    // Reactivar una meta no cancelada se rechaza.
    assert!(matches!(
        e.metas.reactivar(m.id),
        Err(MetasError::NoPermitido(_))
    ));

    // Meta parcial: reactivar la deja activa.
    let m2 = e
        .metas
        .save(Meta::new("Parcial".into(), 1_000_000, None).unwrap())
        .unwrap();
    alta_apartado(&e.movs, &e.metas, m2.id, 200_000, hoy).unwrap();
    e.metas.cancelar(m2.id).unwrap();
    assert_eq!(e.metas.reactivar(m2.id).unwrap().estado, EstadoMeta::Activa);
}

#[test]
fn apartados_de_meta_filtra_por_meta_y_tipo() {
    let e = entorno();
    let hoy = Local::now().date_naive();
    let m1 = e.metas.save(meta_viaje()).unwrap();
    let m2 = e
        .metas
        .save(Meta::new("Otra".into(), 500_000, None).unwrap())
        .unwrap();

    let a1 = alta_apartado(
        &e.movs,
        &e.metas,
        m1.id,
        100_000,
        hoy - chrono::Days::new(2),
    )
    .unwrap();
    std::thread::sleep(std::time::Duration::from_millis(10));
    let a2 = alta_apartado(&e.movs, &e.metas, m1.id, 200_000, hoy).unwrap();
    alta_apartado(&e.movs, &e.metas, m2.id, 50_000, hoy).unwrap();
    // Un gasto con concepto parecido NO es un apartado de la meta.
    e.movs
        .save(
            Movement::new(
                MovementType::Gasto,
                999,
                "Apartado falso".into(),
                hoy,
                MovementOrigin::Formulario,
            )
            .unwrap(),
        )
        .unwrap();

    let de_m1 = e.metas.apartados_de_meta(m1.id).unwrap();
    assert_eq!(de_m1.len(), 2);
    // Orden del historial: fecha DESC, luego registro DESC.
    assert_eq!(de_m1[0].id, a2.id);
    assert_eq!(de_m1[1].id, a1.id);
    assert!(de_m1
        .iter()
        .all(|m| m.tipo == MovementType::Aporte && m.meta_id == Some(m1.id)));
    assert_eq!(e.metas.apartados_de_meta(m2.id).unwrap().len(), 1);
    assert!(e
        .metas
        .apartados_de_meta(Uuid::new_v4())
        .unwrap()
        .is_empty());
}

// --- Integration: flujo completo del spec (S2, S3, S6) ---

#[test]
fn flujo_spec_meta_apartados_progreso_completada_y_plan() {
    let e = entorno();
    let hoy = Local::now().date_naive();

    // Datos de la 001: ingreso y gasto de la quincena en curso.
    e.movs
        .save(
            Movement::new(
                MovementType::Ingreso,
                1_500_000,
                "Pago quincena".into(),
                hoy,
                MovementOrigin::Formulario,
            )
            .unwrap(),
        )
        .unwrap();
    e.movs
        .save(
            Movement::new(
                MovementType::Gasto,
                300_000,
                "Mercado".into(),
                hoy,
                MovementOrigin::Formulario,
            )
            .unwrap(),
        )
        .unwrap();
    let summary_antes = e.movs.summary(None, None).unwrap();
    assert_eq!(summary_antes.ingresos_centavos, 1_500_000);
    assert_eq!(summary_antes.gastos_centavos, 300_000);

    // US1.1: meta de 2.000.000 sin fecha → activa, progreso 0.
    let meta = e.metas.save(meta_viaje()).unwrap();
    assert_eq!(meta.estado, EstadoMeta::Activa);
    assert!(e.metas.apartados_de_meta(meta.id).unwrap().is_empty());

    // US2.1: apartado de 200.000 → progreso 200.000, historial como 'aporte'.
    let apartado = alta_apartado(&e.movs, &e.metas, meta.id, 200_000, hoy).unwrap();
    assert_eq!(apartado.tipo, MovementType::Aporte);
    assert_eq!(apartado.tipo.label(), "aporte");
    assert_eq!(apartado.meta_id, Some(meta.id));
    assert_eq!(apartado.origen, MovementOrigin::Formulario);
    assert_eq!(apartado.concepto, "Apartado · Viaje a la costa");
    assert_eq!(apartado.format_monto(), "→ $200.000");
    let historial = e.movs.query(None, None).unwrap();
    assert!(historial
        .iter()
        .any(|m| m.id == apartado.id && m.tipo == MovementType::Aporte));
    let acumulado: i64 = e
        .metas
        .apartados_de_meta(meta.id)
        .unwrap()
        .iter()
        .map(|m| m.monto_centavos)
        .sum();
    assert_eq!(acumulado, 200_000);
    assert_eq!(
        e.metas.get_by_id(meta.id).unwrap().unwrap().estado,
        EstadoMeta::Activa
    );

    // US2.2: el Resumen NO incluye el apartado (S2/FR-M02).
    let summary_despues = e.movs.summary(None, None).unwrap();
    assert_eq!(
        summary_despues.ingresos_centavos,
        summary_antes.ingresos_centavos
    );
    assert_eq!(
        summary_despues.gastos_centavos,
        summary_antes.gastos_centavos
    );

    // US2.3 / S3: Planificación resta el apartado del libre (números exactos).
    // 1.500.000 − 300.000 − 200.000 = 1.000.000 (sin compromisos).
    let plan = plan_de_estado(&e.movs, &e.comps, 15, Some(hoy)).unwrap();
    assert_eq!(plan.ingresos_centavos, 1_500_000);
    assert_eq!(plan.gastos_centavos, 300_000);
    assert_eq!(plan.apartados_periodo_centavos, 200_000);
    assert_eq!(plan.libre_quincena_centavos, 1_000_000);
    assert_eq!(plan.libre_real_centavos, 1_000_000);

    // US2.5 / FR-M04: alcanzar el objetivo completa la meta y rechaza nuevos apartados.
    let grande = alta_apartado(&e.movs, &e.metas, meta.id, 1_800_000, hoy).unwrap();
    assert_eq!(
        e.metas.get_by_id(meta.id).unwrap().unwrap().estado,
        EstadoMeta::Completada
    );
    let err = alta_apartado(&e.movs, &e.metas, meta.id, 1, hoy).unwrap_err();
    assert!(
        err.contains("no acepta apartados"),
        "mensaje inesperado: {err}"
    );

    // US3.3 / FR-M05: eliminar el apartado grande baja el acumulado y reactiva.
    assert!(borrar_apartado(&e.movs, &e.metas, grande.id).unwrap());
    assert_eq!(
        e.metas.get_by_id(meta.id).unwrap().unwrap().estado,
        EstadoMeta::Activa
    );
    let acumulado: i64 = e
        .metas
        .apartados_de_meta(meta.id)
        .unwrap()
        .iter()
        .map(|m| m.monto_centavos)
        .sum();
    assert_eq!(acumulado, 200_000);
    assert!(e.movs.get_by_id(grande.id).unwrap().is_none());

    // US3.1: editar el apartado recalcula el progreso (tipo y meta inmutables).
    let editado = editar_apartado(
        &e.movs,
        &e.metas,
        apartado.id,
        500_000,
        "Apartado corregido".into(),
        hoy,
    )
    .unwrap();
    assert_eq!(editado.tipo, MovementType::Aporte);
    assert_eq!(editado.meta_id, Some(meta.id));
    assert_eq!(editado.monto_centavos, 500_000);
    let acumulado: i64 = e
        .metas
        .apartados_de_meta(meta.id)
        .unwrap()
        .iter()
        .map(|m| m.monto_centavos)
        .sum();
    assert_eq!(acumulado, 500_000);
    // El plan refleja el nuevo monto: 1.500.000 − 300.000 − 500.000 = 700.000.
    let plan = plan_de_estado(&e.movs, &e.comps, 15, Some(hoy)).unwrap();
    assert_eq!(plan.apartados_periodo_centavos, 500_000);
    assert_eq!(plan.libre_quincena_centavos, 700_000);
    assert_eq!(plan.libre_real_centavos, 700_000);

    // Editar un movimiento que NO es apartado se rechaza.
    let gasto = e
        .movs
        .query(None, None)
        .unwrap()
        .into_iter()
        .find(|m| m.tipo == MovementType::Gasto)
        .unwrap();
    let err = editar_apartado(&e.movs, &e.metas, gasto.id, 1, "X".into(), hoy).unwrap_err();
    assert!(
        err.contains("no es un apartado"),
        "mensaje inesperado: {err}"
    );
    let err = borrar_apartado(&e.movs, &e.metas, gasto.id).unwrap_err();
    assert!(
        err.contains("no es un apartado"),
        "mensaje inesperado: {err}"
    );

    // US4.2 / C4: cancelar archiva sin devolución; reactivar conserva acumulado.
    e.metas.cancelar(meta.id).unwrap();
    let err = alta_apartado(&e.movs, &e.metas, meta.id, 10_000, hoy).unwrap_err();
    assert!(
        err.contains("no acepta apartados"),
        "mensaje inesperado: {err}"
    );
    let summary_cancelada = e.movs.summary(None, None).unwrap();
    assert_eq!(summary_cancelada.gastos_centavos, 300_000); // sin devolución
    e.metas.reactivar(meta.id).unwrap();
    let acumulado: i64 = e
        .metas
        .apartados_de_meta(meta.id)
        .unwrap()
        .iter()
        .map(|m| m.monto_centavos)
        .sum();
    assert_eq!(acumulado, 500_000);

    // Edge case: subir el objetivo de una meta completada la vuelve activa.
    alta_apartado(&e.movs, &e.metas, meta.id, 1_500_000, hoy).unwrap();
    assert_eq!(
        e.metas.get_by_id(meta.id).unwrap().unwrap().estado,
        EstadoMeta::Completada
    );
    let subida = actualizar_meta(
        &e.metas,
        meta.id,
        "Viaje a la costa".into(),
        3_000_000,
        None,
    )
    .unwrap();
    assert_eq!(subida.estado, EstadoMeta::Activa);

    // Persistencia: cerrar y reabrir conserva metas y apartados.
    let db = e._dir.path().join("kite.db");
    let meta_id = meta.id;
    // Cerrar conexiones pero conservar el directorio temporal hasta el fin del test.
    let _dir = {
        let Entorno { _dir, .. } = e;
        _dir
    };
    let movs = SqliteStorage::new(&db).unwrap();
    movs.migrate().unwrap();
    let metas = SqliteMetasStorage::new(&db).unwrap();
    metas.migrate().unwrap();
    let reabierta = metas.get_by_id(meta_id).unwrap().unwrap();
    assert_eq!(reabierta.nombre, "Viaje a la costa");
    assert_eq!(reabierta.monto_objetivo_centavos, 3_000_000);
    assert_eq!(reabierta.estado, EstadoMeta::Activa);
    let acumulado: i64 = metas
        .apartados_de_meta(meta_id)
        .unwrap()
        .iter()
        .map(|m| m.monto_centavos)
        .sum();
    assert_eq!(acumulado, 2_000_000);
}

#[test]
fn validaciones_de_apartado_y_meta_inexistente() {
    let e = entorno();
    let hoy = Local::now().date_naive();
    let meta = e.metas.save(meta_viaje()).unwrap();

    // US2.4: monto ≤ 0 o fecha futura se rechazan con mensaje.
    let err = alta_apartado(&e.movs, &e.metas, meta.id, 0, hoy).unwrap_err();
    assert!(err.contains("mayor que cero"), "mensaje inesperado: {err}");
    let err =
        alta_apartado(&e.movs, &e.metas, meta.id, 1000, hoy + chrono::Days::new(1)).unwrap_err();
    assert!(err.contains("futura"), "mensaje inesperado: {err}");
    // Nada quedó guardado.
    assert!(e.metas.apartados_de_meta(meta.id).unwrap().is_empty());
    assert_eq!(e.movs.query(None, None).unwrap().len(), 0);

    // Meta obligatoria: no existe → error.
    let err = alta_apartado(&e.movs, &e.metas, Uuid::new_v4(), 1000, hoy).unwrap_err();
    assert!(
        err.contains("Meta no encontrada"),
        "mensaje inesperado: {err}"
    );
    let err =
        editar_apartado(&e.movs, &e.metas, Uuid::new_v4(), 1000, "X".into(), hoy).unwrap_err();
    assert!(err.contains("no encontrado"), "mensaje inesperado: {err}");
}

#[test]
fn listar_metas_expone_progreso_y_ritmo() {
    let e = entorno();
    let hoy = Local::now().date_naive();
    let fecha_objetivo = hoy + chrono::Days::new(30);

    // Sin fecha → sin indicador de ritmo.
    let sin_fecha = e.metas.save(meta_viaje()).unwrap();
    alta_apartado(&e.movs, &e.metas, sin_fecha.id, 200_000, hoy).unwrap();

    // Con fecha y apartados → indicador presente (S4/FR-M08).
    let con_fecha = e
        .metas
        .save(Meta::new("Con fecha".into(), 300_000, Some(fecha_objetivo)).unwrap())
        .unwrap();
    alta_apartado(&e.movs, &e.metas, con_fecha.id, 100_000, hoy).unwrap();

    // Con fecha pero SIN apartados → el indicador no aparece (edge case del spec).
    let sin_apartados = e
        .metas
        .save(Meta::new("Sin apartados".into(), 300_000, Some(fecha_objetivo)).unwrap())
        .unwrap();

    let lista = listar_metas(&e.metas).unwrap();
    assert_eq!(lista.len(), 3);

    let ui_sin_fecha = lista.iter().find(|u| u.meta.id == sin_fecha.id).unwrap();
    assert_eq!(ui_sin_fecha.acumulado_centavos, 200_000);
    assert_eq!(ui_sin_fecha.a_ritmo, None);

    let ui_con_fecha = lista.iter().find(|u| u.meta.id == con_fecha.id).unwrap();
    assert_eq!(ui_con_fecha.acumulado_centavos, 100_000);
    // 100.000 de 300.000 con ~0 días transcurridos de 30: a ritmo.
    assert_eq!(
        ui_con_fecha.a_ritmo,
        Some(a_ritmo(
            100_000,
            300_000,
            con_fecha.creado_en.date_naive(),
            hoy,
            fecha_objetivo
        ))
    );

    let ui_sin_apartados = lista
        .iter()
        .find(|u| u.meta.id == sin_apartados.id)
        .unwrap();
    assert_eq!(ui_sin_apartados.acumulado_centavos, 0);
    assert_eq!(ui_sin_apartados.a_ritmo, None);
}

#[test]
fn recalcular_estado_meta_no_toca_canceladas() {
    let e = entorno();
    let hoy = Local::now().date_naive();
    let m = e.metas.save(meta_viaje()).unwrap();
    alta_apartado(&e.movs, &e.metas, m.id, 2_000_000, hoy).unwrap();
    e.metas.cancelar(m.id).unwrap();
    // Aunque el acumulado alcanza el objetivo, una cancelada sigue cancelada.
    let recargada = recalcular_estado_meta(&e.metas, m.id).unwrap();
    assert_eq!(recargada.estado, EstadoMeta::Cancelada);
    assert!(matches!(
        recalcular_estado_meta(&e.metas, Uuid::new_v4()),
        Err(MetasError::NotFound(_))
    ));
}

// --- Regresión de serialización (S3 compatibilidad) ---

#[test]
fn plan_quincena_sin_apartados_en_json_usa_default_cero() {
    let e = entorno();
    let hoy = Local::now().date_naive();
    let plan = plan_de_estado(&e.movs, &e.comps, 15, Some(hoy)).unwrap();
    assert_eq!(plan.apartados_periodo_centavos, 0);

    // Un consumidor viejo (JSON sin el campo nuevo) sigue deserializando.
    let mut json = serde_json::to_value(&plan).unwrap();
    json.as_object_mut()
        .unwrap()
        .remove("apartados_periodo_centavos");
    let recuperado: kite::planificacion::PlanQuincena = serde_json::from_value(json).unwrap();
    assert_eq!(recuperado.apartados_periodo_centavos, 0);
    assert_eq!(recuperado, plan);
}
