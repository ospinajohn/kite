//! Cálculo puro y ensamblado del simulador de impacto de nuevas deudas (feature 006).
//! Efímero: sin persistencia ni efectos sobre datos reales.

use crate::compromisos::port::CompromisosPort;
use crate::metas::domain::{EstadoMeta, MetaUI};
use crate::metas::port::MetasPort;
use crate::metas::service::listar_metas;
use crate::models::{Compromiso, Frecuencia, Movement, Quincena};
use crate::planificacion::{
    calcular_plan, quincena_de_fecha, quincena_siguiente, PendienteCompromiso, PlanQuincena,
    RangoQuincena,
};
use crate::storage::port::StoragePort;
use chrono::{Datelike, NaiveDate};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;
use uuid::Uuid;

/// Tope razonable de quincenas a proyectar para deudas indefinidas (FR-D09).
pub const MAX_QUINCENAS_PROYECCION: u32 = 120;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DuracionSimulacion {
    Cuotas(u32),
    FechaFin(NaiveDate),
    Indefinida,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeudaSimulada {
    pub concepto: Option<String>,
    pub monto_cuota_centavos: i64,
    pub frecuencia: Frecuencia,
    /// Fecha/inicio del primer vencimiento.
    pub primer_vencimiento: NaiveDate,
    pub duracion: DuracionSimulacion,
    /// Requerido cuando la duración es indefinida: cantidad de quincenas a proyectar.
    pub quincenas_proyeccion: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VencimientoSimulado {
    pub fecha: NaiveDate,
    pub monto_centavos: i64,
    /// Índice de cuota (1-based) dentro del calendario simulado.
    pub numero_cuota: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CalendarioSimulado {
    pub vencimientos: Vec<VencimientoSimulado>,
    /// C6: hubo vencimientos previos a la quincena actual que se omitieron.
    pub inicio_ajustado_a_quincena_actual: bool,
    /// Horizonte efectivo en quincenas proyectadas (informativo).
    pub quincenas_horizonte: u32,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum SimulacionError {
    #[error("El monto de la cuota debe ser mayor que cero")]
    MontoInvalido,
    #[error("El concepto no puede exceder 200 caracteres")]
    ConceptoInvalido,
    #[error("La frecuencia mensual requiere un día entre 1 y 31")]
    FrecuenciaInvalida,
    #[error("El número de cuotas debe ser al menos 1")]
    CuotasInvalidas,
    #[error("La fecha fin no puede ser anterior al primer vencimiento")]
    FechaFinInvalida,
    #[error("Debés indicar cuántas quincenas proyectar para una deuda indefinida")]
    QuincenasProyeccionRequeridas,
    #[error("La cantidad de quincenas a proyectar debe ser al menos 1")]
    QuincenasProyeccionInvalidas,
    #[error(
        "La cantidad de quincenas a proyectar no puede exceder {MAX_QUINCENAS_PROYECCION}"
    )]
    QuincenasProyeccionExcedidas,
    #[error("El primer vencimiento no coincide con la frecuencia indicada")]
    PrimerVencimientoInconsistente,
}

fn ultimo_dia_mes(anio: i32, mes: u32) -> u32 {
    let primero_proximo = if mes == 12 {
        NaiveDate::from_ymd_opt(anio + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(anio, mes + 1, 1)
    };
    primero_proximo
        .expect("mes siguiente válido")
        .pred_opt()
        .expect("fecha válida")
        .day()
}

fn vencimiento_en_mes(frecuencia: &Frecuencia, anio: i32, mes: u32) -> NaiveDate {
    let ultimo = ultimo_dia_mes(anio, mes);
    match frecuencia {
        Frecuencia::Mensual(dia) => {
            let dia = (*dia as u32).min(ultimo);
            NaiveDate::from_ymd_opt(anio, mes, dia).expect("fecha válida tras clamp")
        }
        Frecuencia::Quincenal(Quincena::Primera) => {
            NaiveDate::from_ymd_opt(anio, mes, 15).expect("día 15 siempre válido")
        }
        Frecuencia::Quincenal(Quincena::Segunda) => {
            NaiveDate::from_ymd_opt(anio, mes, ultimo).expect("último día del mes")
        }
    }
}

fn siguiente_mes(anio: i32, mes: u32) -> (i32, u32) {
    if mes == 12 {
        (anio + 1, 1)
    } else {
        (anio, mes + 1)
    }
}

/// Valida la deuda simulada según FR-D01, C4, C5, C7.
pub fn validar_deuda_simulada(deuda: &DeudaSimulada) -> Result<(), SimulacionError> {
    if deuda.monto_cuota_centavos <= 0 {
        return Err(SimulacionError::MontoInvalido);
    }
    if let Some(c) = &deuda.concepto {
        if c.trim().len() > 200 {
            return Err(SimulacionError::ConceptoInvalido);
        }
    }
    match deuda.frecuencia {
        Frecuencia::Mensual(dia) => {
            if dia == 0 || dia > 31 {
                return Err(SimulacionError::FrecuenciaInvalida);
            }
            let esperado = vencimiento_en_mes(
                &deuda.frecuencia,
                deuda.primer_vencimiento.year(),
                deuda.primer_vencimiento.month(),
            );
            if deuda.primer_vencimiento != esperado {
                return Err(SimulacionError::PrimerVencimientoInconsistente);
            }
        }
        Frecuencia::Quincenal(_) => {
            let esperado = vencimiento_en_mes(
                &deuda.frecuencia,
                deuda.primer_vencimiento.year(),
                deuda.primer_vencimiento.month(),
            );
            if deuda.primer_vencimiento != esperado {
                return Err(SimulacionError::PrimerVencimientoInconsistente);
            }
        }
    }
    match deuda.duracion {
        DuracionSimulacion::Cuotas(n) => {
            if n == 0 {
                return Err(SimulacionError::CuotasInvalidas);
            }
        }
        DuracionSimulacion::FechaFin(fecha_fin) => {
            if fecha_fin < deuda.primer_vencimiento {
                return Err(SimulacionError::FechaFinInvalida);
            }
        }
        DuracionSimulacion::Indefinida => match deuda.quincenas_proyeccion {
            None => return Err(SimulacionError::QuincenasProyeccionRequeridas),
            Some(0) => return Err(SimulacionError::QuincenasProyeccionInvalidas),
            Some(n) if n > MAX_QUINCENAS_PROYECCION => {
                return Err(SimulacionError::QuincenasProyeccionExcedidas)
            }
            Some(_) => {}
        },
    }
    Ok(())
}

/// Genera el calendario de vencimientos simulados (FR-D01, C6, C7).
/// `hoy` y `dia_corte` determinan la quincena actual para aplicar C6.
pub fn generar_vencimientos_simulados(
    deuda: &DeudaSimulada,
    hoy: NaiveDate,
    dia_corte: u32,
) -> Result<CalendarioSimulado, SimulacionError> {
    assert!(
        (1..=28).contains(&dia_corte),
        "El día de corte debe estar entre 1 y 28"
    );
    validar_deuda_simulada(deuda)?;

    let quincena_actual = quincena_de_fecha(hoy, dia_corte);
    let inicio_quincena_actual = quincena_actual.desde;

    let (max_cuotas, fecha_limite, quincenas_objetivo): (u32, Option<NaiveDate>, Option<u32>) =
        match deuda.duracion {
            DuracionSimulacion::Cuotas(n) => (n, None, None),
            DuracionSimulacion::FechaFin(f) => (u32::MAX, Some(f), None),
            DuracionSimulacion::Indefinida => (u32::MAX, None, deuda.quincenas_proyeccion),
        };

    let mut vencimientos = Vec::new();
    let mut inicio_ajustado = false;
    let mut quincenas_horizonte: u32 = 0;
    let mut ultima_quincena_contada: Option<NaiveDate> = None;

    let mut anio = deuda.primer_vencimiento.year();
    let mut mes = deuda.primer_vencimiento.month();
    let mut numero_cuota: u32 = 0;

    for _ in 0..2400 {
        let fecha = vencimiento_en_mes(&deuda.frecuencia, anio, mes);
        numero_cuota += 1;
        if numero_cuota > max_cuotas {
            break;
        }

        if let Some(limite) = fecha_limite {
            if fecha > limite {
                break;
            }
        }

        if fecha < deuda.primer_vencimiento {
            let (na, nm) = siguiente_mes(anio, mes);
            anio = na;
            mes = nm;
            continue;
        }

        if let Some(obj) = quincenas_objetivo {
            let q_desde = quincena_de_fecha(fecha.max(inicio_quincena_actual), dia_corte).desde;
            let nueva = match ultima_quincena_contada {
                None => {
                    if fecha >= inicio_quincena_actual {
                        contar_quincenas(inicio_quincena_actual, q_desde, dia_corte)
                    } else {
                        0
                    }
                }
                Some(prev) => {
                    if q_desde > prev {
                        contar_quincenas(prev, q_desde, dia_corte).saturating_sub(1)
                    } else {
                        0
                    }
                }
            };
            let potencial_total = quincenas_horizonte.saturating_add(nueva);
            if fecha >= inicio_quincena_actual && potencial_total > obj {
                break;
            }
        }

        if fecha < inicio_quincena_actual {
            inicio_ajustado = true;
        } else {
            if quincenas_objetivo.is_some() {
                let q_desde = quincena_de_fecha(fecha, dia_corte).desde;
                match ultima_quincena_contada {
                    None => {
                        quincenas_horizonte =
                            contar_quincenas(inicio_quincena_actual, q_desde, dia_corte);
                        ultima_quincena_contada = Some(q_desde);
                    }
                    Some(prev) if q_desde > prev => {
                        quincenas_horizonte = quincenas_horizonte.saturating_add(
                            contar_quincenas(prev, q_desde, dia_corte).saturating_sub(1),
                        );
                        ultima_quincena_contada = Some(q_desde);
                    }
                    Some(_) => {}
                }
            }

            vencimientos.push(VencimientoSimulado {
                fecha,
                monto_centavos: deuda.monto_cuota_centavos,
                numero_cuota,
            });
        }

        let (na, nm) = siguiente_mes(anio, mes);
        anio = na;
        mes = nm;
    }

    if quincenas_objetivo.is_none() {
        quincenas_horizonte = contar_quincenas_de_vencimientos(&vencimientos, dia_corte);
    }

    Ok(CalendarioSimulado {
        vencimientos,
        inicio_ajustado_a_quincena_actual: inicio_ajustado,
        quincenas_horizonte,
    })
}

/// Cuenta cuántas quincenas hay desde `desde` (inicio de quincena) hasta `hasta`
/// (inicio de quincena), inclusive en ambos extremos.
fn contar_quincenas(desde: NaiveDate, hasta: NaiveDate, dia_corte: u32) -> u32 {
    if hasta < desde {
        return 0;
    }
    let mut actual = quincena_de_fecha(desde, dia_corte).desde;
    let hasta_norm = quincena_de_fecha(hasta, dia_corte).desde;
    let mut n: u32 = 0;
    for _ in 0..5000 {
        n += 1;
        if actual >= hasta_norm {
            break;
        }
        let q = quincena_de_fecha(actual, dia_corte);
        let siguiente = q.hasta.succ_opt().expect("fecha válida");
        actual = quincena_de_fecha(siguiente, dia_corte).desde;
        if n > 4000 {
            break;
        }
    }
    n
}

fn contar_quincenas_de_vencimientos(vencimientos: &[VencimientoSimulado], dia_corte: u32) -> u32 {
    if vencimientos.is_empty() {
        return 0;
    }
    let primera = quincena_de_fecha(vencimientos.first().unwrap().fecha, dia_corte).desde;
    let ultima = quincena_de_fecha(vencimientos.last().unwrap().fecha, dia_corte).desde;
    contar_quincenas(primera, ultima, dia_corte)
}

// --- Escenario por quincena (T4, FR-D03, FR-D04) ---

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EscenarioQuincena {
    pub rango: RangoQuincena,
    /// Plan base calculado con el estado real vigente (003), incluyendo compromisos
    /// reales hasta el horizonte analizado.
    pub base: PlanQuincena,
    /// Vencimientos simulados que caen en esta quincena.
    pub vencimientos_simulados: Vec<VencimientoSimulado>,
    pub total_simulado_centavos: i64,
    pub libre_quincena_simulado_centavos: i64,
    pub libre_real_simulado_centavos: i64,
    /// FR-D04: la cuota simulada compite con compromisos pendientes del período o
    /// el libre simulado no alcanza para cubrir pendientes reales + cuota simulada.
    pub conflicto_compromisos: bool,
}

/// Impacto informativo de la simulación sobre una meta activa (T5, FR-D05, C8).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImpactoMeta {
    pub meta_id: Uuid,
    pub nombre: String,
    pub monto_objetivo_centavos: i64,
    pub acumulado_centavos: i64,
    pub faltante_centavos: i64,
    pub fecha_objetivo: Option<NaiveDate>,
    pub a_ritmo_base: Option<bool>,
    /// Mínimo libre quincenal simulado observado en las quincenas relevantes
    /// analizadas (referencia informativa de capacidad de aporte).
    pub libre_simulado_referencia_centavos: Option<i64>,
    /// Señal de ritmo afectado/en riesgo: Some cuando hay fecha objetivo y ritmo
    /// calculable; true si el libre simulado no permite sostener el ritmo necesario.
    pub ritmo_afectado: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResultadoSimulacion {
    pub calendario: CalendarioSimulado,
    pub quincenas: Vec<EscenarioQuincena>,
    pub metas: Vec<ImpactoMeta>,
}

/// Calcula el escenario por quincenas a partir de datos ya obtenidos (puro).
/// `movimientos_horizonte` debe incluir los movimientos relevantes del rango completo.
/// `compromisos_con_vencimientos` son los vencimientos pendientes reales por compromiso
/// ya resueltos contra sus períodos pagados, hasta el fin del horizonte.
pub fn calcular_escenario(
    calendario: CalendarioSimulado,
    movimientos_horizonte: &[Movement],
    compromisos_con_vencimientos: &[(Compromiso, Vec<crate::models::Vencimiento>)],
    metas: &[MetaUI],
    hoy: NaiveDate,
    dia_corte: u32,
) -> ResultadoSimulacion {
    if calendario.vencimientos.is_empty() {
        let quincenas = Vec::new();
        let metas_impacto = calcular_impacto_metas(metas, &quincenas, hoy, dia_corte);
        return ResultadoSimulacion {
            calendario,
            quincenas,
            metas: metas_impacto,
        };
    }

    let quincena_actual = quincena_de_fecha(hoy, dia_corte);
    let primera_fecha = calendario.vencimientos.first().unwrap().fecha;
    let ultima_fecha = calendario.vencimientos.last().unwrap().fecha;

    let inicio = quincena_de_fecha(
        if primera_fecha < quincena_actual.desde {
            quincena_actual.desde
        } else {
            primera_fecha
        },
        dia_corte,
    );
    let fin = quincena_de_fecha(ultima_fecha, dia_corte);

    let mut rangos = Vec::new();
    let mut actual = inicio;
    for _ in 0..500 {
        rangos.push(actual);
        if actual.desde == fin.desde && actual.hasta == fin.hasta {
            break;
        }
        actual = quincena_siguiente(actual);
        if actual.desde > fin.desde {
            rangos.push(fin);
            break;
        }
    }
    if rangos.is_empty() {
        rangos.push(fin);
    }

    let quincenas: Vec<EscenarioQuincena> = rangos
        .into_iter()
        .map(|rango| {
            let base =
                plan_base_quincena(rango, movimientos_horizonte, compromisos_con_vencimientos);
            let vencimientos_simulados: Vec<VencimientoSimulado> = calendario
                .vencimientos
                .iter()
                .filter(|v| rango.contiene(v.fecha))
                .cloned()
                .collect();
            let total_simulado_centavos: i64 =
                vencimientos_simulados.iter().map(|v| v.monto_centavos).sum();
            let libre_quincena_simulado_centavos =
                base.libre_quincena_centavos - total_simulado_centavos;
            let libre_real_simulado_centavos = base.libre_real_centavos - total_simulado_centavos;
            let conflicto_compromisos = if total_simulado_centavos > 0 {
                let total_pendientes_reales: i64 = base
                    .pendientes_periodo
                    .iter()
                    .map(|p| p.monto_centavos)
                    .sum();
                let disponible_periodo = base.ingresos_centavos
                    - base.gastos_centavos
                    - base.apartados_periodo_centavos;
                !base.pendientes_periodo.is_empty()
                    || libre_quincena_simulado_centavos < 0
                    || disponible_periodo < (total_pendientes_reales + total_simulado_centavos)
            } else {
                false
            };
            EscenarioQuincena {
                rango,
                base,
                vencimientos_simulados,
                total_simulado_centavos,
                libre_quincena_simulado_centavos,
                libre_real_simulado_centavos,
                conflicto_compromisos,
            }
        })
        .collect();

    let metas_impacto = calcular_impacto_metas(metas, &quincenas, hoy, dia_corte);

    ResultadoSimulacion {
        calendario,
        quincenas,
        metas: metas_impacto,
    }
}

fn plan_base_quincena(
    rango: RangoQuincena,
    movimientos_horizonte: &[Movement],
    compromisos_con_vencimientos: &[(Compromiso, Vec<crate::models::Vencimiento>)],
) -> PlanQuincena {
    let movimientos_rango: Vec<Movement> = movimientos_horizonte
        .iter()
        .filter(|m| rango.contiene(m.fecha))
        .cloned()
        .collect();

    let mut pendientes = Vec::new();
    let mut atrasados_por_compromiso: HashMap<Uuid, PendienteCompromiso> = HashMap::new();
    for (c, vencimientos) in compromisos_con_vencimientos {
        for v in vencimientos {
            let p = PendienteCompromiso {
                compromiso_id: c.id,
                concepto: c.concepto.clone(),
                monto_centavos: c.monto_centavos,
                fecha: v.fecha,
                periodo_key: v.periodo_key.clone(),
            };
            if rango.contiene(v.fecha) {
                pendientes.push(p);
            } else if v.fecha < rango.desde {
                match atrasados_por_compromiso.get(&c.id) {
                    Some(prev) if prev.fecha >= p.fecha => {}
                    _ => {
                        atrasados_por_compromiso.insert(c.id, p);
                    }
                }
            }
        }
    }
    let orden = |a: &PendienteCompromiso, b: &PendienteCompromiso| {
        a.fecha.cmp(&b.fecha).then(a.concepto.cmp(&b.concepto))
    };
    pendientes.sort_by(orden);
    let mut atrasados: Vec<PendienteCompromiso> = atrasados_por_compromiso.into_values().collect();
    atrasados.sort_by(orden);

    calcular_plan(rango, &movimientos_rango, &pendientes, &atrasados)
}

/// Ensambla el resultado completo desde estado persistido (FR-D08).
/// Calcula la base real vigente y aplica la deuda simulada sin mutar datos.
pub fn simular_impacto(
    storage: &dyn StoragePort,
    compromisos_port: &dyn CompromisosPort,
    metas_port: &dyn MetasPort,
    deuda: &DeudaSimulada,
    hoy: NaiveDate,
    dia_corte: u32,
) -> Result<ResultadoSimulacion, Box<dyn std::error::Error>> {
    if !(1..=28).contains(&dia_corte) {
        return Err("El día de corte debe estar entre 1 y 28".into());
    }
    let calendario = generar_vencimientos_simulados(deuda, hoy, dia_corte)?;

    if calendario.vencimientos.is_empty() {
        let metas = listar_metas(metas_port)?;
        let metas_impacto = calcular_impacto_metas(&metas, &[], hoy, dia_corte);
        return Ok(ResultadoSimulacion {
            calendario,
            quincenas: Vec::new(),
            metas: metas_impacto,
        });
    }

    let quincena_actual = quincena_de_fecha(hoy, dia_corte);
    let primera_fecha = calendario.vencimientos.first().unwrap().fecha;
    let ultima_fecha = calendario.vencimientos.last().unwrap().fecha;
    let inicio_horizonte = if primera_fecha < quincena_actual.desde {
        quincena_actual.desde
    } else {
        quincena_de_fecha(primera_fecha, dia_corte).desde
    };
    let fin_horizonte = quincena_de_fecha(ultima_fecha, dia_corte).hasta;

    let movimientos_horizonte = storage.query(Some(inicio_horizonte), Some(fin_horizonte))?;

    let mut compromisos_con_vencimientos = Vec::new();
    for c in compromisos_port.list()? {
        let pagados = compromisos_port.periodos_pagados(c.id)?;
        let vencimientos = c.vencimientos_hasta_fecha(fin_horizonte, &pagados);
        compromisos_con_vencimientos.push((c, vencimientos));
    }

    let metas = listar_metas(metas_port)?;

    Ok(calcular_escenario(
        calendario,
        &movimientos_horizonte,
        &compromisos_con_vencimientos,
        &metas,
        hoy,
        dia_corte,
    ))
}

fn calcular_impacto_metas(
    metas: &[MetaUI],
    quincenas: &[EscenarioQuincena],
    hoy: NaiveDate,
    dia_corte: u32,
) -> Vec<ImpactoMeta> {
    let quincena_actual = quincena_de_fecha(hoy, dia_corte);
    metas
        .iter()
        .filter(|m| m.meta.estado == EstadoMeta::Activa)
        .map(|m| {
            let faltante_centavos = (m.meta.monto_objetivo_centavos - m.acumulado_centavos).max(0);

            let rangos_relevantes: Vec<&EscenarioQuincena> = match m.meta.fecha_objetivo {
                Some(fecha_obj) => {
                    let q_obj = quincena_de_fecha(fecha_obj, dia_corte);
                    quincenas
                        .iter()
                        .filter(|q| {
                            q.rango.desde >= quincena_actual.desde && q.rango.desde <= q_obj.desde
                        })
                        .collect()
                }
                None => quincenas
                    .iter()
                    .filter(|q| q.rango.desde >= quincena_actual.desde)
                    .collect(),
            };

            let libre_simulado_referencia_centavos = if rangos_relevantes.is_empty() {
                quincenas
                    .iter()
                    .map(|q| q.libre_quincena_simulado_centavos)
                    .min()
            } else {
                rangos_relevantes
                    .iter()
                    .map(|q| q.libre_quincena_simulado_centavos)
                    .min()
            };

            let ritmo_afectado = match m.meta.fecha_objetivo {
                Some(fecha_obj) if m.acumulado_centavos > 0 => {
                    if faltante_centavos <= 0 {
                        Some(false)
                    } else {
                        let q_obj = quincena_de_fecha(fecha_obj, dia_corte);
                        if q_obj.hasta < quincena_actual.desde {
                            Some(true)
                        } else {
                            let quincenas_restantes =
                                contar_quincenas(quincena_actual.desde, q_obj.desde, dia_corte);
                            if quincenas_restantes == 0 {
                                Some(true)
                            } else {
                                let aporte_necesario = ceil_div_i64(
                                    faltante_centavos,
                                    i64::from(quincenas_restantes),
                                );
                                let evaluadas: Vec<&EscenarioQuincena> = rangos_relevantes;
                                if evaluadas.is_empty() {
                                    // Sin quincenas analizadas en el rango relevante: no se puede
                                    // sostener con el escenario analizado.
                                    Some(true)
                                } else {
                                    Some(
                                        evaluadas
                                            .iter()
                                            .any(|q| q.libre_quincena_simulado_centavos < aporte_necesario),
                                    )
                                }
                            }
                        }
                    }
                }
                _ => None,
            };

            ImpactoMeta {
                meta_id: m.meta.id,
                nombre: m.meta.nombre.clone(),
                monto_objetivo_centavos: m.meta.monto_objetivo_centavos,
                acumulado_centavos: m.acumulado_centavos,
                faltante_centavos,
                fecha_objetivo: m.meta.fecha_objetivo,
                a_ritmo_base: m.a_ritmo,
                libre_simulado_referencia_centavos,
                ritmo_afectado,
            }
        })
        .collect()
}

fn ceil_div_i64(a: i64, b: i64) -> i64 {
    if b <= 0 {
        return a;
    }
    (a + b - 1) / b
}
