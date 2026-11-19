//! Orquestación de compromisos: vencimientos agrupados para la UI y confirmación
//! de pagos (crea el movimiento gasto + registra el período, con deduplicación).

use crate::compromisos::port::{CompromisosError, CompromisosPort};
use crate::models::{Compromiso, Movement, MovementOrigin, MovementType, Vencimiento};
use crate::storage::port::StoragePort;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VencimientoUI {
    pub compromiso: Compromiso,
    pub periodo_key: String,
    pub fecha: NaiveDate,
    /// Días desde el vencimiento (positivo = atrasado, 0 = hoy, negativo = futuro).
    pub dias: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VencimientosResponse {
    pub atrasados: Vec<VencimientoUI>,
    pub hoy: Vec<VencimientoUI>,
    pub proximos: Vec<VencimientoUI>,
}

impl VencimientosResponse {
    /// FR-C04: badge = atrasados + vencen hoy.
    pub fn badge(&self) -> usize {
        self.atrasados.len() + self.hoy.len()
    }
}

/// Lista vencimientos de todos los compromisos activos, agrupados por atrasados /
/// hoy / próximos (ventana 7 días). Pura salvo por leer la base.
pub fn calcular_vencimientos(
    compromisos: &dyn CompromisosPort,
    hoy: NaiveDate,
) -> Result<VencimientosResponse, CompromisosError> {
    let mut response = VencimientosResponse {
        atrasados: Vec::new(),
        hoy: Vec::new(),
        proximos: Vec::new(),
    };
    for c in compromisos.list()? {
        let pagados = compromisos.periodos_pagados(c.id)?;
        for v in c.vencimientos_recordar(hoy, &pagados) {
            let dias = (hoy - v.fecha).num_days();
            let ui = VencimientoUI {
                compromiso: c.clone(),
                periodo_key: v.periodo_key,
                fecha: v.fecha,
                dias,
            };
            if dias > 0 {
                response.atrasados.push(ui);
            } else if dias == 0 {
                response.hoy.push(ui);
            } else {
                response.proximos.push(ui);
            }
        }
    }
    let orden = |a: &VencimientoUI, b: &VencimientoUI| {
        a.fecha.cmp(&b.fecha).then(a.compromiso.concepto.cmp(&b.compromiso.concepto))
    };
    response.atrasados.sort_by(orden);
    response.hoy.sort_by(orden);
    response.proximos.sort_by(orden);
    Ok(response)
}

/// Próximo vencimiento (o atrasado más viejo) de un compromiso para su tarjeta de gestión.
pub fn proximo_vencimiento(
    compromiso: &Compromiso,
    hoy: NaiveDate,
    periodos_pagados: &[String],
) -> Option<Vencimiento> {
    compromiso.primer_pendiente(hoy, periodos_pagados)
}

/// Confirma el pago de un compromiso para su período: crea el movimiento gasto,
/// registra el vínculo (dedup) e incrementa el contador de cuotas si aplica.
/// El monto puede diferir del monto base (corrección puntual, C3).
pub fn confirmar_pago(
    storage: &dyn StoragePort,
    compromisos: &dyn CompromisosPort,
    compromiso: &Compromiso,
    periodo_key: String,
    monto_centavos: i64,
    fecha: NaiveDate,
) -> Result<Movement, Box<dyn std::error::Error>> {
    // Si ya estaba pagado, no se duplica (el mensaje amigable lo da el UNIQUE del storage).
    let pagados = compromisos.periodos_pagados(compromiso.id)?;
    if pagados.contains(&periodo_key) {
        return Err(format!(
            "El período {periodo_key} ya tiene un pago confirmado"
        )
        .into());
    }

    let movimiento = Movement::new(
        MovementType::Gasto,
        monto_centavos,
        compromiso.concepto.clone(),
        fecha,
        MovementOrigin::Formulario,
    )?;
    let saved = storage.save(movimiento)?;
    compromisos.registrar_pago(compromiso.id, periodo_key, saved.clone())?;
    Ok(saved)
}
