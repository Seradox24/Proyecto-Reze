# Revisión de temas, mascota y lectura

Fecha: 15 de noviembre de 2026. Rama: `feature/diseno-temas-gatitos`.

## Fallos reproducidos y correcciones

- **Marcas de tiempo dentro de la letra:** la sesión real de Brave entregó una
  letra LRCLIB/Lyricsfile temporizada cuyo campo de texto contenía etiquetas
  `[mm:ss.xx]`. Al entrar en lectura por falta de posición fiable, la interfaz
  prefería ese campo. Ahora muestra las líneas ya procesadas, sin esos prefijos.
  Las letras plain conservan sus caracteres literales.
- **Gato recortado:** cada cuadro fuente de 48 × 48 se pintaba a escala completa
  dentro de una caja de 24 × 24. El tamaño de fondo y las posiciones porcentuales
  muestran ahora una celda completa y permiten conservar el encuadre al escalar.
- **Animación irregular después del reposo:** se reutilizaba el intervalo lento
  entre parpadeos para la reproducción. Cada secuencia inicia ahora su cadencia;
  los snapshots repetidos conservan el descanso y la pose final de pausa detiene
  el temporizador. Cambiar reproducción/pausa cancela un gesto incompatible.

## Comprobaciones

- Rust: 32 pruebas correctas.
- Vitest: 13 pruebas correctas, incluidas las nuevas regresiones de lectura y reloj.
- Playwright/Edge: 20 pruebas correctas; comprobación adicional de las capturas
  Solo letras una vez terminada su transición de color.
- TypeScript/Vite y compilación release de Tauri: correctos.
- `scripts/smoke-karaoke.mjs`: 7 comprobaciones nativas correctas con el reloj
  GSMTC y archivos propios; pausa, seek y fallback por línea incluidos.
- Revisión de escritorio mediante CDP: 8 comprobaciones nativas adicionales.
  Los cuatro temas se guardaron mediante la interfaz conservando el progreso de
  palabra; el menú del gato detuvo/reanudó cuadros; ambas bandas de Solo letras
  usaron los colores esperados y un fondo de ventana transparente.

Evidencias locales, excluidas de Git: `artifacts/karaoke-smoke.json`,
`artifacts/review-native-design.json`, `artifacts/review-native-*.png`.
Estas capturas representan la aplicación compilada y usan letras de prueba propias.

La comprobación posterior de la sesión real encontró otro contenido sin letra
disponible, por lo que no se contó como repetición nativa del caso original.
La regresión automatizada cubre la transición a lectura y la vuelta al seguimiento.

## Arranque y conservación

Se recompiló `target/release/proyecto-reze.exe`. El antiguo `danae.exe` quedó
archivado en `artifacts/legacy-danae-20261115.exe` y se creó/actualizó el acceso
directo `Proyecto Reze` del escritorio para abrir el ejecutable vigente.

La ventana conservaba una geometría de 1735 × 469. Se restauró a 420 × 260;
antes se respaldó el archivo de ajustes en AppData con el prefijo
`settings.before-review-`. Las demás preferencias se conservaron y los cambios
temporales de los ensayos se restauraron al terminar.

La etiqueta y rama del respaldo anterior al diseño permanecen intactas. Las
correcciones se guardan en la rama de implementación, sin merge a `main`.
