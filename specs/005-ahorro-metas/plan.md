# Implementation Plan: Ahorro y metas

**Branch**: `experimento/ui-ux` | **Date**: 2026-11-12
**Spec**: [spec.md](spec.md) | **Predecessors**: 001 (movimientos), 003 (planificación)

## Resumen

Metas (nombre, objetivo, fecha opcional, estados) + apartados como **tercer tipo de movimiento**
(`aporte`, con meta asociada) que no cuenta como gasto/ingreso, resta del dinero libre en
Planificación y se edita/elimina como cualquier movimiento.

## Riesgo declarado

**Item 1 + 3 (alto)**: extender `MovementType` con `Aporte` y la columna `meta_id` exige
**migración 004 con reconstrucción de tabla** (SQLite no permite ALTER de CHECK): crear
`movements_nuevo` con CHECK ampliado + `meta_id TEXT NULL`, copiar, drop, rename,
`user_version=4`. Riesgo de datos → integración con base real pre-poblada demostrando
conservación exacta, y el work-unit de migración va en commit propio para revisión nativa
independiente. El comportamiento de ingresos/gastos existentes NO cambia (los agregados
`summary()` filtran por tipo 'ingreso'/'gasto', por lo que 'aporte' queda excluido solo).

## Decisiones técnicas

1. **Modelo**: `MovementType::Aporte` + `Movement.meta_id: Option<Uuid>` (None en todos los
   ingresos/gastos). Validaciones idénticas (monto>0, concepto requerido, fecha≤hoy);
   `format_monto` signo «→»; `label() = "aporte"`.
2. **Metas**: tabla `metas` (id PK, nombre, monto_objetivo, fecha_objetivo NULL, estado
   CHECK activa/completada/cancelada, creado_en, actualizado_en) + módulo `metas/`
   (port + sqlite_impl) siguiendo el patrón de `compromisos/`. Migración 004 crea `metas`,
   reconstruye `movements` (CHECK tipo IN ('ingreso','gasto','aporte') + meta_id NULL)
   y sube `user_version=4`.
3. **Apartados**: se guardan como movimientos tipo aporte con `meta_id`; el estado de la meta
   (completada) se recalcula tras cada alta/baja/edición de apartado (función pura
   `recalcular_estado(acumulado, objetivo)`).
4. **Planificación**: `plan_de_estado` agrega `apartados_periodo` al `PlanQuincena`
   (componente nuevo: suma de movimientos tipo aporte del rango) y lo resta de
   `libre_quincena` y `libre_real`. Serde: agregar el campo con `#[serde(default)]` para
   no romper consumidores.
5. **Comandos**: `list_metas`, `add_meta`, `update_meta`, `cancelar_meta` (confirma la UI),
   `reactivar_meta`, `add_apartado` (crea movimiento aporte), `edit_aportado` /
   `delete_apartado` (reutilizan la edición de movimientos con recálculo de meta).
6. **UI**: pestaña «Metas» (tarjetas con barra de progreso, sección cerradas, modal
   nueva/editar meta, modal «Apartar» con monto+fecha) + historial: fila tipo
   «Apartado · nombre de meta» + edición/eliminación existente funciona (el modal de edición
   ya edita movimientos; el apartado es uno). Dashboard: sin tarjeta nueva v1
   (PROPUESTO futuro). Planificación: componente «Apartado a metas» en el desglose.
7. **Tests primero**: unit (recalcular_estado, ritmo, validaciones) → contract (MetasPort) →
   integration (migración 004 con base pre-poblada de la 001 conservando datos exactos;
   flujo apartado→progreso→completada; apartado excluido de summary; resta del libre;
   edición/eliminación recalcula) → `node --check` del JS.

## Estructura

```
src-tauri/src/
├── metas/{mod.rs, port.rs, sqlite_impl.rs, service.rs}
├── models/movement.rs      # MovementType::Aporte + meta_id
├── planificacion/service.rs # componente apartados_periodo
└── app.rs / main.rs        # comandos nuevos
migrations/004_metas_aportes.sql
src-tauri/tests/test_metas.rs (contract+integration) / test_migration_004.rs
```

## Criterios de cierre

- Suite completa verde + clippy `-D warnings`; migración 004 demostrada conservando datos.
- Resumen/Balance registrado de la 001 sin cambios con los mismos datos (test de regresión).
- README + ROADMAP; validación manual del líder registrada como pendiente.
