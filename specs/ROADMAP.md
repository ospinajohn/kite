# Kite — Roadmap de features

Índice de features planificadas. El MVP (registro/consulta/corrección/resumen) está especificado en
`specs/001-registro-consulta-movimientos/` y su cierre de empaquetado (E6: cargo-dist, MSI, tag v1.0.0)
queda **al final de esta lista**, a decisión del líder.

| # | Feature | Spec | Estado |
| --- | --- | --- | --- |
| 001 | Núcleo de registro y consulta de movimientos (MVP) | `001-registro-consulta-movimientos/` | Implementado (E1–E5); E6 de empaquetado diferido al final |
| 002 | Compromisos recurrentes + recordatorios de pago | `002-compromisos-recurrentes/` | Implementado (modelo, storage, app, UI, notificaciones); pendiente validación manual |
| 003 | Planificación por quincena y dinero libre | `003-planificacion-quincenal/` | Implementado (config, cálculo, comando, UI); pendiente validación manual |
| 004 | Gestión de préstamos por cobrar (tarjetas pendientes) | — | Pendiente de spec |
| 005 | Ahorro y metas | — | Pendiente de spec |
| 006 | Evaluación del impacto de nuevas deudas | — | Pendiente de spec (análisis sobre 002–005) |
| 007 | Cierre MVP: empaquetado y release v1.0.0 | tasks E6 de 001 | Diferido |

## Ideas sin fecha (del contexto original, requieren spec propio)

- Búsqueda por texto del concepto y filtros avanzados
- Exportación y restauración de respaldos
- Sincronización, multi-dispositivo, autenticación
- Implementación del parser con IA (hoy: regex)
