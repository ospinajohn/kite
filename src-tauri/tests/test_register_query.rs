use kite::models::{Movement, MovementType, MovementOrigin};
use kite::parser::regex_impl::RegexParser;
use kite::parser::port::ParserPort;
use kite::storage::port::StoragePort;
use chrono::{Local, NaiveDate};
use tempfile::tempdir;

fn create_test_storage() -> kite::storage::sqlite_impl::SqliteStorage {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("test.db");
    kite::storage::sqlite_impl::SqliteStorage::new(&db_path).unwrap()
}

#[test]
fn test_register_expense_formulario() {
    let storage = create_test_storage();
    storage.migrate().unwrap();

    let movimiento = Movement::new(
        MovementType::Gasto,
        18_000,
        "Almuerzo".into(),
        Local::now().date_naive(),
        MovementOrigin::Formulario,
    ).unwrap();

    storage.save(movimiento.clone()).unwrap();

    let all = storage.query(None, None).unwrap();
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].monto_centavos, 18_000);
    assert_eq!(all[0].concepto, "Almuerzo");
    assert_eq!(all[0].tipo, MovementType::Gasto);
    assert_eq!(all[0].origen, MovementOrigin::Formulario);
}

#[test]
fn test_register_income_formulario() {
    let storage = create_test_storage();
    storage.migrate().unwrap();

    let movimiento = Movement::new(
        MovementType::Ingreso,
        1_500_000,
        "Pago de quincena".into(),
        NaiveDate::from_ymd_opt(2026, 10, 15).unwrap(),
        MovementOrigin::Formulario,
    ).unwrap();

    storage.save(movimiento.clone()).unwrap();

    let all = storage.query(None, None).unwrap();
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].monto_centavos, 1_500_000);
    assert_eq!(all[0].concepto, "Pago de quincena");
    assert_eq!(all[0].tipo, MovementType::Ingreso);
    assert_eq!(all[0].origen, MovementOrigin::Formulario);
}

#[test]
fn test_register_expense_from_message() {
    let storage = create_test_storage();
    storage.migrate().unwrap();
    let parser = RegexParser::new();

    // Flujo: parse message -> proposal -> confirm -> save
    let proposal = parser.parse("gasté 18 mil en almuerzo").unwrap().unwrap();
    assert_eq!(proposal.tipo, MovementType::Gasto);
    assert_eq!(proposal.monto_centavos, 18_000);
    assert_eq!(proposal.concepto, "almuerzo");
    assert_eq!(proposal.origen, MovementOrigin::Mensaje);

    let movimiento = Movement::new(
        proposal.tipo,
        proposal.monto_centavos,
        proposal.concepto.clone(),
        proposal.fecha,
        proposal.origen,
    ).unwrap();

    storage.save(movimiento.clone()).unwrap();

    let all = storage.query(None, None).unwrap();
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].monto_centavos, 18_000);
    assert_eq!(all[0].concepto, "almuerzo");
    assert_eq!(all[0].origen, MovementOrigin::Mensaje);
}

#[test]
fn test_register_income_from_message() {
    let storage = create_test_storage();
    storage.migrate().unwrap();
    let parser = RegexParser::new();

    let proposal = parser.parse("recibí 1.500.000 de pago quincena").unwrap().unwrap();
    assert_eq!(proposal.tipo, MovementType::Ingreso);
    assert_eq!(proposal.monto_centavos, 1_500_000);
    assert_eq!(proposal.concepto, "pago quincena");
    assert_eq!(proposal.origen, MovementOrigin::Mensaje);

    let movimiento = Movement::new(
        proposal.tipo,
        proposal.monto_centavos,
        proposal.concepto.clone(),
        proposal.fecha,
        proposal.origen,
    ).unwrap();

    storage.save(movimiento.clone()).unwrap();

    let all = storage.query(None, None).unwrap();
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].monto_centavos, 1_500_000);
    assert_eq!(all[0].concepto, "pago quincena");
    assert_eq!(all[0].tipo, MovementType::Ingreso);
    assert_eq!(all[0].origen, MovementOrigin::Mensaje);
}

#[test]
fn test_register_multiple_from_message_and_formulario() {
    let storage = create_test_storage();
    storage.migrate().unwrap();
    let parser = RegexParser::new();

    // Gasto por mensaje
    let p1 = parser.parse("gasté 18 mil en almuerzo").unwrap().unwrap();
    let m1 = Movement::new(p1.tipo, p1.monto_centavos, p1.concepto.clone(), p1.fecha, p1.origen).unwrap();
    storage.save(m1).unwrap();

    // Ingreso por formulario
    let m2 = Movement::new(
        MovementType::Ingreso,
        1_500_000,
        "Pago de quincena".into(),
        Local::now().date_naive(),
        MovementOrigin::Formulario,
    ).unwrap();
    storage.save(m2).unwrap();

    // Gasto por mensaje con "ayer" (v1: se usa fecha de hoy)
    let p3 = parser.parse("gasté ayer 12.000 en transporte").unwrap().unwrap();
    let m3 = Movement::new(p3.tipo, p3.monto_centavos, p3.concepto.clone(), p3.fecha, p3.origen).unwrap();
    storage.save(m3).unwrap();

    // Préstamo por mensaje
    let p4 = parser.parse("le presté 50 mil a carlos").unwrap().unwrap();
    let m4 = Movement::new(p4.tipo, p4.monto_centavos, p4.concepto.clone(), p4.fecha, p4.origen).unwrap();
    storage.save(m4).unwrap();

    let all = storage.query(None, None).unwrap();
    assert_eq!(all.len(), 4);

    // Verificar tipos y orígenes
    let ingresos: Vec<_> = all.iter().filter(|m| m.tipo == MovementType::Ingreso).collect();
    assert_eq!(ingresos.len(), 1);
    assert_eq!(ingresos[0].origen, MovementOrigin::Formulario);

    let gastos: Vec<_> = all.iter().filter(|m| m.tipo == MovementType::Gasto).collect();
    assert_eq!(gastos.len(), 3);
    assert!(gastos.iter().all(|m| m.origen == MovementOrigin::Mensaje));
}

#[test]
fn test_query_with_date_range() {
    let storage = create_test_storage();
    storage.migrate().unwrap();

    let hoy = Local::now().date_naive();
    let ayer = hoy.pred_opt().unwrap();
    let anteayer = ayer.pred_opt().unwrap();

    let m1 = Movement::new(MovementType::Gasto, 10_000, "Hoy".into(), hoy, MovementOrigin::Formulario).unwrap();
    let m2 = Movement::new(MovementType::Gasto, 20_000, "Ayer".into(), ayer, MovementOrigin::Formulario).unwrap();
    let m3 = Movement::new(MovementType::Gasto, 30_000, "Anteayer".into(), anteayer, MovementOrigin::Formulario).unwrap();

    storage.save(m1).unwrap();
    storage.save(m2).unwrap();
    storage.save(m3).unwrap();

    // Query solo hoy
    let hoy_only = storage.query(Some(hoy), Some(hoy)).unwrap();
    assert_eq!(hoy_only.len(), 1);
    assert_eq!(hoy_only[0].concepto, "Hoy");

    // Query hoy y ayer
    let dos_dias = storage.query(Some(ayer), Some(hoy)).unwrap();
    assert_eq!(dos_dias.len(), 2);

    // Query todo
    let todo = storage.query(None, None).unwrap();
    assert_eq!(todo.len(), 3);
}