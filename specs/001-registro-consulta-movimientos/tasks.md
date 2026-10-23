---

description: "Task list for Núcleo de registro y consulta de movimientos"
---

# Tasks: Núcleo de registro y consulta de movimientos

**Input**: Design documents from `/specs/001-registro-consulta-movimientos/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/, quickstart.md

**Organization**: Tasks grouped by **Entrega** (delivery) matching Etapa 4 del plan de trabajo. Each entrega delivers an independently testable increment per quickstart.md scenarios.

## Format: `[ID] [P?] [Entrega] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Entrega]**: Delivery number (E1–E6) matching Phase 2 Preview in plan.md
- Exact file paths from plan.md Project Structure

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Initialize Tauri/Rust project, workspace structure, tooling

- [X] T001 Create Rust workspace structure per plan.md (src/, tests/, src-tauri/)
- [X] T002 Initialize Tauri v2 project in `src-tauri/` with `cargo tauri init`
- [X] T003 [P] Add workspace dependencies: `tauri`, `leptos`, `rusqlite`, `regex`, `chrono`, `rust-decimal`, `uuid`, `serde`, `thiserror`, `anyhow`, `pretty_assertions`, `insta`
- [X] T004 [P] Configure `rustfmt`, `clippy` (`-D warnings`), `cargo-llvm-cov`, `cargo-dist`
- [X] T005 [P] Create `.gitignore`, `README.md` with quickstart commands
- [X] T006 [P] Setup `tauri.conf.json` (bundle targets: msi, nsis, updater inactive; icon; window config)
- [ ] T006a [P] Add app icon: `src-tauri/icons/icon.ico` (256x256, 48x48, 32x32, 16x16) para Windows (placeholder transparente usado)

**Checkpoint**: `cargo tauri dev` launches empty window; `cargo test --workspace` passes (0 tests) ✅

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core domain models, storage port, parser port, UI ports — MUST complete before any entrega

- [X] T007 [P] **E1** Create `src/models/movement.rs` with `Movement`, `MovementType`, `MovementOrigin`, validations, `update()`, `format_monto()`, `sort_key()` — per data-model.md
- [X] T008 [P] **E1** Create `src/models/summary.rs` with `PeriodSummary`, `format_cop()`, `label()` ("Balance registrado"), `range_description()`
- [X] T009 [P] **E1** Create `src/models/mod.rs` re-exports
- [X] T010 **E1** Create `src/storage/port.rs` with `StoragePort` trait + `PeriodSummary` + `StorageError` — per contracts/storage_port.rs
- [X] T011 **E1** Create `src/parser/port.rs` with `ParserPort` trait + `ParsedProposal` + `ParserError` — per contracts/parser_port.rs
- [X] T012 **E1** Create `src/ui/port.rs` with `UiPort`, `UiNotifier`, `AppCommand` traits + `MockNotifier` — per contracts/ui_port.rs
- [X] T013 [P] **E1** Unit tests `tests/unit/test_movement.rs` — validations, monto>0, concepto req, fecha≤hoy, update inmutables, sort_key
- [X] T014 [P] **E1** Unit tests `tests/unit/test_summary.rs` — diferencia, format_cop, label, range_description
- [X] T015 [P] **E1** Contract tests `tests/contract/test_storage_port.rs` — generic suite over `StoragePort` (save, update, delete, query, summary, migrate)
- [X] T016 [P] **E1** Contract tests `tests/contract/test_parser_port.rs` — generic suite over `ParserPort` (patrones, magnitudes, separadores, case-insensitive, None cases)

**Checkpoint**: All unit + contract tests pass; `cargo clippy -D warnings` clean ✅

---

---

## Phase 3: Entrega 1 — Modelo + Storage + Persistencia (E1)

**Goal**: App abre, guarda movimiento, cierra, reabre → datos intactos (ESC-04, SC-002)

**Independent Test**: `cargo test --test integration_test_persistence` + manual ESC-04

### Implementation

- [X] T017 **E1** Create `src/storage/sqlite_impl.rs` implementing `StoragePort` with `rusqlite`:
  - `migrate()`: ejecuta `migrations/001_init.sql`, setea `user_version`
  - `save()`: INSERT en transacción, retorna `Movement` con id/timestamps
  - `update()`: UPDATE monto/concepto/fecha + `actualizado_en`
  - `delete()`: DELETE por id, retorna bool
  - `get_by_id()`: SELECT por id
  - `query()`: SELECT con rango opcional, ORDER BY fecha DESC, creado_en DESC
  - `summary()`: agregación condicional por tipo en rango
- [X] T018 **E1** Create `migrations/001_init.sql` — schema per data-model.md (tabla movements, índices, user_version=1)
- [X] T019 **E1** Create `src/storage/mod.rs` re-exports + constructor `SqliteStorage::new(app_dir: &Path) -> Result<Self>`
- [X] T020 **E1** Create `src/app.rs` wiring: `SqliteStorage` + `RegexParser` (stub) + `AppCommand` impl básico
- [X] T021 **E1** Create `src/main.rs` Tauri entry point: setup `SqliteStorage`, mount `AppCommand` via `tauri::Builder`
- [X] T022 **E1** Integration test `tests/integration/test_persistence.rs`:
  - save 3 movimientos → close DB → reopen → query all = 3, datos idénticos
  - crash-safety: drop connection mid-transaction → reopen → no corrupción
- [X] T023 **E1** Run quickstart ESC-04 manual: build portable, ejecutar, registrar 3, cerrar, reabrir → verifican

**Checkpoint**: ESC-04 pasa; `cargo test --test integration_test_persistence` verde; binario portable generado ✅

---

## Phase 4: Entrega 2 — Registro (Formulario + Mensaje) + Parser (E2)

**Goal**: Registrar gasto/ingreso por formulario y por mensaje interpretado con confirmación (ESC-01, 02, 03, 08, 09; SC-001, SC-007)

**Independent Test**: `cargo test --test integration_test_register_query` + manual ESC-01, 02, 03, 08, 09

### Tests FIRST (Test-First Quality)

- [ ] T024 [P] **E2** Contract test `tests/contract/test_parser_port.rs` — añadir casos ESC-09 (sin monto → None, "ayer" → hoy v1)
- [ ] T025 [P] **E2** Integration test `tests/integration/test_register_query.rs` — flujo formulario: save gasto → query → verify; save ingreso → query → verify
- [ ] T026 [P] **E2** Integration test — flujo mensaje: parse → proposal → confirm → save → query verify

### Implementation

- [ ] T027 **E2** Create `src/parser/regex_impl.rs` implementing `ParserPort`:
  - Patrones prioritarios: gasto con "mil/k", gasto simple, ingreso, préstamo→gasto
  - `parse_monto()`: normaliza separadores, magnitudes (mil/k=1000, millón/M=1_000_000)
  - Retorna `ParsedProposal` con `confidence=1.0` o `None`
- [ ] T028 **E2** Create `src/parser/mod.rs` re-exports
- [ ] T029 **E2** **UI en `src-tauri/dist/index.html`** — no Leptos:
  - Formularios "Gasto manual" e "Ingreso manual" en pestaña Registrar
  - Botón "Registro rápido" en isla + card "Registro rápido"
  - Lanzador superpuesto (command bar overlay): input, preview local (`parseLocal`), ejemplos, footer
  - Validación cliente: `parseLocal` replica reglas del `RegexParser` para preview instantáneo
  - Enter → `invoke('parse_message')` → modal propuesta nativo
- [ ] T030 **E2** Create `src/ui/main_window.rs` — ventana principal: tabs (Registrar/Historial/Resumen) + isla (brand + tabs + botón) + command bar overlay (HTML/JS)
- [ ] T031 **E2** Update `src/app.rs` `AppCommand` impl:
  - `handle_new_expense/income`: valida → `Movement::new` → `storage.save()` → `notifier.show_movement_saved`
  - `handle_parse_message`: `parser.parse()` → `Some` → `notifier.show_proposal` / `None` → error
  - `handle_confirm_proposal`: `Movement::new` desde proposal → `storage.save()` → `notifier.show_movement_saved`
- [ ] T032 **E2** Run quickstart ESC-01, 02, 03, 08, 09 manual + integration tests
- [ ] T032a **E2** Add benchmark: `tests/bench/registration_bench.rs` — mide tiempo formulario (apertura → guardado) y pasos; aserta < 30s / ≤3 pasos (SC-001)
- [ ] T032b **E2** Add parser accuracy logging: en `RegexParser::parse` y `AppCommand::handle_confirm_proposal`, log `(propuesta, confirmada_sin_cambios)`; aserta ≥70% en test de integración (SC-007)

**Checkpoint**: ESC-01/02/03/08/09 pasan; registro < 30s / 3 pasos (benchmark); ≥70% mensajes sin corrección (SC-007)

---

## Phase 5: Entrega 3 — Historial + Filtro de fechas (E3)

**Goal**: Historial completo por defecto (reciente arriba), filtro por rango opcional (ESC-05, SC-003)

**Independent Test**: `cargo test --test integration_test_history` + manual ESC-05

### Tests FIRST

- [ ] T034 [P] **E3** Contract test `tests/contract/test_storage_port.rs` — query con rango, sin rango, orden desc
- [ ] T035 [P] **E3** Integration test `tests/integration/test_history.rs` — query rangos, orden, filtro UI
- [ ] T035a [P] **E3** Edge case tests: rango invertido (desde > hasta → error/auto-swap), fecha muy antigua (≥1900), período sin movimientos → resumen ceros

### Implementation

- [ ] T036 **E3** **UI en `src-tauri/dist/index.html`** — no Leptos:
  - Pestaña Historial: lista de movimientos con `format_monto()`, concepto, fecha, origen
  - Date pickers "Desde" / "Hasta" + botones "Filtrar" / "Limpiar"
  - Sin rango → `invoke('get_history')` → muestra todo; con rango → filtra
  - Click fila → detalle read-only (puede ser modal simple)
- [ ] T037 **E3** Update `AppCommand::handle_open_history` / `handle_history_range_changed` → `storage.query()` → `notifier.show_history()`
- [ ] T038 **E3** Run quickstart ESC-05 manual + integration tests

**Checkpoint**: ESC-05 pasa; historial < 1s con 500 items; orden fecha desc + creado_en desc

---

## Phase 6: Entrega 4 — Corrección (Editar/Eliminar) (E4)

**Goal**: Editar monto/concepto/fecha; eliminar con confirmación (ESC-06, SC-005)

**Independent Test**: `cargo test --test integration_test_edit_delete` + manual ESC-06

### Tests FIRST

- [ ] T039 [P] **E4** Contract test `tests/contract/test_storage_port.rs` — update, delete, delete inexistente
- [ ] T040 [P] **E4** Integration test `tests/integration/test_edit_delete.rs` — edit → summary update; delete con confirm → gone; delete cancel → stays
- [ ] T040a [P] **E4** Edge case tests: editar último movimiento del período → resumen ceros (sin "saldo negativo"), eliminar con confirmación cancelada → movimiento persiste, editar sin cambiar datos → `actualizado_en` se refresca

### Implementation

- [ ] T041 **E4** **UI en `src-tauri/dist/index.html`** — no Leptos:
  - Modal editar: prefilled con datos del movimiento (monto, concepto, fecha)
  - **No permite cambiar tipo** (ingreso/gasto inmutable)
  - Guardar → `invoke('edit_movement', { id, monto_centavos, concepto, fecha })`
  - Eliminar: diálogo confirmación nativo → `invoke('delete_movement', { id })`
- [ ] T042 **E4** Update `AppCommand::handle_edit_movement`:
  - `movement.update(monto, concepto, fecha)` → `storage.update()` → `notifier.show_movement_saved`
  - Valida igual que `Movement::new` (monto>0, concepto req, fecha≤hoy)
- [ ] T043 **E4** Update `AppCommand::handle_delete_movement`:
  - `notifier.ask_confirmation("Eliminar", "¿Eliminar 'Concepto' ($Monto)?")`
  - Si true → `storage.delete(id)` → `notifier.show_history` refreshed
- [ ] T044 **E4** Conectar `HistoryView` click derecho / botón → abre `EditDialog` o llama delete
- [ ] T045 **E4** Run quickstart ESC-06 manual + integration tests

**Checkpoint**: ESC-06 pasa; editar < 1 min; resumen refleja cambios inmediato; confirmación bloquea delete accidental

---

## Phase 7: Entrega 5 — Resumen del período (E5)

**Goal**: Totales ingresos/gastos/diferencia por rango, rótulo "Balance registrado", nunca "dinero libre" (ESC-07, SC-004)

**Independent Test**: `cargo test --test integration_test_summary` + manual ESC-07

### Tests FIRST

- [ ] T046 [P] **E5** Contract test `tests/contract/test_storage_port.rs` — summary con/sin rango, vacío
- [ ] T047 [P] **E5** Integration test `tests/integration/test_summary.rs` — totales cuadran con query manual; rango filtra; vacío ceros
- [ ] T047a [P] **E5** Edge case tests: período sin movimientos → tres ceros sin error, label "Balance registrado" (nunca "dinero libre"/"saldo disponible"/"saldo bancario"), diferencia negativa mostrada con signo "−"

### Implementation

- [ ] T048 **E5** **UI en `src-tauri/dist/index.html`** — no Leptos:
  - 3 cards: Ingresos (verde, `+ $X`), Gastos (rojo, `− $Y`), **Balance registrado** (azul, `+/- $Z`)
  - Date pickers "Desde" / "Hasta" (mismo UX que historial) → `invoke('get_summary', { desde, hasta })`
  - Sin rango → totales acumulados; con rango → recalcula
  - Texto **nunca** "dinero libre", "saldo disponible", "saldo bancario"
- [ ] T049 **E5** Update `AppCommand::handle_open_summary` / `handle_summary_range_changed` → `storage.summary()` → `notifier.show_summary()`
- [ ] T050 **E5** Run quickstart ESC-07 manual + integration tests

**Checkpoint**: ESC-07 pasa; totales exactos vs suma manual; label oficial "Balance registrado"

---

## Phase 8: Entrega 6 — Empaquetado + Validación final (E6)

**Goal**: Binario portable + instalador, suite completa, quickstart manual 10/10 (DoD)

**Independent Test**: `cargo test --workspace` + manual all ESC + Windows limpio

### Implementation

- [ ] T051 **E6** Configure `cargo-dist` en `src-tauri/` → `cargo dist init` → genera workflow / local build
- [ ] T052 **E6** Build release: `cargo tauri build --release` → verifica `.exe` portable en `target/release/`
- [ ] T053 **E6** Build instalador: `cargo dist build --artifacts=msi` → `.msi` en `target/dist/`
- [ ] T054 **E6** Full test suite: `cargo test --workspace --lib --tests` → 0 failures
- [ ] T055 **E6** Cobertura: `cargo llvm-cov --workspace --html` → ≥80% en models, parser, storage, summary
- [ ] T056 **E6** Lint: `cargo clippy --workspace -- -D warnings` → 0 warnings
- [ ] T057 **E6** Quickstart manual **en Windows limpio** (VM o máquina sin datos):
  - ESC-01 a ESC-10 todos pasan
  - Offline verificado: `netstat -an | findstr pocketflow` → vacío
  - Datos en `%APPDATA%\PocketFlow\pocketflow.db` (no en repo)
- [ ] T057a **E6** Add edit/delete timing benchmark: `tests/bench/edit_delete_bench.rs` — mide tiempo editar/eliminar; aserta < 1 min (SC-005)
- [ ] T057b **E6** Add adoption telemetry: en `AppCommand::handle_new_expense/income`, log `(fecha_gasto, fecha_registro)`; script `scripts/analyze_adoption.rs` calcula % mismo-día; aserta ≥80% en validación 1 semana (SC-006)
- [ ] T058 [P] **E6** Update `README.md` con instalación, uso, arquitectura, troubleshooting
- [ ] T059 [P] **E6** Tag release `v1.0.0` + notas (features, known limits: sin búsqueda, sin backup, solo COP)

**Checkpoint**: **Definition of Done completa** → release v1.0 listo para Etapa 5 (Validar con mi rutina)

---

## Dependencies & Execution Order

```
Phase 1 (Setup) 
    ↓
Phase 2 (Foundational: models, ports, unit/contract tests) ← BLOQUEA todo
    ↓
Phase 3 (E1: Storage impl + wiring + persistence) 
    ↓
Phase 4 (E2: Parser + Registro UI) ← depende E1
    ↓
Phase 5 (E3: Historial UI) ← depende E1
    ↓
Phase 6 (E4: Corrección UI) ← depende E1 + E2
    ↓
Phase 7 (E5: Resumen UI) ← depende E1
    ↓
Phase 8 (E6: Packaging + Full validation) ← depende E1–E5
```

### Paralelismo dentro de cada entrega
- Tests `[P]` pueden escribirse/ejecutarse en paralelo
- Models dentro de E1 `[P]` en paralelo
- E3, E4, E5 son **independientes entre sí** una vez E1 done (distintas vistas, mismo `StoragePort`) — pueden desarrollarse en paralelo si hay capacidad

---

## Quick Reference: ESC → Entrega mapping

| ESC | Entrega | SC | FR |
|-----|---------|----|----|
| ESC-01 | E2 | SC-001 | FR-001, FR-013a |
| ESC-02 | E2 | SC-001, SC-007 | FR-001, FR-013b |
| ESC-03 | E2 | SC-001 | FR-002, FR-013a |
| ESC-04 | E1 | SC-002 | FR-003 |
| ESC-05 | E3 | SC-003 | FR-004, FR-005, FR-006 |
| ESC-06 | E4 | SC-005 | FR-008, FR-009 |
| ESC-07 | E5 | SC-004 | FR-010, FR-011 |
| ESC-08 | E2 | SC-001 | FR-001, FR-002 (concepto obligatorio) |
| ESC-09 | E2 | SC-007 | FR-013b (revisión obligatoria) |
| ESC-10 | E2 | SC-003 | FR-001, FR-013b |

---

## Notes

- **Test-First**: Para cada entrega, tests (contract + integration) se escriben **antes** que la impl y deben fallar (red → green → refactor).
- **Constitution gates**: Cada PR debe pasar `cargo test`, `cargo clippy -D warnings`, y constitution check (ports explícitos, tests en scope estrecho, transacciones atómicas).
- **No implementar** búsqueda (FR-007 POSPUESTO), backup exportar/restaurar (FR-012 POSPUESTO), multi-moneda, auth, sync — fuera de v1.
- **Pendientes pre-impl**: simplificación datos (sin categorías/cuenta/notas) — confirmar con líder antes de E2/E3 UI.
- **Stack fijo**: Tauri v2 + Rust + HTML/JS vanilla (WebView2) + rusqlite + regex (research.md §1). No reevaluar en tasks.