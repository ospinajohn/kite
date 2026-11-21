# Feature: Evaluación del impacto de nuevas deudas

Spec del producto: `specs/006-evaluacion-impacto-deudas/spec.md`.
Plan técnico: `specs/006-evaluacion-impacto-deudas/plan.md`. Análisis sobre 002–005; sin
persistencia (simulador).

## Specs

- S1: «empezar con la espec 006 por favor» — petición original (2026-11-20). Roadmap: «Evaluación del impacto de nuevas deudas — análisis sobre 002–005».
- S2: C1 — Solo simulador, no persiste: sin crear compromisos, movimientos, metas ni configuración. [APROBADO 2026-11-20]
- S3: C2 — Nueva deuda = deuda por pagar de tipo cuotas/monto fijo, similar a compromiso/salida futura (002). [APROBADO 2026-11-20]
- S4: C3 — Mostrar impacto en «Libre de la quincena» y «Libre real» (003), conflicto con compromisos existentes (002) e impacto en metas de ahorro (005). [APROBADO 2026-11-20]
- S5: C4 — Frecuencias mensual y quincenal (como 002). [APROBADO 2026-11-20]
- S6: C5 — Duración excluyente: cuotas (N ≥ 1), fecha fin o indefinida + fecha/inicio del primer vencimiento. [APROBADO 2026-11-20]
- S7: C6 — Inicio pasado: proyectar desde la quincena actual en adelante, ignorando vencimientos pasados, con supuesto explícito. [APROBADO 2026-11-20]
- S8: C7 — Horizonte: hasta último vencimiento simulado (finita); si indefinida, el usuario define cuántas quincenas proyectar (máximo razonable). [APROBADO 2026-11-20]
- S9: C8 — Impacto en metas: faltante/libre restante y señal de ritmo afectado/en riesgo cuando haya fecha objetivo y ritmo calculable. [APROBADO 2026-11-20]
- S10: C9 — Acceso desde entrada en Planificación que abre una vista dedicada. [APROBADO 2026-11-20]
- S11: FR-D02/D07/D08 — Sin persistencia ni mutación de datos reales; etiquetas «simulado/escenario/estimado»; cálculo sobre estado real vigente.

## Tasks

- T1 [S1-S4,S11] Spec y plan en borrador con C1–C3 y puntos [PROPUESTO] explícitos. ✓
- T2 [S5-S10] Confirmar decisiones [PROPUESTO] con el líder (frecuencias, duración/inicio, inicio pasado, horizonte, indicador metas, ubicación UI) y actualizar spec/plan. ✓
- T3 [S3,S5-S8,S11] Modelo/cálculo puro efímero: generación de vencimientos simulados y validaciones (alineado a 002/003) — tests. ✓ (14 tests en `src-tauri/tests/test_simulacion_deuda.rs`; módulo `src-tauri/src/simulacion_deuda/`)
- T4 [S4,S8,S11] Cálculo de escenario por quincena: libre base vs simulado, deltas y conflictos con compromisos — tests con datos 002/003. ✓ (5 tests en `src-tauri/tests/test_simulacion_escenario.rs`; `calcular_escenario`/`simular_impacto` en `simulacion_deuda`)
- T5 [S4,S9,S11] Indicador de impacto en metas activas sobre el escenario — tests con datos 005. ✓ (`ImpactoMeta`/`calcular_impacto_metas`: faltante, libre simulado de referencia, ritmo afectado; 4 tests en `test_simulacion_escenario.rs`)
- T6 [S2,S4,S10,S11] UI de simulación (entrada en Planificación → vista dedicada: formulario, resultado por quincenas, conflictos, metas, limpiar) con etiquetas «simulado» — sin persistencia. ✓ (modal `modal-simulador-deuda`, entrada en Planificación, JS `simular_impacto_deuda`)
- T7 [S1-S11] Regresión 001–005, README/ROADMAP/cierre y validación manual del líder registrada. ✓ (`cargo test --workspace` verde; `cargo clippy --workspace -- -D warnings` verde; README/ROADMAP actualizados). Validación manual (`cargo tauri dev`) pendiente por el líder.

## Log

- L1 (2026-11-20): «empezar con la espec 006 por favor»
- L2 (2026-11-20): C1–C3 aprobadas — simulador sin persistencia; deuda por pagar tipo cuotas/monto fijo; impacto en libre 003, conflictos 002 y metas 005. Borrador inicial de spec/plan/tasks.
- L3 (2026-11-20): C4–C9 aprobadas — frecuencias mensual/quincenal; duración cuotas, fecha fin o indefinida + primer vencimiento; inicio pasado se proyecta desde quincena actual; horizonte hasta último vencimiento (indefinida: usuario define quincenas, máximo razonable); metas con faltante/libre restante y señal de ritmo afectado; acceso desde Planificación a vista dedicada. Spec/plan/odd actualizados (T2 ✓).
- L4 (2026-11-20): T3 implementada — `src-tauri/src/simulacion_deuda/{mod,service}.rs` (validación, generación de vencimientos con C6/C7, `MAX_QUINCENAS_PROYECCION=120`) + `src-tauri/tests/test_simulacion_deuda.rs` (14/14 en verde). Módulo registrado en `src-tauri/src/lib.rs`.
- L5 (2026-11-20): T4 ✓ — escenario por quincena (base 003 vs simulado, deltas, conflicto con compromisos 002) y ensamblado `simular_impacto` (sin persistencia). Se expuso `Compromiso::vencimientos_hasta_fecha` para horizonte futuro. 5 tests en `src-tauri/tests/test_simulacion_escenario.rs`.
- L6 (2026-11-20): T5 ✓ — impacto en metas activas (`ImpactoMeta`: faltante/libre simulado de referencia y `ritmo_afectado` cuando aplica, C8). Integrado en `ResultadoSimulacion`. 4 tests adicionales verdes. (T3–T5: 14+9 tests de simulación en verde).
- L7 (2026-11-20): T6 ✓ — comando `simular_impacto_deuda` (Tauri) y UI en `src-tauri/dist/index.html` (entrada desde Planificación, vista/modal dedicada, formulario, resultado por quincenas/conflictos/metas, limpiar, etiquetas de escenario simulado). T7 ✓ — regresión `cargo test --workspace` y clippy `-D warnings` en verde; README y ROADMAP actualizados. Validación manual con `cargo tauri dev` queda pendiente por el líder.
