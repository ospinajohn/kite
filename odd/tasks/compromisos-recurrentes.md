# Feature: Compromisos recurrentes y recordatorios de pago

Spec del producto: `specs/002-compromisos-recurrentes/spec.md`. Implementación Tauri v2 + Rust + SQLite
siguiendo la arquitectura existente (ports & adapters, tests primero). E6 de empaquetado queda fuera de
esta feature (diferida al cierre del roadmap).

## Specs

- S1: «Compromisos recurrentes (arriendo, suscripciones, cuotas) + recordatorios de pago» — petición original del líder (2026-11-12).
- S2: C1 — «Recordar y confirmar»: al llegar la fecha de vencimiento, Kite avisa y ofrece registrar el gasto con monto y concepto precargados; el usuario confirma, corrige o cancela. Nada se guarda automáticamente.
- S3: C2 — Frecuencias: «Mensual» (día fijo del mes) y «Quincenal» (primera y/o segunda quincena del mes). Semanal y anual fuera de esta versión.
- S4: C3 — «Monto con corrección»: el compromiso tiene un monto base; al confirmar el pago puede corregirse el monto para ese pago puntual. La corrección aplica solo a ese pago.
- S5: C4 — «Aviso + notificación de Windows»: aviso al abrir la app (vencidos/por vencer + badge en la pestaña) y notificación de Windows el día del vencimiento si la app está corriendo.
- S6: Coherencia con la 001: los compromisos no cuentan en historial ni resumen hasta confirmarse el pago; fecha del pago precargada en hoy, editable, máximo hoy; eliminación de compromiso pide confirmación.
- S7: C5 — «en la 1 puede ser configurable? por ejemplo que se pueda poner fecha fin, o se pueda poner cuotas o establecer como indefinido, que al momento de crear se pueda poner una de esas opciones mas no todas al mismo tiempo una maximo» — duración configurable, una sola opción al crear. Ventana «Próximos» de 7 días: «lo que recomiendas me parece perfecto».

## Tasks

- T1 [S2-S5] Redactar y someter a aprobación `specs/002-compromisos-recurrentes/spec.md` — commit docs.
- T2 [S1-S7] Plan técnico: research breve (notificaciones Tauri v2 en Windows, manejo de fechas de recurrencia), data-model (migración 002: tabla compromisos + compromisos_pagos), contratos de comandos Tauri.
- T3 [S1,S3,S7] Modelo `Compromiso` + validaciones + duración (indefinido / fecha fin / cuotas N≥1, excluyentes) + cálculo de próximos vencimientos (mensual con día 29-31 en meses cortos, quincenal, ventana 7 días) — unit tests primero.
- T4 [S2,S4,S6] Storage: `CompromisosPort` + implementación SQLite (CRUD, pausar, deduplicación de pago por período) — contract tests.
- T5 [S2,S4,S6] AppCommand: crear/editar/pausar/eliminar compromiso, listar vencimientos (Atrasados/Hoy/Próximos), proponer pago, confirmar pago → movimiento gasto, deduplicación — integration tests.
- T6 [S5,S6] UI: sección Compromisos (lista + estado vacío), badge de pendientes, modal de confirmación de pago con monto editable, confirmación de eliminación — en `src-tauri/dist/index.html` siguiendo el sistema visual existente.
- T7 [S5] Notificación de Windows el día del vencimiento (Tauri v2 notification plugin), mientras la app corre — prueba manual en Windows.
- T8 [S1-S6] Actualizar README (uso de compromisos) + ROADMAP (estado). Bench/manual: ESC del quickstart 002.

## Log

- L1 (2026-11-12): «me gustaria hacer estas, cual seria al plan» + lista de 5 features (verbatím en ROADMAP). Eligió empezar por Compromisos recurrentes. E6 de empaquetado se difiere al final.
- L2 (2026-11-12): Sesión de clarificación aprobó C1–C4 (S2–S5).
- L3 (2026-11-12): C5 aprobado (S7): duración configurable — indefinido / fecha fin / cuotas, una sola opción al crear; ventana Próximos 7 días aprobada. Spec 002 queda Aprobado.
