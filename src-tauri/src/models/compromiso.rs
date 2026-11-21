use chrono::{DateTime, Datelike, Local, NaiveDate};
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// Frecuencia de repetición de un compromiso (C2: solo mensual y quincenal en esta versión).
///
/// La exclusividad día/quincena queda garantizada por el diseño del enum: un compromiso
/// mensual lleva su día (1..=31, clamp al fin de mes) y un quincenal lleva su quincena.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Frecuencia {
    Mensual(u8),
    Quincenal(Quincena),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Quincena {
    Primera,
    Segunda,
}

/// Duración de un compromiso (C5): exactamente una opción, mutuamente excluyentes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Duracion {
    Indefinido,
    FechaFin(NaiveDate),
    /// N cuotas, N >= 1; se agota al confirmarse la última.
    Cuotas(u32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EstadoCompromiso {
    Activo,
    Pausado,
}

/// Un vencimiento pendiente de un compromiso: clave de período + fecha.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Vencimiento {
    pub periodo_key: String,
    pub fecha: NaiveDate,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Compromiso {
    pub id: Uuid,
    pub concepto: String,
    pub monto_centavos: i64,
    pub frecuencia: Frecuencia,
    pub duracion: Duracion,
    pub estado: EstadoCompromiso,
    pub cuotas_confirmadas: u32,
    pub creado_en: DateTime<Local>,
    pub actualizado_en: DateTime<Local>,
}

#[derive(Debug, thiserror::Error)]
pub enum CompromisoError {
    #[error("Monto inválido: {0}")]
    InvalidAmount(String),
    #[error("Concepto inválido: {0}")]
    InvalidConcept(String),
    #[error("Frecuencia inválida: {0}")]
    InvalidFrequency(String),
    #[error("Duración inválida: {0}")]
    InvalidDuration(String),
}

/// Ventana de «Próximos» aprobada (C4): 7 días.
pub const VENTANA_PROXIMOS_DIAS: i64 = 7;

/// Datos crudos de un compromiso tal como viven en la base de datos.
#[derive(Debug, Clone)]
pub struct CompromisoFila {
    pub id: Uuid,
    pub concepto: String,
    pub monto_centavos: i64,
    pub frecuencia: Frecuencia,
    pub duracion: Duracion,
    pub estado: EstadoCompromiso,
    pub cuotas_confirmadas: u32,
    pub creado_en: DateTime<Local>,
    pub actualizado_en: DateTime<Local>,
}

impl Compromiso {
    pub fn new(
        concepto: String,
        monto_centavos: i64,
        frecuencia: Frecuencia,
        duracion: Duracion,
    ) -> Result<Self, CompromisoError> {
        Self::validar(&concepto, monto_centavos, &frecuencia, &duracion)?;
        let now = Local::now();
        Ok(Self {
            id: Uuid::new_v4(),
            concepto: concepto.trim().to_string(),
            monto_centavos,
            frecuencia,
            duracion,
            estado: EstadoCompromiso::Activo,
            cuotas_confirmadas: 0,
            creado_en: now,
            actualizado_en: now,
        })
    }

    /// Reconstruye un compromiso cargado de la base de datos (sin revalidar fecha futura).
    pub fn desde_fila(fila: CompromisoFila) -> Self {
        Self {
            id: fila.id,
            concepto: fila.concepto,
            monto_centavos: fila.monto_centavos,
            frecuencia: fila.frecuencia,
            duracion: fila.duracion,
            estado: fila.estado,
            cuotas_confirmadas: fila.cuotas_confirmadas,
            creado_en: fila.creado_en,
            actualizado_en: fila.actualizado_en,
        }
    }

    pub fn update(
        &self,
        concepto: String,
        monto_centavos: i64,
        frecuencia: Frecuencia,
        duracion: Duracion,
    ) -> Result<Self, CompromisoError> {
        Self::validar(&concepto, monto_centavos, &frecuencia, &duracion)?;
        Ok(Self {
            id: self.id,
            concepto: concepto.trim().to_string(),
            monto_centavos,
            frecuencia,
            duracion,
            estado: self.estado,
            // Cambiar el compromiso no reinicia el contador de cuotas (edge case del spec).
            cuotas_confirmadas: self.cuotas_confirmadas,
            creado_en: self.creado_en,
            actualizado_en: Local::now(),
        })
    }

    fn validar(
        concepto: &str,
        monto_centavos: i64,
        frecuencia: &Frecuencia,
        duracion: &Duracion,
    ) -> Result<(), CompromisoError> {
        if monto_centavos <= 0 {
            return Err(CompromisoError::InvalidAmount(
                "El monto debe ser mayor que cero".into(),
            ));
        }
        let concepto_trim = concepto.trim();
        if concepto_trim.is_empty() {
            return Err(CompromisoError::InvalidConcept(
                "El concepto es obligatorio".into(),
            ));
        }
        if concepto_trim.len() > 200 {
            return Err(CompromisoError::InvalidConcept(
                "El concepto no puede exceder 200 caracteres".into(),
            ));
        }
        match frecuencia {
            Frecuencia::Mensual(dia) => {
                if *dia == 0 || *dia > 31 {
                    return Err(CompromisoError::InvalidFrequency(
                        "El día del mes debe estar entre 1 y 31".into(),
                    ));
                }
            }
            Frecuencia::Quincenal(_) => {}
        }
        match duracion {
            Duracion::Cuotas(n) => {
                if *n == 0 {
                    return Err(CompromisoError::InvalidDuration(
                        "El número de cuotas debe ser al menos 1".into(),
                    ));
                }
            }
            Duracion::Indefinido | Duracion::FechaFin(_) => {}
        }
        Ok(())
    }

    /// Fecha de vencimiento del compromiso dentro del mes dado, con clamp al último día
    /// (día 31 en abril → 30).
    pub fn vencimiento_en_mes(&self, anio: i32, mes: u32) -> NaiveDate {
        let ultimo = ultimo_dia_mes(anio, mes);
        match &self.frecuencia {
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

    /// Clave de período para deduplicación de pagos: `YYYY-MM` (mensual),
    /// `YYYY-MM-Q1` / `YYYY-MM-Q2` (quincenal).
    pub fn periodo_key_en_mes(&self, anio: i32, mes: u32) -> String {
        match &self.frecuencia {
            Frecuencia::Mensual(_) => format!("{anio:04}-{mes:02}"),
            Frecuencia::Quincenal(Quincena::Primera) => format!("{anio:04}-{mes:02}-Q1"),
            Frecuencia::Quincenal(Quincena::Segunda) => format!("{anio:04}-{mes:02}-Q2"),
        }
    }

    /// Vencimientos sin pagar con fecha dentro de [creación, hoy + ventana 7 días].
    /// Incluye atrasados (períodos anteriores sin pagar), hoy y próximos.
    /// Respeta duración (fecha fin / cuotas) y estado pausado.
    pub fn vencimientos_recordar(
        &self,
        hoy: NaiveDate,
        periodos_pagados: &[String],
    ) -> Vec<Vencimiento> {
        self.vencimientos_hasta(
            hoy + chrono::Days::new(VENTANA_PROXIMOS_DIAS as u64),
            periodos_pagados,
            usize::MAX,
        )
    }

    /// Primer vencimiento sin pagar (cronológico), sin límite de ventana.
    /// Para la lista de gestión: si hay atrasado, es ese; si no, el próximo futuro.
    pub fn primer_pendiente(
        &self,
        hoy: NaiveDate,
        periodos_pagados: &[String],
    ) -> Option<Vencimiento> {
        self.vencimientos_hasta(hoy + chrono::Days::new(3660), periodos_pagados, 1)
            .into_iter()
            .next()
    }

    /// Vencimientos sin pagar con fecha <= `limite_fecha`, respetando duración,
    /// estado y períodos ya pagados. Expone el cálculo para usos que necesitan un
    /// horizonte mayor que la ventana de recordatorio (p. ej. simulación).
    pub fn vencimientos_hasta_fecha(
        &self,
        limite_fecha: NaiveDate,
        periodos_pagados: &[String],
    ) -> Vec<Vencimiento> {
        self.vencimientos_hasta(limite_fecha, periodos_pagados, usize::MAX)
    }

    fn vencimientos_hasta(
        &self,
        limite_fecha: NaiveDate,
        periodos_pagados: &[String],
        max_items: usize,
    ) -> Vec<Vencimiento> {
        if self.estado == EstadoCompromiso::Pausado {
            return Vec::new();
        }
        // Duración: fecha fin vencida / cuotas agotadas → nada que recordar.
        let mut restantes_cuotas: Option<u32> = None;
        let fecha_limite_duracion: NaiveDate = match self.duracion {
            Duracion::FechaFin(f) => f,
            Duracion::Indefinido => limite_fecha,
            Duracion::Cuotas(n) => {
                restantes_cuotas = Some(n.saturating_sub(self.cuotas_confirmadas));
                limite_fecha
            }
        };
        if restantes_cuotas == Some(0) {
            return Vec::new();
        }

        let desde = self.creado_en.date_naive();
        let limite = limite_fecha.min(fecha_limite_duracion);

        let mut vencimientos = Vec::new();
        let (mut anio, mut mes) = (desde.year(), desde.month());
        // Bounded: recorre como mucho 120 meses (10 años) para indefinidos antiguos.
        for _ in 0..120 {
            let fecha = self.vencimiento_en_mes(anio, mes);
            if fecha > limite || (restantes_cuotas == Some(0)) {
                break;
            }
            if fecha >= desde {
                let key = self.periodo_key_en_mes(anio, mes);
                if !periodos_pagados.contains(&key) {
                    vencimientos.push(Vencimiento {
                        periodo_key: key,
                        fecha,
                    });
                    if vencimientos.len() >= max_items {
                        break;
                    }
                    if restantes_cuotas == Some(vencimientos.len() as u32) {
                        break;
                    }
                }
            }
            // Avanza de mes.
            if mes == 12 {
                anio += 1;
                mes = 1;
            } else {
                mes += 1;
            }
        }
        vencimientos
    }

    pub fn pausar(&mut self) {
        self.estado = EstadoCompromiso::Pausado;
        self.actualizado_en = Local::now();
    }

    pub fn reactivar(&mut self) {
        self.estado = EstadoCompromiso::Activo;
        self.actualizado_en = Local::now();
    }
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

impl fmt::Display for Compromiso {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let frec = match &self.frecuencia {
            Frecuencia::Mensual(dia) => format!("mensual día {dia}"),
            Frecuencia::Quincenal(Quincena::Primera) => "quincenal (1ª)".to_string(),
            Frecuencia::Quincenal(Quincena::Segunda) => "quincenal (2ª)".to_string(),
        };
        write!(f, "{}  [{}]", self.concepto, frec)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    fn compromiso_mensual(dia: u8, creado: NaiveDate) -> Compromiso {
        let mut c = Compromiso::new(
            "Arriendo".into(),
            500_000,
            Frecuencia::Mensual(dia),
            Duracion::Indefinido,
        )
        .unwrap();
        c.creado_en = creado
            .and_hms_opt(10, 0, 0)
            .unwrap()
            .and_local_timezone(Local)
            .unwrap();
        c
    }

    #[test]
    fn new_ok_mensual() {
        let c = Compromiso::new(
            "Arriendo".into(),
            500_000,
            Frecuencia::Mensual(5),
            Duracion::Indefinido,
        )
        .unwrap();
        assert_eq!(c.monto_centavos, 500_000);
        assert_eq!(c.estado, EstadoCompromiso::Activo);
        assert_eq!(c.cuotas_confirmadas, 0);
    }

    #[test]
    fn new_rechaza_monto_cero() {
        let err = Compromiso::new(
            "Arriendo".into(),
            0,
            Frecuencia::Mensual(5),
            Duracion::Indefinido,
        )
        .unwrap_err();
        assert!(matches!(err, CompromisoError::InvalidAmount(_)));
    }

    #[test]
    fn new_rechaza_concepto_vacio() {
        let err = Compromiso::new(
            "   ".into(),
            1000,
            Frecuencia::Mensual(5),
            Duracion::Indefinido,
        )
        .unwrap_err();
        assert!(matches!(err, CompromisoError::InvalidConcept(_)));
    }

    #[test]
    fn new_rechaza_dia_invalido() {
        for dia in [0u8, 32u8] {
            let err = Compromiso::new(
                "Arriendo".into(),
                1000,
                Frecuencia::Mensual(dia),
                Duracion::Indefinido,
            )
            .unwrap_err();
            assert!(matches!(err, CompromisoError::InvalidFrequency(_)));
        }
    }

    #[test]
    fn new_rechaza_cuotas_cero() {
        let err = Compromiso::new(
            "Televisor".into(),
            1000,
            Frecuencia::Mensual(10),
            Duracion::Cuotas(0),
        )
        .unwrap_err();
        assert!(matches!(err, CompromisoError::InvalidDuration(_)));
    }

    #[test]
    fn vencimiento_mensual_proximo_mes_si_dia_ya_paso() {
        // Creado 2026-11-20, día 5 → primer vencimiento 2026-12-05.
        let c = compromiso_mensual(5, d(2026, 11, 20));
        let hoy = d(2026, 11, 12);
        let pend = c.primer_pendiente(hoy, &[]).unwrap();
        assert_eq!(pend.fecha, d(2026, 12, 5));
        assert_eq!(pend.periodo_key, "2026-12");
    }

    #[test]
    fn vencimiento_mensual_mismo_mes_si_dia_futuro() {
        // Creado 2026-11-12, día 25 → vence 2026-11-25.
        let c = compromiso_mensual(25, d(2026, 11, 12));
        let pend = c.primer_pendiente(d(2026, 11, 12), &[]).unwrap();
        assert_eq!(pend.fecha, d(2026, 11, 25));
        assert_eq!(pend.periodo_key, "2026-11");
    }

    #[test]
    fn vencimiento_mensual_clamp_ultimo_dia() {
        let c = compromiso_mensual(31, d(2026, 1, 1));
        assert_eq!(c.vencimiento_en_mes(2026, 4), d(2026, 4, 30));
        assert_eq!(c.vencimiento_en_mes(2026, 2), d(2026, 2, 28));
        assert_eq!(c.vencimiento_en_mes(2028, 2), d(2028, 2, 29)); // bisiesto
    }

    fn compromiso_quincenal(q: Quincena, creado: NaiveDate) -> Compromiso {
        let mut c = Compromiso::new(
            "Netflix".into(),
            20_000,
            Frecuencia::Quincenal(q),
            Duracion::Indefinido,
        )
        .unwrap();
        c.creado_en = creado
            .and_hms_opt(10, 0, 0)
            .unwrap()
            .and_local_timezone(Local)
            .unwrap();
        c
    }

    #[test]
    fn vencimiento_quincenal_primera_y_segunda() {
        let c1 = compromiso_quincenal(Quincena::Primera, d(2026, 11, 1));
        let c2 = compromiso_quincenal(Quincena::Segunda, d(2026, 11, 1));

        assert_eq!(c1.vencimiento_en_mes(2026, 11), d(2026, 11, 15));
        assert_eq!(c2.vencimiento_en_mes(2026, 11), d(2026, 11, 30));
        assert_eq!(c1.periodo_key_en_mes(2026, 11), "2026-11-Q1");
        assert_eq!(c2.periodo_key_en_mes(2026, 11), "2026-11-Q2");
    }

    #[test]
    fn vencimiento_atrasado_aparece_en_recordar() {
        // Creado 2026-10-20 día 5; hoy 2026-11-12; noviembre quedó atrasado (05-11).
        let c = compromiso_mensual(5, d(2026, 10, 20));
        let recordables = c.vencimientos_recordar(d(2026, 11, 12), &[]);
        let fechas: Vec<NaiveDate> = recordables.iter().map(|v| v.fecha).collect();
        assert_eq!(fechas, vec![d(2026, 11, 5)]); // 12-05 está fuera de la ventana 7 días
    }

    #[test]
    fn pago_confirmado_saca_periodo_de_pendientes() {
        let c = compromiso_mensual(5, d(2026, 10, 20));
        let recordables = c.vencimientos_recordar(d(2026, 11, 12), &["2026-11".to_string()]);
        assert!(recordables.is_empty());
    }

    #[test]
    fn pausado_no_genera_vencimientos() {
        let mut c = compromiso_mensual(5, d(2026, 10, 20));
        c.pausar();
        assert!(c.vencimientos_recordar(d(2026, 11, 12), &[]).is_empty());
        c.reactivar();
        assert!(!c.vencimientos_recordar(d(2026, 11, 12), &[]).is_empty());
    }

    #[test]
    fn cuotas_se_agotan_con_pagos() {
        let creado = d(2026, 9, 20);
        let mut c = compromiso_mensual(5, creado);
        c.duracion = Duracion::Cuotas(2);
        c.cuotas_confirmadas = 0;

        // Sin pagos: pendientes 2026-10-05 y 2026-11-05 (12-05 fuera de ventana con hoy 11-12).
        let recordables = c.vencimientos_recordar(d(2026, 11, 12), &[]);
        let fechas: Vec<NaiveDate> = recordables.iter().map(|v| v.fecha).collect();
        assert_eq!(fechas, vec![d(2026, 10, 5), d(2026, 11, 5)]);

        // Un pago confirmado: queda 2026-11-05.
        c.cuotas_confirmadas = 1;
        let recordables = c.vencimientos_recordar(d(2026, 11, 12), &["2026-10".to_string()]);
        let fechas: Vec<NaiveDate> = recordables.iter().map(|v| v.fecha).collect();
        assert_eq!(fechas, vec![d(2026, 11, 5)]);

        // Dos pagos: agotado, no se recuerda nada.
        c.cuotas_confirmadas = 2;
        assert!(c
            .vencimientos_recordar(
                d(2026, 11, 12),
                &["2026-10".to_string(), "2026-11".to_string()]
            )
            .is_empty());
    }

    #[test]
    fn fecha_fin_limita_vencimientos() {
        let mut c = compromiso_mensual(5, d(2026, 10, 20));
        c.duracion = Duracion::FechaFin(d(2026, 11, 30));
        // Hoy 2026-12-12: 2026-11-05 quedó atrasado y dentro de fecha fin; 12-05 supera el fin.
        let recordables = c.vencimientos_recordar(d(2026, 12, 12), &[]);
        let fechas: Vec<NaiveDate> = recordables.iter().map(|v| v.fecha).collect();
        assert_eq!(fechas, vec![d(2026, 11, 5)]);

        // Pagado noviembre → ya no hay nada que recordar (fin vencido).
        let recordables = c.vencimientos_recordar(d(2026, 12, 12), &["2026-11".to_string()]);
        assert!(recordables.is_empty());
    }

    #[test]
    fn update_no_reinicia_cuotas_confirmadas() {
        let c = Compromiso::new(
            "Televisor".into(),
            120_000,
            Frecuencia::Mensual(10),
            Duracion::Cuotas(12),
        )
        .unwrap();
        let mut c = c;
        c.cuotas_confirmadas = 5;
        let upd = c
            .update(
                "Televisor cuota nueva".into(),
                130_000,
                Frecuencia::Mensual(10),
                Duracion::Cuotas(12),
            )
            .unwrap();
        assert_eq!(upd.cuotas_confirmadas, 5);
        assert_eq!(upd.concepto, "Televisor cuota nueva");
    }

    #[test]
    fn proximos_seis_dias_y_siete_dias_en_ventana() {
        // Vence 2026-11-18; con hoy 11-12 está a 6 días → dentro de la ventana.
        let c = compromiso_mensual(18, d(2026, 11, 1));
        assert_eq!(c.vencimientos_recordar(d(2026, 11, 12), &[]).len(), 1);
        // Con hoy 11-11 está a 7 días → dentro; con hoy 11-10 está a 8 → fuera.
        assert_eq!(c.vencimientos_recordar(d(2026, 11, 11), &[]).len(), 1);
        assert!(c.vencimientos_recordar(d(2026, 11, 10), &[]).is_empty());
    }
}
