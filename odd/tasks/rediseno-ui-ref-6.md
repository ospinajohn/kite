# Feature: Rediseño UI — Opción C (Ref. 6)

Diseño de referencia: `design/kite-opciones-referencias/03-ref-6.html` (canvas: `design/kite-opciones-referencias/`). Implementar en el stack real (`src-tauri/dist/index.html`, CSS/JS inline), mapeando a componentes/estilos existentes sin copiar el HTML del artboard.

## Specs

- S1: «Implement the design artboard design/kite-opciones-referencias/03-ref-6.html» — petición (2026-11-20). «Artboard: design/kite-opciones-referencias/03-ref-6.html. Title: Opción C — Ref. 6. Tradeoff: Simple y cotidiana: ancla visual fuerte, próximos pagos y lista de movimientos con gráfico y progreso; directa y fácil, con menos analítica avanzada a la vista.»
- S2: «Implement it in the project's real stack. Use the artboard as the visual spec: map its parts to the project's existing components and styles, and do not copy the artboard HTML verbatim.»
- S3: «This design is not linked to an ODD feature document: size it under the project's ODD rules as usual.»
- S4: Conservar la isla de navegación (elemento central preferido por el usuario) y toda la funcionalidad existente (Dashboard, Historial, Registrar, Resumen, Compromisos, Planificación, Metas, modales, tema claro/oscuro, simulador, etc.), preservando los IDs/contratos que usa el JS.
- S5: Lenguaje visual Ref. 6: lienzo `#E2E7EF`, superficies blancas redondeadas, tipografía jerárquica (Plus Jakarta Sans), tarjeta ancla oscura (libre real), próximos pagos, lista de compromisos/movimientos, gráfico de evolución, tarjeta oscura de progreso de meta y desglose rápido; estados claros y cifras con `tabular-nums`.

## Tasks

- T1 [S1-S5] Sistema visual base: tokens (canvas/superficies/bordes/tinta/acentos/sombras/radios), tipografía y estilos globales coherentes con Ref. 6; isla refinada conservando estructura/comportamiento. ✓
- T2 [S1,S2,S4,S5] Dashboard (Inicio): mapear a layout Ref. 6 — ancla libre real + KPIs, próximos pagos, compromisos/movimientos recientes, gráfico de evolución, progreso de meta y desglose — preservando IDs/lógica JS existentes. ✓
- T3 [S1,S2,S4,S5] Coherencia visual en vistas restantes (Historial, Resumen, Compromisos, Planificación, Metas) y componentes comunes (tablas, formularios, botones, modales, estados) con el nuevo sistema. ✓
- T4 [S1-S5] Ajustes responsive/tema oscuro, verificación funcional básica (`cargo test --workspace`, `cargo clippy --workspace -- -D warnings` si aplica) y cierre/handoff. ✓

## Log

- L1 (2026-11-20): «Implement the design artboard design/kite-opciones-referencias/03-ref-6.html» (Opción C — Ref. 6). Implementación en `src-tauri/dist/index.html` conservando isla y funcionalidad.
- L2 (2026-11-20): T1–T4 ✓ — sistema visual Ref. 6 aplicado (canvas `#E2E7EF`, superficies blancas, tokens/radios/sombras, tipografía Plus Jakarta Sans, sin grid/cuaderno), isla refinada y conservada, dashboard reestructurado (ancla libre real + balance/KPIs con IDs JS preservados, próximos pasos, movimientos recientes, gráfico semanal, mes en curso, resumen rápido), coherencia base en componentes y ajustes responsive/tema oscuro. Verificación: `cargo test --workspace` y `cargo clippy --workspace -- -D warnings` en verde. Handoff `gentle_design` a implemented.
