# Controles de reproducción por Windows GSMTC

Fecha: 15 de noviembre de 2026.

El widget permite anterior, reproducir/pausar y siguiente junto a los metadatos.
En Solo letras, los controles se revelan al pasar el ratón o recibir foco de teclado.
El audio sigue en la aplicación multimedia original. No se añadieron APIs,
credenciales ni servicios de Spotify o YouTube.

Cada fuente publica sus capacidades. Una sesión puede admitir pausa y reproducción
sin ofrecer anterior/siguiente, como la sesión Brave observada durante la revisión.
Los botones no admitidos quedan desactivados. Esa lectura de capacidades no envió
órdenes al reproductor real del usuario.

Se valida la sesión seleccionada y la generación de la pista en el IPC y antes de
despachar. Las órdenes nunca se redirigen a otra sesión si la elegida desaparece.
Hay una sola orden pendiente, un tiempo de espera limitado y mensajes de rechazo;
no existen reintentos automáticos de salto. La letra y su reloj siguen el estado
real publicado por Windows.

## Verificación

- 34 pruebas Rust, incluidas validación de contexto y rechazo de órdenes concurrentes.
- 13 pruebas Vitest y 22 pruebas de interfaz Playwright/Edge.
- TypeScript/Vite, Clippy con warnings como errores y compilación release correctos.
- `node scripts/smoke-transport.mjs`: 7 comprobaciones nativas con un fixture propio:
  capacidades, pausa y reloj congelado, reproducción e idempotencia, siguiente con
  limpieza de letra, anterior, rechazo de sesión/generación equivocadas y Solo letras.
- `node scripts/smoke-karaoke.mjs`: las 7 regresiones nativas de karaoke siguen pasando.

Los ensayos nativos usaron una sesión sintética sin audio ni consultas de red.
`smoke-transport.mjs` requiere el fixture compilado y una instancia temporal con
depuración WebView2 local en 9222, como los otros scripts smoke. Al terminar
restaura las preferencias y libera la sesión. La aplicación normal no activa CDP.

Evidencia local: `artifacts/transport-native.json`, `artifacts/transport-native.png`
y `artifacts/karaoke-smoke.json`. El respaldo anterior al diseño no se modifica.

Referencia de Windows: [GlobalSystemMediaTransportControlsSession](https://learn.microsoft.com/en-us/uwp/api/windows.media.control.globalsystemmediatransportcontrolssession).
