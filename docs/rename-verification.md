# Verificación del cambio a Proyecto Reze

Fecha: 15 de noviembre de 2026.

La carpeta de trabajo es `D:\Proyecto Reze`. El workspace de Cargo, los paquetes
de npm, la compilación de Tauri y Git funcionan desde esta ruta con espacios.
El ejecutable es `target/release/proyecto-reze.exe` y la ventana muestra
**Proyecto Reze**. Las rutas de código, recursos y archivos de prueba son relativas
al proyecto; no necesitan la ubicación anterior.

Comprobaciones realizadas después de mover la carpeta:

- `cargo fmt --all -- --check`: correcto.
- `cargo test --workspace`: 29 pruebas correctas.
- `npm test`: 7 pruebas correctas.
- `npm run test:e2e`: 10 pruebas correctas en Microsoft Edge.
- `npm run desktop:build`: ejecutable de producción compilado correctamente.
- `cargo build -p proyecto-reze --example gsmtc_fixture` y
  `node scripts/smoke-karaoke.mjs`: 7 comprobaciones nativas correctas, con reloj
  GSMTC y archivos propios cargados desde la nueva carpeta. Se verificaron
  tiempos por palabra, pausa, saltos hacia delante y atrás, avance continuo,
  selector por línea y fallback a línea cuando no hay tiempos por palabra.

Las comprobaciones nativas se ejecutaron con la búsqueda en red desactivada y
restauraron las preferencias al terminar. Sus resultados se guardan localmente en
`artifacts/karaoke-smoke.json`; esta carpeta está excluida de Git.

Las preferencias permanecen en `%APPDATA%\app.proyectoreze.widget\settings.json`.
La referencia al identificador anterior en el arranque se conserva para importar
ajustes de instalaciones previas; no depende del nombre de la carpeta del proyecto.

Repositorio: <https://github.com/Seradox24/Proyecto-Reze>, privado, rama `main`.
Git excluye compilaciones, dependencias, resultados de pruebas y configuración
local. No se añade configuración de facturación ni ejecución automática en GitHub.
