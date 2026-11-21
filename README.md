# Kite - Gestor de finanzas personales

Aplicación de escritorio local para Windows que permite registrar ingresos y gastos, consultar historial y ver resúmenes de período.

## Características

- Registro de gastos e ingresos (formulario + mensaje interpretado)
- Historial ordenado por fecha con filtro de rango opcional
- Corrección de movimientos (editar/eliminar con confirmación)
- Resumen de período con totales (Balance registrado)
- **Compromisos recurrentes** (arriendo, suscripciones, cuotas) con recordatorios y notificación de Windows
- **Planificación por quincena**: día de corte configurable, «Libre de la quincena» y «Libre real» (estimados según lo registrado)
- **Ahorro y metas**: metas con monto objetivo, progreso, fecha/ritmo opcional y apartados (tipo propio, no cuentan como gasto/ingreso y restan del libre estimado)
- **Simulador de impacto de nuevas deudas**: escenario «qué pasaría si» (sin guardar) que muestra efecto sobre el libre quincenal/real, conflictos con compromisos e impacto en metas
- Persistencia local en SQLite (offline, sin servicios externos)
- Un solo usuario, sin autenticación

## Requisitos

- Windows 10/11 (x64 o ARM64)
- WebView2 (incluido en Windows 10/11)

## Instalación

1. Descargar el instalador `.msi` o el ejecutable portable `.exe`
2. Ejecutar e instalar/ejecutar

## Uso

1. Abrir Kite
2. **Registro rápido (Ctrl+K o Ctrl+Space)**: Abrir el lanzador superpuesto y escribir un movimiento con una estructura concreta, por ejemplo:
   - `gasté 18 mil en almuerzo`
   - `recibí 1.500.000 de pago quincena`
   - `le presté 50k a Carlos`
   Luego revisar la propuesta y confirmar.
3. **Formulario manual**: En la pestaña **Registrar**, ingresar monto, concepto y fecha para gastos o ingresos.
4. **Historial**: Ver movimientos ordenados por fecha, filtrar por rango.
5. **Resumen**: Ver totales de ingresos, gastos y balance registrado.
6. **Compromisos**: En la pestaña **Compromisos**, crear compromisos recurrentes (mensual o quincenal) con duración indefinida, fecha fin o N cuotas. Al abrir la app, los que vencen hoy generan una notificación de Windows; los atrasados y próximos (7 días) se listan con badge en la pestaña. «Registrar pago» propone el gasto con monto base corregible; se guarda solo tras confirmar.
7. **Planificación**: En la pestaña **Planificación**, ver «Libre de la quincena» y «Libre real» con desglose (ingresos, gastos, apartados y compromisos pendientes/atrasados). Navegá quincenas con ‹ Anterior / Hoy / Siguiente › y configurá tu día de corte (1–28). Desde Planificación podés abrir el **simulador de impacto** de una nueva deuda (escenario sin guardar) para ver su efecto sobre el libre, los compromisos y las metas. El Dashboard muestra una tarjeta compacta con el «Libre real» de la quincena en curso.
8. **Metas**: En la pestaña **Metas**, crear metas de ahorro (monto objetivo y fecha opcional), apartar dinero a una meta (aparte del gasto/ingreso, resta del libre estimado), ver progreso/ritmo y archivar o reactivar metas sin devolver los apartados. Los apartados se ven en el historial como «Apartado».

## Interfaz

- Fondo cuadriculado sutil sobre tema oscuro.
- Barra superior tipo **isla flotante** con glassmorphism (blur + borde translúcido).
- La isla es la barra de navegación: contiene las pestañas **Registrar**, **Historial** y **Resumen**.
- El registro rápido es un **lanzador superpuesto** independiente que solo aparece al invocarlo con `Ctrl+K` o `Ctrl+Space`.

## Atajos de teclado

- `Ctrl+K` / `Ctrl+Space` - Abrir el lanzador de registro rápido
- `Enter` - Interpretar el mensaje escrito en el lanzador
- `Esc` - Cerrar el lanzador o cancelar la confirmación

## Datos

Los datos se guardan en `%APPDATA%\Kite\Kite.db` (SQLite).
El archivo es portable - copiarlo respalda toda la información.
Si existe una base `pocketflow.db` de la versión anterior, Kite la copia automáticamente a la nueva ubicación al arrancar (sin modificar la original).

## Desarrollo

```bash
# Clonar repositorio
git clone <repo>

# Compilar en desarrollo
cargo tauri dev

# Compilar release
cargo tauri build --release

# Ejecutar tests
cargo test --workspace

# Ver cobertura
cargo llvm-cov --html --open
```

## Arquitectura

- **Tauri v2** + **Rust** + **Leptos** (WebView2)
- **SQLite** (rusqlite) para persistencia local
- **Regex** para interpretación de mensajes
- Arquitectura Ports & Adapters (Hexagonal)

## Licencia

Proyecto personal - Sin licencia pública
