# Material de prueba

El texto de `demo.lrc` fue creado para este proyecto y se puede usar, modificar y
redistribuir bajo CC0-1.0. No corresponde a una canción comercial.

`timing-check.lrc` también es material propio CC0: muestra una marca de diagnóstico
por segundo durante diez minutos. No contiene la letra de ninguna canción; permite
comparar la posición publicada por Windows con la línea visible de Proyecto Reze.

`translation-demo.lrc` contiene dos frases propias CC0 para comprobar traducción
sin utilizar letras comerciales. `scripts/smoke-translation.mjs` es una prueba
explícita en línea y envía únicamente ese archivo al servicio MyMemory.

`karaoke-demo.lyricsfile.yaml` también es material propio CC0. Contiene inicios y
finales de segmentos, y una palabra con solo inicio para verificar que Proyecto Reze no
inventa su duración. `scripts/smoke-karaoke.mjs` comprueba el efecto en el ejecutable
real con el fixture GSMTC y la red desactivada.

El ejemplo `gsmtc_fixture` publica metadatos y posición sintéticos usando SMTC de
Windows. No reproduce audio, consulta servicios ni modifica otras sesiones.

```powershell
cargo run -p proyecto-reze --example gsmtc_fixture
```

Selecciona su sesión en Proyecto Reze e importa `fixtures/demo.lrc`. En la consola del
fixture puedes escribir `pause`, `play`, `seek 30000`, `track Otra pista` y `quit`.
