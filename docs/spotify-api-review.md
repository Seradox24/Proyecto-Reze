# Revisión de Spotify Web API

Consulta de documentación oficial: 15-11-2026. No se añadieron clientes, OAuth,
credenciales ni endpoints privados de Spotify a Proyecto Reze.

La referencia pública consultada no documenta un endpoint de letras, LRC,
traducciones o sincronización de palabras. Search y Get Track sí pueden aportar
nombre, artistas, álbum, duración e identificador ISRC. Esos campos podrían ayudar
a identificar una grabación antes de buscar letras en una fuente independiente;
no aportan las letras ni garantizan que exista un cover en su catálogo.

La API requiere autenticación de aplicación; obtener la reproducción de un usuario
necesita autorización con los scopes correspondientes. Añadirlo implicaría otra
integración y mantenimiento frente a GSMTC, que ya cumple el seguimiento local.

Spotify Compliance Tips incluye entre los usos no permitidos sincronizar
grabaciones accedidas mediante Spotify Platform con letras u otros medios.
Por ello no se presenta la API como una solución autorizada de karaoke. Cualquier
integración futura requiere revisar términos y permisos para ese uso concreto.

Fuentes primarias:

- [Referencia Web API](https://developer.spotify.com/documentation/web-api)
- [Search](https://developer.spotify.com/documentation/web-api/reference/search)
- [Get Track](https://developer.spotify.com/documentation/web-api/reference/get-track)
- [Authorization](https://developer.spotify.com/documentation/web-api/concepts/authorization)
- [Compliance Tips](https://developer.spotify.com/compliance-tips)
