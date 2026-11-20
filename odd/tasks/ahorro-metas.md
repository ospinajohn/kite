# Feature: Ahorro y metas

Spec del producto: `specs/005-ahorro-metas/spec.md`. La 004 (préstamos) quedó POSPUESTA por
decisión del líder (2026-11-12); esta es la feature siguiente.

## Specs

- S1: «Ahorro y metas» — petición original (2026-11-12).
- S2: C1 — Apartado como tipo propio de movimiento: no cuenta como gasto ni ingreso en historial/resumen; se muestra aparte.
- S3: C2 — Los apartados restan del dinero libre (componente separado en Planificación, quincena y real).
- S4: C3 — Fecha objetivo opcional (futura); con fecha y al menos un apartado, indicador de ritmo a ritmo/atrasado.
- S5: C4 — Cancelar una meta solo archiva (confirmación en UI), sin devolución; reactivar conserva el acumulado [PROPUESTO].
- S6: FR-M04/M05 — Progreso = suma de apartados vigentes; meta completada automática al alcanzar objetivo y sin apartados nuevos; editar/eliminar apartado recalcula progreso y estado.
- S7: FR-M09 — Comportamiento de ingresos/gastos de la 001 intacto; migración 004 reconstruye movements (CHECK + meta_id) conservando los datos exactos.

## Tasks

- T1 [S1-S5] Spec + plan escritos con decisiones C1–C4. ✓
- T2 [S7] Migración 004 (reconstrucción de movements + tabla metas) con test de conservación de datos pre-poblados — RIESGO ALTO, commit propio.
- T3 [S2,S6] Modelo: MovementType::Aporte + meta_id + validaciones + recalcular_estado puro — unit tests.
- T4 [S2,S6] MetasPort SQLite (CRUD, estados, progreso) + comandos apartados — contract + integration tests.
- T5 [S3] Planificación: componente «Apartado a metas» restando del libre — tests de regresión del cálculo.
- T6 [S1-S6] UI: pestaña Metas (progreso, cerradas, modales), historial con tipo Apartado, desglose en Planificación — node --check.
- T7 [S1-S6] README + ROADMAP + cierre; validación manual del líder registrada como pendiente.

## Log

- L1 (2026-11-12): «elimina la spec 004, no me interesa por esto tiempos, prifiero continuar con la 005» — 004 POSPUESTA (`d67613f`).
- L2 (2026-11-12): C1–C4 aprobadas: apartado tipo propio; resta del libre; fecha objetivo opcional; cancelar = solo archivar.
