# Proyecto Reze — diseño de interfaz

Widget de operación personal para letras sincronizadas en Windows. Modo
Impeccable: **Operate**. Identidad felina con cuatro temas seleccionables,
mascota animada opcional y vista Solo letras con banda de contraste local.

## Temas

Tokens compartidos vía `data-theme` en `<html>`. Estructura y controles idénticos.

| Tema | `theme_id` | Fondo | Acento |
|---|---|---|---|
| Clásico con gato (inicial) | `classic_cat` | `#1A1F1C` | `#C1E8B3` |
| Gatito en la ventana | `window_cat` | `#211B2A` | `#F3B59E` |
| Cuaderno de gatitos | `cat_notebook` | `#F4F0FA` | `#7241A0` |
| Constelación felina | `cat_constellation` | `#151E35` | `#F2CD8A` |

La vista previa del tema en Ajustes no escribe a disco hasta **Guardar ajustes**.
Escape/Volver restauran el tema guardado. Un `theme_id` desconocido recupera solo
ese campo a `classic_cat`.

## Composición

Ventana inicial 420 × 260 (mínimo 300 × 180):

1. Cabecera (36 px): botón del gatito, marca arrastrable, vista / ajustes / ocultar.
2. Metadatos: título, artista, estado de reproducción.
3. Área central flexible: letra sincronizada o modo lectura.
4. Progreso informativo (sin seek).
5. Herramientas: Sesión · Letra · modo efectivo · tiempo.

Tipografía del sistema (`Segoe UI Variable` / `Segoe UI`). Frase activa inicial
23 px (14–64). Foco visible de 2 px con token `--focus`.

## Menús

- **Letra**: buscar/versiones, importar, ruta, corregir, traducir/alternar/cancelar.
- **Modo**: preferencia por línea/palabra; el badge muestra el modo efectivo
  (Línea / Palabra / Lectura) con explicación de fallback.
- **Gatito**: casilla `Animar gatito` (guarda inmediato) y acceso a Apariencia.

## Mascota

Recursos locales en `ui/src/assets/themes/<theme_id>/` (SVG fuente, PNG 384×288,
manifiesto JSON). Motor en `ui/src/mascot.ts` con reloj independiente del karaoke.

- Secuencias: reposo, reproduciendo, pausa, saludo, búsqueda, póster estático.
- Se detiene con preferencia apagada, movimiento reducido, ventana oculta o
  recurso inválido (fallback estático).
- En Solo letras solo anima con controles revelados.
- Preferencia `cat_animation_enabled` (inicial `true`).

Cada celda fuente de 48 px se ajusta completa a los 24 px CSS del dibujo mediante
tamaño y posición porcentuales de la hoja; no recortar la celda a tamaño fuente.
El temporizador cambia de cadencia al salir de reposo y conserva la espera entre
parpadeos pese a snapshots repetidos. Una pose final de pausa no deja trabajo periódico.

En modo lectura de una letra temporizada, mostrar el texto de las líneas ya
procesadas. El campo `plain_text` de algunos Lyricsfile conserva etiquetas LRC
y no debe sustituir esas líneas cuando la posición deja de ser fiable.

## Solo letras

`view_mode: lyrics_only`. Fondo, borde y sombra de ventana transparentes. En
reposo solo la frase activa sobre una banda ajustada al texto:

- Oscuro (`lyrics_backdrop: dark`): banda `#29292D`, letra `#FFFFFF`.
- Claro (`lyrics_backdrop: light`): banda `#F1F1F4`, letra `#202024`.

Controles recuperables por hover, foco y bandeja (sin `display:none` que los
saque del teclado). Paneles y menús conservan superficie opaca del tema.

## Preferencias nuevas

Persistidas en `%APPDATA%\app.proyectoreze.widget\settings.json` junto al resto:

- `theme_id`
- `cat_animation_enabled`
- `lyrics_backdrop`

Configuraciones antiguas sin estos campos reciben los valores iniciales sin
perder el resto. Valores desconocidos recuperan únicamente ese campo.

## Accesibilidad y movimiento

Contraste pensado ≥4,5:1 en texto normal. Reducir movimiento (app o sistema)
detiene sprites y suavizados decorativos; el karaoke sigue el reloj real.
Nombres accesibles en iconos; el dibujo del gato es `aria-hidden`.

## Verificación visual

Capturas Playwright con datos de prueba propios en `artifacts/`:

- `theme-*-420x260.png`, `theme-*-300x180.png`, `theme-classic_cat-640x400.png`
- `widget-lyrics.png`, `lyrics-only-dark-on-light.png`, `lyrics-only-light-on-dark.png`
- `mascot-playing.png`, `mascot-stopped.png`
- Regresiones: `widget-compact.png`, `karaoke-words.png`, `reading-no-timing*.png`
