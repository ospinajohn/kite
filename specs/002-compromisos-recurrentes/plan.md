# Implementation Plan: Compromisos recurrentes y recordatorios de pago

**Branch**: `experimento/ui-ux` (continúa; commits de work-unit por tarea) | **Date**: 2026-11-12
**Spec**: [spec.md](spec.md) | **Tasks**: `odd/tasks/compromisos-recurrentes.md`

## Resumen

Entidad `Compromiso` (concepto, monto base COP, frecuencia mensual/quincenal, duración
indefinido/fecha fin/cuotas, activo/pausado) persistida en SQLite (migración 002), con cálculo de
próximos vencimientos (Atrasados / Vencen hoy / Próximos 7 días), flujo «Registrar pago» que crea un
movimiento gasto con confirmación explícita, deduplicación de pago por período, badge de pendientes y
notificación de Windows vía plugin oficial de Tauri v2. Los compromisos no afectan historial ni
resumen hasta que su pago se confirma (coherencia con la 001).

## Decisiones técnicas

1. **Notificaciones**: plugin oficial `tauri-plugin-notification` v2. La notificación se dispara en
   el arranque y al marcar el día (la app corre de forma continua; no hay scheduler externo).
2. **Recurrencia**: cálculo puro en Rust (`models/compromiso.rs`), testeado sin dependencia de DB.
   - Mensual día N: clamp al último día del mes (día 31 → 30 en abril).
   - Quincenal: primera quincena → vence el día 15; segunda → último día del mes. [PROPUESTO, validar con uso]
   - Clave de período para deduplicación: `YYYY-MM` (mensual), `YYYY-MM-Q1` / `YYYY-MM-Q2` (quincenal).
3. **Migración 002** (`migrations/002_compromisos.sql`, `user_version=2`):
   - `compromisos(id TEXT PK, concepto TEXT NOT NULL, monto_centavos INTEGER NOT NULL CHECK(monto_centavos>0),
     frecuencia TEXT NOT NULL CHECK(frecuencia IN ('mensual','quincenal')),
     dia_mes INTEGER CHECK(dia_mes BETWEEN 1 AND 31), quincena TEXT CHECK(quincena IN ('primera','segunda')),
     duracion_tipo TEXT NOT NULL CHECK(duracion_tipo IN ('indefinido','fecha_fin','cuotas')),
     fecha_fin TEXT, cuotas_total INTEGER CHECK(cuotas_total IS NULL OR cuotas_total>=1),
     cuotas_confirmadas INTEGER NOT NULL DEFAULT 0, estado TEXT NOT NULL DEFAULT 'activo' CHECK(estado IN ('activo','pausado')),
     creado_en TEXT NOT NULL, actualizado_en TEXT NOT NULL)` + CHECK de exclusividad: fecha_fin y cuotas_total
     no pueden coexistir; dia_mes XOR quincena según frecuencia.
   - `compromisos_pagos(id TEXT PK, compromiso_id TEXT NOT NULL REFERENCES compromisos(id) ON DELETE CASCADE,
     periodo_key TEXT NOT NULL, movimiento_id TEXT NOT NULL REFERENCES movements(id),
     monto_pagado INTEGER NOT NULL, fecha_pago TEXT NOT NULL, UNIQUE(compromiso_id, periodo_key))`.
4. **Comandos Tauri** (`app.rs`, siguiendo el patrón existente):
   - `add_compromiso`, `update_compromiso`, `delete_compromiso`, `set_compromiso_estado`
   - `get_vencimientos` → { atrasados, hoy, proximos(≤7 días) } (fuente del badge)
   - `confirmar_pago_compromiso(id, monto_centavos, fecha)` → valida, crea Movement gasto, inserta
     `compromisos_pagos`, incrementa `cuotas_confirmadas`, retorna movimiento creado.
   - `get_compromiso(id)` → detalle + historial de pagos.
5. **Tests primero**: unit (recurrencia, duración, validaciones) → contract (CompromisosPort) →
   integration (flujo confirmar pago, dedup, cuotas agotadas). Edge: día 29-31 en meses cortos,
   fecha fin pasada, N cuotas agotadas, pausado sin recordatorios.

## Estructura

```
src-tauri/src/
├── models/compromiso.rs     # entidad + recurrencia + duración
├── compromisos/mod.rs       # re-exports
├── compromisos/port.rs      # CompromisosPort trait
├── compromisos/sqlite_impl.rs
└── app.rs                   # comandos nuevos (mismo AppState)
migrations/002_compromisos.sql
src-tauri/tests/test_compromisos.rs   # integration
```

## Criterios de cierre de la feature

- Suite completa verde (`cargo test --workspace`), clippy `-D warnings` limpio.
- ESC del quickstart 002 validados manualmente (pendiente del líder al final de la feature).
- README + ROADMAP actualizados.
