use chrono::{Datelike, Local, NaiveDate};
use kite::compromisos::port::CompromisosPort;
use kite::compromisos::sqlite_impl::SqliteCompromisosStorage;
use kite::config::{ConfigPort, SqliteConfigStorage};
use kite::metas::port::MetasPort;
use kite::metas::sqlite_impl::SqliteMetasStorage;
use kite::models::{
    Compromiso, Duracion, Frecuencia, Movement, MovementOrigin, MovementType, Vencimiento,
};
use kite::simulacion_deuda::{
    calcular_escenario, generar_vencimientos_simulados, simular_impacto, DeudaSimulada,
    DuracionSimulacion,
};
use kite::storage::port::StoragePort;
use kite::storage::sqlite_impl::SqliteStorage;
use tempfile::tempdir;
use uuid::Uuid;

fn d(y: i32, m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, day).unwrap()
}

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

fn compromiso_base(
    concepto: &str,
    monto: i64,
    dia: u8,
    creado: NaiveDate,
    duracion: Duracion,
) -> Compromiso {
    let mut c =
        Compromiso::new(concepto.into(), monto, Frecuencia::Mensual(dia), duracion).unwrap();
    c.creado_en = creado
        .and_hms_opt(10, 0, 0)
        .unwrap()
        .and_local_timezone(Local)
        .unwrap();
    c
}

#[test]
fn escenario_descuenta_deuda_simulada_del_libre_base() {
    let deuda = DeudaSimulada {
        concepto: Some("Préstamo".into()),
        monto_cuota_centavos: 200_000,
        frecuencia: Frecuencia::Mensual(10),
        primer_vencimiento: d(2026, 11, 10),
        duracion: DuracionSimulacion::Cuotas(1),
        quincenas_proyeccion: None,
    };
    let cal = generar_vencimientos_simulados(&deuda, d(2026, 11, 1), 15).unwrap();
    let movimientos = [
        mov(MovementType::Ingreso, 1_500_000, d(2026, 11, 1)),
        mov(MovementType::Gasto, 300_000, d(2026, 11, 5)),
    ];
    let comp = compromiso_base("Arriendo", 500_000, 10, d(2026, 10, 1), Duracion::Indefinido);
    let vencs = vec![Vencimiento {
        periodo_key: "2026-11".into(),
        fecha: d(2026, 11, 10),
    }];
    let res = calcular_escenario(cal, &movimientos, &[(comp, vencs)], &[], d(2026, 11, 1), 15);

    assert_eq!(res.quincenas.len(), 1);
    let q = &res.quincenas[0];
    assert_eq!(q.base.libre_quincena_centavos, 700_000);
    assert_eq!(q.base.libre_real_centavos, 700_000);
    assert_eq!(q.total_simulado_centavos, 200_000);
    assert_eq!(q.libre_quincena_simulado_centavos, 500_000);
    assert_eq!(q.libre_real_simulado_centavos, 500_000);
}

#[test]
fn escenario_marca_conflicto_y_negativo_cuando_no_alcanza() {
    let deuda = DeudaSimulada {
        concepto: None,
        monto_cuota_centavos: 800_000,
        frecuencia: Frecuencia::Mensual(10),
        primer_vencimiento: d(2026, 11, 10),
        duracion: DuracionSimulacion::Cuotas(1),
        quincenas_proyeccion: None,
    };
    let cal = generar_vencimientos_simulados(&deuda, d(2026, 11, 1), 15).unwrap();
    let movimientos = [
        mov(MovementType::Ingreso, 1_500_000, d(2026, 11, 1)),
        mov(MovementType::Gasto, 300_000, d(2026, 11, 5)),
    ];
    let comp = compromiso_base("Arriendo", 500_000, 10, d(2026, 10, 1), Duracion::Indefinido);
    let vencs = vec![Vencimiento {
        periodo_key: "2026-11".into(),
        fecha: d(2026, 11, 10),
    }];
    let res = calcular_escenario(cal, &movimientos, &[(comp, vencs)], &[], d(2026, 11, 1), 15);
    let q = &res.quincenas[0];
    assert_eq!(q.libre_quincena_simulado_centavos, -100_000);
    assert!(q.conflicto_compromisos);
}

#[test]
fn escenario_incluye_atrasados_en_base_y_descuenta_simulado() {
    let deuda = DeudaSimulada {
        concepto: None,
        monto_cuota_centavos: 200_000,
        frecuencia: Frecuencia::Mensual(10),
        primer_vencimiento: d(2026, 11, 10),
        duracion: DuracionSimulacion::Cuotas(1),
        quincenas_proyeccion: None,
    };
    let cal = generar_vencimientos_simulados(&deuda, d(2026, 11, 1), 15).unwrap();
    let movimientos = [
        mov(MovementType::Ingreso, 1_500_000, d(2026, 11, 1)),
        mov(MovementType::Gasto, 300_000, d(2026, 11, 5)),
    ];
    let comp_periodo = compromiso_base("Arriendo", 500_000, 10, d(2026, 10, 1), Duracion::Indefinido);
    let comp_atrasado = compromiso_base("Servicio", 100_000, 20, d(2026, 9, 1), Duracion::Indefinido);
    let res = calcular_escenario(
        cal,
        &movimientos,
        &[
            (
                comp_periodo,
                vec![Vencimiento {
                    periodo_key: "2026-11".into(),
                    fecha: d(2026, 11, 10),
                }],
            ),
            (
                comp_atrasado,
                vec![Vencimiento {
                    periodo_key: "2026-10".into(),
                    fecha: d(2026, 10, 20),
                }],
            ),
        ],
        &[],
        d(2026, 11, 1),
        15,
    );
    let q = &res.quincenas[0];
    assert_eq!(q.base.libre_quincena_centavos, 700_000);
    assert_eq!(q.base.libre_real_centavos, 600_000);
    assert_eq!(q.base.atrasados.len(), 1);
    assert_eq!(q.libre_quincena_simulado_centavos, 500_000);
    assert_eq!(q.libre_real_simulado_centavos, 400_000);
}

#[test]
fn simular_impacto_usa_estado_real_y_no_persiste() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("kite.db");
    let storage = SqliteStorage::new(&db).unwrap();
    storage.migrate().unwrap();
    let compromisos = SqliteCompromisosStorage::new(&db).unwrap();
    compromisos.migrate().unwrap();
    let config = SqliteConfigStorage::new(&db).unwrap();
    config.migrate().unwrap();
    let metas = SqliteMetasStorage::new(&db).unwrap();
    metas.migrate().unwrap();

    let hoy = Local::now().date_naive();
    storage
        .save(
            Movement::new(
                MovementType::Ingreso,
                1_000_000,
                "Salario".into(),
                hoy,
                MovementOrigin::Formulario,
            )
            .unwrap(),
        )
        .unwrap();
    let compromisos_antes = compromisos.list().unwrap().len();
    let movimientos_antes = storage.query(None, None).unwrap().len();

    // Primer vencimiento consistente con mensual(10) en el mes actual.
    let primer = NaiveDate::from_ymd_opt(hoy.year(), hoy.month(), 10).unwrap();
    let deuda = DeudaSimulada {
        concepto: Some("Simulada".into()),
        monto_cuota_centavos: 200_000,
        frecuencia: Frecuencia::Mensual(10),
        primer_vencimiento: primer,
        duracion: DuracionSimulacion::Cuotas(2),
        quincenas_proyeccion: None,
    };
    let _ = simular_impacto(&storage, &compromisos, &metas, &deuda, hoy, 15).unwrap();

    assert_eq!(compromisos.list().unwrap().len(), compromisos_antes);
    assert_eq!(storage.query(None, None).unwrap().len(), movimientos_antes);
}

#[test]
fn sin_vencimientos_en_horizonte_devuelve_escenario_vacio() {
    let deuda = DeudaSimulada {
        concepto: None,
        monto_cuota_centavos: 100_000,
        frecuencia: Frecuencia::Mensual(10),
        primer_vencimiento: d(2020, 1, 10),
        duracion: DuracionSimulacion::Cuotas(1),
        quincenas_proyeccion: None,
    };
    let cal = generar_vencimientos_simulados(&deuda, d(2026, 11, 20), 15).unwrap();
    let res = calcular_escenario(cal, &[], &[], &[], d(2026, 11, 20), 15);
    assert!(res.quincenas.is_empty());
}

use kite::metas::domain::{EstadoMeta, Meta, MetaUI};

fn meta_activa(
    nombre: &str,
    objetivo: i64,
    acumulado: i64,
    fecha_objetivo: Option<NaiveDate>,
    a_ritmo: Option<bool>,
    creado: NaiveDate,
) -> MetaUI {
    let meta = Meta {
        id: Uuid::new_v4(),
        nombre: nombre.into(),
        monto_objetivo_centavos: objetivo,
        fecha_objetivo,
        estado: EstadoMeta::Activa,
        creado_en: creado.and_hms_opt(10, 0, 0).unwrap().and_local_timezone(Local).unwrap(),
        actualizado_en: creado.and_hms_opt(10, 0, 0).unwrap().and_local_timezone(Local).unwrap(),
    };
    MetaUI {
        meta,
        acumulado_centavos: acumulado,
        a_ritmo,
    }
}

#[test]
fn impacto_metas_vacio_cuando_no_hay_metas_activas() {
    let deuda = DeudaSimulada {
        concepto: None,
        monto_cuota_centavos: 100_000,
        frecuencia: Frecuencia::Mensual(10),
        primer_vencimiento: d(2026, 11, 10),
        duracion: DuracionSimulacion::Cuotas(1),
        quincenas_proyeccion: None,
    };
    let cal = generar_vencimientos_simulados(&deuda, d(2026, 11, 1), 15).unwrap();
    let res = calcular_escenario(cal, &[], &[], &[], d(2026, 11, 1), 15);
    assert!(res.metas.is_empty());
}

#[test]
fn impacto_metas_muestra_faltante_y_referencia_sin_fecha_objetivo() {
    let deuda = DeudaSimulada {
        concepto: None,
        monto_cuota_centavos: 200_000,
        frecuencia: Frecuencia::Mensual(10),
        primer_vencimiento: d(2026, 11, 10),
        duracion: DuracionSimulacion::Cuotas(1),
        quincenas_proyeccion: None,
    };
    let cal = generar_vencimientos_simulados(&deuda, d(2026, 11, 1), 15).unwrap();
    let movimientos = [mov(MovementType::Ingreso, 500_000, d(2026, 11, 1))];
    let meta = meta_activa("Viaje", 1_000_000, 300_000, None, None, d(2026, 10, 1));
    let res = calcular_escenario(cal, &movimientos, &[], &[meta], d(2026, 11, 1), 15);
    assert_eq!(res.metas.len(), 1);
    let im = &res.metas[0];
    assert_eq!(im.faltante_centavos, 700_000);
    assert_eq!(im.libre_simulado_referencia_centavos, Some(300_000));
    assert_eq!(im.ritmo_afectado, None);
}

#[test]
fn impacto_metas_senala_ritmo_afectado_cuando_libre_no_alcanza() {
    // Meta con fecha objetivo cercana, requiere aporte por quincena mayor al libre simulado.
    let deuda = DeudaSimulada {
        concepto: None,
        monto_cuota_centavos: 450_000,
        frecuencia: Frecuencia::Mensual(10),
        primer_vencimiento: d(2026, 11, 10),
        duracion: DuracionSimulacion::Cuotas(1),
        quincenas_proyeccion: None,
    };
    // hoy 2026-11-01, corte 15: quincena actual 1..15 nov. fecha objetivo en misma quincena (futura relativa a hoy)
    // pero Meta::new exige fecha > hoy; aquí construimos struct directo.
    let cal = generar_vencimientos_simulados(&deuda, d(2026, 11, 1), 15).unwrap();
    let movimientos = [mov(MovementType::Ingreso, 500_000, d(2026, 11, 1))];
    // faltante 400k, 1 quincena restante hasta objetivo -> necesita 400k, libre simulado 50k
    let meta = meta_activa(
        "Meta",
        500_000,
        100_000,
        Some(d(2026, 11, 14)),
        Some(true),
        d(2026, 10, 1),
    );
    let res = calcular_escenario(cal, &movimientos, &[], &[meta], d(2026, 11, 1), 15);
    assert_eq!(res.metas.len(), 1);
    let im = &res.metas[0];
    assert_eq!(im.faltante_centavos, 400_000);
    assert_eq!(im.ritmo_afectado, Some(true));
}

#[test]
fn impacto_metas_ritmo_no_afectado_cuando_libre_alcanza() {
    let deuda = DeudaSimulada {
        concepto: None,
        monto_cuota_centavos: 50_000,
        frecuencia: Frecuencia::Mensual(10),
        primer_vencimiento: d(2026, 11, 10),
        duracion: DuracionSimulacion::Cuotas(1),
        quincenas_proyeccion: None,
    };
    let cal = generar_vencimientos_simulados(&deuda, d(2026, 11, 1), 15).unwrap();
    let movimientos = [mov(MovementType::Ingreso, 500_000, d(2026, 11, 1))];
    let meta = meta_activa(
        "Meta",
        500_000,
        100_000,
        Some(d(2026, 11, 14)),
        Some(true),
        d(2026, 10, 1),
    );
    let res = calcular_escenario(cal, &movimientos, &[], &[meta], d(2026, 11, 1), 15);
    let im = &res.metas[0];
    // libre simulado 450k, necesita 400k en 1 quincena
    assert_eq!(im.ritmo_afectado, Some(false));
}
