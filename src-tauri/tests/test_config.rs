//! Contract tests del puerto de configuración (feature 003, T2 / S2).

use kite::config::{ConfigPort, SqliteConfigStorage};
use tempfile::tempdir;

#[test]
fn migracion_idempotente_y_user_version_3() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("kite.db");
    let config = SqliteConfigStorage::new(&db).unwrap();
    config.migrate().unwrap();
    // Segunda llamada idempotente: no falla ni pierde datos.
    config.set("dia_corte", "15").unwrap();
    config.migrate().unwrap();
    assert_eq!(config.get("dia_corte").unwrap(), Some("15".to_string()));

    let conn = rusqlite::Connection::open(&db).unwrap();
    let version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0)).unwrap();
    assert_eq!(version, 3);
}

#[test]
fn set_get_roundtrip() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("kite.db");
    let config = SqliteConfigStorage::new(&db).unwrap();
    config.migrate().unwrap();

    config.set("dia_corte", "14").unwrap();
    assert_eq!(config.get("dia_corte").unwrap(), Some("14".to_string()));
}

#[test]
fn get_inexistente_devuelve_none() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("kite.db");
    let config = SqliteConfigStorage::new(&db).unwrap();
    config.migrate().unwrap();

    assert_eq!(config.get("dia_corte").unwrap(), None);
}

#[test]
fn update_de_clave_existente_conserva_una_sola_fila() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("kite.db");
    let config = SqliteConfigStorage::new(&db).unwrap();
    config.migrate().unwrap();

    config.set("dia_corte", "15").unwrap();
    config.set("dia_corte", "14").unwrap();
    assert_eq!(config.get("dia_corte").unwrap(), Some("14".to_string()));

    let conn = rusqlite::Connection::open(&db).unwrap();
    let filas: i64 = conn
        .query_row("SELECT COUNT(*) FROM configuracion", [], |r| r.get(0))
        .unwrap();
    assert_eq!(filas, 1);
}

#[test]
fn persiste_al_reabrir_la_base() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("kite.db");
    {
        let config = SqliteConfigStorage::new(&db).unwrap();
        config.migrate().unwrap();
        config.set("dia_corte", "20").unwrap();
    }
    // Nueva instancia sobre el mismo archivo: el ajuste persiste (US2.2).
    let config = SqliteConfigStorage::new(&db).unwrap();
    config.migrate().unwrap();
    assert_eq!(config.get("dia_corte").unwrap(), Some("20".to_string()));
}