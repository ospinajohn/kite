//! SearchPort — Contrato de búsqueda de movimientos (v2+)
//!
//! POSPUESTO en v1 (FR-007). Definido aquí para future-proofing y migración sin breaking changes.

use crate::models::movement::Movement;
use chrono::NaiveDate;
use std::error::Error;
use uuid::Uuid;

/// Errores del puerto de búsqueda
#[derive(Debug, thiserror::Error)]
pub enum SearchError {
    #[error("Índice no disponible: {0}")]
    IndexUnavailable(String),
    #[error("Consulta inválida: {0}")]
    InvalidQuery(String),
    #[error("Error interno: {0}")]
    Internal(String),
}

/// Puerto de búsqueda (trait object safe)
pub trait SearchPort: Send + Sync {
    /// Busca movimientos por texto en concepto (case-insensitive, partial match).
    /// Retorna movimientos ordenados por relevancia y fecha desc.
    fn search_by_concepto(
        &self,
        query: &str,
        desde: Option<NaiveDate>,
        hasta: Option<NaiveDate>,
        limit: usize,
    ) -> Result<Vec<Movement>, SearchError>;

    /// Busca movimientos por monto exacto o rango.
    fn search_by_monto(
        &self,
        min_centavos: Option<i64>,
        max_centavos: Option<i64>,
        desde: Option<NaiveDate>,
        hasta: Option<NaiveDate>,
        limit: usize,
    ) -> Result<Vec<Movement>, SearchError>;

    /// Reconstruye/actualiza índice de búsqueda (ej. tras migración o bulk import).
    fn reindex(&self) -> Result<(), SearchError>;
}

#[cfg(test)]
mod contract_tests {
    use super::*;
    use crate::models::movement::{Movement, MovementType, MovementOrigin};
    use chrono::{Local, NaiveDate};

    /// Test genérico contra cualquier SearchPort (cuando se implemente)
    pub fn test_search_port<S: SearchPort>(_search: &S) {
        // TODO: implementar cuando SearchPort tenga implementación real
        // Casos esperados:
        // - search_by_concepto("almuerzo") → movimientos con "almuerzo" en concepto
        // - search_by_concepto("ALMUERZO") → case-insensitive
        // - search_by_monto(10000, 20000) → rango de montos
        // - reindex() → no error
    }
}