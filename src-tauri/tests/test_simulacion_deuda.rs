use chrono::NaiveDate;
use kite::models::{Frecuencia, Quincena};
use kite::simulacion_deuda::{
    generar_vencimientos_simulados, validar_deuda_simulada, DeudaSimulada, DuracionSimulacion,
    SimulacionError, MAX_QUINCENAS_PROYECCION,
};

fn d(y: i32, m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, day).unwrap()
}

fn deuda_base() -> DeudaSimulada {
    DeudaSimulada {
        concepto: Some("Préstamo".into()),
        monto_cuota_centavos: 200_000,
        frecuencia: Frecuencia::Mensual(10),
        primer_vencimiento: d(2026, 11, 10),
        duracion: DuracionSimulacion::Cuotas(3),
        quincenas_proyeccion: None,
    }
}

#[test]
fn valida_monto_positivo() {
    let mut deuda = deuda_base();
    deuda.monto_cuota_centavos = 0;
    assert_eq!(
        validar_deuda_simulada(&deuda),
        Err(SimulacionError::MontoInvalido)
    );
    deuda.monto_cuota_centavos = -1;
    assert_eq!(
        validar_deuda_simulada(&deuda),
        Err(SimulacionError::MontoInvalido)
    );
}

#[test]
fn valida_cuotas_minimo_uno() {
    let mut deuda = deuda_base();
    deuda.duracion = DuracionSimulacion::Cuotas(0);
    assert_eq!(
        validar_deuda_simulada(&deuda),
        Err(SimulacionError::CuotasInvalidas)
    );
}

#[test]
fn valida_fecha_fin_no_anterior_al_inicio() {
    let mut deuda = deuda_base();
    deuda.duracion = DuracionSimulacion::FechaFin(d(2026, 11, 9));
    assert_eq!(
        validar_deuda_simulada(&deuda),
        Err(SimulacionError::FechaFinInvalida)
    );
}

#[test]
fn valida_indefinida_requiere_quincenas() {
    let mut deuda = deuda_base();
    deuda.duracion = DuracionSimulacion::Indefinida;
    deuda.quincenas_proyeccion = None;
    assert_eq!(
        validar_deuda_simulada(&deuda),
        Err(SimulacionError::QuincenasProyeccionRequeridas)
    );
    deuda.quincenas_proyeccion = Some(0);
    assert_eq!(
        validar_deuda_simulada(&deuda),
        Err(SimulacionError::QuincenasProyeccionInvalidas)
    );
    deuda.quincenas_proyeccion = Some(MAX_QUINCENAS_PROYECCION + 1);
    assert_eq!(
        validar_deuda_simulada(&deuda),
        Err(SimulacionError::QuincenasProyeccionExcedidas)
    );
}

#[test]
fn valida_primer_vencimiento_consistente_con_frecuencia_mensual() {
    let mut deuda = deuda_base();
    deuda.primer_vencimiento = d(2026, 11, 11);
    assert_eq!(
        validar_deuda_simulada(&deuda),
        Err(SimulacionError::PrimerVencimientoInconsistente)
    );
}

#[test]
fn valida_mensual_dia_31_se_ajusta_a_fin_de_mes() {
    let mut deuda = deuda_base();
    deuda.frecuencia = Frecuencia::Mensual(31);
    // Noviembre tiene 30 días: el vencimiento consistente es el 30 (clamp).
    deuda.primer_vencimiento = d(2026, 11, 30);
    assert!(validar_deuda_simulada(&deuda).is_ok());
    // El 29 no coincide con el día 31 clampeado a fin de mes.
    deuda.primer_vencimiento = d(2026, 11, 29);
    assert_eq!(
        validar_deuda_simulada(&deuda),
        Err(SimulacionError::PrimerVencimientoInconsistente)
    );
}

#[test]
fn valida_quincenal_primera_y_segunda() {
    let mut deuda = deuda_base();
    deuda.frecuencia = Frecuencia::Quincenal(Quincena::Primera);
    deuda.primer_vencimiento = d(2026, 11, 15);
    assert!(validar_deuda_simulada(&deuda).is_ok());
    deuda.primer_vencimiento = d(2026, 11, 14);
    assert_eq!(
        validar_deuda_simulada(&deuda),
        Err(SimulacionError::PrimerVencimientoInconsistente)
    );

    deuda.frecuencia = Frecuencia::Quincenal(Quincena::Segunda);
    deuda.primer_vencimiento = d(2026, 11, 30);
    assert!(validar_deuda_simulada(&deuda).is_ok());
    deuda.primer_vencimiento = d(2026, 2, 28);
    assert!(validar_deuda_simulada(&deuda).is_ok());
}

#[test]
fn genera_cuotas_mensuales_desde_primer_vencimiento() {
    let deuda = deuda_base();
    let cal = generar_vencimientos_simulados(&deuda, d(2026, 11, 1), 15).unwrap();
    assert_eq!(cal.vencimientos.len(), 3);
    assert_eq!(cal.vencimientos[0].fecha, d(2026, 11, 10));
    assert_eq!(cal.vencimientos[1].fecha, d(2026, 12, 10));
    assert_eq!(cal.vencimientos[2].fecha, d(2027, 1, 10));
    assert_eq!(cal.vencimientos[0].monto_centavos, 200_000);
    assert_eq!(cal.vencimientos[0].numero_cuota, 1);
    assert_eq!(cal.vencimientos[2].numero_cuota, 3);
    assert!(!cal.inicio_ajustado_a_quincena_actual);
}

#[test]
fn mensual_dia_31_clamp_en_meses_cortos() {
    let deuda = DeudaSimulada {
        concepto: None,
        monto_cuota_centavos: 100_000,
        frecuencia: Frecuencia::Mensual(31),
        primer_vencimiento: d(2026, 1, 31),
        duracion: DuracionSimulacion::Cuotas(4),
        quincenas_proyeccion: None,
    };
    let cal = generar_vencimientos_simulados(&deuda, d(2026, 1, 1), 15).unwrap();
    assert_eq!(cal.vencimientos[0].fecha, d(2026, 1, 31));
    assert_eq!(cal.vencimientos[1].fecha, d(2026, 2, 28));
    assert_eq!(cal.vencimientos[2].fecha, d(2026, 3, 31));
    assert_eq!(cal.vencimientos[3].fecha, d(2026, 4, 30));
}

#[test]
fn quincenal_segunda_es_fin_de_mes() {
    let deuda = DeudaSimulada {
        concepto: None,
        monto_cuota_centavos: 50_000,
        frecuencia: Frecuencia::Quincenal(Quincena::Segunda),
        primer_vencimiento: d(2026, 11, 30),
        duracion: DuracionSimulacion::Cuotas(3),
        quincenas_proyeccion: None,
    };
    let cal = generar_vencimientos_simulados(&deuda, d(2026, 11, 1), 15).unwrap();
    assert_eq!(cal.vencimientos[0].fecha, d(2026, 11, 30));
    assert_eq!(cal.vencimientos[1].fecha, d(2026, 12, 31));
    assert_eq!(cal.vencimientos[2].fecha, d(2027, 1, 31));
}

#[test]
fn duracion_fecha_fin_incluye_vencimiento_en_la_fecha() {
    let deuda = DeudaSimulada {
        concepto: None,
        monto_cuota_centavos: 100_000,
        frecuencia: Frecuencia::Mensual(10),
        primer_vencimiento: d(2026, 11, 10),
        duracion: DuracionSimulacion::FechaFin(d(2027, 1, 10)),
        quincenas_proyeccion: None,
    };
    let cal = generar_vencimientos_simulados(&deuda, d(2026, 11, 1), 15).unwrap();
    assert_eq!(cal.vencimientos.len(), 3);
    assert_eq!(cal.vencimientos[2].fecha, d(2027, 1, 10));
}

#[test]
fn inicio_pasado_se_proyecta_desde_quincena_actual() {
    let deuda = DeudaSimulada {
        concepto: None,
        monto_cuota_centavos: 100_000,
        frecuencia: Frecuencia::Mensual(10),
        primer_vencimiento: d(2026, 9, 10),
        duracion: DuracionSimulacion::Cuotas(5),
        quincenas_proyeccion: None,
    };
    let cal = generar_vencimientos_simulados(&deuda, d(2026, 11, 20), 15).unwrap();
    assert!(cal.inicio_ajustado_a_quincena_actual);
    assert!(cal.vencimientos.iter().all(|v| v.fecha >= d(2026, 11, 16)));
    assert_eq!(cal.vencimientos[0].fecha, d(2026, 12, 10));
    assert_eq!(cal.vencimientos[1].fecha, d(2027, 1, 10));
    assert_eq!(cal.vencimientos.len(), 2);
}

#[test]
fn indefinida_respeta_quincenas_proyectadas_por_usuario() {
    let deuda = DeudaSimulada {
        concepto: None,
        monto_cuota_centavos: 100_000,
        frecuencia: Frecuencia::Mensual(10),
        primer_vencimiento: d(2026, 11, 10),
        duracion: DuracionSimulacion::Indefinida,
        quincenas_proyeccion: Some(2),
    };
    let cal = generar_vencimientos_simulados(&deuda, d(2026, 11, 5), 15).unwrap();
    assert!(cal.vencimientos.iter().all(|v| v.fecha <= d(2026, 11, 30)));
    assert_eq!(cal.vencimientos.len(), 1);
    assert_eq!(cal.vencimientos[0].fecha, d(2026, 11, 10));
}

#[test]
fn concepto_muy_largo_es_invalido() {
    let mut deuda = deuda_base();
    deuda.concepto = Some("a".repeat(201));
    assert_eq!(
        validar_deuda_simulada(&deuda),
        Err(SimulacionError::ConceptoInvalido)
    );
}
