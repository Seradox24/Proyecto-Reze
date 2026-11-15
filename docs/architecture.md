# Arquitectura

## Flujo

GSMTC → `MediaSample` → coordinador Rust → reloj / letra activa → snapshot Tauri
→ interfaz. LRCLIB y el importador LRC producen `LyricsDocument`; no intervienen
en la detección ni en el control del reproductor.

El worker WinRT posee administrador, sesiones y suscripciones. Recibe eventos por
un canal acotado y hace un refresco de respaldo cada 1 s visible / 5 s oculto.
La selección es manual y fija; la desaparición no elige otro reproductor.
Los objetos de sesión reciben identificadores de ejecución, no se guardan en disco.

La posición de GSMTC corresponde a `LastUpdatedTime`, no al instante de lectura.
Se estima la antigüedad UTC al capturar y luego se usa `Instant` monotónico.
Muestras repetidas no reinician el reloj. Posición inválida, duración ausente o
muestra con antigüedad superior a cinco minutos desactivan extrapolación y
resaltado. No se inventa precisión para fuentes que no entregan tiempo.
El reloj respeta pausa, velocidad y límites de duración; las nuevas muestras
reancoran inmediatamente, incluyendo búsquedas hacia atrás.

El cálculo de línea ocurre aproximadamente a 30 Hz mientras la ventana es visible;
se publican cambios de línea y progreso por segundo. Las consultas de red nunca
ocurren por fotograma. Cada cambio de sesión/pista invalida la generación anterior,
cancela consultas y limpia letra, candidatos y carátula. La importación también
verifica su generación después de cerrar el diálogo nativo.

## IPC

- `get_state`: snapshot completo versionado y listado de sesiones.
- `select_session({id})`: selección manual; `null` la libera.
- `import_lrc`: diálogo nativo restringido a LRC; lectura acotada a 2 MiB + 1.
- `import_lrc_path({path})`: alternativa visible para una ruta absoluta `.lrc`;
  comparte lector, parser y verificación de generación con el diálogo nativo.
- `search_lyrics`: reintento manual; requiere que la búsqueda en Internet esté activada.
- `choose_candidate({id,generation})`: elige entre respuestas de la pista actual.
- `correct_search({title,artist,generation,reset})`: corrige la consulta sin tocar
  metadatos del reproductor; se descarta al cambiar de pista.
- `translate_lyrics({source,generation,lyricsRevision})` y `cancel_translation`:
  traducción optativa con documentos separados y descarte de respuestas obsoletas.
- `update_settings({settings})`: valida y persiste preferencias; geometría nativa.
- `reset_window`, `hide_window`, `quit`: ciclo de vida y recuperación.
- Evento `proyecto-reze:state`: mismo contrato que `get_state`; revisiones anteriores se
  descartan en la interfaz. Evento `proyecto-reze:settings`: abre el panel desde bandeja.

## Búsqueda

Consulta estable `/api/get` si hay duración válida y artista. Después usa búsquedas
secuenciales con metadatos originales, identidad derivada, consulta libre y título
solo como último recurso (máximo seis solicitudes con el lookup exacto incluido).
La identidad derivada extrae «Artista - Canción» de títulos de video, reconoce
comillas japonesas, elimina adornos oficiales y sufijos VEVO/Topic y normaliza
puntuación/acentos para comparar. Los metadatos originales no se modifican.

El intérprete de un cover conserva prioridad sobre el autor citado en el título.
«Cover», idiomas, «Live» y «Remix» son señales de versión; nunca se usan para
seleccionar silenciosamente la grabación original. La elección automática exige
título, artista, álbum cuando exista y duración ±2 s, sin ambigüedad de versión.
Se deduplican resultados por ID y se ordenan por título, intérprete y duración.
`matchNote` informa diferencias de artista, idioma/versión o duración antes de
confirmar. Entre búsquedas de respaldo se esperan 250 ms y se detienen al encontrar
una coincidencia de título/intérprete. Sin artista se puede buscar por título,
pero la elección permanece manual.

El cliente tiene timeout 10 s, dos solicitudes concurrentes y dos reintentos
transitorios. `429` establece cooldown compartido, respeta `Retry-After` y añade
jitter. Desactivar la red aborta búsquedas pendientes. Seleccionar candidatos ya
descargados no vuelve a consultar la API. No hay caché persistente.

## Karaoke

`karaoke_mode` (`line` / `word`) es una preferencia persistente; las configuraciones
anteriores cargan `line`. El modo efectivo se decide por fragmento: solo se usan
palabras con tiempos válidos, versión confirmada y reloj GSMTC fiable. Sin esos
datos, se conserva el modo por línea o lectura. La traducción elimina los segmentos
del original y siempre utiliza tiempos de línea.

Se consume `lyricsfile` de los registros v1 de LRCLIB, además de `hasWordSync`.
El contenido validado prevalece sobre el indicador. El lector Lyricsfile 1.0 aplica
límites de tamaño/segmentos, orden, rango temporal y reconstrucción de texto; una
línea con palabras inconsistentes conserva su texto y tiempo, sin resaltado por
palabra. Un YAML incompatible intenta el LRC o texto alternativo del registro.
No se inventan finales ausentes ni se aplica el offset ambiguo del borrador YAML.

La posición del backend se comunica a 30 Hz por `proyecto-reze:position` cuando hay un
fragmento por palabra. El paquete solo incluye posición, línea, secuencia y claves
de generación/revisión; no contiene letras ni provoca consultas HTTP. El frontend
descarta eventos obsoletos y actualiza spans de texto con progreso calculado desde
los tiempos originales. Pausas y búsquedas usan el mismo reloj que las líneas.
Movimiento reducido utiliza un cambio discreto al inicio de cada segmento.
Una duración que llega tarde desde Windows no invalida el documento de la pista;
los cambios reales de metadatos o de versión sí lo hacen.

## Almacenamiento y permisos

La selección predeterminada conserva todos los candidatos y publica un aviso si
hay varios. La coincidencia mejor ordenada se usa como lectura cuando faltan
garantías de versión; confirmar un candidato habilita sus tiempos. Las revisiones
de documento invalidan traducciones incluso al cambiar de letra en la misma pista.
MyMemory se consulta solo por acción explícita: líneas de hasta 500 bytes, máximo
5000 caracteres / 100 líneas distintas por operación, deduplicación y llamadas
secuenciales. Los errores y cuotas conservan el original. No se utilizan endpoints
de publicación de memorias de traducción ni se envían correos o credenciales.

Preferencias JSON versionadas, validadas y reemplazadas mediante archivo temporal
sincronizado. Una configuración inválida se respalda antes de usar valores iniciales.
Ventana y bandeja se controlan desde Rust; JavaScript solo recibe permiso para
eventos y arrastre. CSP sin contenido remoto; HTTPS de letras exclusivamente en
Rust, sin redirecciones. Las letras se insertan con `textContent`.

La interfaz muestra únicamente datos reales de sesión; las capturas de pruebas
usan IPC simulado y material identificado como prueba. El frontend en navegador
se identifica explícitamente como vista previa y no simula reproductores reales.
