use pocketflow::models::summary::PeriodSummary;
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

#[test]
fn ingresos_fmt() {
    let s = PeriodSummary::new(1_500_000, 0, None, None);
    assert_eq!(s.ingresos_fmt(), "+ $1.500.000");
}

#[test]
fn gastos_fmt() {
    let s = PeriodSummary::new(0, 18_000, None, None);
    assert_eq!(s.gastos_fmt(), "− $18.000");
}

#[test]
fn empty_summary() {
    let s = PeriodSummary::empty();
    assert_eq!(s.ingresos_centavos, 0);
    assert_eq!(s.gastos_centavos, 0);
    assert_eq!(s.diferencia_centavos, 0);
    assert_eq!(s.desde, None);
    assert_eq!(s.hasta, None);
}