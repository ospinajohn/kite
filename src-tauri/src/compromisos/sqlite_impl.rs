use crate::compromisos::port::{CompromisosError, CompromisosPort, PagoCompromiso};
use crate::models::{
    Compromiso, CompromisoFila, Duracion, EstadoCompromiso, Frecuencia, Movement, Quincena,
};
use chrono::{DateTime, Local, NaiveDate};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

pub struct SqliteCompromisosStorage {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteCompromisosStorage {
    pub fn new(db_path: &Path) -> Result<Self, CompromisosError> {
        std::fs::create_dir_all(db_path.parent().unwrap())?;
        let conn = Connection::open(db_path)?;
        conn.execute_batch("PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL;")?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    fn init_schema(&self) -> Result<(), CompromisosError> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch(
            r#"
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
            "#,
        )?;
        Ok(())
    }

    /// Mapea una fila al modelo. Igual que en el storage de movimientos, devuelve
    /// `rusqlite::Error` para que funcione con `query_row(...).optional()` y `query_map`;
    /// los errores de integridad viajan como `ToSqlConversionFailure`.
    fn row_to_compromiso(row: &rusqlite::Row) -> Result<Compromiso, rusqlite::Error> {
        row_to_compromiso_impl(row).map_err(|e: CompromisosError| match e {
            CompromisosError::Database(db) => db,
            other => rusqlite::Error::ToSqlConversionFailure(Box::new(other)),
        })
    }

    fn mapear_frecuencia(f: &Frecuencia) -> (&'static str, Option<i64>, Option<&'static str>) {
        match f {
            Frecuencia::Mensual(dia) => ("mensual", Some(i64::from(*dia)), None),
            Frecuencia::Quincenal(Quincena::Primera) => ("quincenal", None, Some("primera")),
            Frecuencia::Quincenal(Quincena::Segunda) => ("quincenal", None, Some("segunda")),
        }
    }

    fn mapear_duracion(d: &Duracion) -> (&'static str, Option<String>, Option<i64>) {
        match d {
            Duracion::Indefinido => ("indefinido", None, None),
            Duracion::FechaFin(f) => ("fecha_fin", Some(f.format("%Y-%m-%d").to_string()), None),
            Duracion::Cuotas(n) => ("cuotas", None, Some(i64::from(*n))),
        }
    }
}

fn row_to_compromiso_impl(row: &rusqlite::Row) -> Result<Compromiso, CompromisosError> {
    let id: Uuid = Uuid::parse_str(&row.get::<_, String>("id")?)
        .map_err(|_| CompromisosError::Integrity("UUID de compromiso inválido".into()))?;
    let concepto: String = row.get("concepto")?;
    let monto_centavos: i64 = row.get("monto_centavos")?;
    let frecuencia_str: String = row.get("frecuencia")?;
    let dia_mes: Option<i64> = row.get("dia_mes")?;
    let quincena_str: Option<String> = row.get("quincena")?;
    let duracion_tipo: String = row.get("duracion_tipo")?;
    let fecha_fin_str: Option<String> = row.get("fecha_fin")?;
    let cuotas_total: Option<i64> = row.get("cuotas_total")?;
    let cuotas_confirmadas: i64 = row.get("cuotas_confirmadas")?;
    let estado_str: String = row.get("estado")?;
    let creado_en_str: String = row.get("creado_en")?;
    let actualizado_en_str: String = row.get("actualizado_en")?;

    let frecuencia = match frecuencia_str.as_str() {
        "mensual" => Frecuencia::Mensual(
            u8::try_from(
                dia_mes.ok_or_else(|| CompromisosError::Integrity("mensual sin día".into()))?,
            )
            .map_err(|_| CompromisosError::Integrity("día fuera de rango".into()))?,
        ),
        "quincenal" => Frecuencia::Quincenal(match quincena_str.as_deref() {
            Some("primera") => Quincena::Primera,
            Some("segunda") => Quincena::Segunda,
            _ => return Err(CompromisosError::Integrity("quincena inválida".into())),
        }),
        _ => return Err(CompromisosError::Integrity("frecuencia inválida".into())),
    };

    let duracion = match duracion_tipo.as_str() {
        "indefinido" => Duracion::Indefinido,
        "fecha_fin" => {
            Duracion::FechaFin(parse_fecha(&fecha_fin_str.ok_or_else(|| {
                CompromisosError::Integrity("fecha_fin sin valor".into())
            })?)?)
        }
        "cuotas" => Duracion::Cuotas(
            u32::try_from(
                cuotas_total
                    .ok_or_else(|| CompromisosError::Integrity("cuotas sin total".into()))?,
            )
            .map_err(|_| CompromisosError::Integrity("cuotas fuera de rango".into()))?,
        ),
        _ => return Err(CompromisosError::Integrity("duración inválida".into())),
    };

    let estado = match estado_str.as_str() {
        "activo" => EstadoCompromiso::Activo,
        "pausado" => EstadoCompromiso::Pausado,
        _ => return Err(CompromisosError::Integrity("estado inválido".into())),
    };

    Ok(Compromiso::desde_fila(CompromisoFila {
        id,
        concepto,
        monto_centavos,
        frecuencia,
        duracion,
        estado,
        cuotas_confirmadas: u32::try_from(cuotas_confirmadas)
            .map_err(|_| CompromisosError::Integrity("cuotas_confirmadas negativas".into()))?,
        creado_en: parse_timestamp(&creado_en_str)?,
        actualizado_en: parse_timestamp(&actualizado_en_str)?,
    }))
}

impl CompromisosPort for SqliteCompromisosStorage {
    fn save(&self, compromiso: Compromiso) -> Result<Compromiso, CompromisosError> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;
        let (frec_str, dia_mes, quincena) = Self::mapear_frecuencia(&compromiso.frecuencia);
        let (dur_str, fecha_fin, cuotas_total) = Self::mapear_duracion(&compromiso.duracion);
        tx.execute(
            r#"
            INSERT INTO compromisos (
                id, concepto, monto_centavos, frecuencia, dia_mes, quincena,
                duracion_tipo, fecha_fin, cuotas_total, cuotas_confirmadas,
                estado, creado_en, actualizado_en
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
            "#,
            params![
                compromiso.id.to_string(),
                compromiso.concepto,
                compromiso.monto_centavos,
                frec_str,
                dia_mes,
                quincena,
                dur_str,
                fecha_fin,
                cuotas_total,
                compromiso.cuotas_confirmadas,
                match compromiso.estado {
                    EstadoCompromiso::Activo => "activo",
                    EstadoCompromiso::Pausado => "pausado",
                },
                compromiso.creado_en.to_rfc3339(),
                compromiso.actualizado_en.to_rfc3339(),
            ],
        )?;
        tx.commit()?;
        Ok(compromiso)
    }

    fn update(&self, compromiso: Compromiso) -> Result<Compromiso, CompromisosError> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;
        let (frec_str, dia_mes, quincena) = Self::mapear_frecuencia(&compromiso.frecuencia);
        let (dur_str, fecha_fin, cuotas_total) = Self::mapear_duracion(&compromiso.duracion);
        let affected = tx.execute(
            r#"
            UPDATE compromisos SET
                concepto = ?2, monto_centavos = ?3, frecuencia = ?4, dia_mes = ?5,
                quincena = ?6, duracion_tipo = ?7, fecha_fin = ?8, cuotas_total = ?9,
                estado = ?10, actualizado_en = ?11
            WHERE id = ?1
            "#,
            params![
                compromiso.id.to_string(),
                compromiso.concepto,
                compromiso.monto_centavos,
                frec_str,
                dia_mes,
                quincena,
                dur_str,
                fecha_fin,
                cuotas_total,
                match compromiso.estado {
                    EstadoCompromiso::Activo => "activo",
                    EstadoCompromiso::Pausado => "pausado",
                },
                compromiso.actualizado_en.to_rfc3339(),
            ],
        )?;
        if affected == 0 {
            return Err(CompromisosError::NotFound(compromiso.id));
        }
        tx.commit()?;
        Ok(compromiso)
    }

    fn delete(&self, id: Uuid) -> Result<bool, CompromisosError> {
        let conn = self.conn.lock().unwrap();
        let affected = conn.execute(
            "DELETE FROM compromisos WHERE id = ?1",
            params![id.to_string()],
        )?;
        Ok(affected > 0)
    }

    fn get_by_id(&self, id: Uuid) -> Result<Option<Compromiso>, CompromisosError> {
        let conn = self.conn.lock().unwrap();
        let row = conn
            .query_row(
                "SELECT * FROM compromisos WHERE id = ?1",
                params![id.to_string()],
                Self::row_to_compromiso,
            )
            .optional()?;
        Ok(row)
    }

    fn list(&self) -> Result<Vec<Compromiso>, CompromisosError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT * FROM compromisos ORDER BY creado_en DESC")?;
        let rows = stmt
            .query_map([], Self::row_to_compromiso)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    fn set_estado(
        &self,
        id: Uuid,
        estado: EstadoCompromiso,
    ) -> Result<Compromiso, CompromisosError> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;
        let estado_str = match estado {
            EstadoCompromiso::Activo => "activo",
            EstadoCompromiso::Pausado => "pausado",
        };
        let affected = tx.execute(
            "UPDATE compromisos SET estado = ?2, actualizado_en = ?3 WHERE id = ?1",
            params![id.to_string(), estado_str, Local::now().to_rfc3339()],
        )?;
        if affected == 0 {
            return Err(CompromisosError::NotFound(id));
        }
        // Lee dentro de la misma transacción: get_by_id tomaría el mismo lock (deadlock).
        let recargado = tx
            .query_row(
                "SELECT * FROM compromisos WHERE id = ?1",
                params![id.to_string()],
                Self::row_to_compromiso,
            )
            .optional()?
            .ok_or(CompromisosError::NotFound(id))?;
        tx.commit()?;
        Ok(recargado)
    }

    fn periodos_pagados(&self, id: Uuid) -> Result<Vec<String>, CompromisosError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT periodo_key FROM compromisos_pagos WHERE compromiso_id = ?1 ORDER BY periodo_key",
        )?;
        let keys = stmt
            .query_map(params![id.to_string()], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(keys)
    }

    fn registrar_pago(
        &self,
        compromiso_id: Uuid,
        periodo_key: String,
        movimiento: Movement,
    ) -> Result<PagoCompromiso, CompromisosError> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;

        // El compromiso debe existir.
        let duracion_tipo: String = tx
            .query_row(
                "SELECT duracion_tipo FROM compromisos WHERE id = ?1",
                params![compromiso_id.to_string()],
                |row| row.get(0),
            )
            .optional()?
            .ok_or(CompromisosError::NotFound(compromiso_id))?;

        let pago = PagoCompromiso {
            compromiso_id,
            periodo_key: periodo_key.clone(),
            movimiento_id: movimiento.id,
            monto_pagado: movimiento.monto_centavos,
            fecha_pago: movimiento.fecha,
        };

        tx.execute(
            r#"
            INSERT INTO compromisos_pagos (
                id, compromiso_id, periodo_key, movimiento_id, monto_pagado, fecha_pago
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            "#,
            params![
                Uuid::new_v4().to_string(),
                pago.compromiso_id.to_string(),
                pago.periodo_key,
                pago.movimiento_id.to_string(),
                pago.monto_pagado,
                pago.fecha_pago.format("%Y-%m-%d").to_string(),
            ],
        )
        .map_err(|e| match e {
            rusqlite::Error::SqliteFailure(err, _)
                if err.code == rusqlite::ErrorCode::ConstraintViolation =>
            {
                CompromisosError::Integrity(format!(
                    "El período {periodo_key} ya tiene un pago confirmado"
                ))
            }
            other => CompromisosError::Database(other),
        })?;

        // FR-C11: el contador de cuotas solo incrementa en duración por cuotas.
        if duracion_tipo == "cuotas" {
            tx.execute(
                "UPDATE compromisos SET cuotas_confirmadas = cuotas_confirmadas + 1, actualizado_en = ?2 WHERE id = ?1",
                params![compromiso_id.to_string(), Local::now().to_rfc3339()],
            )?;
        }

        tx.commit()?;
        Ok(pago)
    }

    fn confirmar_pago_atomico(
        &self,
        compromiso_id: Uuid,
        periodo_key: String,
        movimiento: Movement,
    ) -> Result<Movement, CompromisosError> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;

        let duracion_tipo: String = tx
            .query_row(
                "SELECT duracion_tipo FROM compromisos WHERE id = ?1",
                params![compromiso_id.to_string()],
                |row| row.get(0),
            )
            .optional()?
            .ok_or(CompromisosError::NotFound(compromiso_id))?;

        let ya_pagado: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM compromisos_pagos WHERE compromiso_id = ?1 AND periodo_key = ?2)",
            params![compromiso_id.to_string(), periodo_key.clone()],
            |row| row.get(0),
        )?;
        if ya_pagado {
            return Err(CompromisosError::Integrity(format!(
                "El período {periodo_key} ya tiene un pago confirmado"
            )));
        }

        let tipo_str = match movimiento.tipo {
            crate::models::MovementType::Ingreso => "ingreso",
            crate::models::MovementType::Gasto => "gasto",
            crate::models::MovementType::Aporte => "aporte",
        };
        let origen_str = match movimiento.origen {
            crate::models::MovementOrigin::Formulario => "formulario",
            crate::models::MovementOrigin::Mensaje => "mensaje",
        };

        tx.execute(
            r#"
            INSERT INTO movements (id, tipo, monto_centavos, concepto, fecha, origen, meta_id, creado_en, actualizado_en)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            "#,
            params![
                movimiento.id.to_string(),
                tipo_str,
                movimiento.monto_centavos,
                movimiento.concepto,
                movimiento.fecha.format("%Y-%m-%d").to_string(),
                origen_str,
                movimiento.meta_id.map(|id| id.to_string()),
                movimiento.creado_en.to_rfc3339(),
                movimiento.actualizado_en.to_rfc3339(),
            ],
        )?;

        tx.execute(
            r#"
            INSERT INTO compromisos_pagos (
                id, compromiso_id, periodo_key, movimiento_id, monto_pagado, fecha_pago
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            "#,
            params![
                Uuid::new_v4().to_string(),
                compromiso_id.to_string(),
                periodo_key.clone(),
                movimiento.id.to_string(),
                movimiento.monto_centavos,
                movimiento.fecha.format("%Y-%m-%d").to_string(),
            ],
        )
        .map_err(|e| match e {
            rusqlite::Error::SqliteFailure(err, _)
                if err.code == rusqlite::ErrorCode::ConstraintViolation =>
            {
                CompromisosError::Integrity(format!(
                    "El período {periodo_key} ya tiene un pago confirmado"
                ))
            }
            other => CompromisosError::Database(other),
        })?;

        if duracion_tipo == "cuotas" {
            tx.execute(
                "UPDATE compromisos SET cuotas_confirmadas = cuotas_confirmadas + 1, actualizado_en = ?2 WHERE id = ?1",
                params![compromiso_id.to_string(), Local::now().to_rfc3339()],
            )?;
        }

        tx.commit()?;
        Ok(movimiento)
    }

    fn historial_pagos(&self, id: Uuid) -> Result<Vec<PagoCompromiso>, CompromisosError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            r#"
            SELECT compromiso_id, periodo_key, movimiento_id, monto_pagado, fecha_pago
            FROM compromisos_pagos WHERE compromiso_id = ?1 ORDER BY fecha_pago DESC
            "#,
        )?;
        let pagos = stmt
            .query_map(params![id.to_string()], |row| {
                Ok(PagoCompromiso {
                    compromiso_id: Uuid::parse_str(&row.get::<_, String>("compromiso_id")?)
                        .map_err(|_| {
                            rusqlite::Error::InvalidColumnType(
                                0,
                                "compromiso_id".into(),
                                rusqlite::types::Type::Text,
                            )
                        })?,
                    periodo_key: row.get("periodo_key")?,
                    movimiento_id: Uuid::parse_str(&row.get::<_, String>("movimiento_id")?)
                        .map_err(|_| {
                            rusqlite::Error::InvalidColumnType(
                                0,
                                "movimiento_id".into(),
                                rusqlite::types::Type::Text,
                            )
                        })?,
                    monto_pagado: row.get("monto_pagado")?,
                    fecha_pago: parse_fecha(&row.get::<_, String>("fecha_pago")?)
                        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(pagos)
    }

    fn migrate(&self) -> Result<(), CompromisosError> {
        self.init_schema()?;
        let conn = self.conn.lock().unwrap();
        let version: i64 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;
        if version < 2 {
            conn.execute("PRAGMA user_version = 2", [])?;
        }
        Ok(())
    }
}

fn parse_fecha(s: &str) -> Result<NaiveDate, CompromisosError> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .map_err(|_| CompromisosError::Integrity(format!("Fecha inválida: {s}")))
}

fn parse_timestamp(s: &str) -> Result<DateTime<Local>, CompromisosError> {
    DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Local))
        .map_err(|_| CompromisosError::Integrity(format!("Timestamp inválido: {s}")))
}
