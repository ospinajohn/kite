# Implementation Plan: Evaluación del impacto de nuevas deudas

**Branch**: `experimento/ui-ux` (work-units propios) | **Date**: 2026-11-20
**Spec**: [spec.md](spec.md)
**Predecessors**: 002 (compromisos), 003 (planificación/libre), 005 (metas/apartados)

## Resumen

Simulador «qué pasaría si» (sin persistencia) para evaluar una deuda por pagar de tipo
cuotas/monto fijo: el usuario ingresa los datos, se calcula un calendario de vencimientos
simulados y se muestra, por quincena del horizonte, el impacto sobre «Libre de la quincena»
y «Libre real» (003), los conflictos con compromisos existentes (002) y el impacto en metas
activas (005). No se crean ni modifican datos reales.

## Riesgo declarado

**Item 3 (alto, potencial)**: el simulador no cambia contratos públicos ni persiste datos, pero
consume y combina lógica de 002/003/005; un cálculo mal integrado podría mostrar resultados
engañosos o inducir cambios compartidos que alteren comportamiento real. Mitigación: cálculo
del escenario como capa derivada/pura que reutiliza o replica reglas existentes sin mutar estado,
tests de regresión sobre 002/003/005 y verificación de que los datos reales no cambian.

## Decisiones técnicas

1. **Sin persistencia ni migraciones (C1)**: no hay tablas nuevas, no hay cambios de
   `user_version`, no hay comandos de guardado. La deuda simulada y el resultado son efímeros
   (respuesta de comando de solo lectura y/o estado de UI). Implementar el cálculo como lógica
   pura/derivada que no escriba en puertos de storage.
2. **Modelo efímero de deuda (C2, C4, C5)**: concepto opcional, monto por cuota (> 0),
   frecuencia mensual o quincenal, fecha/inicio del primer vencimiento y duración excluyente
   (cuotas N ≥ 1, fecha fin o indefinida). Si es indefinida, incluye cantidad de quincenas a
   proyectar (C7). Generación de vencimientos coherente con 002/003 (asignación por fecha real,
   ajuste fin de mes, día de corte).
3. **Inicio pasado y horizonte (C6, C7)**: si el inicio es pasado, proyectar desde la quincena
   actual en adelante ignorando vencimientos pasados (supuesto explícito). Horizonte: hasta la
   quincena del último vencimiento simulado para duraciones finitas; para indefinida, las
   quincenas definidas por el usuario con un máximo razonable impuesto por la implementación.
4. **Base real de cálculo (FR-D08)**: tomar el estado vigente al simular (configuración/día de
   corte, movimientos del rango, vencimientos/compromisos pendientes de 002 y metas/apartados
   de 005). Construir el escenario como `plan_base + vencimientos_simulados` sin mutar origen.
5. **Impacto en libre (C3, FR-D03)**: por cada quincena del horizonte, exponer base vs simulado
   (`libre_quincena`, `libre_real`) y delta; marcar negativos y períodos en riesgo. Reutilizar la
   semántica de `PlanQuincena` (003) para no divergir.
6. **Conflicto con compromisos (C3, FR-D04)**: en cada período, listar compromisos pendientes
   reales y cuota simulada como componentes; detectar insuficiencia para cubrir pendientes del
   período + cuota simulada y señalizar conflicto. No modificar estados de 002.
7. **Impacto en metas (C3, C8, FR-D05)**: para metas activas, mostrar faltante/libre restante y
   señal de ritmo afectado/en riesgo cuando haya fecha objetivo y ritmo calculable (005). Sin
   metas activas, estado vacío informativo. No crear ni modificar apartados.
8. **UI (C9, FR-D07, FR-D10)**: entrada en Planificación que abre una vista dedicada con
   formulario de entrada (incluye quincenas a proyectar cuando sea indefinida), acción
   simular/recalcular, resultado por quincenas, bloque de conflictos, bloque de metas y acción
   limpiar. Etiquetas explícitas «Simulado/escenario/estimado», sin acciones de persistencia.

## Estructura (tentativa)

```
specs/006-evaluacion-impacto-deudas/{spec.md, plan.md}
odd/tasks/evaluacion-impacto-deudas.md
src-tauri/src/simulacion_deuda/{mod.rs, service.rs}   # cálculo puro efímero [a confirmar]
src-tauri/src/app.rs / main.rs                         # comando(s) de solo lectura [si aplica]
src-tauri/dist/index.html                              # entrada en Planificación + vista dedicada
src-tauri/tests/test_simulacion_deuda.rs               # unit/integration del escenario
```

(La ubicación/nombre de módulo y si el cálculo vive en backend o frontend se confirmarán al
implementar, priorizando no duplicar reglas de 003/002/005 y no tocar comportamiento real.)

## Pruebas previstas

- Unit: generación de calendario simulado (mensual/quincenal, cuotas/fecha fin/indefinida con
  horizonte definido, ajuste fin de mes, inicio pasado → desde quincena actual, corte 003) y
  cálculo de deltas/conflictos/indicadores de metas sobre datos controlados.
- Integration: escenario con datos reales en SQLite (compromisos 002, movimientos/plan 003,
  metas/apartados 005) verificando resultado correcto y ausencia de cambios persistidos.
- Regresión: suite existente de 001/002/003/005 sigue verde; `cargo clippy --workspace -- -D warnings`.
- UI/manual: etiquetas «simulado»/supuesto de inicio pasado, ausencia de persistencia, acceso
  desde Planificación a vista dedicada.

## Criterios de cierre

- Simulación correcta según escenarios del spec, sin persistencia y sin regresiones en 001/002/003/005.
- README/ROADMAP/odd tasks actualizados; validación manual del líder registrada.
