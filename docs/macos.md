# Distribución en macOS

## Qué se publica

Cada tag `v<versión>` dispara `.github/workflows/release-macos.yml`, que genera
`salmon-<versión>-macos-universal.tar.gz` y su `.sha256`. El archivo contiene:

- `salmon`: binario universal (`arm64` + `x86_64`).
- `README.md`.
- `examples/`: los tres ejemplos `.salmon`.

Se eligió un binario universal en lugar de dos archivos separados: es una sola
descarga, no hay que saber qué procesador tiene la Mac y pesa alrededor de 1 MB.

El workflow no publica nada solo: deja un release en borrador para que alguien
del equipo lo revise y lo publique.

## Compatibilidad

| Arquitectura | macOS mínimo declarado | Probado en |
| --- | --- | --- |
| Apple Silicon (`aarch64-apple-darwin`) | 11.0 | CI `macos-15`, macOS 27 local |
| Intel (`x86_64-apple-darwin`) | 11.0 | CI `macos-15-intel`, Rosetta en macOS 27 local |

El mínimo se fija con `MACOSX_DEPLOYMENT_TARGET=11.0` y el script de build
comprueba que ambas arquitecturas lo declaren. No hay runners de versiones
anteriores a macOS 15, así que entre 11 y 14 no se probó.

## Firma y Gatekeeper

El binario tiene solo firma ad hoc (`codesign --sign -`), no una firma de
Apple Developer ID ni notarización. Eso requiere una cuenta paga de Apple
Developer y credenciales en los secretos del repositorio.

Consecuencias verificadas:

- Si el archivo se descarga con un navegador, macOS lo marca con el atributo
  `com.apple.quarantine`. `tar` copia la marca al binario extraído y, al
  ejecutarlo, Gatekeeper lo termina sin mensaje (código 137).
- `xattr -d com.apple.quarantine salmon` quita la marca y el binario funciona.
- Un archivo descargado con `curl` no recibe la marca.

## Instalación, actualización y desinstalación

Los pasos están en el README, sección *Installing on macOS*. Resumen:

1. Verificar con `shasum -a 256 -c`.
2. Extraer, quitar la cuarentena y mover `salmon` a `~/.local/bin`.
3. Si hace falta, agregar `~/.local/bin` al `PATH` en `~/.zshrc`.

Para actualizar se repiten los pasos. Para desinstalar, `rm ~/.local/bin/salmon`.

## Homebrew

Se evaluó y no se implementa por ahora. Un tap propio
(`brew install Matias-Sanmiguel/salmon/salmon`) necesita un repositorio
`homebrew-salmon` con una fórmula que apunte al `.tar.gz` y su SHA-256, y
actualizarla en cada release. Homebrew no aplica cuarentena a lo que instala,
así que evitaría el paso de `xattr`. Conviene hacerlo cuando haya releases
periódicos, junto con la automatización de #41.

## Pruebas del binario

`scripts/macos/smoke-test.sh <binario> <carpeta-de-ejemplos>` prueba el
binario ya extraído, no el código. Copia los ejemplos a una ruta con espacios y
caracteres Unicode, crea una versión CRLF y verifica:

- uso sin argumentos, `check` válido, inválido e inexistente;
- `run` con palabra aceptada, rechazada, vacía, fuera del alfabeto y con
  comilla sin cerrar;
- archivos CRLF;
- sesión interactiva: palabras, línea vacía, error, `:quit`, entrada CRLF y
  fin de entrada.

El workflow lo corre sobre el archivo descargado en `macos-15` y
`macos-15-intel`. Para correrlo local:

```bash
./scripts/macos/build-universal.sh
tar -xzf dist/salmon-0.1.0-macos-universal.tar.gz -C /tmp
./scripts/macos/smoke-test.sh /tmp/salmon-0.1.0-macos-universal/salmon /tmp/salmon-0.1.0-macos-universal/examples
```

En una Mac Apple Silicon, `arch -x86_64 /bin/bash ./scripts/macos/smoke-test.sh ...`
prueba la parte Intel con Rosetta.
