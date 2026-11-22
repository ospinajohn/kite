# Feature Specification: Evaluación del impacto de nuevas deudas

**Feature Branch**: `experimento/ui-ux` (work-units propios)
**Created**: 2026-11-20
**Status**: Aprobado (borrador resuelto) — decisiones C1–C9 acordadas
**Predecessors**: `002-compromisos-recurrentes`, `003-planificacion-quincenal`, `005-ahorro-metas` (no se modifican; esta feature consume su información para el análisis)

**Input**: User request: «empezar con la espec 006 por favor» (2026-11-20). Roadmap: «Evaluación del impacto de nuevas deudas — análisis sobre 002–005».

---

## Decisiones de la sesión de clarificación (2026-11-20)

- **C1 — Solo simulador, no persiste.** La evaluación es un «qué pasaría si»: calcula y muestra el impacto de una deuda hipotética sin crear compromisos, movimientos, metas ni configuración. [APROBADO]
- **C2 — Nueva deuda = deuda por pagar de tipo cuotas/monto fijo.** Se modela como una salida futura similar a un compromiso (monto, frecuencia y duración), comparable con los compromisos de la 002. [APROBADO]
- **C3 — Impacto a mostrar.** La evaluación debe mostrar: efecto sobre «Libre de la quincena» y «Libre real» (003), conflicto/competencia con compromisos existentes (002) e impacto en metas de ahorro (005). [APROBADO]
- **C4 — Frecuencias.** La deuda simulada acepta frecuencias mensual y quincenal, alineadas a 002. [APROBADO]
- **C5 — Duración e inicio.** La duración se define con una sola opción excluyente: cuotas (N ≥ 1), fecha fin o indefinida, junto con la fecha/inicio del primer vencimiento (alineado a 002). [APROBADO]
- **C6 — Inicio en el pasado.** Si el inicio/primer vencimiento queda en el pasado, el simulador proyecta desde la quincena actual en adelante, ignorando vencimientos ya pasados y dejando explícito el supuesto. [APROBADO]
- **C7 — Horizonte.** Para deudas con cuotas o fecha fin, el análisis cubre hasta la quincena del último vencimiento simulado. Para deudas indefinidas (sin último vencimiento), el usuario define cuántas quincenas proyectar al simular (con un máximo razonable). [APROBADO]
- **C8 — Impacto en metas.** Se muestran ambos indicadores: faltante/libre restante por meta activa y señal de ritmo afectado/en riesgo cuando haya fecha objetivo y ritmo calculable. [APROBADO]
- **C9 — Ubicación UI.** El simulador se accede desde una entrada en Planificación que abre una vista dedicada. [APROBADO]

---

## User Scenarios & Testing

### User Story 1 — Simular una nueva deuda y ver el impacto en mi dinero libre (P1)

Como usuario, quiero ingresar los datos de una deuda que estoy considerando (monto, frecuencia, inicio y duración) para ver cómo cambiarían mi «Libre de la quincena» y mi «Libre real» en el horizonte simulado, sin guardar nada.

**Acceptance Scenarios**:

1. **Given** la quincena actual con datos calculados por Planificación (003), **When** simulo una deuda mensual de $200.000 que inicia en la quincena en curso, **Then** veo, para cada quincena del horizonte simulado, el libre base y el libre con la deuda aplicada, con la diferencia visible.
2. **Given** una simulación, **When** un vencimiento simulado cae dentro de una quincena, **Then** ese monto se descuenta del «Libre de la quincena» y del «Libre real» de ese período en el escenario simulado (sin alterar los valores reales).
3. **Given** una simulación, **When** el libre simulado de una quincena queda negativo, **Then** se muestra con signo «−» y se resalta como período en riesgo.
4. **Given** que cierro la simulación o salgo de la vista dedicada, **When** vuelvo a Planificación/Metas/Compromisos, **Then** no existe ningún registro nuevo ni cambio persistido derivado de la simulación.
5. **Given** la simulación, **When** cambio cualquier dato de entrada (monto, frecuencia, inicio, duración u horizonte cuando aplique), **Then** el resultado se recalcula con los nuevos valores.

### User Story 2 — Detectar conflicto con compromisos existentes (P1)

Como usuario, quiero saber si la nueva deuda compite con mis compromisos ya registrados, para evitar asumir un pago que me deje sin cubrir lo que ya debo.

**Acceptance Scenarios**:

1. **Given** compromisos activos/pendientes en una quincena (002) y una deuda simulada con vencimiento en ese mismo período, **When** veo el resultado, **Then** el detalle del período muestra los compromisos del período y el vencimiento simulado como componentes separados, indicando que compiten por el mismo dinero libre.
2. **Given** una quincena donde el libre real simulado no alcanza para cubrir los compromisos pendientes del período más la cuota simulada, **When** veo el resultado, **Then** el período se marca con conflicto/alerta visible (ej. «no alcanza para cubrir compromisos y la deuda simulada»).
3. **Given** compromisos atrasados que ya afectan el «Libre real» base (003), **When** simulo la deuda, **Then** el impacto simulado se calcula sobre ese libre real base (no los oculta ni los elimina del análisis).

### User Story 3 — Ver impacto en metas de ahorro (P2)

Como usuario, quiero saber si asumir la nueva deuda afecta mi capacidad de seguir apartando a mis metas, para decidir sin desatender mis objetivos.

**Acceptance Scenarios**:

1. **Given** metas activas (005), **When** simulo una deuda, **Then** el resultado incluye una sección de impacto en metas que muestra, por meta activa, su faltante/progreso y el libre simulado disponible en los períodos analizados (indicador informativo), sin crear ni modificar apartados.
2. **Given** una meta activa con fecha objetivo y ritmo calculable (005, FR-M08), **When** el libre simulado de los períodos relevantes no permite sostener el ritmo necesario, **Then** la meta se señala como «en riesgo» o «ritmo afectado» en el escenario simulado.
3. **Given** una meta sin fecha objetivo o sin indicador de ritmo, **When** simulo la deuda, **Then** se muestra el impacto disponible (faltante/libre restante) sin inventar un ritmo.
4. **Given** no hay metas activas, **When** simulo la deuda, **Then** la sección de metas muestra un estado vacío informativo, sin error.

### User Story 4 — Definir y ajustar la deuda hipotética (P1)

Como usuario, quiero describir la deuda que estoy evaluando con datos claros, para que la simulación refleje el caso que tengo en mente.

**Acceptance Scenarios**:

1. **Given** la vista dedicada al simulador (accedida desde Planificación), **When** ingreso concepto (opcional), monto por cuota (> 0), frecuencia (mensual/quincenal), fecha/inicio del primer vencimiento y duración (cuotas, fecha fin o indefinida), **Then** puedo ejecutar la simulación.
2. **Given** un monto ≤ 0, una duración inválida (ej. N < 1, fecha fin inconsistente) o un inicio no interpretable, **When** intento simular, **Then** no se calcula y se indica qué dato falta o es inválido.
3. **Given** una deuda por N cuotas o con fecha fin, **When** simulo, **Then** el horizonte cubre desde el inicio relevante hasta la quincena del último vencimiento simulado.
4. **Given** una deuda con duración indefinida, **When** simulo, **Then** debo indicar cuántas quincenas proyectar y el análisis cubre ese horizonte (sujeto a un máximo razonable definido por la implementación).
5. **Given** un inicio/primer vencimiento en el pasado, **When** simulo, **Then** la proyección se realiza desde la quincena actual en adelante ignorando vencimientos pasados, y la UI indica explícitamente ese supuesto.
6. **Given** una simulación ya calculada, **When** quiero probar otro caso, **Then** puedo limpiar o reemplazar los datos y recalcular sin efectos residuales.

### Edge Cases

- Inicio/primer vencimiento pasado: se simula desde la quincena actual en adelante (C6); no se crean pagos históricos ni se reescriben períodos pasados.
- Frecuencia quincenal: la asignación de vencimientos simulados a quincenas sigue la fecha de vencimiento real y el día de corte configurado (003), igual que la lógica de planificación/compromisos.
- Vencimiento simulado el día 29/30/31 en meses cortos: se ajusta al último día del mes, coherente con 002.
- Sin ingresos/gastos/compromisos registrados: la simulación se calcula sobre $0 y muestra el impacto (posiblemente negativo) sin error.
- Múltiples vencimientos simulados en la misma quincena (según definición): se suman en el componente «deuda simulada» del período.
- El simulador NO debe ofrecer ni ejecutar registro de pago, creación de compromiso, creación de meta ni edición de datos reales.
- Libre simulado en $0 exacto: se muestra en $0; el conflicto se evalúa por insuficiencia para cubrir componentes comprometidos del período, no solo por signo negativo.
- Deuda indefinida con horizonte muy largo: se aplica un máximo razonable de quincenas a proyectar para no degradar la experiencia; el límite debe ser claro para el usuario.

## Requirements

- **FR-D01**: El sistema DEBE permitir ingresar una deuda hipotética con concepto opcional, monto por cuota (> 0), frecuencia (mensual o quincenal), fecha/inicio del primer vencimiento y duración (una sola: cuotas N ≥ 1, fecha fin o indefinida), y calcular su calendario de vencimientos simulado. [APROBADO — C2, C4, C5]
- **FR-D02**: La simulación NO DEBE persistir datos ni modificar movimientos, compromisos, metas, apartados ni configuración existentes. [APROBADO — C1]
- **FR-D03**: Para cada quincena del horizonte analizado, el sistema DEBE mostrar el «Libre de la quincena» y el «Libre real» base (según 003) y los valores resultantes al aplicar la deuda simulada, con la diferencia/impacto. [APROBADO — C3]
- **FR-D04**: El sistema DEBE identificar y mostrar conflicto con compromisos existentes (002) cuando la deuda simulada compita con ellos en un período o cuando el escenario simulado deje insuficiente el libre para cubrir los compromisos pendientes del período y la cuota simulada. [APROBADO — C3]
- **FR-D05**: El sistema DEBE mostrar el impacto en metas activas (005) con ambos indicadores: faltante/libre restante por meta y señal de ritmo afectado/en riesgo cuando haya fecha objetivo y ritmo calculable; sin metas activas debe mostrar estado vacío informativo. [APROBADO — C3, C8]
- **FR-D06**: El sistema DEBE recalcular la simulación al cambiar los datos de entrada y permitir limpiar/reiniciar el escenario sin efectos residuales. [APROBADO]
- **FR-D07**: Los rótulos del escenario simulado DEBEN diferenciarse claramente de los datos reales (ej. «simulado», «escenario») y mantener la nota de estimado según lo registrado; nunca presentar el resultado como saldo garantizado. [APROBADO — coherencia con 001/003]
- **FR-D08**: El cálculo del escenario simulado DEBE basarse en el estado real vigente al momento de simular: movimientos registrados, configuración (día de corte), compromisos/vencimientos pendientes y metas/apartados actuales. [APROBADO]
- **FR-D09**: El horizonte DEBE cubrir hasta la quincena del último vencimiento simulado para duraciones finitas (cuotas o fecha fin). Para duración indefinida, el usuario DEBE poder definir cuántas quincenas proyectar y el sistema DEBE aplicar un máximo razonable; si el inicio es pasado, la proyección inicia en la quincena actual con supuesto explícito. [APROBADO — C6, C7]
- **FR-D10**: El simulador DEBE ser accesible desde una entrada en Planificación que abra una vista dedicada. [APROBADO — C9]

## Exclusiones explícitas

- Persistir escenarios, comparar varios escenarios guardados o compartir simulaciones: fuera de esta versión (C1).
- Crear, confirmar o modificar compromisos/deudas reales desde el simulador: fuera de esta versión.
- Considerar préstamos por cobrar / ingresos esperados de la feature 004 (eliminada): fuera.
- Tasas de interés, costo financiero total, tabla de amortización bancaria o proyección de inflación: fuera.
- Recomendación automática definitiva «te conviene / no te conviene»: fuera; se muestran datos e indicadores para que el usuario decida.
- Modificar Resumen, Balance registrado (001), lógica de compromisos (002), Planificación real (003) o metas/apartados (005).

## Key Entities

- **Deuda simulada** (efímera, no persistida): concepto opcional, monto por cuota (COP), frecuencia (mensual/quincenal), fecha/inicio del primer vencimiento, duración (cuotas N ≥ 1, fecha fin o indefinida) y, si es indefinida, cantidad de quincenas a proyectar; calendario de vencimientos simulados derivado.
- **Resultado de simulación** (vista derivada, no persistida): por quincena del horizonte, libre base (003), componentes simulados aplicados, libre simulado, señales de conflicto con compromisos (002) e indicadores de impacto en metas activas (005: faltante/libre restante y señal de ritmo afectado cuando aplique).

## Decisiones

| Decisión | Estado |
| --- | --- |
| Modo simulador sin persistencia | [APROBADO — C1, 2026-11-20] |
| Deuda por pagar tipo cuotas/monto fijo (similar a compromiso) | [APROBADO — C2, 2026-11-20] |
| Mostrar impacto en libre 003, conflicto con compromisos 002 e impacto en metas 005 | [APROBADO — C3, 2026-11-20] |
| Frecuencias soportadas: mensual y quincenal | [APROBADO — C4, 2026-11-20] |
| Duración/inicio: cuotas, fecha fin o indefinida + primer vencimiento (excluyentes) | [APROBADO — C5, 2026-11-20] |
| Inicio pasado: simular desde la quincena actual en adelante (supuesto explícito) | [APROBADO — C6, 2026-11-20] |
| Horizonte: hasta último vencimiento; indefinida → usuario define quincenas a proyectar (máximo razonable) | [APROBADO — C7, 2026-11-20] |
| Impacto en metas: faltante/libre restante y señal de ritmo afectado cuando aplique | [APROBADO — C8, 2026-11-20] |
| Ubicación UI: entrada en Planificación que abre vista dedicada | [APROBADO — C9, 2026-11-20] |
