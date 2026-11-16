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

## Temas

Los colores de `ui/src/style.css` son roles (`--text`, `--accent`, `--field-bg`…)
definidos en `:root`. Un tema los redefine bajo `[data-theme='<id>']` y añade sus
propias formas en `ui/src/skins/<id>.css`; el aspecto clásico no depende de ningún
skin. El ajuste `theme_id` se guarda con el resto de preferencias, se previsualiza
desde Ajustes → Tema y un identificador desconocido vuelve a `classic`.

**Gatito mochi (`mochi_cat`).** Tema claro de leche de fresa: fondo crema rosado
`#fff5f1`, texto cacao `#4a2f3a`, acento fresa `#cf2f69` y controles en píldora.
La marca es una carita de gato y un gatito camina por el carril de progreso
siguiendo la canción: mueve la cola mientras suena y duerme en pausa o sin pista.
El gatito es decorativo (`aria-hidden`), se detiene con movimiento reducido, se
oculta en Solo letras y en ventanas de menos de 230 px de alto, y nunca sustituye
a la barra de progreso. Dibujos propios en SVG, sin recursos remotos.

Verificación visual: `artifacts/widget-compact.png` y `artifacts/widget-lyrics.png`
se generan con datos propios en las pruebas Playwright, no son capturas de una
sesión comercial.
