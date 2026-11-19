# Feature: Planificación por quincena y dinero libre

Spec del producto: `specs/003-planificacion-quincenal/spec.md`. Implementación Tauri v2 + Rust + SQLite.
NO modificar módulos de compromisos (002) ni el modelo de movimientos (001); solo consumirlos.

## Specs

- S1: «Planificación por quincena y cálculo de dinero libre» — petición original (2026-11-12).
- S2: C1 — Quincena con día de corte configurable (1–28, default 15): primera = 1..D, segunda = D+1..fin de mes.
- S3: C2 — Dos números: «Libre de la quincena» (ingresos − gastos − compromisos no pagados del período) y «Libre real» (además atrasados de quincenas anteriores), ambos visibles con componentes.
- S4: C3 — Pestaña «Planificación» nueva + tarjeta compacta en Dashboard que enlaza a ella.
- S5: C4 — Abre en la quincena en curso; navegación anterior/siguiente/hoy.
- S6: FR-P04 — Compromisos asignados por fecha de vencimiento real; pagados/pausados/agotados no descuentan.
- S7: FR-P08/09 — Rótulos «Libre de la quincena»/«Libre real» con nota «según lo registrado»; nunca «saldo»; vista derivada que no crea registros; el «Balance registrado» de la 001 no cambia.

## Tasks

- T1 [S1-S7] Spec + plan técnico escritos y aprobados (C1–C4). ✓
- T2 [S2] `config` port + SQLite + migración 003 (`dia_corte`, default 15) — contract tests.
- T3 [S3,S6] Cálculo puro `PlanQuincena` en `planificacion/service.rs` — unit tests primero (casos del spec: 700k, atrasado 100k→600k, pagado no descuenta, pausado/agotado, febrero corte 28, negativos).
- T4 [S3,S5] Comando `get_planificacion(desde?, hasta?)` + navegación de quincena en el servicio — integration tests.
- T5 [S4,S5,S7] UI: pestaña Planificación con navegación y desglose, editor del corte, tarjeta Dashboard — `node --check`.
- T6 [S1-S7] README + ROADMAP + cierre del documento. Validación manual del líder registrada como pendiente.

## Log

- L1 (2026-11-12): Feature elegida tras posponer la validación manual de la 002 (revisión nativa pendiente por saldo del proveedor). El líder eligió: corte configurable, ambos números, pestaña + tarjeta dashboard, actual + navegación.
- L2 (2026-11-12): C1–C4 aprobadas vía sesión de clarificación; spec y plan escritos.
