pub mod service;

pub use service::{
    calcular_escenario, generar_vencimientos_simulados, simular_impacto, validar_deuda_simulada,
    CalendarioSimulado, DeudaSimulada, DuracionSimulacion, EscenarioQuincena, ImpactoMeta,
    ResultadoSimulacion, SimulacionError, VencimientoSimulado, MAX_QUINCENAS_PROYECCION,
};
