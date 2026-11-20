//! Migración 004 (feature 005, T2 — S7): reconstrucción de `movements`
//! (CHECK ampliado con 'aporte' + columna `meta_id`) y creación de `metas`.
//!
//! CRÍTICO: la migración toca la tabla de datos del usuario. Estos tests crean
//! una base REAL pre-poblada en versión 3 (schema antiguo, datos con todos los
//! campos), cierran las conexiones y reabren con el código nuevo: cada fila
//! debe conservarse EXACTAMENTE (id, tipo, monto, concepto, fecha, origen,
//! timestamps). También: idempotencia y nueva instalación.

use chrono::{DateTime, Local, NaiveDate};
use kite::compromisos::port::CompromisosPort;
use kite::compromisos::sqlite_impl::SqliteCompromisosStorage;
use kite::config::{ConfigPort, SqliteConfigStorage};
use kite::models::{MovementOrigin, MovementType};
use kite::storage::port::StoragePort;
use kite::storage::sqlite_impl::SqliteStorage;
use std::path::Path;
use tempfile::tempdir;
use uuid::Uuid;

/// Schema v1 exacto de migrations/001_init.sql (sin meta_id, CHECK sin 'aporte').
const SCHEMA_MOVIMIENTOS_V1: &str = r#"
CREATE TABLE IF NOT EXISTS movements (
    id TEXT PRIMARY KEY,
    tipo TEXT NOT NULL CHECK(tipo IN ('ingreso','gasto')),
    monto_centavos INTEGER NOT NULL CHECK(monto_centavos > 0),
    concepto TEXT NOT NULL CHECK(trim(concepto) != ''),
    fecha TEXT NOT NULL,
    origen TEXT NOT NULL CHECK(origen IN ('formulario','mensaje')) DEFAULT 'formulario',
    creado_en TEXT NOT NULL,
    actualizado_en TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_movements_fecha ON movements(fecha DESC, creado_en DESC);
CREATE INDEX IF NOT EXISTS idx_movements_tipo_fecha ON movements(tipo, fecha DESC);
"#;

/// Schema v2 exacto de migrations/002_compromisos.sql (pagos referencia movements(id)).
const SCHEMA_COMPROMISOS_V2: &str = r#"
CREATE TABLE IF NOT EXISTS compromisos (
    id TEXT PRIMARY KEY,
    concepto TEXT NOT NULL CHECK(trim(concepto) != ''),
    monto_centavos INTEGER NOT NULL CHECK(monto_centavos > 0),
    frecuencia TEXT NOT NULL CHECK(frecuencia IN ('mensual','quincenal')),
    dia_mes INTEGER CHECK(dia_mes IS NULL OR (dia_mes BETWEEN 1 AND 31)),
    quincena TEXT CHECK(quincena IS NULL OR quincena IN ('primera','segunda')),
    duracion_tipo TEXT NOT NULL CHECK(duracion_tipo IN ('indefinido','fecha_fin','cuotas')),
    fecha_fin TEXT,
    cuotas_total INTEGER CHECK(cuotas_total IS NULL OR cuotas_total >= 1),
    cuotas_confirmadas INTEGER NOT NULL DEFAULT 0 CHECK(cuotas_confirmadas >= 0),
    estado TEXT NOT NULL DEFAULT 'activo' CHECK(estado IN ('activo','pausado')),
    creado_en TEXT NOT NULL,
    actualizado_en TEXT NOT NULL,
    CHECK(
        (duracion_tipo = 'indefinido' AND fecha_fin IS NULL AND cuotas_total IS NULL)
        OR (duracion_tipo = 'fecha_fin' AND fecha_fin IS NOT NULL AND cuotas_total IS NULL)
        OR (duracion_tipo = 'cuotas' AND fecha_fin IS NULL AND cuotas_total IS NOT NULL)
    ),
    CHECK(
        (frecuencia = 'mensual' AND dia_mes IS NOT NULL AND quincena IS NULL)
        OR (frecuencia = 'quincenal' AND dia_mes IS NULL AND quincena IS NOT NULL)
    )
);
CREATE INDEX IF NOT EXISTS idx_compromisos_estado ON compromisos(estado, concepto);
CREATE TABLE IF NOT EXISTS compromisos_pagos (
    id TEXT PRIMARY KEY,
    compromiso_id TEXT NOT NULL REFERENCES compromisos(id) ON DELETE CASCADE,
    periodo_key TEXT NOT NULL,
    movimiento_id TEXT NOT NULL REFERENCES movements(id),
    monto_pagado INTEGER NOT NULL CHECK(monto_pagado > 0),
    fecha_pago TEXT NOT NULL,
    UNIQUE(compromiso_id, periodo_key)
);
CREATE INDEX IF NOT EXISTS idx_pagos_compromiso ON compromisos_pagos(compromiso_id, fecha_pago);
"#;

const SCHEMA_CONFIG_V3: &str = r#"
CREATE TABLE IF NOT EXISTS configuracion (
    clave TEXT PRIMARY KEY,
    valor TEXT NOT NULL
);
"#;

/// Fila de movimiento sembrada en la base v3, con TODOS sus campos conocidos.
struct FilaSembrada {
    id: Uuid,
    tipo: MovementType,
    tipo_str: &'static str,
    monto_centavos: i64,
    concepto: &'static str,
    fecha: &'static str,
    origen: MovementOrigin,
    origen_str: &'static str,
    creado_en: &'static str,
    actualizado_en: &'static str,
}

fn filas_sembradas() -> Vec<FilaSembrada> {
    vec![
        FilaSembrada {
            id: Uuid::parse_str("11111111-1111-4111-8111-111111111111").unwrap(),
            tipo: MovementType::Ingreso,
            tipo_str: "ingreso",
            monto_centavos: 1_500_000,
            concepto: "Pago quincena",
            fecha: "2026-10-15",
            origen: MovementOrigin::Formulario,
            origen_str: "formulario",
            creado_en: "2026-10-15T08:00:00-05:00",
            actualizado_en: "2026-10-15T08:00:00-05:00",
        },
        FilaSembrada {
            id: Uuid::parse_str("22222222-2222-4222-8222-222222222222").unwrap(),
            tipo: MovementType::Gasto,
            tipo_str: "gasto",
            monto_centavos: 500_000,
            concepto: "Arriendo octubre",
            fecha: "2026-10-05",
            origen: MovementOrigin::Formulario,
            origen_str: "formulario",
            creado_en: "2026-10-05T09:30:00-05:00",
            actualizado_en: "2026-10-06T10:00:00-05:00",
        },
        FilaSembrada {
            id: Uuid::parse_str("33333333-3333-4333-8333-333333333333").unwrap(),
            tipo: MovementType::Gasto,
            tipo_str: "gasto",
            monto_centavos: 18_000,
            concepto: "Almuerzo  ",
            fecha: "2026-10-20",
            origen: MovementOrigin::Mensaje,
            origen_str: "mensaje",
            creado_en: "2026-10-20T13:00:00-05:00",
            actualizado_en: "2026-10-20T13:00:00-05:00",
        },
    ]
}

const COMPROMISO_ID: &str = "44444444-4444-4444-8444-444444444444";
const PAGO_ID: &str = "55555555-5555-4555-8555-555555555555";
const COMPROMISO_CREADO: &str = "2026-09-01T10:00:00-05:00";
const COMPROMISO_ACTUALIZADO: &str = "2026-10-05T09:35:00-05:00";

/// Crea una base REAL en versión 3 con el schema antiguo y datos pre-poblados
/// (movimientos con todos los campos, un compromiso y un pago confirmado).
/// Cierra la conexión al terminar: la migración se prueba sobre el archivo.
fn crear_base_v3(db: &Path) {
    let conn = rusqlite::Connection::open(db).unwrap();
    conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
    conn.execute_batch(SCHEMA_MOVIMIENTOS_V1).unwrap();
    conn.execute_batch(SCHEMA_COMPROMISOS_V2).unwrap();
    conn.execute_batch(SCHEMA_CONFIG_V3).unwrap();

    for f in filas_sembradas() {
        conn.execute(
            "INSERT INTO movements (id, tipo, monto_centavos, concepto, fecha, origen, creado_en, actualizado_en)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![
                f.id.to_string(), f.tipo_str, f.monto_centavos, f.concepto,
                f.fecha, f.origen_str, f.creado_en, f.actualizado_en
            ],
        )
        .unwrap();
    }

    conn.execute(
        "INSERT INTO compromisos (id, concepto, monto_centavos, frecuencia, dia_mes, quincena,
            duracion_tipo, fecha_fin, cuotas_total, cuotas_confirmadas, estado, creado_en, actualizado_en)
         VALUES (?1, 'Arriendo', 500000, 'mensual', 5, NULL, 'indefinido', NULL, NULL, 0, 'activo', ?2, ?3)",
        rusqlite::params![COMPROMISO_ID, COMPROMISO_CREADO, COMPROMISO_ACTUALIZADO],
    )
    .unwrap();

    // El pago referencia al movimiento de gasto (FK real hacia movements).
    conn.execute(
        "INSERT INTO compromisos_pagos (id, compromiso_id, periodo_key, movimiento_id, monto_pagado, fecha_pago)
         VALUES (?1, ?2, '2026-10', ?3, 500000, '2026-10-05')",
        rusqlite::params![
            PAGO_ID,
            COMPROMISO_ID,
            "22222222-2222-4222-8222-222222222222"
        ],
    )
    .unwrap();

    conn.execute(
        "INSERT INTO configuracion (clave, valor) VALUES ('dia_corte', '15')",
        [],
    )
    .unwrap();

    conn.execute_batch("PRAGMA user_version = 3;").unwrap();
}

fn user_version(db: &Path) -> i64 {
    let conn = rusqlite::Connection::open(db).unwrap();
    conn.query_row("PRAGMA user_version", [], |r| r.get(0))
        .unwrap()
}

fn ts(s: &str) -> DateTime<Local> {
    DateTime::parse_from_rfc3339(s)
        .unwrap()
        .with_timezone(&Local)
}

/// Verifica fila por fila que cada movimiento sembrado sobrevivió EXACTAMENTE.
fn verificar_movimientos_conservados(storage: &SqliteStorage) {
    for f in filas_sembradas() {
        let m = storage
            .get_by_id(f.id)
            .unwrap()
            .unwrap_or_else(|| panic!("movimiento {} desapareció tras migrar", f.id));
        assert_eq!(m.id, f.id);
        assert_eq!(m.tipo, f.tipo, "tipo cambió en {}", f.id);
        assert_eq!(
            m.monto_centavos, f.monto_centavos,
            "monto cambió en {}",
            f.id
        );
        assert_eq!(m.concepto, f.concepto, "concepto cambió en {}", f.id);
        assert_eq!(
            m.fecha,
            NaiveDate::parse_from_str(f.fecha, "%Y-%m-%d").unwrap()
        );
        assert_eq!(m.origen, f.origen, "origen cambió en {}", f.id);
        assert_eq!(m.creado_en, ts(f.creado_en), "creado_en cambió en {}", f.id);
        assert_eq!(
            m.actualizado_en,
            ts(f.actualizado_en),
            "actualizado_en cambió en {}",
            f.id
        );
    }
    assert_eq!(storage.query(None, None).unwrap().len(), 3);
}

#[test]
fn migracion_004_conserva_todos_los_datos_de_una_base_v3() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("kite.db");
    crear_base_v3(&db);
    assert_eq!(user_version(&db), 3);

    // Reabre con el código nuevo y migra (la conexión anterior ya se cerró).
    let storage = SqliteStorage::new(&db).unwrap();
    storage.migrate().unwrap();

    assert_eq!(
        user_version(&db),
        4,
        "la migración 004 debe dejar user_version=4"
    );
    verificar_movimientos_conservados(&storage);

    // El resumen de la 001 no cambia con los mismos datos (S7/FR-M09).
    let summary = storage.summary(None, None).unwrap();
    assert_eq!(summary.ingresos_centavos, 1_500_000);
    assert_eq!(summary.gastos_centavos, 518_000);

    // Compromisos y pagos intactos (incluida la FK hacia movements(id)).
    let comps = SqliteCompromisosStorage::new(&db).unwrap();
    comps.migrate().unwrap();
    let lista = comps.list().unwrap();
    assert_eq!(lista.len(), 1);
    assert_eq!(lista[0].id.to_string(), COMPROMISO_ID);
    assert_eq!(lista[0].concepto, "Arriendo");
    assert_eq!(lista[0].monto_centavos, 500_000);
    assert_eq!(lista[0].creado_en, ts(COMPROMISO_CREADO));
    assert_eq!(lista[0].actualizado_en, ts(COMPROMISO_ACTUALIZADO));
    let pagos = comps.historial_pagos(lista[0].id).unwrap();
    assert_eq!(pagos.len(), 1);
    assert_eq!(
        pagos[0].movimiento_id.to_string(),
        "22222222-2222-4222-8222-222222222222"
    );
    assert_eq!(pagos[0].periodo_key, "2026-10");
    assert_eq!(pagos[0].monto_pagado, 500_000);

    // Configuración intacta.
    let config = SqliteConfigStorage::new(&db).unwrap();
    config.migrate().unwrap();
    assert_eq!(config.get("dia_corte").unwrap().as_deref(), Some("15"));

    // Schema nuevo operativo: tabla metas existe, movements acepta 'aporte'
    // con meta_id, y los índices de movements se conservan.
    let conn = rusqlite::Connection::open(&db).unwrap();
    let metas_existe: bool = conn
        .query_row(
            "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='metas'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert!(metas_existe, "la migración 004 debe crear la tabla metas");

    let meta_id = Uuid::new_v4();
    conn.execute(
        "INSERT INTO metas (id, nombre, monto_objetivo_centavos, fecha_objetivo, estado, creado_en, actualizado_en)
         VALUES (?1, 'Viaje', 2000000, NULL, 'activa', ?2, ?2)",
        rusqlite::params![meta_id.to_string(), "2026-11-01T10:00:00-05:00"],
    )
    .unwrap();
    let aporte_id = Uuid::new_v4();
    conn.execute(
        "INSERT INTO movements (id, tipo, monto_centavos, concepto, fecha, origen, meta_id, creado_en, actualizado_en)
         VALUES (?1, 'aporte', 200000, 'Apartado · Viaje', '2026-11-02', 'formulario', ?2, ?3, ?3)",
        rusqlite::params![
            aporte_id.to_string(),
            meta_id.to_string(),
            "2026-11-02T10:00:00-05:00"
        ],
    )
    .unwrap();
    let (tipo_leido, meta_leida): (String, Option<String>) = conn
        .query_row(
            "SELECT tipo, meta_id FROM movements WHERE id = ?1",
            rusqlite::params![aporte_id.to_string()],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(tipo_leido, "aporte");
    assert_eq!(meta_leida.as_deref(), Some(meta_id.to_string().as_str()));
    // El aporte NO infla ingresos ni gastos (S2).
    let summary2 = storage.summary(None, None).unwrap();
    assert_eq!(summary2.ingresos_centavos, 1_500_000);
    assert_eq!(summary2.gastos_centavos, 518_000);

    for idx in ["idx_movements_fecha", "idx_movements_tipo_fecha"] {
        let existe: bool = conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='index' AND name=?1",
                rusqlite::params![idx],
                |r| r.get(0),
            )
            .unwrap();
        assert!(existe, "el índice {idx} debe sobrevivir la reconstrucción");
    }

    // Integridad referencial tras el rebuild (compromisos_pagos → movements).
    let mut stmt = conn.prepare("PRAGMA foreign_key_check").unwrap();
    let rotas = stmt
        .query_map([], |r| r.get::<_, String>(0))
        .unwrap()
        .count();
    assert_eq!(rotas, 0, "la reconstrucción no puede dejar FKs rotas");
}

#[test]
fn migracion_004_es_idempotente() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("kite.db");
    crear_base_v3(&db);

    // Primera migración.
    {
        let storage = SqliteStorage::new(&db).unwrap();
        storage.migrate().unwrap();
        // Segunda llamada sobre la misma conexión: no debe tocar nada.
        storage.migrate().unwrap();
    }
    assert_eq!(user_version(&db), 4);

    // Tercera migración tras cerrar y reabrir: los datos siguen exactos.
    let storage = SqliteStorage::new(&db).unwrap();
    storage.migrate().unwrap();
    assert_eq!(user_version(&db), 4);
    verificar_movimientos_conservados(&storage);

    let comps = SqliteCompromisosStorage::new(&db).unwrap();
    comps.migrate().unwrap();
    assert_eq!(comps.list().unwrap().len(), 1);
    assert_eq!(
        comps
            .historial_pagos(comps.list().unwrap()[0].id)
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn instalacion_nueva_queda_directamente_en_version_4() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("kite.db");

    let storage = SqliteStorage::new(&db).unwrap();
    storage.migrate().unwrap();
    assert_eq!(user_version(&db), 4);

    // movements acepta 'aporte' y meta_id desde el primer día.
    let meta_id = Uuid::new_v4();
    let conn = rusqlite::Connection::open(&db).unwrap();
    conn.execute(
        "INSERT INTO metas (id, nombre, monto_objetivo_centavos, fecha_objetivo, estado, creado_en, actualizado_en)
         VALUES (?1, 'Fondo', 1000000, NULL, 'activa', ?2, ?2)",
        rusqlite::params![meta_id.to_string(), "2026-11-01T10:00:00-05:00"],
    )
    .unwrap();
    drop(conn);

    let aporte_id = Uuid::new_v4();
    let conn = rusqlite::Connection::open(&db).unwrap();
    conn.execute(
        "INSERT INTO movements (id, tipo, monto_centavos, concepto, fecha, origen, meta_id, creado_en, actualizado_en)
         VALUES (?1, 'aporte', 100000, 'Apartado · Fondo', '2026-11-02', 'formulario', ?2, ?3, ?3)",
        rusqlite::params![
            aporte_id.to_string(),
            meta_id.to_string(),
            "2026-11-02T10:00:00-05:00"
        ],
    )
    .unwrap();
    drop(conn);
    let leido = storage.get_by_id(aporte_id).unwrap().unwrap();
    assert_eq!(leido.monto_centavos, 100_000);

    // Los ingresos/gastos siguen funcionando igual.
    let gasto = kite::models::Movement::new(
        MovementType::Gasto,
        18_000,
        "Almuerzo".into(),
        NaiveDate::from_ymd_opt(2026, 11, 3).unwrap(),
        MovementOrigin::Formulario,
    )
    .unwrap();
    storage.save(gasto.clone()).unwrap();
    let leido = storage.get_by_id(gasto.id).unwrap().unwrap();
    assert_eq!(leido.monto_centavos, 18_000);
    let summary = storage.summary(None, None).unwrap();
    assert_eq!(summary.ingresos_centavos, 0);
    assert_eq!(summary.gastos_centavos, 18_000);
}
