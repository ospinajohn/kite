use crate::models::{Movement, MovementType, MovementOrigin, PeriodSummary};
use crate::storage::port::{StoragePort, StorageError};
use chrono::{DateTime, Local, NaiveDate};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

pub struct SqliteStorage {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteStorage {
    pub fn new(db_path: &Path) -> Result<Self, StorageError> {
        std::fs::create_dir_all(db_path.parent().unwrap())?;
        let conn = Connection::open(db_path)?;
        conn.execute_batch("PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL;")?;
        Ok(Self { conn: Arc::new(Mutex::new(conn)) })
    }

    fn init_schema(&self) -> Result<(), StorageError> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch(
            r#"
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
            "#
        )?;
        Ok(())
    }

    fn row_to_movement(row: &rusqlite::Row) -> Result<Movement, rusqlite::Error> {
        let tipo_str: String = row.get("tipo")?;
        let origen_str: String = row.get("origen")?;
        let creado_en_str: String = row.get("creado_en")?;
        let actualizado_en_str: String = row.get("actualizado_en")?;

        Ok(Movement {
            id: Uuid::parse_str(&row.get::<_, String>("id")?).map_err(|_| rusqlite::Error::InvalidColumnType(0, "id".into(), rusqlite::types::Type::Text))?,
            tipo: Self::parse_movement_type(&tipo_str).map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?,
            monto_centavos: row.get("monto_centavos")?,
            concepto: row.get("concepto")?,
            fecha: NaiveDate::parse_from_str(&row.get::<_, String>("fecha")?, "%Y-%m-%d").map_err(|_| rusqlite::Error::InvalidColumnType(0, "fecha".into(), rusqlite::types::Type::Text))?,
            origen: Self::parse_origin(&origen_str).map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?,
            creado_en: DateTime::parse_from_rfc3339(&creado_en_str).map_err(|_| rusqlite::Error::InvalidColumnType(0, "creado_en".into(), rusqlite::types::Type::Text))?.with_timezone(&Local),
            actualizado_en: DateTime::parse_from_rfc3339(&actualizado_en_str).map_err(|_| rusqlite::Error::InvalidColumnType(0, "actualizado_en".into(), rusqlite::types::Type::Text))?.with_timezone(&Local),
        })
    }

    fn row_to_movement_storage(&self, row: &rusqlite::Row) -> Result<Movement, StorageError> {
        let tipo_str: String = row.get("tipo").map_err(StorageError::Database)?;
        let origen_str: String = row.get("origen").map_err(StorageError::Database)?;
        let creado_en_str: String = row.get("creado_en").map_err(StorageError::Database)?;
        let actualizado_en_str: String = row.get("actualizado_en").map_err(StorageError::Database)?;

        Ok(Movement {
            id: Uuid::parse_str(&row.get::<_, String>("id").map_err(StorageError::Database)?)
                .map_err(|_| StorageError::Integrity("Invalid UUID".into()))?,
            tipo: Self::parse_movement_type(&tipo_str)?,
            monto_centavos: row.get("monto_centavos").map_err(StorageError::Database)?,
            concepto: row.get("concepto").map_err(StorageError::Database)?,
            fecha: NaiveDate::parse_from_str(&row.get::<_, String>("fecha").map_err(StorageError::Database)?, "%Y-%m-%d")
                .map_err(|_| StorageError::Integrity("Invalid date format".into()))?,
            origen: Self::parse_origin(&origen_str)?,
            creado_en: DateTime::parse_from_rfc3339(&creado_en_str)
                .map_err(|_| StorageError::Integrity("Invalid created_at format".into()))?.with_timezone(&Local),
            actualizado_en: DateTime::parse_from_rfc3339(&actualizado_en_str)
                .map_err(|_| StorageError::Integrity("Invalid updated_at format".into()))?.with_timezone(&Local),
        })
    }
}

impl StoragePort for SqliteStorage {
    fn save(&self, movement: Movement) -> Result<Movement, StorageError> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;

        let id_str = movement.id.to_string();
        let tipo_str = Self::map_movement_type(movement.tipo);
        let origen_str = Self::map_origin(movement.origen);
        let fecha_str = movement.fecha.format("%Y-%m-%d").to_string();
        let creado_en_str = movement.creado_en.to_rfc3339();
        let actualizado_en_str = movement.actualizado_en.to_rfc3339();

        tx.execute(
            r#"
            INSERT INTO movements (id, tipo, monto_centavos, concepto, fecha, origen, creado_en, actualizado_en)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            "#,
            params![
                id_str,
                tipo_str,
                movement.monto_centavos,
                movement.concepto,
                fecha_str,
                origen_str,
                creado_en_str,
                actualizado_en_str
            ],
        )?;

        tx.commit()?;
        Ok(movement)
    }

    fn update(&self, movement: Movement) -> Result<Movement, StorageError> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;

        let tipo_str = Self::map_movement_type(movement.tipo);
        let fecha_str = movement.fecha.format("%Y-%m-%d").to_string();
        let actualizado_en_str = movement.actualizado_en.to_rfc3339();

        let rows = tx.execute(
            r#"
            UPDATE movements
            SET monto_centavos = ?1, concepto = ?2, fecha = ?3, actualizado_en = ?4
            WHERE id = ?5 AND tipo = ?6
            "#,
            params![
                movement.monto_centavos,
                movement.concepto,
                fecha_str,
                actualizado_en_str,
                movement.id.to_string(),
                tipo_str
            ],
        )?;

        if rows == 0 {
            return Err(StorageError::NotFound(movement.id));
        }

        tx.commit()?;
        Ok(movement)
    }

    fn delete(&self, id: Uuid) -> Result<bool, StorageError> {
        let mut conn = self.conn.lock().unwrap();
        let rows = conn.execute(
            "DELETE FROM movements WHERE id = ?1",
            params![id.to_string()],
        )?;
        Ok(rows > 0)
    }

    fn get_by_id(&self, id: Uuid) -> Result<Option<Movement>, StorageError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT * FROM movements WHERE id = ?1"
        )?;
        let movement = stmt.query_row(params![id.to_string()], |row| {
            self.row_to_movement_storage(row).map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))
        }).optional()?;
        Ok(movement)
    }

    fn query(
        &self,
        desde: Option<NaiveDate>,
        hasta: Option<NaiveDate>,
    ) -> Result<Vec<Movement>, StorageError> {
        let conn = self.conn.lock().unwrap();
        let mut sql = String::from("SELECT * FROM movements");
        let mut params_vec = Vec::new();
        let mut conditions = Vec::new();

        if let Some(d) = desde {
            conditions.push("fecha >= ?".to_string());
            params_vec.push(d.format("%Y-%m-%d").to_string());
        }
        if let Some(h) = hasta {
            conditions.push("fecha <= ?".to_string());
            params_vec.push(h.format("%Y-%m-%d").to_string());
        }

        if !conditions.is_empty() {
            sql.push_str(" WHERE ");
            sql.push_str(&conditions.join(" AND "));
        }

        sql.push_str(" ORDER BY fecha DESC, creado_en DESC");

        let mut stmt = conn.prepare(&sql)?;
        let param_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|s| s as &dyn rusqlite::ToSql).collect();
        let rows = stmt.query_map(rusqlite::params_from_iter(param_refs), |row| SqliteStorage::row_to_movement(row))?;

        let mut movements = Vec::new();
        for row in rows {
            movements.push(row?);
        }
        Ok(movements)
    }

    fn summary(
        &self,
        desde: Option<NaiveDate>,
        hasta: Option<NaiveDate>,
    ) -> Result<PeriodSummary, StorageError> {
        let conn = self.conn.lock().unwrap();
        let mut sql = String::from(
            r#"
            SELECT
                COALESCE(SUM(CASE WHEN tipo = 'ingreso' THEN monto_centavos ELSE 0 END), 0) as ingresos,
                COALESCE(SUM(CASE WHEN tipo = 'gasto' THEN monto_centavos ELSE 0 END), 0) as gastos
            FROM movements
            "#
        );
        let mut params_vec = Vec::new();
        let mut conditions = Vec::new();

        if let Some(d) = desde {
            conditions.push("fecha >= ?".to_string());
            params_vec.push(d.format("%Y-%m-%d").to_string());
        }
        if let Some(h) = hasta {
            conditions.push("fecha <= ?".to_string());
            params_vec.push(h.format("%Y-%m-%d").to_string());
        }

        if !conditions.is_empty() {
            sql.push_str(" WHERE ");
            sql.push_str(&conditions.join(" AND "));
        }

        let mut stmt = conn.prepare(&sql)?;
        let param_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|s| s as &dyn rusqlite::ToSql).collect();
        let (ingresos, gastos): (i64, i64) = stmt.query_row(
            rusqlite::params_from_iter(param_refs),
            |row| Ok((row.get(0)?, row.get(1)?))
        )?;

        Ok(PeriodSummary::new(ingresos, gastos, desde, hasta))
    }

    fn migrate(&self) -> Result<(), StorageError> {
        self.init_schema()?;
        let conn = self.conn.lock().unwrap();
        let version: i64 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;
        if version < 1 {
            conn.execute("PRAGMA user_version = 1", [])?;
        }
        Ok(())
    }
}

impl SqliteStorage {
    fn map_movement_type(t: MovementType) -> &'static str {
        match t {
            MovementType::Ingreso => "ingreso",
            MovementType::Gasto => "gasto",
        }
    }

    fn parse_movement_type(s: &str) -> Result<MovementType, StorageError> {
        match s {
            "ingreso" => Ok(MovementType::Ingreso),
            "gasto" => Ok(MovementType::Gasto),
            _ => Err(StorageError::Integrity(format!("Tipo inválido: {}", s))),
        }
    }

    fn map_origin(o: MovementOrigin) -> &'static str {
        match o {
            MovementOrigin::Formulario => "formulario",
            MovementOrigin::Mensaje => "mensaje",
        }
    }

    fn parse_origin(s: &str) -> Result<MovementOrigin, StorageError> {
        match s {
            "formulario" => Ok(MovementOrigin::Formulario),
            "mensaje" => Ok(MovementOrigin::Mensaje),
            _ => Err(StorageError::Integrity(format!("Origen inválido: {}", s))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Movement, MovementType, MovementOrigin};
    use chrono::{Local, NaiveDate};
    use tempfile::tempdir;

    fn create_test_storage() -> SqliteStorage {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        SqliteStorage::new(&db_path).unwrap()
    }

    #[test]
    fn test_migrate() {
        let storage = create_test_storage();
        storage.migrate().unwrap();
    }

    #[test]
    fn test_save_and_get() {
        let storage = create_test_storage();
        storage.migrate().unwrap();

        let movement = Movement::new(
            MovementType::Gasto,
            18_000,
            "Almuerzo".into(),
            Local::now().date_naive(),
            MovementOrigin::Formulario,
        ).unwrap();

        let saved = storage.save(movement.clone()).unwrap();
        assert_eq!(saved.id, movement.id);

        let retrieved = storage.get_by_id(saved.id).unwrap().unwrap();
        assert_eq!(retrieved.id, movement.id);
        assert_eq!(retrieved.monto_centavos, 18_000);
        assert_eq!(retrieved.concepto, "Almuerzo");
    }

    #[test]
    fn test_save_multiple_and_query() {
        let storage = create_test_storage();
        storage.migrate().unwrap();

        let m1 = Movement::new(MovementType::Ingreso, 1_500_000, "Pago".into(), NaiveDate::from_ymd_opt(2026,10,15).unwrap(), MovementOrigin::Formulario).unwrap();
        let m2 = Movement::new(MovementType::Gasto, 18_000, "Almuerzo".into(), Local::now().date_naive(), MovementOrigin::Formulario).unwrap();
        let m3 = Movement::new(MovementType::Gasto, 12_000, "Transporte".into(), Local::now().date_naive(), MovementOrigin::Mensaje).unwrap();

        storage.save(m1.clone()).unwrap();
        storage.save(m2.clone()).unwrap();
        storage.save(m3.clone()).unwrap();

        let all = storage.query(None, None).unwrap();
        assert_eq!(all.len(), 3);
        assert!(all[0].fecha >= all[1].fecha);
        assert!(all[1].fecha >= all[2].fecha);
    }

    #[test]
    fn test_query_with_range() {
        let storage = create_test_storage();
        storage.migrate().unwrap();

        let m1 = Movement::new(MovementType::Gasto, 1000, "A".into(), NaiveDate::from_ymd_opt(2020,10,1).unwrap(), MovementOrigin::Formulario).unwrap();
        let m2 = Movement::new(MovementType::Gasto, 2000, "B".into(), NaiveDate::from_ymd_opt(2020,10,15).unwrap(), MovementOrigin::Formulario).unwrap();
        let m3 = Movement::new(MovementType::Gasto, 3000, "C".into(), NaiveDate::from_ymd_opt(2020,11,1).unwrap(), MovementOrigin::Formulario).unwrap();

        storage.save(m1).unwrap();
        storage.save(m2).unwrap();
        storage.save(m3).unwrap();

        let oct = storage.query(
            Some(NaiveDate::from_ymd_opt(2020,10,1).unwrap()),
            Some(NaiveDate::from_ymd_opt(2020,10,31).unwrap()),
        ).unwrap();
        assert_eq!(oct.len(), 2);
    }

    #[test]
    fn test_summary() {
        let storage = create_test_storage();
        storage.migrate().unwrap();

        storage.save(Movement::new(MovementType::Ingreso, 1_500_000, "Pago".into(), NaiveDate::from_ymd_opt(2026,10,15).unwrap(), MovementOrigin::Formulario).unwrap()).unwrap();
        storage.save(Movement::new(MovementType::Gasto, 18_000, "Almuerzo".into(), Local::now().date_naive(), MovementOrigin::Formulario).unwrap()).unwrap();
        storage.save(Movement::new(MovementType::Gasto, 12_000, "Transporte".into(), Local::now().date_naive(), MovementOrigin::Mensaje).unwrap()).unwrap();

        let summary = storage.summary(None, None).unwrap();
        assert_eq!(summary.ingresos_centavos, 1_500_000);
        assert_eq!(summary.gastos_centavos, 30_000);
        assert_eq!(summary.diferencia_centavos, 1_470_000);
    }

    #[test]
    fn test_update() {
        let storage = create_test_storage();
        storage.migrate().unwrap();

        let mut m = Movement::new(MovementType::Gasto, 18_000, "Almuerzo".into(), Local::now().date_naive(), MovementOrigin::Formulario).unwrap();
        storage.save(m.clone()).unwrap();

        m = m.update(19_000, "Almuerzo corregido".into(), m.fecha).unwrap();
        storage.update(m.clone()).unwrap();

        let retrieved = storage.get_by_id(m.id).unwrap().unwrap();
        assert_eq!(retrieved.monto_centavos, 19_000);
        assert_eq!(retrieved.concepto, "Almuerzo corregido");
    }

    #[test]
    fn test_delete() {
        let storage = create_test_storage();
        storage.migrate().unwrap();

        let m = Movement::new(MovementType::Gasto, 18_000, "Almuerzo".into(), Local::now().date_naive(), MovementOrigin::Formulario).unwrap();
        storage.save(m.clone()).unwrap();

        let deleted = storage.delete(m.id).unwrap();
        assert!(deleted);

        let retrieved = storage.get_by_id(m.id).unwrap();
        assert!(retrieved.is_none());

        let not_deleted = storage.delete(Uuid::nil()).unwrap();
        assert!(!not_deleted);
    }

    #[test]
    fn test_persistence_across_connections() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");

        {
            let storage = SqliteStorage::new(&db_path).unwrap();
            storage.migrate().unwrap();
            let m = Movement::new(MovementType::Gasto, 18_000, "Almuerzo".into(), Local::now().date_naive(), MovementOrigin::Formulario).unwrap();
            storage.save(m).unwrap();
        }

        {
            let storage = SqliteStorage::new(&db_path).unwrap();
            let all = storage.query(None, None).unwrap();
            assert_eq!(all.len(), 1);
            assert_eq!(all[0].monto_centavos, 18_000);
        }
    }
}