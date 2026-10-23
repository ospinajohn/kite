# Feature Specification: Núcleo de registro y consulta de movimientos

**Feature Branch**: `001-registro-consulta-movimientos`

**Created**: 2026-10-17

**Status**: Draft — decisiones Q1–Q3 del contexto y 5 preguntas de la sesión de clarificación resueltas (2026-10-17); decisiones no bloqueantes listadas al final

**Input**: User description: contexto inicial de PocketFlow. Objetivo inmediato: definir el núcleo de registro y consulta de movimientos de finanzas personales.

---

**Leyenda de estados usada en este documento** (regla de trabajo del contexto):

- **[APROBADO]** — decisión tomada y explícitamente acordada (incluye decisiones del líder registradas el 2026-10-17: Q1, Q2, Q3, y la sesión de Clarifications del mismo día).
- **[PROPUESTO]** — propuesta de este documento, pendiente de aprobación del líder del producto.
- **[PENDIENTE]** — decisión aún no tomada; no debe implementarse hasta aclararse.
- **[POSPUESTO]** — acordado como parte del producto, pero deliberadamente fuera de la primera versión.

> Las contradicciones detectadas en el contexto se señalan en «Contradicciones y su estado»; las resueltas corresponden a decisiones explícitas del líder, no a resoluciones unilaterales. Este documento no elige tecnologías ni implementa código, y no modifica Horizonte.

**Alcance acordado de esta caracterización** [APROBADO — «orden de alcance acordado» del contexto]: el foco inicial es **registrar movimientos y consultar en qué se fue el dinero**. El MVP debe centrarse en registrar y recordar; no se presenta como herramienta que conoce dinero libre ni capacidad de endeudamiento.

**Decisiones del líder registradas el 2026-10-17**:

- **Q1 (forma de registro): C — ambos mecanismos desde la primera versión.** Formulario corto y mensaje interpretado con revisión antes de guardar.
- **Q2 (ingresos): A — sí, ingresos desde la primera versión.**
- **Q3 (búsqueda, filtros y respaldo): C — conservación de datos + rango de fechas para historial y resumen entran; búsqueda por texto y respaldo (exportar/restaurar) se posponen a etapas siguientes.**

---

## Clarifications

### Session 2026-10-17

- Q: ¿Quieres que la primera versión de PocketFlow funcione con una sola moneda —pesos colombianos— o que desde el inicio pueda registrar movimientos en varias monedas configurables? → A: Solo pesos colombianos (COP); sin campo de moneda en los movimientos; la multi-moneda queda para una etapa futura.
- Q: ¿Cómo quieres manejar el dinero que ya tienes el día en que empiezas a usar la app, antes de tu primer registro —por ejemplo, lo que te sobra de la quincena anterior—? → A: Registrarlo como un ingreso normal (concepto «Saldo inicial» u otro); no existe un concepto especial de saldo inicial en el modelo.
- Q: Cuando abras la app y entres al historial o al resumen sin elegir fechas, ¿qué periodo debería mostrarse por defecto? → A: Todo el historial, del más reciente al más antiguo; el rango de fechas es opcional para filtrar. El resumen muestra el total acumulado hasta hoy.
- Q: Al registrar un movimiento con el formulario, ¿el concepto —«en qué gasté»— debe ser obligatorio o puede quedar vacío? → A: Obligatorio; no se guarda un movimiento sin concepto y el historial siempre muestra «en qué» fue.
- Q: ¿La primera versión es solo para tu uso personal en tu equipo, o alguien más —por ejemplo tu esposa— también podría entrar a consultar o registrar movimientos? → A: Solo yo: una única persona usa la app, sin contraseña ni perfiles.

---

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Registrar un gasto en el momento (Priority: P1)

Como usuario, quiero anotar rápidamente un gasto —monto, concepto y fecha— en el momento o poco después de hacerlo, para no depender de mi memoria días más tarde. Puedo hacerlo de dos formas aprobadas: formulario corto, o escribiendo un mensaje como «gasté 18 mil en almuerzo» y revisando lo que la app propone antes de guardar.

**Estado**: [APROBADO] el poder registrar gastos (foco inicial) y ambos mecanismos de registro (Q1: C, 2026-10-17). El detalle de campos (monto + concepto + fecha precargada) era propuesto en 6.2 y queda aprobado como conjunto mínimo.

**Why this priority**: Es el problema inmediato que origina el producto. Sin registro no existe nada que consultar después.

**Independent Test**: Registrar varios gastos —por formulario y por mensaje— en distintos días y verificar que cada uno queda guardado con los datos correctos, incluso después de cerrar y reabrir la aplicación.

**Acceptance Scenarios**:

1. **Given** la aplicación abierta el día de hoy, **When** registro un gasto con monto 18.000, concepto «Almuerzo» y dejo la fecha en hoy, **Then** el movimiento aparece en el historial como gasto de −$18.000 «Almuerzo» con la fecha de hoy.
2. **Given** el registro de un gasto, **When** intento guardarlo sin monto o sin concepto, **Then** no se guarda y se me indica qué dato falta.
3. **Given** un gasto registrado, **When** cierro y vuelvo a abrir la aplicación, **Then** el gasto sigue presente con los mismos datos (FR-003).
4. **Given** un gasto registrado hoy, **When** registro otro gasto hoy, **Then** ambos aparecen en el historial del día en un orden estable y reconocible.
5. **Given** un gasto registrado con fecha de ayer, **When** lo registro hoy con la fecha cambiada, **Then** aparece bajo la fecha de ayer, no en el día de hoy.
6. **Given** que escribo el mensaje «gasté 18 mil en almuerzo», **When** envío el mensaje, **Then** la aplicación propone un gasto con monto 18.000, concepto «Almuerzo» y la fecha actual, y me los muestra para revisar **antes de guardar**.
7. **Given** la propuesta generada desde un mensaje, **When** la confirmo sin cambios, **Then** el movimiento se guarda con los datos propuestos; **When** corrijo un dato (monto, concepto o fecha) antes de confirmar, **Then** se guarda con mi corrección.
8. **Given** un mensaje interpretado, **When** la interpretación no logra extraer un monto o un concepto válido, **Then** no se guarda nada y se me pide completar los datos manualmente.

---

### User Story 2 - Registrar un ingreso (Priority: P2)

Como usuario, quiero registrar el dinero que recibo —por ejemplo el pago de quincena— con monto, concepto y fecha, por formulario o por mensaje, para poder comparar después qué recibí contra qué gasté.

**Estado**: [APROBADO] (6.1 del contexto; decisión Q2: A del 2026-10-17 confirma su inclusión en la primera versión).

**Why this priority**: El resumen «cuánto recibí vs. cuánto salió» y el propio recorrido del MVP («recibí mi pago, fui registrando…») requieren ingresos. Es P2 porque su inclusión se confirmó explícitamente, pero el registro de gastos sigue siendo el problema más inmediato.

**Independent Test**: Registrar un ingreso de $1.500.000 «Pago de quincena» y verificar que aparece en el historial y se totaliza por separado de los gastos.

**Acceptance Scenarios**:

1. **Given** el registro de un ingreso, **When** guardo monto 1.500.000, concepto «Pago de quincena» y fecha 15 de octubre, **Then** aparece en el historial como +$1.500.000.
2. **Given** ingresos y gastos registrados, **When** consulto el resumen del período, **Then** los ingresos se muestran totalizados por separado de los gastos.
3. **Given** que escribo «recibí 1500000 de pago de quincena», **When** envío el mensaje, **Then** la aplicación propone un ingreso con esos datos para revisar antes de guardar (mismo mecanismo de confirmación que los gastos).

---

### User Story 3 - Consultar el historial (Priority: P2)

Como usuario, quiero ver mis ingresos y gastos ordenados por fecha —seleccionando el rango de fechas que me interese—, con conceptos y montos reconocibles, para responder «¿en qué se me fue la plata?» —incluida la conversación con mi esposa— sin reconstruirlo de memoria.

**Estado**: [APROBADO] el historial ordenado y el rango de fechas (6.3 + decisión Q3: C). La búsqueda por texto del concepto quedó [POSPUESTO] a etapas siguientes.

**Why this priority**: Es la segunda mitad del problema. El registro solo aporta valor si después puedo consultarlo; es además el uso que motiva la conversación familiar del contexto.

**Independent Test**: Tras registrar movimientos en varios días, abrir el historial y localizar un gasto específico de días anteriores; verificar orden, montos, conceptos y filtrado por rango de fechas.

**Acceptance Scenarios**:

1. **Given** movimientos registrados en varias fechas, **When** abro el historial, **Then** los veo completos, del más reciente al más antiguo, con tipo, concepto y monto reconocibles.
2. **Given** el historial completo por defecto, **When** selecciono un rango de fechas, **Then** solo se muestran los movimientos dentro de ese rango; **When** quito el rango, **Then** vuelvo al historial completo (FR-005).
3. **Given** el historial, **When** abro un movimiento, **Then** veo sus datos: tipo, monto, concepto y fecha (FR-006).
4. **Given** el historial completo, **When** quiero localizar un movimiento por palabras del concepto, **Then** esa búsqueda no está disponible en esta versión [POSPUESTO — Q3: C]; puedo recorrer el historial por fechas.

---

### User Story 4 - Corregir un movimiento (Priority: P3)

Como usuario, quiero editar el monto, concepto o fecha de un movimiento mal registrado, y eliminarlo con confirmación previa, para mantener un historial confiable.

**Estado**: [APROBADO] (6.4 del contexto; consistente con las decisiones Q1–Q3 sobre el alcance de la primera versión: registro, historial, corrección y resumen).

**Why this priority**: El registro rápido se equivoca —y más con interpretación de mensajes—; sin corrección, el historial pierde confianza y el usuario vuelve a su memoria.

**Independent Test**: Registrar un gasto con monto equivocado, editarlo al valor correcto y eliminar otro tras confirmar; verificar que historial y totales reflejan los cambios.

**Acceptance Scenarios**:

1. **Given** un movimiento registrado con monto 18.000, **When** lo edito a 19.000, **Then** el historial y el resumen del período reflejan 19.000.
2. **Given** un movimiento registrado, **When** solicito eliminarlo, **Then** se me pide confirmación antes de borrarlo.
3. **Given** que pido eliminar un movimiento, **When** no confirmo, **Then** el movimiento se conserva sin cambios.
4. **Given** un movimiento, **When** lo edito, **Then** puedo cambiar monto, concepto o fecha, pero su tipo (ingreso o gasto) no cambia en esta operación.

---

### User Story 5 - Ver el resumen del período (Priority: P3)

Como usuario, quiero ver para las fechas seleccionadas el total de ingresos registrados, el total de gastos registrados y la diferencia entre ambos, para entender de un vistazo qué registré que entró y qué salió.

**Estado**: [APROBADO] el resumen y el rango de fechas (6.5 + Q3: C). Restricción explícita [APROBADO]: no presentarlo como «dinero libre» ni como saldo bancario real. El rótulo final («balance registrado» u otro) sigue [PENDIENTE].

**Why this priority**: Responde de un vistazo sin sumar a mano; depende de que existan movimientos registrados, por eso después del registro y la consulta.

**Independent Test**: Con un conjunto conocido de movimientos, seleccionar un rango y verificar que los tres totales coinciden exactamente con la suma manual.

**Acceptance Scenarios**:

1. **Given** un período con ingresos por $1.500.000 y gastos por $75.000, **When** veo el resumen, **Then** muestra ingresos $1.500.000, gastos $75.000 y diferencia $1.425.000.
2. **Given** el resumen mostrado, **When** cambio el rango de fechas, **Then** los totales se recalculan usando solo los movimientos del nuevo rango; **When** no hay rango seleccionado, **Then** el resumen muestra el total acumulado de todos los movimientos hasta hoy (vista por defecto).
3. **Given** el resumen mostrado, **When** lo leo, **Then** el texto no lo presenta como «dinero libre» ni como saldo de cuenta bancaria.
4. **Given** un período sin movimientos, **When** selecciono ese período, **Then** el resumen muestra cero en las tres cifras, sin error.

---

### User Story 6 - Conservar los datos entre sesiones (Priority: P2)

Como usuario, quiero que mis movimientos se conserven al cerrar y volver a abrir la aplicación, para confiar en que el registro que hago hoy será consultable en los próximos días y quincenas.

**Estado**: [APROBADO] la conservación de datos (Q3: C). La **exportación y restauración de respaldos** quedó [POSPUESTO] a etapas siguientes; su formato y comportamiento se definirán entonces.

**Why this priority**: La persistencia es requisito invisible pero crítico: sin ella no hay «recordar» y el resto del MVP no aporta valor. Sube a P2 al haberse confirmado explícitamente en Q3.

**Independent Test**: Registrar movimientos, cerrar y reabrir la aplicación, y verificar que todos persisten con sus datos intactos.

**Acceptance Scenarios**:

1. **Given** movimientos registrados, **When** cierro y reabro la aplicación, **Then** todos los movimientos siguen disponibles con sus datos intactos.
2. **Given** varios movimientos registrados, **When** la aplicación se cierra de forma inesperada tras un guardado confirmado, **Then** los movimientos guardados no se pierden.

---

### Edge Cases

- Monto cero o negativo: el movimiento no se guarda y se pide un monto mayor que cero (el signo lo aporta el tipo, ingreso o gasto). Los decimales al escribir el monto se aceptan.
- Fecha futura: no se acepta; la fecha máxima al registrar o editar es hoy (coherente con que programar pagos está fuera del MVP). Las fechas pasadas sí se pueden elegir libremente.
- Concepto vacío o compuesto solo por espacios: el movimiento no se guarda y se pide escribir un concepto (obligatorio en ingresos y gastos).
- Fecha futura o fecha muy antigua al registrar o editar.
- Rango de fechas invertido (fecha «desde» posterior a la fecha «hasta»).
- Período seleccionado sin movimientos: el resumen debe mostrar cero sin error ni pantalla vacía sin explicación.
- Eliminar el último movimiento de un período (el resumen queda en cero, sin «saldo negativo» fantasma).
- Varias operaciones el mismo día: el orden dentro del día debe ser estable y reconocible (la app NO muestra hora; el orden dentro de un mismo día es el orden en que se registraron los movimientos).
- Mensaje interpretado con monto ambiguo («gasté 18 mil en almuerzo»: magnitud «mil»), concepto faltante, mensaje incompleto o sin intención financiera clara → la app propone lo que pudo interpretar y deja el resto para completar; nunca guarda sin revisión.
- Mensaje «le presté tanto a mi esposa» → se propone como gasto con concepto libre (la gestión de préstamos por cobrar es idea post-MVP); el usuario revisa antes de guardar.
- Interpretación que no logra extraer monto → no se guarda nada; se pide completar datos manualmente.

---

## Requirements *(mandatory)*

### Functional Requirements

**Registro**

- **FR-001**: El sistema DEBE permitir registrar un gasto con monto, concepto y fecha; la fecha se precarga con el día actual y puede cambiarse. El concepto es obligatorio: no se guarda un gasto sin él. [APROBADO — comportamiento inicial + Clarifications 2026-10-17]
- **FR-002**: El sistema DEBE permitir registrar un ingreso con monto, concepto y fecha (misma forma que un gasto); el concepto es obligatorio también en los ingresos. [APROBADO — decisión Q2: A + Clarifications 2026-10-17]
- **FR-003**: El sistema DEBE conservar los movimientos registrados al cerrar y volver a abrir la aplicación. [APROBADO — decisión Q3: C, 2026-10-17]
- **FR-013**: El sistema DEBE soportar dos mecanismos de registro, ambos en la primera versión: (a) **formulario corto** con monto, concepto y fecha precargada; (b) **mensaje interpretado**, donde el usuario escribe texto libre (ej. «gasté 18 mil en almuerzo»), la aplicación propone tipo, monto, concepto y fecha, y el usuario revisa, corrige o cancela antes de guardar. Ningún movimiento derivado de un mensaje se guarda sin confirmación explícita del usuario. [APROBADO — decisión Q1: C, 2026-10-17]. La forma de implementar la interpretación (reglas simples o servicio de IA) se evaluará en la etapa de solución técnica, sin alterar este comportamiento.

**Consulta**

- **FR-004**: El sistema DEBE mostrar el historial de ingresos y gastos ordenado por fecha, con tipo, monto y concepto reconocibles. Por defecto se muestra el historial completo, del más reciente al más antiguo; el rango de fechas es opcional para filtrar. [APROBADO — comportamiento inicial + Clarifications 2026-10-17]
- **FR-005**: El sistema DEBE permitir seleccionar opcionalmente un rango de fechas para filtrar historial y resumen; sin rango, se muestra todo (historial completo y total acumulado). [APROBADO — decisión Q3: C + Clarifications 2026-10-17]
- **FR-006**: El sistema DEBE permitir abrir un movimiento para revisar sus datos completos. [APROBADO]
- **FR-007**: El sistema DEBE permitir buscar movimientos por texto del concepto. [POSPUESTO — decisión Q3: C, 2026-10-17: fuera de la primera versión, etapa siguiente]

**Corrección**

- **FR-008**: El sistema DEBE permitir editar monto, concepto y fecha de un movimiento existente. [APROBADO]
- **FR-009**: El sistema DEBE permitir eliminar un movimiento únicamente tras confirmación previa del usuario. [APROBADO]

**Resumen**

- **FR-010**: El sistema DEBE mostrar, para las fechas seleccionadas, el total de ingresos registrados, el total de gastos registrados y la diferencia entre ambos. [APROBADO]
- **FR-011**: La diferencia NO DEBE presentarse como «dinero libre» ni como saldo bancario real. El rótulo es **«Balance registrado»**. [APROBADO — 6.5 + Clarifications 2026-10-17]

**Datos y respaldo**

- **FR-012**: La exportación de respaldos y la restauración desde un respaldo quedan [POSPUESTO] a etapas siguientes —decisión Q3: C, 2026-10-17—. Cuando se retomen, deberán definirse el formato y el comportamiento de restauración (reemplazo, fusión o confirmación) antes de implementarse. La conservación de datos entre sesiones NO depende de esta función y es requisito de la primera versión (FR-003).

**Exclusiones explícitas**

- **FR-X1**: Esta caracterización NO incluye compromisos recurrentes, reservas para pagos futuros, dinero libre por quincena, metas de ahorro, evaluación de nuevas deudas, gestión de préstamos por cobrar, sincronización bancaria, múltiples cuentas y transferencias, presupuestos por categoría, gráficas avanzadas, reportes PDF ni un asistente financiero general. [APROBADO — fuera del MVP según el contexto]. Un pago recurrente (Netflix, arriendo) puede registrarse manualmente como gasto; lo que el MVP no hará es programarlo, repetirlo o reservar dinero.
- **FR-X2**: NO se modifica Horizonte ni sus funcionalidades; PocketFlow es un proyecto independiente y NO debe copiar su arquitectura por defecto. [APROBADO — reglas de trabajo]
- **FR-X3**: Esta especificación NO elige tecnologías, lenguajes ni plataformas; eso corresponde a una etapa posterior con aprobación previa. [APROBADO — reglas de trabajo]

### Key Entities *(include if feature involves data)*

- **Movimiento**: registro de una entrada o salida de dinero. Datos aprobados: **tipo** (ingreso o gasto), **monto** (expresado siempre en pesos colombianos, COP; sin campo de moneda), **concepto** (texto libre) y **fecha**. Puede originarse en un formulario o en un mensaje interpretado (en cuyo caso su origen queda claro en el flujo de revisión). Sin categorías, comprobantes, notas adicionales ni cuenta/medio de pago en esta versión — simplificación [PROPUESTO] que el contexto pide revisar con el líder antes de implementar.
- **Resumen del período**: vista derivada —no un registro nuevo— calculada a partir de los movimientos dentro de las fechas seleccionadas: total de ingresos, total de gastos y diferencia («balance registrado», nombre provisional).

---

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Registrar un gasto toma menos de 30 segundos y no más de 3 pasos desde que el usuario abre la aplicación hasta que el movimiento queda guardado (formulario).
  - *Definición de "paso"*: 1) Navegar a pantalla de registro (click "Nuevo gasto" o atajo), 2) Completar 3 campos (monto, concepto, fecha), 3) Pulsar "Guardar". No cuentan: abrir app, cerrar modal de éxito.
  - *Método de medición*: Benchmark automatizado (`tests/bench/registration_bench.rs`) + cronómetro manual en validación.

- **SC-002**: Durante la prueba de una quincena, el 100% de los movimientos registrados siguen disponibles al cerrar y reabrir la aplicación, con datos intactos.

- **SC-003**: Durante una quincena de uso real, el usuario puede responder «¿en qué gasté?» consultando el historial para al menos el 90% de sus gastos de la semana anterior, en menos de 2 minutos por consulta, sin reconstruirlo de memoria.
  - *Método de medición*: Usuario completa checklist semanal (gastos reales vs registrados) + cronómetro por consulta. Validación en Etapa 5.

- **SC-004**: Los totales del resumen coinciden exactamente con la suma de los movimientos listados en el mismo período en el 100% de los casos de prueba manuales definidos.

- **SC-005**: Corregir un movimiento mal registrado —editar o eliminar— toma menos de 1 minuto y el cambio se refleja de inmediato en historial y resumen.
  - *Método de medición*: Benchmark automatizado (`tests/bench/edit_delete_bench.rs`) + validación manual.

- **SC-006**: En la validación con uso real, el usuario registra al menos el 80% de sus gastos del período el mismo día en que los realiza (señal de que el registro es suficientemente cómodo para su rutina).
  - *Método de medición*: Telemetría local (log `fecha_gasto` vs `fecha_registro` en cada `AppCommand::handle_new_expense/income`); script `scripts/analyze_adoption.rs` procesa logs al final de la quincena.

- **SC-007**: Al registrar mediante mensaje interpretado, el 100% de los movimientos se guarda solo tras revisión y confirmación del usuario; durante la quincena de prueba, al menos el 70% de las propuestas generadas desde mensajes se guardan sin corrección manual (señal de que la interpretación acierta con los patrones de escritura reales del usuario).
  - *Método de medición*: Log en `RegexParser::parse` (propuesta) y `AppCommand::handle_confirm_proposal` (confirmación sin cambios); aserta ≥70% match en test de integración.

---

## Assumptions

- Un solo usuario en esta versión: la app es de uso personal de una única persona, sin contraseña ni perfiles. [APROBADO — Clarifications 2026-10-17]
- Los datos se consideran privados y personales; almacenamiento local en SQLite (`%APPDATA%\PocketFlow\pocketflow.db`) sin servicios externos. [APROBADO — research.md §2, plan.md Technical Context]
- Plataforma inicial preferida: aplicación de escritorio en Windows, uso local (7.1). Es una **preferencia expresada**, no una decisión cerrada; esta especificación no fija tecnología (ver FR-X3).
- Moneda única aprobada: pesos colombianos (COP). No hay campo de moneda en los movimientos, no hay conversión de divisas y el historial y el resumen se muestran siempre en COP. La multi-moneda configurable (7.4) queda fuera hasta que se tome una decisión futura explícita. [APROBADO — Clarifications 2026-10-17]
- No existe concepto especial de «saldo inicial»: el dinero de arranque se registra como un ingreso normal (concepto «Saldo inicial» u otro texto libre). [APROBADO — Clarifications 2026-10-17]
- La fecha se maneja en zona horaria local; el día actual se precarga y puede cambiarse.
- Los conceptos son texto libre; la versión no exige ni sugiere categorías.
- La interpretación de mensajes debe cubrir al menos los patrones reales del usuario: «gasté X en …», «gasté tanto en …», «recibí X de …», «le presté X a …» (propuestos como gasto con concepto).
- «Balance registrado» es el rótulo oficial aprobado. [APROBADO — FR-011]
- Horizonte permanece intacta y no aporta decisiones de arquitectura a este proyecto.

---

## Out of Scope — visión futura (no son requisitos de la primera versión)

Ideas registradas en el contexto que **no** se implementan en esta caracterización y que requieren su propia definición y aprobación cuando llegue su momento:

- **Búsqueda por texto del concepto y filtros avanzados** — pospuestos por decisión Q3: C.
- **Exportación y restauración de respaldos** — pospuestos por decisión Q3: C; al retomarse, definir formato y comportamiento de restauración.
- Compromisos recurrentes (arriendo, suscripciones, servicios, cuotas) y recordatorios de pago.
- Planificación por quincena y cálculo de dinero libre.
- Ahorro y metas.
- Evaluación del impacto de nuevas deudas.
- Idea post-MVP: gestión de préstamos por cobrar mediante tarjetas pendientes (registrar préstamo, devoluciones parciales, monto restante visible, cierre o cancelación, historial; sin contar lo pendiente como dinero disponible). Hasta entonces, un préstamo hecho por el usuario se registra como gasto con concepto libre.
- Sincronización, acceso multi-dispositivo, autenticación.

---

## Contradicciones y su estado

Contradicciones detectadas en el contexto original:

1. **Búsqueda, filtros y respaldo** (6.3/6.6 vs. 7.4) — **RESUELTA el 2026-10-17 (Q3: C)**: entran conservación de datos y rango de fechas para historial y resumen; búsqueda por texto y respaldo exportar/restaurar se posponen.
2. **Ingresos desde la primera versión** (6.1 y recorrido del MVP vs. 7.4) — **RESUELTA el 2026-10-17 (Q2: A)**: los ingresos entran desde la primera versión.
3. **Moneda** (ejemplos en COP vs. multi-moneda configurable en 7.4) — **RESUELTA el 2026-10-17 (Clarifications)**: la primera versión trabaja exclusivamente en pesos colombianos (COP), sin campo de moneda ni conversión; la multi-moneda queda para una etapa futura.
4. **Saldo inicial** (7.4: ¿saldo inicial o solo movimientos?; la nota sugiere registrarlo como ingreso) — **RESUELTA el 2026-10-17 (Clarifications)**: no hay concepto especial; el dinero de arranque se registra como un ingreso normal.

## Decisiones pendientes que no bloquean esta especificación pero deben resolverse antes o durante la etapa técnica

| Decisión | Origen | Estado |
| --- | --- | --- |
| Implementación de la interpretación de mensajes (reglas simples vs. servicio de IA) | 7.2 | [APROBADO v1] Regex simple (patrones: 'gasté X en Y', 'recibí X de Y', 'le presté X a Z'); IA evaluación futura — comportamiento FR-013 inmutable |
| Plataforma (confirmar escritorio Windows local) | 7.1 | [APROBADO] Tauri v2 + WebView2 (Windows 10/11 x64); portable .exe + .msi via cargo-dist |
| Almacenamiento y privacidad (dónde se guardan los datos, multi-dispositivo, offline, sincronización, servicios externos) | 7.3 | [APROBADO] SQLite en `%APPDATA%\PocketFlow\pocketflow.db`; offline, sin sync, sin servicios externos; autenticación resuelta: usuario único sin contraseña |
| Nombre definitivo del resumen («balance registrado» u otro) | 6.5 | PENDIENTE |
| Simplificación de datos del movimiento (sin categorías, comprobantes, notas, cuenta) | 6.2 | [APROBADO v1] Solo tipo, monto, concepto, fecha, origen; sin categorías, comprobantes, notas, cuenta/medio de pago |
| Formato y comportamiento de respaldo (al retomarse el tema) | 6.6 / Q3: C | POSPUESTO |
