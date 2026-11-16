# Punto de respaldo antes del diseño

Fecha: 15 de noviembre de 2026. Carpeta de trabajo: `D:\Proyecto Reze`.
Repositorio privado: <https://github.com/Seradox24/Proyecto-Reze>.

Este punto conserva la versión funcional anterior al trabajo de diseño:

- Etiqueta: `respaldo-antes-diseno-2026-11-15`.
- Rama de respaldo: `respaldo/antes-diseno-2026-11-15`.
- Código funcional de referencia: `afe9e70`. El commit inicial que añadió esta
  nota solo documentó el traspaso; el paquete visual posterior tampoco implementa
  el rediseño de la aplicación.

## Paquete visual listo para el agente implementador

Leer primero [ESPECIFICACION_DISENO_TEMAS_GATOS.md](ESPECIFICACION_DISENO_TEMAS_GATOS.md).
Junto a ese documento están las referencias aprobadas y propuestas:

- [Temas, sprites y menú del gato](reze-temas-gatitos-sprites.png).
- [Solo letras con banda oscura/clara y transparencia](reze-solo-letras-contraste.png).
- Los archivos `.png.json` documentan la procedencia y el prompt de la vista
  Solo letras. Estas imágenes son mockups, no recursos de sprites ya utilizables.

Implementar los cuatro temas seleccionables, sprites locales con interruptor
`Animar gatito` en su menú y la vista sin fondo general con respaldo de contraste
solo detrás de la letra. Los nuevos ajustes propuestos son `theme_id`,
`cat_animation_enabled` y `lyrics_backdrop`, con valores iniciales compatibles.
Mantener el comportamiento funcional descrito abajo.

Crear la rama de implementación desde el commit que contiene este paquete
documental, para conservar la especificación y las imágenes. Utilizar la etiqueta
de respaldo como referencia del código previo; no sobrescribirla ni moverla.
Las imágenes generadas ilustran el objetivo; reconstruir la interfaz y producir
los sprites finales siguiendo las reglas, estados y criterios de aceptación del MD.

> Nota posterior: ya existe el sistema de temas (`theme_id`, roles de color y
> `ui/src/skins/`) con un primer skin propio, **Gatito mochi** (`mochi_cat`). Los
> cuatro temas de la especificación siguen sin implementar y pueden añadirse como
> skins nuevos sobre esa base. Consultar `DESIGN.md`.

## Estado funcional

- Rust + Tauri 2 y una interfaz Vite + TypeScript, sin framework.
- Windows GSMTC proporciona sesiones, metadatos y posición de reproducción.
  No se utilizan APIs ni credenciales de Spotify o YouTube.
- LRCLIB busca letras automáticamente, con identificación propia de la app,
  corrección manual de título/artista y elección entre versiones.
- Importación de LRC y Lyricsfile YAML locales, incluidas rutas con espacios.
- Karaoke por línea o por palabra con tiempos reales; si faltan tiempos por
  palabra, se mantiene el seguimiento por línea. No se inventan tiempos.
- Sin sincronización utilizable, aviso de modo lectura persistente y letra
  neutra sin resaltado karaoke.
- Traducción al español opcional mediante MyMemory, con confirmación previa.
- Vista compacta y solo letras, bandeja, nivel de ventana y ajustes persistentes.
- Nombre visible y ejecutable: Proyecto Reze / `proyecto-reze.exe`.

Las preferencias están en `%APPDATA%\app.proyectoreze.widget\settings.json`.
El arranque importa ajustes del identificador anterior cuando corresponde.
No hay credenciales de pago, historial de escucha ni caché de letras en Git.

## Puntos de entrada para el agente de diseño

- `DESIGN.md`: descripción del diseño actual; puede evolucionar con el rediseño.
- `ui/index.html`: estructura, paneles, controles y textos.
- `ui/src/style.css`: presentación y adaptación al tamaño de ventana.
- `ui/src/main.ts`: eventos, estado, paneles y comandos nativos.
- `ui/src/view.ts`: presentación de estados y modo lectura.
- `ui/src/karaoke.ts`: representación de palabras y progreso temporal.
- `ui/src/types.ts`: contrato de datos con Rust.
- `assets/icon.svg` y `src-tauri/icons`: iconos del proyecto.
- `tests/ui/widget.spec.ts`: pruebas de interfaz y capturas de referencia.
- `src-tauri` y `crates/proyecto-reze-core`: integración y lógica de reproducción.

El alcance previsto para el siguiente trabajo es el diseño. Conviene desarrollar
en una rama nueva y conservar la etiqueta y la rama de respaldo. Los selectores
por ID y los eventos `proyecto-reze:*` enlazan la interfaz con su comportamiento y
sus pruebas: cualquier cambio debe actualizar ambos lados. El resaltado depende
de la posición real, la versión confirmada y los tiempos disponibles.

La ventana actual comienza en 420 × 260 y admite un mínimo de 300 × 180.
Conservar acceso a los controles en ambos tamaños y en la vista transparente,
foco de teclado visible, texto seguro sin HTML del proveedor y movimiento reducido.

## Desarrollo y verificación

```powershell
Set-Location -LiteralPath 'D:\Proyecto Reze'
npm ci
npm run desktop
```

La última comprobación funcional completa pasó después del cambio de carpeta:
29 pruebas de Rust, 7 de Vitest, 10 de Playwright y 7 comprobaciones nativas
de karaoke. Consultar `docs/rename-verification.md`. No se repiten estas pruebas
para guardar este respaldo, porque este commit solo añade documentación.

```powershell
npm test
npm run test:e2e
npm run build
npm run desktop:build
```

Usar Playwright y sus capturas de la aplicación para comprobar el diseño. La
captura continua del escritorio produjo antes un borde amarillo en esta máquina.
Las pruebas nativas de `scripts/smoke-karaoke.mjs` requieren el fixture compilado
y una instancia temporal con depuración WebView2; el arranque normal no la activa.

## Recuperar el respaldo sin sobrescribir el trabajo de diseño

Crear otra carpeta de trabajo a partir de la etiqueta:

```powershell
git fetch origin --tags
git worktree add -b recuperar-respaldo '..\Proyecto Reze - respaldo' respaldo-antes-diseno-2026-11-15
```

La copia local adicional está en `D:\Respaldos\Proyecto Reze`, en una subcarpeta
con la fecha, hora y commit. Incluye un ZIP de los archivos versionados, el
ejecutable de producción y sus hashes SHA-256. Las dependencias y herramientas
de compilación se recuperan con los archivos de bloqueo y los requisitos del
README; los ajustes personales siguen en AppData y no se publican.
