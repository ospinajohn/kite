//! Configuración clave/valor persistida (feature 003): por ahora `dia_corte`.
//! Mismo patrón que los storage de movimientos y compromisos: schema inline +
//! migrate con `PRAGMA user_version`.

use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;
use std::sync::{Arc, Mutex};

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Violación de integridad: {0}")]
    Integrity(String),
    #[error("Error de base de datos: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("Error de E/S: {0}")]
    Io(#[from] std::io::Error),
    #[error("Error de migración: {0}")]
    Migration(String),
}

pub trait ConfigPort: Send + Sync {
    /// Valor persistido de una clave, o `None` si no existe.
    fn get(&self, clave: &str) -> Result<Option<String>, ConfigError>;
    /// Crea o actualiza la clave (upsert: una sola fila por clave).
    fn set(&self, clave: &str, valor: &str) -> Result<(), ConfigError>;
    fn migrate(&self) -> Result<(), ConfigError>;
}

pub struct SqliteConfigStorage {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteConfigStorage {
    pub fn new(db_path: &Path) -> Result<Self, ConfigError> {
        std::fs::create_dir_all(db_path.parent().unwrap())?;
        let conn = Connection::open(db_path)?;
        conn.execute_batch("PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL;")?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    fn init_schema(&self) -> Result<(), ConfigError> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS configuracion (
                clave TEXT PRIMARY KEY,
                valor TEXT NOT NULL
            );
            "#,
        )?;
        Ok(())
    }
}

impl ConfigPort for SqliteConfigStorage {
    fn get(&self, clave: &str) -> Result<Option<String>, ConfigError> {
        let conn = self.conn.lock().unwrap();
        let valor = conn
            .query_row(
                "SELECT valor FROM configuracion WHERE clave = ?1",
                params![clave],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        Ok(valor)
    }

    fn set(&self, clave: &str, valor: &str) -> Result<(), ConfigError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            r#"
            INSERT INTO configuracion (clave, valor) VALUES (?1, ?2)
            ON CONFLICT(clave) DO UPDATE SET valor = excluded.valor
            "#,
            params![clave, valor],
        )?;
        Ok(())
    }

    fn migrate(&self) -> Result<(), ConfigError> {
        // init_schema toma y suelta el lock antes de volver a tomarlo aquí:
        // nunca se llama a un método que toma el lock dentro de otro que ya lo tiene.
        self.init_schema()?;
        let conn = self.conn.lock().unwrap();
        let version: i64 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;
        if version < 3 {
            conn.execute("PRAGMA user_version = 3", [])?;
        }
        Ok(())
    }
}
