# Proyecto Reze

Widget Windows de letras sincronizadas: **Windows GSMTC** entrega sesiones,
metadatos y tiempo; los LRC locales o LRCLIB entregan las letras. No usa APIs,
cuentas ni credenciales de Spotify o YouTube. También permite controlar la sesión
elegida: anterior, reproducir/pausar y siguiente. El audio permanece en el
reproductor original y Proyecto Reze no lo descarga.

## Requisitos

- Windows 10 1809 o posterior / Windows 11, con WebView2 Runtime.
- Rust estable con MSVC, herramientas C++ de Visual Studio y SDK de Windows.
- Node.js 22.12+ o 24 y npm.

## Desarrollo

```powershell
git clone https://github.com/Seradox24/Proyecto-Reze.git "Proyecto Reze"
Set-Location -LiteralPath ".\Proyecto Reze"
npm ci
npm run desktop
```

La CLI de Tauri es una dependencia local reproducible; Rust está instalado para
el usuario y disponible desde cualquier carpeta. Si la terminal estaba abierta
al instalar Rust, abre una nueva para cargar su PATH.

## Compilar el ejecutable

```powershell
npm run desktop:build
.\target\release\proyecto-reze.exe
```

No se genera instalador, firma ni actualizador en esta entrega. `cargo run` por
sí solo no inicia Vite: para desarrollo usa `npm run desktop`.

## Primer uso

1. Abre un reproductor que publique una sesión multimedia de Windows.
2. En **Elegir sesión**, selecciona la sesión que quieras seguir.
3. Proyecto Reze busca automáticamente en LRCLIB al cambiar de canción. En ajustes puedes
   desactivar **Buscar letras automáticamente** y utilizar un `.lrc` UTF-8 local.
   No se solicitan cuentas, correos ni URLs; Proyecto Reze envía su propia identificación.
   Si prefieres pegar una ruta, abre ajustes → **Abrir un LRC por su ruta**.
   Ese flujo tampoco necesita Internet, correo ni una cuenta musical.
4. Ajusta el desfase si hace falta: valores positivos adelantan la letra.
5. Alterna vista compacta/solo letras y nivel encima/normal/debajo en ajustes.
6. Usa los botones junto al título para anterior, reproducir/pausar y siguiente.
   Windows indica qué controles admite cada sesión; los demás se desactivan.
   En Solo letras, aparecen con los controles al pasar el ratón o usar el teclado.

Cerrar u ocultar conserva el proceso si existe bandeja. Usa su menú para mostrar,
recuperar la posición, abrir ajustes o salir. Si falla la bandeja, cerrar termina
la aplicación y ocultar permanece bloqueado para evitar perder el control.

Las preferencias se guardan en `%APPDATA%\app.proyectoreze.widget\settings.json`. Las
letras y selecciones de sesión solo permanecen en memoria. No hay historial de
escucha ni caché de letras en disco. Una importación local tiene prioridad hasta
cambiar de pista. Si no hay posición fiable, la letra se presenta en modo lectura.

Al iniciar por primera vez con el nuevo nombre, se importan los ajustes de
`%APPDATA%\app.danae.widget\settings.json` si todavía no existen preferencias de
Proyecto Reze. El archivo anterior se conserva y nunca reemplaza ajustes nuevos.

## Corregir búsquedas, versiones y traducción

- **Corregir búsqueda** permite cambiar título e intérprete, o buscar sin artista.
  Solo afecta a la pista actual y se puede volver a los datos de Windows.
- Si hay varias letras, Proyecto Reze selecciona la mejor disponible, avisa y conserva
  **Versiones (N)** para cambiarla. Una coincidencia incierta se abre en lectura;
  seleccionarla explícitamente en Versiones confirma el uso de sus tiempos.
- **Traducir al español** es opcional. Primero se muestra el idioma sugerido y se
  informa de que el texto se enviará a MyMemory. No hay traducción automática por
  cambio de canción ni se requiere correo. Puedes cancelar y alternar original /
  español; la traducción conserva las marcas temporales, no la métrica cantada.
- El servicio gratuito limita tamaño y uso. Si falla, se conserva el original.
  Las traducciones y correcciones no se guardan al cambiar de pista.
- La búsqueda ampliada de HE-MAN devuelve alternativas homónimas; no se encontró
  el registro exacto de The Covers Duo. Corregir metadatos no crea letras ausentes
  de LRCLIB. Usa un LRC local si ninguna alternativa corresponde a la grabación.

## Karaoke por línea o por palabra

En **Ajustes → Modo karaoke**, elige **Por línea** o **Por palabra** y guarda.
La preferencia se conserva al reiniciar. El indicador del reproductor muestra el
modo efectivo: si la letra no tiene tiempos válidos por palabra, permanece en
**Línea** aunque hayas elegido Por palabra.

LRCLIB proporciona los segmentos temporizados en Lyricsfile cuando están
disponibles. Proyecto Reze utiliza sus marcas de inicio/fin sin repartir artificialmente
el tiempo de la línea. Si solo hay inicio de palabra, se destaca al comenzar sin
inventar una duración. Las líneas sin segmentos válidos usan el resaltado por línea.
La traducción conserva los tiempos de línea y nunca reutiliza palabras del original.

También puedes importar un `.lyricsfile.yaml` local mediante el mismo selector o
campo de ruta. `fixtures/karaoke-demo.lyricsfile.yaml` contiene texto propio para
probar el efecto. No se reproduce ni descarga audio.

## Ejecutar comprobaciones

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
npm run build
npm test
npx playwright test
```

Playwright utiliza Microsoft Edge instalado. No consulta servicios musicales ni
LRCLIB: los casos HTTP usan un servidor local simulado. Para una sesión GSMTC
propia de prueba consulta [fixtures/README.md](fixtures/README.md).

## Estructura

- `crates/proyecto-reze-core`: modelos, reloj, parser, coincidencias, LRCLIB y ajustes.
- `src-tauri`: integración GSMTC, coordinador, comandos, ventana y bandeja.
- `ui`: interfaz Vite + TypeScript, sin framework.
- `tests/ui`: pruebas de interfaz con IPC simulado.
- `docs`: arquitectura, privacidad y matriz de aceptación/compatibilidad.

`rust-verificacion` conserva la prueba inicial de la instalación; es independiente
y está excluida del proyecto y de Git.

## Límites

Que Windows publique una sesión no garantiza datos precisos ni autorización para
sincronizar contenido. Spotify de escritorio y YouTube Music en Edge/Chrome son
objetivos de pruebas, no compatibilidades garantizadas. Usa contenido propio o
autorizado para validar sincronización. Consulta [docs/compliance.md](docs/compliance.md).
