//! BackupPort — Contrato de exportación/restauración de respaldos (v2+)
//!
//! POSPUESTO en v1 (FR-012). Definido aquí para future-proofing.

use crate::models::movement::Movement;
use chrono::NaiveDate;
use std::error::Error;
use std::path::Path;

/// Errores del puerto de respaldo
#[derive(Debug, thiserror::Error)]
pub enum BackupError {
    #[error("Error de E/S: {0}")]
    Io(#[from] std::io::Error),
    #[error("Formato inválido: {0}")]
    InvalidFormat(String),
    #[error("Respaldo corrupto: {0}")]
    Corrupt(String),
    #[error("Versión incompatible: {0}")]
    VersionMismatch(String),
    #[error("Error interno: {0}")]
    Internal(String),
}

/// Formato de respaldo soportado
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackupFormat {
    /// JSON Lines: un movimiento por línea, campos completos
    JsonLines,
    /// CSV con headers: id,tipo,monto_centavos,concepto,fecha,origen,creado_en,actualizado_en
    Csv,
    /// Dump SQLite completo (copia bit-a-bit del archivo .db)
    SqliteDump,
}

/// Metadatos de un respaldo
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackupMetadata {
    pub formato: BackupFormat,
    pub version_esquema: u32,
    pub total_movimientos: usize,
    pub fecha_desde: Option<NaiveDate>,
    pub fecha_hasta: Option<NaiveDate>,
    pub creado_en: chrono::DateTime<chrono::Local>,
    pub checksum_sha256: String,
}

/// Puerto de respaldo (trait object safe)
pub trait BackupPort: Send + Sync {
    /// Exporta todos los movimientos (o rango) al formato especificado.
    /// Retorna ruta del archivo creado y metadatos.
    fn export(
        &self,
        destino: &Path,
        formato: BackupFormat,
        desde: Option<NaiveDate>,
        hasta: Option<NaiveDate>,
    ) -> Result<BackupMetadata, BackupError>;

    /// Restaura movimientos desde un archivo de respaldo.
    /// Comportamiento de restauración:
    /// - `Replace`: borra datos actuales y carga solo el respaldo
    /// - `Merge`: inserta movimientos que no existan (por id), actualiza los que difieran
    /// - `Confirm`: pide confirmación por cada conflicto (no implementable headless; para UI)
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum RestoreMode {
        Replace,
        Merge,
    }

    fn restore(
        &self,
        origen: &Path,
        modo: RestoreMode,
    ) -> Result<BackupMetadata, BackupError>;

    /// Valida integridad de un archivo de respaldo sin restaurar.
    fn validate(&self, archivo: &Path) -> Result<BackupMetadata, BackupError>;
}

#[cfg(test)]
mod contract_tests {
    use super::*;
    use std::path::PathBuf;

    /// Test genérico contra cualquier BackupPort (cuando se implemente)
    pub fn test_backup_port<B: BackupPort>(_backup: &B) {
        // TODO: implementar cuando BackupPort tenga implementación real
        // Casos esperados:
        // - export(JsonLines) → archivo .jsonl válido, metadatos correctos
        // - export(Csv) → archivo .csv válido
        // - export(SqliteDump) → copia fiel del .db
        // - restore(Replace) → datos idénticos al export
        // - restore(Merge) → sin duplicados, respeta ids existentes
        // - validate() → detecta corrupción, versión incompatible
    }
}