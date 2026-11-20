//! Tests de la planificación quincenal (feature 003, T3/T4 — S2, S3, S5, S6).
//! Unit del cálculo puro + integration del ensamblado con SQLite real.

use chrono::{Datelike, Local, NaiveDate};
use kite::compromisos::port::CompromisosPort;
use kite::compromisos::sqlite_impl::SqliteCompromisosStorage;
use kite::compromisos::confirmar_pago;
use kite::config::{ConfigPort, SqliteConfigStorage};
use kite::models::{
    Compromiso, Duracion, EstadoCompromiso, Frecuencia, Movement, MovementOrigin, MovementType,
    Quincena,
};
use kite::planificacion::{
    calcular_plan, plan_de_estado, quincena_anterior, quincena_de_fecha, quincena_siguiente,
    PendienteCompromiso, RangoQuincena,
};
use kite::storage::port::StoragePort;
use kite::storage::sqlite_impl::SqliteStorage;
use tempfile::tempdir;
use uuid::Uuid;

fn d(y: i32, m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, day).unwrap()
}

fn rango(desde: NaiveDate, hasta: NaiveDate) -> RangoQuincena {
    RangoQuincena { desde, hasta }
}

/// Movimiento construido directo (sin validar fecha futura) para los tests puros.
fn mov(tipo: MovementType, monto: i64, fecha: NaiveDate) -> Movement {
    Movement {
        id: Uuid::new_v4(),
        tipo,
        monto_centavos: monto,
        concepto: "Prueba".into(),
        fecha,
        origen: MovementOrigin::Formulario,
        meta_id: None,
        creado_en: Local::now(),
        actualizado_en: Local::now(),
    }
}

fn pend(concepto: &str, monto: i64, fecha: NaiveDate) -> PendienteCompromiso {
    PendienteCompromiso {
        compromiso_id: Uuid::new_v4(),
        concepto: concepto.into(),
        monto_centavos: monto,
        fecha,
        periodo_key: format!("{:04}-{:02}", fecha.year(), fecha.month()),
    }
}

// --- Unit: quincena_de_fecha (S2) ---

#[test]
fn corte_15_dia_15_cae_en_la_primera() {
    assert_eq!(
        quincena_de_fecha(d(2026, 11, 15), 15),
        rango(d(2026, 11, 1), d(2026, 11, 15))
    );
    assert_eq!(
        quincena_de_fecha(d(2026, 11, 1), 15),
        rango(d(2026, 11, 1), d(2026, 11, 15))
    );
}

#[test]
fn corte_15_dia_16_cae_en_la_segunda() {
    assert_eq!(
        quincena_de_fecha(d(2026, 11, 16), 15),
        rango(d(2026, 11, 16), d(2026, 11, 30))
    );
}

#[test]
fn corte_28_en_febrero() {
    // Febrero no bisiesto: todo el mes es primera quincena (la segunda queda vacía).
    assert_eq!(
        quincena_de_fecha(d(2026, 2, 28), 28),
        rango(d(2026, 2, 1), d(2026, 2, 28))
    );
    // Bisiesto: el 29 es la segunda quincena, de un solo día.
    assert_eq!(
        quincena_de_fecha(d(2028, 2, 29), 28),
        rango(d(2028, 2, 29), d(2028, 2, 29))
    );
}

#[test]
fn hasta_es_el_ultimo_dia_real_del_mes() {
    assert_eq!(quincena_de_fecha(d(2026, 10, 20), 15).hasta, d(2026, 10, 31));
    assert_eq!(quincena_de_fecha(d(2026, 11, 20), 15).hasta, d(2026, 11, 30));
    assert_eq!(quincena_de_fecha(d(2026, 2, 20), 15).hasta, d(2026, 2, 28));
    assert_eq!(quincena_de_fecha(d(2028, 2, 20), 15).hasta, d(2028, 2, 29));
}

#[test]
#[should_panic(expected = "El día de corte debe estar entre 1 y 28")]
fn corte_0_es_invalido() {
    quincena_de_fecha(d(2026, 11, 10), 0);
}

#[test]
#[should_panic(expected = "El día de corte debe estar entre 1 y 28")]
fn corte_29_es_invalido() {
    quincena_de_fecha(d(2026, 11, 10), 29);
}

// --- Unit: navegación anterior/siguiente (S5) ---

#[test]
fn anterior_dentro_del_mismo_mes() {
    let segunda = quincena_de_fecha(d(2026, 11, 20), 15);
    assert_eq!(quincena_anterior(segunda), rango(d(2026, 11, 1), d(2026, 11, 15)));
}

#[test]
fn anterior_de_la_primera_cruza_al_mes_previo() {
    let primera = quincena_de_fecha(d(2026, 11, 10), 15);
    assert_eq!(quincena_anterior(primera), rango(d(2026, 10, 16), d(2026, 10, 31)));
    // Cruce de año.
    let enero = quincena_de_fecha(d(2027, 1, 5), 15);
    assert_eq!(quincena_anterior(enero), rango(d(2026, 12, 16), d(2026, 12, 31)));
}

#[test]
fn siguiente_dentro_del_mismo_mes() {
    let primera = quincena_de_fecha(d(2026, 11, 10), 15);
    assert_eq!(quincena_siguiente(primera), rango(d(2026, 11, 16), d(2026, 11, 30)));
}

#[test]
fn siguiente_de_la_segunda_cruza_al_mes_proximo() {
    let segunda = quincena_de_fecha(d(2026, 11, 20), 15);
    assert_eq!(quincena_siguiente(segunda), rango(d(2026, 12, 1), d(2026, 12, 15)));
    // Cruce de año.
    let diciembre = quincena_de_fecha(d(2026, 12, 20), 15);
    assert_eq!(quincena_siguiente(diciembre), rango(d(2027, 1, 1), d(2027, 1, 15)));
}

#[test]
fn navegacion_salta_la_segunda_vacia_de_febrero_con_corte_28() {
    // Febrero 2026 tiene 28 días: la segunda quincena (29..28) no existe y se salta.
    let primera_feb = quincena_de_fecha(d(2026, 2, 10), 28);
    assert_eq!(quincena_siguiente(primera_feb), rango(d(2026, 3, 1), d(2026, 3, 28)));
    let primera_mar = quincena_de_fecha(d(2026, 3, 10), 28);
    assert_eq!(quincena_anterior(primera_mar), rango(d(2026, 2, 1), d(2026, 2, 28)));
}

// --- Unit: calcular_plan (S3, casos del spec) ---

#[test]
fn caso_spec_libre_700_mil() {
    // US1.1: ingresos 1.500.000 − gastos 300.000 − pendiente 500.000 = 700.000.
    let r = rango(d(2026, 11, 1), d(2026, 11, 15));
    let movimientos = [
        mov(MovementType::Ingreso, 1_500_000, d(2026, 11, 1)),
        mov(MovementType::Gasto, 300_000, d(2026, 11, 5)),
    ];
    let pendientes = [pend("Arriendo", 500_000, d(2026, 11, 10))];
    let plan = calcular_plan(r, &movimientos, &pendientes, &[]);

    assert_eq!(plan.ingresos_centavos, 1_500_000);
    assert_eq!(plan.gastos_centavos, 300_000);
    assert_eq!(plan.pendientes_periodo.len(), 1);
    assert_eq!(plan.pendientes_periodo[0].concepto, "Arriendo");
    assert!(plan.atrasados.is_empty());
    assert_eq!(plan.libre_quincena_centavos, 700_000);
    assert_eq!(plan.libre_real_centavos, 700_000);
}

#[test]
fn caso_spec_atrasado_100_mil_da_libre_real_600_mil() {
    // US1.2: el atrasado se lista como componente y baja el libre real.
    let r = rango(d(2026, 11, 1), d(2026, 11, 15));
    let movimientos = [
        mov(MovementType::Ingreso, 1_500_000, d(2026, 11, 1)),
        mov(MovementType::Gasto, 300_000, d(2026, 11, 5)),
    ];
    let pendientes = [pend("Arriendo", 500_000, d(2026, 11, 10))];
    let atrasados = [pend("Servicio", 100_000, d(2026, 10, 20))];
    let plan = calcular_plan(r, &movimientos, &pendientes, &atrasados);

    assert_eq!(plan.libre_quincena_centavos, 700_000);
    assert_eq!(plan.libre_real_centavos, 600_000);
    assert_eq!(plan.atrasados.len(), 1);
    assert_eq!(plan.atrasados[0].concepto, "Servicio");
    assert_eq!(plan.atrasados[0].monto_centavos, 100_000);
}

#[test]
fn quincena_vacia_da_cero_sin_error() {
    // US1.4.
    let r = rango(d(2026, 11, 1), d(2026, 11, 15));
    let plan = calcular_plan(r, &[], &[], &[]);
    assert_eq!(plan.ingresos_centavos, 0);
    assert_eq!(plan.gastos_centavos, 0);
    assert!(plan.pendientes_periodo.is_empty());
    assert!(plan.atrasados.is_empty());
    assert_eq!(plan.libre_quincena_centavos, 0);
    assert_eq!(plan.libre_real_centavos, 0);
}

#[test]
fn libre_negativo_no_se_trunca() {
    // US1.5: el valor numérico queda negativo, sin ocultarlo.
    let r = rango(d(2026, 11, 1), d(2026, 11, 15));
    let movimientos = [
        mov(MovementType::Ingreso, 100_000, d(2026, 11, 1)),
        mov(MovementType::Gasto, 300_000, d(2026, 11, 5)),
    ];
    let pendientes = [pend("Arriendo", 500_000, d(2026, 11, 10))];
    let atrasados = [pend("Servicio", 100_000, d(2026, 10, 20))];
    let plan = calcular_plan(r, &movimientos, &pendientes, &atrasados);
    assert_eq!(plan.libre_quincena_centavos, -700_000);
    assert_eq!(plan.libre_real_centavos, -800_000);
}

#[test]
fn solo_cuenta_lo_que_cae_en_el_rango() {
    let r = rango(d(2026, 11, 1), d(2026, 11, 15));
    let movimientos = [
        mov(MovementType::Ingreso, 100_000, d(2026, 11, 10)), // dentro
        mov(MovementType::Ingreso, 999_000, d(2026, 10, 31)), // antes
        mov(MovementType::Gasto, 999_000, d(2026, 11, 16)),   // después
    ];
    let pendientes = [
        pend("Dentro", 50_000, d(2026, 11, 15)),
        pend("Fuera", 999_000, d(2026, 11, 16)),
    ];
    // Un «atrasado» con fecha dentro del rango no es atrasado: no se descuenta.
    let atrasados = [pend("NoAtrasado", 999_000, d(2026, 11, 1))];
    let plan = calcular_plan(r, &movimientos, &pendientes, &atrasados);
    assert_eq!(plan.ingresos_centavos, 100_000);
    assert_eq!(plan.gastos_centavos, 0);
    assert_eq!(plan.pendientes_periodo.len(), 1);
    assert_eq!(plan.pendientes_periodo[0].concepto, "Dentro");
    assert!(plan.atrasados.is_empty());
    assert_eq!(plan.libre_quincena_centavos, 50_000);
    assert_eq!(plan.libre_real_centavos, 50_000);
}

#[test]
fn plan_serializa_para_la_ui() {
    let r = rango(d(2026, 11, 1), d(2026, 11, 15));
    let plan = calcular_plan(
        r,
        &[mov(MovementType::Ingreso, 1_500_000, d(2026, 11, 1))],
        &[pend("Arriendo", 500_000, d(2026, 11, 10))],
        &[pend("Servicio", 100_000, d(2026, 10, 20))],
    );
    let json = serde_json::to_string(&plan).unwrap();
    let recuperado: kite::planificacion::PlanQuincena = serde_json::from_str(&json).unwrap();
    assert_eq!(recuperado, plan);
}

// --- Integration: plan_de_estado con SQLite real (T4) ---

struct Entorno {
    movs: SqliteStorage,
    comps: SqliteCompromisosStorage,
    config: SqliteConfigStorage,
    _dir: tempfile::TempDir,
}

fn entorno() -> Entorno {
    let dir = tempdir().unwrap();
    let db = dir.path().join("kite.db");
    // Las tres implementaciones migran sobre el mismo archivo (movements primero:
    // compromisos_pagos referencia movements(id)).
    let movs = SqliteStorage::new(&db).unwrap();
    movs.migrate().unwrap();
    let comps = SqliteCompromisosStorage::new(&db).unwrap();
    comps.migrate().unwrap();
    let config = SqliteConfigStorage::new(&db).unwrap();
    config.migrate().unwrap();
    Entorno { movs, comps, config, _dir: dir }
}

fn compromiso_mensual(concepto: &str, monto: i64, dia: u8, creado: NaiveDate, duracion: Duracion) -> Compromiso {
    let mut c = Compromiso::new(concepto.into(), monto, Frecuencia::Mensual(dia), duracion).unwrap();
    c.creado_en = creado.and_hms_opt(10, 0, 0).unwrap().and_local_timezone(Local).unwrap();
    c
}

fn guardar_movimiento(movs: &SqliteStorage, tipo: MovementType, monto: i64, fecha: NaiveDate) {
    let m = Movement::new(tipo, monto, "Prueba".into(), fecha, MovementOrigin::Formulario).unwrap();
    movs.save(m).unwrap();
}

#[test]
fn plan_vacio_en_la_quincena_en_curso() {
    let e = entorno();
    let hoy = Local::now().date_naive();
    let plan = plan_de_estado(&e.movs, &e.comps, 15, None).unwrap();
    assert_eq!(plan.rango, quincena_de_fecha(hoy, 15));
    assert_eq!(plan.ingresos_centavos, 0);
    assert_eq!(plan.gastos_centavos, 0);
    assert!(plan.pendientes_periodo.is_empty());
    assert!(plan.atrasados.is_empty());
    assert_eq!(plan.libre_quincena_centavos, 0);
    assert_eq!(plan.libre_real_centavos, 0);
}

#[test]
fn caso_spec_con_sqlite_real() {
    let e = entorno();
    let hoy = Local::now().date_naive();
    let r = quincena_de_fecha(hoy, 15);
    guardar_movimiento(&e.movs, MovementType::Ingreso, 1_500_000, r.desde);
    guardar_movimiento(&e.movs, MovementType::Gasto, 300_000, r.desde);
    // Vence hoy (dentro de la quincena en curso), sin pagar.
    let c = compromiso_mensual("Arriendo", 500_000, hoy.day() as u8, hoy, Duracion::Indefinido);
    e.comps.save(c).unwrap();

    let plan = plan_de_estado(&e.movs, &e.comps, 15, Some(hoy)).unwrap();
    assert_eq!(plan.rango, r);
    assert_eq!(plan.ingresos_centavos, 1_500_000);
    assert_eq!(plan.gastos_centavos, 300_000);
    assert_eq!(plan.pendientes_periodo.len(), 1);
    assert_eq!(plan.pendientes_periodo[0].concepto, "Arriendo");
    assert_eq!(plan.pendientes_periodo[0].monto_centavos, 500_000);
    assert_eq!(plan.pendientes_periodo[0].fecha, hoy);
    assert!(plan.atrasados.is_empty());
    assert_eq!(plan.libre_quincena_centavos, 700_000);
    assert_eq!(plan.libre_real_centavos, 700_000);
}

#[test]
fn pagado_no_descuenta_como_pendiente() {
    // US1.3 / FR-P04: el pago ya está en los gastos; no se descuenta dos veces.
    let e = entorno();
    let hoy = Local::now().date_naive();
    let r = quincena_de_fecha(hoy, 15);
    guardar_movimiento(&e.movs, MovementType::Ingreso, 1_500_000, r.desde);
    guardar_movimiento(&e.movs, MovementType::Gasto, 300_000, r.desde);
    let c = compromiso_mensual("Arriendo", 500_000, hoy.day() as u8, hoy, Duracion::Indefinido);
    let saved = e.comps.save(c).unwrap();
    let periodo = saved.periodo_key_en_mes(hoy.year(), hoy.month());
    confirmar_pago(&e.movs, &e.comps, &saved, periodo, 500_000, hoy).unwrap();

    let plan = plan_de_estado(&e.movs, &e.comps, 15, Some(hoy)).unwrap();
    assert!(plan.pendientes_periodo.is_empty());
    assert!(plan.atrasados.is_empty());
    assert_eq!(plan.gastos_centavos, 800_000); // 300.000 + el pago de 500.000
    assert_eq!(plan.libre_quincena_centavos, 700_000);
    assert_eq!(plan.libre_real_centavos, 700_000);
}

#[test]
fn pausado_no_descuenta() {
    let e = entorno();
    let hoy = Local::now().date_naive();
    let c = compromiso_mensual("Arriendo", 500_000, hoy.day() as u8, hoy, Duracion::Indefinido);
    let saved = e.comps.save(c).unwrap();
    e.comps.set_estado(saved.id, EstadoCompromiso::Pausado).unwrap();

    let plan = plan_de_estado(&e.movs, &e.comps, 15, Some(hoy)).unwrap();
    assert!(plan.pendientes_periodo.is_empty());
    assert!(plan.atrasados.is_empty());
    assert_eq!(plan.libre_quincena_centavos, 0);
}

#[test]
fn cuotas_agotadas_no_descuenta() {
    let e = entorno();
    let hoy = Local::now().date_naive();
    let c = compromiso_mensual("Televisor", 120_000, hoy.day() as u8, hoy, Duracion::Cuotas(1));
    let saved = e.comps.save(c).unwrap();
    let periodo = saved.periodo_key_en_mes(hoy.year(), hoy.month());
    confirmar_pago(&e.movs, &e.comps, &saved, periodo, 120_000, hoy).unwrap();

    let plan = plan_de_estado(&e.movs, &e.comps, 15, Some(hoy)).unwrap();
    assert!(plan.pendientes_periodo.is_empty());
    assert!(plan.atrasados.is_empty());
    // El único gasto es el pago confirmado.
    assert_eq!(plan.gastos_centavos, 120_000);
    assert_eq!(plan.libre_quincena_centavos, -120_000);
}

#[test]
fn atrasado_de_varios_periodos_se_descuenta_una_sola_vez() {
    // Edge case del spec: el más reciente no pagado, una sola vez en «Libre real».
    let e = entorno();
    let hoy = Local::now().date_naive();
    let creado = hoy - chrono::Days::new(70);
    let c = compromiso_mensual("Servicio", 100_000, 1, creado, Duracion::Indefinido);
    let saved = e.comps.save(c).unwrap();

    let plan = plan_de_estado(&e.movs, &e.comps, 15, Some(hoy)).unwrap();
    let previas: Vec<NaiveDate> = saved
        .vencimientos_recordar(hoy, &[])
        .into_iter()
        .map(|v| v.fecha)
        .filter(|f| *f < plan.rango.desde)
        .collect();
    assert!(!previas.is_empty(), "el setup debe generar atrasados");
    assert_eq!(plan.atrasados.len(), 1);
    assert_eq!(plan.atrasados[0].fecha, *previas.iter().max().unwrap());
    assert_eq!(plan.atrasados[0].monto_centavos, 100_000);
    assert_eq!(plan.libre_real_centavos, plan.libre_quincena_centavos - 100_000);
}

#[test]
fn corte_configurado_reasigna_por_fecha_real() {
    // US2.4 / S6: quincenal «primera» que vence el 15 con corte 14 → SEGUNDA quincena.
    let e = entorno();
    e.config.set("dia_corte", "14").unwrap();
    let corte: u32 = e.config.get("dia_corte").unwrap().unwrap().parse().unwrap();
    assert_eq!(corte, 14);

    let hoy = Local::now().date_naive();
    let mut c = Compromiso::new(
        "Netflix".into(),
        20_000,
        Frecuencia::Quincenal(Quincena::Primera),
        Duracion::Indefinido,
    )
    .unwrap();
    c.creado_en = hoy.with_day(1).unwrap().and_hms_opt(10, 0, 0).unwrap().and_local_timezone(Local).unwrap();
    let saved = e.comps.save(c).unwrap();

    let plan14 = plan_de_estado(&e.movs, &e.comps, corte, Some(hoy)).unwrap();
    let plan15 = plan_de_estado(&e.movs, &e.comps, 15, Some(hoy)).unwrap();
    let venc15 = hoy.with_day(15).unwrap();

    // Con corte 14 el 15 abre la segunda quincena; con 15 cierra la primera.
    if plan14.rango.contiene(venc15) {
        assert_eq!(plan14.rango.desde, venc15);
        assert!(plan14
            .pendientes_periodo
            .iter()
            .any(|p| p.compromiso_id == saved.id && p.fecha == venc15));
    }
    if plan15.rango.contiene(venc15) {
        assert_eq!(plan15.rango.hasta, venc15);
    }
    // Ningún vencimiento del compromiso cae en la primera quincena con corte 14.
    for v in saved.vencimientos_recordar(hoy, &[]) {
        if plan14.rango.contiene(v.fecha) {
            assert_eq!(plan14.rango.desde.day(), 15);
            assert!(plan14
                .pendientes_periodo
                .iter()
                .any(|p| p.compromiso_id == saved.id && p.fecha == v.fecha));
        }
    }
}

#[test]
fn navegacion_anterior_siguiente_y_hoy() {
    let e = entorno();
    let hoy = Local::now().date_naive();
    guardar_movimiento(&e.movs, MovementType::Ingreso, 200_000, hoy);

    let actual = plan_de_estado(&e.movs, &e.comps, 15, None).unwrap();
    assert_eq!(actual.rango, quincena_de_fecha(hoy, 15));
    assert_eq!(actual.ingresos_centavos, 200_000);

    let rango_ant = quincena_anterior(actual.rango);
    let anterior = plan_de_estado(&e.movs, &e.comps, 15, Some(rango_ant.desde)).unwrap();
    assert_eq!(anterior.rango, rango_ant);
    assert_eq!(anterior.ingresos_centavos, 0);
    assert_eq!(anterior.gastos_centavos, 0);

    let rango_sig = quincena_siguiente(actual.rango);
    let siguiente = plan_de_estado(&e.movs, &e.comps, 15, Some(rango_sig.desde)).unwrap();
    assert_eq!(siguiente.rango, rango_sig);
    assert_eq!(siguiente.ingresos_centavos, 0);
    assert_eq!(siguiente.gastos_centavos, 0);

    // «Hoy» devuelve la quincena en curso con los mismos números.
    let de_vuelta = plan_de_estado(&e.movs, &e.comps, 15, None).unwrap();
    assert_eq!(de_vuelta, actual);
}

#[test]
fn corte_invalido_se_rechaza_con_mensaje() {
    let e = entorno();
    for corte in [0u32, 29] {
        let err = plan_de_estado(&e.movs, &e.comps, corte, None).unwrap_err();
        assert!(err.to_string().contains("entre 1 y 28"), "mensaje inesperado: {err}");
    }
}