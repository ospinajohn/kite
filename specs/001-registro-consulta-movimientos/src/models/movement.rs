//! Modelo de dominio: Movimiento
//!
//! Entidad principal del núcleo de registro. Inmutable en tipo y origen;
//! monto, concepto y fecha editables con validación.

use chrono::{DateTime, Local, NaiveDate};
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MovementType {
    Ingreso,
    Gasto,
}

impl MovementType {
    pub fn sign(&self) -> i8 {
        match self {
            MovementType::Ingreso => 1,
            MovementType::Gasto => -1,
        }
    }
    pub fn label(&self) -> &'static str {
        match self {
            MovementType::Ingreso => "ingreso",
            MovementType::Gasto => "gasto",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MovementOrigin {
    Formulario,
    Mensaje,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Movement {
    pub id: Uuid,
    pub tipo: MovementType,
    pub monto_centavos: i64,
    pub concepto: String,
    pub fecha: NaiveDate,
    pub origen: MovementOrigin,
    pub creado_en: DateTime<Local>,
    pub actualizado_en: DateTime<Local>,
}

impl Movement {
    /// Crea un nuevo movimiento con validaciones de dominio.
    /// `monto_centavos` en centavos de COP (1 COP = 1 centavo).
    pub fn new(
        tipo: MovementType,
        monto_centavos: i64,
        concepto: String,
        fecha: NaiveDate,
        origen: MovementOrigin,
    ) -> Result<Self, MovementError> {
        // Validaciones
        if monto_centavos <= 0 {
            return Err(MovementError::InvalidAmount(
                "El monto debe ser mayor que cero".into(),
            ));
        }
        let concepto_trim = concepto.trim();
        if concepto_trim.is_empty() {
            return Err(MovementError::InvalidConcept(
                "El concepto es obligatorio".into(),
            ));
        }
        if concepto_trim.len() > 200 {
            return Err(MovementError::InvalidConcept(
                "El concepto no puede exceder 200 caracteres".into(),
            ));
        }
        let today = Local::now().date_naive();
        if fecha > today {
            return Err(MovementError::InvalidDate(
                "La fecha no puede ser futura".into(),
            ));
        }

        let now = Local::now();
        Ok(Self {
            id: Uuid::new_v4(),
            tipo,
            monto_centavos,
            concepto: concepto_trim.to_string(),
            fecha,
            origen,
            creado_en: now,
            actualizado_en: now,
        })
    }

    /// Actualiza campos editables (monto, concepto, fecha).
    /// Retorna nuevo Movement con `actualizado_en` refrescado.
    pub fn update(
        &self,
        monto_centavos: i64,
        concepto: String,
        fecha: NaiveDate,
    ) -> Result<Self, MovementError> {
        if monto_centavos <= 0 {
            return Err(MovementError::InvalidAmount(
                "El monto debe ser mayor que cero".into(),
            ));
        }
        let concepto_trim = concepto.trim();
        if concepto_trim.is_empty() {
            return Err(MovementError::InvalidConcept(
                "El concepto es obligatorio".into(),
            ));
        }
        if concepto_trim.len() > 200 {
            return Err(MovementError::InvalidConcept(
                "El concepto no puede exceder 200 caracteres".into(),
            ));
        }
        let today = Local::now().date_naive();
        if fecha > today {
            return Err(MovementError::InvalidDate(
                "La fecha no puede ser futura".into(),
            ));
        }

        Ok(Self {
            id: self.id,
            tipo: self.tipo,          // inmutable
            monto_centavos,
            concepto: concepto_trim.to_string(),
            fecha,
            origen: self.origen,      // inmutable
            creado_en: self.creado_en, // inmutable
            actualizado_en: Local::now(),
        })
    }

    /// Formato legible para UI: "+ $1.500.000" / "− $18.000"
    pub fn format_monto(&self) -> String {
        let sign = if self.tipo == MovementType::Ingreso { "+" } else { "−" };
        let abs = self.monto_centavos.abs();
        let miles = abs / 1_000;
        let resto = abs % 1_000;
        if miles > 0 {
            format!("{sign} ${miles}.{resto:03}")
        } else {
            format!("{sign} ${resto}")
        }
    }

    /// Para ordenación: fecha desc, luego creado_en desc
    pub fn sort_key(&self) -> (NaiveDate, DateTime<Local>) {
        (self.fecha, self.creado_en)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum MovementError {
    #[error("Monto inválido: {0}")]
    InvalidAmount(String),
    #[error("Concepto inválido: {0}")]
    InvalidConcept(String),
    #[error("Fecha inválida: {0}")]
    InvalidDate(String),
}

impl fmt::Display for Movement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}  {}  [{}]  {}",
            self.format_monto(),
            self.concepto,
            self.fecha.format("%d %b"),
            self.tipo.label()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Local, NaiveDate};

    #[test]
    fn new_gasto_ok() {
        let m = Movement::new(
            MovementType::Gasto,
            18_000,
            "Almuerzo".into(),
            Local::now().date_naive(),
            MovementOrigin::Formulario,
        ).unwrap();
        assert_eq!(m.tipo, MovementType::Gasto);
        assert_eq!(m.monto_centavos, 18_000);
        assert_eq!(m.concepto, "Almuerzo");
        assert_eq!(m.origen, MovementOrigin::Formulario);
    }

    #[test]
    fn new_ingreso_ok() {
        let m = Movement::new(
            MovementType::Ingreso,
            1_500_000,
            "Pago quincena".into(),
            NaiveDate::from_ymd_opt(2026, 10, 15).unwrap(),
            MovementOrigin::Mensaje,
        ).unwrap();
        assert_eq!(m.tipo, MovementType::Ingreso);
        assert_eq!(m.format_monto(), "+ $1.500.000");
    }

    #[test]
    fn new_rechaza_monto_cero() {
        let err = Movement::new(
            MovementType::Gasto,
            0,
            "Test".into(),
            Local::now().date_naive(),
            MovementOrigin::Formulario,
        ).unwrap_err();
        assert!(matches!(err, MovementError::InvalidAmount(_)));
    }

    #[test]
    fn new_rechaza_concepto_vacio() {
        let err = Movement::new(
            MovementType::Gasto,
            1000,
            "   ".into(),
            Local::now().date_naive(),
            MovementOrigin::Formulario,
        ).unwrap_err();
        assert!(matches!(err, MovementError::InvalidConcept(_)));
    }

    #[test]
    fn new_rechaza_fecha_futura() {
        let tomorrow = Local::now().date_naive() + chrono::Days::new(1);
        let err = Movement::new(
            MovementType::Gasto,
            1000,
            "Test".into(),
            tomorrow,
            MovementOrigin::Formulario,
        ).unwrap_err();
        assert!(matches!(err, MovementError::InvalidDate(_)));
    }

    #[test]
    fn update_refresca_timestamp_y_campos() {
        let mut m = Movement::new(
            MovementType::Gasto,
            18_000,
            "Almuerzo".into(),
            Local::now().date_naive(),
            MovementOrigin::Formulario,
        ).unwrap();
        let old_updated = m.actualizado_en;

        // Pequeña pausa para que timestamp cambie
        std::thread::sleep(std::time::Duration::from_millis(10));

        m = m.update(19_000, "Almuerzo corregido".into(), m.fecha).unwrap();
        assert_eq!(m.monto_centavos, 19_000);
        assert_eq!(m.concepto, "Almuerzo corregido");
        assert!(m.actualizado_en > old_updated);
        assert_eq!(m.creado_en, old_updated - (m.actualizado_en - old_updated)); // created inmutable
    }

    #[test]
    fn update_no_cambia_tipo_ni_origen() {
        let m = Movement::new(
            MovementType::Gasto,
            18_000,
            "Test".into(),
            Local::now().date_naive(),
            MovementOrigin::Mensaje,
        ).unwrap();
        let updated = m.update(20_000, "Nuevo".into(), m.fecha).unwrap();
        assert_eq!(updated.tipo, MovementType::Gasto);
        assert_eq!(updated.origen, MovementOrigin::Mensaje);
    }

    #[test]
    fn sort_key_ordena_fecha_desc_luego_creado_desc() {
        let today = Local::now().date_naive();
        let m1 = Movement::new(MovementType::Gasto, 1000, "A".into(), today, MovementOrigin::Formulario).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let m2 = Movement::new(MovementType::Gasto, 2000, "B".into(), today, MovementOrigin::Formulario).unwrap();
        // m2 más reciente → sort_key mayor → orden desc pone m2 antes que m1
        assert!(m2.sort_key() > m1.sort_key());
    }
}