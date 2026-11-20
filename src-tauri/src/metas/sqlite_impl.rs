use crate::metas::domain::{EstadoMeta, Meta, MetaFila, MetasError};
use crate::metas::port::MetasPort;
use crate::metas::service::recalcular_estado;
use crate::models::{Movement, MovementOrigin, MovementType};
use chrono::{DateTime, Local, NaiveDate};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

pub struct SqliteMetasStorage {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteMetasStorage {
    pub fn new(db_path: &Path) -> Result<Self, MetasError> {
        std::fs::create_dir_all(db_path.parent().unwrap())?;
        let conn = Connection::open(db_path)?;
        conn.execute_batch("PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL;")?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    fn init_schema(&self) -> Result<(), MetasError> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS metas (
                id TEXT PRIMARY KEY,
                nombre TEXT NOT NULL CHECK(trim(nombre) != ''),
                monto_objetivo_centavos INTEGER NOT NULL CHECK(monto_objetivo_centavos > 0),
                fecha_objetivo TEXT,
                estado TEXT NOT NULL CHECK(estado IN ('activa','completada','cancelada')),
                creado_en TEXT NOT NULL,
                actualizado_en TEXT NOT NULL
            );
            "#,
        )?;
        Ok(())
    }

    fn row_to_meta(row: &rusqlite::Row) -> Result<Meta, rusqlite::Error> {
        row_to_meta_impl(row).map_err(|e: MetasError| match e {
            MetasError::Database(db) => db,
            other => rusqlite::Error::ToSqlConversionFailure(Box::new(other)),
        })
    }

    fn row_to_movement(row: &rusqlite::Row) -> Result<Movement, rusqlite::Error> {
        row_to_movement_impl(row).map_err(|e: MetasError| match e {
            MetasError::Database(db) => db,
            other => rusqlite::Error::ToSqlConversionFailure(Box::new(other)),
        })
    }
}

fn row_to_meta_impl(row: &rusqlite::Row) -> Result<Meta, MetasError> {
    let id = Uuid::parse_str(&row.get::<_, String>("id")?)
        .map_err(|_| MetasError::Migration("UUID de meta inválido".into()))?;
    let nombre: String = row.get("nombre")?;
    let monto_objetivo_centavos: i64 = row.get("monto_objetivo_centavos")?;
    let fecha_objetivo_str: Option<String> = row.get("fecha_objetivo")?;
    let estado_str: String = row.get("estado")?;
    let creado_en_str: String = row.get("creado_en")?;
    let actualizado_en_str: String = row.get("actualizado_en")?;

    let fecha_objetivo = fecha_objetivo_str
        .map(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d"))
        .transpose()
        .map_err(|_| MetasError::Migration("Fecha objetivo inválida".into()))?;
    let estado = match estado_str.as_str() {
        "activa" => EstadoMeta::Activa,
        "completada" => EstadoMeta::Completada,
        "cancelada" => EstadoMeta::Cancelada,
        _ => return Err(MetasError::Migration("Estado de meta inválido".into())),
    };

    Ok(Meta::desde_fila(MetaFila {
        id,
        nombre,
        monto_objetivo_centavos,
        fecha_objetivo,
        estado,
        creado_en: parse_timestamp(&creado_en_str)?,
        actualizado_en: parse_timestamp(&actualizado_en_str)?,
    }))
}

fn row_to_movement_impl(row: &rusqlite::Row) -> Result<Movement, MetasError> {
    let tipo_str: String = row.get("tipo")?;
    let origen_str: String = row.get("origen")?;
    let meta_id_str: Option<String> = row.get("meta_id")?;
    let creado_en_str: String = row.get("creado_en")?;
    let actualizado_en_str: String = row.get("actualizado_en")?;
    let tipo = match tipo_str.as_str() {
        "ingreso" => MovementType::Ingreso,
        "gasto" => MovementType::Gasto,
        "aporte" => MovementType::Aporte,
        _ => return Err(MetasError::Migration("Tipo de movimiento inválido".into())),
    };
    let origen = match origen_str.as_str() {
        "formulario" => MovementOrigin::Formulario,
        "mensaje" => MovementOrigin::Mensaje,
        _ => return Err(MetasError::Migration("Origen inválido".into())),
    };
    Ok(Movement {
        id: Uuid::parse_str(&row.get::<_, String>("id")?)
            .map_err(|_| MetasError::Migration("UUID de movimiento inválido".into()))?,
        tipo,
        monto_centavos: row.get("monto_centavos")?,
        concepto: row.get("concepto")?,
        fecha: NaiveDate::parse_from_str(&row.get::<_, String>("fecha")?, "%Y-%m-%d")
            .map_err(|_| MetasError::Migration("Fecha inválida".into()))?,
        origen,
        meta_id: meta_id_str
            .map(|s| Uuid::parse_str(&s))
            .transpose()
            .map_err(|_| MetasError::Migration("meta_id inválido".into()))?,
        creado_en: parse_timestamp(&creado_en_str)?,
        actualizado_en: parse_timestamp(&actualizado_en_str)?,
    })
}

fn parse_timestamp(s: &str) -> Result<DateTime<Local>, MetasError> {
    DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Local))
        .map_err(|_| MetasError::Migration(format!("Timestamp inválido: {s}")))
}

impl MetasPort for SqliteMetasStorage {
    fn save(&self, meta: Meta) -> Result<Meta, MetasError> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;
        tx.execute(
            r#"
            INSERT INTO metas (id, nombre, monto_objetivo_centavos, fecha_objetivo, estado, creado_en, actualizado_en)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            "#,
            params![
                meta.id.to_string(),
                meta.nombre,
                meta.monto_objetivo_centavos,
                meta.fecha_objetivo.map(|f| f.format("%Y-%m-%d").to_string()),
                match meta.estado { EstadoMeta::Activa => "activa", EstadoMeta::Completada => "completada", EstadoMeta::Cancelada => "cancelada" },
                meta.creado_en.to_rfc3339(),
                meta.actualizado_en.to_rfc3339(),
            ],
        )?;
        tx.commit()?;
        Ok(meta)
    }

    fn update(&self, meta: Meta) -> Result<Meta, MetasError> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;
        let affected = tx.execute(
            r#"
            UPDATE metas SET nombre=?2, monto_objetivo_centavos=?3, fecha_objetivo=?4, estado=?5, actualizado_en=?6
            WHERE id=?1
            "#,
            params![
                meta.id.to_string(),
                meta.nombre,
                meta.monto_objetivo_centavos,
                meta.fecha_objetivo.map(|f| f.format("%Y-%m-%d").to_string()),
                match meta.estado { EstadoMeta::Activa => "activa", EstadoMeta::Completada => "completada", EstadoMeta::Cancelada => "cancelada" },
                meta.actualizado_en.to_rfc3339(),
            ],
        )?;
        if affected == 0 {
            return Err(MetasError::NotFound(meta.id));
        }
        tx.commit()?;
        Ok(meta)
    }

    fn get_by_id(&self, id: Uuid) -> Result<Option<Meta>, MetasError> {
        let conn = self.conn.lock().unwrap();
        let row = conn
            .query_row(
                "SELECT * FROM metas WHERE id=?1",
                params![id.to_string()],
                Self::row_to_meta,
            )
            .optional()?;
        Ok(row)
    }

    fn list(&self) -> Result<Vec<Meta>, MetasError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT * FROM metas ORDER BY creado_en DESC")?;
        let rows = stmt
            .query_map([], Self::row_to_meta)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    fn cancelar(&self, id: Uuid) -> Result<Meta, MetasError> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;
        let meta = tx
            .query_row(
                "SELECT * FROM metas WHERE id=?1",
                params![id.to_string()],
                Self::row_to_meta,
            )
            .optional()?
            .ok_or(MetasError::NotFound(id))?;
        if meta.estado == EstadoMeta::Cancelada {
            return Err(MetasError::NoPermitido("La meta ya está cancelada".into()));
        }
        tx.execute(
            "UPDATE metas SET estado='cancelada', actualizado_en=?2 WHERE id=?1",
            params![id.to_string(), Local::now().to_rfc3339()],
        )?;
        let recargada = tx
            .query_row(
                "SELECT * FROM metas WHERE id=?1",
                params![id.to_string()],
                Self::row_to_meta,
            )
            .optional()?
            .ok_or(MetasError::NotFound(id))?;
        tx.commit()?;
        Ok(recargada)
    }

    fn reactivar(&self, id: Uuid) -> Result<Meta, MetasError> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;
        let meta = tx
            .query_row(
                "SELECT * FROM metas WHERE id=?1",
                params![id.to_string()],
                Self::row_to_meta,
            )
            .optional()?
            .ok_or(MetasError::NotFound(id))?;
        if meta.estado != EstadoMeta::Cancelada {
            return Err(MetasError::NoPermitido(
                "Solo se puede reactivar una meta cancelada".into(),
            ));
        }
        let acumulado: i64 = tx
            .query_row(
                "SELECT COALESCE(SUM(monto_centavos), 0) FROM movements WHERE tipo='aporte' AND meta_id=?1",
                params![id.to_string()],
                |row| row.get(0),
            )?;
        let nuevo = recalcular_estado(acumulado, meta.monto_objetivo_centavos);
        let estado_str = match nuevo {
            EstadoMeta::Activa => "activa",
            EstadoMeta::Completada => "completada",
            EstadoMeta::Cancelada => "cancelada",
        };
        tx.execute(
            "UPDATE metas SET estado=?2, actualizado_en=?3 WHERE id=?1",
            params![id.to_string(), estado_str, Local::now().to_rfc3339()],
        )?;
        let recargada = tx
            .query_row(
                "SELECT * FROM metas WHERE id=?1",
                params![id.to_string()],
                Self::row_to_meta,
            )
            .optional()?
            .ok_or(MetasError::NotFound(id))?;
        tx.commit()?;
        Ok(recargada)
    }

    fn migrate(&self) -> Result<(), MetasError> {
        self.init_schema()?;
        Ok(())
    }

    fn apartados_de_meta(&self, id: Uuid) -> Result<Vec<Movement>, MetasError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT * FROM movements WHERE tipo='aporte' AND meta_id=?1 ORDER BY fecha DESC, creado_en DESC",
        )?;
        let rows = stmt
            .query_map(params![id.to_string()], Self::row_to_movement)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }
}
