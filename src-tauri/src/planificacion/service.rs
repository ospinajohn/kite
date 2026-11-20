//! Cálculo puro de la planificación quincenal (feature 003) y ensamblado desde
//! los puertos de storage/compromisos. La vista es derivada: no crea registros
//! ni modifica movimientos o compromisos (FR-P09).

use crate::compromisos::port::CompromisosPort;
use crate::models::{Movement, MovementType};
use crate::storage::port::StoragePort;
use chrono::{Datelike, Local, NaiveDate};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Rango inclusivo de una quincena: primera = 1..corte, segunda = corte+1..fin de mes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RangoQuincena {
    pub desde: NaiveDate,
    pub hasta: NaiveDate,
}

impl RangoQuincena {
    pub fn contiene(&self, fecha: NaiveDate) -> bool {
        self.desde <= fecha && fecha <= self.hasta
    }
}

/// Quincena que contiene a `fecha` según el día de corte (1..=28, S2/FR-P01).
///
/// # Panics
///
/// Si `dia_corte` no está en 1..=28 (se eligió panic para mantener la función
/// total en su dominio válido; `plan_de_estado` valida antes de llamarla y
/// devuelve un error con mensaje en español).
pub fn quincena_de_fecha(fecha: NaiveDate, dia_corte: u32) -> RangoQuincena {
    assert!(
        (1..=28).contains(&dia_corte),
        "El día de corte debe estar entre 1 y 28"
    );
    let ultimo = ultimo_dia_mes(fecha.year(), fecha.month());
    if fecha.day() <= dia_corte {
        RangoQuincena {
            desde: fecha.with_day(1).expect("día 1 siempre válido"),
            hasta: fecha
                .with_day(dia_corte)
                .expect("corte <= 28 siempre válido"),
        }
    } else {
        RangoQuincena {
            desde: fecha
                .with_day(dia_corte + 1)
                .expect("el mes tiene más días que el corte"),
            hasta: fecha.with_day(ultimo).expect("último día del mes válido"),
        }
    }
}

/// Quincena anterior en el mismo mes; desde la primera pasa a la segunda del
/// mes previo. Si la segunda del mes previo quedaría vacía (corte 28 en febrero
/// no bisiesto), la salta y devuelve la primera de ese mes (edge case del spec).
/// Asume un rango generado por `quincena_de_fecha`.
pub fn quincena_anterior(rango: RangoQuincena) -> RangoQuincena {
    let corte = corte_del_rango(&rango);
    if !es_primera(&rango) {
        return quincena_de_fecha(rango.desde.with_day(1).expect("día 1 válido"), corte);
    }
    let ultimo_prev = rango.desde.pred_opt().expect("fecha válida");
    if corte < ultimo_prev.day() {
        RangoQuincena {
            desde: ultimo_prev.with_day(corte + 1).expect("segunda no vacía"),
            hasta: ultimo_prev,
        }
    } else {
        quincena_de_fecha(ultimo_prev.with_day(1).expect("día 1 válido"), corte)
    }
}

/// Quincena siguiente en el mismo mes; desde la segunda pasa a la primera del
/// mes próximo. Salta la segunda quincena vacía (corte 28 en febrero no bisiesto).
/// Asume un rango generado por `quincena_de_fecha`.
pub fn quincena_siguiente(rango: RangoQuincena) -> RangoQuincena {
    let corte = corte_del_rango(&rango);
    let primero_actual = rango.desde.with_day(1).expect("día 1 válido");
    let ultimo_actual = ultimo_dia_mes(primero_actual.year(), primero_actual.month());
    if es_primera(&rango) && corte < ultimo_actual {
        RangoQuincena {
            desde: primero_actual
                .with_day(corte + 1)
                .expect("segunda no vacía"),
            hasta: primero_actual
                .with_day(ultimo_actual)
                .expect("fin de mes válido"),
        }
    } else {
        let primero_proximo = primero_actual
            .with_day(ultimo_actual)
            .expect("fin de mes válido")
            .succ_opt()
            .expect("fecha válida");
        quincena_de_fecha(primero_proximo, corte)
    }
}

/// Un compromiso pendiente tal como lo consume el cálculo: desacoplado del
/// modelo de la feature 002 (el ensamblado lo construye desde sus traits).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendienteCompromiso {
    pub compromiso_id: Uuid,
    pub concepto: String,
    pub monto_centavos: i64,
    pub fecha: NaiveDate,
    pub periodo_key: String,
}

/// Vista derivada de una quincena (FR-P02/03), serializable para la UI.
/// «Libre de la quincena» y «Libre real» son estimados según lo registrado
/// (FR-P08); nunca se truncan a cero.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanQuincena {
    pub rango: RangoQuincena,
    pub ingresos_centavos: i64,
    pub gastos_centavos: i64,
    pub pendientes_periodo: Vec<PendienteCompromiso>,
    pub atrasados: Vec<PendienteCompromiso>,
    /// Suma de apartados a metas del período (tipo Aporte), resta del libre (S3/005).
    #[serde(default)]
    pub apartados_periodo_centavos: i64,
    pub libre_quincena_centavos: i64,
    pub libre_real_centavos: i64,
}

/// Cálculo puro (sin DB): solo cuenta lo que cae en el rango.
/// `pendientes` son vencimientos no pagados del período; `atrasados`, de
/// quincenas anteriores (fecha < desde, ya deduplicados por el ensamblado).
/// `libre_quincena = ingresos − gastos − pendientes − apartados`; `libre_real`
/// resta además los atrasados. Los negativos se conservan tal cual (US1.5).
pub fn calcular_plan(
    rango: RangoQuincena,
    movimientos: &[Movement],
    pendientes: &[PendienteCompromiso],
    atrasados: &[PendienteCompromiso],
) -> PlanQuincena {
    let suma_movimientos = |tipo: MovementType| -> i64 {
        movimientos
            .iter()
            .filter(|m| m.tipo == tipo && rango.contiene(m.fecha))
            .map(|m| m.monto_centavos)
            .sum()
    };
    let ingresos_centavos = suma_movimientos(MovementType::Ingreso);
    let gastos_centavos = suma_movimientos(MovementType::Gasto);
    let apartados_periodo_centavos = suma_movimientos(MovementType::Aporte);

    let pendientes_periodo: Vec<PendienteCompromiso> = pendientes
        .iter()
        .filter(|p| rango.contiene(p.fecha))
        .cloned()
        .collect();
    let atrasados_previos: Vec<PendienteCompromiso> = atrasados
        .iter()
        .filter(|p| p.fecha < rango.desde)
        .cloned()
        .collect();

    let total_pendientes: i64 = pendientes_periodo.iter().map(|p| p.monto_centavos).sum();
    let total_atrasados: i64 = atrasados_previos.iter().map(|p| p.monto_centavos).sum();
    let libre_quincena_centavos =
        ingresos_centavos - gastos_centavos - total_pendientes - apartados_periodo_centavos;
    let libre_real_centavos = libre_quincena_centavos - total_atrasados;

    PlanQuincena {
        rango,
        ingresos_centavos,
        gastos_centavos,
        pendientes_periodo,
        atrasados: atrasados_previos,
        apartados_periodo_centavos,
        libre_quincena_centavos,
        libre_real_centavos,
    }
}

/// Ensambla el plan de una quincena desde el estado persistido: movimientos del
/// rango + vencimientos no pagados de los compromisos (pagados, pausados y
/// agotados no descuentan, FR-P04, garantizado por `vencimientos_recordar`).
/// `referencia` define la quincena navegada; `None` usa la quincena en curso.
///
/// Limitación v1: `vencimientos_recordar` solo cubre vencimientos hasta hoy+7;
/// los compromisos que vencen más allá no aparecen al navegar a quincenas
/// futuras (aceptable en v1, el líder lo valida).
pub fn plan_de_estado(
    storage: &dyn StoragePort,
    compromisos: &dyn CompromisosPort,
    corte: u32,
    referencia: Option<NaiveDate>,
) -> Result<PlanQuincena, Box<dyn std::error::Error>> {
    if !(1..=28).contains(&corte) {
        return Err("El día de corte debe estar entre 1 y 28".into());
    }
    let hoy = Local::now().date_naive();
    let rango = quincena_de_fecha(referencia.unwrap_or(hoy), corte);
    let movimientos = storage.query(Some(rango.desde), Some(rango.hasta))?;

    let mut pendientes = Vec::new();
    let mut atrasados_por_compromiso: HashMap<Uuid, PendienteCompromiso> = HashMap::new();
    for c in compromisos.list()? {
        let pagados = compromisos.periodos_pagados(c.id)?;
        for v in c.vencimientos_recordar(hoy, &pagados) {
            let p = PendienteCompromiso {
                compromiso_id: c.id,
                concepto: c.concepto.clone(),
                monto_centavos: c.monto_centavos,
                fecha: v.fecha,
                periodo_key: v.periodo_key,
            };
            if rango.contiene(v.fecha) {
                pendientes.push(p);
            } else if v.fecha < rango.desde {
                // Atrasado: una sola vez por compromiso, el período más reciente
                // no pagado (coherente con la 002).
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

    Ok(calcular_plan(rango, &movimientos, &pendientes, &atrasados))
}

/// El corte se deriva del rango: primera → hasta.day(); segunda → desde.day() − 1.
fn corte_del_rango(rango: &RangoQuincena) -> u32 {
    if es_primera(rango) {
        rango.hasta.day()
    } else {
        rango.desde.day() - 1
    }
}

fn es_primera(rango: &RangoQuincena) -> bool {
    rango.desde.day() == 1
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
