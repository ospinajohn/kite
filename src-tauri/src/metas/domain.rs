use chrono::{DateTime, Local, NaiveDate};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EstadoMeta {
    Activa,
    Completada,
    Cancelada,
}

#[derive(Debug, thiserror::Error)]
pub enum MetasError {
    #[error("Nombre inválido: {0}")]
    InvalidNombre(String),
    #[error("Monto inválido: {0}")]
    InvalidMonto(String),
    #[error("Fecha inválida: {0}")]
    InvalidFecha(String),
    #[error("Meta no encontrada: {0}")]
    NotFound(Uuid),
    #[error("Operación no permitida: {0}")]
    NoPermitido(String),
    #[error("Error de base de datos: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("Error de E/S: {0}")]
    Io(#[from] std::io::Error),
    #[error("Error de migración: {0}")]
    Migration(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Meta {
    pub id: Uuid,
    pub nombre: String,
    pub monto_objetivo_centavos: i64,
    pub fecha_objetivo: Option<NaiveDate>,
    pub estado: EstadoMeta,
    pub creado_en: DateTime<Local>,
    pub actualizado_en: DateTime<Local>,
}

impl Meta {
    pub fn new(
        nombre: String,
        monto_objetivo_centavos: i64,
        fecha_objetivo: Option<NaiveDate>,
    ) -> Result<Self, MetasError> {
        Self::validar(&nombre, monto_objetivo_centavos, fecha_objetivo)?;
        let now = Local::now();
        Ok(Self {
            id: Uuid::new_v4(),
            nombre: nombre.trim().to_string(),
            monto_objetivo_centavos,
            fecha_objetivo,
            estado: EstadoMeta::Activa,
            creado_en: now,
            actualizado_en: now,
        })
    }

    pub fn desde_fila(fila: MetaFila) -> Self {
        Self {
            id: fila.id,
            nombre: fila.nombre,
            monto_objetivo_centavos: fila.monto_objetivo_centavos,
            fecha_objetivo: fila.fecha_objetivo,
            estado: fila.estado,
            creado_en: fila.creado_en,
            actualizado_en: fila.actualizado_en,
        }
    }

    pub fn update(
        &self,
        nombre: String,
        monto_objetivo_centavos: i64,
        fecha_objetivo: Option<NaiveDate>,
    ) -> Result<Self, MetasError> {
        Self::validar(&nombre, monto_objetivo_centavos, fecha_objetivo)?;
        Ok(Self {
            id: self.id,
            nombre: nombre.trim().to_string(),
            monto_objetivo_centavos,
            fecha_objetivo,
            estado: self.estado,
            creado_en: self.creado_en,
            actualizado_en: Local::now(),
        })
    }

    fn validar(
        nombre: &str,
        monto_objetivo_centavos: i64,
        fecha_objetivo: Option<NaiveDate>,
    ) -> Result<(), MetasError> {
        let nombre_trim = nombre.trim();
        if nombre_trim.is_empty() {
            return Err(MetasError::InvalidNombre("El nombre es obligatorio".into()));
        }
        if nombre_trim.len() > 200 {
            return Err(MetasError::InvalidNombre(
                "El nombre no puede exceder 200 caracteres".into(),
            ));
        }
        if monto_objetivo_centavos <= 0 {
            return Err(MetasError::InvalidMonto(
                "El monto objetivo debe ser mayor que cero".into(),
            ));
        }
        if let Some(fecha) = fecha_objetivo {
            let hoy = Local::now().date_naive();
            if fecha <= hoy {
                return Err(MetasError::InvalidFecha(
                    "La fecha objetivo debe ser futura".into(),
                ));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct MetaFila {
    pub id: Uuid,
    pub nombre: String,
    pub monto_objetivo_centavos: i64,
    pub fecha_objetivo: Option<NaiveDate>,
    pub estado: EstadoMeta,
    pub creado_en: DateTime<Local>,
    pub actualizado_en: DateTime<Local>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MetaUI {
    pub meta: Meta,
    pub acumulado_centavos: i64,
    pub a_ritmo: Option<bool>,
}
