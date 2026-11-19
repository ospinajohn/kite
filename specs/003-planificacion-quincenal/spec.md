# Feature Specification: Planificación por quincena y dinero libre

**Feature Branch**: `experimento/ui-ux` (work-units propios)
**Created**: 2026-11-12
**Status**: Aprobado — decisiones C1–C4 de la sesión de clarificación (2026-11-12)
**Predecessors**: `001-registro-consulta-movimientos` (no se modifica), `002-compromisos-recurrentes` (consume sus compromisos y pagos; no se modifica)

**Input**: User request: «Planificación por quincena y cálculo de dinero libre».

---

## Decisiones de la sesión de clarificación (2026-11-12)

- **C1 — Quincena: día de corte configurable.** El usuario define el día de corte D (1–28, default 15). Primera quincena = días 1..D; segunda = D+1..último día del mes. Se persiste y es editable desde la vista de Planificación.
- **C2 — Dinero libre: ambos números.** «Libre de la quincena» = ingresos − gastos − compromisos no pagados que vencen dentro de la quincena. «Libre real» = libre de la quincena − compromisos atrasados de quincenas anteriores. Ambos visibles, con sus componentes.
- **C3 — Ubicación: pestaña + tarjeta dashboard.** Nueva pestaña «Planificación» con el detalle completo y una tarjeta compacta de dinero libre en el Dashboard que enlaza a la pestaña.
- **C4 — Períodos: actual + navegación.** La vista abre en la quincena en curso y permite navegar a la anterior y la siguiente con flechas.

## User Scenarios & Testing

### User Story 1 — Ver mi dinero libre (P1)

Como usuario, quiero ver cuánto me queda de la quincena después de lo que ya gasté y lo que todavía debo pagar, para decidir si puedo hacer un gasto nuevo.

**Acceptance Scenarios**:

1. **Given** ingresos de $1.500.000 y gastos de $300.000 en la quincena, y un compromiso «Arriendo» de $500.000 que vence dentro de la quincena sin pagar, **When** abro Planificación, **Then** «Libre de la quincena» = $700.000.
2. **Given** el caso anterior más un compromiso atrasado de $100.000 de una quincena anterior, **When** veo el detalle, **Then** «Libre real» = $600.000 y el atrasado aparece listado como componente.
3. **Given** un compromiso de la quincena ya pagado, **When** veo el cálculo, **Then** NO se descuenta (el pago ya está en los gastos).
4. **Given** una quincena sin movimientos ni compromisos, **When** la consulto, **Then** ambos números son $0 sin error.
5. **Given** compromisos que hacen el libre negativo, **When** veo el número, **Then** se muestra con signo «−», sin ocultarlo.

### User Story 2 — Configurar mi quincena (P1)

Como usuario, quiero definir el día de corte de mi quincena (mi pago no cae exacto el 15), para que la planificación coincida con mi ritmo real.

**Acceptance Scenarios**:

1. **Given** el día de corte por defecto 15, **When** abro Planificación por primera vez, **Then** la quincena en curso se calcula con 1–15 / 16–fin.
2. **Given** que cambio el corte a 14, **When** guardo, **Then** la vista se recalcula de inmediato (1–14 / 15–fin) y el ajuste persiste al cerrar y reabrir.
3. **Given** un intento de corte 0 o 29+, **When** guardo, **Then** se rechaza con mensaje y el valor anterior se conserva.
4. **Given** un compromiso quincenal «primera» que vence el día 15 y corte configurado en 14, **When** veo la planificación, **Then** ese compromiso se asigna a la SEGUNDA quincena (la asignación es por fecha de vencimiento real, no por la etiqueta del compromiso).

### User Story 3 — Navegar quincenas (P2)

Como usuario, quiero ver la quincena anterior y la siguiente, para revisar cómo cerré y qué se me viene.

**Acceptance Scenarios**:

1. **Given** la quincena en curso, **When** pulso «anterior», **Then** veo la quincena previa con sus movimientos y compromisos de ese período.
2. **Given** la quincena en curso, **When** pulso «siguiente», **Then** veo la próxima quincena: sin movimientos (aún no hay registros futuros) pero con los compromisos que vencerán en ella.
3. **Given** cualquier quincena navegada, **When** vuelvo a «hoy», **Then** regreso a la quincena en curso.

### User Story 4 — Tarjeta en el Dashboard (P2)

Como usuario, quiero ver mi dinero libre de un vistazo al abrir la app, sin entrar a Planificación.

**Acceptance Scenarios**:

1. **Given** la app abierta, **When** veo el Dashboard, **Then** una tarjeta compacta muestra «Libre real» de la quincena en curso.
2. **Given** la tarjeta, **When** la pulso, **Then** navego a la pestaña Planificación.

### Edge Cases

- Meses cortos: la segunda quincena termina el último día real del mes (28/29/30/31).
- Corte = 28: febrero no-bisiesto termina el 28 → segunda quincena de febrero puede quedar vacía (0 días); la vista lo muestra sin error y la navegación la salta o la muestra en cero.
- Compromiso pausado, con fecha fin vencida o cuotas agotadas: NO se descuenta.
- Compromiso atrasado de hace varios períodos: se descuenta una sola vez en «Libre real» (el período más reciente no pagado, coherente con la 002).
- Cambiar el corte reasigna compromisos y movimientos a otras quincenas; los totales históricos del Resumen (feature 001) NO cambian.
- Números negativos: signo «−» explícito, nunca se ocultan ni se truncan a cero.

## Requirements

- **FR-P01**: El sistema DEBE persistir un día de corte configurable (1–28, default 15) y usarlo para definir las quincenas: 1..D y D+1..fin de mes. [APROBADO — C1]
- **FR-P02**: El sistema DEBE calcular para una quincena: ingresos, gastos, compromisos no pagados con vencimiento en el período, «Libre de la quincena» y «Libre real». [APROBADO — C2]
- **FR-P03**: «Libre real» DEBE descontar además los compromisos atrasados de quincenas anteriores (una vez cada uno), listándolos como componente separado. [APROBADO — C2]
- **FR-P04**: Los compromisos se asignan a quincenas por su fecha de vencimiento real; pagados, pausados o agotados (fecha fin / cuotas) NO se descuentan. [APROBADO — coherencia con 002]
- **FR-P05**: La vista DEBE abrir en la quincena en curso y permitir navegar anterior/siguiente y volver a hoy. [APROBADO — C4]
- **FR-P06**: El Dashboard DEBE mostrar una tarjeta compacta con «Libre real» de la quincena en curso que enlaza a Planificación. [APROBADO — C3]
- **FR-P07**: El día de corte DEBE ser editable desde Planificación con validación (1–28) y recálculo inmediato. [APROBADO — C1]
- **FR-P08**: Los rótulos son «Libre de la quincena» y «Libre real», con nota visible de que es un estimado según lo registrado — nunca «saldo bancario» ni dinero garantizado. El «Balance registrado» de la 001 no cambia. [APROBADO — coherencia con FR-011 de la 001]
- **FR-P09**: La planificación es una vista derivada: no crea registros nuevos ni modifica movimientos o compromisos. [APROBADO]

## Exclusiones explícitas

- Metas de ahorro como descuento del libre: feature 005.
- Reservas automáticas o programación de pagos: fuera del roadmap actual.
- Proyección de ingresos futuros (solo se usa lo registrado).
- Modificar el Resumen o el «Balance registrado» de la 001.

## Key Entities

- **Configuración**: clave/valor persistido; por ahora `dia_corte` (entero 1–28, default 15).
- **PlanQuincena** (vista derivada, no persistida): rango de fechas, ingresos, gastos, compromisos pendientes del período, atrasados anteriores, libre_quincena, libre_real.
