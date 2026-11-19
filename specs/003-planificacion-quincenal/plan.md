# Implementation Plan: Planificación por quincena y dinero libre

**Branch**: `experimento/ui-ux` | **Date**: 2026-11-12
**Spec**: [spec.md](spec.md) | **Predecessor**: feature 002 (compromisos, NO modificar sus módulos)

## Resumen

Vista derivada «Planificación» por quincena (corte configurable 1–28, default 15): ingresos, gastos,
compromisos pendientes del período, «Libre de la quincena» y «Libre real» (con atrasados). Pestaña nueva
con navegación anterior/siguiente/hoy + tarjeta compacta en Dashboard. Persistencia mínima: día de corte.

## Decisiones técnicas

1. **Configuración**: tabla `configuracion(clave TEXT PK, valor TEXT NOT NULL)` vía migración 003
   (`user_version=3`). Solo `dia_corte`. Nuevo puerto `ConfigPort` con `get/set`; implementación SQLite
   en `src-tauri/src/config/`. No se reusa `compromisos` ni se toca las tablas existentes.
2. **Cálculo puro**: `src-tauri/src/planificacion/service.rs` — funciones puras que reciben
   `(dia_corte, fecha_desde, fecha_hasta, movimientos, vencimientos_pendientes)` y devuelven
   `PlanQuincena`. 100% testeable sin DB.
   - Quincena de una fecha: si `dia <= corte` → (1, corte); si no → (corte+1, último día del mes).
   - Atrasados: vencimientos con `fecha < desde` sin pagar, deduplicados por compromiso (el más
     reciente no pagado, igual que la 002).
   - `libre_quincena = ingresos - gastos - pendientes_del_periodo`;
     `libre_real = libre_quincena - atrasados`.
3. **Comando Tauri**: `get_planificacion(desde?, hasta?)` — sin argumentos usa la quincena en curso;
   con rango explícito calcula la quincena que lo contiene (la UI navega moviendo el rango).
   Lee: `StoragePort.query(desde, hasta)` + `calcular_vencimientos` + `ConfigPort.get(dia_corte)`.
   Devuelve `PlanQuincena` serializable con componentes para el desglose en la UI.
4. **UI** (`src-tauri/dist/index.html`): pestaña «Planificación» en la isla, panel con navegación
   (‹ › y «Hoy»), tarjetas «Libre de la quincena» / «Libre real» con desglose expandible de
   componentes, editor del día de corte, y tarjeta compacta en Dashboard que llama `switchTab`.
   Rótulos FR-P08: nota «según lo registrado», nunca «saldo».
5. **Tests primero**: unit del cálculo puro (escenarios del spec: caso 700k, atrasado 100k → 600k,
   pagado no descuenta, pausado/agotado no descuenta, febrero corte 28, negativos con signo) →
   integration del comando (con SQLite real, migración 003, persistencia del corte) → `node --check`
   del JS.

## Estructura

```
src-tauri/src/
├── config/mod.rs          # ConfigPort + SqliteConfigStorage
├── planificacion/mod.rs   # re-exports
└── planificacion/service.rs  # cálculo puro de PlanQuincena
migrations/003_configuracion.sql
src-tauri/tests/test_planificacion.rs
```

## Criterios de cierre

- Suite completa verde + clippy `-D warnings`.
- ESC de planificación validados por los tests automáticos (casos del spec).
- Validación manual del líder (cargo tauri dev) queda registrada como pendiente, igual que la 002.
- README + ROADMAP actualizados.
