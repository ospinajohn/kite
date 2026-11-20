# Feature Specification: Ahorro y metas

**Feature Branch**: `experimento/ui-ux` (work-units propios)
**Created**: 2026-11-12
**Status**: Aprobado — decisiones C1–C4 de la sesión de clarificación (2026-11-12)
**Predecessors**: `001` (modelo de movimientos; se EXTiende con un tipo nuevo, sin alterar ingresos/gastos existentes), `003` (Planificación; se agrega un componente nuevo)

**Input**: User request: «Ahorro y metas» (del listado pospuesto del contexto, promovido el 2026-11-12 tras posponer la 004).

---

## Decisiones de la sesión de clarificación (2026-11-12)

- **C1 — Apartado como tipo propio de movimiento.** Ahorrar se registra como «aporte a meta»: el dinero sale de lo disponible pero NO cuenta como gasto en el historial ni en el resumen; se muestra aparte (ej. «Viaje: aparté $200k»).
- **C2 — Los apartados restan del dinero libre.** En Planificación, los apartados del período aparecen como componente separado y restan del libre (junto a gastos y compromisos).
- **C3 — Fecha objetivo opcional.** Una meta puede llevar fecha objetivo (ej. «viaje en diciembre»); con fecha, muestra si vas a ritmo. Sin fecha, solo monto objetivo.
- **C4 — Cancelar una meta: solo archivar.** Sin movimiento de devolución; el dinero apartado queda como está.

## User Scenarios & Testing

### User Story 1 — Crear una meta (P1)

Como usuario, quiero crear una meta con nombre y monto objetivo (y fecha opcional), para seguir cuánto me falta para algo que quiero.

**Acceptance Scenarios**:

1. **Given** la pestaña Metas, **When** creo «Viaje a la costa» con objetivo $2.000.000 sin fecha, **Then** aparece como activa con progreso $0 de $2.000.000 (0%).
2. **Given** la creación de una meta, **When** guardo sin nombre o con objetivo ≤ 0, **Then** no se guarda y se indica qué falta.
3. **Given** la creación, **When** pongo fecha objetivo pasada o de hoy, **Then** se rechaza con mensaje (la fecha objetivo es futura).
4. **Given** varias metas activas, **When** abro la pestaña, **Then** todas se listan con su progreso.
5. **Given** una meta con fecha objetivo, **When** veo su tarjeta, **Then** muestra si el ritmo actual alcanza (indicador simple: a ritmo / atrasado).

### User Story 2 — Apartar dinero (P1)

Como usuario, quiero registrar que aparté dinero para una meta, para ver el progreso subir y que ese dinero deje de contar como disponible.

**Acceptance Scenarios**:

1. **Given** una meta activa, **When** registro un apartado de $200.000 hoy, **Then** el progreso sube a $200.000 de $2.000.000 y el apartado aparece en el historial como «Apartado · Viaje a la costa» — NO como gasto.
2. **Given** un apartado registrado, **When** veo el Resumen, **Then** los totales de ingresos/gastos NO lo incluyen (no infla gastos).
3. **Given** un apartado en la quincena actual, **When** veo Planificación, **Then** aparece el componente «Apartado a metas» restando del libre de la quincena y del libre real.
4. **Given** un apartado con monto ≤ 0 o fecha futura, **When** intento guardarlo, **Then** se rechaza con mensaje.
5. **Given** la meta llega a su objetivo con un apartado, **When** se confirma el apartado, **Then** la meta pasa a «completada» y deja de aceptar apartados nuevos (se felicita en la UI).

### User Story 3 — Corregir apartados (P2)

Como usuario, quiero editar o eliminar un apartado mal registrado, para que el progreso de la meta sea confiable.

**Acceptance Scenarios**:

1. **Given** un apartado, **When** lo edito (monto o fecha), **Then** el progreso de la meta se recalcula.
2. **Given** un apartado, **When** lo elimino tras confirmación, **Then** desaparece del historial y el progreso baja.
3. **Given** un apartado de una meta completada, **When** lo elimino y el acumulado queda por debajo del objetivo, **Then** la meta vuelve a activa.

### User Story 4 — Cerrar una meta (P2)

Como usuario, quiero completar o cancelar una meta, para tener la lista al día.

**Acceptance Scenarios**:

1. **Given** una meta con acumulado ≥ objetivo, **When** se registra el apartado que la alcanza, **Then** se marca completada automáticamente (C-no requerida).
2. **Given** una meta activa o completada, **When** la cancelo, **Then** se pide confirmación y queda archivada como cancelada; el dinero apartado queda como está, sin devolución (C4).
3. **Given** una meta cancelada o completada, **When** veo la pestaña, **Then** aparece en la sección de cerradas, sin aceptar apartados nuevos.

### Edge Cases

- Apartado mayor que el faltante de la meta: se acepta (puede sobrar); el progreso muestra el acumulado real y la meta completa.
- Meta cancelada con apartados y luego reactivada (si se permite reactivar): el acumulado se conserva. [PROPUESTO: permitir reactivar metas canceladas]
- Fecha objetivo sin apartados: ritmo «atrasado» no debe asustar — el indicador solo aparece con al menos un apartado.
- Cambiar el monto objetivo de una meta ya completada hacia arriba: la meta vuelve a activa si el acumulado queda debajo.
- Apartados el mismo día: orden estable por hora de registro (como movimientos).
- La persistencia sigue el estándar: cerrar y reabrir conserva metas y apartados.

## Requirements

- **FR-M01**: El sistema DEBE permitir crear/editar/eliminar una meta con: nombre, monto objetivo (> 0), fecha objetivo opcional (futura). [APROBADO — C3]
- **FR-M02**: El sistema DEBE registrar apartados como tipo propio de movimiento («aporte»): monto > 0, fecha ≤ hoy, meta obligatoria. Los apartados NO cuentan como gasto ni ingreso en el Resumen ni en el Balance registrado. [APROBADO — C1]
- **FR-M03**: El historial DEBE mostrar los apartados con su tipo visible («Apartado») y el nombre de la meta. [APROBADO — C1]
- **FR-M04**: El progreso de una meta es la suma de sus apartados vigentes; al alcanzar el objetivo la meta pasa a completada automáticamente y no acepta apartados nuevos. [APROBADO]
- **FR-M05**: El sistema DEBE permitir editar y eliminar apartados (con confirmación en el borrado), recalculando el progreso y el estado de la meta. [APROBADO]
- **FR-M06**: Cancelar una meta requiere confirmación y solo la archiva; sin movimiento de devolución. Reactivar una meta cancelada conserva su acumulado. [APROBADO — C4; reactivación PROPUESTO]
- **FR-M07**: Planificación DEBE incluir el componente «Apartado a metas» del período, restando del «Libre de la quincena» y del «Libre real», listado en el desglose. [APROBADO — C2]
- **FR-M08**: Con fecha objetivo y al menos un apartado, la meta muestra un indicador simple de ritmo (a ritmo / atrasado). [APROBADO — C3; cálculo del ritmo PROPUESTO: acumulado ≥ objetivo × días transcurridos / días totales]
- **FR-M09**: Vista derivada para metas y progreso; los apartados son movimientos reales (editables como tales); el modelo de ingresos y gastos de la 001 NO cambia de comportamiento. [APROBADO]

## Exclusiones explícitas

- Interpretación por mensaje del apartado («aparté 200k para el viaje» en el registro rápido): POSPUESTO; v1 registra apartados desde la pestaña Metas.
- Interés o rendimiento del ahorro: fuera (no hay cuentas bancarias).
- Múltiples «bolsillos» por meta (sub-metas): fuera.
- Transferir apartados entre metas sin eliminar/recrear: POSPUESTO.
- Notificaciones de metas: fuera de v1.

## Key Entities

- **Meta**: id, nombre, monto objetivo (COP), fecha objetivo opcional, estado (activa / completada / cancelada), creado_en, actualizado_en.
- **Apartado**: movimiento de tipo «aporte» con meta asociada, monto y fecha; el vínculo meta↔apartado permite recalcular progreso y editar como cualquier movimiento.
