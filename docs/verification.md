# Verificación y compatibilidad

## Karaoke por palabra

Implementado selector persistente Por línea / Por palabra. Los datos vienen de
`lyricsfile` de LRCLIB; una respuesta simulada v1 prueba la integración real del
cliente HTTP y el parser sin letras comerciales. La disponibilidad no se deduce
solo del indicador `hasWordSync`: se validan los segmentos.

Pruebas: parser Lyricsfile, finales ausentes, texto inconsistente, tiempos fuera de
rango, versión incompatible, fallback al LRC, preferencia de configuraciones
anteriores y eliminación de palabras al traducir. Pruebas UI verifican progreso
parcial, pausa, salto adelante/atrás, paquetes viejos y fallback a línea.

`scripts/smoke-karaoke.mjs` utiliza el ejecutable Windows, una sesión GSMTC propia
y `fixtures/karaoke-demo.lyricsfile.yaml`, con red desactivada. El informe se guarda
en `artifacts/karaoke-smoke.json` únicamente si todas las comprobaciones pasan.
La prueba detectó además una llegada tardía de duración que se trataba como nueva
pista; se corrigió y se añadió una regresión de identidad de metadatos.

Resultado final: 42 pruebas automatizadas (27 Rust, 6 de presentación y 9 de
interfaz) y las 7 comprobaciones nativas de karaoke superadas. Formato, Clippy sin
advertencias y ejecutable release verificados. No se inventaron tiempos para una
letra sin segmentos ni se usó una traducción como fuente de tiempos por palabra.

## Corrección manual, selección predeterminada y traducción opcional

La búsqueda de HE-MAN ahora prueba títulos reducidos sin «Intro Cover» y filtra
canciones que solo comparten palabras sueltas. La consulta real devolvió cuatro
versiones homónimas de otros artistas, no el cover de The Covers Duo. Se muestran
como alternativas; sus tiempos requieren confirmación.

Verificados: 21 casos Rust, 3 de presentación y 7 de interfaz. Se añadieron pruebas
de abreviación de HE-MAN, descarte de títulos ajenos, envío de corrección manual
con generación, aviso de varios resultados, opción de traducir sin petición previa,
conservación del original, deduplicación de líneas traducidas, tiempos intactos y
errores/cuotas del traductor.

Pruebas nativas reales:

- `scripts/smoke-search-controls.mjs`: edición manual desde interfaz, selección
  predeterminada, acceso a alternativas, metadatos originales intactos, limpieza
  al cambiar de pista y rechazo de una corrección tardía.
- `scripts/smoke-translation.mjs`: dos frases propias enviadas explícitamente a
  MyMemory; traducción completada conservando documento original y timestamps.
  Informe en `artifacts/translation-smoke.json`.

La traducción es automática en calidad, opcional en ejecución y no implica
adaptación cantable. El idioma se detecta localmente como sugerencia corregible;
solo el botón de confirmación envía texto. No se traducen ni publican letras de
forma automática. Spotify se revisó por separado en `spotify-api-review.md`.

## Reconocimiento de títulos de YouTube Music

Se reprodujo el caso comunicado mediante GSMTC: el título de video
`Black Clover Opening 5 『Miyuna - Gamushara』Cover [ES]` y el intérprete
`The Covers Duo` no producían resultados al enviarlos literalmente a LRCLIB.
Con el adaptador actualizado, `cargo run -p proyecto-reze --example inspect_lyrics --
--cover-regression` obtuvo el registro **12756272**, `Gamushara`, `The Covers Duo`,
con letra sincronizada. No se guardaron sus letras como fixtures.

El video reportaba 216,121 s y el registro 196 s. El resultado requiere elección
manual y muestra una diferencia aproximada de 20 s; no se aplica un desplazamiento
inventado ni se sustituye por la versión de Miyuna. Mientras se verificaba cambió
la sesión a otro cover (HE-MAN); su búsqueda amplia tampoco encontró resultados.
La corrección de metadatos no garantiza que la base contenga todas las grabaciones.

Pruebas: 18 casos Rust, 3 de presentación y 4 en Edge. Incluyen el título real
problemático contra un servidor HTTP simulado, conservación del intérprete,
duraciones distintas, versiones/idiomas, acentos/Unicode, sufijos VEVO/Topic,
ausencia de artista y advertencia visible antes de seleccionar la letra.

## Búsqueda automática sin datos personales

Por solicitud del usuario, Proyecto Reze identifica su cliente internamente por nombre,
versión e identificador de aplicación. Se retiró `client_contact` de preferencias,
IPC y formulario; los valores antiguos se ignoran y no se vuelven a guardar.
La búsqueda está activada por defecto y se activó en esta instalación. El
interruptor permite desactivarla sin afectar al LRC local.

Verificados: 12 casos Rust, 3 de presentación y 3 de interfaz; el servidor HTTP
simulado comprueba el User-Agent enviado. `scripts/verify-automatic-lyrics.mjs`
comprobó en el ejecutable release que no existe campo de contacto, que la opción
está activa y que seleccionar la sesión dispara una búsqueda sin pulsar Buscar.
La consulta real terminó sin error y sin coincidencias para la pista de ese momento.
Eso acredita la consulta automática, no disponibilidad de letras para toda canción.

## Comprobación adicional con YouTube Music

El 15-11-2026 el usuario dejó YouTube Music reproduciendo en Brave. El mismo
adaptador GSMTC de Proyecto Reze detectó la sesión de Brave y obtuvo metadatos, duración,
estado y una posición con `LastUpdatedTime`. No se usó ninguna API de YouTube.

`scripts/smoke-youtube-lrc.mjs` cargó `fixtures/timing-check.lrc` mediante el
formulario visible de importación por ruta en la aplicación release. Se verificaron
seis muestras consecutivas: posición, índice de línea y texto visible coincidieron;
el reloj avanzó entre muestras de Windows y la red permaneció desactivada.
Evidencia sin títulos ni historial: `artifacts/youtube-lrc-check.json`.

`scripts/smoke-gsmtc.mjs` se amplió a siete comprobaciones nativas: carga del LRC
local, pausa, salto a 30 s con línea 5 activa, reanudación, cambio de pista y cierre.
Los criterios CA-003/004/008 quedan comprobados por la importación alternativa por
ruta; completar la selección del diálogo nativo sigue siendo una prueba separada.
El diálogo ahora recibe explícitamente la ventana principal como propietaria.

Spotify quedó pospuesto por decisión del usuario; no se declara verificado.
Edge y Chrome no fueron los navegadores usados en esta comprobación de reproducción.
La tabla inferior conserva el detalle de la primera entrega y debe leerse junto
con estos resultados posteriores.

Fecha: 15 de noviembre de 2026. Equipo verificado: Windows 11 Home, compilación
26300; Rust 1.100.0 MSVC; WebView2 157.0.4341.57. Los resultados no acreditan otras
versiones de Windows ni compatibilidad universal de reproductores.

## Pruebas automatizadas

- Rust: parser UTF-8/LRC, marcas repetidas, desfase, límites de tamaño, reloj,
  pausa, búsqueda, velocidad, jitter, invalidación de pista, persistencia y red.
- HTTP con Wiremock: respuesta exacta, coincidencias ambiguas, versiones distintas,
  texto simple, instrumental, vacío, JSON inválido, 429 y servicio inaccesible.
- Frontend: estados y formato de tiempo; Playwright sobre Edge verifica interfaz,
  metadatos largos, controles, formulario, teclado y overlay a 300 × 180.
- Clippy sin advertencias; TypeScript y compilación Vite verificados.
- Ejecutable Tauri release compilado y abierto sin depender de Vite.

## Pruebas nativas

`scripts/smoke-native.mjs` comprobó arranque sin selección, red desactivada, llamadas
nativas para encima/debajo/normal, cambio de vista, recuperación por teclado y
posición válida. `scripts/smoke-gsmtc.mjs` comprobó enumeración, metadatos, timeline,
pausa estable, búsqueda, reanudación, generación nueva y desaparición de sesión.
Los informes están en `artifacts/native-smoke.json` y `artifacts/gsmtc-smoke.json`.

Los scripts nativos requieren abrir una ejecución de pruebas con
`WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9222`, en loopback.
No se activa depuración remota en una ejecución normal. El fixture debe estar
compilado mediante `cargo build -p proyecto-reze --example gsmtc_fixture`.

El diálogo nativo de importación LRC se abrió y expuso sus controles; la herramienta
de automatización no pudo completar el clic de selección. Queda pendiente confirmar
manualmente el flujo completo con `fixtures/demo.lrc`; el parser y sus límites sí
están cubiertos por pruebas automatizadas.

## Criterios del documento

| Criterios | Evidencia / estado |
|---|---|
| CA-001, CA-002 | Arranque y sesión sintética GSMTC verificados en ejecutable real. |
| CA-003 | Parser verificado; selección del archivo mediante diálogo pendiente manual. |
| CA-004 | Reloj y selección de línea probados por separado; comparación visual con importación pendiente. |
| CA-005, CA-006 | Pausa, búsqueda y reanudación verificadas con GSMTC real sintético. |
| CA-007 | Cambio de generación real y limpieza de documentos probada en Rust. |
| CA-008 | Red apagada inicialmente y pruebas locales; ensayo completo de importación offline pendiente. |
| CA-009, CA-010 | API nativa aceptó los tres modos; orden respecto de otras apps pendiente de observación manual. |
| CA-011 | Persistencia y recuperación verificadas; cambio físico de monitor/DPI pendiente. |
| CA-012, CA-013 | Estados sin letra/metadatos cubiertos en frontend y degradación en backend. |
| CA-014 | Generaciones y cancelación implementadas; ensayo de carrera extremo pendiente. |

No se declara superada la definición completa de terminado mientras resten pruebas
manuales. No se hicieron búsquedas reales de letras ni se habilitó permiso de red
durante la verificación.

## Matriz de compatibilidad

| Fuente / entorno | Estado |
|---|---|
| Fixture SMTC propio, Windows 11 26300 | Enumeración, metadatos, posición, pausa, búsqueda y cambio de pista verificados. |
| Spotify de escritorio | Pendiente de prueba; sin API directa ni credenciales. |
| YouTube Music en Edge | Pendiente de prueba; depende de lo publicado por GSMTC. |
| YouTube Music en Chrome | Pendiente de prueba; depende de lo publicado por GSMTC. |
| Windows 10 1809+ | Objetivo técnico; no probado en este equipo. |
| Monitores con DPI distinto | Restauración implementada; prueba con hardware pendiente. |

Mediciones de CPU/RAM, funcionamiento en pantalla completa exclusiva y empaquetado
firmado quedan para validación posterior; no se anuncian cifras de rendimiento.

## Rendimiento, mantenimiento y CI (15-11-2026)

Cambios: snapshot sin carátula ni letras de alternativas, avance por
`proyecto-reze:position`, worker sin sondeo, guardado de geometría agrupado,
espera y caché en memoria para la búsqueda automática, candado tolerante a
pánicos, estados tipados, Prettier, ESLint y workflow de GitHub Actions.

Comprobado fuera de Windows: formato y Clippy sin advertencias en todo el
workspace (el crate de Tauri compilado en cruz para `x86_64-pc-windows-gnu`),
30 pruebas del crate core, 7 de Vitest y 12 de Playwright con Chromium.
Las 6 pruebas del crate de Tauri solo se ejecutan en Windows y quedan a cargo
del workflow. No se repitieron los scripts nativos de `scripts/`, que requieren
el ejecutable, una sesión GSMTC y WebView2: conviene volver a pasarlos en el
equipo antes de considerar verificado el comportamiento en tiempo real.
