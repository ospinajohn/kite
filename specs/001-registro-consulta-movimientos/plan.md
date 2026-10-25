# Implementation Plan: Núcleo de registro y consulta de movimientos

**Branch**: `001-registro-consulta-movimientos` | **Date**: 2026-10-18 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/001-registro-consulta-movimientos/spec.md`

## Summary

Aplicación de escritorio local para Windows que permite registrar ingresos y gastos (vía formulario corto o mensaje interpretado con confirmación), consultar historial ordenado por fecha con filtro de rango opcional, corregir movimientos, ver resumen de período (ingresos, gastos, diferencia = "balance registrado") y persistir datos entre sesiones. Sin búsqueda, sin respaldo exportable, sin multi-moneda (solo COP), un solo usuario sin autenticación. Arquitectura simple, almacenamiento local en archivo, sin dependencias externas.

## Technical Context (RESUELTO — ver research.md)

**Language/Version**: **Rust 1.82+** (edition 2024) — decisión en research.md §1

**Primary Dependencies**:
- `tauri` v2 (WebView2, bundler, updater)
- `leptos` 0.7 (UI reactiva WASM) o HTML/JS vanilla en WebView2
- `rusqlite` 0.31 (SQLite sync, zero-dep)
- `regex` 1.10 (parser v1)
- `chrono` 0.4 + `chrono-tz` (fechas/zonas)
- `rust-decimal` 1.35 (Decimal para UI)
- `uuid` 1.8 (v4)
- `serde` + `serde_json` (serialización)
- `thiserror` / `anyhow` (errores)
- `pretty_assertions` / `insta` (tests)

**Storage**: **SQLite** (`Kite.db` en `%APPDATA%\Kite\`; si el directorio de datos no está disponible, `Kite.db` en el directorio de trabajo) — research.md §2. Al arrancar, si no existe `Kite.db` y hay una base `pocketflow.db` de la versión anterior, se copia a la nueva ubicación sin modificar la original. Esquema versionado (`PRAGMA user_version`), migraciones numeradas.

**Testing**: **`cargo test` nativo** + `cargo-llvm-cov` (cobertura) — research.md §4. Unit, integration, contract tests. Test-First Quality obligatorio.

**Target Platform**: Windows 10/11 (x64, ARM64 via `aarch64-pc-windows-msvc`), WebView2 (incluido en OS). Portable `.exe` + instalador `.msi` via `cargo-dist`.

**Project Type**: desktop-app (single-user, local-first, no backend).

**Performance Goals**: Registro < 30 s / 3 pasos (SC-001); historial < 1 s con cientos de items; persistencia atómica (SC-002).

**Constraints**:
- Offline-capable (sin conexión a internet)
- Sin autenticación ni multi-usuario
- Sin servicios externos (privacidad, 7.3)
- No modificar Horizonte (proyecto independiente)
- Simplicity: diseño más simple que satisfaga requisitos (Principio V)

**Scale/Scope**: Un usuario, ~50–200 movimientos/quincena, historial años en un archivo. Sin concurrencia.

## Constitution Check (Pre-Phase 0)

*GATE: Must pass before Phase 0 research.*

| Principio | Cumplimiento en el Plan | Notas / Justificación |
|-----------|------------------------|----------------------|
| I. Library-First Design | ✅ | Módulos separados: `models`, `storage`, `parser`, `ui`. Cada uno con propósito público documentado. |
| II. Explicit Interfaces | ✅ | Traits `StoragePort`, `ParserPort`, `UiPort` / `UiNotifier` / `AppCommand`. Machine-checkables. |
| III. Test-First Quality | ✅ | Criterios de aceptación de SC-001..SC-007 con métodos de medición definidos. Tests unitarios, integración, contratos, benchmarks (SC-001, SC-005), telemetría adopción (SC-006), parser accuracy logging (SC-007). |
| IV. Integration Contracts | ✅ | Storage y parser cruzan frontera módulo/datos; tests de integración con SQLite real. |
| V. Simplicity and Observability | ✅ | Arquitectura mínima (4 crates internos), sin capas innecesarias. Logs estructurados. |
| Security and Reliability | ✅ | Datos locales, sin secretos, validación en fronteras, transacciones SQLite atómicas. |
| Development Workflow | ✅ | Entregas por hitos (Etapa 4), scope + acceptance criteria + validation commands por PR. |

**GATE RESULT**: **PASS** — no violations unjustified.

## Project Structure

### Documentation (this feature)

```text
specs/001-registro-consulta-movimientos/
├── plan.md              # This file
├── research.md          # Phase 0 output ✅
├── data-model.md        # Phase 1 output ✅
├── quickstart.md        # Phase 1 output ✅
├── contracts/           # Phase 1 output ✅
│   ├── storage_port.rs
│   ├── parser_port.rs
│   └── ui_port.rs
└── tasks.md             # Phase 2 output (/speckit.tasks command)
```

### Source Code (repository root — Rust/Tauri)

```text
src/
├── models/
│   ├── mod.rs
│   ├── movement.rs      # Entidad Movimiento + validaciones
│   └── summary.rs       # PeriodSummary (vista derivada)
├── storage/
│   ├── mod.rs
│   ├── port.rs          # StoragePort (trait)
│   └── sqlite_impl.rs   # Implementación rusqlite
├── parser/
│   ├── mod.rs
│   ├── port.rs          # ParserPort (trait)
│   └── regex_impl.rs    # Implementación regex v1
├── ui/
│   ├── mod.rs
│   ├── port.rs          # UiPort / UiNotifier / AppCommand (traits)
│   ├── main_window.rs   # Ventana principal: tabs + isla + command bar overlay
│   ├── entry_form.rs    # Formulario corto (gasto/ingreso)
│   ├── history_view.rs  # Historial + filtro fechas
│   ├── summary_view.rs  # Resumen del período
│   └── edit_dialog.rs   # Editar/eliminar con confirmación
├── app.rs               # Punto de entrada, wiring DI
└── main.rs              # Tauri entry point
```

**Nota UI**: El registro rápido por mensaje ya no es un componente `message_entry.rs` integrado en la pestaña Registrar. Ahora es un **lanzador superpuesto (command bar overlay)** invocado con `Ctrl+K` / `Ctrl+Space` / click en "Registro rápido" de la isla. La validación local (`parseLocal`) vive en el HTML/JS del overlay y replica las reglas del `RegexParser` para preview instantáneo.

tests/
├── unit/
│   ├── test_movement.rs
│   ├── test_parser.rs
│   ├── test_storage.rs
│   └── test_summary.rs
├── integration/
│   ├── test_register_query.rs
│   ├── test_edit_delete.rs
│   └── test_persistence.rs
└── contract/
    ├── test_storage_port.rs
    └── test_parser_port.rs

src-tauri/
├── tauri.conf.json
├── Cargo.toml
├── icons/
└── build.rs
```

**Structure Decision**: Single-project Tauri/Rust (Option 1 adaptado). Crates internos por responsabilidad con traits puertos. Tests paralelos a `src/`. `src-tauri/` para configuración Tauri.

## Complexity Tracking

> No violations → tabla vacía.

---

## Phase 0: Research ✅ COMPLETADA

**Archivo**: [research.md](research.md)

Decisiones documentadas con rationale y alternativas:

1. **Stack**: Tauri v2 + Rust + Leptos/WebView2 — binario nativo <10 MB, offline, testing nativo.
2. **Storage**: SQLite (`rusqlite`) en `%APPDATA%\Kite\Kite.db` — consultas por rango, ACID, migraciones versionadas.
3. **Parser**: Regex simple (`regex` crate) — patrones reales del usuario, `confidence=1.0`, swap futuro sin breaking.
4. **Testing**: `cargo test` + contract tests genéricos + headless UI + cobertura ≥80%.
5. **Distribución**: `cargo-dist` → MSI + portable EXE, firmable, auto-update futuro.

Todas las `NEEDS CLARIFICATION` del Technical Context resueltas.

---

## Phase 1: Design & Contracts ✅ COMPLETADA

### 1. data-model.md ✅
[Archivo](data-model.md) — Entidad `Movement` con campos, validaciones, enums, esquema SQL, migraciones futuras, vistas derivadas (`PeriodSummary`, `Historial`), ciclo de vida, reglas de negocio.

### 2. contracts/ ✅
- [contracts/storage_port.rs](contracts/storage_port.rs) — `StoragePort` trait + `PeriodSummary` + contract tests genéricos
- [contracts/parser_port.rs](contracts/parser_port.rs) — `ParserPort` trait + `ParsedProposal` + contract tests genéricos
- [contracts/ui_port.rs](contracts/ui_port.rs) — `UiPort`/`UiNotifier`/`AppCommand` traits + mock notifier para tests headless

### 3. quickstart.md ✅
[Archivo](quickstart.md) — 10 escenarios de validación manual (ESC-01..ESC-10) mapeados a SC-001..SC-007 y FRs, matriz de trazabilidad, Definition of Done, comandos de debugging.

### 4. Modelos en código ✅
- [src/models/movement.rs](src/models/movement.rs) — `Movement`, `MovementType`, `MovementOrigin`, validaciones, `update()`, `format_monto()`, `sort_key()`
- [src/models/summary.rs](src/models/summary.rs) — `PeriodSummary`, `format_cop()`, `label()` ("Balance registrado"), `range_description()`
- [src/models/mod.rs](src/models/mod.rs) — re-exports

---

## Constitution Check (Post-Phase 1 Re-evaluación)

*GATE: Re-check after Phase 1 design.*

| Principio | Evidencia en Artefactos | Estado |
|-----------|-------------------------|--------|
| I. Library-First Design | 4 módulos internos (`models`, `storage`, `parser`, `ui`) con propósito único, sin capas organizacionales. `Movement` y `PeriodSummary` en `models` reutilizables. | ✅ PASS |
| II. Explicit Interfaces | 3 traits públicos (`StoragePort`, `ParserPort`, `UiPort`/`UiNotifier`/`AppCommand`) en `contracts/`. Tipados, object-safe, documentados. | ✅ PASS |
| III. Test-First Quality | Contract tests genéricos en cada port; unit tests en `movement.rs`, `summary.rs`; integration tests planificados en `quickstart.md`; cobertura ≥80% target. | ✅ PASS |
| IV. Integration Contracts | `StoragePort` probado contra `SqliteStorage` real (integration test `test_persistence.rs`); `ParserPort` probado contra `RegexParser` real. Esquema SQL versionado. | ✅ PASS |
| V. Simplicity and Observability | 4 módulos, ~15 archivos fuente planeados. Sin DI framework complejo (wiring manual en `app.rs`). Logs en `storage::sqlite_impl` y `parser::regex_impl`. | ✅ PASS |
| Security and Reliability | Validación en `Movement::new/update` (monto>0, concepto req, fecha≤hoy). Transacciones SQLite (`BEGIN IMMEDIATE`). Sin secretos, sin red. | ✅ PASS |
| Development Workflow | Plan de 6 entregas (Phase 2 Preview) con scope, acceptance criteria, validation commands. Constitution check en cada PR gate. | ✅ PASS |

**POST-PHASE-1 GATE RESULT**: **PASS** — todos los principios constitucionales evidenciados en artefactos de diseño.

---

## Phase 2 Preview (tasks.md — generado por `/speckit.tasks`)

Entregas propuestas (alineadas con Etapa 4 del contexto):

| Entrega | Alcance | Criterio comprobable (quickstart) |
|---------|---------|-----------------------------------|
| 1. Modelo + Storage + Persistencia | `Movement`, `StoragePort`, `SqliteImpl`, wiring básico, app abre/guarda/lee | ESC-04 (persistencia) |
| 2. Registro (formulario + mensaje) + Parser | `EntryForm`, `MessageEntry`, `ParserPort`, `RegexImpl`, validaciones | ESC-01, 02, 03, 08, 09 |
| 3. Historial + Filtro de fechas | `HistoryView`, query por rango, orden desc | ESC-05 |
| 4. Corrección (editar/eliminar) | `EditDialog`, confirmación, update/delete en storage | ESC-06 |
| 5. Resumen del período | `SummaryView`, cálculo totales, rótulo "balance registrado" | ESC-07 |
| 6. Empaquetado + Validación final | Build portable, suite completa, quickstart manual | Todos los escenarios |

---

## Próximos pasos

1. Ejecutar `/speckit.tasks` para generar `tasks.md` con breakdown detallado por entrega.
2. Iniciar Entrega 1: `models` + `storage` + wiring Tauri básico.
3. Validar cada entrega contra `quickstart.md` antes de continuar.
