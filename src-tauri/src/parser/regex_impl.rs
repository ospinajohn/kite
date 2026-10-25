use crate::models::MovementType;
use crate::parser::port::{ParserPort, ParserError, ParsedProposal};
use regex::Regex;
use std::sync::LazyLock;

static PATTERNS: LazyLock<Vec<(Regex, MovementType)>> = LazyLock::new(|| {
    vec![
        // Gasto con "ayer" opcional: "gasté ayer 18 mil en almuerzo" o "gasté 18 mil en almuerzo"
        // Soporta magnitudes con o sin espacio: "4 millones", "4millones", "4M", "4M"
        // mill[oó]n(es)? = "millón" o "millones" (con o sin acento)
        (Regex::new(r"(?i)^\s*gast[ée]\s+(?:ayer\s+)?(?P<monto>[\d.,_]+)\s*(?P<mag>mil|k|mill[oó]n(es)?|m)?\s+en\s+(?P<concepto>.+)$").unwrap(), MovementType::Gasto),
        // Gasto simple con "ayer" opcional: "gasté ayer 12.000 en transporte"
        (Regex::new(r"(?i)^\s*gast[ée]\s+(?:ayer\s+)?(?P<monto>[\d.,_]+)\s+en\s+(?P<concepto>.+)$").unwrap(), MovementType::Gasto),
        // Ingreso con "ayer" opcional: "recibí ayer 1.500.000 de pago quincena"
        (Regex::new(r"(?i)^\s*recib[ií]\s+(?:ayer\s+)?(?P<monto>[\d.,_]+)\s*(?P<mag>mil|k|mill[oó]n(es)?|m)?\s+de\s+(?P<concepto>.+)$").unwrap(), MovementType::Ingreso),
        // Préstamo con "ayer" opcional: "le presté ayer 50 mil a carlos"
        (Regex::new(r"(?i)^\s*le?\s*prest[ée]\s+(?:ayer\s+)?(?P<monto>[\d.,_]+)\s*(?P<mag>mil|k|mill[oó]n(es)?|m)?\s+a\s+(?P<concepto>.+)$").unwrap(), MovementType::Gasto),
    ]
});

fn parse_monto(raw: &str, mag: Option<&str>) -> Option<i64> {
    let clean = raw.replace(['.', ',', '_'], "");
    let base: i64 = clean.parse().ok()?;
    let mag_lower = mag.map(|s| s.to_lowercase());
    let mult = match mag_lower.as_deref() {
        Some("mil") | Some("k") => 1_000,
        Some("millón") | Some("millones") | Some("m") => 1_000_000,
        _ => 1,
    };
    Some(base * mult)
}

pub struct RegexParser;

impl RegexParser {
    pub fn new() -> Self {
        Self
    }
}

impl Default for RegexParser {
    fn default() -> Self {
        Self::new()
    }
}

impl ParserPort for RegexParser {
    fn parse(&self, text: &str) -> Result<Option<ParsedProposal>, ParserError> {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return Ok(None);
        }

        for (re, tipo) in PATTERNS.iter() {
            if let Some(caps) = re.captures(trimmed) {
                let monto_str = caps.name("monto").map(|m| m.as_str()).unwrap_or("");
                let mag = caps.name("mag").map(|m| m.as_str());
                let concepto = caps.name("concepto").map(|m| m.as_str().trim()).unwrap_or("").to_string();

                if concepto.is_empty() {
                    continue;
                }

                let monto_centavos = match parse_monto(monto_str, mag) {
                    Some(m) if m > 0 => m,
                    _ => continue,
                };

                let fecha = chrono::Local::now().date_naive();

                return Ok(Some(ParsedProposal::new(
                    *tipo,
                    monto_centavos,
                    concepto,
                    fecha,
                )));
            }
        }

        Ok(None)
    }

    fn supported_patterns(&self) -> Vec<&'static str> {
        vec![
            "gasté 18 mil en almuerzo",
            "gasté 12.000 en transporte",
            "recibí 1.500.000 de pago quincena",
            "le presté 50 mil a carlos",
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::MovementType;
    use chrono::Local;

    #[test]
    fn parse_gasto_con_mil() {
        let parser = RegexParser::new();
        let r = parser.parse("gasté 18 mil en almuerzo").unwrap().unwrap();
        assert_eq!(r.tipo, MovementType::Gasto);
        assert_eq!(r.monto_centavos, 18_000);
        assert_eq!(r.concepto, "almuerzo");
        assert_eq!(r.fecha, Local::now().date_naive());
    }

    #[test]
    fn parse_gasto_simple() {
        let parser = RegexParser::new();
        let r = parser.parse("gasté 12.000 en transporte").unwrap().unwrap();
        assert_eq!(r.monto_centavos, 12_000);
        assert_eq!(r.concepto, "transporte");
    }

    #[test]
    fn parse_ingreso() {
        let parser = RegexParser::new();
        let r = parser.parse("recibí 1.500.000 de pago quincena").unwrap().unwrap();
        assert_eq!(r.tipo, MovementType::Ingreso);
        assert_eq!(r.monto_centavos, 1_500_000);
        assert_eq!(r.concepto, "pago quincena");
    }

    #[test]
    fn parse_prestamo() {
        let parser = RegexParser::new();
        let r = parser.parse("le presté 50 mil a carlos").unwrap().unwrap();
        assert_eq!(r.tipo, MovementType::Gasto);
        assert_eq!(r.monto_centavos, 50_000);
        assert_eq!(r.concepto, "carlos");
    }

    #[test]
    fn parse_sin_monto_retorna_none() {
        let parser = RegexParser::new();
        let r = parser.parse("compré algo").unwrap();
        assert!(r.is_none());
    }

    #[test]
    fn parse_texto_no_financiero_retorna_none() {
        let parser = RegexParser::new();
        let r = parser.parse("hola mundo").unwrap();
        assert!(r.is_none());
    }

    #[test]
    fn parse_magnitud_k() {
        let parser = RegexParser::new();
        let r = parser.parse("gasté 1k en café").unwrap().unwrap();
        assert_eq!(r.monto_centavos, 1_000);
    }

    #[test]
    fn parse_magnitud_M() {
        let parser = RegexParser::new();
        let r = parser.parse("gasté 2 M en casa").unwrap().unwrap();
        assert_eq!(r.monto_centavos, 2_000_000);
    }

    #[test]
    fn parse_magnitud_millon() {
        let parser = RegexParser::new();
        let r = parser.parse("gasté 1 millón en algo").unwrap().unwrap();
        assert_eq!(r.monto_centavos, 1_000_000);
    }

    #[test]
    fn parse_separadores() {
        let parser = RegexParser::new();
        assert_eq!(parser.parse("gasté 1.200 en algo").unwrap().unwrap().monto_centavos, 1_200);
        assert_eq!(parser.parse("gasté 1,200 en algo").unwrap().unwrap().monto_centavos, 1_200);
        assert_eq!(parser.parse("gasté 1_200 en algo").unwrap().unwrap().monto_centavos, 1_200);
    }

    #[test]
    fn parse_case_insensitive() {
        let parser = RegexParser::new();
        assert!(parser.parse("GASTÉ 100 EN CAFÉ").unwrap().is_some());
        assert!(parser.parse("Gasté 100 en café").unwrap().is_some());
    }

    #[test]
    fn parse_espacios_extra() {
        let parser = RegexParser::new();
        assert!(parser.parse("  gasté 100 en café  ").unwrap().is_some());
    }

    #[test]
    fn parse_con_ayer_gasto() {
        let parser = RegexParser::new();
        let r = parser.parse("gasté ayer 18 mil en almuerzo").unwrap().unwrap();
        assert_eq!(r.tipo, MovementType::Gasto);
        assert_eq!(r.monto_centavos, 18_000);
        assert_eq!(r.concepto, "almuerzo");
        assert_eq!(r.fecha, Local::now().date_naive());
    }

    #[test]
    fn parse_con_ayer_ingreso() {
        let parser = RegexParser::new();
        let r = parser.parse("recibí ayer 1.500.000 de pago quincena").unwrap().unwrap();
        assert_eq!(r.tipo, MovementType::Ingreso);
        assert_eq!(r.monto_centavos, 1_500_000);
        assert_eq!(r.concepto, "pago quincena");
        assert_eq!(r.fecha, Local::now().date_naive());
    }

    #[test]
    fn parse_con_ayer_prestamo() {
        let parser = RegexParser::new();
        let r = parser.parse("le presté ayer 50 mil a carlos").unwrap().unwrap();
        assert_eq!(r.tipo, MovementType::Gasto);
        assert_eq!(r.monto_centavos, 50_000);
        assert_eq!(r.concepto, "carlos");
        assert_eq!(r.fecha, Local::now().date_naive());
    }

    // --- Nuevos casos: plural "millones" y magnitudes sin espacio ---
    #[test]
    fn parse_millones_plural() {
        let parser = RegexParser::new();
        let r = parser.parse("gasté 4 millones en celulares").unwrap().unwrap();
        assert_eq!(r.tipo, MovementType::Gasto);
        assert_eq!(r.monto_centavos, 4_000_000);
        assert_eq!(r.concepto, "celulares");
    }

    #[test]
    fn parse_millones_plural_ingreso() {
        let parser = RegexParser::new();
        let r = parser.parse("recibí 2 millones de bono").unwrap().unwrap();
        assert_eq!(r.tipo, MovementType::Ingreso);
        assert_eq!(r.monto_centavos, 2_000_000);
        assert_eq!(r.concepto, "bono");
    }

    #[test]
    fn parse_magnitud_sin_espacio_millones() {
        let parser = RegexParser::new();
        let r = parser.parse("gasté 4millones en celulares").unwrap().unwrap();
        assert_eq!(r.monto_centavos, 4_000_000);
        assert_eq!(r.concepto, "celulares");
    }

    #[test]
    fn parse_magnitud_sin_espacio_mil() {
        let parser = RegexParser::new();
        let r = parser.parse("gasté 18mil en almuerzo").unwrap().unwrap();
        assert_eq!(r.monto_centavos, 18_000);
        assert_eq!(r.concepto, "almuerzo");
    }

    #[test]
    fn parse_magnitud_sin_espacio_k() {
        let parser = RegexParser::new();
        let r = parser.parse("gasté 5k en café").unwrap().unwrap();
        assert_eq!(r.monto_centavos, 5_000);
        assert_eq!(r.concepto, "café");
    }

    #[test]
    fn parse_magnitud_sin_espacio_M() {
        let parser = RegexParser::new();
        let r = parser.parse("gasté 3M en casa").unwrap().unwrap();
        assert_eq!(r.monto_centavos, 3_000_000);
        assert_eq!(r.concepto, "casa");
    }
}