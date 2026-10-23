use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeriodSummary {
    pub ingresos_centavos: i64,
    pub gastos_centavos: i64,
    pub diferencia_centavos: i64,
    pub desde: Option<NaiveDate>,
    pub hasta: Option<NaiveDate>,
}

impl PeriodSummary {
    pub fn new(
        ingresos_centavos: i64,
        gastos_centavos: i64,
        desde: Option<NaiveDate>,
        hasta: Option<NaiveDate>,
    ) -> Self {
        Self {
            ingresos_centavos,
            gastos_centavos,
            diferencia_centavos: ingresos_centavos - gastos_centavos,
            desde,
            hasta,
        }
    }

    pub fn empty() -> Self {
        Self::new(0, 0, None, None)
    }

    pub fn format_cop(centavos: i64) -> String {
        if centavos == 0 {
            return "+ $0".to_string();
        }
        let sign = if centavos >= 0 { "+" } else { "−" };
        let abs = centavos.abs();
        let mut s = abs.to_string();
        // Agregar separadores de miles
        let mut result = String::new();
        let len = s.len();
        for (i, ch) in s.chars().rev().enumerate() {
            if i > 0 && i % 3 == 0 {
                result.push('.');
            }
            result.push(ch);
        }
        let formatted: String = result.chars().rev().collect();
        format!("{sign} ${formatted}")
    }

    pub fn ingresos_fmt(&self) -> String {
        Self::format_cop(self.ingresos_centavos)
    }

    pub fn gastos_fmt(&self) -> String {
        Self::format_cop(self.gastos_centavos)
    }

    pub fn diferencia_fmt(&self) -> String {
        Self::format_cop(self.diferencia_centavos)
    }

    pub fn label(&self) -> &'static str {
        "Balance registrado"
    }

    pub fn range_description(&self) -> String {
        match (self.desde, self.hasta) {
            (Some(d), Some(h)) if d == h => format!("{}", d.format("%-d %b %Y")).to_lowercase(),
            (Some(d), Some(h)) => format!("{} – {}", d.format("%-d %b"), h.format("%-d %b %Y")).to_lowercase(),
            (Some(d), None) => format!("Desde {}", d.format("%-d %b %Y")).to_lowercase(),
            (None, Some(h)) => format!("Hasta {}", h.format("%-d %b %Y")).to_lowercase(),
            (None, None) => "Todo el historial (acumulado)".into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn calcula_diferencia_correcta() {
        let s = PeriodSummary::new(1_500_000, 30_000, None, None);
        assert_eq!(s.diferencia_centavos, 1_470_000);
        assert_eq!(s.diferencia_fmt(), "+ $1.470.000");
    }

    #[test]
    fn formato_cop_varios_casos() {
        assert_eq!(PeriodSummary::format_cop(0), "+ $0");
        assert_eq!(PeriodSummary::format_cop(500), "+ $500");
        assert_eq!(PeriodSummary::format_cop(1_500), "+ $1.500");
        assert_eq!(PeriodSummary::format_cop(1_500_000), "+ $1.500.000");
        assert_eq!(PeriodSummary::format_cop(-18_000), "− $18.000");
        assert_eq!(PeriodSummary::format_cop(-1_500_000), "− $1.500.000");
    }

    #[test]
    fn range_description() {
        let d = NaiveDate::from_ymd_opt(2026, 10, 1).unwrap();
        let h = NaiveDate::from_ymd_opt(2026, 10, 15).unwrap();
        assert_eq!(
            PeriodSummary::new(0, 0, Some(d), Some(h)).range_description(),
            "1 oct – 15 oct 2026"
        );
        assert_eq!(
            PeriodSummary::new(0, 0, Some(d), Some(d)).range_description(),
            "1 oct 2026"
        );
        assert_eq!(
            PeriodSummary::new(0, 0, None, None).range_description(),
            "Todo el historial (acumulado)"
        );
    }

    #[test]
    fn label_oficial() {
        assert_eq!(PeriodSummary::new(0,0,None,None).label(), "Balance registrado");
    }
}