# Privacidad, fuentes y límites

- La integración principal utiliza Windows GSMTC. No existen clientes de Spotify
  Web API, YouTube Data API, OAuth ni extracción de sesiones de navegador.
- Búsqueda LRCLIB automática solicitada por el usuario; se puede desactivar en
  ajustes. Se envían metadatos de la sesión elegida, sin correo personal ni cuentas.
  El cliente se identifica internamente como `ProyectoReze/<versión> (Windows desktop;
  app.proyectoreze.widget)`. Esta copia personal no tiene web pública ni contacto de
  proyecto: no se inventó una URL. Para una futura distribución, completar esa
  información de aplicación conforme a la documentación de LRCLIB.
- No se conserva historial de escucha ni letras en disco. Los diagnósticos no
  registran títulos, artistas, letras, credenciales o cuerpos de respuestas.
- La procedencia de cada documento distingue archivo local y LRCLIB, con ID de
  resultado cuando existe. Derechos no especificados se mantienen como desconocidos;
  el acceso a la API no se interpreta como una licencia de letras.
- Los archivos originales importados no se modifican. El material de prueba propio
  se ofrece bajo CC0; no se incluyen letras comerciales ni audio protegido.
- El uso personal y la disponibilidad de GSMTC no constituyen por sí mismos una
  autorización. Antes de distribuir deben revisarse condiciones de cada plataforma
  y permisos del contenido. No se anuncia sincronización autorizada con Spotify.

Referencias de implementación:

- [GSMTC Microsoft](https://learn.microsoft.com/en-us/uwp/api/windows.media.control.globalsystemmediatransportcontrolssessionmanager)
- [Timeline Microsoft](https://learn.microsoft.com/en-us/uwp/api/windows.media.control.globalsystemmediatransportcontrolssessiontimelineproperties)
- [LRCLIB API](https://lrclib.net/docs.txt)
- [Spotify Developer Policy](https://developer.spotify.com/policy)
- [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/)

Fuera de esta entrega: reproducción/descarga de audio, tiempos por palabra,
extensión de navegador, click-through, arranque con Windows, instalador firmado,
actualizaciones automáticas y caché persistente de letras.
