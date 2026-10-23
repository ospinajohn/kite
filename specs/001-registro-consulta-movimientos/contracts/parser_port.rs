//! ParserPort — Contrato de interpretación de mensajes en lenguaje natural
//!
//! Define la frontera entre la entrada de texto libre del usuario y la propuesta
//! de movimiento estructurada. Implementaciones: `RegexParser` (v1), `NlpParser` (futuro).

use crate::models::movement::{MovementType, MovementOrigin};
use chrono::NaiveDate;
use rust_decimal::Decimal;
use std::error::Error;

/// Errores del parser
#[derive(Debug, thiserror::Error)]
pub enum ParserError {
    #[error("Patrón no reconocido")]
    Unrecognized,
    #[error("Monto inválido: {0}")]
    InvalidAmount(String),
    #[error("Fecha inválida: {0}")]
    InvalidDate(String),
    #[error("Error interno: {0}")]
    Internal(String),
}

/// Propuesta de movimiento extraída de un mensaje
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedProposal {
    pub tipo: MovementType,
    pub monto_centavos: i64,
    pub concepto: String,
    pub fecha: NaiveDate,          // hoy por defecto
    pub confidence: f32,           // 0.0–1.0; v1: 1.0 si matchea, 0.0 si no
    pub origen: MovementOrigin,    // siempre Mensaje
}

impl ParsedProposal {
    pub fn new(
        tipo: MovementType,
        monto_centavos: i64,
        concepto: String,
        fecha: NaiveDate,
    ) -> Self {
        Self {
            tipo,
            monto_centavos,
            concepto,
            fecha,
            confidence: 1.0,
            origen: MovementOrigin::Mensaje,
        }
    }
}

/// Puerto de parsing (trait object safe)
pub trait ParserPort: Send + Sync {
    /// Intenta extraer una propuesta de movimiento del texto.
    /// Retorna `Ok(Some(proposal))` si reconoce patrón financiero válido.
    /// Retorna `Ok(None)` si el texto no parece un registro financiero.
    /// Retorna `Err(ParserError)` solo si hay error interno inesperado.
    fn parse(&self, text: &str) -> Result<Option<ParsedProposal>, ParserError>;

    /// Lista de patrones/ejemplos soportados (para ayuda/UI)
    fn supported_patterns(&self) -> Vec<&'static str> {
        vec![]
    }
}

#[cfg(test)]
mod contract_tests {
    use super::*;
    use crate::models::movement::MovementType;
    use chrono::{Local, NaiveDate};

    /// Test genérico contra cualquier ParserPort
    pub fn test_parser_port<P: ParserPort>(parser: &P) {
        let today = Local::now().date_naive();

        // Gasto con "mil"
        let r = parser.parse("gasté 18 mil en almuerzo").expect("parse ok");
        assert!(r.is_some());
        let p = r.unwrap();
        assert_eq!(p.tipo, MovementType::Gasto);
        assert_eq!(p.monto_centavos, 18_000);
        assert_eq!(p.concepto, "almuerzo");
        assert_eq!(p.fecha, today);
        assert_eq!(p.confidence, 1.0);

        // Gasto simple
        let r = parser.parse("gasté 12.000 en transporte").expect("parse ok");
        assert!(r.is_some());
        let p = r.unwrap();
        assert_eq!(p.monto_centavos, 12_000);
        assert_eq!(p.concepto, "transporte");

        // Ingreso
        let r = parser.parse("recibí 1.500.000 de pago quincena").expect("parse ok");
        assert!(r.is_some());
        let p = r.unwrap();
        assert_eq!(p.tipo, MovementType::Ingreso);
        assert_eq!(p.monto_centavos, 1_500_000);
        assert_eq!(p.concepto, "pago quincena");

        // Préstamo → gasto con concepto
        let r = parser.parse("le presté 50 mil a carlos").expect("parse ok");
        assert!(r.is_some());
        let p = r.unwrap();
        assert_eq!(p.tipo, MovementType::Gasto);
        assert_eq!(p.monto_centavos, 50_000);
        assert_eq!(p.concepto, "carlos");

        // Sin monto → None
        let r = parser.parse("compré algo").expect("parse ok");
        assert!(r.is_none(), "sin monto debe retornar None");

        // Texto no financiero → None
        let r = parser.parse("hola mundo").expect("parse ok");
        assert!(r.is_none());

        // Confianza 1.0 en matcheos
        let r = parser.parse("gasté 100 en café").expect("parse ok");
        assert!(r.is_some());
        assert_eq!(r.unwrap().confidence, 1.0);
    }

    /// Casos edge para validar robustez
    pub fn test_parser_edge_cases<P: ParserPort>(parser: &P) {
        // Magnitudes
        assert_eq!(
            parser.parse("gasté 1k en café").unwrap().unwrap().monto_centavos,
            1_000
        );
        assert_eq!(
            parser.parse("gasté 2 M en casa").unwrap().unwrap().monto_centavos,
            2_000_000
        );
        assert_eq!(
            parser.parse("gasté 1 millón en algo").unwrap().unwrap().monto_centavos,
            1_000_000
        );

        // Separadores
        assert_eq!(
            parser.parse("gasté 1.200 en algo").unwrap().unwrap().monto_centavos,
            1_200
        );
        assert_eq!(
            parser.parse("gasté 1,200 en algo").unwrap().unwrap().monto_centavos,
            1_200
        );
        assert_eq!(
            parser.parse("gasté 1_200 en algo").unwrap().unwrap().monto_centavos,
            1_200
        );

        // Case insensitive
        assert!(parser.parse("GASTÉ 100 EN CAFÉ").unwrap().is_some());
        assert!(parser.parse("Gasté 100 en café").unwrap().is_some());

        // Espacios extra
        assert!(parser.parse("  gasté 100 en café  ").unwrap().is_some());
    }
}