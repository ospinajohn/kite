# Quickstart: Validación del Núcleo de Registro y Consulta

**Versión**: 1.0 | **Fecha**: 2026-10-18 | **Feature**: `001-registro-consulta-movimientos`

---

## Prerrequisitos

- Windows 10/11 (x64)
- Rust toolchain estable (`rustup default stable`)
- Git
- Opcional: `cargo-dist` para empaquetado (`cargo install cargo-dist`)

```bash
# Clonar y entrar al directorio del proyecto
git clone <repo-url> Kite
cd Kite

# Verificar toolchain
rustc --version
cargo --version
```

---

## Build y ejecución (desarrollo)

```bash
# Compilar en modo debug (rápido, con logs)
cargo tauri dev

# O solo Rust (sin UI) para tests de lógica
cargo test --workspace

# Compilar release (optimizado)
cargo tauri build --release

# El binario portable queda en:
# src-tauri/target/release/bundle/msi/Kite_<version>_x64_en-US.msi
# src-tauri/target/release/Kite.exe (portable)
```

---

## Ejecutar suite de validación completa

```bash
# 1. Tests unitarios + integración + contratos
cargo test --workspace --lib --tests

# 2. Tests de contrato (StoragePort, ParserPort, UiPort)
cargo test --test contract_tests

# 3. Con cobertura (requiere cargo-llvm-cov)
cargo install cargo-llvm-cov
cargo llvm-cov --workspace --html --open

# 4. Linting y type-check
cargo clippy --workspace -- -D warnings
cargo check --workspace
```

**Criterio de paso**: 0 warnings, 0 errors, cobertura ≥ 80% en `models`, `parser`, `storage`, `summary`.

---

## Escenarios de validación manual (mapean a Success Criteria SC-001..SC-007)

> Ejecutar con el binario portable `Kite.exe` en Windows limpio (sin datos previos).

### ESC-01: Registrar gasto por formulario (< 30 s, ≤ 3 pasos) — **SC-001**

1. Abrir `Kite.exe` → Ventana principal (pestaña **Registrar** activa)
2. Completar formulario **Gasto manual**:
   - Monto: `18000`
   - Concepto: `Almuerzo`
   - Fecha: (hoy, precargada) → **Guardar gasto**
3. **Verificar**: En historial aparece `− $18.000  Almuerzo  [hoy]`

**Pasos contados**: Abrir → Completar → Guardar = 3 pasos máx.

---

### ESC-02: Registrar gasto por mensaje interpretado — **SC-001, SC-007**

1. Abrir lanzador: `Ctrl+K` **o** `Ctrl+Space` **o** click en botón "Registro rápido" de la isla **o** click en card "Registro rápido"
2. Escribir: `gasté 18 mil en almuerzo` → **Enter**
   - Preview en vivo muestra: **Gasto** · **− $18.000** · **Almuerzo** · **Hoy**
3. Revisar propuesta modal:
   - Tipo: **Gasto**
   - Monto: **$18.000**
   - Concepto: **Almuerzo**
   - Fecha: **hoy**
   - Origen: **Registro rápido**
4. Click **Confirmar**
5. **Verificar**: En historial aparece idéntico a ESC-01

**Métrica SC-007**: Repetir 10 mensajes variados → ≥ 7 se guardan sin corrección manual.

---

### ESC-03: Registrar ingreso — **SC-001**

1. Completar formulario **Ingreso manual** (pestaña Registrar):
   - Monto: `1500000`
   - Concepto: `Pago de quincena`
   - Fecha: `15/10/2026` (cambiar de hoy) → **Guardar ingreso**
2. **Verificar**: En historial aparece `+ $1.500.000  Pago de quincena  [15 oct]`

---

### ESC-04: Persistencia entre sesiones — **SC-002**

1. Registrar 3 movimientos (mix ingreso/gasto) via ESC-01/02/03
2. Cerrar app completamente (click ✕ o `Ctrl+Q`)
3. Reabrir `Kite.exe`
4. **Verificar**: Los 3 movimientos están en historial con datos idénticos (monto, concepto, fecha, tipo)

**Extra**: Matar proceso (`taskkill /F /IM Kite.exe`) tras guardar → reabrir → datos intactos (escritura atómica).

---

### ESC-05: Historial con filtro de fechas — **SC-003**

1. Click pestaña **"Historial"** (vista por defecto)
2. **Verificar por defecto**: Lista completa, más reciente arriba, sin rango activo
3. Seleccionar rango: **Desde** `01/10/2026` **Hasta** `15/10/2026` → Aplicar
4. **Verificar**: Solo movimientos de octubre 1–15
5. Click **"Limpiar rango"** (o borrar campos)
6. **Verificar**: Vuelve historial completo

---

### ESC-06: Corregir movimiento (editar/eliminar) — **SC-005**

**Editar**:
1. En historial, click derecho en `− $18.000 Almuerzo` → **Editar**
2. Cambiar monto a `19000` → **Guardar**
3. **Verificar**: Historial y resumen reflejan `− $19.000`

**Eliminar con confirmación**:
1. Click derecho en un gasto → **Eliminar**
2. Diálogo: *"¿Eliminar 'Transporte' ($12.000)?"* → **Cancelar**
3. **Verificar**: Movimiento sigue en historial
4. Repetir → **Confirmar**
5. **Verificar**: Desaparece de historial y resumen

---

### ESC-07: Resumen del período — **SC-004**

1. Click pestaña **"Resumen"**
2. **Sin filtro** (por defecto): Ver totales acumulados
   - Ingresos: `+ $1.500.000`
   - Gastos: `− $30.000` (18k + 12k)
   - **Balance registrado: $1.470.000**
3. Aplicar rango `01/10`–`15/10`
4. **Verificar**: Totales recalculados solo para ese rango
5. Rango sin movimientos (ej. `2025-01-01` a `2025-01-31`)
6. **Verificar**: `Ingresos: $0  Gastos: $0  Balance registrado: $0` (sin error)
7. **Verificar texto**: En ningún momento aparece "dinero libre", "saldo disponible", "saldo bancario", "libre"

---

### ESC-08: Concepto obligatorio — **SC-001, validación**

1. Formulario **Gasto manual** → Monto `10000` → Concepto *(vacío)* → **Guardar gasto**
2. **Verificar**: Error visible *"El concepto es obligatorio"* → no guarda
3. Llenar concepto → **Guardar gasto** → ok
4. Repetir con monto `0` o negativo → error *"El monto debe ser mayor que cero"*

---

### ESC-09: Mensaje ambiguo / interpretación fallida — **SC-007**

1. Abrir lanzador (`Ctrl+K` / `Ctrl+Space`) → Escribir: `compré algo` → Enter
2. **Verificar**: No aparece preview; toast *"No pude interpretarlo. Usa el formulario manual."*
3. Escribir: `ayer gasté 20000 en cena` (v1 no parsea "ayer")
4. **Verificar**: Preview con fecha **hoy** (no ayer) → usuario corrige fecha manual si necesita

---

### ESC-10: Préstamo informal → gasto con concepto — **SC-003**

1. Abrir lanzador (`Ctrl+K` / `Ctrl+Space`) → Escribir: `le presté 50 mil a carlos` → Enter
2. Preview: **Gasto** · **− $50.000** · **carlos** · **Hoy**
3. Click **Confirmar** (o edita concepto a `Préstamo a Carlos`)
4. **Verificar**: En historial como gasto normal; idea post-MVP (tarjetas por cobrar) no implementada aún.

---

## Matriz de trazabilidad (Escenario ↔ Success Criteria ↔ FR)

| Escenario | SC | FR cubiertos |
|-----------|----|--------------|
| ESC-01 | SC-001 | FR-001, FR-013(a) |
| ESC-02 | SC-001, SC-007 | FR-001, FR-013(b) |
| ESC-03 | SC-001 | FR-002, FR-013(a) |
| ESC-04 | SC-002 | FR-003 |
| ESC-05 | SC-003 | FR-004, FR-005, FR-006 |
| ESC-06 | SC-005 | FR-008, FR-009 |
| ESC-07 | SC-004 | FR-010, FR-011 |
| ESC-08 | SC-001 | FR-001, FR-002 (concepto obligatorio) |
| ESC-09 | SC-007 | FR-013(b) (revisión obligatoria) |
| ESC-10 | SC-003 | FR-001, FR-013(b) |

---

## Definition of Done (release v1.0)

- [ ] Todos los tests automatizados pasan (`cargo test --workspace`)
- [ ] 10 escenarios manuales validados en Windows limpio
- [ ] Cobertura ≥ 80% en `models`, `parser`, `storage`, `summary`
- [ ] `cargo clippy -D warnings` limpio
- [ ] Binario portable `.exe` generado y probado
- [ ] Instalador `.msi` generado (opcional v1.0)
- [ ] Documentación de usuario mínima (README + ayuda in-app)
- [ ] Archivo de datos `Kite.db` en `%APPDATA%\Kite\` (no en repo)
- [ ] Sin dependencias de red al ejecutar (offline verificado con `netstat -an | findstr Kite` → vacío)

---

## Comandos útiles para debugging

```bash
# Ver logs de la app (RUST_LOG=debug)
$env:RUST_LOG=debug; cargo tauri dev

# Inspeccionar BD SQLite directamente
sqlite3 "%APPDATA%\Kite\Kite.db" ".schema"
sqlite3 "%APPDATA%\Kite\Kite.db" "SELECT * FROM movements;"

# Resetear datos (borrar BD)
del "%APPDATA%\Kite\Kite.db"

# Ejecutar solo tests de contrato
cargo test --test contract_tests -- --nocapture
```
