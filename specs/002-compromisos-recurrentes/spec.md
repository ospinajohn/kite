# Feature Specification: Compromisos recurrentes y recordatorios de pago

**Feature Branch**: `002-compromisos-recurrentes`
**Created**: 2026-11-12
**Status**: Aprobado — decisiones C1–C5 resueltas (2026-11-12)
**Predecessor**: `001-registro-consulta-movimientos` (no se modifica; esta feature se apoya en su modelo de movimientos)

**Input**: User request: «Compromisos recurrentes (arriendo, suscripciones, cuotas) + recordatorios de pago».

---

## Decisiones de la sesión de clarificación (2026-11-12)

- **C1 — Registro del pago: recordar y confirmar.** Al llegar la fecha de vencimiento, Kite avisa y ofrece registrar el gasto con monto y concepto precargados; el usuario confirma, corrige o cancela. Nada se guarda automáticamente.
- **C2 — Frecuencias: mensual y quincenal.** Mensual = día fijo del mes (ej. arriendo el 5). Quincenal = primera y/o segunda quincena del mes. Semanal y anual quedan fuera de esta versión.
- **C3 — Monto: monto base con corrección puntual.** El compromiso declara un monto base; al confirmar el pago el usuario puede corregir el monto de ese pago puntual (cuota que sube, factura variable). La corrección aplica solo a ese pago.
- **C4 — Recordatorio: aviso al abrir + notificación de Windows.** Al abrir Kite se muestran los vencimientos atrasados y del día (con badge en la app), y además Kite envía una notificación de Windows el día del vencimiento si la app está corriendo. La ventana de «Próximos» es de 7 días [APROBADO].
- **C5 — Duración: configurable, una sola opción al crear.** Al crear un compromiso se elige UNA de estas duraciones (nunca varias a la vez): **indefinido** (default), **fecha fin** (deja de recordarse después de esa fecha) o **cuotas** (N cuotas; se agota al confirmarse la última).

---

## User Scenarios & Testing

### User Story 1 — Crear un compromiso (P1)

Como usuario, quiero crear un compromiso —por ejemplo «Arriendo», monto $500.000, mensual, día 5— para que Kite me recuerde pagarlo cada mes.

**Acceptance Scenarios**:

1. **Given** la pantalla de compromisos, **When** creo uno con concepto «Arriendo», monto 500.000, frecuencia mensual, día 5, **Then** queda guardado y activo, con su próximo vencimiento calculado.
2. **Given** la creación de un compromiso, **When** intento guardarlo sin concepto o con monto ≤ 0, **Then** no se guarda y se indica qué falta.
3. **Given** un compromiso quincenal, **When** lo creo marcando «primera quincena», **Then** su vencimiento se calcula en la primera quincena de cada mes.

### User Story 2 — Ver los vencimientos (P1)

Como usuario, quiero que al abrir Kite vea qué compromisos vencen hoy, cuáles están atrasados y cuáles vienen, para no depender de mi memoria.

**Acceptance Scenarios**:

1. **Given** un compromiso con vencimiento hoy, **When** abro la app, **Then** aparece en la sección «Vencen hoy» y la app muestra una notificación de Windows (C4).
2. **Given** un compromiso con vencimiento hace 3 días sin pago confirmado, **When** abro la app, **Then** aparece como «Atrasado» hasta que se confirme su pago.
3. **Given** compromisos con vencimiento en los próximos 7 días, **When** abro la app, **Then** aparecen en «Próximos» con su fecha.
4. **Given** que hay compromisos vencidos o por vencer hoy, **When** navego por la app, **Then** un badge con la cantidad pendiente queda visible en la pestaña de compromisos.

### User Story 3 — Confirmar un pago (P1)

Como usuario, quiero confirmar el pago de un compromiso cuando lo hago, para que quede como gasto en mi historial y el compromiso pase al siguiente período.

**Acceptance Scenarios**:

1. **Given** un compromiso «Vence hoy», **When** pulso «Registrar pago», **Then** Kite propone un gasto con concepto, monto base y fecha precargados, y lo muestra para revisar antes de guardar (C1).
2. **Given** la propuesta de pago, **When** la confirmo sin cambios, **Then** se guarda el movimiento, aparece en historial y resumen, y el compromiso pasa al siguiente período.
3. **Given** la propuesta de pago, **When** corrijo el monto a uno mayor antes de confirmar, **Then** el movimiento se guarda con el monto corregido y el compromiso conserva su monto base para el próximo período (C3).
4. **Given** la propuesta de pago, **When** la cancelo, **Then** no se guarda ningún movimiento y el compromiso sigue pendiente en su período.
5. **Given** un compromiso ya pagado en su período actual, **When** intento registrar otro pago, **Then** la app no ofrece el registro (evita duplicados).

### User Story 4 — Gestionar compromisos (P2)

Como usuario, quiero editar, pausar y eliminar compromisos, para mantener la lista al día cuando cambia un arriendo, cancelo una suscripción o ya no aplica.

**Acceptance Scenarios**:

1. **Given** un compromiso, **When** edito su monto base, **Then** los próximos vencimientos usan el nuevo monto; los pagos ya confirmados no cambian.
2. **Given** un compromiso, **When** lo pauso, **Then** deja de aparecer en vencimientos y notificaciones sin perderse; **When** lo reactivo, **Then** vuelve a recordarse desde el siguiente vencimiento.
3. **Given** un compromiso, **When** pido eliminarlo, **Then** se pide confirmación previa; los movimientos de pagos ya confirmados se conservan en el historial.
4. **Given** un compromiso, **When** lo edito, **Then** puedo cambiar concepto, monto, frecuencia y día de vencimiento.
5. **Given** un compromiso con duración «cuotas» de 12, **When** se confirma el pago 12, **Then** deja de aparecer en vencimientos (agotado); **When** intento registrar el pago 13, **Then** la app no lo ofrece.
6. **Given** un compromiso con fecha fin ya pasada, **When** abro la app, **Then** no aparece en vencimientos ni genera notificaciones.
7. **Given** un compromiso, **When** lo creo, **Then** se elige exactamente una duración —indefinido, fecha fin o cuotas—, nunca varias al mismo tiempo.

### Edge Cases

- Compromiso mensual el día 29/30/31 en meses cortos: vence el último día del mes.
- Duración «cuotas» con N=0 o negativo: no se guarda; se pide un N ≥ 1.
- Duración «fecha fin» anterior a hoy al crear: se advierte que el compromiso no generará vencimientos.
- Duración «cuotas»: el contador de cuotas confirmadas incrementa solo con pagos confirmados; editar el compromiso no reinicia el contador.
- Compromiso vencido hace mucho tiempo (múltiples períodos sin pagar): se muestra como atrasado del período más reciente no pagado; confirmar el pago salta al siguiente período vigente.
- Confirmación del pago el mismo día del vencimiento vs. días después: la fecha del movimiento se precarga con hoy y puede corregirse (coherente con FR de la 001: fecha ≤ hoy).
- Notificación cuando la app no está corriendo: no hay notificación; el aviso al abrir cubre el caso.
- Sin compromisos: la sección muestra un estado vacío explicativo, sin error.
- Concepto duplicado (dos compromisos con el mismo nombre): se permite; se distinguen por monto y frecuencia.

## Requirements

- **FR-C01**: El sistema DEBE permitir crear, editar y eliminar un compromiso con: concepto, monto base (COP), frecuencia (mensual / quincenal) y día o quincena de vencimiento. [APROBADO — C1, C2, C3]
- **FR-C02**: El sistema DEBE pausar y reactivar un compromiso sin eliminarlo; pausado = sin recordatorios. [APROBADO — C4]
- **FR-C03**: El sistema DEBE calcular los próximos vencimientos por compromiso y agruparlos en Atrasados / Vencen hoy / Próximos (7 días). [APROBADO — C4; ventana de 7 días PROPUESTO]
- **FR-C04**: El sistema DEBE mostrar un badge con la cantidad de compromisos pendientes (atrasados + hoy) visible mientras se navega la app. [APROBADO — C4]
- **FR-C05**: El sistema DEBE ofrecer «Registrar pago» para un compromiso pendiente, proponiendo un gasto precargado (concepto, monto base, fecha hoy) que el usuario confirma, corrige o cancela. [APROBADO — C1]
- **FR-C06**: El sistema DEBE permitir corregir el monto solo para el pago puntual; el monto base del compromiso no cambia por esa corrección. [APROBADO — C3]
- **FR-C07**: El sistema DEBE evitar más de un pago confirmado por compromiso en el mismo período. [APROBADO — deduplicación]
- **FR-C08**: El sistema DEBE enviar una notificación de Windows el día del vencimiento de cada compromiso activo, mientras la app esté corriendo. [APROBADO — C4]
- **FR-C09**: Los compromisos NO DEBEN aparecer en el historial ni en el resumen («Balance registrado») hasta que su pago se confirme como movimiento. [APROBADO — coherencia con FR-010/FR-011 de la 001]
- **FR-C10**: El pago confirmado se registra con la fecha precargada en hoy, editable, con máximo hoy (coherente con la 001). [APROBADO]
- **FR-C11**: El sistema DEBE permitir elegir al crear UNA duración por compromiso —indefinido (default), fecha fin o cuotas (N ≥ 1)—, mutuamente excluyentes; con fecha fin vencida o cuotas agotadas el compromiso deja de recordarse sin eliminarse. [APROBADO — C5]

## Exclusiones explícitas

- Cálculo de dinero libre y planificación por quincena: feature 003; aquí solo se muestran vencimientos.
- Frecuencias semanal y anual: fuera de esta versión.
- Combinar duraciones en un mismo compromiso (ej. fecha fin y cuotas a la vez): excluido por C5.
- Sincronización, recordatorios cuando la app está cerrada, recordatorios por email.
- Autenticación / multiusuario.

## Key Entities

- **Compromiso**: concepto, monto base (COP), frecuencia (mensual / quincenal), día de vencimiento (o quincena primera/segunda), duración (indefinido / fecha fin / N cuotas), estado (activo / pausado), creado_en, actualizado_en.
- **Pago de compromiso**: vínculo entre un compromiso y el movimiento gasto que confirma su pago, con el período cubierto y el monto efectivamente pagado (permite deduplicar y conservar el monto corregido).

## Decisiones cerradas

| Decisión | Estado |
| --- | --- |
| Duración finita (fecha fin / N cuotas) para suscripciones con término | [APROBADO — C5] configurable, una opción al crear, mutuamente excluyentes |
| Ventana de «Próximos» (7 días) | [APROBADO — C4] |
