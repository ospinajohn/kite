use crate::models::{Compromiso, EstadoCompromiso, Movement};
use chrono::NaiveDate;
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum CompromisosError {
    #[error("Compromiso no encontrado: {0}")]
    NotFound(Uuid),
    #[error("Violación de integridad: {0}")]
    Integrity(String),
    #[error("Error de base de datos: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("Error de E/S: {0}")]
    Io(#[from] std::io::Error),
    #[error("Error de migración: {0}")]
    Migration(String),
}

/// Un pago confirmado de un compromiso, con el período que cubre.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PagoCompromiso {
    pub compromiso_id: Uuid,
    pub periodo_key: String,
    pub movimiento_id: Uuid,
    pub monto_pagado: i64,
    pub fecha_pago: NaiveDate,
}

pub trait CompromisosPort: Send + Sync {
    fn save(&self, compromiso: Compromiso) -> Result<Compromiso, CompromisosError>;
    fn update(&self, compromiso: Compromiso) -> Result<Compromiso, CompromisosError>;
    fn delete(&self, id: Uuid) -> Result<bool, CompromisosError>;
    fn get_by_id(&self, id: Uuid) -> Result<Option<Compromiso>, CompromisosError>;
    fn list(&self) -> Result<Vec<Compromiso>, CompromisosError>;
    fn set_estado(
        &self,
        id: Uuid,
        estado: EstadoCompromiso,
    ) -> Result<Compromiso, CompromisosError>;

    /// Claves de período ya pagados de un compromiso (deduplicación FR-C07).
    fn periodos_pagados(&self, id: Uuid) -> Result<Vec<String>, CompromisosError>;

    /// Registra el pago de un período: inserta el vínculo con el movimiento creado
    /// e incrementa el contador de cuotas si la duración es por cuotas.
    /// Falla con Integrity si el período ya está pagado.
    fn registrar_pago(
        &self,
        compromiso_id: Uuid,
        periodo_key: String,
        movimiento: Movement,
    ) -> Result<PagoCompromiso, CompromisosError>;

    fn historial_pagos(&self, id: Uuid) -> Result<Vec<PagoCompromiso>, CompromisosError>;

    fn migrate(&self) -> Result<(), CompromisosError>;
}
