use crate::metas::domain::{EstadoMeta, Meta, MetaUI, MetasError};
use crate::metas::port::MetasPort;
use chrono::{Local, NaiveDate};

/// Estado derivado del acumulado frente al objetivo (FR-M04).
pub fn recalcular_estado(acumulado: i64, objetivo: i64) -> EstadoMeta {
    if acumulado >= objetivo {
        EstadoMeta::Completada
    } else {
        EstadoMeta::Activa
    }
}

/// Indicador de ritmo (FR-M08): ¿el acumulado va a la par del tiempo transcurrido?
/// `inicio` es la fecha de referencia (p. ej. creación de la meta).
pub fn a_ritmo(
    acumulado: i64,
    objetivo: i64,
    inicio: NaiveDate,
    hoy: NaiveDate,
    fecha_objetivo: NaiveDate,
) -> bool {
    if acumulado >= objetivo {
        return true;
    }
    let dias_totales = (fecha_objetivo - inicio).num_days();
    if dias_totales <= 0 {
        return acumulado >= objetivo;
    }
    let dias_transcurridos = (hoy - inicio).num_days().clamp(0, dias_totales);
    let esperado = (objetivo as f64) * (dias_transcurridos as f64) / (dias_totales as f64);
    (acumulado as f64) >= esperado
}

/// Recalcula y persiste el estado de una meta según sus apartados vigentes.
/// Una meta cancelada no cambia de estado.
pub fn recalcular_estado_meta(metas: &dyn MetasPort, id: uuid::Uuid) -> Result<Meta, MetasError> {
    let meta = metas.get_by_id(id)?.ok_or(MetasError::NotFound(id))?;
    if meta.estado == EstadoMeta::Cancelada {
        return Ok(meta);
    }
    let acumulado: i64 = metas
        .apartados_de_meta(id)?
        .iter()
        .map(|m| m.monto_centavos)
        .sum();
    let nuevo = recalcular_estado(acumulado, meta.monto_objetivo_centavos);
    if nuevo != meta.estado {
        let mut actualizado = meta.clone();
        actualizado.estado = nuevo;
        actualizado.actualizado_en = Local::now();
        metas.update(actualizado)
    } else {
        Ok(meta)
    }
}

/// Lista metas con progreso y, si aplica, indicador de ritmo.
pub fn listar_metas(metas: &dyn MetasPort) -> Result<Vec<MetaUI>, MetasError> {
    let hoy = Local::now().date_naive();
    let mut items = Vec::new();
    for meta in metas.list()? {
        let apartados = metas.apartados_de_meta(meta.id)?;
        let acumulado_centavos: i64 = apartados.iter().map(|m| m.monto_centavos).sum();
        let ritmo = match (meta.fecha_objetivo, acumulado_centavos > 0) {
            (Some(fecha_objetivo), true) => Some(a_ritmo(
                acumulado_centavos,
                meta.monto_objetivo_centavos,
                meta.creado_en.date_naive(),
                hoy,
                fecha_objetivo,
            )),
            _ => None,
        };
        items.push(MetaUI {
            meta,
            acumulado_centavos,
            a_ritmo: ritmo,
        });
    }
    Ok(items)
}
