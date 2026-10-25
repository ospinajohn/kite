use kite::models::{Movement, MovementType, MovementOrigin, MovementError};
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
fn new_rechaza_monto_negativo() {
    let err = Movement::new(
        MovementType::Gasto,
        -100,
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
fn new_rechaza_concepto_largo() {
    let long_concept = "a".repeat(201);
    let err = Movement::new(
        MovementType::Gasto,
        1000,
        long_concept,
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
fn new_acepta_fecha_hoy() {
    let m = Movement::new(
        MovementType::Gasto,
        1000,
        "Test".into(),
        Local::now().date_naive(),
        MovementOrigin::Formulario,
    ).unwrap();
    assert_eq!(m.fecha, Local::now().date_naive());
}

#[test]
fn new_acepta_fecha_pasada() {
    let past = NaiveDate::from_ymd_opt(2020, 1, 1).unwrap();
    let m = Movement::new(
        MovementType::Gasto,
        1000,
        "Test".into(),
        past,
        MovementOrigin::Formulario,
    ).unwrap();
    assert_eq!(m.fecha, past);
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

    std::thread::sleep(std::time::Duration::from_millis(10));

    m = m.update(19_000, "Almuerzo corregido".into(), m.fecha).unwrap();
    assert_eq!(m.monto_centavos, 19_000);
    assert_eq!(m.concepto, "Almuerzo corregido");
    assert!(m.actualizado_en > old_updated);
}

#[test]
fn update_rechaza_monto_cero() {
    let m = Movement::new(
        MovementType::Gasto,
        18_000,
        "Test".into(),
        Local::now().date_naive(),
        MovementOrigin::Formulario,
    ).unwrap();
    let err = m.update(0, "Test".into(), m.fecha).unwrap_err();
    assert!(matches!(err, MovementError::InvalidAmount(_)));
}

#[test]
fn update_rechaza_concepto_vacio() {
    let m = Movement::new(
        MovementType::Gasto,
        18_000,
        "Test".into(),
        Local::now().date_naive(),
        MovementOrigin::Formulario,
    ).unwrap();
    let err = m.update(1000, "   ".into(), m.fecha).unwrap_err();
    assert!(matches!(err, MovementError::InvalidConcept(_)));
}

#[test]
fn update_rechaza_fecha_futura() {
    let m = Movement::new(
        MovementType::Gasto,
        18_000,
        "Test".into(),
        Local::now().date_naive(),
        MovementOrigin::Formulario,
    ).unwrap();
    let tomorrow = Local::now().date_naive() + chrono::Days::new(1);
    let err = m.update(1000, "Test".into(), tomorrow).unwrap_err();
    assert!(matches!(err, MovementError::InvalidDate(_)));
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
    assert!(m2.sort_key() > m1.sort_key());
}

#[test]
fn format_monto_gasto() {
    let m = Movement::new(
        MovementType::Gasto,
        18_000,
        "Test".into(),
        Local::now().date_naive(),
        MovementOrigin::Formulario,
    ).unwrap();
    assert_eq!(m.format_monto(), "− $18.000");
}

#[test]
fn format_monto_ingreso() {
    let m = Movement::new(
        MovementType::Ingreso,
        1_500_000,
        "Test".into(),
        Local::now().date_naive(),
        MovementOrigin::Formulario,
    ).unwrap();
    assert_eq!(m.format_monto(), "+ $1.500.000");
}

#[test]
fn format_monto_sin_miles() {
    let m = Movement::new(
        MovementType::Gasto,
        500,
        "Test".into(),
        Local::now().date_naive(),
        MovementOrigin::Formulario,
    ).unwrap();
    assert_eq!(m.format_monto(), "− $500");
}