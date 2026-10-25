# Research: Núcleo de registro y consulta de movimientos

**Fecha**: 2026-10-18 | **Feature**: `001-registro-consulta-movimientos`

---

## 1. Stack tecnológico (lenguaje + framework UI + empaquetado)

### Decision
**Tauri v2 + Rust + Leptos (WASM) / WebView2** — binario nativo Windows, bundle < 10 MB, Rust para lógica de dominio y persistencia, Leptos para UI reactiva compilada a WASM (o HTML/JS simple en WebView2), `cargo test` nativo, `sqlx` + `rusqlite` para SQLite, `cargo-dist` / `tauri-bundler` para instalador .msi y portable.

### Rationale
- **Windows nativo**: WebView2 (ya en Windows 10/11) + binario Rust → sensación nativa, sin dependencias de runtime externas.
- **Tamaño y distribución**: ~5–8 MB comprimido; instalador MSIX/MSI + portable .exe; auto-update futuro vía Tauri updater.
- **Constitución**: Library-First (Rust modules = crate boundaries claros), Explicit Interfaces (traits = ports), Test-First (`cargo test` integrado), Simplicity (pocos moving parts: Rust + HTML/CSS/JS mínimo).
- **Persistencia**: `rusqlite` (sync, zero-dep) o `sqlx` (async, compile-time checked queries) — ambos maduros.
- **Parser**: `regex` crate (Rust) para v1; fácil migrar a `lingua` / `whatlang` / LLM local (llama.cpp) sin cambiar `ParserPort`.
- **Curva de aprendizaje**: Rust + Leptos requiere inversión inicial, pero paga en mantenimiento (tipado fuerte, cero data races, refactoring seguro). Alternativa pragmática si el equipo prefiere JS: **Tauri + TypeScript + Svelte** (misma arquitectura, WASM opcional).

### Alternatives Considered

| Alternative | Pros | Cons | Verdict |
|-------------|------|------|---------|
| **.NET 8 + WPF / WinUI 3** | 100% nativo, tooling VS excelente, MSIX nativo, xUnit, EF Core | Windows-only (ok para v1), bundle mayor (~50–100 MB), menos portátil si futuro web/móvil | **Deferred** — fuerte candidato si equipo prefiere C# |
| **Electron + TypeScript + Svelte** | Ecosistema enorme, DX familiar, Jest/Playwright, mejor-sqlite3 | Bundle ~80–120 MB, RAM alta, WebView no nativo (Chromium embebido) | **Rejected** — sobrecarga innecesaria para app local simple |
| **Python 3.12 + PySide6 (Qt)** | Scripting rápido, pytest, SQLite nativo, Qt maduro | PyInstaller/Nuitka → binario ~30–50 MB, startup lento, GIL, tipado opcional | **Deferred** — viable si prioridad es velocidad de desarrollo inicial |
| **Go + Wails v3** | Binario único ~10 MB, testing nativo, SQLite (modernc.org/sqlite), cross-compile fácil | UI en Go templates / HTML, menos maduro que Tauri, ecosystem UI menor | **Deferred** — buena alternativa a Tauri |
| **Tauri + TypeScript + Svelte (sin Rust logic)** | DX familiar, mismo bundle pequeño | Lógica de dominio en JS → menos garantías de corrección, parser regex en JS ok pero migración a Rust luego requiere rewrite | **Rejected** — perdería ventajas de Rust en dominio |

---

## 2. Persistencia local: SQLite vs JSON Lines vs embedded KV

### Decision
**SQLite (archivo único `Kite.db` en `%APPDATA%\Kite\`)** con esquema versionado (`PRAGMA user_version`). Al arrancar, si no existe `Kite.db` y hay una base `pocketflow.db` de la versión anterior, se copia a la ubicación nueva (la base original no se modifica). `rusqlite` (sync) para simplicidad v1; migración a `sqlx` si se necesita async en UI.

### Rationale
- **Consultas por rango + orden**: `WHERE fecha BETWEEN ? AND ? ORDER BY fecha DESC, creado_en DESC` — nativo, eficiente, índices simples.
- **Integridad atómica**: Transacciones ACID; `INSERT`/`UPDATE`/`DELETE` en transacción → sin corrupción tras crash (SC-002).
- **Migraciones futuras**: `user_version` + scripts SQL numerados (001_init.sql, 002_add_campo.sql) — estándar, testeable.
- **Cero config**: Archivo único, sin servidor, backup = copiar archivo.
- **Tamaño**: ~KB por cientos de movimientos; `VACUUM` ocasional.
- **JSON Lines**: Rechazado — sin índices, scan completo para rangos, sin transacciones atómicas fáciles.
- **KV (bbolt/redb)**: Rechazado — sobreingeniería para tabla única; consultas por rango requieren cursor manual.

### Esquema v1 (001_init.sql)

```sql
CREATE TABLE movements (
    id           TEXT PRIMARY KEY,           -- UUID v4
    tipo         TEXT NOT NULL CHECK(tipo IN ('ingreso','gasto')),
    monto        INTEGER NOT NULL CHECK(monto > 0),  -- centavos (COP * 100) para precisión exacta
    concepto     TEXT NOT NULL CHECK(trim(concepto) != ''),
    fecha        TEXT NOT NULL,              -- ISO 8601 date: YYYY-MM-DD
    origen       TEXT NOT NULL CHECK(origen IN ('formulario','mensaje')) DEFAULT 'formulario',
    creado_en    TEXT NOT NULL,              -- ISO 8601 datetime: YYYY-MM-DDTHH:MM:SS
    actualizado_en TEXT NOT NULL             -- ISO 8601 datetime
);

CREATE INDEX idx_movements_fecha ON movements(fecha DESC, creado_en DESC);
CREATE INDEX idx_movements_tipo_fecha ON movements(tipo, fecha DESC);
```

**Nota**: `monto` en centavos (entero) evita problemas de punto flotante; COP no tiene sub-centavo. Conversión a/desde Decimal en capa de modelo.

### Migraciones
- `migrations/001_init.sql` → aplicado al abrir si `user_version < 1`.
- Futuras: `002_add_campo.sql` → `PRAGMA user_version = 2`.
- Test de migración: `storage_test::test_migration_v1_to_v2`.

---

## 3. Parser de mensajes interpretados (v1: regex simple)

### Decision
**Regex simple (crate `regex`)** con patrones nombrados, priorizados por especificidad. Retorna `ParsedProposal` con `confidence = 1.0` si matchea, `None` si no. Sin NLP ni LLM en v1.

### Rationale
- **Comportamiento aprobado (FR-013)**: usuario escribe → app propone → usuario revisa/confirma. Regex cubre patrones reales del usuario con precisión determinista.
- **Patrones objetivo** (del contexto y clarificaciones):
  - `gasté 18 mil en almuerzo` → gasto 18000 "almuerzo"
  - `gasté 12000 en transporte` → gasto 12000 "transporte"
  - `recibí 1500000 de pago de quincena` → ingreso 1500000 "pago de quincena"
  - `le presté 50000 a carlos` → gasto 50000 "carlos" (préstamo por cobrar = idea post-MVP, hoy gasto con concepto)
- **Manejo de magnitudes**: "mil" / "k" / "K" → ×1000; "millón" / "M" → ×1_000_000; separadores `.` `,` `_` ignorados.
- **Fecha implícita**: hoy (local). Si el mensaje incluye "ayer" / "anteayer" / "el 15" → parseo opcional v1.1.
- **Confianza**: 1.0 = matcheo exacto; 0.0 = no matchea → app pide formulario manual.
- **Extensibilidad**: `ParserPort` permite swap a implementación NLP/LLM sin tocar UI ni tests de contrato.

### Gramática (regex con captura nombrada)

```rust
// Pseudocódigo de patrones (orden de prioridad)
let patterns = [
    // Gasto con "mil"/"k"
    r"(?i)^\s*gast[ée]\s+(?P<monto>[\d.,_]+)\s*(?P<mag>mil|k)?\s+en\s+(?P<concepto>.+)$",
    // Gasto simple
    r"(?i)^\s*gast[ée]\s+(?P<monto>[\d.,_]+)\s+en\s+(?P<concepto>.+)$",
    // Ingreso
    r"(?i)^\s*recib[ií]\s+(?P<monto>[\d.,_]+)\s*(?P<mag>mil|k|mill[oó]n|m)?\s+de\s+(?P<concepto>.+)$",
    // Préstamo (→ gasto con concepto)
    r"(?i)^\s*le?\s*prest[ée]\s+(?P<monto>[\d.,_]+)\s*(?P<mag>mil|k|mill[oó]n|m)?\s+a\s+(?P<concepto>.+)$",
];
```

**Normalización de monto**:
```rust
fn parse_monto(raw: &str, mag: Option<&str>) -> Option<i64> {
    let clean = raw.replace(['.', ',', '_'], "");
    let base: i64 = clean.parse().ok()?;
    let mult = match mag.map(|s| s.to_lowercase().as_str()) {
        Some("mil") | Some("k") => 1_000,
        Some("millón") | Some("m") => 1_000_000,
        _ => 1,
    };
    Some(base * mult)
}
```

### Casos de prueba (unit + contract)

| Input | Expected Proposal |
|-------|-------------------|
| `gasté 18 mil en almuerzo` | Gasto 18000 "almuerzo" hoy |
| `gasté 12.000 en transporte` | Gasto 12000 "transporte" hoy |
| `recibí 1.500.000 de pago quincena` | Ingreso 1500000 "pago quincena" hoy |
| `le presté 50 mil a carlos` | Gasto 50000 "carlos" hoy |
| `compré algo` | None (sin monto) |
| `ayer gasté 20000 en cena` | v1: None (fecha no parseada) → v1.1: Gasto 20000 "cena" ayer |

---

## 4. Estrategia de testing

### Stack
- **Unit/Integration**: `cargo test` (built-in) + `pretty_assertions` / `instamaybe` para snapshots.
- **Contract tests**: Suite compartida contra `StoragePort` y `ParserPort` (generics sobre trait objects) → valida ambas implementaciones (SQLite + mock, Regex + mock).
- **UI headless**: Leptos `TestRuntime` / `wasm-bindgen-test` para lógica de vista sin navegador; o tests de integración vía `tauri-driver` (WebDriver) para flujos E2E reales.
- **Cobertura**: `cargo llvm-cov` / `grcov` → target ≥ 80% en `models`, `parser`, `storage`, `summary`.

### Organización
```
tests/
├── unit/
│   ├── test_movement.rs       # validaciones, monto centavos, concepto trim
│   ├── test_parser.rs         # regex patterns, magnitudes, edge cases
│   ├── test_storage.rs        # save/update/delete/query/summary (mock port)
│   └── test_summary.rs        # cálculo diferencia, rangos, vacíos
├── integration/
│   ├── test_register_query.rs # flujo completo: form → save → query → summary
│   ├── test_edit_delete.rs    # edit → summary update, delete → confirm
│   └── test_persistence.rs    # close/reopen, crash-safety (sync + fsync)
└── contract/
    ├── test_storage_port.rs   # genérico sobre StoragePort
    └── test_parser_port.rs    # genérico sobre ParserPort
```

### Comandos de validación (quickstart)

```bash
# Unit + integration
cargo test --workspace --lib --integration-test

# Solo contract (rápido, sin DB real)
cargo test --test contract

# Con cobertura
cargo llvm-cov --html --open

# Build portable
cargo tauri build --release
```

---

## 5. Empaquetado y distribución Windows

### Decision
**Tauri bundler + `cargo-dist`** → genera:
- `.msi` (instalador Windows Installer, firmable)
- `.exe` portable (standalone, sin instalación)
- Auto-update manifest (para v1.1+)

### Rationale
- `cargo-dist` integra con GitHub Actions / CI local, firma opcional, checksums, notas de release.
- Portable `.exe` cubre "app local sin instalación" (contexto 7.3: control de datos).
- MSIX/Store opcional futuro.

### Config (`tauri.conf.json` relevante)

```json
{
  "bundle": {
    "active": true,
    "targets": ["msi", "nsis", "updater"],
    "icon": ["icons/icon.ico"],
    "windows": {
      "certificateThumbprint": null,
      "digestAlgorithm": "sha256",
      "timestampUrl": ""
    }
  },
  "updater": {
    "active": false,
    "endpoints": [],
    "pubkey": ""
  }
}
```

---

## Resumen de decisiones

| Tema | Decisión | Impacto en arquitectura |
|------|----------|------------------------|
| Stack | Tauri v2 + Rust + Leptos/WebView2 | Módulos Rust = Library-First; UI desacoplada via ports |
| Storage | SQLite (`rusqlite`) en `%APPDATA%` | `StoragePort` trait + `SqliteImpl`; migraciones versionadas |
| Parser | Regex simple (`regex` crate) | `ParserPort` trait + `RegexImpl`; swap futuro sin breaking |
| Testing | `cargo test` + contract tests + headless UI | Test-First Quality; integration contracts en boundary real |
| Distribución | `cargo-dist` → MSI + portable EXE | Offline-capable, sin deps externas, firmable |

Todas las decisiones respetan: **offline**, **single-user**, **sin servicios externos**, **simplicidad**, **constitución Kite**.
