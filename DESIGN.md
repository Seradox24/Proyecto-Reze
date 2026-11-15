# Proyecto Reze — diseño de interfaz

Widget de operación, personal y discreto. Dirección aprobada: compacto oscuro,
tres líneas, ventana inicial 420 × 260 y alternativa transparente.

- Superficie verde carbón `#1a1f1c`, texto `#f2f3ee`, secundario `#aeb6aa` y
  línea activa verde claro `#c1e8b3`.
- Segoe UI del sistema: metadatos 14/12 px, controles 11/12 px y letra activa
  23 px configurables entre 14 y 64 px.
- Jerarquía: pista arriba, letra en el centro, origen e importación abajo.
  Los ajustes ocupan la ventana con desplazamiento; no crean ventanas flotantes.
- Vectores de trazo consistente para acciones; carátula solo si existe.
- Estados explícitos, focos visibles y soporte para movimiento reducido. La vista
  transparente conserva controles de edición y recuperación desde bandeja.
- Letras siempre como texto, sin HTML del proveedor. Truncar metadatos largos,
  permitir saltos de línea en letras y conservar acceso a controles a 300 × 180.

Verificación visual: `artifacts/widget-compact.png` y `artifacts/widget-lyrics.png`
se generan con datos propios en las pruebas Playwright, no son capturas de una
sesión comercial.
