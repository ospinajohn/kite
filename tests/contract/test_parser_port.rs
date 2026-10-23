use pocketflow::models::{MovementType, MovementOrigin};
use pocketflow::parser::port::{ParserPort, ParsedProposal};
use chrono::{Local, NaiveDate};

pub fn test_parser_port<P: ParserPort>(parser: &P) {
    let today = Local::now().date_naive();

    let r = parser.parse("gasté 18 mil en almuerzo").expect("parse ok");
    assert!(r.is_some());
    let p = r.unwrap();
    assert_eq!(p.tipo, MovementType::Gasto);
    assert_eq!(p.monto_centavos, 18_000);
    assert_eq!(p.concepto, "almuerzo");
    assert_eq!(p.fecha, today);
    assert_eq!(p.confidence, 1.0);

    let r = parser.parse("gasté 12.000 en transporte").expect("parse ok");
    assert!(r.is_some());
    let p = r.unwrap();
    assert_eq!(p.monto_centavos, 12_000);
    assert_eq!(p.concepto, "transporte");

    let r = parser.parse("recibí 1.500.000 de pago quincena").expect("parse ok");
    assert!(r.is_some());
    let p = r.unwrap();
    assert_eq!(p.tipo, MovementType::Ingreso);
    assert_eq!(p.monto_centavos, 1_500_000);
    assert_eq!(p.concepto, "pago quincena");

    let r = parser.parse("le presté 50 mil a carlos").expect("parse ok");
    assert!(r.is_some());
    let p = r.unwrap();
    assert_eq!(p.tipo, MovementType::Gasto);
    assert_eq!(p.monto_centavos, 50_000);
    assert_eq!(p.concepto, "carlos");

    let r = parser.parse("compré algo").expect("parse ok");
    assert!(r.is_none());

    let r = parser.parse("hola mundo").expect("parse ok");
    assert!(r.is_none());

    let r = parser.parse("gasté 100 en café").expect("parse ok");
    assert!(r.is_some());
    assert_eq!(r.unwrap().confidence, 1.0);
}

pub fn test_parser_edge_cases<P: ParserPort>(parser: &P) {
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

    assert!(parser.parse("GASTÉ 100 EN CAFÉ").unwrap().is_some());
    assert!(parser.parse("Gasté 100 en café").unwrap().is_some());

    assert!(parser.parse("  gasté 100 en café  ").unwrap().is_some());
}

#[cfg(test)]
mod tests {
    use super::*;
    use pocketflow::parser::regex_impl::RegexParser;

    #[test]
    fn regex_parser_passes_contract() {
        let parser = RegexParser::new();
        test_parser_port(&parser);
        test_parser_edge_cases(&parser);
    }
}